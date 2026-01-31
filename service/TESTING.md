# Termionix Service - Testing Documentation

## Overview

This document describes the comprehensive test suite for the termionix-service crate, including integration tests, benchmarks, and testing strategies.

## Test Structure

### Integration Tests (`tests/integration_tests.rs`)

The integration test suite is organized into several categories:

#### 1. Basic Server Lifecycle Tests
- **test_server_accepts_connections**: Verifies server can accept TCP connections
- **test_server_handles_multiple_connections**: Tests handling of multiple concurrent connections
- **test_server_enforces_connection_limit**: Validates max connection limit enforcement
- **test_server_graceful_shutdown**: Tests graceful shutdown with active connections
- **test_server_restart**: Verifies server can be restarted after shutdown

#### 2. Connection Management Tests
- **test_connection_receives_data**: Tests data reception from clients
- **test_concurrent_connections**: Validates handling of many concurrent connections (20+)
- **test_connection_state_tracking**: Verifies connection state transitions
- **test_connection_metadata**: Tests connection info retrieval
- **test_connection_user_data**: Validates user-defined metadata storage/retrieval

#### 3. Communication Tests
- **test_client_server_conversation**: Tests bidirectional communication
- **test_multiple_clients_conversation**: Validates multiple clients communicating simultaneously
- **test_echo_conversation**: Tests character-by-character echo functionality
- **test_sequential_commands**: Verifies sequential command processing
- **test_burst_messages**: Tests handling of rapid message bursts (10 messages)
- **test_large_message_handling**: Validates processing of large messages (1000+ chars)
- **test_empty_message_handling**: Tests empty line handling
- **test_special_characters**: Validates special character processing

#### 4. Broadcast Tests
- **test_broadcast_to_connections**: Tests broadcasting to all connections
- **test_broadcast_during_conversation**: Validates broadcast during active conversations
- **test_broadcast_filtered**: Tests filtered broadcast (conditional)
- **test_broadcast_except**: Tests broadcast excluding specific connections

#### 5. Metrics & Monitoring Tests
- **test_server_metrics**: Validates metrics tracking accuracy
- **test_metrics_accuracy**: Tests detailed metrics calculations

#### 6. Timeout & Error Handling Tests
- **test_connection_timeout**: Tests idle timeout enforcement (marked as flaky)

#### 7. Stress & Edge Case Tests
- **test_rapid_connection_cycling**: Tests rapid connect/disconnect cycles (20 iterations)
- **test_connection_limit_enforcement**: Validates strict connection limit enforcement

## Benchmarks (`benches/server_benchmarks.rs`)

### Core Performance Benchmarks

#### Connection Management
- **bench_connection_creation**: Measures connection creation overhead
- **bench_connection_lifecycle**: Measures full connection lifecycle (create → add → remove)
- **bench_manager_add_connection**: Tests connection manager add operation

#### Broadcast Performance
- **bench_broadcast_scaling**: Tests broadcast performance with varying connection counts (10, 50, 100, 500)
- **bench_filtered_broadcast**: Measures filtered broadcast performance (50 connections)
- **bench_broadcast_except**: Tests broadcast with exclusion list (50 connections, 5 excluded)

#### Metrics Performance
- **bench_metrics_updates**: Tests atomic metrics update performance
  - connection_opened
  - bytes_sent
  - snapshot creation
- **bench_metrics_snapshot**: Measures snapshot with calculations

#### Concurrency Benchmarks
- **bench_concurrent_operations**: Tests concurrent connection queries (100 queries, 10 connections)
- **bench_concurrent_manager_access**: Tests high concurrency (50 concurrent queries, 20 connections)

#### State Management
- **bench_state_transitions**: Measures atomic state transition performance

#### Enhanced Benchmarks
- **bench_message_throughput**: Tests message sending throughput (100, 500, 1000 messages)
- **bench_metadata_operations**: Measures connection info query performance
- **bench_memory_patterns**: Tests connection churn patterns (20 rapid add/remove cycles)

## Test Coverage Areas

### ✅ Well Covered
- Server lifecycle (start, stop, restart)
- Connection acceptance and management
- Basic communication (send/receive)
- Broadcast operations (all, filtered, except)
- Metrics tracking
- Connection state management
- User data storage
- Concurrent operations

### ⚠️ Partially Covered
- Timeout handling (tests marked as flaky due to timing sensitivity)
- Resource exhaustion scenarios (some covered in stress tests)

### ✅ Now Fully Covered (New Test Files Added)

#### Protocol Tests (`tests/protocol_tests.rs`) - 11 tests
- Telnet IAC escape sequences
- WILL/WONT/DO/DONT command handling
- NAWS (window size) negotiation
- Terminal type negotiation
- Suppress Go-Ahead option
- Mixed telnet commands and text
- Incomplete and fragmented commands

