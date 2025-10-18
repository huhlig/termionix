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

use crate::TerminalBuffer;
use crate::utility::RwLockReadReference;
use futures_util::stream::SplitSink;
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock, RwLockReadGuard};
use termionix_ansicodes::{SegmentedString, StyledString};
use termionix_codec::{TelnetCodec, TelnetFrame};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;

pub struct TelnetConnection {
    active: Arc<AtomicBool>,
    address: SocketAddr,
    buffer: Arc<RwLock<TerminalBuffer>>,
    writer: SplitSink<Framed<TcpStream, TelnetCodec>, TelnetFrame>,
}

impl TelnetConnection {
    pub fn wrap(
        address: SocketAddr,
        writer: SplitSink<Framed<TcpStream, TelnetCodec>, TelnetFrame>,
        active: Arc<AtomicBool>,
        mut receiver: mpsc::Receiver<TelnetFrame>,
    ) -> TelnetConnection {
        let buffer = Arc::new(RwLock::new(TerminalBuffer::new()));
        let connection = TelnetConnection {
            active,
            address,
            writer,
            buffer: buffer.clone(),
        };

        tokio::spawn(async move {
            while let Some(frame) = receiver.recv().await {
                match frame {
                    TelnetFrame::Data(byte) => {
                        buffer
                            .write()
                            .expect("Poisoned Lock on buffer")
                            .push_byte(byte);
                    }
                    TelnetFrame::Line(line) => {
                        buffer
                            .write()
                            .expect("Poisoned Lock on buffer")
                            .append_line(line);
                    }
                    TelnetFrame::NoOperation => {
                        // Do Nothing
                    }
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
                }
            }
        });

        connection
    }

    pub fn address(&self) -> SocketAddr {
        self.address.clone()
    }

    pub fn active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Gets the current terminal size
    pub fn terminal_size(&self) -> (usize, usize) {
        self.buffer.read().expect("Poisoned Lock on buffer").size()
    }

    /// Gets the terminal width
    pub fn width(&self) -> usize {
        self.buffer
            .read()
            .expect("Poisoned Lock on buffer")
            .size()
            .0
    }

    /// Gets the terminal height
    pub fn height(&self) -> usize {
        self.buffer
            .read()
            .expect("Poisoned Lock on buffer")
            .size()
            .1
    }

    /// Gets the current cursor position
    pub fn cursor_position(&self) -> (usize, usize) {
        self.buffer
            .read()
            .expect("Poisoned Lock on buffer")
            .cursor_position()
    }

    /// Erases the last character from the current line buffer
    pub fn erase_character(&mut self) {
        self.buffer
            .write()
            .expect("Poisoned Lock on buffer")
            .erase_character()
    }

    /// Gets the current character count in the current line
    pub fn current_line_length(&self) -> usize {
        self.buffer
            .read()
            .expect("Poisoned Lock on buffer")
            .current_line_length()
    }

    /// Gets a reference to the current line being typed
    pub fn current_line(&self, f: impl FnOnce(&SegmentedString)) {
        f(self
            .buffer
            .read()
            .expect("Poisoned Lock on buffer")
            .current_line())
    }

    /// Checks if the current line buffer is empty
    pub fn is_current_line_empty(&self) -> bool {
        self.buffer
            .read()
            .expect("Poisoned Lock on buffer")
            .is_current_line_empty()
    }

    // ===== Line-level API =====

    /// Completes the current line and adds it to completed lines
    pub fn complete_line(&mut self) {
        self.buffer
            .write()
            .expect("Poisoned Lock on buffer")
            .complete_line()
    }

    /// Erases the entire current line
    pub fn erase_line(&mut self) {
        self.buffer
            .write()
            .expect("Poisoned Lock on buffer")
            .erase_line()
    }

    /// Gets the number of completed lines
    pub fn completed_line_count(&self) -> usize {
        self.buffer
            .read()
            .expect("Poisoned Lock on buffer")
            .completed_line_count()
    }

    /// Gets a reference to all completed lines
    pub fn completed_lines(&self, f: impl FnOnce(&[SegmentedString])) {
        f(self
            .buffer
            .read()
            .expect("Poisoned Lock on buffer")
            .completed_lines())
    }

    /// Pops the oldest-completed line
    pub fn pop_completed_line(&mut self, f: impl FnOnce(Option<SegmentedString>)) {
        f(self
            .buffer
            .write()
            .expect("Poisoned Lock on buffer")
            .pop_completed_line())
    }

    /// Takes all completed lines, leaving the buffer empty
    pub fn take_completed_lines(&mut self, f: impl FnOnce(Vec<SegmentedString>)) {
        f(self
            .buffer
            .write()
            .expect("Poisoned Lock on buffer")
            .take_completed_lines())
    }

    /// Clears all completed lines
    pub fn clear_completed_lines(&mut self) {
        self.buffer
            .write()
            .expect("Poisoned Lock on buffer")
            .clear_completed_lines()
    }

    /// Appends a pre-formed line to the completed lines (useful for echoing)
    pub fn append_line(&mut self, line: String) {
        self.buffer
            .write()
            .expect("Poisoned Lock on buffer")
            .append_line(line);
    }

    /// Appends a pre-formed line to the completed lines (useful for echoing)
    pub fn append_styled_line(&mut self, line: StyledString) {
        self.buffer
            .write()
            .expect("Poisoned Lock on buffer")
            .append_styled_line(line);
    }

    /// Gets the current line with ANSI codes optionally stripped
    pub fn current_line_stripped(&self) -> String {
        self.buffer
            .read()
            .expect("Poisoned Lock on buffer")
            .current_line_stripped()
    }

    /// Gets completed lines with ANSI codes optionally stripped
    pub fn completed_lines_stripped(&self) -> Vec<String> {
        self.buffer
            .read()
            .expect("Poisoned Lock on buffer")
            .completed_lines_stripped()
    }

    // ===== Buffer Management =====

    /// Clears the entire buffer (current line and completed lines)
    pub fn clear(&mut self) {
        self.buffer
            .write()
            .expect("Poisoned Lock on buffer")
            .clear()
    }

    /// Gets the total line count (completed + current if non-empty)
    pub fn total_line_count(&self) -> usize {
        self.buffer
            .read()
            .expect("Poisoned Lock on buffer")
            .total_line_count()
    }
}
