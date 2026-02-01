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

use bytes::BytesMut;
use std::fmt;

///
/// GMCP (Generic Mud Communication Protocol) Message
///
/// GMCP uses JSON syntax to define structured and typed data.
/// Each GMCP message consists of a package name and optional JSON data.
///
/// Format: `<package.subpackage.command> <json_data>`
///
/// # Examples
///
/// ```text
/// Core.Hello {"client": "TinTin++", "version": "2.02.0"}
/// Char.Vitals {"hp": 100, "maxhp": 120, "mp": 50, "maxmp": 80}
/// Room.Info {"num": 1234, "name": "Town Square"}
/// ```
///
/// # References
///
/// - [GMCP Protocol Specification](https://tintin.mudhalla.net/protocols/gmcp/)
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GmcpMessage {
    /// The package name (e.g., "Core.Hello", "Char.Vitals", "MSDP")
    /// Package names are typically case-insensitive, except for "MSDP" which
    /// must be fully capitalized when using MSDP over GMCP.
    package: String,

    /// Optional JSON data payload
    /// This should be valid JSON when present, using UTF-8 encoding.
    /// The data field is separated from the package by a single space.
    data: Option<String>,
}

impl GmcpMessage {
    /// Creates a new GMCP message with a package name and optional data.
    ///
    /// # Arguments
    ///
    /// * `package` - The package name (e.g., "Core.Hello", "Char.Vitals")
    /// * `data` - Optional JSON data as a string
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_telnetcodec::gmcp::GmcpMessage;
    ///
    /// let msg = GmcpMessage::new("Core.Hello", Some(r#"{"client":"MyClient"}"#));
    /// ```
    pub fn new<S: Into<String>, D: Into<String>>(package: S, data: Option<D>) -> Self {
        Self {
            package: package.into(),
            data: data.map(|d| d.into()),
        }
    }

    /// Creates a GMCP message without data (command only).
    ///
    /// # Arguments
    ///
    /// * `package` - The package name
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_telnetcodec::gmcp::GmcpMessage;
    ///
    /// let msg = GmcpMessage::command("Core.Ping");
    /// ```
    pub fn command<S: Into<String>>(package: S) -> Self {
        Self {
            package: package.into(),
            data: None,
        }
    }

    /// Parses a GMCP message from raw bytes.
    ///
    /// The format is: `<package> <json_data>` or just `<package>`
    /// The space between package and data is optional if there's no data.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The raw GMCP subnegotiation payload
    ///
    /// # Returns
    ///
    /// Returns `Some(GmcpMessage)` if parsing succeeds, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use bytes::BytesMut;
    /// use termionix_telnetcodec::gmcp::GmcpMessage;
    ///
    /// let data = BytesMut::from(&b"Core.Hello {\"client\":\"Test\"}"[..]);
    /// let msg = GmcpMessage::parse(&data).unwrap();
    /// assert_eq!(msg.package(), "Core.Hello");
    /// ```
    pub fn parse(bytes: &BytesMut) -> Option<Self> {
        // Convert bytes to UTF-8 string
        let text = std::str::from_utf8(bytes).ok()?;

        // Find the first space to separate package from data
        if let Some(space_pos) = text.find(' ') {
            let package = text[..space_pos].to_string();
            let data = text[space_pos + 1..].to_string();
            Some(Self {
                package,
                data: Some(data),
            })
        } else {
            // No space found, entire text is the package name
            Some(Self {
                package: text.to_string(),
                data: None,
            })
        }
    }

    /// Returns the package name.
    pub fn package(&self) -> &str {
        &self.package
    }

    /// Returns the JSON data if present.
    pub fn data(&self) -> Option<&str> {
        self.data.as_deref()
    }

    /// Checks if this message has data.
    pub fn has_data(&self) -> bool {
        self.data.is_some()
    }

    /// Encodes the GMCP message to bytes.
    ///
    /// # Returns
    ///
    /// A `BytesMut` containing the encoded message.
    pub fn encode(&self) -> BytesMut {
        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(self.package.as_bytes());

        if let Some(ref data) = self.data {
            bytes.extend_from_slice(b" ");
            bytes.extend_from_slice(data.as_bytes());
        }

        bytes
    }

