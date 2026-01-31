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

//! Connection worker implementation
//!
//! The ConnectionWorker is responsible for managing the lifecycle of a single
//! connection, including:
//! - Event processing loop
//! - Timeout management (read, idle, write)
//! - Control message handling
//! - Broadcast message handling
//! - Resource cleanup

use crate::{ConnectionId, ConnectionState, Result, ServerHandler, TelnetConnection, TelnetError};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};
use termionix_terminal::TerminalCommand;
use tokio::select;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout};

/// Control messages for the worker
#[derive(Debug)]
pub enum ControlMessage {
    /// Gracefully close the connection
    Close,
    /// Send a command to the connection
    SendCommand(TerminalCommand),
    /// Broadcast message (sent to all connections)
    Broadcast(TerminalCommand),
}

/// Worker configuration
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Read timeout (max time to wait for data)
    pub read_timeout: Duration,
    /// Idle timeout (max time without activity)
    pub idle_timeout: Duration,
    /// Write timeout (max time for send operations)
    pub write_timeout: Duration,
    /// Control channel buffer size
    pub control_buffer_size: usize,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            read_timeout: Duration::from_secs(300), // 5 minutes
            idle_timeout: Duration::from_secs(600), // 10 minutes
            write_timeout: Duration::from_secs(30), // 30 seconds
            control_buffer_size: 100,
        }
    }
}

/// Connection worker that manages a single connection's lifecycle
pub struct ConnectionWorker {
    /// Connection ID
    id: ConnectionId,
    /// The connection being managed
    connection: TelnetConnection,
    /// Event handler
    handler: Arc<dyn ServerHandler>,
    /// Configuration
    config: WorkerConfig,
    /// Current state (atomic for lock-free access)
    state: Arc<AtomicU8>,
    /// Control message receiver
    control_rx: mpsc::Receiver<ControlMessage>,
    /// Last activity timestamp
    last_activity: Instant,
}

impl ConnectionWorker {
    /// Create a new connection worker
    pub fn new(
        id: ConnectionId,
        connection: TelnetConnection,
        handler: Arc<dyn ServerHandler>,
        config: WorkerConfig,
        state: Arc<AtomicU8>,
    ) -> (Self, mpsc::Sender<ControlMessage>) {
        let (control_tx, control_rx) = mpsc::channel(config.control_buffer_size);

        let worker = Self {
            id,
            connection,
            handler,
            config,
            state,
            control_rx,
            last_activity: Instant::now(),
        };

        (worker, control_tx)
    }

    /// Get the current state
    pub fn state(&self) -> ConnectionState {
        ConnectionState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Set the state
    fn set_state(&self, new_state: ConnectionState) {
        self.state.store(new_state.as_u8(), Ordering::Release);
    }

    /// Update last activity timestamp
    fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if connection is idle
    fn is_idle(&self) -> bool {
        self.last_activity.elapsed() > self.config.idle_timeout
    }

    /// Run the worker event loop
    ///
    /// This is the main entry point for the worker. It will run until the
    /// connection is closed or an error occurs.
    pub async fn run(mut self) {
        // Transition to Active state
        self.set_state(ConnectionState::Active);

        // Notify handler of connection
        self.handler.on_connect(self.id, &self.connection).await;

        // Main event loop
        let result = self.event_loop().await;

        // Handle any errors
        if let Err(e) = result {
            self.handler.on_error(self.id, &self.connection, e).await;
        }

        // Cleanup
        self.cleanup().await;
    }

    /// Main event processing loop
    async fn event_loop(&mut self) -> Result<()> {
        loop {
            // Check for idle timeout
            if self.is_idle() {
                self.handler
                    .on_idle_timeout(self.id, &self.connection)
                    .await;
                return Err(TelnetError::Timeout);
            }

            // Wait for next event with timeout
            select! {
                // Handle incoming events from the connection
                result = timeout(self.config.read_timeout, self.connection.next()) => {
                    match result {
                        Ok(Ok(Some(event))) => {
                            self.update_activity();
                            self.set_state(ConnectionState::Active);
                            self.handler.on_event(self.id, &self.connection, event).await;
                            
                            // Flush any protocol responses generated during decode
                            if self.connection.has_pending_responses().await {
                                if let Err(e) = self.connection.flush_responses().await {
                                    tracing::warn!(
                                        connection_id = %self.id,
                                        error = ?e,
                                        "Failed to flush protocol responses"
                                    );
                                }
                            }
                        }
                        Ok(Ok(None)) => {
                            // Connection closed by peer
                            return Ok(());
                        }
                        Ok(Err(e)) => {
                            // Error reading from connection
                            return Err(e);
                        }
                        Err(_) => {
                            // Read timeout
                            self.handler.on_timeout(self.id, &self.connection).await;
                            return Err(TelnetError::Timeout);
                        }
                    }
                }

                // Handle control messages
                msg = self.control_rx.recv() => {
                    match msg {
                        Some(ControlMessage::Close) => {
                            // Graceful close requested
                            return Ok(());
                        }
                        Some(ControlMessage::SendCommand(cmd)) => {
                            // Send command with timeout
                            if let Err(e) = timeout(
                                self.config.write_timeout,
                                self.connection.send_command(&cmd)
                            ).await {
                                return Err(TelnetError::Other(format!("Write timeout: {}", e)));
                            }
                            self.update_activity();
                        }
                        Some(ControlMessage::Broadcast(cmd)) => {
                            // Handle broadcast (best effort, don't fail on error)
                            let _ = timeout(
                                self.config.write_timeout,
                                self.connection.send_command(&cmd)
                            ).await;
                            self.update_activity();
                        }
                        None => {
                            // Control channel closed, shutdown
                            return Ok(());
                        }
                    }
                }

                // Check for idle state transition
                _ = sleep(Duration::from_secs(10)) => {
                    if self.last_activity.elapsed() > Duration::from_secs(60) {
                        self.set_state(ConnectionState::Idle);
                    }
                }
            }
        }
    }

    /// Cleanup resources
    async fn cleanup(&mut self) {
        // Transition to Closing state
        self.set_state(ConnectionState::Closing);

        // Notify handler of disconnection
        self.handler.on_disconnect(self.id, &self.connection).await;

        // Drain any remaining control messages
        while self.control_rx.try_recv().is_ok() {}

        // Transition to Closed state
        self.set_state(ConnectionState::Closed);
    }
}

impl std::fmt::Debug for ConnectionWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionWorker")
            .field("id", &self.id)
            .field("state", &self.state())
            .field("last_activity", &self.last_activity)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServerHandler;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use termionix_terminal::TerminalEvent;
    use tokio::net::{TcpListener, TcpStream};

