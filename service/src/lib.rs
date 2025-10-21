//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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
mod client;
mod connection;
mod result;
mod server;
mod utility;

pub use self::buffer::TerminalBuffer;
pub use self::client::TelnetClient;
pub use self::connection::TelnetConnection;
pub use self::result::{TelnetError, TelnetResult};
pub use self::server::TelnetServer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = (2 + 2);
        assert_eq!(result, 4);
    }
}
