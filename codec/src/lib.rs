//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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

//! # Termionix Telnet Protocol Codec
//!
//! This crate provides a complete implementation of the Telnet protocol codec for encoding and
//! decoding Telnet protocol messages. It is designed to work with asynchronous networking libraries
//! like Tokio and provides a stateful, byte-oriented interface for handling Telnet communication.
//!
//! ## Overview
//!
//! The Telnet protocol (RFC 854) is a service protocol used for interactive text-oriented
//! communication over TCP. This codec handles:
//!
//! # Telnet Protocol Codec
//!
//! This module provides a complete implementation of the Telnet protocol codec for encoding and
//! decoding Telnet protocol messages. It is designed to work with asynchronous networking libraries
//! like Tokio and provides a stateful, byte-oriented interface for handling Telnet communication.
//!
//! ## Overview
//!
//! The Telnet protocol (RFC 854) is a service protocol used for interactive text-oriented
//! communication over TCP. This codec handles:
//!
//! - **Data transmission**: Raw byte data with proper IAC (Interpret As Command) escaping
//! - **Control commands**: Break, Interrupt Process, Abort Output, etc.
//! - **Option negotiation**: DO, DONT, WILL, WONT commands for enabling protocol features
//! - **Subnegotiation**: Extended option negotiation with parameters
//!
//! ## Core Components
//!
//! ### [`TelnetCodec`]
//!
//! The main codec structure that implements both [`Encoder`] and [`Decoder`] traits from
//! `tokio_util::codec`. It maintains internal state for parsing Telnet protocol sequences
//! and manages option negotiation state.
//!
//! ### [`TelnetFrame`]
//!
//! An enumeration representing all possible Telnet protocol frames:
//! - Data bytes
//! - Control commands (NoOperation, DataMark, Break, etc.)
//! - Negotiation commands (Do, Dont, Will, Wont)
//! - Subnegotiation sequences
//!
//! ### [`TelnetOption`]
//!
//! Represents Telnet protocol options that can be negotiated between client and server,
//! including standard options like Echo, Binary Transmission, and various MUD-specific
//! extensions (GMCP, MSDP, etc.).
//!
//! ### [`TelnetEvent`]
//!
//! Higher-level events generated from processing frames, providing a more semantic
//! interface for application code.
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use termionix_codec::{TelnetCodec, TelnetFrame, TelnetOption};
//! use tokio_util::codec::{Decoder, Encoder, Framed};
//! use tokio::net::TcpStream;
//! use bytes::BytesMut;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a new codec
//! let mut codec = TelnetCodec::new();
//!
//! // Encoding data
//! let mut buffer = BytesMut::new();
//! codec.encode(TelnetFrame::Data(b'H'), &mut buffer)?;
//! codec.encode(TelnetFrame::Data(b'i'), &mut buffer)?;
//!
//! // Send a negotiation command
//! codec.encode(TelnetFrame::Will(TelnetOption::Echo), &mut buffer)?;
//!
//! // Decoding data
//! let mut input = BytesMut::from(&b"Hello\xFF\xFD\x01"[..]); // Data + DO Echo
//! while let Some(frame) = codec.decode(&mut input)? {
//!     match frame {
//!         TelnetFrame::Data(byte) => println!("Received: {}", byte as char),
//!         TelnetFrame::Do(option) => println!("Server requests: DO {:?}", option),
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Protocol Details
//!
//! ### Command Structure
//!
//! All Telnet commands start with the IAC (Interpret As Command) byte (0xFF). The basic
//! command structure is:
//!
//! - 2-byte commands: `IAC <command>` (e.g., `IAC NOP`)
//! - 3-byte negotiation: `IAC <DO|DONT|WILL|WONT> <option>`
//! - Subnegotiation: `IAC SB <option> <data...> IAC SE`
//!
//! ### IAC Escaping
//!
//! Since 0xFF (IAC) is a special byte, it must be escaped when transmitted as data by
//! sending it twice: `IAC IAC` represents a literal 0xFF byte in the data stream.
//!
//! ### Option Negotiation
//!
//! The codec automatically tracks option negotiation state through [`TelnetOptions`]:
//!
//! - **DO**: Request the other party enable an option
//! - **DONT**: Request the other party disable an option
//! - **WILL**: Offer to enable an option locally
//! - **WONT**: Refuse or disable an option locally
//!
//! Check option status using:
//! ```rust
//! use termionix_codec::{TelnetCodec, TelnetOption};
//!
//! let codec = TelnetCodec::new();
//! if codec.is_enabled_local(TelnetOption::Echo) {
//!     // Echo is enabled on our side
//! }
//! if codec.is_enabled_remote(TelnetOption::SuppressGoAhead) {
//!     // Remote side has SGA enabled
//! }
//! ```
//!
//! ## Error Handling
//!
//! The codec uses [`CodecError`] for encoding and decoding errors. In practice, the current
//! implementation is resilient and handles malformed input by returning `NoOperation` frames
//! or skipping invalid sequences.
//!
//! ## Performance Considerations
//!
//! - The codec maintains internal buffers for partial frame assembly
//! - Subnegotiation data is buffered until the complete sequence is received
//! - State machine transitions occur byte-by-byte for accurate protocol parsing
//! - The codec reserves appropriate buffer space before encoding to minimize allocations
//!
//! ## Thread Safety
//!
//! `TelnetCodec` is **not** thread-safe and should not be shared between threads without
//! appropriate synchronization. Typically, each connection has its own codec instance.
//!
//! ## Testing
//!
//! The module includes comprehensive tests covering:
//! - Encoding and decoding of all frame types
//! - Round-trip encode/decode verification
//! - Partial frame handling (streaming scenarios)
//! - IAC escaping and edge cases
//! - RFC 854 compliance scenarios
//!
//! ## Related RFCs
//!
//! - RFC 854: Telnet Protocol Specification
//! - RFC 855: Telnet Option Specifications
//! - RFC 856: Telnet Binary Transmission
//! - RFC 857: Telnet Echo Option
//! - RFC 858: Telnet Suppress Go Ahead Option
//!
//! ## MUD Protocol Extensions
//!
//! This implementation includes support for several MUD (Multi-User Dungeon) specific
//! protocol extensions commonly used in text-based games:
//!
//! - **GMCP** (Generic MUD Communication Protocol): Structured data exchange
//! - **MSDP** (MUD Server Data Protocol): Server state reporting
//! - **MSSP** (MUD Server Status Protocol): Server metadata
//! - **MCCP** (MUD Client Compression Protocol): Stream compression
//! - And others (see [`TelnetOption`] for full list)

