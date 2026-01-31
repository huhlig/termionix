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

use crate::AnsiResult;
pub use crate::style::{
    AnsiSelectGraphicRendition, Blink, Color, Font, Ideogram, Intensity, SGRParameter, Script,
    Underline,
};
use termionix_telnetcodec::{TelnetArgument, TelnetOption, TelnetSide};
use tokio_util::bytes::BufMut;

/// Ansi Sequence represents a series of bytes read from a [TelnetCodec] which translates to a valid
/// Ansi Sequence. Sequences include individual characters, control commands, etc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnsiSequence {
    /// A single ASCII character in the range 0x20-0x7E (printable ASCII).
    ///
    /// This excludes escape sequences (ESC), control codes, and multi-byte UTF-8
    /// characters. These are standard printable ASCII characters that can be
    /// directly rendered or processed as text.
    Character(char),

    /// A multi-byte UTF-8 encoded Unicode character.
    ///
    /// This is returned after successfully parsing a 2-4 byte UTF-8 sequence.
    /// Characters in the range U+0080 and above are represented here. Invalid
    /// UTF-8 sequences are replaced with the Unicode replacement character U+FFFD.
    ///
    /// # UTF-8 Byte Sequences
    /// - 2-byte: 0xC0-0xDF (followed by 1 continuation byte)
    /// - 3-byte: 0xE0-0xEF (followed by 2 continuation bytes)
    /// - 4-byte: 0xF0-0xF7 (followed by 3 continuation bytes)
    Unicode(char),

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

    /// Passthrough Telnet Command
    TelnetCommand(TelnetCommand),
}

impl AnsiSequence {
    /// Returns the encoded byte length of this ANSI sequence.
    ///
    /// This method calculates the total number of bytes that will be produced when this
    /// sequence is encoded to its byte representation. The length includes all escape codes,
    /// parameters, and terminators, but represents the wire format size, not the semantic content.
    ///
    /// # Returns
    ///
    /// The total byte length in the encoded form. For simple sequences like a single character,
    /// this matches the character's UTF-8 byte length. For complex escape sequences, this includes
    /// all prefix, parameter, and suffix bytes.
    ///
    /// # Performance
    ///
    /// This is an O(1) or O(n) operation depending on variant:
    /// - **O(1)** for variants with fixed sizes (Character, Unicode, AnsiEscape, AnsiST)
    /// - **O(n)** for variants with dynamic sizes that delegate to nested types (Control, CSI, SGR, etc.)
    ///   where n is the complexity of the nested structure
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::AnsiSequence;
    ///
    /// // Single ASCII character
    /// let seq = AnsiSequence::Character('A');
    /// assert_eq!(seq.len(), 1);
    ///
    /// // Unicode character (3 bytes)
    /// let seq = AnsiSequence::Unicode('世');
    /// assert_eq!(seq.len(), 3);
    ///
    /// // Control code (1 byte)
    /// let seq = AnsiSequence::Control(AnsiControlCode::BEL);
    /// assert_eq!(seq.len(), 1);
    ///
    /// // Escape sequence (5 bytes: ESC [ 3 1 m)
    /// let seq = AnsiSequence::AnsiSGR(AnsiSelectGraphicRendition::Unknown(vec![b'3', b'1']));
    /// assert_eq!(seq.len(), 4 + 2); // ESC [ ... m + data
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Buffer allocation**: Pre-allocate buffers for encoding before calling `encode()`
    /// - **Progress tracking**: Calculate total bytes to be written
    /// - **Wire protocol**: Determine message sizes for network transmission
    /// - **Statistics**: Analyze the size distribution of sequences
    pub fn len(&self) -> usize {
        match self {
            AnsiSequence::Character(c) => c.len_utf8(),
            AnsiSequence::Unicode(c) => c.len_utf8(),
            AnsiSequence::Control(code) => code.len(),
            AnsiSequence::AnsiEscape => 1,
            AnsiSequence::AnsiCSI(csi) => csi.len(),
            AnsiSequence::AnsiSGR(sgr) => sgr.len(None),
            AnsiSequence::AnsiOSC(osc) => osc.len(),
            AnsiSequence::AnsiDCS(dcs) => dcs.len(),
            AnsiSequence::AnsiSOS(sos) => sos.len(),
            AnsiSequence::AnsiST => 2, // ESC \
            AnsiSequence::AnsiPM(pm) => pm.len(),
            AnsiSequence::AnsiAPC(apc) => apc.len(),
            AnsiSequence::TelnetCommand(cmd) => cmd.len(),
        }
    }

    /// Encode this ANSI sequence to a `BufMut` buffer.
    ///
    /// This method encodes the sequence into its byte representation and writes it to the
    /// provided mutable buffer. The buffer will be advanced by the number of bytes written.
    /// This is the preferred method for encoding into buffers that implement `BufMut`.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer implementing `BufMut` (e.g., `BytesMut`, `Vec<u8>` via
    ///   `BufMut::writer()`, etc.). The buffer is advanced by the bytes written.
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success, containing the exact number of bytes that were
    /// written to the buffer. Returns an `AnsiResult` error if encoding fails (rare, typically
    /// only due to I/O errors in the underlying writer).
    ///
    /// # Performance
    ///
    /// This method is O(n) where n is the length of the encoded sequence. It performs a single
    /// pass through the data, writing directly to the buffer without intermediate allocations.
    ///
    /// # Buffer Behavior
    ///
    /// - The buffer is only written to; existing content is preserved
    /// - The buffer's write position is advanced automatically
    /// - No bounds checking is performed; the buffer must have sufficient capacity
    /// - For `BytesMut`, automatic growth occurs if needed
    ///
    /// # Examples
    ///
    /// ## Basic Encoding
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiSequence;
    /// use bytes::BytesMut;
    ///
    /// let seq = AnsiSequence::Character('X');
    /// let mut buffer = BytesMut::new();
    /// let written = seq.encode(&mut buffer).unwrap();
    /// assert_eq!(written, 1);
    /// assert_eq!(&buffer[..], b"X");
    /// ```
    ///
    /// ## Encoding Multiple Sequences
    ///
    /// ```rust
    /// use termionix_ansicodec::{AnsiSequence, ansi::AnsiControlCode};
    /// use bytes::BytesMut;
    ///
    /// let mut buffer = BytesMut::new();
    ///
    /// // Encode "Hello"
    /// for c in "Hello".chars() {
    ///     AnsiSequence::Character(c).encode(&mut buffer).unwrap();
    /// }
    ///
    /// // Encode a newline
    /// AnsiSequence::Control(AnsiControlCode::LF).encode(&mut buffer).unwrap();
    ///
    /// assert_eq!(&buffer[..], b"Hello\n");
    /// ```
    ///
    /// ## Encoding ANSI Escape Sequences
    ///
    /// ```rust
    /// use termionix_ansicodec::{AnsiSequence, ansi::AnsiSelectGraphicRendition};
    /// use bytes::BytesMut;
    ///
    /// let sgr = AnsiSelectGraphicRendition(vec![31]); // Red
    /// let seq = AnsiSequence::AnsiSGR(sgr);
    /// let mut buffer = BytesMut::new();
    /// seq.encode(&mut buffer).unwrap();
    /// // Result: ESC[31m
    /// ```
    ///
    /// # Error Handling
    ///
    /// While encoding to `BufMut` rarely fails, errors can occur if:
    /// - The underlying buffer is unable to allocate additional capacity
    /// - The writer encounters an I/O error (rare for in-memory buffers)
    ///
    /// # See Also
    ///
    /// - [`write()`](AnsiSequence::write) - Write to a `std::io::Write` trait object
    /// - [`len()`](AnsiSequence::len) - Get the encoded byte length without encoding
    /// - [`Display`](std::fmt::Display) - Convert to a string representation
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> AnsiResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Write this ANSI sequence to a `std::io::Write` writer.
    ///
    /// This method performs the actual encoding of the sequence into its byte representation
    /// and writes the bytes to the provided writer. This is the low-level method that both
    /// `encode()` and `Display` delegate to internally.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write` (e.g., `File`, `BufWriter`,
    ///   `Vec<u8>`, a socket, etc.). The writer's internal position is advanced by the bytes
    ///   written.
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success, containing the exact number of bytes written.
    /// Returns `std::io::Error` if the writer fails (e.g., disk full, broken pipe, etc.).
    ///
    /// # Performance
    ///
    /// This is O(n) where n is the length of the encoded sequence. It performs one or more
    /// `write_all()` calls to the writer, depending on the sequence complexity.
    ///
    /// # Behavior by Variant
    ///
    /// - **Character**: Encodes as 1-4 bytes (UTF-8 encoded character)
    /// - **Unicode**: Encodes as 2-4 bytes (UTF-8 encoded character)
    /// - **Control**: Encodes as 1 byte (the control code byte)
    /// - **AnsiEscape**: Writes the ESC byte (0x1B)
    /// - **AnsiCSI**: Delegates to the CSI command's `write()` method
    /// - **AnsiSGR**: Writes `ESC [ <data> m` format
    /// - **AnsiOSC**: Writes `ESC ] <data> ST` format
    /// - **AnsiDCS**: Writes `ESC P <data> ST` format
    /// - **AnsiSOS**: Writes `ESC X <data> ST` format
    /// - **AnsiST**: Writes the ST terminator sequence
    /// - **AnsiPM**: Writes `ESC ^ <data> ST` format
    /// - **AnsiAPC**: Writes `ESC _ <data> ST` format
    /// - **TelnetCommand**: Delegates to the Telnet command's `write()` method
    ///
    /// # Examples
    ///
    /// ## Writing to a Vector
    ///
    /// ```rust
    /// use termionix_ansicodec::AnsiSequence;
    ///
    /// let seq = AnsiSequence::Character('A');
    /// let mut output = Vec::new();
    /// let written = seq.write(&mut output).unwrap();
    /// assert_eq!(written, 1);
    /// assert_eq!(output, b"A");
    /// ```
    ///
    /// ## Writing to a File
    ///
    /// ```rust
    /// use termionix_ansicodec::AnsiSequence;
    /// use std::io::Cursor;
    ///
    /// let seq = AnsiSequence::Character('X');
    /// let mut cursor = Cursor::new(Vec::new());
    /// let written = seq.write(&mut cursor).unwrap();
    /// assert_eq!(written, 1);
    /// ```
    ///
    /// ## Writing Multiple Sequences
    ///
    /// ```rust
    /// use termionix_ansicodec::{AnsiSequence, AnsiControlCode};
    ///
    /// let mut output = Vec::new();
    ///
    /// AnsiSequence::Character('H').write(&mut output).unwrap();
    /// AnsiSequence::Character('i').write(&mut output).unwrap();
    /// AnsiSequence::Control(AnsiControlCode::LF).write(&mut output).unwrap();
    ///
    /// assert_eq!(output, b"Hi\n");
    /// ```
    ///
    /// ## Streaming to stdout
    ///
    /// ```rust
    /// use termionix_ansicodec::AnsiSequence;
    /// use std::io::stdout;
    ///
    /// let seq = AnsiSequence::Character('!');
    /// // This would write to the terminal
    /// // seq.write(&mut stdout()).unwrap();
    /// ```
    ///
    /// # Error Handling
    ///
    /// Errors from the writer are propagated directly. Common errors include:
    /// - `io::ErrorKind::WriteZero`: Writer refused to write any bytes
    /// - `io::ErrorKind::Interrupted`: I/O operation was interrupted
    /// - `io::ErrorKind::BrokenPipe`: Broken pipe when writing to a process or network
    /// - `io::ErrorKind::PermissionDenied`: No write permission on the target
    ///
    /// # Performance Considerations
    ///
    /// - For repeated writes to the same destination, consider wrapping in `BufWriter`
    /// - Writing to `Vec<u8>` is typically faster than writing to disk or network
    /// - The method makes minimal allocations (typically none for fixed-size sequences)
    ///
    /// # See Also
    ///
    /// - [`encode()`](AnsiSequence::encode) - Encode to a `BufMut` buffer
    /// - [`len()`](AnsiSequence::len) - Get the byte length without writing
    /// - [`Display`](std::fmt::Display) - Convert to a string representation
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        match self {
            AnsiSequence::Character(c) => {
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf);
                writer.write_all(encoded.as_bytes())?;
                Ok(encoded.len())
            }
            AnsiSequence::Unicode(c) => {
                let mut buf = [0u8; 4];
                let encoded = c.encode_utf8(&mut buf);
                writer.write_all(encoded.as_bytes())?;
                Ok(encoded.len())
            }
            AnsiSequence::Control(code) => code.write(writer),
            AnsiSequence::AnsiEscape => {
                writer.write_all(&[0x1B])?;
                Ok(1)
            }
            AnsiSequence::AnsiCSI(csi) => csi.write(writer),
            AnsiSequence::AnsiSGR(sgr) => sgr.write(writer, None),
            AnsiSequence::AnsiOSC(osc) => osc.write(writer),
            AnsiSequence::AnsiDCS(dcs) => dcs.write(writer),
            AnsiSequence::AnsiSOS(sos) => sos.write(writer),
            AnsiSequence::AnsiST => {
                writer.write_all(&[0x1B, 0x5C])?;
                Ok(2)
            }
            AnsiSequence::AnsiPM(pm) => pm.write(writer),
            AnsiSequence::AnsiAPC(apc) => apc.write(writer),
            AnsiSequence::TelnetCommand(cmd) => cmd.write(writer),
        }
    }
}

