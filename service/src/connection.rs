//
// Copyright 2017-2025 Hans W. Uhlig. All Rights Reserved.
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

//! Telnet connection implementation for 

use crate::{ConnectionId, Result, TelnetError};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use termionix_ansicodec::{AnsiCodec, AnsiConfig};
use termionix_codec::TelnetCodec;
use termionix_compress::{Algorithm, CompressionStream};
use termionix_terminal::{TerminalCodec, TerminalEvent};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::codec::{Encoder, Framed};

/// Type alias for the complete codec stack
type FullTerminalCodec = TerminalCodec<AnsiCodec<TelnetCodec>>;

/// A Telnet connection ( implementation)
///
/// This is a simplified connection that doesn't manage its own task.
/// Task management is handled by the ConnectionWorker.
#[derive(Clone)]
pub struct TelnetConnection {
    // Core I/O
    framed: Arc<Mutex<Framed<CompressionStream<TcpStream>, FullTerminalCodec>>>,

    // Metadata (lock-free access)
    id: ConnectionId,
    peer_addr: SocketAddr,
    created_at: Instant,

    // Metrics (lock-free)
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    messages_sent: Arc<AtomicU64>,
    messages_received: Arc<AtomicU64>,
}

impl TelnetConnection {
    /// Wrap a TCP stream into a TelnetConnection
    pub fn wrap(socket: TcpStream, id: ConnectionId) -> Result<Self> {
        let peer_addr = socket.peer_addr()?;
        
        // Create the codec stack: TelnetCodec -> AnsiCodec -> TerminalCodec
        let telnet_codec = TelnetCodec::new();
        let ansi_codec = AnsiCodec::new(AnsiConfig::default(), telnet_codec);
        let terminal_codec = TerminalCodec::new(ansi_codec);
        
        Ok(Self {
            framed: Arc::new(Mutex::new(Framed::new(
                CompressionStream::new(socket, Algorithm::None),
                terminal_codec,
            ))),
            id,
            peer_addr,
            created_at: Instant::now(),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            messages_sent: Arc::new(AtomicU64::new(0)),
            messages_received: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Get the connection ID
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    /// Get the peer address
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Get when the connection was created
    pub fn created_at(&self) -> Instant {
        self.created_at
    }

    /// Get bytes sent
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    /// Get bytes received
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    /// Get messages sent
    pub fn messages_sent(&self) -> u64 {
        self.messages_sent.load(Ordering::Relaxed)
    }

    /// Get messages received
    pub fn messages_received(&self) -> u64 {
        self.messages_received.load(Ordering::Relaxed)
    }

    /// Send a message
    pub async fn send<M>(&self, msg: M) -> Result<()>
    where
        FullTerminalCodec: Encoder<M>,
        TelnetError: From<<FullTerminalCodec as Encoder<M>>::Error>,
        M: Send,
    {
        self.framed.lock().await.send(msg).await?;
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    
    /// Send a character
    pub async fn send_char(&self, ch: char) -> Result<()> {
        self.framed.lock().await.send(ch).await?;
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
    
    /// Send a terminal command
    pub async fn send_command(&self, cmd: &termionix_terminal::TerminalCommand) -> Result<()> {
        use futures_util::SinkExt;
        let mut framed = self.framed.lock().await;
        // Use SinkExt::send with explicit type annotation
        SinkExt::<&termionix_terminal::TerminalCommand>::send(&mut *framed, cmd).await?;
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Receive the next event
    pub async fn next(&mut self) -> Result<Option<TerminalEvent>> {
        match self.framed.lock().await.next().await {
            Some(result) => {
                let event = result?;
                self.messages_received.fetch_add(1, Ordering::Relaxed);
                Ok(Some(event))
            }
            None => Ok(None),
        }
    }

}

impl std::fmt::Debug for TelnetConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelnetConnection")
            .field("id", &self.id)
            .field("peer_addr", &self.peer_addr)
            .field("created_at", &self.created_at)
            .finish()
    }
}


