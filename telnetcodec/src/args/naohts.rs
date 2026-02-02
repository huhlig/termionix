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

//! NAOHTS Option
//!
//! https://tools.ietf.org/html/rfc653
//!
//! NAOHTS is a subnegotiation of the NAOHTS option.
//!
//! The NAOHTS subnegotiation is used to negotiate about the horizontal tabstops.
//! The data is sent in a series of bytes.
//!
//! The first byte is the number of tabstops. The remaining bytes are the tabstops.

use crate::TelnetCodecResult;
use byteorder::WriteBytesExt;
use bytes::{Buf, BufMut};

/// Negotiation data for Output Horizontal Tab Stops.
///
/// The `NAOHTS` option negotiates the horizontal tab stop positions between telnet client
/// and server, as defined in [RFC 653](https://tools.ietf.org/html/rfc653).
///
/// Tab stops are represented as column positions (0-255) where text should stop when a
/// horizontal tab character is encountered. This is particularly useful for terminal
/// applications that need to align text at specific column boundaries.
///
/// # Structure
///
/// The NAOHTS subnegotiation consists of a series of bytes, where each byte represents
/// a tab stop column position. Multiple tab stops can be specified in a single negotiation.
///
/// # Examples
///
/// Create a new NAOHTS with specific tab stops:
///
/// ```rust
/// # use termionix_telnetcodec::naohts::NAOHTS;
/// let naohts = NAOHTS::new(vec![8, 16, 24, 32]);
/// assert_eq!(naohts.tab_stops, vec![8, 16, 24, 32]);
/// ```
///
/// Create NAOHTS with default tab stops (every 8 columns):
///
/// ```rust
/// # use termionix_telnetcodec::naohts::NAOHTS;
/// let naohts = NAOHTS::default_tabs(80);
/// // Creates tab stops at: [8, 16, 24, 32, 40, 48, 56, 64, 72]
/// ```
///
/// # Encoding and Decoding
///
/// NAOHTS values can be encoded to bytes for transmission:
///
/// ```rust
/// # use termionix_telnetcodec::args::NAOHTS;
/// # use bytes::BytesMut;
/// let naohts = NAOHTS::new(vec![8, 16, 24, 32]);
/// let mut buffer = BytesMut::with_capacity(10);
/// let bytes_written = naohts.encode(&mut buffer).unwrap();
/// assert_eq!(buffer.as_ref(), &[8, 16, 24, 32]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NAOHTS {
    /// List of horizontal tab stop positions (column numbers from 0-255)
    pub tab_stops: Vec<u8>,
}

impl NAOHTS {
    /// Creates a new `NAOHTS` with the specified tab stops.
    ///
    /// # Arguments
    ///
    /// * `tab_stops` - A vector of byte values representing tab stop column positions.
    ///   Each value should be in the range 0-255, representing column positions.
    ///
    /// # Returns
    ///
    /// A new `NAOHTS` instance containing the specified tab stops.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use telnetcodec::args::NAOHTS;
    /// let naohts = NAOHTS::new(vec![8, 16, 24, 32]);
    /// ```
    pub fn new(tab_stops: Vec<u8>) -> Self {
        Self { tab_stops }
    }

    /// Creates a `NAOHTS` with default tab stops spaced every 8 columns.
    ///
    /// This is a common convention for terminal applications. Tab stops are generated
    /// at positions 8, 16, 24, 32, etc., up to but not exceeding the specified width.
    ///
    /// # Arguments
    ///
    /// * `width` - The maximum column width to consider. Tab stops will be generated
    ///   at positions 8, 16, 24, ... that are less than this width.
    ///
    /// # Returns
    ///
    /// A new `NAOHTS` instance with default tab stops.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use telnetcodec::args::NAOHTS;
    /// let naohts = NAOHTS::default_tabs(80);
    /// assert_eq!(naohts.tab_stops, vec![8, 16, 24, 32, 40, 48, 56, 64, 72]);
    /// ```
    pub fn default_tabs(width: u8) -> Self {
        let mut tab_stops = Vec::new();
        let mut pos = 8u8;
        while pos < width {
            tab_stops.push(pos);
            pos = pos.saturating_add(8);
        }
        Self { tab_stops }
    }

