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

//! Line Mode Options

use crate::consts;
use bytes::BufMut;

/// Telnet Line Mode Option subnegotiation commands and arguments (RFC 1184)
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LineModeOption {
    /// MODE command - set line mode and flags
    Mode {
        /// Mode flags
        mode: LineModeFlags,
    },
    /// FORWARDMASK command - set forward mask
    ForwardMask {
        /// Mask of characters that should cause forwarding
        mask: [bool; 256],
    },
    /// SLC (Special Line Character) command
    Slc {
        /// List of special line character definitions
        chars: Vec<SlcDefinition>,
    },
}

impl LineModeOption {
    /// Parse line mode option from bytes
    pub fn parse(data: &[u8]) -> Result<Self, &'static str> {
        if data.is_empty() {
            return Err("Empty linemode subnegotiation");
        }

        match data[0] {
            consts::option::linemode::MODE => {
                if data.len() < 2 {
                    return Err("MODE requires at least 1 byte of data");
                }
                Ok(Self::Mode {
                    mode: LineModeFlags::from_byte(data[1]),
                })
            }
            consts::option::linemode::FORWARDMASK => {
                if data.len() < 33 {
                    return Err("FORWARDMASK requires 32 bytes of data");
                }
                let mut mask = [false; 256];
                for byte_idx in 0..32 {
                    let byte = data[1 + byte_idx];
                    for bit_idx in 0..8 {
                        let char_idx = byte_idx * 8 + bit_idx;
                        mask[char_idx] = (byte & (1 << (7 - bit_idx))) != 0;
                    }
                }
                Ok(Self::ForwardMask { mask })
            }
            consts::option::linemode::SLC => {
                if (data.len() - 1) % 3 != 0 {
                    return Err("SLC data must be in triplets");
                }
                let mut chars = Vec::new();
                for i in (1..data.len()).step_by(3) {
                    if i + 2 >= data.len() {
                        break;
                    }
                    chars.push(SlcDefinition {
                        function: SlcFunction::from_byte(data[i]),
                        flags: SlcFlags::from_byte(data[i + 1]),
                        value: data[i + 2],
                    });
                }
                Ok(Self::Slc { chars })
            }
            _ => Err("Unknown linemode subnegotiation command"),
        }
    }

    /// Serialize line mode option to bytes
    pub fn serialize(&self, buf: &mut impl BufMut) {
        match self {
            Self::Mode { mode } => {
                buf.put_u8(consts::option::linemode::MODE);
                buf.put_u8(mode.to_byte());
            }
            Self::ForwardMask { mask } => {
                buf.put_u8(consts::option::linemode::FORWARDMASK);
                for byte_idx in 0..32 {
                    let mut byte = 0u8;
                    for bit_idx in 0..8 {
                        let char_idx = byte_idx * 8 + bit_idx;
                        if mask[char_idx] {
                            byte |= 1 << (7 - bit_idx);
                        }
                    }
                    buf.put_u8(byte);
                }
            }
            Self::Slc { chars } => {
                buf.put_u8(consts::option::linemode::SLC);
                for slc in chars {
                    buf.put_u8(slc.function.to_byte());
                    buf.put_u8(slc.flags.to_byte());
                    buf.put_u8(slc.value);
                }
            }
        }
    }
}

impl std::fmt::Display for LineModeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

/// Line Mode flags (used with MODE command)
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LineModeFlags {
    /// Edit mode - client performs line editing
    pub edit: bool,
    /// Trapsig mode - client handles signals locally
    pub trapsig: bool,
    /// Soft tab mode - client converts tabs to spaces
    pub soft_tab: bool,
    /// Lit echo mode - literal echo of all characters
    pub lit_echo: bool,
    /// ACK mode - acknowledgment of mode changes
    pub ack: bool,
}

impl LineModeFlags {
    /// Create flags from a byte value
    pub fn from_byte(byte: u8) -> Self {
        Self {
            edit: (byte & 0x01) != 0,
            trapsig: (byte & 0x02) != 0,
            soft_tab: (byte & 0x04) != 0,
            lit_echo: (byte & 0x08) != 0,
            ack: (byte & 0x10) != 0,
        }
    }

