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

//! Benchmarks for the Telnet server

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::Arc;
use std::time::Duration;
use termionix_service::{
    ConnectionId, ConnectionManager, ServerConfig, ServerHandler, ServerMetrics, TelnetConnection,
    WorkerConfig,
};
use termionix_terminal::TerminalCommand;
use tokio::net::{TcpListener, TcpStream};

// Simple test handler
struct BenchHandler;

#[async_trait::async_trait]
impl ServerHandler for BenchHandler {}

// Helper to create test connections
async fn create_test_connection() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let client_task = tokio::spawn(async move { TcpStream::connect(addr).await.unwrap() });

    let (server, _) = listener.accept().await.unwrap();
    let client = client_task.await.unwrap();

    (server, client)
}

// Benchmark connection creation
fn bench_connection_creation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("connection_creation", |b| {
        b.to_async(&runtime).iter(|| async {
            let (server, _client) = create_test_connection().await;
            let id = ConnectionId::new(1);
            let connection = TelnetConnection::wrap(server, id).unwrap();
            black_box(connection);
        });
    });
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
                    tokio::time::sleep(Duration::from_millis(50)).await;

                    // Benchmark broadcast
                    let result = manager.broadcast(TerminalCommand::SendEraseLine).await;
                    black_box(result);

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
    use termionix_service::ConnectionState;
    use std::sync::atomic::{AtomicU8, Ordering};

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

criterion_main!(benches);


