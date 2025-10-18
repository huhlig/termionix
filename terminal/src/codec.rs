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

use super::{TerminalBuffer, TerminalError, TerminalEvent};
use crate::command::TerminalCommand;
use crate::types::CursorPosition;
use termionix_ansicodes::{
    AnsiConfig, AnsiMapper, AnsiMapperResult, CSICommand, ControlCode, EraseInDisplayMode,
    EraseInLineMode, Segment, SegmentedString, Style,
};
use termionix_codec::{TelnetCodec, TelnetFrame};
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

/// Wraps a [`TelnetCodec`] and parses all Unicode and Ansi Escape Sequences.
pub struct TerminalCodec {
    buffer: TerminalBuffer,
    config: AnsiConfig,
    mapper: AnsiMapper,
    codec: TelnetCodec,
}

impl TerminalCodec {
    pub fn new() -> Self {
        Self::new_with_config(AnsiConfig::default())
    }

    pub fn new_with_config(config: AnsiConfig) -> Self {
        Self {
            config,
            buffer: Default::default(),
            mapper: AnsiMapper::default(),
            codec: TelnetCodec::default(),
        }
    }

    pub fn ansi_mapper(&self) -> &AnsiMapper {
        &self.mapper
    }
    pub fn ansi_mapper_mut(&mut self) -> &mut AnsiMapper {
        &mut self.mapper
    }

    pub fn telnet_codec(&self) -> &TelnetCodec {
        &self.codec
    }
    pub fn telnet_codec_mut(&mut self) -> &mut TelnetCodec {
        &mut self.codec
    }
    pub fn terminal_buffer(&self) -> &TerminalBuffer {
        &self.buffer
    }
    pub fn terminal_buffer_mut(&mut self) -> &mut TerminalBuffer {
        &mut self.buffer
    }
}

impl Decoder for TerminalCodec {
    type Item = TerminalEvent;
    type Error = TerminalError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.codec.decode(src)? {
            Some(frame) => match frame {
                TelnetFrame::Data(byte) => match self.mapper.next(byte) {
                    AnsiMapperResult::Incomplete => Ok(None),
                    AnsiMapperResult::Character(ch) => {
                        self.buffer.append_char(ch);
                        Ok(Some(TerminalEvent::CharacterData {
                            cursor: self.buffer.cursor_position(),
                            character: ch,
                        }))
                    }
                    AnsiMapperResult::Unicode(ch) => {
                        self.buffer.append_char(ch);
                        Ok(Some(TerminalEvent::CharacterData {
                            cursor: self.buffer.cursor_position(),
                            character: ch,
                        }))
                    }
                    AnsiMapperResult::Control(control) => match control {
                        ControlCode::NUL => Ok(None),                      // Null - ignore
                        ControlCode::SOH => Ok(None), // Start of Heading - ignore
                        ControlCode::STX => Ok(None), // Start of Text - ignore
                        ControlCode::ETX => Ok(None), // End of Text - ignore
                        ControlCode::EOT => Ok(None), // End of Transmission - ignore
                        ControlCode::ENQ => Ok(None), // Enquiry - ignore
                        ControlCode::ACK => Ok(None), // Acknowledge - ignore
                        ControlCode::BEL => Ok(Some(TerminalEvent::Bell)), // Bell
                        ControlCode::BS => {
                            // Backspace - move the cursor back and erase character
                            self.buffer.erase_character();
                            Ok(Some(TerminalEvent::EraseCharacter {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        ControlCode::HT => {
                            // Horizontal Tab - add tab character
                            self.buffer.append_char('\t');
                            Ok(Some(TerminalEvent::CharacterData {
                                cursor: self.buffer.cursor_position(),
                                character: '\t',
                            }))
                        }
                        ControlCode::LF => {
                            // Line Feed - complete the current line and advance
                            let completed_line = self.buffer.complete_line();
                            Ok(Some(TerminalEvent::LineCompleted {
                                cursor: self.buffer.cursor_position(),
                                line: completed_line,
                            }))
                        }
                        ControlCode::VT => {
                            // Vertical Tab - treat as line feed
                            let completed_line = self.buffer.complete_line();
                            Ok(Some(TerminalEvent::LineCompleted {
                                cursor: self.buffer.cursor_position(),
                                line: completed_line,
                            }))
                        }
                        ControlCode::FF => {
                            // Form Feed - clear screen
                            self.buffer.clear();
                            Ok(Some(TerminalEvent::Clear {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        ControlCode::CR => {
                            // Carriage Return - move cursor to start of line
                            let pos = self.buffer.cursor_position();
                            self.buffer.set_cursor_position(0, pos.row);
                            Ok(Some(TerminalEvent::CursorPosition {
                                cursor: CursorPosition::new(0, pos.row),
                            }))
                        }
                        ControlCode::SO => Ok(None), // Shift Out - ignore
                        ControlCode::SI => Ok(None), // Shift In - ignore
                        ControlCode::DLE => Ok(None), // Data Link Escape - ignore
                        ControlCode::DC1 => Ok(None), // Device Control 1 (XON) - ignore
                        ControlCode::DC2 => Ok(None), // Device Control 2 - ignore
                        ControlCode::DC3 => Ok(None), // Device Control 3 (XOFF) - ignore
                        ControlCode::DC4 => Ok(None), // Device Control 4 - ignore
                        ControlCode::NAK => Ok(None), // Negative Acknowledge - ignore
                        ControlCode::SYN => Ok(None), // Synchronous Idle - ignore
                        ControlCode::ETB => Ok(None), // End of Transmission Block - ignore
                        ControlCode::CAN => Ok(None), // Cancel - ignore
                        ControlCode::EM => Ok(None), // End of Medium - ignore
                        ControlCode::SUB => Ok(None), // Substitute - ignore
                        ControlCode::FS => Ok(None), // File Separator - ignore
                        ControlCode::GS => Ok(None), // Group Separator - ignore
                        ControlCode::RS => Ok(None), // Record Separator - ignore
                        ControlCode::US => Ok(None), // Unit Separator - ignore
                        ControlCode::DEL => {
                            // Delete - erase character at cursor
                            self.buffer.erase_character();
                            Ok(Some(TerminalEvent::EraseCharacter {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        ControlCode::PAD => Ok(None), // Padding Character - ignore
                        ControlCode::HOP => Ok(None), // High Octet Preset - ignore
                        ControlCode::BPH => Ok(None), // Break Permitted Here - ignore
                        ControlCode::NBH => Ok(None), // No Break Here - ignore
                        ControlCode::IND => {
                            // Index - move cursor down one line
                            Ok(Some(TerminalEvent::CursorPosition {
                                cursor: self.buffer.move_cursor(1, 0),
                            }))
                        }
                        ControlCode::NEL => {
                            // Next Line - CR + LF
                            self.buffer.complete_line();
                            Ok(Some(TerminalEvent::CursorPosition {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        ControlCode::SSA => Ok(None), // Start of Selected Area - ignore
                        ControlCode::ESA => Ok(None), // End of Selected Area - ignore
                        ControlCode::HTS => Ok(None), // Horizontal Tab Set - ignore
                        ControlCode::HTJ => Ok(None), // Horizontal Tab with Justification - ignore
                        ControlCode::VTS => Ok(None), // Vertical Tab Set - ignore
                        ControlCode::PLD => Ok(None), // Partial Line Down - ignore
                        ControlCode::PLU => Ok(None), // Partial Line Up - ignore
                        ControlCode::RI => {
                            // Reverse Index - move cursor up one line ;
                            Ok(Some(TerminalEvent::CursorPosition {
                                cursor: self.buffer.move_cursor(-1, 0),
                            }))
                        }
                        ControlCode::SS2 => Ok(None), // Single Shift 2 - ignore
                        ControlCode::SS3 => Ok(None), // Single Shift 3 - ignore
                        ControlCode::DCS => Ok(None), // Device Control String - handled by AnsiMapperResult::DCS
                        ControlCode::PU1 => Ok(None), // Private Use 1 - ignore
                        ControlCode::PU2 => Ok(None), // Private Use 2 - ignore
                        ControlCode::STS => Ok(None), // Set Transmit State - ignore
                        ControlCode::CCH => Ok(None), // Cancel Character - ignore
                        ControlCode::MW => Ok(None),  // Message Waiting - ignore
                        ControlCode::SPA => Ok(None), // Start of Protected Area - ignore
                        ControlCode::EPA => Ok(None), // End of Protected Area - ignore
                        ControlCode::SOS => Ok(None), // Start of String - handled by AnsiMapperResult::SOS
                        ControlCode::SGCI => Ok(None), // Single Graphic Character Introducer - ignore
                        ControlCode::SCI => Ok(None),  // Single Character Introducer - ignore
                        ControlCode::CSI => Ok(None), // Control Sequence Introducer - handled by AnsiMapperResult::CSI
                        ControlCode::StC1 => Ok(None), // String Terminator - handled by AnsiMapperResult::ST
                        ControlCode::OscC1 => Ok(None), // Operating System Command - handled by AnsiMapperResult::OSC
                        ControlCode::PmC1 => Ok(None), // Privacy Message - handled by AnsiMapperResult::PM
                        ControlCode::ApcC1 => Ok(None), // Application Program Command - handled by AnsiMapperResult::APC
                    },
                    AnsiMapperResult::Escape => Ok(None),
                    AnsiMapperResult::CSI(csi) => match csi {
                        CSICommand::CursorUp(n) => Ok(Some(TerminalEvent::CursorPosition {
                            cursor: self.buffer.move_cursor(0, -(n as isize)),  // col=0, row=-n (move up)
                        })),
                        CSICommand::CursorDown(n) => Ok(Some(TerminalEvent::CursorPosition {
                            cursor: self.buffer.move_cursor(0, n as isize),     // col=0, row=+n (move down)
                        })),
                        CSICommand::CursorForward(n) => Ok(Some(TerminalEvent::CursorPosition {
                            cursor: self.buffer.move_cursor(n as isize, 0),     // col=+n, row=0 (move right)
                        })),
                        CSICommand::CursorBack(n) => Ok(Some(TerminalEvent::CursorPosition {
                            cursor: self.buffer.move_cursor(-(n as isize), 0),  // col=-n, row=0 (move left)
                        })),
                        CSICommand::CursorNextLine(n) => {
                            for _ in 0..n {
                                self.buffer.complete_line();
                            }
                            Ok(Some(TerminalEvent::CursorPosition {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        CSICommand::CursorPreviousLine(n) => {
                            for _ in 0..n {
                                self.buffer.move_cursor(0, -1);
                            }
                            Ok(Some(TerminalEvent::CursorPosition {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        CSICommand::CursorHorizontalAbsolute(col) => {
                            let pos = self.buffer.cursor_position();
                            self.buffer
                                .set_cursor_position((col as usize).saturating_sub(1), pos.row);
                            Ok(Some(TerminalEvent::CursorPosition {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        CSICommand::CursorPosition { row, col } => {
                            self.buffer.set_cursor_position(
                                (col as usize).saturating_sub(1),
                                (row as usize).saturating_sub(1),
                            );
                            Ok(Some(TerminalEvent::CursorPosition {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        CSICommand::DeviceStatusReport => Ok(None),
                        CSICommand::SaveCursorPosition => Ok(None), // Could store cursor position
                        CSICommand::RestoreCursorPosition => Ok(None), // Could restore cursor position
                        CSICommand::EraseInDisplay(mode) => {
                            match mode {
                                EraseInDisplayMode::EraseToEndOfScreen => {
                                    // Clear from cursor to end of screen
                                    self.buffer.clear_completed_lines();
                                }
                                EraseInDisplayMode::EraseToBeginningOfScreen => {
                                    // Clear from cursor to beginning of screen
                                    self.buffer.erase_line();
                                }
                                EraseInDisplayMode::EraseEntireScreen
                                | EraseInDisplayMode::EraseEntireScreenAndSavedLines => {
                                    // Clear the entire screen (3 also clears scrollback)
                                    self.buffer.clear();
                                }
                            }
                            Ok(Some(TerminalEvent::Clear {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        CSICommand::EraseInLine(mode) => {
                            match mode {
                                EraseInLineMode::EraseEntireLine
                                | EraseInLineMode::EraseToEndOfLine
                                | EraseInLineMode::EraseToStartOfLine => {
                                    // All modes: erase the line
                                    self.buffer.erase_line();
                                }
                            }
                            Ok(Some(TerminalEvent::EraseLine {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        CSICommand::SetMode => Ok(None),
                        CSICommand::ResetMode => Ok(None),
                        CSICommand::DECPrivateModeSet => Ok(None),
                        CSICommand::DECPrivateModeReset => Ok(None),
                        CSICommand::ScrollUp => Ok(None),
                        CSICommand::ScrollDown => Ok(None),
                        CSICommand::InsertCharacter => Ok(None),
                        CSICommand::DeleteCharacter => {
                            self.buffer.erase_character();
                            Ok(Some(TerminalEvent::EraseCharacter {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        CSICommand::InsertLine => {
                            let completed_line = self.buffer.complete_line();
                            Ok(Some(TerminalEvent::LineCompleted {
                                cursor: self.buffer.cursor_position(),
                                line: completed_line,
                            }))
                        }
                        CSICommand::DeleteLine => {
                            self.buffer.erase_line();
                            Ok(Some(TerminalEvent::EraseLine {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        CSICommand::EraseCharacter => {
                            self.buffer.erase_character();
                            Ok(Some(TerminalEvent::EraseCharacter {
                                cursor: self.buffer.cursor_position(),
                            }))
                        }
                        CSICommand::TextCursorEnableMode => Ok(None),
                        CSICommand::AlternativeScreenBuffer => Ok(None),
                        CSICommand::SetKeyboardStrings => Ok(None),
                        CSICommand::Unknown => Ok(None),
                    },
                    AnsiMapperResult::SGR(_style) => {
                        // Style changes don't mutate the buffer directly
                        // They would be applied to subsequent characters
                        Ok(None)
                    }
                    AnsiMapperResult::OSC(_osc) => Ok(None),
                    AnsiMapperResult::DCS(_dcs) => Ok(None),
                    AnsiMapperResult::SOS(_sos) => Ok(None),
                    AnsiMapperResult::ST(_st) => Ok(None),
                    AnsiMapperResult::PM(_pm) => Ok(None),
                    AnsiMapperResult::APC(_apc) => Ok(None),
                },
                TelnetFrame::NoOperation => Ok(Some(TerminalEvent::NoOperation)),
                TelnetFrame::DataMark => Ok(None),
                TelnetFrame::Break => Ok(Some(TerminalEvent::Break)),
                TelnetFrame::InterruptProcess => Ok(Some(TerminalEvent::InterruptProcess)),
                TelnetFrame::AbortOutput => Ok(None),
                TelnetFrame::AreYouThere => Ok(None),
                TelnetFrame::EraseCharacter => {
                    self.buffer.erase_character();
                    Ok(Some(TerminalEvent::EraseCharacter {
                        cursor: self.buffer.cursor_position(),
                    }))
                }
                TelnetFrame::EraseLine => {
                    self.buffer.erase_line();
                    Ok(Some(TerminalEvent::EraseLine {
                        cursor: self.buffer.cursor_position(),
                    }))
                }
                TelnetFrame::GoAhead => Ok(None),
                TelnetFrame::Do(_) => Ok(None),
                TelnetFrame::Dont(_) => Ok(None),
                TelnetFrame::Will(_) => Ok(None),
                TelnetFrame::Wont(_) => Ok(None),
                TelnetFrame::Subnegotiate(_option, _argument) => Ok(None),
            },
            None => Ok(None),
        }
    }
}

impl Encoder<char> for TerminalCodec {
    type Error = TerminalError;

    fn encode(&mut self, item: char, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.codec.encode(item, dst).map_err(From::from)
    }
}

impl Encoder<&str> for TerminalCodec {
    type Error = TerminalError;

    fn encode(&mut self, item: &str, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.codec.encode(item, dst).map_err(From::from)
    }
}

impl Encoder<&TerminalCommand> for TerminalCodec {
    type Error = TerminalError;

    fn encode(&mut self, item: &TerminalCommand, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            TerminalCommand::SendBreak => self
                .codec
                .encode(TelnetFrame::Break, dst)
                .map_err(From::from),
            TerminalCommand::SendInterruptProcess => self
                .codec
                .encode(TelnetFrame::InterruptProcess, dst)
                .map_err(From::from),
            TerminalCommand::SendAbortOutput => self
                .codec
                .encode(TelnetFrame::AbortOutput, dst)
                .map_err(From::from),
            TerminalCommand::SendAreYouThere => self
                .codec
                .encode(TelnetFrame::AreYouThere, dst)
                .map_err(From::from),
            TerminalCommand::SendEraseCharacter => self
                .codec
                .encode(TelnetFrame::EraseCharacter, dst)
                .map_err(From::from),
            TerminalCommand::SendEraseLine => self
                .codec
                .encode(TelnetFrame::EraseLine, dst)
                .map_err(From::from),
        }
    }
}

impl Encoder<&ControlCode> for TerminalCodec {
    type Error = TerminalError;

    fn encode(&mut self, item: &ControlCode, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let byte = match item {
            ControlCode::NUL => 0x00,
            ControlCode::SOH => 0x01,
            ControlCode::STX => 0x02,
            ControlCode::ETX => 0x03,
            ControlCode::EOT => 0x04,
            ControlCode::ENQ => 0x05,
            ControlCode::ACK => 0x06,
            ControlCode::BEL => 0x07,
            ControlCode::BS => 0x08,
            ControlCode::HT => 0x09,
            ControlCode::LF => 0x0A,
            ControlCode::VT => 0x0B,
            ControlCode::FF => 0x0C,
            ControlCode::CR => 0x0D,
            ControlCode::SO => 0x0E,
            ControlCode::SI => 0x0F,
            ControlCode::DLE => 0x10,
            ControlCode::DC1 => 0x11,
            ControlCode::DC2 => 0x12,
            ControlCode::DC3 => 0x13,
            ControlCode::DC4 => 0x14,
            ControlCode::NAK => 0x15,
            ControlCode::SYN => 0x16,
            ControlCode::ETB => 0x17,
            ControlCode::CAN => 0x18,
            ControlCode::EM => 0x19,
            ControlCode::SUB => 0x1A,
            ControlCode::FS => 0x1C,
            ControlCode::GS => 0x1D,
            ControlCode::RS => 0x1E,
            ControlCode::US => 0x1F,
            ControlCode::DEL => 0x7F,
            ControlCode::PAD => 0x80,
            ControlCode::HOP => 0x81,
            ControlCode::BPH => 0x82,
            ControlCode::NBH => 0x83,
            ControlCode::IND => 0x84,
            ControlCode::NEL => 0x85,
            ControlCode::SSA => 0x86,
            ControlCode::ESA => 0x87,
            ControlCode::HTS => 0x88,
            ControlCode::HTJ => 0x89,
            ControlCode::VTS => 0x8A,
            ControlCode::PLD => 0x8B,
            ControlCode::PLU => 0x8C,
            ControlCode::RI => 0x8D,
            ControlCode::SS2 => 0x8E,
            ControlCode::SS3 => 0x8F,
            ControlCode::DCS => 0x90,
            ControlCode::PU1 => 0x91,
            ControlCode::PU2 => 0x92,
            ControlCode::STS => 0x93,
            ControlCode::CCH => 0x94,
            ControlCode::MW => 0x95,
            ControlCode::SPA => 0x96,
            ControlCode::EPA => 0x97,
            ControlCode::SOS => 0x98,
            ControlCode::SGCI => 0x99,
            ControlCode::SCI => 0x9A,
            ControlCode::CSI => 0x9B,
            ControlCode::StC1 => 0x9C,
            ControlCode::OscC1 => 0x9D,
            ControlCode::PmC1 => 0x9E,
            ControlCode::ApcC1 => 0x9F,
        };
        self.codec
            .encode(TelnetFrame::Data(byte), dst)
            .map_err(From::from)
    }
}

impl Encoder<CSICommand> for TerminalCodec {
    type Error = TerminalError;

    fn encode(&mut self, item: CSICommand, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // CSI sequences start with ESC [
        self.codec.encode(TelnetFrame::Data(0x1B), dst)?; // ESC
        self.codec.encode(TelnetFrame::Data(b'['), dst)?; // [

        match item {
            CSICommand::CursorUp(n) => {
                if n > 1 {
                    for byte in n.to_string().bytes() {
                        self.codec.encode(TelnetFrame::Data(byte), dst)?;
                    }
                }
                self.codec.encode(TelnetFrame::Data(b'A'), dst)?;
            }
            CSICommand::CursorDown(n) => {
                if n > 1 {
                    for byte in n.to_string().bytes() {
                        self.codec.encode(TelnetFrame::Data(byte), dst)?;
                    }
                }
                self.codec.encode(TelnetFrame::Data(b'B'), dst)?;
            }
            CSICommand::CursorForward(n) => {
                if n > 1 {
                    for byte in n.to_string().bytes() {
                        self.codec.encode(TelnetFrame::Data(byte), dst)?;
                    }
                }
                self.codec.encode(TelnetFrame::Data(b'C'), dst)?;
            }
            CSICommand::CursorBack(n) => {
                if n > 1 {
                    for byte in n.to_string().bytes() {
                        self.codec.encode(TelnetFrame::Data(byte), dst)?;
                    }
                }
                self.codec.encode(TelnetFrame::Data(b'D'), dst)?;
            }
            CSICommand::CursorNextLine(n) => {
                if n > 1 {
                    for byte in n.to_string().bytes() {
                        self.codec.encode(TelnetFrame::Data(byte), dst)?;
                    }
                }
                self.codec.encode(TelnetFrame::Data(b'E'), dst)?;
            }
            CSICommand::CursorPreviousLine(n) => {
                if n > 1 {
                    for byte in n.to_string().bytes() {
                        self.codec.encode(TelnetFrame::Data(byte), dst)?;
                    }
                }
                self.codec.encode(TelnetFrame::Data(b'F'), dst)?;
            }
            CSICommand::CursorHorizontalAbsolute(col) => {
                for byte in col.to_string().bytes() {
                    self.codec.encode(TelnetFrame::Data(byte), dst)?;
                }
                self.codec.encode(TelnetFrame::Data(b'G'), dst)?;
            }
            CSICommand::CursorPosition { row, col } => {
                for byte in row.to_string().bytes() {
                    self.codec.encode(TelnetFrame::Data(byte), dst)?;
                }
                self.codec.encode(TelnetFrame::Data(b';'), dst)?;
                for byte in col.to_string().bytes() {
                    self.codec.encode(TelnetFrame::Data(byte), dst)?;
                }
                self.codec.encode(TelnetFrame::Data(b'H'), dst)?;
            }
            CSICommand::DeviceStatusReport => {
                self.codec.encode(TelnetFrame::Data(b'6'), dst)?;
                self.codec.encode(TelnetFrame::Data(b'n'), dst)?;
            }
            CSICommand::SaveCursorPosition => {
                self.codec.encode(TelnetFrame::Data(b's'), dst)?;
            }
            CSICommand::RestoreCursorPosition => {
                self.codec.encode(TelnetFrame::Data(b'u'), dst)?;
            }
            CSICommand::EraseInDisplay(mode) => {
                let mode_num = match mode {
                    EraseInDisplayMode::EraseToEndOfScreen => 0,
                    EraseInDisplayMode::EraseToBeginningOfScreen => 1,
                    EraseInDisplayMode::EraseEntireScreen => 2,
                    EraseInDisplayMode::EraseEntireScreenAndSavedLines => 3,
                };
                if mode_num > 0 {
                    for byte in mode_num.to_string().bytes() {
                        self.codec.encode(TelnetFrame::Data(byte), dst)?;
                    }
                }
                self.codec.encode(TelnetFrame::Data(b'J'), dst)?;
            }
            CSICommand::EraseInLine(mode) => {
                let mode_num = match mode {
                    EraseInLineMode::EraseToEndOfLine => 0,
                    EraseInLineMode::EraseToStartOfLine => 1,
                    EraseInLineMode::EraseEntireLine => 2,
                };
                if mode_num > 0 {
                    for byte in mode_num.to_string().bytes() {
                        self.codec.encode(TelnetFrame::Data(byte), dst)?;
                    }
                }
                self.codec.encode(TelnetFrame::Data(b'K'), dst)?;
            }
            CSICommand::ScrollUp => {
                self.codec.encode(TelnetFrame::Data(b'S'), dst)?;
            }
            CSICommand::ScrollDown => {
                self.codec.encode(TelnetFrame::Data(b'T'), dst)?;
            }
            CSICommand::InsertCharacter => {
                self.codec.encode(TelnetFrame::Data(b'@'), dst)?;
            }
            CSICommand::DeleteCharacter => {
                self.codec.encode(TelnetFrame::Data(b'P'), dst)?;
            }
            CSICommand::InsertLine => {
                self.codec.encode(TelnetFrame::Data(b'L'), dst)?;
            }
            CSICommand::DeleteLine => {
                self.codec.encode(TelnetFrame::Data(b'M'), dst)?;
            }
            CSICommand::EraseCharacter => {
                self.codec.encode(TelnetFrame::Data(b'X'), dst)?;
            }
            CSICommand::TextCursorEnableMode => {
                self.codec.encode(TelnetFrame::Data(b'?'), dst)?;
                self.codec.encode(TelnetFrame::Data(b'2'), dst)?;
                self.codec.encode(TelnetFrame::Data(b'5'), dst)?;
                self.codec.encode(TelnetFrame::Data(b'h'), dst)?;
            }
            CSICommand::AlternativeScreenBuffer => {
                self.codec.encode(TelnetFrame::Data(b'?'), dst)?;
                self.codec.encode(TelnetFrame::Data(b'1'), dst)?;
                self.codec.encode(TelnetFrame::Data(b'0'), dst)?;
                self.codec.encode(TelnetFrame::Data(b'4'), dst)?;
                self.codec.encode(TelnetFrame::Data(b'9'), dst)?;
                self.codec.encode(TelnetFrame::Data(b'h'), dst)?;
            }
            CSICommand::SetMode
            | CSICommand::ResetMode
            | CSICommand::DECPrivateModeSet
            | CSICommand::DECPrivateModeReset
            | CSICommand::SetKeyboardStrings
            | CSICommand::Unknown => {
                // These commands don't have a standard encoding or need additional parameters
                // For now, just ignore them
            }
        }

        Ok(())
    }
}

impl Encoder<&Style> for TerminalCodec {
    type Error = TerminalError;

    fn encode(&mut self, item: &Style, dst: &mut BytesMut) -> Result<(), Self::Error> {
        for byte in item.to_string(Some(&self.config)).bytes() {
            self.codec.encode(TelnetFrame::Data(byte), dst)?;
        }
        Ok(())
    }
}

impl Encoder<&Segment> for TerminalCodec {
    type Error = TerminalError;

    fn encode(&mut self, item: &Segment, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            Segment::ASCII(data) => {
                self.encode(data.as_str(), dst)?;
            }
            Segment::Unicode(data) => {
                self.encode(data.as_str(), dst)?;
            }
            Segment::Control(ctrl) => {
                self.encode(ctrl, dst)?;
            }
            Segment::Escape => {
                self.codec.encode(TelnetFrame::Data(0x27), dst)?;
            }
            Segment::CSI(csi) => {
                self.encode(*csi, dst)?;
            }
            Segment::SGR(style) => {
                self.encode(style, dst)?;
            }
            Segment::OSC(osc) => {
                self.encode(osc.as_slice(), dst)?;
            }
            Segment::DCS(dcs) => {
                self.encode(dcs.as_slice(), dst)?;
            }
            Segment::SOS(sos) => {
                self.encode(sos.as_slice(), dst)?;
            }
            Segment::ST(st) => {
                self.encode(st.as_slice(), dst)?;
            }
            Segment::PM(pm) => {
                self.encode(pm.as_slice(), dst)?;
            }
            Segment::APC(apc) => {
                self.encode(apc.as_slice(), dst)?;
            }
        }

        Ok(())
    }
}

impl Encoder<SegmentedString> for TerminalCodec {
    type Error = TerminalError;

    fn encode(&mut self, item: SegmentedString, dst: &mut BytesMut) -> Result<(), Self::Error> {
        for segment in item.segments() {
            self.encode(segment, dst)?;
        }
        Ok(())
    }
}

impl Encoder<&[u8]> for TerminalCodec {
    type Error = TerminalError;

    fn encode(&mut self, item: &[u8], dst: &mut BytesMut) -> Result<(), Self::Error> {
        for byte in item.iter() {
            self.codec.encode(TelnetFrame::Data(*byte), dst)?;
        }
        Ok(())
    }
}


// ... existing code ...

#[cfg(test)]
mod tests {
    use super::*;
    use termionix_ansicodes::{Color, ControlCode, CSICommand, EraseInDisplayMode, EraseInLineMode, Style};
    use tokio_util::bytes::BytesMut;
    use tokio_util::codec::{Decoder, Encoder};
    // ===== Codec Creation Tests =====

    #[test]
    fn test_codec_new() {
        let codec = TerminalCodec::new();
        assert_eq!(codec.terminal_buffer().width(), 80);
        assert_eq!(codec.terminal_buffer().height(), 24);
    }

    #[test]
    fn test_codec_new_with_config() {
        let config = AnsiConfig::default();
        let codec = TerminalCodec::new_with_config(config);
        assert_eq!(codec.terminal_buffer().width(), 80);
    }

    // ===== Accessors Tests =====

    #[test]
    fn test_codec_accessors() {
        let mut codec = TerminalCodec::new();

        let _ = codec.ansi_mapper();
        let _ = codec.ansi_mapper_mut();
        let _ = codec.telnet_codec();
        let _ = codec.telnet_codec_mut();
        let _ = codec.terminal_buffer();
        let _ = codec.terminal_buffer_mut();
    }

    // ===== Decoder Tests - Basic Characters =====

    #[test]
    fn test_decode_ascii_character() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"A"[..]);

        let result = codec.decode(&mut buffer).unwrap();

        assert!(result.is_some());
        match result.unwrap() {
            TerminalEvent::CharacterData { character, .. } => {
                assert_eq!(character, 'A');
            }
            _ => panic!("Expected CharacterData event"),
        }
    }

    #[test]
    fn test_decode_unicode_character() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from("こ");

        // Unicode characters may require multiple decode calls
        let mut result = codec.decode(&mut buffer);
        while result.is_ok() && result.as_ref().unwrap().is_none() && !buffer.is_empty() {
            result = codec.decode(&mut buffer);
        }

        if let Ok(Some(TerminalEvent::CharacterData { character, .. })) = result {
            assert_eq!(character, 'こ');
        }
    }

    #[test]
    fn test_decode_multiple_characters() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"Hello"[..]);

        let mut chars = Vec::new();
        while !buffer.is_empty() {
            if let Ok(Some(TerminalEvent::CharacterData { character, .. })) = codec.decode(&mut buffer) {
                chars.push(character);
            }
        }

        assert_eq!(chars, vec!['H', 'e', 'l', 'l', 'o']);
    }

    // ===== Decoder Tests - Control Codes =====

    #[test]
    fn test_decode_bell() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&[0x07][..]); // BEL

        let result = codec.decode(&mut buffer).unwrap();
        assert!(matches!(result, Some(TerminalEvent::Bell)));
    }

    #[test]
    fn test_decode_backspace() {
        let mut codec = TerminalCodec::new();

        // First add a character
        let mut buffer = BytesMut::from(&b"A"[..]);
        codec.decode(&mut buffer).unwrap();

        // Then send backspace
        let mut buffer = BytesMut::from(&[0x08][..]); // BS
        let result = codec.decode(&mut buffer).unwrap();

        assert!(matches!(result, Some(TerminalEvent::EraseCharacter { .. })));
    }

    #[test]
    fn test_decode_tab() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&[0x09][..]); // HT

        let result = codec.decode(&mut buffer).unwrap();

        match result.unwrap() {
            TerminalEvent::CharacterData { character, .. } => {
                assert_eq!(character, '\t');
            }
            _ => panic!("Expected CharacterData with tab"),
        }
    }

    #[test]
    fn test_decode_line_feed() {
        let mut codec = TerminalCodec::new();

        // Add some characters first
        let mut buffer = BytesMut::from(&b"Test"[..]);
        while !buffer.is_empty() {
            codec.decode(&mut buffer).unwrap();
        }

        // Send line feed
        let mut buffer = BytesMut::from(&[0x0A][..]); // LF
        let result = codec.decode(&mut buffer).unwrap();

        assert!(matches!(result, Some(TerminalEvent::LineCompleted { .. })));
    }

    #[test]
    fn test_decode_carriage_return() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&[0x0D][..]); // CR

        let result = codec.decode(&mut buffer).unwrap();

        assert!(matches!(result, Some(TerminalEvent::CursorPosition { .. })));
    }

    #[test]
    fn test_decode_form_feed_clears_screen() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&[0x0C][..]); // FF

        let result = codec.decode(&mut buffer).unwrap();

        assert!(matches!(result, Some(TerminalEvent::Clear { .. })));
    }

    #[test]
    fn test_decode_delete() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&[0x7F][..]); // DEL

        let result = codec.decode(&mut buffer).unwrap();

        assert!(matches!(result, Some(TerminalEvent::EraseCharacter { .. })));
    }

    // ===== Decoder Tests - CSI Commands =====

    #[test]
    fn test_decode_cursor_up() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"\x1B[3A"[..]); // ESC [ 3 A

        let mut result = codec.decode(&mut buffer);
        while result.is_ok() && result.as_ref().unwrap().is_none() && !buffer.is_empty() {
            result = codec.decode(&mut buffer);
        }

        assert!(matches!(result, Ok(Some(TerminalEvent::CursorPosition { .. }))));
    }

    #[test]
    fn test_decode_cursor_down() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"\x1B[2B"[..]); // ESC [ 2 B

        let mut result = codec.decode(&mut buffer);
        while result.is_ok() && result.as_ref().unwrap().is_none() && !buffer.is_empty() {
            result = codec.decode(&mut buffer);
        }

        assert!(matches!(result, Ok(Some(TerminalEvent::CursorPosition { .. }))));
    }

    #[test]
    fn test_decode_cursor_forward() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"\x1B[5C"[..]); // ESC [ 5 C

        let mut result = codec.decode(&mut buffer);
        while result.is_ok() && result.as_ref().unwrap().is_none() && !buffer.is_empty() {
            result = codec.decode(&mut buffer);
        }

        assert!(matches!(result, Ok(Some(TerminalEvent::CursorPosition { .. }))));
    }

    #[test]
    fn test_decode_cursor_back() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"\x1B[4D"[..]); // ESC [ 4 D

        let mut result = codec.decode(&mut buffer);
        while result.is_ok() && result.as_ref().unwrap().is_none() && !buffer.is_empty() {
            result = codec.decode(&mut buffer);
        }

        assert!(matches!(result, Ok(Some(TerminalEvent::CursorPosition { .. }))));
    }

    #[test]
    fn test_decode_cursor_position() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"\x1B[10;20H"[..]); // ESC [ 10 ; 20 H

        let mut result = codec.decode(&mut buffer);
        while result.is_ok() && result.as_ref().unwrap().is_none() && !buffer.is_empty() {
            result = codec.decode(&mut buffer);
        }

        match result {
            Ok(Some(TerminalEvent::CursorPosition { cursor })) => {
                assert_eq!(cursor.col, 19); // 1-based to 0-based
                assert_eq!(cursor.row, 9);
            }
            _ => panic!("Expected CursorPosition event"),
        }
    }

    #[test]
    fn test_decode_erase_in_display() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"\x1B[2J"[..]); // ESC [ 2 J (clear screen)

        let mut result = codec.decode(&mut buffer);
        while result.is_ok() && result.as_ref().unwrap().is_none() && !buffer.is_empty() {
            result = codec.decode(&mut buffer);
        }

        assert!(matches!(result, Ok(Some(TerminalEvent::Clear { .. }))));
    }

    #[test]
    fn test_decode_erase_in_line() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"\x1B[2K"[..]); // ESC [ 2 K (erase entire line)

        let mut result = codec.decode(&mut buffer);
        while result.is_ok() && result.as_ref().unwrap().is_none() && !buffer.is_empty() {
            result = codec.decode(&mut buffer);
        }

        assert!(matches!(result, Ok(Some(TerminalEvent::EraseLine { .. }))));
    }

    #[test]
    fn test_decode_delete_character_csi() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"\x1B[P"[..]); // ESC [ P

        let mut result = codec.decode(&mut buffer);
        while result.is_ok() && result.as_ref().unwrap().is_none() && !buffer.is_empty() {
            result = codec.decode(&mut buffer);
        }

        assert!(matches!(result, Ok(Some(TerminalEvent::EraseCharacter { .. }))));
    }

    // ===== Decoder Tests - TelnetFrame Events =====

    #[test]
    fn test_decode_telnet_no_operation() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&[0xFF, 0xF1][..]); // IAC NOP

        let result = codec.decode(&mut buffer).unwrap();

        assert!(matches!(result, Some(TerminalEvent::NoOperation)));
    }

    #[test]
    fn test_decode_telnet_break() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&[0xFF, 0xF3][..]); // IAC BREAK

        let result = codec.decode(&mut buffer).unwrap();

        assert!(matches!(result, Some(TerminalEvent::Break)));
    }

    #[test]
    fn test_decode_telnet_interrupt_process() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&[0xFF, 0xF4][..]); // IAC IP

        let result = codec.decode(&mut buffer).unwrap();

        assert!(matches!(result, Some(TerminalEvent::InterruptProcess)));
    }

    #[test]
    fn test_decode_telnet_erase_character() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&[0xFF, 0xF7][..]); // IAC EC

        let result = codec.decode(&mut buffer).unwrap();

        assert!(matches!(result, Some(TerminalEvent::EraseCharacter { .. })));
    }

    #[test]
    fn test_decode_telnet_erase_line() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&[0xFF, 0xF8][..]); // IAC EL

        let result = codec.decode(&mut buffer).unwrap();

        assert!(matches!(result, Some(TerminalEvent::EraseLine { .. })));
    }

    // ===== Encoder Tests - Basic Types =====

    #[test]
    fn test_encode_char() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode('X', &mut buffer).unwrap();

        assert!(!buffer.is_empty());
        assert_eq!(buffer[0], b'X');
    }

    #[test]
    fn test_encode_str() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode("Hello", &mut buffer).unwrap();

        assert!(!buffer.is_empty());
        assert_eq!(&buffer[..5], b"Hello");
    }

    #[test]
    fn test_encode_bytes() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();
        let data: &[u8] = &[0x48, 0x69]; // "Hi"

        codec.encode(data, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 2);
    }

    // ===== Encoder Tests - TerminalCommand =====

    #[test]
    fn test_encode_terminal_command_break() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(&TerminalCommand::SendBreak, &mut buffer).unwrap();

        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_encode_terminal_command_interrupt_process() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(&TerminalCommand::SendInterruptProcess, &mut buffer).unwrap();

        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_encode_terminal_command_abort_output() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(&TerminalCommand::SendAbortOutput, &mut buffer).unwrap();

        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_encode_terminal_command_are_you_there() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(&TerminalCommand::SendAreYouThere, &mut buffer).unwrap();

        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_encode_terminal_command_erase_character() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(&TerminalCommand::SendEraseCharacter, &mut buffer).unwrap();

        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_encode_terminal_command_erase_line() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(&TerminalCommand::SendEraseLine, &mut buffer).unwrap();

        assert!(!buffer.is_empty());
    }

    // ===== Encoder Tests - ControlCode =====

    #[test]
    fn test_encode_control_code_bell() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(&ControlCode::BEL, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], 0x07);
    }

    #[test]
    fn test_encode_control_code_line_feed() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(&ControlCode::LF, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], 0x0A);
    }

    #[test]
    fn test_encode_control_code_carriage_return() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(&ControlCode::CR, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], 0x0D);
    }

    // ===== Encoder Tests - CSICommand =====

    #[test]
    fn test_encode_csi_cursor_up() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(CSICommand::CursorUp(5), &mut buffer).unwrap();

        assert!(!buffer.is_empty());
        // Should contain ESC [ 5 A
        let s = String::from_utf8_lossy(&buffer);
        assert!(s.contains("\x1B["));
        assert!(s.contains("A"));
    }

    #[test]
    fn test_encode_csi_cursor_position() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(CSICommand::CursorPosition { row: 10, col: 20 }, &mut buffer).unwrap();

        assert!(!buffer.is_empty());
        let s = String::from_utf8_lossy(&buffer);
        assert!(s.contains("\x1B["));
        assert!(s.contains("10"));
        assert!(s.contains(";"));
        assert!(s.contains("20"));
        assert!(s.contains("H"));
    }

    #[test]
    fn test_encode_csi_erase_in_display() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(CSICommand::EraseInDisplay(EraseInDisplayMode::EraseEntireScreen), &mut buffer).unwrap();

        assert!(!buffer.is_empty());
        let s = String::from_utf8_lossy(&buffer);
        assert!(s.contains("\x1B["));
        assert!(s.contains("2J"));
    }

    #[test]
    fn test_encode_csi_erase_in_line() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        codec.encode(CSICommand::EraseInLine(EraseInLineMode::EraseEntireLine), &mut buffer).unwrap();

        assert!(!buffer.is_empty());
        let s = String::from_utf8_lossy(&buffer);
        assert!(s.contains("\x1B["));
        assert!(s.contains("2K"));
    }

    // ===== Encoder Tests - Style =====

    #[test]
    fn test_encode_style() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();
        let style = Style::default();

        codec.encode(&style, &mut buffer).unwrap();

        // Default style has no attributes, so output may be empty
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_encode_style_with_color() {
        let mut codec = TerminalCodec::new();
        codec.config = AnsiConfig::default();
        let mut buffer = BytesMut::new();
        let style = Style {
            foreground: Some(Color::Red),
            ..Default::default()
        };

        codec.encode(&style, &mut buffer).unwrap();

        // Style with color should produce ANSI escape sequence
        assert!(!buffer.is_empty());
        let s = String::from_utf8_lossy(&buffer);
        assert!(s.contains("\x1B["));
        assert!(s.contains("31")); // Red foreground code
        assert!(s.contains("m"));
    }

    // ===== Encoder Tests - Segment =====

    #[test]
    fn test_encode_segment_ascii() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();
        let segment = Segment::ASCII("Hello".to_string());

        codec.encode(&segment, &mut buffer).unwrap();

        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_encode_segment_unicode() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();
        let segment = Segment::Unicode("世界".to_string());

        codec.encode(&segment, &mut buffer).unwrap();

        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_encode_segment_control() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();
        let segment = Segment::Control(ControlCode::BEL);

        codec.encode(&segment, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], 0x07);
    }

    #[test]
    fn test_encode_segment_csi() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();
        let segment = Segment::CSI(CSICommand::CursorUp(1));

        codec.encode(&segment, &mut buffer).unwrap();

        assert!(!buffer.is_empty());
    }

    // ===== Encoder Tests - SegmentedString =====

    #[test]
    fn test_encode_segmented_string() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();
        let segmented = SegmentedString::from("Test");

        codec.encode(segmented, &mut buffer).unwrap();

        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_encode_segmented_string_empty() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();
        let segmented = SegmentedString::from("");

        codec.encode(segmented, &mut buffer).unwrap();

        // Empty string should still encode successfully (even if buffer is empty)
        assert!(buffer.is_empty());
    }

    // ===== Integration Tests =====

    #[test]
    fn test_encode_decode_roundtrip_simple() {
        let mut encoder = TerminalCodec::new();
        let mut decoder = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        encoder.encode("A", &mut buffer).unwrap();

        let result = decoder.decode(&mut buffer).unwrap();
        match result {
            Some(TerminalEvent::CharacterData { character, .. }) => {
                assert_eq!(character, 'A');
            }
            _ => panic!("Expected CharacterData"),
        }
    }

    #[test]
    fn test_buffer_state_after_decoding() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"Test"[..]);

        while !buffer.is_empty() {
            codec.decode(&mut buffer).unwrap();
        }

        assert_eq!(codec.terminal_buffer().current_line_length(), 4);
        assert!(!codec.terminal_buffer().is_current_line_empty());
    }

    #[test]
    fn test_decode_mixed_content() {
        let mut codec = TerminalCodec::new();
        // Mix of text and control codes
        let mut buffer = BytesMut::from(&b"A\x07B\x08C"[..]); // A, BEL, B, BS, C

        let mut events = Vec::new();
        while !buffer.is_empty() {
            if let Ok(Some(event)) = codec.decode(&mut buffer) {
                events.push(event);
            }
        }

        assert!(events.len() >= 3); // At least A, BEL, and some backspace/character events
    }

    #[test]
    fn test_incomplete_sequence_handling() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::from(&b"\x1B["[..]); // Incomplete CSI sequence

        let result = codec.decode(&mut buffer).unwrap();

        // Should return None for incomplete sequence
        assert!(result.is_none());
    }

    #[test]
    fn test_encoder_error_propagation() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();

        // All encoding operations should succeed for valid inputs
        assert!(codec.encode('A', &mut buffer).is_ok());
        assert!(codec.encode("test", &mut buffer).is_ok());
        assert!(codec.encode(&TerminalCommand::SendBreak, &mut buffer).is_ok());
    }

    #[test]
    fn test_cursor_position_updates() {
        let mut codec = TerminalCodec::new();

        // Start at 0,0
        assert_eq!(codec.terminal_buffer().cursor_position(), CursorPosition::new(0, 0));

        // Add a character
        let mut buffer = BytesMut::from(&b"A"[..]);
        codec.decode(&mut buffer).unwrap();

        // Cursor should have moved
        let pos = codec.terminal_buffer().cursor_position();
        assert!(pos.col > 0 || pos.row > 0);
    }

    #[test]
    fn test_encode_all_terminal_commands() {
        let mut codec = TerminalCodec::new();
        let commands = vec![
            TerminalCommand::SendBreak,
            TerminalCommand::SendInterruptProcess,
            TerminalCommand::SendAbortOutput,
            TerminalCommand::SendAreYouThere,
            TerminalCommand::SendEraseCharacter,
            TerminalCommand::SendEraseLine,
        ];

        for command in commands {
            let mut buffer = BytesMut::new();
            assert!(codec.encode(&command, &mut buffer).is_ok());
            assert!(!buffer.is_empty());
        }
    }

    #[test]
    fn test_encode_escape_sequence() {
        let mut codec = TerminalCodec::new();
        let mut buffer = BytesMut::new();
        let segment = Segment::Escape;

        codec.encode(&segment, &mut buffer).unwrap();

        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0], 0x27);
    }
}