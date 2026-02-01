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

//! Comprehensive correctness tests for Termionix
//!
//! This test suite performs a full assessment of a client connected to a server,
//! testing option handling, subnegotiation, text data, binary data, and protocol correctness.

use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use termionix_service::{
    ConnectionId, ServerConfig, ServerHandler, TelnetArgument, TelnetConnection, TelnetOption,
    TelnetServer, TerminalEvent,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Notify, RwLock};

// ============================================================================
// Test Server Handler
// ============================================================================

struct TestServerHandler {
    echo_enabled: Arc<AtomicBool>,
    binary_enabled: Arc<AtomicBool>,
    naws_received: Arc<RwLock<Option<(u16, u16)>>>,
    terminal_type_received: Arc<RwLock<Option<String>>>,
    messages_received: Arc<AtomicU64>,
    characters_received: Arc<AtomicU64>,
    lines_received: Arc<RwLock<Vec<String>>>,
    options_enabled: Arc<RwLock<Vec<(TelnetOption, bool)>>>,
    subnegotiations: Arc<RwLock<Vec<TelnetArgument>>>,
    connected: Arc<Notify>,
    disconnected: Arc<Notify>,
}

impl TestServerHandler {
    fn new() -> Self {
        Self {
            echo_enabled: Arc::new(AtomicBool::new(false)),
            binary_enabled: Arc::new(AtomicBool::new(false)),
            naws_received: Arc::new(RwLock::new(None)),
            terminal_type_received: Arc::new(RwLock::new(None)),
            messages_received: Arc::new(AtomicU64::new(0)),
            characters_received: Arc::new(AtomicU64::new(0)),
            lines_received: Arc::new(RwLock::new(Vec::new())),
            options_enabled: Arc::new(RwLock::new(Vec::new())),
            subnegotiations: Arc::new(RwLock::new(Vec::new())),
            connected: Arc::new(Notify::new()),
            disconnected: Arc::new(Notify::new()),
        }
    }
}

#[async_trait]
impl ServerHandler for TestServerHandler {
    async fn on_connect(&self, _id: ConnectionId, _conn: &TelnetConnection) {
        self.connected.notify_one();
    }

    async fn on_disconnect(&self, _id: ConnectionId, _conn: &TelnetConnection) {
        self.disconnected.notify_one();
    }

    async fn on_event(&self, _id: ConnectionId, conn: &TelnetConnection, event: TerminalEvent) {
        match event {
            TerminalEvent::LineCompleted { line, .. } => {
                self.messages_received.fetch_add(1, Ordering::Relaxed);
                let text = line.to_string();
                self.lines_received.write().await.push(text.clone());
                let _ = conn.send(&format!("ECHO: {}\r\n", text)).await;
            }
            TerminalEvent::CharacterData { character, .. } => {
                self.characters_received.fetch_add(1, Ordering::Relaxed);
                if self.echo_enabled.load(Ordering::Relaxed) {
                    let _ = conn.send_char(character).await;
                }
            }
            _ => {}
        }
    }

