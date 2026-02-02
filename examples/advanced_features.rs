/// Advanced Features Example
///
/// This example demonstrates the new features added to Termionix:
/// - Connection metadata storage
/// - New terminal events (WindowSize, TerminalType, Disconnected)
/// - Tracing integration
/// - Metrics integration
/// - Negotiation status API
/// - Broadcast helpers
///
/// Run with: cargo run --example advanced_features
/// Enable tracing: RUST_LOG=debug cargo run --example advanced_features

use std::net::SocketAddr;
use std::sync::Arc;
use termionix_server::{
    ConnectionManager, TelnetConnection, TelnetHandler, TelnetServer, TelnetServerConfig,
};
use termionix_terminal::{TerminalCommand, TerminalEvent};
use tokio::sync::RwLock;
use tracing::{info, warn};
use tracing_subscriber;

/// Player data stored in connection metadata
#[derive(Debug, Clone)]
struct PlayerData {
    name: String,
    room_id: u32,
    login_time: std::time::Instant,
}

/// Room data stored in connection metadata
#[derive(Debug, Clone)]
struct RoomData {
    id: u32,
    name: String,
}

/// Custom handler demonstrating advanced features
struct AdvancedHandler {
    manager: Arc<ConnectionManager>,
}

impl AdvancedHandler {
    fn new(manager: Arc<ConnectionManager>) -> Self {
        Self { manager }
    }

    /// Handle player login and store metadata
    async fn handle_login(&self, conn: &TelnetConnection, name: String) {
        info!("Player '{}' logging in", name);

        // Store player data in connection metadata
        let player = PlayerData {
            name: name.clone(),
            room_id: 1, // Start in room 1
            login_time: std::time::Instant::now(),
        };
        conn.set_data("player", player.clone());

        // Store initial room data
        let room = RoomData {
            id: 1,
            name: "Town Square".to_string(),
        };
        conn.set_data("room", room);

        // Send welcome message using ergonomic &String encoder
        conn.send(&format!("Welcome, {}!\r\n", name)).await.ok();
        conn.send("You are in the Town Square.\r\n").await.ok();

        // Announce to other players (broadcast except this connection)
        let announcement = format!("{} has entered the game.\r\n", name);
        self.manager
            .broadcast_except(&announcement, &[conn.id()])
            .await;
    }

    /// Handle player movement between rooms
    async fn handle_move(&self, conn: &TelnetConnection, direction: &str) {
        // Retrieve player data from metadata
        let player: Option<PlayerData> = conn.get_data("player");
        let current_room: Option<RoomData> = conn.get_data("room");

        if let (Some(player), Some(current_room)) = (player, current_room) {
            // Simple room navigation (in real app, use proper room graph)
            let new_room_id = match direction {
                "north" => current_room.id + 1,
                "south" => current_room.id.saturating_sub(1),
                _ => {
                    conn.send("You can't go that way.\r\n").await.ok();
                    return;
                }
            };

            // Update room data
            let new_room = RoomData {
                id: new_room_id,
                name: format!("Room {}", new_room_id),
            };

            // Announce departure to old room
            let departure = format!("{} leaves {}.\r\n", player.name, direction);
            self.broadcast_to_room(current_room.id, &departure, Some(conn.id()))
                .await;

            // Update metadata
            conn.set_data("room", new_room.clone());

            // Send to player
            conn.send(&format!("You go {}.\r\n", direction)).await.ok();
            conn.send(&format!("You are in {}.\r\n", new_room.name))
                .await
                .ok();

            // Announce arrival to new room
            let arrival = format!("{} arrives.\r\n", player.name);
            self.broadcast_to_room(new_room_id, &arrival, Some(conn.id()))
                .await;
        }
    }

    /// Broadcast to all players in a specific room
    async fn broadcast_to_room(&self, room_id: u32, message: &str, exclude: Option<u64>) {
        let exclude_ids = exclude.map(|id| vec![id]).unwrap_or_default();

        self.manager
            .broadcast_filtered(message, |conn| {
                // Check if connection is in the target room
                if let Some(room) = conn.get_data::<RoomData>("room") {
                    room.id == room_id && !exclude_ids.contains(&conn.id())
                } else {
                    false
                }
            })
            .await;
    }

    /// Display player info including negotiation status
    async fn show_info(&self, conn: &TelnetConnection) {
        let mut info = String::from("=== Connection Info ===\r\n");

        // Player data
        if let Some(player) = conn.get_data::<PlayerData>("player") {
            let elapsed = player.login_time.elapsed();
            info.push_str(&format!("Name: {}\r\n", player.name));
            info.push_str(&format!("Room: {}\r\n", player.room_id));
            info.push_str(&format!("Online: {:?}\r\n", elapsed));
        }

        // Terminal negotiation status
        if let Some((width, height)) = conn.window_size().await {
            info.push_str(&format!("Window Size: {}x{}\r\n", width, height));
        } else {
            info.push_str("Window Size: Not negotiated\r\n");
        }

        if let Some(term_type) = conn.terminal_type().await {
            info.push_str(&format!("Terminal Type: {}\r\n", term_type));
        } else {
            info.push_str("Terminal Type: Unknown\r\n");
        }

        // Check specific telnet options
        use termionix_telnetcodec::TelnetOption;
        info.push_str(&format!(
            "Echo: {}\r\n",
            if conn.is_option_enabled(TelnetOption::Echo).await {
                "Enabled"
            } else {
                "Disabled"
            }
        ));
        info.push_str(&format!(
            "Suppress Go-Ahead: {}\r\n",
            if conn
                .is_option_enabled(TelnetOption::SuppressGoAhead)
                .await
            {
                "Enabled"
            } else {
                "Disabled"
            }
        ));

        conn.send(&info).await.ok();
    }
}

