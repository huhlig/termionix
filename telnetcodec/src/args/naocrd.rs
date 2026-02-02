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

//! Negotiate About Output Carriage-Return Disposition Data Sender (NAOCRD)
//!
//! https://tools.ietf.org/html/rfc8549#section-3.1.1
//!
//! NAOCRD is a subnegotiation of the NAOCRD option.
//!
//! The NAOCRD subnegotiation is used to negotiate about the output carriage-return disposition
//! data sender. The data is sent in a single byte.
//!

use crate::{TelnetCodecError, consts, result::TelnetCodecResult};
use byteorder::WriteBytesExt;
use bytes::{Buf, BufMut};

/// Negotiate About Output Carriage-Return Disposition Data Sender (NAOCRD)
///
/// This enum represents the NAOCRD subnegotiation option as defined in
/// [RFC 8549 Section 3.1.1](https://tools.ietf.org/html/rfc8549#section-3.1.1).
///
/// The NAOCRD option is used in TELNET sidechannel negotiations to agree upon the handling
/// of carriage-return characters in output data. It allows both the server and client
/// to communicate their preferred carriage-return disposition.
///
/// # Variants
///
/// - `Sender(u8)` - Carriage-return disposition sent by the data sender. The value
///   is a single byte indicating the desired disposition mode.
/// - `Receiver(u8)` - Carriage-return disposition from the data receiver's perspective.
///   The value is a single byte indicating the receiver's preferred disposition mode.
/// - `Unknown(u8, u8)` - An unrecognized subnegotiation with an unknown side identifier
///   and associated value. The first byte is the side identifier, and the second is the data.
///
/// # Examples
///
/// ```ignore
/// use bytes::BytesMut;
///
/// // Create a sender disposition
/// let naocrd = NAOCRD::Sender(0);
/// let mut buf = BytesMut::new();
/// naocrd.encode(&mut buf)?;
///
/// // Decode from buffer
/// let decoded = NAOCRD::decode(&mut buf)?;
/// ```
#[derive(Clone, Debug)]
pub enum NAOCRD {
    /// Carriage-return disposition from the data sender
    Sender(u8),
    /// Carriage-return disposition from the data receiver
    Receiver(u8),
    /// An unrecognized subnegotiation variant with unknown side identifier and value
    Unknown(u8, u8),
}

