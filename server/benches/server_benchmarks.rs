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

//! Benchmarks for the Telnet server

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;
use termionix_server::{
    ConnectionId, ConnectionManager, ServerConfig, ServerHandler, ServerMetrics, TelnetConnection,
    TerminalCommand, WorkerConfig,
};
use tokio::net::{TcpListener, TcpStream};

// Simple test handler
struct BenchHandler;

#[async_trait::async_trait]
impl ServerHandler for BenchHandler {}

// Helper to create test connections
async fn create_test_connection() -> (TcpStream, TcpStream) {
    let mut attempts = 0;
    let listener = loop {
        match TcpListener::bind("127.0.0.1:0").await {
            Ok(listener) => break listener,
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse && attempts < 10 => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => panic!(
                "Failed to bind to ephemeral port after {} attempts: {}",
                attempts, e
            ),
        }
    };
    let addr = listener.local_addr().unwrap();

    let client_task = tokio::spawn(async move {
        TcpStream::connect(addr)
            .await
            .expect("Failed to connect to server")
    });

    let (server, _) = listener
        .accept()
        .await
        .expect("Failed to accept connection");
    let client = client_task.await.expect("Client task failed");

    (server, client)
}

// Benchmark connection creation
fn bench_connection_creation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Configure benchmark to use fewer samples to avoid port exhaustion
    let mut group = c.benchmark_group("connection_creation");
    group.sample_size(50); // Reduce from default 100
    group.measurement_time(Duration::from_secs(10)); // Give more time per sample

    group.bench_function("create", |b| {
        b.to_async(&runtime).iter(|| async {
            let (server, client) = create_test_connection().await;
            let id = ConnectionId::new(1);
            let connection = TelnetConnection::wrap(server, id).unwrap();
            black_box(&connection);

            // Properly close connections
            drop(connection);
            drop(client);

            // Small delay to allow port cleanup
            tokio::time::sleep(Duration::from_millis(1)).await;
        });
    });

    group.finish();
}

// Benchmark metrics updates
fn bench_metrics_updates(c: &mut Criterion) {
    let metrics = Arc::new(ServerMetrics::new());

    c.bench_function("metrics_connection_opened", |b| {
        b.iter(|| {
            metrics.connection_opened();
            black_box(&metrics);
        });
    });

    c.bench_function("metrics_bytes_sent", |b| {
        b.iter(|| {
            metrics.bytes_sent(1024);
            black_box(&metrics);
        });
    });

    c.bench_function("metrics_snapshot", |b| {
        b.iter(|| {
            let snapshot = metrics.snapshot();
            black_box(snapshot);
        });
    });
}

// Benchmark connection manager operations
fn bench_manager_operations(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("manager_add_connection", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = ServerConfig::default();
            let metrics = Arc::new(ServerMetrics::new());
            let worker_config = WorkerConfig {
                read_timeout: config.read_timeout,
                idle_timeout: config.idle_timeout,
                write_timeout: config.write_timeout,
                control_buffer_size: 100,
            };
            let manager = ConnectionManager::new(metrics, worker_config);

            let (server, _client) = create_test_connection().await;
            let id = ConnectionId::new(1);
            let connection = TelnetConnection::wrap(server, id).unwrap();

            let result = manager
                .add_connection(connection, Arc::new(BenchHandler))
                .unwrap();
            black_box(result);

            // Cleanup
            manager.shutdown().await;
        });
    });
}

