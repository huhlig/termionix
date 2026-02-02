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

//! Unit tests for termionix-compress

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use termionix_compress::{CompressionAlgorithm, CompressionStream};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

// ============================================================================
// Mock Stream for Testing
// ============================================================================

#[derive(Debug, Clone)]
struct MockStream {
    read_buf: Vec<u8>,
    write_buf: Vec<u8>,
    read_pos: usize,
    read_error: Option<io::ErrorKind>,
    write_error: Option<io::ErrorKind>,
}

impl MockStream {
    fn new() -> Self {
        Self {
            read_buf: Vec::new(),
            write_buf: Vec::new(),
            read_pos: 0,
            read_error: None,
            write_error: None,
        }
    }

    fn with_read_data(data: Vec<u8>) -> Self {
        Self {
            read_buf: data,
            write_buf: Vec::new(),
            read_pos: 0,
            read_error: None,
            write_error: None,
        }
    }

    fn with_read_error(error: io::ErrorKind) -> Self {
        Self {
            read_buf: Vec::new(),
            write_buf: Vec::new(),
            read_pos: 0,
            read_error: Some(error),
            write_error: None,
        }
    }

    fn with_write_error(error: io::ErrorKind) -> Self {
        Self {
            read_buf: Vec::new(),
            write_buf: Vec::new(),
            read_pos: 0,
            read_error: None,
            write_error: Some(error),
        }
    }

    fn written_data(&self) -> &[u8] {
        &self.write_buf
    }

    fn bytes_written(&self) -> usize {
        self.write_buf.len()
    }

    fn bytes_read(&self) -> usize {
        self.read_pos
    }
}

impl AsyncRead for MockStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if let Some(error) = self.read_error {
            return Poll::Ready(Err(io::Error::new(error, "mock read error")));
        }

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
        if let Some(error) = self.write_error {
            return Poll::Ready(Err(io::Error::new(error, "mock write error")));
        }

        self.write_buf.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }
}

// ============================================================================
// Algorithm Tests
// ============================================================================

#[test]
fn test_algorithm_equality() {
    assert_eq!(CompressionAlgorithm::None, CompressionAlgorithm::None);
    assert_eq!(CompressionAlgorithm::Gzip, CompressionAlgorithm::Gzip);
    assert_eq!(CompressionAlgorithm::Deflate, CompressionAlgorithm::Deflate);
    assert_eq!(CompressionAlgorithm::Brotli, CompressionAlgorithm::Brotli);
    assert_eq!(CompressionAlgorithm::Zlib, CompressionAlgorithm::Zlib);
    assert_eq!(CompressionAlgorithm::Zstd, CompressionAlgorithm::Zstd);
}

#[test]
fn test_algorithm_inequality() {
    assert_ne!(CompressionAlgorithm::None, CompressionAlgorithm::Gzip);
    assert_ne!(CompressionAlgorithm::Gzip, CompressionAlgorithm::Deflate);
    assert_ne!(CompressionAlgorithm::Deflate, CompressionAlgorithm::Brotli);
    assert_ne!(CompressionAlgorithm::Brotli, CompressionAlgorithm::Zlib);
    assert_ne!(CompressionAlgorithm::Zlib, CompressionAlgorithm::Zstd);
    assert_ne!(CompressionAlgorithm::Zstd, CompressionAlgorithm::None);
}

#[test]
fn test_algorithm_clone() {
    let algo = CompressionAlgorithm::Gzip;
    let cloned = algo.clone();
    assert_eq!(algo, cloned);
}

#[test]
fn test_algorithm_copy() {
    let algo = CompressionAlgorithm::Gzip;
    let copied = algo;
    assert_eq!(algo, copied);
}

