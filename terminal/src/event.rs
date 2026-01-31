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
use enum_kinds::EnumKind;
use termionix_ansicodec::SegmentedString;
use termionix_telnetcodec::msdp::MudServerData;
use termionix_telnetcodec::mssp::MudServerStatus;
use termionix_telnetcodec::status::TelnetOptionStatus;

/// Terminal Events
#[derive(Clone, Debug, EnumKind)]
#[enum_kind(TerminalEventKind)]
pub enum TerminalEvent {
    /// Character Received
    CharacterData {
        cursor: CursorPosition,
        character: char,
    },
    /// Line Created
    LineCompleted {
        cursor: CursorPosition,
        line: SegmentedString,
    },
    /// Trigger a bell or sound
    Bell,
    /// Clear Screen
    Clear {
        cursor: CursorPosition,
    },
    /// Erase Line
    EraseLine {
        cursor: CursorPosition,
    },
    /// Backspace
    EraseCharacter {
        cursor: CursorPosition,
    },
    /// NoOp
    NoOperation,
    Break,
    InterruptProcess,
    CursorPosition {
        cursor: CursorPosition,
    },
    ResizeWindow {
        old: TerminalSize,
        new: TerminalSize,
    },
    /// Window size changed (NAWS)
    WindowSize {
        width: u16,
        height: u16,
    },
    /// Terminal type received
    TerminalType {
        terminal_type: String,
    },
    /// Connection disconnected
    Disconnected,
    // Telnet Subnegotiation Passthrough
    /// Telnet Status
    TelnetOptionStatus(TelnetOptionStatus),
    /// Mud Server Data
    MudServerData(MudServerData),
    /// Mud Server Status
    MudServerStatus(MudServerStatus),
    // TODO: Add More Sidechannel Data Types
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_character_data() {
        let event = TerminalEvent::CharacterData {
            cursor: CursorPosition::new(5, 10),
            character: 'A',
        };

        match event {
            TerminalEvent::CharacterData { cursor, character } => {
                assert_eq!(cursor.col, 5);
                assert_eq!(cursor.row, 10);
                assert_eq!(character, 'A');
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_line_completed() {
        let line = SegmentedString::from("test line");
        let event = TerminalEvent::LineCompleted {
            cursor: CursorPosition::new(0, 1),
            line: line.clone(),
        };

        match event {
            TerminalEvent::LineCompleted { cursor, line: l } => {
                assert_eq!(cursor.col, 0);
                assert_eq!(cursor.row, 1);
                assert!(!l.is_empty());
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_bell() {
        let event = TerminalEvent::Bell;
        assert!(matches!(event, TerminalEvent::Bell));
    }

    #[test]
    fn test_event_clear() {
        let event = TerminalEvent::Clear {
            cursor: CursorPosition::new(0, 0),
        };

        match event {
            TerminalEvent::Clear { cursor } => {
                assert_eq!(cursor.col, 0);
                assert_eq!(cursor.row, 0);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_erase_line() {
        let event = TerminalEvent::EraseLine {
            cursor: CursorPosition::new(10, 5),
        };

        match event {
            TerminalEvent::EraseLine { cursor } => {
                assert_eq!(cursor.col, 10);
                assert_eq!(cursor.row, 5);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_erase_character() {
        let event = TerminalEvent::EraseCharacter {
            cursor: CursorPosition::new(15, 8),
        };

        match event {
            TerminalEvent::EraseCharacter { cursor } => {
                assert_eq!(cursor.col, 15);
                assert_eq!(cursor.row, 8);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_no_operation() {
        let event = TerminalEvent::NoOperation;
        assert!(matches!(event, TerminalEvent::NoOperation));
    }

    #[test]
    fn test_event_break() {
        let event = TerminalEvent::Break;
        assert!(matches!(event, TerminalEvent::Break));
    }

    #[test]
    fn test_event_interrupt_process() {
        let event = TerminalEvent::InterruptProcess;
        assert!(matches!(event, TerminalEvent::InterruptProcess));
    }

    #[test]
    fn test_event_cursor_position() {
        let event = TerminalEvent::CursorPosition {
            cursor: CursorPosition::new(20, 15),
        };

        match event {
            TerminalEvent::CursorPosition { cursor } => {
                assert_eq!(cursor.col, 20);
                assert_eq!(cursor.row, 15);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_resize_window() {
        let old = TerminalSize::new(80, 24);
        let new = TerminalSize::new(120, 40);
        let event = TerminalEvent::ResizeWindow { old, new };

        match event {
            TerminalEvent::ResizeWindow { old: o, new: n } => {
                assert_eq!(o.cols, 80);
                assert_eq!(o.rows, 24);
                assert_eq!(n.cols, 120);
                assert_eq!(n.rows, 40);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_window_size() {
        let event = TerminalEvent::WindowSize {
            width: 100,
            height: 50,
        };

        match event {
            TerminalEvent::WindowSize { width, height } => {
                assert_eq!(width, 100);
                assert_eq!(height, 50);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_terminal_type() {
        let event = TerminalEvent::TerminalType {
            terminal_type: "xterm-256color".to_string(),
        };

        match event {
            TerminalEvent::TerminalType { terminal_type } => {
                assert_eq!(terminal_type, "xterm-256color");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_disconnected() {
        let event = TerminalEvent::Disconnected;
        assert!(matches!(event, TerminalEvent::Disconnected));
    }

    #[test]
    fn test_event_clone() {
        let event1 = TerminalEvent::Bell;
        let event2 = event1.clone();

        assert!(matches!(event1, TerminalEvent::Bell));
        assert!(matches!(event2, TerminalEvent::Bell));
    }

    #[test]
    fn test_event_debug() {
        let event = TerminalEvent::Bell;
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("Bell"));
    }

    #[test]
    fn test_event_character_data_unicode() {
        let event = TerminalEvent::CharacterData {
            cursor: CursorPosition::new(0, 0),
            character: '世',
        };

        match event {
            TerminalEvent::CharacterData { character, .. } => {
                assert_eq!(character, '世');
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_character_data_special_chars() {
        let special_chars = vec!['\n', '\r', '\t', '\0', ' '];

        for ch in special_chars {
            let event = TerminalEvent::CharacterData {
                cursor: CursorPosition::new(0, 0),
                character: ch,
            };

            match event {
                TerminalEvent::CharacterData { character, .. } => {
                    assert_eq!(character, ch);
                }
                _ => panic!("Wrong event type"),
            }
        }
    }

    #[test]
    fn test_event_kind_enum() {
        // Test that EnumKind derive works
        let event = TerminalEvent::Bell;
        let kind = TerminalEventKind::from(&event);
        assert!(matches!(kind, TerminalEventKind::Bell));
    }

    #[test]
    fn test_all_event_variants_have_kinds() {
        // Verify all event variants can be converted to kinds
        let events = vec![
            TerminalEvent::CharacterData {
                cursor: CursorPosition::new(0, 0),
                character: 'A',
            },
            TerminalEvent::LineCompleted {
                cursor: CursorPosition::new(0, 0),
                line: SegmentedString::from("test"),
            },
            TerminalEvent::Bell,
            TerminalEvent::Clear {
                cursor: CursorPosition::new(0, 0),
            },
            TerminalEvent::EraseLine {
                cursor: CursorPosition::new(0, 0),
            },
            TerminalEvent::EraseCharacter {
                cursor: CursorPosition::new(0, 0),
            },
            TerminalEvent::NoOperation,
            TerminalEvent::Break,
            TerminalEvent::InterruptProcess,
            TerminalEvent::CursorPosition {
                cursor: CursorPosition::new(0, 0),
            },
            TerminalEvent::ResizeWindow {
                old: TerminalSize::new(80, 24),
                new: TerminalSize::new(100, 30),
            },
            TerminalEvent::WindowSize {
                width: 80,
                height: 24,
            },
            TerminalEvent::TerminalType {
                terminal_type: "xterm".to_string(),
            },
            TerminalEvent::Disconnected,
        ];

        for event in events {
            let _kind = TerminalEventKind::from(&event);
        }
    }

    #[test]
    fn test_event_match_exhaustive() {
        let events = vec![
            TerminalEvent::Bell,
            TerminalEvent::NoOperation,
            TerminalEvent::Break,
            TerminalEvent::InterruptProcess,
            TerminalEvent::Disconnected,
        ];

        for event in events {
            let _result = match event {
                TerminalEvent::CharacterData { .. } => "char",
                TerminalEvent::LineCompleted { .. } => "line",
                TerminalEvent::Bell => "bell",
                TerminalEvent::Clear { .. } => "clear",
                TerminalEvent::EraseLine { .. } => "erase_line",
                TerminalEvent::EraseCharacter { .. } => "erase_char",
                TerminalEvent::NoOperation => "noop",
                TerminalEvent::Break => "break",
                TerminalEvent::InterruptProcess => "interrupt",
                TerminalEvent::CursorPosition { .. } => "cursor",
                TerminalEvent::ResizeWindow { .. } => "resize",
                TerminalEvent::WindowSize { .. } => "window_size",
                TerminalEvent::TerminalType { .. } => "term_type",
                TerminalEvent::Disconnected => "disconnected",
                TerminalEvent::TelnetOptionStatus(_) => "telnet_status",
                TerminalEvent::MudServerData(_) => "msdp",
                TerminalEvent::MudServerStatus(_) => "mssp",
            };
        }
    }
}
