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

//! ANSI Color and Style Demonstration
//!
//! This example showcases the ANSI color and styling capabilities of Termionix.
//! It creates a telnet server that displays various text styles, colors, and
//! formatting options.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example ansi_demo
//! ```
//!
//! Connect with a telnet client that supports ANSI colors:
//! ```bash
//! telnet localhost 2323
//! ```

use std::sync::Arc;
use termionix_ansicodec::{Color, Intensity, SegmentedString, Style, Underline};
use termionix_server::{
    CallbackHandler, ConnectionId, ServerConfig, ServerHandler, TelnetConnection, TelnetServer,
};
use termionix_terminal::TerminalEvent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("Starting ANSI Demo Server on 127.0.0.1:2323");
    println!("Connect with: telnet localhost 2323\n");

    let config = ServerConfig::new("127.0.0.1:2323".parse()?);
    let server = TelnetServer::new(config).await?;
    let handler = Arc::new(AnsiDemoHandler::new());

    server.start(handler).await?;
    tokio::signal::ctrl_c().await?;
    server.shutdown().await?;

    Ok(())
}

struct AnsiDemoHandler {
    demo_content: SegmentedString,
}

impl AnsiDemoHandler {
    fn new() -> Self {
        let mut content = SegmentedString::new();

        // Title
        content.push_style(Style {
            foreground: Some(Color::BrightCyan),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });
        content.push_str("╔════════════════════════════════════════════════╗\r\n");
        content.push_str("║     TERMIONIX ANSI COLOR DEMONSTRATION         ║\r\n");
        content.push_str("╚════════════════════════════════════════════════╝\r\n\r\n");

        // Basic Colors
        content.push_style(Style {
            foreground: Some(Color::Yellow),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });
        content.push_str("═══ Basic Colors ═══\r\n");
        content.push_style(Style::default());

        let basic_colors = [
            (Color::Black, "Black"),
            (Color::Red, "Red"),
            (Color::Green, "Green"),
            (Color::Yellow, "Yellow"),
            (Color::Blue, "Blue"),
            (Color::Magenta, "Magenta"),
            (Color::Cyan, "Cyan"),
            (Color::White, "White"),
        ];

        for (color, name) in &basic_colors {
            content.push_style(Style {
                foreground: Some(*color),
                ..Default::default()
            });
            content.push_str(&format!("  ● {:<10}", name));
            content.push_style(Style::default());
        }
        content.push_str("\r\n\r\n");

        // Bright Colors
        content.push_style(Style {
            foreground: Some(Color::Yellow),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });
        content.push_str("═══ Bright Colors ═══\r\n");
        content.push_style(Style::default());

        let bright_colors = [
            (Color::BrightBlack, "Bright Black"),
            (Color::BrightRed, "Bright Red"),
            (Color::BrightGreen, "Bright Green"),
            (Color::BrightYellow, "Bright Yellow"),
            (Color::BrightBlue, "Bright Blue"),
            (Color::BrightMagenta, "Bright Magenta"),
            (Color::BrightCyan, "Bright Cyan"),
            (Color::BrightWhite, "Bright White"),
        ];

        for (color, name) in &bright_colors {
            content.push_style(Style {
                foreground: Some(*color),
                ..Default::default()
            });
            content.push_str(&format!("  ● {:<15}", name));
            content.push_style(Style::default());
        }
        content.push_str("\r\n\r\n");

        // Text Styles
        content.push_style(Style {
            foreground: Some(Color::Yellow),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });
        content.push_str("═══ Text Styles ═══\r\n");
        content.push_style(Style::default());

        // Bold
        content.push_style(Style {
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });
        content.push_str("  Bold Text\r\n");

        // Dim
        content.push_style(Style {
            intensity: Some(Intensity::Faint),
            ..Default::default()
        });
        content.push_str("  Dim Text\r\n");

        // Italic
        content.push_style(Style {
            italic: Some(true),
            ..Default::default()
        });
        content.push_str("  Italic Text\r\n");

        // Underline
        content.push_style(Style {
            underline: Some(Underline::Single),
            ..Default::default()
        });
        content.push_str("  Underlined Text\r\n");

        // Double Underline
        content.push_style(Style {
            underline: Some(Underline::Double),
            ..Default::default()
        });
        content.push_str("  Double Underlined Text\r\n");

        // Strike-through
        content.push_style(Style {
            strike: Some(true),
            ..Default::default()
        });
        content.push_str("  Strike-through Text\r\n");

        content.push_style(Style::default());
        content.push_str("\r\n");

        // Combined Styles
        content.push_style(Style {
            foreground: Some(Color::Yellow),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });
        content.push_str("═══ Combined Styles ═══\r\n");
        content.push_style(Style::default());

        content.push_style(Style {
            foreground: Some(Color::BrightGreen),
            intensity: Some(Intensity::Bold),
            underline: Some(Underline::Single),
            ..Default::default()
        });
        content.push_str("  Bold + Underline + Green\r\n");

        content.push_style(Style {
            foreground: Some(Color::BrightRed),
            background: Some(Color::Yellow),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });
        content.push_str("  Bold Red on Yellow Background\r\n");

        content.push_style(Style {
            foreground: Some(Color::White),
            background: Some(Color::Blue),
            italic: Some(true),
            ..Default::default()
        });
        content.push_str("  Italic White on Blue\r\n");

        content.push_style(Style::default());
        content.push_str("\r\n");

        // RGB Colors (True Color)
        content.push_style(Style {
            foreground: Some(Color::Yellow),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        });
        content.push_str("═══ True Color (RGB) ═══\r\n");
        content.push_style(Style::default());

        // Gradient effect
        let gradient_colors = [
            (255, 0, 0),     // Red
            (255, 127, 0),   // Orange
            (255, 255, 0),   // Yellow
            (0, 255, 0),     // Green
            (0, 0, 255),     // Blue
            (75, 0, 130),    // Indigo
            (148, 0, 211),   // Violet
        ];

        content.push_str("  ");
        for (r, g, b) in &gradient_colors {
            content.push_style(Style {
                foreground: Some(Color::Rgb(*r, *g, *b)),
                ..Default::default()
            });
            content.push_str("████ ");
        }
        content.push_style(Style::default());
        content.push_str("\r\n\r\n");

        // Footer
        content.push_style(Style {
            foreground: Some(Color::BrightBlack),
            ..Default::default()
        });
        content.push_str("─────────────────────────────────────────────────\r\n");
        content.push_str("Type 'quit' to logout.txt\r\n");
        content.push_style(Style::default());
        content.push_str("> ");

        Self {
            demo_content: content,
        }
    }
}

#[async_trait::async_trait]
impl ServerHandler for AnsiDemoHandler {
    async fn on_connect(&self, id: ConnectionId, conn: &TelnetConnection) {
        tracing::info!("Client {} connected", id);
        let _ = conn.send(self.demo_content.clone()).await;
    }

    async fn on_event(&self, id: ConnectionId, conn: &TelnetConnection, event: TerminalEvent) {
        if let TerminalEvent::LineCompleted { line, .. } = event {
            let text = line.stripped();
            if text.trim().eq_ignore_ascii_case("quit") {
                let mut goodbye = SegmentedString::new();
                goodbye.push_style(Style {
                    foreground: Some(Color::BrightGreen),
                    ..Default::default()
                });
                goodbye.push_str("\r\nGoodbye! Thanks for viewing the demo.\r\n");
                goodbye.push_style(Style::default());
                let _ = conn.send(goodbye).await;
            } else {
                let _ = conn.send("> ").await;
            }
        }
    }

    async fn on_disconnect(&self, id: ConnectionId, _conn: &TelnetConnection) {
        tracing::info!("Client {} disconnected", id);
    }
}


