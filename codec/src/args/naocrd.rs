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

//! Negotiate About Output Carriage-Return Disposition Data Sender (NAOCRD)
//!
//! https://tools.ietf.org/html/rfc8549#section-3.1.1
//!
//! NAOCRD is a subnegotiation of the NAOCRD option.
//!
//! The NAOCRD subnegotiation is used to negotiate about the output carriage-return disposition
//! data sender. The data is sent in a single byte.
//!

use crate::{CodecError, consts, result::CodecResult, status::TelnetOptionStatus};
use bytes::{Buf, BufMut};

///
/// Negotiate About Output Carriage-Return Disposition Data Sender (NAOCRD)
///
#[derive(Clone, Debug)]
pub enum NAOCRD {
    ///
    Sender(u8),
    ///
    Receiver(u8),
    ///
    Unknown(u8, u8),
}

impl NAOCRD {
    ///
    pub fn encoded_len(&self) -> usize {
        2
    }
    ///
    pub fn encode<T: BufMut>(&self, dst: &mut T) {
        match *self {
            NAOCRD::Sender(value) => {
                dst.put_u8(consts::option::naocrd::DS);
                dst.put_u8(value);
            }
            NAOCRD::Receiver(value) => {
                dst.put_u8(consts::option::naocrd::DR);
                dst.put_u8(value);
            }
            NAOCRD::Unknown(side, value) => {
                dst.put_u8(side);
                dst.put_u8(value);
            }
        }
    }

