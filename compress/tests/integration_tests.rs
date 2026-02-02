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

//! Integration tests for termionix-compress
//!
//! These tests verify end-to-end compression/decompression workflows,
//! real-world usage patterns, and interoperability with actual I/O streams.

use termionix_compress::{CompressionAlgorithm, CompressionStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ============================================================================
// Round-trip Compression/Decompression Tests
// ============================================================================

#[tokio::test]
async fn test_gzip_roundtrip() {
    let original_data = b"Hello, World! This is a test of Gzip compression.";
    let (client, server) = tokio::io::duplex(8192);

    // Compress in background task
    let compress_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Gzip);
        compressor.write_all(original_data).await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    // Decompress
    use async_compression::tokio::bufread::GzipDecoder;
    use tokio::io::BufReader;

    let reader = BufReader::new(server);
    let mut decoder = GzipDecoder::new(reader);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).await.unwrap();

    compress_handle.await.unwrap();
    assert_eq!(&decompressed, original_data);
}

#[tokio::test]
async fn test_deflate_roundtrip() {
    let original_data = b"Testing Deflate compression algorithm.";
    let (client, server) = tokio::io::duplex(8192);

    let compress_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Deflate);
        compressor.write_all(original_data).await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    use async_compression::tokio::bufread::DeflateDecoder;
    use tokio::io::BufReader;

    let reader = BufReader::new(server);
    let mut decoder = DeflateDecoder::new(reader);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).await.unwrap();

    compress_handle.await.unwrap();
    assert_eq!(&decompressed, original_data);
}

#[tokio::test]
async fn test_zlib_roundtrip() {
    let original_data = b"Testing Zlib compression with checksums.";
    let (client, server) = tokio::io::duplex(8192);

    let compress_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Zlib);
        compressor.write_all(original_data).await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    use async_compression::tokio::bufread::ZlibDecoder;
    use tokio::io::BufReader;

    let reader = BufReader::new(server);
    let mut decoder = ZlibDecoder::new(reader);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).await.unwrap();

    compress_handle.await.unwrap();
    assert_eq!(&decompressed, original_data);
}

#[tokio::test]
async fn test_zstd_roundtrip() {
    let original_data = b"Testing Zstandard compression for speed and ratio.";
    let (client, server) = tokio::io::duplex(8192);

    let compress_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Zstd);
        compressor.write_all(original_data).await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    use async_compression::tokio::bufread::ZstdDecoder;
    use tokio::io::BufReader;

    let reader = BufReader::new(server);
    let mut decoder = ZstdDecoder::new(reader);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).await.unwrap();

    compress_handle.await.unwrap();
    assert_eq!(&decompressed, original_data);
}

#[tokio::test]
async fn test_brotli_roundtrip() {
    let original_data = b"Testing Brotli compression for web content.";
    let (client, server) = tokio::io::duplex(8192);

    let compress_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Brotli);
        compressor.write_all(original_data).await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    use async_compression::tokio::bufread::BrotliDecoder;
    use tokio::io::BufReader;

    let reader = BufReader::new(server);
    let mut decoder = BrotliDecoder::new(reader);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).await.unwrap();

    compress_handle.await.unwrap();
    assert_eq!(&decompressed, original_data);
}

// ============================================================================
// DuplexStream Integration Tests
// ============================================================================

#[tokio::test]
async fn test_duplex_stream_compression() {
    let (client, server) = tokio::io::duplex(1024);

    let write_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Gzip);
        compressor.write_all(b"Hello from client!").await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    use async_compression::tokio::bufread::GzipDecoder;
    use tokio::io::BufReader;

    let reader = BufReader::new(server);
    let mut decoder = GzipDecoder::new(reader);
    let mut received = Vec::new();
    decoder.read_to_end(&mut received).await.unwrap();

    write_handle.await.unwrap();
    assert_eq!(&received, b"Hello from client!");
}

#[tokio::test]
async fn test_bidirectional_compression() {
    let (client, server) = tokio::io::duplex(2048);

    let client_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Gzip);
        compressor.write_all(b"Request from client").await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    let server_handle = tokio::spawn(async move {
        use async_compression::tokio::bufread::GzipDecoder;
        use tokio::io::BufReader;

        let reader = BufReader::new(server);
        let mut decoder = GzipDecoder::new(reader);
        let mut received = Vec::new();
        decoder.read_to_end(&mut received).await.unwrap();
        received
    });

    client_handle.await.unwrap();
    let received = server_handle.await.unwrap();
    assert_eq!(&received, b"Request from client");
}

// ============================================================================
// Algorithm Switching Integration Tests
// ============================================================================
// Note: Algorithm switching with duplex streams has timing complexities
// that are better tested in unit tests with MockStream

// ============================================================================
// Large Data Integration Tests
// ============================================================================

