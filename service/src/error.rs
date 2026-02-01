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

//! Error types for the  Telnet server

use crate::types::ConnectionId;
use thiserror::Error;

/// Result type for operations
pub type Result<T> = std::result::Result<T, TelnetError>;

/// Telnet server error types
#[derive(Debug, Error)]
pub enum TelnetError {
    /// I/O error from the underlying TCP stream
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Protocol error from the codec layer
    #[error("Protocol error: {0}")]
    Protocol(#[from] termionix_telnetcodec::CodecError),

    /// Terminal error from the terminal layer
    #[error("Terminal error: {0}")]
    Terminal(#[from] termionix_terminal::TerminalError),

    /// Connection with the given ID was not found
    #[error("Connection {0} not found")]
    ConnectionNotFound(ConnectionId),

    /// Connection has been closed
    #[error("Connection closed")]
    ConnectionClosed,

    /// Operation timed out
    #[error("Operation timed out")]
    Timeout,

    /// Server is not running
    #[error("Server not running")]
    ServerNotRunning,

    /// Server is shutting down
    #[error("Server is shutting down")]
    ServerShuttingDown,

    /// Maximum number of connections reached
    #[error("Maximum connections ({0}) reached")]
    MaxConnectionsReached(usize),

    /// Resource cleanup failed
    #[error("Resource cleanup failed: {0}")]
    CleanupFailed(String),

    /// Generic error with a message
    #[error("{0}")]
    Other(String),
}

impl TelnetError {
    /// Check if the error is recoverable
    ///
    /// Recoverable errors are those that don't indicate a fatal condition
    /// and where retrying the operation might succeed.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            TelnetError::Timeout | TelnetError::ConnectionClosed | TelnetError::Io(_)
        )
    }

    /// Check if the error is a connection error
    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            TelnetError::ConnectionNotFound(_) | TelnetError::ConnectionClosed | TelnetError::Io(_)
        )
    }

    /// Check if the error is a sidechannel error
    pub fn is_protocol_error(&self) -> bool {
        matches!(self, TelnetError::Protocol(_) | TelnetError::Terminal(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_recoverable() {
        assert!(TelnetError::Timeout.is_recoverable());
        assert!(TelnetError::ConnectionClosed.is_recoverable());
        assert!(!TelnetError::ServerNotRunning.is_recoverable());
        assert!(!TelnetError::MaxConnectionsReached(100).is_recoverable());
    }

    #[test]
    fn test_error_is_connection_error() {
        assert!(TelnetError::ConnectionNotFound(ConnectionId::new(1)).is_connection_error());
        assert!(TelnetError::ConnectionClosed.is_connection_error());
        assert!(!TelnetError::Timeout.is_connection_error());
    }

    #[test]
    fn test_error_display() {
        let err = TelnetError::ConnectionNotFound(ConnectionId::new(42));
        assert_eq!(err.to_string(), "Connection conn-42 not found");

        let err = TelnetError::MaxConnectionsReached(1000);
        assert_eq!(err.to_string(), "Maximum connections (1000) reached");
    }
}