impl std::fmt::Display for AnsiSequence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnsiSequence::Character(c) => write!(f, "{}", c),
            AnsiSequence::Unicode(c) => write!(f, "{}", c),
            AnsiSequence::Control(code) => write!(f, "{}", code),
            AnsiSequence::AnsiEscape => write!(f, "\x1b"),
            AnsiSequence::AnsiCSI(csi) => write!(f, "{}", csi),
            AnsiSequence::AnsiSGR(sgr) => write!(f, "{}", sgr),
            AnsiSequence::AnsiOSC(osc) => write!(f, "{}", osc),
            AnsiSequence::AnsiDCS(dcs) => write!(f, "{}", dcs),
            AnsiSequence::AnsiSOS(sos) => write!(f, "{}", sos),
            AnsiSequence::AnsiST => write!(f, "\x1b\\"),
            AnsiSequence::AnsiPM(pm) => write!(f, "{}", pm),
            AnsiSequence::AnsiAPC(apc) => write!(f, "{}", apc),
            AnsiSequence::TelnetCommand(cmd) => write!(f, "{}", cmd),
        }
    }
}

/// Telnet command types
///
/// This enum represents various Telnet protocol commands as defined in RFC 854 and related RFCs.
/// Telnet commands are used to control the connection and request specific operations from the
/// remote host or terminal.
///
/// # Format
///
/// Telnet commands are typically encoded as `IAC <command>` where IAC is the Interpret As Command
/// byte (0xFF). Some commands (like Negotiation and Subnegotiation) require additional parameters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TelnetCommand {
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

    /// Telnet Negotiation Result.
    ///
    /// # Arguments
    ///
    /// - `TelnetOption` - The option being negotiated (e.g., ECHO, LINEMODE, etc.)
    /// - `OptionSide` - Which side the option is being updated
    /// - `bool` - Enabled or Not
    OptionStatus(TelnetOption, TelnetSide, bool),

    /// Telnet Subnegotiation - Send subnegotiation data for a negotiated option.
    ///
    /// Format: `IAC SB <option> <data...> IAC SE` (0xFF 0xFA <option> <data...> 0xFF 0xF0)
    ///
    /// # Arguments
    ///
    /// - `TelnetArgument` - Contains the option and associated data for that option
    ///
    /// After an option is negotiated via Negotiation, subnegotiation allows sending
    /// additional parameters or data specific to that option.
    Subnegotiation(TelnetArgument),
}

impl TelnetCommand {
    /// Returns the encoded byte length of this Telnet command.
    ///
    /// This method calculates the number of bytes that will be produced when the command
    /// is encoded to its byte representation. Most Telnet commands are encoded as `IAC <command>`
    /// (3 bytes total: 0xFF + command_byte), while Subnegotiation commands have variable
    /// length depending on their data payload.
    ///
    /// # Returns
    ///
    /// The number of bytes in the encoded form:
    /// - Simple commands: 3 bytes (IAC + command + option/parameter)
    /// - Subnegotiation: 2 + data_length + 2 (IAC SB + option + data + IAC SE)
    ///
    /// # Performance
    ///
    /// This is O(1) for simple commands and O(n) for Subnegotiation where n is the data length.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::TelnetCommand;
    ///
    /// // Simple commands are always 3 bytes
    /// assert_eq!(TelnetCommand::NoOperation.len(), 3);
    /// assert_eq!(TelnetCommand::Break.len(), 3);
    /// assert_eq!(TelnetCommand::AreYouThere.len(), 3);
    ///
    /// // Subnegotiation includes the data length
    /// // Subnegotiation(arg) => 2 (IAC SB) + 1 (option) + arg.data().len() + 2 (IAC SE)
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Buffer allocation**: Pre-allocate space before encoding
    /// - **Protocol implementation**: Calculate message sizes for Telnet protocol
    /// - **Stream management**: Track total bytes in a command sequence
    pub fn len(&self) -> usize {
        match self {
            TelnetCommand::NoOperation => 3,
            TelnetCommand::DataMark => 3,
            TelnetCommand::Break => 3,
            TelnetCommand::InterruptProcess => 3,
            TelnetCommand::AbortOutput => 3,
            TelnetCommand::AreYouThere => 3,
            TelnetCommand::EraseCharacter => 3,
            TelnetCommand::EraseLine => 3,
            TelnetCommand::GoAhead => 3,
            TelnetCommand::EndOfRecord => 3,
            TelnetCommand::OptionStatus(_, _, _) => 0,
            TelnetCommand::Subnegotiation(arg) => 2 + arg.len(),
        }
    }

    /// Encode this Telnet command to a `BufMut` buffer.
    ///
    /// This method encodes the command into its wire format (Telnet protocol bytes) and writes
    /// it to the provided mutable buffer. The buffer's write position is automatically advanced.
    /// This is the preferred method for encoding into buffers that implement `BufMut`.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer implementing `BufMut`. The buffer is advanced by the bytes written.
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success. Returns an `AnsiResult` error if encoding fails.
    ///
    /// # Wire Format
    ///
    /// Commands are encoded as:
    /// - **Simple commands**: `0xFF <command_byte>` (3 bytes total with optional parameter)
    /// - **Subnegotiation**: `0xFF 0xFA <option> <data...> 0xFF 0xF0`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::TelnetCommand;
    /// use bytes::BytesMut;
    ///
    /// let cmd = TelnetCommand::Break;
    /// let mut buffer = BytesMut::new();
    /// let written = cmd.encode(&mut buffer).unwrap();
    /// assert_eq!(written, 3);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`write()`](TelnetCommand::write) - Write to a `std::io::Write` trait object
    /// - [`len()`](TelnetCommand::len) - Get the encoded byte length
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> AnsiResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Write this Telnet command to a `std::io::Write` writer.
    ///
    /// This method performs the actual encoding of the Telnet command into its wire format
    /// and writes it to the provided writer. This is the low-level method that `encode()` delegates to.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success, containing the number of bytes written.
    /// Returns `std::io::Error` if writing fails.
    ///
    /// # Encoding Behavior
    ///
    /// Each command is encoded according to RFC 854:
    ///
    /// - **NoOperation**: `IAC NOP` → `0xFF 0xF1`
    /// - **DataMark**: `IAC DM` → `0xFF 0xF2`
    /// - **Break**: `IAC BRK` → `0xFF 0xF3`
    /// - **InterruptProcess**: `IAC IP` → `0xFF 0xF4`
    /// - **AbortOutput**: `IAC AO` → `0xFF 0xF5`
    /// - **AreYouThere**: `IAC AYT` → `0xFF 0xF6`
    /// - **EraseCharacter**: `IAC EC` → `0xFF 0xF7`
    /// - **EraseLine**: `IAC EL` → `0xFF 0xF8`
    /// - **GoAhead**: `IAC GA` → `0xFF 0xF9`
    /// - **Negotiation**: `IAC <request> <option>` → `0xFF <request_byte> <option_byte>`
    /// - **Subnegotiation**: `IAC SB <option> <data...> IAC SE` → `0xFF 0xFA <option> <data...> 0xFF 0xF0`
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::TelnetCommand;
    ///
    /// let cmd = TelnetCommand::AreYouThere;
    /// let mut output = Vec::new();
    /// let written = cmd.write(&mut output).unwrap();
    /// assert_eq!(written, 3);
    /// assert_eq!(output, vec![0xFF, 0xF6]);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`encode()`](TelnetCommand::encode) - Encode to a `BufMut` buffer
    /// - [`len()`](TelnetCommand::len) - Get the encoded byte length
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        match self {
            TelnetCommand::NoOperation => {
                writer.write_all(&[0xFF, 0xF1])?;
                Ok(3)
            }
            TelnetCommand::DataMark => {
                writer.write_all(&[0xFF, 0xF2])?;
                Ok(3)
            }
            TelnetCommand::Break => {
                writer.write_all(&[0xFF, 0xF3])?;
                Ok(3)
            }
            TelnetCommand::InterruptProcess => {
                writer.write_all(&[0xFF, 0xF4])?;
                Ok(3)
            }
            TelnetCommand::AbortOutput => {
                writer.write_all(&[0xFF, 0xF5])?;
                Ok(3)
            }
            TelnetCommand::AreYouThere => {
                writer.write_all(&[0xFF, 0xF6])?;
                Ok(3)
            }
            TelnetCommand::EraseCharacter => {
                writer.write_all(&[0xFF, 0xF7])?;
                Ok(3)
            }
            TelnetCommand::EraseLine => {
                writer.write_all(&[0xFF, 0xF8])?;
                Ok(3)
            }
            TelnetCommand::GoAhead => {
                writer.write_all(&[0xFF, 0xF9])?;
                Ok(3)
            }
            TelnetCommand::EndOfRecord => {
                writer.write_all(&[0xFF, 0xEF])?;
                Ok(3)
            }
            TelnetCommand::OptionStatus(_option, _side, _enabled) => Ok(3),
            TelnetCommand::Subnegotiation(arg) => {
                writer.write_all(&[0xFF, 0xFA])?;
                writer.write_all(&[arg.option().to_u8()])?;
                let len = arg.write(writer)?;
                writer.write_all(&[0xFF, 0xF0])?;
                Ok(5 + len)
            }
        }
    }
}

impl std::fmt::Display for TelnetCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelnetCommand::NoOperation => write!(f, "$NOP$"),
            TelnetCommand::DataMark => write!(f, "$DM$"),
            TelnetCommand::Break => write!(f, "$BRK$"),
            TelnetCommand::InterruptProcess => write!(f, "$INT$"),
            TelnetCommand::AbortOutput => write!(f, "$ABRT$"),
            TelnetCommand::AreYouThere => write!(f, "$AYT$"),
            TelnetCommand::EraseCharacter => write!(f, "$EC$"),
            TelnetCommand::EraseLine => write!(f, "$EL$"),
            TelnetCommand::GoAhead => write!(f, "$GA$"),
            TelnetCommand::EndOfRecord => write!(f, "$EOR$"),
            TelnetCommand::OptionStatus(option, side, enabled) => {
                write!(f, "$TELNEG({},{}, {})$", option, side, enabled)
            }
            TelnetCommand::Subnegotiation(arg) => {
                write!(f, "$TELSUB({}, {})$", arg.option(), arg)
            }
        }
    }
}

/// ISO 6429 Control Codes (C0 and C1 sets)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiControlCode {
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

