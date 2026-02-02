# Termionix Service - Unified Connection Layer

A unified connection abstraction with split read/write streams that prevents blocking issues in telnet/terminal applications.

## Overview

The `termionix-service` crate provides a `SplitConnection` type that separates read and write operations into independent background workers. This architecture solves the common problem where buffered writes wait for read timeouts before being flushed.

## Problem Statement

Traditional connection implementations suffer from read/write coupling:

```rust
// Old approach - writes block on reads
loop {
    tokio::select! {
        // Writes are buffered here
        _ = send_data() => {},
        
        // But only flushed when this completes or times out!
        result = framed.next() => {
            // Process result
        }
    }
}
```

**Result**: Messages sit in buffers waiting for read operations to complete.

## Solution: Dual-Worker Architecture

`SplitConnection` uses two independent background workers:

```
┌─────────────────────────────────────────────────────────┐
│                   SplitConnection                        │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────────┐         ┌──────────────┐             │
│  │ Read Worker  │         │ Write Worker │             │
│  │  (Task)      │         │   (Task)     │             │
│  └──────┬───────┘         └──────┬───────┘             │
│         │                        │                      │
│         │ ReadCommand            │ WriteCommand         │
│         │ (oneshot)              │ (unbounded)          │
│         │                        │                      │
│  ┌──────▼───────┐         ┌──────▼───────┐             │
│  │  read_tx/rx  │         │ write_tx/rx  │             │
│  └──────────────┘         └──────────────┘             │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### Benefits

- ✅ **Reads never block writes**
- ✅ **Writes never block reads**
- ✅ Configurable flush strategies
- ✅ Automatic telnet negotiation handling
- ✅ Cloneable for multi-threaded access
- ✅ Generic over stream types and codecs

## Quick Start

### Basic Usage

```rust
use termionix_server::{SplitConnection, FlushStrategy};
use tokio::net::TcpStream;

// Connect to server
let stream = TcpStream::connect("localhost:23").await?;

// Create connection with codec
let conn = SplitConnection::from_stream(
    stream,
    codec_read,
    codec_write,
);

// Send data (never blocks on reads)
conn.send(data, true).await?;

// Receive data (never blocks on writes)
while let Some(event) = conn.next().await? {
    println!("Received: {:?}", event);
}
```

### With Configuration

```rust
use termionix_server::{ClientConfig, Config, SplitConnection};

// Create client configuration
let config = ClientConfig::new("localhost", 23)
    .with_auto_reconnect(true)
    .with_reconnect_delay(Duration::from_secs(5))
    .with_terminal_size(120, 40);

// Create connection with config
let conn = SplitConnection::from_stream_with_config(
    stream,
    codec_read,
    codec_write,
    Config::Client(config),
);

// Access configuration
if let Some(cfg) = conn.config() {
    if let Some(client_cfg) = cfg.as_client() {
        println!("Connecting to {}", client_cfg.address());
    }
}
```

### Concurrent Read/Write

```rust
// Clone connection for concurrent access
let read_conn = conn.clone();
let write_conn = conn.clone();

// Spawn reader task
tokio::spawn(async move {
    while let Some(event) = read_conn.next().await.unwrap() {
        println!("Received: {:?}", event);
    }
});

