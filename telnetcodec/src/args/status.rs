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

//! Telnet Option Status
//!
//! https://tools.ietf.org/html/rfc8549#section-3.1.1
//!
//! Telnet Option Status is a subnegotiation of the Telnet Option Status
//! option.
//!
//! The Telnet Option Status subnegotiation is used to negotiate about the
//! status of the Telnet options. The data is sent in a series of bytes.
//!

use crate::{CodecError, CodecResult, SubnegotiationErrorKind, TelnetOption, consts};
use byteorder::WriteBytesExt;
use bytes::{Buf, BufMut};
use std::collections::HashMap;

/// Status subnegotiation command types
#[derive(Clone, Debug, PartialEq)]
pub enum StatusCommand {
    /// SEND - Request status information
    Send,
    /// IS - Provide status information
    Is,
}

impl StatusCommand {
    /// Status Command From Byte
    pub fn from_byte(byte: u8) -> CodecResult<Self> {
        match byte {
            consts::option::status::IS => Ok(StatusCommand::Is),
            consts::option::status::SEND => Ok(StatusCommand::Send),
            _ => Err(CodecError::SubnegotiationError {
                option: Some(consts::option::STATUS),
                reason: SubnegotiationErrorKind::InvalidCommand {
                    command: byte,
                    expected: Some(vec![
                        consts::option::status::IS,
                        consts::option::status::SEND,
                    ]),
                },
            }),
        }
    }
    /// Status Command To Byte
    pub fn to_byte(&self) -> u8 {
        match self {
            StatusCommand::Send => consts::option::status::SEND,
            StatusCommand::Is => consts::option::status::IS,
        }
    }
}

/// Telnet Option Status
///
/// Represents the status of codec options as pairs of (option, DO/DONT, WILL/WONT)
#[derive(Clone, Debug, PartialEq)]
pub struct TelnetOptionStatus {
    ///
    pub command: StatusCommand,
    /// Map of option -> (DO/DONT state, WILL/WONT state)
    /// true for DO/WILL, false for DONT/WONT
    pub options: HashMap<TelnetOption, (bool, bool)>,
}

impl TelnetOptionStatus {
    ///
    /// Get Encoded Length of `TelnetOptionStatus`
    ///
    pub fn len(&self) -> usize {
        // 1 byte for command + 3 bytes per option (verb + option code)
        // Each option has 2 entries: one for DO/DONT and one for WILL/WONT
        1 + self.options.len() * 4 // Two bytes per item
    }
    ///
    /// Encode `TelnetOptionStatus` to `BufMut`
    ///
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> CodecResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Writer for TelnetOptionStatus
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut len = 1;
        writer.write_u8(self.command.to_byte())?;

