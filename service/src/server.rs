//
// Copyright 2017-2025 Hans W. Uhlig. All Rights Reserved.
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

use crate::{TelnetConnection, TelnetError, TelnetResult};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use termionix_ansicodes::SegmentedString;
use termionix_codec::msdp::MudServerData;
use termionix_codec::mssp::MudServerStatus;
use termionix_codec::status::TelnetOptionStatus;
use termionix_terminal::{CursorPosition, TerminalCodec, TerminalEvent, TerminalSize};
use tokio::net::TcpListener;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::codec::Encoder;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, trace};

#[derive(Clone)]
pub struct TelnetServer(Arc<Mutex<InnerServer>>);

struct InnerServer {
    handle: Option<JoinHandle<()>>,
    connections: HashMap<usize, TelnetConnection>,
    token: CancellationToken,
    sequence: AtomicUsize,
    listener: TcpListener,
}

impl TelnetServer {
    pub fn create(listener: TcpListener) -> TelnetResult<TelnetServer> {
        Ok(TelnetServer(Arc::new(Mutex::new(InnerServer {
            handle: None,
            connections: HashMap::default(),
            token: CancellationToken::default(),
            sequence: AtomicUsize::new(1),
            listener,
        }))))
    }

    /// Get Server `SocketAddr`
    pub async fn addr(&self) -> TelnetResult<SocketAddr> {
        Ok(self.0.lock().await.listener.local_addr()?)
    }

    pub async fn broadcast<M, F>(&self, msg: M, filter: Option<F>) -> TelnetResult<()>
    where
        M: Clone,
        F: Fn(TelnetServer, TelnetConnection) -> bool,
        TerminalCodec: Encoder<M>,
        TelnetError: From<<TerminalCodec as Encoder<M>>::Error>,
    {
        let connections: Vec<TelnetConnection> = {
            let lock = self.0.lock().await;
            if let Some(ref filter_fn) = filter {
                lock.connections
                    .values()
                    .filter(|conn| filter_fn(self.clone(), (*conn).clone()))
                    .cloned()
                    .collect()
            } else {
                lock.connections.values().cloned().collect()
            }
        };

        // Send to all filtered connections
        for mut conn in connections {
            conn.send(msg.clone()).await?;
        }

        Ok(())
    }

    pub async fn connections<F>(&self, f: F)
    where
        F: Fn(&mut TelnetConnection),
    {
        for conn in self.0.lock().await.connections.values() {
            f(&mut conn.clone());
        }
    }

    pub async fn run<H>(&mut self, handler: Arc<H>, handle: Option<Handle>) -> TelnetResult<()>
    where
        H: TelnetServerHandler,
    {
        let handle = handle.unwrap_or_else(Handle::current);
        let cloned_token = self.0.lock().await.token.clone();
        let cloned_server = self.clone();
        let cloned_handle = handle.clone();
        let cloned_handler = handler.clone();
        let server_handle = handle.spawn(async move {
            trace!(
                "Telnet Server Listening on {}",
                cloned_server.0.lock().await.listener.local_addr().unwrap()
            );
            cloned_handler.on_startup(cloned_server.clone());
            loop {
                server_select(
                    cloned_token.clone(),
                    cloned_server.clone(),
                    cloned_handler.clone(),
                    cloned_handle.clone(),
                )
                .await;
            }
        });
        self.0.lock().await.handle = Some(server_handle);
        Ok(())
    }
    pub async fn shutdown(&self) -> TelnetResult<()> {
        // Cancel all operations
        self.0.lock().await.token.cancel();

        // Close all client connections gracefully
        let connections: Vec<TelnetConnection> =
            self.0.lock().await.connections.values().cloned().collect();

        for mut conn in connections {
            conn.cancel();
        }

        // Wait for a server task to complete
        if let Some(handle) = self.0.lock().await.handle.take() {
            let _ = handle.await;
        }

        // Clear connections
        self.0.lock().await.connections.clear();

        Ok(())
    }
}

