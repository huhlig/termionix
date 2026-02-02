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

//! Negotiate About Window Size
//!

use crate::{TelnetCodecError, TelnetCodecResult};
use byteorder::{BigEndian, WriteBytesExt};
use bytes::{Buf, BufMut};

/// Represents the Negotiate About Window Size (NAWS) option data.
///
/// This struct encodes the window dimensions (width and height) used in Telnet
/// negotiation. The NAWS option allows a Telnet client and server to communicate
/// the terminal window size, typically used to adjust text wrapping and display.
///
/// # Format
/// The window size is encoded as four bytes in big-endian format:
/// - 2 bytes for columns (width)
/// - 2 bytes for rows (height)
///
/// # Example
/// ```
/// use termionix_telnetcodec::naws::WindowSize;
///
/// let size = WindowSize::new(80, 24);
/// assert_eq!(size.cols, 80);
/// assert_eq!(size.rows, 24);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowSize {
    /// The number of columns (characters) in the terminal window
    pub cols: u16,
    /// The number of rows (lines) in the terminal window
    pub rows: u16,
}

impl WindowSize {
    /// Creates a new `WindowSize` with the specified columns and rows.
    ///
    /// # Arguments
    /// * `cols` - The number of columns (width) in the terminal window
    /// * `rows` - The number of rows (height) in the terminal window
    ///
    /// # Returns
    /// A new `WindowSize` instance with the given dimensions.
    ///
    /// # Example
    /// ```
    /// let size = WindowSize::new(100, 30);
    /// ```
    pub fn new(cols: u16, rows: u16) -> Self {
        WindowSize { cols, rows }
    }

    /// Returns the encoded length of this `WindowSize` in bytes.
    ///
    /// The NAWS subnegotiation data always occupies exactly 4 bytes:
    /// 2 bytes for columns and 2 bytes for rows.
    ///
    /// # Returns
    /// Always returns `4`.
    pub fn len(&self) -> usize {
        4
    }

    /// Encodes this `WindowSize` into a byte buffer using big-endian format.
    ///
    /// The window size is encoded as four bytes: columns (2 bytes) followed by
    /// rows (2 bytes), both in big-endian byte order.
    ///
    /// # Arguments
    /// * `dst` - A mutable buffer that implements `BufMut` to receive the encoded bytes
    ///
    /// # Returns
    /// `Ok(4)` on successful encoding, or a `CodecError` if the encoding fails.
    ///
    /// # Example
    /// ```
    /// use bytes::BytesMut;
    /// let size = WindowSize::new(80, 24);
    /// let mut buf = BytesMut::new();
    /// size.encode(&mut buf)?;
    /// ```
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> TelnetCodecResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Writes this `WindowSize` to a writer using big-endian format.
    ///
    /// This is the underlying implementation for serialization. It writes
    /// the columns followed by the rows as big-endian u16 values.
    ///
    /// # Arguments
    /// * `writer` - A mutable writer implementing `std::io::Write`
    ///
    /// # Returns
    /// `Ok(4)` if the write succeeds, or an `std::io::Error` if writing fails.
    ///
    /// # Example
    /// ```
    /// use std::io::Cursor;
    /// let size = WindowSize::new(80, 24);
    /// let mut cursor = Cursor::new(Vec::new());
    /// size.write(&mut cursor)?;
    /// ```
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        writer.write_u16::<BigEndian>(self.cols)?;
        writer.write_u16::<BigEndian>(self.rows)?;
        Ok(4)
    }

    /// Decodes a `WindowSize` from a byte buffer in big-endian format.
    ///
    /// This method reads 4 bytes from the provided buffer: 2 bytes for columns
    /// and 2 bytes for rows, interpreting them as big-endian unsigned integers.
    ///
    /// # Arguments
    /// * `src` - A buffer implementing `Buf` containing the encoded window size data
    ///
    /// # Returns
    /// `Ok(WindowSize)` containing the decoded dimensions, or a `CodecError` if:
    /// - The buffer contains fewer than 4 bytes (`InsufficientData`)
    /// - The decoding process fails
    ///
    /// # Errors
    /// Returns `CodecError::SubnegotiationError` with `InsufficientData` if
    /// fewer than 4 bytes are available in the buffer.
    ///
    /// # Example
    /// ```
    /// use bytes::BytesMut;
    /// use termionix_telnetcodec::naws::WindowSize;
    ///
    /// let mut buf = BytesMut::from(&[0x00, 0x50, 0x00, 0x18][..]);
    /// let size = WindowSize::decode(&mut buf)?;
    /// assert_eq!(size.cols, 80);
    /// assert_eq!(size.rows, 24);
    /// ```
    pub fn decode<T: Buf>(src: &mut T) -> TelnetCodecResult<WindowSize> {
        // NAWS format: WIDTH-HIGH WIDTH-LOW HEIGHT-HIGH HEIGHT-LOW
        if src.remaining() >= 4 {
            Ok(WindowSize {
                cols: src.get_u16(),
                rows: src.get_u16(),
            })
        } else {
            Err(TelnetCodecError::SubnegotiationError {
                option: Some(crate::consts::option::NAWS),
                reason: crate::SubnegotiationErrorKind::InsufficientData {
                    required: 4,
                    available: src.remaining(),
                },
            })
        }
    }
}

impl Default for WindowSize {
    /// Returns a default `WindowSize` representing a standard 80x24 terminal.
    ///
    /// This is the traditional terminal dimensions, commonly used as a fallback
    /// when window size negotiation is unavailable or incomplete.
    ///
    /// # Returns
    /// A `WindowSize` with 80 columns and 24 rows.
    ///
    /// # Example
    /// ```
    /// let size = WindowSize::default();
    /// assert_eq!(size.cols, 80);
    /// assert_eq!(size.rows, 24);
    /// ```
    fn default() -> Self {
        WindowSize { cols: 80, rows: 24 }
    }
}

impl std::fmt::Display for WindowSize {
    /// Formats the `WindowSize` as a human-readable string.
    ///
    /// The output format is `(cols,rows)`, for example: `(80,24)`.
    ///
    /// # Example
    /// ```
    /// let size = WindowSize::new(100, 30);
    /// println!("{}", size);  // Outputs: (100,30)
    /// ```
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{})", self.cols, self.rows)
    }
}
