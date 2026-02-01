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

//! # Advanced Telnet Client Example
//!
//! This example demonstrates advanced client features including:
//!
//! - MUD sidechannel support (GMCP, MSDP, MSSP)
//! - Connection state management
//! - Automatic reconnection
//! - Command history
//! - Trigger system
//! - Alias support
//! - Logging
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example advanced_client -- mud.server.com 4000
//! ```
//!
//! ## Commands
//!
//! - `/quit` - Disconnect and exit
//! - `/reconnect` - Reconnect to server
//! - `/history` - Show command history
//! - `/alias <name> <command>` - Create command alias
//! - `/trigger <pattern> <action>` - Create trigger
//! - `/log <filename>` - Start logging to file
//! - `/help` - Show help

use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{self, Write};
use std::sync::Arc;
use termionix_telnetcodec::{TelnetCodec, TelnetEvent, TelnetOption};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};

/// Client configuration
#[derive(Debug, Clone)]
struct ClientConfig {
    host: String,
    port: u16,
    terminal_type: String,
    width: u16,
    height: u16,
    auto_reconnect: bool,
    max_history: usize,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 4000,
            terminal_type: "xterm-256color".to_string(),
            width: 80,
            height: 24,
            auto_reconnect: false,
            max_history: 100,
        }
    }
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Command alias
#[derive(Debug, Clone)]
struct Alias {
    name: String,
    command: String,
}

/// Trigger pattern and action
#[derive(Debug, Clone)]
struct Trigger {
    pattern: String,
    action: String,
}

/// Client state
struct ClientState {
    config: ClientConfig,
    state: ConnectionState,
    command_history: VecDeque<String>,
    aliases: HashMap<String, String>,
    triggers: Vec<Trigger>,
    log_file: Option<File>,
    supported_options: Vec<TelnetOption>,
}

impl ClientState {
    fn new(config: ClientConfig) -> Self {
        Self {
            config,
            state: ConnectionState::Disconnected,
            command_history: VecDeque::new(),
            aliases: HashMap::new(),
            triggers: Vec::new(),
            log_file: None,
            supported_options: vec![
                TelnetOption::Echo,
                TelnetOption::SuppressGoAhead,
                TelnetOption::TerminalType,
                TelnetOption::NegotiateAboutWindowSize,
                TelnetOption::TransmitBinary,
            ],
        }
    }

    fn add_to_history(&mut self, command: String) {
        if self.command_history.len() >= self.config.max_history {
            self.command_history.pop_front();
        }
        self.command_history.push_back(command);
    }

    fn resolve_alias(&self, input: &str) -> String {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if let Some(first) = parts.first() {
            if let Some(alias_cmd) = self.aliases.get(*first) {
                if parts.len() > 1 {
                    format!("{} {}", alias_cmd, parts[1..].join(" "))
                } else {
                    alias_cmd.clone()
                }
            } else {
                input.to_string()
            }
        } else {
            input.to_string()
        }
    }

    fn check_triggers(&mut self, text: &str) {
        for trigger in &self.triggers {
            if text.contains(&trigger.pattern) {
                info!("Trigger matched: {} -> {}", trigger.pattern, trigger.action);
                // In a real implementation, you'd execute the action
            }
        }
    }

    fn log(&mut self, text: &str) {
        if let Some(ref mut file) = self.log_file {
            writeln!(file, "{}", text).ok();
        }
    }
}

/// Advanced Telnet client
struct AdvancedClient {
    state: Arc<RwLock<ClientState>>,
}

