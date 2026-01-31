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

use crate::{TerminalBuffer, TerminalCodec, TerminalError, TerminalEvent};
use bytes::BytesMut;
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use termionix_telnetcodec::{TelnetCodec, TelnetFrame, TelnetOption, TelnetOptions};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};

/// Terminal represents a telnet connection with ANSI terminal emulation
#[derive(Clone)]
pub struct Terminal {
    inner: Arc<TerminalInner>,
}

struct TerminalInner {
    /// Remote address
    address: SocketAddr,
    /// Terminal metadata (environment variables, etc.)
    metadata: RwLock<HashMap<String, String>>,
    /// Framed telnet codec
    codec: RwLock<Framed<TcpStream, TelnetCodec>>,
    /// Terminal codec for ANSI processing
    terminal_codec: RwLock<TerminalCodec>,
    /// Terminal buffer state
    buffer: RwLock<TerminalBuffer>,
    /// Telnet option negotiation state (from codec crate)
    options: RwLock<TelnetOptions>,
    /// Active flag
    active: AtomicBool,
    /// Event sender
    event_tx: mpsc::UnboundedSender<TerminalEvent>,
}

impl Terminal {
    /// Create a new Terminal from a TcpStream
    pub async fn new(stream: TcpStream) -> Result<(Self, mpsc::UnboundedReceiver<TerminalEvent>), TerminalError> {
        let address = stream.peer_addr()?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        let terminal = Terminal {
            inner: Arc::new(TerminalInner {
                address,
                metadata: RwLock::new(HashMap::new()),
                codec: RwLock::new(Framed::new(stream, TelnetCodec::default())),
                terminal_codec: RwLock::new(TerminalCodec::new()),
                buffer: RwLock::new(TerminalBuffer::new()),
                options: RwLock::new(TelnetOptions::default()),
                active: AtomicBool::new(true),
                event_tx,
            }),
        };

        Ok((terminal, event_rx))
    }

