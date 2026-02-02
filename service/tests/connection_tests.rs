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

//! Comprehensive tests for SplitTerminalConnection

use termionix_ansicodec::{AnsiCodec, AnsiConfig, SegmentedString};
use termionix_service::{FlushStrategy, SplitTerminalConnection};
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

    // Create separate codecs for read and write for each connection
    let codec1 = create_codec();
    let codec2 = create_codec();

    let (r1, w1) = tokio::io::split(stream1);
    let (r2, w2) = tokio::io::split(stream2);

    let conn1 = SplitTerminalConnection::new(r1, w1, codec1.clone(), codec1);
    let conn2 = SplitTerminalConnection::new(r2, w2, codec2.clone(), codec2);

    (conn1, conn2)
}

#[tokio::test]
async fn test_connection_creation() {
    let (conn1, conn2) = create_test_connection();

    // Connections should be created successfully
    assert!(conn1.flush_strategy().await == FlushStrategy::OnNewline);
    assert!(conn2.flush_strategy().await == FlushStrategy::OnNewline);
}

#[tokio::test]
async fn test_send_and_receive_text() {
    let (conn1, conn2) = create_test_connection();

    // Send text from conn1 with newline to trigger LineCompleted
    conn1
        .send(TerminalCommand::Text("Hello, World!\n".to_string()), true)
        .await
        .unwrap();

    // Receive events until we get LineCompleted
    let mut line_completed = None;
    for _ in 0..20 {
        let result = timeout(Duration::from_millis(100), conn2.next()).await;

        if let Ok(Ok(Some(event))) = result {
            if let TerminalEvent::LineCompleted { line, .. } = event {
                line_completed = Some(line);
                break;
            }
        } else {
            break;
        }
    }

    assert!(line_completed.is_some(), "Expected LineCompleted event");
    assert_eq!(
        line_completed.unwrap(),
        SegmentedString::from("Hello, World!")
    );
}

#[tokio::test]
async fn test_send_multiple_messages() {
    let (conn1, conn2) = create_test_connection();

    // Send multiple messages with newlines
    for i in 0..10 {
        let msg = format!("Message {}\n", i);
        conn1
            .send(TerminalCommand::Text(msg.to_string()), true)
            .await
            .unwrap();
    }

    // Receive all messages
    for i in 0..10 {
        let mut line_completed = None;
        for _ in 0..20 {
            let result = timeout(Duration::from_millis(100), conn2.next()).await;

            if let Ok(Ok(Some(event))) = result {
                if let TerminalEvent::LineCompleted { line, .. } = event {
                    line_completed = Some(line);
                    break;
                }
            } else {
                break;
            }
        }

        assert!(
            line_completed.is_some(),
            "Expected LineCompleted event for message {}",
            i
        );
        let expected = format!("Message {}", i);
        assert_eq!(line_completed.unwrap().to_string(), expected);
    }
}

#[tokio::test]
async fn test_bidirectional_communication() {
    let (conn1, conn2) = create_test_connection();

    // Send from conn1 to conn2 with newline
    conn1
        .send(TerminalCommand::Text("From 1 to 2\n".to_string()), true)
        .await
        .unwrap();

    // Wait for LineCompleted event
    let mut found = false;
    for _ in 0..20 {
        if let Ok(Ok(Some(event))) = timeout(Duration::from_millis(100), conn2.next()).await {
            if matches!(event, TerminalEvent::LineCompleted { .. }) {
                found = true;
                break;
            }
        } else {
            break;
        }
    }
    assert!(found, "Expected LineCompleted event from conn1 to conn2");

    // Send from conn2 to conn1 with newline
    conn2
        .send(TerminalCommand::Text("From 2 to 1\n".to_string()), true)
        .await
        .unwrap();

    // Wait for LineCompleted event
    let mut found = false;
    for _ in 0..20 {
        if let Ok(Ok(Some(event))) = timeout(Duration::from_millis(100), conn1.next()).await {
            if matches!(event, TerminalEvent::LineCompleted { .. }) {
                found = true;
                break;
            }
        } else {
            break;
        }
    }
    assert!(found, "Expected LineCompleted event from conn2 to conn1");
}