impl std::fmt::Debug for TelnetServer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        loop {
            if let Ok(inner) = self.0.try_lock() {
                f.debug_struct("TelnetServer")
                    .field("connections", &inner.connections.len())
                    .field("address", &inner.listener.local_addr().unwrap())
                    .finish()?;
                return Ok(());
            }
        }
    }
}

pub trait TelnetServerHandler: Send + Sync + 'static {
    fn on_startup(&self, _server: TelnetServer) {
        trace!("Service Started");
    }
    /// Called on a new Client Connection
    fn on_connect(&self, _server: TelnetServer, _connection: TelnetConnection) {
        trace!("Client Connected");
    }
    /// Called when a line is completed.
    fn on_event(&self, _server: TelnetServer, _connection: TelnetConnection, event: TerminalEvent) {
        trace!("Terminal Event Received: {event:?}");
    }
    /// Called when a client experiences an error.
    fn on_error(&self, _server: TelnetServer, _connection: TelnetConnection, error: TelnetError) {
        trace!("Connection Error: {error:?}");
    }
    /// Called when a client times out.
    fn on_timeout(&self, _server: TelnetServer, _connection: TelnetConnection) {
        trace!("Client Timeout");
    }
    /// Called when a Client Disconnects
    fn on_disconnect(&self, _server: TelnetServer, _connection: TelnetConnection) {
        trace!("Client Disconnected");
    }
    /// Called when the service is shutting down.
    fn on_shutdown(&self, _server: TelnetServer) {
        trace!("Service Shutting Down");
    }
}

/// A Telnet Service that allows individual Closures to be set
pub struct ServerCallbackHandler {
    /// Called on Server Run
    pub on_startup: Option<Box<dyn Fn(TelnetServer) + Send + Sync + 'static>>,
    /// Called on a new Client Connection
    pub on_connect: Option<Box<dyn Fn(TelnetServer, TelnetConnection) + Send + Sync + 'static>>,
    /// Called on any Terminal event
    pub on_event: ServerEventHandler,
    /// Called when a client experiences an error.
    pub on_error:
        Option<Box<dyn Fn(TelnetServer, TelnetConnection, TelnetError) + Send + Sync + 'static>>,
    /// Called when a client times out.
    pub on_timeout: Option<Box<dyn Fn(TelnetServer, TelnetConnection) + Send + Sync + 'static>>,
    /// Called when a Client Disconnects
    pub on_disconnect: Option<Box<dyn Fn(TelnetServer, TelnetConnection) + Send + Sync + 'static>>,
    /// Called when the service is shutting down.
    pub on_shutdown: Option<Box<dyn Fn(TelnetServer) + Send + Sync + 'static>>,
}

