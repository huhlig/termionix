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
    AnsiDeviceControlString, AnsiOperatingSystemCommand, AnsiPrivacyMessage, AnsiStartOfString,
    EraseInDisplayMode, EraseInLineMode,
};
use crate::string::{Segment, SegmentedString};
use std::ops::{Index, Range};

/// A collection of [`Span`] objects representing parsed segments of an ANSI-formatted string.
///
/// `SpannedString` provides a convenient wrapper around a vector of spans, offering methods to
/// query the total byte length, count of spans, and iterate over individual segments.
/// This type is returned by [`SpannedString::parse`] and represents the complete parsing
/// result of an input string.
///
/// # Structure
///
/// Internally, `SpannedString` is a newtype wrapper around `Vec<Span>`, providing a more
/// semantic interface for working with parsed ANSI strings. Each span in the collection
/// represents a contiguous segment of the original input with its byte range and type.
///
/// # Methods
///
/// - [`len()`](SpannedString::len) - Returns the total byte length from first to last span
/// - [`count()`](SpannedString::count) - Returns the number of spans in the collection
/// - [`iter()`](SpannedString::iter) - Returns an iterator over the spans
/// - [`parse()`](SpannedString::parse) - Parses a string into a `SpannedString`
///
/// The collection also supports indexing via `[usize]` to access individual spans.
///
/// # Examples
///
/// Basic usage:
///
/// ```rust
/// use termionix_ansicodec::SpannedString;
///
/// let spans = SpannedString::parse("\x1b[31mHello\x1b[0m");
///
/// // Total byte length of the parsed string
/// assert_eq!(spans.len(), 14);
///
/// // Number of distinct spans (CSI, ASCII, CSI)
/// assert_eq!(spans.count(), 3);
///
/// // Access individual spans by index
/// let first_span = &spans[0];
/// assert_eq!(first_span.start(), 0);
/// ```
///
/// Iterating over spans:
///
/// ```rust
/// use termionix_ansicodec::{SpannedString, Span};
///
/// let input = "Hello\nWorld";
/// let spans = SpannedString::parse(input);
///
/// for span in spans.iter() {
///     match span {
///         Span::ASCII { range } => println!("Text: {:?}", &input[range.clone()]),
///         Span::Control { value, .. } => println!("Control: {:?}", value),
///         _ => {}
///     }
/// }
/// ```
///
/// # Performance
///
/// - Indexing is O(1)
/// - Iteration is O(n) where n is the span count
/// - `len()` is O(1) as it only checks first and last spans
/// - `count()` is O(1) as it returns the vector length
///
/// # See Also
///
/// - [`Span`] - Individual span structure with range and type
/// - [`CSICommand`] - CSI command enumeration
/// - [`ControlCode`] - Control code enumeration
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpannedString(Vec<Span>);

impl SpannedString {
    /// Returns the total byte length from the first to last span.
    ///
    /// This method calculates the span of bytes from the start of the first span
    /// to the end of the last span. For an empty `SpannedString`, returns 0.
    ///
    /// # Returns
    ///
    /// The total byte length covered by all spans, or 0 if empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::SpannedString;
    ///
    /// let spans = SpannedString::parse("Hello");
    /// assert_eq!(spans.len(), 5);
    ///
    /// let empty = SpannedString::parse("");
    /// assert_eq!(empty.len(), 0);
    /// ```
    ///
    /// # Performance
    ///
    /// This is an O(1) operation as it only accesses the first and last elements.
    pub fn len(&self) -> usize {
        if let Some(start) = self.0.first() {
            if let Some(end) = self.0.last() {
                end.end() - start.start()
            } else {
                start.end() - start.start()
            }
        } else {
            0
        }
    }
    /// Returns the number of spans in the collection.
    ///
    /// Each span represents a contiguous segment of the parsed string, such as
    /// a block of ASCII text, a control sequence, or a Unicode character sequence.
    ///
    /// # Returns
    ///
    /// The number of [`Span`] elements in the collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::SpannedString;
    ///
    /// let spans = SpannedString::parse("\x1b[31mRed\x1b[0m");
    /// assert_eq!(spans.count(), 3); // CSI, ASCII text, CSI
    ///
    /// let simple = SpannedString::parse("Hello");
    /// assert_eq!(simple.count(), 1); // Single ASCII span
    /// ```
    ///
    /// # Performance
    ///
    /// This is an O(1) operation.
    pub fn count(&self) -> usize {
        self.0.len()
    }
    /// Returns an iterator over the spans in the collection.
    ///
    /// The iterator yields references to each [`Span`] in order, from the
    /// beginning to the end of the parsed string.
    ///
    /// # Returns
    ///
    /// A slice iterator over the internal span collection.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::{SpannedString, Span};
    ///
    /// let spans = SpannedString::parse("Hello\nWorld");
    ///
    /// for span in spans.iter() {
    ///     println!("Span: {} bytes", span.len());
    /// }
    /// ```
    ///
    /// Filtering specific span types:
    ///
    /// ```rust
    /// use termionix_ansicodec::{SpannedString, Span};
    ///
    /// let input = "\x1b[31mRed\x1b[0m Text";
    /// let spans = SpannedString::parse(input);
    ///
    /// let text_spans: Vec<_> = spans.iter()
    ///     .filter(|s| matches!(s, Span::ASCII { .. } | Span::Unicode { .. }))
    ///     .collect();
    /// ```
    pub fn iter(&self) -> std::slice::Iter<'_, Span> {
        self.0.iter()
    }
}

