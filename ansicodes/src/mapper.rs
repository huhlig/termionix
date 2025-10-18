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

use crate::{CSICommand, ControlCode, EraseInDisplayMode, EraseInLineMode, Style};

/// Result type returned by the ANSI mapper after processing input bytes.
///
/// This enum represents all possible outcomes when parsing a byte stream containing
/// ANSI escape sequences, control codes, and text characters. The mapper operates
/// as a state machine, processing bytes incrementally and returning results as
/// complete sequences are recognized.
///
/// # Examples
///
/// ```rust
/// use termionix_ansicodes::AnsiMapper;
///
/// let mut mapper = AnsiMapper::new();
///
/// // Process regular ASCII character
/// let result = mapper.next(b'A');
/// // Returns AnsiMapperResult::Character('A')
///
/// // Process escape sequence (incomplete state followed by complete)
/// let result = mapper.next(0x1B); // ESC
/// // Returns AnsiMapperResult::Incomplete
/// let result = mapper.next(b'[');
/// // Returns AnsiMapperResult::Incomplete
/// let result = mapper.next(b'H');
/// // Returns AnsiMapperResult::CSI(...)
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnsiMapperResult {
    /// Indicates that more bytes are needed to complete the current sequence.
    ///
    /// This is returned when the mapper is in the middle of parsing a multi-byte
    /// sequence (such as an escape sequence or UTF-8 character) and needs additional
    /// input before it can produce a complete result.
    Incomplete,

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
    Control(ControlCode),

    /// A standalone ESC character (0x1B) that is not part of a recognized sequence.
    ///
    /// This occurs when an ESC character is followed by a byte that doesn't
    /// initiate a known ANSI escape sequence. The ESC was not consumed as part
    /// of a control sequence.
    Escape,

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
    CSI(CSICommand),

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
    /// - `ESC[1;31;42m` - Bold red text on green background
    SGR(Style),

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
    OSC(Vec<u8>),

    /// Device Control String - a sequence for device-specific control.
    ///
    /// Format: `ESC P <params> ST`
    ///
    /// DCS sequences are used to send device-specific control strings to the
    /// terminal. They are terminated by ST (ESC \). The contents are device-
    /// dependent and returned as raw bytes.
    DCS(Vec<u8>),

    /// Start of String - a legacy control sequence.
    ///
    /// Format: `ESC X <data> ST`
    ///
    /// SOS is a rarely used control function from ISO 6429. It marks the start
    /// of a control string that is terminated by ST (ESC \). The contents are
    /// returned as raw bytes.
    SOS(Vec<u8>),

    /// String Terminator - marks the end of a string control sequence.
    ///
    /// Format: `ESC \`
    ///
    /// ST is used to terminate string-type control sequences (OSC, DCS, SOS, PM, APC).
    /// When encountered outside of a string sequence context, it's returned as a
    /// standalone result with empty data.
    ST(Vec<u8>),

    /// Privacy Message - a control sequence for private data.
    ///
    /// Format: `ESC ^ <data> ST`
    ///
    /// PM is a control function from ISO 6429 used to delimit privacy messages.
    /// The sequence is terminated by ST (ESC \) and the contents are returned
    /// as raw bytes.
    PM(Vec<u8>),

    /// Application Program Command - a control sequence for application-specific commands.
    ///
    /// Format: `ESC _ <data> ST`
    ///
    /// APC sequences allow applications to send custom commands through the
    /// terminal. The sequence is terminated by ST (ESC \) and the contents are
    /// returned as raw bytes for application-specific interpretation.
    APC(Vec<u8>),
}

/// Internal state machine states for the ANSI mapper parser.
///
/// The `State` enum represents the current parsing state of the `AnsiMapper` as it
/// processes a byte stream. The mapper transitions between states based on the input
/// bytes, building up complete ANSI sequences, UTF-8 characters, or plain text.
///
/// # State Machine Flow
///
/// The typical flow starts in `Normal` state:
/// - `Normal` → `Escape` (on ESC byte 0x1B)
/// - `Escape` → `CSI`/`OSC`/`DCS`/etc. (based on next byte)
/// - Sequence states → `Normal` (when sequence completes)
/// - `Normal` → `UTF8` (on multi-byte UTF-8 start)
/// - `UTF8` → `Normal` (when UTF-8 character completes)
///
/// # Examples
///
/// ```text
/// Input: "A"
/// State: Normal → Normal
/// Result: Character('A')
///
/// Input: ESC [ H
/// States: Normal → Escape → CSI → Normal
/// Results: Incomplete → Incomplete → CSI(...)
///
/// Input: 0xE2 0x82 0xAC (€)
/// States: Normal → UTF8{expected:2} → UTF8{expected:1} → Normal
/// Results: Incomplete → Incomplete → Unicode('€')
/// ```
enum State {
    /// Normal text parsing mode - the default state.
    ///
    /// In this state, the mapper processes:
    /// - ASCII printable characters (0x20-0x7E)
    /// - Control codes (0x00-0x1F, 0x7F, 0x80-0x9F)
    /// - UTF-8 multi-byte sequence starts (0xC0-0xF7)
    /// - ESC character (0x1B) which transitions to `Escape` state
    ///
    /// This is the state the mapper returns to after completing any sequence.
    Normal,

    /// After receiving an ESC character (0x1B).
    ///
    /// In this state, the mapper waits for the next byte to determine what type
    /// of escape sequence is being parsed:
    /// - `[` → transitions to `CSI` state
    /// - `]` → transitions to `OSC` state
    /// - `P` → transitions to `DCS` state
    /// - `X` → transitions to `SOS` state
    /// - `^` → transitions to `PM` state
    /// - `_` → transitions to `APC` state
    /// - `\` → returns `ST` result and transitions to `Normal`
    /// - Other → returns `Escape` result and transitions to `Normal`
    Escape,

    /// Parsing a CSI (Control Sequence Introducer) sequence.
    ///
    /// Format: `ESC [ <parameters> <final_byte>`
    ///
    /// Entered after receiving `ESC [`. The mapper accumulates bytes until it
    /// encounters a final byte in the range 0x40-0x7E, which completes the sequence.
    /// Parameters can include digits (0-9), semicolons (;), and other intermediate
    /// characters.
    ///
    /// Special handling: If the final byte is 'm', the sequence is parsed as an
    /// SGR (Select Graphic Rendition) command and returns `AnsiMapperResult::SGR`.
    CSI,

    /// Parsing an OSC (Operating System Command) sequence.
    ///
    /// Format: `ESC ] <parameters> ST` or `ESC ] <parameters> BEL`
    ///
    /// Entered after receiving `ESC ]`. The mapper accumulates all bytes until
    /// it encounters either:
    /// - BEL (0x07) - Bell character terminator
    /// - ST (ESC \) - String Terminator sequence
    ///
    /// The accumulated bytes (excluding terminators) are returned as raw data.
    OSC,

    /// Parsing a DCS (Device Control String) sequence.
    ///
    /// Format: `ESC P <data> ST`
    ///
    /// Entered after receiving `ESC P`. The mapper accumulates all bytes until
    /// it encounters ST (ESC \) which terminates the sequence. The accumulated
    /// bytes are returned as raw device-specific data.
    DCS,

    /// Parsing an SOS (Start of String) sequence.
    ///
    /// Format: `ESC X <data> ST`
    ///
    /// Entered after receiving `ESC X`. The mapper accumulates all bytes until
    /// it encounters ST (ESC \) which terminates the sequence. This is a rarely
    /// used ISO 6429 control function.
    SOS,

    /// Parsing a PM (Privacy Message) sequence.
    ///
    /// Format: `ESC ^ <data> ST`
    ///
    /// Entered after receiving `ESC ^`. The mapper accumulates all bytes until
    /// it encounters ST (ESC \) which terminates the sequence. The accumulated
    /// bytes represent a privacy message as defined in ISO 6429.
    PM,

