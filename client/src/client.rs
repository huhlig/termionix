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

use crate::{ClientConfig, ClientError, Result};
use std::sync::Arc;
use termionix_service::{
    AnsiCodec, AnsiConfig, CompressionAlgorithm, SplitTerminalConnection, TelnetArgument,
    TelnetCodec, TelnetOption, TerminalCodec, TerminalCommand, TerminalEvent,
};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::timeout;
use tracing::{error, info};

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting to server
    Connecting,
    /// Connected and active
    Connected,
    /// Reconnecting after disconnect
    Reconnecting,
    /// Shutting down
    ShuttingDown,
}

/// Terminal client handler trait
#[async_trait::async_trait]
pub trait TerminalHandler: Send + Sync + 'static {
    async fn on_connect(&self, _conn: &TerminalConnection) {}
    async fn on_disconnect(&self, _conn: &TerminalConnection) {}
    async fn on_character(&self, _conn: &TerminalConnection, _ch: char) {}
    async fn on_line(&self, _conn: &TerminalConnection, _line: &str) {}
    async fn on_bell(&self, _conn: &TerminalConnection) {}
    async fn on_resize(&self, _conn: &TerminalConnection, _width: usize, _height: usize) {}

    /// Called when a Telnet option state changes
    ///
    /// This is called when an option negotiation completes successfully,
    /// whether the option is being enabled or disabled.
    ///
    /// # Arguments
    ///
    /// * `conn` - The connection handle
    /// * `option` - The Telnet option that changed
    /// * `enabled` - `true` if the option was enabled, `false` if disabled
    /// * `local` - `true` if the option changed locally, `false` if remotely
    async fn on_option_changed(
        &self,
        _conn: &TerminalConnection,
        _option: TelnetOption,
        _enabled: bool,
        _local: bool,
    ) {
    }

    /// Called when a Telnet subnegotiation is received
    ///
    /// This is called when a complete subnegotiation sequence is received
    /// from the server. Subnegotiations provide additional parameters for
    /// negotiated options.
    async fn on_subnegotiation(&self, _conn: &TerminalConnection, _subneg: TelnetArgument) {}

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

type ClientSplitConnection = SplitTerminalConnection<
    tokio::io::ReadHalf<TcpStream>,
    tokio::io::WriteHalf<TcpStream>,
    TerminalCodec<AnsiCodec<TelnetCodec>>,
>;

struct TerminalConnectionInner {
    config: ClientConfig,
    state: RwLock<ConnectionState>,
    split: ClientSplitConnection,
}

impl TerminalConnection {
    fn new(config: ClientConfig, split: ClientSplitConnection) -> Self {
        Self {
            inner: Arc::new(TerminalConnectionInner {
                config,
                state: RwLock::new(ConnectionState::Disconnected),
                split,
            }),
        }
    }

    pub async fn state(&self) -> ConnectionState {
        *self.inner.state.read().await
    }

    pub async fn is_connected(&self) -> bool {
        *self.inner.state.read().await == ConnectionState::Connected
    }

    /// Send any message type that can be encoded by the terminal codec
    ///
    /// This generic method can handle:
    /// - Text strings (`&str`, `String`)
    /// - Characters (`char`)
    /// - Terminal commands (`TerminalCommand`)
    /// - ANSI codes and sequences
    /// - Raw bytes (`Vec<u8>`)
    ///
    /// The `flush` parameter controls whether to flush immediately after sending.
    pub async fn send<M>(&self, msg: M, flush: bool) -> Result<()>
    where
        M: Into<TerminalCommand> + Send,
    {
        self.inner
            .split
            .send(msg.into(), flush)
            .await
            .map_err(|e| ClientError::Io(e.to_string()))?;
        Ok(())
    }

    /// Send a line of text (automatically adds line ending and flushes)
    pub async fn send_line(&self, text: &str) -> Result<()> {
        let line = if text.ends_with("\r\n") {
            text.to_string()
        } else if text.ends_with('\n') {
            format!("{}\r", text.trim_end_matches('\n'))
        } else {
            format!("{}\r\n", text)
        };

        self.send(line, true).await
    }

    /// Send a terminal command (always flushes)
    pub async fn send_command(&self, cmd: TerminalCommand) -> Result<()> {
        self.send(cmd, true).await
    }

    pub async fn disconnect(&self) -> Result<()> {
        *self.inner.state.write().await = ConnectionState::ShuttingDown;
        self.inner
            .split
            .close()
            .await
            .map_err(|e| ClientError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn config(&self) -> &ClientConfig {
        &self.inner.config
    }

    /// Set the compression algorithm for the connection
    ///
    /// This dynamically switches the compression algorithm used for both
    /// reading and writing. Typically called in response to Telnet MCCP
    /// (Mud Client Compression Protocol) negotiation.
    ///
    /// # Parameters
    ///
    /// - `algorithm`: The compression algorithm to use
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use termionix_client::TerminalConnection;
    /// # use termionix_compress::CompressionAlgorithm;
    /// # async fn example(conn: &TerminalConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// // Enable Gzip compression (MCCP2)
    /// conn.set_compression_algorithm(CompressionAlgorithm::Gzip).await?;
    ///
    /// // Disable compression
    /// conn.set_compression_algorithm(CompressionAlgorithm::None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn set_compression_algorithm(&self, algorithm: CompressionAlgorithm) -> Result<()> {
        self.inner
            .split
            .set_compression_algorithm(algorithm)
            .await
            .map_err(|e| ClientError::Io(e.to_string()))?;
        Ok(())
    }

    async fn set_state(&self, state: ConnectionState) {
        *self.inner.state.write().await = state;
    }

    async fn next(&self) -> Result<Option<TerminalEvent>> {
        self.inner
            .split
            .next()
            .await
            .map_err(|e| ClientError::Io(e.to_string()))
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

        // Create split connection with turbofish to help type inference
        let split = SplitTerminalConnection::<
            tokio::io::ReadHalf<TcpStream>,
            tokio::io::WriteHalf<TcpStream>,
            TerminalCodec<AnsiCodec<TelnetCodec>>,
        >::from_stream(stream, terminal_codec.clone(), terminal_codec);

        let connection = TerminalConnection::new(self.config.clone(), split);
        connection.set_state(ConnectionState::Connected).await;
        self.connection = Some(connection.clone());

        handler.on_connect(&connection).await;

        self.run_connection(connection, handler).await
    }

    async fn run_connection<H: TerminalHandler>(
        &self,
        connection: TerminalConnection,
        handler: Arc<H>,
    ) -> Result<()> {
        loop {
            match connection.next().await {
                Ok(Some(event)) => {
                    if !self
                        .handle_terminal_event(&connection, event, &handler)
                        .await?
                    {
                        break;
                    }
                }
                Ok(None) => {
                    info!("Server closed connection");
                    break;
                }
                Err(e) => {
                    error!("Terminal error: {}", e);
                    handler.on_error(&connection, e).await;
                    return Err(ClientError::Io("Terminal error".to_string()));
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

    pub fn connection(&self) -> Option<&TerminalConnection> {
        self.connection.as_ref()
    }
}


