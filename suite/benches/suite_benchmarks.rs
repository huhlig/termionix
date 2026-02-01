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

//! Integration Benchmarks for Termionix
//!
//! This benchmark suite tests the full server-client integration over loopback,
//! measuring performance, efficiency, and correctness of the complete system.

use async_trait::async_trait;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main, Throughput};
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use termionix_client::{ClientConfig, TerminalClient, TerminalConnection, TerminalHandler};
use termionix_service::{
    ConnectionId, ServerConfig, ServerHandler, TelnetConnection, TelnetServer, TerminalEvent,
};
use tokio::sync::{Barrier, Notify};

// ============================================================================
// Test Handlers
// ============================================================================

/// Server handler that echoes data back to clients
struct EchoServerHandler {
    messages_received: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    messages_sent: Arc<AtomicU64>,
    bytes_sent: Arc<AtomicU64>,
}

impl EchoServerHandler {
    fn new() -> Self {
        Self {
            messages_received: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            messages_sent: Arc::new(AtomicU64::new(0)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
        }
    }

    fn stats(&self) -> (u64, u64, u64, u64) {
        (
            self.messages_received.load(Ordering::Relaxed),
            self.bytes_received.load(Ordering::Relaxed),
            self.messages_sent.load(Ordering::Relaxed),
            self.bytes_sent.load(Ordering::Relaxed),
        )
    }

    fn reset(&self) {
        self.messages_received.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.messages_sent.store(0, Ordering::Relaxed);
        self.bytes_sent.store(0, Ordering::Relaxed);
    }
}

#[async_trait]
impl ServerHandler for EchoServerHandler {
    async fn on_event(&self, _id: ConnectionId, conn: &TelnetConnection, event: TerminalEvent) {
        match event {
            TerminalEvent::CharacterData { character, .. } => {
                self.messages_received.fetch_add(1, Ordering::Relaxed);
                self.bytes_received.fetch_add(1, Ordering::Relaxed);
                let _ = conn.send_char(character).await;
                self.messages_sent.fetch_add(1, Ordering::Relaxed);
                self.bytes_sent.fetch_add(1, Ordering::Relaxed);
            }
            TerminalEvent::LineCompleted { line, .. } => {
                self.messages_received.fetch_add(1, Ordering::Relaxed);
                let text = line.to_string();
                let bytes = text.len() as u64;
                self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
                let response = format!("{}\r\n", text);
                let response_bytes = response.len() as u64;
                let _ = conn.send(&response).await;
                self.messages_sent.fetch_add(1, Ordering::Relaxed);
                self.bytes_sent.fetch_add(response_bytes, Ordering::Relaxed);
            }
            _ => {}
        }
    }
}

/// Client handler that counts received messages
struct BenchmarkClientHandler {
    messages_received: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    lines_received: Arc<AtomicU64>,
    chars_received: Arc<AtomicU64>,
    connected: Arc<Notify>,
    expected_messages: Arc<AtomicUsize>,
    completion_notify: Arc<Notify>,
}

impl BenchmarkClientHandler {
    fn new(expected_messages: usize) -> Self {
        Self {
            messages_received: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            lines_received: Arc::new(AtomicU64::new(0)),
            chars_received: Arc::new(AtomicU64::new(0)),
            connected: Arc::new(Notify::new()),
            expected_messages: Arc::new(AtomicUsize::new(expected_messages)),
            completion_notify: Arc::new(Notify::new()),
        }
    }

    fn stats(&self) -> (u64, u64, u64, u64) {
        (
            self.messages_received.load(Ordering::Relaxed),
            self.bytes_received.load(Ordering::Relaxed),
            self.lines_received.load(Ordering::Relaxed),
            self.chars_received.load(Ordering::Relaxed),
        )
    }

    fn reset(&self) {
        self.messages_received.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        self.lines_received.store(0, Ordering::Relaxed);
        self.chars_received.store(0, Ordering::Relaxed);
    }

    async fn wait_connected(&self) {
        self.connected.notified().await;
    }

    async fn wait_completion(&self) {
        self.completion_notify.notified().await;
    }

    fn set_expected(&self, count: usize) {
        self.expected_messages.store(count, Ordering::Relaxed);
    }
}

#[async_trait]
impl TerminalHandler for BenchmarkClientHandler {
    async fn on_connect(&self, _conn: &TerminalConnection) {
        self.connected.notify_waiters();
    }

    async fn on_character(&self, _conn: &TerminalConnection, _ch: char) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(1, Ordering::Relaxed);
        self.chars_received.fetch_add(1, Ordering::Relaxed);
        