    /// Parsing an APC (Application Program Command) sequence.
    ///
    /// Format: `ESC _ <data> ST`
    ///
    /// Entered after receiving `ESC _`. The mapper accumulates all bytes until
    /// it encounters ST (ESC \) which terminates the sequence. The accumulated
    /// bytes contain application-specific commands.
    APC,

    /// Parsing UTF-8 continuation bytes for a multi-byte character.
    ///
    /// This state is entered when a UTF-8 start byte (0xC0-0xF7) is encountered
    /// in `Normal` state. The mapper then expects 1-3 continuation bytes (0x80-0xBF)
    /// to complete the character.
    ///
    /// # Fields
    ///
    /// * `expected` - Number of continuation bytes still needed (1-3). Decremented
    ///   with each valid continuation byte received.
    /// * `accumulated` - The Unicode code point being assembled. Each continuation
    ///   byte contributes 6 bits to build the final character value.
    ///
    /// # UTF-8 Encoding
    ///
    /// - 2-byte (0xC0-0xDF): 1 continuation byte expected
    /// - 3-byte (0xE0-0xEF): 2 continuation bytes expected
    /// - 4-byte (0xF0-0xF7): 3 continuation bytes expected
    ///
    /// If an invalid continuation byte is received, the mapper returns the Unicode
    /// replacement character (U+FFFD) and transitions back to `Normal` state.
    UTF8 {
        /// Number of continuation bytes still needed to complete the UTF-8 character
        expected: usize,
        /// Partially assembled Unicode code point value
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
///
/// # Design
///
/// The mapper is designed for streaming input processing where bytes arrive one at
/// a time or in chunks. It returns `AnsiMapperResult::Incomplete` when more data is
/// needed and produces complete results only when a full sequence or character has
/// been recognized.
///
/// # State Machine
///
/// Internally maintains a `State` that tracks:
/// - Current parsing context (normal text, inside escape sequence, etc.)
/// - Partially accumulated data for incomplete sequences
/// - UTF-8 decoding progress for multi-byte characters
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use termionix_ansicodes::{AnsiMapper, AnsiMapperResult};
///
/// let mut mapper = AnsiMapper::new();
///
/// // Process regular text
/// match mapper.next(b'H') {
///     AnsiMapperResult::Character('H') => println!("Got character H"),
///     _ => {}
/// }
///
/// // Process an escape sequence incrementally
/// assert!(matches!(mapper.next(0x1B), AnsiMapperResult::Incomplete)); // ESC
/// assert!(matches!(mapper.next(b'['), AnsiMapperResult::Incomplete));  // [
/// match mapper.next(b'H') {
///     AnsiMapperResult::CSI(_) => println!("Got CSI command"),
///     _ => {}
/// }
/// ```
///
/// ## Processing a Stream
///
/// ```rust
/// use termionix_ansicodes::{AnsiMapper, AnsiMapperResult};
///
/// let mut mapper = AnsiMapper::new();
/// let input = b"\x1b[31mHello\x1b[0m";
///
/// for &byte in input {
///     match mapper.next(byte) {
///         AnsiMapperResult::SGR(style) => {
///             println!("Style change: {:?}", style);
///         }
///         AnsiMapperResult::Character(ch) => {
///             print!("{}", ch);
///         }
///         AnsiMapperResult::Incomplete => {
///             // Need more data, continue reading
///         }
///         _ => {}
///     }
/// }
/// ```
///
/// ## Handling UTF-8
///
/// ```rust
/// use termionix_ansicodes::{AnsiMapper, AnsiMapperResult};
///
/// let mut mapper = AnsiMapper::new();
/// let euro = "€".as_bytes(); // [0xE2, 0x82, 0xAC]
///
/// assert!(matches!(mapper.next(euro[0]), AnsiMapperResult::Incomplete));
/// assert!(matches!(mapper.next(euro[1]), AnsiMapperResult::Incomplete));
/// match mapper.next(euro[2]) {
///     AnsiMapperResult::Unicode('€') => println!("Got euro symbol"),
///     _ => {}
/// }
/// ```
///
/// # Thread Safety
///
/// `AnsiMapper` is not thread-safe and should not be shared between threads without
/// external synchronization. Each thread processing terminal input should have its
/// own mapper instance.
///
/// # Performance
///
/// The mapper is optimized for streaming input with minimal allocations. The internal
/// byte buffer only grows when accumulating escape sequence parameters, and is cleared
/// when sequences complete. Most single-byte operations (ASCII characters, control codes)
/// have no allocation overhead.
pub struct AnsiMapper {
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

impl AnsiMapper {
    /// Creates a new ANSI mapper in its initial state.
    ///
    /// The mapper starts in `State::Normal`, ready to process text and escape sequences.
    /// The internal buffer is empty and will only allocate when needed for multi-byte
    /// sequences.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::AnsiMapper;
    ///
    /// let mapper = AnsiMapper::new();
    /// // Ready to process input
    /// ```
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            state: State::Normal,
        }
    }

    /// Resets the ANSI mapper to its initial state, clearing all accumulated data.
    ///
    /// This method discards any partially parsed sequences, UTF-8 characters, or accumulated
    /// bytes in the internal buffer, and returns the mapper to the `Normal` state. It's
    /// equivalent to the state of a newly created mapper via [`AnsiMapper::new()`].
    ///
    /// # When to Use
    ///
    /// Call `clear()` when you need to:
    /// - **Reset after errors**: Recover from corrupted or invalid input sequences
    /// - **Process new stream**: Start parsing a fresh input stream unrelated to previous data
    /// - **Abort incomplete sequences**: Discard partial escape sequences that won't complete
    /// - **Synchronize state**: Ensure the mapper is in a known clean state
    /// - **Reuse parser**: Prepare an existing mapper for a new parsing context
    ///
    /// # Effects
    ///
    /// After calling `clear()`:
    /// - Internal byte buffer is emptied (no accumulated sequence data)
    /// - State machine returns to `State::Normal`
    /// - Partial UTF-8 characters are discarded
    /// - Incomplete escape sequences are discarded
    /// - The mapper is ready to process new input from scratch
    ///
    /// # Performance
    ///
    /// This is an efficient O(1) operation. The internal buffer's capacity is retained,
    /// making subsequent operations potentially more efficient if the mapper is reused.
    ///
    /// # Examples
    ///
    /// ## Basic Usage
    ///
    /// ```rust
    /// use termionix_ansicodes::{AnsiMapper, AnsiMapperResult};
    ///
    /// let mut mapper = AnsiMapper::new();
    ///
    /// // Start parsing a sequence
    /// mapper.next(0x1B); // ESC
    /// mapper.next(b'['); // [
    ///
    /// // Reset the mapper
    /// mapper.clear();
    ///
    /// // Now in clean state, can process new input
    /// let result = mapper.next(b'A');
    /// assert!(matches!(result, AnsiMapperResult::Character('A')));
    /// ```
    ///
    /// ## Handling Corrupted Input
    ///
    /// ```rust
    /// use termionix_ansicodes::{AnsiMapper, AnsiMapperResult};
    ///
    /// let mut mapper = AnsiMapper::new();
    ///
    /// // Receive some invalid or corrupted sequence
    /// mapper.next(0x1B); // ESC
    /// mapper.next(0xFF); // Invalid byte
    ///
    /// // Clear to recover
    /// mapper.clear();
    ///
    /// // Continue with valid input
    /// match mapper.next(b'H') {
    ///     AnsiMapperResult::Character('H') => println!("Recovered successfully"),
    ///     _ => {}
    /// }
    /// ```
    ///
    /// ## Processing Multiple Independent Streams
    ///
    /// ```rust
    /// use termionix_ansicodes::{AnsiMapper, AnsiMapperResult};
    ///
    /// let mut mapper = AnsiMapper::new();
    ///
    /// // Process first stream
    /// for &byte in b"Hello\x1b[31mWorld\x1b[0m" {
    ///     let _ = mapper.next(byte);
    /// }
    ///
    /// // Clear before processing unrelated stream
    /// mapper.clear();
    ///
    /// // Process second stream with fresh state
    /// for &byte in b"\x1b[1mBold\x1b[0m" {
    ///     let _ = mapper.next(byte);
    /// }
    /// ```
    ///
    /// ## Aborting Incomplete Sequences
    ///
    /// ```rust
    /// use termionix_ansicodes::{AnsiMapper, AnsiMapperResult};
    ///
    /// let mut mapper = AnsiMapper::new();
    ///
    /// // Start a UTF-8 sequence
    /// let result = mapper.next(0xE2); // Start of 3-byte sequence
    /// assert!(matches!(result, AnsiMapperResult::Incomplete));
    ///
    /// // Decide to abort instead of completing it
    /// mapper.clear();
    ///
    /// // Mapper is now ready for new input
    /// let result = mapper.next(b'X');
    /// assert!(matches!(result, AnsiMapperResult::Character('X')));
    /// ```
    ///
    /// ## Timeout/Error Recovery Pattern
    ///
    /// ```rust
    /// use termionix_ansicodes::{AnsiMapper, AnsiMapperResult};
    /// use std::time::{Duration, Instant};
    ///
    /// let mut mapper = AnsiMapper::new();
    /// let mut last_incomplete = Instant::now();
    /// let timeout = Duration::from_millis(100);
    ///
    /// fn process_byte(mapper: &mut AnsiMapper, byte: u8, last_incomplete: &mut Instant) {
    ///     match mapper.next(byte) {
    ///         AnsiMapperResult::Incomplete => {
    ///             // Check if we've been incomplete too long
    ///             if last_incomplete.elapsed() > Duration::from_millis(100) {
    ///                 // Timeout - clear and restart
    ///                 mapper.clear();
    ///                 *last_incomplete = Instant::now();
    ///             }
    ///         }
    ///         _ => {
    ///             // Got complete result, reset timer
    ///             *last_incomplete = Instant::now();
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// ## Reusing in a Loop
    ///
    /// ```rust
    /// use termionix_ansicodes::{AnsiMapper, AnsiMapperResult};
    ///
    /// let mut mapper = AnsiMapper::new();
    ///
    /// let messages = vec![
    ///     b"\x1b[31mError\x1b[0m".as_slice(),
    ///     b"\x1b[32mSuccess\x1b[0m".as_slice(),
    ///     b"\x1b[33mWarning\x1b[0m".as_slice(),
    /// ];
    ///
    /// for message in messages {
    ///     // Process each message
    ///     for &byte in message {
    ///         let _ = mapper.next(byte);
    ///     }
    ///
    ///     // Clear between messages for independent parsing
    ///     mapper.clear();
    /// }
    /// ```
    ///
    /// # State Consistency
    ///
    /// The `clear()` method ensures the mapper is in a consistent, predictable state.
    /// Unlike continuing to feed bytes through an incomplete sequence, `clear()` guarantees
    /// that the next byte will be interpreted in `Normal` state, regardless of what was
    /// previously parsed.
    ///
    /// # Use Cases
    ///
    /// - **Error recovery**: Reset after encountering invalid sequences
    /// - **Stream boundaries**: Separate parsing contexts for independent data streams
    /// - **Timeout handling**: Discard incomplete sequences after a timeout period
    /// - **Parser reuse**: Efficiently reuse the same mapper instance across contexts
    /// - **State synchronization**: Ensure known state before critical parsing operations
    /// - **Memory management**: Release accumulated bytes while retaining capacity
    ///
    /// # Comparison with Other Operations
    ///
    /// - [`AnsiMapper::new()`]: Creates a new instance with allocation
    /// - `clear()`: Resets existing instance, reusing allocated memory
    /// - Processing to completion: Finishes current sequence then returns to normal
    /// - `clear()`: Immediately discards partial state and resets
    ///
    /// # See Also
    ///
    /// - [`AnsiMapper::new()`](AnsiMapper::new) - Create a new mapper instance
    /// - [`AnsiMapper::next()`](AnsiMapper::next) - Process the next byte
    /// - [`AnsiMapperResult::Incomplete`](AnsiMapperResult::Incomplete) - Indicates more bytes needed
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
    /// An `AnsiMapperResult` which may be:
    /// - `Incomplete` - More bytes needed to complete the current sequence
    /// - A complete result (Character, Unicode, Control, CSI, SGR, etc.)
    ///
    /// # State Transitions
    ///
    /// This method may change the internal state based on the input byte:
    /// - Receiving ESC (0x1B) in Normal state → Escape state
    /// - Receiving '[' in Escape state → CSI state
    /// - Receiving final byte in CSI state → Normal state
    /// - And many others depending on the sequence being parsed
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::{AnsiMapper, AnsiMapperResult};
    ///
    /// let mut mapper = AnsiMapper::new();
    ///
    /// // Single ASCII character - immediate result
    /// let result = mapper.next(b'A');
    /// assert!(matches!(result, AnsiMapperResult::Character('A')));
    ///
    /// // Start of escape sequence - incomplete
    /// let result = mapper.next(0x1B);
    /// assert!(matches!(result, AnsiMapperResult::Incomplete));
    ///
    /// // Continue escape sequence - still incomplete
    /// let result = mapper.next(b'[');
    /// assert!(matches!(result, AnsiMapperResult::Incomplete));
    ///
    /// // Final byte - complete CSI command
    /// let result = mapper.next(b'A');
    /// assert!(matches!(result, AnsiMapperResult::CSI(_)));
    /// ```
    ///
    /// # Performance
    ///
    /// For single-byte results (ASCII characters, control codes), this method has
    /// minimal overhead with no allocations. Multi-byte sequences may cause the
    /// internal buffer to grow, but the buffer is reused across sequences.

    pub fn next(&mut self, byte: u8) -> AnsiMapperResult {
        match self.state {
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
        }
    }

    fn process_normal(&mut self, byte: u8) -> AnsiMapperResult {
        match byte {
            // ESC character
            0x1B => {
                self.state = State::Escape;
                self.bytes.clear();
                AnsiMapperResult::Incomplete
            }
            // ASCII control characters (excluding ESC)
            0x00..=0x1F | 0x7F => {
                if let Some(control) = ControlCode::from_byte(byte) {
                    AnsiMapperResult::Control(control)
                } else {
                    AnsiMapperResult::Character(byte as char)
                }
            }
            // C1 control characters (0x80-0x9F)
            0x80..=0x9F => {
                if let Some(control) = ControlCode::from_byte(byte) {
                    AnsiMapperResult::Control(control)
                } else {
                    AnsiMapperResult::Character(byte as char)
                }
            }
            // ASCII printable characters
            0x20..=0x7E => AnsiMapperResult::Character(byte as char),
            // UTF-8 multibyte sequences
            0xC0..=0xDF => {
                // 2-byte sequence
                self.state = State::UTF8 {
                    expected: 1,
                    accumulated: (byte as u32 & 0x1F) << 6,
                };
                AnsiMapperResult::Incomplete
            }
            0xE0..=0xEF => {
                // 3-byte sequence
                self.state = State::UTF8 {
                    expected: 2,
                    accumulated: (byte as u32 & 0x0F) << 12,
                };
                AnsiMapperResult::Incomplete
            }
            0xF0..=0xF7 => {
                // 4-byte sequence
                self.state = State::UTF8 {
                    expected: 3,
                    accumulated: (byte as u32 & 0x07) << 18,
                };
                AnsiMapperResult::Incomplete
            }
            _ => AnsiMapperResult::Character(byte as char),
        }
    }

    fn process_escape(&mut self, byte: u8) -> AnsiMapperResult {
        self.state = State::Normal;

        match byte {
            b'[' => {
                // CSI - Control Sequence Introducer
                self.state = State::CSI;
                self.bytes.clear();
                AnsiMapperResult::Incomplete
            }
            b']' => {
                // OSC - Operating System Command
                self.state = State::OSC;
                self.bytes.clear();
                AnsiMapperResult::Incomplete
            }
            b'P' => {
                // DCS - Device Control String
                self.state = State::DCS;
                self.bytes.clear();
                AnsiMapperResult::Incomplete
            }
            b'X' => {
                // SOS - Start of String
                self.state = State::SOS;
                self.bytes.clear();
                AnsiMapperResult::Incomplete
            }
            b'^' => {
                // PM - Privacy Message
                self.state = State::PM;
                self.bytes.clear();
                AnsiMapperResult::Incomplete
            }
            b'_' => {
                // APC - Application Program Command
                self.state = State::APC;
                self.bytes.clear();
                AnsiMapperResult::Incomplete
            }
            b'\\' => {
                // ST - String Terminator
                AnsiMapperResult::ST(Vec::new())
            }
            _ => {
                // Standalone ESC or unknown sequence
                AnsiMapperResult::Escape
            }
        }
    }

    fn process_csi(&mut self, byte: u8) -> AnsiMapperResult {
        self.bytes.push(byte);

        // CSI sequences end with a letter (0x40-0x7E)
        if (0x40..=0x7E).contains(&byte) {
            self.state = State::Normal;

            // Check if it's an SGR sequence (ends with 'm')
            if byte == b'm' {
                // Parse SGR codes
                if let Some(style) = self.parse_sgr() {
                    return AnsiMapperResult::SGR(style);
                }
            }

            // Parse as general CSI command
            let command = self.parse_csi();
            return AnsiMapperResult::CSI(command);
        }

        AnsiMapperResult::Incomplete
    }

    fn process_osc(&mut self, byte: u8) -> AnsiMapperResult {
        // OSC sequences end with BEL (0x07) or ST (ESC \)
        if byte == 0x07 {
            self.state = State::Normal;
            let data = std::mem::take(&mut self.bytes);
            return AnsiMapperResult::OSC(data);
        }

        if byte == 0x1B {
            // Could be start of ST sequence
            self.bytes.push(byte);
            return AnsiMapperResult::Incomplete;
        }

        if !self.bytes.is_empty() && self.bytes[self.bytes.len() - 1] == 0x1B && byte == b'\\' {
            // ST sequence found
            self.state = State::Normal;
            let mut data = std::mem::take(&mut self.bytes);
            data.pop(); // Remove ESC
            return AnsiMapperResult::OSC(data);
        }

        self.bytes.push(byte);
        AnsiMapperResult::Incomplete
    }

    fn process_dcs(&mut self, byte: u8) -> AnsiMapperResult {
        self.process_string_sequence(byte, |data| AnsiMapperResult::DCS(data))
    }

    fn process_sos(&mut self, byte: u8) -> AnsiMapperResult {
        self.process_string_sequence(byte, |data| AnsiMapperResult::SOS(data))
    }

    fn process_pm(&mut self, byte: u8) -> AnsiMapperResult {
        self.process_string_sequence(byte, |data| AnsiMapperResult::PM(data))
    }

    fn process_apc(&mut self, byte: u8) -> AnsiMapperResult {
        self.process_string_sequence(byte, |data| AnsiMapperResult::APC(data))
    }

    fn process_string_sequence<F>(&mut self, byte: u8, constructor: F) -> AnsiMapperResult
    where
        F: FnOnce(Vec<u8>) -> AnsiMapperResult,
    {
        // String sequences end with ST (ESC \)
        if byte == 0x1B {
            self.bytes.push(byte);
            return AnsiMapperResult::Incomplete;
        }

        if !self.bytes.is_empty() && self.bytes[self.bytes.len() - 1] == 0x1B && byte == b'\\' {
            // ST sequence found
            self.state = State::Normal;
            let mut data = std::mem::take(&mut self.bytes);
            data.pop(); // Remove ESC
            return constructor(data);
        }

        self.bytes.push(byte);
        AnsiMapperResult::Incomplete
    }

    fn process_utf8(&mut self, byte: u8, expected: usize, accumulated: u32) -> AnsiMapperResult {
        // UTF-8 continuation bytes are 10xxxxxx
        if (byte & 0xC0) != 0x80 {
            // Invalid continuation byte
            self.state = State::Normal;
            return AnsiMapperResult::Character('\u{FFFD}'); // Replacement character
        }

        let accumulated = accumulated | ((byte as u32 & 0x3F) << ((expected - 1) * 6));

        if expected == 1 {
            // Last byte
            self.state = State::Normal;
            if let Some(ch) = char::from_u32(accumulated) {
                AnsiMapperResult::Unicode(ch)
            } else {
                AnsiMapperResult::Character('\u{FFFD}')
            }
        } else {
            // More bytes expected
            self.state = State::UTF8 {
                expected: expected - 1,
                accumulated,
            };
            AnsiMapperResult::Incomplete
        }
    }

    fn parse_sgr(&self) -> Option<Style> {
        // Parse SGR codes from self.bytes
        // This is a simplified parser
        let _codes_str = std::str::from_utf8(&self.bytes[..self.bytes.len() - 1]).ok()?;

        // For now, return default style
        // A full implementation would parse the numeric codes
        Some(Style::default())
    }

    fn parse_csi(&self) -> CSICommand {
        if self.bytes.is_empty() {
            return CSICommand::Unknown;
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
            b'A' => CSICommand::CursorUp(n),
            b'B' => CSICommand::CursorDown(n),
            b'C' => CSICommand::CursorForward(n),
            b'D' => CSICommand::CursorBack(n),
            b'E' => CSICommand::CursorNextLine(n),
            b'F' => CSICommand::CursorPreviousLine(n),
            b'G' => CSICommand::CursorHorizontalAbsolute(n),
            b'H' | b'f' => {
                // Cursor Position - ESC[row;colH or ESC[row;colf
                let row = params.get(0).copied().unwrap_or(1);
                let col = params.get(1).copied().unwrap_or(1);
                CSICommand::CursorPosition { row, col }
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
                CSICommand::EraseInDisplay(mode)
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
                CSICommand::EraseInLine(mode)
            }
            b'S' => CSICommand::ScrollUp,
            b'T' => CSICommand::ScrollDown,
            b'@' => CSICommand::InsertCharacter,
            b'P' => CSICommand::DeleteCharacter,
            b'L' => CSICommand::InsertLine,
            b'M' => CSICommand::DeleteLine,
            b'X' => CSICommand::EraseCharacter,
            b's' => CSICommand::SaveCursorPosition,
            b'u' => CSICommand::RestoreCursorPosition,
            b'n' => {
                if params_str == "6" {
                    CSICommand::DeviceStatusReport
                } else {
                    CSICommand::Unknown
                }
            }
            b'h' => {
                if params_str.starts_with('?') {
                    CSICommand::DECPrivateModeSet
                } else {
                    CSICommand::SetMode
                }
            }
            b'l' => {
                if params_str.starts_with('?') {
                    CSICommand::DECPrivateModeReset
                } else {
                    CSICommand::ResetMode
                }
            }
            _ => CSICommand::Unknown,
        }
    }
}

impl Default for AnsiMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Basic ASCII Character Tests
    // ============================================================================

