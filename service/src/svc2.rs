use crate::{Connection, TelnetServer};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use termionix_ansicodes::SegmentedString;
use termionix_terminal::{CursorPosition, TerminalError, TerminalEvent};
use tokio::net::TcpListener;
use tokio::runtime::Handle;
use tracing::trace;

trait AppState: Send + Sync + 'static {}
impl<T> AppState for T where T: Send + Sync + 'static {}

type OnServiceStartupHandler<S: AppState> = Box<dyn Fn(&S) + Send + 'static>;
type OnServiceShutdownHandler<S: AppState> = Box<dyn Fn(&S) + Send + 'static>;
type OnTerminalConnectHandler<S: AppState> = Box<dyn Fn(&S, Connection) + Send + 'static>;
type OnTerminalDisconnectHandler<S: AppState> = Box<dyn Fn(&S, Connection) + Send + 'static>;
type OnTerminalTimeoutHandler<S: AppState> = Box<dyn Fn(&S, Connection) + Send + 'static>;
type OnTerminalErrorHandler<S: AppState> = Box<dyn Fn(&S, Connection, TerminalError) + Send + 'static>;
type OnTerminalEventHandler<S: AppState> = Box<dyn Fn(&S, Connection, TerminalEvent) + Send + 'static>;
type OnTerminalMessageHandler<S: AppState> = Box<dyn Fn(&S, Connection, SegmentedString) + Send + 'static>;
type OnTerminalCursorUpdateHandler<S: AppState> =
    Box<dyn Fn(&S, Connection, CursorPosition) + Send + 'static>;

pub struct TelnetService<S = ()> {
    connections: HashMap<SocketAddr, Connection>,
    listener: TcpListener,
    active: AtomicBool,
    handle: Handle,
    appstate: S,
    // Service Handlers
    on_startup: Option<OnServiceStartupHandler<S>>,
    on_shutdown: Option<OnServiceShutdownHandler<S>>,
    // Connection Handlers
    on_connect: Option<OnTerminalConnectHandler<S>>,
    on_disconnect: Option<OnTerminalDisconnectHandler<S>>,
    on_timeout: Option<OnTerminalTimeoutHandler<S>>,
    on_error: Option<OnTerminalErrorHandler<S>>,
    on_event: Option<OnTerminalEventHandler<S>>,
}

impl<S> TelnetService<S> {
    pub fn new(listener: TcpListener) -> TelnetService<S> {}
}
