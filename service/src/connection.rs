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

//! Terminal-specific split connection implementation
//!
//! This module provides a specialized split connection for terminal I/O with
//! independent read and write workers to prevent blocking issues.
//!
//! # Architecture
//!
//! The [`SplitTerminalConnection`] uses a dual-worker architecture:
//!
//! - **Read Worker**: Handles incoming terminal events independently
//! - **Write Worker**: Handles outgoing terminal commands independently
//!
//! This separation ensures that reads never block writes and vice versa, solving
//! the common problem where buffered writes wait for read timeouts.
//!
//! # Examples
//!
//! ```no_run
//! use termionix_service::SplitTerminalConnection;
//! use termionix_terminal::{TerminalCodec, TerminalCommand};
//! use termionix_ansicodec::{AnsiCodec, AnsiConfig};
//! use termionix_telnetcodec::TelnetCodec;
//! use tokio::net::TcpStream;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let stream = TcpStream::connect("localhost:23").await?;
//!
//! // Create codec stack
//! let telnet_codec = TelnetCodec::new();
//! let ansi_codec = AnsiCodec::new(AnsiConfig::default(), telnet_codec);
//! let codec = TerminalCodec::new(ansi_codec);
//!
//! let conn = SplitTerminalConnection::from_stream(
//!     stream,
//!     codec.clone(),
//!     codec,
//! );
//!
//! // Send command (never blocks on reads)
//! conn.send(TerminalCommand::Text("Hello".to_string()), true).await?;
//!
//! // Receive events (never blocks on writes)
//! while let Some(event) = conn.next().await? {
//!     println!("Received: {:?}", event);
//! }
//! # Ok(())
//! # }
//! ```

use crate::{ConnectionError, ConnectionResult, FlushStrategy};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use termionix_compress::{CompressionAlgorithm, CompressionReader, CompressionWriter};
use termionix_terminal::{TerminalCommand, TerminalEvent};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{Mutex, RwLock, mpsc};
use tokio_util::codec::{Encoder, FramedRead, FramedWrite};

/// Write command for terminal output
enum WriteCommand {
    Send(TerminalCommand, bool), // (command, force_flush)
    Flush,
    Close,
    SetCompression(CompressionAlgorithm), // Set compression algorithm
}

/// Read command for terminal input
enum ReadCommand {
    ReadNext(tokio::sync::oneshot::Sender<ConnectionResult<Option<TerminalEvent>>>),
    Close,
    SetCompression(CompressionAlgorithm), // Set decompression algorithm
}

/// Terminal-specific split connection
///
/// This connection uses concrete types:
/// - Input: TerminalEvent (events coming from the terminal)
/// - Output: TerminalCommand (commands going to the terminal)
/// - Codec: TerminalCodec
///
/// Note: This struct can be cloned to create multiple handles to the same connection.
/// The underlying reader and writer tasks are shared through channels.
#[derive(Debug)]
pub struct SplitTerminalConnection<R, W, C>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
    C: Encoder<TerminalCommand> + Clone + Send + 'static,
{
    /// Command sender for write operations
    write_tx: mpsc::UnboundedSender<WriteCommand>,

    /// Command sender for read operations
    read_tx: mpsc::UnboundedSender<ReadCommand>,

    /// Flush strategy
    flush_strategy: Arc<RwLock<FlushStrategy>>,

    /// Reader task handle
    reader_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,

    /// Writer task handle
    writer_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,

    /// Phantom data
    _phantom: std::marker::PhantomData<(R, W, C)>,
}

