//
// Copyright 2017-2026 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//! Terminal-aware Telnet client implementation

use crate::{ClientConfig, ClientError, ConnectionState, Result};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use termionix_ansicodec::{AnsiCodec, AnsiConfig};
use termionix_telnetcodec::TelnetCodec;
use termionix_terminal::{TerminalCodec, TerminalEvent};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tokio_util::codec::Framed;
use tracing::{error, info};

/// Terminal client handler trait
#[async_trait::async_trait]
pub trait TerminalHandler: Send + Sync + 'static {
    async fn on_connect(&self, _conn: &TerminalConnection) {}
    async fn on_disconnect(&self, _conn: &TerminalConnection) {}
    async fn on_character(&self, _conn: &TerminalConnection, _ch: char) {}
    async fn on_line(&self, _conn: &TerminalConnection, _line: &str) {}
    async fn on_bell(&self, _conn: &TerminalConnection) {}
    async fn on_resize(&self, _conn: &TerminalConnection, _width: usize, _height: usize) {}

    /// Called when a Telnet option is enabled
    ///
    /// This is called when a Telnet option negotiation completes successfully
    /// and the option is enabled for either the local or remote side.
    async fn on_option_enabled(
        &self,
        _conn: &TerminalConnection,
        _option: termionix_telnetcodec::TelnetOption,
        _local: bool,
    ) {
    }

    /// Called when a Telnet subnegotiation is received
    ///
    /// This is called when a complete subnegotiation sequence is received
    /// from the server. Subnegotiations provide additional parameters for
    /// negotiated options.
    async fn on_subnegotiation(
        &self,
        _conn: &TerminalConnection,
        _subneg: termionix_telnetcodec::TelnetArgument,
    ) {
    }

    async fn on_error(&self, _conn: &TerminalConnection, _error: ClientError) {}
    async fn on_reconnect_attempt(&self, _conn: &TerminalConnection, _attempt: u32) -> bool {
        true
    }
    async fn on_reconnect_failed(&self, _conn: &TerminalConnection) {}
}

/// Terminal connection wrapper
#[derive(Clone)]
pub struct TerminalConnection {
    inner: Arc<TerminalConnectionInner>,
}

struct TerminalConnectionInner {
    config: ClientConfig,
    state: RwLock<ConnectionState>,
    tx: mpsc::UnboundedSender<TerminalCommand>,
}

#[derive(Debug)]
enum TerminalCommand {
    SendText(String),
    SendLine(String),
    Disconnect,
}

impl TerminalConnection {
    fn new(config: ClientConfig, tx: mpsc::UnboundedSender<TerminalCommand>) -> Self {
        Self {
            inner: Arc::new(TerminalConnectionInner {
                config,
                state: RwLock::new(ConnectionState::Disconnected),
                tx,
            }),
        }
    }

    pub async fn state(&self) -> ConnectionState {
        *self.inner.state.read().await
    }

    pub async fn is_connected(&self) -> bool {
        *self.inner.state.read().await == ConnectionState::Connected
    }

    pub async fn send(&self, text: &str) -> Result<()> {
        self.inner
            .tx
            .send(TerminalCommand::SendText(text.to_string()))
            .map_err(|_| ClientError::NotConnected)?;
        Ok(())
    }

    pub async fn send_line(&self, text: &str) -> Result<()> {
        self.inner
            .tx
            .send(TerminalCommand::SendLine(text.to_string()))
            .map_err(|_| ClientError::NotConnected)?;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        self.inner
            .tx
            .send(TerminalCommand::Disconnect)
            .map_err(|_| ClientError::NotConnected)?;
        *self.inner.state.write().await = ConnectionState::ShuttingDown;
        Ok(())
    }

    pub fn config(&self) -> &ClientConfig {
        &self.inner.config
    }

    async fn set_state(&self, state: ConnectionState) {
        *self.inner.state.write().await = state;
    }
}

/// Terminal-aware Telnet client
pub struct TerminalClient {
    config: ClientConfig,
    connection: Option<TerminalConnection>,
}