        let received = self.chars_received.load(Ordering::Relaxed);
        let expected = self.expected_messages.load(Ordering::Relaxed) as u64;
        
        if received >= expected {
            self.completion_notify.notify_waiters();
        }
    }

    async fn on_line(&self, _conn: &TerminalConnection, line: &str) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
        self.lines_received.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(line.len() as u64, Ordering::Relaxed);
        
        let received = self.lines_received.load(Ordering::Relaxed);
        let expected = self.expected_messages.load(Ordering::Relaxed) as u64;
        
        if received >= expected {
            self.completion_notify.notify_waiters();
        }
    }
}

// ============================================================================
// Test Infrastructure
// ============================================================================

/// Setup a test server and return its address
async fn setup_test_server(
    handler: Arc<dyn ServerHandler>,
) -> Result<(TelnetServer, std::net::SocketAddr), Box<dyn std::error::Error>> {
    let config = ServerConfig::new("127.0.0.1:0".parse()?)
        .with_max_connections(1000)
        .with_idle_timeout(Duration::from_secs(300));

    let server = TelnetServer::new(config).await?;
    let addr = server.bind_address();

    server.start(handler).await?;

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    Ok((server, addr))
}

/// Create a connected client and return the stream for direct I/O
async fn create_client_stream(
    addr: std::net::SocketAddr,
) -> Result<tokio::net::TcpStream, Box<dyn std::error::Error>> {
    let stream = tokio::net::TcpStream::connect(addr).await?;
    Ok(stream)
}

// ============================================================================
// Benchmark: Throughput - Small Messages
// ============================================================================

fn bench_throughput_small_messages(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("throughput_small_messages");
    group.measurement_time(Duration::from_secs(10));

    for client_count in [1, 5, 10, 25].iter() {
        for msg_size in [10, 50, 100].iter() {
            let messages_per_client = 100;
            let total_bytes = client_count * messages_per_client * msg_size;
            
            group.throughput(Throughput::Bytes(total_bytes as u64));
            
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{}clients_{}bytes", client_count, msg_size)),
                &(client_count, msg_size),
                |b, &(&client_count, &msg_size)| {
                    b.to_async(&runtime).iter(|| async move {
                        let server_handler = Arc::new(EchoServerHandler::new());
                        let (server, addr) = setup_test_server(server_handler.clone()).await.unwrap();

                        let barrier = Arc::new(Barrier::new(client_count + 1));
                        let mut client_handles = Vec::new();

                        // Create message payload
                        let message = "x".repeat(msg_size);

                        // Create and connect clients
                        for _ in 0..client_count {
                            let addr = addr.clone();
                            let barrier = barrier.clone();
                            let message = message.clone();

                            let handle = tokio::spawn(async move {
                                let mut stream = create_client_stream(addr).await.unwrap();
                                use tokio::io::AsyncWriteExt;
                                
                                barrier.wait().await;

                                let start = Instant::now();
                                for _ in 0..messages_per_client {
                                    let msg = format!("{}\r\n", message);
                                    let _ = stream.write_all(msg.as_bytes()).await;
                                    let _ = stream.flush().await;
                                }

                                // Give time for responses to arrive
                                tokio::time::sleep(Duration::from_millis(100)).await;
                                let elapsed = start.elapsed();

                                drop(stream);
                                elapsed
                            });
                            
                            client_handles.push(handle);
                        }

                        barrier.wait().await;
                        let overall_start = Instant::now();

                        let mut max_client_time = Duration::ZERO;
                        for handle in client_handles {
                            if let Ok(elapsed) = handle.await {
                                max_client_time = max_client_time.max(elapsed);
                            }
                        }

                        let overall_elapsed = overall_start.elapsed();
                        let (_, server_bytes_rx, _, server_bytes_tx) = server_handler.stats();

                        black_box((overall_elapsed, max_client_time, server_bytes_rx, server_bytes_tx));

                        server.shutdown().await.unwrap();
                    });
                },
            );
        }
    }
    group.finish();
}

// ============================================================================
// Benchmark: Throughput - Large Messages
// ============================================================================

