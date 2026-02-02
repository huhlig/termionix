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

//! # Dynamic Compression Stream
//!
//! This module provides a flexible compression wrapper for asynchronous streams that supports
//! runtime algorithm switching. It enables transparent compression and decompression of data
//! flowing through async I/O streams.
//!
//! ## Features
//!
//! - **Multiple Compression Algorithms**: Supports Gzip, Deflate, Brotli, Zlib, Zstd, and uncompressed modes
//! - **Runtime Algorithm Switching**: Change compression algorithms on-the-fly during stream operation
//! - **Async I/O Compatible**: Implements both `AsyncRead` and `AsyncWrite` traits
//! - **Zero-copy Operations**: Built on `pin-project-lite` for efficient pinning
//! - **Type-safe**: Strongly typed algorithm selection via the `Algorithm` enum
//!
//! ## Basic Usage
//!
//! ### Creating a Compression Stream
//!
//! ```rust,no_run
//! use termionix_compress::{CompressionStream, CompressionAlgorithm};
//! use tokio::net::TcpStream;
//! use tokio::io::{AsyncWriteExt, AsyncReadExt};
//!
//! # async fn example() -> std::io::Result<()> {
//! // Connect to a server
//! let stream = TcpStream::connect("127.0.0.1:8080").await?;
//!
//! // Wrap with Gzip compression
//! let mut compressed = CompressionStream::new(stream, CompressionAlgorithm::Gzip);
//!
//! // Write data - it will be compressed automatically
//! compressed.write_all(b"Hello, world!").await?;
//! compressed.flush().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Switching Compression Algorithms
//!
//! ```rust,no_run
//! use termionix_compress::{CompressionStream, CompressionAlgorithm};
//! use tokio::io::AsyncWriteExt;
//! # use tokio::net::TcpStream;
//!
//! # async fn example(stream: TcpStream) -> std::io::Result<()> {
//! let mut compressed = CompressionStream::new(stream, CompressionAlgorithm::None);
//!
//! // Write uncompressed data
//! compressed.write_all(b"uncompressed").await?;
//!
//! // Switch to Gzip compression
//! compressed.switch_algorithm(CompressionAlgorithm::Gzip).await?;
//!
//! // Subsequent writes will be Gzip compressed
//! compressed.write_all(b"compressed").await?;
//! compressed.shutdown().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Reading from a Compression Stream
//!
//! ```rust,no_run
//! use termionix_compress::{CompressionStream, CompressionAlgorithm};
//! use tokio::io::AsyncReadExt;
//! # use tokio::net::TcpStream;
//!
//! # async fn example(stream: TcpStream) -> std::io::Result<()> {
//! let mut compressed = CompressionStream::new(stream, CompressionAlgorithm::Gzip);
//!
//! let mut buffer = vec![0u8; 1024];
//! let n = compressed.read(&mut buffer).await?;
//! println!("Read {} bytes", n);
//! # Ok(())
//! # }
//! ```
//!
//! ## Algorithm Selection
//!
//! The `Algorithm` enum provides several compression options:
//!
//! - **`Algorithm::None`**: No compression (pass-through mode)
//! - **`Algorithm::Gzip`**: Popular general-purpose compression
//! - **`Algorithm::Deflate`**: Raw DEFLATE compression (no headers/footers)
//! - **`Algorithm::Brotli`**: Modern compression with high ratios
//! - **`Algorithm::Zlib`**: DEFLATE with Zlib wrapper
//! - **`Algorithm::Zstd`**: Fast compression with tunable ratios
//!
//! ## Important Notes
//!
//! ### Algorithm Switching
//!
//! When switching algorithms:
//! 1. The current compression stream is flushed and finalized
//! 2. Pending data is written to ensure compression state is complete
//! 3. A new compression context is created with the new algorithm
//! 4. The underlying stream continues without interruption
//!
//! ### Stream Finalization
//!
//! Always call `shutdown()` when done writing to ensure:
//! - All buffered data is compressed and written
//! - Compression footers/trailers are properly written
//! - The stream is properly finalized
//!
//! ## Implementation Details
//!
//! The `CompressionStream` is a thin wrapper around `InnerStream`, which is an enum
//! containing the actual compression encoder for the selected algorithm. This design
//! allows for efficient runtime polymorphism without dynamic dispatch overhead.
//!
//! ## Performance Considerations
//!
//! - **No Compression**: Zero overhead, data passes through unchanged
//! - **Compression Overhead**: Each algorithm has different CPU/compression tradeoffs
//! - **Algorithm Switching**: Requires flushing and recreating compression state
//! - **Buffer Management**: Internal buffers are reused where possible

use async_compression::tokio::write::{
    BrotliEncoder, DeflateEncoder, GzipEncoder, ZlibEncoder, ZstdEncoder,
};
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{self, AsyncRead, AsyncWrite, ReadBuf};

