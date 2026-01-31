# Changelog

All notable changes to the Termionix project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

#### Telnet Protocol Support
- Added support for Telnet EOR (End of Record) option (RFC 885)
  - `TelnetFrame::EndOfRecord` - Frame variant for IAC EOR command
  - `TelnetEvent::EndOfRecord` - Event variant emitted when EOR is received
  - `TelnetCommand::EndOfRecord` - ANSI command variant for EOR
  - EOR option (25) enabled in SUPPORT table for both local and remote
  - Used by MUD servers to mark the end of prompts (lines without \r\n)
  - Allows clients to distinguish between regular output and prompts
  - Encodes as `IAC EOR` (0xFF 0xEF)
  - Full negotiation support via WILL/WONT/DO/DONT

#### Terminal Events
- Added `WindowSize` event variant to `TerminalEvent` for NAWS window size changes
  - Contains `width: u16` and `height: u16` fields
  - Emitted when terminal window size is negotiated or changed
- Added `TerminalType` event variant to `TerminalEvent` for terminal type information
  - Contains `terminal_type: String` field
  - Emitted when terminal type is received via TERMINAL-TYPE option
- Added `Disconnected` event variant to `TerminalEvent`
  - Signals that a connection has been disconnected
  - Useful for cleanup and state management

#### Encoder Support
- Implemented `Encoder<&String>` for `TerminalCodec`
  - Enables ergonomic usage: `.send(&format!("Hello {}", name))` instead of `.send(format!(...).as_str())`
  - Delegates to existing `&str` encoder for efficiency
  - Proper lifetime handling with `&'a String`

#### Connection Metadata Storage
- Added type-safe metadata storage to `TelnetConnection`
  - `set_data<T>(&self, key: &str, value: T)` - Store typed metadata
  - `get_data<T>(&self, key: &str) -> Option<T>` - Retrieve with type safety
  - `remove_data(&self, key: &str) -> bool` - Remove metadata by key
  - `has_data(&self, key: &str) -> bool` - Check if metadata exists
  - Thread-safe with `Arc<RwLock<HashMap>>` backing storage
  - Supports any type that implements `Any + Send + Sync + Clone`

#### Tracing Integration
- Integrated `tracing` crate for structured logging
  - Connection lifecycle spans with `#[instrument]` macro
  - Automatic connection_id field injection
  - Event-level logging (trace, debug, info, error)
  - Spans for `wrap()`, `send()`, `receive()` operations
  - Error logging with context

#### Metrics Integration  
- Integrated `metrics` crate (v0.24) for observability
  - Connection metrics:
    - `termionix.connections.total` (counter) - Total connections created
    - `termionix.connections.active` (gauge) - Currently active connections
  - Throughput metrics:
    - `termionix.messages.sent` (counter) - Total messages sent
    - `termionix.messages.received` (counter) - Total messages received
    - `termionix.characters.sent` (counter) - Total characters sent
    - `termionix.commands.sent` (counter) - Total commands sent
  - Latency metrics:
    - `termionix.message.send_duration` (histogram) - Send operation duration
    - `termionix.message.receive_duration` (histogram) - Receive operation duration
  - Error metrics:
    - `termionix.errors.send` (counter) - Send errors
    - `termionix.errors.receive` (counter) - Receive errors

#### Negotiation Status API
- Added read-only negotiation status methods to `TelnetConnection`:
  - `window_size() -> Option<(u16, u16)>` - Get negotiated window size (NAWS)
  - `terminal_type() -> Option<String>` - Get negotiated terminal type
  - `is_option_enabled(option: TelnetOption) -> bool` - Check if telnet option is enabled
  - All methods are async and properly documented

#### Broadcast Helpers
- Added `broadcast_except()` method to `ConnectionManager`
  - Broadcasts to all connections except specified ones
  - Useful for "echo to all except sender" patterns
  - Returns `BroadcastResult` with statistics
  - Concurrent execution for performance
- Note: `broadcast_filtered()` already existed and provides predicate-based filtering

### Changed
- Updated workspace dependencies to include `metrics` crate
- Fixed import paths: `termionix-codec` renamed to `termionix-telnetcodec` throughout

### Documentation
- Added comprehensive inline documentation for all new APIs
- Added usage examples in doc comments
- Updated `TERMIONIX_INTEGRATION_REFACTOR.md` with implementation details
- Created `CHANGELOG.md` (this file)

## [0.1.0] - 2025-01-XX

### Added
- Initial release of Termionix library
- RFC 854 compliant Telnet protocol implementation
- ANSI escape sequence handling
- Terminal emulation layer
- High-level server framework
- MUD-specific protocol extensions (GMCP, MSDP, MSSP, MCCP, NAWS)
- Async-first design with Tokio integration

[Unreleased]: https://github.com/huhlig/termionix/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/huhlig/termionix/releases/tag/v0.1.0