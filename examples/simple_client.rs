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

//! Simple Telnet Client Example using the high-level client library
//!
//! This example demonstrates the easiest way to create a Telnet client
//! using the termionix-client library.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example simple_client -- localhost 4000
//! ```

use async_trait::async_trait;
use std::io::{self, Write};
use std::sync::Arc;
use termionix_client::{ClientConfig, ClientConnection, ClientHandler, TelnetClient};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

/// Simple handler that prints server output and sends user input
struct SimpleHandler {
    input_tx: mpsc::UnboundedSender<String>,
}

#[async_trait]
impl ClientHandler for SimpleHandler {
    async fn on_connect(&self, _conn: &ClientConnection) {
        println!("\n=== Connected to server ===");
        println!("Type your commands and press Enter.");
        println!("Press Ctrl+C to logout.txt.\n");
    }
    
    async fn on_data(&self, _conn: &ClientConnection, data: &[u8]) {
        // Print server data directly to stdout
        print!("{}", String::from_utf8_lossy(data));
        io::stdout().flush().ok();
    }
    
    async fn on_disconnect(&self, _conn: &ClientConnection) {
        println!("\n=== Disconnected from server ===");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();
    
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let host = args.get(1).map(|s| s.as_str()).unwrap_or("localhost");
    let port: u16 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(4000);
    
    println!("Simple Telnet Client");
    println!("====================");
    println!("Connecting to: {}:{}", host, port);
    println!();
    
    // Create configuration
    let config = ClientConfig::new(host, port)
        .with_terminal_type("xterm-256color")
        .with_terminal_size(80, 24);
    
    // Create client
    let mut client = TelnetClient::new(config);
    
    // Create channel for user input
    let (input_tx, mut input_rx) = mpsc::unbounded_channel::<String>();
    
    // Spawn task to read user input
    let input_tx_clone = input_tx.clone();
    tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();
        
        while let Ok(Some(line)) = lines.next_line().await {
            if input_tx_clone.send(line).is_err() {
                break;
            }
        }
    });
    
    // Create handler
    let handler = Arc::new(SimpleHandler { input_tx });
    
    // Spawn connection task
    let client_handle = {
        let handler = handler.clone();
        tokio::spawn(async move {
            client.connect(handler).await
        })
    };
    
    // Handle user input in main task
    while let Some(line) = input_rx.recv().await {
        if let Some(conn) = client_handle.is_finished().then(|| ()).and(None) {
            break;
        }
        
        // Get connection from client (this is a simplified approach)
        // In a real implementation, you'd want better connection access
        // For now, we'll just print that we can't send yet
        println!("Note: Sending not yet implemented in this simple example");
        println!("You typed: {}", line);
    }
    
    // Wait for client to finish
    client_handle.await??;
    
    Ok(())
}


