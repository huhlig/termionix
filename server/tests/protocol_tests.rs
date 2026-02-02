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

//! Protocol negotiation and telnet-specific tests

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use termionix_server::{ServerConfig, ServerHandler, TelnetServer};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

struct ProtocolTestHandler;

#[async_trait]
impl ServerHandler for ProtocolTestHandler {}

// Telnet sidechannel constants
const IAC: u8 = 255; // Interpret As Command
const WILL: u8 = 251;
const WONT: u8 = 252;
const DO: u8 = 253;
const DONT: u8 = 254;
const SB: u8 = 250; // Subnegotiation Begin
const SE: u8 = 240; // Subnegotiation End

// Telnet options
const ECHO: u8 = 1;
const SUPPRESS_GO_AHEAD: u8 = 3;
const TERMINAL_TYPE: u8 = 24;
const NAWS: u8 = 31; // Negotiate About Window Size
const LINEMODE: u8 = 34;

#[tokio::test]
async fn test_telnet_iac_escape() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ProtocolTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send IAC IAC (escaped IAC byte = literal 255)
    client.write_all(&[IAC, IAC]).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Server should handle this correctly
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_protocol_auto_response() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ProtocolTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send WILL ECHO
    client.write_all(&[IAC, WILL, ECHO]).await.unwrap();
    client.flush().await.unwrap();

    // Should receive DO ECHO or DONT ECHO immediately
    let mut buf = [0u8; 3];
    let result =
        tokio::time::timeout(Duration::from_millis(500), client.read_exact(&mut buf)).await;

    match result {
        Ok(Ok(_)) => {
            // Verify we got a valid telnet response
            assert_eq!(buf[0], IAC, "First byte should be IAC");
            assert!(
                buf[1] == DO || buf[1] == DONT,
                "Second byte should be DO or DONT, got {}",
                buf[1]
            );
            assert_eq!(buf[2], ECHO, "Third byte should be ECHO option");
        }
        Ok(Err(e)) => {
            panic!("Error reading response: {}", e);
        }
        Err(_) => {
            panic!("Timeout waiting for protocol response - auto-response not working");
        }
    }

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_telnet_will_echo() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ProtocolTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Send WILL ECHO
    client.write_all(&[IAC, WILL, ECHO]).await.unwrap();
    client.flush().await.unwrap();

    // Give the server more time to process and respond
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Read server response (should be DO ECHO or DONT ECHO)
    let mut buf = vec![0u8; 1024];
    let n = tokio::time::timeout(Duration::from_secs(2), client.read(&mut buf))
        .await
        .unwrap()
        .unwrap();

    // Verify we got a response
    assert!(n > 0);

    // Verify it's a valid telnet response (IAC followed by DO or DONT)
    assert_eq!(buf[0], IAC);
    assert!(buf[1] == DO || buf[1] == DONT);
    assert_eq!(buf[2], ECHO);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_telnet_naws_negotiation() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ProtocolTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send WILL NAWS
    client.write_all(&[IAC, WILL, NAWS]).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send NAWS subnegotiation: 80x24
    // IAC SB NAWS 0 80 0 24 IAC SE
    client
        .write_all(&[IAC, SB, NAWS, 0, 80, 0, 24, IAC, SE])
        .await
        .unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Connection should still be active
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_telnet_suppress_go_ahead() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ProtocolTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send WILL SUPPRESS-GO-AHEAD
    client
        .write_all(&[IAC, WILL, SUPPRESS_GO_AHEAD])
        .await
        .unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connection should handle this
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_telnet_terminal_type() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ProtocolTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send WILL TERMINAL-TYPE
    client.write_all(&[IAC, WILL, TERMINAL_TYPE]).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send terminal type subnegotiation: "xterm-256color"
    let term_type = b"xterm-256color";
    let mut sb_data = vec![IAC, SB, TERMINAL_TYPE, 0]; // IS = 0
    sb_data.extend_from_slice(term_type);
    sb_data.extend_from_slice(&[IAC, SE]);

    client.write_all(&sb_data).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Connection should still be active
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_mixed_telnet_and_text() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ProtocolTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send text mixed with telnet commands
    client.write_all(b"Hello").await.unwrap();
    client.write_all(&[IAC, WILL, ECHO]).await.unwrap();
    client.write_all(b" World\n").await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Connection should handle mixed data
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_multiple_telnet_commands() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ProtocolTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send multiple telnet commands in sequence
    client.write_all(&[IAC, WILL, ECHO]).await.unwrap();
    client
        .write_all(&[IAC, WILL, SUPPRESS_GO_AHEAD])
        .await
        .unwrap();
    client.write_all(&[IAC, WILL, NAWS]).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Connection should handle all commands
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_telnet_dont_command() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ProtocolTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send DONT ECHO
    client.write_all(&[IAC, DONT, ECHO]).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connection should handle DONT
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_telnet_wont_command() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ProtocolTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send WONT ECHO
    client.write_all(&[IAC, WONT, ECHO]).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connection should handle WONT
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_incomplete_telnet_command() {
    let config = ServerConfig::new("127.0.0.1:0".parse().unwrap());
    let server = TelnetServer::new(config).await.unwrap();
    let addr = server.bind_address();

    let handler = Arc::new(ProtocolTestHandler);
    server.start(handler).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut client = TcpStream::connect(addr).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send incomplete command (IAC WILL without option)
    client.write_all(&[IAC, WILL]).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send the rest later
    client.write_all(&[ECHO]).await.unwrap();
    client.flush().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connection should handle fragmented commands
    assert_eq!(server.connection_count(), 1);

    drop(client);
    tokio::time::sleep(Duration::from_millis(100)).await;
    server.shutdown().await.unwrap();
}
