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

//! Error types for the ansicodec crate.
//!
//! This module provides comprehensive error handling for ANSI string operations,
//! including parsing errors, validation errors, and buffer management errors.

/// Result type alias for operations that may fail with an [`AnsiCodecError`].
pub type AnsiCodecResult<T> = Result<T, AnsiCodecError>;

/// Errors that can occur when working with ANSI strings.
#[derive(Debug)]
pub enum AnsiCodecError {
    /// IO Error
    IoError(std::io::Error),
    /// Invalid UTF-8 sequence encountered at the specified position.
    ///
    /// This error occurs when the input contains bytes that don't form valid UTF-8.
    InvalidUtf8 {
        /// The byte position where the invalid UTF-8 was encountered
        position: usize,
    },

    /// Malformed ANSI escape sequence encountered.
    ///
    /// This error occurs when an ANSI sequence doesn't follow the expected format.
    MalformedAnsi {
        /// The byte position where the malformed sequence starts
        position: usize,
        /// Description of what's wrong with the sequence
        description: String,
    },

    /// Incomplete ANSI sequence at the end of input.
    ///
    /// This error occurs when the input ends in the middle of an ANSI sequence.
    IncompleteSequence {
        /// The byte position where the incomplete sequence starts
        position: usize,
    },

    /// Range is out of bounds for the string.
    ///
    /// This error occurs when trying to apply a style or operation to a range
    /// that extends beyond the string's length.
    RangeOutOfBounds {
        /// The range that was requested
        range: std::ops::Range<usize>,
        /// The maximum valid position
        max: usize,
    },

    /// ANSI sequence exceeds maximum allowed length.
    ///
    /// This error occurs when an ANSI sequence is longer than the configured limit,
    /// which may indicate malformed input or a potential attack.
    SequenceTooLong {
        /// The actual length of the sequence
        length: usize,
        /// The maximum allowed length
        max: usize,
    },

    /// Buffer overflow prevented.
    ///
    /// This error occurs when an operation would exceed the buffer's capacity.
    BufferOverflow {
        /// The number of bytes attempted to write
        attempted: usize,
        /// The buffer's capacity
        capacity: usize,
    },

    /// Invalid parameter value.
    ///
    /// This error occurs when a parameter has an invalid value.
    InvalidParameter {
        /// Name of the parameter
        name: String,
        /// The invalid value
        value: String,
        /// Description of why it's invalid
        reason: String,
    },
}

impl std::fmt::Display for AnsiCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnsiCodecError::IoError(err) => {
                write!(f, "IOError {}", err)
            }
            AnsiCodecError::InvalidUtf8 { position } => {
                write!(f, "Invalid UTF-8 sequence at position {}", position)
            }
            AnsiCodecError::MalformedAnsi {
                position,
                description,
            } => {
                write!(
                    f,
                    "Malformed ANSI sequence at position {}: {}",
                    position, description
                )
            }
            AnsiCodecError::IncompleteSequence { position } => {
                write!(
                    f,
                    "Incomplete ANSI sequence at end of input (started at position {})",
                    position
                )
            }
            AnsiCodecError::RangeOutOfBounds { range, max } => {
                write!(
                    f,
                    "Range out of bounds: {:?} (maximum valid position: {})",
                    range, max
                )
            }
            AnsiCodecError::SequenceTooLong { length, max } => {
                write!(
                    f,
                    "Sequence too long: {} bytes (maximum allowed: {})",
                    length, max
                )
            }
            AnsiCodecError::BufferOverflow {
                attempted,
                capacity,
            } => {
                write!(
                    f,
                    "Buffer overflow: attempted to write {} bytes to buffer of size {}",
                    attempted, capacity
                )
            }
            AnsiCodecError::InvalidParameter {
                name,
                value,
                reason,
            } => {
                write!(
                    f,
                    "Invalid parameter '{}' with value '{}': {}",
                    name, value, reason
                )
            }
        }
    }
}

impl std::error::Error for AnsiCodecError {}

impl From<std::io::Error> for AnsiCodecError {
    fn from(error: std::io::Error) -> Self {
        AnsiCodecError::IoError(error)
    }
}

impl From<termionix_telnetcodec::TelnetCodecError> for AnsiCodecError {
    fn from(error: termionix_telnetcodec::TelnetCodecError) -> Self {
        AnsiCodecError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Codec error: {:?}", error),
        ))
    }
}
