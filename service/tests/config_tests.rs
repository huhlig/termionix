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

//! Unit tests for configuration types

use std::time::Duration;
use termionix_service::{
    ClientConnectionConfig, Config, ConnectionConfig, FlushStrategy, ServerConnectionConfig,
};

#[test]
fn test_connection_config_defaults() {
    let config = ConnectionConfig::default();

    assert_eq!(config.terminal_type, "xterm-256color");
    assert_eq!(config.terminal_width, 80);
    assert_eq!(config.terminal_height, 24);
    assert_eq!(config.buffer_size, 8192);
    assert!(config.keepalive);
    assert_eq!(config.keepalive_interval, Duration::from_secs(60));
    assert_eq!(config.read_timeout, Some(Duration::from_secs(300)));
}

#[test]
fn test_connection_config_builder() {
    let config = ConnectionConfig::default()
        .with_terminal_type("xterm")
        .with_terminal_size(120, 40)
        .with_buffer_size(16384)
        .with_keepalive(false)
        .with_keepalive_interval(Duration::from_secs(30))
        .with_read_timeout(None);

    assert_eq!(config.terminal_type, "xterm");
    assert_eq!(config.terminal_width, 120);
    assert_eq!(config.terminal_height, 40);
    assert_eq!(config.buffer_size, 16384);
    assert!(!config.keepalive);
    assert_eq!(config.keepalive_interval, Duration::from_secs(30));
    assert_eq!(config.read_timeout, None);
}

#[test]
fn test_client_config_defaults() {
    let config = ClientConnectionConfig::default();

    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 23);
    assert_eq!(config.connect_timeout, Duration::from_secs(10));
    assert!(!config.auto_reconnect);
    assert_eq!(config.reconnect_delay, Duration::from_secs(5));
    assert_eq!(config.max_reconnect_attempts, Some(3));
}

#[test]
fn test_client_config_new() {
    let config = ClientConnectionConfig::new("example.com", 8080);

    assert_eq!(config.host, "example.com");
    assert_eq!(config.port, 8080);
}

#[test]
fn test_client_config_builder() {
    let config = ClientConnectionConfig::new("test.com", 9000)
        .with_connect_timeout(Duration::from_secs(20))
        .with_auto_reconnect(true)
        .with_reconnect_delay(Duration::from_secs(10))
        .with_max_reconnect_attempts(Some(5))
        .with_terminal_type("vt100")
        .with_terminal_size(100, 30)
        .with_buffer_size(4096);

    assert_eq!(config.host, "test.com");
    assert_eq!(config.port, 9000);
    assert_eq!(config.connect_timeout, Duration::from_secs(20));
    assert!(config.auto_reconnect);
    assert_eq!(config.reconnect_delay, Duration::from_secs(10));
    assert_eq!(config.max_reconnect_attempts, Some(5));
    assert_eq!(config.common.terminal_type, "vt100");
    assert_eq!(config.common.terminal_width, 100);
    assert_eq!(config.common.terminal_height, 30);
    assert_eq!(config.common.buffer_size, 4096);
}

#[test]
fn test_client_config_address() {
    let config = ClientConnectionConfig::new("example.com", 23);
    assert_eq!(config.address(), "example.com:23");

    let config = ClientConnectionConfig::new("192.168.1.1", 8080);
    assert_eq!(config.address(), "192.168.1.1:8080");
}

#[test]
fn test_server_config_defaults() {
    let config = ServerConnectionConfig::default();

    assert_eq!(config.max_idle_time, Some(Duration::from_secs(600)));
    assert_eq!(config.max_connection_time, None);
    assert!(!config.rate_limiting);
    assert_eq!(config.max_messages_per_second, None);
}

#[test]
fn test_server_config_new() {
    let config = ServerConnectionConfig::new();

    assert_eq!(config.max_idle_time, Some(Duration::from_secs(600)));
    assert_eq!(config.max_connection_time, None);
}

#[test]
fn test_server_config_builder() {
    let config = ServerConnectionConfig::new()
        .with_max_idle_time(Some(Duration::from_secs(300)))
        .with_max_connection_time(Some(Duration::from_secs(3600)))
        .with_rate_limiting(true, Some(100))
        .with_terminal_type("ansi")
        .with_terminal_size(132, 43)
        .with_buffer_size(32768);

    assert_eq!(config.max_idle_time, Some(Duration::from_secs(300)));
    assert_eq!(config.max_connection_time, Some(Duration::from_secs(3600)));
    assert!(config.rate_limiting);
    assert_eq!(config.max_messages_per_second, Some(100));
    assert_eq!(config.common.terminal_type, "ansi");
    assert_eq!(config.common.terminal_width, 132);
    assert_eq!(config.common.terminal_height, 43);
    assert_eq!(config.common.buffer_size, 32768);
}