impl<R, W, C> SplitTerminalConnection<R, W, C>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
    C: tokio_util::codec::Decoder<Item = TerminalEvent>
        + Encoder<TerminalCommand>
        + Clone
        + Send
        + 'static,
    <C as tokio_util::codec::Decoder>::Error: std::error::Error + Send + Sync + 'static,
    <C as Encoder<TerminalCommand>>::Error: std::error::Error + Send + Sync + 'static,
{
    /// Create a new terminal split connection with compression support
    ///
    /// The reader and writer are wrapped with `CompressionReader` and `CompressionWriter`
    /// respectively, starting with `Algorithm::None`. Compression can be enabled later
    /// via `set_compression_algorithm`.
    pub fn new(reader: R, writer: W, codec_read: C, codec_write: C) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (write_tx, write_rx) = mpsc::unbounded_channel();
        let (read_tx, read_rx) = mpsc::unbounded_channel();
        let flush_strategy = Arc::new(RwLock::new(FlushStrategy::default()));

        // Wrap reader and writer with compression support
        let compressed_reader = CompressionReader::new(reader, CompressionAlgorithm::None);
        let compressed_writer = CompressionWriter::new(writer, CompressionAlgorithm::None);

        // Spawn reader task
        let reader_handle = tokio::spawn(Self::reader_task(
            FramedRead::new(compressed_reader, codec_read),
            read_rx,
        ));

        // Spawn writer task
        let writer_handle = tokio::spawn(Self::writer_task(
            FramedWrite::new(compressed_writer, codec_write),
            write_rx,
            Arc::clone(&flush_strategy),
        ));

        Self {
            write_tx,
            read_tx,
            flush_strategy,
            reader_handle: Arc::new(Mutex::new(Some(reader_handle))),
            writer_handle: Arc::new(Mutex::new(Some(writer_handle))),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create from a bidirectional stream with integrated compression
    ///
    /// The stream is split and each half is wrapped with compression support.
    pub fn from_stream<S>(
        stream: S,
        codec_read: C,
        codec_write: C,
    ) -> SplitTerminalConnection<tokio::io::ReadHalf<S>, tokio::io::WriteHalf<S>, C>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (reader, writer) = tokio::io::split(stream);
        SplitTerminalConnection::new(reader, writer, codec_read, codec_write)
    }

    /// Reader task with compression support
    async fn reader_task(
        mut reader: FramedRead<CompressionReader<R>, C>,
        mut rx: mpsc::UnboundedReceiver<ReadCommand>,
    ) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                ReadCommand::ReadNext(response_tx) => {
                    let result = match reader.next().await {
                        Some(Ok(item)) => Ok(Some(item)),
                        Some(Err(e)) => Err(ConnectionError::Codec(e.to_string())),
                        None => Ok(None),
                    };
                    let _ = response_tx.send(result);
                }
                ReadCommand::SetCompression(algorithm) => {
                    // Switch decompression algorithm
                    if let Err(e) = reader.get_mut().switch_algorithm(algorithm) {
                        eprintln!("Failed to switch decompression algorithm: {:?}", e);
                    }
                }
                ReadCommand::Close => break,
            }
        }
    }

    /// Writer task with compression support
    async fn writer_task(
        mut writer: FramedWrite<CompressionWriter<W>, C>,
        mut rx: mpsc::UnboundedReceiver<WriteCommand>,
        _flush_strategy: Arc<RwLock<FlushStrategy>>,
    ) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                WriteCommand::Send(item, force_flush) => {
                    // Send by reference since codec implements Encoder<&TerminalCommand>
                    if let Err(e) = writer.send(item).await {
                        eprintln!("Write error: {:?}", e);
                        break;
                    }
                    if force_flush {
                        if let Err(e) = writer.flush().await {
                            eprintln!("Flush error: {:?}", e);
                            break;
                        }
                    }
                }
                WriteCommand::Flush => {
                    if let Err(e) = writer.flush().await {
                        eprintln!("Flush error: {:?}", e);
                        break;
                    }
                }
                WriteCommand::SetCompression(algorithm) => {
                    // Switch compression algorithm
                    if let Err(e) = writer.get_mut().switch_algorithm(algorithm).await {
                        eprintln!("Failed to switch compression algorithm: {:?}", e);
                    }
                }
                WriteCommand::Close => {
                    let _ = writer.flush().await;
                    break;
                }
            }
        }
    }

    /// Send a terminal command
    pub async fn send(
        &self,
        data: impl Into<TerminalCommand>,
        force_flush: bool,
    ) -> ConnectionResult<()> {
        self.write_tx
            .send(WriteCommand::Send(data.into(), force_flush))
            .map_err(|_| ConnectionError::Closed)?;
        Ok(())
    }

    /// Receive the next terminal event
    pub async fn next(&self) -> ConnectionResult<Option<TerminalEvent>> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        self.read_tx
            .send(ReadCommand::ReadNext(response_tx))
            .map_err(|_| ConnectionError::Closed)?;

        response_rx
            .await
            .map_err(|_| ConnectionError::ReceiveFailed("Reader task closed".to_string()))?
    }

    /// Manually flush
    pub async fn flush(&self) -> ConnectionResult<()> {
        self.write_tx
            .send(WriteCommand::Flush)
            .map_err(|_| ConnectionError::Closed)?;
        Ok(())
    }

    /// Set flush strategy
    pub async fn set_flush_strategy(&self, strategy: FlushStrategy) {
        *self.flush_strategy.write().await = strategy;
    }

    /// Get flush strategy
    pub async fn flush_strategy(&self) -> FlushStrategy {
        *self.flush_strategy.read().await
    }

    /// Set the compression algorithm for both read and write operations
    ///
    /// This dynamically switches the compression algorithm used by the connection.
    /// The change takes effect immediately for all subsequent data.
    ///
    /// # Parameters
    ///
    /// - `algorithm`: The compression algorithm to use (None, Gzip, Deflate, Brotli, Zlib, Zstd)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use termionix_service::SplitTerminalConnection;
    /// # use termionix_compress::CompressionAlgorithm;
    /// # async fn example(conn: &SplitTerminalConnection<_, _, _>) {
    /// // Enable Gzip compression
    /// conn.set_compression_algorithm(CompressionAlgorithm::Gzip).await.unwrap();
    ///
    /// // Disable compression
    /// conn.set_compression_algorithm(CompressionAlgorithm::None).await.unwrap();
    /// # }
    /// ```
    pub async fn set_compression_algorithm(
        &self,
        algorithm: CompressionAlgorithm,
    ) -> ConnectionResult<()> {
        // Send compression change command to both reader and writer tasks
        self.read_tx
            .send(ReadCommand::SetCompression(algorithm))
            .map_err(|_| ConnectionError::Closed)?;

        self.write_tx
            .send(WriteCommand::SetCompression(algorithm))
            .map_err(|_| ConnectionError::Closed)?;

        Ok(())
    }

    /// Close the connection
    pub async fn close(&self) -> ConnectionResult<()> {
        let _ = self.write_tx.send(WriteCommand::Close);
        let _ = self.read_tx.send(ReadCommand::Close);

        if let Some(handle) = self.writer_handle.lock().await.take() {
            let _ = handle.await;
        }
        if let Some(handle) = self.reader_handle.lock().await.take() {
            let _ = handle.await;
        }

        Ok(())
    }
}

// Manual Clone implementation since ReadHalf and WriteHalf don't implement Clone,
// but the connection can be cloned through its channel-based architecture
impl<R, W, C> Clone for SplitTerminalConnection<R, W, C>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
    C: Encoder<TerminalCommand> + Clone + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            write_tx: self.write_tx.clone(),
            read_tx: self.read_tx.clone(),
            flush_strategy: Arc::clone(&self.flush_strategy),
            reader_handle: Arc::clone(&self.reader_handle),
            writer_handle: Arc::clone(&self.writer_handle),
            _phantom: std::marker::PhantomData,
        }
    }
}
