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

//! Terminal Codec Direct Usage Example
//!
//! Demonstrates using the TerminalCodec directly with a TCP stream.
//! This shows the low-level codec API without the service layer.
//!
//! For most applications, using the service layer (see echo_server.rs) is recommended.
//! This example is useful for understanding the codec internals or building custom solutions.

use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use termionix_ansicodec::{AnsiCodec, AnsiConfig};
use termionix_telnetcodec::TelnetCodec;
use termionix_terminal::{TerminalCodec, TerminalEvent};
use tokio::net::TcpListener;
use tokio_util::codec::Framed;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:2323").await?;
    println!("Terminal Codec Direct Example listening on 127.0.0.1:2323");
    println!("Connect with: telnet localhost 2323\n");

    let connection_count = Arc::new(AtomicUsize::new(0));

    loop {
        let (socket, addr) = listener.accept().await?;
        let conn_id = connection_count.fetch_add(1, Ordering::SeqCst) + 1;
        println!("[Connection {}] New connection from: {}", conn_id, addr);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, conn_id).await {
                eprintln!("[Connection {}] Error: {}", conn_id, e);
            }
            println!("[Connection {}] Connection closed", conn_id);
        });
    }
}

async fn handle_connection(
    socket: tokio::net::TcpStream,
    conn_id: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create the codec stack: TelnetCodec -> AnsiCodec -> TerminalCodec
    let telnet_codec = TelnetCodec::new();
    let ansi_codec = AnsiCodec::new(AnsiConfig::default(), telnet_codec);
    let terminal_codec = TerminalCodec::new(ansi_codec);

    // Wrap the socket with the codec
    let mut framed = Framed::new(socket, terminal_codec);

    println!("[Connection {}] Codec stack initialized", conn_id);

    // Send welcome message
    framed.send("\r\n").await?;
    framed
        .send("╔══════════════════════════════════════╗\r\n")
        .await?;
    framed
        .send("║   Terminal Codec Direct Demo        ║\r\n")
        .await?;
    framed
        .send("╚══════════════════════════════════════╝\r\n")
        .await?;
    framed.send("\r\n").await?;
    framed
        .send("This example uses the TerminalCodec directly.\r\n")
        .await?;
    framed
        .send("Type anything and it will be echoed back.\r\n")
        .await?;
    framed.send("Type 'quit' to logout.txt.\r\n").await?;
    framed.send("\r\n> ").await?;

    // Process events
    let mut char_count = 0;
    let mut line_count = 0;

    while let Some(result) = framed.next().await {
        match result {
            Ok(event) => {
                match event {
                    TerminalEvent::CharacterData { character, cursor } => {
                        char_count += 1;
                        println!(
                            "[Connection {}] Character: '{}' at {:?}",
                            conn_id, character, cursor
                        );

                        // Echo character back
                        framed.send(character).await?;
                    }
                    TerminalEvent::LineCompleted { line, cursor } => {
                        line_count += 1;
                        let text = line.to_string();
                        println!(
                            "[Connection {}] Line completed: '{}' at {:?}",
                            conn_id, text, cursor
                        );

                        // Check for quit command
                        if text.trim().eq_ignore_ascii_case("quit") {
                            framed.send("\r\n\r\nGoodbye!\r\n").await?;
                            let stats_msg = format!(
                                "Stats: {} characters, {} lines\r\n",
                                char_count, line_count
                            );
                            framed.send(stats_msg.as_str()).await?;
                            break;
                        }

                        // Echo line back with stats
                        let typed_msg = format!("\r\nYou typed: {}\r\n", text);
                        framed.send(typed_msg.as_str()).await?;
                        let stats_prompt =
                            format!("(chars: {}, lines: {})\r\n> ", char_count, line_count);
                        framed.send(stats_prompt.as_str()).await?;
                    }
                    TerminalEvent::ResizeWindow { old, new } => {
                        println!(
                            "[Connection {}] Window resized from {}x{} to {}x{}",
                            conn_id, old.cols, old.rows, new.cols, new.rows
                        );
                        let resize_msg =
                            format!("\r\n[Window resized to {}x{}]\r\n> ", new.cols, new.rows);
                        framed.send(resize_msg.as_str()).await?;
                    }
                    TerminalEvent::EraseCharacter { cursor } => {
                        println!("[Connection {}] Erase character at {:?}", conn_id, cursor);
                    }
                    TerminalEvent::EraseLine { cursor } => {
                        println!("[Connection {}] Erase line at {:?}", conn_id, cursor);
                    }
                    TerminalEvent::Bell => {
                        println!("[Connection {}] Bell!", conn_id);
                    }
                    TerminalEvent::Break => {
                        println!("[Connection {}] Break signal", conn_id);
                        framed.send("\r\n[Break]\r\n> ").await?;
                    }
                    TerminalEvent::InterruptProcess => {
                        println!("[Connection {}] Interrupt process signal", conn_id);
                        framed.send("\r\n[Interrupted]\r\n> ").await?;
                    }
                    _ => {
                        println!("[Connection {}] Other event: {:?}", conn_id, event);
                    }
                }
            }
            Err(e) => {
                eprintln!("[Connection {}] Codec error: {}", conn_id, e);
                break;
            }
        }
    }

    println!("[Connection {}] Event loop ended", conn_id);
    Ok(())
}