    /// Convert flags to a byte value
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.edit {
            byte |= 0x01;
        }
        if self.trapsig {
            byte |= 0x02;
        }
        if self.soft_tab {
            byte |= 0x04;
        }
        if self.lit_echo {
            byte |= 0x08;
        }
        if self.ack {
            byte |= 0x10;
        }
        byte
    }
}

impl std::fmt::Display for LineModeFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

/// Special Line Character definition
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct SlcDefinition {
    /// SLC function code
    pub function: SlcFunction,
    /// SLC flags
    pub flags: SlcFlags,
    /// Character value (or 0 if not supported)
    pub value: u8,
}

impl std::fmt::Display for SlcDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

/// SLC function codes
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SlcFunction {
    /// Sync signal
    Synch = 1,
    /// Break signal
    Brk = 2,
    /// Interrupt Process
    Ip = 3,
    /// Abort Output
    Ao = 4,
    /// Are You There
    Ayt = 5,
    /// Erase Character
    Ec = 6,
    /// Erase Line
    El = 7,
    /// End of File
    Eof = 8,
    /// Suspend Process
    Susp = 9,
    /// Abort Process
    Abort = 10,
    /// End of Record
    Eor = 11,
    /// Literal Next
    Lnext = 12,
    /// Erase Word
    Ew = 13,
    /// Reprint Line
    Rp = 14,
    /// X-On character
    Xon = 15,
    /// X-Off character
    Xoff = 16,
    /// Forward Char
    ForwardChar = 17,
    /// Other/Unknown function
    Other(u8),
}

impl SlcFunction {
    /// Create SLC function from byte value
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            1 => Self::Synch,
            2 => Self::Brk,
            3 => Self::Ip,
            4 => Self::Ao,
            5 => Self::Ayt,
            6 => Self::Ec,
            7 => Self::El,
            8 => Self::Eof,
            9 => Self::Susp,
            10 => Self::Abort,
            11 => Self::Eor,
            12 => Self::Lnext,
            13 => Self::Ew,
            14 => Self::Rp,
            15 => Self::Xon,
            16 => Self::Xoff,
            17 => Self::ForwardChar,
            other => Self::Other(other),
        }
    }

    /// Convert SLC function to byte value
    pub fn to_byte(&self) -> u8 {
        match self {
            Self::Synch => 1,
            Self::Brk => 2,
            Self::Ip => 3,
            Self::Ao => 4,
            Self::Ayt => 5,
            Self::Ec => 6,
            Self::El => 7,
            Self::Eof => 8,
            Self::Susp => 9,
            Self::Abort => 10,
            Self::Eor => 11,
            Self::Lnext => 12,
            Self::Ew => 13,
            Self::Rp => 14,
            Self::Xon => 15,
            Self::Xoff => 16,
            Self::ForwardChar => 17,
            Self::Other(val) => *val,
        }
    }
}

impl std::fmt::Display for SlcFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

/// SLC flags
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct SlcFlags {
    /// Level of support/acknowledgment
    pub level: SlcLevel,
    /// Acknowledgment flag
    pub ack: bool,
    /// Flushin flag
    pub flushin: bool,
    /// Flushout flag
    pub flushout: bool,
}

impl SlcFlags {
    /// Create SLC flags from byte value
    pub fn from_byte(byte: u8) -> Self {
        Self {
            level: SlcLevel::from_byte(byte & 0x03),
            ack: (byte & 0x80) != 0,
            flushin: (byte & 0x40) != 0,
            flushout: (byte & 0x20) != 0,
        }
    }

    /// Convert SLC flags to byte value
    pub fn to_byte(&self) -> u8 {
        let mut byte = self.level.to_byte();
        if self.ack {
            byte |= 0x80;
        }
        if self.flushin {
            byte |= 0x40;
        }
        if self.flushout {
            byte |= 0x20;
        }
        byte
    }
}

impl std::fmt::Display for SlcFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

/// SLC support level
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SlcLevel {
    /// Not supported
    NoSupport = 0,
    /// Use my default value
    Default = 1,
    /// Use the value I provide
    Value = 2,
    /// I want to use your value
    CantChange = 3,
}

