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

use termionix_ansicodec::{Segment, SegmentedString};

/// Intelligent Word-wrap text to a specified width, preserving ANSI escape sequences.
///
/// This function wraps text to fit within the specified width while:
/// - Preserving ANSI escape sequences (colors, styles, etc.)
/// - Respecting word boundaries when possible
/// - Preserving double spaces and indentation
/// - Breaking long words that exceed the width
/// - Handling paragraphs (double newlines)
/// - Maintaining visual consistency
///
/// # Arguments
///
/// * `text` - The input text to wrap (may contain ANSI escape sequences)
/// * `width` - The maximum width in characters for each line
///
/// # Returns
///
/// A `SegmentedString` containing the wrapped text with preserved ANSI sequences
///
/// # Examples
///
/// ```rust
/// use termionix_terminal::terminal_word_wrap;
///
/// let text = "This is a long line that needs to be wrapped to fit within a specific width.";
/// let wrapped = terminal_word_wrap(text, 20);
/// ```
pub fn terminal_word_wrap(text: &str, width: usize) -> SegmentedString {
    if width == 0 {
        return SegmentedString::empty();
    }

    // Parse the input text into segments
    let input = SegmentedString::parse(text);
    let mut output = SegmentedString::empty();

    // Track current line state
    let mut current_line_width = 0;
    let mut word_buffer = SegmentedString::empty();
    let mut word_width = 0;
    let mut active_styles: Vec<Segment> = Vec::new(); // Stack of active style segments
    let mut line_indent = 0;
    let mut preserve_indent = false;
    let mut last_was_newline = false;

    for segment in input.segments() {
        match segment {
            // Handle text content
            Segment::ASCII(s) | Segment::Unicode(s) => {
                for ch in s.chars() {
                    match ch {
                        // Newline handling
                        '\n' => {
                            // Flush word buffer
                            if word_width > 0 {
                                if current_line_width + word_width > width && current_line_width > 0
                                {
                                    output.push_char('\n');
                                    // Re-apply active styles after newline
                                    for style in &active_styles {
                                        output.push_segment(style.clone());
                                    }
                                    // Apply indentation if needed
                                    if preserve_indent && line_indent > 0 {
                                        for _ in 0..line_indent {
                                            output.push_char(' ');
                                        }
                                        current_line_width = line_indent;
                                    } else {
                                        current_line_width = 0;
                                    }
                                }

                                // Add word to output
                                for seg in word_buffer.segments() {
                                    output.push_segment(seg.clone());
                                }
                                current_line_width += word_width;
                                word_buffer = SegmentedString::empty();
                                word_width = 0;
                            }

                            output.push_char('\n');

                            // Check for paragraph break (double newline)
                            if last_was_newline {
                                preserve_indent = false;
                                line_indent = 0;
                            }

                            current_line_width = 0;
                            last_was_newline = true;
                            continue;
                        }

                        // Space handling
                        ' ' => {
                            last_was_newline = false;

                            // Flush word buffer if we have one
                            if word_width > 0 {
                                // Check if word fits on current line
                                if current_line_width + word_width > width && current_line_width > 0
                                {
                                    // Word doesn't fit, wrap to next line
                                    output.push_char('\n');
                                    // Re-apply active styles after newline
                                    for style in &active_styles {
                                        output.push_segment(style.clone());
                                    }
                                    // Apply indentation if needed
                                    if preserve_indent && line_indent > 0 {
                                        for _ in 0..line_indent {
                                            output.push_char(' ');
                                        }
                                        current_line_width = line_indent;
                                    } else {
                                        current_line_width = 0;
                                    }
                                }

                                // Add word to output
                                for seg in word_buffer.segments() {
                                    output.push_segment(seg.clone());
                                }
                                current_line_width += word_width;
                                word_buffer = SegmentedString::empty();
                                word_width = 0;
                            }

                            // Handle space
                            if current_line_width == 0 {
                                // Leading space - track as indentation
                                if !preserve_indent {
                                    line_indent += 1;
                                }
                                output.push_char(' ');
                                current_line_width += 1;
                            } else if current_line_width < width {
                                // Space fits on current line
                                output.push_char(' ');
                                current_line_width += 1;
                                preserve_indent = true;
                            }
                            // If space would exceed width, skip it (it's at line break)
                        }

                        // Regular character
                        _ => {
                            last_was_newline = false;

                            // Add character to word buffer
                            word_buffer.push_char(ch);
                            word_width += 1;

                            // Check if word is too long and needs breaking
                            if word_width > width {
                                // Flush what we have so far
                                if current_line_width > 0 {
                                    output.push_char('\n');
                                    // Re-apply active styles after newline
                                    for style in &active_styles {
                                        output.push_segment(style.clone());
                                    }
                                    current_line_width = 0;
                                }

                                // Break the long word into chunks
                                let mut chars_to_output = Vec::new();
                                for seg in word_buffer.segments() {
                                    if let Segment::ASCII(s) | Segment::Unicode(s) = seg {
                                        chars_to_output.extend(s.chars());
                                    }
                                }

                                word_buffer = SegmentedString::empty();
                                word_width = 0;

                                for ch in chars_to_output {
                                    if current_line_width >= width {
                                        output.push_char('\n');
                                        // Re-apply active styles after newline
                                        for style in &active_styles {
                                            output.push_segment(style.clone());
                                        }
                                        current_line_width = 0;
                                    }
                                    output.push_char(ch);
                                    current_line_width += 1;
                                }
                            }
                        }
                    }
                }
            }

            // Handle control codes
            Segment::Control(_ctrl) => {
                // Add control to word buffer (it doesn't affect width)
                word_buffer.push_segment(segment.clone());
            }

            // Handle style segments (SGR, CSI, etc.)
            Segment::SGR(_sgr) => {
                // Track active styles
                active_styles.push(segment.clone());
                // Add to word buffer
                word_buffer.push_segment(segment.clone());
            }

            // Handle other ANSI sequences
            Segment::Escape
            | Segment::CSI(_)
            | Segment::OSC(_)
            | Segment::DCS(_)
            | Segment::SOS(_)
            | Segment::ST
            | Segment::PM(_)
            | Segment::APC(_)
            | Segment::TelnetCommand(_) => {
                // Add to word buffer (doesn't affect width)
                word_buffer.push_segment(segment.clone());
            }
        }
    }

    // Flush any remaining word buffer
    if word_width > 0 {
        if current_line_width + word_width > width && current_line_width > 0 {
            output.push_char('\n');
            // Re-apply active styles after newline
            for style in &active_styles {
                output.push_segment(style.clone());
            }
        }

        for seg in word_buffer.segments() {
            output.push_segment(seg.clone());
        }
    }

    output
}

