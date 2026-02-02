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

//! Lock-free metrics for the  Telnet server

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Lock-free server metrics
///
/// All metrics are stored as atomics and can be accessed concurrently
/// without locks. Use the `snapshot()` method to get a consistent view
/// of all metrics at a point in time.
#[derive(Debug)]
pub struct ServerMetrics {
    // Connection counts
    total_connections: AtomicU64,
    active_connections: AtomicU64,

    // Throughput
    bytes_sent: AtomicU64,
    bytes_received: AtomicU64,
    messages_sent: AtomicU64,
    messages_received: AtomicU64,

    // Errors
    connection_errors: AtomicU64,
    protocol_errors: AtomicU64,
    timeout_errors: AtomicU64,

    // Timing (stored as nanoseconds)
    total_connection_duration_ns: AtomicU64,

    // Server start time
    started_at: Instant,
}

impl Default for ServerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerMetrics {
    /// Create a new metrics instance
    pub fn new() -> Self {
        Self {
            total_connections: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            connection_errors: AtomicU64::new(0),
            protocol_errors: AtomicU64::new(0),
            timeout_errors: AtomicU64::new(0),
            total_connection_duration_ns: AtomicU64::new(0),
            started_at: Instant::now(),
        }
    }

    // Connection tracking

    /// Record a new connection being opened
    pub fn connection_opened(&self) {
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a connection being closed
    pub fn connection_closed(&self, duration: Duration) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
        self.total_connection_duration_ns
            .fetch_add(duration.as_nanos() as u64, Ordering::Relaxed);
    }

    /// Get the current number of active connections
    pub fn active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Get the total number of connections since server start
    pub fn total_connections(&self) -> u64 {
        self.total_connections.load(Ordering::Relaxed)
    }

    // Throughput tracking

    /// Record bytes sent
    pub fn bytes_sent(&self, count: u64) {
        self.bytes_sent.fetch_add(count, Ordering::Relaxed);
    }

    /// Record bytes received
    pub fn bytes_received(&self, count: u64) {
        self.bytes_received.fetch_add(count, Ordering::Relaxed);
    }

    /// Record a message sent
    pub fn message_sent(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a message received
    pub fn message_received(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    // Error tracking

    /// Record a connection error
    pub fn connection_error(&self) {
        self.connection_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a sidechannel error
    pub fn protocol_error(&self) {
        self.protocol_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a timeout error
    pub fn timeout_error(&self) {
        self.timeout_errors.fetch_add(1, Ordering::Relaxed);
    }

    // Snapshot

    /// Get a consistent snapshot of all metrics
    ///
    /// This creates a point-in-time view of all metrics. Note that the
    /// snapshot may not be perfectly consistent if metrics are being
    /// updated concurrently, but it will be close enough for monitoring
    /// purposes.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_connections: self.total_connections.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            bytes_received: self.bytes_received.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            connection_errors: self.connection_errors.load(Ordering::Relaxed),
            protocol_errors: self.protocol_errors.load(Ordering::Relaxed),
            timeout_errors: self.timeout_errors.load(Ordering::Relaxed),
            uptime: self.started_at.elapsed(),
            avg_connection_duration: self.average_connection_duration(),
        }
    }

    fn average_connection_duration(&self) -> Duration {
        let total = self.total_connections.load(Ordering::Relaxed);
        if total == 0 {
            return Duration::ZERO;
        }
        let total_ns = self.total_connection_duration_ns.load(Ordering::Relaxed);
        Duration::from_nanos(total_ns / total)
    }
}

/// A snapshot of server metrics at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    /// Total connections since server start
    pub total_connections: u64,
    /// Current active connections
    pub active_connections: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total connection errors
    pub connection_errors: u64,
    /// Total sidechannel errors
    pub protocol_errors: u64,
    /// Total timeout errors
    pub timeout_errors: u64,
    /// Server uptime
    pub uptime: Duration,
    /// Average connection duration
    pub avg_connection_duration: Duration,
}

impl MetricsSnapshot {
    /// Calculate messages per second (sent)
    pub fn messages_sent_per_sec(&self) -> f64 {
        if self.uptime.is_zero() {
            return 0.0;
        }
        self.messages_sent as f64 / self.uptime.as_secs_f64()
    }

    /// Calculate messages per second (received)
    pub fn messages_received_per_sec(&self) -> f64 {
        if self.uptime.is_zero() {
            return 0.0;
        }
        self.messages_received as f64 / self.uptime.as_secs_f64()
    }

    /// Calculate bytes per second (sent)
    pub fn bytes_sent_per_sec(&self) -> f64 {
        if self.uptime.is_zero() {
            return 0.0;
        }
        self.bytes_sent as f64 / self.uptime.as_secs_f64()
    }

    /// Calculate bytes per second (received)
    pub fn bytes_received_per_sec(&self) -> f64 {
        if self.uptime.is_zero() {
            return 0.0;
        }
        self.bytes_received as f64 / self.uptime.as_secs_f64()
    }

    /// Calculate total error count
    pub fn total_errors(&self) -> u64 {
        self.connection_errors + self.protocol_errors + self.timeout_errors
    }

    /// Calculate error rate (errors per second)
    pub fn error_rate(&self) -> f64 {
        if self.uptime.is_zero() {
            return 0.0;
        }
        self.total_errors() as f64 / self.uptime.as_secs_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_connection_tracking() {
        let metrics = ServerMetrics::new();

        assert_eq!(metrics.active_connections(), 0);
        assert_eq!(metrics.total_connections(), 0);

        metrics.connection_opened();
        assert_eq!(metrics.active_connections(), 1);
        assert_eq!(metrics.total_connections(), 1);

        metrics.connection_opened();
        assert_eq!(metrics.active_connections(), 2);
        assert_eq!(metrics.total_connections(), 2);

        metrics.connection_closed(Duration::from_secs(10));
        assert_eq!(metrics.active_connections(), 1);
        assert_eq!(metrics.total_connections(), 2);
    }

    #[test]
    fn test_throughput_tracking() {
        let metrics = ServerMetrics::new();

        metrics.bytes_sent(100);
        metrics.bytes_received(200);
        metrics.message_sent();
        metrics.message_received();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.bytes_sent, 100);
        assert_eq!(snapshot.bytes_received, 200);
        assert_eq!(snapshot.messages_sent, 1);
        assert_eq!(snapshot.messages_received, 1);
    }

    #[test]
    fn test_error_tracking() {
        let metrics = ServerMetrics::new();

        metrics.connection_error();
        metrics.protocol_error();
        metrics.timeout_error();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.connection_errors, 1);
        assert_eq!(snapshot.protocol_errors, 1);
        assert_eq!(snapshot.timeout_errors, 1);
        assert_eq!(snapshot.total_errors(), 3);
    }

    #[test]
    fn test_concurrent_updates() {
        let metrics = std::sync::Arc::new(ServerMetrics::new());
        let mut handles = vec![];

        // Spawn multiple threads updating metrics
        for _ in 0..10 {
            let metrics = metrics.clone();
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    metrics.connection_opened();
                    metrics.bytes_sent(10);
                    metrics.message_sent();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Check final counts
        assert_eq!(metrics.total_connections(), 1000);
        assert_eq!(metrics.active_connections(), 1000);
        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.bytes_sent, 10000);
        assert_eq!(snapshot.messages_sent, 1000);
    }
}