fn bench_throughput_large_messages(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("throughput_large_messages");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    for client_count in [1, 5, 10].iter() {
        for msg_size in [1024, 4096, 8192].iter() {
            let messages_per_client = 50;
            let total_bytes = client_count * messages_per_client * msg_size;
            
            group.throughput(Throughput::Bytes(total_bytes as u64));
            
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{}clients_{}KB", client_count, msg_size / 1024)),
                &(client_count, msg_size),
                |b, &(&client_count, &msg_size)| {
                    b.to_async(&runtime).iter(|| async move {
                        let server_handler = Arc::new(EchoServerHandler::new());
                        let (server, addr) = setup_test_server(server_handler.clone()).await.unwrap();

                        let barrier = Arc::new(Barrier::new(client_count + 1));
                        let mut client_handles = Vec::new();

                        let message = "x".repeat(msg_size);

                        for _ in 0..client_count {
                            let addr = addr.clone();
                            let barrier = barrier.clone();
                            let message = message.clone();

                            let handle = tokio::spawn(async move {
                                let mut stream = create_client_stream(addr).await.unwrap();
                                use tokio::io::AsyncWriteExt;
                                
                                barrier.wait().await;

                                let start = Instant::now();
                                for _ in 0..messages_per_client {
                                    let msg = format!("{}\r\n", message);
                                    let _ = stream.write_all(msg.as_bytes()).await;
                                    let _ = stream.flush().await;
                                }

                                // Give time for responses to arrive
                                tokio::time::sleep(Duration::from_millis(100)).await;
                                let elapsed = start.elapsed();

                                drop(stream);
                                elapsed
                            });
                            
                            client_handles.push(handle);
                        }

                        barrier.wait().await;
                        let overall_start = Instant::now();

                        let mut max_client_time = Duration::ZERO;
                        for handle in client_handles {
                            if let Ok(elapsed) = handle.await {
                                max_client_time = max_client_time.max(elapsed);
                            }
                        }

                        let overall_elapsed = overall_start.elapsed();
                        let (_, server_bytes_rx, _, server_bytes_tx) = server_handler.stats();

                        black_box((overall_elapsed, max_client_time, server_bytes_rx, server_bytes_tx));

                        server.shutdown().await.unwrap();
                    });
                },
            );
        }
    }
    group.finish();
}

// ============================================================================
// Benchmark: Character-by-Character Throughput
// ============================================================================

fn bench_character_throughput(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("character_throughput");
    group.measurement_time(Duration::from_secs(10));

    for client_count in [1, 5, 10].iter() {
        let chars_per_client = 500;
        let total_chars = client_count * chars_per_client;
        
        group.throughput(Throughput::Elements(total_chars as u64));
        
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}clients", client_count)),
            client_count,
            |b, &client_count| {
                b.to_async(&runtime).iter(|| async move {
                    let server_handler = Arc::new(EchoServerHandler::new());
                    let (server, addr) = setup_test_server(server_handler.clone()).await.unwrap();

                    let barrier = Arc::new(Barrier::new(client_count + 1));
                    let mut client_handles = Vec::new();

                    for _ in 0..client_count {
                        let addr = addr.clone();
                        let barrier = barrier.clone();

                        let handle = tokio::spawn(async move {
                            let mut stream = create_client_stream(addr).await.unwrap();
                            use tokio::io::AsyncWriteExt;
                            
                            barrier.wait().await;

                            let start = Instant::now();
                            for i in 0..chars_per_client {
                                let ch = (b'a' + (i % 26) as u8) as char;
                                let _ = stream.write_all(&[ch as u8]).await;
                                let _ = stream.flush().await;
                            }

                            // Give time for responses to arrive
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            let elapsed = start.elapsed();

                            drop(stream);
                            elapsed
                        });
                        
                        client_handles.push(handle);
                    }

                    barrier.wait().await;
                    let overall_start = Instant::now();

                    for handle in client_handles {
                        let _ = handle.await;
                    }

                    let overall_elapsed = overall_start.elapsed();
                    let (_, server_bytes_rx, _, server_bytes_tx) = server_handler.stats();

                    black_box((overall_elapsed, server_bytes_rx, server_bytes_tx));

                    server.shutdown().await.unwrap();
                });
            },
        );
    }
    group.finish();
}

// ============================================================================
// Benchmark: Mixed Workload
// ============================================================================

