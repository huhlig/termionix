//
// Copyright 2017-2025 Hans W. Uhlig. All Rights Reserved.
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

use crate::Connection;
use termionix_ansicodes::SegmentedString;
use termionix_terminal::{CursorPosition, TerminalError, TerminalEvent};
use tracing::trace;



pub trait TelnetService: Send + Sync + 'static {
    /// Called on a new Client Connection
    fn on_connect(&self, connection: &Connection) {
        trace!("Client Connected");
    }
    /// Called on any Terminal event
    fn on_event(&self, connection: &Connection, update: TerminalEvent) {
        trace!("Terminal Event Received");
    }
    /// Called when a line is completed.
    fn on_message(&self, connection: &Connection, message: SegmentedString) {
        trace!("Terminal Message Received");
    }
    /// Called when the cursor is updated.
    fn on_cursor_update(&self, connection: &Connection, pos: CursorPosition) {
        trace!("Terminal Cursor Update Received");
    }
    /// Called when a client experiences an error.
    fn on_error(&self, connection: &Connection, error: TerminalError) {
        trace!("Client Error");
    }
    /// Called when a client times out.
    fn on_timeout(&self, connection: &Connection) {
        trace!("Client Timeout");
    }
    /// Called when a Client Disconnects
    fn on_disconnect(&self, connection: &Connection) {
        trace!("Client Disconnected");
    }
    /// Called when the service is shutting down.
    fn on_shutdown(&self) {
        trace!("Service Shutting Down");
    }
}