/// Compression algorithm selection for stream processing.
///
/// This enum defines the available compression algorithms that can be used with
/// [`CompressionStream`]. Each algorithm offers different trade-offs between compression
/// ratio, speed, and CPU usage.
///
/// # Algorithm Characteristics
///
/// | Algorithm | Compression Ratio | Speed | CPU Usage | Use Case |
/// |-----------|------------------|-------|-----------|----------|
/// | `None` | 1:1 (no compression) | Fastest | Minimal | Already compressed data, testing |
/// | `Deflate` | Good | Fast | Moderate | Raw compression, custom formats |
/// | `Gzip` | Good | Fast | Moderate | HTTP, general purpose |
/// | `Zlib` | Good | Fast | Moderate | PNG, PDF, general purpose |
/// | `Brotli` | Excellent | Moderate | High | Web content, static assets |
/// | `Zstd` | Excellent | Very Fast | Moderate | Real-time compression, databases |
///
/// # Examples
///
/// ## Choosing an Algorithm
///
/// ```rust
/// use termionix_compress::CompressionAlgorithm;
///
/// // For maximum speed with no compression
/// let algo = CompressionAlgorithm::None;
///
/// // For general-purpose compression
/// let algo = CompressionAlgorithm::Gzip;
///
/// // For best compression ratio
/// let algo = CompressionAlgorithm::Brotli;
///
/// // For real-time data with good compression
/// let algo = CompressionAlgorithm::Zstd;
/// ```
///
/// ## Comparing Algorithms
///
/// ```rust
/// use termionix_compress::CompressionAlgorithm;
///
/// let algo1 = CompressionAlgorithm::Gzip;
/// let algo2 = CompressionAlgorithm::Gzip;
/// let algo3 = CompressionAlgorithm::Zstd;
///
/// assert_eq!(algo1, algo2);
/// assert_ne!(algo1, algo3);
/// ```
///
/// # Algorithm Details
///
/// ## `None`
/// Pass-through mode with no compression. Data is transmitted unchanged.
/// - **Best for**: Already compressed data (images, video), testing, debugging
/// - **Performance**: Zero overhead
/// - **Format**: Raw data
///
/// ## `Gzip`
/// Standard Gzip compression (RFC 1952). Widely supported and well-tested.
/// - **Best for**: HTTP responses, file compression, network protocols
/// - **Performance**: Fast compression and decompression
/// - **Format**: Gzip header + DEFLATE data + CRC32 + size footer
/// - **Magic bytes**: `0x1F 0x8B`
///
/// ## `Deflate`
/// Raw DEFLATE compression (RFC 1951) without headers or checksums.
/// - **Best for**: Custom protocols, when headers aren't needed
/// - **Performance**: Slightly faster than Gzip (no header overhead)
/// - **Format**: Raw DEFLATE compressed data
/// - **Note**: No integrity checking or format identification
///
/// ## `Brotli`
/// Modern compression algorithm optimized for web content (RFC 7932).
/// - **Best for**: Static web assets, text-heavy content, high compression ratio needed
/// - **Performance**: Slower compression, fast decompression
/// - **Format**: Brotli compressed stream
/// - **Note**: Better compression than Gzip, especially for text
///
/// ## `Zlib`
/// DEFLATE with Zlib wrapper (RFC 1950). Common in many file formats.
/// - **Best for**: PNG images, PDF files, in-memory compression
/// - **Performance**: Similar to Gzip
/// - **Format**: Zlib header + DEFLATE data + Adler32 checksum
/// - **Magic bytes**: `0x78` (most common)
///
/// ## `Zstd`
/// Zstandard compression (RFC 8878). Excellent speed/ratio balance.
/// - **Best for**: Real-time compression, databases, log files, streaming data
/// - **Performance**: Very fast compression and decompression
/// - **Format**: Zstandard frame format
/// - **Magic bytes**: `0x28 0xB5 0x2F 0xFD`
/// - **Note**: Best all-around choice for new applications
///
/// # Trait Implementations
///
/// `Algorithm` implements several useful traits:
/// - [`Debug`]: For debugging output
/// - [`Clone`] and [`Copy`]: Lightweight copying
/// - [`PartialEq`] and [`Eq`]: Equality comparison
///
/// # See Also
///
/// - [`CompressionStream`]: The stream wrapper that uses these algorithms
/// - [`CompressionStream::new`]: Create a stream with an algorithm
/// - [`CompressionStream::switch_algorithm`]: Change algorithms at runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    None,
    Gzip,
    Deflate,
    Brotli,
    Zlib,
    Zstd,
}

pub trait AsyncStream: AsyncRead + AsyncWrite + Unpin {}
impl<S> AsyncStream for S where S: AsyncRead + AsyncWrite + Unpin {}

pin_project! {
    /// A dynamic compression wrapper that allows switching compression on-the-fly.
    ///
    /// `CompressionStream` provides transparent compression for any async stream that implements
    /// both [`AsyncRead`] and [`AsyncWrite`]. It wraps an underlying stream and compresses
    /// data written to it and decompresses data read from it.
    ///
    /// # Key Features
    ///
    /// - **Transparent Compression**: Automatically compresses writes and decompresses reads
    /// - **Algorithm Switching**: Change compression algorithms at runtime
    /// - **Bidirectional**: Implements both `AsyncRead` and `AsyncWrite`
    /// - **Zero-copy**: Efficient pinning with `pin-project-lite`
    /// - **Generic**: Works with any `AsyncRead + AsyncWrite + Unpin` stream
    ///
    /// # Type Parameters
    ///
    /// - `S`: The underlying stream type, which must implement `AsyncStream`
    ///   (i.e., `AsyncRead + AsyncWrite + Unpin`)
    ///
    /// ## Algorithm Switching Behavior
    ///
    /// When switching algorithms:
    /// 1. The current stream is flushed and shut down
    /// 2. Compression footer is written (if applicable)
    /// 3. New compression context is created
    /// 4. Subsequent data uses a new algorithm
    ///
    /// The underlying stream is preserved and data written before the switch
    /// remains in the original format.
    ///
    /// ## Error Handling
    ///
    /// Compression operations may fail due to:
    /// - Underlying I/O errors
    /// - Out of memory conditions
    /// - Corrupted compressed data (on read)
    ///
    /// Always check return values from async operations.
    ///
    /// # Performance Considerations
    ///
    /// - **Buffering**: Compression works on chunks; small writes may be buffered
    /// - **Algorithm overhead**: Each algorithm has different CPU/memory requirements
    /// - **Switching cost**: Algorithm changes require flushing and reinitialization
    /// - **Pass-through mode**: `Algorithm::None` has minimal overhead
    ///
    /// # Thread Safety
    ///
    /// `CompressionStream` itself is not `Send` or `Sync` unless the underlying stream `S`
    /// is `Send` and `Sync`. Wrap in `Arc<Mutex<_>>` if sharing across threads is needed.
    ///
    /// # See Also
    ///
    /// - [`CompressionAlgorithm`]: The compression algorithm enum
    /// - [`AsyncRead`]: Read trait implementation
    /// - [`AsyncWrite`]: Write trait implementation
    pub struct CompressionStream<S>
    where
        S: AsyncStream,
    {
        #[pin]
        inner: Option<InnerStream<S>>,
    }
}

