//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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

/// ISO 6429 Control Codes (C0 and C1 sets)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlCode {
    // C0 Control Codes (0x00-0x1F, 0x7F)
    /// Null character
    NUL,
    /// Start of Heading
    SOH,
    /// Start of Text
    STX,
    /// End of Text
    ETX,
    /// End of Transmission
    EOT,
    /// Enquiry
    ENQ,
    /// Acknowledge
    ACK,
    /// Bell/Alert
    BEL,
    /// Backspace
    BS,
    /// Horizontal Tab
    HT,
    /// Line Feed
    LF,
    /// Vertical Tab
    VT,
    /// Form Feed
    FF,
    /// Carriage Return
    CR,
    /// Shift Out
    SO,
    /// Shift In
    SI,
    /// Data Link Escape
    DLE,
    /// Device Control 1
    DC1,
    /// Device Control 2
    DC2,
    /// Device Control 3
    DC3,
    /// Device Control 4
    DC4,
    /// Negatively Acknowledge
    NAK,
    /// Synchronous Idle
    SYN,
    /// End of Transmission Block
    ETB,
    /// Cancel
    CAN,
    /// End of Medium
    EM,
    /// Substitute
    SUB,
    // ESC (0x1B) is handled separately as Escape sequences
    /// File Separator
    FS,
    /// Group Separator
    GS,
    /// Record Separator
    RS,
    /// Unit Separator
    US,
    /// Delete
    DEL,

    // C1 Control Codes (0x80-0x9F) - rarely used in modern terminals
    /// Padding Character
    PAD,
    /// High Octet Preset
    HOP,
    /// Break Permitted Here
    BPH,
    /// No Break Here
    NBH,
    /// Index
    IND,
    /// Next Line
    NEL,
    /// Start of Selected Area
    SSA,
    /// End of Selected Area
    ESA,
    /// Character Tabulation Set
    HTS,
    /// Character Tabulation with Justification
    HTJ,
    /// Line Tabulation Set
    VTS,
    /// Partial Line Forward
    PLD,
    /// Partial Line Backward
    PLU,
    /// Reverse Index
    RI,
    /// Single Shift Two
    SS2,
    /// Single Shift Three
    SS3,
    /// Device Control String
    DCS,
    /// Private Use One
    PU1,
    /// Private Use Two
    PU2,
    /// Set Transmit State
    STS,
    /// Cancel Character
    CCH,
    /// Message Waiting
    MW,
    /// Start of Guarded Area
    SPA,
    /// End of Guarded Area
    EPA,
    /// Start of String
    SOS,
    // SGCI (0x99) - Single Graphic Character Introducer
    /// Single Graphic Character Introducer
    SGCI,
    /// Single Character Introducer
    SCI,
    /// Control Sequence Introducer
    CSI,
    /// String Terminator
    StC1,
    /// Operating System Command
    OscC1,
    /// Privacy Message
    PmC1,
    /// Application Program Command
    ApcC1,
}

impl ControlCode {
    /// Convert control code to byte representation
    pub fn to_byte(&self) -> u8 {
        match self {
            // C0 control codes
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
            // C1 control codes
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
        }
    }

    /// Convert a byte to its corresponding control code
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            // C0 Control codes
            0x00 => Some(ControlCode::NUL),
            0x01 => Some(ControlCode::SOH),
            0x02 => Some(ControlCode::STX),
            0x03 => Some(ControlCode::ETX),
            0x04 => Some(ControlCode::EOT),
            0x05 => Some(ControlCode::ENQ),
            0x06 => Some(ControlCode::ACK),
            0x07 => Some(ControlCode::BEL),
            0x08 => Some(ControlCode::BS),
            0x09 => Some(ControlCode::HT),
            0x0A => Some(ControlCode::LF),
            0x0B => Some(ControlCode::VT),
            0x0C => Some(ControlCode::FF),
            0x0D => Some(ControlCode::CR),
            0x0E => Some(ControlCode::SO),
            0x0F => Some(ControlCode::SI),
            0x10 => Some(ControlCode::DLE),
            0x11 => Some(ControlCode::DC1),
            0x12 => Some(ControlCode::DC2),
            0x13 => Some(ControlCode::DC3),
            0x14 => Some(ControlCode::DC4),
            0x15 => Some(ControlCode::NAK),
            0x16 => Some(ControlCode::SYN),
            0x17 => Some(ControlCode::ETB),
            0x18 => Some(ControlCode::CAN),
            0x19 => Some(ControlCode::EM),
            0x1A => Some(ControlCode::SUB),
            // 0x1B is ESC - handled separately
            0x1C => Some(ControlCode::FS),
            0x1D => Some(ControlCode::GS),
            0x1E => Some(ControlCode::RS),
            0x1F => Some(ControlCode::US),
            0x7F => Some(ControlCode::DEL),