impl NAOCRD {
    /// Returns the encoded length of this NAOCRD subnegotiation.
    ///
    /// This always returns 2, as the NAOCRD subnegotiation consists of:
    /// - 1 byte for the side identifier (Sender, Receiver, or Unknown)
    /// - 1 byte for the disposition value
    ///
    /// # Returns
    ///
    /// Always returns `2`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let naocrd = NAOCRD::Sender(42);
    /// assert_eq!(naocrd.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        2
    }

    /// Encodes this NAOCRD subnegotiation into the provided buffer.
    ///
    /// This method serializes the NAOCRD data into binary format suitable for transmission
    /// over a TELNET connection. The encoding follows the TELNET sidechannel specification.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable reference to a buffer implementing `BufMut` where the encoded
    ///   data will be written.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written (always 2 on success)
    /// * `Err(CodecError)` - An error if encoding fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use bytes::BytesMut;
    ///
    /// let mut buf = BytesMut::new();
    /// let naocrd = NAOCRD::Receiver(123);
    /// let bytes_written = naocrd.encode(&mut buf)?;
    /// assert_eq!(bytes_written, 2);
    /// ```
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> TelnetCodecResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Writes this NAOCRD subnegotiation to the provided I/O writer.
    ///
    /// This is the underlying implementation that performs the actual byte serialization.
    /// It handles encoding of the side identifier and disposition value.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable reference to a type implementing `std::io::Write` where
    ///   the encoded bytes will be written.
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written (always 2 on success)
    /// * `Err(std::io::Error)` - An I/O error if writing fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::io::Cursor;
    ///
    /// let mut writer = Cursor::new(vec![]);
    /// let naocrd = NAOCRD::Sender(42);
    /// let bytes_written = naocrd.write(&mut writer)?;
    /// assert_eq!(bytes_written, 2);
    /// ```
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        match *self {
            NAOCRD::Sender(value) => {
                writer.write_u8(consts::option::naocrd::DS)?;
                writer.write_u8(value)?;
                Ok(2)
            }
            NAOCRD::Receiver(value) => {
                writer.write_u8(consts::option::naocrd::DR)?;
                writer.write_u8(value)?;
                Ok(2)
            }
            NAOCRD::Unknown(side, value) => {
                writer.write_u8(side)?;
                writer.write_u8(value)?;
                Ok(2)
            }
        }
    }

    /// Decodes a NAOCRD subnegotiation from the provided buffer.
    ///
    /// This method deserializes binary TELNET sidechannel data into a `NAOCRD` enum variant.
    /// It reads exactly 2 bytes from the buffer: the side identifier and the disposition value.
    ///
    /// # Arguments
    ///
    /// * `src` - A mutable reference to a buffer implementing `Buf` containing the data to decode
    ///
    /// # Returns
    ///
    /// * `Ok(NAOCRD)` - The decoded subnegotiation
    /// * `Err(CodecError)` - An error if:
    ///   - There is insufficient data (fewer than 2 bytes remaining)
    ///   - The side identifier is unrecognized (treated as `Unknown`)
    ///
    /// # Errors
    ///
    /// Returns `CodecError::SubnegotiationError` with `SubnegotiationErrorKind::InsufficientData`
    /// if fewer than 2 bytes are available in the buffer.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use bytes::BytesMut;
    ///
    /// let mut buf = BytesMut::new();
    /// buf.put_u8(consts::option::naocrd::DS);
    /// buf.put_u8(42);
    ///
    /// let naocrd = NAOCRD::decode(&mut buf)?;
    /// match naocrd {
    ///     NAOCRD::Sender(value) => println!("Sender disposition: {}", value),
    ///     _ => {}
    /// }
    /// ```
    pub fn decode<T: Buf>(src: &mut T) -> TelnetCodecResult<NAOCRD> {
        if src.remaining() < 2 {
            return Err(TelnetCodecError::SubnegotiationError {
                option: Some(crate::consts::option::NAOCRD),
                reason: crate::SubnegotiationErrorKind::InsufficientData {
                    required: 2,
                    available: src.remaining(),
                },
            });
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

        naocrd.encode(&mut buf).unwrap();

        assert_eq!(buf.len(), 2);
        assert_eq!(buf[0], consts::option::naocrd::DS);
        assert_eq!(buf[1], 42);
    }

    #[test]
    fn test_receiver_encode() {
        let mut buf = BytesMut::new();
        let naocrd = NAOCRD::Receiver(123);

        naocrd.encode(&mut buf).unwrap();

        assert_eq!(buf.len(), 2);
        assert_eq!(buf[0], consts::option::naocrd::DR);
        assert_eq!(buf[1], 123);
    }

    #[test]
    fn test_unknown_encode() {
        let mut buf = BytesMut::new();
        let naocrd = NAOCRD::Unknown(99, 55);

        naocrd.encode(&mut buf).unwrap();

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
            Err(TelnetCodecError::SubnegotiationError { option, reason }) => {
                assert_eq!(option, Some(consts::option::NAOCRD));
                assert!(matches!(
                    reason,
                    crate::SubnegotiationErrorKind::InsufficientData { .. }
                ));
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
            Err(TelnetCodecError::SubnegotiationError { option, reason }) => {
                assert_eq!(option, Some(consts::option::NAOCRD));
                assert!(matches!(
                    reason,
                    crate::SubnegotiationErrorKind::InsufficientData { .. }
                ));
            }
            _ => panic!("Expected SubnegotiationError"),
        }
    }

    #[test]
    fn test_encoded_len() {
        assert_eq!(NAOCRD::Sender(0).len(), 2);
        assert_eq!(NAOCRD::Receiver(0).len(), 2);
        assert_eq!(NAOCRD::Unknown(0, 0).len(), 2);
    }

    #[test]
    fn test_roundtrip_sender() {
        let original = NAOCRD::Sender(200);
        let mut buf = BytesMut::new();

        original.encode(&mut buf).unwrap();
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

        original.encode(&mut buf).unwrap();
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

        original.encode(&mut buf).unwrap();
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

            naocrd.encode(&mut buf).unwrap();
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

            naocrd.encode(&mut buf).unwrap();
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
