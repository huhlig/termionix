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

//! Connection configuration types and builders
//!
//! This module provides configuration types for both client and server connections,
//! including common settings, flush strategies, and connection-specific options.
//!
//! # Examples
//!
//! ## Client Configuration
//!
//! ```
//! use termionix_service::ClientConnectionConfig;
//! use std::time::Duration;
//!
//! let config = ClientConnectionConfig::new("example.com", 23)
//!     .with_auto_reconnect(true)
//!     .with_reconnect_delay(Duration::from_secs(5))
//!     .with_terminal_size(120, 40);
//! ```
//!
//! ## Server Configuration
//!
//! ```
//! use termionix_service::ServerConnectionConfig;
//! use std::time::Duration;
//!
//! let config = ServerConnectionConfig::new()
//!     .with_max_idle_time(Some(Duration::from_secs(600)))
//!     .with_terminal_size(80, 24);
//! ```

use std::time::Duration;

/// Common connection configuration shared by both client and server
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Terminal type (e.g., "xterm-256color")
    pub terminal_type: String,

    /// Terminal width in columns
    pub terminal_width: u16,

    /// Terminal height in rows
    pub terminal_height: u16,

    /// Buffer size for incoming/outgoing data
    pub buffer_size: usize,

    /// Enable TCP keepalive
    pub keepalive: bool,

    /// Keepalive interval
    pub keepalive_interval: Duration,

    /// Read timeout (None for no timeout)
    pub read_timeout: Option<Duration>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            terminal_type: "xterm-256color".to_string(),
            terminal_width: 80,
            terminal_height: 24,
            buffer_size: 8192,
            keepalive: true,
            keepalive_interval: Duration::from_secs(60),
            read_timeout: Some(Duration::from_secs(300)), // 5 minutes
        }
    }
}

impl ConnectionConfig {
    /// Set the terminal type
    pub fn with_terminal_type(mut self, terminal_type: impl Into<String>) -> Self {
        self.terminal_type = terminal_type.into();
        self
    }

    /// Set the terminal size
    pub fn with_terminal_size(mut self, width: u16, height: u16) -> Self {
        self.terminal_width = width;
        self.terminal_height = height;
        self
    }

    /// Set the buffer size
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Enable or disable keepalive
    pub fn with_keepalive(mut self, enabled: bool) -> Self {
        self.keepalive = enabled;
        self
    }

    /// Set the keepalive interval
    pub fn with_keepalive_interval(mut self, interval: Duration) -> Self {
        self.keepalive_interval = interval;
        self
    }

    /// Set the read timeout
    pub fn with_read_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.read_timeout = timeout;
        self
    }
}

/// Client-side connection configuration
///
/// This configuration is used by client connections to manage connection
/// behavior, reconnection strategies, and server connection details.
#[derive(Debug, Clone)]
pub struct ClientConnectionConfig {
    /// Common connection settings
    pub common: ConnectionConfig,

    /// Server hostname or IP address
    pub host: String,

    /// Server port
    pub port: u16,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Enable automatic reconnection on disconnect
    pub auto_reconnect: bool,

    /// Delay before reconnection attempt
    pub reconnect_delay: Duration,

    /// Maximum number of reconnection attempts (None for unlimited)
    pub max_reconnect_attempts: Option<usize>,
}

impl Default for ClientConnectionConfig {
    fn default() -> Self {
        Self {
            common: ConnectionConfig::default(),
            host: "localhost".to_string(),
            port: 23,
            connect_timeout: Duration::from_secs(10),
            auto_reconnect: false,
            reconnect_delay: Duration::from_secs(5),
            max_reconnect_attempts: Some(3),
        }
    }
}

impl ClientConnectionConfig {
    /// Create a new client configuration with the given host and port
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            ..Default::default()
        }
    }

    /// Set the connection timeout
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Enable automatic reconnection
    pub fn with_auto_reconnect(mut self, enabled: bool) -> Self {
        self.auto_reconnect = enabled;
        self
    }

    /// Set the reconnection delay
    pub fn with_reconnect_delay(mut self, delay: Duration) -> Self {
        self.reconnect_delay = delay;
        self
    }

    /// Set the maximum reconnection attempts
    pub fn with_max_reconnect_attempts(mut self, max: Option<usize>) -> Self {
        self.max_reconnect_attempts = max;
        self
    }

    /// Set the terminal type
    pub fn with_terminal_type(mut self, terminal_type: impl Into<String>) -> Self {
        self.common.terminal_type = terminal_type.into();
        self
    }

    /// Set the terminal size
    pub fn with_terminal_size(mut self, width: u16, height: u16) -> Self {
        self.common.terminal_width = width;
        self.common.terminal_height = height;
        self
    }

    /// Set the buffer size
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.common.buffer_size = size;
        self
    }

    /// Get the server address as a string
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Server-side connection configuration
///
/// This configuration is used by server connections to manage per-connection
/// behavior and limits.
#[derive(Debug, Clone)]
pub struct ServerConnectionConfig {
    /// Common connection settings
    pub common: ConnectionConfig,

