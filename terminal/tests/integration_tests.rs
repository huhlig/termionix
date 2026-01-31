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

use termionix_terminal::{
    CursorPosition, TerminalBuffer, TerminalCodec, TerminalCommand, TerminalEvent, TerminalSize,
};
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

fn create_test_codec()
-> TerminalCodec<termionix_ansicodec::AnsiCodec<termionix_telnetcodec::TelnetCodec>> {
    let telnet_codec = termionix_telnetcodec::TelnetCodec::new();
    let ansi_codec = termionix_ansicodec::AnsiCodec::new(
        termionix_ansicodec::AnsiConfig::default(),
        telnet_codec,
    );
    TerminalCodec::new(ansi_codec)
}

// ===== Terminal Buffer Integration Tests =====

#[test]
fn test_buffer_full_workflow() {
    let mut buffer = TerminalBuffer::new();

    // Add characters
    buffer.append_char('H');
    buffer.append_char('e');
    buffer.append_char('l');
    buffer.append_char('l');
    buffer.append_char('o');

    assert_eq!(buffer.current_line_length(), 5);
    assert!(!buffer.is_current_line_empty());

    // Complete the line
    buffer.complete_line();
    assert_eq!(buffer.completed_line_count(), 1);
    assert!(buffer.is_current_line_empty());

    // Add another line
    buffer.append_line("World");
    assert_eq!(buffer.completed_line_count(), 2);

    // Pop lines
    let line1 = buffer.pop_completed_line().unwrap();
    assert!(!line1.is_empty());
    assert_eq!(buffer.completed_line_count(), 1);

    let line2 = buffer.pop_completed_line().unwrap();
    assert!(!line2.is_empty());
    assert_eq!(buffer.completed_line_count(), 0);
}

#[test]
fn test_buffer_cursor_movement_workflow() {
    let mut buffer = TerminalBuffer::new_with_size(80, 24);

    // Start at origin
    assert_eq!(buffer.cursor_position(), CursorPosition::new(0, 0));

    // Add text and check cursor movement
    for _ in 0..10 {
        buffer.append_char('X');
    }
    assert_eq!(buffer.cursor_position().col, 10);

    // Carriage return
    buffer.append_char('\r');
    assert_eq!(buffer.cursor_position().col, 0);

    // Line feed
    buffer.append_char('\n');
    assert_eq!(buffer.cursor_position().row, 1);
}

#[test]
fn test_buffer_resize_workflow() {
    let mut buffer = TerminalBuffer::new();

    // Add content
    buffer.append_line("Line 1");
    buffer.append_line("Line 2");
    buffer.append_char('T');
    buffer.append_char('e');
    buffer.append_char('s');
    buffer.append_char('t');

    // Resize
    let old_size = buffer.size();
    buffer.set_size(120, 40);
    let new_size = buffer.size();

    assert_ne!(old_size, new_size);
    assert_eq!(new_size.cols, 120);
    assert_eq!(new_size.rows, 40);

    // Content should be preserved
    assert_eq!(buffer.completed_line_count(), 2);
    assert!(!buffer.is_current_line_empty());
}

#[test]
fn test_buffer_environment_variables_workflow() {
    let mut buffer = TerminalBuffer::new();

    // Set multiple environment variables
    buffer.set_environment("TERM", "xterm-256color");
    buffer.set_environment("USER", "testuser");
    buffer.set_environment("SHELL", "/bin/bash");

    // Retrieve them
    assert_eq!(
        buffer.get_environment("TERM"),
        Some(&"xterm-256color".to_string())
    );
    assert_eq!(
        buffer.get_environment("USER"),
        Some(&"testuser".to_string())
    );
    assert_eq!(
        buffer.get_environment("SHELL"),
        Some(&"/bin/bash".to_string())
    );

    // Count them
    let count = buffer.environment().count();
    assert_eq!(count, 3);
}

// ===== Terminal Codec Integration Tests =====

#[test]
fn test_codec_decode_simple_text() {
    let mut codec = create_test_codec();
    let mut buffer = BytesMut::from("Hello, World!");

    let mut events = Vec::new();
    while let Ok(Some(event)) = codec.decode(&mut buffer) {
        events.push(event);
    }

    // Should have character events for each character
    assert!(!events.is_empty());

    // Verify buffer state
    assert!(!codec.buffer().is_current_line_empty());
}

