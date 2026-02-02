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

//! Client event handler traits

use crate::{ClientConnection, ClientError};
use async_trait::async_trait;
use termionix_telnetcodec::{TelnetEvent, TelnetOption};

/// Client event handler trait
///
/// Implement this trait to handle events from the Telnet client.
/// All methods are async and have default implementations that do nothing.
///
/// # Example
///
/// ```no_run
/// use termionix_client::{ClientHandler, ClientConnection};
/// use termionix_telnetcodec::TelnetEvent;
/// use async_trait::async_trait;
///
/// struct MyHandler;
///
/// #[async_trait]
/// impl ClientHandler for MyHandler {
///     async fn on_connect(&self, conn: &ClientConnection) {
///         println!("Connected to server!");
///     }
///     
///     async fn on_data(&self, conn: &ClientConnection, data: &[u8]) {
///         print!("{}", String::from_utf8_lossy(data));
///     }
/// }
/// ```
#[async_trait]
pub trait ClientHandler: Send + Sync + 'static {
    /// Called when connection is established
    ///
    /// This is called after the TCP connection is established but before
    /// any sidechannel negotiation occurs.
    async fn on_connect(&self, _conn: &ClientConnection) {}

    /// Called when raw data is received from the server
    ///
    /// This is called for every data byte received. For line-based processing,
    /// use `on_line` instead.
    async fn on_data(&self, _conn: &ClientConnection, _data: &[u8]) {}

    /// Called when a complete line is received
    ///
    /// A line is defined as text terminated by CR LF, LF, or CR.
    async fn on_line(&self, _conn: &ClientConnection, _line: &str) {}

    /// Called when a Telnet event is received
    ///
    /// This provides access to the raw Telnet sidechannel events for advanced
    /// handling of sidechannel negotiation and commands.
    async fn on_telnet_event(&self, _conn: &ClientConnection, _event: TelnetEvent) {}

    /// Called when a Telnet option is negotiated
    ///
    /// This is called when an option negotiation completes successfully.
    async fn on_option_changed(
        &self,
        _conn: &ClientConnection,
        _option: TelnetOption,
        _enabled: bool,
    ) {
    }

    /// Called when window size is requested by server
    ///
    /// Return the desired window size, or None to use the configured default.
    async fn on_window_size_request(&self, _conn: &ClientConnection) -> Option<(u16, u16)> {
        None
    }

    /// Called when terminal type is requested by server
    ///
    /// Return the desired terminal type, or None to use the configured default.
    async fn on_terminal_type_request(&self, _conn: &ClientConnection) -> Option<String> {
        None
    }

    /// Called when an error occurs
    ///
    /// The connection will be closed after this method returns unless
    /// auto-reconnect is enabled.
    async fn on_error(&self, _conn: &ClientConnection, _error: ClientError) {}

    /// Called when connection is disconnected
    ///
    /// This is called when the connection is closed, either by the server,
    /// the client, or due to an error.
    async fn on_disconnect(&self, _conn: &ClientConnection) {}

    /// Called before attempting to reconnect
    ///
    /// Return false to cancel the reconnection attempt.
    async fn on_reconnect_attempt(&self, _conn: &ClientConnection, _attempt: usize) -> bool {
        true
    }

    /// Called when reconnection succeeds
    async fn on_reconnected(&self, _conn: &ClientConnection) {}

    /// Called when all reconnection attempts have failed
    async fn on_reconnect_failed(&self, _conn: &ClientConnection) {}
}

/// Callback-based handler implementation
///
/// This provides a flexible way to implement handlers using closures instead
/// of implementing the `ClientHandler` trait.
///
/// # Example
///
/// ```no_run
/// use termionix_client::CallbackHandler;
/// use std::sync::Arc;
///
/// let handler = Arc::new(CallbackHandler {
///     on_connect: Some(Box::new(|_conn| {
///         println!("Connected!");
///     })),
///     on_data: Some(Box::new(|_conn, data| {
///         print!("{}", String::from_utf8_lossy(data));
///     })),
///     on_disconnect: Some(Box::new(|_conn| {
///         println!("Disconnected!");
///     })),
///     ..Default::default()
/// });
/// ```
pub struct CallbackHandler {
    /// Called on connection establishment
    pub on_connect: Option<Box<dyn Fn(&ClientConnection) + Send + Sync + 'static>>,

