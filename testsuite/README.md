# Termionix Integration Test Suite

This crate contains comprehensive integration benchmarks and tests for the Termionix project.

## Overview

The integration benchmark suite tests the full server-client stack over loopback connections, measuring:

- **Performance**: Throughput, latency, and scalability
- **Efficiency**: Resource usage and connection management
- **Correctness**: Message delivery and protocol compliance

## Benchmarks

### Single Client Echo Test
Tests basic echo functionality with a single client sending 100 messages to the server.

### Multiple Concurrent Clients
Scales from 5 to 50 concurrent clients, each sending 20 messages simultaneously to test server capacity and fairness.

### Throughput Test
Measures data throughput with varying message sizes (64, 256, 1024, 4096 bytes) to identify optimal packet sizes and potential bottlenecks.

### Connection Churn
Tests rapid connection/disconnection cycles (50 iterations) to verify proper resource cleanup and connection lifecycle management.

### Latency Test
Measures round-trip latency for single messages to establish baseline performance characteristics.

### Stress Test
Simulates high load with 100 concurrent clients each sending multiple messages to test system stability under stress.

### Correctness Verification
Validates that all sent messages are received correctly by comparing message counts between server and client.

## Running the Benchmarks

```bash
# Run all benchmarks
cargo bench --package termionix-testsuite

# Run specific benchmark
cargo bench --package termionix-testsuite --bench benchmarks -- single_client_echo

# Generate HTML report
cargo bench --package termionix-testsuite -- --save-baseline my-baseline
```

## Benchmark Results

Results are saved in `target/criterion/` with detailed HTML reports including:
- Performance graphs
- Statistical analysis
- Comparison with previous runs
- Regression detection

## Architecture

The benchmark suite uses:
- **Server**: Echo server that reflects all received data back to clients
- **Clients**: Multiple concurrent client connections over loopback (127.0.0.1)
- **Metrics**: Atomic counters for messages and bytes sent/received
- **Synchronization**: Barriers for coordinated concurrent testing

## Test Handlers

### EchoServerHandler
- Tracks messages and bytes received
- Echoes character data and completed lines back to clients
- Thread-safe atomic counters for statistics

### BenchClientHandler
- Tracks messages and bytes received from server
- Provides connection/disconnection notifications
- Collects performance metrics

## Future Enhancements

Potential additions to the benchmark suite:
- Protocol negotiation benchmarks (NAWS, TTYPE, etc.)
- Compression performance tests (MCCP)
- Large message handling (>64KB)
- Network simulation (latency, packet loss)
- Memory profiling and leak detection
- Long-running stability tests

## Notes

- All tests run over loopback to ensure consistent, reproducible results
- Server binds to port 0 (random available port) to avoid conflicts
- Proper cleanup ensures no resource leaks between benchmark runs
- Timeouts prevent hanging on connection failures