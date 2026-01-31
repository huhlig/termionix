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

//! TODO: Fix StyledString

use crate::style::{AnsiSelectGraphicRendition, Blink, Color, Intensity, Underline};
use crate::{AnsiConfig, AnsiResult, SegmentedString};
use std::ops::Range;

/// Represents a string with internal data for the ANSI escape sequences, so it
/// can be constructed when the `Display` is called. It is preferred to use the
/// `Styled` trait to interact with your strings instead of manually
/// constructing a `StyledString`, which is more verbose.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct StyledString {
    segments: Vec<Segment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Segment {
    range: Range<usize>,
    buffer: String,
    style: AnsiSelectGraphicRendition,
}

impl PartialOrd for Segment {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Self::cmp(self, other))
    }
}

impl Ord for Segment {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.range.start.cmp(&other.range.start)
    }
}

impl StyledString {
    /// Creates a new empty `StyledString` with no segments.
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let styled = StyledString::empty();
    /// assert_eq!(styled.stripped_len(), 0);
    /// ```
    pub fn empty() -> StyledString {
        StyledString {
            segments: Vec::default(),
        }
    }

    /// Creates a new `StyledString` from a string slice with an optional style.
    ///
    /// # Arguments
    ///
    /// * `str` - The text content to create the styled string from
    /// * `style` - Optional style to apply to the entire string. If `None`, uses default style.
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::{StyledString, Style, Color, Intensity};
    ///
    /// // Create without style
    /// let plain = StyledString::from_string("Hello", None);
    ///
    /// // Create with style
    /// let styled = StyledString::from_string("Hello", Some(Style {
    ///     intensity: Some(Intensity::Bold),
    ///     foreground: Some(Color::Red),
    ///     ..Default::default()
    /// }));
    /// ```
    pub fn from_string(str: &str, style: Option<AnsiSelectGraphicRendition>) -> StyledString {
        StyledString {
            segments: vec![Segment {
                range: 0..str.len(),
                buffer: String::from(str),
                style: style.unwrap_or_default(),
            }],
        }
    }