#[async_trait::async_trait]
impl TelnetHandler for AdvancedHandler {
    async fn on_connect(&self, conn: &TelnetConnection) {
        info!("New connection: {}", conn.id());

        // Send initial prompt
        conn.send("Enter your name: ").await.ok();
    }

    async fn on_disconnect(&self, conn: &TelnetConnection) {
        // Retrieve player data before cleanup
        if let Some(player) = conn.get_data::<PlayerData>("player") {
            info!("Player '{}' disconnected", player.name);

            // Announce to other players
            let message = format!("{} has left the game.\r\n", player.name);
            self.manager
                .broadcast_except(&message, &[conn.id()])
                .await;
        }

        // Metadata is automatically cleaned up when connection drops
    }

    async fn on_data(&self, conn: &TelnetConnection, data: &str) {
        let input = data.trim();

        // Check if player is logged in
        if !conn.has_data("player") {
            // First input is the player name
            if !input.is_empty() {
                self.handle_login(conn, input.to_string()).await;
            } else {
                conn.send("Please enter a name: ").await.ok();
            }
            return;
        }

        // Handle commands
        match input {
            "quit" => {
                conn.send("Goodbye!\r\n").await.ok();
                conn.disconnect().await;
            }
            "info" => {
                self.show_info(conn).await;
            }
            "who" => {
                let mut list = String::from("=== Players Online ===\r\n");
                let connections = self.manager.connections().await;
                for other_conn in connections {
                    if let Some(player) = other_conn.get_data::<PlayerData>("player") {
                        list.push_str(&format!("- {}\r\n", player.name));
                    }
                }
                conn.send(&list).await.ok();
            }
            "north" | "south" => {
                self.handle_move(conn, input).await;
            }
            "say" => {
                conn.send("Say what? Usage: say <message>\r\n").await.ok();
            }
            _ if input.starts_with("say ") => {
                if let Some(player) = conn.get_data::<PlayerData>("player") {
                    let message = &input[4..];
                    let formatted = format!("{} says: {}\r\n", player.name, message);

                    // Broadcast to room
                    if let Some(room) = conn.get_data::<RoomData>("room") {
                        self.broadcast_to_room(room.id, &formatted, None).await;
                    }
                }
            }
            "help" => {
                conn.send("Commands: info, who, north, south, say <message>, quit\r\n")
                    .await
                    .ok();
            }
            "" => {
                // Ignore empty input
            }
            _ => {
                conn.send("Unknown command. Type 'help' for commands.\r\n")
                    .await
                    .ok();
            }
        }
    }

    async fn on_event(&self, conn: &TelnetConnection, event: TerminalEvent) {
        match event {
            // Handle window size changes
            TerminalEvent::WindowSize { width, height } => {
                info!(
                    "Connection {} window size changed: {}x{}",
                    conn.id(),
                    width,
                    height
                );
                conn.send(&format!(
                    "\r\n[Window size updated: {}x{}]\r\n",
                    width, height
                ))
                .await
                .ok();
            }

            // Handle terminal type information
            TerminalEvent::TerminalType { terminal_type } => {
                info!(
                    "Connection {} terminal type: {}",
                    conn.id(),
                    terminal_type
                );
                conn.send(&format!(
                    "\r\n[Terminal type detected: {}]\r\n",
                    terminal_type
                ))
                .await
                .ok();
            }

            // Handle disconnection event
            TerminalEvent::Disconnected => {
                warn!("Connection {} disconnected event", conn.id());
                // Cleanup is handled in on_disconnect
            }

            _ => {
                // Handle other events as needed
            }
        }
    }

    async fn on_command(&self, conn: &TelnetConnection, command: TerminalCommand) {
        // Handle terminal commands if needed
        info!("Connection {} sent command: {:?}", conn.id(), command);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Starting Advanced Features Example Server");

    // Create connection manager
    let manager = Arc::new(ConnectionManager::new());

    // Create handler with manager reference
    let handler = Arc::new(AdvancedHandler::new(manager.clone()));

    // Configure server
    let config = TelnetServerConfig {
        address: "127.0.0.1:4000".parse()?,
        max_connections: 100,
        ..Default::default()
    };

    // Create and start server
    let server = TelnetServer::new(config, handler, manager);

    info!("Server listening on 127.0.0.1:4000");
    info!("Connect with: telnet localhost 4000");
    info!("Press Ctrl+C to stop");

    // Run server
    server.run().await?;

    Ok(())
}


