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

//! Terminal Client Example
//!
//! Demonstrates using the high-level TerminalClient with automatic
//! ANSI parsing and line buffering.

use std::sync::Arc;
use termionix_client::{ClientConfig, TerminalClient, TerminalConnection, TerminalHandler};
use tracing::{info, Level};
use tracing_subscriber;

struct MyTerminalHandler;

#[async_trait::async_trait]
impl TerminalHandler for MyTerminalHandler {
    async fn on_connect(&self, conn: &TerminalConnection) {
        info!("Connected to server!");
        
        // Send a greeting
        if let Err(e) = conn.send_line("Hello from terminal client!").await {
            eprintln!("Failed to send greeting: {}", e);
        }
    }

    async fn on_disconnect(&self, _conn: &TerminalConnection) {
        info!("Disconnected from server");
    }

    async fn on_character(&self, _conn: &TerminalConnection, ch: char) {
        // Print each character as it arrives
        print!("{}", ch);
        use std::io::Write;
        std::io::stdout().flush().unwrap();
    }

    async fn on_line(&self, _conn: &TerminalConnection, line: &str) {
        // This is called when a complete line is received
        info!("Received complete line: {}", line);
    }

    async fn on_bell(&self, _conn: &TerminalConnection) {
        // Terminal bell received
        print!("\x07"); // ASCII bell
        use std::io::Write;
        std::io::stdout().flush().unwrap();
    }

    async fn on_resize(&self, _conn: &TerminalConnection, width: usize, height: usize) {
        info!("Terminal resized to {}x{}", width, height);
    }

    async fn on_error(&self, _conn: &TerminalConnection, error: termionix_client::ClientError) {
        eprintln!("Connection error: {}", error);
    }

    async fn on_reconnect_attempt(&self, _conn: &TerminalConnection, attempt: u32) -> bool {
        info!("Reconnection attempt #{}", attempt);
        true // Continue trying to reconnect
    }

    async fn on_reconnect_failed(&self, _conn: &TerminalConnection) {
        eprintln!("Reconnection failed after multiple attempts");
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Configure the client
    let config = ClientConfig::new("localhost", 4000)
        .with_auto_reconnect(true)
        .with_max_reconnect_attempts(Some(5))
        .with_terminal_type("xterm-256color")
        .with_terminal_size(120, 40);

    info!("Starting terminal client...");
    info!("Connecting to {}:{}", config.host(), config.port());

    // Create and connect the terminal client
    let mut client = TerminalClient::new(config);
    let handler = Arc::new(MyTerminalHandler);

    // This will block until the connection closes or fails
    match client.connect(handler).await {
        Ok(()) => {
            info!("Client terminated normally");
        }
        Err(e) => {
            eprintln!("Client error: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}


