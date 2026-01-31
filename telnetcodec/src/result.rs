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

/// Result Type for Codec Operations
pub type CodecResult<T> = Result<T, CodecError>;

/// Represents possible errors that can occur in the codec handling process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodecError {
    /// An I/O error occurred while reading from or writing to the underlying stream.
    ///
    /// Contains the error kind and a description of what operation failed.
    IOError {
        /// The kind of I/O error that occurred
        kind: std::io::ErrorKind,
        /// Description of the operation that failed
        operation: String,
    },

    /// Error occurred during telnet option negotiation.
    ///
    /// This error is returned when an invalid or unsupported frame type is
    /// received during the negotiation process.
    NegotiationError {
        /// Description of what went wrong during negotiation
        reason: String,
        /// The frame type that caused the error, if available
        frame_type: Option<String>,
    },

    /// Error occurred during telnet option subnegotiation.
    ///
    /// This error is returned when parsing or encoding subnegotiation data fails.
    SubnegotiationError {
        /// The telnet option being subnegotiated
        option: Option<u8>,
        /// Specific reason for the failure
        reason: SubnegotiationErrorKind,
    },

    /// An unknown or invalid telnet command byte was encountered.
    ///
    /// Contains the invalid command byte value.
    UnknownCommand(u8),
}

/// Specific kinds of subnegotiation errors with structured context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubnegotiationErrorKind {
    /// Insufficient data available to decode the subnegotiation.
    InsufficientData {
        /// Number of bytes required
        required: usize,
        /// Number of bytes available
        available: usize,
    },

    /// Invalid command byte in the subnegotiation data.
    InvalidCommand {
        /// The invalid command byte
        command: u8,
        /// Expected command bytes, if known
        expected: Option<Vec<u8>>,
    },

    /// Invalid verb (DO/DONT/WILL/WONT) in status subnegotiation.
    InvalidVerb {
        /// The invalid verb byte
        verb: u8,
    },

    /// Unknown option code encountered.
    UnknownOption {
        /// The unknown option code
        code: u8,
    },

    /// Unexpected data present when none was expected.
    UnexpectedData {
        /// Description of why the data is unexpected
        reason: String,
    },

    /// Incomplete data structure (e.g., missing second byte of a pair).
    IncompleteData {
        /// Description of what data is incomplete
        description: String,
    },

    /// Encoding failed due to insufficient buffer space.
    EncodingFailed {
        /// Number of bytes required
        required: usize,
        /// Number of bytes available
        available: usize,
    },

    /// Generic subnegotiation error with a description.
    Other {
        /// Description of the error
        description: String,
    },
}

impl std::error::Error for CodecError {}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::IOError { kind, operation } => {
                write!(f, "I/O error during {}: {:?}", operation, kind)
            }
            CodecError::NegotiationError { reason, frame_type } => {
                if let Some(ft) = frame_type {
                    write!(f, "Negotiation error ({}): {}", ft, reason)
                } else {
                    write!(f, "Negotiation error: {}", reason)
                }
            }
            CodecError::SubnegotiationError { option, reason } => {
                if let Some(opt) = option {
                    write!(f, "Subnegotiation error for option {}: {}", opt, reason)
                } else {
                    write!(f, "Subnegotiation error: {}", reason)
                }
            }
            CodecError::UnknownCommand(cmd) => {
                write!(f, "Unknown telnet command: 0x{:02X}", cmd)
            }
        }
    }
}

impl std::fmt::Display for SubnegotiationErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubnegotiationErrorKind::InsufficientData {
                required,
                available,
            } => {
                write!(
                    f,
                    "insufficient data (required: {}, available: {})",
                    required, available
                )
            }
            SubnegotiationErrorKind::InvalidCommand { command, expected } => {
                if let Some(exp) = expected {
                    write!(
                        f,
                        "invalid command 0x{:02X} (expected one of: {:?})",
                        command, exp
                    )
                } else {
                    write!(f, "invalid command: 0x{:02X}", command)
                }
            }
            SubnegotiationErrorKind::InvalidVerb { verb } => {
                write!(f, "invalid verb: 0x{:02X}", verb)
            }
            SubnegotiationErrorKind::UnknownOption { code } => {
                write!(f, "unknown option code: {}", code)
            }
            SubnegotiationErrorKind::UnexpectedData { reason } => {
                write!(f, "unexpected data: {}", reason)
            }
            SubnegotiationErrorKind::IncompleteData { description } => {
                write!(f, "incomplete data: {}", description)
            }
            SubnegotiationErrorKind::EncodingFailed {
                required,
                available,
            } => {
                write!(
                    f,
                    "encoding failed (required: {}, available: {})",
                    required, available
                )
            }
            SubnegotiationErrorKind::Other { description } => {
                write!(f, "{}", description)
            }
        }
    }
}

impl From<std::io::Error> for CodecError {
    fn from(err: std::io::Error) -> Self {
        CodecError::IOError {
            kind: err.kind(),
            operation: err.to_string(),
        }
    }
}