impl TerminalClient {
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            connection: None,
        }
    }

    pub async fn connect<H: TerminalHandler>(&mut self, handler: Arc<H>) -> Result<()> {
        let mut reconnect_attempts = 0;

        loop {
            match self.connect_once(handler.clone()).await {
                Ok(()) => {
                    info!("Connection closed normally");
                    break;
                }
                Err(e) => {
                    error!("Connection error: {}", e);

                    if !self.config.auto_reconnect {
                        return Err(e);
                    }

                    reconnect_attempts += 1;

                    if let Some(max) = self.config.max_reconnect_attempts {
                        if reconnect_attempts >= max {
                            if let Some(ref conn) = self.connection {
                                handler.on_reconnect_failed(conn).await;
                            }
                            return Err(ClientError::ReconnectionFailed(reconnect_attempts));
                        }
                    }

                    if let Some(ref conn) = self.connection {
                        if !handler
                            .on_reconnect_attempt(conn, reconnect_attempts as u32)
                            .await
                        {
                            return Err(ClientError::ReconnectionFailed(reconnect_attempts));
                        }
                    }

                    info!(
                        "Reconnecting in {:?} (attempt {})...",
                        self.config.reconnect_delay, reconnect_attempts
                    );
                    tokio::time::sleep(self.config.reconnect_delay).await;
                }
            }
        }

        Ok(())
    }

    async fn connect_once<H: TerminalHandler>(&mut self, handler: Arc<H>) -> Result<()> {
        let (tx, rx) = mpsc::unbounded_channel();
        let connection = TerminalConnection::new(self.config.clone(), tx);
        connection.set_state(ConnectionState::Connecting).await;

        let addr = self.config.address();
        info!("Connecting to {}...", addr);

        let stream = match timeout(self.config.connect_timeout, TcpStream::connect(&addr)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err(ClientError::ConnectionTimeout),
        };

        info!("Connected to {}", stream.peer_addr()?);

        // Create codec stack: Terminal -> ANSI -> Telnet
        let telnet_codec = TelnetCodec::new();
        let ansi_codec = AnsiCodec::new(AnsiConfig::default(), telnet_codec);
        let terminal_codec = TerminalCodec::new(ansi_codec);
        let framed = Framed::new(stream, terminal_codec);

        connection.set_state(ConnectionState::Connected).await;
        self.connection = Some(connection.clone());

        handler.on_connect(&connection).await;

        self.run_connection(connection, framed, rx, handler).await
    }

    async fn run_connection<H: TerminalHandler>(
        &self,
        connection: TerminalConnection,
        mut framed: Framed<TcpStream, TerminalCodec<AnsiCodec<TelnetCodec>>>,
        mut rx: mpsc::UnboundedReceiver<TerminalCommand>,
        handler: Arc<H>,
    ) -> Result<()> {
        loop {
            tokio::select! {
                result = framed.next() => {
                    match result {
                        Some(Ok(event)) => {
                            if !self.handle_terminal_event(&connection, event, &handler).await? {
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            error!("Terminal error: {}", e);
                            let err = ClientError::Io(e.to_string());
                            handler.on_error(&connection, err).await;
                            return Err(ClientError::Io(format!("Terminal error")));
                        }
                        None => {
                            info!("Server closed connection");
                            break;
                        }
                    }
                }

                Some(cmd) = rx.recv() => {
                    if !self.handle_command(&mut framed, cmd).await? {
                        break;
                    }
                }
            }
        }

        connection.set_state(ConnectionState::Disconnected).await;
        handler.on_disconnect(&connection).await;

        Ok(())
    }

    async fn handle_terminal_event<H: TerminalHandler>(
        &self,
        connection: &TerminalConnection,
        event: TerminalEvent,
        handler: &Arc<H>,
    ) -> Result<bool> {
        match event {
            TerminalEvent::CharacterData { character, .. } => {
                handler.on_character(connection, character).await;
            }
            TerminalEvent::LineCompleted { line, .. } => {
                let line_str = line.to_string();
                handler.on_line(connection, &line_str).await;
            }
            TerminalEvent::Bell => {
                handler.on_bell(connection).await;
            }
            TerminalEvent::ResizeWindow { new, .. } => {
                handler.on_resize(connection, new.cols, new.rows).await;
            }
            _ => {}
        }

        Ok(true)
    }

    async fn handle_command(
        &self,
        framed: &mut Framed<TcpStream, TerminalCodec<AnsiCodec<TelnetCodec>>>,
        cmd: TerminalCommand,
    ) -> Result<bool> {
        match cmd {
            TerminalCommand::SendText(text) => {
                framed.send(&text).await?;
            }
            TerminalCommand::SendLine(text) => {
                let mut line = text;
                line.push_str("\r\n");
                framed.send(&line).await?;
            }
            TerminalCommand::Disconnect => {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn connection(&self) -> Option<&TerminalConnection> {
        self.connection.as_ref()
    }
}