impl TelnetServerHandler for ServerCallbackHandler {
    /// Called when server is started
    fn on_startup(&self, server: TelnetServer) {
        trace!("Server Started");
        if let Some(f) = &self.on_startup {
            f(server)
        }
    }
    /// Called on a new Client Connection
    fn on_connect(&self, server: TelnetServer, connection: TelnetConnection) {
        trace!("Client Connected");
        if let Some(f) = &self.on_connect {
            f(server, connection)
        }
    }
    /// Called on any Terminal event
    fn on_event(&self, server: TelnetServer, connection: TelnetConnection, event: TerminalEvent) {
        trace!("Terminal Event Received");
        match &self.on_event {
            ServerEventHandler::None => {}
            ServerEventHandler::Single { on_event } => on_event(server, connection, event),
            ServerEventHandler::Multiple {
                on_character,
                on_message,
                on_bell,
                on_clear,
                on_erase_line,
                on_erase_char,
                on_break,
                on_interrupt,
                on_cursor,
                on_resize,
                on_telnet_option_status,
                on_mud_server_data,
                on_mud_server_status,
            } => match event {
                TerminalEvent::CharacterData { character, cursor } => {
                    if let Some(f) = on_character {
                        f(server.clone(), connection.clone(), character);
                    }
                    if let Some(f) = on_cursor {
                        f(server.clone(), connection.clone(), cursor);
                    }
                }
                TerminalEvent::LineCompleted { line, cursor } => {
                    if let Some(f) = on_message {
                        f(server.clone(), connection.clone(), line);
                    }
                    if let Some(f) = on_cursor {
                        f(server.clone(), connection.clone(), cursor);
                    }
                }
                TerminalEvent::Bell => {
                    if let Some(f) = on_bell {
                        f(server.clone(), connection.clone());
                    }
                }
                TerminalEvent::Clear { cursor } => {
                    if let Some(f) = on_clear {
                        f(server.clone(), connection.clone());
                    }
                    if let Some(f) = on_cursor {
                        f(server.clone(), connection.clone(), cursor);
                    }
                }
                TerminalEvent::EraseLine { cursor } => {
                    if let Some(f) = on_erase_line {
                        f(server.clone(), connection.clone());
                    }
                    if let Some(f) = on_cursor {
                        f(server.clone(), connection.clone(), cursor);
                    }
                }
                TerminalEvent::EraseCharacter { cursor } => {
                    if let Some(f) = on_erase_char {
                        f(server.clone(), connection.clone());
                    }
                    if let Some(f) = on_cursor {
                        f(server.clone(), connection.clone(), cursor);
                    }
                }
                TerminalEvent::NoOperation => {}
                TerminalEvent::Break => {
                    if let Some(f) = on_break {
                        f(server.clone(), connection.clone());
                    }
                }
                TerminalEvent::InterruptProcess => {
                    if let Some(f) = on_interrupt {
                        f(server.clone(), connection.clone());
                    }
                }
                TerminalEvent::CursorPosition { cursor } => {
                    if let Some(f) = on_cursor {
                        f(server.clone(), connection.clone(), cursor);
                    }
                }
                TerminalEvent::ResizeWindow { old, new } => {
                    if let Some(f) = on_resize {
                        f(server.clone(), connection.clone(), old, new);
                    }
                }
                TerminalEvent::TelnetOptionStatus(tos) => {
                    if let Some(f) = on_telnet_option_status {
                        f(server.clone(), connection.clone(), tos);
                    }
                }
                TerminalEvent::MudServerData(msd) => {
                    if let Some(f) = on_mud_server_data {
                        f(server.clone(), connection.clone(), msd);
                    }
                }
                TerminalEvent::MudServerStatus(mss) => {
                    if let Some(f) = on_mud_server_status {
                        f(server.clone(), connection.clone(), mss);
                    }
                }
            },
        }
    }
    /// Called when a client experiences an error.
    fn on_error(&self, server: TelnetServer, connection: TelnetConnection, error: TelnetError) {
        trace!("Client Error");
        if let Some(f) = &self.on_error {
            f(server, connection, error)
        }
    }
    /// Called when a client times out.
    fn on_timeout(&self, server: TelnetServer, connection: TelnetConnection) {
        trace!("Client Timeout");
        if let Some(f) = &self.on_timeout {
            f(server, connection)
        }
    }
    /// Called when a Client Disconnects
    fn on_disconnect(&self, server: TelnetServer, connection: TelnetConnection) {
        trace!("Client Disconnected");
        if let Some(f) = &self.on_disconnect {
            f(server, connection)
        }
    }
    /// Called when the service is shutting down.
    fn on_shutdown(&self, server: TelnetServer) {
        trace!("Service Shutting Down");
        if let Some(f) = &self.on_shutdown {
            f(server)
        }
    }
}