#[test]
fn test_codec_decode_with_line_feed() {
    let mut codec = create_test_codec();
    let mut buffer = BytesMut::from("Line 1\nLine 2\n");

    let mut line_completed_count = 0;
    while let Ok(Some(event)) = codec.decode(&mut buffer) {
        if matches!(event, TerminalEvent::LineCompleted { .. }) {
            line_completed_count += 1;
        }
    }

    assert_eq!(line_completed_count, 2);
}

#[test]
fn test_codec_decode_control_codes() {
    let mut codec = create_test_codec();

    // Bell
    let mut buffer = BytesMut::from(&[0x07][..]);
    match codec.decode(&mut buffer) {
        Ok(Some(TerminalEvent::Bell)) => {}
        other => panic!("Expected Bell, got {:?}", other),
    }

    // Backspace
    codec.buffer_mut().append_char('A');
    let mut buffer = BytesMut::from(&[0x08][..]);
    match codec.decode(&mut buffer) {
        Ok(Some(TerminalEvent::EraseCharacter { .. })) => {}
        other => panic!("Expected EraseCharacter, got {:?}", other),
    }
}

#[test]
fn test_codec_encode_decode_roundtrip() {
    let mut encode_codec = create_test_codec();
    let mut decode_codec = create_test_codec();

    let test_string = "Test String";
    let mut buffer = BytesMut::new();

    // Encode each character
    for ch in test_string.chars() {
        encode_codec.encode(ch, &mut buffer).unwrap();
    }

    // Decode back
    let mut decoded_chars = Vec::new();
    while let Ok(Some(event)) = decode_codec.decode(&mut buffer) {
        if let TerminalEvent::CharacterData { character, .. } = event {
            decoded_chars.push(character);
        }
    }

    let decoded_string: String = decoded_chars.iter().collect();
    assert_eq!(decoded_string, test_string);
}

#[test]
fn test_codec_buffer_state_consistency() {
    let mut codec = create_test_codec();
    let mut buffer = BytesMut::from("ABC\nDEF\n");

    // Process all data
    while codec.decode(&mut buffer).unwrap().is_some() {}

    // Buffer should have no completed lines (they were popped by LineCompleted events)
    assert_eq!(codec.buffer().completed_line_count(), 0);
}

#[test]
fn test_codec_unicode_handling() {
    let mut codec = create_test_codec();
    let mut buffer = BytesMut::from("Hello 世界 🌍");

    let mut char_count = 0;
    while let Ok(Some(event)) = codec.decode(&mut buffer) {
        if matches!(event, TerminalEvent::CharacterData { .. }) {
            char_count += 1;
        }
    }

    assert!(char_count > 0);
}

#[test]
fn test_codec_mixed_content() {
    let mut codec = create_test_codec();
    let mut buffer = BytesMut::from("Text\x07More\x08\nLine2");

    let mut event_types = Vec::new();
    while let Ok(Some(event)) = codec.decode(&mut buffer) {
        match event {
            TerminalEvent::CharacterData { .. } => event_types.push("char"),
            TerminalEvent::Bell => event_types.push("bell"),
            TerminalEvent::EraseCharacter { .. } => event_types.push("erase"),
            TerminalEvent::LineCompleted { .. } => event_types.push("line"),
            _ => {}
        }
    }

    assert!(event_types.contains(&"char"));
    assert!(event_types.contains(&"bell"));
    assert!(event_types.contains(&"line"));
}

// ===== Terminal Command Integration Tests =====

#[test]
fn test_terminal_commands_encoding() {
    let mut codec = create_test_codec();
    let mut buffer = BytesMut::new();

    let commands = vec![
        TerminalCommand::SendBreak,
        TerminalCommand::SendInterruptProcess,
        TerminalCommand::SendAbortOutput,
        TerminalCommand::SendAreYouThere,
        TerminalCommand::SendEraseCharacter,
        TerminalCommand::SendEraseLine,
    ];

    for cmd in commands {
        codec.encode(&cmd, &mut buffer).unwrap();
    }

    assert!(!buffer.is_empty());
}

// ===== Complex Integration Scenarios =====

#[test]
fn test_full_terminal_session_simulation() {
    let mut codec = create_test_codec();

    // Simulate user typing a command
    let mut buffer = BytesMut::from("ls -la\n");
    let mut events = Vec::new();

    while let Ok(Some(event)) = codec.decode(&mut buffer) {
        events.push(event);
    }

    // Should have character events and a line completed event
    let has_chars = events
        .iter()
        .any(|e| matches!(e, TerminalEvent::CharacterData { .. }));
    let has_line = events
        .iter()
        .any(|e| matches!(e, TerminalEvent::LineCompleted { .. }));

    assert!(has_chars);
    assert!(has_line);
}