#![warn(
    clippy::cargo,
    missing_docs,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![allow(
    clippy::option_if_let_else,
    clippy::module_name_repetitions,
    clippy::missing_errors_doc
)]
// Using stable range APIs
// TODO: Remove These before 1.0
#![allow(unused_imports, unused_variables, dead_code, unused_mut)]

mod args;
mod codec;
mod consts;
mod event;
mod frame;
mod options;
mod result;
mod state;

pub use self::args::{TelnetArgument, linemode, msdp, mssp, naocrd, naohts, naws, status};
pub use self::codec::TelnetCodec;
pub use self::event::TelnetEvent;
pub use self::frame::TelnetFrame;
pub use self::options::TelnetOption;
pub use self::result::{CodecError, CodecResult};
pub use self::state::TelnetOptions;

#[cfg(test)]
mod tests {
    use super::{TelnetCodec, TelnetFrame, TelnetOption, consts};
    use bytes::BytesMut;
    use tokio_util::codec::{Decoder, Encoder};

    #[tokio::test]
    async fn telnet_decode() {
        let mut codec = TelnetCodec::new();
        let mut input_buffer = BytesMut::from("Terminated line\r\n");
        let expected_output = vec![
            TelnetFrame::Data(b'T'),
            TelnetFrame::Data(b'e'),
            TelnetFrame::Data(b'r'),
            TelnetFrame::Data(b'm'),
            TelnetFrame::Data(b'i'),
            TelnetFrame::Data(b'n'),
            TelnetFrame::Data(b'a'),
            TelnetFrame::Data(b't'),
            TelnetFrame::Data(b'e'),
            TelnetFrame::Data(b'd'),
            TelnetFrame::Data(b' '),
            TelnetFrame::Data(b'l'),
            TelnetFrame::Data(b'i'),
            TelnetFrame::Data(b'n'),
            TelnetFrame::Data(b'e'),
            TelnetFrame::Data(b'\r'),
            TelnetFrame::Data(b'\n'),
        ];
        let mut actual_output = Vec::new();
        while let Some(frame) = codec.decode(&mut input_buffer).unwrap() {
            actual_output.push(frame)
        }
        assert_eq!(expected_output, actual_output, "telnet_decode didn't match");
    }