    ///
    /// Decode `NAOCRD` from `Buf`
    ///
    pub fn decode<T: Buf>(src: &mut T) -> CodecResult<NAOCRD> {
        if src.remaining() < 2 {
            return Err(CodecError::SubnegotiationError(format!(
                "Unable to decode {}",
                2 - src.remaining()
            )));
        }

        let side = src.get_u8();
        let value = src.get_u8();

        Ok(match side {
            consts::option::naocrd::DS => NAOCRD::Sender(value),
            consts::option::naocrd::DR => NAOCRD::Receiver(value),
            _ => NAOCRD::Unknown(side, value),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_sender_encode() {
        let mut buf = BytesMut::new();
        let naocrd = NAOCRD::Sender(42);

        naocrd.encode(&mut buf);

        assert_eq!(buf.len(), 2);
        assert_eq!(buf[0], consts::option::naocrd::DS);
        assert_eq!(buf[1], 42);
    }

    #[test]
    fn test_receiver_encode() {
        let mut buf = BytesMut::new();
        let naocrd = NAOCRD::Receiver(123);

        naocrd.encode(&mut buf);

        assert_eq!(buf.len(), 2);
        assert_eq!(buf[0], consts::option::naocrd::DR);
        assert_eq!(buf[1], 123);
    }

    #[test]
    fn test_unknown_encode() {
        let mut buf = BytesMut::new();
        let naocrd = NAOCRD::Unknown(99, 55);

        naocrd.encode(&mut buf);

        assert_eq!(buf.len(), 2);
        assert_eq!(buf[0], 99);
        assert_eq!(buf[1], 55);
    }

    #[test]
    fn test_sender_decode() {
        let mut buf = BytesMut::new();
        buf.put_u8(consts::option::naocrd::DS);
        buf.put_u8(42);

        let result = NAOCRD::decode(&mut buf).unwrap();

        match result {
            NAOCRD::Sender(value) => assert_eq!(value, 42),
            _ => panic!("Expected NAOCRD::Sender"),
        }
    }

    #[test]
    fn test_receiver_decode() {
        let mut buf = BytesMut::new();
        buf.put_u8(consts::option::naocrd::DR);
        buf.put_u8(123);

        let result = NAOCRD::decode(&mut buf).unwrap();

        match result {
            NAOCRD::Receiver(value) => assert_eq!(value, 123),
            _ => panic!("Expected NAOCRD::Receiver"),
        }
    }

    #[test]
    fn test_unknown_decode() {
        let mut buf = BytesMut::new();
        buf.put_u8(99);
        buf.put_u8(55);

        let result = NAOCRD::decode(&mut buf).unwrap();

        match result {
            NAOCRD::Unknown(side, value) => {
                assert_eq!(side, 99);
                assert_eq!(value, 55);
            }
            _ => panic!("Expected NAOCRD::Unknown"),
        }
    }

    #[test]
    fn test_decode_insufficient_data() {
        let mut buf = BytesMut::new();
        buf.put_u8(consts::option::naocrd::DS);
        // Only 1 byte, need 2

        let result = NAOCRD::decode(&mut buf);

        assert!(result.is_err());
        match result {
            Err(CodecError::SubnegotiationError(msg)) => {
                assert!(msg.contains("Unable to decode"));
            }
            _ => panic!("Expected SubnegotiationError"),
        }
    }

    #[test]
    fn test_decode_empty_buffer() {
        let mut buf = BytesMut::new();

        let result = NAOCRD::decode(&mut buf);

        assert!(result.is_err());
        match result {
            Err(CodecError::SubnegotiationError(msg)) => {
                assert!(msg.contains("Unable to decode"));
            }
            _ => panic!("Expected SubnegotiationError"),
        }
    }

    #[test]
    fn test_encoded_len() {
        assert_eq!(NAOCRD::Sender(0).encoded_len(), 2);
        assert_eq!(NAOCRD::Receiver(0).encoded_len(), 2);
        assert_eq!(NAOCRD::Unknown(0, 0).encoded_len(), 2);
    }

    #[test]
    fn test_roundtrip_sender() {
        let original = NAOCRD::Sender(200);
        let mut buf = BytesMut::new();

        original.encode(&mut buf);
        let decoded = NAOCRD::decode(&mut buf).unwrap();

        match decoded {
            NAOCRD::Sender(value) => assert_eq!(value, 200),
            _ => panic!("Expected NAOCRD::Sender"),
        }
    }

    #[test]
    fn test_roundtrip_receiver() {
        let original = NAOCRD::Receiver(150);
        let mut buf = BytesMut::new();

        original.encode(&mut buf);
        let decoded = NAOCRD::decode(&mut buf).unwrap();

        match decoded {
            NAOCRD::Receiver(value) => assert_eq!(value, 150),
            _ => panic!("Expected NAOCRD::Receiver"),
        }
    }

    #[test]
    fn test_roundtrip_unknown() {
        let original = NAOCRD::Unknown(77, 88);
        let mut buf = BytesMut::new();

        original.encode(&mut buf);
        let decoded = NAOCRD::decode(&mut buf).unwrap();

        match decoded {
            NAOCRD::Unknown(side, value) => {
                assert_eq!(side, 77);
                assert_eq!(value, 88);
            }
            _ => panic!("Expected NAOCRD::Unknown"),
        }
    }

    #[test]
    fn test_all_byte_values_sender() {
        for i in 0..=255u8 {
            let mut buf = BytesMut::new();
            let naocrd = NAOCRD::Sender(i);

            naocrd.encode(&mut buf);
            let decoded = NAOCRD::decode(&mut buf).unwrap();

            match decoded {
                NAOCRD::Sender(value) => assert_eq!(value, i),
                _ => panic!("Expected NAOCRD::Sender for value {}", i),
            }
        }
    }

    #[test]
    fn test_all_byte_values_receiver() {
        for i in 0..=255u8 {
            let mut buf = BytesMut::new();
            let naocrd = NAOCRD::Receiver(i);

            naocrd.encode(&mut buf);
            let decoded = NAOCRD::decode(&mut buf).unwrap();

            match decoded {
                NAOCRD::Receiver(value) => assert_eq!(value, i),
                _ => panic!("Expected NAOCRD::Receiver for value {}", i),
            }
        }
    }

    #[test]
    fn test_clone() {
        let original = NAOCRD::Sender(42);
        let cloned = original.clone();

        match cloned {
            NAOCRD::Sender(value) => assert_eq!(value, 42),
            _ => panic!("Expected NAOCRD::Sender"),
        }
    }

    #[test]
    fn test_debug_format() {
        let sender = NAOCRD::Sender(42);
        let receiver = NAOCRD::Receiver(123);
        let unknown = NAOCRD::Unknown(99, 55);

        let sender_debug = format!("{:?}", sender);
        let receiver_debug = format!("{:?}", receiver);
        let unknown_debug = format!("{:?}", unknown);

        assert!(sender_debug.contains("Sender"));
        assert!(sender_debug.contains("42"));

        assert!(receiver_debug.contains("Receiver"));
        assert!(receiver_debug.contains("123"));

        assert!(unknown_debug.contains("Unknown"));
        assert!(unknown_debug.contains("99"));
        assert!(unknown_debug.contains("55"));
    }

    #[test]
    fn test_decode_extra_bytes_ignored() {
        let mut buf = BytesMut::new();
        buf.put_u8(consts::option::naocrd::DS);
        buf.put_u8(42);
        buf.put_u8(255); // Extra byte that should be left in buffer

        let result = NAOCRD::decode(&mut buf).unwrap();

        match result {
            NAOCRD::Sender(value) => assert_eq!(value, 42),
            _ => panic!("Expected NAOCRD::Sender"),
        }

        // Check that the extra byte is still in the buffer
        assert_eq!(buf.remaining(), 1);
        assert_eq!(buf.get_u8(), 255);
    }
}
