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

use crate::TelnetOption;
use crate::args::gmcp::GmcpMessage;
use crate::args::naws::WindowSize;
use crate::result::CodecResult;
use bytes::{BufMut, BytesMut};
use std::fmt::Formatter;

/// GMCP (Generic Mud Communication Protocol) argument parsing and handling
pub mod gmcp;
pub mod linemode;
pub mod msdp;
pub mod mssp;
pub mod naocrd;
pub mod naohts;
pub mod naws;
pub mod status;

///
/// Telnet Subnegotiation Argument
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TelnetArgument {
    /// A subnegotiation for the window size, where the first value is the width
    /// and the second value is the height. The values are in characters.
    NAWSWindowSize(WindowSize),
    /// Indicates an intent to begin CHARSET subnegotiation. This can only be
    /// sent after receiving a DO CHARSET after sending a WILL CHARSET (in any
    /// order).
    CharsetRequest(Vec<BytesMut>),
    /// Indicates that the receiver has accepted the charset request.
    CharsetAccepted(BytesMut),
    /// Indicates that the receiver acknowledges the charset request but will
    /// not use any of the requested characters.
    CharsetRejected,
    /// Indicates that the receiver acknowledges a TTABLE-IS message but is
    /// unable to handle it. This will terminate subnegotiation.
    CharsetTTableRejected,
    /// GMCP (Generic Mud Communication Protocol) message.
    /// Contains a package name and optional JSON data payload.
    GMCP(GmcpMessage),
    /// A subnegotiation for an unknown option.
    Unknown(TelnetOption, BytesMut),
}

