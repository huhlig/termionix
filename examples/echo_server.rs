//
// Copyright 2017-2025 Hans W. Uhlig. All Rights Reserved.
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

//! Simple Echo Server Example
//!
//! This example demonstrates a basic telnet echo server that:
//! - Accepts connections on port 2323
//! - Echoes back any text received from clients
//! - Handles multiple concurrent connections
//! - Demonstrates basic ANSI color usage
//!
//! ## Usage
//!
//! Run the server:
//! ```bash
//! cargo run --example echo_server
//! ```
//!
//! Connect with a telnet client:
//! ```bash
//! telnet localhost 2323
//! ```

use std::sync::Arc;
use termionix_ansicodec::SegmentedString;
use termionix_ansicodec::ansi::{AnsiSelectGraphicRendition, Color, Intensity};
use termionix_service::{ServerConfig, ServerHandler, TelnetConnection, TelnetError, TelnetServer};
use termionix_terminal::TerminalEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("Starting Termionix Echo Server on 127.0.0.1:2323");
    println!("Connect with: telnet localhost 2323");
    println!("Press Ctrl+C to stop the server\n");

    // Configure the server
    let config = ServerConfig::new("127.0.0.1:2323".parse()?)
        .with_max_connections(100)
        .with_idle_timeout(std::time::Duration::from_secs(300));

    // Create the server
    let server = TelnetServer::new(config).await?;

    // Create a custom handler
    let handler = Arc::new(EchoHandler::new());

    // Start the server
    server.start(handler).await?;

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down server...");

    // Graceful shutdown
    server.shutdown().await?;
    println!("Server stopped");

    Ok(())
}

/// Custom handler for the echo server
struct EchoHandler {
    welcome_message: String,
}

impl EchoHandler {
    fn new() -> Self {
        let mut welcome = SegmentedString::empty();

        // Create a colorful welcome message
        welcome.push_style(AnsiSelectGraphicRendition {
            foreground: Some(Color::BrightCyan),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });
        welcome.push_str("╔══════════════════════════════════════╗\r\n");
        welcome.push_str("║   Welcome to Termionix Echo Server   ║\r\n");
        welcome.push_str("╚══════════════════════════════════════╝\r\n");

        welcome.push_style(AnsiSelectGraphicRendition::default());
        welcome.push_str("\r\n");

        welcome.push_style(AnsiSelectGraphicRendition {
            foreground: Some(Color::Yellow),
            ..Default::default()
        });
        welcome.push_str("Type anything and it will be echoed back.\r\n");
        welcome.push_str("Type 'quit' or 'exit' to disconnect.\r\n");

        welcome.push_style(AnsiSelectGraphicRendition::default());
        welcome.push_str("\r\n> ");

        Self {
            welcome_message: welcome.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl ServerHandler for EchoHandler {
    async fn on_connect(&self, id: termionix_service::ConnectionId, conn: &TelnetConnection) {
        tracing::info!("Client {} connected", id);

        // Send welcome message
        if let Err(e) = conn.send(self.welcome_message.as_str()).await {
            tracing::error!("Failed to send welcome message to {}: {}", id, e);
        }
    }

    async fn on_event(
        &self,
        id: termionix_service::ConnectionId,
        conn: &TelnetConnection,
        event: TerminalEvent,
    ) {
        match event {
            TerminalEvent::CharacterData { character, .. } => {
                // Echo the character back
                if let Err(e) = conn.send_char(character).await {
                    tracing::error!("Failed to echo character to {}: {}", id, e);
                }
            }
            TerminalEvent::LineCompleted { line, .. } => {
                let text = line.to_string();
                tracing::debug!("Client {} sent line: {}", id, text);

                // Check for quit commands
                if text.trim().eq_ignore_ascii_case("quit")
                    || text.trim().eq_ignore_ascii_case("exit")
                {
                    let mut goodbye = SegmentedString::empty();
                    goodbye.push_style(AnsiSelectGraphicRendition {
                        foreground: Some(Color::BrightGreen),
                        ..Default::default()
                    });
                    goodbye.push_str("\r\nGoodbye!\r\n");
                    goodbye.push_style(AnsiSelectGraphicRendition::default());

                    let goodbye_str = goodbye.to_string();
                    let _ = conn.send(goodbye_str.as_str()).await;
                    // Connection will be closed by the client
                    return;
                }

                // Echo the line back with color
                let mut response = SegmentedString::empty();
                response.push_str("\r\n");

                response.push_style(AnsiSelectGraphicRendition {
                    foreground: Some(Color::BrightGreen),
                    ..Default::default()
                });
                response.push_str("Echo: ");

                response.push_style(AnsiSelectGraphicRendition {
                    foreground: Some(Color::White),
                    ..Default::default()
                });
                response.push_str(&text);

                response.push_style(AnsiSelectGraphicRendition::default());
                response.push_str("\r\n> ");

                let response_str = response.to_string();
                if let Err(e) = conn.send(response_str.as_str()).await {
                    tracing::error!("Failed to send echo to {}: {}", id, e);
                }
            }
            _ => {
                tracing::debug!("Client {} sent event: {:?}", id, event);
            }
        }
    }

    async fn on_error(
        &self,
        id: termionix_service::ConnectionId,
        _conn: &TelnetConnection,
        error: TelnetError,
    ) {
        tracing::error!("Error for client {}: {}", id, error);
    }

    async fn on_timeout(&self, id: termionix_service::ConnectionId, _conn: &TelnetConnection) {
        tracing::warn!("Client {} timed out", id);
    }

    async fn on_idle_timeout(&self, id: termionix_service::ConnectionId, _conn: &TelnetConnection) {
        tracing::info!("Client {} idle timeout", id);
    }

    async fn on_disconnect(&self, id: termionix_service::ConnectionId, _conn: &TelnetConnection) {
        tracing::info!("Client {} disconnected", id);
    }
}

// Made with Bob
