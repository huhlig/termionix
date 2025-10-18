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

use crate::Connection;
use crate::result::TelnetResult;
use crate::service::TelnetService;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpListener;
use tokio::runtime::Handle;
use tracing::error;

pub struct TelnetServer<S: TelnetService> {
    connections: HashMap<SocketAddr, Connection>,
    listener: TcpListener,
    active: AtomicBool,
    service: Arc<S>,
    handle: Handle,
}

impl<S: TelnetService> TelnetServer<S> {
    pub fn create(listener: TcpListener, service: Arc<S>, handle: Option<Handle>) -> TelnetResult<TelnetServer<S>> {
        Ok(TelnetServer {
            connections: HashMap::default(),
            listener,
            active: AtomicBool::new(true),
            service,
            handle: handle.unwrap_or(Handle::current()),
        })
    }

    pub async fn listen(&mut self) -> TelnetResult<()> {
        tracing::trace!("Listening on {}", self.listener.local_addr()?);
        while self.active.load(Ordering::Relaxed) {
            match self.listener.accept().await {
                Ok((socket, addr)) => {
                    tracing::trace!("Accepted connection from {}", addr);
                    let terminal = Connection::create(self.handle.clone(), socket, self.service.clone());
                    self.service.on_connect(&terminal);
                    self.connections.insert(addr, terminal.clone());
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

impl<S: TelnetService> std::fmt::Debug for TelnetServer<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelnetServer")
            .field("connections", &self.connections.len())
            .field("service", &std::any::type_name_of_val(&self.service))
            .field("address", &self.listener.local_addr().unwrap())
            .field("active", &self.active)
            .finish()
    }
}