    #[test]
    fn test_single_ascii_character() {
        let mut mapper = AnsiMapper::new();
        let result = mapper.next(b'A');

        match result {
            AnsiMapperResult::Character(ch) => assert_eq!(ch, 'A'),
            _ => panic!("Expected Character result"),
        }
    }

    #[test]
    fn test_multiple_ascii_characters() {
        let mut mapper = AnsiMapper::new();

        assert!(matches!(
            mapper.next(b'H'),
            AnsiMapperResult::Character('H')
        ));
        assert!(matches!(
            mapper.next(b'e'),
            AnsiMapperResult::Character('e')
        ));
        assert!(matches!(
            mapper.next(b'l'),
            AnsiMapperResult::Character('l')
        ));
        assert!(matches!(
            mapper.next(b'l'),
            AnsiMapperResult::Character('l')
        ));
        assert!(matches!(
            mapper.next(b'o'),
            AnsiMapperResult::Character('o')
        ));
    }

    #[test]
    fn test_ascii_space() {
        let mut mapper = AnsiMapper::new();
        let result = mapper.next(b' ');

        match result {
            AnsiMapperResult::Character(ch) => assert_eq!(ch, ' '),
            _ => panic!("Expected Character result"),
        }
    }

    #[test]
    fn test_ascii_digits() {
        let mut mapper = AnsiMapper::new();

        for digit in b'0'..=b'9' {
            match mapper.next(digit) {
                AnsiMapperResult::Character(ch) => assert_eq!(ch, digit as char),
                _ => panic!("Expected Character for digit {}", digit),
            }
        }
    }