fn bench_mixed_workload(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("mixed_workload");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(20);

    for client_count in [5, 10, 25].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}clients", client_count)),
            client_count,
            |b, &client_count| {
                b.to_async(&runtime).iter(|| async move {
                    let server_handler = Arc::new(EchoServerHandler::new());
                    let (server, addr) = setup_test_server(server_handler.clone()).await.unwrap();

                    let barrier = Arc::new(Barrier::new(client_count + 1));
                    let mut client_handles = Vec::new();

                    for client_id in 0..client_count {
                        let addr = addr.clone();
                        let barrier = barrier.clone();

                        let handle = tokio::spawn(async move {
                            let mut stream = create_client_stream(addr).await.unwrap();
                            use tokio::io::AsyncWriteExt;
                            
                            barrier.wait().await;

                            let start = Instant::now();
                            
                            // Mix of small, medium, and large messages
                            for i in 0..100 {
                                let msg = match i % 3 {
                                    0 => format!("small{}\r\n", i),
                                    1 => format!("medium message {}\r\n", "x".repeat(50)),
                                    _ => format!("large message {}\r\n", "x".repeat(500)),
                                };
                                let _ = stream.write_all(msg.as_bytes()).await;
                                let _ = stream.flush().await;
                                
                                // Vary the delay
                                if client_id % 2 == 0 {
                                    tokio::time::sleep(Duration::from_micros(50)).await;
                                }
                            }

                            // Give time for responses to arrive
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            let elapsed = start.elapsed();

                            drop(stream);
                            elapsed
                        });
                        
                        client_handles.push(handle);
                    }

                    barrier.wait().await;
                    let overall_start = Instant::now();

                    for handle in client_handles {
                        let _ = handle.await;
                    }

                    let overall_elapsed = overall_start.elapsed();
                    let (_, server_bytes_rx, _, server_bytes_tx) = server_handler.stats();

                    black_box((overall_elapsed, server_bytes_rx, server_bytes_tx));

                    server.shutdown().await.unwrap();
                });
            },
        );
    }
    group.finish();
}

// ============================================================================
// Benchmark: Latency Distribution
// ============================================================================

fn bench_latency_distribution(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("latency_distribution_100_samples", |b| {
        b.to_async(&runtime).iter(|| async {
            let server_handler = Arc::new(EchoServerHandler::new());
            let (server, addr) = setup_test_server(server_handler.clone()).await.unwrap();

            let mut latencies = Vec::new();
            
            let mut stream = create_client_stream(addr).await.unwrap();
            use tokio::io::AsyncWriteExt;

            for i in 0..100 {
                let start = Instant::now();
                let msg = format!("ping {}\r\n", i);
                let _ = stream.write_all(msg.as_bytes()).await;
                let _ = stream.flush().await;
                
                tokio::time::sleep(Duration::from_millis(5)).await;
                
                latencies.push(start.elapsed());
            }

            drop(stream);

            black_box(latencies);

            server.shutdown().await.unwrap();
        });
    });
}

// ============================================================================
// Benchmark: Sustained Load Test
// ============================================================================

fn bench_sustained_load(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap();

    let mut group = c.benchmark_group("sustained_load");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10);

    for client_count in [10, 25, 50].iter() {
        let messages_per_client = 200;
        group.throughput(Throughput::Elements(*client_count as u64 * messages_per_client));
        
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}clients_sustained", client_count)),
            client_count,
            |b, &client_count| {
                b.to_async(&runtime).iter(|| async move {
                    let server_handler = Arc::new(EchoServerHandler::new());
                    let (server, addr) = setup_test_server(server_handler.clone()).await.unwrap();

                    let mut client_handles = Vec::new();

                    for client_id in 0..client_count {
                        let addr = addr.clone();
                        
                        let handle = tokio::spawn(async move {
                            let mut stream = create_client_stream(addr).await.unwrap();
                            use tokio::io::AsyncWriteExt;

                            let start = Instant::now();
                            
                            for i in 0..messages_per_client {
                                let msg = format!("Client {} msg {}\r\n", client_id, i);
                                let _ = stream.write_all(msg.as_bytes()).await;
                                let _ = stream.flush().await;
                                
                                tokio::time::sleep(Duration::from_micros(100)).await;
                            }

                            // Give time for responses to arrive
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            let elapsed = start.elapsed();

                            drop(stream);
                            elapsed
                        });
                        
                        client_handles.push(handle);
                    }

                    let overall_start = Instant::now();

                    for handle in client_handles {
                        let _ = handle.await;
                    }

                    let overall_elapsed = overall_start.elapsed();
                    let (_, server_bytes_rx, _, server_bytes_tx) = server_handler.stats();

                    black_box((overall_elapsed, server_bytes_rx, server_bytes_tx));

                    server.shutdown().await.unwrap();
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    integration_benches,
    bench_throughput_small_messages,
    bench_throughput_large_messages,
    bench_character_throughput,
    bench_mixed_workload,
    bench_latency_distribution,
    bench_sustained_load,
);

criterion_main!(integration_benches);


