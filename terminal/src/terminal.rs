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

use crate::{TerminalBuffer, TerminalError};
use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use std::any::Any;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use termionix_telnetcodec::{TelnetCodec, TelnetFrame, TelnetOption};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;
use tracing::warn;

///
pub struct Terminal {
    metadata: HashMap<String, String>,
    buffer: TerminalBuffer,
    address: SocketAddr,
}

impl std::fmt::Debug for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Terminal")
            .field("address", &self.address)
            .field("active", &self.active)
            .finish()
    }
}

impl Terminal {
    ///
    pub async fn wrap(stream: TcpStream) -> Result<Terminal, TerminalError> {
        let terminal = Terminal {
            address: stream.peer_addr()?,
            metadata: RwLock::new(HashMap::new()),
            codec: Framed::new(stream, TelnetCodec::default()),
            buffer: RwLock::new(BytesMut::default()),
            active: AtomicBool::new(true),
        };
    }

    ///
    pub async fn wrap2(stream: TcpStream) -> Result<Terminal, TerminalError> {
        let address = stream.peer_addr()?;
        let metadata = Arc::new(RwLock::new(HashMap::<String, String>::new()));
        let codec = TelnetCodec::default();
        let framed = Framed::new(stream, codec);
        let active = AtomicBool::new(true);
        let buffer = TerminalBuffer::default();
        let (mut sink, mut stream) = framed.split();
        let (mut tx, mut rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let addr = address.clone();
            while true {
                match stream.next().await.unwrap() {
                    //
                    Ok(frame) => match frame {
                        TelnetFrame::Data(_) | TelnetFrame::Line(_) => {
                            //
                        }
                        TelnetFrame::NoOperation => {}
                        TelnetFrame::DataMark => {}
                        TelnetFrame::Break => {}
                        TelnetFrame::InterruptProcess => {}
                        TelnetFrame::AbortOutput => {}
                        TelnetFrame::AreYouThere => {}
                        TelnetFrame::EraseCharacter => {}
                        TelnetFrame::EraseLine => {}
                        TelnetFrame::GoAhead => {}
                        TelnetFrame::Do(_) => {}
                        TelnetFrame::Dont(_) => {}
                        TelnetFrame::Will(_) => {}
                        TelnetFrame::Wont(_) => {}
                        TelnetFrame::Subnegotiate(_, _) => {}
                    },
                    Err(error) => {
                        warn!("Error processing Telnet frame: {}", error);
                    }
                }
            }
        });

        unimplemented!()
    }

    pub(crate) async fn spawn(&mut self) -> JoinHandle<()> {
        let nvt = self.clone();
        tokio::spawn(async move {
            while nvt.inner.active.load(std::sync::atomic::Ordering::Relaxed) {
                match nvt.inner.codec.write().await.next().await {
                    Some(Ok(frame)) => match frame {
                        TelnetFrame::Data(_) | TelnetFrame::Line(_) => {
                            // TODO: Handle data
                            // TODO: Notify Handler
                            if let Some(handler) = &nvt.inner.handler {
                                let event = TerminalEvent::NoOperation;
                                handler.on_event(nvt.clone(), event);
                            }
                        }
                        TelnetFrame::NoOperation => {}
                        TelnetFrame::DataMark => {}
                        TelnetFrame::Break => {}
                        TelnetFrame::InterruptProcess => {}
                        TelnetFrame::AbortOutput => {}
                        TelnetFrame::AreYouThere => {}
                        TelnetFrame::EraseCharacter => {}
                        TelnetFrame::EraseLine => {}
                        TelnetFrame::GoAhead => {}
                        TelnetFrame::Do(_) => {}
                        TelnetFrame::Dont(_) => {}
                        TelnetFrame::Will(_) => {}
                        TelnetFrame::Wont(_) => {}
                        TelnetFrame::Subnegotiate(_, _) => {}
                    },
                    Some(Err(e)) => {
                        warn!("Error processing Telnet frame: {}", e);
                    }
                    None => {}
                }
            }
            if let Some(handler) = &nvt.inner.handler {
                handler.on_close(nvt.clone());
            }
        })
    }

    /// Asynchronously sends a sequence of bytes over the `TelnetCodec` connection.
    ///
    /// This function acquires a write lock on the codec and sends each byte from
    /// the provided byte slice as a `TelnetFrame::Data` frame. If sending any
    /// byte fails, the function will panic with an error message.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A reference to a slice of bytes (`&[u8]`) that will be sent
    ///             individually over the connection.
    ///
    /// # Panics
    ///
    /// This function will panic if sending a byte fails unexpectedly. This can occur
    /// if the underlying transport or codec encounters an unrecoverable error during
    /// the send operation (e.g., a service failure).
    ///
    /// # Notes
    ///
    /// * This method is asynchronous and requires the Tokio runtime to be active.
    /// * Each byte in the provided slice is sent individually as its own frame.
    ///
    pub async fn send_bytes(&mut self, bytes: &[u8]) {
        let mut lock = self.inner.codec.write().await;
        for byte in bytes {
            lock.send(TelnetFrame::Data(*byte))
                .await
                .expect("failed to send data");
        }
    }

    /// Sends a single Unicode character asynchronously to the underlying transport.
    ///
    /// This method takes a `char` as an argument, encodes it into UTF-8, and sends
    /// the resulting byte sequence using the `send_bytes` method. The function
    /// operates asynchronously and requires an `await` when called.
    ///
    /// # Arguments
    /// * `ch` - A single Unicode character to be sent.
    ///
    ///
    /// # Notes
    /// * The encoded character will be stored in a temporary buffer with a size of 4 bytes,
    ///   which is sufficient to hold any UTF-8 encoded character.
    /// * Ensure that the `send_bytes` method is implemented for the struct and handles
    ///   sending data properly.
    ///
    /// # Errors
    /// Depending on the implementation of the `send_bytes` method, this function could
    /// propagate any asynchronous I/O-related errors that occur during transmission.
    pub async fn send_char(&mut self, ch: char) {
        let mut buf = [0; 4];
        ch.encode_utf8(&mut buf);
        self.send_bytes(&buf).await;
    }

    /// Sends a string as bytes to the underlying output stream.
    ///
    /// This asynchronous function converts the provided string slice (`&str`) into a byte slice
    /// and sends it using the `send_bytes` method. The primary use of this function is to send
    /// textual data over a communication channel that expects raw bytes.
    ///
    /// # Parameters
    /// - `s`: A string slice reference (`&str`) to be sent. The string will be converted to bytes
    ///        internally before being sent.
    ///
    /// # Behavior
    /// This function calls `self.send_bytes` with the byte representation of the input string
    /// and awaits its completion.
    ///
    /// # Note
    /// This method assumes that the receiver understands how to interpret the byte representation
    /// of the string (e.g., UTF-8 encoding). Ensure proper encoding is used if the receiver expects
    /// a specific character representation.
    pub async fn send_string(&mut self, s: &str) {
        self.send_bytes(s.as_bytes()).await;
    }

    /// Retrieves the socket address associated with the current object.
    ///
    /// This method provides a reference to the `SocketAddr` stored within the
    /// `inner` field. It allows access to the address value without transferring
    /// ownership.
    ///
    /// # Notes
    /// - Ensure that the `inner` field and the `address` member are properly initialized
    ///   before calling this function to avoid unexpected behavior.
    pub fn addr(&self) -> &SocketAddr {
        &self.inner.address
    }
}

impl std::fmt::Debug for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Terminal")
            .field("address", &self.inner.address)
            .field("active", &self.inner.active)
            .finish()
    }
}