    /// Returns the encoded byte length of this message.
    pub fn len(&self) -> usize {
        let mut len = self.package.len();
        if let Some(ref data) = self.data {
            len += 1 + data.len(); // space + data
        }
        len
    }

    /// Checks if the message is empty (has no package name).
    pub fn is_empty(&self) -> bool {
        self.package.is_empty()
    }

    /// Writes the GMCP message to a writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable writer implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written.
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut written = writer.write(self.package.as_bytes())?;

        if let Some(ref data) = self.data {
            written += writer.write(b" ")?;
            written += writer.write(data.as_bytes())?;
        }

        Ok(written)
    }
}

impl fmt::Display for GmcpMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.package)?;
        if let Some(ref data) = self.data {
            write!(f, " {}", data)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gmcp_message_new() {
        let msg = GmcpMessage::new("Core.Hello", Some(r#"{"client":"Test"}"#));
        assert_eq!(msg.package(), "Core.Hello");
        assert_eq!(msg.data(), Some(r#"{"client":"Test"}"#));
        assert!(msg.has_data());
    }

    #[test]
    fn test_gmcp_message_command() {
        let msg = GmcpMessage::command("Core.Ping");
        assert_eq!(msg.package(), "Core.Ping");
        assert_eq!(msg.data(), None);
        assert!(!msg.has_data());
    }

    #[test]
    fn test_gmcp_parse_with_data() {
        let data = BytesMut::from(&b"Core.Hello {\"client\":\"Test\"}"[..]);
        let msg = GmcpMessage::parse(&data).unwrap();
        assert_eq!(msg.package(), "Core.Hello");
        assert_eq!(msg.data(), Some(r#"{"client":"Test"}"#));
    }

    #[test]
    fn test_gmcp_parse_without_data() {
        let data = BytesMut::from(&b"Core.Ping"[..]);
        let msg = GmcpMessage::parse(&data).unwrap();
        assert_eq!(msg.package(), "Core.Ping");
        assert_eq!(msg.data(), None);
    }

    #[test]
    fn test_gmcp_parse_msdp_over_gmcp() {
        let data = BytesMut::from(&b"MSDP {\"LIST\":\"COMMANDS\"}"[..]);
        let msg = GmcpMessage::parse(&data).unwrap();
        assert_eq!(msg.package(), "MSDP");
        assert_eq!(msg.data(), Some(r#"{"LIST":"COMMANDS"}"#));
    }

    #[test]
    fn test_gmcp_encode() {
        let msg = GmcpMessage::new("Core.Hello", Some(r#"{"client":"Test"}"#));
        let encoded = msg.encode();
        assert_eq!(
            std::str::from_utf8(&encoded).unwrap(),
            r#"Core.Hello {"client":"Test"}"#
        );
    }

    #[test]
    fn test_gmcp_encode_command_only() {
        let msg = GmcpMessage::command("Core.Ping");
        let encoded = msg.encode();
        assert_eq!(std::str::from_utf8(&encoded).unwrap(), "Core.Ping");
    }

    #[test]
    fn test_gmcp_len() {
        let msg = GmcpMessage::new("Core.Hello", Some(r#"{"client":"Test"}"#));
        assert_eq!(msg.len(), 28); // "Core.Hello " + "{\"client\":\"Test\"}"

        let msg2 = GmcpMessage::command("Core.Ping");
        assert_eq!(msg2.len(), 9); // "Core.Ping"
    }

    #[test]
    fn test_gmcp_display() {
        let msg = GmcpMessage::new("Core.Hello", Some(r#"{"client":"Test"}"#));
        assert_eq!(format!("{}", msg), r#"Core.Hello {"client":"Test"}"#);

        let msg2 = GmcpMessage::command("Core.Ping");
        assert_eq!(format!("{}", msg2), "Core.Ping");
    }

    #[test]
    fn test_gmcp_roundtrip() {
        let original = GmcpMessage::new("Char.Vitals", Some(r#"{"hp":100,"mp":50}"#));
        let encoded = original.encode();
        let parsed = GmcpMessage::parse(&encoded).unwrap();
        assert_eq!(original, parsed);
    }
}
