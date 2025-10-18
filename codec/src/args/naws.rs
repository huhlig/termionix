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

//! Negotiate About Window Size
//!

use crate::{
    CodecError, CodecResult, consts,
    msdp::{MudServerDataArray, MudServerDataTable, MudServerDataValue},
};
use bytes::{Buf, BufMut};

///
#[derive(Clone, Debug, PartialEq)]
pub struct WindowSize {
    /// Window Columns
    pub cols: u16,
    /// Window Rows
    pub rows: u16,
}

impl WindowSize {
    ///
    pub fn new(cols: u16, rows: u16) -> Self {
        WindowSize { cols, rows }
    }
}

impl WindowSize {
    ///
    /// Get Encoded Length of `WindowSize`
    ///
    pub fn encoded_len(&self) -> usize {
        4
    }
    ///
    /// Encode `WindowSize` to `BufMut`
    ///
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> CodecResult<()> {
        dst.put_u16(self.cols);
        dst.put_u16(self.rows);
        Ok(())
    }
    ///
    /// Decode `WindowSize` from `Buf`
    ///
    pub fn decode<T: Buf>(src: &mut T) -> CodecResult<WindowSize> {
        // NAWS format: WIDTH-HIGH WIDTH-LOW HEIGHT-HIGH HEIGHT-LOW
        if src.remaining() >= 4 {
            Ok(WindowSize {
                cols: src.get_u16(),
                rows: src.get_u16(),
            })
        } else {
            Err(CodecError::SubnegotiationError(String::from(
                "WindowSize decode error",
            )))
        }
    }
}

impl Default for WindowSize {
    fn default() -> Self {
        WindowSize { cols: 80, rows: 24 }
    }
}
