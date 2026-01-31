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

use crate::ansi::{
    AnsiApplicationProgramCommand, AnsiControlCode, AnsiControlSequenceIntroducer,
    AnsiDeviceControlString, AnsiOperatingSystemCommand, AnsiPrivacyMessage, AnsiSequence,
    AnsiStartOfString, EraseInDisplayMode, EraseInLineMode,
};
use crate::consts::MAX_SEQUENCE_LENGTH;
use crate::style::AnsiSelectGraphicRendition;
use crate::{AnsiError, AnsiResult};

/// Internal state machine states for the ANSI mapper parser.
///
/// The `State` enum represents the current parsing state of the `AnsiMapper` as it
/// processes a byte stream. The mapper transitions between states based on the input
/// bytes, accumulating data until a complete sequence can be returned.
#[derive(Clone, Debug, PartialEq, Eq)]
enum State {
    /// Normal text processing state.
    ///
    /// In this state, the mapper processes:
    /// - ASCII printable characters (0x20-0x7E) → returned as `Character`
    /// - Control codes (0x00-0x1F, 0x7F, 0x80-0x9F) → returned as `Control`
    /// - ESC (0x1B) → transitions to `Escape` state
    /// - UTF-8 start bytes (0xC0-0xF7) → transitions to `UTF8` state
    Normal,

    /// Inside an escape sequence, waiting for the next byte.
    ///
    /// After receiving ESC (0x1B), the next byte determines the sequence type:
    /// - '[' → CSI sequence (transitions to `CSI` state)
    /// - ']' → OSC sequence (transitions to `OSC` state)
    /// - 'P' → DCS sequence (transitions to `DCS` state)
    /// - 'X' → SOS sequence (transitions to `SOS` state)
    /// - '^' → PM sequence (transitions to `PM` state)
    /// - '_' → APC sequence (transitions to `APC` state)
    /// - '\\' → ST (String Terminator)
    /// - Other → standalone ESC
    Escape,

    /// Inside a CSI (Control Sequence Introducer) sequence.
    ///
    /// Format: `ESC [ <params> <final_byte>`
    ///
    /// Accumulates parameter bytes (0x20-0x3F) and intermediate bytes (0x20-0x2F)
    /// until a final byte (0x40-0x7E) is received. The final byte determines the
    /// specific CSI command.
    CSI,

    /// Inside an OSC (Operating System Command) sequence.
    ///
    /// Format: `ESC ] <data> ST` or `ESC ] <data> BEL`
    ///
    /// Accumulates bytes until terminated by either:
    /// - BEL (0x07)
    /// - ST (ESC \\)
    OSC,

    /// Inside a DCS (Device Control String) sequence.
    ///
    /// Format: `ESC P <data> ST`
    ///
    /// Accumulates bytes until terminated by ST (ESC \\).
    DCS,

    /// Inside a SOS (Start of String) sequence.
    ///
    /// Format: `ESC X <data> ST`
    ///
    /// Accumulates bytes until terminated by ST (ESC \\).
    SOS,

    /// Inside a PM (Privacy Message) sequence.
    ///
    /// Format: `ESC ^ <data> ST`
    ///
    /// Accumulates bytes until terminated by ST (ESC \\).
    PM,

    /// Inside an APC (Application Program Command) sequence.
    ///
    /// Format: `ESC _ <data> ST`
    ///
    /// Accumulates bytes until terminated by ST (ESC \\).
    APC,

    /// Decoding a multi-byte UTF-8 character.
    ///
    /// Tracks the number of continuation bytes still expected and the accumulated
    /// code point value. UTF-8 sequences can be 2-4 bytes long.
    UTF8 {
        /// Number of continuation bytes still expected (1-3)
        expected: usize,
        /// Accumulated Unicode code point value
        accumulated: u32,
    },
}