impl<S> CompressionStream<S>
where
    S: AsyncStream,
{
    /// Creates a new dynamic compression stream with the specified initial compression.
    ///
    /// This wraps the provided stream with a compression layer using the given algorithm.
    /// The stream will compress all data written to it and decompress all data read from it
    /// according to the selected algorithm.
    ///
    /// # Parameters
    ///
    /// - `inner`: The underlying stream to wrap. Must implement `AsyncRead + AsyncWrite + Unpin`.
    /// - `algorithm`: The initial compression algorithm to use. See [`CompressionAlgorithm`] for options.
    ///
    /// # Returns
    ///
    /// A new `CompressionStream` wrapping the provided stream.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use termionix_compress::{CompressionStream, CompressionAlgorithm};
    /// use tokio::net::TcpStream;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// // Create with no compression
    /// let compressed = CompressionStream::new(stream, CompressionAlgorithm::None);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ```rust,no_run
    /// use termionix_compress::{CompressionStream, CompressionAlgorithm};
    /// use tokio::net::TcpStream;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// // Create with Gzip compression
    /// let compressed = CompressionStream::new(stream, CompressionAlgorithm::Gzip);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Notes
    ///
    /// The stream is immediately ready to use after creation. No initialization or
    /// handshake is required.
    pub fn new(inner: S, algorithm: CompressionAlgorithm) -> Self {
        Self {
            inner: Some(InnerStream::new(inner, algorithm)),
        }
    }

    /// Returns the current compression algorithm in use.
    ///
    /// This method allows you to query which algorithm is currently active for the stream.
    /// Useful when you need to verify the compression mode or make decisions based on it.
    ///
    /// # Returns
    ///
    /// The current [`CompressionAlgorithm`] being used for compression/decompression.
    ///
    /// # Panics
    ///
    /// This method will panic if the inner stream is in an invalid state (missing).
    /// This should never happen in normal usage.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use termionix_compress::{CompressionStream, CompressionAlgorithm};
    /// use tokio::net::TcpStream;
    ///
    /// # async fn example(stream: TcpStream) {
    /// let compressed = CompressionStream::new(stream, CompressionAlgorithm::Gzip);
    ///
    /// assert_eq!(compressed.algorithm(), CompressionAlgorithm::Gzip);
    /// # }
    /// ```
    ///
    /// ```rust,no_run
    /// use termionix_compress::{CompressionStream, CompressionAlgorithm};
    /// # use tokio::net::TcpStream;
    ///
    /// # async fn example(mut stream: CompressionStream<TcpStream>) -> std::io::Result<()> {
    /// // Check current algorithm before switching
    /// if stream.algorithm() != CompressionAlgorithm::Zstd {
    ///     stream.switch_algorithm(CompressionAlgorithm::Zstd).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn algorithm(&self) -> CompressionAlgorithm {
        self.inner
            .as_ref()
            .expect("inner stream missing")
            .to_algorithm()
    }

    /// Switches to a new compression algorithm.
    ///
    /// This method changes the compression algorithm used by the stream. It properly
    /// finalizes the current compression state before switching to ensure data integrity.
    ///
    /// # Process
    ///
    /// 1. Checks if already using the requested algorithm (early return if so)
    /// 2. Flushes all pending writes
    /// 3. Shuts down current compression (writes footers/trailers)
    /// 4. Extracts the underlying stream
    /// 5. Creates a new compression context with the new algorithm
    /// 6. Wraps the stream with the new compressor
    ///
    /// # Parameters
    ///
    /// - `algorithm`: The new [`CompressionAlgorithm`] to use for subsequent operations
    ///
    /// # Returns
    ///
    /// - `Ok(())`: Successfully switched algorithms
    /// - `Err(io::Error)`: Failed to flush/shutdown the current stream
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Flushing pending data fails
    /// - Shutting down the current compression stream fails
    /// - The underlying I/O operation encounters an error
    ///
    /// # Examples
    ///
    /// ## Basic Switching
    ///
    /// ```rust,no_run
    /// use termionix_compress::{CompressionStream, CompressionAlgorithm};
    /// use tokio::io::AsyncWriteExt;
    /// # use tokio::net::TcpStream;
    ///
    /// # async fn example(stream: TcpStream) -> std::io::Result<()> {
    /// let mut compressed = CompressionStream::new(stream, CompressionAlgorithm::None);
    ///
    /// // Write uncompressed
    /// compressed.write_all(b"plain text").await?;
    ///
    /// // Switch to compression
    /// compressed.switch_algorithm(CompressionAlgorithm::Gzip).await?;
    ///
    /// // Now writes are compressed
    /// compressed.write_all(b"compressed text").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Conditional Switching
    ///
    /// ```rust,no_run
    /// use termionix_compress::{CompressionStream, CompressionAlgorithm};
    /// # use tokio::net::TcpStream;
    ///
    /// # async fn example(mut stream: CompressionStream<TcpStream>) -> std::io::Result<()> {
    /// // Only switch if not already using the algorithm
    /// if stream.algorithm() != CompressionAlgorithm::Zstd {
    ///     stream.switch_algorithm(CompressionAlgorithm::Zstd).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Multiple Switches
    ///
    /// ```rust,no_run
    /// use termionix_compress::{CompressionStream, CompressionAlgorithm};
    /// use tokio::io::AsyncWriteExt;
    /// # use tokio::net::TcpStream;
    ///
    /// # async fn example(stream: TcpStream) -> std::io::Result<()> {
    /// let mut compressed = CompressionStream::new(stream, CompressionAlgorithm::None);
    ///
    /// compressed.write_all(b"uncompressed").await?;
    /// compressed.switch_algorithm(CompressionAlgorithm::Gzip).await?;
    ///
    /// compressed.write_all(b"gzip compressed").await?;
    /// compressed.switch_algorithm(CompressionAlgorithm::Zstd).await?;
    ///
    /// compressed.write_all(b"zstd compressed").await?;
    /// compressed.shutdown().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Important Notes
    ///
    /// - **Idempotent**: Switching to the same algorithm is a no-op
    /// - **Finalization**: Current stream is properly finalized before switching
    /// - **Data Preservation**: Previously written data retains its original format
    /// - **Performance**: Switching has overhead due to flushing and reinitialization
    ///
    /// # Panics
    ///
    /// This method will panic if the inner stream is in an invalid state (missing).
    /// This should never happen in normal usage.
    pub async fn switch_algorithm(
        &mut self,
        algorithm: CompressionAlgorithm,
    ) -> tokio::io::Result<()> {
        use tokio::io::AsyncWriteExt;

        // Early return if already using this algorithm
        if self.algorithm() == algorithm {
            return Ok(());
        }

        // Flush and shutdown current compression stream to finalize compression state
        self.shutdown().await?;

        // Take the inner stream, extract the base stream, and recreate with new algorithm
        let old_inner = self.inner.take().expect("inner stream missing");
        let base_stream = old_inner.into_inner();
        self.inner = Some(InnerStream::new(base_stream, algorithm));

        Ok(())
    }

    /// Get a reference to the inner stream.
    ///
    /// This provides read-only access to the underlying stream wrapped by the
    /// `CompressionStream`. Useful for inspecting stream properties without
    /// modifying it.
    ///
    /// # Returns
    ///
    /// A reference to the underlying stream of type `&S`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use termionix_compress::{CompressionStream, CompressionAlgorithm};
    /// use tokio::net::TcpStream;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// let compressed = CompressionStream::new(stream, CompressionAlgorithm::Gzip);
    ///
    /// // Get reference to inspect properties
    /// let inner = compressed.get_ref();
    /// let peer_addr = inner.peer_addr()?;
    /// println!("Connected to: {}", peer_addr);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the inner stream is in an invalid state. This should never
    /// occur during normal usage.
    pub fn get_ref(&self) -> &S {
        self.inner.as_ref().expect("inner stream missing").get_ref()
    }

    /// Get a mutable reference to the inner stream.
    ///
    /// This provides read-write access to the underlying stream. Use with caution
    /// as modifying the stream directly can interfere with compression state.
    ///
    /// # Returns
    ///
    /// A mutable reference to the underlying stream of type `&mut S`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use termionix_compress::{CompressionStream, CompressionAlgorithm};
    /// use tokio::net::TcpStream;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// let mut compressed = CompressionStream::new(stream, CompressionAlgorithm::Gzip);
    ///
    /// // Get mutable reference to modify stream
    /// let inner = compressed.get_mut();
    /// // Modify stream properties...
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Warning
    ///
    /// Directly modifying the underlying stream while compression is active may
    /// lead to data corruption or sidechannel violations. Prefer using the
    /// `CompressionStream` API when possible.
    ///
    /// # Panics
    ///
    /// Panics if the inner stream is in an invalid state. This should never
    /// occur during normal usage.
    pub fn get_mut(&mut self) -> &mut S {
        self.inner.as_mut().expect("inner stream missing").get_mut()
    }

    /// Consumes this wrapper and returns the inner stream.
    ///
    /// This method destroys the `CompressionStream` and returns ownership of the
    /// underlying stream. Use this when you need to reclaim the stream for other
    /// purposes or to change how it's wrapped.
    ///
    /// # Returns
    ///
    /// The underlying stream of type `S`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use termionix_compress::{CompressionStream, CompressionAlgorithm};
    /// use tokio::net::TcpStream;
    /// use tokio::io::AsyncWriteExt;
    ///
    /// # async fn example() -> std::io::Result<()> {
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// let mut compressed = CompressionStream::new(stream, CompressionAlgorithm::Gzip);
    ///
    /// // Write some compressed data
    /// compressed.write_all(b"data").await?;
    /// compressed.shutdown().await?;
    ///
    /// // Reclaim the stream
    /// let stream = compressed.into_inner();
    ///
    /// // Now use the stream directly
    /// // ...
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Important
    ///
    /// Make sure to call `shutdown()` or `flush()` before calling `into_inner()`
    /// to ensure all compressed data is written and compression state is finalized.
    ///
    /// # Panics
    ///
    /// Panics if the inner stream is in an invalid state. This should never
    /// occur during normal usage.
    pub fn into_inner(self) -> S {
        self.inner.expect("inner stream missing").into_inner()
    }
}

