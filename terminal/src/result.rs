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

use termionix_ansicodec::{AnsiCodecError, TelnetCodecError};

/// Result type for the terminal
pub type TerminalResult<T> = Result<T, TerminalError>;

#[derive(Debug)]
pub enum TerminalError {
    IOError(std::io::Error),
    CodecError(TelnetCodecError),
    AnsiError(AnsiCodecError),
}

impl std::fmt::Display for TerminalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for TerminalError {}

impl From<std::io::Error> for TerminalError {
    fn from(error: std::io::Error) -> Self {
        TerminalError::IOError(error)
    }
}

impl From<TelnetCodecError> for TerminalError {
    fn from(error: TelnetCodecError) -> Self {
        TerminalError::CodecError(error)
    }
}

impl From<AnsiCodecError> for TerminalError {
    fn from(error: AnsiCodecError) -> Self {
        TerminalError::AnsiError(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_error_from_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let terminal_error: TerminalError = io_error.into();

        match terminal_error {
            TerminalError::IOError(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_terminal_error_from_codec_error() {
        let codec_error = TelnetCodecError::IOError {
            kind: std::io::ErrorKind::Other,
            operation: "test".to_string(),
        };
        let terminal_error: TerminalError = codec_error.into();

        assert!(matches!(terminal_error, TerminalError::CodecError(_)));
    }

    #[test]
    fn test_terminal_error_from_ansi_error() {
        let ansi_error =
            AnsiCodecError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        let terminal_error: TerminalError = ansi_error.into();

        assert!(matches!(terminal_error, TerminalError::AnsiError(_)));
    }

    #[test]
    fn test_terminal_error_display() {
        let error =
            TerminalError::from(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
        let display_str = format!("{}", error);
        assert!(!display_str.is_empty());
    }

    #[test]
    fn test_terminal_error_debug() {
        let error =
            TerminalError::from(std::io::Error::new(std::io::ErrorKind::Other, "test error"));
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("IOError"));
    }

    #[test]
    fn test_terminal_result_ok() {
        let result: TerminalResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_terminal_result_err() {
        let result: TerminalResult<i32> = Err(TerminalError::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test",
        )));
        assert!(result.is_err());
    }

    #[test]
    fn test_terminal_error_is_error_trait() {
        let error = TerminalError::from(std::io::Error::new(std::io::ErrorKind::Other, "test"));

        // Verify it implements std::error::Error
        let _: &dyn std::error::Error = &error;
    }

    #[test]
    fn test_all_error_variants() {
        let io_err = TerminalError::IOError(std::io::Error::new(std::io::ErrorKind::Other, "io"));
        let codec_err = TerminalError::CodecError(TelnetCodecError::IOError {
            kind: std::io::ErrorKind::Other,
            operation: "test".to_string(),
        });
        let ansi_err = TerminalError::AnsiError(AnsiCodecError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test",
        )));

        assert!(matches!(io_err, TerminalError::IOError(_)));
        assert!(matches!(codec_err, TerminalError::CodecError(_)));
        assert!(matches!(ansi_err, TerminalError::AnsiError(_)));
    }

    #[test]
    fn test_error_conversion_chain() {
        // Test that we can convert through the chain
        let io_error = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let terminal_error: TerminalError = io_error.into();
        let result: TerminalResult<()> = Err(terminal_error);

        assert!(result.is_err());
    }

    #[test]
    fn test_result_with_different_types() {
        let result_int: TerminalResult<i32> = Ok(42);
        let result_string: TerminalResult<String> = Ok("test".to_string());
        let result_unit: TerminalResult<()> = Ok(());

        assert_eq!(result_int.unwrap(), 42);
        assert_eq!(result_string.unwrap(), "test");
        assert_eq!(result_unit.unwrap(), ());
    }

    #[test]
    fn test_error_propagation() {
        fn inner_function() -> TerminalResult<i32> {
            Err(TerminalError::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                "inner error",
            )))
        }

        fn outer_function() -> TerminalResult<i32> {
            inner_function()?;
            Ok(42)
        }

        let result = outer_function();
        assert!(result.is_err());
    }
}
