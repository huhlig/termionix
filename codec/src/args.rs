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

use crate::TelnetOption;
use crate::args::naws::WindowSize;
use crate::result::CodecResult;
use bytes::{Buf, BufMut, BytesMut};

pub mod linemode;
pub mod msdp;
pub mod mssp;
pub mod naocrd;
pub mod naohts;
pub mod naws;
pub mod status;

///
/// Telnet Subnegotiation Argument
///
#[derive(Clone, Debug, PartialEq)]
pub enum TelnetArgument {
    /// A subnegotiation for the window size, where the first value is the width
    /// and the second value is the height. The values are in characters.
    NAWSWindowSize(WindowSize),
    /// Indicates an intent to begin CHARSET subnegotiation. This can only be
    /// sent after receiving a DO CHARSET after sending a WILL CHARSET (in any
    /// order).
    CharsetRequest(Vec<BytesMut>),
    /// Indicates that the receiver has accepted the charset request.
    CharsetAccepted(BytesMut),
    /// Indicates that the receiver acknowledges the charset request but will
    /// not use any of the requested characters.
    CharsetRejected,
    /// Indicates that the receiver acknowledges a TTABLE-IS message but is
    /// unable to handle it. This will terminate subnegotiation.
    CharsetTTableRejected,
    /// A subnegotiation for an unknown option.
    Unknown(BytesMut),
}

impl TelnetArgument {
    ///
    /// Get Encoded Length of `TelnetOptionStatus`
    ///
    pub fn encoded_len(&self) -> usize {
        match self {
            TelnetArgument::NAWSWindowSize(inner) => inner.encoded_len(),
            TelnetArgument::Unknown(inner) => inner.len(),
            _ => unimplemented!(),
        }
    }
    ///
    /// Encode `TelnetOptionStatus` to `BufMut`
    ///
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> CodecResult<()> {
        match self {
            TelnetArgument::NAWSWindowSize(inner) => inner.encode(dst),
            TelnetArgument::Unknown(inner) => {
                dst.put(&inner[..]);
                Ok(())
            }
            _ => unimplemented!(),
        }
    }
}
