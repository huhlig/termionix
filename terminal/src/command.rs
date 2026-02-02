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

use termionix_ansicodec::gmcp::GmcpMessage;
use termionix_ansicodec::msdp::MudServerData;
use termionix_ansicodec::mssp::MudServerStatus;
use termionix_ansicodec::{
    AnsiApplicationProgramCommand, AnsiControlCode, AnsiControlSequenceIntroducer,
    AnsiDeviceControlString, AnsiOperatingSystemCommand, AnsiPrivacyMessage,
    AnsiSelectGraphicRendition, AnsiSequence, AnsiStartOfString,
};

/// Terminal commands for output and control
///
/// This enum represents all commands that can be sent to a terminal,
/// including text output, control sequences, and telnet commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalCommand {
    // Text output variants
    /// Send a single character
    Char(char),
    /// Send text as a string
    Text(String),
    /// Send raw bytes
    Bytes(Vec<u8>),

    // ANSI Sequences
    /// A C0 or C1 control character.
    ///
    /// These are non-printable control codes in the ranges:
    /// - C0: 0x00-0x1F (excluding ESC at 0x1B)
    /// - DEL: 0x7F
    /// - C1: 0x80-0x9F
    ///
    /// Common examples include NULL (0x00), Bell (0x07), Backspace (0x08),
    /// Tab (0x09), Line Feed (0x0A), and Carriage Return (0x0D).
    Control(AnsiControlCode),

    /// A standalone ESC character (0x1B) that is not part of a recognized sequence.
    ///
    /// This occurs when an ESC character is followed by a byte that doesn't
    /// initiate a known ANSI escape sequence. The ESC was not consumed as part
    /// of a control sequence.
    AnsiEscape,

    /// Control Sequence Introducer - a general CSI escape sequence.
    ///
    /// Format: `ESC [ <params> <final_byte>`
    ///
    /// CSI sequences are used for cursor movement, screen manipulation, and other
    /// terminal control operations. The final byte (0x40-0x7E) determines the
    /// specific command. Common examples:
    /// - `ESC[H` - Cursor Home
    /// - `ESC[2J` - Clear Screen
    /// - `ESC[10;20H` - Move cursor to row 10, column 20
    ///
    /// Note: SGR sequences (ending with 'm') are parsed separately and returned
    /// as the `SGR` variant instead.
    AnsiCSI(AnsiControlSequenceIntroducer),

    /// Select Graphic Rendition - a specialized CSI sequence for text styling.
    ///
    /// Format: `ESC [ <params> m`
    ///
    /// SGR sequences control text appearance including colors, bold, italic,
    /// underline, and other visual attributes. This is a specialized form of
    /// CSI sequence that is parsed into a `Style` object for convenience.
    ///
    /// Examples:
    /// - `ESC[0m` - Reset all attributes
    /// - `ESC[1m` - Bold
    /// - `ESC[31m` - Red foreground
    /// - `ESC[1;31;42m` - Bold red text on a green background
    AnsiSGR(AnsiSelectGraphicRendition),

    /// Operating System Command - a sequence for terminal-specific operations.
    ///
    /// Format: `ESC ] <params> ST` or `ESC ] <params> BEL`
    ///
    /// OSC sequences communicate with the terminal emulator to perform operations
    /// like setting the window title, changing color palettes, or other OS-level
    /// terminal features. The sequence is terminated by either ST (String Terminator,
    /// ESC \) or BEL (0x07).
    ///
    /// The raw bytes (excluding the terminator) are returned for interpretation
    /// by the application.
    AnsiOSC(AnsiOperatingSystemCommand),

    /// Device Control String - a sequence for device-specific control.
    ///
    /// Format: `ESC P <params> ST`
    ///
    /// DCS sequences are used to send device-specific control strings to the
    /// terminal. They are terminated by ST (ESC \). The contents are device-
    /// dependent and returned as raw bytes.
    AnsiDCS(AnsiDeviceControlString),

    /// Start of String - a legacy control sequence.
    ///
    /// Format: `ESC X <data> ST`
    ///
    /// SOS is a rarely used control function from ISO 6429. It marks the start
    /// of a control string that is terminated by ST (ESC \). The contents are
    /// returned as raw bytes.
    AnsiSOS(AnsiStartOfString),

    /// String Terminator - marks the end of a string control sequence.
    ///
    /// Format: `ESC \`
    ///
    /// ST is used to terminate string-type control sequences (OSC, DCS, SOS, PM, APC).
    /// When encountered outside of a string sequence context, it's returned as a
    /// standalone result with empty data.
    AnsiST,

    /// Privacy Message - a control sequence for private data.
    ///
    /// Format: `ESC ^ <data> ST`
    ///
    /// PM is a control function from ISO 6429 used to delimit privacy messages.
    /// The sequence is terminated by ST (ESC \) and the contents are returned
    /// as raw bytes.
    AnsiPM(AnsiPrivacyMessage),

    /// Application Program Command - a control sequence for application-specific commands.
    ///
    /// Format: `ESC _ <data> ST`
    ///
    /// APC sequences allow applications to send custom commands through the
    /// terminal. The sequence is terminated by ST (ESC \), and the contents are
    /// returned as raw bytes for application-specific interpretation.
    AnsiAPC(AnsiApplicationProgramCommand),

    // Telnet Commands
    /// No Operation - Does nothing but sends an IAC command.
    ///
    /// Format: `IAC NOP` (0xFF 0xF1)
    ///
    /// This command is often used as a keep-alive or to test the connection.
    NoOperation,

    /// End of urgent Data Stream - Marks the end of urgent data.
    ///
    /// Format: `IAC DM` (0xFF 0xF2)
    ///
    /// Used to synchronize urgent and normal data streams in the connection.
    DataMark,

    /// Break - Operator pressed the Break key or the Attention key.
    ///
    /// Format: `IAC BRK` (0xFF 0xF3)
    ///
    /// Sends an interrupt signal to the remote system, typically used to interrupt
    /// a running process on the server.
    Break,

    /// Interrupt Process - Request immediate interrupt of the current process.
    ///
    /// Format: `IAC IP` (0xFF 0xF4)
    ///
    /// Sends a stronger interrupt signal than Break, typically mapped to SIGINT in Unix-like systems.
    InterruptProcess,

    /// Cancel Output - Request that the remote system cancel output to the client.
    ///
    /// Format: `IAC AO` (0xFF 0xF5)
    ///
    /// Used when the client no longer wants to receive output from the remote process.
    AbortOutput,

    /// Are You There - Request acknowledgment from the remote system.
    ///
    /// Format: `IAC AYT` (0xFF 0xF6)
    ///
    /// Used to test if the connection is still alive or if the remote host is responding.
    AreYouThere,

    /// Erase Character - Request that the operator erase the previous character.
    ///
    /// Format: `IAC EC` (0xFF 0xF7)
    ///
    /// Equivalent to sending a backspace or delete request to the remote system.
    EraseCharacter,

    /// Erase Line - Request that the operator erase the previous line.
    ///
    /// Format: `IAC EL` (0xFF 0xF8)
    ///
    /// Requests the remote system to clear the current input line.
    EraseLine,

    /// Go Ahead - End of input for half-duplex connections.
    ///
    /// Format: `IAC GA` (0xFF 0xF9)
    ///
    /// Used in half-duplex mode to indicate that the sender has finished transmitting
    /// and the receiver may now send data. Rarely used in modern systems.
    GoAhead,

    /// End of Record - Marks the end of a prompt.
    ///
    /// Format: `IAC EOR` (0xFF 0xEF)
    ///
    /// Used by MUD servers to mark the end of a prompt. A prompt is considered
    /// any line that does not end with \r\n. This allows clients to distinguish
    /// between regular output and prompts that require user input.
    EndOfRecord,

    // Telnet Subnegotation Messages
    /// Generic Mud Communication Protocol
    GMCP(GmcpMessage),
    /// Mud Server Data Protocol
    MSDP(MudServerData),
    /// Mud Server Status Protocol
    MSSP(MudServerStatus),
}