    async fn on_option_enabled(
        &self,
        _id: ConnectionId,
        _conn: &TelnetConnection,
        option: TelnetOption,
        local: bool,
    ) {
        self.options_enabled.write().await.push((option, local));

        match option {
            TelnetOption::Echo => {
                self.echo_enabled.store(true, Ordering::Relaxed);
            }
            TelnetOption::TransmitBinary => {
                self.binary_enabled.store(true, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    async fn on_subnegotiation(
        &self,
        _id: ConnectionId,
        _conn: &TelnetConnection,
        subneg: TelnetArgument,
    ) {
        // Store for verification
        self.subnegotiations.write().await.push(subneg.clone());

        // Extract specific data
        match subneg {
            TelnetArgument::NAWSWindowSize(windowsize) => {
                *self.naws_received.write().await = Some((windowsize.cols, windowsize.rows));
            }
            TelnetArgument::TerminalType(terminal_type) => {
                *self.terminal_type_received.write().await = Some(terminal_type);
            }
            _ => {}
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn setup_test_server(
    handler: Arc<dyn ServerHandler>,
) -> Result<(TelnetServer, std::net::SocketAddr), Box<dyn std::error::Error>> {
    let config = ServerConfig::new("127.0.0.1:0".parse()?)
        .with_max_connections(100)
        .with_idle_timeout(Duration::from_secs(60));

    let server = TelnetServer::new(config).await?;
    let addr = server.bind_address();
    server.start(handler).await?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    Ok((server, addr))
}

// ============================================================================
// Test: Basic Connection and Disconnection
// ============================================================================

#[tokio::test]
async fn test_basic_connection() {
    let handler = Arc::new(TestServerHandler::new());
    let (server, addr) = setup_test_server(handler.clone()).await.unwrap();

    // Connect client
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();

    // Wait for connection
    tokio::time::timeout(Duration::from_secs(1), handler.connected.notified())
        .await
        .unwrap();

    // Disconnect
    drop(stream);

    // Wait for disconnection
    tokio::time::timeout(Duration::from_secs(1), handler.disconnected.notified())
        .await
        .unwrap();

    server.shutdown().await.unwrap();
}

// ============================================================================
// Test: Text Data Transfer
// ============================================================================

#[tokio::test]
async fn test_text_data_transfer() {
    let handler = Arc::new(TestServerHandler::new());
    let (server, addr) = setup_test_server(handler.clone()).await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

    tokio::time::timeout(Duration::from_secs(1), handler.connected.notified())
        .await
        .unwrap();

    // Send text messages
    for i in 0..10 {
        let msg = format!("Test message {}\r\n", i);
        stream.write_all(msg.as_bytes()).await.unwrap();
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify messages received
    assert_eq!(handler.messages_received.load(Ordering::Relaxed), 10);

    // Verify lines were captured
    let lines = handler.lines_received.read().await;
    assert_eq!(lines.len(), 10);
    assert_eq!(lines[0], "Test message 0");
    assert_eq!(lines[9], "Test message 9");

    // Read echoed responses
    let mut buffer = vec![0u8; 2048];
    let n = tokio::time::timeout(Duration::from_secs(1), stream.read(&mut buffer))
        .await
        .unwrap()
        .unwrap();

    assert!(n > 0);
    let response = String::from_utf8_lossy(&buffer[..n]);
    assert!(response.contains("ECHO:"));
    assert!(response.contains("Test message 0"));

    server.shutdown().await.unwrap();
}

// ============================================================================
// Test: Binary Data Transfer
// ============================================================================

#[tokio::test]
async fn test_binary_data_transfer() {
    let handler = Arc::new(TestServerHandler::new());
    let (server, addr) = setup_test_server(handler.clone()).await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

    tokio::time::timeout(Duration::from_secs(1), handler.connected.notified())
        .await
        .unwrap();

    // Send IAC WILL TRANSMIT-BINARY
    stream.write_all(&[255, 251, 0]).await.unwrap();

    // Wait for negotiation
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify option was enabled
    assert!(handler.binary_enabled.load(Ordering::Relaxed));

    // Send binary data (including bytes that look like IAC)
    let binary_data: Vec<u8> = (0..=255).collect();
    stream.write_all(&binary_data).await.unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    server.shutdown().await.unwrap();
}

// ============================================================================
// Test: Option Negotiation - ECHO
// ============================================================================

#[tokio::test]
async fn test_echo_option_negotiation() {
    let handler = Arc::new(TestServerHandler::new());
    let (server, addr) = setup_test_server(handler.clone()).await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

    tokio::time::timeout(Duration::from_secs(1), handler.connected.notified())
        .await
        .unwrap();

    // Send IAC WILL ECHO
    stream.write_all(&[255, 251, 1]).await.unwrap();

    // Wait for negotiation
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify option was tracked
    let options = handler.options_enabled.read().await;
    assert!(!options.is_empty());

    // Read response (should be IAC DO ECHO or IAC DONT ECHO)
    let mut buffer = vec![0u8; 10];
    let n = tokio::time::timeout(Duration::from_secs(1), stream.read(&mut buffer))
        .await
        .unwrap()
        .unwrap();

    assert!(n >= 3);
    assert_eq!(buffer[0], 255); // IAC

    server.shutdown().await.unwrap();
}

// ============================================================================
// Test: Option Negotiation - NAWS (Negotiate About Window Size)
// ============================================================================

#[tokio::test]
async fn test_naws_subnegotiation() {
    let handler = Arc::new(TestServerHandler::new());
    let (server, addr) = setup_test_server(handler.clone()).await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

    tokio::time::timeout(Duration::from_secs(1), handler.connected.notified())
        .await
        .unwrap();

    // Send IAC WILL NAWS
    stream.write_all(&[255, 251, 31]).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send IAC SB NAWS width(80) height(24) IAC SE
    // Width = 80 (0x0050), Height = 24 (0x0018)
    stream
        .write_all(&[255, 250, 31, 0, 80, 0, 24, 255, 240])
        .await
        .unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify NAWS was received
    let naws = handler.naws_received.read().await;
    assert_eq!(*naws, Some((80, 24)));

    // Verify subnegotiation was tracked
    let subneg = handler.subnegotiations.read().await;
    assert_eq!(subneg.len(), 1);

    server.shutdown().await.unwrap();
}

// ============================================================================
// Test: Option Negotiation - Terminal Type
// ============================================================================

#[tokio::test]
async fn test_terminal_type_subnegotiation() {
    let handler = Arc::new(TestServerHandler::new());
    let (server, addr) = setup_test_server(handler.clone()).await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

    tokio::time::timeout(Duration::from_secs(1), handler.connected.notified())
        .await
        .unwrap();

    // Send IAC WILL TERMINAL-TYPE
    stream.write_all(&[255, 251, 24]).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send IAC SB TERMINAL-TYPE IS "xterm-256color" IAC SE
    let mut subneg = vec![255, 250, 24, 0]; // IAC SB TTYPE IS
    subneg.extend_from_slice(b"xterm-256color");
    subneg.extend_from_slice(&[255, 240]); // IAC SE
    stream.write_all(&subneg).await.unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify terminal type was received
    let ttype = handler.terminal_type_received.read().await;
    assert_eq!(ttype.as_deref(), Some("xterm-256color"));

    // Verify subnegotiation was tracked
    let subneg = handler.subnegotiations.read().await;
    assert_eq!(subneg.len(), 1);

    server.shutdown().await.unwrap();
}

// ============================================================================
// Test: Multiple Options Negotiation
// ============================================================================

#[tokio::test]
async fn test_multiple_options_negotiation() {
    let handler = Arc::new(TestServerHandler::new());
    let (server, addr) = setup_test_server(handler.clone()).await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

    tokio::time::timeout(Duration::from_secs(1), handler.connected.notified())
        .await
        .unwrap();

    // Negotiate multiple options
    stream.write_all(&[255, 251, 1]).await.unwrap(); // WILL ECHO
    stream.write_all(&[255, 251, 0]).await.unwrap(); // WILL TRANSMIT-BINARY
    stream.write_all(&[255, 251, 31]).await.unwrap(); // WILL NAWS
    stream.write_all(&[255, 251, 24]).await.unwrap(); // WILL TERMINAL-TYPE

    // Wait for negotiations
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Send NAWS subnegotiation
    stream
        .write_all(&[255, 250, 31, 0, 80, 0, 24, 255, 240])
        .await
        .unwrap();

    // Send Terminal Type subnegotiation
    let mut subneg = vec![255, 250, 24, 0];
    subneg.extend_from_slice(b"xterm");
    subneg.extend_from_slice(&[255, 240]);
    stream.write_all(&subneg).await.unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify all options were processed
    let options = handler.options_enabled.read().await;
    assert!(!options.is_empty());

    let naws = handler.naws_received.read().await;
    assert_eq!(*naws, Some((80, 24)));

    let ttype = handler.terminal_type_received.read().await;
    assert_eq!(ttype.as_deref(), Some("xterm"));

    let subneg = handler.subnegotiations.read().await;
    assert_eq!(subneg.len(), 2);

    server.shutdown().await.unwrap();
}

// ============================================================================
// Test: IAC Escaping in Data
// ============================================================================

#[tokio::test]
async fn test_iac_escaping() {
    let handler = Arc::new(TestServerHandler::new());
    let (server, addr) = setup_test_server(handler.clone()).await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

    tokio::time::timeout(Duration::from_secs(1), handler.connected.notified())
        .await
        .unwrap();

    // Send data containing IAC byte (0xFF) - should be escaped as IAC IAC
    let data_with_escaped_iac = b"Test\xFF\xFFData\r\n";
    stream.write_all(data_with_escaped_iac).await.unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Should receive the message
    assert_eq!(handler.messages_received.load(Ordering::Relaxed), 1);

    server.shutdown().await.unwrap();
}

// ============================================================================
// Test: Concurrent Connections
// ============================================================================

#[tokio::test]
async fn test_concurrent_connections() {
    let handler = Arc::new(TestServerHandler::new());
    let (server, addr) = setup_test_server(handler.clone()).await.unwrap();

    let mut handles = Vec::new();

    // Create 10 concurrent connections
    for i in 0..10 {
        let addr = addr.clone();
        let handle = tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

            // Send a message
            let msg = format!("Client {} message\r\n", i);
            stream.write_all(msg.as_bytes()).await.unwrap();

            // Wait a bit
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Read response
            let mut buffer = vec![0u8; 256];
            let _ = stream.read(&mut buffer).await;
        });
        handles.push(handle);
    }

    // Wait for all connections
    for handle in handles {
        handle.await.unwrap();
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Verify messages received
    assert_eq!(handler.messages_received.load(Ordering::Relaxed), 10);

    server.shutdown().await.unwrap();
}

// ============================================================================
// Test: Large Data Transfer
// ============================================================================

#[tokio::test]
async fn test_large_data_transfer() {
    let handler = Arc::new(TestServerHandler::new());
    let (server, addr) = setup_test_server(handler.clone()).await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

    tokio::time::timeout(Duration::from_secs(1), handler.connected.notified())
        .await
        .unwrap();

    // Send large message (10KB)
    let large_msg = "X".repeat(10000);
    let msg_with_newline = format!("{}\r\n", large_msg);
    stream.write_all(msg_with_newline.as_bytes()).await.unwrap();

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify message received
    assert_eq!(handler.messages_received.load(Ordering::Relaxed), 1);

    // Verify the line was captured correctly
    let lines = handler.lines_received.read().await;
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].len(), 10000);

    server.shutdown().await.unwrap();
}

// ============================================================================
// Test: Protocol Correctness - Reject Unsupported Options
// ============================================================================

#[tokio::test]
async fn test_reject_unsupported_options() {
    let handler = Arc::new(TestServerHandler::new());
    let (server, addr) = setup_test_server(handler.clone()).await.unwrap();

    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();

    tokio::time::timeout(Duration::from_secs(1), handler.connected.notified())
        .await
        .unwrap();

    // Send IAC WILL for an unsupported option (e.g., option 200)
    stream.write_all(&[255, 251, 200]).await.unwrap();

    // Wait for response
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Read response (should be IAC DONT 200)
    let mut buffer = vec![0u8; 10];
    let n = tokio::time::timeout(Duration::from_secs(1), stream.read(&mut buffer))
        .await
        .unwrap()
        .unwrap();

    assert!(n >= 3);
    assert_eq!(buffer[0], 255); // IAC
    assert_eq!(buffer[1], 254); // DONT
    assert_eq!(buffer[2], 200); // Option 200

    server.shutdown().await.unwrap();
}