            // C1 Control codes (0x80-0x9F)
            0x80 => Some(ControlCode::PAD),
            0x81 => Some(ControlCode::HOP),
            0x82 => Some(ControlCode::BPH),
            0x83 => Some(ControlCode::NBH),
            0x84 => Some(ControlCode::IND),
            0x85 => Some(ControlCode::NEL),
            0x86 => Some(ControlCode::SSA),
            0x87 => Some(ControlCode::ESA),
            0x88 => Some(ControlCode::HTS),
            0x89 => Some(ControlCode::HTJ),
            0x8A => Some(ControlCode::VTS),
            0x8B => Some(ControlCode::PLD),
            0x8C => Some(ControlCode::PLU),
            0x8D => Some(ControlCode::RI),
            0x8E => Some(ControlCode::SS2),
            0x8F => Some(ControlCode::SS3),
            0x90 => Some(ControlCode::DCS),
            0x91 => Some(ControlCode::PU1),
            0x92 => Some(ControlCode::PU2),
            0x93 => Some(ControlCode::STS),
            0x94 => Some(ControlCode::CCH),
            0x95 => Some(ControlCode::MW),
            0x96 => Some(ControlCode::SPA),
            0x97 => Some(ControlCode::EPA),
            0x98 => Some(ControlCode::SOS),
            0x99 => Some(ControlCode::SGCI),
            0x9A => Some(ControlCode::SCI),
            0x9B => Some(ControlCode::CSI),
            0x9C => Some(ControlCode::StC1),
            0x9D => Some(ControlCode::OscC1),
            0x9E => Some(ControlCode::PmC1),
            0x9F => Some(ControlCode::ApcC1),

            _ => None,
        }
    }
}