impl AnsiControlCode {
    /// Returns the byte length of a single control code.
    ///
    /// Control codes are always single bytes, so this method always returns 1. This method
    /// exists for consistency with other ANSI sequence types that have variable lengths.
    ///
    /// # Returns
    ///
    /// Always returns 1, as all control codes (C0, C1, and DEL) are represented by a single byte.
    ///
    /// # Performance
    ///
    /// This is an O(1) operation that simply returns a constant.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiControlCode;
    ///
    /// assert_eq!(AnsiControlCode::NUL.len(), 1);
    /// assert_eq!(AnsiControlCode::LF.len(), 1);
    /// assert_eq!(AnsiControlCode::BEL.len(), 1);
    /// assert_eq!(AnsiControlCode::DEL.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        1
    }

    /// Encode this control code to a `BufMut` buffer.
    ///
    /// This method encodes the control code as a single byte and writes it to the provided
    /// mutable buffer. The buffer's write position is automatically advanced.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer implementing `BufMut`
    ///
    /// # Returns
    ///
    /// Returns `Ok(1)` on success (exactly 1 byte is written). Returns an `AnsiResult` error
    /// if the buffer write fails.
    ///
    /// # Performance
    ///
    /// This is an O(1) operation that writes a single byte.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bytes::BytesMut;
    /// use termionix_ansicodec::ansi::AnsiControlCode;
    ///
    /// let code = AnsiControlCode::BEL;
    /// let mut buffer = BytesMut::new();
    /// let written = code.encode(&mut buffer).unwrap();
    /// assert_eq!(written, 1);
    /// assert_eq!(&buffer[..], b"\x07");
    /// ```
    ///
    /// # See Also
    ///
    /// - [`write()`](AnsiControlCode::write) - Write to a `std::io::Write` trait object
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> AnsiResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Write this control code to a `std::io::Write` writer.
    ///
    /// This method performs the actual encoding of the control code into its single byte
    /// representation and writes it to the provided writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// Returns `Ok(1)` on success (exactly 1 byte written). Returns `std::io::Error` if writing fails.
    ///
    /// # Encoding
    ///
    /// Converts the control code to its byte representation using `to_byte()` and writes
    /// the single byte to the writer.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiControlCode;
    ///
    /// let code = AnsiControlCode::LF;
    /// let mut output = Vec::new();
    /// let written = code.write(&mut output).unwrap();
    /// assert_eq!(written, 1);
    /// assert_eq!(output, b"\n");
    /// ```
    ///
    /// # See Also
    ///
    /// - [`encode()`](AnsiControlCode::encode) - Encode to a `BufMut` buffer
    /// - [`to_byte()`](AnsiControlCode::to_byte) - Get the byte value
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        writer.write(&[self.to_byte()])
    }

    /// Convert control code to byte representation
    pub fn to_byte(&self) -> u8 {
        match self {
            // C0 control codes
            AnsiControlCode::NUL => 0x00,
            AnsiControlCode::SOH => 0x01,
            AnsiControlCode::STX => 0x02,
            AnsiControlCode::ETX => 0x03,
            AnsiControlCode::EOT => 0x04,
            AnsiControlCode::ENQ => 0x05,
            AnsiControlCode::ACK => 0x06,
            AnsiControlCode::BEL => 0x07,
            AnsiControlCode::BS => 0x08,
            AnsiControlCode::HT => 0x09,
            AnsiControlCode::LF => 0x0A,
            AnsiControlCode::VT => 0x0B,
            AnsiControlCode::FF => 0x0C,
            AnsiControlCode::CR => 0x0D,
            AnsiControlCode::SO => 0x0E,
            AnsiControlCode::SI => 0x0F,
            AnsiControlCode::DLE => 0x10,
            AnsiControlCode::DC1 => 0x11,
            AnsiControlCode::DC2 => 0x12,
            AnsiControlCode::DC3 => 0x13,
            AnsiControlCode::DC4 => 0x14,
            AnsiControlCode::NAK => 0x15,
            AnsiControlCode::SYN => 0x16,
            AnsiControlCode::ETB => 0x17,
            AnsiControlCode::CAN => 0x18,
            AnsiControlCode::EM => 0x19,
            AnsiControlCode::SUB => 0x1A,
            AnsiControlCode::FS => 0x1C,
            AnsiControlCode::GS => 0x1D,
            AnsiControlCode::RS => 0x1E,
            AnsiControlCode::US => 0x1F,
            AnsiControlCode::DEL => 0x7F,
            // C1 control codes
            AnsiControlCode::PAD => 0x80,
            AnsiControlCode::HOP => 0x81,
            AnsiControlCode::BPH => 0x82,
            AnsiControlCode::NBH => 0x83,
            AnsiControlCode::IND => 0x84,
            AnsiControlCode::NEL => 0x85,
            AnsiControlCode::SSA => 0x86,
            AnsiControlCode::ESA => 0x87,
            AnsiControlCode::HTS => 0x88,
            AnsiControlCode::HTJ => 0x89,
            AnsiControlCode::VTS => 0x8A,
            AnsiControlCode::PLD => 0x8B,
            AnsiControlCode::PLU => 0x8C,
            AnsiControlCode::RI => 0x8D,
            AnsiControlCode::SS2 => 0x8E,
            AnsiControlCode::SS3 => 0x8F,
            AnsiControlCode::DCS => 0x90,
            AnsiControlCode::PU1 => 0x91,
            AnsiControlCode::PU2 => 0x92,
            AnsiControlCode::STS => 0x93,
            AnsiControlCode::CCH => 0x94,
            AnsiControlCode::MW => 0x95,
            AnsiControlCode::SPA => 0x96,
            AnsiControlCode::EPA => 0x97,
            AnsiControlCode::SOS => 0x98,
            AnsiControlCode::SGCI => 0x99,
            AnsiControlCode::SCI => 0x9A,
            AnsiControlCode::CSI => 0x9B,
            AnsiControlCode::StC1 => 0x9C,
            AnsiControlCode::OscC1 => 0x9D,
            AnsiControlCode::PmC1 => 0x9E,
            AnsiControlCode::ApcC1 => 0x9F,
        }
    }

    /// Convert a byte to its corresponding control code
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            // C0 Control codes
            0x00 => Some(AnsiControlCode::NUL),
            0x01 => Some(AnsiControlCode::SOH),
            0x02 => Some(AnsiControlCode::STX),
            0x03 => Some(AnsiControlCode::ETX),
            0x04 => Some(AnsiControlCode::EOT),
            0x05 => Some(AnsiControlCode::ENQ),
            0x06 => Some(AnsiControlCode::ACK),
            0x07 => Some(AnsiControlCode::BEL),
            0x08 => Some(AnsiControlCode::BS),
            0x09 => Some(AnsiControlCode::HT),
            0x0A => Some(AnsiControlCode::LF),
            0x0B => Some(AnsiControlCode::VT),
            0x0C => Some(AnsiControlCode::FF),
            0x0D => Some(AnsiControlCode::CR),
            0x0E => Some(AnsiControlCode::SO),
            0x0F => Some(AnsiControlCode::SI),
            0x10 => Some(AnsiControlCode::DLE),
            0x11 => Some(AnsiControlCode::DC1),
            0x12 => Some(AnsiControlCode::DC2),
            0x13 => Some(AnsiControlCode::DC3),
            0x14 => Some(AnsiControlCode::DC4),
            0x15 => Some(AnsiControlCode::NAK),
            0x16 => Some(AnsiControlCode::SYN),
            0x17 => Some(AnsiControlCode::ETB),
            0x18 => Some(AnsiControlCode::CAN),
            0x19 => Some(AnsiControlCode::EM),
            0x1A => Some(AnsiControlCode::SUB),
            // 0x1B is ESC - handled separately
            0x1C => Some(AnsiControlCode::FS),
            0x1D => Some(AnsiControlCode::GS),
            0x1E => Some(AnsiControlCode::RS),
            0x1F => Some(AnsiControlCode::US),
            0x7F => Some(AnsiControlCode::DEL),

            // C1 Control codes (0x80-0x9F)
            0x80 => Some(AnsiControlCode::PAD),
            0x81 => Some(AnsiControlCode::HOP),
            0x82 => Some(AnsiControlCode::BPH),
            0x83 => Some(AnsiControlCode::NBH),
            0x84 => Some(AnsiControlCode::IND),
            0x85 => Some(AnsiControlCode::NEL),
            0x86 => Some(AnsiControlCode::SSA),
            0x87 => Some(AnsiControlCode::ESA),
            0x88 => Some(AnsiControlCode::HTS),
            0x89 => Some(AnsiControlCode::HTJ),
            0x8A => Some(AnsiControlCode::VTS),
            0x8B => Some(AnsiControlCode::PLD),
            0x8C => Some(AnsiControlCode::PLU),
            0x8D => Some(AnsiControlCode::RI),
            0x8E => Some(AnsiControlCode::SS2),
            0x8F => Some(AnsiControlCode::SS3),
            0x90 => Some(AnsiControlCode::DCS),
            0x91 => Some(AnsiControlCode::PU1),
            0x92 => Some(AnsiControlCode::PU2),
            0x93 => Some(AnsiControlCode::STS),
            0x94 => Some(AnsiControlCode::CCH),
            0x95 => Some(AnsiControlCode::MW),
            0x96 => Some(AnsiControlCode::SPA),
            0x97 => Some(AnsiControlCode::EPA),
            0x98 => Some(AnsiControlCode::SOS),
            0x99 => Some(AnsiControlCode::SGCI),
            0x9A => Some(AnsiControlCode::SCI),
            0x9B => Some(AnsiControlCode::CSI),
            0x9C => Some(AnsiControlCode::StC1),
            0x9D => Some(AnsiControlCode::OscC1),
            0x9E => Some(AnsiControlCode::PmC1),
            0x9F => Some(AnsiControlCode::ApcC1),

            _ => None,
        }
    }
}

impl std::fmt::Display for AnsiControlCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_byte() as char)
    }
}