impl Index<usize> for SpannedString {
    type Output = Span;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl SpannedString {
    /// Parses a string containing ANSI escape sequences into a `SpannedString`.
    ///
    /// This method performs a single-pass analysis of the input string, identifying and
    /// categorizing each sequence of bytes into distinct span types. It handles ASCII text,
    /// Unicode characters, control codes, and all common ANSI escape sequences including
    /// CSI, OSC, DCS, and others.
    ///
    /// # Arguments
    ///
    /// * `string` - A string slice to parse. Can contain any combination of:
    ///   - Plain ASCII text (0x20-0x7E)
    ///   - Unicode characters (multi-byte UTF-8 sequences)
    ///   - C0/C1 control codes (newlines, tabs, etc.)
    ///   - ANSI escape sequences (colors, cursor movement, etc.)
    ///
    /// # Returns
    ///
    /// A `SpannedString` containing a collection of [`Span`] objects, each representing
    /// a contiguous segment of the input with its byte range and type. The spans cover
    /// the entire input from byte 0 to the end, with no gaps or overlaps.
    ///
    /// # Parsing Behavior
    ///
    /// ## Span Types Detected
    ///
    /// - **ASCII**: Consecutive printable ASCII characters (0x20-0x7E)
    /// - **Unicode**: Multi-byte UTF-8 sequences (non-ASCII characters)
    /// - **Control**: C0 control codes (0x00-0x1F except ESC) and C1 codes (0x80-0x9F)
    /// - **CSI**: Control Sequence Introducer (ESC [ ... final_byte)
    /// - **OSC**: Operating System Command (ESC ] ... ST or BEL)
    /// - **DCS**: Device Control String (ESC P ... ST)
    /// - **SOS**: Start of String (ESC X ... ST)
    /// - **ST**: String Terminator (ESC \)
    /// - **PM**: Privacy Message (ESC ^ ... ST)
    /// - **APC**: Application Program Command (ESC _ ... ST)
    /// - **Escape**: Standalone ESC character or unrecognized escape sequences
    ///
    /// ## Greedy Segment Merging
    ///
    /// The parser employs intelligent merging strategies to minimize the number of spans:
    ///
    /// - **Consecutive ASCII** → Single ASCII span
    /// - **Consecutive Unicode** → Single Unicode span
    /// - **ASCII followed by Unicode** → Single Unicode span (promoted)
    /// - **Unicode followed by ASCII** → Single Unicode span (merged)
    /// - **Consecutive identical control codes** → Single Control span
    ///
    /// This optimization reduces memory usage while preserving semantic meaning.
    ///
    /// ## CSI Command Parsing
    ///
    /// CSI sequences (ESC [ ... final_byte) are parsed into specific [`CSICommand`] variants:
    ///
    /// - Cursor movement: CursorUp, CursorDown, CursorPosition, etc.
    /// - Erasing: EraseInDisplay, EraseInLine
    /// - Scrolling: ScrollUp, ScrollDown
    /// - Mode setting: SetMode, ResetMode
    /// - Unknown sequences: CSICommand::Unknown
    ///
    /// ## Escape Sequence Termination
    ///
    /// String-type escape sequences (OSC, DCS, SOS, PM, APC) are terminated by:
    /// - **ST** (String Terminator): ESC \ or 0x9C
    /// - **BEL** (Bell): 0x07 (for OSC only)
    ///
    /// If no valid terminator is found, the sequence extends to the end of the input.
    ///
    /// # Performance
    ///
    /// - **Time Complexity**: O(n) where n is the length of the input string in bytes
    /// - **Space Complexity**: O(m) where m is the number of distinct segments
    /// - **Single Pass**: The entire string is processed in one forward iteration
    /// - **No Allocations During Parse**: Spans only store byte ranges, not copied data
    ///
    /// # Examples
    ///
    /// Parse plain text:
    ///
    /// ```rust
    /// use termionix_ansicodec::SpannedString;
    ///
    /// let spans = SpannedString::parse("Hello World");
    /// assert_eq!(spans.count(), 1); // Single ASCII span
    /// assert_eq!(spans.len(), 11);
    /// ```
    ///
    /// Parse text with ANSI colors:
    ///
    /// ```rust
    /// use termionix_ansicodec::{SpannedString, Span};
    ///
    /// let input = "\x1b[31mRed\x1b[0m Normal";
    /// let spans = SpannedString::parse(input);
    ///
    /// // Structure: CSI, ASCII("Red"), CSI, ASCII(" Normal")
    /// assert_eq!(spans.count(), 4);
    /// ```
    ///
    /// Parse mixed content with control codes:
    ///
    /// ```rust
    /// use termionix_ansicodec::{SpannedString, Span, ControlCode};
    ///
    /// let input = "Line 1\nLine 2\tTabbed";
    /// let spans = SpannedString::parse(input);
    ///
    /// // Structure: ASCII, Control(LF), ASCII, Control(HT), ASCII
    /// assert_eq!(spans.count(), 5);
    /// ```
    ///
    /// Parse Unicode text:
    ///
    /// ```rust
    /// use termionix_ansicodec::SpannedString;
    ///
    /// let input = "Hello 世界";
    /// let spans = SpannedString::parse(input);
    ///
    /// // ASCII and Unicode merged into single Unicode span
    /// assert_eq!(spans.count(), 1);
    /// ```
    ///
    /// Parse complex ANSI sequences:
    ///
    /// ```rust
    /// use termionix_ansicodec::{SpannedString, Span};
    ///
    /// let input = "\x1b]0;Window Title\x07\x1b[2J\x1b[H";
    /// let spans = SpannedString::parse(input);
    ///
    /// // Structure: OSC, CSI(EraseInDisplay), CSI(CursorPosition)
    /// assert_eq!(spans.count(), 3);
    /// ```
    ///
    /// Accessing span details:
    ///
    /// ```rust
    /// use termionix_ansicodec::{SpannedString, Span};
    ///
    /// let input = "\x1b[31mRed\x1b[0m";
    /// let spans = SpannedString::parse(input);
    ///
    /// for span in spans.iter() {
    ///     match span {
    ///         Span::CSI { range, value } => {
    ///             println!("CSI command at bytes {}..{}", range.start, range.end);
    ///         }
    ///         Span::ASCII { range } => {
    ///             println!("Text at bytes {}..{}", range.start, range.end);
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    ///
    /// # Edge Cases
    ///
    /// - **Empty String**: Returns an empty `SpannedString` with zero spans
    /// - **Incomplete Escape Sequences**: Treated as Escape span containing the incomplete bytes
    /// - **Invalid UTF-8**: Each byte is treated individually (may create invalid ranges)
    /// - **Malformed CSI**: Parsed as CSICommand::Unknown with available parameters
    /// - **Unterminated String Sequences**: Extend to the end of input
    ///
    /// # Use Cases
    ///
    /// This method is ideal for:
    ///
    /// - **ANSI String Analysis**: Understanding the structure of terminal output
    /// - **Syntax Highlighting**: Identifying different types of content
    /// - **Content Filtering**: Extracting or removing specific span types
    /// - **Format Conversion**: Converting to other representations like [`SegmentedString`]
    /// - **Debugging**: Inspecting the composition of ANSI-formatted strings
    ///
    /// # See Also
    ///
    /// - [`SpannedString::into_segmented_string`] - Convert to `SegmentedString` with owned data
    /// - [`Span`] - Individual span enum with all variants
    /// - [`CSICommand`] - Parsed CSI command types
    /// - [`ControlCode`] - Control code enumeration
    pub fn parse(string: &str) -> SpannedString {
        let bytes = string.as_bytes();
        let mut spans = Vec::new();
        let mut pos = 0;

        while pos < bytes.len() {
            let start = pos;

            match bytes[pos] {
                // ESC - Start of escape sequence
                0x1B => {
                    if pos + 1 >= bytes.len() {
                        // Lone ESC at end
                        spans.push(Span::Escape {
                            range: start..start + 1,
                        });
                        pos += 1;
                    } else {
                        match bytes[pos + 1] {
                            // CSI - Control Sequence Introducer
                            b'[' => {
                                pos += 2; // Skip ESC [
                                let param_start = pos;
                                // Read parameter bytes (0x30-0x3F) and intermediate bytes (0x20-0x2F)
                                while pos < bytes.len()
                                    && (bytes[pos] >= 0x20 && bytes[pos] <= 0x3F)
                                {
                                    pos += 1;
                                }
                                // Capture the parameter bytes and final byte
                                let param_bytes = &bytes[param_start..pos];
                                let final_byte = if pos < bytes.len()
                                    && (bytes[pos] >= 0x40 && bytes[pos] <= 0x7E)
                                {
                                    let fb = bytes[pos];
                                    pos += 1;
                                    Some(fb)
                                } else {
                                    None
                                };

                                // Parse the CSI command
                                let command = parse_csi_command(param_bytes, final_byte);

                                spans.push(Span::CSI {
                                    range: start..pos,
                                    value: command,
                                });
                            }
                            // OSC - Operating System Command
                            b']' => {
                                pos += 2; // Skip ESC ]
                                // Read until ST (ESC \) or BEL (0x07)
                                while pos < bytes.len() {
                                    if bytes[pos] == 0x07 {
                                        pos += 1;
                                        break;
                                    } else if bytes[pos] == 0x1B
                                        && pos + 1 < bytes.len()
                                        && bytes[pos + 1] == b'\\'
                                    {
                                        pos += 2;
                                        break;
                                    }
                                    pos += 1;
                                }
                                spans.push(Span::OSC { range: start..pos });
                            }
                            // DCS - Device Control String
                            b'P' => {
                                pos += 2; // Skip ESC P
                                // Read until ST (ESC \)
                                while pos < bytes.len() {
                                    if bytes[pos] == 0x1B
                                        && pos + 1 < bytes.len()
                                        && bytes[pos + 1] == b'\\'
                                    {
                                        pos += 2;
                                        break;
                                    }
                                    pos += 1;
                                }
                                spans.push(Span::DCS { range: start..pos });
                            }
                            // SOS - Start of String
                            b'X' => {
                                pos += 2; // Skip ESC X
                                // Read until ST (ESC \)
                                while pos < bytes.len() {
                                    if bytes[pos] == 0x1B
                                        && pos + 1 < bytes.len()
                                        && bytes[pos + 1] == b'\\'
                                    {
                                        pos += 2;
                                        break;
                                    }
                                    pos += 1;
                                }
                                spans.push(Span::SOS { range: start..pos });
                            }
                            // PM - Privacy Message
                            b'^' => {
                                pos += 2; // Skip ESC ^
                                // Read until ST (ESC \)
                                while pos < bytes.len() {
                                    if bytes[pos] == 0x1B
                                        && pos + 1 < bytes.len()
                                        && bytes[pos + 1] == b'\\'
                                    {
                                        pos += 2;
                                        break;
                                    }
                                    pos += 1;
                                }
                                spans.push(Span::PM { range: start..pos });
                            }
                            // APC - Application Program Command
                            b'_' => {
                                pos += 2; // Skip ESC _
                                // Read until ST (ESC \)
                                while pos < bytes.len() {
                                    if bytes[pos] == 0x1B
                                        && pos + 1 < bytes.len()
                                        && bytes[pos + 1] == b'\\'
                                    {
                                        pos += 2;
                                        break;
                                    }
                                    pos += 1;
                                }
                                spans.push(Span::APC { range: start..pos });
                            }
                            // ST - String Terminator
                            b'\\' => {
                                pos += 2;
                                spans.push(Span::ST { range: start..pos });
                            }
                            // Other escape sequences (2-byte sequences)
                            _ => {
                                pos += 2;
                                spans.push(Span::Escape { range: start..pos });
                            }
                        }
                    }
                }
                // C0 Control codes (0x00-0x1F except ESC) and DEL (0x7F)
                0x00..=0x1A | 0x1C..=0x1F | 0x7F => {
                    if let Some(control_code) = AnsiControlCode::from_byte(bytes[pos]) {
                        pos += 1;
                        // Greedy: consume consecutive identical control codes
                        while pos < bytes.len() {
                            if let Some(next_code) = AnsiControlCode::from_byte(bytes[pos]) {
                                if next_code == control_code {
                                    pos += 1;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        spans.push(Span::Control {
                            range: start..pos,
                            value: control_code,
                        });
                    } else {
                        pos += 1;
                    }
                }
                // ASCII (single byte, 0x20-0x7E)
                0x20..=0x7E => {
                    // Greedy: consume all consecutive ASCII characters
                    while pos < bytes.len() && (bytes[pos] >= 0x20 && bytes[pos] <= 0x7E) {
                        pos += 1;
                    }
                    // Check if the previous span is Unicode, merge if so
                    if let Some(Span::Unicode { range }) = spans.last_mut() {
                        if range.end == start {
                            // Merge: extend the previous Unicode span to include ASCII
                            range.end = pos;
                            // Keep as Unicode since we're already in a Unicode context
                            continue;
                        }
                    }
                    spans.push(Span::ASCII { range: start..pos });
                }
                // C1 Control codes (0x80-0x9F) or multi-byte UTF-8
                0x80..=0x9F => {
                    // Check if it's a C1 control code
                    if let Some(control_code) = AnsiControlCode::from_byte(bytes[pos]) {
                        pos += 1;
                        // Greedy: consume consecutive identical control codes
                        while pos < bytes.len() {
                            if let Some(next_code) = AnsiControlCode::from_byte(bytes[pos]) {
                                if next_code == control_code {
                                    pos += 1;
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                        spans.push(Span::Control {
                            range: start..pos,
                            value: control_code,
                        });
                    } else {
                        // Treat as start of UTF-8 sequence - greedy Unicode
                        let char_len = utf8_char_len(bytes[pos]);
                        pos = std::cmp::min(pos + char_len, bytes.len());
                        // Greedy: consume all consecutive Unicode characters
                        while pos < bytes.len() && bytes[pos] >= 0x80 {
                            // Check if it's a C1 control code
                            if bytes[pos] <= 0x9F
                                && AnsiControlCode::from_byte(bytes[pos]).is_some()
                            {
                                break;
                            }
                            let char_len = utf8_char_len(bytes[pos]);
                            let next_pos = std::cmp::min(pos + char_len, bytes.len());
                            if next_pos == pos {
                                break;
                            }
                            pos = next_pos;
                        }
                        // Check if previous span is ASCII, merge by converting to Unicode
                        if let Some(Span::ASCII { range }) = spans.last_mut() {
                            if range.end == start {
                                // Merge: extend the previous ASCII span and convert to Unicode
                                let old_range = range.clone();
                                spans.pop();
                                spans.push(Span::Unicode {
                                    range: old_range.start..pos,
                                });
                                continue;
                            }
                        } else if let Some(Span::Unicode { range }) = spans.last_mut() {
                            if range.end == start {
                                // Merge: extend the previous Unicode span
                                range.end = pos;
                                continue;
                            }
                        }
                        spans.push(Span::Unicode { range: start..pos });
                    }
                }
                // Multi-byte UTF-8 (0xA0-0xFF)
                _ => {
                    // Greedy: consume all consecutive Unicode characters
                    while pos < bytes.len() && bytes[pos] >= 0xA0 {
                        let char_len = utf8_char_len(bytes[pos]);
                        let next_pos = std::cmp::min(pos + char_len, bytes.len());
                        if next_pos == pos {
                            break;
                        }
                        pos = next_pos;
                    }
                    // Check if previous span is ASCII, merge by converting to Unicode
                    if let Some(Span::ASCII { range }) = spans.last_mut() {
                        if range.end == start {
                            // Merge: extend previous ASCII span and convert to Unicode
                            let old_range = range.clone();
                            spans.pop();
                            spans.push(Span::Unicode {
                                range: old_range.start..pos,
                            });
                            continue;
                        }
                    } else if let Some(Span::Unicode { range }) = spans.last_mut() {
                        if range.end == start {
                            // Merge: extend previous Unicode span
                            range.end = pos;
                            continue;
                        }
                    }
                    spans.push(Span::Unicode { range: start..pos });
                }
            }
        }

        SpannedString(spans)
    }

    /// Converts this `SpannedString` into a `SegmentedString` by extracting actual content
    /// from the source string.
    ///
    /// This method transforms a lightweight parse result ([`SpannedString`]) that only contains
    /// byte ranges into a [`SegmentedString`] with actual string data. This is useful when you
    /// need to:
    ///
    /// - Build terminal output from parsed ANSI strings
    /// - Manipulate the structure while preserving content
    /// - Convert between different string representations
    /// - Apply transformations to parsed ANSI content
    ///
    /// # Arguments
    ///
    /// * `source` - The original string that was parsed to create this `SpannedString`.
    ///              The byte ranges in each [`Span`] reference positions in this string.
    ///
    /// # Returns
    ///
    /// A new [`SegmentedString`] containing the actual text and control sequences,
    /// ready for further manipulation or rendering.
    ///
    /// # Segment Conversion
    ///
    /// Each [`Span`] variant is converted to its corresponding [`Segment`] variant:
    ///
    /// - **Text Segments**: `ASCII` and `Unicode` spans extract the text from the source
    ///   string and push it using [`push_str`](SegmentedString::push_str), which automatically
    ///   handles merging of adjacent compatible segments.
    ///
    /// - **Control Codes**: `Control` spans are converted to control code segments using
    ///   [`push_control`](SegmentedString::push_control).
    ///
    /// - **Escape Sequences**: `CSI`, `OSC`, `DCS`, etc. are converted to their corresponding
    ///   segment types. For escape sequences containing raw data (OSC, DCS, SOS, ST, PM, APC),
    ///   the byte ranges are extracted from the source and stored as `Vec<u8>`.
    ///
    /// # Memory and Performance
    ///
    /// - **Memory**: This method allocates new `String` and `Vec<u8>` buffers for each segment.
    ///   The resulting `SegmentedString` owns all its data, unlike `SpannedString` which only
    ///   stores ranges.
    ///
    /// - **Performance**: O(n) where n is the number of spans. Each span requires a string
    ///   slice extraction or byte copy operation.
    ///
    /// - **Segment Merging**: Text segments may be merged by `SegmentedString::push_str`,
    ///   potentially reducing the total number of segments in the result.
    ///
    /// # Examples
    ///
    /// Basic conversion:
    ///
    /// ```rust
    /// use termionix_ansicodec::SpannedString;
    ///
    /// let input = "\x1b[31mHello\x1b[0m World";
    /// let spanned = SpannedString::parse(input);
    /// let segmented = spanned.into_segmented_string(input);
    /// ```
    ///
    /// Parsing and converting in one step:
    ///
    /// ```rust
    /// use termionix_ansicodec::SpannedString;
    ///
    /// let ansi_text = "\x1b[1;32mSuccess!\x1b[0m\n";
    /// let segmented = SpannedString::parse(ansi_text)
    ///     .into_segmented_string(ansi_text);
    /// ```
    ///
    /// Working with Unicode content:
    ///
    /// ```rust
    /// use termionix_ansicodes::SpannedString;
    ///
    /// let input = "Hello 世界\n";
    /// let spanned = SpannedString::parse(input);
    /// let segmented = spanned.into_segmented_string(input);
    /// ```
    ///
    /// # Panics
    ///
    /// This method does not panic. If a byte range in any span is invalid (out of bounds or
    /// not on a UTF-8 character boundary), that segment will be silently skipped. This is
    /// handled gracefully by the [`str::get`] method which returns `None` for invalid ranges.
    ///
    /// # See Also
    ///
    /// - [`SpannedString::parse`] - Parse a string into a `SpannedString`
    /// - [`SegmentedString`] - The target type with owned segment data
    /// - [`SegmentedString::push_str`] - How text segments are added (with merging)
    /// - [`SegmentedString::push_segment`] - How non-text segments are added
    pub fn into_segmented_string(&self, source: &str) -> SegmentedString {
        let mut segmented = SegmentedString::empty();

        for span in &self.0 {
            match span {
                Span::ASCII { range } | Span::Unicode { range } => {
                    // Extract the text content from the source string
                    if let Some(text) = source.get(range.clone()) {
                        segmented.push_str(text);
                    }
                }
                Span::Control { value, .. } => {
                    // Push control codes directly
                    segmented.push_ansi_control(*value);
                }
                Span::Escape { .. } => {
                    // Push standalone escape character
                    segmented.push_segment(Segment::Escape);
                }
                Span::CSI { value, .. } => {
                    // Push CSI command
                    segmented.push_segment(Segment::CSI(value.clone()));
                }
                Span::OSC { range } => {
                    // Extract OSC data
                    if let Some(data) = source.get(range.clone()) {
                        segmented.push_segment(Segment::OSC(AnsiOperatingSystemCommand::Unknown(
                            data.as_bytes().to_vec(),
                        )));
                    }
                }
                Span::DCS { range } => {
                    // Extract DCS data
                    if let Some(data) = source.get(range.clone()) {
                        segmented.push_segment(Segment::DCS(AnsiDeviceControlString::Unknown(
                            data.as_bytes().to_vec(),
                        )));
                    }
                }
                Span::SOS { range } => {
                    // Extract SOS data
                    if let Some(data) = source.get(range.clone()) {
                        segmented.push_segment(Segment::SOS(AnsiStartOfString::Unknown(
                            data.as_bytes().to_vec(),
                        )));
                    }
                }
                Span::ST { range } => {
                    // Extract ST data
                    if let Some(_data) = source.get(range.clone()) {
                        segmented.push_segment(Segment::ST);
                    }
                }
                Span::PM { range } => {
                    // Extract PM data
                    if let Some(data) = source.get(range.clone()) {
                        segmented.push_segment(Segment::PM(AnsiPrivacyMessage::Unknown(
                            data.as_bytes().to_vec(),
                        )));
                    }
                }
                Span::APC { range } => {
                    // Extract APC data
                    if let Some(data) = source.get(range.clone()) {
                        segmented.push_segment(Segment::APC(
                            AnsiApplicationProgramCommand::Unknown(data.as_bytes().to_vec()),
                        ));
                    }
                }
            }
        }

        segmented
    }
}

/// Represents a parsed segment of an ANSI-formatted string with its byte range.
///
/// `Span` is the fundamental building block returned by [`SpannedString::parse`], representing
/// a contiguous segment of the input string with a specific type classification. Unlike
/// [`Segment`](crate::segment::Segment) which stores the actual content, `Span` only stores
/// byte ranges (as `Range<usize>`) that reference positions in the original source string.
///
/// This lightweight design makes `Span` ideal for:
///
/// - **Memory-efficient parsing**: No string allocations during parse operations
/// - **Zero-copy analysis**: Inspect string structure without copying data
/// - **Lazy extraction**: Extract content only when needed
/// - **Range-based operations**: Manipulate strings using byte positions
///
/// # Design Philosophy
///
/// `Span` follows a "range-only" design where each variant contains a `range: Range<usize>`
/// field that specifies the byte positions `[start..end)` in the source string. Some variants
/// also include parsed metadata (like `CSICommand` or `ControlCode`) to avoid re-parsing.
///
/// This approach enables:
/// - O(1) span creation during parsing
/// - Minimal memory overhead (just two `usize` per span, plus metadata)
/// - Efficient conversion to other types when needed
///
/// # Variant Categories
///
/// ## Text Content Spans
///
/// - **[`ASCII`](Span::ASCII)**: Contiguous ASCII text (0x20-0x7E)
/// - **[`Unicode`](Span::Unicode)**: Multi-byte UTF-8 sequences
///
/// These variants use greedy matching - consecutive compatible characters are merged into
/// a single span during parsing for efficiency.
///
/// ## Control Character Spans
///
/// - **[`Control`](Span::Control)**: C0/C1 control codes with parsed [`ControlCode`] value
/// - **[`Escape`](Span::Escape)**: Standalone ESC or unrecognized escape sequences
///
/// Control spans include the parsed control code value to avoid re-parsing when converting
/// to other representations.
///
/// ## ANSI Escape Sequence Spans
///
/// - **[`CSI`](Span::CSI)**: Control Sequence Introducer with parsed [`CSICommand`]
/// - **[`OSC`](Span::OSC)**: Operating System Command
/// - **[`DCS`](Span::DCS)**: Device Control String
/// - **[`SOS`](Span::SOS)**: Start of String
/// - **[`ST`](Span::ST)**: String Terminator
/// - **[`PM`](Span::PM)**: Privacy Message
/// - **[`APC`](Span::APC)**: Application Program Command
///
/// CSI spans include the parsed command structure, while other escape sequences store
/// only their byte ranges.
///
/// # Methods
///
/// All span variants support three common operations:
///
/// - [`len()`](Span::len) - Returns the byte length of the span
/// - [`start()`](Span::start) - Returns the starting byte position
/// - [`end()`](Span::end) - Returns the ending byte position
///
/// These methods provide O(1) access to span boundaries.
///
/// # Examples
///
/// ## Extracting Content from Spans
///
/// ```rust
/// use termionix_ansicodec::{SpannedString, Span};
///
/// let input = "\x1b[31mRed\x1b[0m";
/// let spans = SpannedString::parse(input);
///
/// for span in spans.iter() {
///     match span {
///         Span::CSI { range, value } => {
///             println!("CSI at {}..{}: {:?}", range.start, range.end, value);
///         }
///         Span::ASCII { range } => {
///             let text = &input[range.clone()];
///             println!("Text: {:?}", text);
///         }
///         _ => {}
///     }
/// }
/// ```
///
/// ## Inspecting Span Boundaries
///
/// ```rust
/// use termionix_ansicodec::{SpannedString, Span};
///
/// let input = "Hello World";
/// let spans = SpannedString::parse(input);
/// let span = &spans[0];
///
/// assert_eq!(span.start(), 0);
/// assert_eq!(span.end(), 11);
/// assert_eq!(span.len(), 11);
/// ```
///
/// ## Working with Control Codes
///
/// ```rust
/// use termionix_ansicodes::{SpannedString, Span, ControlCode};
///
/// let input = "Line 1\nLine 2";
/// let spans = SpannedString::parse(input);
///
/// for span in spans.iter() {
///     if let Span::Control { range, value } = span {
///         if *value == ControlCode::LF {
///             println!("Newline at byte position {}", range.start);
///         }
///     }
/// }
/// ```
///
/// ## Filtering Specific Span Types
///
/// ```rust
/// use termionix_ansicodes::{SpannedString, Span};
///
/// let input = "\x1b[31mRed\x1b[0m Normal";
/// let spans = SpannedString::parse(input);
///
/// // Count only text spans
/// let text_spans: Vec<_> = spans.iter()
///     .filter(|s| matches!(s, Span::ASCII { .. } | Span::Unicode { .. }))
///     .collect();
///
/// println!("Found {} text spans", text_spans.len());
/// ```
///
/// # Range Semantics
///
/// All `range` fields follow Rust's standard half-open range semantics `[start..end)`:
/// - `start` is inclusive (first byte of the span)
/// - `end` is exclusive (one past the last byte)
/// - Length is always `end - start`
///
/// This matches the behavior of slice indexing: `&string[range]` correctly extracts
/// the span's content.
///
/// # Memory Layout
///
/// Each span variant is approximately:
/// - 16-24 bytes for text/escape spans (just the range)
/// - 24-32 bytes for control spans (range + enum discriminant)
/// - 32-48 bytes for CSI spans (range + parsed command structure)
///
/// This is significantly more compact than storing the actual string content.
///
/// # Conversion
///
/// Spans can be converted to [`Segment`](crate::segment::Segment) instances using
/// [`SpannedString::into_segmented_string`], which extracts the actual content from
/// the source string and creates owned data structures.
///
/// # Comparison with Related Types
///
/// - **[`Span`]** (this type): Lightweight parse result with byte ranges only
/// - **[`Segment`](crate::segment::Segment)**: Owned content for building output
/// - **[`SpannedString`]**: Collection of spans representing a parsed string
/// - **[`SegmentedString`](crate::segment::SegmentedString)**: Collection of segments with owned data
///
/// # Performance Characteristics
///
/// - **Creation**: O(1) - just stores two integers
/// - **Size**: 16-48 bytes depending on variant
/// - **Content Access**: O(1) via slice indexing with the range
/// - **Memory**: No allocations, no data copying
///
/// # See Also
///
/// - [`SpannedString`] - Collection of spans from parsing
/// - [`SpannedString::parse`] - Creates spans from ANSI strings
/// - [`CSICommand`] - Parsed CSI command types
/// - [`ControlCode`] - Control code enumeration
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Span {
    /// ASCII text segment containing printable ASCII characters (0x20-0x7E).
    ///
    /// This variant represents a contiguous sequence of standard ASCII characters,
    /// excluding control codes and the ESC character. ASCII spans are the most
    /// common type in typical English text and are the most efficient to process.
    ///
    /// # Greedy Matching
    ///
    /// During parsing, consecutive ASCII characters are merged into a single span
    /// for efficiency. This means "Hello World" becomes one ASCII span, not eleven
    /// separate character spans.
    ///
    /// # Promotion to Unicode
    ///
    /// If an ASCII span is followed by Unicode characters, the ASCII span is
    /// automatically promoted to a Unicode span and merged. This ensures minimal
    /// span fragmentation while preserving correctness.
    ///
    /// # Character Range
    ///
    /// - Includes: Space (0x20) through tilde (0x7E)
    /// - Excludes: Control codes (0x00-0x1F, 0x7F), ESC (0x1B)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "Hello World";
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::ASCII { range } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "Hello World");
    ///     assert_eq!(range.len(), 11);
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `range`: Byte range `[start..end)` in the source string where this ASCII
    ///   text is located. The range is half-open, meaning `start` is inclusive and
    ///   `end` is exclusive.
    ASCII {
        /// The byte range in the source string where this ASCII text segment is located.
        ///
        /// This is a half-open range `[start..end)` where:
        /// - `start` is the byte offset of the first ASCII character (inclusive)
        /// - `end` is the byte offset after the last ASCII character (exclusive)
        ///
        /// The range can be used directly with slice indexing: `&source[range.clone()]`
        /// will extract the exact ASCII text represented by this span.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use termionix_ansicodes::{SpannedString, Span};
        ///
        /// let input = "Hello";
        /// let spans = SpannedString::parse(input);
        ///
        /// if let Span::ASCII { range } = &spans[0] {
        ///     assert_eq!(range.start, 0);
        ///     assert_eq!(range.end, 5);
        ///     assert_eq!(&input[range.clone()], "Hello");
        /// }
        /// ```
        range: Range<usize>,
    },
    /// Unicode text segment containing multi-byte UTF-8 characters.
    ///
    /// This variant represents a contiguous sequence of non-ASCII UTF-8 characters,
    /// which includes most international text, emoji, and special symbols. Unicode
    /// spans may also contain ASCII characters that were merged during parsing.
    ///
    /// # Greedy Matching
    ///
    /// During parsing, consecutive Unicode characters are merged into a single span.
    /// Additionally, ASCII characters adjacent to Unicode are promoted and merged
    /// into the Unicode span, creating fewer, larger spans.
    ///
    /// # Merging Behavior
    ///
    /// - `Unicode + Unicode` → merged Unicode span
    /// - `ASCII + Unicode` → promoted to Unicode span
    /// - `Unicode + ASCII` → merged into Unicode span
    ///
    /// This means "Hello世界World" becomes a single Unicode span, not three separate
    /// spans (ASCII, Unicode, ASCII).
    ///
    /// # UTF-8 Encoding
    ///
    /// The range includes all bytes of the UTF-8 encoded characters:
    /// - 2-byte sequences: U+0080 to U+07FF
    /// - 3-byte sequences: U+0800 to U+FFFF
    /// - 4-byte sequences: U+10000 to U+10FFFF
    ///
    /// # Examples
    ///
    /// Pure Unicode:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "世界";
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::Unicode { range } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "世界");
    ///     assert_eq!(range.len(), 6); // Each character is 3 bytes
    /// }
    /// ```
    ///
    /// Mixed ASCII and Unicode (merged):
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "Hello世界";
    /// let spans = SpannedString::parse(input);
    ///
    /// // Parsed as a single Unicode span due to merging
    /// assert_eq!(spans.count(), 1);
    /// if let Span::Unicode { range } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "Hello世界");
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `range`: Byte range `[start..end)` in the source string where this Unicode
    ///   text is located. The range spans all bytes of all UTF-8 encoded characters.
    Unicode {
        /// The byte range in the source string where this Unicode text segment is located.
        ///
        /// This is a half-open range `[start..end)` where:
        /// - `start` is the byte offset of the first character (inclusive)
        /// - `end` is the byte offset after the last character (exclusive)
        ///
        /// The range includes all bytes of the UTF-8 encoded characters. For example,
        /// a 3-byte character like "世" (U+4E16) occupies 3 bytes, and the range
        /// length reflects this.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use termionix_ansicodes::{SpannedString, Span};
        ///
        /// let input = "こんにちは"; // 5 characters, 15 bytes
        /// let spans = SpannedString::parse(input);
        ///
        /// if let Span::Unicode { range } = &spans[0] {
        ///     assert_eq!(range.start, 0);
        ///     assert_eq!(range.end, 15);
        ///     assert_eq!(&input[range.clone()], "こんにちは");
        /// }
        /// ```
        range: Range<usize>,
    },
    /// Control character segment (C0 or C1 control codes).
    ///
    /// This variant represents terminal control characters that affect cursor
    /// position, output behavior, or terminal state, but are not ANSI escape
    /// sequences. Common examples include line feeds, tabs, and carriage returns.
    ///
    /// # Control Code Categories
    ///
    /// - **C0 codes** (0x00-0x1F, 0x7F): Basic control characters
    ///   - `LF` (0x0A): Line feed/newline
    ///   - `CR` (0x0D): Carriage return
    ///   - `HT` (0x09): Horizontal tab
    ///   - `BEL` (0x07): Bell/alert
    ///   - `BS` (0x08): Backspace
    ///   - `DEL` (0x7F): Delete
    ///
    /// - **C1 codes** (0x80-0x9F): Extended control characters
    ///   - Rarely used in modern terminals
    ///   - Include `NEL` (Next Line), `IND` (Index), etc.
    ///
    /// # Greedy Matching
    ///
    /// Consecutive *identical* control codes are merged into a single Control span.
    /// For example, three consecutive newlines (`\n\n\n`) become one Control span
    /// with a 3-byte range. Different control codes create separate spans.
    ///
    /// # Examples
    ///
    /// Single control code:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span, ControlCode};
    ///
    /// let input = "Hello\nWorld";
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::Control { range, value } = &spans[1] {
    ///     assert_eq!(*value, ControlCode::LF);
    ///     assert_eq!(&input[range.clone()], "\n");
    /// }
    /// ```
    ///
    /// Multiple consecutive control codes (merged):
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span, ControlCode};
    ///
    /// let input = "\n\n\n";
    /// let spans = SpannedString::parse(input);
    ///
    /// // Three newlines merged into one span
    /// assert_eq!(spans.count(), 1);
    /// if let Span::Control { range, value } = &spans[0] {
    ///     assert_eq!(*value, ControlCode::LF);
    ///     assert_eq!(range.len(), 3);
    /// }
    /// ```
    ///
    /// Different control codes (not merged):
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "\n\t"; // Newline followed by tab
    /// let spans = SpannedString::parse(input);
    ///
    /// // Different control codes create separate spans
    /// assert_eq!(spans.count(), 2);
    /// ```
    ///
    /// # Fields
    ///
    /// - `range`: Byte range where the control code(s) are located
    /// - `value`: The parsed control code type (see [`ControlCode`])
    Control {
        /// The byte range in the source string where this control code segment is located.
        ///
        /// This is a half-open range `[start..end)`. For a single control code, the
        /// range length is 1 byte. For consecutive identical control codes that have
        /// been merged, the range length equals the number of occurrences.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use termionix_ansicodes::{SpannedString, Span};
        ///
        /// let input = "A\nB"; // 'A', newline, 'B'
        /// let spans = SpannedString::parse(input);
        ///
        /// if let Span::Control { range, .. } = &spans[1] {
        ///     assert_eq!(range.start, 1);
        ///     assert_eq!(range.end, 2);
        ///     assert_eq!(range.len(), 1);
        /// }
        /// ```
        range: Range<usize>,

        /// The specific control code represented by this span.
        ///
        /// This field contains the parsed control code type, avoiding the need to
        /// re-parse when converting to other representations. All consecutive
        /// identical control codes in the range share this same value.
        ///
        /// See [`ControlCode`] for the complete list of supported control codes.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use termionix_ansicodes::{SpannedString, Span, ControlCode};
        ///
        /// let input = "Line 1\rLine 2"; // Carriage return
        /// let spans = SpannedString::parse(input);
        ///
        /// if let Span::Control { value, .. } = &spans[1] {
        ///     assert_eq!(*value, ControlCode::CR);
        /// }
        /// ```
        value: AnsiControlCode,
    },

    /// Standalone escape character or unrecognized escape sequence.
    ///
    /// This variant represents either:
    /// 1. A lone ESC character (0x1B) at the end of input
    /// 2. An ESC character followed by an unrecognized sequence
    /// 3. A 2-byte escape sequence that doesn't match known patterns
    ///
    /// # Common Cases
    ///
    /// - **Incomplete sequences**: ESC at end of string or followed by invalid bytes
    /// - **Unknown sequences**: ESC followed by characters that don't form a valid
    ///   CSI, OSC, DCS, or other recognized sequence
    /// - **Legacy sequences**: Older or terminal-specific escape codes
    ///
    /// # Examples
    ///
    /// Lone escape at end:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "Hello\x1b"; // ESC at end
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::Escape { range } = &spans[1] {
    ///     assert_eq!(&input[range.clone()], "\x1b");
    ///     assert_eq!(range.len(), 1);
    /// }
    /// ```
    ///
    /// Unknown escape sequence:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "\x1bZ"; // ESC followed by 'Z' (not a recognized sequence)
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::Escape { range } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "\x1bZ");
    ///     assert_eq!(range.len(), 2);
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `range`: Byte range of the escape character and any following bytes
    Escape {
        /// The byte range in the source string where this escape sequence is located.
        ///
        /// This is a half-open range `[start..end)` that includes:
        /// - The ESC character (0x1B) at position `start`
        /// - Any following byte(s) that don't form a recognized sequence
        ///
        /// The range length is typically 1 (lone ESC) or 2 (ESC + one character).
        ///
        /// # Examples
        ///
        /// ```rust
        /// use termionix_ansicodes::{SpannedString, Span};
        ///
        /// let input = "Text\x1b"; // Incomplete escape at end
        /// let spans = SpannedString::parse(input);
        ///
        /// if let Span::Escape { range } = &spans[1] {
        ///     assert_eq!(range.start, 4);
        ///     assert_eq!(range.end, 5);
        ///     assert_eq!(&input[range.clone()], "\x1b");
        /// }
        /// ```
        range: Range<usize>,
    },

    /// CSI (Control Sequence Introducer) command segment.
    ///
    /// This variant represents a complete CSI sequence used for cursor control,
    /// erasing, scrolling, and other terminal operations. CSI sequences have the
    /// format: `ESC [ <parameters> <final_byte>`
    ///
    /// # Structure
    ///
    /// - **Introducer**: ESC [ (or 0x9B in 8-bit mode)
    /// - **Parameters**: Optional numeric parameters separated by semicolons
    /// - **Final byte**: A character in range 0x40-0x7E that determines the command
    ///
    /// # Command Categories
    ///
    /// - **Cursor movement**: CursorUp, CursorDown, CursorPosition, etc.
    /// - **Erasing**: EraseInDisplay, EraseInLine
    /// - **Scrolling**: ScrollUp, ScrollDown
    /// - **Modes**: SetMode, ResetMode, DECPrivateModeSet
    /// - **Insertion/Deletion**: InsertCharacter, DeleteLine, etc.
    ///
    /// # Examples
    ///
    /// Cursor positioning:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span, CSICommand};
    ///
    /// let input = "\x1b[10;20H"; // Move cursor to row 10, column 20
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::CSI { range, value } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "\x1b[10;20H");
    ///     assert!(matches!(value, CSICommand::CursorPosition { row: 10, col: 20 }));
    /// }
    /// ```
    ///
    /// Erase screen:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span, CSICommand, EraseInDisplayMode};
    ///
    /// let input = "\x1b[2J"; // Clear entire screen
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::CSI { range, value } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "\x1b[2J");
    ///     assert!(matches!(
    ///         value,
    ///         CSICommand::EraseInDisplay(EraseInDisplayMode::EraseEntireScreen)
    ///     ));
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `range`: Byte range of the complete CSI sequence
    /// - `value`: The parsed CSI command with its parameters (see [`CSICommand`])
    CSI {
        /// The byte range in the source string where this CSI sequence is located.
        ///
        /// This is a half-open range `[start..end)` that includes:
        /// - The introducer: `ESC [` (2 bytes)
        /// - Parameter bytes: numeric values and semicolons
        /// - The final byte: command identifier (0x40-0x7E)
        ///
        /// # Examples
        ///
        /// ```rust
        /// use termionix_ansicodes::{SpannedString, Span};
        ///
        /// let input = "Before\x1b[31mRed\x1b[0mAfter";
        /// let spans = SpannedString::parse(input);
        ///
        /// // First CSI: "\x1b[31m" at bytes 6-11
        /// if let Span::CSI { range, .. } = &spans[1] {
        ///     assert_eq!(range.start, 6);
        ///     assert_eq!(range.end, 11);
        ///     assert_eq!(&input[range.clone()], "\x1b[31m");
        /// }
        /// ```
        range: Range<usize>,

        /// The parsed CSI command with its parameters.
        ///
        /// This field contains the fully parsed command structure, so you don't need
        /// to re-parse the raw bytes. The command type and parameters are extracted
        /// during initial parsing for efficient access.
        ///
        /// See [`CSICommand`] for all supported command types.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use termionix_ansicodes::{SpannedString, Span, CSICommand};
        ///
        /// let input = "\x1b[5A"; // Move cursor up 5 lines
        /// let spans = SpannedString::parse(input);
        ///
        /// if let Span::CSI { value, .. } = &spans[0] {
        ///     if let CSICommand::CursorUp(lines) = value {
        ///         assert_eq!(*lines, 5);
        ///     }
        /// }
        /// ```
        value: AnsiControlSequenceIntroducer,
    },

    /// OSC (Operating System Command) segment.
    ///
    /// This variant represents Operating System Commands used to set terminal
    /// properties like window title, icon name, and other OS-level features.
    /// OSC sequences have the format: `ESC ] <command> ; <data> BEL` or `ESC ] <command> ; <data> ST`
    ///
    /// # Structure
    ///
    /// - **Introducer**: ESC ] (or 0x9D in 8-bit mode)
    /// - **Command**: Numeric code (e.g., 0 for icon + title, 2 for title only)
    /// - **Separator**: Semicolon
    /// - **Data**: The actual command data (text, color specs, etc.)
    /// - **Terminator**: BEL (0x07) or ST (ESC \)
    ///
    /// # Common OSC Commands
    ///
    /// - `ESC ] 0 ; title BEL` - Set window title and icon name
    /// - `ESC ] 2 ; title BEL` - Set window title only
    /// - `ESC ] 4 ; index ; color BEL` - Set color palette entry
    /// - `ESC ] 10 ; color BEL` - Set default foreground color
    /// - `ESC ] 11 ; color BEL` - Set default background color
    ///
    /// # Examples
    ///
    /// Setting window title:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "\x1b]0;My Window\x07"; // Set title with BEL terminator
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::OSC { range } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "\x1b]0;My Window\x07");
    /// }
    /// ```
    ///
    /// With ST terminator:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "\x1b]2;Title\x1b\\"; // Set title with ST terminator
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::OSC { range } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "\x1b]2;Title\x1b\\");
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `range`: Byte range of the complete OSC sequence including terminators
    OSC {
        /// The byte range in the source string where this OSC sequence is located.
        ///
        /// This is a half-open range `[start..end)` that includes:
        /// - The introducer: `ESC ]` (2 bytes)
        /// - The command number and data
        /// - The terminator: BEL (1 byte) or ST (2 bytes: `ESC \`)
        ///
        /// If no terminator is found, the range extends to the end of the input.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use termionix_ansicodes::{SpannedString, Span};
        ///
        /// let input = "\x1b]0;Window Title\x07";
        /// let spans = SpannedString::parse(input);
        ///
        /// if let Span::OSC { range } = &spans[0] {
        ///     assert_eq!(range.start, 0);
        ///     assert_eq!(range.len(), 17); // Full sequence including terminators
        /// }
        /// ```
        range: Range<usize>,
    },

    /// DCS (Device Control String) segment.
    ///
    /// This variant represents Device Control Strings used for advanced terminal
    /// features and device-specific commands. DCS sequences are less common than
    /// CSI or OSC in modern terminals but are used for features like sixel graphics,
    /// terminal queries, and programmable keys.
    ///
    /// # Structure
    ///
    /// - **Introducer**: ESC P (or 0x90 in 8-bit mode)
    /// - **Parameters**: Optional parameters similar to CSI
    /// - **Data**: Device-specific command data
    /// - **Terminator**: ST (ESC \) or 0x9C
    ///
    /// # Common Uses
    ///
    /// - Sixel graphics: `ESC P <sixel_data> ST`
    /// - Terminal ID queries and responses
    /// - Programmable function keys
    /// - User-defined keys (DECUDK)
    ///
    /// # Examples
    ///
    /// Simple DCS sequence:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "\x1bP1$tx\x1b\\"; // Request terminal ID
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::DCS { range } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "\x1bP1$tx\x1b\\");
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `range`: Byte range of the complete DCS sequence including terminators
    DCS {
        /// The byte range in the source string where this DCS sequence is located.
        ///
        /// This is a half-open range `[start..end)` that includes:
        /// - The introducer: `ESC P` (2 bytes)
        /// - Parameters and data (variable length)
        /// - The terminator: ST (`ESC \`, 2 bytes) or 0x9C (1 byte)
        ///
        /// If no ST terminator is found, the range extends to the end of the input.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use termionix_ansicodes::{SpannedString, Span};
        ///
        /// let input = "\x1bPData\x1b\\";
        /// let spans = SpannedString::parse(input);
        ///
        /// if let Span::DCS { range } = &spans[0] {
        ///     assert_eq!(range.start, 0);
        ///     assert_eq!(range.end, 8); // Full sequence
        /// }
        /// ```
        range: Range<usize>,
    },

