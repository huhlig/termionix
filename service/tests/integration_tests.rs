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

//! Integration tests for the termionix-service crate

use termionix_ansicodec::{AnsiCodec, AnsiConfig};
use termionix_service::{
    ClientConnectionConfig, Config, ConnectionError, FlushStrategy, ServerConnectionConfig,
    SplitTerminalConnection,
};
use termionix_telnetcodec::TelnetCodec;
use termionix_terminal::{TerminalCodec, TerminalCommand, TerminalEvent};
use tokio::io::duplex;
use tokio::time::{Duration, timeout};

type TestCodecStack = TerminalCodec<AnsiCodec<TelnetCodec>>;

/// Helper to create a codec
fn create_codec() -> TestCodecStack {
    let telnet_codec = TelnetCodec::new();
    let ansi_codec = AnsiCodec::new(AnsiConfig::default(), telnet_codec);
    TerminalCodec::new(ansi_codec)
}

/// Helper to create a test connection pair
fn create_test_connection() -> (
    SplitTerminalConnection<
        tokio::io::ReadHalf<tokio::io::DuplexStream>,
        tokio::io::WriteHalf<tokio::io::DuplexStream>,
        TestCodecStack,
    >,
    SplitTerminalConnection<
        tokio::io::ReadHalf<tokio::io::DuplexStream>,
        tokio::io::WriteHalf<tokio::io::DuplexStream>,
        TestCodecStack,
    >,
) {
    let (stream1, stream2) = duplex(8192);
    let codec1 = create_codec();
    let codec2 = create_codec();

    let (r1, w1) = tokio::io::split(stream1);
    let (r2, w2) = tokio::io::split(stream2);

    let conn1 = SplitTerminalConnection::new(r1, w1, codec1.clone(), codec1);
    let conn2 = SplitTerminalConnection::new(r2, w2, codec2.clone(), codec2);

    (conn1, conn2)
}

#[tokio::test]
async fn test_basic_send_receive() {
    let (conn1, conn2) = create_test_connection();

    // Send a message with newline
    conn1
        .send(TerminalCommand::text("Hello\n"), true)
        .await
        .unwrap();

    // Receive events until we get LineCompleted
    let mut found = false;
    for _ in 0..20 {
        if let Ok(Some(event)) = conn2.next().await {
            if matches!(event, TerminalEvent::LineCompleted { .. }) {
                found = true;
                break;
            }
        } else {
            break;
        }
    }
    assert!(found, "Expected LineCompleted event");
}

#[tokio::test]
async fn test_client_config_integration() {
    let config = ClientConnectionConfig::new("localhost", 23)
        .with_auto_reconnect(true)
        .with_terminal_size(120, 40)
        .with_buffer_size(16384);

    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 23);
    assert!(config.auto_reconnect);
    assert_eq!(config.common.terminal_width, 120);
    assert_eq!(config.common.terminal_height, 40);
    assert_eq!(config.common.buffer_size, 16384);
}

#[tokio::test]
async fn test_server_config_integration() {
    let config = ServerConnectionConfig::new()
        .with_max_idle_time(Some(Duration::from_secs(300)))
        .with_rate_limiting(true, Some(100))
        .with_terminal_size(80, 24);

    assert_eq!(config.max_idle_time, Some(Duration::from_secs(300)));
    assert!(config.rate_limiting);
    assert_eq!(config.max_messages_per_second, Some(100));
    assert_eq!(config.common.terminal_width, 80);
    assert_eq!(config.common.terminal_height, 24);
}

#[tokio::test]
async fn test_config_enum_usage() {
    let client_config = ClientConnectionConfig::new("example.com", 8080);
    let config = Config::Client(client_config);

    assert!(config.is_client());
    assert!(!config.is_server());

    let common = config.common();
    assert_eq!(common.terminal_type, "xterm-256color");
}

#[tokio::test]
async fn test_flush_strategy_workflow() {
    let (conn1, conn2) = create_test_connection();

    // Test default strategy
    assert_eq!(conn1.flush_strategy().await, FlushStrategy::OnNewline);

    // Change to immediate
    conn1.set_flush_strategy(FlushStrategy::Immediate).await;
    assert_eq!(conn1.flush_strategy().await, FlushStrategy::Immediate);

    // Send without explicit flush
    conn1
        .send(TerminalCommand::text("Test"), false)
        .await
        .unwrap();

    // Should receive due to immediate flush
    let event = timeout(Duration::from_secs(1), conn2.next())
        .await
        .unwrap()
        .unwrap();
    assert!(event.is_some());
}