pub enum ServerEventHandler {
    None,
    Single {
        on_event:
            Box<dyn Fn(TelnetServer, TelnetConnection, TerminalEvent) + Send + Sync + 'static>,
    },
    Multiple {
        on_character:
            Option<Box<dyn Fn(TelnetServer, TelnetConnection, char) + Send + Sync + 'static>>,
        on_message: Option<
            Box<dyn Fn(TelnetServer, TelnetConnection, SegmentedString) + Send + Sync + 'static>,
        >,
        on_bell: Option<Box<dyn Fn(TelnetServer, TelnetConnection) + Send + Sync + 'static>>,
        on_clear: Option<Box<dyn Fn(TelnetServer, TelnetConnection) + Send + Sync + 'static>>,
        on_erase_line: Option<Box<dyn Fn(TelnetServer, TelnetConnection) + Send + Sync + 'static>>,
        on_erase_char: Option<Box<dyn Fn(TelnetServer, TelnetConnection) + Send + Sync + 'static>>,
        on_break: Option<Box<dyn Fn(TelnetServer, TelnetConnection) + Send + Sync + 'static>>,
        on_interrupt: Option<Box<dyn Fn(TelnetServer, TelnetConnection) + Send + Sync + 'static>>,
        on_cursor: Option<
            Box<dyn Fn(TelnetServer, TelnetConnection, CursorPosition) + Send + Sync + 'static>,
        >,
        on_resize: Option<
            Box<
                dyn Fn(TelnetServer, TelnetConnection, TerminalSize, TerminalSize)
                    + Send
                    + Sync
                    + 'static,
            >,
        >,
        on_telnet_option_status: Option<
            Box<dyn Fn(TelnetServer, TelnetConnection, TelnetOptionStatus) + Send + Sync + 'static>,
        >,
        on_mud_server_data: Option<
            Box<dyn Fn(TelnetServer, TelnetConnection, MudServerData) + Send + Sync + 'static>,
        >,
        on_mud_server_status: Option<
            Box<dyn Fn(TelnetServer, TelnetConnection, MudServerStatus) + Send + Sync + 'static>,
        >,
    },
}

async fn server_select<H: TelnetServerHandler>(
    token: CancellationToken,
    server: TelnetServer,
    handler: Arc<H>,
    handle: Handle,
) -> bool {
    let mut lock = server.0.lock().await;
    tokio::select! {
        _ = token.cancelled() => {
            info!("Server Shutdown Initiated");
            handler.on_shutdown(server.clone());
            false
        }
        result = lock.listener.accept() => {
            match result{
                Ok((socket, address)) => {
                    trace!("Accepted connection from {}", address);
                    let client_token = CancellationToken::new();
                    let cloned_token = client_token.clone();
                    let cloned_server = server.clone();
                    let connection_id = lock.sequence.fetch_add(1, Ordering::Relaxed);
                    let connection = TelnetConnection::wrap(socket, client_token, connection_id);
                    let cloned_connection = connection.clone();
                    lock.connections.insert(connection_id, connection.clone());
                    *connection.handle.lock().await = Some(handle.spawn(async move {
                        loop {
                            client_select(cloned_token.clone(), cloned_server.clone(), cloned_connection.clone(), handler.clone() ).await;
                        }
                    }));
                    true
                }
                Err(err) => {
                    error!("Error Accepting Connection: {}", err);
                    false
                }
            }
        }
    }
}

async fn client_select<H: TelnetServerHandler>(
    token: CancellationToken,
    server: TelnetServer,
    mut connection: TelnetConnection,
    handler: Arc<H>,
) -> bool {
    tokio::select! {
        _ = token.cancelled() => {
            println!("Client Worker cancelled");
            handler.on_disconnect(server.clone(), connection.clone());
            false
        }
        result = connection.next() => {
            match result {
                Ok(Some(event)) => {
                    trace!("Incoming Event: {:?}", event);
                    handler.on_event(server.clone(), connection.clone(), event);
                    true
                }
                Ok(None) => {
                    info!("Connection Closed");
                    server.0.lock().await.connections.remove(&connection.id());
                    handler.on_disconnect(server.clone(), connection.clone());
                    false
                }
                Err(err) => {
                    error!("Connection Error {}", err);
                    server.0.lock().await.connections.remove(&connection.id());
                    handler.on_error(server.clone(), connection.clone(), err);
                    false
                }
            }
        }
    }
}