// Write from main thread
loop {
    write_conn.send(data, true).await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

## Configuration

### Common Configuration

Settings shared by both client and server:

```rust
use termionix_server::ConnectionConfig;

let config = ConnectionConfig::default()
    .with_terminal_type("xterm-256color")
    .with_terminal_size(120, 40)
    .with_buffer_size(8192)
    .with_keepalive(true)
    .with_keepalive_interval(Duration::from_secs(60));
```

### Client Configuration

Client-specific settings including reconnection:

```rust
use termionix_server::ClientConfig;

let config = ClientConfig::new("example.com", 23)
    .with_connect_timeout(Duration::from_secs(10))
    .with_auto_reconnect(true)
    .with_reconnect_delay(Duration::from_secs(5))
    .with_max_reconnect_attempts(Some(3))
    .with_terminal_size(120, 40);
```

### Server Configuration

Server-specific settings including limits:

```rust
use termionix_server::ServerConfig;

let config = ServerConfig::new()
    .with_max_idle_time(Some(Duration::from_secs(600)))
    .with_max_connection_time(None)
    .with_rate_limiting(true, Some(100))
    .with_terminal_size(80, 24);
```

## Flush Strategies

Control when buffered data is flushed:

```rust
use termionix_server::FlushStrategy;

// Manual flush only
conn.set_flush_strategy(FlushStrategy::Manual).await;
conn.send(data, false).await?;
conn.flush().await?; // Explicit flush

// Immediate flush (every send)
conn.set_flush_strategy(FlushStrategy::Immediate).await;
conn.send(data, false).await?; // Flushes immediately

// Flush on newline (default)
conn.set_flush_strategy(FlushStrategy::OnNewline).await;
conn.send("Hello\n", false).await?; // Flushes automatically

// Flush on threshold
conn.set_flush_strategy(FlushStrategy::OnThreshold(1024)).await;
conn.send(data, false).await?; // Flushes when buffer >= 1024 bytes
```

## Telnet Negotiation

The codec layer automatically handles telnet negotiation:

1. **Incoming IAC sequences** are parsed by TelnetCodec
2. **Automatic responses** are generated and stored
3. **Negotiations are filtered** from the event stream
4. **Users only see** application-level data

```rust
// Incoming: "IAC WILL ECHO Hello\r\n"
//           ↓
// TelnetCodec: Handles IAC, generates "IAC DO ECHO", returns "Hello\r\n"
//           ↓
// User sees: "Hello\r\n" only

let event = conn.next().await?;
// event contains "Hello\r\n", negotiation handled transparently
```

## API Reference

### SplitConnection

```rust
impl<R, W, C> SplitConnection<R, W, C> {
    // Create from separate read/write halves
    pub fn new(reader: R, writer: W, codec_read: C, codec_write: C) -> Self;
    
    // Create with configuration
    pub fn new_with_config(reader: R, writer: W, codec_read: C, codec_write: C, config: Config) -> Self;
    
    // Create from bidirectional stream
    pub fn from_stream<S>(stream: S, codec_read: C, codec_write: C) -> SplitConnection<...>;
    
    // Create from stream with config
    pub fn from_stream_with_config<S>(stream: S, codec_read: C, codec_write: C, config: Config) -> SplitConnection<...>;
    
    // Send data (force_flush overrides strategy)
    pub async fn send(&self, item: C::Item, force_flush: bool) -> Result<()>;
    
    // Manually flush write buffer
    pub async fn flush(&self) -> Result<()>;
    
    // Receive next item (never blocks writes)
    pub async fn next(&self) -> Result<Option<C::Item>>;
    
    // Set flush strategy
    pub async fn set_flush_strategy(&self, strategy: FlushStrategy);
    
    // Get flush strategy
    pub async fn flush_strategy(&self) -> FlushStrategy;
    
    // Get configuration
    pub fn config(&self) -> Option<&Arc<Config>>;
    
    // Check connection type
    pub fn is_client(&self) -> bool;
    pub fn is_server(&self) -> bool;
    
    // Close gracefully
    pub async fn close(&self) -> Result<()>;
}
```

## Examples

### Echo Server

```rust
use termionix_server::{ServerConfig, Config, SplitConnection};

let listener = TcpListener::bind("0.0.0.0:23").await?;

loop {
    let (stream, addr) = listener.accept().await?;
    
    tokio::spawn(async move {
        let config = ServerConfig::new()
            .with_max_idle_time(Some(Duration::from_secs(300)));
        
        let conn = SplitConnection::from_stream_with_config(
            stream,
            codec_read,
            codec_write,
            Config::Server(config),
        );
        
        while let Some(event) = conn.next().await.unwrap() {
            // Echo back
            conn.send(event, true).await.unwrap();
        }
    });
}
```

### Telnet Client

```rust
use termionix_server::{ClientConfig, Config, SplitConnection};

let config = ClientConfig::new("localhost", 23)
    .with_auto_reconnect(true);

let stream = TcpStream::connect(config.address()).await?;

let conn = SplitConnection::from_stream_with_config(
    stream,
    codec_read,
    codec_write,
    Config::Client(config),
);

// Spawn reader
let read_conn = conn.clone();
tokio::spawn(async move {
    while let Some(event) = read_conn.next().await.unwrap() {
        println!("{:?}", event);
    }
});

// Send commands
conn.send("help\r\n", true).await?;
```

## Performance

The dual-worker architecture provides:

- **Zero blocking**: Reads and writes are completely independent
- **Low latency**: Background workers process immediately
- **High throughput**: No waiting for timeouts
- **Efficient**: Minimal overhead from channels

## License

Licensed under the Apache License, Version 2.0.