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

//! Benchmarks for terminal service operations
//!
//! TODO: Move socket/codec creation OUTSIDE the hot loop for most benchmarks. Create a separate new connection benchmark.

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use termionix_ansicodec::{AnsiCodec, AnsiConfig};
use termionix_service::{
    ClientConnectionConfig, ConnectionConfig, FlushStrategy, ServerConnectionConfig,
    SplitTerminalConnection,
};
use termionix_telnetcodec::TelnetCodec;
use termionix_terminal::{TerminalCodec, TerminalCommand};
use tokio::io::duplex;
use tokio::runtime::Runtime;

fn create_codec() -> TerminalCodec<AnsiCodec<TelnetCodec>> {
    TerminalCodec::new(AnsiCodec::new(
        AnsiConfig::default(),
        TelnetCodec::default(),
    ))
}

fn bench_config_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_creation");

    group.bench_function("connection_config_default", |b| {
        b.iter(|| black_box(ConnectionConfig::default()))
    });

    group.bench_function("connection_config_builder", |b| {
        b.iter(|| {
            black_box(
                ConnectionConfig::default()
                    .with_terminal_type("xterm-256color")
                    .with_terminal_size(120, 40)
                    .with_buffer_size(8192),
            )
        })
    });

    group.bench_function("client_config_new", |b| {
        b.iter(|| black_box(ClientConnectionConfig::new("localhost", 23)))
    });

    group.bench_function("client_config_builder", |b| {
        b.iter(|| {
            black_box(
                ClientConnectionConfig::new("localhost", 23)
                    .with_auto_reconnect(true)
                    .with_terminal_size(120, 40),
            )
        })
    });

    group.bench_function("server_config_new", |b| {
        b.iter(|| black_box(ServerConnectionConfig::new()))
    });

    group.bench_function("server_config_builder", |b| {
        b.iter(|| {
            black_box(
                ServerConnectionConfig::new()
                    .with_rate_limiting(true, Some(100))
                    .with_terminal_size(80, 24),
            )
        })
    });

    group.finish();
}

fn bench_connection_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("connection_operations");

    group.bench_function("connection_creation", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(8192);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            black_box(SplitTerminalConnection::new(r, w, codec.clone(), codec))
        })
    });

    group.bench_function("flush_strategy_get", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(8192);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            let conn = SplitTerminalConnection::new(r, w, codec.clone(), codec);
            black_box(conn.flush_strategy().await)
        })
    });

    group.bench_function("flush_strategy_set", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(8192);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            let conn = SplitTerminalConnection::new(r, w, codec.clone(), codec);
            conn.set_flush_strategy(black_box(FlushStrategy::Immediate))
                .await
        })
    });

    group.bench_function("connection_clone", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(8192);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            let conn = SplitTerminalConnection::new(r, w, codec.clone(), codec);
            black_box(conn.clone())
        })
    });

    group.finish();
}

fn bench_send_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("send_operations");

    group.bench_function("send_small_message", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(8192);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            let conn = SplitTerminalConnection::new(r, w, codec.clone(), codec);
            conn.send(black_box(TerminalCommand::Text("Hello".to_string())), true)
                .await
                .unwrap()
        })
    });

    group.bench_function("send_medium_message", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(8192);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            let conn = SplitTerminalConnection::new(r, w, codec.clone(), codec);
            let msg = "A".repeat(1024);
            conn.send(black_box(TerminalCommand::Text(msg)), true)
                .await
                .unwrap()
        })
    });

    group.bench_function("send_large_message", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(16384);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            let conn = SplitTerminalConnection::new(r, w, codec.clone(), codec);
            let msg = "A".repeat(8192);
            conn.send(black_box(TerminalCommand::Text(msg.to_string())), true)
                .await
                .unwrap()
        })
    });

    group.bench_function("send_without_flush", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(8192);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            let conn = SplitTerminalConnection::new(r, w, codec.clone(), codec);
            conn.send(black_box(TerminalCommand::Text("Hello".to_string())), false)
                .await
                .unwrap()
        })
    });

    group.bench_function("manual_flush", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(8192);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            let conn = SplitTerminalConnection::new(r, w, codec.clone(), codec);
            conn.send(TerminalCommand::Text("Hello".to_string()), false)
                .await
                .unwrap();
            black_box(conn.flush().await.unwrap())
        })
    });

    group.finish();
}

fn bench_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("throughput");

    group.throughput(Throughput::Elements(100));
    group.bench_function("send_100_messages", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(8192);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            let conn = SplitTerminalConnection::new(r, w, codec.clone(), codec);
            for i in 0..100 {
                conn.send(
                    black_box(TerminalCommand::Text(format!("Message {}", i))),
                    true,
                )
                .await
                .unwrap();
            }
        })
    });

    group.throughput(Throughput::Bytes(1024 * 100));
    group.bench_function("send_100kb", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(16384);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            let conn = SplitTerminalConnection::new(r, w, codec.clone(), codec);
            let msg = "A".repeat(1024);
            for _ in 0..100 {
                conn.send(black_box(TerminalCommand::Text(msg.clone())), true)
                    .await
                    .unwrap();
            }
        })
    });

    group.finish();
}

fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_operations");

    group.bench_function("concurrent_sends_2_tasks", |b| {
        b.to_async(&rt).iter(|| async {
            let (stream1, _stream2) = duplex(8192);
            let codec = create_codec();
            let (r, w) = tokio::io::split(stream1);
            let conn = SplitTerminalConnection::new(r, w, codec.clone(), codec);
            let conn_clone = conn.clone();

            let task1 = tokio::spawn(async move {
                for i in 0..50 {
                    conn.send(TerminalCommand::Text(format!("Task1-{}", i)), true)
                        .await
                        .unwrap();
                }
            });

            let task2 = tokio::spawn(async move {
                for i in 0..50 {
                    conn_clone
                        .send(TerminalCommand::Text(format!("Task2-{}", i)), true)
                        .await
                        .unwrap();
                }
            });

            black_box(task1.await.unwrap());
            black_box(task2.await.unwrap());
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_config_creation,
    bench_connection_operations,
    bench_send_operations,
    bench_throughput,
    bench_concurrent_operations
);
criterion_main!(benches);


