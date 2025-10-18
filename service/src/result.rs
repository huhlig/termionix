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

/// Result type for the terminal
pub type TelnetResult<T> = Result<T, TelnetError>;

#[derive(Debug)]
pub enum TelnetError {
    PoisonError(String),
    UnknownError(String),
    TokioError(tokio::io::Error),
    BoxedError(Box<dyn std::error::Error + Send>),
    CodecError(termionix_codec::CodecError),
    TerminalError(termionix_terminal::TerminalError),
}

impl std::fmt::Display for TelnetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelnetError::PoisonError(msg) => {
                write!(f, "Internal synchronization error: {}", msg)
            }
            TelnetError::UnknownError(msg) => {
                write!(f, "Unknown error: {}", msg)
            }
            TelnetError::TokioError(err) => {
                write!(f, "I/O error: {}", err)
            }
            TelnetError::BoxedError(err) => {
                write!(f, "Error: {}", err)
            }
            TelnetError::CodecError(err) => {
                write!(f, "Protocol error: {}", err)
            }
            TelnetError::TerminalError(err) => {
                write!(f, "Terminal error: {}", err)
            }
        }
    }
}

impl std::error::Error for TelnetError {}

impl From<&str> for TelnetError {
    fn from(value: &str) -> Self {
        TelnetError::UnknownError(value.to_string())
    }
}

impl From<tokio::io::Error> for TelnetError {
    fn from(value: tokio::io::Error) -> Self {
        TelnetError::TokioError(value)
    }
}

impl From<Box<dyn std::error::Error + Send>> for TelnetError {
    fn from(value: Box<dyn std::error::Error + Send>) -> Self {
        TelnetError::BoxedError(value)
    }
}

impl From<termionix_codec::CodecError> for TelnetError {
    fn from(value: termionix_codec::CodecError) -> Self {
        TelnetError::CodecError(value)
    }
}

impl From<termionix_terminal::TerminalError> for TelnetError {
    fn from(value: termionix_terminal::TerminalError) -> Self {
        TelnetError::TerminalError(value)
    }
}

impl<T> From<std::sync::PoisonError<T>> for TelnetError {
    /// Converts a poisoned mutex guard error into a `TelnetError`.
    ///
    /// This allows lock poisoning on the underlying framed stream to be surfaced
    /// as a regular `TelnetError`, making it easier to handle uniformly at
    /// higher layers.
    fn from(value: std::sync::PoisonError<T>) -> Self {
        TelnetError::PoisonError(value.to_string())
    }
}
