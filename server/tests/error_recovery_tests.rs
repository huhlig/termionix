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

//! Error recovery and network failure simulation tests

use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use termionix_server::{
    ConnectionId, ServerConfig, ServerHandler, TelnetConnection, TelnetError, TelnetServer,
    TerminalCommand,
};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// Handler that tracks errors
struct ErrorTrackingHandler {
    error_count: Arc<AtomicUsize>,
    timeout_count: Arc<AtomicUsize>,
    disconnect_count: Arc<AtomicUsize>,
}

impl ErrorTrackingHandler {
    fn new() -> Self {
        Self {
            error_count: Arc::new(AtomicUsize::new(0)),
            timeout_count: Arc::new(AtomicUsize::new(0)),
            disconnect_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn error_count(&self) -> usize {
        self.error_count.load(Ordering::SeqCst)
    }

    fn timeout_count(&self) -> usize {
        self.timeout_count.load(Ordering::SeqCst)
    }

    fn disconnect_count(&self) -> usize {
        self.disconnect_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ServerHandler for ErrorTrackingHandler {
    async fn on_error(&self, _id: ConnectionId, _conn: &TelnetConnection, _error: TelnetError) {
        self.error_count.fetch_add(1, Ordering::SeqCst);
    }

    async fn on_timeout(&self, _id: ConnectionId, _conn: &TelnetConnection) {
        self.timeout_count.fetch_add(1, Ordering::SeqCst);
    }

    async fn on_disconnect(&self, _id: ConnectionId, _conn: &TelnetConnection) {
        self.disconnect_count.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn test_abrupt_client_disconnect() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ErrorTrackingHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect and immediately drop without proper close
    {
        let _client = TcpStream::connect(addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        // Client drops here without graceful shutdown
    }

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Should detect logout.txt
    assert!(handler.disconnect_count() >= 1);

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_multiple_rapid_disconnects() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ErrorTrackingHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create and drop multiple connections rapidly
    for _ in 0..10 {
        let _client = TcpStream::connect(addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        // Immediate drop
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Should handle all disconnects
    assert!(handler.disconnect_count() >= 8); // Allow some timing variance

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_partial_write_disconnect() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ErrorTrackingHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Write partial data and logout.txt
    client.write_all(b"Partial").await.unwrap();
    // Don't flush, just drop
    drop(client);

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Should handle partial write
    assert!(handler.disconnect_count() >= 1);

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_connection_during_shutdown() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ErrorTrackingHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Start shutdown
    let shutdown_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        server.shutdown().await.unwrap();
    });

    // Try to connect during shutdown
    tokio::time::sleep(Duration::from_millis(25)).await;
    let result = TcpStream::connect(addr).await;

    // Connection might succeed or fail depending on timing
    if let Ok(client) = result {
        drop(client);
    }

    shutdown_task.await.unwrap();
}

#[tokio::test]
async fn test_server_restart_after_error() {
    let bind_addr = "127.0.0.1:0".parse().unwrap();

    // First server instance
    let config1 = ServerConfig::new(bind_addr);
    let server1 = TelnetServer::new(config1).await.unwrap();
    let addr1 = server1.bind_address();

    let handler1 = Arc::new(ErrorTrackingHandler::new());
    server1.start(handler1.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect a client
    let client1 = TcpStream::connect(addr1).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Shutdown first server
    server1.shutdown().await.unwrap();
    drop(client1);

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Start new server (will get different port)
    let config2 = ServerConfig::new(bind_addr);
    let server2 = TelnetServer::new(config2).await.unwrap();
    let addr2 = server2.bind_address();

    let handler2 = Arc::new(ErrorTrackingHandler::new());
    server2.start(handler2.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect to new server
    let client2 = TcpStream::connect(addr2).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(server2.connection_count(), 1);

    drop(client2);
    server2.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_broadcast_with_failed_connections() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ErrorTrackingHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect multiple clients
    let mut clients = Vec::new();
    for _ in 0..5 {
        let client = TcpStream::connect(addr).await.unwrap();
        clients.push(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Drop some clients to simulate failures
    drop(clients.pop());
    drop(clients.pop());
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Broadcast should handle partial failures
    let manager = server.manager();
    let result = manager.broadcast(TerminalCommand::EraseLine).await;

    // Some should succeed, some might fail
    assert!(result.total > 0);

    drop(clients);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_connection_limit_recovery() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap()).with_max_connections(3);
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ErrorTrackingHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Fill to capacity
    let mut clients = Vec::new();
    for _ in 0..3 {
        let client = TcpStream::connect(addr).await.unwrap();
        clients.push(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert_eq!(server.connection_count(), 3);

    // Try to connect beyond limit (should be rejected)
    let extra_result = TcpStream::connect(addr).await;
    if let Ok(extra) = extra_result {
        tokio::time::sleep(Duration::from_millis(100)).await;
        // Should still be at limit
        assert!(server.connection_count() <= 3);
        drop(extra);
    }

    // Drop one connection
    drop(clients.pop());
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Should be able to connect again
    let new_client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(server.connection_count() <= 3);

    drop(new_client);
    drop(clients);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_error_handler_invocation() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ErrorTrackingHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect and abruptly logout.txt
    let client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    drop(client);

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Error or logout.txt handler should be called
    assert!(handler.error_count() + handler.disconnect_count() >= 1);

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_concurrent_errors() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ErrorTrackingHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create multiple connections and drop them concurrently
    let mut handles = Vec::new();
    for _ in 0..10 {
        let addr = addr;
        handles.push(tokio::spawn(async move {
            if let Ok(client) = TcpStream::connect(addr).await {
                tokio::time::sleep(Duration::from_millis(50)).await;
                drop(client);
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Should handle all concurrent errors/disconnects
    assert!(handler.disconnect_count() >= 8);

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_metrics_after_errors() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ErrorTrackingHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let metrics = server.metrics();
    let initial_total = metrics.total_connections();

    // Create and drop connections
    for _ in 0..5 {
        let client = TcpStream::connect(addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    tokio::time::sleep(Duration::from_millis(300)).await;

    // Metrics should still be accurate
    assert_eq!(metrics.total_connections(), initial_total + 5);
    assert_eq!(metrics.active_connections(), 0);

    server.shutdown().await.unwrap();
}