/// Control Sequence Introducer (CSI) Command
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AnsiControlSequenceIntroducer {
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

impl AnsiControlSequenceIntroducer {
    /// Returns the encoded byte length of this CSI command.
    ///
    /// This method calculates the total number of bytes produced when the CSI command
    /// is encoded to its escape sequence representation. The length includes the ESC [
    /// introducer (2 bytes), all parameter bytes, and the final command byte.
    ///
    /// # Returns
    ///
    /// The number of bytes in the encoded form. Minimum is 3 bytes for simple commands
    /// (e.g., `ESC [ A`), up to 10+ bytes for commands with multiple parameters.
    ///
    /// # Performance
    ///
    /// This is O(1) for most commands. Some parameterized commands require digit counting
    /// which is O(log n) where n is the parameter value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiControlSequenceIntroducer;
    ///
    /// assert_eq!(AnsiControlSequenceIntroducer::CursorUp(1).len(), 3); // ESC[1A
    /// assert_eq!(AnsiControlSequenceIntroducer::CursorUp(255).len(), 6); // ESC[255A
    /// assert_eq!(AnsiControlSequenceIntroducer::SaveCursorPosition.len(), 3); // ESC[s
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Buffer allocation**: Pre-allocate space before encoding
    /// - **Terminal control**: Calculate message sizes for terminal commands
    pub fn len(&self) -> usize {
        match self {
            AnsiControlSequenceIntroducer::CursorUp(n) => 3 + self.count_digits(*n),
            AnsiControlSequenceIntroducer::CursorDown(n) => 3 + self.count_digits(*n),
            AnsiControlSequenceIntroducer::CursorForward(n) => 3 + self.count_digits(*n),
            AnsiControlSequenceIntroducer::CursorBack(n) => 3 + self.count_digits(*n),
            AnsiControlSequenceIntroducer::CursorNextLine(n) => 3 + self.count_digits(*n),
            AnsiControlSequenceIntroducer::CursorPreviousLine(n) => 3 + self.count_digits(*n),
            AnsiControlSequenceIntroducer::CursorHorizontalAbsolute(col) => {
                3 + self.count_digits(*col)
            }
            AnsiControlSequenceIntroducer::CursorPosition { row, col } => {
                4 + self.count_digits(*row) + self.count_digits(*col)
            }
            AnsiControlSequenceIntroducer::DeviceStatusReport => 4,
            AnsiControlSequenceIntroducer::SaveCursorPosition => 3,
            AnsiControlSequenceIntroducer::RestoreCursorPosition => 3,
            AnsiControlSequenceIntroducer::EraseInDisplay(_) => 4,
            AnsiControlSequenceIntroducer::EraseInLine(_) => 4,
            AnsiControlSequenceIntroducer::SetMode => 4,
            AnsiControlSequenceIntroducer::ResetMode => 4,
            AnsiControlSequenceIntroducer::DECPrivateModeSet => 4,
            AnsiControlSequenceIntroducer::DECPrivateModeReset => 4,
            AnsiControlSequenceIntroducer::ScrollUp => 3,
            AnsiControlSequenceIntroducer::ScrollDown => 3,
            AnsiControlSequenceIntroducer::InsertCharacter => 3,
            AnsiControlSequenceIntroducer::DeleteCharacter => 3,
            AnsiControlSequenceIntroducer::InsertLine => 3,
            AnsiControlSequenceIntroducer::DeleteLine => 3,
            AnsiControlSequenceIntroducer::EraseCharacter => 3,
            AnsiControlSequenceIntroducer::TextCursorEnableMode => 6,
            AnsiControlSequenceIntroducer::AlternativeScreenBuffer => 7,
            AnsiControlSequenceIntroducer::SetKeyboardStrings => 3,
            AnsiControlSequenceIntroducer::Unknown => 0,
        }
    }

    /// Encode this CSI command to a `BufMut` buffer.
    ///
    /// This method encodes the CSI command into its escape sequence representation and writes
    /// it to the provided mutable buffer. The buffer's write position is automatically advanced.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer implementing `BufMut`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success, containing the number of bytes written.
    /// Returns an `AnsiResult` error if encoding fails.
    ///
    /// # CSI Format
    ///
    /// All CSI commands are encoded as `ESC [ <parameters> <final_byte>`:
    /// - ESC: 0x1B
    /// - [: 0x5B
    /// - Parameters: numeric values and semicolons (variable length)
    /// - Final byte: command identifier (0x40-0x7E)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bytes::BytesMut;
    /// use termionix_ansicodec::ansi::AnsiControlSequenceIntroducer;
    ///
    /// let cmd = AnsiControlSequenceIntroducer::CursorUp(5);
    /// let mut buffer = BytesMut::new();
    /// let written = cmd.encode(&mut buffer).unwrap();
    /// assert_eq!(written, 4); // ESC[5A
    /// ```
    ///
    /// # See Also
    ///
    /// - [`write()`](AnsiControlSequenceIntroducer::write) - Write to a `std::io::Write` trait object
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> AnsiResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Write this CSI command to a `std::io::Write` writer.
    ///
    /// This method performs the actual encoding of the CSI command into its escape sequence
    /// representation and writes it to the provided writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success, containing the number of bytes written.
    /// Returns `std::io::Error` if writing fails.
    ///
    /// # Encoding Behavior
    ///
    /// Each command produces its specific escape sequence:
    ///
    /// - **CursorUp(n)**: `ESC [ n A`
    /// - **CursorDown(n)**: `ESC [ n B`
    /// - **CursorForward(n)**: `ESC [ n C`
    /// - **CursorBack(n)**: `ESC [ n D`
    /// - **CursorPosition { row, col }**: `ESC [ row ; col H`
    /// - **EraseInDisplay(mode)**: `ESC [ mode J`
    /// - **EraseInLine(mode)**: `ESC [ mode K`
    /// - **SaveCursorPosition**: `ESC [ s`
    /// - **RestoreCursorPosition**: `ESC [ u`
    /// - And others as documented in [`AnsiControlSequenceIntroducer`] variants
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiControlSequenceIntroducer;
    ///
    /// let cmd = AnsiControlSequenceIntroducer::SaveCursorPosition;
    /// let mut output = Vec::new();
    /// let written = cmd.write(&mut output).unwrap();
    /// assert_eq!(written, 3); // ESC[s
    /// assert_eq!(output, b"\x1b[s");
    /// ```
    ///
    /// # See Also
    ///
    /// - [`encode()`](AnsiControlSequenceIntroducer::encode) - Encode to a `BufMut` buffer
    /// - [`len()`](AnsiControlSequenceIntroducer::len) - Get the encoded byte length
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        match self {
            AnsiControlSequenceIntroducer::CursorUp(n) => {
                writer.write_all(format!("\x1b[{}A", n).as_bytes())?;
                Ok(format!("\x1b[{}A", n).len())
            }
            AnsiControlSequenceIntroducer::CursorDown(n) => {
                writer.write_all(format!("\x1b[{}B", n).as_bytes())?;
                Ok(format!("\x1b[{}B", n).len())
            }
            AnsiControlSequenceIntroducer::CursorForward(n) => {
                writer.write_all(format!("\x1b[{}C", n).as_bytes())?;
                Ok(format!("\x1b[{}C", n).len())
            }
            AnsiControlSequenceIntroducer::CursorBack(n) => {
                writer.write_all(format!("\x1b[{}D", n).as_bytes())?;
                Ok(format!("\x1b[{}D", n).len())
            }
            AnsiControlSequenceIntroducer::CursorNextLine(n) => {
                writer.write_all(format!("\x1b[{}E", n).as_bytes())?;
                Ok(format!("\x1b[{}E", n).len())
            }
            AnsiControlSequenceIntroducer::CursorPreviousLine(n) => {
                writer.write_all(format!("\x1b[{}F", n).as_bytes())?;
                Ok(format!("\x1b[{}F", n).len())
            }
            AnsiControlSequenceIntroducer::CursorHorizontalAbsolute(col) => {
                writer.write_all(format!("\x1b[{}G", col).as_bytes())?;
                Ok(format!("\x1b[{}G", col).len())
            }
            AnsiControlSequenceIntroducer::CursorPosition { row, col } => {
                writer.write_all(format!("\x1b[{};{}H", row, col).as_bytes())?;
                Ok(format!("\x1b[{};{}H", row, col).len())
            }
            AnsiControlSequenceIntroducer::DeviceStatusReport => {
                writer.write_all(b"\x1b[6n")?;
                Ok(4)
            }
            AnsiControlSequenceIntroducer::SaveCursorPosition => {
                writer.write_all(b"\x1b[s")?;
                Ok(3)
            }
            AnsiControlSequenceIntroducer::RestoreCursorPosition => {
                writer.write_all(b"\x1b[u")?;
                Ok(3)
            }
            AnsiControlSequenceIntroducer::EraseInDisplay(mode) => {
                writer.write_all(format!("\x1b[{}J", *mode as u8).as_bytes())?;
                Ok(format!("\x1b[{}J", *mode as u8).len())
            }
            AnsiControlSequenceIntroducer::EraseInLine(mode) => {
                writer.write_all(format!("\x1b[{}K", *mode as u8).as_bytes())?;
                Ok(format!("\x1b[{}K", *mode as u8).len())
            }
            AnsiControlSequenceIntroducer::SetMode => {
                writer.write_all(b"\x1b[=h")?;
                Ok(4)
            }
            AnsiControlSequenceIntroducer::ResetMode => {
                writer.write_all(b"\x1b[=l")?;
                Ok(4)
            }
            AnsiControlSequenceIntroducer::DECPrivateModeSet => {
                writer.write_all(b"\x1b[?h")?;
                Ok(4)
            }
            AnsiControlSequenceIntroducer::DECPrivateModeReset => {
                writer.write_all(b"\x1b[?l")?;
                Ok(4)
            }
            AnsiControlSequenceIntroducer::ScrollUp => {
                writer.write_all(b"\x1b[S")?;
                Ok(3)
            }
            AnsiControlSequenceIntroducer::ScrollDown => {
                writer.write_all(b"\x1b[T")?;
                Ok(3)
            }
            AnsiControlSequenceIntroducer::InsertCharacter => {
                writer.write_all(b"\x1b[@")?;
                Ok(3)
            }
            AnsiControlSequenceIntroducer::DeleteCharacter => {
                writer.write_all(b"\x1b[P")?;
                Ok(3)
            }
            AnsiControlSequenceIntroducer::InsertLine => {
                writer.write_all(b"\x1b[L")?;
                Ok(3)
            }
            AnsiControlSequenceIntroducer::DeleteLine => {
                writer.write_all(b"\x1b[M")?;
                Ok(3)
            }
            AnsiControlSequenceIntroducer::EraseCharacter => {
                writer.write_all(b"\x1b[X")?;
                Ok(3)
            }
            AnsiControlSequenceIntroducer::TextCursorEnableMode => {
                writer.write_all(b"\x1b[?25h")?;
                Ok(6)
            }
            AnsiControlSequenceIntroducer::AlternativeScreenBuffer => {
                writer.write_all(b"\x1b[?1049h")?;
                Ok(8)
            }
            AnsiControlSequenceIntroducer::SetKeyboardStrings => {
                writer.write_all(b"\x1b[p")?;
                Ok(3)
            }
            AnsiControlSequenceIntroducer::Unknown => Ok(0),
        }
    }

    fn count_digits(&self, n: u8) -> usize {
        if n == 0 {
            1
        } else {
            (n as f32).log10().floor() as usize + 1
        }
    }
}

impl std::fmt::Display for AnsiControlSequenceIntroducer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Cursor movement commands
            AnsiControlSequenceIntroducer::CursorUp(n) => {
                write!(f, "\x1b[{}A", n)
            }
            AnsiControlSequenceIntroducer::CursorDown(n) => {
                write!(f, "\x1b[{}B", n)
            }
            AnsiControlSequenceIntroducer::CursorForward(n) => {
                write!(f, "\x1b[{}C", n)
            }
            AnsiControlSequenceIntroducer::CursorBack(n) => {
                write!(f, "\x1b[{}D", n)
            }
            AnsiControlSequenceIntroducer::CursorNextLine(n) => {
                write!(f, "\x1b[{}E", n)
            }
            AnsiControlSequenceIntroducer::CursorPreviousLine(n) => {
                write!(f, "\x1b[{}F", n)
            }
            AnsiControlSequenceIntroducer::CursorHorizontalAbsolute(col) => {
                write!(f, "\x1b[{}G", col)
            }
            AnsiControlSequenceIntroducer::CursorPosition { row, col } => {
                write!(f, "\x1b[{};{}H", row, col)
            }

            // Device status and cursor save/restore
            AnsiControlSequenceIntroducer::DeviceStatusReport => {
                write!(f, "\x1b[6n")
            }
            AnsiControlSequenceIntroducer::SaveCursorPosition => {
                write!(f, "\x1b[s")
            }
            AnsiControlSequenceIntroducer::RestoreCursorPosition => {
                write!(f, "\x1b[u")
            }

            // Erase functions
            AnsiControlSequenceIntroducer::EraseInDisplay(mode) => {
                write!(f, "\x1b[{}J", *mode as u8)
            }
            AnsiControlSequenceIntroducer::EraseInLine(mode) => {
                write!(f, "\x1b[{}K", *mode as u8)
            }

            // Screen modes
            AnsiControlSequenceIntroducer::SetMode => {
                write!(f, "\x1b[=h")
            }
            AnsiControlSequenceIntroducer::ResetMode => {
                write!(f, "\x1b[=l")
            }
            AnsiControlSequenceIntroducer::DECPrivateModeSet => {
                write!(f, "\x1b[?h")
            }
            AnsiControlSequenceIntroducer::DECPrivateModeReset => {
                write!(f, "\x1b[?l")
            }

            // Scrolling
            AnsiControlSequenceIntroducer::ScrollUp => {
                write!(f, "\x1b[S")
            }
            AnsiControlSequenceIntroducer::ScrollDown => {
                write!(f, "\x1b[T")
            }

            // Insert/Delete
            AnsiControlSequenceIntroducer::InsertCharacter => {
                write!(f, "\x1b[@")
            }
            AnsiControlSequenceIntroducer::DeleteCharacter => {
                write!(f, "\x1b[P")
            }
            AnsiControlSequenceIntroducer::InsertLine => {
                write!(f, "\x1b[L")
            }
            AnsiControlSequenceIntroducer::DeleteLine => {
                write!(f, "\x1b[M")
            }
            AnsiControlSequenceIntroducer::EraseCharacter => {
                write!(f, "\x1b[X")
            }

            // Cursor visibility
            AnsiControlSequenceIntroducer::TextCursorEnableMode => {
                write!(f, "\x1b[?25h")
            }

            // Alternative screen buffer
            AnsiControlSequenceIntroducer::AlternativeScreenBuffer => {
                write!(f, "\x1b[?1049h")
            }

            // Keyboard strings
            AnsiControlSequenceIntroducer::SetKeyboardStrings => {
                write!(f, "\x1b[p")
            }

            // Unknown commands
            AnsiControlSequenceIntroducer::Unknown => {
                // Don't output anything for unknown commands
                Ok(())
            }
        }
    }
}

