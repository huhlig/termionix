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

use crate::{TelnetConnection, TelnetResult};
use futures_util::StreamExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use termionix_codec::TelnetCodec;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::sync::mpsc;
use tokio_util::codec::Framed;

pub struct TelnetClient;

impl TelnetClient {
    /// Connect to a Remote Telnet Server
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> TelnetResult<TelnetConnection> {
        let socket = TcpStream::connect(addr).await?;
        tracing::trace!("Connected to {}", socket.peer_addr()?);
        TelnetClient::wrap(socket).await
    }

    /// Wrap a socket in a Telnet Connection
    pub async fn wrap(socket: TcpStream) -> TelnetResult<TelnetConnection> {
        let address = socket.peer_addr().expect("Unable to get peer Address");
        let framed = Framed::new(socket, TelnetCodec::new());
        let (mut writer, mut reader) = framed.split();
        let (send, recv) = mpsc::channel(50);
        let active = Arc::new(AtomicBool::new(true));
        let connection = TelnetConnection::wrap(address, writer, active.clone(), recv);

        tokio::spawn(async move {
            while active.load(Ordering::Relaxed) {
                while let Some(Ok(frame)) = reader.next().await {
                    send.send(frame)
                        .await
                        .expect("Unable to send frame to connection");
                }
            }
        });
        Ok(connection)
    }
}
