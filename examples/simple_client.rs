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
use termionix_client::{ClientConfig, TerminalClient, TerminalConnection, TerminalHandler};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

/// Simple handler that prints server output and sends user input
struct SimpleHandler {
    input_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<String>>>,
}

#[async_trait]
impl TerminalHandler for SimpleHandler {
    async fn on_connect(&self, conn: &TerminalConnection) {
        println!("\n=== Connected to server ===");
        println!("Type your commands and press Enter.");
        println!("Press Ctrl+C to disconnect.\n");

        // Start input handler
        let conn = conn.clone();
        let input_rx = self.input_rx.clone();
        tokio::spawn(async move {
            let mut rx = input_rx.lock().await;
            while let Some(line) = rx.recv().await {
                if let Err(e) = conn.send_line(&line).await {
                    eprintln!("Error sending: {}", e);
                    break;
                }
            }
        });
    }
    
    async fn on_character(&self, _conn: &TerminalConnection, ch: char) {
        // Print characters as they arrive
        print!("{}", ch);
        io::stdout().flush().ok();
    }
    
    async fn on_disconnect(&self, _conn: &TerminalConnection) {
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
    
    // Create channel for user input
    let (input_tx, input_rx) = mpsc::unbounded_channel::<String>();
    
    // Spawn task to read user input
    tokio::spawn(async move {
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();
        
        while let Ok(Some(line)) = lines.next_line().await {
            if input_tx.send(line).is_err() {
                break;
            }
        }
    });
    
    // Create handler
    let handler = Arc::new(SimpleHandler {
        input_rx: Arc::new(tokio::sync::Mutex::new(input_rx)),
    });
    
    // Create and run client
    let mut client = TerminalClient::new(config);
    
    // Handle Ctrl+C gracefully
    let result = tokio::select! {
        result = client.connect(handler) => result,
        _ = tokio::signal::ctrl_c() => {
            println!("\n\nReceived Ctrl+C, disconnecting...");
            if let Some(conn) = client.connection() {
                conn.disconnect().await.ok();
            }
            Ok(())
        }
    };
    
    match result {
        Ok(()) => {
            println!("Disconnected.");
            Ok(())
        }
        Err(e) => {
            eprintln!("Client error: {}", e);
            Err(e.into())
        }
    }
}