// Benchmark broadcast operations with varying connection counts
fn bench_broadcast_scaling(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("broadcast_scaling");

    for conn_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(conn_count),
            conn_count,
            |b, &conn_count| {
                b.to_async(&runtime).iter(|| async move {
                    let config = ServerConfig::default();
                    let metrics = Arc::new(ServerMetrics::new());
                    let worker_config = WorkerConfig {
                        read_timeout: config.read_timeout,
                        idle_timeout: config.idle_timeout,
                        write_timeout: config.write_timeout,
                        control_buffer_size: 100,
                    };
                    let manager = ConnectionManager::new(metrics, worker_config);

                    // Add connections
                    let mut clients = Vec::new();
                    for i in 0..conn_count {
                        let (server, client) = create_test_connection().await;
                        let id = ConnectionId::new(i as u64);
                        let connection = TelnetConnection::wrap(server, id).unwrap();
                        manager
                            .add_connection(connection, Arc::new(BenchHandler))
                            .unwrap();
                        clients.push(client);
                    }

                    // Give connections time to initialize
                    tokio::time::sleep(Duration::from_millis(100)).await;

                    // Benchmark broadcast
                    let result = manager.broadcast(TerminalCommand::EraseLine).await;
                    black_box(result);

                    // Give time for broadcast to complete
                    tokio::time::sleep(Duration::from_millis(50)).await;

                    // Cleanup
                    manager.shutdown().await;
                    drop(clients);
                });
            },
        );
    }
    group.finish();
}

// Benchmark concurrent connection operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("concurrent_connection_queries", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = ServerConfig::default();
            let metrics = Arc::new(ServerMetrics::new());
            let worker_config = WorkerConfig {
                read_timeout: config.read_timeout,
                idle_timeout: config.idle_timeout,
                write_timeout: config.write_timeout,
                control_buffer_size: 100,
            };
            let manager = Arc::new(ConnectionManager::new(metrics, worker_config));

            // Add some connections
            let mut clients = Vec::new();
            for i in 0..10 {
                let (server, client) = create_test_connection().await;
                let id = ConnectionId::new(i);
                let connection = TelnetConnection::wrap(server, id).unwrap();
                manager
                    .add_connection(connection, Arc::new(BenchHandler))
                    .unwrap();
                clients.push(client);
            }

            // Spawn concurrent queries
            let mut handles = Vec::new();
            for _ in 0..100 {
                let mgr = manager.clone();
                handles.push(tokio::spawn(async move {
                    let _count = mgr.connection_count();
                    let _ids = mgr.get_connection_ids();
                    let _infos = mgr.get_all_connection_infos();
                }));
            }

            // Wait for all queries
            for handle in handles {
                handle.await.unwrap();
            }

            // Cleanup
            manager.shutdown().await;
            drop(clients);
        });
    });
}

// Benchmark state transitions
fn bench_state_transitions(c: &mut Criterion) {
    use std::sync::atomic::{AtomicU8, Ordering};
    use termionix_server::ConnectionState;

    let state = AtomicU8::new(ConnectionState::Connecting.as_u8());

    c.bench_function("state_transition", |b| {
        b.iter(|| {
            state.store(ConnectionState::Active.as_u8(), Ordering::Release);
            let current = ConnectionState::from_u8(state.load(Ordering::Acquire));
            black_box(current);
        });
    });
}

criterion_group!(
    benches,
    bench_connection_creation,
    bench_metrics_updates,
    bench_manager_operations,
    bench_broadcast_scaling,
    bench_concurrent_operations,
    bench_state_transitions,
);

// ============================================================================
// ENHANCED BENCHMARKS - Additional Performance Tests
// ============================================================================

// Benchmark message throughput
fn bench_message_throughput(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("message_throughput");

    for msg_count in [100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(msg_count),
            msg_count,
            |b, &msg_count| {
                b.to_async(&runtime).iter(|| async move {
                    let config = ServerConfig::default();
                    let metrics = Arc::new(ServerMetrics::new());
                    let worker_config = WorkerConfig {
                        read_timeout: config.read_timeout,
                        idle_timeout: config.idle_timeout,
                        write_timeout: config.write_timeout,
                        control_buffer_size: 100,
                    };
                    let manager = ConnectionManager::new(metrics, worker_config);

                    let (server, _client) = create_test_connection().await;
                    let id = ConnectionId::new(1);
                    let connection = TelnetConnection::wrap(server, id).unwrap();
                    manager
                        .add_connection(connection, Arc::new(BenchHandler))
                        .unwrap();

                    // Send multiple messages
                    for _ in 0..msg_count {
                        let _ = manager
                            .send_to_connection(id, TerminalCommand::EraseLine)
                            .await;
                    }

                    manager.shutdown().await;
                });
            },
        );
    }
    group.finish();
}