    struct TestHandler {
        connected: AtomicBool,
        disconnected: AtomicBool,
        event_count: AtomicUsize,
    }

    impl TestHandler {
        fn new() -> Self {
            Self {
                connected: AtomicBool::new(false),
                disconnected: AtomicBool::new(false),
                event_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl ServerHandler for TestHandler {
        async fn on_connect(&self, _id: ConnectionId, _conn: &TelnetConnection) {
            self.connected.store(true, Ordering::SeqCst);
        }

        async fn on_event(
            &self,
            _id: ConnectionId,
            _conn: &TelnetConnection,
            _event: TerminalEvent,
        ) {
            self.event_count.fetch_add(1, Ordering::SeqCst);
        }

        async fn on_disconnect(&self, _id: ConnectionId, _conn: &TelnetConnection) {
            self.disconnected.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_worker_lifecycle() {
        // Create a test connection
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            socket
        });

        let client = TcpStream::connect(addr).await.unwrap();
        let server = server_task.await.unwrap();

        let id = ConnectionId::new(1);
        let connection = TelnetConnection::wrap(server, id).unwrap();
        let handler = Arc::new(TestHandler::new());
        let config = WorkerConfig::default();
        let state = Arc::new(AtomicU8::new(ConnectionState::Connecting.as_u8()));

        let (worker, control_tx) = ConnectionWorker::new(id, connection, handler.clone(), config, state);

        // Start worker
        let worker_task = tokio::spawn(async move {
            worker.run().await;
        });

        // Give it time to connect
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(handler.connected.load(Ordering::SeqCst));

        // Close the connection
        control_tx.send(ControlMessage::Close).await.unwrap();
        drop(control_tx);

        // Wait for worker to finish
        worker_task.await.unwrap();

        // Verify disconnection was called
        assert!(handler.disconnected.load(Ordering::SeqCst));

        // Cleanup
        drop(client);
    }

    #[tokio::test]
    async fn test_worker_control_messages() {
        // Create a test connection
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            socket
        });

        let client = TcpStream::connect(addr).await.unwrap();
        let server = server_task.await.unwrap();

        let id = ConnectionId::new(1);
        let connection = TelnetConnection::wrap(server, id).unwrap();
        let handler = Arc::new(TestHandler::new());
        let config = WorkerConfig::default();
        let state = Arc::new(AtomicU8::new(ConnectionState::Connecting.as_u8()));

        let (worker, control_tx) = ConnectionWorker::new(id, connection, handler.clone(), config, state);

        // Start worker
        let worker_task = tokio::spawn(async move {
            worker.run().await;
        });

        // Send a command
        control_tx
            .send(ControlMessage::SendCommand(TerminalCommand::SendEraseLine))
            .await
            .unwrap();

        // Give it time to process
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Close
        control_tx.send(ControlMessage::Close).await.unwrap();
        drop(control_tx);

        worker_task.await.unwrap();

        // Cleanup
        drop(client);
    }
}