/// ED - Erase in Display mode parameter
///
/// Specifies which portion of the display to erase when using the ED (Erase in Display)
/// CSI command (ESC[nJ). This operation does not change the cursor position.
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
    /// Erase from the cursor position to the end of the line (inclusive)
    ///
    /// ESC[0K or ESC[K - Clears all characters from the cursor position to the end of the
    /// current line, including the character at the cursor position.
    EraseToEndOfLine = 0,

    /// Erase from the beginning of line to cursor position (inclusive)
    ///
    /// ESC[1K - Clears all characters from the beginning of the current line to the cursor
    /// position, including the character at the cursor position.
    EraseToStartOfLine = 1,

    /// Erase the entire line
    ///
    /// ESC[2K - Clears all characters on the current line. The cursor remains at its
    /// current position within the now-blank line.
    EraseEntireLine = 2,
}

/// Device Control String (DCS) - Device-specific control sequences
///
/// DCS sequences allow sending device-specific commands to the terminal or connected devices.
/// These are typically used for advanced terminal features like sixel graphics, DECSIXEL,
/// or device interrogation commands.
///
/// # Format
///
/// `ESC P <params> ST` where ST is the String Terminator (ESC \)
///
/// # Uses
///
/// - Terminal graphics protocols (e.g., Sixel, ReGIS)
/// - Device interrogation and status reporting
/// - Custom device commands through passthrough protocols
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnsiDeviceControlString {
    /// Unrecognized or device-specific DCS command
    ///
    /// Contains the raw bytes of the DCS sequence parameters, allowing applications
    /// to handle custom DCS commands not explicitly defined in this enum.
    Unknown(Vec<u8>),
}

impl AnsiDeviceControlString {
    /// Returns the encoded byte length of this DCS sequence.
    ///
    /// Calculates the total bytes when encoded, including the ESC P introducer (2 bytes),
    /// the data payload, and the ST terminator (2 bytes: ESC \).
    ///
    /// # Returns
    ///
    /// The number of bytes in the encoded form: 4 + data_length
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiDeviceControlString;
    ///
    /// let dcs = AnsiDeviceControlString::Unknown(vec![b't', b'x']);
    /// assert_eq!(dcs.len(), 6); // ESC P t x ESC \
    /// ```
    pub fn len(&self) -> usize {
        match self {
            AnsiDeviceControlString::Unknown(data) => 4 + data.len(), // ESC P ... ST
        }
    }

    /// Encode this DCS sequence to a `BufMut` buffer.
    ///
    /// This method encodes the Device Control String (DCS) sequence into its wire format
    /// and writes it to the provided mutable buffer. The buffer's write position is automatically
    /// advanced by the number of bytes written.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer implementing `BufMut` (e.g., `BytesMut`, `Vec<u8>`, etc.).
    ///   The buffer is advanced by the bytes written.
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success, containing the exact number of bytes written.
    /// Returns an `AnsiResult` error if encoding fails (typically due to buffer allocation or I/O issues).
    ///
    /// # DCS Wire Format
    ///
    /// DCS sequences are encoded in the following format:
    /// - **ESC P**: 0x1B 0x50 (7-bit representation) or 0x90 (8-bit C1 control code)
    /// - **Data**: Device-specific command parameters (variable length, can be empty)
    /// - **ST**: 0x1B 0x5C (7-bit representation) or 0x9C (8-bit C1 control code)
    ///
    /// # Performance
    ///
    /// This is an O(n) operation where n is the length of the encoded sequence. It performs
    /// minimal allocations and writes directly to the provided buffer.
    ///
    /// # Common Device Control String Sequences
    ///
    /// - **Sixel Graphics**: `ESC P q ... ST` - Graphics protocol for displaying images
    /// - **ReGIS Graphics**: `ESC P p ... ST` - Vector graphics protocol
    /// - **Device Status**: `ESC P s ... ST` - Device interrogation and status reporting
    /// - **Device Configuration**: `ESC P $ ... ST` - Configuration commands
    ///
    /// # Examples
    ///
    /// ## Encoding a Simple DCS Sequence
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiDeviceControlString;
    /// use bytes::BytesMut;
    ///
    /// let dcs = AnsiDeviceControlString::Unknown(b"q\"1;1;100;100#0;2;0;0;0#1;2;100;100;100".to_vec());
    /// let mut buffer = BytesMut::new();
    /// let written = dcs.encode(&mut buffer).unwrap();
    /// // Result: ESC P q"1;1;100;100#0;2;0;0;0#1;2;100;100;100 ESC \
    /// ```
    ///
    /// ## Building a Buffer Incrementally
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiDeviceControlString;
    /// use bytes::BytesMut;
    ///
    /// let mut buffer = BytesMut::new();
    ///
    /// // Encode multiple DCS sequences
    /// let dcs1 = AnsiDeviceControlString::Unknown(b"1$t".to_vec());
    /// let dcs2 = AnsiDeviceControlString::Unknown(b"2$t".to_vec());
    ///
    /// dcs1.encode(&mut buffer).unwrap();
    /// dcs2.encode(&mut buffer).unwrap();
    ///
    /// // Both sequences are now in the buffer
    /// ```
    ///
    /// ## Pre-allocating Buffer Space
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiDeviceControlString;
    /// use bytes::BytesMut;
    ///
    /// let dcs = AnsiDeviceControlString::Unknown(b"graphics_data".to_vec());
    /// let mut buffer = BytesMut::with_capacity(dcs.len()); // Pre-allocate
    /// dcs.encode(&mut buffer).unwrap();
    /// ```
    ///
    /// # Error Handling
    ///
    /// While encoding to `BufMut` rarely fails, errors can occur if:
    /// - The underlying buffer cannot allocate additional capacity
    /// - I/O errors occur (rare for in-memory buffers, common for file/network writers)
    ///
    /// # Buffer Behavior
    ///
    /// - The buffer is only written to; existing content is preserved
    /// - The buffer's write position is advanced automatically
    /// - No bounds checking is performed; the buffer must have sufficient capacity
    /// - For `BytesMut`, automatic growth occurs if needed
    ///
    /// # Use Cases
    ///
    /// - **Graphics protocols**: Sending Sixel or ReGIS graphics to terminal emulators
    /// - **Device interrogation**: Querying terminal capabilities and status
    /// - **Configuration**: Sending device-specific configuration commands
    /// - **Protocol pass-through**: Forwarding device-specific commands to connected terminals
    ///
    /// # Related Methods
    ///
    /// - [`write()`](AnsiDeviceControlString::write) - Write to a `std::io::Write` trait object for lower-level control
    /// - [`len()`](AnsiDeviceControlString::len) - Get the encoded byte length without encoding
    /// - [`Display`](std::fmt::Display) - Convert to a string representation for debugging
    ///
    /// # Standards Reference
    ///
    /// This method implements the Device Control String (DCS) as defined in:
    /// - ISO/IEC 6429 (Information technology — Control functions for coded character sets)
    /// - ANSI X3.64-1979 (Extended Control Functions)
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> AnsiResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Write this DCS sequence to a `std::io::Write` writer.
    ///
    /// Performs the actual encoding of the Device Control String sequence into its
    /// byte representation and writes it to the provided writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success. Returns `std::io::Error` if writing fails.
    ///
    /// # DCS Format
    ///
    /// DCS sequences are encoded as `ESC P <data> ST`:
    /// - ESC P: 0x1B 0x50 (or 0x90 in 8-bit)
    /// - Data: Device-specific command data
    /// - ST: 0x1B 0x5C or 0x9C (String Terminator)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::AnsiDeviceControlString;
    ///
    /// let dcs = AnsiDeviceControlString::Unknown(b"1$t".to_vec());
    /// let mut output = Vec::new();
    /// dcs.write(&mut output).unwrap();
    /// // Result: b"\x1bP1$t\x1b\\"
    /// ```
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        match self {
            AnsiDeviceControlString::Unknown(data) => {
                writer.write_all(b"\x1bP")?;
                writer.write_all(data)?;
                writer.write_all(b"\x1b\\")?;
                Ok(4 + data.len())
            }
        }
    }
}

impl std::fmt::Display for AnsiDeviceControlString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnsiDeviceControlString::Unknown(data) => {
                write!(f, "\x1bP")?;
                if let Ok(s) = std::str::from_utf8(data) {
                    write!(f, "{}", s)?;
                }
                write!(f, "\x1b\\")
            }
        }
    }
}

/// Operating System Command (OSC) - Terminal and OS-level operations
///
/// OSC sequences communicate with the terminal emulator to perform operations that
/// affect the terminal's behavior or presentation, such as setting window titles,
/// configuring colors, or executing hyperlink commands.
///
/// # Format
///
/// `ESC ] <params> ST` or `ESC ] <params> BEL`
///
/// The sequence is terminated by either:
/// - ST (String Terminator): ESC \ (0x1B 0x5C)
/// - BEL (Bell): 0x07
///
/// # Common OSC Commands
///
/// - OSC 0 ; title ST - Set window title and icon name
/// - OSC 1 ; title ST - Set icon name
/// - OSC 2 ; title ST - Set window title
/// - OSC 4 ; index ; color ST - Define color palette entry
/// - OSC 8 ; params ; URL ST - Create hyperlink
/// - OSC 9 ; params ST - Set terminal notification (hyperlink hover)
/// - OSC 52 ; c ; data ST - Copy to clipboard (xterm extension)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnsiOperatingSystemCommand {
    /// Unrecognized or custom OSC command
    ///
    /// Contains the raw bytes of the OSC sequence parameters, allowing applications
    /// to handle OSC commands not explicitly defined in this enum.
    Unknown(Vec<u8>),
}

impl AnsiOperatingSystemCommand {
    /// Returns the encoded byte length of this OSC sequence.
    ///
    /// Calculates the total bytes when encoded, including the ESC ] introducer (2 bytes),
    /// the command data, and the ST terminator (2 bytes: ESC \).
    ///
    /// # Returns
    ///
    /// The number of bytes in the encoded form: 4 + data_length
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiOperatingSystemCommand;
    ///
    /// let osc = AnsiOperatingSystemCommand::Unknown(b"0;My Title".to_vec());
    /// assert_eq!(osc.len(), 14); // ESC ] 0;My Title ST
    /// ```
    pub fn len(&self) -> usize {
        match self {
            AnsiOperatingSystemCommand::Unknown(data) => 4 + data.len(), // ESC ] ... ST
        }
    }