    #[test]
    fn test_ascii_punctuation() {
        let mut mapper = AnsiMapper::new();
        let punctuation = b"!@#$%^&*()_+-=[]{}|;':\",./<>?";

        for &byte in punctuation {
            match mapper.next(byte) {
                AnsiMapperResult::Character(ch) => assert_eq!(ch, byte as char),
                _ => panic!("Expected Character for {}", byte as char),
            }
        }
    }

    // ============================================================================
    // Control Code Tests
    // ============================================================================

    #[test]
    fn test_line_feed() {
        let mut mapper = AnsiMapper::new();
        let result = mapper.next(0x0A); // LF

        match result {
            AnsiMapperResult::Control(ControlCode::LF) => {}
            _ => panic!("Expected Control(LF)"),
        }
    }

    #[test]
    fn test_carriage_return() {
        let mut mapper = AnsiMapper::new();
        let result = mapper.next(0x0D); // CR

        match result {
            AnsiMapperResult::Control(ControlCode::CR) => {}
            _ => panic!("Expected Control(CR)"),
        }
    }

    #[test]
    fn test_tab() {
        let mut mapper = AnsiMapper::new();
        let result = mapper.next(0x09); // HT

        match result {
            AnsiMapperResult::Control(ControlCode::HT) => {}
            _ => panic!("Expected Control(HT)"),
        }
    }

