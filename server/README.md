# Termionix Telnet Service

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Actions Status](https://github.com/huhlig/termionix/workflows/rust/badge.svg)](https://github.com/huhlig/termionix/actions)

([API Docs])

> High-level telnet service framework for building MUD servers and interactive terminal applications.

## Features

- **Connection Management** - Centralized connection tracking and lifecycle management
- **Event-Driven Architecture** - Handler-based API for connection events
- **Type-Safe Metadata** - Store arbitrary typed data per connection
- **Broadcasting** - Send messages to multiple connections with filtering
- **Observability** - Built-in tracing and metrics integration
- **Negotiation Status** - Query telnet option states (NAWS, terminal type, etc.)
- **Async-First** - Built on Tokio for high-performance async I/O

## Quick Start

```rust
use std::sync::Arc;
use termionix_server::{
    ConnectionManager, TelnetConnection, TelnetHandler,
    TelnetServer, TelnetServerConfig,
};
use termionix_terminal::TerminalEvent;

struct MyHandler;

#[async_trait::async_trait]
impl TelnetHandler for MyHandler {
    async fn on_connect(&self, conn: &TelnetConnection) {
        conn.send("Welcome!\r\n").await.ok();
    }

    async fn on_data(&self, conn: &TelnetConnection, data: &str) {
        conn.send(&format!("Echo: {}\r\n", data)).await.ok();
    }

    async fn on_event(&self, conn: &TelnetConnection, event: TerminalEvent) {
        // Handle terminal events
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = TelnetServerConfig {
        address: "127.0.0.1:4000".parse()?,
        ..Default::default()
    };
    
    let manager = Arc::new(ConnectionManager::new());
    let handler = Arc::new(MyHandler);
    let server = TelnetServer::new(config, handler, manager);
    
    server.run().await?;
    Ok(())
}
```

## Connection Metadata

Store typed data per connection for session management:

```rust
#[derive(Clone)]
struct PlayerData {
    name: String,
    level: u32,
    room_id: u32,
}

// Store data
let player = PlayerData {
    name: "Alice".to_string(),
    level: 5,
    room_id: 1,
};
conn.set_data("player", player);

// Retrieve data with type safety
if let Some(player) = conn.get_data::<PlayerData>("player") {
    println!("Player: {} (Level {})", player.name, player.level);
}

// Check if data exists
if conn.has_data("player") {
    // Player is logged in
}

// Remove data
conn.remove_data("player");
```

The metadata system is:
- **Type-safe**: Automatic downcasting with compile-time type checking
- **Thread-safe**: Uses `Arc<RwLock<HashMap>>` for concurrent access
- **Flexible**: Store any type that implements `Any + Send + Sync + Clone`

## Negotiation Status

Query the state of telnet option negotiation:

```rust
use termionix_telnetcodec::TelnetOption;

// Get window size (NAWS)
if let Some((width, height)) = conn.window_size().await {
    println!("Terminal: {}x{}", width, height);
}

// Get terminal type
if let Some(term_type) = conn.terminal_type().await {
    println!("Client: {}", term_type);
}

// Check if specific option is enabled
if conn.is_option_enabled(TelnetOption::Echo).await {
    println!("Echo is enabled");
}

if conn.is_option_enabled(TelnetOption::SuppressGoAhead).await {
    println!("Suppress Go-Ahead is enabled");
}
```

## Broadcasting

Send messages to multiple connections efficiently:

```rust
// Broadcast to all connections
manager.broadcast("Server announcement\r\n").await;

// Broadcast except specific connections (e.g., exclude sender)
manager.broadcast_except(
    "Player joined the game\r\n",
    &[sender_conn_id]
).await;

// Broadcast with custom filter predicate
manager.broadcast_filtered("Room message\r\n", |conn| {
    // Only send to connections in the same room
    conn.get_data::<RoomData>("room")
        .map(|r| r.id == target_room_id)
        .unwrap_or(false)
}).await;
```

All broadcast methods return `BroadcastResult` with statistics:
- `total`: Total connections checked
- `sent`: Successfully sent messages
- `failed`: Failed sends

## Terminal Events

Handle new terminal events:

```rust
async fn on_event(&self, conn: &TelnetConnection, event: TerminalEvent) {
    match event {
        // Window size changed (NAWS)
        TerminalEvent::WindowSize { width, height } => {
            println!("Window: {}x{}", width, height);
        }
        
        // Terminal type received
        TerminalEvent::TerminalType { terminal_type } => {
            println!("Terminal: {}", terminal_type);
        }
        
        // Connection disconnected
        TerminalEvent::Disconnected => {
            println!("Client disconnected");
        }
        
        _ => {}
    }
}
```

## Observability

### Tracing

Enable structured logging with the `tracing` crate:

```rust
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_env_filter(
        tracing_subscriber::EnvFilter::from_default_env()
    )
    .init();
```

Run with: `RUST_LOG=debug cargo run`

The service automatically logs:
- Connection lifecycle (connect, disconnect)
- Message send/receive operations
- Errors with context
- Connection IDs for correlation

### Metrics

The service automatically tracks metrics using the `metrics` crate:

**Connection Metrics:**
- `termionix.connections.total` (counter) - Total connections created
- `termionix.connections.active` (gauge) - Currently active connections

**Throughput Metrics:**
- `termionix.messages.sent` (counter) - Total messages sent
- `termionix.messages.received` (counter) - Total messages received
- `termionix.characters.sent` (counter) - Total characters sent
- `termionix.commands.sent` (counter) - Total commands sent

**Latency Metrics:**
- `termionix.message.send_duration` (histogram) - Send operation duration
- `termionix.message.receive_duration` (histogram) - Receive operation duration

**Error Metrics:**
- `termionix.errors.send` (counter) - Send errors
- `termionix.errors.receive` (counter) - Receive errors

Integrate with your metrics backend:

```rust
use metrics_exporter_prometheus::PrometheusBuilder;

PrometheusBuilder::new()
    .install()
    .expect("failed to install Prometheus recorder");
```

## Examples

See the main project `examples/` directory:
- `simple_server.rs` - Basic echo server
- `echo_server.rs` - Echo with connection management
- `advanced_features.rs` - Complete feature demonstration

## Architecture

The service layer provides:

1. **TelnetServer** - Main server that accepts connections
2. **ConnectionManager** - Tracks and manages all active connections
3. **TelnetConnection** - Wrapper around terminal codec with metadata
4. **TelnetHandler** - Trait for implementing custom connection logic
5. **Worker** - Per-connection task that handles I/O

## License

This project is licensed under [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as 
defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions.

[API Docs]: https://huhlig.github.io/termionix/termionix_server/