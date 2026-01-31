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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerminalSize {
    pub cols: usize,
    pub rows: usize,
}

impl TerminalSize {
    pub fn new(cols: usize, rows: usize) -> Self {
        Self { cols, rows }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CursorPosition {
    pub col: usize,
    pub row: usize,
}

impl CursorPosition {
    pub fn new(col: usize, row: usize) -> Self {
        Self { col, row }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== TerminalSize Tests =====

    #[test]
    fn test_terminal_size_new() {
        let size = TerminalSize::new(80, 24);
        assert_eq!(size.cols, 80);
        assert_eq!(size.rows, 24);
    }

    #[test]
    fn test_terminal_size_zero() {
        let size = TerminalSize::new(0, 0);
        assert_eq!(size.cols, 0);
        assert_eq!(size.rows, 0);
    }

    #[test]
    fn test_terminal_size_large() {
        let size = TerminalSize::new(1920, 1080);
        assert_eq!(size.cols, 1920);
        assert_eq!(size.rows, 1080);
    }

    #[test]
    fn test_terminal_size_equality() {
        let size1 = TerminalSize::new(80, 24);
        let size2 = TerminalSize::new(80, 24);
        let size3 = TerminalSize::new(100, 30);

        assert_eq!(size1, size2);
        assert_ne!(size1, size3);
    }

    #[test]
    fn test_terminal_size_ordering() {
        let small = TerminalSize::new(40, 20);
        let medium = TerminalSize::new(80, 24);
        let large = TerminalSize::new(120, 40);

        assert!(small < medium);
        assert!(medium < large);
        assert!(small < large);
    }

    #[test]
    fn test_terminal_size_clone() {
        let size1 = TerminalSize::new(80, 24);
        let size2 = size1.clone();
        assert_eq!(size1, size2);
    }

    #[test]
    fn test_terminal_size_copy() {
        let size1 = TerminalSize::new(80, 24);
        let size2 = size1; // Copy
        assert_eq!(size1, size2);
    }

    #[test]
    fn test_terminal_size_debug() {
        let size = TerminalSize::new(80, 24);
        let debug_str = format!("{:?}", size);
        assert!(debug_str.contains("80"));
        assert!(debug_str.contains("24"));
    }

    #[test]
    fn test_terminal_size_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(TerminalSize::new(80, 24));
        set.insert(TerminalSize::new(80, 24)); // Duplicate
        set.insert(TerminalSize::new(100, 30));

        assert_eq!(set.len(), 2); // Only 2 unique sizes
    }

    // ===== CursorPosition Tests =====

    #[test]
    fn test_cursor_position_new() {
        let pos = CursorPosition::new(10, 5);
        assert_eq!(pos.col, 10);
        assert_eq!(pos.row, 5);
    }

    #[test]
    fn test_cursor_position_origin() {
        let pos = CursorPosition::new(0, 0);
        assert_eq!(pos.col, 0);
        assert_eq!(pos.row, 0);
    }

    #[test]
    fn test_cursor_position_large() {
        let pos = CursorPosition::new(1000, 500);
        assert_eq!(pos.col, 1000);
        assert_eq!(pos.row, 500);
    }

    #[test]
    fn test_cursor_position_equality() {
        let pos1 = CursorPosition::new(10, 5);
        let pos2 = CursorPosition::new(10, 5);
        let pos3 = CursorPosition::new(20, 10);

        assert_eq!(pos1, pos2);
        assert_ne!(pos1, pos3);
    }

    #[test]
    fn test_cursor_position_ordering() {
        let pos1 = CursorPosition::new(5, 5);
        let pos2 = CursorPosition::new(10, 5);
        let pos3 = CursorPosition::new(5, 10);

        assert!(pos1 < pos2);
        assert!(pos1 < pos3);
    }

    #[test]
    fn test_cursor_position_clone() {
        let pos1 = CursorPosition::new(10, 5);
        let pos2 = pos1.clone();
        assert_eq!(pos1, pos2);
    }

    #[test]
    fn test_cursor_position_copy() {
        let pos1 = CursorPosition::new(10, 5);
        let pos2 = pos1; // Copy
        assert_eq!(pos1, pos2);
    }

    #[test]
    fn test_cursor_position_debug() {
        let pos = CursorPosition::new(10, 5);
        let debug_str = format!("{:?}", pos);
        assert!(debug_str.contains("10"));
        assert!(debug_str.contains("5"));
    }

    #[test]
    fn test_cursor_position_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(CursorPosition::new(10, 5));
        set.insert(CursorPosition::new(10, 5)); // Duplicate
        set.insert(CursorPosition::new(20, 10));

        assert_eq!(set.len(), 2); // Only 2 unique positions
    }

    // ===== Integration Tests =====

    #[test]
    fn test_cursor_within_terminal_size() {
        let size = TerminalSize::new(80, 24);
        let pos = CursorPosition::new(40, 12);

        assert!(pos.col < size.cols);
        assert!(pos.row < size.rows);
    }

    #[test]
    fn test_cursor_at_terminal_edge() {
        let size = TerminalSize::new(80, 24);
        let pos = CursorPosition::new(79, 23);

        assert!(pos.col < size.cols);
        assert!(pos.row < size.rows);
    }

    #[test]
    fn test_cursor_beyond_terminal_size() {
        let size = TerminalSize::new(80, 24);
        let pos = CursorPosition::new(100, 50);

        assert!(pos.col >= size.cols);
        assert!(pos.row >= size.rows);
    }
}
