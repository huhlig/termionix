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

use crate::types::{CursorPosition, TerminalSize};
use std::collections::BTreeMap;
use termionix_ansicodec::utility::StyledString;
use termionix_ansicodec::{AnsiConfig, SegmentedString};

/// Virtual Terminal Buffer
pub struct TerminalBuffer {
    /// Terminal dimensions (width, height)
    size: TerminalSize,
    /// Current cursor position (column, row) - 0-indexed
    cursor: CursorPosition,
    /// Terminal Environment variables
    environment: BTreeMap<String, String>,
    /// Completed lines
    completed_lines: Vec<SegmentedString>,
    /// The current line buffer (data being typed)
    current_line: SegmentedString,
}

impl TerminalBuffer {
    /// Creates a new terminal buffer with a default size of (80x24)
    pub fn new() -> Self {
        Self::new_with_size(80, 24)
    }

    /// Creates a new terminal buffer with specified dimensions
    pub fn new_with_size(width: usize, height: usize) -> Self {
        TerminalBuffer {
            size: TerminalSize::new(width, height),
            cursor: CursorPosition::new(0, 0),
            environment: BTreeMap::new(),
            current_line: SegmentedString::empty(),
            completed_lines: Vec::new(),
        }
    }

    // ===== Terminal Size Management =====

    /// Sets the terminal size
    pub fn set_size(&mut self, width: usize, height: usize) {
        self.size = TerminalSize::new(width, height);
        // Adjust the cursor if it's now out of bounds
        if self.cursor.col >= self.size.cols {
            self.cursor.col = self.size.cols.saturating_sub(1);
        }
        if self.cursor.row >= self.size.rows {
            self.cursor.row = self.size.rows.saturating_sub(1);
        }
    }

    /// Gets the current terminal size
    pub fn size(&self) -> TerminalSize {
        self.size
    }

    /// Gets the terminal width
    pub fn width(&self) -> usize {
        self.size.cols
    }

    /// Gets the terminal height
    pub fn height(&self) -> usize {
        self.size.rows
    }

    // ===== Cursor Management =====

    /// Gets the current cursor position
    pub fn cursor_position(&self) -> CursorPosition {
        self.cursor
    }

    /// Sets the cursor position (clamped to terminal bounds)
    pub fn set_cursor_position(&mut self, col: usize, row: usize) {
        self.cursor = CursorPosition::new(
            col.min(self.size.cols.saturating_sub(1)),
            row.min(self.size.rows.saturating_sub(1)),
        );
    }

    // ===== Environment Management =====

    /// Returns an iterator over all environment variable key-value pairs.
    ///
    /// This method provides read-only access to the terminal's environment variables
    /// stored in the buffer. The iterator yields references to both keys and values.
    ///
    /// # Returns
    ///
    /// An iterator that yields tuples of `(&String, &String)` representing
    /// (key, value) pairs of environment variables.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use termionix_terminal::TerminalBuffer;
    /// let mut buffer = TerminalBuffer::new();
    /// buffer.set_environment("USER", "alice");
    /// buffer.set_environment("SHELL", "/bin/bash");
    ///
    /// for (key, value) in buffer.environment() {
    ///     println!("{} = {}", key, value);
    /// }
    /// ```
    pub fn environment(&self) -> impl Iterator<Item = (&String, &String)> {
        self.environment.iter()
    }

    /// Sets an environment variable in the terminal buffer.
    ///
    /// This method stores or updates an environment variable key-value pair.
    /// If the key already exists, its value will be replaced with the new value.
    /// Both the key and value are converted to owned `String` instances.
    ///
    /// # Parameters
    ///
    /// * `key` - The environment variable name
    /// * `value` - The environment variable value
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use termionix_terminal::TerminalBuffer;
    /// let mut buffer = TerminalBuffer::new();
    ///
    /// // Set a new environment variable
    /// buffer.set_environment("PATH", "/usr/local/bin:/usr/bin");
    ///
    /// // Update an existing environment variable
    /// buffer.set_environment("PATH", "/opt/bin:/usr/local/bin:/usr/bin");
    /// ```
    pub fn set_environment(&mut self, key: &str, value: &str) {
        self.environment.insert(key.to_string(), value.to_string());
    }

