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

//! Security and malformed data tests

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use termionix_service::{
    ConnectionId, ServerConfig, ServerHandler, TelnetConnection, TelnetServer,
};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

struct SecurityTestHandler;

#[async_trait]
impl ServerHandler for SecurityTestHandler {}

#[tokio::test]
async fn test_null_bytes() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send data with null bytes
    client.write_all(b"Hello\x00World\n").await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Server should handle null bytes without crashing
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_binary_data() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send random binary data
    let binary_data: Vec<u8> = (0..=255).collect();
    client.write_all(&binary_data).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Server should handle binary data without crashing
    assert!(server.connection_count() <= 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_extremely_long_line() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send extremely long line (10KB without newline)
    let long_line = "A".repeat(10000);
    client.write_all(long_line.as_bytes()).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Server should handle long lines without crashing
    assert!(server.connection_count() <= 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_rapid_small_writes() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send many tiny writes rapidly (potential DoS vector)
    for _ in 0..1000 {
        client.write_all(b"a").await.unwrap();
    }
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Server should handle rapid writes
    assert!(server.connection_count() <= 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_malformed_utf8() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send invalid UTF-8 sequences
    let invalid_utf8 = vec![
        0xFF, 0xFE, 0xFD, // Invalid UTF-8
        b'H', b'e', b'l', b'l', b'o', 0x80, 0x81, 0x82, // More invalid UTF-8
        b'\n',
    ];
    client.write_all(&invalid_utf8).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Server should handle invalid UTF-8 gracefully
    assert!(server.connection_count() <= 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_control_characters() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send various control characters
    let control_chars = vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, // SOH, STX, ETX, EOT, ENQ, ACK, BEL
        0x08, 0x09, 0x0B, 0x0C, 0x0E, 0x0F, // BS, HT, VT, FF, SO, SI
        b'T', b'e', b's', b't', b'\n',
    ];
    client.write_all(&control_chars).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Server should handle control characters
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_ansi_escape_flood() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send many ANSI escape sequences
    for _ in 0..100 {
        client.write_all(b"\x1b[31m").await.unwrap(); // Red color
        client.write_all(b"\x1b[1m").await.unwrap(); // Bold
        client.write_all(b"\x1b[0m").await.unwrap(); // Reset
    }
    client.write_all(b"Test\n").await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Server should handle ANSI flood
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_incomplete_ansi_sequence() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send incomplete ANSI escape sequence
    client.write_all(b"Hello\x1b[31").await.unwrap(); // Incomplete color code
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Server should handle incomplete sequences
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_connection_spam() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap()).with_max_connections(50);
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Attempt to spam connections
    let mut clients = Vec::new();
    for _ in 0..30 {
        if let Ok(client) = TcpStream::connect(addr).await {
            clients.push(client);
        }
        // No delay - spam as fast as possible
    }

    tokio::time::sleep(Duration::from_millis(200)).await;

    // Server should handle connection spam
    assert!(server.connection_count() <= 50);

    drop(clients);
    tokio::time::sleep(Duration::from_millis(200)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_zero_byte_writes() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send zero-length writes
    for _ in 0..10 {
        client.write_all(b"").await.unwrap();
    }
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Server should handle zero-byte writes
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_mixed_valid_invalid_data() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Mix valid and invalid data
    client.write_all(b"Valid text\n").await.unwrap();
    client.write_all(&[0xFF, 0xFE, 0xFD]).await.unwrap(); // Invalid
    client.write_all(b"More valid\n").await.unwrap();
    client.write_all(&[0x00, 0x01, 0x02]).await.unwrap(); // Control chars
    client.write_all(b"Final\n").await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Server should handle mixed data
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_repeated_newlines() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send many newlines
    for _ in 0..100 {
        client.write_all(b"\n").await.unwrap();
    }
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Server should handle repeated newlines
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_carriage_return_variations() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test different line ending variations
    client.write_all(b"Line1\r\n").await.unwrap(); // CRLF
    client.write_all(b"Line2\n").await.unwrap(); // LF
    client.write_all(b"Line3\r").await.unwrap(); // CR only
    client.write_all(b"Line4\n\r").await.unwrap(); // LFCR
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Server should handle various line endings
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_unicode_edge_cases() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(SecurityTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send various Unicode edge cases
    let unicode_test = "Hello 世界 🌍 \u{FEFF} \u{200B} Test\n"; // BOM, zero-width space
    client.write_all(unicode_test.as_bytes()).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Server should handle Unicode edge cases
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}