    /// Start the terminal processing loop
    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move {
            if let Err(e) = self.run().await {
                error!("Terminal error for {}: {}", self.inner.address, e);
            }
            info!("Terminal {} closed", self.inner.address);
        })
    }

    /// Main processing loop
    async fn run(&self) -> Result<(), TerminalError> {
        // Perform initial option negotiation
        self.negotiate_initial_options().await?;

        // Process frames
        while self.inner.active.load(Ordering::Relaxed) {
            let mut codec = self.inner.codec.write().await;
            
            match codec.next().await {
                Some(Ok(frame)) => {
                    drop(codec); // Release lock before processing
                    self.process_frame(frame).await?;
                }
                Some(Err(e)) => {
                    error!("Codec error: {}", e);
                    return Err(e.into());
                }
                None => {
                    // Connection closed
                    break;
                }
            }
        }

        Ok(())
    }

    /// Perform initial option negotiation
    async fn negotiate_initial_options(&self) -> Result<(), TerminalError> {
        let mut options = self.inner.options.write().await;
        let mut codec = self.inner.codec.write().await;

        // Request options we want to enable locally
        if let Some(frame) = options.request_will(TelnetOption::Echo) {
            codec.send(frame).await?;
        }
        if let Some(frame) = options.request_will(TelnetOption::SuppressGoAhead) {
            codec.send(frame).await?;
        }

        // Request options we want remote to enable
        if let Some(frame) = options.request_do(TelnetOption::SuppressGoAhead) {
            codec.send(frame).await?;
        }
        if let Some(frame) = options.request_do(TelnetOption::TerminalType) {
            codec.send(frame).await?;
        }
        if let Some(frame) = options.request_do(TelnetOption::NAWS) {
            codec.send(frame).await?;
        }
        if let Some(frame) = options.request_do(TelnetOption::Linemode) {
            codec.send(frame).await?;
        }

        codec.flush().await?;
        Ok(())
    }

    /// Process a single telnet frame
    async fn process_frame(&self, frame: TelnetFrame) -> Result<(), TerminalError> {
        // First, let TelnetOptions handle negotiation frames
        let mut options = self.inner.options.write().await;
        if let Ok(Some(response)) = options.handle_received(frame.clone()) {
            drop(options);
            let mut codec = self.inner.codec.write().await;
            codec.send(response).await?;
            codec.flush().await?;
            return Ok(());
        }
        drop(options);

        // Handle data and other frames
        match frame {
            TelnetFrame::Data(byte) => {
                self.process_data_byte(byte).await?;
            }
            TelnetFrame::Line(bytes) => {
                self.process_line(&bytes).await?;
            }
            TelnetFrame::NoOperation => {
                // Ignore
            }
            TelnetFrame::Break => {
                self.send_event(TerminalEvent::Break).await;
            }
            TelnetFrame::InterruptProcess => {
                self.send_event(TerminalEvent::InterruptProcess).await;
            }
            TelnetFrame::EraseCharacter => {
                let mut buffer = self.inner.buffer.write().await;
                buffer.erase_character();
                let cursor = buffer.cursor_position();
                drop(buffer);
                self.send_event(TerminalEvent::EraseCharacter { cursor }).await;
            }
            TelnetFrame::EraseLine => {
                let mut buffer = self.inner.buffer.write().await;
                buffer.erase_line();
                let cursor = buffer.cursor_position();
                drop(buffer);
                self.send_event(TerminalEvent::EraseLine { cursor }).await;
            }
            TelnetFrame::Subnegotiate(option, data) => {
                self.handle_subnegotiation(option, data).await?;
            }
            _ => {
                // Other frames handled by TelnetOptions
            }
        }
        Ok(())
    }

    /// Process a single data byte through the terminal codec
    async fn process_data_byte(&self, byte: u8) -> Result<(), TerminalError> {
        let mut terminal_codec = self.inner.terminal_codec.write().await;
        let mut src = BytesMut::from(&[byte][..]);
        
        // Decode through terminal codec
        while let Some(event) = terminal_codec.decode(&mut src)? {
            drop(terminal_codec); // Release lock before sending event
            self.send_event(event).await;
            terminal_codec = self.inner.terminal_codec.write().await;
        }
        
        Ok(())
    }

    /// Process a complete line
    async fn process_line(&self, bytes: &[u8]) -> Result<(), TerminalError> {
        let mut terminal_codec = self.inner.terminal_codec.write().await;
        let mut src = BytesMut::from(bytes);
        
        // Decode through terminal codec
        while let Some(event) = terminal_codec.decode(&mut src)? {
            drop(terminal_codec); // Release lock before sending event
            self.send_event(event).await;
            terminal_codec = self.inner.terminal_codec.write().await;
        }
        
        Ok(())
    }

    /// Handle subnegotiation
    async fn handle_subnegotiation(
        &self,
        option: TelnetOption,
        data: Vec<u8>,
    ) -> Result<(), TerminalError> {
        match option {
            TelnetOption::NAWS => {
                // Parse window size
                use termionix_telnetcodec::naws::WindowSize;
                if let Ok(size) = WindowSize::decode(&mut &data[..]) {
                    self.set_size(size.cols as usize, size.rows as usize).await;
                    let (width, height) = self.size().await;
                    self.send_event(TerminalEvent::ResizeWindow {
                        old: crate::types::TerminalSize::new(80, 24),
                        new: crate::types::TerminalSize::new(width, height),
                    }).await;
                }
            }
            TelnetOption::TerminalType => {
                // Store terminal type in metadata
                if let Ok(term_type) = String::from_utf8(data) {
                    self.set_metadata("TERM".to_string(), term_type).await;
                }
            }
            _ => {
                // Ignore unknown subnegotiations
                debug!("Unhandled subnegotiation for option: {:?}", option);
            }
        }
        Ok(())
    }

    /// Send an event to the event channel
    async fn send_event(&self, event: TerminalEvent) {
        if let Err(e) = self.inner.event_tx.send(event) {
            error!("Failed to send event: {}", e);
        }
    }

    /// Send data to the client
    pub async fn send(&self, data: impl Into<Vec<u8>>) -> Result<(), TerminalError> {
        let bytes = data.into();
        let mut codec = self.inner.codec.write().await;
        
        for byte in bytes {
            codec.send(TelnetFrame::Data(byte)).await?;
        }
        codec.flush().await?;
        
        Ok(())
    }

    /// Send a string to the client
    pub async fn send_str(&self, s: &str) -> Result<(), TerminalError> {
        self.send(s.as_bytes().to_vec()).await
    }

    /// Send a character to the client
    pub async fn send_char(&self, ch: char) -> Result<(), TerminalError> {
        let mut buf = [0u8; 4];
        let bytes = ch.encode_utf8(&mut buf);
        self.send(bytes.as_bytes().to_vec()).await
    }

    /// Get the remote address
    pub fn address(&self) -> SocketAddr {
        self.inner.address
    }

    /// Check if the terminal is active
    pub fn is_active(&self) -> bool {
        self.inner.active.load(Ordering::Relaxed)
    }

    /// Close the terminal
    pub fn close(&self) {
        self.inner.active.store(false, Ordering::Relaxed);
    }

    /// Get metadata value
    pub async fn get_metadata(&self, key: &str) -> Option<String> {
        self.inner.metadata.read().await.get(key).cloned()
    }

    /// Set metadata value
    pub async fn set_metadata(&self, key: String, value: String) {
        self.inner.metadata.write().await.insert(key, value);
    }

    /// Get terminal size
    pub async fn size(&self) -> (usize, usize) {
        let buffer = self.inner.buffer.read().await;
        (buffer.width(), buffer.height())
    }

    /// Set terminal size
    pub async fn set_size(&self, width: usize, height: usize) {
        let mut buffer = self.inner.buffer.write().await;
        buffer.set_size(width, height);
    }

    /// Check if an option is enabled
    pub async fn is_option_enabled(&self, option: TelnetOption) -> bool {
        let options = self.inner.options.read().await;
        options.local_enabled(option) || options.remote_enabled(option)
    }
}

impl std::fmt::Debug for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Terminal")
            .field("address", &self.inner.address)
            .field("active", &self.inner.active.load(Ordering::Relaxed))
            .finish()
    }
}


