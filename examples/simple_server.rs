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

//! Minimal Telnet Server Example
//!
//! This example demonstrates the absolute minimum code needed to create
//! a working telnet server with Termionix.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example simple_server
//! ```
//!
//! Then connect with:
//! ```bash
//! telnet localhost 2323
//! ```

use std::sync::Arc;
use termionix_server::{CallbackHandler, ServerConfig, TelnetServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure server to listen on port 2323
    let config = ServerConfig::new("127.0.0.1:2323".parse()?);

    // Create server
    let server = TelnetServer::new(config).await?;

    // Create a simple handler that just logs events
    let handler = Arc::new(CallbackHandler::default());

    println!("Simple Telnet Server running on 127.0.0.1:2323");
    println!("Press Ctrl+C to stop");

    // Start server
    server.start(handler).await?;

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    // Shutdown
    server.shutdown().await?;

    Ok(())
}