#[tokio::test]
async fn test_compress_large_text_file() {
    let mut large_text = String::new();
    for i in 0..1000 {
        large_text.push_str(&format!(
            "Line {}: This is a test line with some repetitive content.\n",
            i
        ));
    }
    let original_data = large_text.into_bytes();

    let (client, server) = tokio::io::duplex(1024 * 1024);

    let data_clone = original_data.clone();
    let compress_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Gzip);
        compressor.write_all(&data_clone).await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    use async_compression::tokio::bufread::GzipDecoder;
    use tokio::io::BufReader;

    let reader = BufReader::new(server);
    let mut decoder = GzipDecoder::new(reader);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).await.unwrap();

    compress_handle.await.unwrap();
    assert_eq!(&decompressed, &original_data);
}

#[tokio::test]
async fn test_compress_binary_data() {
    let binary_data: Vec<u8> = (0..10000).map(|i| ((i * 7 + 13) % 256) as u8).collect();

    let (client, server) = tokio::io::duplex(1024 * 1024);

    let data_clone = binary_data.clone();
    let compress_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Zstd);
        compressor.write_all(&data_clone).await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    use async_compression::tokio::bufread::ZstdDecoder;
    use tokio::io::BufReader;

    let reader = BufReader::new(server);
    let mut decoder = ZstdDecoder::new(reader);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).await.unwrap();

    compress_handle.await.unwrap();
    assert_eq!(&decompressed, &binary_data);
}

#[tokio::test]
async fn test_streaming_large_data() {
    let (client, server) = tokio::io::duplex(1024 * 1024);

    let write_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Gzip);

        let chunk_size = 1024;
        let num_chunks = 100;
        let chunk_data = vec![b'X'; chunk_size];

        for _ in 0..num_chunks {
            compressor.write_all(&chunk_data).await.unwrap();
        }
        compressor.shutdown().await.unwrap();
    });

    let read_handle = tokio::spawn(async move {
        let mut compressed = Vec::new();
        let mut server = server;
        server.read_to_end(&mut compressed).await.unwrap();
        compressed
    });

    write_handle.await.unwrap();
    let compressed = read_handle.await.unwrap();
    let original_size = 1024 * 100;

    // Should compress well due to repetition
    assert!(compressed.len() < original_size / 10);
}

// ============================================================================
// Real-world Scenario Tests
// ============================================================================

#[tokio::test]
async fn test_http_response_compression() {
    let http_body = r#"
    {
        "status": "success",
        "data": {
            "users": [
                {"id": 1, "name": "Alice", "email": "alice@example.com"},
                {"id": 2, "name": "Bob", "email": "bob@example.com"},
                {"id": 3, "name": "Charlie", "email": "charlie@example.com"}
            ]
        }
    }
    "#;

    let (client, server) = tokio::io::duplex(4096);

    let write_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Gzip);
        compressor.write_all(http_body.as_bytes()).await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    let read_handle = tokio::spawn(async move {
        let mut compressed = Vec::new();
        let mut server = server;
        server.read_to_end(&mut compressed).await.unwrap();
        compressed
    });

    write_handle.await.unwrap();
    let compressed = read_handle.await.unwrap();

    // Should compress JSON well
    assert!(compressed.len() < http_body.len());
    assert_eq!(&compressed[0..2], &[0x1f, 0x8b]);
}

#[tokio::test]
async fn test_log_file_compression() {
    let mut log_data = String::new();
    for i in 0..1000 {
        log_data.push_str(&format!(
            "[2026-01-31 12:00:{:02}] INFO: Processing request #{}\n",
            i % 60,
            i
        ));
    }
    let log_data_len = log_data.len();

    let (client, server) = tokio::io::duplex(1024 * 1024);

    let write_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Zstd);
        compressor.write_all(log_data.as_bytes()).await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    let read_handle = tokio::spawn(async move {
        let mut compressed = Vec::new();
        let mut server = server;
        server.read_to_end(&mut compressed).await.unwrap();
        compressed
    });

    write_handle.await.unwrap();
    let compressed = read_handle.await.unwrap();

    // Log files should compress very well
    assert!(compressed.len() < log_data_len / 10);
}

#[tokio::test]
async fn test_telnet_protocol_compression() {
    let telnet_data = b"This is telnet data that might be compressed in a MUD server.";

    let (client, server) = tokio::io::duplex(4096);

    let compress_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Deflate);
        compressor.write_all(telnet_data).await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    use async_compression::tokio::bufread::DeflateDecoder;
    use tokio::io::BufReader;

    let reader = BufReader::new(server);
    let mut decoder = DeflateDecoder::new(reader);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).await.unwrap();

    compress_handle.await.unwrap();
    assert_eq!(&decompressed, telnet_data);
}

// ============================================================================
// Error Handling Integration Tests
// ============================================================================

#[tokio::test]
async fn test_graceful_shutdown_on_error() {
    let (client, _server) = tokio::io::duplex(1024);
    let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Gzip);

    compressor.write_all(b"some data").await.unwrap();

    // Shutdown should succeed even if there's pending data
    let result = compressor.shutdown().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_shutdowns() {
    let (client, _server) = tokio::io::duplex(1024);
    let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Gzip);

    compressor.write_all(b"data").await.unwrap();
    compressor.shutdown().await.unwrap();

    // Second shutdown should be safe
    let result = compressor.shutdown().await;
    assert!(result.is_ok());
}

