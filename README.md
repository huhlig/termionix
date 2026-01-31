# Termionix - Ansi Telnet Library for Tokio 

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Actions Status](https://github.com/huhlig/termionix/workflows/rust/badge.svg)](https://github.com/huhlig/termionix/actions)

([API Docs])

> Termionix is an Ansi Enabled Telnet Library for Tokio.

## Features

- **RFC 854 Compliant Telnet Protocol** - Full implementation of the Telnet protocol
- **ANSI Escape Sequence Handling** - Parse and generate ANSI codes for terminal control
- **MUD Protocol Extensions** - Support for GMCP, MSDP, MSSP, MCCP, NAWS, and more
- **Async-First Design** - Built on Tokio for high-performance async I/O
- **Connection Metadata** - Type-safe storage for per-connection data
- **Observability** - Integrated tracing and metrics support
- **Ergonomic API** - Easy-to-use high-level abstractions

## Quick Start

Add Termionix to your `Cargo.toml`:

```toml
[dependencies]
termionix-service = "0.1"
termionix-terminal = "0.1"
tokio = { version = "1", features = ["full"] }
```

Create a simple telnet server:

```rust
use std::sync::Arc;
use termionix_service::{
    ConnectionManager, TelnetConnection, TelnetHandler, 
    TelnetServer, TelnetServerConfig,
};
use termionix_terminal::{TerminalCommand, TerminalEvent};

struct MyHandler;

#[async_trait::async_trait]
impl TelnetHandler for MyHandler {
    async fn on_connect(&self, conn: &TelnetConnection) {
        conn.send("Welcome to my server!\r\n").await.ok();
    }

    async fn on_data(&self, conn: &TelnetConnection, data: &str) {
        // Echo back to client
        conn.send(&format!("You said: {}\r\n", data)).await.ok();
    }

    async fn on_event(&self, conn: &TelnetConnection, event: TerminalEvent) {
        match event {
            TerminalEvent::WindowSize { width, height } => {
                conn.send(&format!("Window: {}x{}\r\n", width, height)).await.ok();
            }
            _ => {}
        }
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

## Advanced Features

### Connection Metadata

Store typed data per connection:

```rust
#[derive(Clone)]
struct PlayerData {
    name: String,
    level: u32,
}

// Store data
let player = PlayerData { name: "Alice".to_string(), level: 5 };
conn.set_data("player", player);

// Retrieve data
if let Some(player) = conn.get_data::<PlayerData>("player") {
    println!("Player: {} (Level {})", player.name, player.level);
}

// Check existence
if conn.has_data("player") {
    // ...
}

// Remove data
conn.remove_data("player");
```

### Negotiation Status

Query telnet option negotiation state:

```rust
// Get window size (NAWS)
if let Some((width, height)) = conn.window_size().await {
    println!("Terminal size: {}x{}", width, height);
}

// Get terminal type
if let Some(term_type) = conn.terminal_type().await {
    println!("Terminal: {}", term_type);
}

// Check if option is enabled
use termionix_telnetcodec::TelnetOption;
if conn.is_option_enabled(TelnetOption::Echo).await {
    println!("Echo is enabled");
}
```

### Broadcasting

Send messages to multiple connections:

```rust
// Broadcast to all connections
manager.broadcast("Server announcement\r\n").await;

// Broadcast except specific connections
manager.broadcast_except("Player joined\r\n", &[conn.id()]).await;

// Broadcast with custom filter
manager.broadcast_filtered("Room message\r\n", |conn| {
    // Only send to connections in the same room
    conn.get_data::<RoomData>("room")
        .map(|r| r.id == target_room_id)
        .unwrap_or(false)
}).await;
```

### Tracing Integration

Enable structured logging:

```rust
use tracing_subscriber;

tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();
```

Run with: `RUST_LOG=debug cargo run`

### Metrics Integration

Termionix automatically tracks:
- Connection counts (total, active)
- Message throughput (sent, received)
- Character counts
- Operation latency
- Error rates

Integrate with your metrics backend:

```rust
use metrics_exporter_prometheus::PrometheusBuilder;

PrometheusBuilder::new()
    .install()
    .expect("failed to install Prometheus recorder");
```

## Examples

See the `examples/` directory for complete examples:

- `simple_server.rs` - Basic telnet echo server
- `echo_server.rs` - Echo server with connection management
- `advanced_features.rs` - Demonstrates all advanced features
- `ansi_demo.rs` - ANSI escape sequence handling

Run an example:
```bash
cargo run --example advanced_features
```

Then connect with a telnet client:
```bash
telnet localhost 4000
```

## Project Structure

* `.github` - GitHub Actions Workflows and Issue Templates
* `ansicodec` - ANSI String Handling Library
* `telnetcodec` - Telnet Framed Codec for Tokio
* `terminal` - ANSI Enabled Telnet Terminal
* `service` - High-Level Telnet Service Framework
* `compress` - MCCP Compression Support
* `doc` - Documentation, Specifications, and RFCs
* `examples` - Usage Examples

## Documentation

- [API Documentation][API Docs]
- [CHANGELOG](CHANGELOG.md) - Version history and changes
- [Integration Guide](../TERMIONIX_INTEGRATION_REFACTOR.md) - Downstream integration notes

## License

This project is licensed under [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as 
defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions.

[API Docs]: https://huhlig.github.io/termionix/