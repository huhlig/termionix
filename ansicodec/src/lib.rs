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

pub mod ansi;
mod codec;
mod config;
mod consts;
mod parser;
mod result;
mod string;
mod style;
pub mod utility;

pub use self::codec::AnsiCodec;
pub use self::config::{AnsiConfig, ColorMode};
pub use self::parser::AnsiParser;
pub use self::result::{AnsiError, AnsiResult};
pub use self::string::{Segment, SegmentedString};

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        //
    }
}