// From trait implementations for convenient conversions
impl From<String> for TerminalCommand {
    fn from(s: String) -> Self {
        TerminalCommand::Text(s)
    }
}

impl From<&str> for TerminalCommand {
    fn from(s: &str) -> Self {
        TerminalCommand::Text(s.to_string())
    }
}

impl From<char> for TerminalCommand {
    fn from(c: char) -> Self {
        TerminalCommand::Char(c)
    }
}

impl From<Vec<u8>> for TerminalCommand {
    fn from(bytes: Vec<u8>) -> Self {
        TerminalCommand::Bytes(bytes)
    }
}

impl From<&[u8]> for TerminalCommand {
    fn from(bytes: &[u8]) -> Self {
        TerminalCommand::Bytes(bytes.to_vec())
    }
}

impl From<AnsiControlCode> for TerminalCommand {
    fn from(code: AnsiControlCode) -> Self {
        TerminalCommand::Control(code)
    }
}

impl From<AnsiControlSequenceIntroducer> for TerminalCommand {
    fn from(csi: AnsiControlSequenceIntroducer) -> Self {
        TerminalCommand::AnsiCSI(csi)
    }
}

impl From<AnsiSelectGraphicRendition> for TerminalCommand {
    fn from(sgr: AnsiSelectGraphicRendition) -> Self {
        TerminalCommand::AnsiSGR(sgr)
    }
}

