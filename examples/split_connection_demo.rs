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

//! Split Connection Architecture Demo
//!
//! This example demonstrates the new split read/write architecture that solves
//! the blocking issue where buffered writes would wait for read timeouts.
//!
//! Key features demonstrated:
//! - Independent read and write operations
//! - Configurable flush strategies
//! - Concurrent read/write without blocking
//! - Clean separation of concerns

use termionix_server::{
    ClientConfig, Config, ConnectionConfig, FlushStrategy, SplitConnection,
};
use termionix_ansicodec::{AnsiCodec, AnsiConfig};
use termionix_telnetcodec::TelnetCodec;
use termionix_terminal::TerminalCodec;
use tokio::net::TcpStream;
use std::time::Duration;

type FullCodec = TerminalCodec<AnsiCodec<TelnetCodec>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Split Connection Architecture Demo ===\n");

    // Example 1: Basic usage with default settings
    println!("Example 1: Basic Connection");
    demo_basic_connection().await?;

    // Example 2: Configurable flush strategies
    println!("\nExample 2: Flush Strategies");
    demo_flush_strategies().await?;

    // Example 3: Concurrent read/write operations
    println!("\nExample 3: Concurrent Operations");
    demo_concurrent_operations().await?;

    // Example 4: With configuration
    println!("\nExample 4: With Configuration");
    demo_with_config().await?;

    println!("\n=== Demo Complete ===");
    Ok(())
}

/// Demonstrates basic connection usage
async fn demo_basic_connection() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Connecting to echo server...");
    
    // Connect to a server (you'll need a running server for this to work)
    // For demo purposes, we'll show the API without actually connecting
    println!("  API Example:");
    println!("    let stream = TcpStream::connect(\"localhost:23\").await?;");
    println!("    let codec = create_codec();");
    println!("    let conn = SplitConnection::from_stream(stream, codec.clone(), codec);");
    println!("    ");
    println!("    // Send data - automatically flushes on newline (default strategy)");
    println!("    conn.send(\"Hello, World!\\n\", false).await?;");
    println!("    ");
    println!("    // Receive data - doesn't block writes!");
    println!("    if let Some(event) = conn.next().await? {{");
    println!("        println!(\"Received: {{:?}}\", event);");
    println!("    }}");
    
    Ok(())
}

/// Demonstrates different flush strategies
async fn demo_flush_strategies() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Available flush strategies:");
    println!("    - FlushStrategy::Manual       // No automatic flushing");
    println!("    - FlushStrategy::Immediate    // Flush after every send");
    println!("    - FlushStrategy::OnNewline    // Flush on \\n (default)");
    println!("    - FlushStrategy::OnThreshold  // Flush on buffer size");
    println!();
    println!("  Usage:");
    println!("    conn.set_flush_strategy(FlushStrategy::Immediate).await;");
    println!("    conn.send(\"This flushes immediately\", false).await?;");
    println!();
    println!("    conn.set_flush_strategy(FlushStrategy::Manual).await;");
    println!("    conn.send(\"Buffered\", false).await?;");
    println!("    conn.send(\"Also buffered\", false).await?;");
    println!("    conn.flush().await?; // Manual flush");
    
    Ok(())
}

/// Demonstrates concurrent read/write operations
async fn demo_concurrent_operations() -> Result<(), Box<dyn std::error::Error>> {
    println!("  The split architecture allows true concurrent operations:");
    println!();
    println!("  // Spawn a task to continuously send data");
    println!("  let write_handle = tokio::spawn(async move {{");
    println!("      loop {{");
    println!("          conn.send(\"Heartbeat\\n\", false).await?;");
    println!("          tokio::time::sleep(Duration::from_secs(1)).await;");
    println!("      }}");
    println!("  }});");
    println!();
    println!("  // Meanwhile, read operations continue without blocking writes");
    println!("  while let Some(event) = conn.next().await? {{");
    println!("      handle_event(event).await?;");
    println!("  }}");
    println!();
    println!("  Key benefit: Writes don't wait for reads to complete!");
    
    Ok(())
}

/// Demonstrates using configuration
async fn demo_with_config() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Creating connection with custom configuration:");
    
    // Create configuration
    let conn_config = ConnectionConfig {
        terminal_type: "xterm-256color".to_string(),
        terminal_width: 120,
        terminal_height: 40,
        read_buffer_size: 8192,
        write_buffer_size: 8192,
        keepalive_interval: Some(Duration::from_secs(30)),
    };
    
    let client_config = ClientConfig {
        connection: conn_config,
        host: "localhost".to_string(),
        port: 23,
        reconnect_strategy: termionix_server::ReconnectStrategy::ExponentialBackoff {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
        },
        connect_timeout: Duration::from_secs(10),
    };
    
    let config = Config::Client(client_config);
    
    println!("  Configuration created:");
    println!("    - Terminal: xterm-256color (120x40)");
    println!("    - Buffers: 8KB read/write");
    println!("    - Keepalive: 30s");
    println!("    - Reconnect: Exponential backoff");
    println!();
    println!("  Usage:");
    println!("    let conn = SplitConnection::from_stream_with_config(");
    println!("        stream, codec_read, codec_write, config");
    println!("    );");
    
    Ok(())
}

/// Helper function to create the codec stack
#[allow(dead_code)]
fn create_codec() -> FullCodec {
    let telnet_codec = TelnetCodec::new();
    let ansi_codec = AnsiCodec::new(AnsiConfig::default(), telnet_codec);
    TerminalCodec::new(ansi_codec)
}

/// Example of a real connection (commented out - requires running server)
#[allow(dead_code)]
async fn real_connection_example() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to server
    let stream = TcpStream::connect("localhost:23").await?;
    
    // Create codec stack
    let codec = create_codec();
    
    // Create split connection
    let conn = SplitConnection::from_stream(stream, codec.clone(), codec);
    
    // Set flush strategy
    conn.set_flush_strategy(FlushStrategy::OnNewline).await;
    
    // Send some data
    conn.send("Hello, Server!\n", false).await?;
    
    // Spawn a task to send periodic heartbeats
    let conn_clone = conn.clone();
    let heartbeat_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            if let Err(e) = conn_clone.send("PING\n", false).await {
                eprintln!("Heartbeat failed: {}", e);
                break;
            }
        }
    });
    
    // Read events (doesn't block heartbeat sends!)
    while let Some(event) = conn.next().await? {
        println!("Received: {:?}", event);
        
        // Echo back
        if let termionix_terminal::TerminalEvent::Data(data) = event {
            conn.send(format!("Echo: {}\n", data), false).await?;
        }
    }
    
    // Cleanup
    heartbeat_handle.abort();
    conn.close().await?;
    
    Ok(())
}


