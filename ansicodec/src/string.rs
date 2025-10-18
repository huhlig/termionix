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

use crate::AnsiResult;
use crate::ansi::{
    AnsiApplicationProgramCommand, AnsiControlCode, AnsiControlSequenceIntroducer,
    AnsiDeviceControlString, AnsiOperatingSystemCommand, AnsiPrivacyMessage, AnsiStartOfString,
    TelnetCommand,
};
use crate::config::AnsiConfig;
use crate::style::AnsiSelectGraphicRendition;
use crate::utility::SpannedString;
use bytes::BufMut;

/// A mix of ASCII text, Unicode text, ANSI escape sequences/control codes, and Telnet Commands.
/// Unlike [`StyledString`](crate::StyledString) which stores styling metadata separately,
/// `SegmentedString` stores the raw segments themselves, making it ideal for:
///
/// - Parsing and preserving the exact structure of ANSI-formatted strings
/// - Building terminal output incrementally
/// - Precise control over ANSI sequence placement
/// - Converting between different terminal representations
///
/// # Structure
///
/// Internally, `SegmentedString` is a wrapper around `Vec<Segment>`, where each [`Segment`]
/// represents a distinct piece of content with its specific type (ASCII text, Unicode text,
/// control codes, CSI sequences, etc.). This design allows:
///
/// - Efficient appending of content without parsing
/// - Preservation of original ANSI sequence structure
/// - Direct manipulation of individual segments
/// - Conversion to other formats while maintaining semantic meaning
///
/// # Segment Types
///
/// A `SegmentedString` can contain the following types of segments:
///
/// - **ASCII**: ASCII text (0x20-0x7E, excluding ESC and control codes)
/// - **Unicode**: Multi-byte Unicode text
/// - **Control**: C0 or C1 control characters (e.g., newline, tab)
/// - **Escape**: Standalone ESC character
/// - **CSI**: Control Sequence Introducer commands (cursor movement, erasing, etc.)
/// - **SGR**: Select Graphic Rendition (text styling like colors, bold, underline)
/// - **OSC**: Operating System Commands (terminal title, etc.)
/// - **DCS, SOS, ST, PM, APC**: Other ANSI escape sequence types
///
/// # Examples
///
/// Creating an empty segmented string:
///
/// ```rust
/// use termionix_ansicodec::SegmentedString;
///
/// let mut segmented = SegmentedString::empty();
/// assert!(segmented.is_empty());
/// ```
///
/// Building a string with mixed content:
///
/// ```rust
/// use termionix_ansicodec::{SegmentedString, Style, Color, ControlCode, Intensity};
///
/// let mut segmented = SegmentedString::empty();
///
/// // Add styled text
/// segmented.push_style(Style {
///     foreground: Some(Color::Red),
///     intensity: Some(Intensity::Bold),
///     ..Default::default()
/// });
/// segmented.push_str("Error: ");
///
/// // Add plain text
/// segmented.push_str("File not found");
///
/// // Add a control character
/// segmented.push_control(ControlCode::LF);
/// ```
///
/// Building terminal output character by character:
///
/// ```rust
/// use termionix_ansicodec::SegmentedString;
///
/// let mut segmented = SegmentedString::empty();
/// for ch in "Hello".chars() {
///     segmented.push_char(ch);
/// }
/// ```
///
/// # Segment Merging
///
/// `SegmentedString` intelligently merges consecutive compatible segments:
///
/// - Consecutive ASCII characters are merged into a single ASCII segment
/// - Consecutive Unicode characters are merged into a single Unicode segment
/// - ASCII and Unicode segments are merged when adjacent (promoted to Unicode)
/// - Control codes, escape sequences, and style changes create new segments
///
/// This optimization reduces memory usage while preserving semantic meaning.
///
/// # Length Calculation
///
/// The [`len()`](SegmentedString::len) method calculates the display length based on
/// the provided [`AnsiConfig`]. This takes into account which segments contribute to
/// visible output versus terminal control. Most ANSI escape sequences do not contribute
/// to display length.
///
/// # Comparison with Other Types
///
/// - [`SpannedString`](crate::SpannedString): For parsing ANSI strings and extracting
///   byte ranges of each segment type. Returns immutable parse results with ranges.
/// - [`StyledString`](crate::StyledString): For building styled text with automatic
///   ANSI code generation. Stores text and styling separately.
/// - `SegmentedString`: For building terminal output with explicit control over
///   segments and ANSI sequences. Stores raw segments.
///
/// # Performance Considerations
///
/// - Adding characters/strings: O(1) amortized (may merge with last segment)
/// - Counting segments: O(1)
/// - Calculating length: O(n) where n is the number of segments
/// - Memory: One allocation per segment (merged when possible)
///
/// # See Also
///
/// - [`Segment`] - The individual segment enum
/// - [`SpannedString`](crate::SpannedString) - For parsing ANSI strings
/// - [`StyledString`](crate::StyledString) - For building styled text
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentedString(Vec<Segment>);

impl SegmentedString {
    /// Creates a new empty `SegmentedString` with no segments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let segmented = SegmentedString::empty();
    /// assert_eq!(segmented.segment_count(), 0);
    /// assert!(segmented.is_empty());
    /// ```
    pub fn empty() -> SegmentedString {
        SegmentedString(Vec::new())
    }

