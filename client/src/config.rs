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

//! Client configuration

use std::time::Duration;

/// Telnet client configuration
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Server hostname or IP address
    pub host: String,

    /// Server port
    pub port: u16,

    /// Terminal type to report (e.g., "xterm-256color")
    pub terminal_type: String,

    /// Terminal width in columns
    pub terminal_width: u16,

    /// Terminal height in rows
    pub terminal_height: u16,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Read timeout (None for no timeout)
    pub read_timeout: Option<Duration>,

    /// Enable automatic reconnection on logout.txt
    pub auto_reconnect: bool,

    /// Delay before reconnection attempt
    pub reconnect_delay: Duration,

    /// Maximum number of reconnection attempts (None for unlimited)
    pub max_reconnect_attempts: Option<usize>,

    /// Buffer size for incoming data
    pub buffer_size: usize,

    /// Enable keepalive
    pub keepalive: bool,

    /// Keepalive interval
    pub keepalive_interval: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 23,
            terminal_type: "xterm-256color".to_string(),
            terminal_width: 80,
            terminal_height: 24,
            connect_timeout: Duration::from_secs(10),
            read_timeout: Some(Duration::from_secs(300)), // 5 minutes
            auto_reconnect: false,
            reconnect_delay: Duration::from_secs(5),
            max_reconnect_attempts: Some(3),
            buffer_size: 8192,
            keepalive: true,
            keepalive_interval: Duration::from_secs(60),
        }
    }
}

impl ClientConfig {
    /// Create a new client configuration with the given host and port
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            ..Default::default()
        }
    }

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

    /// Set the connection timeout
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the read timeout
    pub fn with_read_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.read_timeout = timeout;
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

    /// Get the server address as a string
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