    /// Encode this OSC sequence to a `BufMut` buffer.
    ///
    /// Encodes the Operating System Command into its byte representation and writes it
    /// to the provided mutable buffer.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer implementing `BufMut`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success. Returns an `AnsiResult` error if encoding fails.
    ///
    /// # OSC Format
    ///
    /// OSC sequences are encoded as `ESC ] <command>;<data> ST`:
    /// - ESC ]: 0x1B 0x5D (or 0x9D in 8-bit)
    /// - Command: Numeric code (e.g., 0, 2, 4, etc.)
    /// - Data: Command-specific data
    /// - Terminator: ST (ESC \ or BEL)
    ///
    /// # Common OSC Commands
    ///
    /// - 0: Set window title and icon
    /// - 2: Set window title
    /// - 4: Define color palette
    /// - 8: Create hyperlink
    /// - 52: Set clipboard (xterm)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiOperatingSystemCommand;
    /// use bytes::BytesMut;
    ///
    /// // Set window title
    /// let osc = AnsiOperatingSystemCommand::Unknown(b"2;MyWindow".to_vec());
    /// let mut buffer = BytesMut::new();
    /// osc.encode(&mut buffer).unwrap();
    /// ```
    ///
    /// # See Also
    ///
    /// - [`write()`](AnsiOperatingSystemCommand::write) - Write to a `std::io::Write` trait object
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> AnsiResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Write this OSC sequence to a `std::io::Write` writer.
    ///
    /// Performs the actual encoding and writes to the provided writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success. Returns `std::io::Error` if writing fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiOperatingSystemCommand;
    ///
    /// let osc = AnsiOperatingSystemCommand::Unknown(b"0;Title".to_vec());
    /// let mut output = Vec::new();
    /// osc.write(&mut output).unwrap();
    /// ```
    ///
    /// # See Also
    ///
    /// - [`encode()`](AnsiOperatingSystemCommand::encode) - Encode to a `BufMut` buffer
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        match self {
            AnsiOperatingSystemCommand::Unknown(data) => {
                writer.write_all(b"\x1b]")?;
                writer.write_all(data)?;
                writer.write_all(b"\x1b\\")?;
                Ok(4 + data.len())
            }
        }
    }
}

impl std::fmt::Display for AnsiOperatingSystemCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnsiOperatingSystemCommand::Unknown(data) => {
                write!(f, "\x1b]")?;
                if let Ok(s) = std::str::from_utf8(data) {
                    write!(f, "{}", s)?;
                }
                write!(f, "\x1b\\")
            }
        }
    }
}

/// Start of String (SOS) - Legacy ISO 6429 control function
///
/// SOS is an ISO 6429 control function used to mark the beginning of a string
/// delimited by a String Terminator (ST). This is a rarely used control sequence
/// in modern terminals and is included for completeness in ISO 6429 compliance.
///
/// # Format
///
/// `ESC X <data> ST` where ST is the String Terminator (ESC \)
///
/// # Notes
///
/// This control sequence is seldom encountered in practice. Most terminal applications
/// use OSC or APC sequences instead for their specific use cases.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnsiStartOfString {
    /// Unrecognized SOS command or payload
    ///
    /// Contains the raw bytes of the SOS sequence data, allowing applications to
    /// interpret custom SOS commands.
    Unknown(Vec<u8>),
}

impl AnsiStartOfString {
    /// Returns the encoded byte length of this SOS sequence.
    ///
    /// Calculates the total bytes when encoded, including the ESC X introducer (2 bytes),
    /// the data payload, and the ST terminator (2 bytes: ESC \).
    ///
    /// # Returns
    ///
    /// The number of bytes in the encoded form: 4 + data_length
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiStartOfString;
    ///
    /// let sos = AnsiStartOfString::Unknown(b"data".to_vec());
    /// assert_eq!(sos.len(), 8); // ESC X data ESC \
    /// ```
    pub fn len(&self) -> usize {
        match self {
            AnsiStartOfString::Unknown(data) => 4 + data.len(), // ESC X ... ST
        }
    }

    /// Encode this SOS sequence to a `BufMut` buffer.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer implementing `BufMut`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success.
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> AnsiResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Write this SOS sequence to a `std::io::Write` writer.
    ///
    /// Encodes as `ESC X <data> ST` format.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success.
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        match self {
            AnsiStartOfString::Unknown(data) => {
                writer.write_all(b"\x1bX")?;
                writer.write_all(data)?;
                writer.write_all(b"\x1b\\")?;
                Ok(4 + data.len())
            }
        }
    }
}

impl std::fmt::Display for AnsiStartOfString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnsiStartOfString::Unknown(data) => {
                write!(f, "\x1bX")?;
                if let Ok(s) = std::str::from_utf8(data) {
                    write!(f, "{}", s)?;
                }
                write!(f, "\x1b\\")
            }
        }
    }
}

/// String Terminator - End marker for string-type control sequences
///
/// ST is an ISO 6429 control sequence terminator used to mark the end of
/// string-type sequences such as OSC, DCS, SOS, PM, and APC. It can be
/// represented as either the two-byte sequence ESC \ or the single-byte
/// C1 control code 0x9C (in terminals that support C1 control codes).
///
/// # Format
///
/// Two representations:
/// - Two-byte: `ESC \` (0x1B 0x5C)
/// - Single-byte C1: 0x9C
///
/// # Usage
///
/// String Terminators are encountered in two contexts:
/// 1. **In sequence context**: Automatically parsed and discarded when terminating
///    OSC, DCS, SOS, PM, or APC sequences
/// 2. **Standalone**: Returned as `AnsiSequence::AnsiST` when encountered outside
///    of a recognized string sequence (typically indicates malformed input)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnsiStringTerminator();

/// Privacy Message (PM) - ISO 6429 private data delimiters
///
/// PM is an ISO 6429 control function used to delimit sequences containing
/// private or implementation-defined data. This is a rarely used control sequence
/// in modern terminals but is included for ISO 6429 compliance.
///
/// # Format
///
/// `ESC ^ <data> ST` where ST is the String Terminator (ESC \)
///
/// # Distinction from OSC
///
/// While OSC (0x9D or ESC ]) is used for Operating System Commands with standard
/// interpretations, PM (0x9E or ESC ^) is reserved for implementation-defined private uses.
///
/// # Notes
///
/// This control sequence is rarely used in practice. Most implementations prefer OSC
/// or APC sequences for their specific needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnsiPrivacyMessage {
    /// Unrecognized PM command or payload
    ///
    /// Contains the raw bytes of the PM sequence data, allowing applications to
    /// interpret implementation-specific PM commands.
    Unknown(Vec<u8>),
}

impl AnsiPrivacyMessage {
    /// Returns the encoded byte length of this PM sequence.
    ///
    /// Calculates the total bytes when encoded, including the ESC ^ introducer (2 bytes),
    /// the data payload, and the ST terminator (2 bytes: ESC \).
    ///
    /// # Returns
    ///
    /// The number of bytes in the encoded form: 4 + data_length
    pub fn len(&self) -> usize {
        match self {
            AnsiPrivacyMessage::Unknown(data) => 4 + data.len(), // ESC ^ ... ST
        }
    }

    /// Encode this PM sequence to a `BufMut` buffer.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer implementing `BufMut`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success.
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> AnsiResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Write this PM sequence to a `std::io::Write` writer.
    ///
    /// Encodes as `ESC ^ <data> ST` format.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success.
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        match self {
            AnsiPrivacyMessage::Unknown(data) => {
                writer.write_all(b"\x1b^")?;
                writer.write_all(data)?;
                writer.write_all(b"\x1b\\")?;
                Ok(4 + data.len())
            }
        }
    }
}

impl std::fmt::Display for AnsiPrivacyMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnsiPrivacyMessage::Unknown(data) => {
                write!(f, "\x1b^")?;
                if let Ok(s) = std::str::from_utf8(data) {
                    write!(f, "{}", s)?;
                }
                write!(f, "\x1b\\")
            }
        }
    }
}

/// Application Program Command (APC) - Application-specific commands
///
/// APC sequences allow applications to send custom commands through the terminal
/// without interfering with standard terminal operations. These are application-defined
/// sequences that the terminal passes through without interpretation.
///
/// # Format
///
/// `ESC _ <data> ST` where ST is the String Terminator (ESC \)
///
/// # Common Uses
///
/// - IDE/editor integration commands
/// - Custom application protocols
/// - Extended terminal features not defined by ANSI/ISO standards
/// - Semantic markup and formatting (e.g., iTerm2 inline images)
///
/// # Examples
///
/// - iTerm2 inline image protocol uses APC sequences
/// - Some terminal multiplexers use APC for control
/// - IDE remote execution protocols
///
/// # Notes
///
/// APC sequences are implementation-defined and their interpretation depends entirely
/// on the receiving application. The terminal typically passes them through unchanged
/// or logs them for debugging purposes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnsiApplicationProgramCommand {
    /// Unrecognized or custom APC command
    ///
    /// Contains the raw bytes of the APC sequence data, allowing applications to
    /// handle custom APC commands according to their own protocol specifications.
    Unknown(Vec<u8>),
}

impl AnsiApplicationProgramCommand {
    /// Returns the encoded byte length of this APC sequence.
    ///
    /// Calculates the total bytes when encoded, including the ESC _ introducer (2 bytes),
    /// the data payload, and the ST terminator (2 bytes: ESC \).
    ///
    /// # Returns
    ///
    /// The number of bytes in the encoded form: 4 + data_length
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::ansi::AnsiApplicationProgramCommand;
    ///
    /// let apc = AnsiApplicationProgramCommand::Unknown(b"custom".to_vec());
    /// assert_eq!(apc.len(), 10); // ESC _ custom ESC \
    /// ```
    pub fn len(&self) -> usize {
        match self {
            AnsiApplicationProgramCommand::Unknown(data) => 4 + data.len(), // ESC _ ... ST
        }
    }

    /// Encode this APC sequence to a `BufMut` buffer.
    ///
    /// Encodes the Application Program Command into its byte representation and writes it
    /// to the provided mutable buffer.
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable buffer implementing `BufMut`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success.
    ///
    /// # APC Format
    ///
    /// APC sequences are encoded as `ESC _ <data> ST`:
    /// - ESC _: 0x1B 0x5F (or 0x9F in 8-bit)
    /// - Data: Application-specific command data
    /// - ST: 0x1B 0x5C (String Terminator)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bytes::BytesMut;
    /// use termionix_ansicodec::ansi::AnsiApplicationProgramCommand;
    ///
    /// let apc = AnsiApplicationProgramCommand::Unknown(b"command".to_vec());
    /// let mut buffer = BytesMut::new();
    /// apc.encode(&mut buffer).unwrap();
    /// ```
    ///
    /// # See Also
    ///
    /// - [`write()`](AnsiApplicationProgramCommand::write) - Write to a `std::io::Write` trait object
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> AnsiResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Write this APC sequence to a `std::io::Write` writer.
    ///
    /// Performs the actual encoding and writes to the provided writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// Returns `Ok(bytes_written)` on success.
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        match self {
            AnsiApplicationProgramCommand::Unknown(data) => {
                writer.write_all(b"\x1b_")?;
                writer.write_all(data)?;
                writer.write_all(b"\x1b\\")?;
                Ok(4 + data.len())
            }
        }
    }
}

impl std::fmt::Display for AnsiApplicationProgramCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnsiApplicationProgramCommand::Unknown(data) => {
                write!(f, "\x1b_")?;
                if let Ok(s) = std::str::from_utf8(data) {
                    write!(f, "{}", s)?;
                }
                write!(f, "\x1b\\")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    // ============================================================================
    // AnsiSequence Tests
    // ============================================================================

    #[test]
    fn test_ansi_sequence_character_len() {
        let seq = AnsiSequence::Character('A');
        assert_eq!(seq.len(), 1);

        let seq = AnsiSequence::Character('€');
        assert_eq!(seq.len(), 3);
    }

    #[test]
    fn test_ansi_sequence_unicode_len() {
        let seq = AnsiSequence::Unicode('世');
        assert_eq!(seq.len(), 3);

        let seq = AnsiSequence::Unicode('😀');
        assert_eq!(seq.len(), 4);
    }

    #[test]
    fn test_ansi_sequence_control_len() {
        let seq = AnsiSequence::Control(AnsiControlCode::LF);
        assert_eq!(seq.len(), 1);
    }