#[test]
fn test_algorithm_debug() {
    assert_eq!(format!("{:?}", CompressionAlgorithm::None), "None");
    assert_eq!(format!("{:?}", CompressionAlgorithm::Gzip), "Gzip");
    assert_eq!(format!("{:?}", CompressionAlgorithm::Deflate), "Deflate");
    assert_eq!(format!("{:?}", CompressionAlgorithm::Brotli), "Brotli");
    assert_eq!(format!("{:?}", CompressionAlgorithm::Zlib), "Zlib");
    assert_eq!(format!("{:?}", CompressionAlgorithm::Zstd), "Zstd");
}

// ============================================================================
// CompressionStream Creation Tests
// ============================================================================

#[tokio::test]
async fn test_new_with_none() {
    let stream = MockStream::new();
    let compression = CompressionStream::new(stream, CompressionAlgorithm::None);
    assert_eq!(compression.algorithm(), CompressionAlgorithm::None);
}

#[tokio::test]
async fn test_new_with_gzip() {
    let stream = MockStream::new();
    let compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);
    assert_eq!(compression.algorithm(), CompressionAlgorithm::Gzip);
}

#[tokio::test]
async fn test_new_with_deflate() {
    let stream = MockStream::new();
    let compression = CompressionStream::new(stream, CompressionAlgorithm::Deflate);
    assert_eq!(compression.algorithm(), CompressionAlgorithm::Deflate);
}

#[tokio::test]
async fn test_new_with_brotli() {
    let stream = MockStream::new();
    let compression = CompressionStream::new(stream, CompressionAlgorithm::Brotli);
    assert_eq!(compression.algorithm(), CompressionAlgorithm::Brotli);
}

#[tokio::test]
async fn test_new_with_zlib() {
    let stream = MockStream::new();
    let compression = CompressionStream::new(stream, CompressionAlgorithm::Zlib);
    assert_eq!(compression.algorithm(), CompressionAlgorithm::Zlib);
}

#[tokio::test]
async fn test_new_with_zstd() {
    let stream = MockStream::new();
    let compression = CompressionStream::new(stream, CompressionAlgorithm::Zstd);
    assert_eq!(compression.algorithm(), CompressionAlgorithm::Zstd);
}

// ============================================================================
// Write Tests
// ============================================================================

#[tokio::test]
async fn test_write_no_compression() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    let data = b"Hello, World!";
    compression.write_all(data).await.unwrap();
    compression.flush().await.unwrap();

    let inner = compression.into_inner();
    assert_eq!(inner.written_data(), data);
}

#[tokio::test]
async fn test_write_empty_data() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    compression.write_all(b"").await.unwrap();
    compression.flush().await.unwrap();

    let inner = compression.into_inner();
    assert_eq!(inner.written_data(), b"");
}

#[tokio::test]
async fn test_write_single_byte() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    compression.write_all(b"A").await.unwrap();
    compression.flush().await.unwrap();

    let inner = compression.into_inner();
    assert_eq!(inner.written_data(), b"A");
}

#[tokio::test]
async fn test_multiple_writes() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    compression.write_all(b"Hello").await.unwrap();
    compression.write_all(b", ").await.unwrap();
    compression.write_all(b"World!").await.unwrap();
    compression.flush().await.unwrap();

    let inner = compression.into_inner();
    assert_eq!(inner.written_data(), b"Hello, World!");
}

#[tokio::test]
async fn test_write_with_gzip() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

    let data = b"Test data for compression";
    compression.write_all(data).await.unwrap();
    compression.shutdown().await.unwrap();

    let inner = compression.into_inner();
    let compressed = inner.written_data();

    // Verify gzip magic number
    assert_eq!(&compressed[0..2], &[0x1f, 0x8b]);
    assert_ne!(compressed, data);
}

#[tokio::test]
async fn test_write_with_deflate() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Deflate);

    let data = b"Test data for compression";
    compression.write_all(data).await.unwrap();
    compression.shutdown().await.unwrap();

    let inner = compression.into_inner();
    let compressed = inner.written_data();

    assert!(!compressed.is_empty());
    assert_ne!(compressed, data);
}

