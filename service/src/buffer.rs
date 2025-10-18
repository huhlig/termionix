//
// Copyright 2019-2025 Hans W. Uhlig. All Rights Reserved.
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

use termionix_ansicodes::{
    AnsiMapper, AnsiMapperResult, ControlCode, Segment, SegmentedString, StyledString,
};
use termionix_codec::{TelnetArgument, TelnetFrame, TelnetOption};
use tracing::trace;

/// terminal size tracking, line management, and character/line erasures.
pub struct TerminalBuffer {
    /// Terminal dimensions (width, height)
    size: (usize, usize),
    /// Current cursor position (column, row) - 0-indexed
    cursor: (usize, usize),
    /// Completed lines
    completed_lines: Vec<SegmentedString>,
    /// The current line buffer (data being typed)
    current_line: SegmentedString,
    /// Raw byte buffer for data that hasn't been processed yet
    mapper: AnsiMapper,
    /// Whether to keep ANSI codes when processing text
    ansi: bool,
}

impl TerminalBuffer {
    /// Creates a new terminal buffer with a default size of (80x24)
    pub fn new() -> Self {
        Self::new_with_size(80, 24)
    }

    /// Creates a new terminal buffer with specified dimensions
    pub fn new_with_size(width: usize, height: usize) -> Self {
        TerminalBuffer {
            size: (width, height),
            cursor: (0, 0),
            current_line: SegmentedString::empty(),
            completed_lines: Vec::new(),
            mapper: AnsiMapper::default(),
            ansi: true,
        }
    }

    // ===== Terminal Size Management =====

    /// Sets the terminal size
    pub fn set_size(&mut self, width: usize, height: usize) {
        self.size = (width, height);
        // Adjust cursor if it's now out of bounds
        if self.cursor.0 >= width {
            self.cursor.0 = width.saturating_sub(1);
        }
        if self.cursor.1 >= height {
            self.cursor.1 = height.saturating_sub(1);
        }
    }

    /// Gets the current terminal size
    pub fn size(&self) -> (usize, usize) {
        self.size
    }

    /// Gets the terminal width
    pub fn width(&self) -> usize {
        self.size.0
    }

    /// Gets the terminal height
    pub fn height(&self) -> usize {
        self.size.1
    }

    // ===== Cursor Management =====

    /// Gets the current cursor position
    pub fn cursor_position(&self) -> (usize, usize) {
        self.cursor
    }

    /// Sets the cursor position (clamped to terminal bounds)
    pub fn set_cursor_position(&mut self, col: usize, row: usize) {
        self.cursor = (
            col.min(self.size.0.saturating_sub(1)),
            row.min(self.size.1.saturating_sub(1)),
        );
    }

    // ===== Ansi-Code API =====

    /// Enables or disables ANSI code stripping
    pub fn set_ansi_status(&mut self, ansi: bool) {
        if ansi {
            self.ansi = true;
        } else {
            // TODO: Strip Ansi Codes out of existing lines
            self.ansi = false;
        }
    }

    /// Returns whether ANSI code stripping is enabled
    pub fn is_ansi_enabled(&self) -> bool {
        self.ansi
    }

    // ===== Character-level API =====