    /// Retrieves the value of an environment variable by key.
    ///
    /// This method looks up an environment variable in the terminal buffer
    /// and returns a reference to its value if found.
    ///
    /// # Parameters
    ///
    /// * `key` - The environment variable name to look up
    ///
    /// # Returns
    ///
    /// * `Some(&String)` - A reference to the value if the key exists
    /// * `None` - If the key doesn't exist in the environment
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use termionix_terminal::TerminalBuffer;
    /// let mut buffer = TerminalBuffer::new();
    /// buffer.set_environment("HOME", "/home/user");
    ///
    /// // Retrieve an existing variable
    /// if let Some(home) = buffer.get_environment("HOME") {
    ///     println!("Home directory: {}", home);
    /// }
    ///
    /// // Try to retrieve a non-existent variable
    /// assert_eq!(buffer.get_environment("NONEXISTENT"), None);
    /// ```
    pub fn get_environment(&self, key: &str) -> Option<&String> {
        self.environment.get(key)
    }

    /// Moves the cursor by a relative offset in both column and row directions.
    ///
    /// This method adjusts the cursor position by the specified column and row deltas,
    /// with negative values moving left/up and positive values moving right/down.
    /// The resulting position is automatically clamped to the terminal bounds.
    ///
    /// # Parameters
    ///
    /// * `col` - The column delta to move the cursor. Positive values move right,
    ///           negative values move left.
    /// * `row` - The row delta to move the cursor. Positive values move down,
    ///           negative values move up.
    ///
    /// # Returns
    ///
    /// Returns a tuple `(usize, usize)` representing the new cursor position after
    /// the move operation, where the first element is the column and the second is
    /// the row. Both values are 0-indexed and clamped to the terminal bounds.
    ///
    /// # Bounds Handling
    ///
    /// * Column values are clamped to `[0, width - 1]`
    /// * Row values are clamped to `[0, height - 1]`
    /// * Operations use saturating arithmetic to prevent overflow/underflow
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use termionix_terminal::{CursorPosition, TerminalBuffer};
    /// let mut buffer = TerminalBuffer::new_with_size(80, 24);
    /// buffer.set_cursor_position(10, 5);
    ///
    /// // Move right 5 columns and down 2 rows
    /// let pos = buffer.move_cursor(5, 2);
    /// assert_eq!(pos, CursorPosition::new(15, 7));
    ///
    /// // Move left 20 columns (will clamp to 0)
    /// let pos = buffer.move_cursor(-20, 0);
    /// assert_eq!(pos, CursorPosition::new(0, 7));
    ///
    /// // Move to a position that exceeds bounds (will clamp)
    /// buffer.set_cursor_position(75, 20);
    /// let pos = buffer.move_cursor(100, 100);
    /// assert_eq!(pos, CursorPosition::new(79, 23)); // Clamped to terminal bounds (80x24)
    /// ```
    pub fn move_cursor(&mut self, col_delta: isize, row_delta: isize) -> CursorPosition {
        // Calculate the new column position
        let new_col = if col_delta >= 0 {
            self.cursor.col.saturating_add(col_delta as usize)
        } else {
            self.cursor.col.saturating_sub(col_delta.unsigned_abs())
        };

        // Calculate the new row position
        let new_row = if row_delta >= 0 {
            self.cursor.row.saturating_add(row_delta as usize)
        } else {
            self.cursor.row.saturating_sub(row_delta.unsigned_abs())
        };

        // Clamp to the terminal bounds
        self.cursor.col = new_col.min(self.size.cols.saturating_sub(1));
        self.cursor.row = new_row.min(self.size.rows.saturating_sub(1));

        self.cursor
    }

    /// Erases the last character from the current line buffer
    pub fn erase_character(&mut self) {
        if self.current_line.pop().is_some() {
            if self.cursor.col > 0 {
                self.cursor.col -= 1;
            } else if self.cursor.row > 0 {
                // Wrapped to the previous line
                self.cursor.row -= 1;
                self.cursor.row = self.size.rows.saturating_sub(1);
            }
        }
    }