    /// Check if StyledString is empty
    ///
    /// Returns `true` if the styled string contains no segments or if all segments are empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let empty = StyledString::empty();
    /// assert!(empty.is_empty());
    ///
    /// let mut styled = StyledString::empty();
    /// styled.concat("Hello");
    /// assert!(!styled.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.stripped_len() == 0
    }

    /// Returns the total length of the stripped string in bytes.
    ///
    /// This sums up the lengths of all segments in the styled string.
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::utility::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.concat("Hello");
    /// styled.concat(" World");
    /// assert_eq!(styled.stripped_len(), 11);
    /// ```
    pub fn stripped_len(&self) -> usize {
        if let Some(start) = self.segments.first() {
            if let Some(end) = self.segments.last() {
                end.range.end - start.range.start
            } else {
                start.range.end - start.range.start
            }
        } else {
            0
        }
    }

    /// Returns the total length of the styled string including ANSI escape codes
    /// for the specified color mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - The color mode determining which ANSI codes to include in the length calculation
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::{ ColorMode, AnsiConfig};
    /// use termionix_ansicodec::ansi::{Color, Intensity};
    /// use termionix_ansicodec::utility::StyledString;
    ///
    /// let config = AnsiConfig::enabled();
    /// let styled = StyledString::from_string("Hello", Some(Style {
    ///     intensity: Some(Intensity::Bold),
    ///     foreground: Some(Color::Red),
    ///     ..Default::default()
    /// }));
    /// let len = styled.styled_len(Some(&config));
    /// assert!(len > 5); // Longer than just "Hello" due to ANSI codes
    /// ```
    ///
    pub fn styled_len(&self, config: Option<&AnsiConfig>) -> AnsiResult<usize> {
        let mut total = 0;

        for segment in &self.segments {
            // Count the style codes
            let mut temp = String::new();
            segment
                .style
                .write_str(&mut temp, Some(config.unwrap().color_mode))
                .unwrap();

            total += temp.len();

            // Count the segment text
            total += segment.buffer.len();

            // Count the reset code "\x1b[0m" (4 bytes)
            total += 4;
        }

        Ok(total)
    }

    /// Clears all content from the `StyledString`, removing all segments and styles.
    ///
    /// This method resets the styled string to an empty state, equivalent to creating
    /// a new `StyledString` with [`StyledString::empty()`]. All text content and
    /// associated styling information is discarded.
    ///
    /// After calling this method:
    /// - [`len()`](StyledString::stripped_len) will return 0
    /// - [`is_empty()`](StyledString::is_empty) will return `true`
    /// - [`stripped()`](StyledString::stripped) will return an empty string
    /// - All internal segments and their styles are removed
    ///
    /// # Performance
    ///
    /// This is an efficient operation that simply clears the internal segment vector.
    /// The underlying memory capacity is retained, making subsequent operations
    /// potentially more efficient if the `StyledString` is reused.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use termionix_ansicodec::utility::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.concat("Hello World");
    /// assert_eq!(styled.stripped_len(), 11);
    ///
    /// styled.clear();
    /// assert_eq!(styled.stripped_len(), 0);
    /// assert!(styled.is_empty());
    /// ```
    ///
    /// Clearing styled content:
    ///
    /// ```
    /// use termionix_ansicodec::{StyledString, Style, Color, Intensity};
    ///
    /// let mut styled = StyledString::from_string("Bold Text", Some(Style {
    ///     intensity: Some(Intensity::Bold),
    ///     foreground: Some(Color::Red),
    ///     ..Default::default()
    /// }));
    ///
    /// styled.clear();
    /// assert_eq!(styled.stripped(), "");
    /// ```
    ///
    /// Reusing after clear:
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.concat("First message");
    /// styled.clear();
    ///
    /// // Reuse the same StyledString
    /// styled.concat("Second message");
    /// assert_eq!(styled.stripped(), "Second message");
    /// ```
    ///
    /// Clearing multiple segments:
    ///
    /// ```
    /// use termionix_ansicodec::{StyledString, Style, Color};
    ///
    /// let mut styled = StyledString::empty();
    /// styled.concat("Normal ");
    /// styled.concat_with_style("Red", Style {
    ///     foreground: Some(Color::Red),
    ///     ..Default::default()
    /// });
    /// styled.concat_with_style(" Blue", Style {
    ///     foreground: Some(Color::Blue),
    ///     ..Default::default()
    /// });
    ///
    /// styled.clear();
    /// assert_eq!(styled.stripped_len(), 0);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`StyledString::empty()`](StyledString::empty) - Create a new empty styled string
    /// - [`StyledString::is_empty()`](StyledString::is_empty) - Check if the string is empty
    /// - [`StyledString::len()`](StyledString::stripped_len) - Get the length of the string
    pub fn clear(&mut self) {
        self.segments.clear();
    }

    /// Appends a single character to the end of this `StyledString`.
    ///
    /// This method behaves similarly to [`String::push()`], appending a character
    /// to the last segment in the styled string. If the styled string is empty,
    /// a new segment with default styling is created.
    ///
    /// The character is added to the last segment's buffer, and the segment's
    /// range is extended accordingly. This preserves the existing style of the
    /// last segment rather than creating a new segment for each character.
    ///
    /// # Arguments
    ///
    /// * `ch` - The character to append to the styled string
    ///
    /// # Performance
    ///
    /// This operation is efficient for building strings character-by-character,
    /// as it reuses existing segments rather than creating new ones for each
    /// character. The underlying buffer may reallocate if it needs to grow.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.push('H');
    /// styled.push('i');
    /// styled.push('!');
    ///
    /// assert_eq!(styled.stripped(), "Hi!");
    /// assert_eq!(styled.stripped_len(), 3);
    /// ```
    ///
    /// Unicode character support:
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.push('🦀'); // Rust crab emoji (4 bytes in UTF-8)
    /// styled.push('日'); // Japanese character (3 bytes in UTF-8)
    ///
    /// assert_eq!(styled.stripped(), "🦀日");
    /// assert_eq!(styled.stripped_len(), 7); // 4 + 3 bytes
    /// ```
    ///
    /// Building styled text character by character:
    ///
    /// ```
    /// use termionix_ansicodec::{StyledString, Style, Color, Intensity};
    ///
    /// let mut styled = StyledString::from_string("Bold", Some(Style {
    ///     intensity: Some(Intensity::Bold),
    ///     ..Default::default()
    /// }));
    ///
    /// // Characters pushed after the initial creation inherit the last segment's style
    /// styled.push('!');
    ///
    /// assert_eq!(styled.stripped(), "Bold!");
    /// ```
    ///
    /// # See Also
    ///
    /// - [`concat()`](StyledString::concat) - Append an entire string with default styling
    /// - [`concat_with_style()`](StyledString::concat_with_style) - Append a string with specific styling
    pub fn push(&mut self, ch: char) {
        if let Some(last_segment) = self.segments.last_mut() {
            // If there's an existing segment, append to it
            last_segment.buffer.push(ch);
            last_segment.range.end += ch.len_utf8();
        } else {
            // If there are no segments, create a new one with the default style
            let char_len = ch.len_utf8();
            self.segments.push(Segment {
                range: 0..char_len,
                buffer: ch.to_string(),
                style: AnsiSelectGraphicRendition::default(),
            });
        }
    }

    /// Appends a string slice to the end of this `StyledString`.
    ///
    /// This method behaves similarly to [`String::push_str()`], appending a string
    /// to the last segment in the styled string. If the styled string is empty,
    /// a new segment with default styling is created. Empty strings are ignored
    /// and don't affect the styled string.
    ///
    /// The string is added to the last segment's buffer, and the segment's
    /// range is extended accordingly. This preserves the existing style of the
    /// last segment rather than creating a new segment for each string.
    ///
    /// # Arguments
    ///
    /// * `str` - A value that can be referenced as a string slice (e.g., `&str`, `String`, `&String`)
    ///
    /// # Performance
    ///
    /// This operation is efficient for building strings incrementally, as it reuses
    /// existing segments rather than creating new ones for each string. The underlying
    /// buffer may reallocate if it needs to grow.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.push_str("Hello");
    /// styled.push_str(" ");
    /// styled.push_str("World");
    ///
    /// assert_eq!(styled.stripped(), "Hello World");
    /// assert_eq!(styled.stripped_len(), 11);
    /// ```
    ///
    /// Accepting different string types:
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.push_str("str slice");           // &str
    /// styled.push_str(&String::from(" String")); // &String
    /// styled.push_str(String::from(" owned")); // String
    ///
    /// assert_eq!(styled.stripped(), "str slice String owned");
    /// ```
    ///
    /// Unicode string support:
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.push_str("Hello ");
    /// styled.push_str("世界");  // "World" in Japanese
    /// styled.push_str("! 🦀"); // With emoji
    ///
    /// assert_eq!(styled.stripped(), "Hello 世界! 🦀");
    /// ```
    ///
    /// Building styled text string by string:
    ///
    /// ```
    /// use termionix_ansicodec::{StyledString, Style, Color, Intensity};
    ///
    /// let mut styled = StyledString::from_string("Bold text", Some(Style {
    ///     intensity: Some(Intensity::Bold),
    ///     ..Default::default()
    /// }));
    ///
    /// // Strings pushed after the initial creation inherit the last segment's style
    /// styled.push_str(" continues");
    ///
    /// assert_eq!(styled.stripped(), "Bold text continues");
    /// ```
    ///
    /// Empty strings are no-ops:
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.push_str("");
    /// styled.push_str("Hello");
    /// styled.push_str("");
    ///
    /// assert_eq!(styled.stripped(), "Hello");
    /// assert_eq!(styled.stripped_len(), 5);
    /// ```
    ///
    /// # See Also
    ///
    /// - [`push()`](StyledString::push) - Append a single character
    /// - [`concat()`](StyledString::concat) - Append a string with default styling
    /// - [`concat_with_style()`](StyledString::concat_with_style) - Append a string with specific styling
    pub fn push_str<S: AsRef<str>>(&mut self, str: S) {
        let str = str.as_ref();

        // Early return for empty strings
        if str.is_empty() {
            return;
        }

        if let Some(last_segment) = self.segments.last_mut() {
            // If there's an existing segment, append to it
            last_segment.buffer.push_str(str);
            last_segment.range.end += str.len();
        } else {
            // If there are no segments, create a new one with the default style
            let str_len = str.len();
            self.segments.push(Segment {
                range: 0..str_len,
                buffer: str.to_string(),
                style: AnsiSelectGraphicRendition::default(),
            });
        }
    }

    /// Removes the last character from this `StyledString` and returns it,
    /// or `None` if the string is empty.
    ///
    /// This method behaves similarly to [`String::pop()`], removing a character
    /// from the last segment in the styled string. If removing the character
    /// leaves a segment empty, that segment is removed. The segment's range
    /// is adjusted accordingly.
    ///
    /// # Returns
    ///
    /// Returns `Some(char)` containing the last character if the string is not empty,
    /// or `None` if the string is empty.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.push('a');
    /// styled.push('b');
    /// styled.push('c');
    ///
    /// assert_eq!(styled.pop(), Some('c'));
    /// assert_eq!(styled.pop(), Some('b'));
    /// assert_eq!(styled.stripped(), "a");
    /// ```
    ///
    /// Popping from an empty string:
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// assert_eq!(styled.pop(), None);
    /// ```
    ///
    /// Unicode character support:
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.push('🦀'); // Rust crab emoji (4 bytes in UTF-8)
    /// styled.push('日'); // Japanese character (3 bytes in UTF-8)
    ///
    /// assert_eq!(styled.pop(), Some('日'));
    /// assert_eq!(styled.pop(), Some('🦀'));
    /// assert_eq!(styled.pop(), None);
    /// ```
    pub fn pop(&mut self) -> Option<char> {
        if let Some(last_segment) = self.segments.last_mut() {
            if let Some(ch) = last_segment.buffer.pop() {
                // Adjust the range to reflect the removed character
                last_segment.range.end -= ch.len_utf8();

                // If the segment is now empty, remove it
                if last_segment.buffer.is_empty() {
                    self.segments.pop();
                }

                Some(ch)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Appends a string to the end of this `StyledString` with default styling.
    ///
    /// # Arguments
    ///
    /// * `str` - The text to append
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::StyledString;
    ///
    /// let mut styled = StyledString::empty();
    /// styled.concat("Hello");
    /// styled.concat(" World");
    /// assert_eq!(styled.stripped_len(), 11);
    /// ```
    pub fn concat(&mut self, str: &str) {
        Self::concat_with_style(self, str, AnsiSelectGraphicRendition::default());
    }

    /// Appends a string to the end of this `StyledString` with the specified style.
    ///
    /// # Arguments
    ///
    /// * `str` - The text to append
    /// * `style` - The style to apply to the appended text
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::{StyledString, Style, Color, Intensity};
    ///
    /// let mut styled = StyledString::empty();
    /// styled.concat("Normal ");
    /// styled.concat_with_style("Bold", Style {
    ///     intensity: Some(Intensity::Bold),
    ///     ..Default::default()
    /// });
    /// ```
    pub fn concat_with_style(&mut self, str: &str, style: AnsiSelectGraphicRendition) {
        self.segments.push(Segment {
            range: self.stripped_len()..self.stripped_len() + str.len(),
            buffer: String::from(str),
            style,
        })
    }

    /// Applies a style to a specific range of the string.
    ///
    /// This method splits segments as necessary to apply the style only to the specified range.
    /// If the range overlaps multiple segments, each segment is split appropriately, creating
    /// new segments for the portions before, within, and after the styled range.
    ///
    /// # Arguments
    ///
    /// * `style` - The style to apply
    /// * `range` - The byte range to apply the style to (start..end)
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// use termionix_ansicodec::ansi::Underline;
    /// use termionix_ansicodec::utility::StyledString;
    /// let mut styled = StyledString::from_string("Hello World", None);
    /// styled.set_style(Style {
    ///     underline: Some(Underline::Single),
    ///     ..Default::default()
    /// }, 6..11); // Underline "World"
    /// ```
    pub fn set_style(&mut self, style: AnsiSelectGraphicRendition, range: Range<usize>) {
        let mut new_segments = Vec::new();
        let mut i = 0;

        while i < self.segments.len() {
            let segment = &self.segments[i];

            // Case 1: Range completely before the segment
            if range.end <= segment.range.start {
                new_segments.push(segment.clone());
            }
            // Case 2: Range completely after the segment
            else if range.start >= segment.range.end {
                new_segments.push(segment.clone());
            }
            // Case 3: Range overlaps with segment
            else {
                // Add segment before range if exists
                if range.start > segment.range.start {
                    new_segments.push(Segment {
                        range: segment.range.start..range.start,
                        buffer: segment.buffer[..(range.start - segment.range.start)].to_string(),
                        style: segment.style.clone(),
                    });
                }

                // Add the styled segment
                let start = range.start.max(segment.range.start);
                let end = range.end.min(segment.range.end);
                new_segments.push(Segment {
                    range: start..end,
                    buffer: segment.buffer
                        [(start - segment.range.start)..(end - segment.range.start)]
                        .to_string(),
                    style: style.clone(),
                });

                // Add segment after range if exists
                if range.end < segment.range.end {
                    new_segments.push(Segment {
                        range: range.end..segment.range.end,
                        buffer: segment.buffer[(range.end - segment.range.start)..].to_string(),
                        style: segment.style.clone(),
                    });
                }
            }
            i += 1;
        }

        self.segments = new_segments;
    }

    /// Returns the string content without any ANSI styling codes.
    ///
    /// This method concatenates all segment buffers into a plain string,
    /// effectively stripping all styling information.
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// use termionix_ansicodec::ansi::{Color, Intensity};
    /// use termionix_ansicodec::utility::StyledString;
    ///
    /// let styled = StyledString::from_string("Hello", Some(Style {
    ///     intensity: Some(Intensity::Bold),
    ///     foreground: Some(Color::Red),
    ///     ..Default::default()
    /// }));
    /// assert_eq!(styled.stripped(), "Hello");
    /// ```
    pub fn stripped(&self) -> String {
        self.segments
            .iter()
            .map(|segment| segment.buffer.clone())
            .collect::<String>()
    }

    /// Converts this `StyledString` into a `SegmentedString` with explicit style segments.
    ///
    /// This method transforms a `StyledString` that stores text with styling metadata
    /// into a `SegmentedString` with explicit SGR (Select Graphic Rendition) segments.
    /// Each styled segment in the `StyledString` is converted into a sequence of segments
    /// in the resulting `SegmentedString`:
    /// 1. An SGR segment with the style (if not default)
    /// 2. The text content
    /// 3. A style reset SGR segment (if styling was applied)
    ///
    /// This conversion is useful when you need to:
    /// - Generate ANSI escape sequences for terminal output
    /// - Convert styled text to a format with explicit control over segments
    /// - Preserve the exact structure of styles and resets
    /// - Interface with systems that expect explicit style boundaries
    ///
    /// # Segment Generation
    ///
    /// For each internal segment with style and text:
    /// - **Non-default style**: Generates `SGR(style) → Text → SGR(default)`
    /// - **Default style**: Generates only `Text` (no style segments)
    ///
    /// This ensures that styles are properly scoped and reset, preventing style
    /// "bleed" into subsequent segments.
    ///
    /// # Returns
    ///
    /// A new `SegmentedString` containing:
    /// - SGR segments for style changes
    /// - Text segments (ASCII or Unicode) for content
    /// - SGR reset segments after each styled portion
    ///
    /// # Performance
    ///
    /// This is an O(n) operation where n is the number of segments in the `StyledString`.
    /// Each segment is processed once, and the resulting `SegmentedString` may contain
    /// up to 3× the number of segments (style, text, reset for each styled segment).
    ///
    /// # Examples
    ///
    /// Basic conversion:
    ///
    /// ```rust
    /// use termionix_ansicodes::{StyledString, Style, Color, Intensity};
    ///
    /// let mut styled = StyledString::empty();
    /// styled.concat_with_style("Error", Style {
    ///     foreground: Some(Color::Red),
    ///     intensity: Some(Intensity::Bold),
    ///     ..Default::default()
    /// });
    ///
    /// let segmented = styled.segmented();
    /// // Results in: SGR(red+bold) → "Error" → SGR(reset)
    /// assert!(segmented.segment_count() >= 1);
    /// ```
    ///
    /// Multiple styled segments:
    ///
    /// ```rust
    /// use termionix_ansicodes::{StyledString, Style, Color};
    ///
    /// let mut styled = StyledString::empty();
    /// styled.concat_with_style("Red", Style {
    ///     foreground: Some(Color::Red),
    ///     ..Default::default()
    /// });
    /// styled.concat(" "); // Default style
    /// styled.concat_with_style("Blue", Style {
    ///     foreground: Some(Color::Blue),
    ///     ..Default::default()
    /// });
    ///
    /// let segmented = styled.segmented();
    /// // Results in: SGR(red) → "Red" → SGR(reset) → " " → SGR(blue) → "Blue" → SGR(reset)
    /// ```
    ///
    /// Plain text without styling:
    ///
    /// ```rust
    /// use termionix_ansicodes::StyledString;
    ///
    /// let styled = StyledString::from_string("Plain text", None);
    /// let segmented = styled.segmented();
    ///
    /// // Only text segments, no style segments
    /// assert_eq!(segmented.stripped(), "Plain text");
    /// ```
    ///
    /// Complex styling with range-based styles:
    ///
    /// ```rust
    /// use termionix_ansicodes::{StyledString, Style, Color};
    ///
    /// let mut styled = StyledString::from_string("Hello World", None);
    /// styled.set_style(Style {
    ///     foreground: Some(Color::Red),
    ///     ..Default::default()
    /// }, 0..5); // Style "Hello"
    ///
    /// let segmented = styled.segmented();
    /// // Converts the split segments into explicit style boundaries
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Terminal output**: Generate ANSI codes for displaying styled text
    /// - **Serialization**: Convert to a format with explicit style boundaries
    /// - **Protocol compatibility**: Interface with systems expecting SGR segments
    /// - **Testing**: Verify style application and reset behavior
    /// - **Format conversion**: Bridge between `StyledString` and `SegmentedString`
    ///
    /// # Comparison with Other Conversions
    ///
    /// - [`SpannedString::into_segmented_string`](crate::SpannedString::into_segmented_string):
    ///   Converts byte ranges to segments by extracting from source string
    /// - `StyledString::into_segmented_string`: Converts style metadata to explicit SGR segments
    /// - Both produce `SegmentedString` but from different source representations
    ///
    /// # Style Reset Behavior
    ///
    /// The method ensures clean style boundaries by:
    /// 1. Emitting an SGR segment before each styled text portion
    /// 2. Always resetting to default style after styled portions
    /// 3. Skipping style segments for text with default styling
    ///
    /// This prevents styles from unintentionally affecting subsequent text.
    ///
    /// # See Also
    ///
    /// - [`SegmentedString`] - The target type with explicit segments
    /// - [`SegmentedString::push_style`] - How style segments are added
    /// - [`SegmentedString::push_str`] - How text segments are added
    /// - [`SpannedString::into_segmented_string`] - Similar conversion from parsed spans
    /// - [`AnsiSelectGraphicRendition`] - The style type used in SGR segments
    pub fn segmented(&self) -> SegmentedString {
        let mut segmented = SegmentedString::empty();

        for segment in &self.segments {
            // Push the style if it's not default
            if segment.style != AnsiSelectGraphicRendition::default() {
                segmented.push_style(segment.style.clone());
            }

            // Push the text content
            segmented.push_str(&segment.buffer);

            // Push a style reset if the style was not default
            if segment.style != AnsiSelectGraphicRendition::default() {
                segmented.push_style(AnsiSelectGraphicRendition::default());
            }
        }

        segmented
    }

    /// Writes the styled string with ANSI escape codes to a writer.
    ///
    /// This method generates the appropriate ANSI escape sequences based on the
    /// color mode and writes them along with the text content to the provided writer.
    /// Each segment is written with its opening ANSI codes, content, and a reset code.
    ///
    /// # Arguments
    ///
    /// * `mode` - The color mode determining which ANSI codes to generate (None, Basic, Extended, TrueColor)
    /// * `writer` - The writer to output the styled string to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or a `std::fmt::Error` if writing fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::{ColorMode, AnsiConfig};
    /// use termionix_ansicodec::utility::StyledString;
    ///
    /// let config = AnsiConfig::default();
    /// let styled = StyledString::from_string("Hello", None);
    /// let mut output = String::new();
    /// styled.write_str(&mut output, Some(&config)).unwrap();
    /// ```
    pub fn write_str<W: std::fmt::Write>(
        &self,
        writer: &mut W,
        config: Option<&AnsiConfig>,
    ) -> std::fmt::Result {
        // Write the styled segments
        for segment in &self.segments {
            // Write opening ANSI escape codes for this segment's style
            segment
                .style
                .write_str(writer, Some(config.unwrap().color_mode))?;

            // Write the segment's text
            writer.write_str(&segment.buffer)?;

            // Reset style after each segment
            writer.write_str("\x1b[0m")?;
        }
        Ok(())
    }
}

impl std::str::FromStr for StyledString {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut segments = Vec::new();
        let mut current_style = AnsiSelectGraphicRendition::default();
        let mut buffer = String::new();
        let mut pos = 0;
        let bytes = s.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            // Check for ANSI escape sequence (ESC [)
            if bytes[i] == b'\x1b' && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                // Save current segment if there's any text
                if !buffer.is_empty() {
                    segments.push(Segment {
                        range: pos..(pos + buffer.len()),
                        buffer: buffer.clone(),
                        style: current_style.clone(),
                    });
                    pos += buffer.len();
                    buffer.clear();
                }

                // Parse the escape sequence
                i += 2; // Skip ESC [
                let code_start = i;

                // Find the end of the escape sequence (the 'm' character)
                while i < bytes.len() && bytes[i] != b'm' {
                    i += 1;
                }

                if i < bytes.len() {
                    // Extract the parameter string
                    let code_str = std::str::from_utf8(&bytes[code_start..i]).unwrap_or("");
                    i += 1; // Skip 'm'

                    // Parse the codes
                    if code_str.is_empty() || code_str == "0" {
                        // Reset all attributes
                        current_style = AnsiSelectGraphicRendition::default();
                    } else {
                        // Split by semicolons and parse each code
                        let codes: Vec<&str> = code_str.split(';').collect();
                        parse_ansi_codes(&codes, &mut current_style);
                    }
                }
            } else {
                // Regular character - append to buffer
                // Handle UTF-8 multi-byte sequences properly
                let char_start = i;
                let char_len = if bytes[i] < 0x80 {
                    1 // ASCII
                } else if bytes[i] & 0b1110_0000 == 0b1100_0000 {
                    2 // 2-byte UTF-8
                } else if bytes[i] & 0b1111_0000 == 0b1110_0000 {
                    3 // 3-byte UTF-8
                } else if bytes[i] & 0b1111_1000 == 0b1111_0000 {
                    4 // 4-byte UTF-8
                } else {
                    1 // Invalid, treat as single byte
                };

                i += char_len;
                if let Ok(ch) = std::str::from_utf8(&bytes[char_start..i]) {
                    buffer.push_str(ch);
                }
            }
        }

        // Add final segment if there's any text
        if !buffer.is_empty() {
            segments.push(Segment {
                range: pos..(pos + buffer.len()),
                buffer,
                style: current_style,
            });
        }

        Ok(StyledString { segments })
    }
}

