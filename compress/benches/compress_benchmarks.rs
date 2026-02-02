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

//! Benchmarks for termionix-compress

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use termionix_compress::{Algorithm, CompressionStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_compressible_data(size: usize) -> Vec<u8> {
    vec![b'A'; size]
}

fn create_random_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| ((i * 7 + 13) % 256) as u8).collect()
}

fn create_text_data(size: usize) -> Vec<u8> {
    let text = "The quick brown fox jumps over the lazy dog. ";
    text.as_bytes().iter().cycle().take(size).copied().collect()
}

// ============================================================================
// Compression Algorithm Benchmarks
// ============================================================================

fn bench_compression_algorithms(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_algorithms");

    let sizes = vec![1024, 10 * 1024]; // 1KB, 10KB
    let algorithms = vec![
        Algorithm::None,
        Algorithm::Gzip,
        Algorithm::Deflate,
        Algorithm::Zlib,
        Algorithm::Zstd,
        Algorithm::Brotli,
    ];

    for size in sizes {
        let data = create_compressible_data(size);
        group.throughput(Throughput::Bytes(size as u64));

        for algo in &algorithms {
            group.bench_with_input(
                BenchmarkId::new(format!("{:?}", algo), size),
                &data,
                |b, data| {
                    b.to_async(tokio::runtime::Runtime::new().unwrap())
                        .iter(|| async {
                            let (client, mut server) = tokio::io::duplex(1024 * 1024);
                            let data_clone = data.clone();
                            let algo_copy = *algo;

                            tokio::spawn(async move {
                                let mut compressor =
                                    CompressionStream::new(client, black_box(algo_copy));
                                compressor.write_all(black_box(&data_clone)).await.unwrap();
                                compressor.shutdown().await.unwrap();
                            });

                            let mut result = Vec::new();
                            server.read_to_end(&mut result).await.unwrap();
                            result
                        });
                },
            );
        }
    }

    group.finish();
}

// ============================================================================
// Data Type Benchmarks
// ============================================================================

fn bench_data_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_types");
    let size = 10 * 1024; // 10KB
    group.throughput(Throughput::Bytes(size as u64));

    let data_types = vec![
        ("compressible", create_compressible_data(size)),
        ("random", create_random_data(size)),
        ("text", create_text_data(size)),
    ];

    for (name, data) in data_types {
        group.bench_with_input(BenchmarkId::new("gzip", name), &data, |b, data| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| async {
                    let (client, mut server) = tokio::io::duplex(1024 * 1024);
                    let data_clone = data.clone();

                    tokio::spawn(async move {
                        let mut compressor = CompressionStream::new(client, Algorithm::Gzip);
                        compressor.write_all(black_box(&data_clone)).await.unwrap();
                        compressor.shutdown().await.unwrap();
                    });

                    let mut result = Vec::new();
                    server.read_to_end(&mut result).await.unwrap();
                    result
                });
        });
    }

    group.finish();
}

// ============================================================================
// Write Size Benchmarks
// ============================================================================

fn bench_write_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_sizes");

    let write_sizes = vec![256, 1024, 4096]; // Various write sizes
    let total_size = 50 * 1024; // 50KB total

    for write_size in write_sizes {
        let data = create_compressible_data(write_size);
        let num_writes = total_size / write_size;

        group.throughput(Throughput::Bytes(total_size as u64));
        group.bench_with_input(
            BenchmarkId::new("gzip", write_size),
            &(data, num_writes),
            |b, (data, num_writes)| {
                b.to_async(tokio::runtime::Runtime::new().unwrap())
                    .iter(|| async {
                        let (client, mut server) = tokio::io::duplex(1024 * 1024);
                        let data_clone = data.clone();
                        let num = *num_writes;

                        tokio::spawn(async move {
                            let mut compressor = CompressionStream::new(client, Algorithm::Gzip);
                            for _ in 0..num {
                                compressor.write_all(black_box(&data_clone)).await.unwrap();
                            }
                            compressor.shutdown().await.unwrap();
                        });

                        let mut result = Vec::new();
                        server.read_to_end(&mut result).await.unwrap();
                        result
                    });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Algorithm Switching Benchmarks
// ============================================================================

fn bench_algorithm_switching(c: &mut Criterion) {
    let mut group = c.benchmark_group("algorithm_switching");

    let data = create_compressible_data(1024);

    group.bench_function("switch_none_to_gzip", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let (client, mut server) = tokio::io::duplex(1024 * 1024);
                let data_clone = data.clone();

                tokio::spawn(async move {
                    let mut compressor = CompressionStream::new(client, Algorithm::None);
                    compressor.write_all(black_box(&data_clone)).await.unwrap();
                    compressor
                        .switch_algorithm(black_box(Algorithm::Gzip))
                        .await
                        .unwrap();
                    compressor.write_all(black_box(&data_clone)).await.unwrap();
                    compressor.shutdown().await.unwrap();
                });

                let mut result = Vec::new();
                server.read_to_end(&mut result).await.unwrap();
                result
            });
    });

    group.bench_function("switch_gzip_to_deflate", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let (client, mut server) = tokio::io::duplex(1024 * 1024);
                let data_clone = data.clone();

                tokio::spawn(async move {
                    let mut compressor = CompressionStream::new(client, Algorithm::Gzip);
                    compressor.write_all(black_box(&data_clone)).await.unwrap();
                    compressor
                        .switch_algorithm(black_box(Algorithm::Deflate))
                        .await
                        .unwrap();
                    compressor.write_all(black_box(&data_clone)).await.unwrap();
                    compressor.shutdown().await.unwrap();
                });

                let mut result = Vec::new();
                server.read_to_end(&mut result).await.unwrap();
                result
            });
    });

    group.finish();
}