    /// Called on data received
    pub on_data: Option<Box<dyn Fn(&ClientConnection, &[u8]) + Send + Sync + 'static>>,

    /// Called on line received
    pub on_line: Option<Box<dyn Fn(&ClientConnection, &str) + Send + Sync + 'static>>,

    /// Called on Telnet event
    pub on_telnet_event:
        Option<Box<dyn Fn(&ClientConnection, TelnetEvent) + Send + Sync + 'static>>,

    /// Called on option changed
    pub on_option_changed:
        Option<Box<dyn Fn(&ClientConnection, TelnetOption, bool) + Send + Sync + 'static>>,

    /// Called on error
    pub on_error: Option<Box<dyn Fn(&ClientConnection, ClientError) + Send + Sync + 'static>>,

    /// Called on disconnection
    pub on_disconnect: Option<Box<dyn Fn(&ClientConnection) + Send + Sync + 'static>>,

    /// Called on reconnect attempt
    pub on_reconnect_attempt:
        Option<Box<dyn Fn(&ClientConnection, usize) -> bool + Send + Sync + 'static>>,

    /// Called on reconnected
    pub on_reconnected: Option<Box<dyn Fn(&ClientConnection) + Send + Sync + 'static>>,

    /// Called on reconnect failed
    pub on_reconnect_failed: Option<Box<dyn Fn(&ClientConnection) + Send + Sync + 'static>>,
}

impl Default for CallbackHandler {
    fn default() -> Self {
        Self {
            on_connect: None,
            on_data: None,
            on_line: None,
            on_telnet_event: None,
            on_option_changed: None,
            on_error: None,
            on_disconnect: None,
            on_reconnect_attempt: None,
            on_reconnected: None,
            on_reconnect_failed: None,
        }
    }
}

#[async_trait]
impl ClientHandler for CallbackHandler {
    async fn on_connect(&self, conn: &ClientConnection) {
        if let Some(ref f) = self.on_connect {
            f(conn);
        }
    }

    async fn on_data(&self, conn: &ClientConnection, data: &[u8]) {
        if let Some(ref f) = self.on_data {
            f(conn, data);
        }
    }

    async fn on_line(&self, conn: &ClientConnection, line: &str) {
        if let Some(ref f) = self.on_line {
            f(conn, line);
        }
    }

    async fn on_telnet_event(&self, conn: &ClientConnection, event: TelnetEvent) {
        if let Some(ref f) = self.on_telnet_event {
            f(conn, event);
        }
    }

    async fn on_option_changed(
        &self,
        conn: &ClientConnection,
        option: TelnetOption,
        enabled: bool,
    ) {
        if let Some(ref f) = self.on_option_changed {
            f(conn, option, enabled);
        }
    }

    async fn on_error(&self, conn: &ClientConnection, error: ClientError) {
        if let Some(ref f) = self.on_error {
            f(conn, error);
        }
    }

    async fn on_disconnect(&self, conn: &ClientConnection) {
        if let Some(ref f) = self.on_disconnect {
            f(conn);
        }
    }

    async fn on_reconnect_attempt(&self, conn: &ClientConnection, attempt: usize) -> bool {
        if let Some(ref f) = self.on_reconnect_attempt {
            f(conn, attempt)
        } else {
            true
        }
    }

    async fn on_reconnected(&self, conn: &ClientConnection) {
        if let Some(ref f) = self.on_reconnected {
            f(conn);
        }
    }

    async fn on_reconnect_failed(&self, conn: &ClientConnection) {
        if let Some(ref f) = self.on_reconnect_failed {
            f(conn);
        }
    }
}
