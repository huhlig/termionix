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

//! Telnet Server Implementation
//!
//! This module provides a production-ready, async-first Telnet server implementation
//! that addresses all critical issues identified in the V1 implementation:
//!
//! - No race conditions in connection management
//! - Guaranteed resource cleanup with timeouts
//! - Proper timeout handling for idle connections
//! - Concurrent broadcast with backpressure
//! - Lock-free metrics and monitoring
//! - Clear separation of concerns
//!
//! # Architecture
//!
//! The  implementation follows a layered architecture:
//!
//! ```text
//! TelnetServer
//!     ↓
//! ConnectionManager
//!     ↓
//! ConnectionWorker → TelnetConnection
//! ```
//!
//! # Example
//!
//! ```no_run
//! use termionix_service::{TelnetServer, ServerConfig, ServerHandler, ConnectionId, TelnetConnection};
//! use termionix_terminal::TerminalEvent;
//! use async_trait::async_trait;
//!
//! struct MyHandler;
//!
//! #[async_trait]
//! impl ServerHandler for MyHandler {
//!     async fn on_event(
//!         &self,
//!         id: ConnectionId,
//!         conn: &TelnetConnection,
//!         event: TerminalEvent,
//!     ) {
//!         // Handle events
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ServerConfig::default();
//!     let server = TelnetServer::new(config).await?;
//!     server.start(std::sync::Arc::new(MyHandler)).await?;
//!     Ok(())
//! }
//! ```

mod config;
mod connection;
mod error;
mod handler;
mod manager;
mod metrics;
mod server;
mod types;
mod worker;

pub use config::ServerConfig;
pub use connection::TelnetConnection;
pub use error::{Result, TelnetError};
pub use handler::{CallbackHandler, EventHandler, ServerHandler};
pub use manager::{BroadcastResult, ConnectionManager};
pub use metrics::{MetricsSnapshot, ServerMetrics};
pub use server::TelnetServer;
pub use types::{ConnectionId, ConnectionInfo, ConnectionState, ServerSnapshot};
pub use worker::{ConnectionWorker, ControlMessage, WorkerConfig};

// Re-export types from termionix_terminal
pub use termionix_terminal::{
    CursorPosition, TerminalBuffer, TerminalCodec, TerminalCommand, TerminalError, TerminalEvent,
    TerminalResult, TerminalSize,
};

// Re-export types from termionix_telnetcodec
pub use termionix_telnetcodec::{
    CodecError as TelnetCodecError, CodecResult as TelnetCodecResult, SubnegotiationErrorKind,
    TelnetArgument, TelnetCodec, TelnetEvent, TelnetFrame, TelnetOption, TelnetSide,
};

// Re-export telnet argument modules
pub use termionix_telnetcodec::{gmcp, linemode, msdp, mssp, naocrd, naohts, naws, status};

// Re-export types from termionix_ansicodec
pub use termionix_ansicodec::{
    AnsiApplicationProgramCommand, AnsiCodec, AnsiConfig, AnsiControlCode,
    AnsiControlSequenceIntroducer, AnsiDeviceControlString, AnsiError, AnsiOperatingSystemCommand,
    AnsiParser, AnsiPrivacyMessage, AnsiResult, AnsiSelectGraphicRendition, AnsiSequence,
    AnsiStartOfString, Blink, Color, ColorMode, Font, Ideogram, Intensity, SGRParameter, Script,
    Segment, SegmentedString, Span, SpannedString, StyledString, Underline,
};

// Re-export ansi utility functions
pub use termionix_ansicodec::utility::strip_ansi_codes;

// Type aliases for convenience
pub use termionix_ansicodec::{ControlCode, Style, TelnetCommand};
