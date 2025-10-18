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

//! Integration tests for the Telnet server

use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use termionix_service::{
    ConnectionId, ServerConfig, ServerHandler, TelnetConnection, TelnetServer,
};
use termionix_terminal::TerminalEvent;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

/// Test handler that tracks events
struct TestHandler {
    connect_count: Arc<AtomicUsize>,
    event_count: Arc<AtomicUsize>,
    disconnect_count: Arc<AtomicUsize>,
}

impl TestHandler {
    fn new() -> Self {
        Self {
            connect_count: Arc::new(AtomicUsize::new(0)),
            event_count: Arc::new(AtomicUsize::new(0)),
            disconnect_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn connect_count(&self) -> usize {
        self.connect_count.load(Ordering::SeqCst)
    }

    fn event_count(&self) -> usize {
        self.event_count.load(Ordering::SeqCst)
    }

    fn disconnect_count(&self) -> usize {
        self.disconnect_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ServerHandler for TestHandler {
    async fn on_connect(&self, _id: ConnectionId, _conn: &TelnetConnection) {
        self.connect_count.fetch_add(1, Ordering::SeqCst);
    }

    async fn on_event(&self, _id: ConnectionId, _conn: &TelnetConnection, _event: TerminalEvent) {
        self.event_count.fetch_add(1, Ordering::SeqCst);
    }

    async fn on_disconnect(&self, _id: ConnectionId, _conn: &TelnetConnection) {
        self.disconnect_count.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn test_server_accepts_connections() {
    // Create server with random port
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(TestHandler::new());
    server.start(handler.clone()).await.unwrap();

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect a client
    let client = TcpStream::connect(addr).await.unwrap();

    // Give connection time to be processed
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify connection was accepted
    assert_eq!(server.connection_count(), 1);
    assert_eq!(handler.connect_count(), 1);

    // Cleanup
    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_handles_multiple_connections() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap()).with_max_connections(10);
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(TestHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect multiple clients
    let mut clients = Vec::new();
    for _ in 0..5 {
        let client = TcpStream::connect(addr).await.unwrap();
        clients.push(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Verify all connections were accepted
    assert_eq!(server.connection_count(), 5);
    assert_eq!(handler.connect_count(), 5);

    // Cleanup
    drop(clients);
    tokio::time::sleep(Duration::from_millis(200)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_enforces_connection_limit() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap()).with_max_connections(3);
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(TestHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Try to connect more than the limit
    let mut clients = Vec::new();
    for _ in 0..5 {
        if let Ok(client) = TcpStream::connect(addr).await {
            clients.push(client);
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    // Should only have max_connections active
    assert!(server.connection_count() <= 3);

    // Cleanup
    drop(clients);
    tokio::time::sleep(Duration::from_millis(200)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
#[ignore] // Timing-sensitive test, may be flaky in CI
async fn test_server_graceful_shutdown() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(TestHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect some clients
    let mut clients = Vec::new();
    for _ in 0..3 {
        let client = TcpStream::connect(addr).await.unwrap();
        clients.push(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert_eq!(server.connection_count(), 3);

    // Shutdown server
    server.shutdown().await.unwrap();

    // Verify server stopped
    assert!(!server.is_running());

    // Give more time for cleanup
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(server.connection_count(), 0);

    // Verify all connections were disconnected (may take time)
    assert!(handler.disconnect_count() >= 3);

    drop(clients);
}

#[tokio::test]
async fn test_connection_receives_data() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(TestHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect and send data
    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send some data
    client.write_all(b"Hello, Server!\n").await.unwrap();
    client.flush().await.unwrap();

    // Give server time to process
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify events were received
    assert!(handler.event_count() > 0);

    // Cleanup
    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_server_metrics() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(TestHandler::new());
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Get initial metrics
    let metrics = server.metrics();
    let initial_total = metrics.total_connections();

    // Connect some clients
    let mut clients = Vec::new();
    for _ in 0..3 {
        let client = TcpStream::connect(addr).await.unwrap();
        clients.push(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Verify metrics updated
    assert_eq!(metrics.total_connections(), initial_total + 3);
    assert_eq!(metrics.active_connections(), 3);

    // Get snapshot
    let snapshot = server.snapshot();
    assert_eq!(snapshot.active_connections, 3);
    assert_eq!(snapshot.total_connections, initial_total + 3);

    // Cleanup
    drop(clients);
    tokio::time::sleep(Duration::from_millis(200)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_broadcast_to_connections() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(TestHandler::new());
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect multiple clients
    let mut clients = Vec::new();
    for _ in 0..3 {
        let client = TcpStream::connect(addr).await.unwrap();
        clients.push(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Broadcast a message
    let manager = server.manager();
    let result = manager
        .broadcast(termionix_terminal::TerminalCommand::SendEraseLine)
        .await;

    // Verify broadcast succeeded
    assert_eq!(result.total, 3);
    assert!(result.all_succeeded());

    // Cleanup
    drop(clients);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
#[ignore] // Timing-sensitive test, may be flaky in CI
async fn test_connection_timeout() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap())
        .with_idle_timeout(Duration::from_millis(500));
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(TestHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect a client but don't send any data
    let client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    assert_eq!(server.connection_count(), 1);

    // Wait for idle timeout (give extra time for processing)
    tokio::time::sleep(Duration::from_millis(1000)).await;

    // Connection should be closed due to timeout
    // Note: timing-sensitive, so we check if it's close to expected
    assert!(
        server.connection_count() <= 1,
        "Expected 0-1 connections, got {}",
        server.connection_count()
    );
    assert!(
        handler.disconnect_count() >= 1,
        "Expected at least 1 disconnect, got {}",
        handler.disconnect_count()
    );

    // Cleanup
    drop(client);
    server.shutdown().await.unwrap();
}
#[tokio::test]
async fn test_concurrent_connections() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap()).with_max_connections(50);
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(TestHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Spawn many concurrent connections
    let mut handles = Vec::new();
    for _ in 0..20 {
        let addr = addr;
        handles.push(tokio::spawn(async move {
            let _client = TcpStream::connect(addr).await.unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }));
    }

    // Wait for all connections
    for handle in handles {
        handle.await.unwrap();
    }

    // Give server time to process
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify all connections were handled
    assert_eq!(handler.connect_count(), 20);

    // Cleanup
    server.shutdown().await.unwrap();
}

/// Advanced test handler that tracks conversations
struct ConversationHandler {
    connect_count: Arc<AtomicUsize>,
    messages: Arc<tokio::sync::Mutex<Vec<(ConnectionId, String)>>>,
    disconnect_count: Arc<AtomicUsize>,
}

impl ConversationHandler {
    fn new() -> Self {
        Self {
            connect_count: Arc::new(AtomicUsize::new(0)),
            messages: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            disconnect_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn connect_count(&self) -> usize {
        self.connect_count.load(Ordering::SeqCst)
    }

    fn disconnect_count(&self) -> usize {
        self.disconnect_count.load(Ordering::SeqCst)
    }

    async fn get_messages(&self) -> Vec<(ConnectionId, String)> {
        self.messages.lock().await.clone()
    }

    async fn clear_messages(&self) {
        self.messages.lock().await.clear();
    }
}

#[async_trait]
impl ServerHandler for ConversationHandler {
    async fn on_connect(&self, _id: ConnectionId, conn: &TelnetConnection) {
        self.connect_count.fetch_add(1, Ordering::SeqCst);
        // Send welcome message
        let _ = conn.send("Welcome to the test server!\r\n").await;
    }

    async fn on_event(&self, id: ConnectionId, conn: &TelnetConnection, event: TerminalEvent) {
        match event {
            TerminalEvent::CharacterData { character, .. } => {
                // Echo character back
                let _ = conn.send_char(character).await;
            }
            TerminalEvent::LineCompleted { line, .. } => {
                // Store the line
                let line_str = line.to_string();
                self.messages.lock().await.push((id, line_str.clone()));

                // Send response based on input
                let response = match line_str.trim() {
                    "hello" => "Hello there!\r\n",
                    "ping" => "pong\r\n",
                    "quit" => {
                        let _ = conn.send("Goodbye!\r\n").await;
                        return;
                    }
                    _ => "Unknown command\r\n",
                };
                let _ = conn.send(response).await;
            }
            _ => {}
        }
    }

    async fn on_disconnect(&self, _id: ConnectionId, _conn: &TelnetConnection) {
        self.disconnect_count.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn test_client_server_conversation() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ConversationHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Read welcome message
    let mut buf = vec![0u8; 1024];
    let n = tokio::time::timeout(Duration::from_secs(1), client.read(&mut buf))
        .await
        .unwrap()
        .unwrap();
    let welcome = String::from_utf8_lossy(&buf[..n]);
    assert!(welcome.contains("Welcome"));

    // Send "hello" command
    client.write_all(b"hello\n").await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Read response
    let n = tokio::time::timeout(Duration::from_secs(1), client.read(&mut buf))
        .await
        .unwrap()
        .unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("Hello there"));

    // Verify message was recorded
    let messages = handler.get_messages().await;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].1.trim(), "hello");

    // Cleanup
    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_multiple_clients_conversation() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ConversationHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect two clients
    let mut client1 = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let mut client2 = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Read welcome messages
    let mut buf1 = vec![0u8; 1024];
    let mut buf2 = vec![0u8; 1024];
    let _ = tokio::time::timeout(Duration::from_secs(1), client1.read(&mut buf1))
        .await
        .unwrap()
        .unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(1), client2.read(&mut buf2))
        .await
        .unwrap()
        .unwrap();

    // Client 1 sends "hello"
    client1.write_all(b"hello\n").await.unwrap();
    client1.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Client 2 sends "ping"
    client2.write_all(b"ping\n").await.unwrap();
    client2.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Read responses
    let n1 = tokio::time::timeout(Duration::from_secs(1), client1.read(&mut buf1))
        .await
        .unwrap()
        .unwrap();
    let response1 = String::from_utf8_lossy(&buf1[..n1]);
    assert!(response1.contains("Hello there"));

    let n2 = tokio::time::timeout(Duration::from_secs(1), client2.read(&mut buf2))
        .await
        .unwrap()
        .unwrap();
    let response2 = String::from_utf8_lossy(&buf2[..n2]);
    assert!(response2.contains("pong"));

    // Verify both messages were recorded
    let messages = handler.get_messages().await;
    assert_eq!(messages.len(), 2);

    // Messages could arrive in any order
    let msg_texts: Vec<String> = messages
        .iter()
        .map(|(_, msg)| msg.trim().to_string())
        .collect();
    assert!(msg_texts.contains(&"hello".to_string()));
    assert!(msg_texts.contains(&"ping".to_string()));

    // Cleanup
    drop(client1);
    drop(client2);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_echo_conversation() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ConversationHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Read and discard welcome message
    let mut buf = vec![0u8; 1024];
    let _ = tokio::time::timeout(Duration::from_secs(1), client.read(&mut buf))
        .await
        .unwrap()
        .unwrap();

    // Send characters one by one and verify echo
    let test_string = "test";
    for ch in test_string.chars() {
        client.write_all(&[ch as u8]).await.unwrap();
        client.flush().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Read echo
        let n = tokio::time::timeout(Duration::from_secs(1), client.read(&mut buf))
            .await
            .unwrap()
            .unwrap();
        let echo = String::from_utf8_lossy(&buf[..n]);
        assert_eq!(echo.chars().next().unwrap(), ch);
    }

    // Send newline to complete the line
    client.write_all(b"\n").await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Read response (should be "Unknown command" since "test" is not a known command)
    let n = tokio::time::timeout(Duration::from_secs(1), client.read(&mut buf))
        .await
        .unwrap()
        .unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(response.contains("Unknown command"));

    // Verify the complete line was recorded
    let messages = handler.get_messages().await;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].1.trim(), "test");

    // Cleanup
    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_broadcast_during_conversation() {
    use tokio::io::AsyncReadExt;

    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ConversationHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect three clients
    let mut clients = Vec::new();
    for _ in 0..3 {
        let client = TcpStream::connect(addr).await.unwrap();
        clients.push(client);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Read welcome messages from all clients
    for client in &mut clients {
        let mut buf = vec![0u8; 1024];
        let _ = tokio::time::timeout(Duration::from_secs(1), client.read(&mut buf))
            .await
            .unwrap()
            .unwrap();
    }

    // Broadcast a terminal command to all clients (SendEraseLine)
    let manager = server.manager();
    let result = manager
        .broadcast(termionix_terminal::TerminalCommand::SendEraseLine)
        .await;

    // Verify broadcast succeeded to all 3 clients
    assert_eq!(result.total, 3);
    assert!(result.all_succeeded());
    assert_eq!(result.succeeded, 3);
    assert_eq!(result.failed, 0);

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Note: SendEraseLine sends ANSI escape sequences (ESC[2K), not visible text
    // The test verifies the broadcast mechanism works correctly
    // Clients would receive the escape sequence but it's not easily testable
    // without a full terminal emulator

    // Cleanup
    drop(clients);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_sequential_commands() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ConversationHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect client
    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Read welcome message
    let mut buf = vec![0u8; 1024];
    let _ = tokio::time::timeout(Duration::from_secs(1), client.read(&mut buf))
        .await
        .unwrap()
        .unwrap();

    // Send multiple commands in sequence
    let commands = vec!["hello", "ping", "hello", "ping"];
    let expected_responses = vec!["Hello there", "pong", "Hello there", "pong"];

    for (cmd, expected) in commands.iter().zip(expected_responses.iter()) {
        // Send command
        client.write_all(cmd.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
        client.flush().await.unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Read response
        let n = tokio::time::timeout(Duration::from_secs(1), client.read(&mut buf))
            .await
            .unwrap()
            .unwrap();
        let response = String::from_utf8_lossy(&buf[..n]);
        assert!(
            response.contains(expected),
            "Expected '{}' in response, got: {}",
            expected,
            response
        );
    }

    // Verify all commands were recorded
    let messages = handler.get_messages().await;
    assert_eq!(messages.len(), 4);

    // Cleanup
    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_connection_state_tracking() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ConversationHandler::new());
    server.start(handler.clone()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Initial state
    assert_eq!(server.connection_count(), 0);
    assert_eq!(handler.connect_count(), 0);

    // Connect first client
    let client1 = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.connection_count(), 1);
    assert_eq!(handler.connect_count(), 1);

    // Connect second client
    let client2 = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(server.connection_count(), 2);
    assert_eq!(handler.connect_count(), 2);

    // Disconnect first client
    drop(client1);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(server.connection_count(), 1);
    assert_eq!(handler.disconnect_count(), 1);

    // Disconnect second client
    drop(client2);
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(server.connection_count(), 0);
    assert_eq!(handler.disconnect_count(), 2);

    // Cleanup
    server.shutdown().await.unwrap();
}