    /// Maximum idle time before disconnecting (None for no limit)
    pub max_idle_time: Option<Duration>,

    /// Maximum connection duration (None for no limit)
    pub max_connection_time: Option<Duration>,

    /// Enable connection rate limiting
    pub rate_limiting: bool,

    /// Maximum messages per second (if rate limiting enabled)
    pub max_messages_per_second: Option<usize>,
}

impl Default for ServerConnectionConfig {
    fn default() -> Self {
        Self {
            common: ConnectionConfig::default(),
            max_idle_time: Some(Duration::from_secs(600)), // 10 minutes
            max_connection_time: None,
            rate_limiting: false,
            max_messages_per_second: None,
        }
    }
}

impl ServerConnectionConfig {
    /// Create a new server configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum idle time
    pub fn with_max_idle_time(mut self, timeout: Option<Duration>) -> Self {
        self.max_idle_time = timeout;
        self
    }

    /// Set the maximum connection time
    pub fn with_max_connection_time(mut self, timeout: Option<Duration>) -> Self {
        self.max_connection_time = timeout;
        self
    }

    /// Enable rate limiting
    pub fn with_rate_limiting(mut self, enabled: bool, max_per_second: Option<usize>) -> Self {
        self.rate_limiting = enabled;
        self.max_messages_per_second = max_per_second;
        self
    }

    /// Set the terminal type
    pub fn with_terminal_type(mut self, terminal_type: impl Into<String>) -> Self {
        self.common.terminal_type = terminal_type.into();
        self
    }

    /// Set the terminal size
    pub fn with_terminal_size(mut self, width: u16, height: u16) -> Self {
        self.common.terminal_width = width;
        self.common.terminal_height = height;
        self
    }

    /// Set the buffer size
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.common.buffer_size = size;
        self
    }
}

/// Connection configuration enum that can be either client or server
#[derive(Debug, Clone)]
pub enum Config {
    /// Client-side configuration
    Client(ClientConnectionConfig),
    /// Server-side configuration
    Server(ServerConnectionConfig),
}

impl Config {
    /// Get the common configuration
    pub fn common(&self) -> &ConnectionConfig {
        match self {
            Config::Client(c) => &c.common,
            Config::Server(s) => &s.common,
        }
    }

    /// Get the common configuration mutably
    pub fn common_mut(&mut self) -> &mut ConnectionConfig {
        match self {
            Config::Client(c) => &mut c.common,
            Config::Server(s) => &mut s.common,
        }
    }

    /// Check if this is a client configuration
    pub fn is_client(&self) -> bool {
        matches!(self, Config::Client(_))
    }

    /// Check if this is a server configuration
    pub fn is_server(&self) -> bool {
        matches!(self, Config::Server(_))
    }

    /// Get as client config if it is one
    pub fn as_client(&self) -> Option<&ClientConnectionConfig> {
        match self {
            Config::Client(c) => Some(c),
            _ => None,
        }
    }

    /// Get as server config if it is one
    pub fn as_server(&self) -> Option<&ServerConnectionConfig> {
        match self {
            Config::Server(s) => Some(s),
            _ => None,
        }
    }
}

impl From<ClientConnectionConfig> for Config {
    fn from(config: ClientConnectionConfig) -> Self {
        Config::Client(config)
    }
}

impl From<ServerConnectionConfig> for Config {
    fn from(config: ServerConnectionConfig) -> Self {
        Config::Server(config)
    }
}

/// Flush strategy determines when buffered data should be flushed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushStrategy {
    /// Never auto-flush, manual flush only
    Manual,

    /// Flush on every send operation
    Immediate,

    /// Flush when newline is detected
    OnNewline,

    /// Flush when buffer reaches threshold (in bytes)
    OnThreshold(usize),
}

impl Default for FlushStrategy {
    fn default() -> Self {
        Self::OnNewline
    }
}
