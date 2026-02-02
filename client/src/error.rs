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

//! Client error types

use std::fmt;
use std::io;
use termionix_service::{TelnetCodecError, TerminalError};

/// Client error type
#[derive(Debug, Clone)]
pub enum ClientError {
    /// I/O error
    Io(String),

    /// Connection timeout
    ConnectionTimeout,

    /// Read timeout
    ReadTimeout,

    /// Connection closed by server
    ConnectionClosed,

    /// Connection refused
    ConnectionRefused,

    /// Protocol error
    ProtocolError(String),

    /// Codec error
    CodecError(String),

    /// Already connected
    AlreadyConnected,

    /// Not connected
    NotConnected,

    /// Reconnection failed
    ReconnectionFailed(usize),

    /// Custom error
    Custom(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {}", e),
            Self::ConnectionTimeout => write!(f, "Connection timeout"),
            Self::ReadTimeout => write!(f, "Read timeout"),
            Self::ConnectionClosed => write!(f, "Connection closed by server"),
            Self::ConnectionRefused => write!(f, "Connection refused"),
            Self::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            Self::CodecError(msg) => write!(f, "Codec error: {}", msg),
            Self::AlreadyConnected => write!(f, "Already connected"),
            Self::NotConnected => write!(f, "Not connected"),
            Self::ReconnectionFailed(attempts) => {
                write!(f, "Reconnection failed after {} attempts", attempts)
            }
            Self::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<io::Error> for ClientError {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::TimedOut => Self::ReadTimeout,
            io::ErrorKind::ConnectionRefused => Self::ConnectionRefused,
            io::ErrorKind::ConnectionReset | io::ErrorKind::BrokenPipe => Self::ConnectionClosed,
            _ => Self::Io(error.to_string()),
        }
    }
}

impl From<TelnetCodecError> for ClientError {
    fn from(error: TelnetCodecError) -> Self {
        Self::CodecError(error.to_string())
    }
}

impl From<TerminalError> for ClientError {
    fn from(error: TerminalError) -> Self {
        Self::CodecError(error.to_string())
    }
}

/// Client result type
pub type Result<T> = std::result::Result<T, ClientError>;