// Benchmark connection lifecycle overhead
fn bench_connection_lifecycle(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("full_connection_lifecycle", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = ServerConfig::default();
            let metrics = Arc::new(ServerMetrics::new());
            let worker_config = WorkerConfig {
                read_timeout: config.read_timeout,
                idle_timeout: config.idle_timeout,
                write_timeout: config.write_timeout,
                control_buffer_size: 100,
            };
            let manager = ConnectionManager::new(metrics, worker_config);

            // Create connection
            let (server, _client) = create_test_connection().await;
            let id = ConnectionId::new(1);
            let connection = TelnetConnection::wrap(server, id).unwrap();

            // Add connection
            let conn_id = manager
                .add_connection(connection, Arc::new(BenchHandler))
                .unwrap();

            // Remove connection
            let _ = manager.remove_connection(conn_id).await;

            manager.shutdown().await;
        });
    });
}

// Benchmark metadata operations
fn bench_metadata_operations(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("connection_info_queries", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = ServerConfig::default();
            let metrics = Arc::new(ServerMetrics::new());
            let worker_config = WorkerConfig {
                read_timeout: config.read_timeout,
                idle_timeout: config.idle_timeout,
                write_timeout: config.write_timeout,
                control_buffer_size: 100,
            };
            let manager = ConnectionManager::new(metrics, worker_config);

            // Add connections
            let mut clients = Vec::new();
            for i in 0..10 {
                let (server, client) = create_test_connection().await;
                let id = ConnectionId::new(i);
                let connection = TelnetConnection::wrap(server, id).unwrap();
                manager
                    .add_connection(connection, Arc::new(BenchHandler))
                    .unwrap();
                clients.push(client);
            }

            // Query all connection info
            let _infos = manager.get_all_connection_infos();
            let _ids = manager.get_connection_ids();
            let _count = manager.connection_count();

            manager.shutdown().await;
            drop(clients);
        });
    });
}

// Benchmark filtered broadcast
fn bench_filtered_broadcast(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("broadcast_filtered", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = ServerConfig::default();
            let metrics = Arc::new(ServerMetrics::new());
            let worker_config = WorkerConfig {
                read_timeout: config.read_timeout,
                idle_timeout: config.idle_timeout,
                write_timeout: config.write_timeout,
                control_buffer_size: 100,
            };
            let manager = ConnectionManager::new(metrics, worker_config);

            // Add connections
            let mut clients = Vec::new();
            for i in 0..50 {
                let (server, client) = create_test_connection().await;
                let id = ConnectionId::new(i);
                let connection = TelnetConnection::wrap(server, id).unwrap();
                manager
                    .add_connection(connection, Arc::new(BenchHandler))
                    .unwrap();
                clients.push(client);
            }

            tokio::time::sleep(Duration::from_millis(100)).await;

            // Broadcast with filter (only even IDs)
            let result = manager
                .broadcast_filtered(TerminalCommand::EraseLine, |info| info.id.as_u64() % 2 == 0)
                .await;
            black_box(result);

            // Give time for broadcast to complete
            tokio::time::sleep(Duration::from_millis(50)).await;

            manager.shutdown().await;
            drop(clients);
        });
    });
}