    /// Gets the current character count in the current line
    pub fn current_line_length(&self) -> usize {
        self.current_line
            .len(Some(&AnsiConfig::strip_all()))
            .unwrap()
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
    pub fn complete_line(&mut self) -> SegmentedString {
        let line = std::mem::take(&mut self.current_line);
        self.completed_lines.push(line.clone());
        self.cursor.col = 0;
        self.cursor.row = (self.cursor.row + 1).min(self.size.rows.saturating_sub(1));
        line
    }

    /// Erases the entire current line
    pub fn erase_line(&mut self) {
        self.current_line.clear();
        self.cursor.col = 0;
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

    /// Appends a character to the current line buffer and advances the cursor position.
    ///
    /// This method adds a character to the end of the current line and updates the cursor
    /// position according to the terminal size constraints. The cursor will automatically
    /// wrap to the next line when reaching the rightmost column, provided there are
    /// additional rows available.
    ///
    /// # Cursor Behavior
    ///
    /// The cursor advancement follows these rules:
    /// - **Normal case**: If the cursor is not at the rightmost column, it advances one position to the right
    /// - **Line wrap**: If the cursor is at the rightmost column and not on the last row, it wraps to column 0 of the next row
    /// - **Bottom-right corner**: If the cursor is at the bottom-right corner of the terminal, it remains at the rightmost position
    ///
    /// # Terminal Size Handling
    ///
    /// The method respects the terminal's dimensions (`self.size.cols` and `self.size.rows`):
    /// - Characters appended at the right edge trigger line wrapping when possible
    /// - The cursor never exceeds `cols - 1` or `rows - 1`
    /// - Uses saturating arithmetic to prevent underflow in boundary calculations
    ///
    /// # Arguments
    ///
    /// * `c` - The character to append to the current line
    ///
    /// # Examples
    ///
    /// Basic character appending:
    ///
    /// ```rust
    /// # use termionix_terminal::TerminalBuffer;
    /// let mut buffer = TerminalBuffer::new_with_size(80, 24);
    ///
    /// buffer.append_char('H');
    /// buffer.append_char('i');
    ///
    /// assert_eq!(buffer.current_line_stripped(), "Hi");
    /// assert_eq!(buffer.cursor_position().col, 2);
    /// ```
    ///
    /// Line wrapping at terminal edge:
    ///
    /// ```rust
    /// # use termionix_terminal::TerminalBuffer;
    /// let mut buffer = TerminalBuffer::new_with_size(5, 3);
    ///
    /// // Fill the first line
    /// for ch in "Test".chars() {
    ///     buffer.append_char(ch);
    /// }
    ///
    /// // Cursor should be at rightmost column
    /// assert_eq!(buffer.cursor_position().col, 4);
    /// assert_eq!(buffer.cursor_position().row, 0);
    ///
    /// // Next character wraps to next line
    /// buffer.append_char('!');
    /// assert_eq!(buffer.cursor_position().col, 0);
    /// assert_eq!(buffer.cursor_position().row, 1);
    /// ```
    ///
    /// Bottom-right corner behavior:
    ///
    /// ```rust
    /// # use termionix_terminal::TerminalBuffer;
    /// let mut buffer = TerminalBuffer::new_with_size(3, 2);
    ///
    /// // Move to bottom-right corner
    /// buffer.set_cursor_position(2, 1);
    ///
    /// // Append character - cursor stays at rightmost position
    /// buffer.append_char('X');
    /// assert_eq!(buffer.cursor_position().col, 2);
    /// assert_eq!(buffer.cursor_position().row, 1);
    /// ```
    ///
    /// Unicode character support:
    ///
    /// ```rust
    /// # use termionix_terminal::TerminalBuffer;
    /// let mut buffer = TerminalBuffer::new();
    ///
    /// buffer.append_char('🦀'); // Multi-byte UTF-8 character
    /// buffer.append_char('日'); // Japanese character
    ///
    /// assert_eq!(buffer.current_line_stripped(), "🦀日");
    /// assert_eq!(buffer.cursor_position().col, 2);
    /// ```
    ///
    /// # Performance
    ///
    /// This is an O(1) operation. The character is appended to the current line's
    /// internal buffer, and cursor position is updated with simple arithmetic.
    ///
    /// # See Also
    ///
    /// - [`erase_character()`](TerminalBuffer::erase_character) - Remove the last character and move cursor back
    /// - [`complete_line()`](TerminalBuffer::complete_line) - Finish the current line and move to the next
    /// - [`set_cursor_position()`](TerminalBuffer::set_cursor_position) - Manually set cursor position
    /// - [`move_cursor()`](TerminalBuffer::move_cursor) - Move cursor by relative offset

    pub fn append_char(&mut self, c: char) {
        // Handle control characters
        match c {
            '\x08' => {
                // Backspace (BS) - erase previous character
                self.erase_character();
                return;
            }
            '\x07' => {
                // Bell (BEL) - audible alert (no visual effect on buffer)
                return;
            }
            '\x09' => {
                // '\t'
                // Horizontal Tab (HT) - add tab character
                self.current_line.push_char(c);
                // Move cursor to next tab stop (typically 8 columns)
                let tab_width = 8;
                let next_tab_stop = ((self.cursor.col / tab_width) + 1) * tab_width;
                self.cursor.col = next_tab_stop.min(self.size.cols.saturating_sub(1));
                return;
            }
            '\x0A' => {
                // '\n'
                // Line Feed (LF) - complete the current line and move down
                self.complete_line();
                return;
            }
            '\x0B' => {
                // Vertical Tab (VT) - treat like line feed
                self.complete_line();
                return;
            }
            '\x0C' => {
                // Form Feed (FF) - clear screen (complete current line, clear all)
                self.complete_line();
                self.clear_completed_lines();
                return;
            }
            '\x0D' => {
                // '\r'
                // Carriage Return (CR) - move the cursor to the beginning of the line
                self.cursor.col = 0;
                return;
            }
            '\x7F' => {
                // Delete (DEL) - same as backspace
                self.erase_character();
                return;
            }
            // Ignore other C0 control codes (0x00-0x1F) except those handled above
            '\x00'..='\x1F' => {
                // Skip other control codes (NUL, SOH, STX, ETX, EOT, ENQ, ACK, etc.)
                return;
            }
            _ => {
                // Regular printable character - append it
            }
        }

        self.current_line.push_char(c);

        // Check if the cursor would exceed terminal width after advancing
        let max_col = self.size.cols.saturating_sub(1);
        if self.cursor.col >= max_col {
            // At or beyond the right edge - wrap to the next line if not at bottom
            let max_row = self.size.rows.saturating_sub(1);
            if self.cursor.row < max_row {
                // Move to the next line
                self.cursor.col = 0;
                self.cursor.row += 1;
            } else {
                // At the bottom-right corner - stay at rightmost position
                self.cursor.col = max_col;
            }
        } else {
            // Normal case - just advance cursor
            self.cursor.col += 1;
        }
    }

    /// Appends a pre-formed line to the completed lines (useful for echoing)
    pub fn append_line(&mut self, line: &str) {
        self.completed_lines.push(SegmentedString::parse(line));
    }

    /// Appends a pre-formed line to the completed lines (useful for echoing)
    pub fn append_styled_line(&mut self, line: StyledString) {
        self.completed_lines.push(line.segmented());
    }

    /// Appends a pre-formed line to the completed lines (useful for echoing)
    pub fn append_segmented_line(&mut self, line: SegmentedString) {
        self.completed_lines.push(line);
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
        self.cursor = CursorPosition::new(0, 0);
    }

    /// Gets the total line count (completed + current if non-empty)
    pub fn total_line_count(&self) -> usize {
        let current = if self.current_line.is_empty() { 0 } else { 1 };
        self.completed_lines.len() + current
    }

    // ============================ Helpers =====================================
    /// Advance the cursor by one column, clamping to terminal bounds.
    /// If at the right-most column, do not increment past width - 1.
    pub fn advance_cursor_by_one(&mut self) {
        // If the width is zero, stay at column 0
        if self.size.cols == 0 {
            self.cursor.col = 0;
            return;
        }

        // Only advance if we're not at the right-most column
        let max_col = self.size.cols.saturating_sub(1);
        if self.cursor.col < max_col {
            self.cursor.col += 1;
        } else {
            // At right-most column: keep column at max, but if not at the bottom row, allow moving
            // to the next row on completion of line only.
            self.cursor.col = max_col;
        }
    }
}

impl Default for TerminalBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TerminalBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalBuffer")
            .field("size", &self.size())
            .field("cursor_position", &self.cursor_position())
            .field("completed_line_count", &self.completed_line_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Constructor Tests =====

    #[test]
    fn test_new_creates_default_buffer() {
        let buffer = TerminalBuffer::new();
        assert_eq!(buffer.size(), TerminalSize::new(80, 24));
        assert_eq!(buffer.cursor_position(), CursorPosition::new(0, 0));
        assert!(buffer.is_current_line_empty());
        assert_eq!(buffer.completed_line_count(), 0);
    }

    #[test]
    fn test_new_with_size() {
        let buffer = TerminalBuffer::new_with_size(120, 40);
        assert_eq!(buffer.size(), TerminalSize::new(120, 40));
        assert_eq!(buffer.width(), 120);
        assert_eq!(buffer.height(), 40);
    }

    #[test]
    fn test_default_trait() {
        let buffer = TerminalBuffer::default();
        assert_eq!(buffer.size(), TerminalSize::new(80, 24));
    }

    // ===== Terminal Size Management Tests =====

    #[test]
    fn test_set_size() {
        let mut buffer = TerminalBuffer::new();
        buffer.set_size(100, 30);
        assert_eq!(buffer.size(), TerminalSize::new(100, 30));
        assert_eq!(buffer.width(), 100);
        assert_eq!(buffer.height(), 30);
    }

    #[test]
    fn test_set_size_clamps_cursor() {
        let mut buffer = TerminalBuffer::new_with_size(80, 24);
        buffer.set_cursor_position(75, 20);

        // Reduce size so the cursor is out of bounds
        buffer.set_size(50, 15);

        let pos = buffer.cursor_position();
        assert!(pos.col < 50);
        assert!(pos.row < 15);
    }

    #[test]
    fn test_set_size_with_cursor_at_edge() {
        let mut buffer = TerminalBuffer::new_with_size(80, 24);
        buffer.set_cursor_position(79, 23);

        // Reduce to a smaller size
        buffer.set_size(40, 12);

        assert_eq!(buffer.cursor_position(), CursorPosition::new(39, 11));
    }

    // ===== Cursor Management Tests =====

    #[test]
    fn test_set_cursor_position() {
        let mut buffer = TerminalBuffer::new();
        buffer.set_cursor_position(10, 5);
        assert_eq!(buffer.cursor_position(), CursorPosition::new(10, 5));
    }

    #[test]
    fn test_cursor_clamped_to_bounds() {
        let mut buffer = TerminalBuffer::new_with_size(80, 24);
        buffer.set_cursor_position(100, 50);
        assert_eq!(buffer.cursor_position(), CursorPosition::new(79, 23));
    }

    #[test]
    fn test_cursor_moves_with_characters() {
        let mut buffer = TerminalBuffer::new();
        buffer.append_char('H');
        buffer.append_char('i');
        assert_eq!(buffer.cursor_position(), CursorPosition::new(2, 0));
    }

    #[test]
    fn test_cursor_resets_on_carriage_return() {
        let mut buffer = TerminalBuffer::new();
        buffer.append_char('H');
        buffer.append_char('i');
        buffer.append_char('\r');
        assert_eq!(buffer.cursor_position(), CursorPosition::new(0, 0));
    }

    #[test]
    fn test_cursor_advances_on_line_feed() {
        let mut buffer = TerminalBuffer::new();
        buffer.append_char('H');
        buffer.append_char('i');
        buffer.append_char('\n');
        assert_eq!(buffer.cursor_position(), CursorPosition::new(0, 1));
    }

    // ===== Character-level API Tests =====

    #[test]
    fn test_push_byte_ascii() {
        let mut buffer = TerminalBuffer::new();
        buffer.append_char('A');
        assert_eq!(buffer.current_line_length(), 1);
        assert_eq!(buffer.current_line_stripped(), "A");
    }

    #[test]
    fn test_erase_character() {
        let mut buffer = TerminalBuffer::new();
        buffer.append_char('A');
        buffer.append_char('B');
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
        buffer.append_char('A');
        buffer.append_char('B');
        buffer.append_char('C');
        assert_eq!(buffer.cursor_position(), CursorPosition::new(3, 0));

        buffer.erase_character();
        assert_eq!(buffer.cursor_position(), CursorPosition::new(2, 0));
    }

    #[test]
    fn test_backspace_control_code() {
        let mut buffer = TerminalBuffer::new();
        buffer.append_char('A');
        buffer.append_char('B');
        buffer.append_char('\x08'); // Backspace
        assert_eq!(buffer.current_line_stripped(), "A");
    }

    #[test]
    fn test_current_line_length() {
        let mut buffer = TerminalBuffer::new();
        assert_eq!(buffer.current_line_length(), 0);

        for ch in "Test".chars() {
            buffer.append_char(ch);
        }
        assert_eq!(buffer.current_line_length(), 4);
    }

    #[test]
    fn test_is_current_line_empty() {
        let mut buffer = TerminalBuffer::new();
        assert!(buffer.is_current_line_empty());

        buffer.append_char('X');
        assert!(!buffer.is_current_line_empty());

        buffer.erase_character();
        assert!(buffer.is_current_line_empty());
    }

    // ===== Line-level API Tests =====

    #[test]
    fn test_complete_line() {
        let mut buffer = TerminalBuffer::new();
        for byte in "First line".chars() {
            buffer.append_char(byte);
        }
        buffer.complete_line();

        assert_eq!(buffer.completed_line_count(), 1);
        assert!(buffer.is_current_line_empty());
        assert_eq!(buffer.cursor_position(), CursorPosition::new(0, 1));
    }

    #[test]
    fn test_complete_multiple_lines() {
        let mut buffer = TerminalBuffer::new();

        for ch in "Line 1".chars() {
            buffer.append_char(ch);
        }
        buffer.complete_line();

        for ch in "Line 2".chars() {
            buffer.append_char(ch);
        }
        buffer.complete_line();

        assert_eq!(buffer.completed_line_count(), 2);
    }

    #[test]
    fn test_line_feed_completes_line() {
        let mut buffer = TerminalBuffer::new();
        for ch in "Test\n".chars() {
            buffer.append_char(ch);
        }

        assert_eq!(buffer.completed_line_count(), 1);
        assert!(buffer.is_current_line_empty());
    }

    #[test]
    fn test_erase_line() {
        let mut buffer = TerminalBuffer::new();
        for ch in "Some text".chars() {
            buffer.append_char(ch);
        }
        buffer.erase_line();

        assert!(buffer.is_current_line_empty());
        assert_eq!(buffer.cursor_position(), CursorPosition::new(0, 0));
    }

    #[test]
    fn test_completed_lines() {
        let mut buffer = TerminalBuffer::new();

        for ch in "Line 1\n".chars() {
            buffer.append_char(ch);
        }
        for ch in "Line 2\n".chars() {
            buffer.append_char(ch);
        }

        let lines = buffer.completed_lines();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_pop_completed_line() {
        let mut buffer = TerminalBuffer::new();

        for ch in "First\n".chars() {
            buffer.append_char(ch);
        }
        for ch in "Second\n".chars() {
            buffer.append_char(ch);
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

        for ch in "Line 1\n".chars() {
            buffer.append_char(ch);
        }
        for ch in "Line 2\n".chars() {
            buffer.append_char(ch);
        }

        let lines = buffer.take_completed_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(buffer.completed_line_count(), 0);
    }

    #[test]
    fn test_clear_completed_lines() {
        let mut buffer = TerminalBuffer::new();

        for ch in "Line 1\n".chars() {
            buffer.append_char(ch);
        }
        for ch in "Line 2\n".chars() {
            buffer.append_char(ch);
        }

        buffer.clear_completed_lines();
        assert_eq!(buffer.completed_line_count(), 0);
    }

    #[test]
    fn test_append_line() {
        let mut buffer = TerminalBuffer::new();
        buffer.append_line("Appended line");

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

        for ch in "Line A\n".chars() {
            buffer.append_char(ch);
        }
        for ch in "Line B\n".chars() {
            buffer.append_char(ch);
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

        for ch in "Current".chars() {
            buffer.append_char(ch);
        }
        for ch in "Completed\n".chars() {
            buffer.append_char(ch);
        }

        buffer.clear();

        assert!(buffer.is_current_line_empty());
        assert_eq!(buffer.completed_line_count(), 0);
        assert_eq!(buffer.cursor_position(), CursorPosition::new(0, 0));
    }

    #[test]
    fn test_total_line_count_empty() {
        let buffer = TerminalBuffer::new();
        assert_eq!(buffer.total_line_count(), 0);
    }

    #[test]
    fn test_total_line_count_with_current() {
        let mut buffer = TerminalBuffer::new();
        buffer.append_char('X');
        assert_eq!(buffer.total_line_count(), 1);
    }

    #[test]
    fn test_total_line_count_with_completed() {
        let mut buffer = TerminalBuffer::new();
        for ch in "Line 1\n".chars() {
            buffer.append_char(ch);
        }
        for ch in "Line 2\n".chars() {
            buffer.append_char(ch);
        }
        assert_eq!(buffer.total_line_count(), 2);
    }

    #[test]
    fn test_total_line_count_with_both() {
        let mut buffer = TerminalBuffer::new();
        for ch in "Completed\n".chars() {
            buffer.append_char(ch);
        }
        buffer.append_char('C');
        assert_eq!(buffer.total_line_count(), 2);
    }

    // ===== Edge Case Tests =====

    #[test]
    fn test_cursor_at_bottom_right() {
        let mut buffer = TerminalBuffer::new_with_size(80, 24);
        buffer.set_cursor_position(79, 23);

        let pos = buffer.cursor_position();
        assert_eq!(pos.col, 79);
        assert_eq!(pos.row, 23);

        // Cursor should stay in bounds
        buffer.append_char('X');
        let pos = buffer.cursor_position();
        assert!(pos.col < 80, "Column Position >=80: {}", pos.col);
        assert!(pos.row < 24, "Row Position >=24: {}", pos.row);
    }

    #[test]
    fn test_multiple_carriage_returns() {
        let mut buffer = TerminalBuffer::new();
        for ch in "Test".chars() {
            buffer.append_char(ch);
        }
        buffer.append_char('\r');
        buffer.append_char('\r');
        buffer.append_char('\r');
        assert_eq!(buffer.cursor_position(), CursorPosition::new(0, 0));
    }

    #[test]
    fn test_mixed_lf_cr() {
        let mut buffer = TerminalBuffer::new();
        for ch in "Line1\r\n".chars() {
            buffer.append_char(ch);
        }
        assert_eq!(buffer.completed_line_count(), 1);
        assert_eq!(buffer.cursor_position(), CursorPosition::new(0, 1));
    }

    #[test]
    fn test_empty_line_completion() {
        let mut buffer = TerminalBuffer::new();
        buffer.append_char('\n');
        assert_eq!(buffer.completed_line_count(), 1);

        let lines = buffer.completed_lines_stripped();
        assert_eq!(lines[0], "");
    }

    #[test]
    fn test_large_text_input() {
        let mut buffer = TerminalBuffer::new();
        let text = "A".repeat(1000);
        for byte in text.chars() {
            buffer.append_char(byte);
        }
        assert_eq!(buffer.current_line_length(), 1000);
    }

    #[test]
    fn test_many_completed_lines() {
        let mut buffer = TerminalBuffer::new();
        for i in 0..100 {
            for byte in format!("Line {}\n", i).chars() {
                buffer.append_char(byte);
            }
        }
        assert_eq!(buffer.completed_line_count(), 100);
    }

    #[test]
    fn test_cursor_wrapping_prevention() {
        let mut buffer = TerminalBuffer::new_with_size(5, 5);

        // Fill up way past the width
        for _ in 0..10 {
            buffer.append_char('X');
        }

        let pos = buffer.cursor_position();
        assert!(pos.col < 100); // Should have advanced but not wrapped weirdly
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
}