        for (option, (do_state, will_state)) in &self.options {
            // Encode DO/DONT state
            if *do_state {
                writer.write_u8(consts::DO)?; // DO
            } else {
                writer.write_u8(consts::DONT)?; // DONT
            }
            writer.write_u8(option.to_u8())?;

            // Encode WILL/WONT state
            if *will_state {
                writer.write_u8(consts::WILL)?; // WILL
            } else {
                writer.write_u8(consts::WONT)?; // WONT
            }
            writer.write_u8(option.to_u8())?;
            len += 4;
        }
        Ok(len)
    }
    ///
    /// Decode `TelnetOptionStatus` from `Buf`
    ///
    pub fn decode<T: Buf>(src: &mut T) -> CodecResult<TelnetOptionStatus> {
        if src.remaining() < 1 {
            return Err(CodecError::SubnegotiationError {
                option: Some(consts::option::STATUS),
                reason: SubnegotiationErrorKind::InsufficientData {
                    required: 1,
                    available: src.remaining(),
                },
            });
        }

        let command = StatusCommand::from_byte(src.get_u8())?;

        let mut options = HashMap::new();

        // SEND command should have no additional data
        if matches!(command, StatusCommand::Send) && src.remaining() > 0 {
            return Err(CodecError::SubnegotiationError {
                option: Some(consts::option::STATUS),
                reason: SubnegotiationErrorKind::UnexpectedData {
                    reason: "Status SEND should not contain option data".into(),
                },
            });
        }

        // IS command contains option status pairs
        while src.remaining() >= 2 {
            let verb = src.get_u8();
            let option_code = src.get_u8();

            let option = TelnetOption::try_from(option_code).map_err(|_| {
                CodecError::SubnegotiationError {
                    option: Some(consts::option::STATUS),
                    reason: SubnegotiationErrorKind::UnknownOption { code: option_code },
                }
            })?;

            let entry = options.entry(option).or_insert((false, false));

            match verb {
                consts::DO => entry.0 = true,    // DO
                consts::DONT => entry.0 = false, // DONT
                consts::WILL => entry.1 = true,  // WILL
                consts::WONT => entry.1 = false, // WONT
                _ => {
                    return Err(CodecError::SubnegotiationError {
                        option: Some(consts::option::STATUS),
                        reason: SubnegotiationErrorKind::InvalidVerb { verb },
                    });
                }
            }
        }

        if src.remaining() > 0 {
            return Err(CodecError::SubnegotiationError {
                option: Some(consts::option::STATUS),
                reason: SubnegotiationErrorKind::IncompleteData {
                    description: "incomplete option pair".into(),
                },
            });
        }

        Ok(Self { command, options })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn test_status_command_from_byte() {
        assert_eq!(
            StatusCommand::from_byte(consts::option::status::SEND).unwrap(),
            StatusCommand::Send
        );
        assert_eq!(
            StatusCommand::from_byte(consts::option::status::IS).unwrap(),
            StatusCommand::Is
        );
        assert!(StatusCommand::from_byte(99).is_err());
    }

    #[test]
    fn test_status_command_to_byte() {
        assert_eq!(StatusCommand::Send.to_byte(), consts::option::status::SEND);
        assert_eq!(StatusCommand::Is.to_byte(), consts::option::status::IS);
    }

    #[test]
    fn test_telnet_option_status_encode_send() {
        let status = TelnetOptionStatus {
            command: StatusCommand::Send,
            options: HashMap::new(),
        };

        let mut buf = BytesMut::new();
        status.encode(&mut buf).expect("error encoding status");

        assert_eq!(buf.len(), 1);
        assert_eq!(buf[0], consts::option::status::SEND);
    }

    #[test]
    fn test_telnet_option_status_encode_is_with_options() {
        let mut options = HashMap::new();
        options.insert(TelnetOption::Echo, (true, false)); // DO, WONT
        options.insert(TelnetOption::SuppressGoAhead, (false, true)); // DONT, WILL

        let status = TelnetOptionStatus {
            command: StatusCommand::Is,
            options,
        };

        let mut buf = BytesMut::new();
        status.encode(&mut buf).expect("error encoding status");

        // Command byte + 4 bytes per option (DO/DONT + code, WILL/WONT + code)
        assert_eq!(buf.len(), 1 + 4 * 2);
        assert_eq!(buf[0], consts::option::status::IS);
    }

    #[test]
    fn test_telnet_option_status_decode_send() {
        let mut buf = BytesMut::new();
        buf.put_u8(consts::option::status::SEND);

        let result = TelnetOptionStatus::decode(&mut buf).unwrap();

        assert_eq!(result.command, StatusCommand::Send);
        assert!(result.options.is_empty());
    }

    #[test]
    fn test_telnet_option_status_decode_send_with_data_should_fail() {
        let mut buf = BytesMut::new();
        buf.put_u8(consts::option::status::SEND);
        buf.put_u8(consts::DO);
        buf.put_u8(TelnetOption::Echo.to_u8());

        let result = TelnetOptionStatus::decode(&mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_telnet_option_status_decode_is_with_options() {
        let mut buf = BytesMut::new();
        buf.put_u8(consts::option::status::IS);
        buf.put_u8(consts::DO);
        buf.put_u8(TelnetOption::Echo.to_u8());
        buf.put_u8(consts::WILL);
        buf.put_u8(TelnetOption::Echo.to_u8());
        buf.put_u8(consts::DONT);
        buf.put_u8(TelnetOption::SuppressGoAhead.to_u8());
        buf.put_u8(consts::WONT);
        buf.put_u8(TelnetOption::SuppressGoAhead.to_u8());

        let result = TelnetOptionStatus::decode(&mut buf).unwrap();

        assert_eq!(result.command, StatusCommand::Is);
        assert_eq!(result.options.len(), 2);
        assert_eq!(result.options.get(&TelnetOption::Echo), Some(&(true, true)));
        assert_eq!(
            result.options.get(&TelnetOption::SuppressGoAhead),
            Some(&(false, false))
        );
    }

    #[test]
    fn test_telnet_option_status_decode_incomplete_pair() {
        let mut buf = BytesMut::new();
        buf.put_u8(consts::option::status::IS);
        buf.put_u8(consts::DO);
        // Missing option code

        let result = TelnetOptionStatus::decode(&mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_telnet_option_status_decode_invalid_verb() {
        let mut buf = BytesMut::new();
        buf.put_u8(consts::option::status::IS);
        buf.put_u8(99); // Invalid verb
        buf.put_u8(TelnetOption::Echo.to_u8());

        let result = TelnetOptionStatus::decode(&mut buf);
        assert!(result.is_err());
    }

    #[test]
    fn test_telnet_option_status_encode_decode_round_trip() {
        let mut options = HashMap::new();
        options.insert(TelnetOption::Echo, (true, false));
        options.insert(TelnetOption::SuppressGoAhead, (false, true));
        options.insert(TelnetOption::Status, (true, true));

        let original = TelnetOptionStatus {
            command: StatusCommand::Is,
            options,
        };

        let mut buf = BytesMut::new();
        original.encode(&mut buf).expect("error encoding status");

        let decoded = TelnetOptionStatus::decode(&mut buf).unwrap();

        assert_eq!(decoded.command, original.command);
        assert_eq!(decoded.options, original.options);
    }

    #[test]
    fn test_telnet_option_status_encoded_len() {
        let mut options = HashMap::new();
        options.insert(TelnetOption::Echo, (true, false));
        options.insert(TelnetOption::SuppressGoAhead, (false, true));

        let status = TelnetOptionStatus {
            command: StatusCommand::Is,
            options,
        };

        let expected_len = 1 + 2 * 4; // 1 command + 2 options * 4 bytes each
        assert_eq!(status.len(), expected_len);

        let mut buf = BytesMut::new();
        status.encode(&mut buf).expect("error encoding status");
        assert_eq!(buf.len(), expected_len);
    }

    #[test]
    fn test_telnet_option_status_decode_empty_buffer() {
        let mut buf = BytesMut::new();
        let result = TelnetOptionStatus::decode(&mut buf);
        assert!(result.is_err());
    }
}
