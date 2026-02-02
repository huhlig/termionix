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

//! Error types and result aliases for connection operations

/// Result type for connection operations
///
/// This is a convenience type alias that uses [`ConnectionError`] as the error type.
///
/// # Examples
///
/// ```
/// use termionix_service::ConnectionResult;
///
/// fn example() -> ConnectionResult<()> {
///     Ok(())
/// }
/// ```
pub type ConnectionResult<T> = Result<T, ConnectionError>;

/// Errors that can occur during connection operations
///
/// This enum represents all possible errors that can occur when working with
/// split terminal connections, including I/O errors, codec errors, and channel
/// communication failures.
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    /// An I/O error occurred
    ///
    /// This wraps standard I/O errors from the underlying stream operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A codec error occurred during encoding or decoding
    ///
    /// This error is raised when the codec fails to encode or decode data.
    /// The string contains details about the codec error.
    #[error("Codec error: {0}")]
    Codec(String),

    /// The connection has been closed
    ///
    /// This error is returned when attempting to use a connection that has
    /// already been closed or when the background workers have terminated.
    #[error("Connection closed")]
    Closed,

    /// Failed to send data through the write channel
    ///
    /// This error occurs when the write worker is no longer accepting commands,
    /// typically because it has been shut down or encountered a fatal error.
    #[error("Send failed: {0}")]
    SendFailed(String),

    /// Failed to receive data from the read channel
    ///
    /// This error occurs when the read worker is no longer responding to commands,
    /// typically because it has been shut down or encountered a fatal error.
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),
}