#[test]
fn test_terminal_buffer_with_ansi_sequences() {
    let mut buffer = TerminalBuffer::new();

    // Add styled text (the buffer stores SegmentedString which preserves styles)
    buffer.append_line("Normal text");
    buffer.append_line("More text");

    assert_eq!(buffer.completed_line_count(), 2);

    // Get stripped versions
    let stripped = buffer.completed_lines_stripped();
    assert_eq!(stripped.len(), 2);
    assert_eq!(stripped[0], "Normal text");
    assert_eq!(stripped[1], "More text");
}

#[test]
fn test_terminal_size_changes() {
    let mut buffer = TerminalBuffer::new();

    // Initial size
    assert_eq!(buffer.size(), TerminalSize::new(80, 24));

    // Add content at various positions
    buffer.set_cursor_position(40, 12);
    buffer.append_char('X');

    // Resize smaller
    buffer.set_size(60, 20);

    // Cursor should be clamped
    let pos = buffer.cursor_position();
    assert!(pos.col < 60);
    assert!(pos.row < 20);

    // Resize larger
    buffer.set_size(120, 40);
    assert_eq!(buffer.size(), TerminalSize::new(120, 40));
}

#[test]
fn test_codec_error_recovery() {
    let mut codec = create_test_codec();

    // Send valid data
    let mut buffer = BytesMut::from("Valid");
    while codec.decode(&mut buffer).unwrap().is_some() {}

    // Buffer should still be functional
    assert!(!codec.buffer().is_current_line_empty());

    // Continue with more valid data
    let mut buffer = BytesMut::from(" Text");
    while codec.decode(&mut buffer).unwrap().is_some() {}

    assert_eq!(codec.buffer().current_line_length(), 10); // "Valid Text"
}

#[test]
fn test_multiple_line_operations() {
    let mut buffer = TerminalBuffer::new();

    // Add multiple lines
    for i in 0..10 {
        buffer.append_line(&format!("Line {}", i));
    }

    assert_eq!(buffer.completed_line_count(), 10);

    // Take all lines
    let lines = buffer.take_completed_lines();
    assert_eq!(lines.len(), 10);
    assert_eq!(buffer.completed_line_count(), 0);

    // Add more lines
    buffer.append_line("New line 1");
    buffer.append_line("New line 2");
    assert_eq!(buffer.completed_line_count(), 2);

    // Clear all
    buffer.clear();
    assert_eq!(buffer.completed_line_count(), 0);
    assert!(buffer.is_current_line_empty());
}

#[test]
fn test_cursor_position_tracking_through_codec() {
    let mut codec = create_test_codec();
    let mut buffer = BytesMut::from("ABC");

    let mut last_cursor = CursorPosition::new(0, 0);
    while let Ok(Some(event)) = codec.decode(&mut buffer) {
        if let TerminalEvent::CharacterData { cursor, .. } = event {
            // Cursor should advance
            assert!(cursor.col >= last_cursor.col);
            last_cursor = cursor;
        }
    }

    // Final cursor position should reflect the characters added
    assert!(last_cursor.col > 0);
}

#[test]
fn test_terminal_event_sequence() {
    let mut codec = create_test_codec();
    let mut buffer = BytesMut::from("A\x07B\nC");

    let mut event_sequence = Vec::new();
    while let Ok(Some(event)) = codec.decode(&mut buffer) {
        match event {
            TerminalEvent::CharacterData { character, .. } => {
                event_sequence.push(format!("Char({})", character));
            }
            TerminalEvent::Bell => {
                event_sequence.push("Bell".to_string());
            }
            TerminalEvent::LineCompleted { .. } => {
                event_sequence.push("Line".to_string());
            }
            _ => {}
        }
    }

    // Verify sequence: A, Bell, B, Line, C
    assert!(event_sequence.contains(&"Char(A)".to_string()));
    assert!(event_sequence.contains(&"Bell".to_string()));
    assert!(event_sequence.contains(&"Char(B)".to_string()));
    assert!(event_sequence.contains(&"Line".to_string()));
    assert!(event_sequence.contains(&"Char(C)".to_string()));
}