/// Remove excess line breaks while preserving paragraph breaks and ANSI escape sequences.
///
/// This function unwraps text by removing single line breaks (soft wraps) while preserving
/// double line breaks (paragraph breaks). It maintains:
/// - ANSI escape sequences (colors, styles, etc.)
/// - Paragraph structure (double newlines)
/// - Proper spacing between words
/// - Active styles across unwrapped lines
///
/// # Arguments
///
/// * `text` - The input text to unwrap (may contain ANSI escape sequences)
///
/// # Returns
///
/// A `SegmentedString` containing the unwrapped text with preserved ANSI sequences
///
/// # Examples
///
/// ```rust
/// use termionix_terminal::terminal_word_unwrap;
///
/// let text = "This is a line\nthat was wrapped\n\nThis is a new paragraph";
/// let unwrapped = terminal_word_unwrap(text);
/// // Result: "This is a line that was wrapped\n\nThis is a new paragraph"
/// ```
pub fn terminal_word_unwrap(text: &str) -> SegmentedString {
    // Parse the input text into segments
    let input = SegmentedString::parse(text);
    let mut output = SegmentedString::empty();

    // Track state
    let mut last_was_newline = false;
    let mut last_was_space = false;
    let mut pending_newline = false;
    let mut at_line_start = true;

    for segment in input.segments() {
        match segment {
            // Handle text content
            Segment::ASCII(s) | Segment::Unicode(s) => {
                for ch in s.chars() {
                    match ch {
                        '\n' => {
                            if last_was_newline {
                                // Double newline - preserve as paragraph break
                                if pending_newline {
                                    output.push_char('\n');
                                    pending_newline = false;
                                }
                                output.push_char('\n');
                                output.push_char('\n');
                                at_line_start = true;
                                last_was_space = false;
                            } else {
                                // Single newline - mark as pending (might be soft wrap)
                                pending_newline = true;
                            }
                            last_was_newline = true;
                        }

                        ' ' => {
                            // Skip leading spaces at line start
                            if at_line_start {
                                continue;
                            }

                            // If we have a pending newline, convert it to a space
                            if pending_newline {
                                if !last_was_space {
                                    output.push_char(' ');
                                    last_was_space = true;
                                }
                                pending_newline = false;
                            } else if !last_was_space {
                                // Add space if we don't already have one
                                output.push_char(' ');
                                last_was_space = true;
                            }
                            last_was_newline = false;
                        }

                        // Regular character
                        _ => {
                            // If we have a pending newline, convert it to a space
                            if pending_newline {
                                if !last_was_space && !at_line_start {
                                    output.push_char(' ');
                                }
                                pending_newline = false;
                            }

                            output.push_char(ch);
                            at_line_start = false;
                            last_was_space = false;
                            last_was_newline = false;
                        }
                    }
                }
            }

            // Handle control codes (preserve them)
            Segment::Control(_ctrl) => {
                output.push_segment(segment.clone());
            }

            // Handle style segments and other ANSI sequences (preserve them)
            Segment::SGR(_)
            | Segment::Escape
            | Segment::CSI(_)
            | Segment::OSC(_)
            | Segment::DCS(_)
            | Segment::SOS(_)
            | Segment::ST
            | Segment::PM(_)
            | Segment::APC(_)
            | Segment::TelnetCommand(_) => {
                output.push_segment(segment.clone());
            }
        }
    }

    // If we have a pending newline at the end, preserve it
    if pending_newline {
        output.push_char('\n');
    }

    output
}