impl TelnetArgument {
    /// Returns the encoded byte length of this `TelnetArgument`.
    ///
    /// Calculates the total number of bytes that will be produced when this argument
    /// is encoded to its wire format representation. This is useful for pre-allocating
    /// buffers before encoding.
    ///
    /// # Returns
    ///
    /// The number of bytes in the encoded form.
    ///
    /// # Behavior by Variant
    ///
    /// - `NAWSWindowSize(inner)` - Returns the encoded length of the window size data
    /// - `Unknown(option, payload)` - Returns the length of the payload bytes
    /// - Other variants - Currently unimplemented
    ///
    /// # Performance
    ///
    /// This is an O(1) operation for most variants, though some may require
    /// data length inspection.
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_telnetcodec::TelnetArgument;
    /// use termionix_telnetcodec::naws::WindowSize;
    ///
    /// let arg = TelnetArgument::NAWSWindowSize(WindowSize::new(80, 24));
    /// let size = arg.len();
    /// ```
    pub fn len(&self) -> usize {
        match self {
            TelnetArgument::NAWSWindowSize(inner) => inner.len(),
            TelnetArgument::GMCP(inner) => inner.len(),
            TelnetArgument::Unknown(_option, inner) => inner.len(),
            _ => unimplemented!(),
        }
    }
    /// Encodes this `TelnetArgument` to a `BufMut` buffer.
    ///
    /// Writes the argument's byte representation to a mutable buffer, automatically
    /// advancing the buffer's write position. This is the preferred method for encoding
    /// into buffers that implement `BufMut`.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer implementing `BufMut` (e.g., `BytesMut`).
    ///   The buffer is advanced by the bytes written.
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success, containing the exact number of bytes
    /// that were written to the buffer. Returns a `CodecResult` error if encoding fails.
    ///
    /// # Performance
    ///
    /// This is an O(n) operation where n is the length of the encoded argument.
    /// It performs a single pass through the data, writing directly to the buffer
    /// without intermediate allocations.
    ///
    /// # Buffer Behavior
    ///
    /// - The buffer is only written to; existing content is preserved
    /// - The buffer's write position is advanced automatically
    /// - No bounds checking is performed; the buffer must have sufficient capacity
    /// - For `BytesMut`, automatic growth occurs if needed
    ///
    /// # Examples
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use termionix_telnetcodec::naws::WindowSize;
    /// use termionix_telnetcodec::TelnetArgument;
    ///
    /// let arg = TelnetArgument::NAWSWindowSize(WindowSize::new(80, 24));
    /// let mut buffer = BytesMut::new();
    /// let written = arg.encode(&mut buffer)?;
    /// println!("Wrote {} bytes", written);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> CodecResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Writes this `TelnetArgument` to a `std::io::Write` writer.
    ///
    /// Performs the actual encoding of the argument into its byte representation
    /// and writes the bytes to the provided writer. This is the low-level method
    /// that `encode()` delegates to internally.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write` (e.g., `Vec<u8>`,
    ///   `File`, socket, etc.). The writer's internal position is advanced by the
    ///   bytes written.
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success, containing the exact number of bytes written.
    /// Returns `std::io::Error` if the writer fails (e.g., disk full, broken pipe).
    ///
    /// # Encoding Behavior by Variant
    ///
    /// - `NAWSWindowSize(inner)` - Delegates to the window size's `write()` method
    /// - `Unknown(option, payload)` - Writes the raw payload bytes as-is
    /// - Other variants - Will panic (currently unimplemented)
    ///
    /// # Examples
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use termionix_telnetcodec::{TelnetArgument, TelnetOption};
    ///
    /// let arg = TelnetArgument::Unknown(
    ///     TelnetOption::NAWS,
    ///     BytesMut::from(&b"\x00\x50\x00\x18"[..])
    /// );
    /// let mut output = Vec::new();
    /// let written = arg.write(&mut output)?;
    /// assert_eq!(written, 4);
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        match self {
            TelnetArgument::NAWSWindowSize(inner) => inner.write(writer),
            TelnetArgument::GMCP(inner) => inner.write(writer),
            TelnetArgument::Unknown(_option, payload) => {
                // Write payload with IAC escaping
                let mut written = 0;
                for &byte in payload.iter() {
                    if byte == 0xFF {
                        // IAC byte must be escaped as IAC IAC
                        writer.write_all(&[0xFF, 0xFF])?;
                        written += 2;
                    } else {
                        writer.write_all(&[byte])?;
                        written += 1;
                    }
                }
                Ok(written)
            }
            _ => unimplemented!(),
        }
    }

    /// Returns the `TelnetOption` associated with this argument.
    ///
    /// Identifies which Telnet option this argument is for, enabling proper routing
    /// and handling of subnegotiation data during protocol processing.
    ///
    /// # Returns
    ///
    /// A `TelnetOption` enum value corresponding to the argument's option type.
    ///
    /// # Behavior by Variant
    ///
    /// - `NAWSWindowSize(_)` → Returns `TelnetOption::NAWS`
    /// - `CharsetRequest(_)` → Returns `TelnetOption::Charset`
    /// - `CharsetAccepted(_)` → Returns `TelnetOption::Charset`
    /// - `CharsetRejected` → Returns `TelnetOption::Charset`
    /// - `CharsetTTableRejected` → Returns `TelnetOption::Charset`
    /// - `Unknown(option, _)` → Returns the contained `option`
    ///
    /// # Performance
    ///
    /// This is an O(1) operation that simply matches and returns a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_telnetcodec::{TelnetArgument, TelnetOption};
    /// use termionix_telnetcodec::naws::WindowSize;
    ///
    /// let arg = TelnetArgument::NAWSWindowSize(WindowSize::new(80, 24));
    /// assert_eq!(arg.option(), TelnetOption::NAWS);
    ///
    /// let arg = TelnetArgument::CharsetRequest(vec![]);
    /// assert_eq!(arg.option(), TelnetOption::Charset);
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Option routing**: Direct subnegotiation data to the correct handler
    /// - **Protocol validation**: Verify option consistency in negotiation sequences
    /// - **Option tracking**: Maintain state of which options are active
    pub fn option(&self) -> TelnetOption {
        match self {
            TelnetArgument::NAWSWindowSize(_) => TelnetOption::NAWS,
            TelnetArgument::CharsetRequest(_) => TelnetOption::Charset,
            TelnetArgument::CharsetAccepted(_) => TelnetOption::Charset,
            TelnetArgument::CharsetRejected => TelnetOption::Charset,
            TelnetArgument::CharsetTTableRejected => TelnetOption::Charset,
            TelnetArgument::GMCP(_) => TelnetOption::GMCP,
            TelnetArgument::Unknown(option, _) => TelnetOption::Unknown(option.to_u8()),
        }
    }
}

impl std::fmt::Display for TelnetArgument {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TelnetArgument::NAWSWindowSize(v) => write!(f, "{v}"),
            TelnetArgument::CharsetRequest(v) => write!(f, "CharsetRequest({v:?})"),
            TelnetArgument::CharsetAccepted(v) => write!(f, "CharsetAccepted({v:?})"),
            TelnetArgument::CharsetRejected => write!(f, "CharsetRejected"),
            TelnetArgument::CharsetTTableRejected => write!(f, "CharsetTableRejected"),
            TelnetArgument::GMCP(v) => write!(f, "GMCP({})", v),
            TelnetArgument::Unknown(o, v) => write!(f, "{o}-{v:?}"),
        }
    }
}