    /// Adds a single character to the current line buffer
    pub fn push_byte(&mut self, byte: u8) {
        match self.mapper.next(byte) {
            AnsiMapperResult::Incomplete => {
                // Need more bytes to complete the sequence
            }
            AnsiMapperResult::Character(ch) => {
                // Regular ASCII character
                self.current_line.push_char(ch);
                self.advance_cursor_by_one();
            }
            AnsiMapperResult::Unicode(ch) => {
                // Unicode character
                self.current_line.push_char(ch);
                self.advance_cursor_by_one();
            }
            AnsiMapperResult::Control(ctrl) => {
                // Handle control codes
                match ctrl {
                    ControlCode::LF => {
                        // Line feed: complete current line
                        self.complete_line();
                    }
                    ControlCode::CR => {
                        // Carriage return: move cursor to start of line
                        self.cursor.0 = 0;
                    }
                    ControlCode::BS => {
                        // Backspace: erase the last character
                        self.erase_character();
                    }
                    _ => {
                        // Other control codes: add to the current line if ansi enabled
                        if self.ansi {
                            self.current_line.push_control(ctrl);
                        }
                    }
                }
            }
            AnsiMapperResult::Escape => {
                // Standalone escape character
                if self.ansi {
                    self.current_line.push_segment(Segment::Escape);
                }
            }
            AnsiMapperResult::CSI(cmd) => {
                // CSI command (cursor movement, erasing, etc.)
                if self.ansi {
                    self.current_line.push_segment(Segment::CSI(cmd));
                }
            }
            AnsiMapperResult::SGR(style) => {
                // Style/color change
                if self.ansi {
                    self.current_line.push_style(style);
                }
            }
            AnsiMapperResult::OSC(data) => {
                // Operating System Command
                if self.ansi {
                    self.current_line.push_segment(Segment::OSC(data));
                }
            }
            AnsiMapperResult::DCS(data) => {
                // Device Control String
                if self.ansi {
                    self.current_line.push_segment(Segment::DCS(data));
                }
            }
            AnsiMapperResult::SOS(data) => {
                // Start of String
                if self.ansi {
                    self.current_line.push_segment(Segment::SOS(data));
                }
            }
            AnsiMapperResult::ST(data) => {
                // String Terminator
                if self.ansi {
                    self.current_line.push_segment(Segment::ST(data));
                }
            }
            AnsiMapperResult::PM(data) => {
                // Privacy Message
                if self.ansi {
                    self.current_line.push_segment(Segment::PM(data));
                }
            }
            AnsiMapperResult::APC(data) => {
                // Application Program Command
                if self.ansi {
                    self.current_line.push_segment(Segment::APC(data));
                }
            }
        }
    }

    /// Advance the cursor by one column, clamping to terminal bounds.
    /// If at the right-most column, do not increment past width - 1.
    fn advance_cursor_by_one(&mut self) {
        // If width is zero, stay at column 0
        if self.size.0 == 0 {
            self.cursor.0 = 0;
            return;
        }

        // Only advance if we're not at the right-most column
        let max_col = self.size.0.saturating_sub(1);
        if self.cursor.0 < max_col {
            self.cursor.0 += 1;
        } else {
            // At right-most column: keep column at max, but if not at bottom row, allow moving to next row on line completion only.
            self.cursor.0 = max_col;
        }
    }

    /// Erases the last character from the current line buffer
    pub fn erase_character(&mut self) {
        if self.current_line.pop().is_some() {
            if self.cursor.0 > 0 {
                self.cursor.0 -= 1;
            } else if self.cursor.1 > 0 {
                // Wrapped to previous line
                self.cursor.1 -= 1;
                self.cursor.0 = self.size.0.saturating_sub(1);
            }
        }
    }

    /// Gets the current character count in the current line
    pub fn current_line_length(&self) -> usize {
        self.current_line.stripped_len()
    }

    /// Gets a reference to the current line being typed
    pub fn current_line(&self) -> &SegmentedString {
        &self.current_line
    }

    /// Checks if the current line buffer is empty
    pub fn is_current_line_empty(&self) -> bool {
        self.current_line.is_empty()
    }

    // ===== Line-level API =====

    /// Completes the current line and adds it to completed lines
    pub fn complete_line(&mut self) {
        self.completed_lines
            .push(std::mem::take(&mut self.current_line));
        self.cursor.0 = 0;
        self.cursor.1 = (self.cursor.1 + 1).min(self.size.1.saturating_sub(1));
    }

    /// Erases the entire current line
    pub fn erase_line(&mut self) {
        self.current_line.clear();
        self.cursor.0 = 0;
    }

    /// Gets the number of completed lines
    pub fn completed_line_count(&self) -> usize {
        self.completed_lines.len()
    }

    /// Gets a reference to all completed lines
    pub fn completed_lines(&self) -> &[SegmentedString] {
        &self.completed_lines
    }

    /// Pops the oldest-completed line
    pub fn pop_completed_line(&mut self) -> Option<SegmentedString> {
        if self.completed_lines.is_empty() {
            None
        } else {
            Some(self.completed_lines.remove(0))
        }
    }

    /// Takes all completed lines, leaving the buffer empty
    pub fn take_completed_lines(&mut self) -> Vec<SegmentedString> {
        std::mem::take(&mut self.completed_lines)
    }

    /// Clears all completed lines
    pub fn clear_completed_lines(&mut self) {
        self.completed_lines.clear();
    }