    #[test]
    fn test_bell() {
        let mut mapper = AnsiMapper::new();
        let result = mapper.next(0x07); // BEL

        match result {
            AnsiMapperResult::Control(ControlCode::BEL) => {}
            _ => panic!("Expected Control(BEL)"),
        }
    }

    #[test]
    fn test_backspace() {
        let mut mapper = AnsiMapper::new();
        let result = mapper.next(0x08); // BS

        match result {
            AnsiMapperResult::Control(ControlCode::BS) => {}
            _ => panic!("Expected Control(BS)"),
        }
    }

    #[test]
    fn test_delete() {
        let mut mapper = AnsiMapper::new();
        let result = mapper.next(0x7F); // DEL

        match result {
            AnsiMapperResult::Control(ControlCode::DEL) => {}
            _ => panic!("Expected Control(DEL)"),
        }
    }

    #[test]
    fn test_null() {
        let mut mapper = AnsiMapper::new();
        let result = mapper.next(0x00); // NUL

        match result {
            AnsiMapperResult::Control(ControlCode::NUL) => {}
            _ => panic!("Expected Control(NUL)"),
        }
    }

    // ============================================================================
    // ESC Character Tests
    // ============================================================================

    #[test]
    fn test_standalone_escape() {
        let mut mapper = AnsiMapper::new();

        // Send ESC followed by non-sequence character
        assert!(matches!(mapper.next(0x1B), AnsiMapperResult::Incomplete));
        assert!(matches!(mapper.next(b'x'), AnsiMapperResult::Escape));
    }

    #[test]
    fn test_escape_then_normal_char() {
        let mut mapper = AnsiMapper::new();

        mapper.next(0x1B); // ESC
        mapper.next(b'N'); // Not a sequence start

        // Should be able to process normal chars after
        assert_eq!(mapper.next(b'A'), AnsiMapperResult::Character('A'));
    }

    // ============================================================================
    // UTF-8 Multibyte Character Tests
    // ============================================================================

    #[test]
    fn test_two_byte_utf8() {
        let mut mapper = AnsiMapper::new();

        // UTF-8 for '©' (copyright symbol): C2 A9
        assert!(matches!(mapper.next(0xC2), AnsiMapperResult::Incomplete));

        match mapper.next(0xA9) {
            AnsiMapperResult::Unicode(ch) => assert_eq!(ch, '©'),
            _ => panic!("Expected Unicode(©)"),
        }
    }

    #[test]
    fn test_three_byte_utf8() {
        let mut mapper = AnsiMapper::new();

        // UTF-8 for '日' (Japanese character): E6 97 A5
        assert!(matches!(mapper.next(0xE6), AnsiMapperResult::Incomplete));
        assert!(matches!(mapper.next(0x97), AnsiMapperResult::Incomplete));

        match mapper.next(0xA5) {
            AnsiMapperResult::Unicode(ch) => assert_eq!(ch, '日'),
            _ => panic!("Expected Unicode(日)"),
        }
    }

    #[test]
    fn test_four_byte_utf8_emoji() {
        let mut mapper = AnsiMapper::new();

        // UTF-8 for '🦀' (crab emoji): F0 9F A6 80
        assert!(matches!(mapper.next(0xF0), AnsiMapperResult::Incomplete));
        assert!(matches!(mapper.next(0x9F), AnsiMapperResult::Incomplete));
        assert!(matches!(mapper.next(0xA6), AnsiMapperResult::Incomplete));

        match mapper.next(0x80) {
            AnsiMapperResult::Unicode(ch) => assert_eq!(ch, '🦀'),
            _ => panic!("Expected Unicode(🦀)"),
        }
    }

    #[test]
    fn test_multiple_unicode_characters() {
        let mut mapper = AnsiMapper::new();

        // '世' (E4 B8 96) and '界' (E7 95 8C)
        mapper.next(0xE4);
        mapper.next(0xB8);
        assert!(matches!(mapper.next(0x96), AnsiMapperResult::Unicode('世')));

        mapper.next(0xE7);
        mapper.next(0x95);
        assert!(matches!(mapper.next(0x8C), AnsiMapperResult::Unicode('界')));
    }

    #[test]
    fn test_invalid_utf8_continuation() {
        let mut mapper = AnsiMapper::new();

        // Start 2-byte sequence but send invalid continuation
        mapper.next(0xC2);

        // Send byte that's not a valid continuation (not 10xxxxxx)
        match mapper.next(0xFF) {
            AnsiMapperResult::Character('\u{FFFD}') => {} // Replacement character
            _ => panic!("Expected replacement character for invalid UTF-8"),
        }
    }

    #[test]
    fn test_ascii_after_unicode() {
        let mut mapper = AnsiMapper::new();

        // Send unicode character
        mapper.next(0xE6);
        mapper.next(0x97);
        mapper.next(0xA5); // '日'

        // Should be able to send ASCII after
        assert!(matches!(
            mapper.next(b'A'),
            AnsiMapperResult::Character('A')
        ));
    }

    // ============================================================================
    // CSI Sequence Tests
    // ============================================================================

    #[test]
    fn test_csi_sequence_start() {
        let mut mapper = AnsiMapper::new();

        assert!(matches!(mapper.next(0x1B), AnsiMapperResult::Incomplete)); // ESC
        assert!(matches!(mapper.next(b'['), AnsiMapperResult::Incomplete)); // [
    }