    /// SOS (Start of String) segment.
    ///
    /// This variant represents Start of String sequences, which are rarely used in
    /// modern terminals. SOS is part of the C1 control set and is used to introduce
    /// a control string whose purpose depends on the application or terminal.
    ///
    /// # Structure
    ///
    /// - **Introducer**: ESC X (or 0x98 in 8-bit mode)
    /// - **Data**: Application-defined string data
    /// - **Terminator**: ST (ESC \) or 0x9C
    ///
    /// # Usage
    ///
    /// SOS sequences are application-specific and their interpretation depends on
    /// the terminal or application context. They are rarely encountered in typical
    /// terminal output.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "\x1bXsome data\x1b\\";
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::SOS { range } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "\x1bXsome data\x1b\\");
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `range`: Byte range of the complete SOS sequence including terminators
    SOS {
        /// The byte range in the source string where this SOS sequence is located.
        ///
        /// This is a half-open range `[start..end)` that includes:
        /// - The introducer: `ESC X` (2 bytes)
        /// - The string data (variable length)
        /// - The terminator: ST (`ESC \`, 2 bytes) or 0x9C (1 byte)
        ///
        /// If no ST terminator is found, the range extends to the end of the input.
        range: Range<usize>,
    },

    /// ST (String Terminator) segment.
    ///
    /// This variant represents a standalone String Terminator sequence. While ST
    /// is typically used to terminate OSC, DCS, SOS, PM, and APC sequences, this
    /// variant represents an ST that appears independently.
    ///
    /// # Structure
    ///
    /// - **Sequence**: ESC \ (or 0x9C in 8-bit mode)
    ///
    /// # Usage
    ///
    /// In normal parsing, ST is consumed as part of the sequence it terminates
    /// (OSC, DCS, etc.). This variant only appears when ST is encountered without
    /// a corresponding opening sequence, which is unusual but possible in malformed
    /// or edge-case input.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "Text\x1b\\"; // Standalone ST
    /// let spans = SpannedString::parse(input);
    ///
    /// // Will have Text span followed by ST span
    /// if let Some(Span::ST { range }) = spans.iter().find(|s| matches!(s, Span::ST { .. })) {
    ///     assert_eq!(&input[range.clone()], "\x1b\\");
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `range`: Byte range of the ST sequence (always 2 bytes for ESC \)
    ST {
        /// The byte range in the source string where this ST sequence is located.
        ///
        /// This is a half-open range `[start..end)` that includes the 2-byte
        /// ST sequence: `ESC \` (0x1B 0x5C).
        ///
        /// The range length is always 2 for the 7-bit form (ESC \), or 1 for
        /// the 8-bit form (0x9C).
        range: Range<usize>,
    },

    /// PM (Privacy Message) segment.
    ///
    /// This variant represents Privacy Message sequences, which are part of the C1
    /// control set. PM sequences are rarely used and their specific purpose depends
    /// on the terminal implementation. They were designed for security-related
    /// terminal communications.
    ///
    /// # Structure
    ///
    /// - **Introducer**: ESC ^ (or 0x9E in 8-bit mode)
    /// - **Data**: Privacy-related message data
    /// - **Terminator**: ST (ESC \) or 0x9C
    ///
    /// # Usage
    ///
    /// PM sequences are extremely rare in modern terminal applications and most
    /// terminals don't implement special handling for them. They are included
    /// for completeness in ANSI escape sequence parsing.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "\x1b^private data\x1b\\";
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::PM { range } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "\x1b^private data\x1b\\");
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `range`: Byte range of the complete PM sequence including terminators
    PM {
        /// The byte range in the source string where this PM sequence is located.
        ///
        /// This is a half-open range `[start..end)` that includes:
        /// - The introducer: `ESC ^` (2 bytes)
        /// - The message data (variable length)
        /// - The terminator: ST (`ESC \`, 2 bytes) or 0x9C (1 byte)
        ///
        /// If no ST terminator is found, the range extends to the end of the input.
        range: Range<usize>,
    },

    /// APC (Application Program Command) segment.
    ///
    /// This variant represents Application Program Command sequences used for
    /// application-specific communication with the terminal. APC allows applications
    /// to send custom commands that may be interpreted by the terminal or ignored.
    ///
    /// # Structure
    ///
    /// - **Introducer**: ESC _ (or 0x9F in 8-bit mode)
    /// - **Data**: Application-specific command data
    /// - **Terminator**: ST (ESC \) or 0x9C
    ///
    /// # Usage
    ///
    /// APC sequences are used by some terminals for:
    /// - tmux passthrough sequences
    /// - Terminal-specific extensions
    /// - Application-to-terminal communication protocols
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "\x1b_Gcommand=value\x1b\\";
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::APC { range } = &spans[0] {
    ///     assert_eq!(&input[range.clone()], "\x1b_Gcommand=value\x1b\\");
    /// }
    /// ```
    ///
    /// # Fields
    ///
    /// - `range`: Byte range of the complete APC sequence including terminators
    APC {
        /// The byte range in the source string where this APC sequence is located.
        ///
        /// This is a half-open range `[start..end)` that includes:
        /// - The introducer: `ESC _` (2 bytes)
        /// - The command data (variable length)
        /// - The terminator: ST (`ESC \`, 2 bytes) or 0x9C (1 byte)
        ///
        /// If no ST terminator is found, the range extends to the end of the input.
        ///
        /// # Examples
        ///
        /// ```rust
        /// use termionix_ansicodes::{SpannedString, Span};
        ///
        /// let input = "\x1b_application data\x1b\\";
        /// let spans = SpannedString::parse(input);
        ///
        /// if let Span::APC { range } = &spans[0] {
        ///     assert_eq!(range.start, 0);
        ///     assert_eq!(range.end, 20); // Full sequence
        /// }
        /// ```
        range: Range<usize>,
    },
}

