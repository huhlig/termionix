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

//! Server configuration

use std::net::SocketAddr;
use std::time::Duration;

/// Server configuration
///
/// This structure contains all configuration options for the Telnet server.
/// Use the builder pattern methods to customize the configuration.
///
/// # Example
///
/// ```
/// use termionix_server::ServerConfig;
/// use std::time::Duration;
///
/// let config = ServerConfig::default()
///     .with_max_connections(500)
///     .with_idle_timeout(Duration::from_secs(600))
///     .with_compression(true);
/// ```
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind the server to
    pub bind_address: SocketAddr,

    /// Maximum number of concurrent connections
    pub max_connections: usize,

    /// Timeout for idle connections (no activity)
    ///
    /// Connections that have no activity for this duration will be closed.
    pub idle_timeout: Duration,

    /// Timeout for read operations
    ///
    /// If no data is received within this duration, the read operation will timeout.
    pub read_timeout: Duration,

    /// Timeout for write operations
    ///
    /// If data cannot be written within this duration, the write operation will timeout.
    pub write_timeout: Duration,

    /// Timeout for graceful shutdown
    ///
    /// The server will wait this long for connections to close gracefully before
    /// forcing them to close.
    pub shutdown_timeout: Duration,

    /// Enable compression (MCCP)
    ///
    /// When enabled, the server will negotiate compression with clients that support it.
    pub enable_compression: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:23".parse().unwrap(),
            max_connections: 1000,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(10),
            shutdown_timeout: Duration::from_secs(30),
            enable_compression: false,
        }
    }
}

impl ServerConfig {
    /// Create a new configuration with the given bind address
    ///
    /// All other settings will use their default values.
    pub fn new(bind_address: SocketAddr) -> Self {
        Self {
            bind_address,
            ..Default::default()
        }
    }

    /// Set the maximum number of concurrent connections
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// Set the idle timeout duration
    pub fn with_idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Set the read timeout duration
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Set the write timeout duration
    pub fn with_write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = timeout;
        self
    }

    /// Set the shutdown timeout duration
    pub fn with_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// Enable or disable compression
    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.enable_compression = enabled;
        self
    }

    /// Validate the configuration
    ///
    /// Returns an error if the configuration is invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.max_connections == 0 {
            return Err("max_connections must be greater than 0".to_string());
        }

        if self.idle_timeout.is_zero() {
            return Err("idle_timeout must be greater than 0".to_string());
        }

        if self.read_timeout.is_zero() {
            return Err("read_timeout must be greater than 0".to_string());
        }

        if self.write_timeout.is_zero() {
            return Err("write_timeout must be greater than 0".to_string());
        }

        if self.shutdown_timeout.is_zero() {
            return Err("shutdown_timeout must be greater than 0".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.max_connections, 1000);
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert!(!config.enable_compression);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_builder_pattern() {
        let config = ServerConfig::default()
            .with_max_connections(500)
            .with_idle_timeout(Duration::from_secs(600))
            .with_compression(true);

        assert_eq!(config.max_connections, 500);
        assert_eq!(config.idle_timeout, Duration::from_secs(600));
        assert!(config.enable_compression);
    }

    #[test]
    fn test_validation() {
        let mut config = ServerConfig::default();

        // Valid config
        assert!(config.validate().is_ok());

        // Invalid: zero max_connections
        config.max_connections = 0;
        assert!(config.validate().is_err());

        // Invalid: zero timeout
        config.max_connections = 1000;
        config.idle_timeout = Duration::ZERO;
        assert!(config.validate().is_err());
    }
}