/// Control Sequence Introducer (CSI) Command
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CSICommand {
    // Cursor Controls
    /// CUU - Cursor Up
    /// ESC[#A - moves cursor up # lines
    CursorUp(u8),

    /// CUD - Cursor Down
    /// ESC[#B - moves cursor down # lines
    CursorDown(u8),

    /// CUF - Cursor Forward
    /// ESC[#C - moves cursor right # columns
    CursorForward(u8),

    /// CUB - Cursor Back
    /// ESC[#D - moves cursor left # columns
    CursorBack(u8),

    /// CNL - Cursor Next Line
    /// ESC[#E - moves the cursor to the beginning of the next line, # lines down
    CursorNextLine(u8),

    /// CPL - Cursor Previous Line
    /// ESC[#F - moves the cursor to the beginning of the previous line, # lines up
    CursorPreviousLine(u8),

    /// CHA - Cursor Horizontal Absolute
    /// ESC[#G - moves cursor to column #
    CursorHorizontalAbsolute(u8),

    /// CUP - Cursor Position / HVP - Horizontal Vertical Position
    /// ESC[{line};{column}H or ESC[{line};{column}f
    CursorPosition {
        /// Cursor Row
        row: u8,
        /// Cursor Column
        col: u8,
    },

    /// DSR - Device Status Report
    /// ESC[6n - request cursor position (reports as ESC[#;#R)
    DeviceStatusReport,

    /// SCP - Save Cursor Position (SCO)
    /// ESC[s
    SaveCursorPosition,

    /// RCP - Restore Cursor Position (SCO)
    /// ESC[u
    RestoreCursorPosition,

    // Erase Functions
    /// ED - Erase in Display
    /// ESC[J or ESC[0J - erase from cursor until end of screen
    /// ESC[1J - erase from cursor to beginning of screen
    /// ESC[2J - erase entire screen
    /// ESC[3J - erase saved lines
    EraseInDisplay(EraseInDisplayMode),

    /// EL - Erase in Line
    /// ESC[K or ESC[0K - erase from cursor to end of line
    /// ESC[1K - erase start of line to the cursor
    /// ESC[2K - erase the entire line
    EraseInLine(EraseInLineMode),

    // Screen Modes
    /// SM - Set Mode
    /// ESC[={value}h - Changes screen width or type
    SetMode,

    /// RM - Reset Mode
    /// ESC[={value}l - Resets the mode
    ResetMode,

    /// DECSET - DEC Private Mode Set
    /// ESC[?{value}h - DEC private mode set
    DECPrivateModeSet,

    /// DECRST - DEC Private Mode Reset
    /// ESC[?{value}l - DEC private mode reset
    DECPrivateModeReset,

    // Scrolling
    /// SU - Scroll Up
    /// ESC[#S - Scroll up # lines
    ScrollUp,

    /// SD - Scroll Down
    /// ESC[#T - Scroll down # lines
    ScrollDown,

    // Insert/Delete
    /// ICH - Insert Character
    /// ESC[#@ - Insert # blank characters
    InsertCharacter,

    /// DCH - Delete Character
    /// ESC[#P - Delete # characters
    DeleteCharacter,

    /// IL - Insert Line
    /// ESC[#L - Insert # blank lines
    InsertLine,

    /// DL - Delete Line
    /// ESC[#M - Delete # lines
    DeleteLine,

    /// ECH - Erase Character
    /// ESC[#X - Erase # characters from the cursor position
    EraseCharacter,

    // Cursor Visibility
    /// DECTCEM - Text Cursor Enable Mode
    /// ESC[?25h - Show cursor
    /// ESC[?25l - Hide cursor
    TextCursorEnableMode,

    // Alternative Screen Buffer
    /// Alt Screen - Alternative Screen Buffer
    /// ESC[?1049h - Enable alternative buffer
    /// ESC[?1049l - Disable alternative buffer
    AlternativeScreenBuffer,

    // Keyboard String Remapping
    /// Set Keyboard Strings
    /// ESC[{code};{string};{...}p
    SetKeyboardStrings,

    /// Unknown or unsupported CSI command
    Unknown,
}

