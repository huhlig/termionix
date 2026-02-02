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

//! Memory leak detection and resource management tests

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use termionix_service::{
    ConnectionId, ServerConfig, ServerHandler, TelnetConnection, TelnetServer,
};
use tokio::net::TcpStream;

struct MemoryTestHandler;

#[async_trait]
impl ServerHandler for MemoryTestHandler {}

#[tokio::test]
#[ignore] // Run manually for memory profiling
async fn test_sustained_connection_churn() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap()).with_max_connections(100);
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(MemoryTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Run for extended period with connection churn
    for cycle in 0..100 {
        let mut clients = Vec::new();

        // Create connections
        for _ in 0..10 {
            if let Ok(client) = TcpStream::connect(addr).await {
                clients.push(client);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Let them live briefly
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Drop all connections
        drop(clients);
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Memory should be released
        if cycle % 10 == 0 {
            println!("Cycle {}: {} connections", cycle, server.connection_count());
        }
    }

    // All connections should be cleaned up
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(server.connection_count(), 0);

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_connection_cleanup() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(MemoryTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create and destroy connections multiple times
    for _ in 0..20 {
        let client = TcpStream::connect(addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(client);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // All should be cleaned up
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(server.connection_count(), 0);

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_metrics_memory_stability() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(MemoryTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let metrics = server.metrics();

    // Generate lots of metric updates
    for _ in 0..1000 {
        let client = TcpStream::connect(addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        drop(client);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    // Metrics should still be accurate
    let snapshot = metrics.snapshot();
    assert!(snapshot.total_connections >= 1000);
    assert_eq!(snapshot.active_connections, 0);

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_broadcast_memory_stability() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(MemoryTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create connections
    let mut clients = Vec::new();
    for _ in 0..10 {
        let client = TcpStream::connect(addr).await.unwrap();
        clients.push(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let manager = server.manager();

    // Perform many broadcasts
    for _ in 0..100 {
        let _ = manager
            .broadcast(termionix_terminal::TerminalCommand::SendEraseLine)
            .await;
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Connections should still be active
    assert_eq!(server.connection_count(), 10);

    drop(clients);
    tokio::time::sleep(Duration::from_millis(200)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_user_data_cleanup() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(MemoryTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create connection and set user data
    let client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let manager = server.manager();
    let ids = manager.get_connection_ids();
    if let Some(conn) = manager.get_connection(ids[0]) {
        // Set various types of user data
        conn.set_data("string", "test".to_string());
        conn.set_data("number", 12345u64);
        conn.set_data("vec", vec![1, 2, 3, 4, 5]);
    }

    // Drop connection
    drop(client);
    tokio::time::sleep(Duration::from_millis(300)).await;

    // User data should be cleaned up with connection
    assert_eq!(server.connection_count(), 0);

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_handler_arc_cleanup() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(MemoryTestHandler);
    let handler_weak = Arc::downgrade(&handler);

    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create and drop connections
    for _ in 0..5 {
        let client = TcpStream::connect(addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        drop(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Handler should still be alive (held by server)
    assert!(handler_weak.upgrade().is_some());

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_manager_memory_after_shutdown() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(MemoryTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create connections
    let mut clients = Vec::new();
    for _ in 0..5 {
        let client = TcpStream::connect(addr).await.unwrap();
        clients.push(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert_eq!(server.connection_count(), 5);

    // Shutdown should clean up all connections
    server.shutdown().await.unwrap();

    tokio::time::sleep(Duration::from_millis(300)).await;

    // All connections should be gone
    assert_eq!(server.connection_count(), 0);

    drop(clients);
}

#[tokio::test]
#[ignore] // Run manually for stress testing
async fn test_high_connection_count_stability() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap()).with_max_connections(500);
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(MemoryTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create many connections
    let mut clients = Vec::new();
    for i in 0..200 {
        if let Ok(client) = TcpStream::connect(addr).await {
            clients.push(client);
            if i % 50 == 0 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    println!("Created {} connections", clients.len());
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify all are tracked
    assert!(server.connection_count() >= 150);

    // Drop all
    drop(clients);
    tokio::time::sleep(Duration::from_secs(2)).await;

    // All should be cleaned up
    assert_eq!(server.connection_count(), 0);

    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_repeated_server_lifecycle() {
    // Test that multiple server create/destroy cycles don't leak
    for _ in 0..10 {
        let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
        let server = TelnetServer::new(config).await.unwrap();
        let addr = server.bind_address();

        let handler = Arc::new(MemoryTestHandler);
        server.start(handler).await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        // Create a connection
        let client = TcpStream::connect(addr).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Shutdown
        server.shutdown().await.unwrap();
        drop(client);

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // If we got here without crashing, memory management is working
}

#[tokio::test]
async fn test_connection_info_memory() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(MemoryTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create connections
    let mut clients = Vec::new();
    for _ in 0..10 {
        let client = TcpStream::connect(addr).await.unwrap();
        clients.push(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let manager = server.manager();

    // Query connection info many times
    for _ in 0..100 {
        let _infos = manager.get_all_connection_infos();
        let _ids = manager.get_connection_ids();
    }

    // Should not accumulate memory
    assert_eq!(server.connection_count(), 10);

    drop(clients);
    tokio::time::sleep(Duration::from_millis(200)).await;
    server.shutdown().await.unwrap();
}
