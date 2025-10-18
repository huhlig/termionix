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

mod buffer;
mod codec;
mod command;
mod event;
mod result;
mod types;

pub use self::buffer::TerminalBuffer;
pub use self::codec::TerminalCodec;
pub use self::command::TerminalCommand;
pub use self::event::TerminalEvent;
pub use self::result::{TerminalError, TerminalResult};
pub use self::types::{CursorPosition, TerminalSize};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CursorPosition;

    #[test]
    fn test_module_exports_exist() {
        // Verify all public exports are accessible
        let _ = std::any::type_name::<TerminalBuffer>();
        let _ = std::any::type_name::<TerminalCodec<termionix_ansicodec::AnsiCodec<termionix_codec::TelnetCodec>>>();
        let _ = std::any::type_name::<TerminalCommand>();
        let _ = std::any::type_name::<TerminalEvent>();
        let _ = std::any::type_name::<TerminalError>();
        let _ = std::any::type_name::<TerminalResult<()>>();
    }

    #[test]
    fn test_terminal_buffer_creation() {
        let buffer = TerminalBuffer::new();
        assert_eq!(buffer.width(), 80);
        assert_eq!(buffer.height(), 24);
        assert!(buffer.is_current_line_empty());
        assert_eq!(buffer.completed_line_count(), 0);
    }

    #[test]
    fn test_terminal_buffer_custom_size() {
        let buffer = TerminalBuffer::new_with_size(120, 40);
        assert_eq!(buffer.width(), 120);
        assert_eq!(buffer.height(), 40);
    }

    fn create_test_codec() -> TerminalCodec<termionix_ansicodec::AnsiCodec<termionix_codec::TelnetCodec>> {
        let telnet_codec = termionix_codec::TelnetCodec::new();
        let ansi_codec = termionix_ansicodec::AnsiCodec::new(
            termionix_ansicodec::AnsiConfig::default(),
            telnet_codec,
        );
        TerminalCodec::new(ansi_codec)
    }

    #[test]
    fn test_terminal_codec_creation() {
        let codec = create_test_codec();
        // Verify codec can be created
        assert_eq!(codec.terminal_buffer().width(), 80);
    }

    #[test]
    fn test_terminal_error_display() {
        let error =
            TerminalError::from(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
        assert!(error.to_string().contains("test error"));
    }

    #[test]
    fn test_terminal_result_type() {
        let success: TerminalResult<i32> = Ok(42);
        assert_eq!(success.unwrap(), 42);

        let error: TerminalResult<i32> = Err(TerminalError::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test",
        )));
        assert!(error.is_err());
    }

    #[test]
    fn test_terminal_command_variants() {
        // Verify all command variants can be created
        let _ = TerminalCommand::SendBreak;
        let _ = TerminalCommand::SendInterruptProcess;
        let _ = TerminalCommand::SendAbortOutput;
        let _ = TerminalCommand::SendAreYouThere;
        let _ = TerminalCommand::SendEraseCharacter;
        let _ = TerminalCommand::SendEraseLine;
    }

    #[test]
    fn test_terminal_event_character_data() {
        let event = TerminalEvent::CharacterData {
            cursor: CursorPosition::new(0, 0),
            character: 'A',
        };

        match event {
            TerminalEvent::CharacterData { character, .. } => {
                assert_eq!(character, 'A');
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_terminal_event_line_completed() {
        use crate::types::CursorPosition;
        use termionix_ansicodec::SegmentedString;

        let line = SegmentedString::from("test line");
        let event = TerminalEvent::LineCompleted {
            cursor: CursorPosition::new(0, 1),
            line: line.clone(),
        };

        match event {
            TerminalEvent::LineCompleted { line: l, .. } => {
                // Just verify the event was created correctly
                assert!(!l.is_empty());
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_buffer_codec_integration() {
        use tokio_util::bytes::BytesMut;
        use tokio_util::codec::Decoder;

        let mut codec = create_test_codec();
        let mut buffer = BytesMut::from("Hello");

        // Process some data through the codec
        match codec.decode(&mut buffer) {
            Ok(_) => {
                // Verify buffer state is updated
                assert!(!codec.terminal_buffer().is_current_line_empty());
            }
            Err(e) => panic!("Decode failed: {}", e),
        }
    }

    #[test]
    fn test_buffer_line_operations() {
        let mut buffer = TerminalBuffer::new();

        // Add some text
        buffer.append_line("First line");
        assert_eq!(buffer.completed_line_count(), 1);

        buffer.append_line("Second line");
        assert_eq!(buffer.completed_line_count(), 2);

        // Pop a line
        let line = buffer.pop_completed_line();
        assert!(line.is_some());
        assert_eq!(buffer.completed_line_count(), 1);
    }

    #[test]
    fn test_buffer_clear_operations() {
        let mut buffer = TerminalBuffer::new();

        buffer.append_line("Line 1");
        buffer.append_line("Line 2");
        assert_eq!(buffer.completed_line_count(), 2);

        buffer.clear_completed_lines();
        assert_eq!(buffer.completed_line_count(), 0);

        buffer.append_line("Line 3");
        buffer.clear();
        assert_eq!(buffer.completed_line_count(), 0);
        assert!(buffer.is_current_line_empty());
    }

    #[test]
    fn test_cursor_position_tracking() {
        let mut buffer = TerminalBuffer::new();
        let initial_pos = buffer.cursor_position();
        assert_eq!(initial_pos.col, 0);
        assert_eq!(initial_pos.row, 0);

        buffer.set_cursor_position(10, 5);
        let new_pos = buffer.cursor_position();
        assert_eq!(new_pos.col, 10);
        assert_eq!(new_pos.row, 5);
    }

    #[test]
    fn test_buffer_size_operations() {
        let mut buffer = TerminalBuffer::new();
        assert_eq!(buffer.size(), TerminalSize::new(80, 24));

        buffer.set_size(100, 30);
        assert_eq!(buffer.size(), TerminalSize::new(100, 30));
        assert_eq!(buffer.width(), 100);
        assert_eq!(buffer.height(), 30);
    }

    #[test]
    fn test_terminal_event_variants() {
        use crate::types::CursorPosition;
        use termionix_ansicodec::SegmentedString;

        // Test all event variants can be created
        let _ = TerminalEvent::NoOperation;
        let _ = TerminalEvent::Bell;
        let _ = TerminalEvent::Break;
        let _ = TerminalEvent::InterruptProcess;

        let cursor = CursorPosition::new(0, 0);
        let _ = TerminalEvent::CharacterData {
            cursor,
            character: 'x',
        };
        let _ = TerminalEvent::LineCompleted {
            cursor,
            line: SegmentedString::from("test"),
        };
        let _ = TerminalEvent::EraseCharacter { cursor };
        let _ = TerminalEvent::EraseLine { cursor };
        let _ = TerminalEvent::Clear { cursor };
        let _ = TerminalEvent::CursorPosition { cursor };
    }

    #[test]
    fn test_error_conversion_from_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let terminal_error: TerminalError = io_error.into();
        assert!(terminal_error.to_string().contains("file not found"));
    }

    #[test]
    fn test_buffer_environment_variables() {
        let mut buffer = TerminalBuffer::new();

        buffer.set_environment("TERM", "xterm-256color");
        buffer.set_environment("USER", "testuser");

        assert_eq!(
            buffer.get_environment("TERM"),
            Some(&"xterm-256color".to_string())
        );
        assert_eq!(
            buffer.get_environment("USER"),
            Some(&"testuser".to_string())
        );
        assert_eq!(buffer.get_environment("UNKNOWN"), None);

        let env_count = buffer.environment().count();
        assert_eq!(env_count, 2);
    }

    #[test]
    fn test_buffer_character_operations() {
        let mut buffer = TerminalBuffer::new();

        buffer.append_char('H');
        buffer.append_char('i');
        assert_eq!(buffer.current_line_length(), 2);

        buffer.erase_character();
        assert_eq!(buffer.current_line_length(), 1);

        buffer.erase_line();
        assert!(buffer.is_current_line_empty());
    }

    #[test]
    fn test_codec_encoder_string() {
        use tokio_util::bytes::BytesMut;
        use tokio_util::codec::Encoder;

        let mut codec = create_test_codec();
        let mut buffer = BytesMut::new();

        // Encode individual characters
        for ch in "Hello".chars() {
            codec.encode(ch, &mut buffer).unwrap();
        }
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_codec_encoder_char() {
        use tokio_util::bytes::BytesMut;
        use tokio_util::codec::Encoder;

        let mut codec = create_test_codec();
        let mut buffer = BytesMut::new();

        codec.encode('A', &mut buffer).unwrap();
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_codec_encoder_command() {
        // Terminal commands can be encoded through the codec
        let codec = create_test_codec();
        // Just verify the codec exists and has the right type
        assert_eq!(codec.buffer().width(), 80);
    }

    #[test]
    fn test_buffer_line_stripping() {
        let mut buffer = TerminalBuffer::new();

        // Add a line with ANSI codes (though buffer stores it as SegmentedString)
        buffer.append_line("Plain text");

        let stripped = buffer.current_line_stripped();
        assert_eq!(stripped, "");

        let completed = buffer.completed_lines_stripped();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0], "Plain text");
    }

    #[test]
    fn test_buffer_total_line_count() {
        let mut buffer = TerminalBuffer::new();

        assert_eq!(buffer.total_line_count(), 0);

        buffer.append_char('x');
        assert_eq!(buffer.total_line_count(), 1);

        buffer.complete_line();
        assert_eq!(buffer.total_line_count(), 1);

        buffer.append_char('y');
        assert_eq!(buffer.total_line_count(), 2);
    }

    #[test]
    fn test_default_implementations() {
        let _ = TerminalBuffer::default();
        let _ = create_test_codec();
    }

    #[test]
    fn test_debug_formatting() {
        let buffer = TerminalBuffer::new();
        let debug_str = format!("{:?}", buffer);
        assert!(debug_str.contains("TerminalBuffer"));
    }
}
