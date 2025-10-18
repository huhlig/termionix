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

//! Connection manager implementation
//!
//! The ConnectionManager is responsible for:
//! - Managing all active connections
//! - Spawning and tracking connection workers
//! - Broadcasting messages to all connections
//! - Graceful shutdown coordination
//! - Connection lifecycle tracking

use crate::{
    ConnectionId, ConnectionInfo, ConnectionState, ControlMessage, Result, ServerHandler,
    ServerMetrics, TelnetConnection, TelnetError, WorkerConfig,
};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use termionix_terminal::TerminalCommand;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Result of a broadcast operation
#[derive(Debug, Clone)]
pub struct BroadcastResult {
    /// Total number of connections attempted
    pub total: usize,
    /// Number of successful sends
    pub succeeded: usize,
    /// Number of failed sends
    pub failed: usize,
    /// Errors that occurred (ConnectionId and error message)
    pub errors: Vec<(ConnectionId, String)>,
}

impl BroadcastResult {
    /// Create a new empty result
    fn new() -> Self {
        Self {
            total: 0,
            succeeded: 0,
            failed: 0,
            errors: Vec::new(),
        }
    }

    /// Check if all broadcasts succeeded
    pub fn all_succeeded(&self) -> bool {
        self.failed == 0
    }

    /// Get the success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            (self.succeeded as f64 / self.total as f64) * 100.0
        }
    }
}

/// Managed connection entry
struct ManagedConnection {
    /// Connection ID
    id: ConnectionId,
    /// The connection itself
    connection: TelnetConnection,
    /// Control channel sender
    control_tx: mpsc::Sender<ControlMessage>,
    /// Worker task handle
    worker_handle: JoinHandle<()>,
    /// Current state (atomic for lock-free access)
    state: Arc<std::sync::atomic::AtomicU8>,
    /// When the connection was created
    created_at: Instant,
}

impl ManagedConnection {
    /// Get the current state
    fn state(&self) -> ConnectionState {
        ConnectionState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Get connection info snapshot
    fn info(&self) -> ConnectionInfo {
        ConnectionInfo {
            id: self.id,
            state: self.state(),
            peer_addr: self.connection.peer_addr(),
            created_at: self.created_at,
            last_activity: self.created_at, // Worker tracks this internally
            bytes_sent: self.connection.bytes_sent(),
            bytes_received: self.connection.bytes_received(),
            messages_sent: self.connection.messages_sent(),
            messages_received: self.connection.messages_received(),
        }
    }
}

/// Connection manager
pub struct ConnectionManager {
    /// Active connections (lock-free concurrent map)
    connections: Arc<DashMap<ConnectionId, ManagedConnection>>,
    /// Next connection ID (monotonically increasing)
    next_id: Arc<AtomicU64>,
    /// Server metrics
    metrics: Arc<ServerMetrics>,
    /// Worker configuration
    worker_config: WorkerConfig,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new(metrics: Arc<ServerMetrics>, worker_config: WorkerConfig) -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            next_id: Arc::new(AtomicU64::new(1)),
            metrics,
            worker_config,
        }
    }

    /// Get the next connection ID
    fn next_connection_id(&self) -> ConnectionId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        ConnectionId::new(id)
    }

    /// Add a new connection
    ///
    /// This spawns a worker task for the connection and tracks it.
    pub fn add_connection(
        &self,
        connection: TelnetConnection,
        handler: Arc<dyn ServerHandler>,
    ) -> Result<ConnectionId> {
        let id = self.next_connection_id();

        // Create worker
        let worker_connection = connection.clone();
        let (worker, control_tx) =
            crate::ConnectionWorker::new(id, worker_connection, handler, self.worker_config.clone());

        // Get state reference before moving worker
        let state = Arc::new(std::sync::atomic::AtomicU8::new(
            ConnectionState::Connecting.as_u8(),
        ));
        let worker_state = state.clone();

        // Spawn worker task
        let connections = self.connections.clone();
        let metrics = self.metrics.clone();
        let worker_handle = tokio::spawn(async move {
            let start = Instant::now();
            worker.run().await;

            // Cleanup after worker finishes
            connections.remove(&id);
            metrics.connection_closed(start.elapsed());
        });

        // Store managed connection
        let managed = ManagedConnection {
            id,
            connection,
            control_tx,
            worker_handle,
            state: worker_state,
            created_at: Instant::now(),
        };

        self.connections.insert(id, managed);
        self.metrics.connection_opened();

        Ok(id)
    }

    /// Remove a connection
    ///
    /// This sends a close message to the worker and removes it from tracking.
    pub async fn remove_connection(&self, id: ConnectionId) -> Result<()> {
        if let Some((_, managed)) = self.connections.remove(&id) {
            // Send close message (best effort)
            let _ = managed.control_tx.send(ControlMessage::Close).await;

            // Wait for worker to finish (with timeout)
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                managed.worker_handle,
            )
            .await;

            Ok(())
        } else {
            Err(TelnetError::ConnectionNotFound(id))
        }
    }

    /// Get a connection by ID
    pub fn get_connection(&self, id: ConnectionId) -> Option<TelnetConnection> {
        self.connections
            .get(&id)
            .map(|entry| entry.connection.clone())
    }

    /// Get connection info
    pub fn get_connection_info(&self, id: ConnectionId) -> Option<ConnectionInfo> {
        self.connections.get(&id).map(|entry| entry.info())
    }

    /// Get all connection IDs
    pub fn get_connection_ids(&self) -> Vec<ConnectionId> {
        self.connections.iter().map(|entry| *entry.key()).collect()
    }

    /// Get all connection infos
    pub fn get_all_connection_infos(&self) -> Vec<ConnectionInfo> {
        self.connections
            .iter()
            .map(|entry| entry.value().info())
            .collect()
    }

    /// Get the number of active connections
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Send a command to a specific connection
    pub async fn send_to_connection(
        &self,
        id: ConnectionId,
        command: TerminalCommand,
    ) -> Result<()> {
        if let Some(managed) = self.connections.get(&id) {
            managed
                .control_tx
                .send(ControlMessage::SendCommand(command))
                .await
                .map_err(|_| TelnetError::ConnectionClosed)?;
            Ok(())
        } else {
            Err(TelnetError::ConnectionNotFound(id))
        }
    }

    /// Broadcast a command to all connections
    ///
    /// This sends the command to all active connections concurrently.
    /// Returns a result with statistics about the broadcast.
    pub async fn broadcast(&self, command: TerminalCommand) -> BroadcastResult {
        let mut result = BroadcastResult::new();
        result.total = self.connections.len();

        // Collect all send futures
        let mut sends = Vec::new();
        for entry in self.connections.iter() {
            let id = *entry.key();
            let tx = entry.control_tx.clone();
            let cmd = command;

            sends.push(async move {
                match tx.send(ControlMessage::Broadcast(cmd)).await {
                    Ok(_) => (id, Ok(())),
                    Err(e) => (id, Err(e.to_string())),
                }
            });
        }

        // Execute all sends concurrently
        let results = futures_util::future::join_all(sends).await;

        // Collect results
        for (id, res) in results {
            match res {
                Ok(_) => result.succeeded += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push((id, e));
                }
            }
        }

        result
    }

    /// Broadcast to connections matching a filter
    pub async fn broadcast_filtered<F>(
        &self,
        command: TerminalCommand,
        filter: F,
    ) -> BroadcastResult
    where
        F: Fn(&ConnectionInfo) -> bool,
    {
        let mut result = BroadcastResult::new();

        // Collect matching connections
        let mut sends = Vec::new();
        for entry in self.connections.iter() {
            let info = entry.value().info();
            if filter(&info) {
                result.total += 1;
                let id = *entry.key();
                let tx = entry.control_tx.clone();
                let cmd = command;

                sends.push(async move {
                    match tx.send(ControlMessage::Broadcast(cmd)).await {
                        Ok(_) => (id, Ok(())),
                        Err(e) => (id, Err(e.to_string())),
                    }
                });
            }
        }

        // Execute all sends concurrently
        let results = futures_util::future::join_all(sends).await;

        // Collect results
        for (id, res) in results {
            match res {
                Ok(_) => result.succeeded += 1,
                Err(e) => {
                    result.failed += 1;
                    result.errors.push((id, e));
                }
            }
        }

        result
    }

    /// Shutdown all connections gracefully
    pub async fn shutdown(&self) {
        // Send close to all connections
        for entry in self.connections.iter() {
            let _ = entry.control_tx.send(ControlMessage::Close).await;
        }

        // Wait for all workers to finish (with timeout)
        let _handles: Vec<_> = self
            .connections
            .iter()
            .map(|entry| entry.worker_handle.abort())
            .collect();

        // Give workers time to cleanup
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Clear all connections
        self.connections.clear();
    }
}

