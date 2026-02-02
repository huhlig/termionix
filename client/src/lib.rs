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
//! - **Terminal-Aware** - Built-in terminal emulation and ANSI code processing
//! - **Async-First** - Built on Tokio for high-performance async I/O
//!
//! ## Quick Start
//!
//! ```no_run
//! use termionix_client::{TerminalClient, ClientConfig, TerminalHandler, TerminalConnection};
//! use async_trait::async_trait;
//! use std::sync::Arc;
//!
//! struct MyHandler;
//!
//! #[async_trait]
//! impl TerminalHandler for MyHandler {
//!     async fn on_connect(&self, conn: &TerminalConnection) {
//!         println!("Connected!");
//!     }
//!     
//!     async fn on_character(&self, conn: &TerminalConnection, ch: char) {
//!         print!("{}", ch);
//!     }
//!     
//!     async fn on_line(&self, conn: &TerminalConnection, line: &str) {
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
//!     let mut client = TerminalClient::new(config);
//!     client.connect(Arc::new(MyHandler)).await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Sending Data
//!
//! ```no_run
//! # use termionix_client::{TerminalConnection, ClientError};
//! # async fn example(conn: &TerminalConnection) -> Result<(), ClientError> {
//! // Send a string
//! conn.send("Hello, server!", false).await?;
//!
//! // Send a line (appends CR LF and flushes)
//! conn.send_line("quit").await?;
//!
//! // Send a terminal command
//! use termionix_client::TerminalCommand;
//! conn.send_command(TerminalCommand::ClearScreen).await?;
//! # Ok(())
//! # }
//! ```

mod client;
mod config;
mod error;

pub use client::{ConnectionState, TerminalClient, TerminalConnection, TerminalHandler};
pub use config::ClientConfig;
pub use error::{ClientError, Result};

// Re-export types from termionix_service
pub use termionix_service::{
    gmcp, linemode, msdp, mssp, naocrd, naohts, naws, status, strip_ansi_codes,
    terminal_word_unwrap, terminal_word_wrap, AnsiApplicationProgramCommand, AnsiCodec,
    AnsiCodecError, AnsiCodecResult, AnsiConfig, AnsiControlCode, AnsiControlSequenceIntroducer,
    AnsiDeviceControlString, AnsiOperatingSystemCommand, AnsiParser, AnsiPrivacyMessage,
    AnsiSelectGraphicRendition, AnsiSequence, AnsiStartOfString, Blink, Color, ColorMode,
    CompressionAlgorithm, CursorPosition, Font, Ideogram, Intensity, SGRParameter, Script, Segment,
    SegmentedString, Span, SpannedString, StyledString, SubnegotiationErrorKind, TelnetArgument,
    TelnetCodec, TelnetCodecError, TelnetCodecResult, TelnetCommand, TelnetEvent, TelnetFrame,
    TelnetOption, TelnetSide, TerminalBuffer, TerminalCodec, TerminalCommand, TerminalError,
    TerminalEvent, TerminalResult, TerminalSize, Underline,
};