/// A stateful parser for ANSI escape sequences and terminal input.
///
/// `AnsiMapper` processes a byte stream incrementally, recognizing and parsing:
/// - ASCII and UTF-8 text characters
/// - ANSI escape sequences (CSI, OSC, DCS, etc.)
/// - Control codes (C0 and C1)
/// - Multi-byte UTF-8 characters
///
/// The parser operates as a state machine, maintaining internal state between calls
/// to handle incomplete sequences. This allows it to process streaming input where
/// escape sequences may arrive across multiple buffer reads.
pub struct AnsiParser {
    /// Internal buffer for accumulating bytes of escape sequences.
    ///
    /// This buffer stores the parameters and data of multi-byte sequences like CSI,
    /// OSC, DCS, etc. It is cleared when returning to normal text parsing or when
    /// a sequence completes. For single-byte results (ASCII characters, control codes),
    /// this buffer remains empty.
    bytes: Vec<u8>,

    /// The current state of the parser state machine.
    ///
    /// Tracks what kind of input is currently being processed (normal text, inside
    /// an escape sequence, decoding UTF-8, etc.). The state determines how the next
    /// byte will be interpreted.
    state: State,
}

impl AnsiParser {
    /// Creates a new ANSI mapper in its initial state.
    ///
    /// The mapper starts in `State::Normal`, ready to process text and escape sequences.
    /// The internal buffer is empty and will only allocate when needed for multi-byte
    /// sequences.
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            state: State::Normal,
        }
    }

    /// Resets the ANSI mapper to its initial state, clearing all accumulated data.
    ///
    /// This method discards any partially parsed sequences, UTF-8 characters, or accumulated
    /// bytes in the internal buffer, and returns the mapper to the `Normal` state.
    pub fn clear(&mut self) {
        self.bytes.clear();
        self.state = State::Normal;
    }

    /// Process the next byte and return a result.
    ///
    /// This is the main entry point for feeding bytes into the mapper. Each byte is
    /// processed according to the current internal state, potentially causing state
    /// transitions and accumulating data for incomplete sequences.
    ///
    /// # Arguments
    ///
    /// * `byte` - The next byte from the input stream to process
    ///
    /// # Returns
    ///
    /// An `Option<AnsiSequence>` which may be:
    /// - `None` - More bytes needed to complete the current sequence
    /// - `Some(sequence)` - A complete sequence has been parsed
    pub fn next(&mut self, byte: u8) -> AnsiResult<Option<AnsiSequence>> {
        // Check buffer size before processing
        if self.bytes.len() >= MAX_SEQUENCE_LENGTH {
            // Buffer overflow - reset and return error
            let error = AnsiError::SequenceTooLong {
                length: self.bytes.len(),
                max: MAX_SEQUENCE_LENGTH,
            };
            self.clear();
            return Err(error);
        }

        let result = match self.state {
            State::Normal => self.process_normal(byte),
            State::Escape => self.process_escape(byte),
            State::CSI => self.process_csi(byte),
            State::OSC => self.process_osc(byte),
            State::DCS => self.process_dcs(byte),
            State::SOS => self.process_sos(byte),
            State::PM => self.process_pm(byte),
            State::APC => self.process_apc(byte),
            State::UTF8 {
                expected,
                accumulated,
            } => self.process_utf8(byte, expected, accumulated),
        };

        Ok(result)
    }

    fn process_normal(&mut self, byte: u8) -> Option<AnsiSequence> {
        match byte {
            // ESC character
            0x1B => {
                self.state = State::Escape;
                self.bytes.clear();
                None
            }
            // ASCII control characters (excluding ESC)
            0x00..=0x1F | 0x7F => {
                if let Some(control) = AnsiControlCode::from_byte(byte) {
                    Some(AnsiSequence::Control(control))
                } else {
                    Some(AnsiSequence::Character(byte as char))
                }
            }
            // C1 control characters (0x80-0x9F)
            0x80..=0x9F => {
                if let Some(control) = AnsiControlCode::from_byte(byte) {
                    Some(AnsiSequence::Control(control))
                } else {
                    Some(AnsiSequence::Character(byte as char))
                }
            }
            // ASCII printable characters
            0x20..=0x7E => Some(AnsiSequence::Character(byte as char)),
            // UTF-8 multibyte sequences
            0xC0..=0xDF => {
                // 2-byte sequence
                self.state = State::UTF8 {
                    expected: 1,
                    accumulated: (byte as u32 & 0x1F) << 6,
                };
                None
            }
            0xE0..=0xEF => {
                // 3-byte sequence
                self.state = State::UTF8 {
                    expected: 2,
                    accumulated: (byte as u32 & 0x0F) << 12,
                };
                None
            }
            0xF0..=0xF7 => {
                // 4-byte sequence
                self.state = State::UTF8 {
                    expected: 3,
                    accumulated: (byte as u32 & 0x07) << 18,
                };
                None
            }
            _ => Some(AnsiSequence::Character(byte as char)),
        }
    }

    fn process_escape(&mut self, byte: u8) -> Option<AnsiSequence> {
        self.state = State::Normal;

        match byte {
            b'[' => {
                // CSI - Control Sequence Introducer
                self.state = State::CSI;
                self.bytes.clear();
                None
            }
            b']' => {
                // OSC - Operating System Command
                self.state = State::OSC;
                self.bytes.clear();
                None
            }
            b'P' => {
                // DCS - Device Control String
                self.state = State::DCS;
                self.bytes.clear();
                None
            }
            b'X' => {
                // SOS - Start of String
                self.state = State::SOS;
                self.bytes.clear();
                None
            }
            b'^' => {
                // PM - Privacy Message
                self.state = State::PM;
                self.bytes.clear();
                None
            }
            b'_' => {
                // APC - Application Program Command
                self.state = State::APC;
                self.bytes.clear();
                None
            }
            b'\\' => {
                // ST - String Terminator
                Some(AnsiSequence::AnsiST)
            }
            _ => {
                // Standalone ESC or unknown sequence
                Some(AnsiSequence::AnsiEscape)
            }
        }
    }

    fn process_csi(&mut self, byte: u8) -> Option<AnsiSequence> {
        self.bytes.push(byte);

        // CSI sequences end with a letter (0x40-0x7E)
        if (0x40..=0x7E).contains(&byte) {
            self.state = State::Normal;

            // Check if it's an SGR sequence (ends with 'm')
            if byte == b'm' {
                // Parse SGR codes
                if let Some(sgr) = self.parse_sgr() {
                    return Some(AnsiSequence::AnsiSGR(sgr));
                }
            }

            // Parse as general CSI command
            let command = self.parse_csi();
            return Some(AnsiSequence::AnsiCSI(command));
        }

        None
    }

    fn process_osc(&mut self, byte: u8) -> Option<AnsiSequence> {
        // OSC sequences end with BEL (0x07) or ST (ESC \)
        if byte == 0x07 {
            self.state = State::Normal;
            let data = std::mem::take(&mut self.bytes);
            return Some(AnsiSequence::AnsiOSC(AnsiOperatingSystemCommand::Unknown(
                data,
            )));
        }

        if byte == 0x1B {
            // Could be start of ST sequence
            self.bytes.push(byte);
            return None;
        }

        if !self.bytes.is_empty() && self.bytes[self.bytes.len() - 1] == 0x1B && byte == b'\\' {
            // ST sequence found
            self.state = State::Normal;
            let mut data = std::mem::take(&mut self.bytes);
            data.pop(); // Remove ESC
            return Some(AnsiSequence::AnsiOSC(AnsiOperatingSystemCommand::Unknown(
                data,
            )));
        }

        self.bytes.push(byte);
        None
    }

    fn process_dcs(&mut self, byte: u8) -> Option<AnsiSequence> {
        self.process_string_sequence(byte, |data| {
            AnsiSequence::AnsiDCS(AnsiDeviceControlString::Unknown(data))
        })
    }

    fn process_sos(&mut self, byte: u8) -> Option<AnsiSequence> {
        self.process_string_sequence(byte, |data| {
            AnsiSequence::AnsiSOS(AnsiStartOfString::Unknown(data))
        })
    }

    fn process_pm(&mut self, byte: u8) -> Option<AnsiSequence> {
        self.process_string_sequence(byte, |data| {
            AnsiSequence::AnsiPM(AnsiPrivacyMessage::Unknown(data))
        })
    }

    fn process_apc(&mut self, byte: u8) -> Option<AnsiSequence> {
        self.process_string_sequence(byte, |data| {
            AnsiSequence::AnsiAPC(AnsiApplicationProgramCommand::Unknown(data))
        })
    }

    fn process_string_sequence<F>(&mut self, byte: u8, constructor: F) -> Option<AnsiSequence>
    where
        F: FnOnce(Vec<u8>) -> AnsiSequence,
    {
        // String sequences end with ST (ESC \)
        if byte == 0x1B {
            self.bytes.push(byte);
            return None;
        }

        if !self.bytes.is_empty() && self.bytes[self.bytes.len() - 1] == 0x1B && byte == b'\\' {
            // ST sequence found
            self.state = State::Normal;
            let mut data = std::mem::take(&mut self.bytes);
            data.pop(); // Remove ESC
            return Some(constructor(data));
        }

        self.bytes.push(byte);
        None
    }

    fn process_utf8(
        &mut self,
        byte: u8,
        expected: usize,
        accumulated: u32,
    ) -> Option<AnsiSequence> {
        // UTF-8 continuation bytes are 10xxxxxx
        if (byte & 0xC0) != 0x80 {
            // Invalid continuation byte
            self.state = State::Normal;
            return Some(AnsiSequence::Character('\u{FFFD}')); // Replacement character
        }

        let accumulated = accumulated | ((byte as u32 & 0x3F) << ((expected - 1) * 6));

        if expected == 1 {
            // Last byte
            self.state = State::Normal;
            if let Some(ch) = char::from_u32(accumulated) {
                Some(AnsiSequence::Unicode(ch))
            } else {
                Some(AnsiSequence::Character('\u{FFFD}'))
            }
        } else {
            // More bytes expected
            self.state = State::UTF8 {
                expected: expected - 1,
                accumulated,
            };
            None
        }
    }

    fn parse_sgr(&self) -> Option<AnsiSelectGraphicRendition> {
        // Extract the parameter string (remove the 'm' terminator at the end)
        let params_str =
            std::str::from_utf8(&self.bytes[..self.bytes.len().saturating_sub(1)]).ok()?;

        // Parse the semicolon-separated numeric codes
        let mut codes = Vec::new();

        if params_str.is_empty() {
            // Empty SGR sequence defaults to code 0 (reset)
            codes.push(0u8);
        } else {
            for code_str in params_str.split(';') {
                // Handle empty segments (e.g., "1;;31" should treat empty as 0)
                if code_str.is_empty() {
                    codes.push(0u8);
                } else {
                    // Parse the numeric code, limit to u8 range (0-255)
                    if let Ok(code) = code_str.parse::<u32>() {
                        // SGR codes can be larger than u8 for extended colors (38;5;n, 48;5;n, etc)
                        // but we store as individual bytes in the sequence
                        if code <= 255 {
                            codes.push(code as u8);
                        } else {
                            // For codes > 255, we still include them but as multiple bytes
                            codes.push((code & 0xFF) as u8);
                        }
                    } else {
                        // Invalid number, skip this parameter
                        continue;
                    }
                }
            }
        }

        // Use the new API to parse SGR parameters
        Some(AnsiSelectGraphicRendition::parse(&codes))
    }

    fn parse_csi(&self) -> AnsiControlSequenceIntroducer {
        if self.bytes.is_empty() {
            return AnsiControlSequenceIntroducer::Unknown;
        }

        // Get the final byte (command letter)
        let final_byte = self.bytes[self.bytes.len() - 1];

        // Parse parameters (everything except the final byte)
        let params_slice = &self.bytes[..self.bytes.len() - 1];
        let params_str = std::str::from_utf8(params_slice).unwrap_or("");

        // Parse numeric parameters
        let params: Vec<u8> = if params_str.is_empty() {
            vec![]
        } else {
            params_str
                .split(';')
                .filter_map(|s| s.parse::<u8>().ok())
                .collect()
        };

        // Get first parameter with default of 1 for most commands
        let n = params.first().copied().unwrap_or(1);

        match final_byte {
            b'A' => AnsiControlSequenceIntroducer::CursorUp(n),
            b'B' => AnsiControlSequenceIntroducer::CursorDown(n),
            b'C' => AnsiControlSequenceIntroducer::CursorForward(n),
            b'D' => AnsiControlSequenceIntroducer::CursorBack(n),
            b'E' => AnsiControlSequenceIntroducer::CursorNextLine(n),
            b'F' => AnsiControlSequenceIntroducer::CursorPreviousLine(n),
            b'G' => AnsiControlSequenceIntroducer::CursorHorizontalAbsolute(n),
            b'H' | b'f' => {
                // Cursor Position - ESC[row;colH or ESC[row;colf
                let row = params.get(0).copied().unwrap_or(1);
                let col = params.get(1).copied().unwrap_or(1);
                AnsiControlSequenceIntroducer::CursorPosition { row, col }
            }
            b'J' => {
                // Erase in Display - default is 0, not 1
                let mode_param = params.get(0).copied().unwrap_or(0);
                let mode = match mode_param {
                    0 => EraseInDisplayMode::EraseToEndOfScreen,
                    1 => EraseInDisplayMode::EraseToBeginningOfScreen,
                    2 => EraseInDisplayMode::EraseEntireScreen,
                    3 => EraseInDisplayMode::EraseEntireScreenAndSavedLines,
                    _ => EraseInDisplayMode::EraseToEndOfScreen,
                };
                AnsiControlSequenceIntroducer::EraseInDisplay(mode)
            }
            b'K' => {
                // Erase in Line - default is 0, not 1
                let mode_param = params.get(0).copied().unwrap_or(0);
                let mode = match mode_param {
                    0 => EraseInLineMode::EraseToEndOfLine,
                    1 => EraseInLineMode::EraseToStartOfLine,
                    2 => EraseInLineMode::EraseEntireLine,
                    _ => EraseInLineMode::EraseToEndOfLine,
                };
                AnsiControlSequenceIntroducer::EraseInLine(mode)
            }
            b'S' => AnsiControlSequenceIntroducer::ScrollUp,
            b'T' => AnsiControlSequenceIntroducer::ScrollDown,
            b'@' => AnsiControlSequenceIntroducer::InsertCharacter,
            b'P' => AnsiControlSequenceIntroducer::DeleteCharacter,
            b'L' => AnsiControlSequenceIntroducer::InsertLine,
            b'M' => AnsiControlSequenceIntroducer::DeleteLine,
            b'X' => AnsiControlSequenceIntroducer::EraseCharacter,
            b's' => AnsiControlSequenceIntroducer::SaveCursorPosition,
            b'u' => AnsiControlSequenceIntroducer::RestoreCursorPosition,
            b'n' => {
                if params_str == "6" {
                    AnsiControlSequenceIntroducer::DeviceStatusReport
                } else {
                    AnsiControlSequenceIntroducer::Unknown
                }
            }
            b'h' => {
                if params_str.starts_with('?') {
                    AnsiControlSequenceIntroducer::DECPrivateModeSet
                } else {
                    AnsiControlSequenceIntroducer::SetMode
                }
            }
            b'l' => {
                if params_str.starts_with('?') {
                    AnsiControlSequenceIntroducer::DECPrivateModeReset
                } else {
                    AnsiControlSequenceIntroducer::ResetMode
                }
            }
            _ => AnsiControlSequenceIntroducer::Unknown,
        }
    }
}

