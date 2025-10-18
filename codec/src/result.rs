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

/// Result Type for Codec Operations
pub type CodecResult<T> = Result<T, CodecError>;

/// Represents possible errors that can occur in the codec handling process.
#[derive(Debug)]
pub enum CodecError {
    /// An error occurred while reading from the underlying stream.
    IOError(String),
    /// Error Negotiating a Telnet Option.
    NegotiationError(String),
    /// Error Subnegotiating a Telnet Option.
    SubnegotiationError(String),
    /// An unknown or invalid command was used
    UnknownCommand(u8),
}

impl std::error::Error for CodecError {}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for CodecError {
    fn from(err: std::io::Error) -> Self {
        CodecError::IOError(err.to_string())
    }
}