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

//! Handler traits and implementations for the  Telnet server

use crate::{ConnectionId, TelnetConnection, TelnetError};
use async_trait::async_trait;
use termionix_terminal::TerminalEvent;

/// Server event handler trait
///
/// Implement this trait to handle events from the Telnet server.
/// All methods are async and have default implementations that do nothing.
///
/// # Example
///
/// ```no_run
/// use termionix_service::{ServerHandler, ConnectionId, TelnetConnection};
/// use termionix_terminal::TerminalEvent;
/// use async_trait::async_trait;
///
/// struct MyHandler;
///
/// #[async_trait]
/// impl ServerHandler for MyHandler {
///     async fn on_event(
///         &self,
///         id: ConnectionId,
///         conn: &TelnetConnection,
///         event: TerminalEvent,
///     ) {
///         // Handle the event
///     }
/// }
/// ```
#[async_trait]
pub trait ServerHandler: Send + Sync + 'static {
    /// Called when a new connection is established
    ///
    /// This is called after the connection is fully initialized and before
    /// any events are processed.
    async fn on_connect(&self, _id: ConnectionId, _conn: &TelnetConnection) {}

    /// Called when a terminal event is received
    ///
    /// This is the main event processing method. It will be called for every
    /// event received from the client.
    async fn on_event(&self, _id: ConnectionId, _conn: &TelnetConnection, _event: TerminalEvent) {}

    /// Called when an error occurs on a connection
    ///
    /// This is called when an error occurs during event processing. The
    /// connection will be closed after this method returns.
    async fn on_error(&self, _id: ConnectionId, _conn: &TelnetConnection, _error: TelnetError) {}

    /// Called when a read operation times out
    ///
    /// This is called when no data is received within the configured read
    /// timeout. The connection will be closed after this method returns.
    async fn on_timeout(&self, _id: ConnectionId, _conn: &TelnetConnection) {}

    /// Called when a connection is idle for too long
    ///
    /// This is called when there has been no activity on the connection for
    /// the configured idle timeout. The connection will be closed after this
    /// method returns.
    async fn on_idle_timeout(&self, _id: ConnectionId, _conn: &TelnetConnection) {}

    /// Called when a connection is disconnected
    ///
    /// This is called when the connection is closed, either by the client,
    /// the server, or due to an error.
    async fn on_disconnect(&self, _id: ConnectionId, _conn: &TelnetConnection) {}
}

/// Event handler enum for flexible event handling
///
/// This allows handlers to choose between handling all events with a single
/// callback or handling specific event types with dedicated callbacks.
pub enum EventHandler {
    /// No event handling
    None,
    /// Single callback for all events
    Single {
        /// Callback for all events
        on_event: Box<dyn Fn(ConnectionId, TerminalEvent) + Send + Sync + 'static>,
    },
    /// Multiple callbacks for specific event types
    Multiple {
        /// Callback for character data events
        on_character: Option<Box<dyn Fn(ConnectionId, char) + Send + Sync + 'static>>,
        /// Callback for line completed events
        on_line: Option<Box<dyn Fn(ConnectionId, String) + Send + Sync + 'static>>,
        // Add more specific handlers as needed
    },
}

/// Callback-based handler implementation
///
/// This provides a flexible way to implement handlers using closures instead
/// of implementing the `ServerHandler` trait.
///
/// # Example
///
/// ```no_run
/// use termionix_service::{CallbackHandler, EventHandler};
/// use std::sync::Arc;
///
/// let handler = Arc::new(CallbackHandler {
///     on_connect: Some(Box::new(|id, _conn| {
///         println!("Connection {} established", id);
///     })),
///     on_event: EventHandler::Single {
///         on_event: Box::new(|id, event| {
///             println!("Connection {} event: {:?}", id, event);
///         }),
///     },
///     on_disconnect: Some(Box::new(|id, _conn| {
///         println!("Connection {} closed", id);
///     })),
///     ..Default::default()
/// });
/// ```
pub struct CallbackHandler {
    /// Called on connection establishment
    pub on_connect: Option<Box<dyn Fn(ConnectionId, &TelnetConnection) + Send + Sync + 'static>>,
    /// Event handling strategy
    pub on_event: EventHandler,
    /// Called on error
    pub on_error:
        Option<Box<dyn Fn(ConnectionId, &TelnetConnection, TelnetError) + Send + Sync + 'static>>,
    /// Called on timeout
    pub on_timeout: Option<Box<dyn Fn(ConnectionId, &TelnetConnection) + Send + Sync + 'static>>,
    /// Called on idle timeout
    pub on_idle_timeout:
        Option<Box<dyn Fn(ConnectionId, &TelnetConnection) + Send + Sync + 'static>>,
    /// Called on disconnection
    pub on_disconnect: Option<Box<dyn Fn(ConnectionId, &TelnetConnection) + Send + Sync + 'static>>,
}

impl Default for CallbackHandler {
    fn default() -> Self {
        Self {
            on_connect: None,
            on_event: EventHandler::None,
            on_error: None,
            on_timeout: None,
            on_idle_timeout: None,
            on_disconnect: None,
        }
    }
}

#[async_trait]
impl ServerHandler for CallbackHandler {
    async fn on_connect(&self, id: ConnectionId, conn: &TelnetConnection) {
        if let Some(ref f) = self.on_connect {
            f(id, conn);
        }
    }

    async fn on_event(&self, id: ConnectionId, _conn: &TelnetConnection, event: TerminalEvent) {
        match &self.on_event {
            EventHandler::None => {}
            EventHandler::Single { on_event } => {
                on_event(id, event);
            }
            EventHandler::Multiple {
                on_character,
                on_line,
            } => match event {
                TerminalEvent::CharacterData { character, .. } => {
                    if let Some(f) = on_character {
                        f(id, character);
                    }
                }
                TerminalEvent::LineCompleted { line, .. } => {
                    if let Some(f) = on_line {
                        f(id, line.to_string());
                    }
                }
                _ => {}
            },
        }
    }

    async fn on_error(&self, id: ConnectionId, conn: &TelnetConnection, error: TelnetError) {
        if let Some(ref f) = self.on_error {
            f(id, conn, error);
        }
    }

    async fn on_timeout(&self, id: ConnectionId, conn: &TelnetConnection) {
        if let Some(ref f) = self.on_timeout {
            f(id, conn);
        }
    }

    async fn on_idle_timeout(&self, id: ConnectionId, conn: &TelnetConnection) {
        if let Some(ref f) = self.on_idle_timeout {
            f(id, conn);
        }
    }

    async fn on_disconnect(&self, id: ConnectionId, conn: &TelnetConnection) {
        if let Some(ref f) = self.on_disconnect {
            f(id, conn);
        }
    }
}