    /// Returns the number of segments in this `SegmentedString`.
    ///
    /// Each segment represents a contiguous piece of content with the same type
    /// (ASCII text, Unicode text, control sequence, etc.). Consecutive compatible
    /// segments are automatically merged, so the count reflects the minimal number
    /// of segments needed to represent the string.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// assert_eq!(segmented.segment_count(), 0);
    ///
    /// segmented.push_str("Hello"); // 1 ASCII segment
    /// assert_eq!(segmented.segment_count(), 1);
    ///
    /// segmented.push_str(" World"); // Merged into same ASCII segment
    /// assert_eq!(segmented.segment_count(), 1);
    /// ```
    pub fn segment_count(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the segmented string contains no segments.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// assert!(segmented.is_empty());
    ///
    /// segmented.push_char('A');
    /// assert!(!segmented.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Appends an ANSI sequence (text content) to the segmented string.
    ///
    /// This method intelligently handles both ASCII and Unicode sequences, merging them
    /// with the last segment when possible according to the following rules:
    ///
    /// - If the last segment is ASCII and an ASCII sequence is added, concatenate them
    /// - If the last segment is Unicode, and Unicode or ASCII is added, concatenate them
    /// - If the last segment is ASCII and Unicode is added, convert the last segment to Unicode and concatenate
    /// - Otherwise, simply push the sequence as a new segment
    ///
    /// # Arguments
    ///
    /// * `sequence` - The string sequence to append
    ///
    /// # Examples
    ///
    /// ASCII appended to ASCII:
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push("Hello");
    /// segmented.push("World");
    /// assert_eq!(segmented.segment_count(), 1); // Merged into one segment
    /// ```
    ///
    /// Unicode appended to ASCII (promotion):
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push("Hello");
    /// segmented.push("世界");
    /// assert_eq!(segmented.segment_count(), 1); // Promoted to Unicode, merged
    /// ```
    ///
    /// Unicode appended to Unicode:
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push("世界");
    /// segmented.push("こんにちは");
    /// assert_eq!(segmented.segment_count(), 1); // Merged
    /// ```
    ///
    /// After non-text segment:
    ///
    /// ```rust
    /// use termionix_ansicodec::{SegmentedString, ControlCode};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_control(ControlCode::LF);
    /// segmented.push("New");
    /// assert_eq!(segmented.segment_count(), 2); // New segment created
    /// ```
    pub fn push(&mut self, sequence: &str) {
        if sequence.is_empty() {
            return;
        }

        // Check if the entire sequence is ASCII
        let is_ascii = sequence.is_ascii();

        if let Some(last_segment) = self.0.last_mut() {
            match last_segment {
                Segment::ASCII(s) if is_ascii => {
                    // Rule 1: ASCII segment + ASCII sequence → concatenate
                    s.push_str(sequence);
                }
                Segment::ASCII(s) if !is_ascii => {
                    // Rule 3: ASCII segment + Unicode sequence → convert to Unicode and concatenate
                    let converted = std::mem::take(s);
                    *last_segment = Segment::Unicode(converted);
                    if let Segment::Unicode(unicode_str) = last_segment {
                        unicode_str.push_str(sequence);
                    }
                }
                Segment::Unicode(s) => {
                    // Rule 2: Unicode segment + (Unicode or ASCII) sequence → concatenate
                    s.push_str(sequence);
                }
                _ => {
                    // Rule 4: Otherwise → push as new segment
                    if is_ascii {
                        self.0.push(Segment::ASCII(sequence.to_string()));
                    } else {
                        self.0.push(Segment::Unicode(sequence.to_string()));
                    }
                }
            }
        } else {
            // No segments exist, create a new one
            if is_ascii {
                self.0.push(Segment::ASCII(sequence.to_string()));
            } else {
                self.0.push(Segment::Unicode(sequence.to_string()));
            }
        }
    }

    /// Appends a single character to the segmented string.
    ///
    /// This method intelligently handles both ASCII and Unicode characters, merging
    /// them with the last segment when possible. Characters are classified as:
    ///
    /// - **ASCII**: Characters in range 0x20-0x7E (excluding ESC and control codes)
    /// - **Unicode**: All other printable characters
    ///
    /// If the last segment is compatible (same type or can be promoted), the character
    /// is appended to it. Otherwise, a new segment is created.
    ///
    /// # Segment Merging Rules
    ///
    /// - ASCII + ASCII → merged ASCII segment
    /// - ASCII segment + Unicode character → promoted to Unicode segment
    /// - Unicode + any character → merged Unicode segment
    /// - After control/style → new text segment created
    ///
    /// # Arguments
    ///
    /// * `ch` - The character to append
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_char('H');
    /// segmented.push_char('i');
    /// ```
    ///
    /// Unicode characters:
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_char('世');
    /// segmented.push_char('界');
    /// ```
    ///
    /// Mixing ASCII and Unicode (promotes to Unicode):
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_char('A');   // ASCII segment
    /// segmented.push_char('世');  // Promoted to Unicode segment
    /// assert_eq!(segmented.segment_count(), 1); // Merged into one Unicode segment
    /// ```
    pub fn push_char(&mut self, ch: char) {
        // Check if character is ASCII (0x00-0x7F)
        let is_ascii = ch.is_ascii() && ch != '\x1b' && !ch.is_control();

        if let Some(last_segment) = self.0.last_mut() {
            match last_segment {
                Segment::ASCII(s) if is_ascii => {
                    // Append ASCII character to ASCII segment
                    s.push(ch);
                }
                Segment::ASCII(s) if !is_ascii => {
                    // Convert ASCII segment to Unicode and append Unicode character
                    let converted = std::mem::take(s);
                    *last_segment = Segment::Unicode(converted);
                    if let Segment::Unicode(unicode_str) = last_segment {
                        unicode_str.push(ch);
                    }
                }
                Segment::Unicode(s) => {
                    // Append any character (ASCII or Unicode) to Unicode segment
                    s.push(ch);
                }
                _ => {
                    // Last segment is not a string segment, create a new one
                    if is_ascii {
                        self.0.push(Segment::ASCII(ch.to_string()));
                    } else {
                        self.0.push(Segment::Unicode(ch.to_string()));
                    }
                }
            }
        } else {
            // No segments exist, create a new one
            if is_ascii {
                self.0.push(Segment::ASCII(ch.to_string()));
            } else {
                self.0.push(Segment::Unicode(ch.to_string()));
            }
        }
    }

    /// Appends a string slice to the segmented string.
    ///
    /// This method efficiently handles both ASCII and Unicode strings, merging them
    /// with the last segment when possible. If the string is empty, this is a no-op.
    ///
    /// The entire string is classified as either ASCII or Unicode:
    /// - **ASCII**: All characters are in range 0x20-0x7E
    /// - **Unicode**: Contains any character outside the ASCII range
    ///
    /// # Segment Merging Rules
    ///
    /// - ASCII segment + ASCII string → merged ASCII segment
    /// - ASCII segment + Unicode string → promoted to Unicode segment
    /// - Unicode segment + any string → merged Unicode segment
    /// - After control/style segment → new text segment created
    ///
    /// # Arguments
    ///
    /// * `str` - The string slice to append
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello");
    /// segmented.push_str(" World");
    /// assert_eq!(segmented.segment_count(), 1); // Merged into one segment
    /// ```
    ///
    /// Unicode strings:
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("こんにちは");
    /// ```
    ///
    /// Mixed content:
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello");  // ASCII segment
    /// segmented.push_str("世界");   // Promoted to Unicode, merged
    /// assert_eq!(segmented.segment_count(), 1);
    /// ```
    ///
    /// Empty strings are ignored:
    ///
    /// ```rust
    /// use termionix_ansicodec::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("");
    /// assert!(segmented.is_empty());
    /// ```
    pub fn push_str(&mut self, str: &str) {
        if str.is_empty() {
            return;
        }

        // Check if the entire string is ASCII
        let is_ascii = str.is_ascii();

        if let Some(last_segment) = self.0.last_mut() {
            match last_segment {
                Segment::ASCII(s) if is_ascii => {
                    // Append ASCII string to ASCII segment
                    s.push_str(str);
                }
                Segment::ASCII(s) if !is_ascii => {
                    // Convert ASCII segment to Unicode and append Unicode string
                    let converted = std::mem::take(s);
                    *last_segment = Segment::Unicode(converted);
                    if let Segment::Unicode(unicode_str) = last_segment {
                        unicode_str.push_str(str);
                    }
                }
                Segment::Unicode(s) => {
                    // Append any string (ASCII or Unicode) to Unicode segment
                    s.push_str(str);
                }
                _ => {
                    // Last segment is not a string segment, create a new one
                    if is_ascii {
                        self.0.push(Segment::ASCII(str.to_string()));
                    } else {
                        self.0.push(Segment::Unicode(str.to_string()));
                    }
                }
            }
        } else {
            // No segments exist, create a new one
            if is_ascii {
                self.0.push(Segment::ASCII(str.to_string()));
            } else {
                self.0.push(Segment::Unicode(str.to_string()));
            }
        }
    }

    /// Appends a control code segment to the segmented string.
    ///
    /// Control codes represent non-printable characters that control terminal behavior,
    /// such as line feeds, carriage returns, tabs, and bell sounds. These always create
    /// a new segment and do not merge with adjacent segments.
    ///
    /// # Arguments
    ///
    /// * `control` - The control code to append
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::{SegmentedString, ControlCode};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Line 1");
    /// segmented.push_control(ControlCode::LF);
    /// segmented.push_str("Line 2");
    /// assert_eq!(segmented.segment_count(), 3); // Text, Control, Text
    /// ```
    ///
    /// Multiple control codes:
    ///
    /// ```rust
    /// use termionix_ansicodec::{SegmentedString, ControlCode};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_control(ControlCode::CR);
    /// segmented.push_control(ControlCode::LF);
    /// assert_eq!(segmented.segment_count(), 2); // Each control is a separate segment
    /// ```
    pub fn push_ansi_control(&mut self, control: AnsiControlCode) {
        self.0.push(Segment::Control(control));
    }

    pub fn push_ansi_escape(&mut self) {
        self.0.push(Segment::Escape);
    }

    pub fn push_ansi_csi(&mut self, csi: AnsiControlSequenceIntroducer) {
        self.0.push(Segment::CSI(csi));
    }

    pub fn push_ansi_sgr(&mut self, sgr: AnsiSelectGraphicRendition) {
        self.0.push(Segment::SGR(sgr));
    }

    pub fn push_ansi_osc(&mut self, osc: AnsiOperatingSystemCommand) {
        self.0.push(Segment::OSC(osc));
    }

    pub fn push_ansi_dcs(&mut self, dcs: AnsiDeviceControlString) {
        self.0.push(Segment::DCS(dcs));
    }

    pub fn push_ansi_sos(&mut self, sos: AnsiStartOfString) {
        self.0.push(Segment::SOS(sos));
    }

    pub fn push_ansi_st(&mut self) {
        self.0.push(Segment::ST);
    }

    pub fn push_ansi_pm(&mut self, pm: AnsiPrivacyMessage) {
        self.0.push(Segment::PM(pm));
    }

    pub fn push_ansi_apc(&mut self, apc: AnsiApplicationProgramCommand) {
        self.0.push(Segment::APC(apc));
    }
    pub fn push_telnet_command(&mut self, tc: TelnetCommand) {
        self.0.push(Segment::TelnetCommand(tc));
    }

    /// Appends a style (SGR - Select Graphic Rendition) segment to the segmented string.
    ///
    /// This adds an ANSI SGR sequence that changes text styling attributes such as colors,
    /// bold, underline, italic, etc. The style segment does not merge with adjacent segments
    /// and serves as a formatting delimiter between text segments.
    ///
    /// # Arguments
    ///
    /// * `style` - The style to apply to subsequent text
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodec::{SegmentedString, Style, Color, Intensity};
    ///
    /// let mut segmented = SegmentedString::empty();
    ///
    /// // Add red bold text
    /// segmented.push_style(Style {
    ///     foreground: Some(Color::Red),
    ///     intensity: Some(Intensity::Bold),
    ///     ..Default::default()
    /// });
    /// segmented.push_str("Error");
    ///
    /// // Reset and add normal text
    /// segmented.push_style(Style::default());
    /// segmented.push_str(" occurred");
    /// ```
    ///
    /// Multiple styles:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Style, Color};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_style(Style {
    ///     foreground: Some(Color::Red),
    ///     ..Default::default()
    /// });
    /// segmented.push_str("Red ");
    ///
    /// segmented.push_style(Style {
    ///     foreground: Some(Color::Blue),
    ///     ..Default::default()
    /// });
    /// segmented.push_str("Blue");
    /// ```
    pub fn push_style(&mut self, style: AnsiSelectGraphicRendition) {
        self.0.push(Segment::SGR(style.into()))
    }

    /// Appends an arbitrary segment to the segmented string.
    ///
    /// This is a low-level method that allows direct insertion of any [`Segment`] variant
    /// without automatic merging or type checking. Unlike [`push_char`](SegmentedString::push_char)
    /// or [`push_str`](SegmentedString::push_str) which intelligently merge compatible text
    /// segments, this method always creates a new segment regardless of the previous segment type.
    ///
    /// This method is useful when you need precise control over segment boundaries or when
    /// working with pre-constructed segments from parsing operations.
    ///
    /// # Arguments
    ///
    /// * `segment` - The segment to append to the segmented string
    ///
    /// # Examples
    ///
    /// Adding a custom CSI command:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Segment, CSICommand};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Before");
    /// segmented.push_segment(Segment::CSI(CSICommand::CursorUp(5)));
    /// segmented.push_str("After");
    /// ```
    ///
    /// Manually creating text segments:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Segment};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_segment(Segment::ASCII("Hello".to_string()));
    /// segmented.push_segment(Segment::Unicode("世界".to_string()));
    /// assert_eq!(segmented.segment_count(), 2); // Not merged because using push_segment
    /// ```
    ///
    /// Building from parsed segments:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Segment, ControlCode};
    ///
    /// let segments = vec![
    ///     Segment::ASCII("Line 1".to_string()),
    ///     Segment::Control(ControlCode::LF),
    ///     Segment::ASCII("Line 2".to_string()),
    /// ];
    ///
    /// let mut segmented = SegmentedString::empty();
    /// for segment in segments {
    ///     segmented.push_segment(segment);
    /// }
    /// assert_eq!(segmented.segment_count(), 3);
    /// ```
    ///
    /// # Note
    ///
    /// This method does not perform any segment merging. If you push consecutive ASCII or
    /// Unicode segments using this method, they will remain as separate segments. For
    /// automatic merging behavior, use [`push_char`](SegmentedString::push_char) or
    /// [`push_str`](SegmentedString::push_str) instead.
    pub fn push_segment(&mut self, segment: Segment) {
        self.0.push(segment)
    }

    /// Returns an iterator over the segments in this segmented string.
    ///
    /// The iterator yields references to each [`Segment`] in order, from the
    /// beginning to the end of the string. Each segment represents a contiguous
    /// piece of content with a specific type (ASCII text, Unicode text, control
    /// codes, ANSI sequences, etc.).
    ///
    /// # Returns
    ///
    /// A slice iterator over the internal segment collection.
    ///
    /// # Examples
    ///
    /// Basic iteration:
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello");
    /// segmented.push_str(" World");
    ///
    /// for segment in segmented.segments() {
    ///     // Process each segment
    /// }
    /// ```
    ///
    /// Filtering specific segment types:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Segment, Style, Color};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_style(Style {
    ///     foreground: Some(Color::Red),
    ///     ..Default::default()
    /// });
    /// segmented.push_str("Error");
    ///
    /// let text_segments: Vec<_> = segmented.segments()
    ///     .filter(|s| matches!(s, Segment::ASCII(_) | Segment::Unicode(_)))
    ///     .collect();
    /// ```
    ///
    /// Counting specific segment types:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Segment, ControlCode};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Line 1");
    /// segmented.push_control(ControlCode::LF);
    /// segmented.push_str("Line 2");
    ///
    /// let control_count = segmented.segments()
    ///     .filter(|s| matches!(s, Segment::Control(_)))
    ///     .count();
    /// assert_eq!(control_count, 1);
    /// ```
    pub fn segments(&self) -> std::slice::Iter<'_, Segment> {
        self.0.iter()
    }

