//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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
use std::sync::Arc;
use termionix_ansicodes::SegmentedString;
use termionix_codec::msdp::MudServerData;
use termionix_codec::mssp::MudServerStatus;
use termionix_codec::status::TelnetOptionStatus;
use termionix_terminal::{CursorPosition, TerminalEvent, TerminalSize};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, trace};

pub async fn connect<A, H>(
    address: A,
    handler: Arc<H>,
    handle: Option<Handle>,
) -> TelnetResult<TelnetConnection>
where
    A: ToSocketAddrs,
    H: TelnetClientHandler,
{
    let token = CancellationToken::default();
    let connection = TelnetConnection::wrap(TcpStream::connect(address).await?, token.clone(), 0);
    let runtime_handle = handle.unwrap_or_else(Handle::current);
    let cloned_token = token.clone();
    let mut cloned_connection = connection.clone();
    let client_handle = runtime_handle.spawn(async move {
        handler.on_connect(cloned_connection.clone());
        loop {
            tokio::select! {
                _ = cloned_token.cancelled() => {
                    trace!("Client Worker cancelled");
                    handler.on_disconnect(cloned_connection.clone());
                    break;
                }
                result = cloned_connection.next() => {
                    match result  {
                        Ok(Some(event)) => {
                            trace!("Client Event: {:?}", event);
                            handler.on_event(cloned_connection.clone(), event);
                        }
                        Ok(None) => {
                            info!("Client Disconnected");
                            handler.on_disconnect(cloned_connection.clone());
                            break;
                        },
                        Err(err) => {
                            error!("Connection Error {}", err);
                            handler.on_error(cloned_connection.clone(), err);
                            break;
                        }
                    }
                }
            }
        }
    });
    *connection.handle.lock().await = Some(client_handle);
    Ok(connection)
}

pub trait TelnetClientHandler: Send + Sync + 'static {
    /// Called on a new Client Connection
    fn on_connect(&self, _connection: TelnetConnection) {
        trace!("Client Connected");
    }
    /// Called when a line is completed.
    fn on_event(&self, _connection: TelnetConnection, event: TerminalEvent) {
        trace!("Terminal Event Received: {event:?}");
    }
    /// Called when a client experiences an error.
    fn on_error(&self, _connection: TelnetConnection, error: TelnetError) {
        trace!("Connection Error: {error:?}");
    }
    /// Called when a client times out.
    fn on_timeout(&self, _connection: TelnetConnection) {
        trace!("Client Timeout");
    }
    /// Called when a Client Disconnects
    fn on_disconnect(&self, _connection: TelnetConnection) {
        trace!("Client Disconnected");
    }
}

/// A Telnet Service that allows individual Closures to be set
pub struct ClientCallbackHandler {
    /// Called on a new Client Connection
    pub on_connect: Option<Box<dyn Fn(TelnetConnection) + Send + Sync + 'static>>,
    /// Called on any Terminal event
    pub on_event: ClientEventHandler,
    /// Called when a client experiences an error.
    pub on_error: Option<Box<dyn Fn(TelnetConnection, TelnetError) + Send + Sync + 'static>>,
    /// Called when a client times out.
    pub on_timeout: Option<Box<dyn Fn(TelnetConnection) + Send + Sync + 'static>>,
    /// Called when a Client Disconnects
    pub on_disconnect: Option<Box<dyn Fn(TelnetConnection) + Send + Sync + 'static>>,
}

