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

///
/// `TelnetInput` is an input to the telnet sidechannel.
///
#[derive(Clone, Debug, PartialEq)]
pub enum TelnetInput {
    /// Telnet Keypress
    Keypress(u8),
    /// Telnet Message
    Message(String),
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
    /// Request Enabling Telnet Option
    Enable(TelnetOption),
    /// Request Disabling Telnet Option
    Disable(TelnetOption),
    /// Subnegotiation Payload
    Subnegotiate(TelnetArgument),
}