impl std::fmt::Debug for ConnectionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionManager")
            .field("connection_count", &self.connection_count())
            .field("next_id", &self.next_id.load(Ordering::Relaxed))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ServerConfig, ServerHandler};
    use async_trait::async_trait;
    use tokio::net::{TcpListener, TcpStream};

    struct TestHandler;

    #[async_trait]
    impl ServerHandler for TestHandler {}

    async fn create_test_connection() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let client_task = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });

        let (server, _) = listener.accept().await.unwrap();
        let client = client_task.await.unwrap();

        (server, client)
    }

    #[tokio::test]
    async fn test_manager_add_remove() {
        let config = ServerConfig::default();
        let metrics = Arc::new(ServerMetrics::new());
        let worker_config = WorkerConfig {
            read_timeout: config.read_timeout,
            idle_timeout: config.idle_timeout,
            write_timeout: config.write_timeout,
            control_buffer_size: 100,
        };
        let manager = ConnectionManager::new(metrics.clone(), worker_config);

        let (server, _client) = create_test_connection().await;
        let id = ConnectionId::new(1);
        let connection = TelnetConnection::wrap(server, id).unwrap();

        let conn_id = manager
            .add_connection(connection, Arc::new(TestHandler))
            .unwrap();

        assert_eq!(manager.connection_count(), 1);
        assert!(manager.get_connection(conn_id).is_some());

        manager.remove_connection(conn_id).await.unwrap();

        // Give it time to cleanup
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_manager_broadcast() {
        let config = ServerConfig::default();
        let metrics = Arc::new(ServerMetrics::new());
        let worker_config = WorkerConfig {
            read_timeout: config.read_timeout,
            idle_timeout: config.idle_timeout,
            write_timeout: config.write_timeout,
            control_buffer_size: 100,
        };
        let manager = ConnectionManager::new(metrics.clone(), worker_config);

        // Add multiple connections
        let mut clients = Vec::new();
        for i in 0..3 {
            let (server, client) = create_test_connection().await;
            let id = ConnectionId::new(i);
            let connection = TelnetConnection::wrap(server, id).unwrap();
            manager
                .add_connection(connection, Arc::new(TestHandler))
                .unwrap();
            clients.push(client);
        }

        assert_eq!(manager.connection_count(), 3);

        // Broadcast a command
        let result = manager.broadcast(TerminalCommand::SendEraseLine).await;
        assert_eq!(result.total, 3);

        // Cleanup
        manager.shutdown().await;
        drop(clients);
    }
}


