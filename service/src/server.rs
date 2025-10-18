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

use crate::TelnetConnection;
use crate::result::TelnetResult;
use futures_util::StreamExt;
use std::any::Any;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use termionix_codec::TelnetCodec;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;
use tracing::error;

pub trait Subscriber {
    fn subscribe(&self, connection: TelnetConnection);
}

pub struct TelnetServer<S: Subscriber> {
    connections: HashMap<String, TelnetConnection>,
    subscriber: Arc<S>,
    listener: TcpListener,
    active: AtomicBool,
}

impl<S: Subscriber> TelnetServer<S> {
    pub fn create(listener: TcpListener, subscriber: S) -> TelnetResult<TelnetServer<S>> {
        Ok(TelnetServer {
            connections: HashMap::default(),
            subscriber: Arc::new(subscriber),
            listener,
            active: AtomicBool::new(true),
        })
    }

    pub async fn listen(&mut self) -> TelnetResult<()> {
        tracing::trace!("Listening on {}", self.listener.local_addr()?);
        while self.active.load(std::sync::atomic::Ordering::Relaxed) {
            match self.listener.accept().await {
                Ok((socket, addr)) => {
                    tracing::trace!("Accepted connection from {}", addr);
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

                    self.subscriber.subscribe(connection);
                }
                Err(e) => {
                    error!("Failed to accept incoming connection: {}", e);
                    continue;
                }
            };
        }
        Ok(())
    }

    /// Get Server `SocketAddr`
    pub fn addr(&self) -> SocketAddr {
        self.listener.local_addr().unwrap()
    }
}

impl<S: Subscriber> std::fmt::Debug for TelnetServer<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelnetServer")
            .field("connections", &self.connections.len())
            .field("subscriber", &std::any::type_name_of_val(&self.subscriber))
            .field("address", &self.listener.local_addr().unwrap())
            .field("active", &self.active)
            .finish()
    }
}
