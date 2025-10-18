//
// Copyright 2019 Hans W. Uhlig. All Rights Reserved.
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

use crate::{CodecError, CodecResult, consts};
use bytes::{Buf, BufMut};

///
/// `NAOHTS` contains Negotiation data about Output Horizontal Tabstops.
/// [RFC653](http://www.iana.org/go/rfc653)
/// TODO: Implement This
///
///
/// `NAOHTS` contains Negotiation data about Output Horizontal Tabstops.
/// [RFC653](http://www.iana.org/go/rfc653)
///
/// The NAOHTS option is used to transmit a list of horizontal tab stops.
/// Each tab stop is represented as a single byte value indicating the column position.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NAOHTS {
    /// List of horizontal tab stop positions (column numbers)
    pub tab_stops: Vec<u8>,
}

impl NAOHTS {
    /// Create a new NAOHTS with the given tab stops
    pub fn new(tab_stops: Vec<u8>) -> Self {
        Self { tab_stops }
    }

    /// Create a NAOHTS with default tab stops (every 8 columns)
    pub fn default_tabs(width: u8) -> Self {
        let mut tab_stops = Vec::new();
        let mut pos = 8u8;
        while pos < width {
            tab_stops.push(pos);
            pos = pos.saturating_add(8);
        }
        Self { tab_stops }
    }

    ///
    /// Get Encoded Length of `NAOHTS`
    ///
    fn encoded_len(&self) -> usize {
        self.tab_stops.len()
    }

    ///
    /// Encode `NAOHTS` to `BufMut`
    ///
    fn encode<T: BufMut>(&self, dst: &mut T) -> CodecResult<()> {
        if dst.remaining_mut() < self.encoded_len() {
            return Err(CodecError::SubnegotiationError(String::from(
                "Unable to encode",
            )));
        }

        // Write each tab stop position
        for &tab_stop in &self.tab_stops {
            dst.put_u8(tab_stop);
        }

        Ok(())
    }

    ///
    /// Decode `NAOHTS` from `Buf`
    ///
    fn decode<T: Buf>(src: &mut T) -> CodecResult<NAOHTS> {
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
    use bytes::{Buf, BytesMut};

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
    fn test_naohts_encoded_len() {
        let naohts = NAOHTS::new(vec![8, 16, 24, 32]);
        assert_eq!(naohts.encoded_len(), 4);
    }

    #[test]
    fn test_naohts_encoded_len_empty() {
        let naohts = NAOHTS::new(vec![]);
        assert_eq!(naohts.encoded_len(), 0);
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
