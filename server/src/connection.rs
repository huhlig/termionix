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
use metrics::{counter, gauge, histogram};
use std::any::Any;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use termionix_service::{
    AnsiCodec, AnsiConfig, FlushStrategy, SplitTerminalConnection, TelnetCodec, TerminalCodec,
    TerminalCommand, TerminalEvent,
};
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tracing::{debug, error, info, instrument, trace, warn};

/// Type alias for the complete codec stack
type FullTerminalCodec = TerminalCodec<AnsiCodec<TelnetCodec>>;

/// A Telnet connection with split read/write architecture and integrated compression
///
/// This connection uses the unified SplitConnection architecture which separates
/// read and write operations into independent background workers, preventing
/// blocking issues where buffered writes wait for read operations to complete.
///
/// Compression is integrated at the connection level via `CompressionReader` and
/// `CompressionWriter`, which wrap the read and write halves independently.
#[derive(Clone)]
pub struct TelnetConnection {
    // Core I/O - Split connection with independent read/write workers
    // Compression is integrated internally via CompressionReader/CompressionWriter
    split: SplitTerminalConnection<ReadHalf<TcpStream>, WriteHalf<TcpStream>, FullTerminalCodec>,

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
    /// Wrap a TCP stream into a TelnetConnection with split read/write architecture
    #[instrument(skip(socket), fields(connection_id = %id))]
    pub fn wrap(socket: TcpStream, id: ConnectionId) -> Result<Self> {
        let peer_addr = socket.peer_addr()?;

        info!(
            peer_addr = %peer_addr,
            "Creating new telnet connection with split architecture"
        );

        // Metrics: increment connection counter
        counter!("termionix.connections.total").increment(1);
        gauge!("termionix.connections.active").increment(1.0);

        // Create the codec stack: TelnetCodec -> AnsiCodec -> TerminalCodec
        let telnet_codec = TelnetCodec::new();
        let ansi_codec = AnsiCodec::new(AnsiConfig::default(), telnet_codec);
        let terminal_codec = TerminalCodec::new(ansi_codec);

        debug!("Codec stack initialized: TelnetCodec -> AnsiCodec -> TerminalCodec");

        // Create split connection with integrated compression support
        // CompressionReader and CompressionWriter wrap the read/write halves internally
        let split = SplitTerminalConnection::<
            tokio::io::ReadHalf<TcpStream>,
            tokio::io::WriteHalf<TcpStream>,
            TerminalCodec<AnsiCodec<TelnetCodec>>,
        >::from_stream(socket, terminal_codec.clone(), terminal_codec);

        // Set flush strategy to OnNewline for line-based protocols
        tokio::spawn({
            let split = split.clone();
            async move {
                split.set_flush_strategy(FlushStrategy::OnNewline).await;
            }
        });

        Ok(Self {
            split,
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

    /// Send a message to the remote endpoint. Follows autoflush rules unless flush is true.
    #[instrument(skip(self, msg), fields(connection_id = %self.id))]
    pub async fn send<M>(&self, msg: M, flush: bool) -> Result<()>
    where
        M: Send + AsRef<str>,
    {
        trace!("Sending message");
        let start = Instant::now();

        // Wrap message in TerminalCommand::Text and delegate to split connection
        let cmd = TerminalCommand::text(msg.as_ref());
        self.split.send(cmd, flush).await.map_err(|e| {
            TelnetError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        self.messages_sent.fetch_add(1, Ordering::Relaxed);

        // Metrics
        counter!("termionix.messages.sent").increment(1);
        histogram!("termionix.message.send_duration").record(start.elapsed().as_secs_f64());

        trace!("Message sent successfully");
        Ok(())
    }

    /// Send a character. Follows Autoflush rules unless flush is set to true.
    #[instrument(skip(self), fields(connection_id = %self.id, character = %ch))]
    pub async fn send_char(&self, ch: char, flush: bool) -> Result<()> {
        // Wrap character in TerminalCommand::Char and delegate to split connection
        let cmd = TerminalCommand::char(ch);
        self.split.send(cmd, flush).await.map_err(|e| {
            TelnetError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        counter!("termionix.characters.sent").increment(1);
        Ok(())
    }

    /// Send a line of text (automatically adds line ending and flushes)
    #[instrument(skip(self, line), fields(connection_id = %self.id))]
    pub async fn send_line(&self, line: &str) -> Result<()> {
        trace!("Sending line");

        // Add proper line ending if not present
        let line_with_ending = if line.ends_with("\r\n") {
            line.to_string()
        } else if line.ends_with('\n') {
            format!("{}\r", line.trim_end_matches('\n'))
        } else {
            format!("{}\r\n", line)
        };

        // Wrap in TerminalCommand::Text and always flush after sending a complete line
        let cmd = TerminalCommand::text(line_with_ending);
        self.split.send(cmd, true).await.map_err(|e| {
            TelnetError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        counter!("termionix.lines.sent").increment(1);

        Ok(())
    }

    /// Send a terminal command (always flushes immediately)
    #[instrument(skip(self), fields(connection_id = %self.id))]
    pub async fn send_command(&self, cmd: &TerminalCommand) -> Result<()> {
        debug!(command = ?cmd, "Sending terminal command");

        // Clone and send the command, always flush immediately
        self.split.send(cmd.clone(), true).await.map_err(|e| {
            TelnetError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        self.messages_sent.fetch_add(1, Ordering::Relaxed);
        counter!("termionix.commands.sent").increment(1);
        Ok(())
    }

    /// Receive the next event
    #[instrument(skip(self), fields(connection_id = %self.id))]
    pub async fn next(&mut self) -> Result<Option<TerminalEvent>> {
        let start = Instant::now();

        // Delegate to split connection - reads never block writes!
        match self.split.next().await {
            Ok(Some(event)) => {
                self.messages_received.fetch_add(1, Ordering::Relaxed);

                // Metrics
                counter!("termionix.messages.received").increment(1);
                histogram!("termionix.message.receive_duration")
                    .record(start.elapsed().as_secs_f64());

                trace!(event = ?event, "Event received");
                Ok(Some(event))
            }
            Ok(None) => {
                debug!("Connection stream ended");
                gauge!("termionix.connections.active").decrement(1.0);
                Ok(None)
            }
            Err(e) => {
                counter!("termionix.errors.receive").increment(1);
                error!("Error receiving event");
                Err(TelnetError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )))
            }
        }
    }

    /// Store user-defined metadata
    ///
    /// # Example
    /// ```no_run
    /// # use termionix_server::TelnetConnection;
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
    /// # use termionix_server::TelnetConnection;
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
    /// # use termionix_server::TelnetConnection;
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
    /// # use termionix_server::TelnetConnection;
    /// # async fn example(conn: &TelnetConnection) {
    /// if conn.has_data("session_id") {
    ///     println!("Session data exists");
    /// }
    /// # }
    /// ```
    pub fn has_data(&self, key: &str) -> bool {
        self.user_data.read().unwrap().contains_key(key)
    }

    /// Set the flush strategy for this connection
    ///
    /// Controls when buffered data is automatically flushed to the network.
    ///
    /// # Example
    /// ```no_run
    /// # use termionix_server::TelnetConnection;
    /// # use termionix_service::FlushStrategy;
    /// # async fn example(conn: &TelnetConnection) {
    /// conn.set_flush_strategy(FlushStrategy::Immediate).await;
    /// # }
    /// ```
    pub async fn set_flush_strategy(&self, strategy: termionix_service::FlushStrategy) {
        self.split.set_flush_strategy(strategy).await;
    }

    /// Get the current flush strategy
    ///
    /// # Example
    /// ```no_run
    /// # use termionix_server::TelnetConnection;
    /// # async fn example(conn: &TelnetConnection) {
    /// let strategy = conn.flush_strategy().await;
    /// println!("Current strategy: {:?}", strategy);
    /// # }
    /// ```
    pub async fn flush_strategy(&self) -> termionix_service::FlushStrategy {
        self.split.flush_strategy().await
    }

    /// Manually flush any buffered data
    ///
    /// Forces all buffered data to be sent immediately, regardless of the
    /// current flush strategy.
    ///
    /// # Example
    /// ```no_run
    /// # use termionix_server::TelnetConnection;
    /// # async fn example(conn: &TelnetConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// conn.flush().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn flush(&self) -> Result<()> {
        self.split.flush().await.map_err(|e| {
            TelnetError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })
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