/// Parse ANSI SGR (Select Graphic Rendition) codes and update the style
fn parse_ansi_codes(codes: &[&str], style: &mut AnsiSelectGraphicRendition) {
    let mut i = 0;
    while i < codes.len() {
        match codes[i].parse::<u8>() {
            Ok(0) => *style = AnsiSelectGraphicRendition::default(), // Reset

            // Intensity
            Ok(1) => style.intensity = Some(Intensity::Bold),
            Ok(2) => style.intensity = Some(Intensity::Dim),
            Ok(22) => style.intensity = Some(Intensity::Normal),

            // Italic
            Ok(3) => style.italic = Some(true),
            Ok(23) => style.italic = Some(false),

            // Underline
            Ok(4) => style.underline = Some(Underline::Single),
            Ok(21) => style.underline = Some(Underline::Double),
            Ok(24) => style.underline = Some(Underline::Disabled),

            // Blink
            Ok(5) => style.blink = Some(Blink::Slow),
            Ok(6) => style.blink = Some(Blink::Rapid),
            Ok(25) => style.blink = Some(Blink::Off),

            // Reverse
            Ok(7) => style.reverse = Some(true),
            Ok(27) => style.reverse = Some(false),

            // Hidden
            Ok(8) => style.hidden = Some(true),
            Ok(28) => style.hidden = Some(false),

            // Strike
            Ok(9) => style.strike = Some(true),
            Ok(29) => style.strike = Some(false),

            // Foreground colors (30-37, 90-97)
            Ok(30) => style.foreground = Some(Color::Black),
            Ok(31) => style.foreground = Some(Color::Red),
            Ok(32) => style.foreground = Some(Color::Green),
            Ok(33) => style.foreground = Some(Color::Yellow),
            Ok(34) => style.foreground = Some(Color::Blue),
            Ok(35) => style.foreground = Some(Color::Purple),
            Ok(36) => style.foreground = Some(Color::Cyan),
            Ok(37) => style.foreground = Some(Color::White),
            Ok(39) => style.foreground = None, // Default foreground

            Ok(90) => {
                style.intensity = Some(Intensity::Bold);
                style.foreground = Some(Color::Black)
            }
            Ok(91) => style.foreground = Some(Color::BrightRed),
            Ok(92) => style.foreground = Some(Color::BrightGreen),
            Ok(93) => style.foreground = Some(Color::BrightYellow),
            Ok(94) => style.foreground = Some(Color::BrightBlue),
            Ok(95) => style.foreground = Some(Color::BrightPurple),
            Ok(96) => style.foreground = Some(Color::BrightCyan),
            Ok(97) => style.foreground = Some(Color::BrightWhite),

            // Background colors (40-47, 100-107)
            Ok(40) => style.background = Some(Color::Black),
            Ok(41) => style.background = Some(Color::Red),
            Ok(42) => style.background = Some(Color::Green),
            Ok(43) => style.background = Some(Color::Yellow),
            Ok(44) => style.background = Some(Color::Blue),
            Ok(45) => style.background = Some(Color::Purple),
            Ok(46) => style.background = Some(Color::Cyan),
            Ok(47) => style.background = Some(Color::White),
            Ok(49) => style.background = None, // Default background

            Ok(100) => style.background = Some(Color::BrightBlack),
            Ok(101) => style.background = Some(Color::BrightRed),
            Ok(102) => style.background = Some(Color::BrightGreen),
            Ok(103) => style.background = Some(Color::BrightYellow),
            Ok(104) => style.background = Some(Color::BrightBlue),
            Ok(105) => style.background = Some(Color::BrightPurple),
            Ok(106) => style.background = Some(Color::BrightCyan),
            Ok(107) => style.background = Some(Color::BrightWhite),

            // 256-color mode: 38;5;n or 48;5;n
            Ok(38) if i + 2 < codes.len() && codes[i + 1] == "5" => {
                if let Ok(color_num) = codes[i + 2].parse::<u8>() {
                    style.foreground = Some(Color::Fixed(color_num));
                    i += 2; // Skip the next two parameters
                }
            }
            Ok(48) if i + 2 < codes.len() && codes[i + 1] == "5" => {
                if let Ok(color_num) = codes[i + 2].parse::<u8>() {
                    style.background = Some(Color::Fixed(color_num));
                    i += 2; // Skip the next two parameters
                }
            }

            // RGB color mode: 38;2;r;g;b or 48;2;r;g;b
            Ok(38) if i + 4 < codes.len() && codes[i + 1] == "2" => {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    codes[i + 2].parse::<u8>(),
                    codes[i + 3].parse::<u8>(),
                    codes[i + 4].parse::<u8>(),
                ) {
                    style.foreground = Some(Color::RGB(r, g, b));
                    i += 4; // Skip the next four parameters
                }
            }
            Ok(48) if i + 4 < codes.len() && codes[i + 1] == "2" => {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    codes[i + 2].parse::<u8>(),
                    codes[i + 3].parse::<u8>(),
                    codes[i + 4].parse::<u8>(),
                ) {
                    style.background = Some(Color::RGB(r, g, b));
                    i += 4; // Skip the next four parameters
                }
            }

            _ => {} // Ignore unknown codes
        }
        i += 1;
    }
}