#### Error Recovery Tests (`tests/error_recovery_tests.rs`) - 11 tests
- Abrupt client disconnections
- Multiple rapid disconnects
- Partial write handling
- Connection during shutdown
- Server restart after errors
- Broadcast with failed connections
- Connection limit recovery
- Concurrent error handling
- Metrics accuracy after errors

#### Security Tests (`tests/security_tests.rs`) - 15 tests
- Null byte handling
- Binary data processing
- Extremely long lines (10KB+)
- Rapid small writes (DoS vector)
- Malformed UTF-8 sequences
- Control character handling
- ANSI escape sequence flooding
- Incomplete ANSI sequences
- Connection spam attempts
- Zero-byte writes
- Mixed valid/invalid data
- Repeated newlines
- Line ending variations (CRLF, LF, CR)
- Unicode edge cases (BOM, zero-width spaces)

#### Memory Tests (`tests/memory_tests.rs`) - 11 tests
- Sustained connection churn (100 cycles)
- Connection cleanup verification
- Metrics memory stability (1000+ connections)
- Broadcast memory stability
- User data cleanup
- Handler Arc reference cleanup
- Manager memory after shutdown
- High connection count stability (200+ connections)
- Repeated server lifecycle (10 cycles)
- Connection info query memory

### 🔴 Future Enhancements
- Compression stream behavior testing
- Full RFC compliance validation
- Fuzzing integration
- Property-based testing with proptest

## Running Tests

### Run All Tests
```bash
cd service
cargo test
```

### Run Specific Test
```bash
cargo test test_server_accepts_connections
```

### Run Integration Tests Only
```bash
cargo test --test integration_tests
```

### Run with Output
```bash
cargo test -- --nocapture
```

### Run Ignored Tests
```bash
cargo test -- --ignored
```

## Running Benchmarks

### Run All Benchmarks
```bash
cd service
cargo bench
```

### Run Specific Benchmark
```bash
cargo bench bench_broadcast_scaling
```

### Run Benchmark Group
```bash
cargo bench broadcast
```

### Generate Benchmark Report
```bash
cargo bench -- --save-baseline my-baseline
```

## Test Patterns & Best Practices

### 1. Test Handler Pattern
Tests use custom handlers to track events:
```rust
struct TestHandler {
    connect_count: Arc<AtomicUsize>,
    event_count: Arc<AtomicUsize>,
    disconnect_count: Arc<AtomicUsize>,
}
```

### 2. Timing Considerations
- Use `tokio::time::sleep()` to allow async operations to complete
- Tests with strict timing requirements are marked with `#[ignore]`
- Typical wait times: 50-200ms for connection setup, 100-300ms for message processing

### 3. Resource Cleanup
- Always call `server.shutdown().await` in tests
- Drop client connections explicitly
- Use `tokio::time::sleep()` after drops to allow cleanup

### 4. Concurrent Testing
- Use `tokio::spawn()` for concurrent operations
- Collect handles and await all before assertions
- Use appropriate connection limits for test scale

## Performance Baselines

### Expected Performance (Development Machine)
- Connection creation: < 1ms
- Broadcast to 100 connections: < 10ms
- Metrics snapshot: < 1μs
- State transition: < 100ns
- Message throughput: > 10,000 msg/sec per connection

### Regression Detection
Monitor these metrics for performance regressions:
1. Connection creation time
2. Broadcast latency vs connection count
3. Memory usage during connection churn
4. Concurrent query throughput

## Known Issues & Limitations

### Flaky Tests
- `test_server_graceful_shutdown`: Timing-sensitive, may fail in CI
- `test_connection_timeout`: Depends on precise timing, marked as ignored

### Platform Differences
- Windows: May have different TCP behavior
- Linux: Better performance for high connection counts
- macOS: May have different timeout behavior

## Future Test Enhancements

### Planned Additions
1. **Chaos Testing**: Random disconnections, network delays
2. **Load Testing**: Sustained high connection counts (1000+)
3. **Memory Profiling**: Detect leaks under various scenarios
4. **Protocol Compliance**: Telnet RFC compliance tests
5. **Security Testing**: Malformed input, buffer overflow attempts
6. **Performance Regression**: Automated baseline comparison

### Test Infrastructure Improvements
1. Test fixtures for common scenarios
2. Mock network layer for deterministic testing
3. Property-based testing with proptest
4. Fuzzing integration
5. CI/CD integration with performance tracking

## Contributing

When adding new tests:
1. Follow existing naming conventions (`test_*` for tests)
2. Add appropriate documentation comments
3. Consider timing requirements and mark flaky tests
4. Update this document with new test categories
5. Ensure tests clean up resources properly
6. Add corresponding benchmarks for performance-critical paths

## Test Maintenance

### Regular Tasks
- Review and update flaky test timeouts
- Update performance baselines quarterly
- Remove obsolete tests when refactoring
- Keep test documentation synchronized with code

### Before Release
- Run full test suite including ignored tests
- Run benchmarks and compare to baselines
- Review test coverage reports
- Update this documentation