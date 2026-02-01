
# Termionix Telnet Client

High-level Telnet client library for connecting to Telnet servers, MUDs, and other text-based services.

## Features

- **Automatic Protocol Negotiation** - Handles NAWS, terminal type, echo, and other Telnet options
- **Reconnection Support** - Automatic reconnection with configurable retry logic
- **Event-Driven Architecture** - Handler-based API for processing server events
- **Type-Safe Metadata** - Store arbitrary typed data per connection
- **Async-First** - Built on Tokio for high-performance async I/O
- **Connection Management** - Similar API to the service layer for consistency

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
termionix-client = "0.1"
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"
```

Basic example:

```rust
use termionix_client::{TelnetClient, ClientConfig, ClientHandler, ClientConnection};
use async_trait::async_trait;
use std::sync::Arc;

struct MyHandler;

#[async_trait]
impl ClientHandler for MyHandler {
    async fn on_connect(&self, conn: &ClientConnection) {
        println!("Connected to server!");
    }
    
    async fn on_data(&self, conn: &ClientConnection, data: &[u8]) {
        print!("{}", String::from_utf8_lossy(data));
    }
    
    async fn on_line(&self, conn: &ClientConnection, line: