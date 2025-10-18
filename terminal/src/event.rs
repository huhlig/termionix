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

use crate::types::{CursorPosition, TerminalSize};
use enum_kinds::EnumKind;
use termionix_ansicodec::SegmentedString;
use termionix_codec::msdp::MudServerData;
use termionix_codec::mssp::MudServerStatus;
use termionix_codec::status::TelnetOptionStatus;

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
    // Telnet Subnegotiation Passthrough
    /// Telnet Status
    TelnetOptionStatus(TelnetOptionStatus),
    /// Mud Server Data
    MudServerData(MudServerData),
    /// Mud Server Status
    MudServerStatus(MudServerStatus),
    // TODO: Add More Sidechannel Data Types
}