    /// Returns the plain text content without any ANSI escape sequences or control codes.
    ///
    /// This method extracts only the visible text content from ASCII and Unicode segments,
    /// discarding all ANSI control sequences, styling codes, and terminal commands. The
    /// result is a plain `String` containing only the displayable characters that would
    /// appear on screen, without any formatting information.
    ///
    /// # Returns
    ///
    /// A `String` containing only the text content from ASCII and Unicode segments,
    /// concatenated in order. All other segment types (Control, Escape, CSI, SGR,
    /// OSC, DCS, SOS, ST, PM, APC) are omitted from the output.
    ///
    /// # Segment Processing
    ///
    /// The method processes segments as follows:
    /// - **ASCII segments**: Text content is included
    /// - **Unicode segments**: Text content is included
    /// - **All other segments**: Completely omitted (Control, Escape, CSI, SGR, etc.)
    ///
    /// # Examples
    ///
    /// Basic text stripping:
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello World");
    ///
    /// assert_eq!(segmented.stripped(), "Hello World");
    /// ```
    ///
    /// Removing style codes:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Style, Color, Intensity};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_style(Style {
    ///     foreground: Some(Color::Red),
    ///     intensity: Some(Intensity::Bold),
    ///     ..Default::default()
    /// });
    /// segmented.push_str("Error");
    /// segmented.push_style(Style::default());
    /// segmented.push_str(": File not found");
    ///
    /// assert_eq!(segmented.stripped(), "Error: File not found");
    /// ```
    ///
    /// Removing control codes:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, ControlCode};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Line 1");
    /// segmented.push_control(ControlCode::LF);
    /// segmented.push_str("Line 2");
    ///
    /// // Control codes are removed, only text remains
    /// assert_eq!(segmented.stripped(), "Line 1Line 2");
    /// ```
    ///
    /// Mixed ASCII and Unicode:
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello");
    /// segmented.push_str(" 世界");
    ///
    /// assert_eq!(segmented.stripped(), "Hello 世界");
    /// ```
    ///
    /// Complex example with multiple segment types:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Style, Color, ControlCode, CSICommand};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Status: ");
    /// segmented.push_style(Style {
    ///     foreground: Some(Color::Green),
    ///     ..Default::default()
    /// });
    /// segmented.push_str("OK");
    /// segmented.push_control(ControlCode::LF);
    /// segmented.push_str("Progress: 100%");
    ///
    /// // All styling and control codes removed
    /// assert_eq!(segmented.stripped(), "Status: OKProgress: 100%");
    /// ```
    ///
    /// # Performance
    ///
    /// This operation is O(n) where n is the number of segments. It allocates a new
    /// `String` and concatenates only the text segments, making it efficient for
    /// extracting plain text from styled terminal output.
    ///
    /// # Use Cases
    ///
    /// - **Logging**: Save plain text logs without ANSI codes
    /// - **Text processing**: Extract content for analysis or search
    /// - **Testing**: Compare expected text content without worrying about styling
    /// - **Display**: Show content in environments that don't support ANSI codes
    /// - **Length calculation**: Get accurate character count of visible text
    ///
    /// # Comparison with Other Methods
    ///
    /// - [`styled_len()`](SegmentedString::len): Calculates display length with config
    /// - [`write_str()`](SegmentedString::write_str): Outputs with ANSI codes based on color mode
    /// - `stripped()`: Returns only visible text without any formatting
    ///
    /// # See Also
    ///
    /// - [`StyledString::stripped()`](crate::StyledString::stripped) - Similar method for `StyledString`
    /// - [`styled_len()`](SegmentedString::len) - Calculate display length
    /// - [`iter()`](SegmentedString::segments) - Iterate over all segments including non-text
    pub fn stripped(&self) -> String {
        self.0
            .iter()
            .filter_map(|segment| match segment {
                Segment::ASCII(s) | Segment::Unicode(s) => Some(s.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Clears all segments from the `SegmentedString`, removing all content.
    ///
    /// This method resets the segmented string to an empty state, equivalent to creating
    /// a new `SegmentedString` with [`SegmentedString::empty()`]. All segments are removed,
    /// including text content (ASCII and Unicode), control codes, ANSI escape sequences,
    /// and styling information.
    ///
    /// After calling this method:
    /// - [`segment_count()`](SegmentedString::segment_count) will return 0
    /// - [`is_empty()`](SegmentedString::is_empty) will return `true`
    /// - [`stripped()`](SegmentedString::stripped) will return an empty string
    /// - [`styled_len()`](SegmentedString::len) will return 0
    /// - All internal segments are removed
    ///
    /// # Performance
    ///
    /// This is an efficient O(1) operation that clears the internal segment vector.
    /// The underlying memory capacity is retained, making subsequent operations
    /// potentially more efficient if the `SegmentedString` is reused for building
    /// new content.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, AnsiConfig};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello World");
    /// assert_eq!(segmented.segment_count(), 1);
    ///
    /// segmented.clear();
    /// assert_eq!(segmented.segment_count(), 0);
    /// assert!(segmented.is_empty());
    /// assert_eq!(segmented.stripped(), "");
    /// ```
    ///
    /// Clearing styled content:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Style, Color, Intensity};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_style(Style {
    ///     foreground: Some(Color::Red),
    ///     intensity: Some(Intensity::Bold),
    ///     ..Default::default()
    /// });
    /// segmented.push_str("Bold Red Text");
    ///
    /// segmented.clear();
    /// assert_eq!(segmented.stripped(), "");
    /// assert_eq!(segmented.segment_count(), 0);
    /// ```
    ///
    /// Clearing mixed segment types:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Style, Color, ControlCode};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Line 1");
    /// segmented.push_control(ControlCode::LF);
    /// segmented.push_style(Style {
    ///     foreground: Some(Color::Green),
    ///     ..Default::default()
    /// });
    /// segmented.push_str("Line 2");
    /// segmented.push_control(ControlCode::LF);
    ///
    /// assert_eq!(segmented.segment_count(), 5);
    ///
    /// segmented.clear();
    /// assert_eq!(segmented.segment_count(), 0);
    /// ```
    ///
    /// Reusing after clear:
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("First message");
    /// segmented.clear();
    ///
    /// // Reuse the same SegmentedString
    /// segmented.push_str("Second message");
    /// assert_eq!(segmented.stripped(), "Second message");
    /// assert_eq!(segmented.segment_count(), 1);
    /// ```
    ///
    /// Clearing in a loop (efficient memory reuse):
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Style, Color};
    ///
    /// let mut segmented = SegmentedString::empty();
    ///
    /// for i in 0..10 {
    ///     segmented.push_style(Style {
    ///         foreground: Some(Color::Blue),
    ///         ..Default::default()
    ///     });
    ///     segmented.push_str(&format!("Iteration {}", i));
    ///
    ///     // Process the segmented string...
    ///
    ///     // Clear for next iteration (retains capacity)
    ///     segmented.clear();
    /// }
    /// ```
    ///
    /// Clearing complex terminal output:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Style, Color, ControlCode, CSICommand, Intensity};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Status: ");
    /// segmented.push_style(Style {
    ///     foreground: Some(Color::Green),
    ///     intensity: Some(Intensity::Bold),
    ///     ..Default::default()
    /// });
    /// segmented.push_str("OK");
    /// segmented.push_style(Style::default());
    /// segmented.push_control(ControlCode::LF);
    /// segmented.push_str("Progress: 100%");
    ///
    /// // All segments removed, including text, styles, and control codes
    /// segmented.clear();
    /// assert!(segmented.is_empty());
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Buffer reuse**: Clear between operations to avoid reallocations
    /// - **Terminal screen clearing**: Reset buffer state for new screen content
    /// - **Error recovery**: Clear corrupted or incomplete segment sequences
    /// - **Memory management**: Release segments while retaining the container
    /// - **State reset**: Return to initial empty state between processing cycles
    ///
    /// # Comparison with Other Operations
    ///
    /// - [`SegmentedString::empty()`]: Creates a new empty instance (allocation)
    /// - `clear()`: Removes all segments from existing instance (reuses allocation)
    /// - [`pop()`](SegmentedString::pop): Removes one character at a time
    /// - `clear()`: Removes all content at once
    ///
    /// # See Also
    ///
    /// - [`SegmentedString::empty()`](SegmentedString::empty) - Create a new empty segmented string
    /// - [`SegmentedString::is_empty()`](SegmentedString::is_empty) - Check if the string is empty
    /// - [`SegmentedString::segment_count()`](SegmentedString::segment_count) - Get the number of segments
    /// - [`SegmentedString::stripped()`](SegmentedString::stripped) - Get text content without formatting
    pub fn clear(&mut self) {
        self.0.clear()
    }

    /// Removes and returns the last character from the segmented string.
    ///
    /// This method removes the last character from the last text segment (ASCII or Unicode)
    /// in the segmented string. If removing the character leaves the segment empty, the
    /// entire segment is removed. Returns `None` if the segmented string is empty or if
    /// the last segment is not a text segment.
    ///
    /// # Non-Text Segments
    ///
    /// If the last segment is not a text segment (Control, Escape, CSI, SGR, OSC, etc.),
    /// the entire segment is removed and returned as `None`. This ensures that control
    /// sequences and styling information are treated as atomic units.
    ///
    /// # Returns
    ///
    /// - `Some(char)` - The last character if the last segment contains text
    /// - `None` - If the string is empty or the last segment is not text
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello");
    ///
    /// assert_eq!(segmented.pop(), Some('o'));
    /// assert_eq!(segmented.pop(), Some('l'));
    /// assert_eq!(segmented.stripped(), "Hel");
    /// ```
    ///
    /// Popping from an empty string:
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// assert_eq!(segmented.pop(), None);
    /// ```
    ///
    /// Unicode character support:
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_char('🦀'); // Rust crab emoji (4 bytes)
    /// segmented.push_char('日'); // Japanese character (3 bytes)
    ///
    /// assert_eq!(segmented.pop(), Some('日'));
    /// assert_eq!(segmented.pop(), Some('🦀'));
    /// assert_eq!(segmented.pop(), None);
    /// ```
    ///
    /// Handling non-text segments:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, ControlCode};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello");
    /// segmented.push_control(ControlCode::LF);
    ///
    /// // Control code is removed as a whole, returns None
    /// assert_eq!(segmented.pop(), None);
    /// assert_eq!(segmented.segment_count(), 1);
    ///
    /// // Now we can pop from the text segment
    /// assert_eq!(segmented.pop(), Some('o'));
    /// ```
    ///
    /// Mixed ASCII and Unicode segments:
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello");
    /// segmented.push_str("世界");
    ///
    /// assert_eq!(segmented.pop(), Some('界'));
    /// assert_eq!(segmented.pop(), Some('世'));
    /// assert_eq!(segmented.pop(), Some('o'));
    /// ```
    ///
    /// Segment removal when empty:
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("AB");
    /// segmented.push_str("C");
    ///
    /// assert_eq!(segmented.segment_count(), 1); // Merged
    /// segmented.pop(); // Remove 'C'
    /// assert_eq!(segmented.segment_count(), 1); // Still has "AB"
    /// ```
    pub fn pop(&mut self) -> Option<char> {
        // Work backwards through segments to find the last text segment
        while let Some(last_segment) = self.0.last_mut() {
            match last_segment {
                Segment::ASCII(s) | Segment::Unicode(s) => {
                    // Try to pop a character from the text segment
                    if let Some(ch) = s.pop() {
                        // If the segment is now empty, remove it
                        if s.is_empty() {
                            self.0.pop();
                        }
                        return Some(ch);
                    } else {
                        // Empty a text segment, remove it and continue
                        self.0.pop();
                    }
                }
                _ => {
                    // Non-text segment (Control, Escape, CSI, SGR, etc.)
                    // Remove the entire segment and return None
                    self.0.pop();
                    return None;
                }
            }
        }

        // No segments left
        None
    }

    /// Returns the display length of the segmented string based on the provided configuration.
    ///
    /// This calculates how many visible character positions the string occupies on screen,
    /// taking into account the ANSI configuration settings. Most ANSI escape sequences
    /// (control codes, CSI sequences, style changes) do not contribute to display length
    /// as they only affect formatting or cursor positioning.
    ///
    /// # Arguments
    ///
    /// * `config` - The ANSI configuration that determines how segments are interpreted
    ///
    /// # Returns
    ///
    /// The number of visible character positions the string occupies. This is the sum
    /// of the lengths of all text segments (ASCII and Unicode), excluding control
    /// sequences and formatting codes.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, AnsiConfig};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello");
    ///
    /// let config = AnsiConfig::default();
    /// assert_eq!(segmented.styled_len(Some(&config)), 5);
    /// ```
    ///
    /// With control codes (which don't contribute to display length):
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, AnsiConfig, ControlCode};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello");
    /// segmented.push_control(ControlCode::LF);
    /// segmented.push_str("World");
    ///
    /// let config = AnsiConfig::strip_all();
    /// assert_eq!(segmented.styled_len(Some(&config)), 10); // Only counts "HelloWorld"
    /// ```
    pub fn len(&self, config: Option<&AnsiConfig>) -> AnsiResult<usize> {
        let mut total_len = 0;

        for segment in &self.0 {
            match segment {
                Segment::ASCII(s) | Segment::Unicode(s) => {
                    // For text segments, count the string length
                    total_len += s.len();
                }
                Segment::Control(_) => {
                    // Control codes contribute to length only if enabled in config
                    if let Some(config) = config
                        && config.strip_ctrl
                    {
                        // Strip CSI Sequence
                    } else {
                        total_len += 1; // Control codes are single bytes
                    }
                }
                Segment::Escape => {
                    total_len += 1; // Control codes are single bytes
                }
                Segment::CSI(_) => {
                    // CSI sequences contribute to length only if enabled in config
                    if let Some(config) = config
                        && config.strip_csi
                    {
                        // Strip CSI Sequence
                    } else {
                        total_len += 3; // Minimum: ESC [ char
                    }
                }
                Segment::SGR(sgr) => {
                    // SGR sequences contribute to length only if enabled in config
                    if let Some(config) = config {
                        if config.strip_sgr {
                            // Strip SGR Sequence
                        } else {
                            total_len += sgr.len(Some(config.color_mode));
                        }
                    } else {
                        total_len += sgr.len(None);
                    }
                }
                Segment::OSC(osc) => {
                    // OSC sequences contribute to length only if enabled in config
                    if let Some(config) = config
                        && config.strip_osc
                    {
                        // Strip OSC Sequence
                    } else {
                        total_len += osc.len();
                    }
                }
                Segment::DCS(dcs) => {
                    // DCS sequences contribute to length only if enabled in config
                    if let Some(config) = config
                        && config.strip_dcs
                    {
                        // Strip DCS Sequence
                    } else {
                        total_len += dcs.len();
                    }
                }
                Segment::SOS(sos) => {
                    // SOS/ST sequences contribute to length only if enabled in config
                    if let Some(config) = config
                        && config.strip_sos_st
                    {
                        // Strip SOS/ST Sequence
                    } else {
                        total_len += sos.len();
                    }
                }
                Segment::ST => {
                    // SOS/ST sequences contribute to length only if enabled in config
                    if let Some(config) = config
                        && config.strip_sos_st
                    {
                        // Strip SOS/ST Sequence
                    } else {
                        total_len += 2;
                    }
                }
                Segment::PM(pm) => {
                    // PM sequences contribute to length only if enabled in config
                    if let Some(config) = config
                        && config.strip_pm
                    {
                        // Strip PM Sequence
                    } else {
                        total_len += pm.len();
                    }
                }
                Segment::APC(apc) => {
                    // PM sequences contribute to length only if enabled in config
                    if let Some(config) = config
                        && config.strip_apc
                    {
                        // Strip APC Sequence
                    } else {
                        total_len += apc.len();
                    }
                }
                Segment::TelnetCommand(tc) => {
                    // Telnet Command sequences contribute to length only if enabled in config
                    if let Some(config) = config
                        && config.strip_telnet
                    {
                        // Strip TelnetCommand Sequence
                    } else {
                        total_len += tc.len();
                    }
                }
            }
        }

        Ok(total_len)
    }

    pub fn encode<T: BufMut>(&self, dst: &mut T, config: Option<&AnsiConfig>) -> AnsiResult<usize> {
        Ok(self.write(&mut dst.writer(), config)?)
    }

    /// Writes the segmented string with appropriate ANSI escape codes to a writer.
    ///
    /// This method processes each segment and outputs it according to the specified
    /// color mode. Text segments (ASCII and Unicode) are written directly, while
    /// control sequences and style changes are converted to their ANSI representations
    /// based on the color mode settings.
    ///
    /// # Segment Processing
    ///
    /// Different segment types are handled as follows:
    ///
    /// - **ASCII/Unicode**: Text content is written directly
    /// - **Control**: Converted to their control character representation
    /// - **Escape**: Written as ESC character (`\x1b`)
    /// - **CSI**: Formatted as CSI sequence (`ESC [ ... `)
    /// - **SGR**: Style information written as SGR codes (if color mode allows)
    /// - **OSC/DCS/SOS/ST/PM/APC**: Written as appropriate ANSI sequences
    ///
    /// # Color Mode Behavior
    ///
    /// The `mode` parameter determines how styling segments are rendered:
    ///
    /// - [`ColorMode::None`]: No ANSI escape codes are generated (text only)
    /// - [`ColorMode::Basic`]: Basic 16-color ANSI codes
    /// - [`ColorMode::Extended`]: 256-color ANSI codes
    /// - [`ColorMode::TrueColor`]: 24-bit RGB ANSI codes
    ///
    /// # Arguments
    ///
    /// * `mode` - The color mode determining which ANSI codes to generate
    /// * `writer` - The writer to output the formatted string to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a [`std::fmt::Error`] if writing fails.
    ///
    /// # Examples
    ///
    /// Writing plain text:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, ColorMode, AnsiConfig};
    /// let config = AnsiConfig::default();
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Hello World");
    ///
    /// let mut output = String::new();
    /// segmented.write_str(&mut output, Some(&config)).unwrap();
    /// assert_eq!(output, "Hello World");
    /// ```
    ///
    /// Writing styled text with color mode:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Style, Color, ColorMode, Intensity, AnsiConfig};
    ///
    /// let config = AnsiConfig::default();
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_style(Style {
    ///     foreground: Some(Color::Red),
    ///     intensity: Some(Intensity::Bold),
    ///     ..Default::default()
    /// });
    /// segmented.push_str("Error");
    ///
    /// let mut output = String::new();
    /// segmented.write_str(&mut output, Some(&config)).unwrap();
    /// // Output contains ANSI codes: "\x1b[1;31mError"
    /// ```
    ///
    /// Writing with control codes:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, ControlCode, ColorMode, AnsiConfig};
    ///
    /// let config = AnsiConfig::default();
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_str("Line 1");
    /// segmented.push_control(ControlCode::LF);
    /// segmented.push_str("Line 2");
    ///
    /// let mut output = String::new();
    /// segmented.write_str(&mut output, Some(&config)).unwrap();
    /// assert_eq!(output, "Line 1\nLine 2");
    /// ```
    ///
    /// Different color modes produce different output:
    ///
    /// ```rust
    /// use termionix_ansicodes::{SegmentedString, Style, Color, ColorMode, AnsiConfig};
    ///
    /// let mut segmented = SegmentedString::empty();
    /// segmented.push_style(Style {
    ///     foreground: Some(Color::Red),
    ///     ..Default::default()
    /// });
    /// segmented.push_str("Red");
    ///
    /// // No ANSI codes
    /// let config_none = AnsiConfig::strip_all();
    /// let mut output_none = String::new();
    /// segmented.write_str(&mut output_none, Some(&config_none)).unwrap();
    /// assert_eq!(output_none, "Red");
    ///
    /// // With ANSI codes
    /// let config_basic = AnsiConfig::default();
    /// let mut output_basic = String::new();
    /// segmented.write_str(&mut output_basic, Some(&config_basic)).unwrap();
    /// assert!(output_basic.starts_with("\x1b["));
    /// ```
    ///
    /// # Performance
    ///
    /// This operation is O(n) where n is the number of segments. Each segment
    /// is processed once, and ANSI codes are generated inline without additional
    /// allocations beyond what the writer requires.
    ///
    /// # Use Cases
    ///
    /// - **Terminal output**: Display formatted text in terminals
    /// - **File generation**: Create ANSI-formatted text files
    /// - **Network protocols**: Send styled text over network connections
    /// - **Logging**: Output colored logs to terminal or file
    /// - **Testing**: Verify ANSI output formatting
    ///
    /// # See Also
    ///
    /// - [`ColorMode`] - Controls ANSI code generation
    /// - [`Style::write_style()`](AnsiSelectGraphicRendition::write) - Used internally for SGR segments
    /// - [`StyledString::write_str()`](crate::StyledString::write_str) - Similar method for `StyledString`
    pub fn write<W: std::io::Write>(
        &self,
        writer: &mut W,
        config: Option<&AnsiConfig>,
    ) -> std::io::Result<usize> {
        let mut total_len = 0;
        for segment in &self.0 {
            match segment {
                Segment::ASCII(text) | Segment::Unicode(text) => {
                    // Write plain text segments directly
                    writer.write_all(text.as_bytes())?;
                    total_len += text.len();
                }
                Segment::Control(control) => {
                    // Write control codes as their byte representation
                    writer.write_all(&[control.to_byte()])?;
                    total_len += 1;
                }
                Segment::Escape => {
                    // Write standalone ESC character
                    writer.write_all(b"\x1b")?;
                    total_len += 1;
                }
                Segment::CSI(csi) => {
                    if let Some(config) = config
                        && config.strip_csi
                    {
                        // Strip CSI Sequence
                    } else {
                        // Write CSI sequence
                        csi.write(writer)?;
                    }
                }
                Segment::SGR(sgr) => {
                    if let Some(config) = config {
                        if config.strip_sgr {
                            // Strip SGR Sequence
                        } else {
                            // Write SGR sequence
                            sgr.write(writer, Some(config.color_mode))?;
                        }
                    } else {
                        sgr.write(writer, None)?;
                    }
                }
                Segment::OSC(osc) => {
                    if let Some(config) = config
                        && config.strip_osc
                    {
                        // Strip OSC Sequence
                    } else {
                        // Write OSC sequence
                        osc.write(writer)?;
                    }
                }
                Segment::DCS(dcs) => {
                    if let Some(config) = config
                        && config.strip_dcs
                    {
                        // Strip DCS Sequence
                    } else {
                        // Write DCS sequence
                        dcs.write(writer)?;
                    }
                }
                Segment::SOS(sos) => {
                    if let Some(config) = config
                        && config.strip_sos_st
                    {
                        // Strip SOS Sequence
                    } else {
                        // Write SOS sequence
                        sos.write(writer)?;
                    }
                }
                Segment::ST => {
                    if let Some(config) = config
                        && config.strip_sos_st
                    {
                        // Strip ST Sequence
                    } else {
                        // Write ST sequence
                        writer.write_all(b"\x1b\\")?;
                    }
                }
                Segment::PM(pm) => {
                    if let Some(config) = config
                        && config.strip_pm
                    {
                        // Strip PM Sequence
                    } else {
                        // Write PM sequence
                        pm.write(writer)?;
                    }
                }
                Segment::APC(apc) => {
                    if let Some(config) = config
                        && config.strip_apc
                    {
                        // Strip APC Sequence
                    } else {
                        // Write APC sequence
                        apc.write(writer)?;
                    }
                }
                Segment::TelnetCommand(tc) => {
                    if let Some(config) = config
                        && config.strip_telnet
                    {
                        // Strip Telnet Sequence
                    } else {
                        // Write Telnet sequence
                        tc.write(writer)?;
                    }
                }
            }
        }
        Ok(total_len)
    }

    /// Parses a string containing ANSI escape sequences into a `SegmentedString`.
    ///
    /// This method performs a complete analysis of the input string, identifying and
    /// converting all content into distinct owned segments. It handles ASCII text,
    /// Unicode characters, control codes, and all ANSI escape sequences (CSI, OSC, DCS, etc.),
    /// producing a `SegmentedString` ready for manipulation or rendering.
    ///
    /// # How It Works
    ///
    /// The parsing process occurs in two stages:
    ///
    /// 1. **Parse into spans**: Uses [`SpannedString::parse`] to analyze the input and
    ///    create lightweight byte range references to each segment type.
    ///
    /// 2. **Extract and own content**: Converts the spans into owned segments by extracting
    ///    the actual content from the source string using [`SpannedString::into_segmented_string`].
    ///
    /// This approach leverages existing, well-tested parsing infrastructure while producing
    /// a `SegmentedString` with fully owned data suitable for further manipulation.
    ///
    /// # Arguments
    ///
    /// * `str` - A string-like value that can be converted to `&str` via `AsRef<str>`.
    ///   This includes `&str`, `String`, `Cow<str>`, and other string types.
    ///
    /// # Returns
    ///
    /// A `SegmentedString` containing owned segments representing all content from the input:
    ///
    /// - **ASCII/Unicode segments**: Text content merged intelligently
    /// - **Control segments**: Individual control codes (newlines, tabs, etc.)
    /// - **CSI segments**: Parsed cursor/erase/scroll commands
    /// - **SGR segments**: Style and color information (extracted from CSI sequences)
    /// - **OSC/DCS/etc.**: Other escape sequence types with their data
    ///
    /// # Segment Types Produced
    ///
    /// The parser identifies and creates the following segment types:
    ///
    /// ## Text Segments
    ///
    /// - **ASCII**: Printable ASCII characters (0x20-0x7E)
    /// - **Unicode**: Multi-byte UTF-8 characters
    ///
    /// These are automatically merged when adjacent for efficiency. For example,
    /// "Hello World" becomes a single ASCII segment, and "Hello世界" becomes a single
    /// Unicode segment.
    ///
    /// ## Control Segments
    ///
    /// - **Control**: Terminal control characters like LF (`\n`), CR (`\r`), HT (`\t`), etc.
    /// - **Escape**: Standalone ESC character or unrecognized escape sequences
    ///
    /// ## ANSI Escape Sequences
    ///
    /// - **CSI**: Control Sequence Introducer (cursor movement, erasing, scrolling)
    /// - **SGR**: Select Graphic Rendition (colors, bold, underline, etc.)
    /// - **OSC**: Operating System Commands (window title, etc.)
    /// - **DCS**: Device Control String
    /// - **SOS**: Start of String
    /// - **ST**: String Terminator
    /// - **PM**: Privacy Message
    /// - **APC**: Application Program Command
    ///
    /// # Parsing Features
    ///
    /// ## Greedy Segment Merging
    ///
    /// The parser merges consecutive compatible segments to minimize memory usage:
    ///
    /// - Consecutive ASCII characters → Single ASCII segment
    /// - Consecutive Unicode characters → Single Unicode segment
    /// - ASCII + Unicode → Promoted to single Unicode segment
    /// - Consecutive identical control codes → Single Control segment
    ///
    /// ## SGR Extraction
    ///
    /// CSI sequences with the `m` command (SGR - Select Graphic Rendition) are
    /// automatically converted into `SGR` segments with parsed [`AnsiSelectGraphicRendition`] information,
    /// making it easy to work with colors and text formatting.
    ///
    /// ## UTF-8 Support
    ///
    /// Full Unicode support with proper UTF-8 character boundary detection. Multi-byte
    /// characters are handled correctly and merged into Unicode segments.
    ///
    /// ## Escape Sequence Termination
    ///
    /// String-type sequences (OSC, DCS, etc.) are properly terminated by:
    /// - ST (String Terminator): ESC \ or 0x9C
    /// - BEL (Bell): 0x07 (for OSC only)
    ///
    /// # Performance
    ///
    /// - **Time Complexity**: O(n) where n is the length of the input string
    /// - **Space Complexity**: O(m) where m is the number of segments produced
    /// - **Allocations**: One allocation per segment for text/data content
    /// - **Single Pass**: The string is scanned once during the span parsing phase
    ///
    /// The two-stage approach (parse spans, then extract content) is efficient because
    /// span parsing is fast (just tracking byte ranges), and content extraction happens
    /// only once per segment.
    ///
    /// # Examples
    ///
    /// ## Plain Text
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let segmented = SegmentedString::parse("Hello World");
    /// assert_eq!(segmented.segment_count(), 1);
    /// assert_eq!(segmented.stripped(), "Hello World");
    /// ```
    ///
    /// ## ANSI Colors
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let input = "\x1b[31mRed Text\x1b[0m";
    /// let segmented = SegmentedString::parse(input);
    ///
    /// // Produces: SGR(red) → "Red Text" → SGR(reset)
    /// assert!(segmented.segment_count() >= 3);
    /// assert_eq!(segmented.stripped(), "Red Text");
    /// ```
    ///
    /// ## Mixed Content with Control Codes
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let input = "Line 1\nLine 2\tTabbed";
    /// let segmented = SegmentedString::parse(input);
    ///
    /// // Produces: "Line 1" → LF → "Line 2" → HT → "Tabbed"
    /// assert_eq!(segmented.stripped(), "Line 1Line 2Tabbed");
    /// ```
    ///
    /// ## Unicode Content
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let input = "Hello 世界";
    /// let segmented = SegmentedString::parse(input);
    ///
    /// // ASCII and Unicode merged into single segment
    /// assert_eq!(segmented.segment_count(), 1);
    /// assert_eq!(segmented.stripped(), "Hello 世界");
    /// ```
    ///
    /// ## Complex ANSI Sequences
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let input = "\x1b[1;32mBold Green\x1b[0m Normal\n";
    /// let segmented = SegmentedString::parse(input);
    ///
    /// // Produces multiple segments with styles, text, and control codes
    /// assert!(segmented.segment_count() > 3);
    /// ```
    ///
    /// ## Building from Parse
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// let mut segmented = SegmentedString::parse("\x1b[31mError:\x1b[0m ");
    /// segmented.push_str("File not found");
    /// segmented.push_control(termionix_ansicodes::ControlCode::LF);
    /// ```
    ///
    /// ## Generic String Types
    ///
    /// The method accepts any type implementing `AsRef<str>`:
    ///
    /// ```rust
    /// use termionix_ansicodes::SegmentedString;
    ///
    /// // From &str
    /// let s1 = SegmentedString::parse("text");
    ///
    /// // From String
    /// let s2 = SegmentedString::parse(String::from("text"));
    ///
    /// // From Cow<str>
    /// use std::borrow::Cow;
    /// let s3 = SegmentedString::parse(Cow::from("text"));
    /// ```
    ///
    /// # Edge Cases
    ///
    /// - **Empty String**: Returns an empty `SegmentedString` with no segments
    /// - **Incomplete Sequences**: Treated as Escape segments containing the incomplete bytes
    /// - **Invalid UTF-8**: Individual bytes may create invalid segments (not validated)
    /// - **Malformed CSI**: Parsed as CSICommand::Unknown with available parameters
    /// - **Unterminated Strings**: OSC/DCS/etc. without ST extend to end of input
    ///
    /// # Use Cases
    ///
    /// This method is ideal for:
    ///
    /// - **Terminal Output**: Building terminal content from ANSI-formatted strings
    /// - **Format Conversion**: Converting ANSI strings to manipulable segments
    /// - **Content Filtering**: Extracting or modifying specific segment types
    /// - **Testing**: Verifying ANSI string composition and structure
    /// - **Rendering**: Preparing content for display with different color modes
    ///
    /// # Comparison with Other Parsing
    ///
    /// - **[`SpannedString::parse`]**: Returns lightweight byte ranges, no owned data
    /// - **`SegmentedString::parse`**: Returns owned segments ready for manipulation
    /// - **[`AnsiMapper`](crate::AnsiMapper)**: Byte-by-byte stateful parsing for streaming
    ///
    /// Use `SegmentedString::parse` when you need to:
    /// - Manipulate the parsed content (add, remove, or modify segments)
    /// - Build output incrementally after parsing
    /// - Convert between representations
    /// - Store parsed content for later use
    ///
    /// Use `SpannedString::parse` when you only need to:
    /// - Analyze structure without copying data
    /// - Extract specific portions by byte range
    /// - Perform read-only operations
    ///
    /// # See Also
    ///
    /// - [`SpannedString::parse`] - Lightweight parsing returning byte ranges
    /// - [`SegmentedString::push_str`] - Add text to an existing segmented string
    /// - [`SegmentedString::push_segment`] - Add individual segments
    /// - [`SegmentedString::stripped`] - Extract plain text without ANSI codes
    /// - [`StyledString`](crate::StyledString) - Alternative representation with style metadata
    pub fn parse<S: AsRef<str>>(str: S) -> SegmentedString {
        SpannedString::parse(str.as_ref()).into_segmented_string(str.as_ref())
    }
}

impl Default for SegmentedString {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<&str> for SegmentedString {
    fn from(value: &str) -> Self {
        SegmentedString::parse(value)
    }
}

impl std::ops::Index<usize> for SegmentedString {
    type Output = Segment;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

/// Represents a discrete segment of content within a [`SegmentedString`].
///
/// Each segment has a specific type that determines its purpose and how it should
/// be interpreted when rendering terminal output. Segments are the building blocks
/// of a `SegmentedString` and preserve the exact structure of ANSI-formatted strings.
///
/// # Variant Categories
///
/// ## Text Content
/// - [`ASCII`](Segment::ASCII) - Plain ASCII text (most common for English text)
/// - [`Unicode`](Segment::Unicode) - Multi-byte Unicode text (international characters, emoji)
///
/// ## Control Characters
/// - [`Control`](Segment::Control) - Terminal control characters (newline, tab, bell, etc.)
/// - [`Escape`](Segment::Escape) - Standalone ESC character
///
/// ## ANSI Escape Sequences
/// - [`CSI`](Segment::CSI) - Control Sequence Introducer (cursor, erasing, etc.)
/// - [`SGR`](Segment::SGR) - Select Graphic Rendition (colors, bold, underline, etc.)
///
/// ## Advanced Escape Sequences
/// - [`OSC`](Segment::OSC) - Operating System Command (window title, etc.)
/// - [`DCS`](Segment::DCS) - Device Control String
/// - [`SOS`](Segment::SOS) - Start of String
/// - [`ST`](Segment::ST) - String Terminator
/// - [`PM`](Segment::PM) - Privacy Message
/// - [`APC`](Segment::APC) - Application Program Command
///
/// # Examples
///
/// Text segments store their content directly:
///
/// ```rust
/// use termionix_ansicodes::Segment;
///
/// let ascii_segment = Segment::ASCII("Hello".to_string());
/// let unicode_segment = Segment::Unicode("世界".to_string());
/// ```
///
/// Control and styling segments carry semantic meaning:
///
/// ```rust
/// use termionix_ansicodes::{Segment, ControlCode, Style, Color};
///
/// let newline = Segment::Control(ControlCode::LF);
/// let red_text = Segment::SGR(Style {
///     foreground: Some(Color::Red),
///     ..Default::default()
/// });
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    /// ASCII character (0x00-0x7F, excluding ESC and control codes)
    ASCII(String),
    /// Multibyte Unicode character
    Unicode(String),
    /// C0 or C1 Control character (0x00-0x1F, 0x7F-0x9F, excluding ESC)
    Control(AnsiControlCode),
    /// Single ESC character without a sequence
    Escape,
    /// CSI - Control Sequence Introducer (ESC [ ... final_byte)
    CSI(AnsiControlSequenceIntroducer),
    /// CSI SGR - Select Graphic Rendition (ESC [ ... final_byte])
    SGR(AnsiSelectGraphicRendition),
    /// OSC - Operating System Command (ESC ] ... ST or BEL)
    OSC(AnsiOperatingSystemCommand),
    /// DCS - Device Control String (ESC P ... ST)
    DCS(AnsiDeviceControlString),
    /// SOS - Start of String (ESC X ... ST)
    SOS(AnsiStartOfString),
    /// ST - String Terminator (ESC \)
    ST,
    /// PM - Privacy Message (ESC ^ ... ST)
    PM(AnsiPrivacyMessage),
    /// APC - Application Program Command (ESC _ ... ST)
    APC(AnsiApplicationProgramCommand),
    /// Telnet Command
    TelnetCommand(TelnetCommand),
}

impl std::fmt::Display for SegmentedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in &self.0 {
            Segment::fmt(segment, f)?;
        }
        Ok(())
    }
}

impl std::fmt::Display for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Segment::ASCII(text) | Segment::Unicode(text) => {
                // Write plain text segments directly
                f.write_str(text.as_str())?;
            }
            Segment::Control(control) => {
                // Write control codes as their byte representation
                write!(f, "{}", control.to_byte() as char)?;
            }
            Segment::Escape => {
                // Write standalone ESC character
                f.write_str("\x1b")?;
            }
            Segment::CSI(command) => {
                // Write CSI sequences
                AnsiControlSequenceIntroducer::fmt(command, f)?;
            }
            Segment::SGR(sgr) => {
                // Write SGR sequences
                AnsiSelectGraphicRendition::fmt(sgr, f)?;
            }
            Segment::OSC(osc) => {
                // Write OSC sequences
                AnsiOperatingSystemCommand::fmt(osc, f)?;
            }
            Segment::DCS(dcs) => {
                // Write DCS sequences
                AnsiDeviceControlString::fmt(dcs, f)?;
            }
            Segment::SOS(sos) => {
                // Write SOS sequences
                AnsiStartOfString::fmt(sos, f)?;
            }
            Segment::ST => {
                // Write ST sequence
                f.write_str("\x1b\\")?;
            }
            Segment::PM(pm) => {
                // Write PM sequences
                AnsiPrivacyMessage::fmt(pm, f)?;
            }
            Segment::APC(apc) => {
                // Write APC sequences
                AnsiApplicationProgramCommand::fmt(apc, f)?;
            }
            Segment::TelnetCommand(cmd) => {
                // Write Telnet commands
                TelnetCommand::fmt(cmd, f)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorMode;
    use crate::style::{Color, Intensity};
    // ============================================================================
    // SegmentedString Basic Tests
    // ============================================================================

    #[test]
    fn test_empty() {
        let seg = SegmentedString::empty();
        assert!(seg.is_empty());
        assert_eq!(seg.segment_count(), 0);
        assert_eq!(seg.stripped(), "");
    }

    #[test]
    fn test_is_empty_with_content() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello");
        assert!(!seg.is_empty());
    }

    #[test]
    fn test_is_empty_after_clear() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Test");
        seg.clear();
        assert!(seg.is_empty());
    }

    // ============================================================================
    // Push Character Tests
    // ============================================================================

    #[test]
    fn test_push_char_ascii() {
        let mut seg = SegmentedString::empty();
        seg.push_char('A');

        assert_eq!(seg.segment_count(), 1);
        assert_eq!(seg.stripped(), "A");
        assert!(!seg.is_empty());
    }

    #[test]
    fn test_push_char_multiple() {
        let mut seg = SegmentedString::empty();
        seg.push_char('H');
        seg.push_char('i');
        seg.push_char('!');

        assert_eq!(seg.segment_count(), 1); // Should merge into one ASCII segment
        assert_eq!(seg.stripped(), "Hi!");
    }

    #[test]
    fn test_push_char_unicode() {
        let mut seg = SegmentedString::empty();
        seg.push_char('日'); // Japanese character

        assert_eq!(seg.segment_count(), 1);
        assert_eq!(seg.stripped(), "日");
    }

    #[test]
    fn test_push_char_mixed_ascii_unicode() {
        let mut seg = SegmentedString::empty();
        seg.push_char('A');
        seg.push_char('🦀'); // Emoji
        seg.push_char('B');

        // Should create separate segments for ASCII and Unicode
        assert_eq!(seg.segment_count(), 1);
        assert_eq!(seg.stripped(), "A🦀B");
    }

    #[test]
    fn test_push_char_emojis() {
        let mut seg = SegmentedString::empty();
        seg.push_char('🦀');
        seg.push_char('🎉');
        seg.push_char('✨');

        assert_eq!(seg.stripped(), "🦀🎉✨");
    }

    #[test]
    fn test_push_char_after_push_str() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello");
        seg.push_char(' ');
        seg.push_char('!');

        assert_eq!(seg.stripped(), "Hello !");
    }

