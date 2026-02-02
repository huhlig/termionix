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

use super::TelnetOption;
use crate::args::TelnetArgument;
use crate::options::TelnetSide;

///
/// `TelnetEvent` represents user-facing events from the Telnet codec.
/// Unlike `TelnetFrame` which includes low-level sidechannel frames (DO/DONT/WILL/WONT),
/// `TelnetEvent` emits high-level events like `OptionStatus` when negotiation completes.
///
#[derive(Clone, Debug, PartialEq)]
pub enum TelnetEvent {
    /// Telnet Data Byte
    Data(u8),
    /// No Operation
    NoOperation,
    /// End of urgent Data Stream
    DataMark,
    /// Operator pressed the Break key or the Attention key.
    Break,
    /// Interrupt current process.
    InterruptProcess,
    /// Cancel output from the current process.
    AbortOutput,
    /// Request acknowledgment.
    AreYouThere,
    /// Request that the operator erase the previous character.
    EraseCharacter,
    /// Request that the operator erase the previous line.
    EraseLine,
    /// End of input for half-duplex connections.
    GoAhead,
    /// End of Record - marks the end of a prompt
    EndOfRecord,
    /// Indicate a completed Negotiation
    /// Parameters: (option, side, enabled)
    /// - option: The telnet option that was negotiated
    /// - side: Whether this is Local or Remote
    /// - enabled: true if option is now enabled, false if disabled
    OptionStatus(TelnetOption, TelnetSide, bool),
    /// Subnegotiation Payload
    Subnegotiate(TelnetArgument),
}