    #[test]
    fn test_ansi_sequence_ansi_escape_len() {
        let seq = AnsiSequence::AnsiEscape;
        assert_eq!(seq.len(), 1);
    }

    #[test]
    fn test_ansi_sequence_ansi_st_len() {
        let seq = AnsiSequence::AnsiST;
        assert_eq!(seq.len(), 2); // ESC \
    }

    #[test]
    fn test_ansi_sequence_encode_character() {
        let seq = AnsiSequence::Character('X');
        let mut buffer = BytesMut::new();
        let written = seq.encode(&mut buffer).unwrap();
        assert_eq!(written, 1);
        assert_eq!(&buffer[..], b"X");
    }

    #[test]
    fn test_ansi_sequence_encode_unicode() {
        let seq = AnsiSequence::Unicode('€');
        let mut buffer = BytesMut::new();
        let written = seq.encode(&mut buffer).unwrap();
        assert_eq!(written, 3);
        assert_eq!(buffer.as_ref(), "€".as_bytes());
    }

    #[test]
    fn test_ansi_sequence_encode_control() {
        let seq = AnsiSequence::Control(AnsiControlCode::LF);
        let mut buffer = BytesMut::new();
        let written = seq.encode(&mut buffer).unwrap();
        assert_eq!(written, 1);
        assert_eq!(&buffer[..], b"\n");
    }

    #[test]
    fn test_ansi_sequence_encode_ansi_escape() {
        let seq = AnsiSequence::AnsiEscape;
        let mut buffer = BytesMut::new();
        let written = seq.encode(&mut buffer).unwrap();
        assert_eq!(written, 1);
        assert_eq!(&buffer[..], b"\x1b");
    }

    #[test]
    fn test_ansi_sequence_encode_ansi_st() {
        let seq = AnsiSequence::AnsiST;
        let mut buffer = BytesMut::new();
        let written = seq.encode(&mut buffer).unwrap();
        assert_eq!(written, 2);
        assert_eq!(&buffer[..], b"\x1b\\");
    }

    #[test]
    fn test_ansi_sequence_display_character() {
        let seq = AnsiSequence::Character('A');
        assert_eq!(seq.to_string(), "A");
    }

    #[test]
    fn test_ansi_sequence_display_unicode() {
        let seq = AnsiSequence::Unicode('€');
        assert_eq!(seq.to_string(), "€");
    }

    #[test]
    fn test_ansi_sequence_display_ansi_st() {
        let seq = AnsiSequence::AnsiST;
        assert_eq!(seq.to_string(), "\x1b\\");
    }

    // ============================================================================
    // AnsiControlCode Tests
    // ============================================================================

    #[test]
    fn test_ansi_control_code_len() {
        assert_eq!(AnsiControlCode::NUL.len(), 1);
        assert_eq!(AnsiControlCode::LF.len(), 1);
        assert_eq!(AnsiControlCode::DEL.len(), 1);
        assert_eq!(AnsiControlCode::BEL.len(), 1);
    }

    #[test]
    fn test_ansi_control_code_to_byte_c0() {
        assert_eq!(AnsiControlCode::NUL.to_byte(), 0x00);
        assert_eq!(AnsiControlCode::BEL.to_byte(), 0x07);
        assert_eq!(AnsiControlCode::BS.to_byte(), 0x08);
        assert_eq!(AnsiControlCode::HT.to_byte(), 0x09);
        assert_eq!(AnsiControlCode::LF.to_byte(), 0x0A);
        assert_eq!(AnsiControlCode::FF.to_byte(), 0x0C);
        assert_eq!(AnsiControlCode::CR.to_byte(), 0x0D);
        assert_eq!(AnsiControlCode::DEL.to_byte(), 0x7F);
    }

    #[test]
    fn test_ansi_control_code_to_byte_c1() {
        assert_eq!(AnsiControlCode::PAD.to_byte(), 0x80);
        assert_eq!(AnsiControlCode::HOP.to_byte(), 0x81);
        assert_eq!(AnsiControlCode::IND.to_byte(), 0x84);
        assert_eq!(AnsiControlCode::NEL.to_byte(), 0x85);
        assert_eq!(AnsiControlCode::CSI.to_byte(), 0x9B);
        assert_eq!(AnsiControlCode::StC1.to_byte(), 0x9C);
        assert_eq!(AnsiControlCode::OscC1.to_byte(), 0x9D);
        assert_eq!(AnsiControlCode::ApcC1.to_byte(), 0x9F);
    }

    #[test]
    fn test_ansi_control_code_from_byte() {
        assert_eq!(AnsiControlCode::from_byte(0x00), Some(AnsiControlCode::NUL));
        assert_eq!(AnsiControlCode::from_byte(0x07), Some(AnsiControlCode::BEL));
        assert_eq!(AnsiControlCode::from_byte(0x0A), Some(AnsiControlCode::LF));
        assert_eq!(AnsiControlCode::from_byte(0x7F), Some(AnsiControlCode::DEL));
        assert_eq!(AnsiControlCode::from_byte(0x80), Some(AnsiControlCode::PAD));
        assert_eq!(
            AnsiControlCode::from_byte(0x9F),
            Some(AnsiControlCode::ApcC1)
        );
        assert_eq!(AnsiControlCode::from_byte(0xFF), None);
    }

    #[test]
    fn test_ansi_control_code_encode() {
        let code = AnsiControlCode::BEL;
        let mut buffer = BytesMut::new();
        let written = code.encode(&mut buffer).unwrap();
        assert_eq!(written, 1);
        assert_eq!(&buffer[..], b"\x07");
    }

    #[test]
    fn test_ansi_control_code_write() {
        let code = AnsiControlCode::LF;
        let mut output = Vec::new();
        let written = code.write(&mut output).unwrap();
        assert_eq!(written, 1);
        assert_eq!(output, b"\n");
    }

    #[test]
    fn test_ansi_control_code_display() {
        let code = AnsiControlCode::LF;
        assert_eq!(code.to_string().as_bytes(), b"\n");
    }

    // ============================================================================
    // AnsiControlSequenceIntroducer Tests
    // ============================================================================

    #[test]
    fn test_csi_cursor_up_len() {
        assert_eq!(AnsiControlSequenceIntroducer::CursorUp(1).len(), 4);
        assert_eq!(AnsiControlSequenceIntroducer::CursorUp(10).len(), 5);
        assert_eq!(AnsiControlSequenceIntroducer::CursorUp(255).len(), 6);
    }

    #[test]
    fn test_csi_cursor_position_len() {
        assert_eq!(
            AnsiControlSequenceIntroducer::CursorPosition { row: 1, col: 1 }.len(),
            6
        );
        assert_eq!(
            AnsiControlSequenceIntroducer::CursorPosition { row: 10, col: 20 }.len(),
            8
        );
    }

    #[test]
    fn test_csi_simple_commands_len() {
        assert_eq!(AnsiControlSequenceIntroducer::SaveCursorPosition.len(), 3);
        assert_eq!(
            AnsiControlSequenceIntroducer::RestoreCursorPosition.len(),
            3
        );
        assert_eq!(AnsiControlSequenceIntroducer::DeviceStatusReport.len(), 4);
        assert_eq!(AnsiControlSequenceIntroducer::ScrollUp.len(), 3);
        assert_eq!(AnsiControlSequenceIntroducer::TextCursorEnableMode.len(), 6);
    }

