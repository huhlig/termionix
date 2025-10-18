//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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

use crate::{TelnetError, TelnetResult};
use futures_util::{SinkExt, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use termionix_compress::{Algorithm, CompressionStream};
use termionix_terminal::{TerminalCodec, TerminalEvent};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::codec::{Encoder, Framed};
use tokio_util::sync::CancellationToken;
use tracing::error;

/// Connection ID
pub type TelnetConnectionId = usize;

/// A shared, thread-safe Telnet connection wrapper.
///
/// `TelnetConnection` owns a framed, optionally compressed TCP stream that
/// speaks in terms of `TerminalEvent`s. It is internally reference-counted
/// (`Arc`) and synchronized (`Mutex`) so it can be safely cloned and shared
/// across tasks and handlers.
///
/// The second field is a simple numeric identifier that can be used by
/// higher-level components (e.g. server / client managers) to distinguish
/// connections.
///
/// # Concurrency
///
/// The underlying framed stream is protected by a `Mutex`. All operations that
/// access the I/O state (`next`, `send_message`, and the closure helpers)
/// acquire the lock. This keeps the stream consistent but also means:
///
/// - Holding the lock for long periods will block other users of the same
///   connection.
/// - You should avoid doing expensive work while holding the lock; use the
///   provided closure helpers to *extract* data you need, then release it.
///
/// # Compression
///
/// The I/O is wrapped in a [`CompressionStream`] so that different compression
/// algorithms can be used transparently. The initial algorithm is `Algorithm::None`
/// (no compression), but the underlying stream can be reconfigured if needed.

#[derive(Clone)]
pub struct TelnetConnection {
    framed: Arc<Mutex<Framed<CompressionStream<TcpStream>, TerminalCodec>>>,
    pub(crate) handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    token: CancellationToken,
    id: TelnetConnectionId,
}

impl TelnetConnection {
    /// Wraps a raw `TcpStream` into a `TelnetConnection`.
    ///
    /// This constructs a framed terminal stream with no compression and
    /// associates it with the provided connection identifier.
    ///
    /// # Parameters
    ///
    /// * `socket` - The connected TCP stream.
    /// * `id` - A numeric identifier for this connection (e.g. sequence number).
    ///
    /// # Errors
    ///
    /// Returns a [`TelnetError`] if the internal framing or compression setup
    /// fails.
    pub fn wrap(
        socket: TcpStream,
        token: CancellationToken,
        id: TelnetConnectionId,
    ) -> TelnetConnection {
        TelnetConnection {
            framed: Arc::new(Mutex::new(Framed::new(
                CompressionStream::new(socket, Algorithm::None),
                TerminalCodec::new(),
            ))),
            handle: Default::default(),
            token,
            id,
        }
    }

    /// Cancel Reader Thread
    pub fn cancel(&self) {
        self.token.cancel()
    }

    /// Returns the numeric identifier associated with this connection.
    ///
    /// This is useful for logging, metrics, or mapping external state (such as
    /// user sessions) to a particular connection.
    pub fn id(&self) -> TelnetConnectionId {
        self.id
    }

    /// Calls the provided closure with the remote peer address.
    ///
    /// This is a convenience helper that hides the nested wrapper types and
    /// locking needed to reach the underlying `TcpStream`.
    ///
    /// # Parameters
    ///
    /// * `closure` - A function that receives the `SocketAddr` of the peer.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    ///
    /// - The internal `Mutex` is poisoned.
    /// - The underlying socket does not expose a peer address.
    pub async fn address<F>(&self, closure: F) -> TelnetResult<()>
    where
        F: FnOnce(SocketAddr),
    {
        Ok(closure(
            self.framed.lock().await.get_ref().get_ref().peer_addr()?,
        ))
    }

    /// Executes a closure with mutable access to the underlying framed stream.
    ///
    /// This gives direct access to the `Framed<CompressionStream<TcpStream>, TerminalCodec>`
    /// value for advanced use cases (custom reads/writes, configuration, etc.).
    ///
    /// # Parameters
    ///
    /// * `closure` - A function that receives a mutable reference to the framed stream.
    ///
    /// # Errors
    ///
    /// Returns an error if the internal `Mutex` is poisoned.
    ///
    /// # Warning
    ///
    /// Misusing the framed stream (e.g. mixing manual reads/writes with
    /// `next`/`send_message`) can break protocol invariants. Prefer using the
    /// high-level APIs where possible.
    pub async fn framed<F>(&self, closure: F) -> TelnetResult<()>
    where
        F: FnOnce(&mut Framed<CompressionStream<TcpStream>, TerminalCodec>),
    {
        Ok(closure(&mut *self.framed.lock().await))
    }

    /// Executes a closure with mutable access to the underlying compression stream.
    ///
    /// This is useful if you need to inspect or modify compression settings on
    /// the fly (e.g. switching algorithms).
    ///
    /// # Parameters
    ///
    /// * `closure` - A function that receives a mutable reference to the `CompressionStream`.
    ///
    /// # Errors
    ///
    /// Returns an error if the internal `Mutex` is poisoned.
    pub async fn compression_stream<F>(&self, closure: F) -> TelnetResult<()>
    where
        F: FnOnce(&mut CompressionStream<TcpStream>),
    {
        Ok(closure(&mut *self.framed.lock().await.get_mut()))
    }

    /// Executes a closure with mutable access to the terminal codec.
    ///
    /// This can be used to tweak codec-level behavior or inspect internal
    /// terminal parsing state.
    ///
    /// # Parameters
    ///
    /// * `closure` - A function that receives a mutable reference to the `TerminalCodec`.
    ///
    /// # Errors
    ///
    /// Returns an error if the internal `Mutex` is poisoned.
    pub async fn terminal<F>(&self, closure: F) -> TelnetResult<()>
    where
        F: FnOnce(&mut TerminalCodec),
    {
        Ok(closure(&mut *self.framed.lock().await.codec_mut()))
    }

    /// Sends a message to the remote terminal.
    ///
    /// The bytes are passed through the terminal codec and written over the
    /// underlying (optionally compressed) TCP stream.
    ///
    /// # Parameters
    ///
    /// * `msg` - The message to send, typically a line or terminal control
    ///   sequence.
    ///
    /// # Errors
    ///
    /// Returns a [`TelnetError`] if the message cannot be encoded or written,
    /// or if the internal `Mutex` is poisoned.
    pub async fn send<M>(&mut self, msg: M) -> TelnetResult<()>
    where
        TerminalCodec: Encoder<M>,
        TelnetError: From<<TerminalCodec as Encoder<M>>::Error>,
    {
        self.framed
            .lock()
            .await
            .send(msg)
            .await
            .map_err(TelnetError::from)
    }

    /// Reads the next terminal event from the connection.
    ///
    /// This asynchronously waits for the next `TerminalEvent` produced by the
    /// codec. It transparently handles end-of-stream and error conversion.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(event))` - A decoded terminal event.
    /// * `Ok(None)` - The remote side has closed the connection.
    /// * `Err(err)` - An error occurred while reading or decoding.
    pub async fn next(&mut self) -> TelnetResult<Option<TerminalEvent>> {
        match self.framed.lock().await.next().await {
            Some(result) => match result {
                Ok(event) => Ok(Some(event)),
                Err(err) => Err(err.into()),
            },
            None => Ok(None),
        }
    }

    pub async fn disconnect(&self) {
        self.token.cancel();
        if let Some(handle) = self.handle.lock().await.take() {
            match handle.await {
                Ok(_) => {}
                Err(err) => {
                    error!("Disconnect Error: {err}");
                }
            }
        }
    }
}

impl std::fmt::Debug for TelnetConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelnetConnection").finish()
    }
}