#[tokio::test]
async fn test_write_with_zlib() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Zlib);

    let data = b"Test data for compression";
    compression.write_all(data).await.unwrap();
    compression.shutdown().await.unwrap();

    let inner = compression.into_inner();
    let compressed = inner.written_data();

    // Verify zlib magic number
    assert_eq!(compressed[0], 0x78);
    assert_ne!(compressed, data);
}

#[tokio::test]
async fn test_write_with_zstd() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Zstd);

    let data = b"Test data for compression";
    compression.write_all(data).await.unwrap();
    compression.shutdown().await.unwrap();

    let inner = compression.into_inner();
    let compressed = inner.written_data();

    // Verify zstd magic number
    assert_eq!(&compressed[0..4], &[0x28, 0xb5, 0x2f, 0xfd]);
    assert_ne!(compressed, data);
}

#[tokio::test]
async fn test_write_with_brotli() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Brotli);

    let data = b"Test data for compression";
    compression.write_all(data).await.unwrap();
    compression.shutdown().await.unwrap();

    let inner = compression.into_inner();
    let compressed = inner.written_data();

    assert!(!compressed.is_empty());
    assert_ne!(compressed, data);
}

// ============================================================================
// Read Tests
// ============================================================================

#[tokio::test]
async fn test_read_passthrough() {
    let data = b"Hello, World!";
    let stream = MockStream::with_read_data(data.to_vec());
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    let mut buf = vec![0u8; 32];
    let n = compression.read(&mut buf).await.unwrap();

    assert_eq!(n, data.len());
    assert_eq!(&buf[..n], data);
}

#[tokio::test]
async fn test_read_empty() {
    let stream = MockStream::with_read_data(vec![]);
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    let mut buf = vec![0u8; 32];
    let n = compression.read(&mut buf).await.unwrap();

    assert_eq!(n, 0);
}

#[tokio::test]
async fn test_read_partial() {
    let data = b"Hello, World!";
    let stream = MockStream::with_read_data(data.to_vec());
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    let mut buf = vec![0u8; 5];
    let n = compression.read(&mut buf).await.unwrap();

    assert_eq!(n, 5);
    assert_eq!(&buf[..n], b"Hello");
}

#[tokio::test]
async fn test_read_multiple() {
    let data = b"Hello, World!";
    let stream = MockStream::with_read_data(data.to_vec());
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    let mut buf1 = vec![0u8; 5];
    let n1 = compression.read(&mut buf1).await.unwrap();
    assert_eq!(n1, 5);
    assert_eq!(&buf1[..n1], b"Hello");

    let mut buf2 = vec![0u8; 8];
    let n2 = compression.read(&mut buf2).await.unwrap();
    assert_eq!(n2, 8);
    assert_eq!(&buf2[..n2], b", World!");
}

// ============================================================================
// Algorithm Switching Tests
// ============================================================================

#[tokio::test]
async fn test_switch_none_to_gzip() {
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
async fn test_switch_gzip_to_deflate() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

    compression
        .switch_algorithm(CompressionAlgorithm::Deflate)
        .await
        .unwrap();
    assert_eq!(compression.algorithm(), CompressionAlgorithm::Deflate);
}

#[tokio::test]
async fn test_switch_same_algorithm() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

    compression
        .switch_algorithm(CompressionAlgorithm::Gzip)
        .await
        .unwrap();
    assert_eq!(compression.algorithm(), CompressionAlgorithm::Gzip);
}

#[tokio::test]
async fn test_switch_preserves_data() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    compression.write_all(b"before").await.unwrap();
    compression.flush().await.unwrap();

    compression
        .switch_algorithm(CompressionAlgorithm::Gzip)
        .await
        .unwrap();

    compression.write_all(b"after").await.unwrap();
    compression.shutdown().await.unwrap();

    let inner = compression.into_inner();
    let data = inner.written_data();

    assert_eq!(&data[0..6], b"before");
    assert_eq!(&data[6..8], &[0x1f, 0x8b]); // Gzip magic
}