impl Span {
    /// Returns the byte length of this span.
    ///
    /// This method calculates the number of bytes occupied by this span in the source
    /// string by computing `range.end - range.start`. The length represents the total
    /// number of bytes, not the number of characters (which may differ for Unicode text).
    ///
    /// # Returns
    ///
    /// The number of bytes in this span's range. For all span variants, this is always
    /// `end - start`.
    ///
    /// # Performance
    ///
    /// This is an O(1) operation that performs simple integer subtraction on the range
    /// boundaries.
    ///
    /// # Examples
    ///
    /// ## ASCII Text Length
    ///
    /// For ASCII text, the byte length equals the character count:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "Hello";
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::ASCII { range } = &spans[0] {
    ///     assert_eq!(spans[0].len(), 5);
    ///     assert_eq!(range.len(), 5); // Same as span.len()
    /// }
    /// ```
    ///
    /// ## Unicode Text Length
    ///
    /// For Unicode text, the byte length is greater than the character count:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "世界"; // Two characters, 6 bytes (3 bytes each)
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::Unicode { range } = &spans[0] {
    ///     assert_eq!(spans[0].len(), 6); // Byte length
    ///     assert_eq!(input.chars().count(), 2); // Character count
    /// }
    /// ```
    ///
    /// ## ANSI Escape Sequence Length
    ///
    /// For ANSI sequences, includes all bytes including escape codes:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "\x1b[31m"; // Red color CSI sequence
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::CSI { range, .. } = &spans[0] {
    ///     assert_eq!(spans[0].len(), 5); // ESC [ 3 1 m
    /// }
    /// ```
    ///
    /// ## Control Code Length
    ///
    /// Control codes can be merged, so length may be greater than 1:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span, ControlCode};
    ///
    /// let input = "\n\n\n"; // Three newlines merged
    /// let spans = SpannedString::parse(input);
    ///
    /// if let Span::Control { range, value } = &spans[0] {
    ///     assert_eq!(spans[0].len(), 3); // Three bytes
    ///     assert_eq!(*value, ControlCode::LF);
    /// }
    /// ```
    ///
    /// ## Empty Span Check
    ///
    /// While rare, you can check if a span is empty (though the parser doesn't
    /// create empty spans):
    ///
    /// ```rust
    /// use termionix_ansicodes::Span;
    /// use std::ops::Range;
    ///
    /// // Manual span creation (not from parsing)
    /// let span = Span::ASCII { range: 5..5 };
    /// assert_eq!(span.len(), 0); // Empty range
    /// ```
    ///
    /// ## Computing Total Length
    ///
    /// Sum all span lengths to get total byte length:
    ///
    /// ```rust
    /// use termionix_ansicodes::SpannedString;
    ///
    /// let input = "Hello\nWorld";
    /// let spans = SpannedString::parse(input);
    ///
    /// let total_bytes: usize = spans.iter().map(|s| s.len()).sum();
    /// assert_eq!(total_bytes, 11);
    /// assert_eq!(total_bytes, input.len());
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Buffer allocation**: Determine how many bytes to allocate for extraction
    /// - **Progress tracking**: Calculate processing progress through a string
    /// - **Statistics**: Analyze the composition of an ANSI string by span types
    /// - **Validation**: Ensure spans cover the expected byte ranges
    ///
    /// # See Also
    ///
    /// - [`start()`](Span::start) - Get the starting byte position
    /// - [`end()`](Span::end) - Get the ending byte position
    /// - [`SpannedString::len()`] - Get the total length of all spans
    pub fn len(&self) -> usize {
        match self {
            Span::ASCII { range, .. } => range.end - range.start,
            Span::Unicode { range, .. } => range.end - range.start,
            Span::Control { range, .. } => range.end - range.start,
            Span::Escape { range, .. } => range.end - range.start,
            Span::CSI { range, .. } => range.end - range.start,
            Span::OSC { range, .. } => range.end - range.start,
            Span::DCS { range, .. } => range.end - range.start,
            Span::SOS { range, .. } => range.end - range.start,
            Span::ST { range, .. } => range.end - range.start,
            Span::PM { range, .. } => range.end - range.start,
            Span::APC { range, .. } => range.end - range.start,
        }
    }

    /// Returns the starting byte position of this span in the source string.
    ///
    /// This method returns the inclusive start of the byte range, representing the first
    /// byte that belongs to this span. The position is zero-based and suitable for direct
    /// use with slice indexing operations.
    ///
    /// # Returns
    ///
    /// The zero-based byte offset where this span begins (inclusive). This is the `start`
    /// field of the span's internal `range`.
    ///
    /// # Performance
    ///
    /// This is an O(1) operation that simply returns the start value from the range.
    ///
    /// # Examples
    ///
    /// ## Basic Usage
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "Hello World";
    /// let spans = SpannedString::parse(input);
    ///
    /// // First span starts at the beginning
    /// assert_eq!(spans[0].start(), 0);
    /// ```
    ///
    /// ## Multiple Spans
    ///
    /// Consecutive spans have adjacent start positions:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "Hello\nWorld";
    /// let spans = SpannedString::parse(input);
    ///
    /// // "Hello" starts at 0
    /// assert_eq!(spans[0].start(), 0);
    ///
    /// // "\n" starts at 5 (after "Hello")
    /// assert_eq!(spans[1].start(), 5);
    ///
    /// // "World" starts at 6 (after "\n")
    /// assert_eq!(spans[2].start(), 6);
    /// ```
    ///
    /// ## Finding Span Positions
    ///
    /// Locate specific content by finding spans with certain start positions:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "One\nTwo\nThree";
    /// let spans = SpannedString::parse(input);
    ///
    /// // Find the span that starts after the first newline
    /// let after_first_newline = spans.iter()
    ///     .find(|s| s.start() >= 4)
    ///     .unwrap();
    /// ```
    ///
    /// ## Extracting Content by Position
    ///
    /// Use start position to extract content from the source string:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "Hello\x1b[31mRed\x1b[0m";
    /// let spans = SpannedString::parse(input);
    ///
    /// // Extract the first 5 bytes
    /// let text = &input[spans[0].start()..spans[0].end()];
    /// assert_eq!(text, "Hello");
    /// ```
    ///
    /// ## Checking Continuity
    ///
    /// Verify that spans are contiguous (no gaps):
    ///
    /// ```rust
    /// use termionix_ansicodes::SpannedString;
    ///
    /// let input = "Hello World";
    /// let spans = SpannedString::parse(input);
    ///
    /// // Check that each span starts where the previous one ended
    /// for i in 1..spans.count() {
    ///     assert_eq!(spans[i].start(), spans[i-1].end());
    /// }
    /// ```
    ///
    /// ## Computing Relative Positions
    ///
    /// Calculate positions relative to other spans:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "ABCDEF";
    /// let spans = SpannedString::parse(input);
    ///
    /// let first_start = spans[0].start();
    /// // All positions are relative to the first span
    /// for span in spans.iter() {
    ///     let offset = span.start() - first_start;
    ///     println!("Span at offset {}", offset);
    /// }
    /// ```
    ///
    /// ## Finding Overlapping Ranges
    ///
    /// Check if a span overlaps with a given byte position:
    ///
    /// ```rust
    /// use termionix_ansicodes::SpannedString;
    ///
    /// let input = "Hello World";
    /// let spans = SpannedString::parse(input);
    /// let position = 7; // Looking for byte 7
    ///
    /// let containing_span = spans.iter()
    ///     .find(|s| s.start() <= position && position < s.end());
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Substring extraction**: Determine where to start extracting content
    /// - **Position mapping**: Map byte positions to span boundaries
    /// - **Gap detection**: Find gaps between spans in parsed strings
    /// - **Range validation**: Verify span ranges are within expected bounds
    /// - **Content navigation**: Jump to specific positions in the source string
    ///
    /// # Notes
    ///
    /// - The start position is always less than or equal to the end position
    /// - For a properly parsed `SpannedString`, the first span starts at byte 0
    /// - Adjacent spans have `span_n.end() == span_n+1.start()`
    ///
    /// # See Also
    ///
    /// - [`end()`](Span::end) - Get the ending byte position
    /// - [`len()`](Span::len) - Get the byte length of the span
    /// - [`SpannedString::parse`] - Creates spans with correct positions
    pub fn start(&self) -> usize {
        match self {
            Span::ASCII { range, .. } => range.start,
            Span::Unicode { range, .. } => range.start,
            Span::Control { range, .. } => range.start,
            Span::Escape { range, .. } => range.start,
            Span::CSI { range, .. } => range.start,
            Span::OSC { range, .. } => range.start,
            Span::DCS { range, .. } => range.start,
            Span::SOS { range, .. } => range.start,
            Span::ST { range, .. } => range.start,
            Span::PM { range, .. } => range.start,
            Span::APC { range, .. } => range.start,
        }
    }

    /// Returns the ending byte position of this span in the source string.
    ///
    /// This method returns the exclusive end of the byte range, representing one past the
    /// last byte that belongs to this span. This follows standard Rust range semantics
    /// where `range.end` is not included in the range.
    ///
    /// # Returns
    ///
    /// The zero-based byte offset where this span ends (exclusive). This is the `end`
    /// field of the span's internal `range`.
    ///
    /// # Range Semantics
    ///
    /// The end position follows Rust's half-open range convention:
    /// - `start` is inclusive (first byte of the span)
    /// - `end` is exclusive (one past the last byte)
    /// - Valid bytes are at positions `[start..end)`
    /// - Length is always `end - start`
    ///
    /// # Performance
    ///
    /// This is an O(1) operation that simply returns the end value from the range.
    ///
    /// # Examples
    ///
    /// ## Basic Usage
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "Hello";
    /// let spans = SpannedString::parse(input);
    ///
    /// assert_eq!(spans[0].start(), 0);
    /// assert_eq!(spans[0].end(), 5);
    ///
    /// // Extract using the range
    /// let text = &input[spans[0].start()..spans[0].end()];
    /// assert_eq!(text, "Hello");
    /// ```
    ///
    /// ## Half-Open Range Semantics
    ///
    /// The end position is exclusive, so it's one past the last byte:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "ABC";
    /// let spans = SpannedString::parse(input);
    ///
    /// let span = &spans[0];
    /// assert_eq!(span.start(), 0); // First byte is 'A'
    /// assert_eq!(span.end(), 3);   // One past 'C' (which is at index 2)
    /// assert_eq!(span.len(), 3);   // Length is end - start
    /// ```
    ///
    /// ## Consecutive Spans
    ///
    /// For adjacent spans, one's end equals the next's start:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "AB";
    /// let spans = SpannedString::parse(input);
    ///
    /// // Single span "AB"
    /// assert_eq!(spans[0].start(), 0);
    /// assert_eq!(spans[0].end(), 2);
    /// ```
    ///
    /// With multiple spans:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "A\nB";
    /// let spans = SpannedString::parse(input);
    ///
    /// // "A" ends where "\n" begins
    /// assert_eq!(spans[0].end(), 1);
    /// assert_eq!(spans[1].start(), 1);
    ///
    /// // "\n" ends where "B" begins
    /// assert_eq!(spans[1].end(), 2);
    /// assert_eq!(spans[2].start(), 2);
    /// ```
    ///
    /// ## Direct Slice Indexing
    ///
    /// Use start and end directly with slice indexing:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "Hello World";
    /// let spans = SpannedString::parse(input);
    ///
    /// for span in spans.iter() {
    ///     let content = &input[span.start()..span.end()];
    ///     println!("Span content: {:?}", content);
    /// }
    /// ```
    ///
    /// ## Finding the Last Byte
    ///
    /// The last byte of a span is at position `end - 1`:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SpannedString, Span};
    ///
    /// let input = "Hello";
    /// let spans = SpannedString::parse(input);
    ///
    /// let span = &spans[0];
    /// let last_byte_pos = span.end() - 1;
    /// assert_eq!(input.as_bytes()[last_byte_pos], b'o');
    /// ```
    ///
    /// ## Calculating Coverage
    ///
    /// Find the total byte range covered by all spans:
    ///
    /// ```rust
    /// use termionix_ansicodes::SpannedString;
    ///
    /// let input = "Hello World";
    /// let spans = SpannedString::parse(input);
    ///
    /// if let (Some(first), Some(last)) = (spans.iter().next(), spans.iter().last()) {
    ///     let total_start = first.start();
    ///     let total_end = last.end();
    ///     assert_eq!(total_end - total_start, input.len());
    /// }
    /// ```
    ///
    /// ## Checking Span Boundaries
    ///
    /// Verify that spans cover the entire input with no gaps:
    ///
    /// ```rust
    /// use termionix_ansicodes::SpannedString;
    ///
    /// let input = "Hello World";
    /// let spans = SpannedString::parse(input);
    ///
    /// // First span should start at 0
    /// assert_eq!(spans[0].start(), 0);
    ///
    /// // Last span should end at input length
    /// let last_idx = spans.count() - 1;
    /// assert_eq!(spans[last_idx].end(), input.len());
    ///
    /// // Each span should start where previous ended
    /// for i in 1..spans.count() {
    ///     assert_eq!(spans[i].start(), spans[i-1].end());
    /// }
    /// ```
    ///
    /// ## Computing Span Offsets
    ///
    /// Calculate how far into the string each span ends:
    ///
    /// ```rust
    /// use termionix_ansicodes::SpannedString;
    ///
    /// let input = "One\nTwo\nThree";
    /// let spans = SpannedString::parse(input);
    ///
    /// for (i, span) in spans.iter().enumerate() {
    ///     let progress = (span.end() as f64 / input.len() as f64) * 100.0;
    ///     println!("Span {} ends at {:.1}% through the string", i, progress);
    /// }
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Content extraction**: Determine where to stop extracting content
    /// - **Range operations**: Create ranges for slice indexing
    /// - **Boundary checking**: Verify spans don't exceed string bounds
    /// - **Gap analysis**: Find gaps between non-contiguous spans
    /// - **Progress calculation**: Track parsing or processing progress
    ///
    /// # Notes
    ///
    /// - The end position is always greater than or equal to the start position
    /// - `end()` returns a value one past the last valid byte index
    /// - For empty spans (rare), `start() == end()`
    /// - The last span's end typically equals the source string's length
    ///
    /// # See Also
    ///
    /// - [`start()`](Span::start) - Get the starting byte position
    /// - [`len()`](Span::len) - Get the byte length of the span
    /// - [`SpannedString::len()`] - Get the total length from first to last span
    pub fn end(&self) -> usize {
        match self {
            Span::ASCII { range, .. } => range.end,
            Span::Unicode { range, .. } => range.end,
            Span::Control { range, .. } => range.end,
            Span::Escape { range, .. } => range.end,
            Span::CSI { range, .. } => range.end,
            Span::OSC { range, .. } => range.end,
            Span::DCS { range, .. } => range.end,
            Span::SOS { range, .. } => range.end,
            Span::ST { range, .. } => range.end,
            Span::PM { range, .. } => range.end,
            Span::APC { range, .. } => range.end,
        }
    }
}

/// Determine the length of a UTF-8 character from its leading byte
fn utf8_char_len(byte: u8) -> usize {
    if byte & 0b1110_0000 == 0b1100_0000 {
        2 // 2-byte character
    } else if byte & 0b1111_0000 == 0b1110_0000 {
        3 // 3-byte character
    } else if byte & 0b1111_1000 == 0b1111_0000 {
        4 // 4-byte character
    } else {
        1 // Invalid or single byte
    }
}

/// Parses CSI parameter bytes and final byte into a CSICommand
fn parse_csi_command(param_bytes: &[u8], final_byte: Option<u8>) -> AnsiControlSequenceIntroducer {
    let Some(final_byte) = final_byte else {
        return AnsiControlSequenceIntroducer::Unknown;
    };

    // Parse parameters (semicolon-separated numbers)
    let params_str = std::str::from_utf8(param_bytes).unwrap_or("");
    let params: Vec<u8> = params_str
        .split(';')
        .filter_map(|s| s.parse::<u8>().ok())
        .collect();

    match final_byte {
        // Cursor movement
        b'A' => AnsiControlSequenceIntroducer::CursorUp(params.get(0).copied().unwrap_or(1)),
        b'B' => AnsiControlSequenceIntroducer::CursorDown(params.get(0).copied().unwrap_or(1)),
        b'C' => AnsiControlSequenceIntroducer::CursorForward(params.get(0).copied().unwrap_or(1)),
        b'D' => AnsiControlSequenceIntroducer::CursorBack(params.get(0).copied().unwrap_or(1)),
        b'E' => AnsiControlSequenceIntroducer::CursorNextLine(params.get(0).copied().unwrap_or(1)),
        b'F' => {
            AnsiControlSequenceIntroducer::CursorPreviousLine(params.get(0).copied().unwrap_or(1))
        }
        b'G' => AnsiControlSequenceIntroducer::CursorHorizontalAbsolute(
            params.get(0).copied().unwrap_or(1),
        ),
        b'H' | b'f' => AnsiControlSequenceIntroducer::CursorPosition {
            row: params.get(0).copied().unwrap_or(1),
            col: params.get(1).copied().unwrap_or(1),
        },

        // Erase functions
        b'J' => {
            let mode = match params.get(0).copied().unwrap_or(0) {
                0 => EraseInDisplayMode::EraseToEndOfScreen,
                1 => EraseInDisplayMode::EraseToBeginningOfScreen,
                2 => EraseInDisplayMode::EraseEntireScreen,
                3 => EraseInDisplayMode::EraseEntireScreenAndSavedLines,
                _ => EraseInDisplayMode::EraseToEndOfScreen,
            };
            AnsiControlSequenceIntroducer::EraseInDisplay(mode)
        }
        b'K' => {
            let mode = match params.get(0).copied().unwrap_or(0) {
                0 => EraseInLineMode::EraseToEndOfLine,
                1 => EraseInLineMode::EraseToStartOfLine,
                2 => EraseInLineMode::EraseEntireLine,
                _ => EraseInLineMode::EraseToEndOfLine,
            };
            AnsiControlSequenceIntroducer::EraseInLine(mode)
        }

        // Device Status Report
        b'n' if params.get(0) == Some(&6) => AnsiControlSequenceIntroducer::DeviceStatusReport,

        // Save/Restore cursor
        b's' => AnsiControlSequenceIntroducer::SaveCursorPosition,
        b'u' => AnsiControlSequenceIntroducer::RestoreCursorPosition,

        // Scrolling
        b'S' => AnsiControlSequenceIntroducer::ScrollUp,
        b'T' => AnsiControlSequenceIntroducer::ScrollDown,

        // Insert/Delete
        b'@' => AnsiControlSequenceIntroducer::InsertCharacter,
        b'P' => AnsiControlSequenceIntroducer::DeleteCharacter,
        b'L' => AnsiControlSequenceIntroducer::InsertLine,
        b'M' => AnsiControlSequenceIntroducer::DeleteLine,
        b'X' => AnsiControlSequenceIntroducer::EraseCharacter,

        // Set/Reset Mode
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

        // Keyboard strings
        b'p' => AnsiControlSequenceIntroducer::SetKeyboardStrings,

        // SGR - Select Graphic Rendition (m command)
        // Note: SGR commands are now treated as Unknown since SGRCommand was removed
        b'm' => AnsiControlSequenceIntroducer::Unknown,

        _ => AnsiControlSequenceIntroducer::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii() {
        let input = "Hello";
        let spans = SpannedString::parse(input);
        assert_eq!(spans.len(), 5);
        assert_eq!(spans.count(), 1);
        assert!(matches!(spans[0], Span::ASCII { .. }));
        if let Span::ASCII { range } = &spans[0] {
            assert_eq!(range, &(0..5));
            assert_eq!(&input.as_bytes()[range.clone()], b"Hello");
        }
    }

    #[test]
    fn test_unicode() {
        let input = "Hello 世界";
        let spans = SpannedString::parse(input);
        assert_eq!(spans.len(), 12);
        assert_eq!(spans.count(), 1); // "Hello 世界" (Unicode - merged)
        assert!(matches!(spans[0], Span::Unicode { .. }));
        if let Span::Unicode { range } = &spans[0] {
            assert_eq!(range, &(0..12)); // "Hello  世界"
            assert_eq!(
                &input.as_bytes()[range.clone()],
                b"Hello \xE4\xB8\x96\xE7\x95\x8C"
            );
        }
    }

    #[test]
    fn test_csi_color() {
        let input = "\x1b[31mRed\x1b[0m";
        let spans = SpannedString::parse(input);
        assert_eq!(spans.len(), 12);
        assert_eq!(spans.count(), 3); // CSI, "Red" (merged ASCII), CSI

        assert!(matches!(spans[0], Span::CSI { .. }));
        if let Span::CSI { range, .. } = &spans[0] {
            assert_eq!(range, &(0..5)); // "\x1b[31m"
            assert_eq!(&input.as_bytes()[range.clone()], b"\x1b[31m");
        }

        assert!(matches!(spans[1], Span::ASCII { .. }));
        if let Span::ASCII { range } = &spans[1] {
            assert_eq!(range, &(5..8)); // "Red"
            assert_eq!(&input.as_bytes()[range.clone()], b"Red");
        }

        assert!(matches!(spans[2], Span::CSI { .. }));
        if let Span::CSI { range, .. } = &spans[2] {
            assert_eq!(range, &(8..12)); // "\x1b[0m"
            assert_eq!(&input.as_bytes()[range.clone()], b"\x1b[0m");
        }
    }

    #[test]
    fn test_control_codes() {
        let input = "Hello\nWorld\t!";
        let spans = SpannedString::parse(input);
        assert_eq!(spans.count(), 5); // "Hello", \n, "World", \t, "!"
        assert_eq!(spans.len(), 13); // "Hello", \n, "World", \t, "!"

        assert!(matches!(spans[0], Span::ASCII { .. }));
        if let Span::ASCII { range } = &spans[0] {
            assert_eq!(range, &(0..5)); // "Hello"
            assert_eq!(&input.as_bytes()[range.clone()], b"Hello");
        }

        assert!(matches!(
            spans[1],
            Span::Control {
                value: AnsiControlCode::LF,
                ..
            }
        ));
        if let Span::Control { range, .. } = &spans[1] {
            assert_eq!(range, &(5..6)); // \n
            assert_eq!(&input.as_bytes()[range.clone()], b"\n");
        }

        assert!(matches!(spans[2], Span::ASCII { .. }));
        if let Span::ASCII { range } = &spans[2] {
            assert_eq!(range, &(6..11)); // "World"
            assert_eq!(&input.as_bytes()[range.clone()], b"World");
        }

        assert!(matches!(
            spans[3],
            Span::Control {
                value: AnsiControlCode::HT,
                ..
            }
        ));
        if let Span::Control { range, .. } = &spans[3] {
            assert_eq!(range, &(11..12)); // \t
            assert_eq!(&input.as_bytes()[range.clone()], b"\t");
        }

        assert!(matches!(spans[4], Span::ASCII { .. }));
        if let Span::ASCII { range } = &spans[4] {
            assert_eq!(range, &(12..13)); // "!"
            assert_eq!(&input.as_bytes()[range.clone()], b"!");
        }
    }

    #[test]
    fn test_specific_control_codes() {
        let input = "A\x07B\x08C\rD";
        let spans = SpannedString::parse(input);

        assert_eq!(spans.count(), 7);
        assert_eq!(spans.len(), 7);

        assert!(matches!(spans[0], Span::ASCII { .. })); // A
        if let Span::ASCII { range } = &spans[0] {
            assert_eq!(range, &(0..1));
            assert_eq!(&input.as_bytes()[range.clone()], b"A");
        }

        assert!(matches!(
            spans[1],
            Span::Control {
                value: AnsiControlCode::BEL,
                ..
            }
        )); // Bell
        if let Span::Control { range, .. } = &spans[1] {
            assert_eq!(range, &(1..2));
            assert_eq!(&input.as_bytes()[range.clone()], b"\x07");
        }

        assert!(matches!(spans[2], Span::ASCII { .. })); // B
        if let Span::ASCII { range } = &spans[2] {
            assert_eq!(range, &(2..3));
            assert_eq!(&input.as_bytes()[range.clone()], b"B");
        }

        assert!(matches!(
            spans[3],
            Span::Control {
                value: AnsiControlCode::BS,
                ..
            }
        )); // Backspace
        if let Span::Control { range, .. } = &spans[3] {
            assert_eq!(range, &(3..4));
            assert_eq!(&input.as_bytes()[range.clone()], b"\x08");
        }

        assert!(matches!(spans[4], Span::ASCII { .. })); // C
        if let Span::ASCII { range } = &spans[4] {
            assert_eq!(range, &(4..5));
            assert_eq!(&input.as_bytes()[range.clone()], b"C");
        }

        assert!(matches!(
            spans[5],
            Span::Control {
                value: AnsiControlCode::CR,
                ..
            }
        )); // Carriage Return
        if let Span::Control { range, .. } = &spans[5] {
            assert_eq!(range, &(5..6));
            assert_eq!(&input.as_bytes()[range.clone()], b"\x0D");
        }

        assert!(matches!(spans[6], Span::ASCII { .. })); // D
        if let Span::ASCII { range } = &spans[6] {
            assert_eq!(range, &(6..7));
            assert_eq!(&input.as_bytes()[range.clone()], b"D");
        }
    }

    #[test]
    fn test_osc() {
        let input = "\x1b]0;Title\x07";
        let spans = SpannedString::parse(input);
        assert_eq!(spans.len(), 10);
        assert_eq!(spans.count(), 1);
        assert!(matches!(spans[0], Span::OSC { .. }));
        if let Span::OSC { range } = &spans[0] {
            assert_eq!(range, &(0..10));
            assert_eq!(&input.as_bytes()[range.clone()], b"\x1b]0;Title\x07");
        }
    }

    #[test]
    fn test_mixed() {
        let input = "\x1b[1;31mBold Red\x1b[0m Normal";
        let spans = SpannedString::parse(input);
        assert_eq!(spans.len(), 26);
        assert_eq!(spans.count(), 4); // CSI, "Bold Red", CSI, " Normal"

        assert!(matches!(spans[0], Span::CSI { .. }));
        if let Span::CSI { range, .. } = &spans[0] {
            assert_eq!(range, &(0..7)); // "\x1b[1;31m"
            assert_eq!(&input.as_bytes()[range.clone()], b"\x1b[1;31m");
        }

        assert!(matches!(spans[1], Span::ASCII { .. }));
        if let Span::ASCII { range } = &spans[1] {
            assert_eq!(range, &(7..15)); // "Bold Red"
            assert_eq!(&input.as_bytes()[range.clone()], b"Bold Red");
        }

        assert!(matches!(spans[2], Span::CSI { .. }));
        if let Span::CSI { range, .. } = &spans[2] {
            assert_eq!(range, &(15..19)); // "\x1b[0m"
            assert_eq!(&input.as_bytes()[range.clone()], b"\x1b[0m");
        }

        assert!(matches!(spans[3], Span::ASCII { .. }));
        if let Span::ASCII { range } = &spans[3] {
            assert_eq!(range, &(19..26)); // " Normal"
            assert_eq!(&input.as_bytes()[range.clone()], b" Normal");
        }
    }

    #[test]
    fn test_all_c0_controls() {
        // Test a few key C0 control codes
        assert_eq!(AnsiControlCode::from_byte(0x00), Some(AnsiControlCode::NUL));
        assert_eq!(AnsiControlCode::from_byte(0x09), Some(AnsiControlCode::HT));
        assert_eq!(AnsiControlCode::from_byte(0x0A), Some(AnsiControlCode::LF));
        assert_eq!(AnsiControlCode::from_byte(0x0D), Some(AnsiControlCode::CR));
        assert_eq!(AnsiControlCode::from_byte(0x7F), Some(AnsiControlCode::DEL));
    }

    #[test]
    fn test_control_code_enum() {
        let input = "\x00\x01\x1F\x7F";
        let spans = SpannedString::parse(input);
        assert_eq!(spans.len(), 4); // Each control code is separate (different codes)

        assert!(matches!(
            spans[0],
            Span::Control {
                value: AnsiControlCode::NUL,
                ..
            }
        ));
        if let Span::Control { range, .. } = &spans[0] {
            assert_eq!(range, &(0..1));
            assert_eq!(&input.as_bytes()[range.clone()], b"\x00");
        }

        assert!(matches!(
            spans[1],
            Span::Control {
                value: AnsiControlCode::SOH,
                ..
            }
        ));
        if let Span::Control { range, .. } = &spans[1] {
            assert_eq!(range, &(1..2));
            assert_eq!(&input.as_bytes()[range.clone()], b"\x01");
        }

        assert!(matches!(
            spans[2],
            Span::Control {
                value: AnsiControlCode::US,
                ..
            }
        ));
        if let Span::Control { range, .. } = &spans[2] {
            assert_eq!(range, &(2..3));
            assert_eq!(&input.as_bytes()[range.clone()], b"\x1F");
        }

        assert!(matches!(
            spans[3],
            Span::Control {
                value: AnsiControlCode::DEL,
                ..
            }
        ));
        if let Span::Control { range, .. } = &spans[3] {
            assert_eq!(range, &(3..4));
            assert_eq!(&input.as_bytes()[range.clone()], b"\x7F");
        }
    }

    #[test]
    fn test_consecutive_same_control_codes() {
        let input = "\n\n\n";
        // Test that consecutive identical control codes are merged
        let spans = SpannedString::parse(input);
        assert_eq!(spans.len(), 3);
        assert_eq!(spans.count(), 1);
        assert!(matches!(
            spans[0],
            Span::Control {
                value: AnsiControlCode::LF,
                ..
            }
        ));
        if let Span::Control { range, .. } = &spans[0] {
            assert_eq!(range, &(0..3));
            assert_eq!(&input.as_bytes()[range.clone()], b"\n\n\n");
        }
    }

    #[test]
    fn test_consecutive_unicode() {
        let input = "世界你好";
        // Test that consecutive Unicode characters are merged
        let spans = SpannedString::parse(input);
        assert_eq!(spans.len(), 12); // 4 3 byte characters
        assert_eq!(spans.count(), 1); // One Unicode Segment
        assert!(matches!(spans[0], Span::Unicode { .. }));
        if let Span::Unicode { range } = &spans[0] {
            assert_eq!(
                &input.as_bytes()[range.clone()],
                b"\xE4\xB8\x96\xE7\x95\x8C\xE4\xBD\xA0\xE5\xA5\xBD"
            );
        }
    }

    #[test]
    fn test_ascii_unicode_merge() {
        // Test that ASCII next to Unicode merges into Unicode
        let input = "Hello世界";
        let spans = SpannedString::parse(input);
        assert_eq!(spans.count(), 1); // One merged Unicode span
        assert!(matches!(spans[0], Span::Unicode { .. }));
        if let Span::Unicode { range } = &spans[0] {
            assert_eq!(range, &(0..11)); // "Hello" (5 bytes) + "世界" (6 bytes)
        }
    }

    #[test]
    fn test_unicode_ascii_merge() {
        // Test that Unicode followed by ASCII merges into Unicode
        let input = "世界Hello";
        let spans = SpannedString::parse(input);
        assert_eq!(spans.count(), 1); // One merged Unicode span
        assert!(matches!(spans[0], Span::Unicode { .. }));
        if let Span::Unicode { range } = &spans[0] {
            assert_eq!(range, &(0..11)); // "世界" (6 bytes) + "Hello" (5 bytes)
        }
    }

    #[test]
    fn test_multiple_ascii_unicode_merges() {
        // Test that multiple ASCII and Unicode segments merge correctly
        let input = "Hello世界World你好";
        let spans = SpannedString::parse(input);
        assert_eq!(spans.count(), 1); // All merged into one Unicode span
        assert!(matches!(spans[0], Span::Unicode { .. }));
        if let Span::Unicode { range } = &spans[0] {
            assert_eq!(range, &(0..22));
        }
    }

    #[test]
    fn test_ascii_unicode_separated_by_control() {
        // Test that control codes break the merge
        let input = "Hello\n世界";
        let spans = SpannedString::parse(input);
        assert_eq!(spans.count(), 3); // ASCII, Control, Unicode

        assert!(matches!(spans[0], Span::ASCII { .. }));
        if let Span::ASCII { range } = &spans[0] {
            assert_eq!(range, &(0..5));
        }

        assert!(matches!(
            spans[1],
            Span::Control {
                value: AnsiControlCode::LF,
                ..
            }
        ));
        if let Span::Control { range, .. } = &spans[1] {
            assert_eq!(range, &(5..6));
        }

        assert!(matches!(spans[2], Span::Unicode { .. }));
        if let Span::Unicode { range } = &spans[2] {
            assert_eq!(range, &(6..12));
        }
    }
}