impl Default for StyledString {
    fn default() -> Self {
        Self::empty()
    }
}

impl std::ops::Add for StyledString {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        let mut segments = Vec::new();
        segments.extend(self.segments);
        segments.extend(other.segments);
        Self { segments }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorMode;

    #[test]
    fn test_empty_styled_string() {
        let styled = StyledString::empty();
        assert_eq!(styled.stripped_len(), 0);
        assert_eq!(styled.segments.len(), 0);
    }

    #[test]
    fn test_from_string_no_style() {
        let styled = StyledString::from_string("Hello, World!", None);
        assert_eq!(styled.stripped_len(), 13);
        assert_eq!(styled.segments.len(), 1);
        assert_eq!(styled.segments[0].buffer, "Hello, World!");
        assert_eq!(
            styled.segments[0].style,
            AnsiSelectGraphicRendition::default()
        );
    }

    #[test]
    fn test_from_string_with_style() {
        let style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            foreground: Some(Color::Red),
            ..Default::default()
        };
        let styled = StyledString::from_string("Bold Red", Some(style.clone()));
        assert_eq!(styled.stripped_len(), 8);
        assert_eq!(styled.segments.len(), 1);
        assert_eq!(styled.segments[0].style, style);
    }

    #[test]
    fn test_concat_single_segment() {
        let mut styled = StyledString::empty();
        styled.concat("First");

        assert_eq!(styled.stripped_len(), 5);
        assert_eq!(styled.segments.len(), 1);
        assert_eq!(styled.segments[0].buffer, "First");
    }

    #[test]
    fn test_concat_multiple_segments() {
        let mut styled = StyledString::empty();
        styled.concat("Hello");
        styled.concat(" ");
        styled.concat_with_style(
            "World",
            AnsiSelectGraphicRendition {
                intensity: Some(Intensity::Bold),
                ..Default::default()
            },
        );

        assert_eq!(styled.stripped_len(), 11);
        assert_eq!(styled.segments.len(), 3);
        assert_eq!(styled.segments[0].buffer, "Hello");
        assert_eq!(styled.segments[1].buffer, " ");
        assert_eq!(styled.segments[2].buffer, "World");
        assert_eq!(styled.segments[2].style.intensity, Some(Intensity::Bold));
    }

    #[test]
    fn test_set_style_single_segment() {
        let mut styled = StyledString::from_string("Hello", None);
        let new_style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            foreground: Some(Color::Blue),
            ..Default::default()
        };

        styled.set_style(new_style.clone(), 0..5);

        assert_eq!(styled.segments.len(), 1);
        assert_eq!(styled.segments[0].style, new_style);
    }

    #[test]
    fn test_set_style_partial_segment() {
        let mut styled = StyledString::from_string("Hello World", None);
        let new_style = AnsiSelectGraphicRendition {
            underline: Some(Underline::Single),
            ..Default::default()
        };

        styled.set_style(new_style.clone(), 6..11);

        assert_eq!(styled.segments.len(), 2);
        assert_eq!(styled.segments[0].buffer, "Hello ");
        assert_eq!(
            styled.segments[0].style,
            AnsiSelectGraphicRendition::default()
        );
        assert_eq!(styled.segments[1].buffer, "World");
        assert_eq!(styled.segments[1].style, new_style);
    }

    #[test]
    fn test_set_style_middle_segment() {
        let mut styled = StyledString::from_string("Hello World!", None);
        let new_style = AnsiSelectGraphicRendition {
            italic: Some(true),
            ..Default::default()
        };

        styled.set_style(new_style.clone(), 6..11);

        assert_eq!(styled.segments.len(), 3);
        assert_eq!(styled.segments[0].buffer, "Hello ");
        assert_eq!(styled.segments[1].buffer, "World");
        assert_eq!(styled.segments[1].style, new_style);
        assert_eq!(styled.segments[2].buffer, "!");
    }

    #[test]
    fn test_set_style_spanning_multiple_segments() {
        let mut styled = StyledString::empty();
        styled.concat("Hello");
        styled.concat(" ");
        styled.concat("World");

        let new_style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            ..Default::default()
        };

        styled.set_style(new_style.clone(), 3..8);

        // Should split segments properly
        assert!(styled.segments.len() >= 3);

        // Verify the styled portion
        let mut found_styled = false;
        for segment in &styled.segments {
            if segment.buffer.contains("lo") || segment.buffer.contains("Wo") {
                assert_eq!(segment.style, new_style);
                found_styled = true;
            }
        }
        assert!(found_styled);
    }

    #[test]
    fn test_parse_empty_string() {
        let styled: StyledString = "".parse().unwrap();
        assert_eq!(styled.stripped_len(), 0);
        assert_eq!(styled.segments.len(), 0);
    }

    #[test]
    fn test_parse_plain_text() {
        let styled: StyledString = "Plain text".parse().unwrap();
        assert_eq!(styled.stripped_len(), 10);
        assert_eq!(styled.segments.len(), 1);
        assert_eq!(styled.segments[0].buffer, "Plain text");
        assert_eq!(
            styled.segments[0].style,
            AnsiSelectGraphicRendition::default()
        );
    }

    #[test]
    fn test_parse_bold_text() {
        let styled: StyledString = "\x1b[1mBold\x1b[0m".parse().unwrap();
        assert_eq!(styled.stripped_len(), 4);
        assert_eq!(styled.segments.len(), 1);
        assert_eq!(styled.segments[0].buffer, "Bold");
        assert_eq!(styled.segments[0].style.intensity, Some(Intensity::Bold));
    }

    #[test]
    fn test_parse_foreground_color() {
        let styled: StyledString = "\x1b[31mRed text\x1b[0m".parse().unwrap();
        assert_eq!(styled.segments[0].buffer, "Red text");
        assert_eq!(styled.segments[0].style.foreground, Some(Color::Red));
    }

    #[test]
    fn test_parse_background_color() {
        let styled: StyledString = "\x1b[44mBlue background\x1b[0m".parse().unwrap();
        assert_eq!(styled.segments[0].buffer, "Blue background");
        assert_eq!(styled.segments[0].style.background, Some(Color::Blue));
    }

    #[test]
    fn test_parse_all_basic_foreground_colors() {
        let colors = vec![
            ("30", Color::Black),
            ("31", Color::Red),
            ("32", Color::Green),
            ("33", Color::Yellow),
            ("34", Color::Blue),
            ("35", Color::Purple),
            ("36", Color::Cyan),
            ("37", Color::White),
        ];

        for (code, expected_color) in colors {
            let input = format!("\x1b[{}mText\x1b[0m", code);
            let styled: StyledString = input.parse().unwrap();
            assert_eq!(styled.segments[0].style.foreground, Some(expected_color));
        }
    }

    #[test]
    fn test_parse_all_basic_background_colors() {
        let colors = vec![
            ("40", Color::Black),
            ("41", Color::Red),
            ("42", Color::Green),
            ("43", Color::Yellow),
            ("44", Color::Blue),
            ("45", Color::Purple),
            ("46", Color::Cyan),
            ("47", Color::White),
        ];

        for (code, expected_color) in colors {
            let input = format!("\x1b[{}mText\x1b[0m", code);
            let styled: StyledString = input.parse().unwrap();
            assert_eq!(styled.segments[0].style.background, Some(expected_color));
        }
    }

    #[test]
    fn test_parse_combined_attributes() {
        let styled: StyledString = "\x1b[1;3;4mBold Italic Underline\x1b[0m".parse().unwrap();
        assert_eq!(styled.segments[0].style.underline, Some(Underline::Single));
        assert_eq!(styled.segments[0].style.intensity, Some(Intensity::Bold));
        assert_eq!(styled.segments[0].style.italic, Some(true));
    }

    #[test]
    fn test_parse_all_attributes() {
        let styled: StyledString = "\x1b[1;2;3;4;5;7;8;9mAll attributes\x1b[0m"
            .parse()
            .unwrap();
        let style = &styled.segments[0].style;

        // Check intensity (code 1 = Bold, but code 2 = Dim overwrites it)
        assert_eq!(style.intensity, Some(Intensity::Dim));

        // Check italic (code 3)
        assert_eq!(style.italic, Some(true));

        // Check underline (code 4)
        assert_eq!(style.underline, Some(Underline::Single));

        // Check blink (code 5)
        assert_eq!(style.blink, Some(Blink::Slow));

        // Check reverse (code 7)
        assert_eq!(style.reverse, Some(true));

        // Check hidden (code 8)
        assert_eq!(style.hidden, Some(true));

        // Check strike (code 9)
        assert_eq!(style.strike, Some(true));
    }

    #[test]
    fn test_parse_256_color_foreground() {
        let styled: StyledString = "\x1b[38;5;123mColor 123\x1b[0m".parse().unwrap();
        assert_eq!(styled.segments[0].style.foreground, Some(Color::Fixed(123)));
    }

    #[test]
    fn test_parse_256_color_background() {
        let styled: StyledString = "\x1b[48;5;200mColor 200\x1b[0m".parse().unwrap();
        assert_eq!(styled.segments[0].style.background, Some(Color::Fixed(200)));
    }

    #[test]
    fn test_parse_rgb_foreground() {
        let styled: StyledString = "\x1b[38;2;255;128;64mRGB\x1b[0m".parse().unwrap();
        assert_eq!(
            styled.segments[0].style.foreground,
            Some(Color::RGB(255, 128, 64))
        );
    }

    #[test]
    fn test_parse_rgb_background() {
        let styled: StyledString = "\x1b[48;2;10;20;30mRGB BG\x1b[0m".parse().unwrap();
        assert_eq!(
            styled.segments[0].style.background,
            Some(Color::RGB(10, 20, 30))
        );
    }

    #[test]
    fn test_parse_multiple_segments() {
        let styled: StyledString = "\x1b[31mRed\x1b[0m Normal \x1b[32mGreen\x1b[0m"
            .parse()
            .unwrap();
        assert_eq!(styled.segments.len(), 3);
        assert_eq!(styled.segments[0].buffer, "Red");
        assert_eq!(styled.segments[0].style.foreground, Some(Color::Red));
        assert_eq!(styled.segments[1].buffer, " Normal ");
        assert_eq!(
            styled.segments[1].style,
            AnsiSelectGraphicRendition::default()
        );
        assert_eq!(styled.segments[2].buffer, "Green");
        assert_eq!(styled.segments[2].style.foreground, Some(Color::Green));
    }

    #[test]
    fn test_parse_no_reset() {
        let styled: StyledString = "\x1b[1mBold text without reset".parse().unwrap();
        assert_eq!(styled.segments.len(), 1);
        assert_eq!(styled.segments[0].buffer, "Bold text without reset");
        assert_eq!(styled.segments[0].style.intensity, Some(Intensity::Bold));
    }

    #[test]
    fn test_parse_style_carries_over() {
        let styled: StyledString = "\x1b[1mBold \x1b[31mand red".parse().unwrap();
        assert_eq!(styled.segments.len(), 2);
        assert_eq!(styled.segments[1].style.intensity, Some(Intensity::Bold));
        assert_eq!(styled.segments[1].style.foreground, Some(Color::Red));
    }

    #[test]
    fn test_parse_reset_clears_style() {
        let styled: StyledString = "\x1b[1;31mBold Red\x1b[0mNormal".parse().unwrap();
        assert_eq!(styled.segments.len(), 2);
        assert_eq!(styled.segments[0].style.intensity, Some(Intensity::Bold));
        assert_eq!(styled.segments[0].style.foreground, Some(Color::Red));
        assert_eq!(
            styled.segments[1].style,
            AnsiSelectGraphicRendition::default()
        );
    }

    #[test]
    fn test_parse_color_reset_codes() {
        let styled: StyledString = "\x1b[31mRed\x1b[39mDefault FG\x1b[41mRed BG\x1b[49mDefault BG"
            .parse()
            .unwrap();

        // Find the segment after FG reset
        let mut found_fg_reset = false;
        let mut found_bg_reset = false;

        for segment in &styled.segments {
            if segment.buffer.contains("Default FG") {
                assert_eq!(segment.style.foreground, None);
                found_fg_reset = true;
            }
            if segment.buffer.contains("Default BG") {
                assert_eq!(segment.style.background, None);
                found_bg_reset = true;
            }
        }

        assert!(found_fg_reset);
        assert!(found_bg_reset);
    }

    #[test]
    fn test_parse_complex_sequence() {
        let input = "\x1b[1;3;38;2;255;100;50;48;5;234mComplex\x1b[0m";
        let styled: StyledString = input.parse().unwrap();

        let style = &styled.segments[0].style;
        assert_eq!(style.intensity, Some(Intensity::Bold));
        assert_eq!(style.italic, Some(true));
        assert_eq!(style.foreground, Some(Color::RGB(255, 100, 50)));
        assert_eq!(style.background, Some(Color::Fixed(234)));
    }

    #[test]
    fn test_write_str_empty() {
        let styled = StyledString::empty();
        let config = AnsiConfig {
            color_mode: ColorMode::None,
            ..Default::default()
        };
        let mut output = String::new();
        styled.write_str(&mut output, Some(&config)).unwrap();
        assert_eq!(output, "");
    }

    #[test]
    fn test_write_str_plain_text() {
        let styled = StyledString::from_string("Plain", None);
        let config = AnsiConfig {
            color_mode: ColorMode::None,
            ..Default::default()
        };
        let mut output = String::new();
        styled.write_str(&mut output, Some(&config)).unwrap();
        // Should contain the text (ANSI codes depend on ColorMode)
        assert!(output.contains("Plain"));
    }

    #[test]
    fn test_write_str_styled_text() {
        let style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            foreground: Some(Color::Red),
            ..Default::default()
        };
        let config = AnsiConfig {
            color_mode: ColorMode::Basic,
            ..Default::default()
        };
        let styled = StyledString::from_string("Bold Red", Some(style));
        let mut output = String::new();
        styled.write_str(&mut output, Some(&config)).unwrap();

        // Should contain ANSI codes and text
        assert!(output.contains("Bold Red"));
        assert!(output.contains("\x1b["));
    }

    #[test]
    fn test_clone() {
        let styled1 = StyledString::from_string(
            "Test",
            Some(AnsiSelectGraphicRendition {
                intensity: Some(Intensity::Bold),
                ..Default::default()
            }),
        );
        let styled2 = styled1.clone();

        assert_eq!(styled1, styled2);
    }

    #[test]
    fn test_equality() {
        let styled1 = StyledString::from_string("Test", None);
        let styled2 = StyledString::from_string("Test", None);
        let styled3 = StyledString::from_string("Different", None);

        assert_eq!(styled1, styled2);
        assert_ne!(styled1, styled3);
    }

    #[test]
    fn test_len_multiple_segments() {
        let mut styled = StyledString::empty();
        styled.concat("Hello");
        styled.concat(" ");
        styled.concat("World");

        assert_eq!(styled.stripped_len(), 11);
    }

    #[test]
    fn test_parse_empty_escape_sequence() {
        let styled: StyledString = "\x1b[mText\x1b[0m".parse().unwrap();
        assert_eq!(
            styled.segments[0].style,
            AnsiSelectGraphicRendition::default()
        );
    }

    #[test]
    fn test_parse_malformed_escape_sequence() {
        // Should handle gracefully
        let styled: StyledString = "\x1b[999mText".parse().unwrap();
        assert_eq!(styled.segments[0].buffer, "Text");
    }

    #[test]
    fn test_set_style_empty_range() {
        let mut styled = StyledString::from_string("Hello", None);
        let new_style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            ..Default::default()
        };

        styled.set_style(new_style, 0..0);

        // Should not crash and maintain original structure
        assert_eq!(styled.segments.len(), 1);
    }

    #[test]
    fn test_set_style_overlapping_ranges() {
        let mut styled = StyledString::from_string("Hello World", None);

        let style1 = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            ..Default::default()
        };
        let style2 = AnsiSelectGraphicRendition {
            underline: Some(Underline::Single),
            ..Default::default()
        };

        styled.set_style(style1, 0..5);
        styled.set_style(style2, 3..8);

        // The second style should override in the overlapping region
        assert!(
            styled
                .segments
                .iter()
                .any(|s| s.style.underline == Some(Underline::Single))
        );
    }

    // ============================================================================
    // Push Character Tests
    // ============================================================================

    #[test]
    fn test_push_to_empty_string() {
        let mut styled = StyledString::empty();

        styled.push('A');

        assert_eq!(styled.stripped_len(), 1);
        assert_eq!(styled.stripped(), "A");
        assert_eq!(styled.segments.len(), 1);
        assert_eq!(
            styled.segments[0].style,
            AnsiSelectGraphicRendition::default()
        );
    }

    #[test]
    fn test_push_multiple_characters() {
        let mut styled = StyledString::empty();

        styled.push('H');
        styled.push('e');
        styled.push('l');
        styled.push('l');
        styled.push('o');

        assert_eq!(styled.stripped_len(), 5);
        assert_eq!(styled.stripped(), "Hello");
        assert_eq!(styled.segments.len(), 1); // All in one segment
    }

    #[test]
    fn test_push_to_existing_segment() {
        let mut styled = StyledString::from_string("Hello", None);

        styled.push(' ');
        styled.push('W');
        styled.push('o');
        styled.push('r');
        styled.push('l');
        styled.push('d');

        assert_eq!(styled.stripped_len(), 11);
        assert_eq!(styled.stripped(), "Hello World");
        assert_eq!(styled.segments.len(), 1); // Still one segment
    }

    #[test]
    fn test_push_preserves_style() {
        let style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            foreground: Some(Color::Red),
            ..Default::default()
        };
        let mut styled = StyledString::from_string("Bold", Some(style.clone()));

        styled.push('!');

        assert_eq!(styled.stripped(), "Bold!");
        assert_eq!(styled.segments.len(), 1);
        assert_eq!(styled.segments[0].style, style);
    }

    #[test]
    fn test_push_unicode_characters() {
        let mut styled = StyledString::empty();

        styled.push('🦀'); // Rust crab emoji (4 bytes)
        styled.push('日'); // Japanese character (3 bytes)
        styled.push('€'); // Euro sign (3 bytes)

        assert_eq!(styled.stripped_len(), 10); // 4 + 3 + 3 bytes
        assert_eq!(styled.stripped(), "🦀日€");
        assert_eq!(styled.segments.len(), 1);
    }

    #[test]
    fn test_push_updates_range_correctly() {
        let mut styled = StyledString::empty();

        styled.push('A'); // 1 byte
        assert_eq!(styled.segments[0].range, 0..1);

        styled.push('B'); // 1 byte
        assert_eq!(styled.segments[0].range, 0..2);

        styled.push('🦀'); // 4 bytes
        assert_eq!(styled.segments[0].range, 0..6);
    }

    #[test]
    fn test_push_with_multiple_segments() {
        let mut styled = StyledString::empty();
        styled.concat("Normal ");
        styled.concat_with_style(
            "Bold",
            AnsiSelectGraphicRendition {
                intensity: Some(Intensity::Bold),
                ..Default::default()
            },
        );

        let initial_len = styled.stripped_len();
        let segment_count = styled.segments.len();

        styled.push('!');

        // Character is added to last segment
        assert_eq!(styled.stripped_len(), initial_len + 1);
        assert_eq!(styled.segments.len(), segment_count); // No new segment
        assert_eq!(styled.stripped(), "Normal Bold!");
    }

    #[test]
    fn test_push_special_characters() {
        let mut styled = StyledString::empty();

        styled.push('\t');
        styled.push('\n');
        styled.push(' ');

        assert_eq!(styled.stripped_len(), 3);
        assert_eq!(styled.stripped(), "\t\n ");
    }

    #[test]
    fn test_push_ascii_characters() {
        let mut styled = StyledString::empty();

        for ch in b'A'..=b'Z' {
            styled.push(ch as char);
        }

        assert_eq!(styled.stripped_len(), 26);
        assert_eq!(styled.stripped(), "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        assert_eq!(styled.segments.len(), 1);
    }

    #[test]
    fn test_push_after_set_style() {
        let mut styled = StyledString::from_string("Hello World", None);

        styled.set_style(
            AnsiSelectGraphicRendition {
                intensity: Some(Intensity::Bold),
                ..Default::default()
            },
            6..11,
        );

        styled.push('!');

        assert_eq!(styled.stripped(), "Hello World!");
        // Character is added to the last segment
        assert!(styled.segments.last().unwrap().buffer.ends_with('!'));
    }

    #[test]
    fn test_push_zero_width_characters() {
        let mut styled = StyledString::empty();

        styled.push('a');
        styled.push('\u{200B}'); // Zero-width space (3 bytes in UTF-8)
        styled.push('b');

        assert_eq!(styled.stripped_len(), 5); // 1 + 3 + 1
        assert_eq!(styled.stripped(), "a\u{200B}b");
    }

    #[test]
    fn test_push_maintains_segment_ordering() {
        let mut styled = StyledString::empty();
        styled.concat_with_style(
            "Red",
            AnsiSelectGraphicRendition {
                foreground: Some(Color::Red),
                ..Default::default()
            },
        );
        styled.concat_with_style(
            "Blue",
            AnsiSelectGraphicRendition {
                foreground: Some(Color::Blue),
                ..Default::default()
            },
        );

        styled.push('X');

        // Verify segments are in order
        for i in 1..styled.segments.len() {
            assert!(styled.segments[i - 1].range.start <= styled.segments[i].range.start);
        }
    }

    #[test]
    fn test_push_write_str_output() {
        let mut styled = StyledString::empty();
        styled.push('T');
        styled.push('e');
        styled.push('s');
        styled.push('t');

        let config = AnsiConfig {
            color_mode: ColorMode::None,
            ..Default::default()
        };
        let mut output = String::new();
        styled.write_str(&mut output, Some(&config)).unwrap();

        assert!(output.contains("Test"));
    }
}