// ============================================================================
// Stream Operations Benchmarks
// ============================================================================

fn bench_stream_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_operations");

    let data = create_compressible_data(10 * 1024);

    group.bench_function("write_flush_shutdown", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let (client, mut server) = tokio::io::duplex(1024 * 1024);
                let data_clone = data.clone();

                tokio::spawn(async move {
                    let mut compressor = CompressionStream::new(client, Algorithm::Gzip);
                    compressor.write_all(black_box(&data_clone)).await.unwrap();
                    compressor.flush().await.unwrap();
                    compressor.shutdown().await.unwrap();
                });

                let mut result = Vec::new();
                server.read_to_end(&mut result).await.unwrap();
                result
            });
    });

    group.bench_function("multiple_flushes", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let (client, mut server) = tokio::io::duplex(1024 * 1024);
                let data_clone = data.clone();

                tokio::spawn(async move {
                    let mut compressor = CompressionStream::new(client, Algorithm::Gzip);
                    let chunk_size = 1024;
                    for chunk in data_clone.chunks(chunk_size) {
                        compressor.write_all(black_box(chunk)).await.unwrap();
                        compressor.flush().await.unwrap();
                    }
                    compressor.shutdown().await.unwrap();
                });

                let mut result = Vec::new();
                server.read_to_end(&mut result).await.unwrap();
                result
            });
    });

    group.finish();
}

// ============================================================================
// Small Data Benchmarks
// ============================================================================

fn bench_small_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("small_data");

    let sizes = vec![16, 64, 256, 1024];

    for size in sizes {
        let data = create_text_data(size);
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("gzip", size), &data, |b, data| {
            b.to_async(tokio::runtime::Runtime::new().unwrap())
                .iter(|| async {
                    let (client, mut server) = tokio::io::duplex(1024 * 1024);
                    let data_clone = data.clone();

                    tokio::spawn(async move {
                        let mut compressor = CompressionStream::new(client, Algorithm::Gzip);
                        compressor.write_all(black_box(&data_clone)).await.unwrap();
                        compressor.shutdown().await.unwrap();
                    });

                    let mut result = Vec::new();
                    server.read_to_end(&mut result).await.unwrap();
                    result
                });
        });
    }

    group.finish();
}

// ============================================================================
// Large Data Benchmarks
// ============================================================================

fn bench_large_data(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_data");
    group.sample_size(10);

    let size = 1024 * 1024; // 1MB
    let data = create_compressible_data(size);
    group.throughput(Throughput::Bytes(size as u64));

    group.bench_function("gzip_1mb", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let (client, mut server) = tokio::io::duplex(2 * 1024 * 1024);
                let data_clone = data.clone();

                tokio::spawn(async move {
                    let mut compressor = CompressionStream::new(client, Algorithm::Gzip);
                    compressor.write_all(black_box(&data_clone)).await.unwrap();
                    compressor.shutdown().await.unwrap();
                });

                let mut result = Vec::new();
                server.read_to_end(&mut result).await.unwrap();
                result
            });
    });

    group.bench_function("zstd_1mb", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let (client, mut server) = tokio::io::duplex(2 * 1024 * 1024);
                let data_clone = data.clone();

                tokio::spawn(async move {
                    let mut compressor = CompressionStream::new(client, Algorithm::Zstd);
                    compressor.write_all(black_box(&data_clone)).await.unwrap();
                    compressor.shutdown().await.unwrap();
                });

                let mut result = Vec::new();
                server.read_to_end(&mut result).await.unwrap();
                result
            });
    });

    group.finish();
}

// ============================================================================
// Real-world Scenario Benchmarks
// ============================================================================

fn bench_real_world_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_scenarios");

    // HTTP response simulation
    let http_response =
        r#"{"status":"success","data":{"users":[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}]}}"#
            .repeat(100);
    group.throughput(Throughput::Bytes(http_response.len() as u64));

    group.bench_function("http_response_gzip", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let (client, mut server) = tokio::io::duplex(1024 * 1024);
                let data = http_response.clone();

                tokio::spawn(async move {
                    let mut compressor = CompressionStream::new(client, Algorithm::Gzip);
                    compressor
                        .write_all(black_box(data.as_bytes()))
                        .await
                        .unwrap();
                    compressor.shutdown().await.unwrap();
                });

                let mut result = Vec::new();
                server.read_to_end(&mut result).await.unwrap();
                result
            });
    });

    // Log file simulation
    let log_data = "[2026-01-31 12:00:00] INFO: Processing request\n".repeat(1000);
    group.throughput(Throughput::Bytes(log_data.len() as u64));

    group.bench_function("log_file_zstd", |b| {
        b.to_async(tokio::runtime::Runtime::new().unwrap())
            .iter(|| async {
                let (client, mut server) = tokio::io::duplex(1024 * 1024);
                let data = log_data.clone();

                tokio::spawn(async move {
                    let mut compressor = CompressionStream::new(client, Algorithm::Zstd);
                    compressor
                        .write_all(black_box(data.as_bytes()))
                        .await
                        .unwrap();
                    compressor.shutdown().await.unwrap();
                });

                let mut result = Vec::new();
                server.read_to_end(&mut result).await.unwrap();
                result
            });
    });

    group.finish();
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(
    benches,
    bench_compression_algorithms,
    bench_data_types,
    bench_write_sizes,
    bench_algorithm_switching,
    bench_stream_operations,
    bench_small_data,
    bench_large_data,
    bench_real_world_scenarios,
);

criterion_main!(benches);
