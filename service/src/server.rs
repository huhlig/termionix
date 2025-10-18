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

//! Telnet server implementation
//!
//! The TelnetServer is the main entry point for the  implementation.
//! It manages the TCP listener, accepts connections, and coordinates
//! with the ConnectionManager.

use crate::{
    ConnectionId, ConnectionManager, Result, ServerConfig, ServerHandler, ServerMetrics,
    ServerSnapshot, TelnetConnection, TelnetError, WorkerConfig,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// Telnet server ( implementation)
///
/// This is the main server that accepts connections and manages their lifecycle.
///
/// # Example
///
/// ```no_run
/// use termionix_service::{TelnetServer, ServerConfig, ServerHandler};
/// use async_trait::async_trait;
///
/// struct MyHandler;
///
/// #[async_trait]
/// impl ServerHandler for MyHandler {
///     // Implement handler methods
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let config = ServerConfig::default();
///     let server = TelnetServer::new(config).await?;
///     
///     server.start(std::sync::Arc::new(MyHandler)).await?;
///     
///     // Server is now running, wait for shutdown signal
///     // tokio::signal::ctrl_c().await?;
///     server.shutdown().await?;
///     
///     Ok(())
/// }
/// ```
pub struct TelnetServer {
    /// Server configuration
    config: ServerConfig,
    /// Connection manager
    manager: Arc<ConnectionManager>,
    /// Server metrics
    metrics: Arc<ServerMetrics>,
    /// TCP listener (wrapped in Arc<Mutex> for sharing with accept loop)
    listener: Arc<tokio::sync::Mutex<TcpListener>>,
    /// Actual bind address
    bind_address: SocketAddr,
    /// Server start time
    started_at: Instant,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Shutdown notification
    shutdown_notify: Arc<Notify>,
    /// Accept loop task handle
    accept_handle: Arc<tokio::sync::Mutex<Option<JoinHandle<()>>>>,
}

impl TelnetServer {
    /// Create a new server with the given configuration
    ///
    /// This binds to the configured address but does not start accepting connections.
    /// Call `start()` to begin accepting connections.
    pub async fn new(config: ServerConfig) -> Result<Self> {
        // Bind to the configured address
        let listener = TcpListener::bind(config.bind_address).await?;
        let actual_addr = listener.local_addr()?;

        // Create metrics
        let metrics = Arc::new(ServerMetrics::new());

        // Create worker config from server config
        let worker_config = WorkerConfig {
            read_timeout: config.read_timeout,
            idle_timeout: config.idle_timeout,
            write_timeout: config.write_timeout,
            control_buffer_size: 100,
        };

        // Create connection manager
        let manager = Arc::new(ConnectionManager::new(metrics.clone(), worker_config));

        tracing::info!("Telnet server bound to {}", actual_addr);

        Ok(Self {
            config,
            manager,
            metrics,
            listener: Arc::new(tokio::sync::Mutex::new(listener)),
            bind_address: actual_addr,
            started_at: Instant::now(),
            running: Arc::new(AtomicBool::new(false)),
            shutdown_notify: Arc::new(Notify::new()),
            accept_handle: Arc::new(tokio::sync::Mutex::new(None)),
        })
    }

    /// Start the server with the given handler
    ///
    /// This begins accepting connections and spawns a task to handle the accept loop.
    /// The server will continue running until `shutdown()` is called.
    pub async fn start(&self, handler: Arc<dyn ServerHandler>) -> Result<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(TelnetError::Other("Server already running".to_string()));
        }

        tracing::info!("Starting Telnet server on {}", self.config.bind_address);

        // Spawn accept loop
        let handle = self.spawn_accept_loop(handler).await;
        *self.accept_handle.lock().await = Some(handle);

        Ok(())
    }

    /// Spawn the accept loop task
    async fn spawn_accept_loop(&self, handler: Arc<dyn ServerHandler>) -> JoinHandle<()> {
        let listener = self.listener.clone();
        let manager = self.manager.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();
        let running = self.running.clone();
        let shutdown_notify = self.shutdown_notify.clone();

        tokio::spawn(async move {
            loop {
                // Check if we should shutdown
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                // Accept connection with timeout
                let accept_result = tokio::select! {
                    result = async {
                        listener.lock().await.accept().await
                    } => result,
                    _ = shutdown_notify.notified() => break,
                };

                match accept_result {
                    Ok((socket, peer_addr)) => {
                        tracing::debug!("Accepted connection from {}", peer_addr);

                        // Check connection limit
                        if manager.connection_count() >= config.max_connections {
                            tracing::warn!(
                                "Connection limit reached ({}), rejecting connection from {}",
                                config.max_connections,
                                peer_addr
                            );
                            metrics.connection_error();
                            drop(socket);
                            continue;
                        }

                        // Create connection ID (will be assigned by manager)
                        let temp_id = ConnectionId::new(0);

                        // Wrap socket in TelnetConnection
                        match TelnetConnection::wrap(socket, temp_id) {
                            Ok(connection) => {
                                // Add to manager
                                match manager.add_connection(connection, handler.clone()) {
                                    Ok(id) => {
                                        tracing::info!(
                                            "Connection {} established from {}",
                                            id,
                                            peer_addr
                                        );
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to add connection: {}", e);
                                        metrics.connection_error();
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to wrap connection: {}", e);
                                metrics.connection_error();
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to accept connection: {}", e);
                        metrics.connection_error();

                        // Back off on errors to avoid tight loop
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }

            tracing::info!("Accept loop terminated");
        })
    }

    /// Shutdown the server gracefully
    ///
    /// This stops accepting new connections and waits for existing connections
    /// to close gracefully (up to the configured shutdown timeout).
    pub async fn shutdown(&self) -> Result<()> {
        if !self.running.swap(false, Ordering::SeqCst) {
            return Err(TelnetError::ServerNotRunning);
        }

        tracing::info!("Shutting down Telnet server");

        // Notify accept loop to stop
        self.shutdown_notify.notify_waiters();

        // Wait for accept loop to finish
        if let Some(handle) = self.accept_handle.lock().await.take() {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;
        }

        // Shutdown all connections
        self.manager.shutdown().await;

        tracing::info!("Telnet server shutdown complete");

        Ok(())
    }

    /// Check if the server is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get the server's bind address
    pub fn bind_address(&self) -> SocketAddr {
        self.bind_address
    }

    /// Get the number of active connections
    pub fn connection_count(&self) -> usize {
        self.manager.connection_count()
    }

    /// Get a snapshot of the server state
    pub fn snapshot(&self) -> ServerSnapshot {
        ServerSnapshot {
            active_connections: self.manager.connection_count(),
            total_connections: self.metrics.total_connections(),
            bind_address: self.bind_address(),
            uptime: self.started_at.elapsed(),
            started_at: self.started_at,
        }
    }

    /// Get the server metrics
    pub fn metrics(&self) -> Arc<ServerMetrics> {
        self.metrics.clone()
    }

    /// Get the connection manager
    pub fn manager(&self) -> Arc<ConnectionManager> {
        self.manager.clone()
    }

    /// Get the server configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

impl std::fmt::Debug for TelnetServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelnetServer")
            .field("bind_address", &self.bind_address())
            .field("running", &self.is_running())
            .field("connection_count", &self.connection_count())
            .field("uptime", &self.started_at.elapsed())
            .finish()
    }
}

// Implement Drop to ensure cleanup
impl Drop for TelnetServer {
    fn drop(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            tracing::warn!("TelnetServer dropped while still running");
            self.running.store(false, Ordering::SeqCst);
            self.shutdown_notify.notify_waiters();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServerHandler;
    use async_trait::async_trait;

    struct TestHandler;

    #[async_trait]
    impl ServerHandler for TestHandler {}

    #[tokio::test]
    async fn test_server_lifecycle() {
        let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());

        let server = TelnetServer::new(config).await.unwrap();
        assert!(!server.is_running());

        server.start(Arc::new(TestHandler)).await.unwrap();
        assert!(server.is_running());

        // Give it time to start
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        server.shutdown().await.unwrap();
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn test_server_snapshot() {
        let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());

        let server = TelnetServer::new(config).await.unwrap();
        let snapshot = server.snapshot();

        assert_eq!(snapshot.active_connections, 0);
        assert_eq!(snapshot.total_connections, 0);
    }

    #[tokio::test]
    async fn test_server_double_start() {
        let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());

        let server = TelnetServer::new(config).await.unwrap();
        server.start(Arc::new(TestHandler)).await.unwrap();

        // Second start should fail
        let result = server.start(Arc::new(TestHandler)).await;
        assert!(result.is_err());

        server.shutdown().await.unwrap();
    }
}