    /// Appends a pre-formed line to the completed lines (useful for echoing)
    /// TODO: Remove Expect
    pub fn append_line(&mut self, line: String) {
        self.completed_lines
            .push(SegmentedString::parse(line.as_str()));
    }

    /// Appends a pre-formed line to the completed lines (useful for echoing)
    pub fn append_styled_line(&mut self, line: StyledString) {
        self.completed_lines.push(line.segmented());
    }

    /// Gets the current line with ANSI codes optionally stripped
    pub fn current_line_stripped(&self) -> String {
        self.current_line.stripped()
    }

    /// Gets completed lines with ANSI codes optionally stripped
    pub fn completed_lines_stripped(&self) -> Vec<String> {
        self.completed_lines
            .iter()
            .map(|line| line.stripped())
            .collect()
    }

    // ===== Buffer Management =====

    /// Clears the entire buffer (current line and completed lines)
    pub fn clear(&mut self) {
        self.current_line.clear();
        self.completed_lines.clear();
        self.cursor = (0, 0);
        self.mapper.clear();
    }

    /// Gets the total line count (completed + current if non-empty)
    pub fn total_line_count(&self) -> usize {
        let current = if self.current_line.is_empty() { 0 } else { 1 };
        self.completed_lines.len() + current
    }

    // ===== Telnet Event Handling =====

    /// Handles a Telnet frame event
    pub fn handle_event(&mut self, event: TelnetFrame) {
        match &event {
            TelnetFrame::Data(byte) => {
                self.mapper.next(*byte);
            }
            TelnetFrame::Line(line) => {
                self.append_line(line.clone());
            }
            TelnetFrame::EraseCharacter => {
                self.erase_character();
            }
            TelnetFrame::EraseLine => {
                self.erase_line();
            }
            TelnetFrame::NoOperation => {}
            TelnetFrame::DataMark => {}
            TelnetFrame::Break => {
                // Could interrupt the current line
                self.erase_line();
            }
            TelnetFrame::InterruptProcess => {
                // Clear current input
                self.erase_line();
            }
            TelnetFrame::AbortOutput => {}
            TelnetFrame::AreYouThere => {}
            TelnetFrame::GoAhead => {}
            TelnetFrame::Do(_) => {}
            TelnetFrame::Dont(_) => {}
            TelnetFrame::Will(_) => {}
            TelnetFrame::Wont(_) => {}
            TelnetFrame::Subnegotiate(option, argument) => {
                match (option, argument) {
                    // Handle NAWS (Negotiate About Window Size) - Option 31
                    (TelnetOption::NAWS, TelnetArgument::NAWSWindowSize(size)) => {
                        // Only update if dimensions are reasonable (not zero)
                        if size.cols > 0 && size.rows > 0 {
                            self.set_size(size.cols as usize, size.rows as usize);
                        }
                    }
                    (_, _) => {
                        trace!("Unknown Subnegotiation ({:?}, {:?})", option, argument);
                    }
                }
            }
        }
    }
}