impl CSICommand {
    /// Writes the CSI command as an ANSI escape sequence to the provided writer.
    ///
    /// This method generates the appropriate ANSI CSI (Control Sequence Introducer)
    /// escape sequence for the command. CSI sequences have the general format:
    /// `ESC [ <parameters> <final_byte>`
    ///
    /// # Arguments
    ///
    /// * `mode` - The color mode (currently unused for CSI commands, but included for consistency)
    /// * `writer` - The writer to output the CSI sequence to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a [`std::fmt::Error`] if writing fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::{CSICommand, ColorMode};
    ///
    /// let cmd = CSICommand::CursorUp(5);
    /// let mut output = String::new();
    /// cmd.write_csi(&ColorMode::None, &mut output).unwrap();
    /// assert_eq!(output, "\x1b[5A");
    /// ```
    pub fn write_csi<W: std::fmt::Write>(
        &self,
        _mode: &crate::ColorMode,
        writer: &mut W,
    ) -> std::fmt::Result {
        match self {
            // Cursor movement commands
            CSICommand::CursorUp(n) => {
                write!(writer, "\x1b[{}A", n)
            }
            CSICommand::CursorDown(n) => {
                write!(writer, "\x1b[{}B", n)
            }
            CSICommand::CursorForward(n) => {
                write!(writer, "\x1b[{}C", n)
            }
            CSICommand::CursorBack(n) => {
                write!(writer, "\x1b[{}D", n)
            }
            CSICommand::CursorNextLine(n) => {
                write!(writer, "\x1b[{}E", n)
            }
            CSICommand::CursorPreviousLine(n) => {
                write!(writer, "\x1b[{}F", n)
            }
            CSICommand::CursorHorizontalAbsolute(col) => {
                write!(writer, "\x1b[{}G", col)
            }
            CSICommand::CursorPosition { row, col } => {
                write!(writer, "\x1b[{};{}H", row, col)
            }

            // Device status and cursor save/restore
            CSICommand::DeviceStatusReport => {
                write!(writer, "\x1b[6n")
            }
            CSICommand::SaveCursorPosition => {
                write!(writer, "\x1b[s")
            }
            CSICommand::RestoreCursorPosition => {
                write!(writer, "\x1b[u")
            }

            // Erase functions
            CSICommand::EraseInDisplay(mode) => {
                write!(writer, "\x1b[{}J", *mode as u8)
            }
            CSICommand::EraseInLine(mode) => {
                write!(writer, "\x1b[{}K", *mode as u8)
            }

            // Screen modes
            CSICommand::SetMode => {
                write!(writer, "\x1b[=h")
            }
            CSICommand::ResetMode => {
                write!(writer, "\x1b[=l")
            }
            CSICommand::DECPrivateModeSet => {
                write!(writer, "\x1b[?h")
            }
            CSICommand::DECPrivateModeReset => {
                write!(writer, "\x1b[?l")
            }

            // Scrolling
            CSICommand::ScrollUp => {
                write!(writer, "\x1b[S")
            }
            CSICommand::ScrollDown => {
                write!(writer, "\x1b[T")
            }

            // Insert/Delete
            CSICommand::InsertCharacter => {
                write!(writer, "\x1b[@")
            }
            CSICommand::DeleteCharacter => {
                write!(writer, "\x1b[P")
            }
            CSICommand::InsertLine => {
                write!(writer, "\x1b[L")
            }
            CSICommand::DeleteLine => {
                write!(writer, "\x1b[M")
            }
            CSICommand::EraseCharacter => {
                write!(writer, "\x1b[X")
            }

            // Cursor visibility
            CSICommand::TextCursorEnableMode => {
                write!(writer, "\x1b[?25h")
            }

            // Alternative screen buffer
            CSICommand::AlternativeScreenBuffer => {
                write!(writer, "\x1b[?1049h")
            }

            // Keyboard strings
            CSICommand::SetKeyboardStrings => {
                write!(writer, "\x1b[p")
            }

            // Unknown commands
            CSICommand::Unknown => {
                // Don't output anything for unknown commands
                Ok(())
            }
        }
    }
}

/// ED - Erase in Display mode parameter
///
/// Specifies which portion of the display to erase when using the ED (Erase in Display)
/// CSI command (ESC[nJ). The cursor position is not changed by this operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EraseInDisplayMode {
    /// Erase from cursor position to end of screen (inclusive)
    ///
    /// ESC[0J or ESC[J - Clears all characters from the cursor position to the end of the
    /// screen, including the character at the cursor position.
    EraseToEndOfScreen = 0,

    /// Erase from the beginning of screen to cursor position (inclusive)
    ///
    /// ESC[1J - Clears all characters from the beginning of the screen to the cursor
    /// position, including the character at the cursor position.
    EraseToBeginningOfScreen = 1,

    /// Erase the entire screen
    ///
    /// ESC[2J - Clears the entire visible screen. In most modern terminals, this does not
    /// move the cursor and does not clear the scrollback buffer.
    EraseEntireScreen = 2,

    /// Erase the entire screen and scrollback buffer
    ///
    /// ESC[3J - Clears the entire visible screen and also clears the scrollback buffer
    /// (terminal history). This is an extended feature not part of the original standard
    /// and may not be supported by all terminals.
    EraseEntireScreenAndSavedLines = 3,
}

/// EL - Erase in Line mode parameter
///
/// Specifies which portion of the current line to erase when using the EL (Erase in Line)
/// CSI command (ESC[nK). The cursor position is not changed by this operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EraseInLineMode {
    /// Erase from cursor position to end of line (inclusive)
    ///
    /// ESC[0K or ESC[K - Clears all characters from the cursor position to the end of the
    /// current line, including the character at the cursor position.
    EraseToEndOfLine = 0,

    /// Erase from beginning of line to cursor position (inclusive)
    ///
    /// ESC[1K - Clears all characters from the beginning of the current line to the cursor
    /// position, including the character at the cursor position.
    EraseToStartOfLine = 1,

    /// Erase entire line
    ///
    /// ESC[2K - Clears all characters on the current line. The cursor remains at its
    /// current position within the now-blank line.
    EraseEntireLine = 2,
}
