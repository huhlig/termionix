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

//! Telnet connection implementation for

use crate::{ConnectionId, Result, TelnetError};
use futures_util::{SinkExt, StreamExt};
use metrics::{counter, gauge, histogram};
use std::any::Any;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use termionix_ansicodec::{AnsiCodec, AnsiConfig};
use termionix_compress::{Algorithm, CompressionStream};
use termionix_telnetcodec::TelnetCodec;
use termionix_terminal::{TerminalCodec, TerminalEvent};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::codec::{Encoder, Framed};
use tracing::{debug, error, info, instrument, trace, warn};

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

    // User-defined metadata storage
    user_data: Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>,
}

impl TelnetConnection {
    /// Wrap a TCP stream into a TelnetConnection
    #[instrument(skip(socket), fields(connection_id = %id))]
    pub fn wrap(socket: TcpStream, id: ConnectionId) -> Result<Self> {
        let peer_addr = socket.peer_addr()?;

        info!(
            peer_addr = %peer_addr,
            "Creating new telnet connection"
        );

        // Metrics: increment connection counter
        counter!("termionix.connections.total").increment(1);
        gauge!("termionix.connections.active").increment(1.0);

        // Create the codec stack: TelnetCodec -> AnsiCodec -> TerminalCodec
        let telnet_codec = TelnetCodec::new();
        let ansi_codec = AnsiCodec::new(AnsiConfig::default(), telnet_codec);
        let terminal_codec = TerminalCodec::new(ansi_codec);

        debug!("Codec stack initialized: TelnetCodec -> AnsiCodec -> TerminalCodec");

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
            user_data: Arc::new(RwLock::new(HashMap::new())),
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
    #[instrument(skip(self, msg), fields(connection_id = %self.id))]
    pub async fn send<M>(&self, msg: M) -> Result<()>
    where
        FullTerminalCodec: Encoder<M>,
        TelnetError: From<<FullTerminalCodec as Encoder<M>>::Error>,
        M: Send,
    {
        trace!("Sending message");
        let start = Instant::now();

        match self.framed.lock().await.send(msg).await {
            Ok(()) => {
                self.messages_sent.fetch_add(1, Ordering::Relaxed);

                // Metrics
                counter!("termionix.messages.sent").increment(1);
                histogram!("termionix.message.send_duration").record(start.elapsed().as_secs_f64());

                trace!("Message sent successfully");
                Ok(())
            }
            Err(e) => {
                counter!("termionix.errors.send").increment(1);
                error!("Failed to send message");
                Err(e.into())
            }
        }
    }

    /// Send a character
    #[instrument(skip(self), fields(connection_id = %self.id, character = %ch))]
    pub async fn send_char(&self, ch: char) -> Result<()> {
        self.framed.lock().await.send(ch).await?;
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        counter!("termionix.characters.sent").increment(1);
        Ok(())
    }

    /// Send a terminal command
    #[instrument(skip(self), fields(connection_id = %self.id))]
    pub async fn send_command(&self, cmd: &termionix_terminal::TerminalCommand) -> Result<()> {
        use futures_util::SinkExt;
        debug!(command = ?cmd, "Sending terminal command");
        let mut framed = self.framed.lock().await;
        // Use SinkExt::send with explicit type annotation
        SinkExt::<&termionix_terminal::TerminalCommand>::send(&mut *framed, cmd).await?;
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        counter!("termionix.commands.sent").increment(1);
        Ok(())
    }

    /// Receive the next event
    #[instrument(skip(self), fields(connection_id = %self.id))]
    pub async fn next(&mut self) -> Result<Option<TerminalEvent>> {
        let start = Instant::now();

        match self.framed.lock().await.next().await {
            Some(result) => {
                match result {
                    Ok(event) => {
                        self.messages_received.fetch_add(1, Ordering::Relaxed);

                        // Metrics
                        counter!("termionix.messages.received").increment(1);
                        histogram!("termionix.message.receive_duration")
                            .record(start.elapsed().as_secs_f64());

                        trace!(event = ?event, "Event received");
                        Ok(Some(event))
                    }
                    Err(e) => {
                        counter!("termionix.errors.receive").increment(1);
                        error!("Error receiving event");
                        Err(e.into())
                    }
                }
            }
            None => {
                debug!("Connection stream ended");
                gauge!("termionix.connections.active").decrement(1.0);
                Ok(None)
            }
        }
    }

    /// Store user-defined metadata
    ///
    /// # Example
    /// ```no_run
    /// # use termionix_service::TelnetConnection;
    /// # async fn example(conn: &TelnetConnection) {
    /// conn.set_data("session_id", 12345u64);
    /// conn.set_data("username", "player1".to_string());
    /// # }
    /// ```
    pub fn set_data<T: Any + Send + Sync + Clone>(&self, key: &str, value: T) {
        self.user_data
            .write()
            .unwrap()
            .insert(key.to_string(), Box::new(value));
    }

    /// Retrieve user-defined metadata
    ///
    /// Returns `None` if the key doesn't exist or the type doesn't match.
    ///
    /// # Example
    /// ```no_run
    /// # use termionix_service::TelnetConnection;
    /// # async fn example(conn: &TelnetConnection) {
    /// if let Some(session_id) = conn.get_data::<u64>("session_id") {
    ///     println!("Session ID: {}", session_id);
    /// }
    /// # }
    /// ```
    pub fn get_data<T: Any + Send + Sync + Clone>(&self, key: &str) -> Option<T> {
        self.user_data
            .read()
            .unwrap()
            .get(key)
            .and_then(|v| v.downcast_ref::<T>())
            .cloned()
    }

    /// Remove user-defined metadata
    ///
    /// Returns `true` if the key existed and was removed.
    ///
    /// # Example
    /// ```no_run
    /// # use termionix_service::TelnetConnection;
    /// # async fn example(conn: &TelnetConnection) {
    /// conn.remove_data("session_id");
    /// # }
    /// ```
    pub fn remove_data(&self, key: &str) -> bool {
        self.user_data.write().unwrap().remove(key).is_some()
    }

    /// Check if user-defined metadata exists for a key
    ///
    /// # Example
    /// ```no_run
    /// # use termionix_service::TelnetConnection;
    /// # async fn example(conn: &TelnetConnection) {
    /// if conn.has_data("session_id") {
    ///     println!("Session data exists");
    /// }
    /// # }
    /// ```
    pub fn has_data(&self, key: &str) -> bool {
        self.user_data.read().unwrap().contains_key(key)
    }

    /// Get the negotiated window size (NAWS)
    ///
    /// Returns the current terminal window size if it has been negotiated,
    /// or None if NAWS negotiation hasn't completed.
    ///
    /// # Example
    /// ```no_run
    /// # use termionix_service::TelnetConnection;
    /// # async fn example(conn: &TelnetConnection) {
    /// if let Some((width, height)) = conn.window_size().await {
    ///     println!("Terminal size: {}x{}", width, height);
    /// }
    /// # }
    /// ```
    pub async fn window_size(&self) -> Option<(u16, u16)> {
        let framed = self.framed.lock().await;
        let buffer = framed.codec().buffer();
        let size = buffer.size();
        Some((size.cols as u16, size.rows as u16))
    }

    /// Get the negotiated terminal type
    ///
    /// Returns the terminal type string if it has been negotiated via
    /// the TERMINAL-TYPE option, or None if not available.
    ///
    /// # Example
    /// ```no_run
    /// # use termionix_service::TelnetConnection;
    /// # async fn example(conn: &TelnetConnection) {
    /// if let Some(term_type) = conn.terminal_type().await {
    ///     println!("Terminal type: {}", term_type);
    /// }
    /// # }
    /// ```
    pub async fn terminal_type(&self) -> Option<String> {
        let framed = self.framed.lock().await;
        let buffer = framed.codec().buffer();
        buffer.get_environment("TERM").map(|s| s.to_string())
    }

    /// Check if a telnet option is enabled
    ///
    /// This checks the current negotiation state for a specific telnet option.
    /// Note: This is a simplified check based on available state. For full
    /// Q-state tracking, access the underlying codec directly.
    ///
    /// # Example
    /// ```no_run
    /// # use termionix_service::TelnetConnection;
    /// # use termionix_telnetcodec::TelnetOption;
    /// # async fn example(conn: &TelnetConnection) {
    /// if conn.is_option_enabled(TelnetOption::NAWS).await {
    ///     println!("NAWS is enabled");
    /// }
    /// # }
    /// ```
    pub async fn is_option_enabled(&self, option: termionix_telnetcodec::TelnetOption) -> bool {
        // For now, we check based on available data
        // NAWS is enabled if we have non-default size
        // This is a simplified implementation
        match option {
            termionix_telnetcodec::TelnetOption::NAWS => {
                let size = self.window_size().await;
                size.is_some() && size != Some((80, 24))
            }
            _ => {
                // For other options, we'd need to access the codec's option state
                // This would require exposing more internal state
                false
            }
        }
    }

    /// Check if there are pending sidechannel responses that need to be flushed
    pub async fn has_pending_responses(&self) -> bool {
        let framed = self.framed.lock().await;
        let codec = framed.codec();

        // Navigate through the codec stack: TerminalCodec -> AnsiCodec -> TelnetCodec
        codec.codec().inner().has_pending_responses()
    }

    /// Flush any pending sidechannel responses to the connection
    pub async fn flush_responses(&self) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let mut framed = self.framed.lock().await;

        // Create a buffer for the responses
        let mut buffer = tokio_util::bytes::BytesMut::new();

        // Navigate through codec stack and flush responses
        // TerminalCodec -> AnsiCodec -> TelnetCodec
        {
            let codec = framed.codec_mut();
            codec.codec_mut().inner_mut().flush_responses(&mut buffer)?;
        }

        // If we have data to send, write it directly to the underlying stream
        if !buffer.is_empty() {
            let stream = framed.get_mut().get_mut();
            stream.write_all(&buffer).await?;
            stream.flush().await?;
        }

        Ok(())
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
