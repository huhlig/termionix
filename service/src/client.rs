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

use crate::service::TelnetService;
use crate::{TelnetResult, Connection};
use std::sync::Arc;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::runtime::Handle;

pub struct TelnetClient;

impl TelnetClient {
    /// Connect to a Remote Telnet Server
    pub async fn connect<A: ToSocketAddrs, S: TelnetService>(
        addr: A,
        service: Arc<S>,
        handle: Option<Handle>,
    ) -> TelnetResult<Connection> {
        let socket = TcpStream::connect(addr).await?;
        tracing::trace!("Connected to {}", socket.peer_addr()?);
        let terminal = Connection::create(
            handle.unwrap_or_else(|| Handle::current()),
            socket,
            service.clone(),
        );
        service.on_connect(&terminal);
        Ok(terminal)
    }
}