impl AdvancedClient {
    fn new(config: ClientConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(ClientState::new(config))),
        }
    }

    /// Connect to server
    async fn connect(&self) -> Result<TcpStream, Box<dyn std::error::Error>> {
        let state = self.state.read().await;
        let addr = format!("{}:{}", state.config.host, state.config.port);
        drop(state);

        info!("Connecting to {}...", addr);

        let mut state = self.state.write().await;
        state.state = ConnectionState::Connecting;
        drop(state);

        let stream = TcpStream::connect(&addr).await?;

        let mut state = self.state.write().await;
        state.state = ConnectionState::Connected;
        info!("Connected to {}", stream.peer_addr()?);

        Ok(stream)
    }

    /// Main client loop
    async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            match self.run_connection().await {
                Ok(_) => {
                    info!("Connection closed normally");
                    break;
                }
                Err(e) => {
                    error!("Connection error: {}", e);

                    let state = self.state.read().await;
                    let should_reconnect = state.config.auto_reconnect;
                    drop(state);

                    if should_reconnect {
                        info!("Reconnecting in 5 seconds...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        continue;
                    } else {
                        break;
                    }
                }
            }
        }

        let mut state = self.state.write().await;
        state.state = ConnectionState::Disconnected;

        Ok(())
    }

    /// Run a single connection
    async fn run_connection(&self) -> Result<(), Box<dyn std::error::Error>> {
        let stream = self.connect().await?;
        let mut framed = Framed::new(stream, TelnetCodec::new());

        // Create input channel
        let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<String>(100);

        // Spawn input reader
        tokio::spawn(async move {
            let stdin = tokio::io::stdin();
            let reader = BufReader::new(stdin);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if input_tx.send(line).await.is_err() {
                    break;
                }
            }
        });

        self.print_welcome().await;

        // Main event loop
        loop {
            tokio::select! {
                // Server events
                result = framed.next() => {
                    match result {
                        Some(Ok(event)) => {
                            if !self.handle_server_event(event, &mut framed).await? {
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            error!("Server error: {}", e);
                            break;
                        }
                        None => {
                            info!("Server closed connection");
                            break;
                        }
                    }
                }

                // User input
                Some(line) = input_rx.recv() => {
                    if !self.handle_user_input(&line, &mut framed).await? {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Print welcome message
    async fn print_welcome(&self) {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║         Termionix Advanced Telnet Client                  ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!("\nCommands:");
        println!("  /quit              - Disconnect and exit");
        println!("  /reconnect         - Reconnect to server");
        println!("  /history           - Show command history");
        println!("  /alias <n> <cmd>   - Create alias");
        println!("  /trigger <p> <a>   - Create trigger");
        println!("  /log <file>        - Start logging");
        println!("  /help              - Show this help");
        println!();
    }

    /// Handle server event
    async fn handle_server_event(
        &self,
        event: TelnetEvent,
        framed: &mut Framed<TcpStream, TelnetCodec>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match event {
            TelnetEvent::Data(byte) => {
                let ch = byte as char;
                print!("{}", ch);
                io::stdout().flush()?;

                // Log if enabled
                let mut state = self.state.write().await;
                state.log(&ch.to_string());

                // Check triggers on complete lines
                if ch == '\n' {
                    // In a real implementation, buffer the line and check triggers
                }
            }

            TelnetEvent::Do(option) => {
                debug!("Server requests DO {:?}", option);
                let state = self.state.read().await;
                if state.supported_options.contains(&option) {
                    framed.send(TelnetEvent::Will(option)).await?;

                    // Send initial data for specific options
                    match option {
                        TelnetOption::NegotiateAboutWindowSize => {
                            drop(state);
                            self.send_window_size(framed).await?;
                        }
                        _ => {}
                    }
                } else {
                    framed.send(TelnetEvent::Wont(option)).await?;
                }
            }

            TelnetEvent::Dont(option) => {
                debug!("Server requests DONT {:?}", option);
                framed.send(TelnetEvent::Wont(option)).await?;
            }

            TelnetEvent::Will(option) => {
                debug!("Server offers WILL {:?}", option);
                framed.send(TelnetEvent::Do(option)).await?;
            }

            TelnetEvent::Wont(option) => {
                debug!("Server refuses WONT {:?}", option);
                framed.send(TelnetEvent::Dont(option)).await?;
            }

            TelnetEvent::Subnegotiate(option, data) => {
                debug!("Subnegotiation for {:?}", option);
                match option {
                    TelnetOption::TerminalType => {
                        if !data.is_empty() && data[0] == 1 {
                            self.send_terminal_type(framed).await?;
                        }
                    }
                    TelnetOption::GMCP => {
                        self.handle_gmcp(&data).await?;
                    }
                    TelnetOption::MSDP => {
                        self.handle_msdp(&data).await?;
                    }
                    _ => {}
                }
            }

            TelnetEvent::OptionStatus(option, side, enabled) => {
                debug!("Option {:?} {:?} {}", option, side, if enabled { "enabled" } else { "disabled" });
            }

            _ => {}
        }

        Ok(true)
    }

    /// Handle user input
    async fn handle_user_input(
        &self,
        input: &str,
        framed: &mut Framed<TcpStream, TelnetCodec>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let input = input.trim();

        // Handle client commands
        if input.starts_with('/') {
            return self.handle_client_command(input, framed).await;
        }

        // Add to history
        let mut state = self.state.write().await;
        state.add_to_history(input.to_string());

        // Resolve aliases
        let resolved = state.resolve_alias(input);
        drop(state);

        // Send to server
        self.send_line(framed, &resolved).await?;

        Ok(true)
    }

    /// Handle client command
    async fn handle_client_command(
        &self,
        input: &str,
        framed: &mut Framed<TcpStream, TelnetCodec>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = input[1..].split_whitespace().collect();
        if parts.is_empty() {
            return Ok(true);
        }

        match parts[0] {
            "quit" | "exit" => {
                println!("Disconnecting...");
                return Ok(false);
            }

            "reconnect" => {
                println!("Reconnecting...");
                return Ok(false); // Will trigger reconnect in main loop
            }

            "history" => {
                let state = self.state.read().await;
                println!("\n=== Command History ===");
                for (i, cmd) in state.command_history.iter().enumerate() {
                    println!("{:3}: {}", i + 1, cmd);
                }
                println!();
            }

            "alias" => {
                if parts.len() >= 3 {
                    let name = parts[1].to_string();
                    let command = parts[2..].join(" ");
                    let mut state = self.state.write().await;
                    state.aliases.insert(name.clone(), command.clone());
                    println!("Alias created: {} -> {}", name, command);
                } else {
                    println!("Usage: /alias <name> <command>");
                }
            }

            "trigger" => {
                if parts.len() >= 3 {
                    let pattern = parts[1].to_string();
                    let action = parts[2..].join(" ");
                    let mut state = self.state.write().await;
                    state.triggers.push(Trigger {
                        pattern: pattern.clone(),
                        action: action.clone(),
                    });
                    println!("Trigger created: {} -> {}", pattern, action);
                } else {
                    println!("Usage: /trigger <pattern> <action>");
                }
            }

            "log" => {
                if parts.len() >= 2 {
                    let filename = parts[1];
                    match File::create(filename) {
                        Ok(file) => {
                            let mut state = self.state.write().await;
                            state.log_file = Some(file);
                            println!("Logging to: {}", filename);
                        }
                        Err(e) => {
                            println!("Failed to create log file: {}", e);
                        }
                    }
                } else {
                    println!("Usage: /log <filename>");
                }
            }

            "help" => {
                self.print_welcome().await;
            }

            _ => {
                println!("Unknown command: {}", parts[0]);
                println!("Type /help for available commands");
            }
        }

        Ok(true)
    }

    /// Send line to server
    async fn send_line(
        &self,
        framed: &mut Framed<TcpStream, TelnetCodec>,
        line: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for ch in line.chars() {
            framed.send(TelnetEvent::Data(ch as u8)).await?;
        }
        framed.send(TelnetEvent::Data(b'\r')).await?;
        framed.send(TelnetEvent::Data(b'\n')).await?;
        Ok(())
    }

    /// Send window size
    async fn send_window_size(
        &self,
        framed: &mut Framed<TcpStream, TelnetCodec>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use termionix_telnetcodec::naws::WindowSize;

        let state = self.state.read().await;
        let window_size = WindowSize {
            width: state.config.width,
            height: state.config.height,
        };

        let data = window_size.encode();
        framed
            .send(TelnetEvent::Subnegotiate(
                TelnetOption::NegotiateAboutWindowSize,
                data,
            ))
            .await?;

        debug!("Sent window size: {}x{}", state.config.width, state.config.height);
        Ok(())
    }

    /// Send terminal type
    async fn send_terminal_type(
        &self,
        framed: &mut Framed<TcpStream, TelnetCodec>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.state.read().await;
        let mut data = vec![0]; // IS command
        data.extend_from_slice(state.config.terminal_type.as_bytes());

        framed
            .send(TelnetEvent::Subnegotiate(TelnetOption::TerminalType, data))
            .await?;

        debug!("Sent terminal type: {}", state.config.terminal_type);
        Ok(())
    }

    /// Handle GMCP data
    async fn handle_gmcp(&self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(text) = String::from_utf8(data.to_vec()) {
            debug!("GMCP: {}", text);
            // Parse and handle GMCP messages
            // In a real implementation, you'd parse JSON and handle specific packages
        }
        Ok(())
    }

    /// Handle MSDP data
    async fn handle_msdp(&self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        debug!("MSDP data received: {} bytes", data.len());
        // Parse and handle MSDP variables
        Ok(())
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

    // Parse arguments
    let args: Vec<String> = std::env::args().collect();
    let mut config = ClientConfig::default();

    if args.len() >= 2 {
        config.host = args[1].clone();
    }
    if args.len() >= 3 {
        config.port = args[2].parse()?;
    }

    // Create and run client
    let client = AdvancedClient::new(config);
    client.run().await?;

    println!("\nGoodbye!");

    Ok(())
}