impl Default for AnsiParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to parse a complete byte sequence
    fn parse_bytes(bytes: &[u8]) -> Vec<AnsiSequence> {
        let mut parser = AnsiParser::new();
        let mut results = Vec::new();

        for &byte in bytes {
            if let Ok(Some(seq)) = parser.next(byte) {
                results.push(seq);
            }
        }

        results
    }

    #[test]
    fn test_ascii_characters() {
        let input = b"Hello";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 5);
        assert_eq!(results[0], AnsiSequence::Character('H'));
        assert_eq!(results[1], AnsiSequence::Character('e'));
        assert_eq!(results[2], AnsiSequence::Character('l'));
        assert_eq!(results[3], AnsiSequence::Character('l'));
        assert_eq!(results[4], AnsiSequence::Character('o'));
    }

    #[test]
    fn test_control_codes() {
        let mut parser = AnsiParser::new();

        // Test Bell (0x07)
        let result = parser.next(0x07).unwrap();
        assert_eq!(result, Some(AnsiSequence::Control(AnsiControlCode::BEL)));

        // Test Line Feed (0x0A)
        let result = parser.next(0x0A).unwrap();
        assert_eq!(result, Some(AnsiSequence::Control(AnsiControlCode::LF)));

        // Test Carriage Return (0x0D)
        let result = parser.next(0x0D).unwrap();
        assert_eq!(result, Some(AnsiSequence::Control(AnsiControlCode::CR)));

        // Test Tab (0x09)
        let result = parser.next(0x09).unwrap();
        assert_eq!(result, Some(AnsiSequence::Control(AnsiControlCode::HT)));
    }

    #[test]
    fn test_utf8_two_byte() {
        let mut parser = AnsiParser::new();

        // UTF-8 for '©' (U+00A9): 0xC2 0xA9
        let result = parser.next(0xC2).unwrap();
        assert_eq!(result, None); // First byte, waiting for continuation

        let result = parser.next(0xA9).unwrap();
        assert_eq!(result, Some(AnsiSequence::Unicode('©')));
    }

    #[test]
    fn test_utf8_three_byte() {
        let mut parser = AnsiParser::new();

        // UTF-8 for '€' (U+20AC): 0xE2 0x82 0xAC
        let result = parser.next(0xE2).unwrap();
        assert_eq!(result, None);

        let result = parser.next(0x82).unwrap();
        assert_eq!(result, None);

        let result = parser.next(0xAC).unwrap();
        assert_eq!(result, Some(AnsiSequence::Unicode('€')));
    }

    #[test]
    fn test_utf8_four_byte() {
        let mut parser = AnsiParser::new();

        // UTF-8 for '𝄞' (U+1D11E): 0xF0 0x9D 0x84 0x9E
        let result = parser.next(0xF0).unwrap();
        assert_eq!(result, None);

        let result = parser.next(0x9D).unwrap();
        assert_eq!(result, None);

        let result = parser.next(0x84).unwrap();
        assert_eq!(result, None);

        let result = parser.next(0x9E).unwrap();
        assert_eq!(result, Some(AnsiSequence::Unicode('𝄞')));
    }

    #[test]
    fn test_csi_cursor_up() {
        // ESC[5A - Cursor Up 5 lines
        let input = b"\x1b[5A";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::CursorUp(5))
        );
    }

    #[test]
    fn test_csi_cursor_position() {
        // ESC[10;20H - Move cursor to row 10, column 20
        let input = b"\x1b[10;20H";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::CursorPosition {
                row: 10,
                col: 20
            })
        );
    }

    #[test]
    fn test_csi_erase_in_display() {
        // ESC[2J - Clear entire screen
        let input = b"\x1b[2J";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::EraseInDisplay(
                EraseInDisplayMode::EraseEntireScreen
            ))
        );
    }

    #[test]
    fn test_csi_erase_in_line() {
        // ESC[K - Erase to end of line (default mode 0)
        let input = b"\x1b[K";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::EraseInLine(
                EraseInLineMode::EraseToEndOfLine
            ))
        );
    }

    #[test]
    fn test_osc_with_bel_terminator() {
        // ESC]0;Window Title\x07 - Set window title (BEL terminated)
        let input = b"\x1b]0;Window Title\x07";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        match &results[0] {
            AnsiSequence::AnsiOSC(osc) => {
                assert!(matches!(osc, AnsiOperatingSystemCommand::Unknown(_)));
            }
            _ => panic!("Expected OSC sequence"),
        }
    }

    #[test]
    fn test_osc_with_st_terminator() {
        // ESC]0;Window Title ESC\ - Set window title (ST terminated)
        let input = b"\x1b]0;Window Title\x1b\\";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        match &results[0] {
            AnsiSequence::AnsiOSC(osc) => {
                assert!(matches!(osc, AnsiOperatingSystemCommand::Unknown(_)));
            }
            _ => panic!("Expected OSC sequence"),
        }
    }

    #[test]
    fn test_dcs_sequence() {
        // ESC P <data> ESC\ - Device Control String
        let input = b"\x1bPtest data\x1b\\";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        match &results[0] {
            AnsiSequence::AnsiDCS(dcs) => {
                assert!(matches!(dcs, AnsiDeviceControlString::Unknown(_)));
            }
            _ => panic!("Expected DCS sequence"),
        }
    }

    #[test]
    fn test_standalone_escape() {
        // ESC followed by an unrecognized character
        let _input = b"\x1bX";
        let mut parser = AnsiParser::new();

        parser.next(0x1B).unwrap(); // ESC
        let result = parser.next(b'X').unwrap();

        // 'X' starts SOS, so we shouldn't get a standalone escape
        assert_eq!(result, None);
    }

    #[test]
    fn test_string_terminator() {
        // ESC\ - String Terminator
        let input = b"\x1b\\";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0], AnsiSequence::AnsiST);
    }

    #[test]
    fn test_mixed_content() {
        // Mix of text, control codes, and escape sequences
        let input = b"Hello\x1b[1mWorld\x1b[0m\n";
        let results = parse_bytes(input);

        // Should have: H, e, l, l, o, SGR(bold), W, o, r, l, d, SGR(reset), LF
        assert!(results.len() >= 11);
        assert_eq!(results[0], AnsiSequence::Character('H'));
        assert_eq!(results[4], AnsiSequence::Character('o'));
    }

    #[test]
    fn test_parser_reset() {
        let mut parser = AnsiParser::new();

        // Start parsing a sequence
        parser.next(0x1B).unwrap(); // ESC
        parser.next(b'[').unwrap(); // [

        // Reset the parser
        parser.clear();

        // Should be back to normal state
        let result = parser.next(b'A').unwrap();
        assert_eq!(result, Some(AnsiSequence::Character('A')));
    }

    #[test]
    fn test_sequence_too_long() {
        let mut parser = AnsiParser::new();

        // Start a CSI sequence
        parser.next(0x1B).unwrap(); // ESC
        parser.next(b'[').unwrap(); // [

        // Add more than MAX_SEQUENCE_LENGTH bytes
        for _ in 0..300 {
            let result = parser.next(b'1');
            if result.is_err() {
                // Should get an error for sequence too long
                assert!(matches!(result, Err(AnsiError::SequenceTooLong { .. })));
                return;
            }
        }

        panic!("Expected SequenceTooLong error");
    }

    #[test]
    fn test_csi_default_parameters() {
        // ESC[H - Cursor position with default parameters (1,1)
        let input = b"\x1b[H";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::CursorPosition { row: 1, col: 1 })
        );
    }

    #[test]
    fn test_csi_save_restore_cursor() {
        // ESC[s - Save cursor position
        let input = b"\x1b[s";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::SaveCursorPosition)
        );

        // ESC[u - Restore cursor position
        let input = b"\x1b[u";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::RestoreCursorPosition)
        );
    }

    #[test]
    fn test_csi_device_status_report() {
        // ESC[6n - Device Status Report
        let input = b"\x1b[6n";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::DeviceStatusReport)
        );
    }

    #[test]
    fn test_invalid_utf8() {
        let mut parser = AnsiParser::new();

        // Start a 2-byte UTF-8 sequence
        parser.next(0xC2).unwrap();

        // Send an invalid continuation byte
        let result = parser.next(0x20).unwrap(); // Space is not a valid continuation

        // Should get replacement character
        assert_eq!(result, Some(AnsiSequence::Character('\u{FFFD}')));
    }

    #[test]
    fn test_sos_sequence() {
        // ESC X <data> ESC\ - Start of String
        let input = b"\x1bXtest\x1b\\";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        match &results[0] {
            AnsiSequence::AnsiSOS(sos) => {
                assert!(matches!(sos, AnsiStartOfString::Unknown(_)));
            }
            _ => panic!("Expected SOS sequence"),
        }
    }

    #[test]
    fn test_pm_sequence() {
        // ESC ^ <data> ESC\ - Privacy Message
        let input = b"\x1b^private\x1b\\";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        match &results[0] {
            AnsiSequence::AnsiPM(pm) => {
                assert!(matches!(pm, AnsiPrivacyMessage::Unknown(_)));
            }
            _ => panic!("Expected PM sequence"),
        }
    }

    #[test]
    fn test_apc_sequence() {
        // ESC _ <data> ESC\ - Application Program Command
        let input = b"\x1b_command\x1b\\";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        match &results[0] {
            AnsiSequence::AnsiAPC(AnsiApplicationProgramCommand::Unknown(data)) => {
                assert_eq!(data, b"command");
            }
            _ => panic!("Expected APC sequence"),
        }
    }

    #[test]
    fn test_csi_scrolling() {
        // ESC[S - Scroll Up
        let input = b"\x1b[S";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::ScrollUp)
        );

        // ESC[T - Scroll Down
        let input = b"\x1b[T";
        let results = parse_bytes(input);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::ScrollDown)
        );
    }

    #[test]
    fn test_csi_insert_delete() {
        // ESC[@ - Insert Character
        let input = b"\x1b[@";
        let results = parse_bytes(input);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::InsertCharacter)
        );

        // ESC[P - Delete Character
        let input = b"\x1b[P";
        let results = parse_bytes(input);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::DeleteCharacter)
        );

        // ESC[L - Insert Line
        let input = b"\x1b[L";
        let results = parse_bytes(input);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::InsertLine)
        );

        // ESC[M - Delete Line
        let input = b"\x1b[M";
        let results = parse_bytes(input);
        assert_eq!(
            results[0],
            AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::DeleteLine)
        );
    }

    #[test]
    fn test_streaming_input() {
        let mut parser = AnsiParser::new();

        // Simulate streaming input where escape sequence arrives in chunks
        let chunk1 = b"\x1b";
        let chunk2 = b"[";
        let chunk3 = b"1";
        let chunk4 = b"0";
        let chunk5 = b"A";

        assert_eq!(parser.next(chunk1[0]).unwrap(), None);
        assert_eq!(parser.next(chunk2[0]).unwrap(), None);
        assert_eq!(parser.next(chunk3[0]).unwrap(), None);
        assert_eq!(parser.next(chunk4[0]).unwrap(), None);

        let result = parser.next(chunk5[0]).unwrap();
        assert_eq!(
            result,
            Some(AnsiSequence::AnsiCSI(
                AnsiControlSequenceIntroducer::CursorUp(10)
            ))
        );
    }

    #[test]
    fn test_c1_control_codes() {
        let mut parser = AnsiParser::new();

        // Test NEL (Next Line) - 0x85
        let result = parser.next(0x85).unwrap();
        assert_eq!(result, Some(AnsiSequence::Control(AnsiControlCode::NEL)));

        // Test RI (Reverse Index) - 0x8D
        let result = parser.next(0x8D).unwrap();
        assert_eq!(result, Some(AnsiSequence::Control(AnsiControlCode::RI)));
    }
}