impl SlcLevel {
    /// Create SLC level from byte value
    pub fn from_byte(byte: u8) -> Self {
        match byte & 0x03 {
            0 => Self::NoSupport,
            1 => Self::Default,
            2 => Self::Value,
            3 => Self::CantChange,
            _ => unreachable!(),
        }
    }

    /// Convert SLC level to byte value
    pub fn to_byte(&self) -> u8 {
        match self {
            Self::NoSupport => 0,
            Self::Default => 1,
            Self::Value => 2,
            Self::CantChange => 3,
        }
    }
}

impl std::fmt::Display for SlcLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linemode_flags_from_byte() {
        let flags = LineModeFlags::from_byte(0b00011111);
        assert!(flags.edit);
        assert!(flags.trapsig);
        assert!(flags.soft_tab);
        assert!(flags.lit_echo);
        assert!(flags.ack);

        let flags = LineModeFlags::from_byte(0b00000000);
        assert!(!flags.edit);
        assert!(!flags.trapsig);
        assert!(!flags.soft_tab);
        assert!(!flags.lit_echo);
        assert!(!flags.ack);

        let flags = LineModeFlags::from_byte(0b00000101);
        assert!(flags.edit);
        assert!(!flags.trapsig);
        assert!(flags.soft_tab);
        assert!(!flags.lit_echo);
        assert!(!flags.ack);
    }

    #[test]
    fn test_linemode_flags_to_byte() {
        let flags = LineModeFlags {
            edit: true,
            trapsig: true,
            soft_tab: true,
            lit_echo: true,
            ack: true,
        };
        assert_eq!(flags.to_byte(), 0b00011111);

        let flags = LineModeFlags {
            edit: false,
            trapsig: false,
            soft_tab: false,
            lit_echo: false,
            ack: false,
        };
        assert_eq!(flags.to_byte(), 0b00000000);

        let flags = LineModeFlags {
            edit: true,
            trapsig: false,
            soft_tab: true,
            lit_echo: false,
            ack: false,
        };
        assert_eq!(flags.to_byte(), 0b00000101);
    }

    #[test]
    fn test_linemode_flags_roundtrip() {
        for byte in 0..=0b00011111 {
            let flags = LineModeFlags::from_byte(byte);
            assert_eq!(flags.to_byte(), byte);
        }
    }

    #[test]
    fn test_slc_function_from_byte() {
        assert_eq!(SlcFunction::from_byte(1), SlcFunction::Synch);
        assert_eq!(SlcFunction::from_byte(3), SlcFunction::Ip);
        assert_eq!(SlcFunction::from_byte(17), SlcFunction::ForwardChar);
        assert_eq!(SlcFunction::from_byte(255), SlcFunction::Other(255));
    }

    #[test]
    fn test_slc_function_to_byte() {
        assert_eq!(SlcFunction::Synch.to_byte(), 1);
        assert_eq!(SlcFunction::Ip.to_byte(), 3);
        assert_eq!(SlcFunction::ForwardChar.to_byte(), 17);
        assert_eq!(SlcFunction::Other(255).to_byte(), 255);
    }

    #[test]
    fn test_slc_function_roundtrip() {
        for byte in 0..=255 {
            let function = SlcFunction::from_byte(byte);
            assert_eq!(function.to_byte(), byte);
        }
    }

    #[test]
    fn test_slc_level_from_byte() {
        assert_eq!(SlcLevel::from_byte(0), SlcLevel::NoSupport);
        assert_eq!(SlcLevel::from_byte(1), SlcLevel::Default);
        assert_eq!(SlcLevel::from_byte(2), SlcLevel::Value);
        assert_eq!(SlcLevel::from_byte(3), SlcLevel::CantChange);

        // Test masking - only lower 2 bits matter
        assert_eq!(SlcLevel::from_byte(0b11111100), SlcLevel::NoSupport);
        assert_eq!(SlcLevel::from_byte(0b11111101), SlcLevel::Default);
    }

    #[test]
    fn test_slc_level_to_byte() {
        assert_eq!(SlcLevel::NoSupport.to_byte(), 0);
        assert_eq!(SlcLevel::Default.to_byte(), 1);
        assert_eq!(SlcLevel::Value.to_byte(), 2);
        assert_eq!(SlcLevel::CantChange.to_byte(), 3);
    }

    #[test]
    fn test_slc_flags_from_byte() {
        let flags = SlcFlags::from_byte(0b11100010);
        assert_eq!(flags.level, SlcLevel::Value);
        assert!(flags.ack);
        assert!(flags.flushin);
        assert!(flags.flushout);

        let flags = SlcFlags::from_byte(0b00000001);
        assert_eq!(flags.level, SlcLevel::Default);
        assert!(!flags.ack);
        assert!(!flags.flushin);
        assert!(!flags.flushout);

        let flags = SlcFlags::from_byte(0b10000000);
        assert_eq!(flags.level, SlcLevel::NoSupport);
        assert!(flags.ack);
        assert!(!flags.flushin);
        assert!(!flags.flushout);
    }

    #[test]
    fn test_slc_flags_to_byte() {
        let flags = SlcFlags {
            level: SlcLevel::Value,
            ack: true,
            flushin: true,
            flushout: true,
        };
        assert_eq!(flags.to_byte(), 0b11100010);

        let flags = SlcFlags {
            level: SlcLevel::Default,
            ack: false,
            flushin: false,
            flushout: false,
        };
        assert_eq!(flags.to_byte(), 0b00000001);

        let flags = SlcFlags {
            level: SlcLevel::NoSupport,
            ack: true,
            flushin: false,
            flushout: false,
        };
        assert_eq!(flags.to_byte(), 0b10000000);
    }

    #[test]
    fn test_slc_flags_roundtrip() {
        for byte in 0..=255 {
            let flags = SlcFlags::from_byte(byte);
            let result = flags.to_byte();
            // Only certain bits are preserved
            assert_eq!(result & 0b11100011, byte & 0b11100011);
        }
    }

    #[test]
    fn test_mode_parse() {
        let data = vec![consts::option::linemode::MODE, 0b00000111];
        let result = LineModeOption::parse(&data).unwrap();

        match result {
            LineModeOption::Mode { mode } => {
                assert!(mode.edit);
                assert!(mode.trapsig);
                assert!(mode.soft_tab);
                assert!(!mode.lit_echo);
                assert!(!mode.ack);
            }
            _ => panic!("Expected Mode variant"),
        }
    }

    #[test]
    fn test_mode_parse_insufficient_data() {
        let data = vec![consts::option::linemode::MODE];
        let result = LineModeOption::parse(&data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "MODE requires at least 1 byte of data");
    }

    #[test]
    fn test_mode_serialize() {
        let option = LineModeOption::Mode {
            mode: LineModeFlags {
                edit: true,
                trapsig: false,
                soft_tab: true,
                lit_echo: false,
                ack: true,
            },
        };

        let mut buf = Vec::new();
        option.serialize(&mut buf);

        assert_eq!(buf, vec![consts::option::linemode::MODE, 0b00010101]);
    }

    #[test]
    fn test_mode_roundtrip() {
        let original = LineModeOption::Mode {
            mode: LineModeFlags {
                edit: true,
                trapsig: true,
                soft_tab: false,
                lit_echo: true,
                ack: false,
            },
        };

        let mut buf = Vec::new();
        original.serialize(&mut buf);
        let parsed = LineModeOption::parse(&buf).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_forwardmask_parse() {
        let mut data = vec![consts::option::linemode::FORWARDMASK];
        // Add 32 bytes of mask data
        for i in 0..32 {
            data.push(if i % 2 == 0 { 0xFF } else { 0x00 });
        }

        let result = LineModeOption::parse(&data).unwrap();

        match result {
            LineModeOption::ForwardMask { mask } => {
                // Every even byte should have all bits set
                for i in 0..256 {
                    let byte_idx = i / 8;
                    if byte_idx % 2 == 0 {
                        assert!(mask[i], "Expected mask[{}] to be true", i);
                    } else {
                        assert!(!mask[i], "Expected mask[{}] to be false", i);
                    }
                }
            }
            _ => panic!("Expected ForwardMask variant"),
        }
    }

    #[test]
    fn test_forwardmask_parse_insufficient_data() {
        let data = vec![consts::option::linemode::FORWARDMASK, 0xFF];
        let result = LineModeOption::parse(&data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "FORWARDMASK requires 32 bytes of data");
    }

    #[test]
    fn test_forwardmask_serialize() {
        let mut mask = [false; 256];
        // Set every 16th character
        for i in (0..256).step_by(16) {
            mask[i] = true;
        }

        let option = LineModeOption::ForwardMask { mask };

        let mut buf = Vec::new();
        option.serialize(&mut buf);

        assert_eq!(buf.len(), 33); // 1 command byte + 32 mask bytes
        assert_eq!(buf[0], consts::option::linemode::FORWARDMASK);

        // Check the mask bytes
        for byte_idx in 0..32 {
            if byte_idx % 2 == 0 {
                assert_eq!(buf[1 + byte_idx], 0b10000000);
            } else {
                assert_eq!(buf[1 + byte_idx], 0b00000000);
            }
        }
    }

    #[test]
    fn test_forwardmask_roundtrip() {
        let mut mask = [false; 256];
        // Set specific characters: newline, carriage return, EOF
        mask[10] = true; // LF
        mask[13] = true; // CR
        mask[4] = true; // EOF

        let original = LineModeOption::ForwardMask { mask };

        let mut buf = Vec::new();
        original.serialize(&mut buf);
        let parsed = LineModeOption::parse(&buf).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_slc_parse() {
        let data = vec![
            consts::option::linemode::SLC,
            3,
            0b10000010,
            3, // IP function
            6,
            0b01000001,
            127, // EC function
        ];

        let result = LineModeOption::parse(&data).unwrap();

        match result {
            LineModeOption::Slc { chars } => {
                assert_eq!(chars.len(), 2);

                assert_eq!(chars[0].function, SlcFunction::Ip);
                assert_eq!(chars[0].flags.level, SlcLevel::Value);
                assert!(chars[0].flags.ack);
                assert_eq!(chars[0].value, 3);

                assert_eq!(chars[1].function, SlcFunction::Ec);
                assert_eq!(chars[1].flags.level, SlcLevel::Default);
                assert!(chars[1].flags.flushin);
                assert_eq!(chars[1].value, 127);
            }
            _ => panic!("Expected Slc variant"),
        }
    }

    #[test]
    fn test_slc_parse_invalid_triplets() {
        let data = vec![
            consts::option::linemode::SLC,
            3,
            0b10000010, // Missing value byte
        ];

        let result = LineModeOption::parse(&data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "SLC data must be in triplets");
    }

    #[test]
    fn test_slc_serialize() {
        let option = LineModeOption::Slc {
            chars: vec![
                SlcDefinition {
                    function: SlcFunction::Ip,
                    flags: SlcFlags {
                        level: SlcLevel::Value,
                        ack: true,
                        flushin: false,
                        flushout: false,
                    },
                    value: 3,
                },
                SlcDefinition {
                    function: SlcFunction::Ec,
                    flags: SlcFlags {
                        level: SlcLevel::Default,
                        ack: false,
                        flushin: true,
                        flushout: false,
                    },
                    value: 127,
                },
            ],
        };

        let mut buf = Vec::new();
        option.serialize(&mut buf);

        assert_eq!(
            buf,
            vec![
                consts::option::linemode::SLC,
                3,
                0b10000010,
                3,
                6,
                0b01000001,
                127,
            ]
        );
    }

    #[test]
    fn test_slc_roundtrip() {
        let original = LineModeOption::Slc {
            chars: vec![
                SlcDefinition {
                    function: SlcFunction::Synch,
                    flags: SlcFlags {
                        level: SlcLevel::Default,
                        ack: false,
                        flushin: false,
                        flushout: false,
                    },
                    value: 0,
                },
                SlcDefinition {
                    function: SlcFunction::Brk,
                    flags: SlcFlags {
                        level: SlcLevel::Value,
                        ack: true,
                        flushin: true,
                        flushout: true,
                    },
                    value: 255,
                },
            ],
        };

        let mut buf = Vec::new();
        original.serialize(&mut buf);
        let parsed = LineModeOption::parse(&buf).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_slc_empty() {
        let data = vec![consts::option::linemode::SLC];
        let result = LineModeOption::parse(&data).unwrap();

        match result {
            LineModeOption::Slc { chars } => {
                assert_eq!(chars.len(), 0);
            }
            _ => panic!("Expected Slc variant"),
        }
    }

    #[test]
    fn test_parse_empty_data() {
        let data = vec![];
        let result = LineModeOption::parse(&data);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Empty linemode subnegotiation");
    }

    #[test]
    fn test_parse_unknown_command() {
        let data = vec![255, 1, 2, 3];
        let result = LineModeOption::parse(&data);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Unknown linemode subnegotiation command"
        );
    }

    #[test]
    fn test_all_slc_functions() {
        let functions = vec![
            (1, SlcFunction::Synch),
            (2, SlcFunction::Brk),
            (3, SlcFunction::Ip),
            (4, SlcFunction::Ao),
            (5, SlcFunction::Ayt),
            (6, SlcFunction::Ec),
            (7, SlcFunction::El),
            (8, SlcFunction::Eof),
            (9, SlcFunction::Susp),
            (10, SlcFunction::Abort),
            (11, SlcFunction::Eor),
            (12, SlcFunction::Lnext),
            (13, SlcFunction::Ew),
            (14, SlcFunction::Rp),
            (15, SlcFunction::Xon),
            (16, SlcFunction::Xoff),
            (17, SlcFunction::ForwardChar),
        ];

        for (byte, expected) in functions {
            assert_eq!(SlcFunction::from_byte(byte), expected);
            assert_eq!(expected.to_byte(), byte);
        }
    }

    #[test]
    fn test_complex_slc_scenario() {
        // Create a complex SLC with multiple character definitions
        let option = LineModeOption::Slc {
            chars: vec![
                SlcDefinition {
                    function: SlcFunction::Ip,
                    flags: SlcFlags {
                        level: SlcLevel::Value,
                        ack: true,
                        flushin: true,
                        flushout: true,
                    },
                    value: 3, // Ctrl-C
                },
                SlcDefinition {
                    function: SlcFunction::Ao,
                    flags: SlcFlags {
                        level: SlcLevel::Value,
                        ack: false,
                        flushin: false,
                        flushout: true,
                    },
                    value: 15, // Ctrl-O
                },
                SlcDefinition {
                    function: SlcFunction::Ec,
                    flags: SlcFlags {
                        level: SlcLevel::Value,
                        ack: false,
                        flushin: false,
                        flushout: false,
                    },
                    value: 127, // DEL
                },
                SlcDefinition {
                    function: SlcFunction::El,
                    flags: SlcFlags {
                        level: SlcLevel::Value,
                        ack: false,
                        flushin: false,
                        flushout: false,
                    },
                    value: 21, // Ctrl-U
                },
            ],
        };

        let mut buf = Vec::new();
        option.serialize(&mut buf);
        let parsed = LineModeOption::parse(&buf).unwrap();

        assert_eq!(option, parsed);
    }

    #[test]
    fn test_forwardmask_all_set() {
        let mask = [true; 256];
        let option = LineModeOption::ForwardMask { mask };

        let mut buf = Vec::new();
        option.serialize(&mut buf);

        // All bytes should be 0xFF
        for i in 1..33 {
            assert_eq!(buf[i], 0xFF);
        }

        let parsed = LineModeOption::parse(&buf).unwrap();
        assert_eq!(option, parsed);
    }

    #[test]
    fn test_forwardmask_none_set() {
        let mask = [false; 256];
        let option = LineModeOption::ForwardMask { mask };

        let mut buf = Vec::new();
        option.serialize(&mut buf);

        // All bytes should be 0x00
        for i in 1..33 {
            assert_eq!(buf[i], 0x00);
        }

        let parsed = LineModeOption::parse(&buf).unwrap();
        assert_eq!(option, parsed);
    }
}