// ============================================================================
// Performance Characteristic Tests
// ============================================================================

#[tokio::test]
async fn test_compression_ratio_comparison() {
    let test_data = b"AAAAAAAAAA".repeat(1000);

    for algo in [
        CompressionAlgorithm::None,
        CompressionAlgorithm::Gzip,
        CompressionAlgorithm::Deflate,
        CompressionAlgorithm::Zlib,
        CompressionAlgorithm::Zstd,
        CompressionAlgorithm::Brotli,
    ] {
        let (client, server) = tokio::io::duplex(1024 * 1024);
        let data_clone = test_data.clone();

        let write_handle = tokio::spawn(async move {
            let mut compressor = CompressionStream::new(client, algo);
            compressor.write_all(&data_clone).await.unwrap();
            compressor.shutdown().await.unwrap();
        });

        let read_handle = tokio::spawn(async move {
            let mut compressed = Vec::new();
            let mut server = server;
            server.read_to_end(&mut compressed).await.unwrap();
            compressed
        });

        write_handle.await.unwrap();
        let compressed = read_handle.await.unwrap();

        if algo == CompressionAlgorithm::None {
            assert_eq!(compressed.len(), test_data.len());
        } else {
            // All compression algorithms should reduce size significantly
            assert!(
                compressed.len() < test_data.len() / 10,
                "{:?} didn't compress well enough: {} bytes",
                algo,
                compressed.len()
            );
        }
    }
}

#[tokio::test]
async fn test_small_data_overhead() {
    let small_data = b"Hi";

    for algo in [
        CompressionAlgorithm::Gzip,
        CompressionAlgorithm::Deflate,
        CompressionAlgorithm::Zlib,
        CompressionAlgorithm::Zstd,
        CompressionAlgorithm::Brotli,
    ] {
        let (client, server) = tokio::io::duplex(1024);

        let write_handle = tokio::spawn(async move {
            let mut compressor = CompressionStream::new(client, algo);
            compressor.write_all(small_data).await.unwrap();
            compressor.shutdown().await.unwrap();
        });

        let read_handle = tokio::spawn(async move {
            let mut compressed = Vec::new();
            let mut server = server;
            server.read_to_end(&mut compressed).await.unwrap();
            compressed
        });

        write_handle.await.unwrap();
        let compressed = read_handle.await.unwrap();

        // Small data may not compress (overhead of headers)
        // Just verify it doesn't panic and produces output
        assert!(!compressed.is_empty());
    }
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_compression_streams() {
    let mut handles = Vec::new();

    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let (client, server) = tokio::io::duplex(4096);

            let write_handle = tokio::spawn(async move {
                let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Gzip);
                let data = format!("Stream {} data", i);
                compressor.write_all(data.as_bytes()).await.unwrap();
                compressor.shutdown().await.unwrap();
            });

            let read_handle = tokio::spawn(async move {
                let mut result = Vec::new();
                let mut server = server;
                server.read_to_end(&mut result).await.unwrap();
                result
            });

            write_handle.await.unwrap();
            read_handle.await.unwrap()
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(!result.is_empty());
        assert_eq!(&result[0..2], &[0x1f, 0x8b]); // Gzip magic
    }
}

// ============================================================================
// Edge Case Integration Tests
// ============================================================================

#[tokio::test]
async fn test_empty_stream_compression() {
    for algo in [
        CompressionAlgorithm::None,
        CompressionAlgorithm::Gzip,
        CompressionAlgorithm::Deflate,
        CompressionAlgorithm::Zlib,
        CompressionAlgorithm::Zstd,
        CompressionAlgorithm::Brotli,
    ] {
        let (client, server) = tokio::io::duplex(1024);

        let write_handle = tokio::spawn(async move {
            let mut compressor = CompressionStream::new(client, algo);
            compressor.shutdown().await.unwrap();
        });

        let read_handle = tokio::spawn(async move {
            let mut result = Vec::new();
            let mut server = server;
            server.read_to_end(&mut result).await.unwrap();
            result
        });

        write_handle.await.unwrap();
        let result = read_handle.await.unwrap();

        // Empty stream should produce minimal output
        if algo == CompressionAlgorithm::None {
            assert_eq!(result.len(), 0);
        } else {
            // Compressed formats may have headers/footers
            assert!(result.len() < 100);
        }
    }
}

#[tokio::test]
async fn test_single_byte_compression() {
    let (client, server) = tokio::io::duplex(1024);

    let compress_handle = tokio::spawn(async move {
        let mut compressor = CompressionStream::new(client, CompressionAlgorithm::Gzip);
        compressor.write_all(b"A").await.unwrap();
        compressor.shutdown().await.unwrap();
    });

    use async_compression::tokio::bufread::GzipDecoder;
    use tokio::io::BufReader;

    let reader = BufReader::new(server);
    let mut decoder = GzipDecoder::new(reader);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).await.unwrap();

    compress_handle.await.unwrap();
    assert_eq!(&decompressed, b"A");
}