impl From<AnsiOperatingSystemCommand> for TerminalCommand {
    fn from(osc: AnsiOperatingSystemCommand) -> Self {
        TerminalCommand::AnsiOSC(osc)
    }
}

impl From<AnsiDeviceControlString> for TerminalCommand {
    fn from(dcs: AnsiDeviceControlString) -> Self {
        TerminalCommand::AnsiDCS(dcs)
    }
}

impl From<AnsiStartOfString> for TerminalCommand {
    fn from(sos: AnsiStartOfString) -> Self {
        TerminalCommand::AnsiSOS(sos)
    }
}

impl From<AnsiPrivacyMessage> for TerminalCommand {
    fn from(pm: AnsiPrivacyMessage) -> Self {
        TerminalCommand::AnsiPM(pm)
    }
}

impl From<AnsiApplicationProgramCommand> for TerminalCommand {
    fn from(apc: AnsiApplicationProgramCommand) -> Self {
        TerminalCommand::AnsiAPC(apc)
    }
}

impl From<AnsiSequence> for TerminalCommand {
    fn from(seq: AnsiSequence) -> Self {
        match seq {
            AnsiSequence::Character(c) => TerminalCommand::Char(c),
            AnsiSequence::Unicode(c) => TerminalCommand::Char(c),
            AnsiSequence::AnsiControlCode(code) => TerminalCommand::Control(code),
            AnsiSequence::AnsiEscape => TerminalCommand::AnsiEscape,
            AnsiSequence::AnsiCSI(csi) => TerminalCommand::AnsiCSI(csi),
            AnsiSequence::AnsiSGR(sgr) => TerminalCommand::AnsiSGR(sgr),
            AnsiSequence::AnsiOSC(osc) => TerminalCommand::AnsiOSC(osc),
            AnsiSequence::AnsiDCS(dcs) => TerminalCommand::AnsiDCS(dcs),
            AnsiSequence::AnsiSOS(sos) => TerminalCommand::AnsiSOS(sos),
            AnsiSequence::AnsiST => TerminalCommand::AnsiST,
            AnsiSequence::AnsiPM(pm) => TerminalCommand::AnsiPM(pm),
            AnsiSequence::AnsiAPC(apc) => TerminalCommand::AnsiAPC(apc),
            AnsiSequence::TelnetCommand(cmd) => {
                // Convert TelnetCommand to TerminalCommand
                use termionix_ansicodec::TelnetCommand as TC;
                match cmd {
                    TC::NoOperation => TerminalCommand::NoOperation,
                    TC::DataMark => TerminalCommand::DataMark,
                    TC::Break => TerminalCommand::Break,
                    TC::InterruptProcess => TerminalCommand::InterruptProcess,
                    TC::AbortOutput => TerminalCommand::AbortOutput,
                    TC::AreYouThere => TerminalCommand::AreYouThere,
                    TC::EraseCharacter => TerminalCommand::EraseCharacter,
                    TC::EraseLine => TerminalCommand::EraseLine,
                    TC::GoAhead => TerminalCommand::GoAhead,
                    TC::EndOfRecord => TerminalCommand::EndOfRecord,
                    // For subnegotiation commands, we'd need to handle them specially
                    // For now, just use NoOperation as a placeholder
                    _ => TerminalCommand::NoOperation,
                }
            }
        }
    }
}

impl From<GmcpMessage> for TerminalCommand {
    fn from(msg: GmcpMessage) -> Self {
        TerminalCommand::GMCP(msg)
    }
}

impl TerminalCommand {
    /// Create a character command
    pub fn char(c: char) -> Self {
        TerminalCommand::Char(c)
    }

    /// Create a text command from a string
    pub fn text<S: Into<String>>(s: S) -> Self {
        TerminalCommand::Text(s.into())
    }

