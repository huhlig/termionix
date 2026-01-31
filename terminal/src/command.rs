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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TerminalCommand {
    SendBreak,
    SendInterruptProcess,
    SendAbortOutput,
    SendAreYouThere,
    SendEraseCharacter,
    SendEraseLine,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_send_break() {
        let cmd = TerminalCommand::SendBreak;
        assert_eq!(cmd, TerminalCommand::SendBreak);
    }

    #[test]
    fn test_command_send_interrupt_process() {
        let cmd = TerminalCommand::SendInterruptProcess;
        assert_eq!(cmd, TerminalCommand::SendInterruptProcess);
    }

    #[test]
    fn test_command_send_abort_output() {
        let cmd = TerminalCommand::SendAbortOutput;
        assert_eq!(cmd, TerminalCommand::SendAbortOutput);
    }

    #[test]
    fn test_command_send_are_you_there() {
        let cmd = TerminalCommand::SendAreYouThere;
        assert_eq!(cmd, TerminalCommand::SendAreYouThere);
    }

    #[test]
    fn test_command_send_erase_character() {
        let cmd = TerminalCommand::SendEraseCharacter;
        assert_eq!(cmd, TerminalCommand::SendEraseCharacter);
    }

    #[test]
    fn test_command_send_erase_line() {
        let cmd = TerminalCommand::SendEraseLine;
        assert_eq!(cmd, TerminalCommand::SendEraseLine);
    }

    #[test]
    fn test_command_clone() {
        let cmd1 = TerminalCommand::SendBreak;
        let cmd2 = cmd1.clone();
        assert_eq!(cmd1, cmd2);
    }

    #[test]
    fn test_command_copy() {
        let cmd1 = TerminalCommand::SendBreak;
        let cmd2 = cmd1; // Copy
        assert_eq!(cmd1, cmd2);
    }

    #[test]
    fn test_command_debug() {
        let cmd = TerminalCommand::SendBreak;
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("SendBreak"));
    }

    #[test]
    fn test_command_equality() {
        let cmd1 = TerminalCommand::SendBreak;
        let cmd2 = TerminalCommand::SendBreak;
        let cmd3 = TerminalCommand::SendInterruptProcess;

        assert_eq!(cmd1, cmd2);
        assert_ne!(cmd1, cmd3);
    }

    #[test]
    fn test_all_command_variants() {
        let commands = vec![
            TerminalCommand::SendBreak,
            TerminalCommand::SendInterruptProcess,
            TerminalCommand::SendAbortOutput,
            TerminalCommand::SendAreYouThere,
            TerminalCommand::SendEraseCharacter,
            TerminalCommand::SendEraseLine,
        ];

        assert_eq!(commands.len(), 6);

        // Verify all are unique
        for (i, cmd1) in commands.iter().enumerate() {
            for (j, cmd2) in commands.iter().enumerate() {
                if i == j {
                    assert_eq!(cmd1, cmd2);
                } else {
                    assert_ne!(cmd1, cmd2);
                }
            }
        }
    }

    #[test]
    fn test_command_match_pattern() {
        let cmd = TerminalCommand::SendBreak;

        match cmd {
            TerminalCommand::SendBreak => {
                // Success
            }
            _ => panic!("Pattern match failed"),
        }
    }

    #[test]
    fn test_command_exhaustive_match() {
        let commands = vec![
            TerminalCommand::SendBreak,
            TerminalCommand::SendInterruptProcess,
            TerminalCommand::SendAbortOutput,
            TerminalCommand::SendAreYouThere,
            TerminalCommand::SendEraseCharacter,
            TerminalCommand::SendEraseLine,
        ];

        for cmd in commands {
            let _result = match cmd {
                TerminalCommand::SendBreak => "break",
                TerminalCommand::SendInterruptProcess => "interrupt",
                TerminalCommand::SendAbortOutput => "abort",
                TerminalCommand::SendAreYouThere => "ayt",
                TerminalCommand::SendEraseCharacter => "erase_char",
                TerminalCommand::SendEraseLine => "erase_line",
            };
        }
    }
}