    #[test]
    fn test_simple_csi_sequence() {
        let mut mapper = AnsiMapper::new();

        // ESC[A (Cursor Up)
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'A') {
            AnsiMapperResult::CSI(_) => {}
            _ => panic!("Expected CSI result"),
        }
    }

    #[test]
    fn test_csi_with_parameter() {
        let mut mapper = AnsiMapper::new();

        // ESC[5A (Cursor Up 5)
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        assert!(matches!(mapper.next(b'5'), AnsiMapperResult::Incomplete));

        match mapper.next(b'A') {
            AnsiMapperResult::CSI(_) => {}
            _ => panic!("Expected CSI result"),
        }
    }

    #[test]
    fn test_csi_with_multiple_parameters() {
        let mut mapper = AnsiMapper::new();

        // ESC[10;20H (Cursor Position)
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'1');
        mapper.next(b'0');
        mapper.next(b';');
        mapper.next(b'2');
        mapper.next(b'0');

        match mapper.next(b'H') {
            AnsiMapperResult::CSI(_) => {}
            _ => panic!("Expected CSI result"),
        }
    }

    #[test]
    fn test_sgr_sequence() {
        let mut mapper = AnsiMapper::new();

        // ESC[31m (Red foreground)
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'3');
        mapper.next(b'1');

        match mapper.next(b'm') {
            AnsiMapperResult::SGR(_) => {}
            _ => panic!("Expected SGR result"),
        }
    }

    #[test]
    fn test_sgr_reset() {
        let mut mapper = AnsiMapper::new();

        // ESC[0m (Reset)
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'0');

        match mapper.next(b'm') {
            AnsiMapperResult::SGR(_) => {}
            _ => panic!("Expected SGR result"),
        }
    }

    #[test]
    fn test_sgr_multiple_codes() {
        let mut mapper = AnsiMapper::new();

        // ESC[1;31;42m (Bold, Red FG, Green BG)
        mapper.next(0x1B);
        mapper.next(b'[');
        mapper.next(b'1');
        mapper.next(b';');
        mapper.next(b'3');
        mapper.next(b'1');
        mapper.next(b';');
        mapper.next(b'4');
        mapper.next(b'2');

        match mapper.next(b'm') {
            AnsiMapperResult::SGR(_) => {}
            _ => panic!("Expected SGR result"),
        }
    }

    // ============================================================================
    // OSC Sequence Tests
    // ============================================================================

    #[test]
    fn test_osc_sequence_with_bel() {
        let mut mapper = AnsiMapper::new();

        // ESC]0;Title\x07 (Set window title)
        mapper.next(0x1B); // ESC
        mapper.next(b']'); // ]
        mapper.next(b'0');
        mapper.next(b';');
        mapper.next(b'T');
        mapper.next(b'e');
        mapper.next(b's');
        mapper.next(b't');

        match mapper.next(0x07) {
            // BEL
            AnsiMapperResult::OSC(data) => {
                assert_eq!(data, b"0;Test");
            }
            _ => panic!("Expected OSC result"),
        }
    }

    #[test]
    fn test_osc_sequence_with_st() {
        let mut mapper = AnsiMapper::new();

        // ESC]0;Title ESC\ (Set window title with ST terminator)
        mapper.next(0x1B); // ESC
        mapper.next(b']'); // ]
        mapper.next(b'0');
        mapper.next(b';');
        mapper.next(b'T');
        mapper.next(b'e');
        mapper.next(b's');
        mapper.next(b't');
        mapper.next(0x1B); // ESC

        match mapper.next(b'\\') {
            // ST
            AnsiMapperResult::OSC(data) => {
                assert_eq!(data, b"0;Test");
            }
            _ => panic!("Expected OSC result"),
        }
    }

    // ============================================================================
    // DCS Sequence Tests
    // ============================================================================

    #[test]
    fn test_dcs_sequence() {
        let mut mapper = AnsiMapper::new();

        // ESC P ... ESC\ (DCS)
        mapper.next(0x1B); // ESC
        mapper.next(b'P'); // P
        mapper.next(b'd');
        mapper.next(b'a');
        mapper.next(b't');
        mapper.next(b'a');
        mapper.next(0x1B); // ESC

        match mapper.next(b'\\') {
            AnsiMapperResult::DCS(data) => {
                assert_eq!(data, b"data");
            }
            _ => panic!("Expected DCS result"),
        }
    }

    // ============================================================================
    // String Terminator Tests
    // ============================================================================

    #[test]
    fn test_string_terminator() {
        let mut mapper = AnsiMapper::new();

        // ESC\ (ST)
        mapper.next(0x1B); // ESC

        match mapper.next(b'\\') {
            AnsiMapperResult::ST(_) => {}
            _ => panic!("Expected ST result"),
        }
    }

    // ============================================================================
    // PM Sequence Tests
    // ============================================================================

    #[test]
    fn test_pm_sequence() {
        let mut mapper = AnsiMapper::new();

        // ESC^ ... ESC\ (PM)
        mapper.next(0x1B); // ESC
        mapper.next(b'^'); // ^
        mapper.next(b't');
        mapper.next(b'e');
        mapper.next(b's');
        mapper.next(b't');
        mapper.next(0x1B); // ESC

        match mapper.next(b'\\') {
            AnsiMapperResult::PM(data) => {
                assert_eq!(data, b"test");
            }
            _ => panic!("Expected PM result"),
        }
    }

    // ============================================================================
    // APC Sequence Tests
    // ============================================================================

    #[test]
    fn test_apc_sequence() {
        let mut mapper = AnsiMapper::new();

        // ESC_ ... ESC\ (APC)
        mapper.next(0x1B); // ESC
        mapper.next(b'_'); // _
        mapper.next(b't');
        mapper.next(b'e');
        mapper.next(b's');
        mapper.next(b't');
        mapper.next(0x1B); // ESC

        match mapper.next(b'\\') {
            AnsiMapperResult::APC(data) => {
                assert_eq!(data, b"test");
            }
            _ => panic!("Expected APC result"),
        }
    }

    // ============================================================================
    // SOS Sequence Tests
    // ============================================================================

    #[test]
    fn test_sos_sequence() {
        let mut mapper = AnsiMapper::new();

        // ESC X ... ESC\ (SOS)
        mapper.next(0x1B); // ESC
        mapper.next(b'X'); // X
        mapper.next(b't');
        mapper.next(b'e');
        mapper.next(b's');
        mapper.next(b't');
        mapper.next(0x1B); // ESC

        match mapper.next(b'\\') {
            AnsiMapperResult::SOS(data) => {
                assert_eq!(data, b"test");
            }
            _ => panic!("Expected SOS result"),
        }
    }

    // ============================================================================
    // Mixed Content Tests
    // ============================================================================

    #[test]
    fn test_text_with_control_codes() {
        let mut mapper = AnsiMapper::new();

        assert!(matches!(
            mapper.next(b'H'),
            AnsiMapperResult::Character('H')
        ));
        assert!(matches!(
            mapper.next(b'i'),
            AnsiMapperResult::Character('i')
        ));
        assert!(matches!(
            mapper.next(0x0A),
            AnsiMapperResult::Control(ControlCode::LF)
        ));
        assert!(matches!(
            mapper.next(b'B'),
            AnsiMapperResult::Character('B')
        ));
        assert!(matches!(
            mapper.next(b'y'),
            AnsiMapperResult::Character('y')
        ));
        assert!(matches!(
            mapper.next(b'e'),
            AnsiMapperResult::Character('e')
        ));
    }

    #[test]
    fn test_styled_text_sequence() {
        let mut mapper = AnsiMapper::new();

        // ESC[31m (Red)
        mapper.next(0x1B);
        mapper.next(b'[');
        mapper.next(b'3');
        mapper.next(b'1');
        assert!(matches!(mapper.next(b'm'), AnsiMapperResult::SGR(_)));

        // Text
        assert!(matches!(
            mapper.next(b'R'),
            AnsiMapperResult::Character('R')
        ));
        assert!(matches!(
            mapper.next(b'e'),
            AnsiMapperResult::Character('e')
        ));
        assert!(matches!(
            mapper.next(b'd'),
            AnsiMapperResult::Character('d')
        ));

        // ESC[0m (Reset)
        mapper.next(0x1B);
        mapper.next(b'[');
        mapper.next(b'0');
        assert!(matches!(mapper.next(b'm'), AnsiMapperResult::SGR(_)));
    }

    #[test]
    fn test_unicode_with_ansi() {
        let mut mapper = AnsiMapper::new();

        // SGR sequence
        mapper.next(0x1B);
        mapper.next(b'[');
        mapper.next(b'1');
        mapper.next(b'm');

        // Unicode character '日'
        mapper.next(0xE6);
        mapper.next(0x97);
        assert!(matches!(mapper.next(0xA5), AnsiMapperResult::Unicode('日')));
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn test_empty_csi_sequence() {
        let mut mapper = AnsiMapper::new();

        // ESC[A (no parameters)
        mapper.next(0x1B);
        mapper.next(b'[');

        match mapper.next(b'A') {
            AnsiMapperResult::CSI(_) => {}
            _ => panic!("Expected CSI result"),
        }
    }

    #[test]
    fn test_consecutive_escape_sequences() {
        let mut mapper = AnsiMapper::new();

        // First sequence
        mapper.next(0x1B);
        mapper.next(b'[');
        mapper.next(b'A');

        // Second sequence immediately after
        mapper.next(0x1B);
        mapper.next(b'[');
        assert!(matches!(mapper.next(b'B'), AnsiMapperResult::CSI(_)));
    }

    #[test]
    fn test_interrupted_sequence() {
        let mut mapper = AnsiMapper::new();

        // Start a sequence
        mapper.next(0x1B);
        assert!(matches!(mapper.next(b'['), AnsiMapperResult::Incomplete));

        // Don't complete it, send something else
        mapper.next(b'1');
        mapper.next(b'0');

        // Complete it
        assert!(matches!(mapper.next(b'A'), AnsiMapperResult::CSI(_)));
    }

    #[test]
    fn test_long_parameter_sequence() {
        let mut mapper = AnsiMapper::new();

        // ESC[1;2;3;4;5;6;7;8;9m
        mapper.next(0x1B);
        mapper.next(b'[');

        for i in 1..=9 {
            mapper.next(b'0' + i);
            if i < 9 {
                mapper.next(b';');
            }
        }

        assert!(matches!(mapper.next(b'm'), AnsiMapperResult::SGR(_)));
    }

    // ============================================================================
    // State Reset Tests
    // ============================================================================

    #[test]
    fn test_state_reset_after_sequence() {
        let mut mapper = AnsiMapper::new();

        // Complete a CSI sequence
        mapper.next(0x1B);
        mapper.next(b'[');
        mapper.next(b'A');

        // Should be back to normal state
        assert!(matches!(
            mapper.next(b'X'),
            AnsiMapperResult::Character('X')
        ));
    }

    #[test]
    fn test_state_reset_after_unicode() {
        let mut mapper = AnsiMapper::new();

        // Complete unicode character
        mapper.next(0xC2);
        mapper.next(0xA9); // ©

        // Should be back to normal state
        assert!(matches!(
            mapper.next(b'X'),
            AnsiMapperResult::Character('X')
        ));
    }

    // ============================================================================
    // parse_csi() Method Tests
    // ============================================================================

    #[test]
    fn test_parse_csi_cursor_up_default() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'A') {
            AnsiMapperResult::CSI(CSICommand::CursorUp(n)) => {
                assert_eq!(n, 1, "Default parameter should be 1");
            }
            _ => panic!("Expected CursorUp"),
        }
    }

    #[test]
    fn test_parse_csi_cursor_up_with_param() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'5'); // param

        match mapper.next(b'A') {
            AnsiMapperResult::CSI(CSICommand::CursorUp(n)) => {
                assert_eq!(n, 5);
            }
            _ => panic!("Expected CursorUp(5)"),
        }
    }

    #[test]
    fn test_parse_csi_cursor_down() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'1');
        mapper.next(b'0'); // param = 10

        match mapper.next(b'B') {
            AnsiMapperResult::CSI(CSICommand::CursorDown(n)) => {
                assert_eq!(n, 10);
            }
            _ => panic!("Expected CursorDown(10)"),
        }
    }

    #[test]
    fn test_parse_csi_cursor_forward() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'3'); // param = 3

        match mapper.next(b'C') {
            AnsiMapperResult::CSI(CSICommand::CursorForward(n)) => {
                assert_eq!(n, 3);
            }
            _ => panic!("Expected CursorForward(3)"),
        }
    }

    #[test]
    fn test_parse_csi_cursor_back() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'7'); // param = 7

        match mapper.next(b'D') {
            AnsiMapperResult::CSI(CSICommand::CursorBack(n)) => {
                assert_eq!(n, 7);
            }
            _ => panic!("Expected CursorBack(7)"),
        }
    }

    #[test]
    fn test_parse_csi_cursor_next_line() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'2'); // param = 2

        match mapper.next(b'E') {
            AnsiMapperResult::CSI(CSICommand::CursorNextLine(n)) => {
                assert_eq!(n, 2);
            }
            _ => panic!("Expected CursorNextLine(2)"),
        }
    }

    #[test]
    fn test_parse_csi_cursor_previous_line() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'3'); // param = 3

        match mapper.next(b'F') {
            AnsiMapperResult::CSI(CSICommand::CursorPreviousLine(n)) => {
                assert_eq!(n, 3);
            }
            _ => panic!("Expected CursorPreviousLine(3)"),
        }
    }

    #[test]
    fn test_parse_csi_cursor_horizontal_absolute() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'4');
        mapper.next(b'0'); // param = 40

        match mapper.next(b'G') {
            AnsiMapperResult::CSI(CSICommand::CursorHorizontalAbsolute(n)) => {
                assert_eq!(n, 40);
            }
            _ => panic!("Expected CursorHorizontalAbsolute(40)"),
        }
    }

    #[test]
    fn test_parse_csi_cursor_position_default() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'H') {
            AnsiMapperResult::CSI(CSICommand::CursorPosition { row, col }) => {
                assert_eq!(row, 1);
                assert_eq!(col, 1);
            }
            _ => panic!("Expected CursorPosition with defaults"),
        }
    }

    #[test]
    fn test_parse_csi_cursor_position_with_params() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'1');
        mapper.next(b'0'); // row = 10
        mapper.next(b';');
        mapper.next(b'2');
        mapper.next(b'0'); // col = 20

        match mapper.next(b'H') {
            AnsiMapperResult::CSI(CSICommand::CursorPosition { row, col }) => {
                assert_eq!(row, 10);
                assert_eq!(col, 20);
            }
            _ => panic!("Expected CursorPosition(10, 20)"),
        }
    }

    #[test]
    fn test_parse_csi_cursor_position_f_variant() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'5'); // row = 5
        mapper.next(b';');
        mapper.next(b'8'); // col = 8

        match mapper.next(b'f') {
            AnsiMapperResult::CSI(CSICommand::CursorPosition { row, col }) => {
                assert_eq!(row, 5);
                assert_eq!(col, 8);
            }
            _ => panic!("Expected CursorPosition(5, 8) with 'f' variant"),
        }
    }

    #[test]
    fn test_parse_csi_erase_in_display_to_end() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'0'); // mode = 0

        match mapper.next(b'J') {
            AnsiMapperResult::CSI(CSICommand::EraseInDisplay(mode)) => {
                assert!(matches!(mode, EraseInDisplayMode::EraseToEndOfScreen));
            }
            _ => panic!("Expected EraseInDisplay(EraseToEndOfScreen)"),
        }
    }

    #[test]
    fn test_parse_csi_erase_in_display_to_beginning() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'1'); // mode = 1

        match mapper.next(b'J') {
            AnsiMapperResult::CSI(CSICommand::EraseInDisplay(mode)) => {
                assert!(matches!(mode, EraseInDisplayMode::EraseToBeginningOfScreen));
            }
            _ => panic!("Expected EraseInDisplay(EraseToBeginningOfScreen)"),
        }
    }

    #[test]
    fn test_parse_csi_erase_in_display_entire_screen() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'2'); // mode = 2

        match mapper.next(b'J') {
            AnsiMapperResult::CSI(CSICommand::EraseInDisplay(mode)) => {
                assert!(matches!(mode, EraseInDisplayMode::EraseEntireScreen));
            }
            _ => panic!("Expected EraseInDisplay(EraseEntireScreen)"),
        }
    }

    #[test]
    fn test_parse_csi_erase_in_display_entire_screen_and_saved() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'3'); // mode = 3

        match mapper.next(b'J') {
            AnsiMapperResult::CSI(CSICommand::EraseInDisplay(mode)) => {
                assert!(matches!(mode, EraseInDisplayMode::EraseEntireScreenAndSavedLines));
            }
            _ => panic!("Expected EraseInDisplay(EraseEntireScreenAndSavedLines)"),
        }
    }

    #[test]
    fn test_parse_csi_erase_in_line_to_end() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'K') {
            AnsiMapperResult::CSI(CSICommand::EraseInLine(mode)) => {
                assert!(matches!(mode, EraseInLineMode::EraseToEndOfLine));
            }
            _ => panic!("Expected EraseInLine(EraseToEndOfLine)"),
        }
    }

    #[test]
    fn test_parse_csi_erase_in_line_to_start() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'1'); // mode = 1

        match mapper.next(b'K') {
            AnsiMapperResult::CSI(CSICommand::EraseInLine(mode)) => {
                assert!(matches!(mode, EraseInLineMode::EraseToStartOfLine));
            }
            _ => panic!("Expected EraseInLine(EraseToStartOfLine)"),
        }
    }

    #[test]
    fn test_parse_csi_erase_in_line_entire_line() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'2'); // mode = 2

        match mapper.next(b'K') {
            AnsiMapperResult::CSI(CSICommand::EraseInLine(mode)) => {
                assert!(matches!(mode, EraseInLineMode::EraseEntireLine));
            }
            _ => panic!("Expected EraseInLine(EraseEntireLine)"),
        }
    }

    #[test]
    fn test_parse_csi_scroll_up() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'S') {
            AnsiMapperResult::CSI(CSICommand::ScrollUp) => {}
            _ => panic!("Expected ScrollUp"),
        }
    }

    #[test]
    fn test_parse_csi_scroll_down() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'T') {
            AnsiMapperResult::CSI(CSICommand::ScrollDown) => {}
            _ => panic!("Expected ScrollDown"),
        }
    }

    #[test]
    fn test_parse_csi_insert_character() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'@') {
            AnsiMapperResult::CSI(CSICommand::InsertCharacter) => {}
            _ => panic!("Expected InsertCharacter"),
        }
    }

    #[test]
    fn test_parse_csi_delete_character() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'P') {
            AnsiMapperResult::CSI(CSICommand::DeleteCharacter) => {}
            _ => panic!("Expected DeleteCharacter"),
        }
    }

    #[test]
    fn test_parse_csi_insert_line() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'L') {
            AnsiMapperResult::CSI(CSICommand::InsertLine) => {}
            _ => panic!("Expected InsertLine"),
        }
    }

    #[test]
    fn test_parse_csi_delete_line() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'M') {
            AnsiMapperResult::CSI(CSICommand::DeleteLine) => {}
            _ => panic!("Expected DeleteLine"),
        }
    }

    #[test]
    fn test_parse_csi_erase_character() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'X') {
            AnsiMapperResult::CSI(CSICommand::EraseCharacter) => {}
            _ => panic!("Expected EraseCharacter"),
        }
    }

    #[test]
    fn test_parse_csi_save_cursor_position() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b's') {
            AnsiMapperResult::CSI(CSICommand::SaveCursorPosition) => {}
            _ => panic!("Expected SaveCursorPosition"),
        }
    }

    #[test]
    fn test_parse_csi_restore_cursor_position() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'u') {
            AnsiMapperResult::CSI(CSICommand::RestoreCursorPosition) => {}
            _ => panic!("Expected RestoreCursorPosition"),
        }
    }

    #[test]
    fn test_parse_csi_device_status_report() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'6'); // param = 6

        match mapper.next(b'n') {
            AnsiMapperResult::CSI(CSICommand::DeviceStatusReport) => {}
            _ => panic!("Expected DeviceStatusReport"),
        }
    }

    #[test]
    fn test_parse_csi_device_status_report_invalid_param() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'5'); // param = 5 (not 6)

        match mapper.next(b'n') {
            AnsiMapperResult::CSI(CSICommand::Unknown) => {}
            _ => panic!("Expected Unknown for invalid DSR param"),
        }
    }

    #[test]
    fn test_parse_csi_set_mode() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'4'); // param

        match mapper.next(b'h') {
            AnsiMapperResult::CSI(CSICommand::SetMode) => {}
            _ => panic!("Expected SetMode"),
        }
    }

    #[test]
    fn test_parse_csi_dec_private_mode_set() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'?'); // DEC private mode prefix
        mapper.next(b'2');
        mapper.next(b'5'); // param = 25

        match mapper.next(b'h') {
            AnsiMapperResult::CSI(CSICommand::DECPrivateModeSet) => {}
            _ => panic!("Expected DECPrivateModeSet"),
        }
    }

    #[test]
    fn test_parse_csi_reset_mode() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'4'); // param

        match mapper.next(b'l') {
            AnsiMapperResult::CSI(CSICommand::ResetMode) => {}
            _ => panic!("Expected ResetMode"),
        }
    }

    #[test]
    fn test_parse_csi_dec_private_mode_reset() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'?'); // DEC private mode prefix
        mapper.next(b'2');
        mapper.next(b'5'); // param = 25

        match mapper.next(b'l') {
            AnsiMapperResult::CSI(CSICommand::DECPrivateModeReset) => {}
            _ => panic!("Expected DECPrivateModeReset"),
        }
    }

    #[test]
    fn test_parse_csi_unknown_command() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [

        match mapper.next(b'Z') {
            AnsiMapperResult::CSI(CSICommand::Unknown) => {}
            _ => panic!("Expected Unknown for unrecognized command"),
        }
    }

    #[test]
    fn test_parse_csi_large_parameter() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'2');
        mapper.next(b'5');
        mapper.next(b'5'); // param = 255 (max u8)

        match mapper.next(b'A') {
            AnsiMapperResult::CSI(CSICommand::CursorUp(n)) => {
                assert_eq!(n, 255);
            }
            _ => panic!("Expected CursorUp(255)"),
        }
    }

    #[test]
    fn test_parse_csi_multiple_parameters() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'1'); // param 1
        mapper.next(b';');
        mapper.next(b'2'); // param 2
        mapper.next(b';');
        mapper.next(b'3'); // param 3

        // This will parse as CursorPosition with row=1, col=2
        // The third parameter is ignored by CursorPosition
        match mapper.next(b'H') {
            AnsiMapperResult::CSI(CSICommand::CursorPosition { row, col }) => {
                assert_eq!(row, 1);
                assert_eq!(col, 2);
            }
            _ => panic!("Expected CursorPosition(1, 2)"),
        }
    }

    #[test]
    fn test_parse_csi_empty_parameters() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b';'); // Empty first param
        mapper.next(b';'); // Empty second param

        match mapper.next(b'H') {
            AnsiMapperResult::CSI(CSICommand::CursorPosition { row, col }) => {
                assert_eq!(row, 1); // Should default to 1
                assert_eq!(col, 1); // Should default to 1
            }
            _ => panic!("Expected CursorPosition with defaults"),
        }
    }

    #[test]
    fn test_parse_csi_sgr_returns_sgr_result() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'1'); // Bold

        // SGR sequences should return SGR result, not CSI
        match mapper.next(b'm') {
            AnsiMapperResult::SGR(_) => {}
            _ => panic!("Expected SGR result for 'm' command"),
        }
    }

    #[test]
    fn test_parse_csi_partial_parameter_parsing() {
        let mut mapper = AnsiMapper::new();
        mapper.next(0x1B); // ESC
        mapper.next(b'['); // [
        mapper.next(b'1');
        mapper.next(b'2'); // First param = 12
        mapper.next(b';');
        mapper.next(b'3');
        mapper.next(b'4'); // Second param = 34
        mapper.next(b';');
        // No third param before final byte

        match mapper.next(b'H') {
            AnsiMapperResult::CSI(CSICommand::CursorPosition { row, col }) => {
                assert_eq!(row, 12);
                assert_eq!(col, 34);
            }
            _ => panic!("Expected CursorPosition(12, 34)"),
        }
    }
}
