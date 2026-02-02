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

//! # Telnet Client Example
//!
//! This example demonstrates how to use the Termionix client library to connect
//! to remote Telnet servers. It shows:
//!
//! - Connecting to a Telnet server
//! - Handling telnet sidechannel negotiation
//! - Processing server data
//! - Sending user input
//! - Managing connection state
//!
//! ## Usage
//!
//! Connect to a local server:
//! ```bash
//! cargo run --example telnet_client -- localhost 4000
//! ```
//!
//! Connect to a MUD server:
//! ```bash
//! cargo run --example telnet_client -- mud.example.com 4000
//! ```

use async_trait::async_trait;
use std::io::{self, Write};
use std::sync::Arc;
use termionix_client::{ClientConfig, ClientError, TerminalClient, TerminalConnection, TerminalHandler};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tracing::{debug, info};

/// Telnet client handler
struct TelnetHandler {
    /// Channel to receive user input
    input_rx: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<String>>>,
}

#[async_trait]
impl TerminalHandler for TelnetHandler {
    async fn on_connect(&self, conn: &TerminalConnection) {
        info!("Connected to server!");
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║         Connected to Telnet Server                        ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!("\nType your commands and press Enter.");
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
    
    async fn on_line(&self, _conn: &TerminalConnection, line: &str) {
        debug!("Received line: {}", line);
    }

    async fn on_bell(&self, _conn: &TerminalConnection) {
        print!("\x07"); // Terminal bell
        io::stdout().flush().ok();
    }
    
    async fn on_option_changed(
        &self,
        _conn: &TerminalConnection,
        option: termionix_client::TelnetOption,
        enabled: bool,
        local: bool,
    ) {
        debug!(
            "Option {:?} {} ({})",
            option,
            if enabled { "enabled" } else { "disabled" },
            if local { "local" } else { "remote" }
        );
    }
    
    async fn on_error(&self, _conn: &TerminalConnection, error: ClientError) {
        eprintln!("\n[Error: {}]", error);
    }
    
    async fn on_disconnect(&self, _conn: &TerminalConnection) {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║         Disconnected from Server                          ║");
        println!("╚════════════════════════════════════════════════════════════╝\n");
    }
    
    async fn on_reconnect_attempt(&self, _conn: &TerminalConnection, attempt: u32) -> bool {
        println!("\n[Reconnection attempt {}...]", attempt);
        true
    }
    
    async fn on_reconnect_failed(&self, _conn: &TerminalConnection) {
        eprintln!("\n[Reconnection failed - giving up]");
    }
}

/// Parse command line arguments
fn parse_args() -> Result<ClientConfig, Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    let host = args.get(1).map(|s| s.as_str()).unwrap_or("localhost");
    let port: u16 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(4000);
    
    let config = ClientConfig::new(host, port)
        .with_terminal_type("xterm-256color")
        .with_terminal_size(80, 24)
        .with_auto_reconnect(false); // Disable auto-reconnect for this example
    
    Ok(config)
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
    let config = parse_args()?;
    
    println!("Termionix Telnet Client");
    println!("=======================");
    println!("Connecting to: {}", config.address());
    println!("Terminal type: {}", config.terminal_type);
    println!("Terminal size: {}x{}", config.terminal_width, config.terminal_height);
    println!();
    
    // Create channel for user input
    let (input_tx, input_rx) = mpsc::unbounded_channel::<String>();
    
    // Spawn task to read user input from stdin
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
    let handler = Arc::new(TelnetHandler {
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