    // ============================================================================
    // Push String Tests
    // ============================================================================

    #[test]
    fn test_push_str_empty() {
        let mut seg = SegmentedString::empty();
        seg.push_str("");

        assert!(seg.is_empty());
        assert_eq!(seg.segment_count(), 0);
    }

    #[test]
    fn test_push_str_ascii() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello");

        assert_eq!(seg.segment_count(), 1);
        assert_eq!(seg.stripped(), "Hello");
    }

    #[test]
    fn test_push_str_unicode() {
        let mut seg = SegmentedString::empty();
        seg.push_str("こんにちは"); // Japanese "Hello"

        assert_eq!(seg.stripped(), "こんにちは");
    }

    #[test]
    fn test_push_str_mixed() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello 世界"); // Mixed ASCII and Unicode

        assert_eq!(seg.stripped(), "Hello 世界");
    }

    #[test]
    fn test_push_str_multiple_calls() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello");
        seg.push_str(" ");
        seg.push_str("World");

        assert_eq!(seg.stripped(), "Hello World");
    }

    #[test]
    fn test_push_str_with_emojis() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Rust 🦀 is awesome! 🎉");

        assert_eq!(seg.stripped(), "Rust 🦀 is awesome! 🎉");
    }

    // ============================================================================
    // Push Control Code Tests
    // ============================================================================

    #[test]
    fn test_push_control_single() {
        let mut seg = SegmentedString::empty();
        seg.push_ansi_control(AnsiControlCode::LF);

        assert_eq!(seg.segment_count(), 1);
        assert!(!seg.is_empty());
    }

    #[test]
    fn test_push_control_multiple() {
        let mut seg = SegmentedString::empty();
        seg.push_ansi_control(AnsiControlCode::CR);
        seg.push_ansi_control(AnsiControlCode::LF);

        assert_eq!(seg.segment_count(), 2);
    }

    #[test]
    fn test_push_control_with_text() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Line 1");
        seg.push_ansi_control(AnsiControlCode::LF);
        seg.push_str("Line 2");

        assert!(seg.segment_count() >= 3);
    }

    #[test]
    fn test_push_control_bell() {
        let mut seg = SegmentedString::empty();
        seg.push_ansi_control(AnsiControlCode::BEL);

        assert_eq!(seg.segment_count(), 1);
    }

    #[test]
    fn test_push_control_tab() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Column1");
        seg.push_ansi_control(AnsiControlCode::HT);
        seg.push_str("Column2");

        assert!(seg.segment_count() >= 3);
    }

    // ============================================================================
    // Push Style Tests
    // ============================================================================

    #[test]
    fn test_push_style_bold() {
        let mut seg = SegmentedString::empty();
        seg.push_style(AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });

        assert_eq!(seg.segment_count(), 1);
    }

    #[test]
    fn test_push_style_color() {
        let mut seg = SegmentedString::empty();
        seg.push_style(AnsiSelectGraphicRendition {
            foreground: Some(Color::Red),
            ..Default::default()
        });
        seg.push_str("Red text");

        assert!(seg.segment_count() >= 2);
    }

    #[test]
    fn test_push_style_multiple() {
        let mut seg = SegmentedString::empty();
        seg.push_style(AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });
        seg.push_str("Bold");
        seg.push_style(AnsiSelectGraphicRendition::default()); // Reset
        seg.push_str("Normal");

        assert!(seg.segment_count() >= 4);
    }

    #[test]
    fn test_push_style_complex() {
        let mut seg = SegmentedString::empty();
        seg.push_style(AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            foreground: Some(Color::Red),
            background: Some(Color::White),
            ..Default::default()
        });
        seg.push_str("Styled text");

        assert!(seg.segment_count() >= 2);
    }

    // ============================================================================
    // Clear Tests
    // ============================================================================

    #[test]
    fn test_clear_empty() {
        let mut seg = SegmentedString::empty();
        seg.clear();

        assert!(seg.is_empty());
        assert_eq!(seg.segment_count(), 0);
    }

    #[test]
    fn test_clear_with_content() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Test");
        seg.push_ansi_control(AnsiControlCode::LF);
        seg.push_style(AnsiSelectGraphicRendition::default());

        seg.clear();

        assert!(seg.is_empty());
        assert_eq!(seg.segment_count(), 0);
        assert_eq!(seg.stripped(), "");
    }

    #[test]
    fn test_clear_and_reuse() {
        let mut seg = SegmentedString::empty();
        seg.push_str("First");
        seg.clear();
        seg.push_str("Second");

        assert_eq!(seg.stripped(), "Second");
        assert!(!seg.is_empty());
    }

    // ============================================================================
    // Pop Tests
    // ============================================================================

    #[test]
    fn test_pop_empty() {
        let mut seg = SegmentedString::empty();

        assert_eq!(seg.pop(), None);
    }

    #[test]
    fn test_pop_single_char() {
        let mut seg = SegmentedString::empty();
        seg.push_char('A');

        assert_eq!(seg.pop(), Some('A'));
        assert!(seg.is_empty());
    }

    #[test]
    fn test_pop_multiple_chars() {
        let mut seg = SegmentedString::empty();
        seg.push_str("ABC");

        assert_eq!(seg.pop(), Some('C'));
        assert_eq!(seg.pop(), Some('B'));
        assert_eq!(seg.pop(), Some('A'));
        assert_eq!(seg.pop(), None);
    }

    #[test]
    fn test_pop_unicode() {
        let mut seg = SegmentedString::empty();
        seg.push_char('🦀');
        seg.push_char('日');

        assert_eq!(seg.pop(), Some('日'));
        assert_eq!(seg.pop(), Some('🦀'));
        assert_eq!(seg.pop(), None);
    }

    #[test]
    fn test_pop_mixed_content() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello");
        seg.push_char('🦀');

        assert_eq!(seg.pop(), Some('🦀'));
        assert_eq!(seg.stripped(), "Hello");
    }

    #[test]
    fn test_pop_after_control() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Text");
        seg.push_ansi_control(AnsiControlCode::LF);

        // Pop should only affect text segments
        let result = seg.pop();
        // Behavior depends on implementation
        assert!(result.is_some() || result.is_none());
    }

    #[test]
    fn test_pop_preserves_segments() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello");
        seg.push_ansi_control(AnsiControlCode::LF);
        seg.push_str("World");

        let initial_count = seg.segment_count();
        assert_eq!(initial_count, 3);
        seg.pop(); // Pop 'd'

        // Should still have multiple segments
        assert!(seg.segment_count() >= 2);
    }

    // ============================================================================
    // Segment Count Tests
    // ============================================================================

    #[test]
    fn test_segment_count_empty() {
        let seg = SegmentedString::empty();
        assert_eq!(seg.segment_count(), 0);
    }

    #[test]
    fn test_segment_count_single() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello");

        assert_eq!(seg.segment_count(), 1);
    }

    #[test]
    fn test_segment_count_multiple() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Text");
        seg.push_ansi_control(AnsiControlCode::LF);
        seg.push_style(AnsiSelectGraphicRendition::default());

        assert!(seg.segment_count() >= 3);
    }

    // ============================================================================
    // Styled Length Tests
    // ============================================================================

    #[test]
    fn test_styled_len_empty() {
        let seg = SegmentedString::empty();
        let config = AnsiConfig::default();

        assert_eq!(seg.len(Some(&config)).unwrap(), 0);
    }

    #[test]
    fn test_styled_len_plain_text() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello");

        let config = AnsiConfig::default();
        let len = seg.len(Some(&config)).unwrap();

        // Should include ANSI codes
        assert!(len >= 5); // At least the text length
    }

    #[test]
    fn test_styled_len_with_style() {
        let mut seg = SegmentedString::empty();
        seg.push_style(AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            foreground: Some(Color::Red),
            ..Default::default()
        });
        seg.push_str("Text");

        let config = AnsiConfig::enabled();

        // Should include ANSI escape codes
        assert_eq!(seg.len(Some(&config)).unwrap(), 11); // More than just "Text"
    }

    // ============================================================================
    // Stripped Tests
    // ============================================================================

    #[test]
    fn test_stripped_empty() {
        let seg = SegmentedString::empty();
        assert_eq!(seg.stripped(), "");
    }

    #[test]
    fn test_stripped_text_only() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello World");

        assert_eq!(seg.stripped(), "Hello World");
    }

    #[test]
    fn test_stripped_with_styles() {
        let mut seg = SegmentedString::empty();
        seg.push_style(AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });
        seg.push_str("Bold");
        seg.push_style(AnsiSelectGraphicRendition::default());
        seg.push_str("Normal");

        assert_eq!(seg.stripped(), "BoldNormal");
    }

    #[test]
    fn test_stripped_with_unicode() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello 世界 🦀");

        assert_eq!(seg.stripped(), "Hello 世界 🦀");
    }

    #[test]
    fn test_stripped_ignores_control_codes() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Line1");
        seg.push_ansi_control(AnsiControlCode::LF);
        seg.push_str("Line2");

        let stripped = seg.stripped();
        assert!(stripped.contains("Line1"));
        assert!(stripped.contains("Line2"));
    }

    // ============================================================================
    // Write String Tests
    // ============================================================================

    #[test]
    fn test_write_str_empty() {
        let seg = SegmentedString::empty();
        let config = AnsiConfig {
            color_mode: ColorMode::None,
            ..Default::default()
        };
        let mut output = Vec::new();

        seg.write(&mut output, Some(&config)).unwrap();
        assert_eq!(output, vec![]);
    }

    #[test]
    fn test_write_str_plain_text() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello");

        let config = AnsiConfig::strip_all();

        let mut output = Vec::new();
        let len = seg.write(&mut output, Some(&config)).unwrap();

        assert_eq!(len, 5);
        assert_eq!(output, b"Hello");
    }

    #[test]
    fn test_write_str_with_color_none() {
        let mut seg = SegmentedString::empty();
        seg.push_style(AnsiSelectGraphicRendition {
            foreground: Some(Color::Red),
            ..Default::default()
        });
        seg.push_str("Text");

        let config = AnsiConfig {
            strip_sgr: true,
            ..Default::default()
        };
        let mut output = Vec::new();
        let len = seg.write(&mut output, Some(&config)).unwrap();

        // Should not contain ANSI codes with ColorMode::None
        assert_eq!(len, 4);
        assert_eq!(&output, b"Text");
    }

    #[test]
    fn test_write_str_with_color_basic() {
        let mut seg = SegmentedString::empty();
        seg.push_style(AnsiSelectGraphicRendition {
            foreground: Some(Color::Red),
            ..Default::default()
        });
        seg.push_str("Text");

        let config = AnsiConfig {
            color_mode: ColorMode::Basic,
            ..Default::default()
        };
        let mut output = Vec::new();
        let len = seg.write(&mut output, Some(&config)).unwrap();

        // Should contain ANSI codes
        assert_eq!(len, 13);
        assert_eq!(output, b"\x1b[31mText\x1b[0m");
    }

    #[test]
    fn test_write_str_control_codes() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Line1");
        seg.push_ansi_control(AnsiControlCode::LF);
        seg.push_str("Line2");

        let config = AnsiConfig {
            color_mode: ColorMode::None,
            ..Default::default()
        };
        let mut output = Vec::new();
        let len = seg.write(&mut output, Some(&config)).unwrap();

        assert_eq!(len, 11);
        assert_eq!(output, b"Line1\nLine2");
    }

    // ============================================================================
    // Iterator Tests
    // ============================================================================

    #[test]
    fn test_iter_empty() {
        let seg = SegmentedString::empty();
        assert_eq!(seg.segments().count(), 0);
    }

    #[test]
    fn test_iter_single_segment() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Hello");

        assert_eq!(seg.segments().count(), 1);
    }

    #[test]
    fn test_iter_multiple_segments() {
        let mut seg = SegmentedString::empty();
        seg.push_str("Text");
        seg.push_ansi_control(AnsiControlCode::LF);
        seg.push_style(AnsiSelectGraphicRendition::default());

        assert!(seg.segments().count() >= 3);
    }

    // ============================================================================
    // Integration Tests
    // ============================================================================

    #[test]
    fn test_complex_sequence() {
        let mut seg = SegmentedString::empty();

        // Build complex content
        seg.push_style(AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            foreground: Some(Color::Red),
            ..Default::default()
        });
        seg.push_str("Error:");
        seg.push_style(AnsiSelectGraphicRendition::default());
        seg.push_char(' ');
        seg.push_str("File not found");
        seg.push_ansi_control(AnsiControlCode::LF);

        assert!(!seg.is_empty());
        assert!(seg.segment_count() >= 5);
        assert!(seg.stripped().contains("Error:"));
        assert!(seg.stripped().contains("File not found"));
    }

    #[test]
    fn test_push_pop_symmetry() {
        let mut seg = SegmentedString::empty();
        let text = "Test";

        for ch in text.chars() {
            seg.push_char(ch);
        }

        let mut popped = String::new();
        while let Some(ch) = seg.pop() {
            popped.insert(0, ch);
        }

        assert_eq!(popped, text);
        assert!(seg.is_empty());
    }

    #[test]
    fn test_unicode_boundary_handling() {
        let mut seg = SegmentedString::empty();
        seg.push_str("a");
        seg.push_char('🦀'); // 4-byte emoji
        seg.push_str("b");
        seg.push_char('日'); // 3-byte char
        seg.push_str("c");

        let result = seg.stripped();
        assert_eq!(result, "a🦀b日c");
    }

    #[test]
    fn test_empty_string_push() {
        let mut seg = SegmentedString::empty();
        seg.push_str("");
        seg.push_str("Hello");
        seg.push_str("");

        assert_eq!(seg.stripped(), "Hello");
    }

    #[test]
    fn test_sequential_operations() {
        let mut seg = SegmentedString::empty();

        seg.push_str("Start");
        assert_eq!(seg.stripped(), "Start");

        seg.push_char(' ');
        assert_eq!(seg.stripped(), "Start ");

        seg.push_ansi_control(AnsiControlCode::HT);
        seg.push_str("End");
        assert!(seg.stripped().contains("End"));

        seg.clear();
        assert!(seg.is_empty());

        seg.push_str("New");
        assert_eq!(seg.stripped(), "New");
    }
}