// Benchmark broadcast except
fn bench_broadcast_except(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("broadcast_except", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = ServerConfig::default();
            let metrics = Arc::new(ServerMetrics::new());
            let worker_config = WorkerConfig {
                read_timeout: config.read_timeout,
                idle_timeout: config.idle_timeout,
                write_timeout: config.write_timeout,
                control_buffer_size: 100,
            };
            let manager = ConnectionManager::new(metrics, worker_config);

            // Add connections
            let mut clients = Vec::new();
            let mut exclude_ids = Vec::new();
            for i in 0..50 {
                let (server, client) = create_test_connection().await;
                let id = ConnectionId::new(i);
                let connection = TelnetConnection::wrap(server, id).unwrap();
                manager
                    .add_connection(connection, Arc::new(BenchHandler))
                    .unwrap();
                clients.push(client);

                // Exclude first 5 connections
                if i < 5 {
                    exclude_ids.push(id);
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;

            // Broadcast except excluded IDs
            let result = manager
                .broadcast_except(TerminalCommand::EraseLine, &exclude_ids)
                .await;
            black_box(result);

            // Give time for broadcast to complete
            tokio::time::sleep(Duration::from_millis(50)).await;

            manager.shutdown().await;
            drop(clients);
        });
    });
}

// Benchmark memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("connection_churn", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = ServerConfig::default();
            let metrics = Arc::new(ServerMetrics::new());
            let worker_config = WorkerConfig {
                read_timeout: config.read_timeout,
                idle_timeout: config.idle_timeout,
                write_timeout: config.write_timeout,
                control_buffer_size: 100,
            };
            let manager = ConnectionManager::new(metrics, worker_config);

            // Simulate connection churn
            for i in 0..20 {
                let (server, _client) = create_test_connection().await;
                let id = ConnectionId::new(i);
                let connection = TelnetConnection::wrap(server, id).unwrap();
                let conn_id = manager
                    .add_connection(connection, Arc::new(BenchHandler))
                    .unwrap();

                // Immediately remove
                let _ = manager.remove_connection(conn_id).await;
            }

            manager.shutdown().await;
        });
    });
}

// Benchmark metrics snapshot performance
fn bench_metrics_snapshot(c: &mut Criterion) {
    let metrics = Arc::new(ServerMetrics::new());

    // Populate with some data
    for _ in 0..100 {
        metrics.connection_opened();
        metrics.bytes_sent(1024);
        metrics.message_sent();
    }

    c.bench_function("metrics_snapshot_with_calculations", |b| {
        b.iter(|| {
            let snapshot = metrics.snapshot();
            black_box(snapshot.messages_sent_per_sec());
            black_box(snapshot.bytes_sent_per_sec());
            black_box(snapshot.error_rate());
        });
    });
}

// Benchmark concurrent manager access
fn bench_concurrent_manager_access(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("high_concurrency_queries", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = ServerConfig::default();
            let metrics = Arc::new(ServerMetrics::new());
            let worker_config = WorkerConfig {
                read_timeout: config.read_timeout,
                idle_timeout: config.idle_timeout,
                write_timeout: config.write_timeout,
                control_buffer_size: 100,
            };
            let manager = Arc::new(ConnectionManager::new(metrics, worker_config));

            // Add some connections
            let mut clients = Vec::new();
            for i in 0..20 {
                let (server, client) = create_test_connection().await;
                let id = ConnectionId::new(i);
                let connection = TelnetConnection::wrap(server, id).unwrap();
                manager
                    .add_connection(connection, Arc::new(BenchHandler))
                    .unwrap();
                clients.push(client);
            }

            // Spawn many concurrent queries
            let mut handles = Vec::new();
            for _ in 0..50 {
                let mgr = manager.clone();
                handles.push(tokio::spawn(async move {
                    let _count = mgr.connection_count();
                    let _ids = mgr.get_connection_ids();
                    let _infos = mgr.get_all_connection_infos();
                }));
            }

            for handle in handles {
                handle.await.unwrap();
            }

            manager.shutdown().await;
            drop(clients);
        });
    });
}

criterion_group!(
    enhanced_benches,
    bench_message_throughput,
    bench_connection_lifecycle,
    bench_metadata_operations,
    bench_filtered_broadcast,
    bench_broadcast_except,
    bench_memory_patterns,
    bench_metrics_snapshot,
    bench_concurrent_manager_access,
);

criterion_main!(benches, enhanced_benches);
