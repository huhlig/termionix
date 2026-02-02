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

//! Client connection wrapper

use crate::{ClientConfig, ClientError, Result};
use std::any::Any;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use termionix_telnetcodec::{TelnetEvent, TelnetOption};
use tokio::sync::{mpsc, RwLock};
use tracing::debug;

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting to server
    Connecting,
    /// Connected and active
    Connected,
    /// Reconnecting after logout.txt
    Reconnecting,
    /// Shutting down
    ShuttingDown,
}

/// Client connection wrapper
///
/// Provides a high-level interface for interacting with a Telnet connection.
pub struct ClientConnection {
    inner: Arc<ConnectionInner>,
}

struct ConnectionInner {
    config: ClientConfig,
    state: RwLock<ConnectionState>,
    server_addr: RwLock<Option<SocketAddr>>,
    connected_at: RwLock<Option<Instant>>,
    last_activity: RwLock<Instant>,
    metadata: RwLock<HashMap<String, Arc<dyn Any + Send + Sync>>>,
    tx: mpsc::UnboundedSender<ClientCommand>,
    local_options: RwLock<HashMap<TelnetOption, bool>>,
    remote_options: RwLock<HashMap<TelnetOption, bool>>,
}

/// Commands sent to connection worker
#[derive(Debug)]
pub(crate) enum ClientCommand {
    SendData(Vec<u8>),
    SendEvent(TelnetEvent),
    Disconnect,
    UpdateWindowSize(u16, u16),
}

impl ClientConnection {
    pub(crate) fn new(config: ClientConfig, tx: mpsc::UnboundedSender<ClientCommand>) -> Self {
        Self {
            inner: Arc::new(ConnectionInner {
                config,
                state: RwLock::new(ConnectionState::Disconnected),
                server_addr: RwLock::new(None),
                connected_at: RwLock::new(None),
                last_activity: RwLock::new(Instant::now()),
                metadata: RwLock::new(HashMap::new()),
                tx,
                local_options: RwLock::new(HashMap::new()),
                remote_options: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub async fn state(&self) -> ConnectionState {
        *self.inner.state.read().await
    }

    pub async fn is_connected(&self) -> bool {
        *self.inner.state.read().await == ConnectionState::Connected
    }

    pub async fn send_bytes(&self, data: &[u8]) -> Result<()> {
        self.inner
            .tx
            .send(ClientCommand::SendData(data.to_vec()))
            .map_err(|_| ClientError::NotConnected)?;
        *self.inner.last_activity.write().await = Instant::now();
        Ok(())
    }

    pub async fn send(&self, text: &str) -> Result<()> {
        self.send_bytes(text.as_bytes()).await
    }

    pub async fn send_line(&self, text: &str) -> Result<()> {
        let mut data = text.as_bytes().to_vec();
        data.extend_from_slice(b"\r\n");
        self.send_bytes(&data).await
    }

    pub async fn disconnect(&self) -> Result<()> {
        self.inner
            .tx
            .send(ClientCommand::Disconnect)
            .map_err(|_| ClientError::NotConnected)?;
        *self.inner.state.write().await = ConnectionState::ShuttingDown;
        Ok(())
    }

    pub async fn set_data<T: Any + Send + Sync + Clone>(&self, key: &str, value: T) {
        self.inner
            .metadata
            .write()
            .await
            .insert(key.to_string(), Arc::new(value));
    }

    pub async fn get_data<T: Any + Send + Sync + Clone>(&self, key: &str) -> Option<T> {
        self.inner
            .metadata
            .read()
            .await
            .get(key)
            .and_then(|v| v.downcast_ref::<T>())
            .cloned()
    }

    pub async fn is_local_option_enabled(&self, option: TelnetOption) -> bool {
        self.inner
            .local_options
            .read()
            .await
            .get(&option)
            .copied()
            .unwrap_or(false)
    }

    pub fn config(&self) -> &ClientConfig {
        &self.inner.config
    }

    pub(crate) async fn set_state(&self, state: ConnectionState) {
        *self.inner.state.write().await = state;
    }

    pub(crate) async fn set_connected(&self) {
        *self.inner.connected_at.write().await = Some(Instant::now());
        *self.inner.state.write().await = ConnectionState::Connected;
    }

    pub(crate) async fn set_local_option(&self, option: TelnetOption, enabled: bool) {
        debug!("Local option {:?} set to {}", option, enabled);
        self.inner
            .local_options
            .write()
            .await
            .insert(option, enabled);
    }

    pub(crate) async fn set_remote_option(&self, option: TelnetOption, enabled: bool) {
        debug!("Remote option {:?} set to {}", option, enabled);
        self.inner
            .remote_options
            .write()
            .await
            .insert(option, enabled);
    }
}

impl Clone for ClientConnection {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}