#[tokio::test]
async fn test_flush_strategy_immediate() {
    let (conn1, conn2) = create_test_connection();

    // Set immediate flush strategy
    conn1.set_flush_strategy(FlushStrategy::Immediate).await;
    assert_eq!(conn1.flush_strategy().await, FlushStrategy::Immediate);

    // Send without explicit flush
    conn1
        .send(TerminalCommand::Text("Test".to_string()), false)
        .await
        .unwrap();

    // Should still receive due to immediate flush strategy
    let result = timeout(Duration::from_secs(1), conn2.next())
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn test_flush_strategy_manual() {
    let (conn1, conn2) = create_test_connection();

    // Set manual flush strategy
    conn1.set_flush_strategy(FlushStrategy::Manual).await;
    assert_eq!(conn1.flush_strategy().await, FlushStrategy::Manual);

    // Send without flush
    conn1
        .send(TerminalCommand::text("Test"), false)
        .await
        .unwrap();

    // Manually flush
    conn1.flush().await.unwrap();

    // Should receive after manual flush
    let result = timeout(Duration::from_secs(1), conn2.next())
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn test_connection_clone() {
    let (conn1, _conn2) = create_test_connection();

    // Clone the connection
    let conn1_clone = conn1.clone();

    // Both should work independently
    conn1
        .send(TerminalCommand::text("From original"), true)
        .await
        .unwrap();

    conn1_clone
        .send(TerminalCommand::text("From clone"), true)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_concurrent_sends() {
    let (conn1, conn2) = create_test_connection();
    let conn1_clone = conn1.clone();

    // Spawn concurrent send tasks
    let task1 = tokio::spawn(async move {
        for i in 0..50 {
            conn1
                .send(TerminalCommand::Text(format!("Task1-{}", i)), true)
                .await
                .unwrap();
        }
    });

    let task2 = tokio::spawn(async move {
        for i in 0..50 {
            conn1_clone
                .send(TerminalCommand::Text(format!("Task2-{}", i)), true)
                .await
                .unwrap();
        }
    });

    // Wait for sends to complete
    task1.await.unwrap();
    task2.await.unwrap();

    // Receive all 100 messages
    let mut count = 0;
    while count < 100 {
        let result = timeout(Duration::from_secs(2), conn2.next())
            .await
            .unwrap()
            .unwrap();
        if result.is_some() {
            count += 1;
        } else {
            break;
        }
    }

    assert_eq!(count, 100);
}

#[tokio::test]
async fn test_large_message() {
    let (stream1, stream2) = duplex(65536); // Larger buffer for large messages
    let codec1 = create_codec();
    let codec2 = create_codec();

    let (r1, w1) = tokio::io::split(stream1);
    let (r2, w2) = tokio::io::split(stream2);

    let conn1 = SplitTerminalConnection::new(r1, w1, codec1.clone(), codec1);
    let conn2 = SplitTerminalConnection::new(r2, w2, codec2.clone(), codec2);

    // Send a large message (32KB) with newline
    let mut large_msg = vec![b'A'; 32768];
    large_msg.push(b'\n');
    conn1
        .send(TerminalCommand::Bytes(large_msg.clone()), true)
        .await
        .unwrap();

    // Receive events until we get LineCompleted
    let mut line_completed = None;
    for _ in 0..33000 {
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
    // Check length with default config (should be 32768, not including newline)
    assert_eq!(line.len(None).unwrap(), 32768);
    // Convert to string and check content
    let line_str = line.to_string();
    assert_eq!(line_str.len(), 32768);
    assert!(line_str.chars().all(|c| c == 'A'));
}

#[tokio::test]
async fn test_empty_message() {
    let (conn1, conn2) = create_test_connection();

    // Send empty message with newline to trigger LineCompleted
    conn1
        .send(TerminalCommand::Text("\n".to_string()), true)
        .await
        .unwrap();

    // Should still receive the event
    let result = timeout(Duration::from_secs(1), conn2.next())
        .await
        .unwrap()
        .unwrap();

    assert!(result.is_some());
    if let Some(TerminalEvent::LineCompleted { line, .. }) = result {
        assert_eq!(line.len(None).unwrap(), 0);
    } else {
        panic!("Expected LineCompleted event");
    }
}

#[tokio::test]
async fn test_connection_close() {
    let (conn1, conn2) = create_test_connection();

    // Send a message with newline
    conn1
        .send(TerminalCommand::text("Before close\n"), true)
        .await
        .unwrap();

    // Receive events until we get LineCompleted
    let mut found = false;
    for _ in 0..20 {
        if let Ok(Ok(Some(event))) = timeout(Duration::from_millis(100), conn2.next()).await {
            if matches!(event, TerminalEvent::LineCompleted { .. }) {
                found = true;
                break;
            }
        } else {
            break;
        }
    }
    assert!(found, "Expected to receive message before close");

    // Close the connection
    conn1.close().await.unwrap();

    // Drain any remaining CharacterData events, then expect None
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
async fn test_rapid_flush() {
    let (conn1, conn2) = create_test_connection();

    // Send and flush rapidly
    for i in 0..20 {
        conn1
            .send(TerminalCommand::Text(format!("Msg{}", i)), false)
            .await
            .unwrap();
        conn1.flush().await.unwrap();
    }

    // Receive all messages
    for _ in 0..20 {
        let result = timeout(Duration::from_secs(1), conn2.next())
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_some());
    }
}

#[tokio::test]
async fn test_flush_strategy_threshold() {
    let (conn1, conn2) = create_test_connection();

    // Set threshold flush strategy
    conn1
        .set_flush_strategy(FlushStrategy::OnThreshold(100))
        .await;
    assert_eq!(
        conn1.flush_strategy().await,
        FlushStrategy::OnThreshold(100)
    );

    // Send a message
    conn1
        .send(TerminalCommand::text("Test message"), true)
        .await
        .unwrap();

    // Should receive
    let result = timeout(Duration::from_secs(1), conn2.next())
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn test_send_with_force_flush() {
    let (conn1, conn2) = create_test_connection();

    // Send with force flush = true
    conn1
        .send(TerminalCommand::text("Forced flush"), true)
        .await
        .unwrap();

    // Should receive immediately
    let result = timeout(Duration::from_millis(500), conn2.next())
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn test_send_without_force_flush() {
    let (conn1, conn2) = create_test_connection();

    // Send without force flush
    conn1
        .send(TerminalCommand::text("No flush"), false)
        .await
        .unwrap();

    // Manually flush
    conn1.flush().await.unwrap();

    // Should receive after manual flush
    let result = timeout(Duration::from_secs(1), conn2.next())
        .await
        .unwrap()
        .unwrap();
    assert!(result.is_some());
}

#[tokio::test]
async fn test_multiple_clones() {
    let (conn1, conn2) = create_test_connection();

    // Create multiple clones
    let clone1 = conn1.clone();
    let clone2 = conn1.clone();
    let clone3 = conn1.clone();

    // All clones should be able to send
    conn1
        .send(TerminalCommand::text("Original"), true)
        .await
        .unwrap();
    clone1
        .send(TerminalCommand::text("Clone1"), true)
        .await
        .unwrap();
    clone2
        .send(TerminalCommand::text("Clone2"), true)
        .await
        .unwrap();
    clone3
        .send(TerminalCommand::text("Clone3"), true)
        .await
        .unwrap();

    // Receive all 4 messages
    for _ in 0..4 {
        let result = timeout(Duration::from_secs(1), conn2.next())
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_some());
    }
}

#[tokio::test]
async fn test_stress_many_small_messages() {
    let (conn1, conn2) = create_test_connection();

    // Send 1000 small messages
    for i in 0..1000 {
        conn1
            .send(TerminalCommand::Text(format!("{}", i)), true)
            .await
            .unwrap();
    }

    // Receive all 1000 messages
    for _ in 0..1000 {
        let result = timeout(Duration::from_secs(5), conn2.next())
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_some());
    }
}

#[tokio::test]
async fn test_alternating_send_receive() {
    let (conn1, conn2) = create_test_connection();

    // Alternate sending and receiving
    for i in 0..10 {
        // conn1 sends
        conn1
            .send(TerminalCommand::Text(format!("1to2-{}", i)), true)
            .await
            .unwrap();

        // conn2 receives
        let result = timeout(Duration::from_secs(1), conn2.next())
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_some());

        // conn2 sends
        conn2
            .send(TerminalCommand::Text(format!("2to1-{}", i)), true)
            .await
            .unwrap();

        // conn1 receives
        let result = timeout(Duration::from_secs(1), conn1.next())
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_some());
    }
}


