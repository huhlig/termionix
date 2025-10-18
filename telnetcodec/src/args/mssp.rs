//
// Copyright 2017-2025 Hans W. Uhlig. All Rights Reserved.
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

//! MudServerStatus Option
//!
//! https://tools.ietf.org/html/rfc8549#section-3.1.2
//!
//! MSSP is a subnegotiation of the MSSP option.
//!
//! The MSSP subnegotiation is used to send information about the Mud to the
//! client. The information is sent in a series of key-value pairs.
//!
//!
use crate::{CodecResult, consts};
use byteorder::WriteBytesExt;
use bytes::BufMut;
use std::collections::HashMap;

/// Mud Server Status Protocol handler for TELNET negotiation.
///
/// This struct manages MUD server information transmission according to the
/// [MSSP Protocol](https://tintin.sourceforge.io/protocols/mssp/).
///
/// MSSP is a subnegotiation of the TELNET MSSP option that allows servers to send
/// structured information about the MUD to clients in key-value pairs. Each key can
/// have multiple associated values.
///
/// # Examples
///
/// ```
/// use termionix_telnetcodec::mssp::MudServerStatus;
///
/// let mut status = MudServerStatus::new();
/// // Add server information...
/// ```
#[derive(Clone, Debug)]
pub struct MudServerStatus(HashMap<String, Vec<String>>);

impl MudServerStatus {
    /// Creates a new, empty `MudServerStatus` instance.
    ///
    /// Initializes an empty MSSP status with no key-value pairs.
    ///
    /// # Returns
    ///
    /// A new `MudServerStatus` with no entries. The internal HashMap is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use telnetcodec::mssp::MudServerStatus;
    ///
    /// let status = MudServerStatus::new();
    /// assert_eq!(status.len(), 0);
    /// ```
    pub fn new() -> MudServerStatus {
        MudServerStatus(HashMap::new())
    }

    /// Returns the encoded length of the `MudServerStatus` in bytes.
    ///
    /// Calculates the total number of bytes that will be needed to encode this
    /// `MudServerStatus` according to the MSSP protocol. This accounts for all
    /// keys, values, and their separator markers but does not include the effect
    /// of character filtering that occurs during actual encoding.
    ///
    /// # Returns
    ///
    /// The total encoded length in bytes. This is the sum of:
    /// - 1 byte for each key's VAR marker
    /// - The length of each key string
    /// - 1 byte for each value's VAL marker
    /// - The length of each value string
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_telnetcodec::mssp::MudServerStatus;
    ///
    /// let status = MudServerStatus::new();
    /// assert_eq!(status.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        let mut length = 0;
        for (key, values) in &self.0 {
            length += 1;
            length += key.len();
            for value in values {
                length += 1;
                length += value.len();
            }
        }
        length
    }

    /// Encodes `MudServerStatus` to a `BufMut` buffer.
    ///
    /// Serializes the MSSP status information into the provided mutable buffer.
    /// Invalid characters (NUL, IAC, VAR, VAL) are filtered out from keys and values
    /// during encoding to ensure protocol compliance.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer that implements `BufMut` where the encoded data
    ///          will be written
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written to the buffer
    /// * `Err(CodecResult)` - If an I/O error occurs during encoding
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_telnetcodec::mssp::MudServerStatus;
    /// use bytes::BytesMut;
    ///
    /// let status = MudServerStatus::new();
    /// let mut buffer = BytesMut::new();
    /// match status.encode(&mut buffer) {
    ///     Ok(bytes_written) => println!("Encoded {} bytes", bytes_written),
    ///     Err(e) => eprintln!("Encoding error: {:?}", e),
    /// }
    /// ```
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> CodecResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Writes the encoded `MudServerStatus` to a byte writer.
    ///
    /// Low-level method that serializes the MSSP status to an implementer of
    /// `std::io::Write`. Keys and values are filtered to remove any characters
    /// that are invalid in the MSSP protocol (NUL, IAC, VAR, VAL).
    ///
    /// The encoding format uses:
    /// - VAR marker (0x01) to prefix each key
    /// - VAL marker (0x02) to prefix each value
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable reference to a type implementing `std::io::Write`
    ///             where the encoded bytes will be written
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The total number of bytes written
    /// * `Err(std::io::Error)` - If a write error occurs
    ///
    /// # Character Filtering
    ///
    /// The following characters are removed from keys and values:
    /// - NUL (0x00) - Null terminator
    /// - IAC (0xFF) - TELNET Interpret As Command
    /// - VAR (0x01) - MSSP variable marker
    /// - VAL (0x02) - MSSP value marker
    ///
    /// # Examples
    ///
    /// ```
    /// use telnetcodec::mssp::MudServerStatus;
    /// use std::io::Cursor;
    ///
    /// let status = MudServerStatus::new();
    /// let mut buffer = Vec::new();
    /// match status.write(&mut buffer) {
    ///     Ok(bytes_written) => println!("Wrote {} bytes", bytes_written),
    ///     Err(e) => eprintln!("Write error: {}", e),
    /// }
    /// ```
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut len = 0;
        for (key, values) in &self.0 {
            writer.write_u8(consts::option::mssp::VAR)?;
            len += 1 + writer.write(
                key.chars()
                    .filter(|ch| {
                        *ch != consts::NUL as char
                            && *ch != consts::IAC as char
                            && *ch != consts::option::mssp::VAR as char
                            && *ch != consts::option::mssp::VAL as char
                    })
                    .collect::<String>()
                    .as_bytes(),
            )?;
            for value in values {
                writer.write_u8(consts::option::mssp::VAL)?;
                len += 1 + writer.write(
                    value
                        .chars()
                        .filter(|ch| {
                            *ch != consts::NUL as char
                                && *ch != consts::IAC as char
                                && *ch != consts::option::mssp::VAR as char
                                && *ch != consts::option::mssp::VAL as char
                        })
                        .collect::<String>()
                        .as_bytes(),
                )?;
            }
        }
        Ok(len)
    }
}