    #[test]
    fn test_csi_encode_cursor_up() {
        let cmd = AnsiControlSequenceIntroducer::CursorUp(5);
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 4);
        assert_eq!(&buffer[..], b"\x1b[5A");
    }

    #[test]
    fn test_csi_encode_cursor_down() {
        let cmd = AnsiControlSequenceIntroducer::CursorDown(3);
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 4);
        assert_eq!(&buffer[..], b"\x1b[3B");
    }

    #[test]
    fn test_csi_encode_cursor_position() {
        let cmd = AnsiControlSequenceIntroducer::CursorPosition { row: 10, col: 20 };
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 8);
        assert_eq!(&buffer[..], b"\x1b[10;20H");
    }

    #[test]
    fn test_csi_encode_save_cursor() {
        let cmd = AnsiControlSequenceIntroducer::SaveCursorPosition;
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 3);
        assert_eq!(&buffer[..], b"\x1b[s");
    }

    #[test]
    fn test_csi_encode_restore_cursor() {
        let cmd = AnsiControlSequenceIntroducer::RestoreCursorPosition;
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 3);
        assert_eq!(&buffer[..], b"\x1b[u");
    }

    #[test]
    fn test_csi_encode_device_status_report() {
        let cmd = AnsiControlSequenceIntroducer::DeviceStatusReport;
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 4);
        assert_eq!(&buffer[..], b"\x1b[6n");
    }

    #[test]
    fn test_csi_encode_scroll_up() {
        let cmd = AnsiControlSequenceIntroducer::ScrollUp;
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 3);
        assert_eq!(&buffer[..], b"\x1b[S");
    }

    #[test]
    fn test_csi_encode_text_cursor_enable_mode() {
        let cmd = AnsiControlSequenceIntroducer::TextCursorEnableMode;
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 6);
        assert_eq!(&buffer[..], b"\x1b[?25h");
    }

    #[test]
    fn test_csi_encode_alternative_screen_buffer() {
        let cmd = AnsiControlSequenceIntroducer::AlternativeScreenBuffer;
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 8);
        assert_eq!(&buffer[..], b"\x1b[?1049h");
    }

    #[test]
    fn test_csi_display_cursor_up() {
        let cmd = AnsiControlSequenceIntroducer::CursorUp(5);
        assert_eq!(cmd.to_string(), "\x1b[5A");
    }

    #[test]
    fn test_csi_display_cursor_position() {
        let cmd = AnsiControlSequenceIntroducer::CursorPosition { row: 1, col: 1 };
        assert_eq!(cmd.to_string(), "\x1b[1;1H");
    }

    // ============================================================================
    // EraseInDisplayMode Tests
    // ============================================================================

    #[test]
    fn test_erase_in_display_mode_values() {
        assert_eq!(EraseInDisplayMode::EraseToEndOfScreen as u8, 0);
        assert_eq!(EraseInDisplayMode::EraseToBeginningOfScreen as u8, 1);
        assert_eq!(EraseInDisplayMode::EraseEntireScreen as u8, 2);
        assert_eq!(EraseInDisplayMode::EraseEntireScreenAndSavedLines as u8, 3);
    }

    #[test]
    fn test_csi_erase_in_display() {
        let cmd =
            AnsiControlSequenceIntroducer::EraseInDisplay(EraseInDisplayMode::EraseEntireScreen);
        let mut buffer = BytesMut::new();
        let _written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(&buffer[..], b"\x1b[2J");
    }

    // ============================================================================
    // EraseInLineMode Tests
    // ============================================================================

    #[test]
    fn test_erase_in_line_mode_values() {
        assert_eq!(EraseInLineMode::EraseToEndOfLine as u8, 0);
        assert_eq!(EraseInLineMode::EraseToStartOfLine as u8, 1);
        assert_eq!(EraseInLineMode::EraseEntireLine as u8, 2);
    }

    #[test]
    fn test_csi_erase_in_line() {
        let cmd = AnsiControlSequenceIntroducer::EraseInLine(EraseInLineMode::EraseEntireLine);
        let mut buffer = BytesMut::new();
        let _written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(&buffer[..], b"\x1b[2K");
    }

    // ============================================================================
    // AnsiDeviceControlString Tests
    // ============================================================================

    #[test]
    fn test_dcs_len_empty() {
        let dcs = AnsiDeviceControlString::Unknown(vec![]);
        assert_eq!(dcs.len(), 4); // ESC P ST
    }

    #[test]
    fn test_dcs_len_with_data() {
        let dcs = AnsiDeviceControlString::Unknown(b"1$t".to_vec());
        assert_eq!(dcs.len(), 7); // ESC P 1 $ t ST
    }

    #[test]
    fn test_dcs_encode() {
        let dcs = AnsiDeviceControlString::Unknown(b"1$t".to_vec());
        let mut buffer = BytesMut::new();
        let written = dcs.encode(&mut buffer).unwrap();
        assert_eq!(written, 7);
        assert_eq!(&buffer[..], b"\x1bP1$t\x1b\\");
    }

    #[test]
    fn test_dcs_encode_empty() {
        let dcs = AnsiDeviceControlString::Unknown(vec![]);
        let mut buffer = BytesMut::new();
        let written = dcs.encode(&mut buffer).unwrap();
        assert_eq!(written, 4);
        assert_eq!(&buffer[..], b"\x1bP\x1b\\");
    }

    #[test]
    fn test_dcs_write() {
        let dcs = AnsiDeviceControlString::Unknown(b"test".to_vec());
        let mut output = Vec::new();
        let written = dcs.write(&mut output).unwrap();
        assert_eq!(written, 8);
        assert_eq!(output, b"\x1bPtest\x1b\\");
    }

    #[test]
    fn test_dcs_display() {
        let dcs = AnsiDeviceControlString::Unknown(b"data".to_vec());
        assert_eq!(dcs.to_string(), "\x1bPdata\x1b\\");
    }

    // ============================================================================
    // AnsiOperatingSystemCommand Tests
    // ============================================================================

    #[test]
    fn test_osc_len_empty() {
        let osc = AnsiOperatingSystemCommand::Unknown(vec![]);
        assert_eq!(osc.len(), 4); // ESC ] ST
    }

    #[test]
    fn test_osc_len_with_data() {
        let osc = AnsiOperatingSystemCommand::Unknown(b"0;Title".to_vec());
        assert_eq!(osc.len(), 11); // ESC ] 0;Title ST
    }

    #[test]
    fn test_osc_encode_set_title() {
        let osc = AnsiOperatingSystemCommand::Unknown(b"2;MyTitle".to_vec());
        let mut buffer = BytesMut::new();
        let written = osc.encode(&mut buffer).unwrap();
        assert_eq!(written, 13);
        assert_eq!(&buffer[..], b"\x1b]2;MyTitle\x1b\\");
    }

    #[test]
    fn test_osc_write() {
        let osc = AnsiOperatingSystemCommand::Unknown(b"0;Icon".to_vec());
        let mut output = Vec::new();
        let written = osc.write(&mut output).unwrap();
        assert_eq!(written, 10);
        assert_eq!(output, b"\x1b]0;Icon\x1b\\");
    }

    #[test]
    fn test_osc_display() {
        let osc = AnsiOperatingSystemCommand::Unknown(b"52;c;data".to_vec());
        assert_eq!(osc.to_string(), "\x1b]52;c;data\x1b\\");
    }

    // ============================================================================
    // AnsiStartOfString Tests
    // ============================================================================

    #[test]
    fn test_sos_len_empty() {
        let sos = AnsiStartOfString::Unknown(vec![]);
        assert_eq!(sos.len(), 4); // ESC X ST
    }

    #[test]
    fn test_sos_len_with_data() {
        let sos = AnsiStartOfString::Unknown(b"data".to_vec());
        assert_eq!(sos.len(), 8); // ESC X data ST
    }

    #[test]
    fn test_sos_encode() {
        let sos = AnsiStartOfString::Unknown(b"test".to_vec());
        let mut buffer = BytesMut::new();
        let written = sos.encode(&mut buffer).unwrap();
        assert_eq!(written, 8);
        assert_eq!(&buffer[..], b"\x1bXtest\x1b\\");
    }

    #[test]
    fn test_sos_write() {
        let sos = AnsiStartOfString::Unknown(b"content".to_vec());
        let mut output = Vec::new();
        let written = sos.write(&mut output).unwrap();
        assert_eq!(written, 11);
        assert_eq!(output, b"\x1bXcontent\x1b\\");
    }

    #[test]
    fn test_sos_display() {
        let sos = AnsiStartOfString::Unknown(b"msg".to_vec());
        assert_eq!(sos.to_string(), "\x1bXmsg\x1b\\");
    }

    // ============================================================================
    // AnsiPrivacyMessage Tests
    // ============================================================================

    #[test]
    fn test_pm_len_empty() {
        let pm = AnsiPrivacyMessage::Unknown(vec![]);
        assert_eq!(pm.len(), 4); // ESC ^ ST
    }

    #[test]
    fn test_pm_len_with_data() {
        let pm = AnsiPrivacyMessage::Unknown(b"private".to_vec());
        assert_eq!(pm.len(), 11); // ESC ^ private ST
    }

    #[test]
    fn test_pm_encode() {
        let pm = AnsiPrivacyMessage::Unknown(b"data".to_vec());
        let mut buffer = BytesMut::new();
        let written = pm.encode(&mut buffer).unwrap();
        assert_eq!(written, 8);
        assert_eq!(&buffer[..], b"\x1b^data\x1b\\");
    }

    #[test]
    fn test_pm_write() {
        let pm = AnsiPrivacyMessage::Unknown(b"msg".to_vec());
        let mut output = Vec::new();
        let written = pm.write(&mut output).unwrap();
        assert_eq!(written, 7);
        assert_eq!(output, b"\x1b^msg\x1b\\");
    }

    #[test]
    fn test_pm_display() {
        let pm = AnsiPrivacyMessage::Unknown(b"info".to_vec());
        assert_eq!(pm.to_string(), "\x1b^info\x1b\\");
    }

    // ============================================================================
    // AnsiApplicationProgramCommand Tests
    // ============================================================================

    #[test]
    fn test_apc_len_empty() {
        let apc = AnsiApplicationProgramCommand::Unknown(vec![]);
        assert_eq!(apc.len(), 4); // ESC _ ST
    }

    #[test]
    fn test_apc_len_with_data() {
        let apc = AnsiApplicationProgramCommand::Unknown(b"command".to_vec());
        assert_eq!(apc.len(), 11); // ESC _ command ST
    }

    #[test]
    fn test_apc_encode() {
        let apc = AnsiApplicationProgramCommand::Unknown(b"custom".to_vec());
        let mut buffer = BytesMut::new();
        let written = apc.encode(&mut buffer).unwrap();
        assert_eq!(written, 10);
        assert_eq!(&buffer[..], b"\x1b_custom\x1b\\");
    }

    #[test]
    fn test_apc_write() {
        let apc = AnsiApplicationProgramCommand::Unknown(b"action".to_vec());
        let mut output = Vec::new();
        let written = apc.write(&mut output).unwrap();
        assert_eq!(written, 10);
        assert_eq!(output, b"\x1b_action\x1b\\");
    }

    #[test]
    fn test_apc_display() {
        let apc = AnsiApplicationProgramCommand::Unknown(b"cmd".to_vec());
        assert_eq!(apc.to_string(), "\x1b_cmd\x1b\\");
    }

    // ============================================================================
    // TelnetCommand Tests
    // ============================================================================

    #[test]
    fn test_telnet_command_noop_len() {
        assert_eq!(TelnetCommand::NoOperation.len(), 3);
    }

    #[test]
    fn test_telnet_command_break_len() {
        assert_eq!(TelnetCommand::Break.len(), 3);
    }

    #[test]
    fn test_telnet_command_ayt_len() {
        assert_eq!(TelnetCommand::AreYouThere.len(), 3);
    }

    #[test]
    fn test_telnet_command_simple_encode() {
        let cmd = TelnetCommand::Break;
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 3);
        assert_eq!(&buffer[..], b"\xff\xf3");
    }

    #[test]
    fn test_telnet_command_noop_encode() {
        let cmd = TelnetCommand::NoOperation;
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 3);
        assert_eq!(&buffer[..], b"\xff\xf1");
    }

    #[test]
    fn test_telnet_command_ayt_encode() {
        let cmd = TelnetCommand::AreYouThere;
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 3);
        assert_eq!(&buffer[..], b"\xff\xf6");
    }

    #[test]
    fn test_telnet_command_erase_char_encode() {
        let cmd = TelnetCommand::EraseCharacter;
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 3);
        assert_eq!(&buffer[..], b"\xff\xf7");
    }

    #[test]
    fn test_telnet_command_erase_line_encode() {
        let cmd = TelnetCommand::EraseLine;
        let mut buffer = BytesMut::new();
        let written = cmd.encode(&mut buffer).unwrap();
        assert_eq!(written, 3);
        assert_eq!(&buffer[..], b"\xff\xf8");
    }

    #[test]
    fn test_telnet_command_write() {
        let cmd = TelnetCommand::DataMark;
        let mut output = Vec::new();
        let written = cmd.write(&mut output).unwrap();
        assert_eq!(written, 3);
        assert_eq!(output, b"\xff\xf2");
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================

    #[test]
    fn test_multiple_sequences_encode() {
        let mut buffer = BytesMut::new();

        let seq1 = AnsiSequence::Character('H');
        let seq2 = AnsiSequence::Character('i');
        let seq3 = AnsiSequence::Control(AnsiControlCode::LF);

        seq1.encode(&mut buffer).unwrap();
        seq2.encode(&mut buffer).unwrap();
        seq3.encode(&mut buffer).unwrap();

        assert_eq!(&buffer[..], b"Hi\n");
    }

    #[test]
    fn test_mixed_sequence_types() {
        let mut buffer = BytesMut::new();

        let char_seq = AnsiSequence::Character('A');
        let csi_seq = AnsiSequence::AnsiCSI(AnsiControlSequenceIntroducer::SaveCursorPosition);
        let sgr_seq = AnsiSequence::AnsiSGR(AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });

        char_seq.encode(&mut buffer).unwrap();
        csi_seq.encode(&mut buffer).unwrap();
        sgr_seq.encode(&mut buffer).unwrap();

        assert_eq!(&buffer[..], b"A\x1b[s\x1b[1m");
    }

    #[test]
    fn test_string_sequences_encode() {
        let mut buffer = BytesMut::new();

        let dcs = AnsiSequence::AnsiDCS(AnsiDeviceControlString::Unknown(b"test".to_vec()));
        let osc = AnsiSequence::AnsiOSC(AnsiOperatingSystemCommand::Unknown(b"0;Title".to_vec()));

        dcs.encode(&mut buffer).unwrap();
        osc.encode(&mut buffer).unwrap();

        assert_eq!(&buffer[..], b"\x1bPtest\x1b\\\x1b]0;Title\x1b\\");
    }

    #[test]
    fn test_control_code_round_trip() {
        for i in 0u8..=255 {
            if let Some(code) = AnsiControlCode::from_byte(i) {
                assert_eq!(code.to_byte(), i);
            }
        }
    }

    #[test]
    fn test_empty_dcs_osc_sos() {
        let dcs = AnsiDeviceControlString::Unknown(vec![]);
        let osc = AnsiOperatingSystemCommand::Unknown(vec![]);
        let sos = AnsiStartOfString::Unknown(vec![]);

        assert_eq!(dcs.len(), 4);
        assert_eq!(osc.len(), 4);
        assert_eq!(sos.len(), 4);
    }

    #[test]
    fn test_csi_various_movements() {
        let movements = vec![
            (AnsiControlSequenceIntroducer::CursorUp(1), b"\x1b[1A"),
            (AnsiControlSequenceIntroducer::CursorDown(2), b"\x1b[2B"),
            (AnsiControlSequenceIntroducer::CursorForward(3), b"\x1b[3C"),
            (AnsiControlSequenceIntroducer::CursorBack(4), b"\x1b[4D"),
        ];

        for (cmd, expected) in movements {
            let mut buffer = BytesMut::new();
            cmd.encode(&mut buffer).unwrap();
            assert_eq!(&buffer[..], expected);
        }
    }

    #[test]
    fn test_sequence_equality() {
        let seq1 = AnsiSequence::Character('A');
        let seq2 = AnsiSequence::Character('A');
        let seq3 = AnsiSequence::Character('B');

        assert_eq!(seq1, seq2);
        assert_ne!(seq1, seq3);
    }

    #[test]
    fn test_sequence_clone() {
        let original = AnsiSequence::Character('X');
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }
}