#[tokio::test]
async fn test_switch_all_algorithms() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    let algorithms = vec![
        CompressionAlgorithm::None,
        CompressionAlgorithm::Gzip,
        CompressionAlgorithm::Deflate,
        CompressionAlgorithm::Brotli,
        CompressionAlgorithm::Zlib,
        CompressionAlgorithm::Zstd,
        CompressionAlgorithm::None,
    ];

    for algo in algorithms {
        compression.switch_algorithm(algo).await.unwrap();
        assert_eq!(compression.algorithm(), algo);
    }
}

// ============================================================================
// Stream Access Tests
// ============================================================================

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

// ============================================================================
// Large Data Tests
// ============================================================================

#[tokio::test]
async fn test_large_data_compression() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

    // 1MB of repetitive data
    let large_data = vec![b'A'; 1024 * 1024];
    compression.write_all(&large_data).await.unwrap();
    compression.shutdown().await.unwrap();

    let inner = compression.into_inner();
    let compressed = inner.written_data();

    // Should compress significantly
    assert!(compressed.len() < large_data.len() / 10);
    assert_eq!(&compressed[0..2], &[0x1f, 0x8b]);
}

#[tokio::test]
async fn test_highly_compressible_data() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

    let repetitive = b"AAAAAAAAAA".repeat(1000);
    compression.write_all(&repetitive).await.unwrap();
    compression.shutdown().await.unwrap();

    let inner = compression.into_inner();
    let compressed = inner.written_data();

    assert!(compressed.len() < repetitive.len() / 5);
}

#[tokio::test]
async fn test_incompressible_data() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

    // Random-like data (less compressible)
    let data: Vec<u8> = (0..1000).map(|i| (i * 7 + 13) as u8).collect();
    compression.write_all(&data).await.unwrap();
    compression.shutdown().await.unwrap();

    let inner = compression.into_inner();
    let compressed = inner.written_data();

    // May not compress much, but should still have gzip header
    assert_eq!(&compressed[0..2], &[0x1f, 0x8b]);
}

// ============================================================================
// Shutdown and Finalization Tests
// ============================================================================

#[tokio::test]
async fn test_shutdown_finalizes() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::Gzip);

    compression.write_all(b"test data").await.unwrap();
    compression.shutdown().await.unwrap();

    let inner = compression.into_inner();
    let compressed = inner.written_data();

    // Should have complete gzip stream
    assert!(compressed.len() > 10);
    assert_eq!(&compressed[0..2], &[0x1f, 0x8b]);
}

#[tokio::test]
async fn test_flush_without_shutdown() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    compression.write_all(b"test").await.unwrap();
    compression.flush().await.unwrap();

    let inner = compression.into_inner();
    assert_eq!(inner.written_data(), b"test");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_write_after_flush() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    compression.write_all(b"first").await.unwrap();
    compression.flush().await.unwrap();
    compression.write_all(b"second").await.unwrap();
    compression.flush().await.unwrap();

    let inner = compression.into_inner();
    assert_eq!(inner.written_data(), b"firstsecond");
}

#[tokio::test]
async fn test_multiple_flushes() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    compression.write_all(b"data").await.unwrap();
    compression.flush().await.unwrap();
    compression.flush().await.unwrap();
    compression.flush().await.unwrap();

    let inner = compression.into_inner();
    assert_eq!(inner.written_data(), b"data");
}

#[tokio::test]
async fn test_zero_byte_writes() {
    let stream = MockStream::new();
    let mut compression = CompressionStream::new(stream, CompressionAlgorithm::None);

    for _ in 0..10 {
        compression.write_all(b"").await.unwrap();
    }
    compression.flush().await.unwrap();

    let inner = compression.into_inner();
    assert_eq!(inner.written_data(), b"");
}