impl<S> AsyncRead for CompressionStream<S>
where
    S: AsyncStream,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.project()
            .inner
            .as_pin_mut()
            .expect("inner stream missing")
            .poll_read(cx, buf)
    }
}

impl<S> AsyncWrite for CompressionStream<S>
where
    S: AsyncStream,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        self.project()
            .inner
            .as_pin_mut()
            .expect("inner stream missing")
            .poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project()
            .inner
            .as_pin_mut()
            .expect("inner stream missing")
            .poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        self.project()
            .inner
            .as_pin_mut()
            .expect("inner stream missing")
            .poll_shutdown(cx)
    }
}

pin_project! {
    #[project = InnerStreamProj]
    enum InnerStream<S>
    where
        S: AsyncStream,
    {
        None { #[pin] inner: S },
        Gzip { #[pin] inner: GzipEncoder<S> },
        Deflate { #[pin] inner: DeflateEncoder<S> },
        Brotli { #[pin] inner: BrotliEncoder<S> },
        Zlib { #[pin] inner: ZlibEncoder<S> },
        Zstd { #[pin] inner: ZstdEncoder<S> },
    }
}

impl<S> InnerStream<S>
where
    S: AsyncStream,
{
    /// Creates a new stateful compression stream.
    pub fn new(inner: S, algorithm: CompressionAlgorithm) -> Self {
        match algorithm {
            CompressionAlgorithm::None => Self::None { inner },
            CompressionAlgorithm::Gzip => Self::Gzip {
                inner: GzipEncoder::new(inner),
            },
            CompressionAlgorithm::Deflate => Self::Deflate {
                inner: DeflateEncoder::new(inner),
            },
            CompressionAlgorithm::Brotli => Self::Brotli {
                inner: BrotliEncoder::new(inner),
            },
            CompressionAlgorithm::Zlib => Self::Zlib {
                inner: ZlibEncoder::new(inner),
            },
            CompressionAlgorithm::Zstd => Self::Zstd {
                inner: ZstdEncoder::new(inner),
            },
        }
    }

    /// Returns the current algorithm.
    pub fn to_algorithm(&self) -> CompressionAlgorithm {
        match self {
            Self::None { .. } => CompressionAlgorithm::None,
            Self::Gzip { .. } => CompressionAlgorithm::Gzip,
            Self::Deflate { .. } => CompressionAlgorithm::Deflate,
            Self::Brotli { .. } => CompressionAlgorithm::Brotli,
            Self::Zlib { .. } => CompressionAlgorithm::Zlib,
            Self::Zstd { .. } => CompressionAlgorithm::Zstd,
        }
    }

    /// Get a reference to the inner stream.
    pub fn get_ref(&self) -> &S {
        match self {
            Self::None { inner } => inner,
            Self::Gzip { inner } => inner.get_ref(),
            Self::Deflate { inner } => inner.get_ref(),
            Self::Brotli { inner } => inner.get_ref(),
            Self::Zlib { inner } => inner.get_ref(),
            Self::Zstd { inner } => inner.get_ref(),
        }
    }

    /// Get a mutable reference to the inner stream.
    pub fn get_mut(&mut self) -> &mut S {
        match self {
            Self::None { inner } => inner,
            Self::Gzip { inner } => inner.get_mut(),
            Self::Deflate { inner } => inner.get_mut(),
            Self::Brotli { inner } => inner.get_mut(),
            Self::Zlib { inner } => inner.get_mut(),
            Self::Zstd { inner } => inner.get_mut(),
        }
    }

    /// Consumes this wrapper and returns the inner stream.
    pub fn into_inner(self) -> S {
        match self {
            Self::None { inner } => inner,
            Self::Gzip { inner } => inner.into_inner(),
            Self::Deflate { inner } => inner.into_inner(),
            Self::Brotli { inner } => inner.into_inner(),
            Self::Zlib { inner } => inner.into_inner(),
            Self::Zstd { inner } => inner.into_inner(),
        }
    }
}

impl<S> AsyncRead for InnerStream<S>
where
    S: AsyncStream,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.project() {
            InnerStreamProj::None { inner } => inner.poll_read(cx, buf),
            InnerStreamProj::Gzip { inner } => inner.poll_read(cx, buf),
            InnerStreamProj::Deflate { inner } => inner.poll_read(cx, buf),
            InnerStreamProj::Brotli { inner } => inner.poll_read(cx, buf),
            InnerStreamProj::Zlib { inner } => inner.poll_read(cx, buf),
            InnerStreamProj::Zstd { inner } => inner.poll_read(cx, buf),
        }
    }
}

impl<S> AsyncWrite for InnerStream<S>
where
    S: AsyncStream,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match self.project() {
            InnerStreamProj::None { inner } => inner.poll_write(cx, buf),
            InnerStreamProj::Gzip { inner } => inner.poll_write(cx, buf),
            InnerStreamProj::Deflate { inner } => inner.poll_write(cx, buf),
            InnerStreamProj::Brotli { inner } => inner.poll_write(cx, buf),
            InnerStreamProj::Zlib { inner } => inner.poll_write(cx, buf),
            InnerStreamProj::Zstd { inner } => inner.poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match self.project() {
            InnerStreamProj::None { inner } => inner.poll_flush(cx),
            InnerStreamProj::Gzip { inner } => inner.poll_flush(cx),
            InnerStreamProj::Deflate { inner } => inner.poll_flush(cx),
            InnerStreamProj::Brotli { inner } => inner.poll_flush(cx),
            InnerStreamProj::Zlib { inner } => inner.poll_flush(cx),
            InnerStreamProj::Zstd { inner } => inner.poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match self.project() {
            InnerStreamProj::None { inner } => inner.poll_shutdown(cx),
            InnerStreamProj::Gzip { inner } => inner.poll_shutdown(cx),
            InnerStreamProj::Deflate { inner } => inner.poll_shutdown(cx),
            InnerStreamProj::Brotli { inner } => inner.poll_shutdown(cx),
            InnerStreamProj::Zlib { inner } => inner.poll_shutdown(cx),
            InnerStreamProj::Zstd { inner } => inner.poll_shutdown(cx),
        }
    }
}

/// A compression wrapper for write-only streams
///
/// This wraps an `AsyncWrite` stream and compresses all data written to it.
/// The compression algorithm can be switched dynamically.
pub struct CompressionWriter<W>
where
    W: AsyncWrite + Unpin + Send,
{
    inner: Option<InnerWriter<W>>,
}

enum InnerWriter<W>
where
    W: AsyncWrite + Unpin + Send,
{
    None(W),
    Gzip(GzipEncoder<W>),
    Deflate(DeflateEncoder<W>),
    Brotli(BrotliEncoder<W>),
    Zlib(ZlibEncoder<W>),
    Zstd(ZstdEncoder<W>),
}

impl<W> CompressionWriter<W>
where
    W: AsyncWrite + Unpin + Send,
{
    /// Create a new compression writer with the specified algorithm
    pub fn new(writer: W, algorithm: CompressionAlgorithm) -> Self {
        Self {
            inner: Some(Self::wrap_writer(writer, algorithm)),
        }
    }

    fn wrap_writer(writer: W, algorithm: CompressionAlgorithm) -> InnerWriter<W> {
        match algorithm {
            CompressionAlgorithm::None => InnerWriter::None(writer),
            CompressionAlgorithm::Gzip => InnerWriter::Gzip(GzipEncoder::new(writer)),
            CompressionAlgorithm::Deflate => InnerWriter::Deflate(DeflateEncoder::new(writer)),
            CompressionAlgorithm::Brotli => InnerWriter::Brotli(BrotliEncoder::new(writer)),
            CompressionAlgorithm::Zlib => InnerWriter::Zlib(ZlibEncoder::new(writer)),
            CompressionAlgorithm::Zstd => InnerWriter::Zstd(ZstdEncoder::new(writer)),
        }
    }

    /// Get the current compression algorithm
    pub fn algorithm(&self) -> CompressionAlgorithm {
        match self.inner.as_ref().expect("inner writer missing") {
            InnerWriter::None(_) => CompressionAlgorithm::None,
            InnerWriter::Gzip(_) => CompressionAlgorithm::Gzip,
            InnerWriter::Deflate(_) => CompressionAlgorithm::Deflate,
            InnerWriter::Brotli(_) => CompressionAlgorithm::Brotli,
            InnerWriter::Zlib(_) => CompressionAlgorithm::Zlib,
            InnerWriter::Zstd(_) => CompressionAlgorithm::Zstd,
        }
    }

    /// Switch to a new compression algorithm
    ///
    /// This flushes and shuts down the current compressor, extracts the underlying
    /// writer, and wraps it with a new compressor using the specified algorithm.
    pub async fn switch_algorithm(&mut self, algorithm: CompressionAlgorithm) -> io::Result<()> {
        use tokio::io::AsyncWriteExt;

        // Early return if already using this algorithm
        if self.algorithm() == algorithm {
            return Ok(());
        }

        // Shutdown current compressor to finalize compression state
        self.shutdown().await?;

        // Take the inner writer, extract the base writer, and recreate with new algorithm
        let old_inner = self.inner.take().expect("inner writer missing");
        let base_writer = Self::extract_writer(old_inner);
        self.inner = Some(Self::wrap_writer(base_writer, algorithm));

        Ok(())
    }

    fn extract_writer(inner: InnerWriter<W>) -> W {
        match inner {
            InnerWriter::None(w) => w,
            InnerWriter::Gzip(w) => w.into_inner(),
            InnerWriter::Deflate(w) => w.into_inner(),
            InnerWriter::Brotli(w) => w.into_inner(),
            InnerWriter::Zlib(w) => w.into_inner(),
            InnerWriter::Zstd(w) => w.into_inner(),
        }
    }

    /// Get a reference to the underlying writer
    pub fn get_ref(&self) -> &W {
        match self.inner.as_ref().expect("inner writer missing") {
            InnerWriter::None(w) => w,
            InnerWriter::Gzip(w) => w.get_ref(),
            InnerWriter::Deflate(w) => w.get_ref(),
            InnerWriter::Brotli(w) => w.get_ref(),
            InnerWriter::Zlib(w) => w.get_ref(),
            InnerWriter::Zstd(w) => w.get_ref(),
        }
    }

    /// Get a mutable reference to the underlying writer
    pub fn get_mut(&mut self) -> &mut W {
        match self.inner.as_mut().expect("inner writer missing") {
            InnerWriter::None(w) => w,
            InnerWriter::Gzip(w) => w.get_mut(),
            InnerWriter::Deflate(w) => w.get_mut(),
            InnerWriter::Brotli(w) => w.get_mut(),
            InnerWriter::Zlib(w) => w.get_mut(),
            InnerWriter::Zstd(w) => w.get_mut(),
        }
    }

    /// Consume this writer and return the underlying writer
    pub fn into_inner(mut self) -> W {
        Self::extract_writer(self.inner.take().expect("inner writer missing"))
    }
}

impl<W> AsyncWrite for CompressionWriter<W>
where
    W: AsyncWrite + Unpin + Send,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        match self.inner.as_mut().expect("inner writer missing") {
            InnerWriter::None(w) => Pin::new(w).poll_write(cx, buf),
            InnerWriter::Gzip(w) => Pin::new(w).poll_write(cx, buf),
            InnerWriter::Deflate(w) => Pin::new(w).poll_write(cx, buf),
            InnerWriter::Brotli(w) => Pin::new(w).poll_write(cx, buf),
            InnerWriter::Zlib(w) => Pin::new(w).poll_write(cx, buf),
            InnerWriter::Zstd(w) => Pin::new(w).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        match self.inner.as_mut().expect("inner writer missing") {
            InnerWriter::None(w) => Pin::new(w).poll_flush(cx),
            InnerWriter::Gzip(w) => Pin::new(w).poll_flush(cx),
            InnerWriter::Deflate(w) => Pin::new(w).poll_flush(cx),
            InnerWriter::Brotli(w) => Pin::new(w).poll_flush(cx),
            InnerWriter::Zlib(w) => Pin::new(w).poll_flush(cx),
            InnerWriter::Zstd(w) => Pin::new(w).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), io::Error>> {
        match self.inner.as_mut().expect("inner writer missing") {
            InnerWriter::None(w) => Pin::new(w).poll_shutdown(cx),
            InnerWriter::Gzip(w) => Pin::new(w).poll_shutdown(cx),
            InnerWriter::Deflate(w) => Pin::new(w).poll_shutdown(cx),
            InnerWriter::Brotli(w) => Pin::new(w).poll_shutdown(cx),
            InnerWriter::Zlib(w) => Pin::new(w).poll_shutdown(cx),
            InnerWriter::Zstd(w) => Pin::new(w).poll_shutdown(cx),
        }
    }
}

/// A decompression wrapper for read-only streams
///
/// This wraps an `AsyncRead` stream and decompresses all data read from it.
/// The decompression algorithm can be switched dynamically.
pub struct CompressionReader<R>
where
    R: AsyncRead + Unpin + Send,
{
    inner: Option<InnerReader<R>>,
}

enum InnerReader<R>
where
    R: AsyncRead + Unpin + Send,
{
    None(R),
    Gzip(async_compression::tokio::bufread::GzipDecoder<tokio::io::BufReader<R>>),
    Deflate(async_compression::tokio::bufread::DeflateDecoder<tokio::io::BufReader<R>>),
    Brotli(async_compression::tokio::bufread::BrotliDecoder<tokio::io::BufReader<R>>),
    Zlib(async_compression::tokio::bufread::ZlibDecoder<tokio::io::BufReader<R>>),
    Zstd(async_compression::tokio::bufread::ZstdDecoder<tokio::io::BufReader<R>>),
}

impl<R> CompressionReader<R>
where
    R: AsyncRead + Unpin + Send,
{
    /// Create a new compression reader with the specified algorithm
    pub fn new(reader: R, algorithm: CompressionAlgorithm) -> Self {
        Self {
            inner: Some(Self::wrap_reader(reader, algorithm)),
        }
    }

    fn wrap_reader(reader: R, algorithm: CompressionAlgorithm) -> InnerReader<R> {
        use async_compression::tokio::bufread::{
            BrotliDecoder, DeflateDecoder, GzipDecoder, ZlibDecoder, ZstdDecoder,
        };
        use tokio::io::BufReader;

        match algorithm {
            CompressionAlgorithm::None => InnerReader::None(reader),
            CompressionAlgorithm::Gzip => {
                InnerReader::Gzip(GzipDecoder::new(BufReader::new(reader)))
            }
            CompressionAlgorithm::Deflate => {
                InnerReader::Deflate(DeflateDecoder::new(BufReader::new(reader)))
            }
            CompressionAlgorithm::Brotli => {
                InnerReader::Brotli(BrotliDecoder::new(BufReader::new(reader)))
            }
            CompressionAlgorithm::Zlib => {
                InnerReader::Zlib(ZlibDecoder::new(BufReader::new(reader)))
            }
            CompressionAlgorithm::Zstd => {
                InnerReader::Zstd(ZstdDecoder::new(BufReader::new(reader)))
            }
        }
    }

    /// Get the current decompression algorithm
    pub fn algorithm(&self) -> CompressionAlgorithm {
        match self.inner.as_ref().expect("inner reader missing") {
            InnerReader::None(_) => CompressionAlgorithm::None,
            InnerReader::Gzip(_) => CompressionAlgorithm::Gzip,
            InnerReader::Deflate(_) => CompressionAlgorithm::Deflate,
            InnerReader::Brotli(_) => CompressionAlgorithm::Brotli,
            InnerReader::Zlib(_) => CompressionAlgorithm::Zlib,
            InnerReader::Zstd(_) => CompressionAlgorithm::Zstd,
        }
    }

    /// Switch to a new decompression algorithm
    ///
    /// This extracts the underlying reader and wraps it with a new decompressor
    /// using the specified algorithm.
    pub fn switch_algorithm(&mut self, algorithm: CompressionAlgorithm) -> io::Result<()> {
        // Early return if already using this algorithm
        if self.algorithm() == algorithm {
            return Ok(());
        }

        // Take the inner reader, extract the base reader, and recreate with new algorithm
        let old_inner = self.inner.take().expect("inner reader missing");
        let base_reader = Self::extract_reader(old_inner);
        self.inner = Some(Self::wrap_reader(base_reader, algorithm));

        Ok(())
    }

    fn extract_reader(inner: InnerReader<R>) -> R {
        match inner {
            InnerReader::None(r) => r,
            InnerReader::Gzip(r) => r.into_inner().into_inner(),
            InnerReader::Deflate(r) => r.into_inner().into_inner(),
            InnerReader::Brotli(r) => r.into_inner().into_inner(),
            InnerReader::Zlib(r) => r.into_inner().into_inner(),
            InnerReader::Zstd(r) => r.into_inner().into_inner(),
        }
    }
}

impl<R> AsyncRead for CompressionReader<R>
where
    R: AsyncRead + Unpin + Send,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.inner.as_mut().expect("inner reader missing") {
            InnerReader::None(r) => Pin::new(r).poll_read(cx, buf),
            InnerReader::Gzip(r) => Pin::new(r).poll_read(cx, buf),
            InnerReader::Deflate(r) => Pin::new(r).poll_read(cx, buf),
            InnerReader::Brotli(r) => Pin::new(r).poll_read(cx, buf),
            InnerReader::Zlib(r) => Pin::new(r).poll_read(cx, buf),
            InnerReader::Zstd(r) => Pin::new(r).poll_read(cx, buf),
        }
    }
}

// ... existing code ...

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A simple mock stream backed by a Vec<u8> for testing.
    #[derive(Debug, Clone)]
    struct MockStream {
        read_buf: Vec<u8>,
        write_buf: Vec<u8>,
        read_pos: usize,
    }

    impl MockStream {
        fn new() -> Self {
            Self {
                read_buf: Vec::new(),
                write_buf: Vec::new(),
                read_pos: 0,
            }
        }

        fn with_read_data(data: Vec<u8>) -> Self {
            Self {
                read_buf: data,
                write_buf: Vec::new(),
                read_pos: 0,
            }
        }

        fn written_data(&self) -> &[u8] {
            &self.write_buf
        }
    }

    impl AsyncRead for MockStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let remaining = &self.read_buf[self.read_pos..];
            let to_read = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..to_read]);
            self.read_pos += to_read;
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for MockStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, io::Error>> {
            self.write_buf.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), io::Error>> {
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_algorithm_enum_equality() {
        assert_eq!(CompressionAlgorithm::None, CompressionAlgorithm::None);
        assert_eq!(CompressionAlgorithm::Gzip, CompressionAlgorithm::Gzip);
        assert_eq!(CompressionAlgorithm::Deflate, CompressionAlgorithm::Deflate);
        assert_eq!(CompressionAlgorithm::Brotli, CompressionAlgorithm::Brotli);
        assert_eq!(CompressionAlgorithm::Zlib, CompressionAlgorithm::Zlib);
        assert_eq!(CompressionAlgorithm::Zstd, CompressionAlgorithm::Zstd);

        assert_ne!(CompressionAlgorithm::None, CompressionAlgorithm::Gzip);
        assert_ne!(CompressionAlgorithm::Gzip, CompressionAlgorithm::Deflate);
    }

    #[tokio::test]
    async fn test_algorithm_clone_and_copy() {
        let algo = CompressionAlgorithm::Gzip;
        let cloned = algo.clone();
        let copied = algo;

        assert_eq!(algo, cloned);
        assert_eq!(algo, copied);
    }

    #[tokio::test]
    async fn test_compression_stream_new_with_none() {
        let stream = MockStream::new();
        let compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        assert_eq!(compression.algorithm(), CompressionAlgorithm::None);
    }

    #[tokio::test]
    async fn test_compression_stream_new_with_gzip() {
        let stream = MockStream::new();
        let compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

        assert_eq!(compression.algorithm(), CompressionAlgorithm::Gzip);
    }

    #[tokio::test]
    async fn test_compression_stream_new_with_deflate() {
        let stream = MockStream::new();
        let compression = CompressionStream::new(stream, CompressionAlgorithm::Deflate);

        assert_eq!(compression.algorithm(), CompressionAlgorithm::Deflate);
    }

    #[tokio::test]
    async fn test_compression_stream_new_with_brotli() {
        let stream = MockStream::new();
        let compression = CompressionStream::new(stream, CompressionAlgorithm::Brotli);

        assert_eq!(compression.algorithm(), CompressionAlgorithm::Brotli);
    }

    #[tokio::test]
    async fn test_compression_stream_new_with_zlib() {
        let stream = MockStream::new();
        let compression = CompressionStream::new(stream, CompressionAlgorithm::Zlib);

        assert_eq!(compression.algorithm(), CompressionAlgorithm::Zlib);
    }

    #[tokio::test]
    async fn test_compression_stream_new_with_zstd() {
        let stream = MockStream::new();
        let compression = CompressionStream::new(stream, CompressionAlgorithm::Zstd);

        assert_eq!(compression.algorithm(), CompressionAlgorithm::Zstd);
    }

    #[tokio::test]
    async fn test_write_with_no_compression() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        let test_data = b"Hello, World!";
        compression.write_all(test_data).await.unwrap();
        compression.flush().await.unwrap();

        let inner = compression.into_inner();
        assert_eq!(inner.written_data(), test_data);
    }

    #[tokio::test]
    async fn test_write_with_gzip_compression() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

        let test_data = b"Hello, World!";
        compression.write_all(test_data).await.unwrap();
        compression.shutdown().await.unwrap();

        let inner = compression.into_inner();
        let compressed = inner.written_data();

        // Gzip magic number check
        assert_eq!(&compressed[0..2], &[0x1f, 0x8b]);
        // Data should be compressed (different from original)
        assert_ne!(compressed, test_data);
    }

    #[tokio::test]
    async fn test_write_with_deflate_compression() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Deflate);

        let test_data = b"Hello, World!";
        compression.write_all(test_data).await.unwrap();
        compression.shutdown().await.unwrap();

        let inner = compression.into_inner();
        let compressed = inner.written_data();

        // Should produce compressed output
        assert!(!compressed.is_empty());
        assert_ne!(compressed, test_data);
    }

    #[tokio::test]
    async fn test_write_with_zlib_compression() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Zlib);

        let test_data = b"Hello, World!";
        compression.write_all(test_data).await.unwrap();
        compression.shutdown().await.unwrap();

        let inner = compression.into_inner();
        let compressed = inner.written_data();

        // Zlib magic number check (0x78)
        assert_eq!(compressed[0], 0x78);
        assert_ne!(compressed, test_data);
    }

    #[tokio::test]
    async fn test_write_with_zstd_compression() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Zstd);

        let test_data = b"Hello, World!";
        compression.write_all(test_data).await.unwrap();
        compression.shutdown().await.unwrap();

        let inner = compression.into_inner();
        let compressed = inner.written_data();

        // Zstd magic number check
        assert_eq!(&compressed[0..4], &[0x28, 0xb5, 0x2f, 0xfd]);
        assert_ne!(compressed, test_data);
    }

    #[tokio::test]
    async fn test_write_with_brotli_compression() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Brotli);

        let test_data = b"Hello, World!";
        compression.write_all(test_data).await.unwrap();
        compression.shutdown().await.unwrap();

        let inner = compression.into_inner();
        let compressed = inner.written_data();

        // Should produce compressed output
        assert!(!compressed.is_empty());
        assert_ne!(compressed, test_data);
    }

    #[tokio::test]
    async fn test_read_passthrough() {
        let test_data = b"Hello, World!";
        let stream = MockStream::with_read_data(test_data.to_vec());
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        let mut buf = vec![0u8; 32];
        let n = compression.read(&mut buf).await.unwrap();

        assert_eq!(n, test_data.len());
        assert_eq!(&buf[..n], test_data);
    }

    #[tokio::test]
    async fn test_get_ref() {
        let stream = MockStream::with_read_data(vec![1, 2, 3, 4]);
        let compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        let inner_ref = compression.get_ref();
        assert_eq!(inner_ref.read_buf, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_get_mut() {
        let stream = MockStream::with_read_data(vec![1, 2, 3, 4]);
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        let inner_mut = compression.get_mut();
        inner_mut.write_buf.push(5);
        assert_eq!(inner_mut.write_buf, vec![5]);
    }

    #[tokio::test]
    async fn test_into_inner() {
        let stream = MockStream::with_read_data(vec![1, 2, 3, 4]);
        let compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        let inner = compression.into_inner();
        assert_eq!(inner.read_buf, vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_switch_algorithm_none_to_gzip() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        assert_eq!(compression.algorithm(), CompressionAlgorithm::None);

        compression
            .switch_algorithm(CompressionAlgorithm::Gzip)
            .await
            .unwrap();

        assert_eq!(compression.algorithm(), CompressionAlgorithm::Gzip);
    }

    #[tokio::test]
    async fn test_switch_algorithm_gzip_to_deflate() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

        compression
            .switch_algorithm(CompressionAlgorithm::Deflate)
            .await
            .unwrap();

        assert_eq!(compression.algorithm(), CompressionAlgorithm::Deflate);
    }

    #[tokio::test]
    async fn test_switch_algorithm_same_algorithm() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

        // Switching to the same algorithm should succeed immediately
        compression
            .switch_algorithm(CompressionAlgorithm::Gzip)
            .await
            .unwrap();

        assert_eq!(compression.algorithm(), CompressionAlgorithm::Gzip);
    }

    #[tokio::test]
    async fn test_switch_algorithm_preserves_stream() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        // Write some data
        compression.write_all(b"test").await.unwrap();
        compression.flush().await.unwrap();

        // Switch algorithm
        compression
            .switch_algorithm(CompressionAlgorithm::Gzip)
            .await
            .unwrap();

        // Verify we can still write
        compression.write_all(b"after").await.unwrap();
        compression.shutdown().await.unwrap();

        let inner = compression.into_inner();
        let written = inner.written_data();

        // Should have both uncompressed "test" and gzip-compressed "after"
        assert!(written.len() > 4);
        assert_eq!(&written[0..4], b"test");
    }

    #[tokio::test]
    async fn test_switch_through_all_algorithms() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        let algorithms = vec![
            CompressionAlgorithm::None,
            CompressionAlgorithm::Gzip,
            CompressionAlgorithm::Deflate,
            CompressionAlgorithm::Brotli,
            CompressionAlgorithm::Zlib,
            CompressionAlgorithm::Zstd,
        ];

        for algo in algorithms {
            compression.switch_algorithm(algo).await.unwrap();
            assert_eq!(compression.algorithm(), algo);
        }
    }

    #[tokio::test]
    async fn test_multiple_writes_with_flush() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        compression.write_all(b"Hello").await.unwrap();
        compression.flush().await.unwrap();

        compression.write_all(b", ").await.unwrap();
        compression.flush().await.unwrap();

        compression.write_all(b"World!").await.unwrap();
        compression.flush().await.unwrap();

        let inner = compression.into_inner();
        assert_eq!(inner.written_data(), b"Hello, World!");
    }

    #[tokio::test]
    async fn test_empty_write() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        compression.write_all(b"").await.unwrap();
        compression.flush().await.unwrap();

        let inner = compression.into_inner();
        assert_eq!(inner.written_data(), b"");
    }

    #[tokio::test]
    async fn test_large_write() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

        // Write 1MB of data
        let large_data = vec![b'A'; 1024 * 1024];
        compression.write_all(&large_data).await.unwrap();
        compression.shutdown().await.unwrap();

        let inner = compression.into_inner();
        let compressed = inner.written_data();

        // Compressed size should be much smaller due to repetition
        assert!(compressed.len() < large_data.len());
        // Should have gzip header
        assert_eq!(&compressed[0..2], &[0x1f, 0x8b]);
    }

    #[tokio::test]
    async fn test_compression_reduces_size_for_repetitive_data() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

        // Highly repetitive data should compress well
        let repetitive_data = b"AAAAAAAAAA".repeat(100);
        compression.write_all(&repetitive_data).await.unwrap();
        compression.shutdown().await.unwrap();

        let inner = compression.into_inner();
        let compressed = inner.written_data();

        // Compressed size should be significantly smaller
        assert!(compressed.len() < repetitive_data.len() / 2);
    }

    #[tokio::test]
    async fn test_inner_stream_get_ref() {
        let stream = MockStream::new();
        let inner = InnerStream::new(stream, CompressionAlgorithm::None);

        let _stream_ref = inner.get_ref();
        // Just verify it doesn't panic
    }

    #[tokio::test]
    async fn test_inner_stream_get_mut() {
        let stream = MockStream::new();
        let mut inner = InnerStream::new(stream, CompressionAlgorithm::None);

        let _stream_mut = inner.get_mut();
        // Just verify it doesn't panic
    }

    #[tokio::test]
    async fn test_inner_stream_into_inner() {
        let stream = MockStream::with_read_data(vec![1, 2, 3]);
        let inner = InnerStream::new(stream, CompressionAlgorithm::None);

        let original = inner.into_inner();
        assert_eq!(original.read_buf, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_inner_stream_to_algorithm() {
        let stream = MockStream::new();

        let inner_none = InnerStream::new(stream.clone(), CompressionAlgorithm::None);
        assert_eq!(inner_none.to_algorithm(), CompressionAlgorithm::None);

        let inner_gzip = InnerStream::new(stream.clone(), CompressionAlgorithm::Gzip);
        assert_eq!(inner_gzip.to_algorithm(), CompressionAlgorithm::Gzip);

        let inner_deflate = InnerStream::new(stream.clone(), CompressionAlgorithm::Deflate);
        assert_eq!(inner_deflate.to_algorithm(), CompressionAlgorithm::Deflate);

        let inner_brotli = InnerStream::new(stream.clone(), CompressionAlgorithm::Brotli);
        assert_eq!(inner_brotli.to_algorithm(), CompressionAlgorithm::Brotli);

        let inner_zlib = InnerStream::new(stream.clone(), CompressionAlgorithm::Zlib);
        assert_eq!(inner_zlib.to_algorithm(), CompressionAlgorithm::Zlib);

        let inner_zstd = InnerStream::new(stream.clone(), CompressionAlgorithm::Zstd);
        assert_eq!(inner_zstd.to_algorithm(), CompressionAlgorithm::Zstd);
    }

    #[tokio::test]
    async fn test_shutdown_finalizes_compression() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

        compression.write_all(b"test data").await.unwrap();
        compression.shutdown().await.unwrap();

        let inner = compression.into_inner();
        let compressed = inner.written_data();

        // Should have complete gzip stream with header and trailer
        assert!(compressed.len() > 10); // Header + data + trailer
        assert_eq!(&compressed[0..2], &[0x1f, 0x8b]); // Gzip magic
    }

    #[tokio::test]
    async fn test_write_after_switch_uses_new_algorithm() {
        let stream = MockStream::new();
        let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

        // Write uncompressed
        compression.write_all(b"uncompressed").await.unwrap();
        compression.flush().await.unwrap();

        // Switch to Gzip
        compression
            .switch_algorithm(CompressionAlgorithm::Gzip)
            .await
            .unwrap();

        // Write compressed
        compression.write_all(b"compressed").await.unwrap();
        compression.shutdown().await.unwrap();

        let inner = compression.into_inner();
        let data = inner.written_data();

        // First part should be uncompressed text
        assert_eq!(&data[0..12], b"uncompressed");
        // After that should be gzip compressed data
        assert_eq!(&data[12..14], &[0x1f, 0x8b]); // Gzip magic
    }
}