    /// Create a bytes command
    pub fn bytes(b: Vec<u8>) -> Self {
        TerminalCommand::Bytes(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_send_break() {
        let cmd = TerminalCommand::Break;
        assert_eq!(cmd, TerminalCommand::Break);
    }

    #[test]
    fn test_command_send_interrupt_process() {
        let cmd = TerminalCommand::InterruptProcess;
        assert_eq!(cmd, TerminalCommand::InterruptProcess);
    }

    #[test]
    fn test_command_send_abort_output() {
        let cmd = TerminalCommand::AbortOutput;
        assert_eq!(cmd, TerminalCommand::AbortOutput);
    }

    #[test]
    fn test_command_send_are_you_there() {
        let cmd = TerminalCommand::AreYouThere;
        assert_eq!(cmd, TerminalCommand::AreYouThere);
    }

    #[test]
    fn test_command_send_erase_character() {
        let cmd = TerminalCommand::EraseCharacter;
        assert_eq!(cmd, TerminalCommand::EraseCharacter);
    }

    #[test]
    fn test_command_send_erase_line() {
        let cmd = TerminalCommand::EraseLine;
        assert_eq!(cmd, TerminalCommand::EraseLine);
    }

    #[test]
    fn test_command_clone() {
        let cmd1 = TerminalCommand::Break;
        let cmd2 = cmd1.clone();
        assert_eq!(cmd1, cmd2);
    }

    #[test]
    fn test_command_clone_control() {
        let cmd1 = TerminalCommand::Break;
        let cmd2 = cmd1.clone();
        assert_eq!(cmd1, cmd2);
    }

    #[test]
    fn test_command_debug() {
        let cmd = TerminalCommand::Break;
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("SendBreak"));
    }

    #[test]
    fn test_command_equality() {
        let cmd1 = TerminalCommand::Break;
        let cmd2 = TerminalCommand::Break;
        let cmd3 = TerminalCommand::InterruptProcess;

        assert_eq!(cmd1, cmd2);
        assert_ne!(cmd1, cmd3);
    }

    #[test]
    fn test_all_command_variants() {
        let commands = vec![
            TerminalCommand::Break,
            TerminalCommand::InterruptProcess,
            TerminalCommand::AbortOutput,
            TerminalCommand::AreYouThere,
            TerminalCommand::EraseCharacter,
            TerminalCommand::EraseLine,
        ];

        assert_eq!(commands.len(), 6);

        // Verify all are unique
        for (i, cmd1) in commands.iter().enumerate() {
            for (j, cmd2) in commands.iter().enumerate() {
                if i == j {
                    assert_eq!(cmd1, cmd2);
                } else {
                    assert_ne!(cmd1, cmd2);
                }
            }
        }
    }

    #[test]
    fn test_command_match_pattern() {
        let cmd = TerminalCommand::Break;

        match cmd {
            TerminalCommand::Break => {
                // Success
            }
            _ => panic!("Pattern match failed"),
        }
    }

    #[test]
    fn test_command_exhaustive_match() {
        let commands = vec![
            TerminalCommand::Break,
            TerminalCommand::InterruptProcess,
            TerminalCommand::AbortOutput,
            TerminalCommand::AreYouThere,
            TerminalCommand::EraseCharacter,
            TerminalCommand::EraseLine,
        ];

        for cmd in commands {
            let _result = match cmd {
                TerminalCommand::Text(_) => "text",
                TerminalCommand::Char(_) => "char",
                TerminalCommand::Bytes(_) => "bytes",
                TerminalCommand::Break => "break",
                TerminalCommand::InterruptProcess => "interrupt",
                TerminalCommand::AbortOutput => "abort",
                TerminalCommand::AreYouThere => "ayt",
                TerminalCommand::EraseCharacter => "erase_char",
                TerminalCommand::EraseLine => "erase_line",
                TerminalCommand::Control(_) => "control",
                TerminalCommand::AnsiEscape => "escape",
                TerminalCommand::AnsiCSI(_) => "csi",
                TerminalCommand::AnsiSGR(_) => "sgr",
                TerminalCommand::AnsiOSC(_) => "osc",
                TerminalCommand::AnsiDCS(_) => "dcs",
                TerminalCommand::AnsiSOS(_) => "sos",
                TerminalCommand::AnsiST => "st",
                TerminalCommand::AnsiPM(_) => "pm",
                TerminalCommand::AnsiAPC(_) => "apc",
                TerminalCommand::NoOperation => "no_operation",
                TerminalCommand::DataMark => "datamark",
                TerminalCommand::GoAhead => "go_ahead",
                TerminalCommand::EndOfRecord => "end_of_record",
                TerminalCommand::GMCP(_) => "gmccp",
                TerminalCommand::MSDP(_) => "mud_server_data",
                TerminalCommand::MSSP(_) => "mud_server_status",
            };
        }
    }

    #[test]
    fn test_command_text_variant() {
        let cmd = TerminalCommand::text("Hello");
        match cmd {
            TerminalCommand::Text(s) => assert_eq!(s, "Hello"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_command_char_variant() {
        let cmd = TerminalCommand::char('A');
        match cmd {
            TerminalCommand::Char(c) => assert_eq!(c, 'A'),
            _ => panic!("Expected Char variant"),
        }
    }

    #[test]
    fn test_command_bytes_variant() {
        let cmd = TerminalCommand::bytes(vec![1, 2, 3]);
        match cmd {
            TerminalCommand::Bytes(b) => assert_eq!(b, vec![1, 2, 3]),
            _ => panic!("Expected Bytes variant"),
        }
    }
}
