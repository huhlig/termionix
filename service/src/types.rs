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

//! Core types for the  Telnet server

use std::fmt;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Unique identifier for a connection (monotonically increasing, never reused)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnectionId(u64);

impl ConnectionId {
    /// Create a new connection ID
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the underlying u64 value
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "conn-{}", self.0)
    }
}

/// Connection state (stored as atomic u8 for lock-free state management)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectionState {
    /// Connection is being established
    Connecting = 0,
    /// Connection is active and processing events
    Active = 1,
    /// Connection is idle (no recent activity)
    Idle = 2,
    /// Connection is closing (cleanup in progress)
    Closing = 3,
    /// Connection is closed
    Closed = 4,
}

impl ConnectionState {
    /// Convert from u8 (for atomic operations)
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Connecting,
            1 => Self::Active,
            2 => Self::Idle,
            3 => Self::Closing,
            4 => Self::Closed,
            _ => Self::Closed, // Default to closed for invalid values
        }
    }

    /// Convert to u8 (for atomic operations)
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Check if the connection is in a terminal state
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Closing | Self::Closed)
    }

    /// Check if the connection is active
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active | Self::Idle)
    }
}

impl fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connecting => write!(f, "connecting"),
            Self::Active => write!(f, "active"),
            Self::Idle => write!(f, "idle"),
            Self::Closing => write!(f, "closing"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

/// Connection information snapshot (for non-blocking queries)
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Connection ID
    pub id: ConnectionId,
    /// Current state
    pub state: ConnectionState,
    /// Peer address
    pub peer_addr: SocketAddr,
    /// When the connection was created
    pub created_at: Instant,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
}

impl ConnectionInfo {
    /// Get the connection duration
    pub fn duration(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get the idle duration
    pub fn idle_duration(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

/// Server snapshot for non-blocking debug information
#[derive(Debug, Clone)]
pub struct ServerSnapshot {
    /// Number of active connections
    pub active_connections: usize,
    /// Total connections since server start
    pub total_connections: u64,
    /// Server bind address
    pub bind_address: SocketAddr,
    /// Server uptime
    pub uptime: Duration,
    /// Server start time
    pub started_at: Instant,
}

impl fmt::Display for ServerSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TelnetServer {{ active: {}, total: {}, addr: {}, uptime: {:?} }}",
            self.active_connections, self.total_connections, self.bind_address, self.uptime
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_id() {
        let id1 = ConnectionId::new(1);
        let id2 = ConnectionId::new(2);

        assert_eq!(id1.as_u64(), 1);
        assert_eq!(id2.as_u64(), 2);
        assert_ne!(id1, id2);
        assert!(id1 < id2);
    }

    #[test]
    fn test_connection_state_conversion() {
        for state in [
            ConnectionState::Connecting,
            ConnectionState::Active,
            ConnectionState::Idle,
            ConnectionState::Closing,
            ConnectionState::Closed,
        ] {
            let as_u8 = state.as_u8();
            let back = ConnectionState::from_u8(as_u8);
            assert_eq!(state, back);
        }
    }

    #[test]
    fn test_connection_state_terminal() {
        assert!(!ConnectionState::Connecting.is_terminal());
        assert!(!ConnectionState::Active.is_terminal());
        assert!(!ConnectionState::Idle.is_terminal());
        assert!(ConnectionState::Closing.is_terminal());
        assert!(ConnectionState::Closed.is_terminal());
    }

    #[test]
    fn test_connection_state_active() {
        assert!(!ConnectionState::Connecting.is_active());
        assert!(ConnectionState::Active.is_active());
        assert!(ConnectionState::Idle.is_active());
        assert!(!ConnectionState::Closing.is_active());
        assert!(!ConnectionState::Closed.is_active());
    }
}
