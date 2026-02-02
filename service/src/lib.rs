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

//! Termionix Service - Unified connection layer for Telnet/Terminal services
//!
//! This crate provides a unified connection abstraction with split read/write
//! streams to avoid blocking issues where buffered writes wait for read
//! timeouts before being flushed.
//!
//! # Overview
//!
//! The core type is [`SplitTerminalConnection`], which separates read and write
//! operations into independent background workers. This architecture solves the
//! common problem where buffered writes wait for read operations to complete.
//!
//! # Features
//!
//! - **Independent Read/Write**: Reads never block writes and vice versa
//! - **Configurable Flushing**: Multiple flush strategies available
//! - **Cloneable**: Safe to clone for multi-threaded access
//! - **Type-Safe**: Uses concrete terminal types for events and commands
//!
//! # Quick Start
//!
//! ```no_run
//! use termionix_service::{SplitTerminalConnection, ClientConnectionConfig};
//! use termionix_terminal::{TerminalCodec, TerminalCommand};
//! use termionix_ansicodec::{AnsiCodec, AnsiConfig};
//! use termionix_telnetcodec::TelnetCodec;
//! use tokio::net::TcpStream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to server
//! let stream = TcpStream::connect("localhost:23").await?;
//!
//! // Create codec stack
//! let telnet_codec = TelnetCodec::new();
//! let ansi_codec = AnsiCodec::new(AnsiConfig::default(), telnet_codec);
//! let codec = TerminalCodec::new(ansi_codec);
//!
//! // Create connection
//! let conn = SplitTerminalConnection::from_stream(
//!     stream,
//!     codec.clone(),
//!     codec,
//! );
//!
//! // Send data (never blocks on reads)
//! conn.send(TerminalCommand::Text("Hello\n".to_string()), true).await?;
//!
//! // Receive data (never blocks on writes)
//! while let Some(event) = conn.next().await? {
//!     println!("Received: {:?}", event);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Configuration
//!
//! The crate provides configuration types for both client and server connections:
//!
//! - [`ClientConnectionConfig`]: Client-specific settings including reconnection
//! - [`ServerConnectionConfig`]: Server-specific settings including limits
//! - [`ConnectionConfig`]: Common settings shared by both
//!
//! See the [`config`] module for detailed configuration options.

mod config;
mod connection;
mod result;

pub use config::{
    ClientConnectionConfig, Config, ConnectionConfig, FlushStrategy, ServerConnectionConfig,
};
pub use connection::SplitTerminalConnection;
pub use result::{ConnectionError, ConnectionResult};

// Re-export terminal types for convenience
pub use termionix_compress::{CompressionAlgorithm, CompressionStream};
pub use termionix_terminal::{
    AnsiApplicationProgramCommand, AnsiCodec, AnsiCodecError, AnsiCodecResult, AnsiConfig,
    AnsiControlCode, AnsiControlSequenceIntroducer, AnsiDeviceControlString,
    AnsiOperatingSystemCommand, AnsiParser, AnsiPrivacyMessage, AnsiSelectGraphicRendition,
    AnsiSequence, AnsiStartOfString, Blink, Color, ColorMode, CursorPosition, Font, Ideogram,
    Intensity, SGRParameter, Script, Segment, SegmentedString, Span, SpannedString, StyledString,
    SubnegotiationErrorKind, TelnetArgument, TelnetCodec, TelnetCodecError, TelnetCodecResult,
    TelnetCommand, TelnetEvent, TelnetFrame, TelnetOption, TelnetSide, TerminalBuffer,
    TerminalCodec, TerminalCommand, TerminalError, TerminalEvent, TerminalResult, TerminalSize,
    Underline, gmcp, linemode, msdp, mssp, naocrd, naohts, naws, status, strip_ansi_codes,
    terminal_word_unwrap, terminal_word_wrap,
};