    /// Checks if there are no tab stops configured.
    ///
    /// # Returns
    ///
    /// `true` if the tab stops list is empty, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use telnetcodec::args::NAOHTS;
    /// let empty = NAOHTS::new(vec![]);
    /// assert!(empty.is_empty());
    ///
    /// let with_stops = NAOHTS::new(vec![8]);
    /// assert!(!with_stops.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.tab_stops.is_empty()
    }

    /// Returns the number of tab stops in this negotiation.
    ///
    /// # Returns
    ///
    /// The count of tab stop positions currently configured.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use telnetcodec::args::NAOHTS;
    /// let naohts = NAOHTS::new(vec![8, 16, 24, 32]);
    /// assert_eq!(naohts.len(), 4);
    /// ```
    pub fn len(&self) -> usize {
        self.tab_stops.len()
    }

    /// Encodes the `NAOHTS` data into a `BufMut` buffer.
    ///
    /// Each tab stop is written as a single byte in the order they appear in the
    /// `tab_stops` vector.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer implementing `BufMut` where the encoded data will be written.
    ///
    /// # Returns
    ///
    /// A `CodecResult` containing the number of bytes written on success, or an error on failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use telnetcodec::args::NAOHTS;
    /// # use bytes::BytesMut;
    /// let naohts = NAOHTS::new(vec![8, 16, 24, 32]);
    /// let mut buffer = BytesMut::with_capacity(10);
    /// let bytes_written = naohts.encode(&mut buffer).unwrap();
    /// assert_eq!(bytes_written, 4);
    /// assert_eq!(buffer.as_ref(), &[8, 16, 24, 32]);
    /// ```
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> TelnetCodecResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Writes the `NAOHTS` data to an `std::io::Write` implementation.
    ///
    /// This is the underlying implementation used by `encode()`. Each tab stop position
    /// is written as a single byte.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write`.
    ///
    /// # Returns
    ///
    /// An `std::io::Result` containing the number of bytes written on success, or an
    /// I/O error on failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use telnetcodec::args::NAOHTS;
    /// # use std::io::Cursor;
    /// let naohts = NAOHTS::new(vec![8, 16, 24]);
    /// let mut output = Vec::new();
    /// let bytes_written = naohts.write(&mut output).unwrap();
    /// assert_eq!(bytes_written, 3);
    /// assert_eq!(output, vec![8, 16, 24]);
    /// ```
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut len = 0;

        // Write each tab stop position
        for &tab_stop in &self.tab_stops {
            writer.write_u8(tab_stop)?;
            len += 1;
        }

        Ok(len)
    }

    /// Decodes `NAOHTS` data from a `Buf` buffer.
    ///
    /// Reads all remaining bytes in the buffer, treating each byte as a tab stop position.
    ///
    /// # Arguments
    ///
    /// * `src` - A mutable buffer implementing `Buf` from which to read encoded data.
    ///
    /// # Returns
    ///
    /// A `CodecResult` containing the decoded `NAOHTS` instance on success, or an error on failure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use telnetcodec::args::NAOHTS;
    /// # use bytes::BytesMut;
    /// let data = vec![8, 16, 24, 32];
    /// let mut buffer = BytesMut::from(&data[..]);
    /// let naohts = NAOHTS::decode(&mut buffer).unwrap();
    /// assert_eq!(naohts.tab_stops, vec![8, 16, 24, 32]);
    /// ```
    pub fn decode<T: Buf>(src: &mut T) -> TelnetCodecResult<NAOHTS> {
        let mut tab_stops = Vec::new();

        // Read all remaining bytes as tab stop positions
        while src.has_remaining() {
            tab_stops.push(src.get_u8());
        }

        Ok(NAOHTS { tab_stops })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_naohts_new() {
        let tab_stops = vec![8, 16, 24, 32];
        let naohts = NAOHTS::new(tab_stops.clone());
        assert_eq!(naohts.tab_stops, tab_stops);
    }

    #[test]
    fn test_naohts_default_tabs() {
        let naohts = NAOHTS::default_tabs(80);
        assert_eq!(naohts.tab_stops, vec![8, 16, 24, 32, 40, 48, 56, 64, 72]);
    }

    #[test]
    fn test_naohts_default_tabs_small_width() {
        let naohts = NAOHTS::default_tabs(20);
        assert_eq!(naohts.tab_stops, vec![8, 16]);
    }

    #[test]
    fn test_naohts_default_tabs_exact_multiple() {
        let naohts = NAOHTS::default_tabs(24);
        assert_eq!(naohts.tab_stops, vec![8, 16]);
    }

    #[test]
    fn test_naohts_default_tabs_very_small() {
        let naohts = NAOHTS::default_tabs(8);
        assert_eq!(naohts.tab_stops, vec![]);
    }

    #[test]
    fn test_naohts_len() {
        let naohts = NAOHTS::new(vec![8, 16, 24, 32]);
        assert_eq!(naohts.len(), 4);
    }

    #[test]
    fn test_naohts_len_empty() {
        let naohts = NAOHTS::new(vec![]);
        assert_eq!(naohts.len(), 0);
    }

    #[test]
    fn test_naohts_encode() {
        let naohts = NAOHTS::new(vec![8, 16, 24, 32]);
        let mut buffer = BytesMut::with_capacity(10);

        let result = naohts.encode(&mut buffer);
        assert!(result.is_ok());
        assert_eq!(buffer.as_ref(), &[8, 16, 24, 32]);
    }

    #[test]
    fn test_naohts_encode_empty() {
        let naohts = NAOHTS::new(vec![]);
        let mut buffer = BytesMut::with_capacity(10);

        let result = naohts.encode(&mut buffer);
        assert!(result.is_ok());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_naohts_encode_single_tab() {
        let naohts = NAOHTS::new(vec![8]);
        let mut buffer = BytesMut::with_capacity(10);

        let result = naohts.encode(&mut buffer);
        assert!(result.is_ok());
        assert_eq!(buffer.as_ref(), &[8]);
    }

    #[test]
    fn test_naohts_decode() {
        let data = vec![8, 16, 24, 32];
        let mut buffer = BytesMut::from(&data[..]);

        let result = NAOHTS::decode(&mut buffer);
        assert!(result.is_ok());

        let naohts = result.unwrap();
        assert_eq!(naohts.tab_stops, vec![8, 16, 24, 32]);
    }

    #[test]
    fn test_naohts_decode_empty() {
        let mut buffer = BytesMut::new();

        let result = NAOHTS::decode(&mut buffer);
        assert!(result.is_ok());

        let naohts = result.unwrap();
        assert_eq!(naohts.tab_stops, vec![]);
    }

    #[test]
    fn test_naohts_decode_single_tab() {
        let data = vec![8];
        let mut buffer = BytesMut::from(&data[..]);

        let result = NAOHTS::decode(&mut buffer);
        assert!(result.is_ok());

        let naohts = result.unwrap();
        assert_eq!(naohts.tab_stops, vec![8]);
    }

    #[test]
    fn test_naohts_decode_large_values() {
        let data = vec![10, 20, 50, 100, 150, 200, 255];
        let mut buffer = BytesMut::from(&data[..]);

        let result = NAOHTS::decode(&mut buffer);
        assert!(result.is_ok());

        let naohts = result.unwrap();
        assert_eq!(naohts.tab_stops, vec![10, 20, 50, 100, 150, 200, 255]);
    }

    #[test]
    fn test_naohts_encode_decode_roundtrip() {
        let original = NAOHTS::new(vec![8, 16, 24, 32, 40]);
        let mut buffer = BytesMut::with_capacity(10);

        // Encode
        let encode_result = original.encode(&mut buffer);
        assert!(encode_result.is_ok());

        // Decode
        let decode_result = NAOHTS::decode(&mut buffer);
        assert!(decode_result.is_ok());

        let decoded = decode_result.unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_naohts_encode_decode_roundtrip_empty() {
        let original = NAOHTS::new(vec![]);
        let mut buffer = BytesMut::with_capacity(10);

        // Encode
        let encode_result = original.encode(&mut buffer);
        assert!(encode_result.is_ok());

        // Decode
        let decode_result = NAOHTS::decode(&mut buffer);
        assert!(decode_result.is_ok());

        let decoded = decode_result.unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_naohts_encode_decode_roundtrip_default_tabs() {
        let original = NAOHTS::default_tabs(80);
        let mut buffer = BytesMut::with_capacity(20);

        // Encode
        let encode_result = original.encode(&mut buffer);
        assert!(encode_result.is_ok());

        // Decode
        let decode_result = NAOHTS::decode(&mut buffer);
        assert!(decode_result.is_ok());

        let decoded = decode_result.unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_naohts_clone() {
        let naohts = NAOHTS::new(vec![8, 16, 24]);
        let cloned = naohts.clone();
        assert_eq!(naohts, cloned);
    }

    #[test]
    fn test_naohts_debug() {
        let naohts = NAOHTS::new(vec![8, 16, 24]);
        let debug_str = format!("{:?}", naohts);
        assert!(debug_str.contains("NAOHTS"));
        assert!(debug_str.contains("tab_stops"));
    }
}