    #[test]
    fn telnet_encode() {
        let mut codec = TelnetCodec::new();
        let input_frames = vec![
            TelnetFrame::Data(b'R'),
            TelnetFrame::Data(b'a'),
            TelnetFrame::Data(b'w'),
            TelnetFrame::Data(b' '),
            TelnetFrame::Data(b'A'),
            TelnetFrame::Data(b's'),
            TelnetFrame::Data(b'c'),
            TelnetFrame::Data(b'i'),
            TelnetFrame::Data(b'i'),
            TelnetFrame::Data(b' '),
            TelnetFrame::Data(b'D'),
            TelnetFrame::Data(b'a'),
            TelnetFrame::Data(b't'),
            TelnetFrame::Data(b'a'),
            TelnetFrame::Data(b'\r'),
            TelnetFrame::Data(b'\n'),
        ];
        let expected_output = BytesMut::from(&b"Raw Ascii Data\r\n"[..]);
        let mut actual_output = BytesMut::with_capacity(20);
        for frame in input_frames {
            codec.encode(frame, &mut actual_output).unwrap();
        }
        assert_eq!(expected_output, actual_output, "telnet_encode didn't match");
    }

    #[test]
    fn decode_iac_activation() {
        let mut codec = TelnetCodec::new();
        let mut input_buffer = BytesMut::from(
            &[
                // Data
                b'L',
                b'o',
                b'g',
                b'i',
                b'n',
                b':',
                consts::CR,
                consts::LF,
                // Command Do Binary
                consts::IAC,
                consts::DO,
                consts::option::BINARY,
                // Data
                b'P',
                b'a',
                b's',
                b's',
                b'w',
                b'o',
                b'r',
                b'd',
                b':',
                consts::CR,
                consts::LF,
                // Command Will Binary
                consts::IAC,
                consts::WILL,
                consts::option::BINARY,
                // Data
                b'H',
                b'e',
                b'l',
                b'l',
                b'o',
                b'!',
                consts::CR,
                consts::LF,
            ][..],
        );
        let expected_output = vec![
            // Normal Data
            TelnetFrame::Data(b'L'),
            TelnetFrame::Data(b'o'),
            TelnetFrame::Data(b'g'),
            TelnetFrame::Data(b'i'),
            TelnetFrame::Data(b'n'),
            TelnetFrame::Data(b':'),
            TelnetFrame::Data(consts::CR),
            TelnetFrame::Data(consts::LF),
            // Command Do Binary
            TelnetFrame::Do(TelnetOption::TransmitBinary),
            // Data
            TelnetFrame::Data(b'P'),
            TelnetFrame::Data(b'a'),
            TelnetFrame::Data(b's'),
            TelnetFrame::Data(b's'),
            TelnetFrame::Data(b'w'),
            TelnetFrame::Data(b'o'),
            TelnetFrame::Data(b'r'),
            TelnetFrame::Data(b'd'),
            TelnetFrame::Data(b':'),
            TelnetFrame::Data(consts::CR),
            TelnetFrame::Data(consts::LF),
            // Command Will Binary
            TelnetFrame::Will(TelnetOption::TransmitBinary),
            // Data
            TelnetFrame::Data(b'H'),
            TelnetFrame::Data(b'e'),
            TelnetFrame::Data(b'l'),
            TelnetFrame::Data(b'l'),
            TelnetFrame::Data(b'o'),
            TelnetFrame::Data(b'!'),
            TelnetFrame::Data(consts::CR),
            TelnetFrame::Data(consts::LF),
        ];
        let mut actual_output = Vec::new();
        while let Some(frame) = codec.decode(&mut input_buffer).unwrap() {
            actual_output.push(frame)
        }

        assert_eq!(expected_output, actual_output);
    }
}
