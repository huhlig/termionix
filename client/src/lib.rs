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

//! # Termionix Telnet Client
//!
//! High-level Telnet client library with automatic sidechannel negotiation,
//! reconnection support, and event-driven architecture.
//!
//! ## Features
//!
//! - **Automatic Protocol Negotiation** - Handles NAWS, terminal type, and other options
//! - **Reconnection Support** - Automatic reconnection with configurable retry logic
//! - **Event-Driven** - Handler-based API for processing server events
//! - **Type-Safe Metadata** - Store arbitrary typed data per connection
//! - **Async-First** - Built on Tokio for high-performance async I/O
//!
//! ## Quick Start
//!
//! ```no_run
//! use termionix_client::{TelnetClient, ClientConfig, ClientHandler, ClientConnection};
//! use async_trait::async_trait;
//! use std::sync::Arc;
//!
//! struct MyHandler;
//!
//! #[async_trait]
//! impl ClientHandler for MyHandler {
//!     async fn on_connect(&self, conn: &ClientConnection) {
//!         println!("Connected!");
//!     }
//!     
//!     async fn on_data(&self, conn: &ClientConnection, data: &[u8]) {
//!         print!("{}", String::from_utf8_lossy(data));
//!     }
//!     
//!     async fn on_line(&self, conn: &ClientConnection, line: &str) {
//!         println!("Received line: {}", line);
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ClientConfig::new("localhost", 4000)
//!         .with_auto_reconnect(true)
//!         .with_terminal_type("xterm-256color");
//!     
//!     let mut client = TelnetClient::new(config);
//!     client.connect(Arc::new(MyHandler)).await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Sending Data
//!
//! ```no_run
//! # use termionix_client::{ClientConnection, ClientError};
//! # async fn example(conn: &ClientConnection) -> Result<(), ClientError> {
//! // Send raw bytes
//! conn.send_bytes(b"Hello").await?;
//!
//! // Send a string
//! conn.send("Hello, server!").await?;
//!
//! // Send a line (appends CR LF)
//! conn.send_line("quit").await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Connection Metadata
//!
//! ```no_run
//! # use termionix_client::ClientConnection;
//! # async fn example(conn: &ClientConnection) {
//! #[derive(Clone)]
//! struct PlayerData {
//!     name: String,
//!     level: u32,
//! }
//!
//! // Store data
//! let player = PlayerData {
//!     name: "Alice".to_string(),
//!     level: 5,
//! };
//! conn.set_data("player", player).await;
//!
//! // Retrieve data
//! if let Some(player) = conn.get_data::<PlayerData>("player").await {
//!     println!("Player: {} (Level {})", player.name, player.level);
//! }
//! # }
//! ```

mod client;
mod config;
mod connection;
mod error;
mod handler;

pub use client::{TerminalClient, TerminalConnection, TerminalHandler};
pub use config::ClientConfig;
pub use connection::{ClientConnection, ConnectionState};
pub use error::{ClientError, Result};
pub use handler::{CallbackHandler, ClientHandler};

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
