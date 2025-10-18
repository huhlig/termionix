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

use std::io::Error;
use crate::service::TelnetService;
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, atomic::AtomicBool};
use std::task::{Context, Poll};
use termionix_terminal::{TerminalCodec, TerminalEvent, TerminalResult};
use tokio::{net::TcpStream, runtime::Handle, sync::Mutex};
use tokio_util::codec::Framed;
use termionix_compress::{Algorithm, CompressionStream};

/// Represents a connection to a client
#[derive(Clone)]
pub struct Connection {
    framed: Arc<Mutex<Framed<CompressionStream<TcpStream>, TerminalCodec>>>,
    active: Arc<AtomicBool>,
    address: SocketAddr,
}

impl Connection {
    pub(crate) fn create<S: TelnetService>(
        handle: Handle,
        socket: TcpStream,
        service: Arc<S>,
    ) -> Connection {
        let address = socket.peer_addr().expect("Unable to get peer Address");
        let stream = CompressionStream::new(socket, Algorithm::None);
        let framed = Arc::new(Mutex::new(Framed::new(stream, TerminalCodec::new())));
        let active = Arc::new(AtomicBool::new(true));

        let t = Connection {
            framed: framed.clone(),
            active: active.clone(),
            address,
        };

        // Handle Incoming Data and persist it to the buffer
        let terminal = t.clone();
        handle.spawn(async move {
            let framed = framed.clone();
            let active = active.clone();
            while active.load(Relaxed) {
                let mut lock = framed.lock().await;
                if let Some(result) = lock.next().await {
                    match result {
                        Ok(event) => match event {
                            TerminalEvent::LineCompleted { line, .. } => {
                                service.on_message(&terminal, line);
                            }
                            _ => service.on_update(&terminal, event),
                        },
                        Err(err) => {
                            service.on_error(&terminal, err);
                        }
                    }
                }
            }
        });

        t
    }
    pub async fn send(&self, msg: &str) -> TerminalResult<()> {
        self.framed.lock().await.send(msg).await
    }
    pub async fn next(&self) -> Option<TerminalResult<TerminalEvent>> {
        self.framed.lock().await.next().await
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelnetConnection")
            .field("address", &self.address)
            .field("active", &self.active.load(Relaxed))
            .finish()
    }
}