impl Default for TerminalBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Constructor Tests =====

    #[test]
    fn test_new_creates_default_buffer() {
        let buffer = TerminalBuffer::new();
        assert_eq!(buffer.size(), (80, 24));
        assert_eq!(buffer.cursor_position(), (0, 0));
        assert!(buffer.is_current_line_empty());
        assert_eq!(buffer.completed_line_count(), 0);
        assert!(buffer.is_ansi_enabled());
    }

    #[test]
    fn test_new_with_size() {
        let buffer = TerminalBuffer::new_with_size(120, 40);
        assert_eq!(buffer.size(), (120, 40));
        assert_eq!(buffer.width(), 120);
        assert_eq!(buffer.height(), 40);
    }

    #[test]
    fn test_default_trait() {
        let buffer = TerminalBuffer::default();
        assert_eq!(buffer.size(), (80, 24));
    }

    // ===== Terminal Size Management Tests =====

    #[test]
    fn test_set_size() {
        let mut buffer = TerminalBuffer::new();
        buffer.set_size(100, 30);
        assert_eq!(buffer.size(), (100, 30));
        assert_eq!(buffer.width(), 100);
        assert_eq!(buffer.height(), 30);
    }

    #[test]
    fn test_set_size_clamps_cursor() {
        let mut buffer = TerminalBuffer::new_with_size(80, 24);
        buffer.set_cursor_position(75, 20);

        // Reduce size so cursor is out of bounds
        buffer.set_size(50, 15);

        let (col, row) = buffer.cursor_position();
        assert!(col < 50);
        assert!(row < 15);
    }

    #[test]
    fn test_set_size_with_cursor_at_edge() {
        let mut buffer = TerminalBuffer::new_with_size(80, 24);
        buffer.set_cursor_position(79, 23);

        // Reduce to smaller size
        buffer.set_size(40, 12);

        assert_eq!(buffer.cursor_position(), (39, 11));
    }

    // ===== Cursor Management Tests =====

    #[test]
    fn test_set_cursor_position() {
        let mut buffer = TerminalBuffer::new();
        buffer.set_cursor_position(10, 5);
        assert_eq!(buffer.cursor_position(), (10, 5));
    }

    #[test]
    fn test_cursor_clamped_to_bounds() {
        let mut buffer = TerminalBuffer::new_with_size(80, 24);
        buffer.set_cursor_position(100, 50);
        assert_eq!(buffer.cursor_position(), (79, 23));
    }

    #[test]
    fn test_cursor_moves_with_characters() {
        let mut buffer = TerminalBuffer::new();
        buffer.push_byte(b'H');
        buffer.push_byte(b'i');
        assert_eq!(buffer.cursor_position(), (2, 0));
    }

    #[test]
    fn test_cursor_resets_on_carriage_return() {
        let mut buffer = TerminalBuffer::new();
        buffer.push_byte(b'H');
        buffer.push_byte(b'i');
        buffer.push_byte(b'\r');
        assert_eq!(buffer.cursor_position(), (0, 0));
    }

    #[test]
    fn test_cursor_advances_on_line_feed() {
        let mut buffer = TerminalBuffer::new();
        buffer.push_byte(b'H');
        buffer.push_byte(b'i');
        buffer.push_byte(b'\n');
        assert_eq!(buffer.cursor_position(), (0, 1));
    }

    // ===== ANSI Status Tests =====

    #[test]
    fn test_ansi_enabled_by_default() {
        let buffer = TerminalBuffer::new();
        assert!(buffer.is_ansi_enabled());
    }

    #[test]
    fn test_set_ansi_status_enable() {
        let mut buffer = TerminalBuffer::new();
        buffer.set_ansi_status(true);
        assert!(buffer.is_ansi_enabled());
    }

    #[test]
    fn test_set_ansi_status_disable() {
        let mut buffer = TerminalBuffer::new();
        buffer.set_ansi_status(false);
        assert!(!buffer.is_ansi_enabled());
    }

    // ===== Character-level API Tests =====

    #[test]
    fn test_push_byte_ascii() {
        let mut buffer = TerminalBuffer::new();
        buffer.push_byte(b'A');
        assert_eq!(buffer.current_line_length(), 1);
        assert_eq!(buffer.current_line_stripped(), "A");
    }

    #[test]
    fn test_push_byte_multiple_characters() {
        let mut buffer = TerminalBuffer::new();
        for byte in b"Hello" {
            buffer.push_byte(*byte);
        }
        assert_eq!(buffer.current_line_length(), 5);
        assert_eq!(buffer.current_line_stripped(), "Hello");
    }

    #[test]
    fn test_push_byte_utf8_multibyte() {
        let mut buffer = TerminalBuffer::new();
        // Push UTF-8 bytes for "café" (é is 2 bytes in UTF-8)
        for byte in "café".as_bytes() {
            buffer.push_byte(*byte);
        }
        assert_eq!(buffer.current_line_stripped(), "café");
    }

    #[test]
    fn test_push_byte_unicode_emoji() {
        let mut buffer = TerminalBuffer::new();
        for byte in "Hello 👋".as_bytes() {
            buffer.push_byte(*byte);
        }
        assert_eq!(buffer.current_line_stripped(), "Hello 👋");
    }

    #[test]
    fn test_erase_character() {
        let mut buffer = TerminalBuffer::new();
        buffer.push_byte(b'A');
        buffer.push_byte(b'B');
        buffer.erase_character();
        assert_eq!(buffer.current_line_length(), 1);
        assert_eq!(buffer.current_line_stripped(), "A");
    }

    #[test]
    fn test_erase_character_empty_buffer() {
        let mut buffer = TerminalBuffer::new();
        buffer.erase_character();
        assert_eq!(buffer.current_line_length(), 0);
        assert!(buffer.is_current_line_empty());
    }

    #[test]
    fn test_erase_character_moves_cursor_back() {
        let mut buffer = TerminalBuffer::new();
        buffer.push_byte(b'A');
        buffer.push_byte(b'B');
        buffer.push_byte(b'C');
        assert_eq!(buffer.cursor_position(), (3, 0));

        buffer.erase_character();
        assert_eq!(buffer.cursor_position(), (2, 0));
    }

    #[test]
    fn test_backspace_control_code() {
        let mut buffer = TerminalBuffer::new();
        buffer.push_byte(b'A');
        buffer.push_byte(b'B');
        buffer.push_byte(b'\x08'); // Backspace
        assert_eq!(buffer.current_line_stripped(), "A");
    }

    #[test]
    fn test_current_line_length() {
        let mut buffer = TerminalBuffer::new();
        assert_eq!(buffer.current_line_length(), 0);

        for byte in b"Test" {
            buffer.push_byte(*byte);
        }
        assert_eq!(buffer.current_line_length(), 4);
    }

    #[test]
    fn test_is_current_line_empty() {
        let mut buffer = TerminalBuffer::new();
        assert!(buffer.is_current_line_empty());

        buffer.push_byte(b'X');
        assert!(!buffer.is_current_line_empty());

        buffer.erase_character();
        assert!(buffer.is_current_line_empty());
    }

    // ===== Line-level API Tests =====

    #[test]
    fn test_complete_line() {
        let mut buffer = TerminalBuffer::new();
        for byte in b"First line" {
            buffer.push_byte(*byte);
        }
        buffer.complete_line();

        assert_eq!(buffer.completed_line_count(), 1);
        assert!(buffer.is_current_line_empty());
        assert_eq!(buffer.cursor_position(), (0, 1));
    }

    #[test]
    fn test_complete_multiple_lines() {
        let mut buffer = TerminalBuffer::new();

        for byte in b"Line 1" {
            buffer.push_byte(*byte);
        }
        buffer.complete_line();

        for byte in b"Line 2" {
            buffer.push_byte(*byte);
        }
        buffer.complete_line();

        assert_eq!(buffer.completed_line_count(), 2);
    }

    #[test]
    fn test_line_feed_completes_line() {
        let mut buffer = TerminalBuffer::new();
        for byte in b"Test\n" {
            buffer.push_byte(*byte);
        }

        assert_eq!(buffer.completed_line_count(), 1);
        assert!(buffer.is_current_line_empty());
    }

    #[test]
    fn test_erase_line() {
        let mut buffer = TerminalBuffer::new();
        for byte in b"Some text" {
            buffer.push_byte(*byte);
        }
        buffer.erase_line();

        assert!(buffer.is_current_line_empty());
        assert_eq!(buffer.cursor_position(), (0, 0));
    }

    #[test]
    fn test_completed_lines() {
        let mut buffer = TerminalBuffer::new();

        for byte in b"Line 1\n" {
            buffer.push_byte(*byte);
        }
        for byte in b"Line 2\n" {
            buffer.push_byte(*byte);
        }

        let lines = buffer.completed_lines();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_pop_completed_line() {
        let mut buffer = TerminalBuffer::new();

        for byte in b"First\n" {
            buffer.push_byte(*byte);
        }
        for byte in b"Second\n" {
            buffer.push_byte(*byte);
        }

        let line = buffer.pop_completed_line();
        assert!(line.is_some());
        assert_eq!(buffer.completed_line_count(), 1);
    }

    #[test]
    fn test_pop_completed_line_empty() {
        let mut buffer = TerminalBuffer::new();
        let line = buffer.pop_completed_line();
        assert!(line.is_none());
    }

    #[test]
    fn test_take_completed_lines() {
        let mut buffer = TerminalBuffer::new();

        for byte in b"Line 1\n" {
            buffer.push_byte(*byte);
        }
        for byte in b"Line 2\n" {
            buffer.push_byte(*byte);
        }

        let lines = buffer.take_completed_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(buffer.completed_line_count(), 0);
    }

    #[test]
    fn test_clear_completed_lines() {
        let mut buffer = TerminalBuffer::new();

        for byte in b"Line 1\n" {
            buffer.push_byte(*byte);
        }
        for byte in b"Line 2\n" {
            buffer.push_byte(*byte);
        }

        buffer.clear_completed_lines();
        assert_eq!(buffer.completed_line_count(), 0);
    }

    #[test]
    fn test_append_line() {
        let mut buffer = TerminalBuffer::new();
        buffer.append_line("Appended line".to_string());

        assert_eq!(buffer.completed_line_count(), 1);
        let lines = buffer.completed_lines_stripped();
        assert_eq!(lines[0], "Appended line");
    }

    #[test]
    fn test_append_styled_line() {
        use std::str::FromStr;
        let mut buffer = TerminalBuffer::new();
        let styled = StyledString::from_str("Styled text").unwrap();
        buffer.append_styled_line(styled);

        assert_eq!(buffer.completed_line_count(), 1);
    }

    #[test]
    fn test_completed_lines_stripped() {
        let mut buffer = TerminalBuffer::new();

        for byte in b"Line A\n" {
            buffer.push_byte(*byte);
        }
        for byte in b"Line B\n" {
            buffer.push_byte(*byte);
        }

        let stripped = buffer.completed_lines_stripped();
        assert_eq!(stripped.len(), 2);
        assert_eq!(stripped[0], "Line A");
        assert_eq!(stripped[1], "Line B");
    }

    // ===== Buffer Management Tests =====

    #[test]
    fn test_clear() {
        let mut buffer = TerminalBuffer::new();

        for byte in b"Current" {
            buffer.push_byte(*byte);
        }
        for byte in b"Completed\n" {
            buffer.push_byte(*byte);
        }

        buffer.clear();

        assert!(buffer.is_current_line_empty());
        assert_eq!(buffer.completed_line_count(), 0);
        assert_eq!(buffer.cursor_position(), (0, 0));
    }

    #[test]
    fn test_total_line_count_empty() {
        let buffer = TerminalBuffer::new();
        assert_eq!(buffer.total_line_count(), 0);
    }

    #[test]
    fn test_total_line_count_with_current() {
        let mut buffer = TerminalBuffer::new();
        buffer.push_byte(b'X');
        assert_eq!(buffer.total_line_count(), 1);
    }

    #[test]
    fn test_total_line_count_with_completed() {
        let mut buffer = TerminalBuffer::new();
        for byte in b"Line 1\n" {
            buffer.push_byte(*byte);
        }
        for byte in b"Line 2\n" {
            buffer.push_byte(*byte);
        }
        assert_eq!(buffer.total_line_count(), 2);
    }

    #[test]
    fn test_total_line_count_with_both() {
        let mut buffer = TerminalBuffer::new();
        for byte in b"Completed\n" {
            buffer.push_byte(*byte);
        }
        buffer.push_byte(b'C');
        assert_eq!(buffer.total_line_count(), 2);
    }

    // ===== Telnet Event Handling Tests =====

    #[test]
    fn test_handle_event_data() {
        let mut buffer = TerminalBuffer::new();
        buffer.handle_event(TelnetFrame::Data(b'A'));
        // Note: Data frame processes through mapper but doesn't directly add
        assert_eq!(buffer.current_line_length(), 0);
    }

    #[test]
    fn test_handle_event_line() {
        let mut buffer = TerminalBuffer::new();
        buffer.handle_event(TelnetFrame::Line("Test line".to_string()));
        assert_eq!(buffer.completed_line_count(), 1);
    }

    #[test]
    fn test_handle_event_erase_character() {
        let mut buffer = TerminalBuffer::new();
        buffer.push_byte(b'A');
        buffer.push_byte(b'B');
        buffer.handle_event(TelnetFrame::EraseCharacter);
        assert_eq!(buffer.current_line_stripped(), "A");
    }

    #[test]
    fn test_handle_event_erase_line() {
        let mut buffer = TerminalBuffer::new();
        for byte in b"Some text" {
            buffer.push_byte(*byte);
        }
        buffer.handle_event(TelnetFrame::EraseLine);
        assert!(buffer.is_current_line_empty());
    }

    #[test]
    fn test_handle_event_no_operation() {
        let mut buffer = TerminalBuffer::new();
        buffer.push_byte(b'A');
        buffer.handle_event(TelnetFrame::NoOperation);
        assert_eq!(buffer.current_line_stripped(), "A");
    }

    #[test]
    fn test_handle_event_break() {
        let mut buffer = TerminalBuffer::new();
        for byte in b"Text" {
            buffer.push_byte(*byte);
        }
        buffer.handle_event(TelnetFrame::Break);
        assert!(buffer.is_current_line_empty());
    }

    #[test]
    fn test_handle_event_interrupt_process() {
        let mut buffer = TerminalBuffer::new();
        for byte in b"Input" {
            buffer.push_byte(*byte);
        }
        buffer.handle_event(TelnetFrame::InterruptProcess);
        assert!(buffer.is_current_line_empty());
    }

    #[test]
    fn test_handle_event_naws_subnegotiation() {
        let mut buffer = TerminalBuffer::new();
        let size = termionix_codec::naws::WindowSize {
            cols: 120,
            rows: 40,
        };
        buffer.handle_event(TelnetFrame::Subnegotiate(
            TelnetOption::NAWS,
            TelnetArgument::NAWSWindowSize(size),
        ));
        assert_eq!(buffer.size(), (120, 40));
    }

    #[test]
    fn test_handle_event_naws_zero_dimensions_ignored() {
        let mut buffer = TerminalBuffer::new();
        let original_size = buffer.size();

        let size = termionix_codec::naws::WindowSize { cols: 0, rows: 0 };
        buffer.handle_event(TelnetFrame::Subnegotiate(
            TelnetOption::NAWS,
            TelnetArgument::NAWSWindowSize(size),
        ));

        // Size should not change
        assert_eq!(buffer.size(), original_size);
    }

    #[test]
    fn test_handle_event_will() {
        let mut buffer = TerminalBuffer::new();
        buffer.handle_event(TelnetFrame::Will(TelnetOption::Echo));
        // Should not crash, just ignore
        assert_eq!(buffer.size(), (80, 24));
    }

    #[test]
    fn test_handle_event_wont() {
        let mut buffer = TerminalBuffer::new();
        buffer.handle_event(TelnetFrame::Wont(TelnetOption::Echo));
        assert_eq!(buffer.size(), (80, 24));
    }

    #[test]
    fn test_handle_event_do() {
        let mut buffer = TerminalBuffer::new();
        buffer.handle_event(TelnetFrame::Do(TelnetOption::Echo));
        assert_eq!(buffer.size(), (80, 24));
    }

    #[test]
    fn test_handle_event_dont() {
        let mut buffer = TerminalBuffer::new();
        buffer.handle_event(TelnetFrame::Dont(TelnetOption::Echo));
        assert_eq!(buffer.size(), (80, 24));
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_cursor_at_bottom_right() {
        let mut buffer = TerminalBuffer::new_with_size(80, 24);
        buffer.set_cursor_position(79, 23);

        let (col, row) = buffer.cursor_position();
        assert_eq!(col,79);
        assert_eq!(row,23);

        // Cursor should stay in bounds
        buffer.push_byte(b'X');
        let (col, row) = buffer.cursor_position();
        assert!(col < 80, "Column Position >=80: {}", col);
        assert!(row < 24, "Row Position >=24: {}", row);
    }

    #[test]
    fn test_multiple_carriage_returns() {
        let mut buffer = TerminalBuffer::new();
        for byte in b"Test" {
            buffer.push_byte(*byte);
        }
        buffer.push_byte(b'\r');
        buffer.push_byte(b'\r');
        buffer.push_byte(b'\r');
        assert_eq!(buffer.cursor_position(), (0, 0));
    }

    #[test]
    fn test_mixed_lf_cr() {
        let mut buffer = TerminalBuffer::new();
        for byte in b"Line1\r\n" {
            buffer.push_byte(*byte);
        }
        assert_eq!(buffer.completed_line_count(), 1);
        assert_eq!(buffer.cursor_position(), (0, 1));
    }

    #[test]
    fn test_empty_line_completion() {
        let mut buffer = TerminalBuffer::new();
        buffer.push_byte(b'\n');
        assert_eq!(buffer.completed_line_count(), 1);

        let lines = buffer.completed_lines_stripped();
        assert_eq!(lines[0], "");
    }

    #[test]
    fn test_large_text_input() {
        let mut buffer = TerminalBuffer::new();
        let text = "A".repeat(1000);
        for byte in text.as_bytes() {
            buffer.push_byte(*byte);
        }
        assert_eq!(buffer.current_line_length(), 1000);
    }

    #[test]
    fn test_many_completed_lines() {
        let mut buffer = TerminalBuffer::new();
        for i in 0..100 {
            for byte in format!("Line {}\n", i).as_bytes() {
                buffer.push_byte(*byte);
            }
        }
        assert_eq!(buffer.completed_line_count(), 100);
    }

    #[test]
    fn test_ansi_codes_preserved_when_enabled() {
        let mut buffer = TerminalBuffer::new();
        buffer.set_ansi_status(true);

        // Push an ANSI escape sequence (red color)
        for byte in b"\x1b[31mRed\x1b[0m" {
            buffer.push_byte(*byte);
        }

        // The stripped version should just be "Red"
        assert_eq!(buffer.current_line_stripped(), "Red");
    }

    #[test]
    fn test_ansi_codes_stripped_when_disabled() {
        let mut buffer = TerminalBuffer::new();
        buffer.set_ansi_status(false);

        // Push an ANSI escape sequence
        for byte in b"\x1b[31mRed\x1b[0m" {
            buffer.push_byte(*byte);
        }

        assert_eq!(buffer.current_line_stripped(), "Red");
    }

    #[test]
    fn test_cursor_wrapping_prevention() {
        let mut buffer = TerminalBuffer::new_with_size(5, 5);

        // Fill up way past the width
        for _ in 0..10 {
            buffer.push_byte(b'X');
        }

        let (col, _row) = buffer.cursor_position();
        assert!(col < 100); // Should have advanced but not wrapped weirdly
    }

    #[test]
    fn test_resize_to_zero_width() {
        let mut buffer = TerminalBuffer::new();
        buffer.set_size(0, 24);
        // Should handle gracefully - cursor clamped
        assert_eq!(buffer.width(), 0);
    }

    #[test]
    fn test_resize_to_zero_height() {
        let mut buffer = TerminalBuffer::new();
        buffer.set_size(80, 0);
        assert_eq!(buffer.height(), 0);
    }

    // ===== Integration Tests =====

    #[test]
    fn test_full_interaction_flow() {
        let mut buffer = TerminalBuffer::new_with_size(80, 24);

        // Type a command
        for byte in b"echo 'hello'" {
            buffer.push_byte(*byte);
        }
        assert_eq!(buffer.current_line_stripped(), "echo 'hello'");

        // Press enter
        buffer.push_byte(b'\n');
        assert_eq!(buffer.completed_line_count(), 1);
        assert!(buffer.is_current_line_empty());

        // Simulate output
        buffer.append_line("hello".to_string());
        assert_eq!(buffer.completed_line_count(), 2);

        // Type another command
        for byte in b"ls" {
            buffer.push_byte(*byte);
        }

        // Make a typo and backspace
        buffer.push_byte(b'x');
        buffer.push_byte(b'\x08');

        assert_eq!(buffer.current_line_stripped(), "ls");
        assert_eq!(buffer.total_line_count(), 3); // 2 completed + 1 current
    }

    #[test]
    fn test_telnet_resize_interaction() {
        let mut buffer = TerminalBuffer::new_with_size(80, 24);

        // Simulate window resize via NAWS
        let size = termionix_codec::naws::WindowSize {
            cols: 132,
            rows: 43,
        };
        buffer.handle_event(TelnetFrame::Subnegotiate(
            TelnetOption::NAWS,
            TelnetArgument::NAWSWindowSize(size),
        ));

        assert_eq!(buffer.size(), (132, 43));

        // Cursor should still be valid
        let (col, row) = buffer.cursor_position();
        assert!(col < 132);
        assert!(row < 43);
    }

    #[test]
    fn test_clear_preserves_size() {
        let mut buffer = TerminalBuffer::new_with_size(100, 50);

        for byte in b"Some data\n" {
            buffer.push_byte(*byte);
        }

        buffer.clear();

        // Size should be preserved after clear
        assert_eq!(buffer.size(), (100, 50));
        assert!(buffer.is_current_line_empty());
        assert_eq!(buffer.completed_line_count(), 0);
    }
}