impl TelnetClientHandler for ClientCallbackHandler {
    /// Called on a new Client Connection
    fn on_connect(&self, connection: TelnetConnection) {
        trace!("Client Connected");
        if let Some(f) = &self.on_connect {
            f(connection)
        }
    }
    /// Called on any Terminal event
    fn on_event(&self, connection: TelnetConnection, event: TerminalEvent) {
        trace!("Terminal Event Received");
        match &self.on_event {
            ClientEventHandler::None => {}
            ClientEventHandler::Single { on_event } => on_event(connection, event),
            ClientEventHandler::Multiple {
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
                        f(connection.clone(), character);
                    }
                    if let Some(f) = on_cursor {
                        f(connection.clone(), cursor);
                    }
                }
                TerminalEvent::LineCompleted { line, cursor } => {
                    if let Some(f) = on_message {
                        f(connection.clone(), line);
                    }
                    if let Some(f) = on_cursor {
                        f(connection.clone(), cursor);
                    }
                }
                TerminalEvent::Bell => {
                    if let Some(f) = on_bell {
                        f(connection.clone());
                    }
                }
                TerminalEvent::Clear { cursor } => {
                    if let Some(f) = on_clear {
                        f(connection.clone());
                    }
                    if let Some(f) = on_cursor {
                        f(connection.clone(), cursor);
                    }
                }
                TerminalEvent::EraseLine { cursor } => {
                    if let Some(f) = on_erase_line {
                        f(connection.clone());
                    }
                    if let Some(f) = on_cursor {
                        f(connection.clone(), cursor);
                    }
                }
                TerminalEvent::EraseCharacter { cursor } => {
                    if let Some(f) = on_erase_char {
                        f(connection.clone());
                    }
                    if let Some(f) = on_cursor {
                        f(connection.clone(), cursor);
                    }
                }
                TerminalEvent::NoOperation => {}
                TerminalEvent::Break => {
                    if let Some(f) = on_break {
                        f(connection.clone());
                    }
                }
                TerminalEvent::InterruptProcess => {
                    if let Some(f) = on_interrupt {
                        f(connection.clone());
                    }
                }
                TerminalEvent::CursorPosition { cursor } => {
                    if let Some(f) = on_cursor {
                        f(connection.clone(), cursor);
                    }
                }
                TerminalEvent::ResizeWindow { old, new } => {
                    if let Some(f) = on_resize {
                        f(connection.clone(), old, new);
                    }
                }
                TerminalEvent::TelnetOptionStatus(tos) => {
                    if let Some(f) = on_telnet_option_status {
                        f(connection.clone(), tos);
                    }
                }
                TerminalEvent::MudServerData(msd) => {
                    if let Some(f) = on_mud_server_data {
                        f(connection.clone(), msd);
                    }
                }
                TerminalEvent::MudServerStatus(mss) => {
                    if let Some(f) = on_mud_server_status {
                        f(connection.clone(), mss);
                    }
                }
            },
        }
    }
    /// Called when a client experiences an error.
    fn on_error(&self, connection: TelnetConnection, error: TelnetError) {
        trace!("Client Error");
        if let Some(f) = &self.on_error {
            f(connection, error)
        }
    }
    /// Called when a client times out.
    fn on_timeout(&self, connection: TelnetConnection) {
        trace!("Client Timeout");
        if let Some(f) = &self.on_timeout {
            f(connection)
        }
    }
    /// Called when a Client Disconnects
    fn on_disconnect(&self, connection: TelnetConnection) {
        trace!("Client Disconnected");
        if let Some(f) = &self.on_disconnect {
            f(connection)
        }
    }
}

pub enum ClientEventHandler {
    None,
    Single {
        on_event: Box<dyn Fn(TelnetConnection, TerminalEvent) + Send + Sync + 'static>,
    },
    Multiple {
        on_character: Option<Box<dyn Fn(TelnetConnection, char) + Send + Sync + 'static>>,
        on_message: Option<Box<dyn Fn(TelnetConnection, SegmentedString) + Send + Sync + 'static>>,
        on_bell: Option<Box<dyn Fn(TelnetConnection) + Send + Sync + 'static>>,
        on_clear: Option<Box<dyn Fn(TelnetConnection) + Send + Sync + 'static>>,
        on_erase_line: Option<Box<dyn Fn(TelnetConnection) + Send + Sync + 'static>>,
        on_erase_char: Option<Box<dyn Fn(TelnetConnection) + Send + Sync + 'static>>,
        on_break: Option<Box<dyn Fn(TelnetConnection) + Send + Sync + 'static>>,
        on_interrupt: Option<Box<dyn Fn(TelnetConnection) + Send + Sync + 'static>>,
        on_cursor: Option<Box<dyn Fn(TelnetConnection, CursorPosition) + Send + Sync + 'static>>,
        on_resize: Option<
            Box<dyn Fn(TelnetConnection, TerminalSize, TerminalSize) + Send + Sync + 'static>,
        >,
        on_telnet_option_status:
            Option<Box<dyn Fn(TelnetConnection, TelnetOptionStatus) + Send + Sync + 'static>>,
        on_mud_server_data:
            Option<Box<dyn Fn(TelnetConnection, MudServerData) + Send + Sync + 'static>>,
        on_mud_server_status:
            Option<Box<dyn Fn(TelnetConnection, MudServerStatus) + Send + Sync + 'static>>,
    },
}