#[tokio::test]
async fn test_connection_lifecycle() {
    let (conn1, conn2) = create_test_connection();

    // Send initial message with newline
    conn1
        .send(TerminalCommand::text("Start\n"), true)
        .await
        .unwrap();

    // Receive events until we get LineCompleted
    let mut found = false;
    for _ in 0..20 {
        if let Ok(Some(event)) = conn2.next().await {
            if matches!(event, TerminalEvent::LineCompleted { .. }) {
                found = true;
                break;
            }
        } else {
            break;
        }
    }
    assert!(found, "Expected initial message");

    // Send multiple messages with newlines
    for i in 0..5 {
        conn1
            .send(TerminalCommand::Text(format!("Msg{}\n", i)), true)
            .await
            .unwrap();
    }

    // Receive all LineCompleted events
    let mut line_count = 0;
    for _ in 0..100 {
        if let Ok(Ok(Some(event))) = timeout(Duration::from_millis(100), conn2.next()).await {
            if matches!(event, TerminalEvent::LineCompleted { .. }) {
                line_count += 1;
                if line_count >= 5 {
                    break;
                }
            }
        } else {
            break;
        }
    }
    assert_eq!(line_count, 5, "Expected 5 LineCompleted events");

    // Close connection
    conn1.close().await.unwrap();

    // Drain any remaining events, then expect None
    let mut got_none = false;
    for _ in 0..20 {
        match timeout(Duration::from_millis(100), conn2.next()).await {
            Ok(Ok(None)) => {
                got_none = true;
                break;
            }
            Ok(Ok(Some(_))) => continue, // Drain remaining events
            _ => break,
        }
    }
    assert!(got_none, "Expected None after connection close");
}

#[tokio::test]
async fn test_concurrent_operations() {
    let (conn1, conn2) = create_test_connection();
    let conn1_clone = conn1.clone();

    // Spawn concurrent tasks
    let send_task = tokio::spawn(async move {
        for i in 0..100 {
            conn1
                .send(TerminalCommand::Text(format!("Send-{}", i)), true)
                .await
                .unwrap();
        }
    });

    let clone_task = tokio::spawn(async move {
        for i in 0..100 {
            conn1_clone
                .send(TerminalCommand::Text(format!("Clone-{}", i)), true)
                .await
                .unwrap();
        }
    });

    // Wait for sends
    send_task.await.unwrap();
    clone_task.await.unwrap();

    // Receive all 200 messages
    let mut count = 0;
    while count < 200 {
        let event = timeout(Duration::from_secs(5), conn2.next())
            .await
            .unwrap()
            .unwrap();
        if event.is_some() {
            count += 1;
        } else {
            break;
        }
    }

    assert_eq!(count, 200);
}

#[tokio::test]
async fn test_error_handling_closed_connection() {
    let (conn1, _conn2) = create_test_connection();

    // Close connection
    conn1.close().await.unwrap();

    // Try to send after close - should fail
    let result = conn1.send(TerminalCommand::text("After close"), true).await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), ConnectionError::Closed));
}

#[tokio::test]
async fn test_large_data_transfer() {
    let (stream1, stream2) = duplex(131072); // 128KB buffer
    let codec1 = create_codec();
    let codec2 = create_codec();

    let (r1, w1) = tokio::io::split(stream1);
    let (r2, w2) = tokio::io::split(stream2);

    let conn1 = SplitTerminalConnection::new(r1, w1, codec1.clone(), codec1);
    let conn2 = SplitTerminalConnection::new(r2, w2, codec2.clone(), codec2);

    // Send a large message (64KB) with newline
    let large_msg = format!("{}\n", "A".repeat(65536));
    conn1
        .send(TerminalCommand::Text(large_msg.clone()), true)
        .await
        .unwrap();

    // Receive events until we get LineCompleted
    let mut line_completed = None;
    for _ in 0..66000 {
        // Need more iterations for large message
        if let Ok(Ok(Some(event))) = timeout(Duration::from_millis(10), conn2.next()).await {
            if let TerminalEvent::LineCompleted { line, .. } = event {
                line_completed = Some(line);
                break;
            }
        } else {
            break;
        }
    }

    assert!(line_completed.is_some(), "Expected LineCompleted event");
    let line = line_completed.unwrap();
    let line_str = line.to_string();
    assert_eq!(line_str.len(), 65536);
    assert!(line_str.chars().all(|c| c == 'A'));
}