#[test]
fn test_config_enum_client() {
    let client_config = ClientConnectionConfig::new("localhost", 23);
    let config = Config::Client(client_config.clone());

    assert!(config.is_client());
    assert!(!config.is_server());
    assert!(config.as_client().is_some());
    assert!(config.as_server().is_none());

    let retrieved = config.as_client().unwrap();
    assert_eq!(retrieved.host, "localhost");
    assert_eq!(retrieved.port, 23);
}

#[test]
fn test_config_enum_server() {
    let server_config = ServerConnectionConfig::new();
    let config = Config::Server(server_config);

    assert!(!config.is_client());
    assert!(config.is_server());
    assert!(config.as_client().is_none());
    assert!(config.as_server().is_some());
}

#[test]
fn test_config_common_access() {
    let client_config = ClientConnectionConfig::new("localhost", 23).with_terminal_size(100, 50);
    let config = Config::Client(client_config);

    let common = config.common();
    assert_eq!(common.terminal_width, 100);
    assert_eq!(common.terminal_height, 50);
}

#[test]
fn test_config_common_mut_access() {
    let client_config = ClientConnectionConfig::new("localhost", 23);
    let mut config = Config::Client(client_config);

    config.common_mut().terminal_width = 200;
    assert_eq!(config.common().terminal_width, 200);
}

#[test]
fn test_config_from_client() {
    let client_config = ClientConnectionConfig::new("localhost", 23);
    let config: Config = client_config.into();

    assert!(config.is_client());
}

#[test]
fn test_config_from_server() {
    let server_config = ServerConnectionConfig::new();
    let config: Config = server_config.into();

    assert!(config.is_server());
}

#[test]
fn test_flush_strategy_default() {
    let strategy = FlushStrategy::default();
    assert_eq!(strategy, FlushStrategy::OnNewline);
}

#[test]
fn test_flush_strategy_variants() {
    let manual = FlushStrategy::Manual;
    let immediate = FlushStrategy::Immediate;
    let newline = FlushStrategy::OnNewline;
    let threshold = FlushStrategy::OnThreshold(1024);

    assert_eq!(manual, FlushStrategy::Manual);
    assert_eq!(immediate, FlushStrategy::Immediate);
    assert_eq!(newline, FlushStrategy::OnNewline);
    assert_eq!(threshold, FlushStrategy::OnThreshold(1024));
}

#[test]
fn test_flush_strategy_equality() {
    assert_eq!(FlushStrategy::Manual, FlushStrategy::Manual);
    assert_eq!(FlushStrategy::Immediate, FlushStrategy::Immediate);
    assert_eq!(FlushStrategy::OnNewline, FlushStrategy::OnNewline);
    assert_eq!(
        FlushStrategy::OnThreshold(100),
        FlushStrategy::OnThreshold(100)
    );

    assert_ne!(FlushStrategy::Manual, FlushStrategy::Immediate);
    assert_ne!(
        FlushStrategy::OnThreshold(100),
        FlushStrategy::OnThreshold(200)
    );
}

#[test]
fn test_client_config_unlimited_reconnects() {
    let config = ClientConnectionConfig::new("localhost", 23).with_max_reconnect_attempts(None);

    assert_eq!(config.max_reconnect_attempts, None);
}

#[test]
fn test_server_config_no_limits() {
    let config = ServerConnectionConfig::new()
        .with_max_idle_time(None)
        .with_max_connection_time(None);

    assert_eq!(config.max_idle_time, None);
    assert_eq!(config.max_connection_time, None);
}

#[test]
fn test_connection_config_clone() {
    let config1 = ConnectionConfig::default().with_terminal_size(100, 50);
    let config2 = config1.clone();

    assert_eq!(config1.terminal_width, config2.terminal_width);
    assert_eq!(config1.terminal_height, config2.terminal_height);
}

#[test]
fn test_client_config_clone() {
    let config1 = ClientConnectionConfig::new("localhost", 23);
    let config2 = config1.clone();

    assert_eq!(config1.host, config2.host);
    assert_eq!(config1.port, config2.port);
}

#[test]
fn test_server_config_clone() {
    let config1 = ServerConnectionConfig::new();
    let config2 = config1.clone();

    assert_eq!(config1.max_idle_time, config2.max_idle_time);
}

#[test]
fn test_flush_strategy_copy() {
    let strategy1 = FlushStrategy::OnThreshold(1024);
    let strategy2 = strategy1;

    assert_eq!(strategy1, strategy2);
}


