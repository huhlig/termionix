//
// Copyright 2019 Hans W. Uhlig. All Rights Reserved.
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

use super::{CodecError, TelnetFrame, TelnetOption, TelnetOptions, consts};
use crate::args::TelnetArgument;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};
use tracing::warn;

/// A codec for handling the Telnet protocol, providing functionality to encode and decode Telnet messages.
///
/// `TelnetCodec` is responsible for managing the state and buffers required for processing data
/// when implementing the Telnet protocol. It is typically used in conjunction with asynchronous
/// service libraries to handle the transmission and reception of Telnet messages over a connection.
///
/// This struct is typically paired with a transport-like implementation
/// to facilitate stream I/O management for the Telnet protocol.
pub struct TelnetCodec {
    message_buffer: BytesMut,
    decoder_buffer: BytesMut,
    decoder_state: DecoderState,
    options: TelnetOptions,
    unicode: bool,
    lines: bool,
}

impl TelnetCodec {
    /// Creates a new instance of `TelnetCodec`.
    ///
    /// This constructor initializes the `TelnetCodec` struct with default settings:
    /// - `encoder_buffer`: A buffer for encoding data, initialized to its default state.
    /// - `decoder_buffer`: A buffer for decoding data, initialized to its default state.
    /// - `decoder_state`: The initial state of the decoder, set to `DecoderState::NormalData`.
    /// - `options`: A set of Telnet options, initialized with default values.
    ///
    /// # Returns
    /// A `TelnetCodec` instance ready for use.
    ///
    /// # Example
    /// ```
    /// use termionix_codec::TelnetCodec;
    ///
    /// let codec = TelnetCodec::new();
    /// ```
    pub fn new() -> TelnetCodec {
        TelnetCodec::default()
    }

    /// Checks if a specific Telnet option is enabled locally.
    ///
    /// # Parameters
    /// - `option`: The `TelnetOption` to check the local enablement status for.
    ///
    /// # Returns
    /// - `bool`: Returns `true` if the specified Telnet option is enabled locally, otherwise `false`.
    ///
    /// # Example
    /// ```
    /// use termionix_codec::{TelnetCodec, TelnetOption};
    ///
    /// let codec = TelnetCodec::new();
    /// let is_enabled = codec.is_enabled_local(TelnetOption::Echo);
    /// if is_enabled {
    ///     println!("The Echo option is enabled locally.");
    /// } else {
    ///     println!("The Echo option is not enabled locally.");
    /// }
    /// ```
    ///
    /// # Note
    /// This function uses the `local_enabled` method of the `options` field to determine the enablement status.
    pub fn is_enabled_local(&self, option: TelnetOption) -> bool {
        self.options.local_enabled(option)
    }

    /// Checks if a specific Telnet option is enabled on the remote side.
    ///
    /// # Parameters
    /// - `option`: The `TelnetOption` to check for its enabled status.
    ///
    /// # Returns
    /// - `bool`: Returns `true` if the given Telnet option is enabled on the remote side,
    /// otherwise returns `false`.
    ///
    /// # Example
    /// ```
    /// use termionix_codec::{TelnetCodec, TelnetOption};
    ///
    /// let codec = TelnetCodec::new();
    /// let is_enabled = codec.is_enabled_remote(TelnetOption::Echo);
    /// if is_enabled {
    ///     println!("The Echo option is enabled locally.");
    /// } else {
    ///     println!("The Echo option is not enabled locally.");
    /// }
    /// ```
    pub fn is_enabled_remote(&self, option: TelnetOption) -> bool {
        self.options.remote_enabled(option)
    }
}

impl Default for TelnetCodec {
    fn default() -> Self {
        TelnetCodec {
            message_buffer: BytesMut::new(),
            decoder_buffer: BytesMut::new(),
            decoder_state: DecoderState::NormalData,
            options: TelnetOptions::default(),
            unicode: false,
            lines: false,
        }
    }
}

impl Decoder for TelnetCodec {
    type Item = TelnetFrame;
    type Error = CodecError;

    /// Decodes bytes from the provided `src` buffer into a `TelnetFrame` object by interpreting them
    /// using the internal `decoder_state`. The Telnet protocol supports various control and data
    /// transmission commands that this function processes.
    ///
    /// # Parameters
    /// - `src`: A mutable reference to a `BytesMut` buffer containing the raw data to decode.
    ///
    /// # Returns
    /// Returns a `Result` wrapping an `Option<TelnetFrame>`:
    /// - `Ok(Some(TelnetFrame))`: Successfully decoded a Telnet frame.
    /// - `Ok(None)`: No data was available for decoding; no frames were produced.
    /// - `Err(DecodeError)`: If an error occurred while decoding.
    ///
    /// # Telnet Decoder Workflow
    /// The function reads one byte at a time and performs state-based decoding depending on the
    /// `decoder_state` value and the byte's value. It keeps track of its internal state using
    /// `decoder_state`. The supported states and decoding behavior are as follows:
    ///
    /// ## States
    /// - `DecoderState::NormalData`: Default state to process normal data bytes.
    ///   - Switches to `DecoderState::InterpretAsCommand` upon encountering the `IAC (Interpret As Command)` byte.
    ///   - Emits any normal data frame as `TelnetFrame::Data`.
    ///
    /// - `DecoderState::InterpretAsCommand`: Handles bytes representing Telnet commands.
    ///   - Recognizes standard commands and returns the appropriate `TelnetFrame` variants, such as:
    ///     - `NoOperation`, `DataMark`, `Break`, `InterruptProcess`, `AbortOutput`, `AreYouThere`,
    ///       `EraseCharacter`, `EraseLine`, `GoAhead`.
    ///   - Handles negotiation commands such as `DO`, `DONT`, `WILL`, and `WONT` by transitioning
    ///     to respective negotiation states:
    ///     - `NegotiateDo`, `NegotiateDont`, `NegotiateWill`, `NegotiateWont`.
    ///   - Initiates subnegotiation with the `SB (Subnegotiation)` command by transitioning to
    ///     `DecoderState::Subnegotiate`.
    ///   - For unknown commands, it logs a warning and returns `TelnetFrame::NoOperation`.
    ///
    /// - `DecoderState::NegotiateDo/Dont/Will/Wont`: Completes a negotiation operation with the command
    ///   byte and returns the respective frame: `TelnetFrame::Do`, `TelnetFrame::Dont`, `TelnetFrame::Will`, `TelnetFrame::Wont`.
    ///
    /// - `DecoderState::Subnegotiate`: Starts accumulating subnegotiation arguments into an internal buffer.
    ///   - Transitions to `DecoderState::SubnegotiateArgument` when a subnegotiation byte is received.
    ///   - Handles escape sequences (double `IAC`) during argument accumulation by reverting to
    ///     `DecoderState::SubnegotiateArgumentIAC` and storing the `IAC` byte back into the buffer.
    ///   - Completes subnegotiation when the `SE (Subnegotiation End)` command is processed, returning
    ///     a `TelnetFrame::Subnegotiate` frame with the accumulated buffer data.
    ///   - On invalid commands during subnegotiation, aborts the operation, clears the buffer,
    ///     logs a warning, and returns `TelnetFrame::NoOperation`.
    ///
    /// - `DecoderState::SubnegotiateArgumentIAC`: Special state to handle escape sequences (`IAC`) during
    ///   subnegotiation arguments.
    ///
    /// # Behavior
    /// - The function processes one frame at a time.
    /// - Changes decoder state based on the byte and interprets commands as specified by the Telnet protocol.
    /// - Logs warnings for unknown or unexpected commands.
    /// - Uses an internal buffer for subnegotiation data (`decoder_buffer`).
    ///
    /// # Example Return Values
    /// - `Ok(Some(TelnetFrame::Data(byte)))`: Successfully decoded a normal data byte.
    /// - `Ok(Some(TelnetFrame::NoOperation))`: Processed a `No-Op (NOP)` or invalid/unknown command.
    /// - `Ok(Some(TelnetFrame::Will(option)))`: Processed a `WILL` negotiation command for a specific option.
    /// - `Ok(Some(TelnetFrame::Subnegotiate(option, buffer)))`: Processed subnegotiation with the provided data buffer.
    ///
    /// # Errors
    /// This function may return a `CodecError` if an error occurs during decoding.
    ///
    /// # Notes
    /// - The function is stateful and mutates the decoder's `decoder_state` and internal buffer (`decoder_buffer`) as it works.
    /// - When no bytes remain in the source buffer, the function returns `Ok(None)`, signaling there is
    ///   no new frame yet.
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<TelnetFrame>, Self::Error> {
        while src.remaining() > 0 {
            let byte = src.get_u8();
            match (self.decoder_state, byte) {
                (DecoderState::NormalData, consts::IAC) => {
                    self.decoder_state = DecoderState::InterpretAsCommand;
                }
                (DecoderState::NormalData, _) => {
                    self.decoder_state = DecoderState::NormalData;
                    if !self.options.local_enabled(TelnetOption::TransmitBinary) && byte > 0x7F {
                        warn!(
                            "TransmitBinary option is disabled but received non-ASCII byte: 0x{:02X}",
                            byte
                        );
                    }

                    // Handle line mode
                    if self.lines {
                        // Check for line terminators
                        if byte == b'\r' || byte == b'\n' {
                            // Skip lone LF after CR (handle CRLF as single line)
                            if byte == b'\n' && self.message_buffer.ends_with(&[b'\r']) {
                                // Replace CR with nothing since we'll emit the line now
                                let _ =
                                    self.message_buffer.split_off(self.message_buffer.len() - 1);
                            } else if byte == b'\r' {
                                // Don't add CR to buffer yet, wait to see if LF follows
                                self.message_buffer.put_u8(b'\r');
                            }

                            // Emit line if we have LF or if buffer ends with something other than just CR
                            if byte == b'\n' || !self.message_buffer.is_empty() {
                                let line = String::from_utf8_lossy(&mut self.message_buffer);
                                if byte == b'\n' || !line.is_empty() {
                                    return Ok(Some(TelnetFrame::Line(line.to_string())));
                                }
                            }
                        } else {
                            // Regular character - add to line buffer
                            if self.unicode {
                                // Handle UTF-8 decoding
                                match std::str::from_utf8(&[byte]) {
                                    Ok(s) => self.message_buffer.put_slice(s.as_bytes()),
                                    Err(_) => {
                                        // Invalid UTF-8 sequence, try to collect more bytes
                                        // For now, use replacement character
                                        self.message_buffer.put_u8(byte);
                                    }
                                }
                            } else {
                                // Binary or ASCII mode - just push the byte as-is
                                self.message_buffer.put_u8(byte);
                            }
                        }
                    } else {
                        // Not in line mode - emit data byte directly
                        return Ok(Some(TelnetFrame::Data(byte)));
                    }
                }
                (DecoderState::InterpretAsCommand, consts::NOP) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::NoOperation));
                }
                (DecoderState::InterpretAsCommand, consts::DM) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::DataMark));
                }
                (DecoderState::InterpretAsCommand, consts::BRK) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::Break));
                }
                (DecoderState::InterpretAsCommand, consts::IP) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::InterruptProcess));
                }
                (DecoderState::InterpretAsCommand, consts::AO) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::AbortOutput));
                }
                (DecoderState::InterpretAsCommand, consts::AYT) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::AreYouThere));
                }
                (DecoderState::InterpretAsCommand, consts::EC) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::EraseCharacter));
                }
                (DecoderState::InterpretAsCommand, consts::EL) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::EraseLine));
                }
                (DecoderState::InterpretAsCommand, consts::GA) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::GoAhead));
                }
                (DecoderState::InterpretAsCommand, consts::IAC) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::Data(consts::IAC)));
                }
                (DecoderState::InterpretAsCommand, consts::DO) => {
                    self.decoder_state = DecoderState::NegotiateDo;
                }
                (DecoderState::InterpretAsCommand, consts::DONT) => {
                    self.decoder_state = DecoderState::NegotiateDont;
                }
                (DecoderState::InterpretAsCommand, consts::WILL) => {
                    self.decoder_state = DecoderState::NegotiateWill;
                }
                (DecoderState::InterpretAsCommand, consts::WONT) => {
                    self.decoder_state = DecoderState::NegotiateWont;
                }
                (DecoderState::InterpretAsCommand, consts::SB) => {
                    self.decoder_state = DecoderState::Subnegotiate;
                }
                (DecoderState::InterpretAsCommand, _) => {
                    // Return to NormalData State, and return a No Operation
                    warn!("Received Unknown Command {:#X}", byte);
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::NoOperation));
                }
                (DecoderState::NegotiateDo, _) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::Do(byte.into())));
                }
                (DecoderState::NegotiateDont, _) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::Dont(byte.into())));
                }
                (DecoderState::NegotiateWill, _) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::Will(byte.into())));
                }
                (DecoderState::NegotiateWont, _) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetFrame::Wont(byte.into())));
                }
                (DecoderState::Subnegotiate, _) => {
                    self.decoder_state = DecoderState::SubnegotiateArgument(byte);
                }
                (DecoderState::SubnegotiateArgument(option), consts::IAC) => {
                    self.decoder_state = DecoderState::SubnegotiateArgumentIAC(option);
                }
                (DecoderState::SubnegotiateArgument(_option), _) => {
                    self.decoder_buffer.put_u8(byte);
                }
                (DecoderState::SubnegotiateArgumentIAC(option), consts::IAC) => {
                    self.decoder_state = DecoderState::SubnegotiateArgument(option);
                    self.decoder_buffer.put_u8(consts::IAC);
                }
                (DecoderState::SubnegotiateArgumentIAC(option), consts::SE) => {
                    self.decoder_state = DecoderState::NormalData;
                    let option = TelnetOption::from_u8(option);
                    let buffer = BytesMut::from(self.decoder_buffer.as_ref());
                    let argument = match option {
                        _ => TelnetArgument::Unknown(buffer),
                    };
                    self.decoder_buffer.clear();
                    return Ok(Some(TelnetFrame::Subnegotiate(option, argument)));
                }
                (DecoderState::SubnegotiateArgumentIAC(_), _) => {
                    // TODO: Evaluate if better to return back to SubnegotiateArgumentIAC state and keep buffer
                    self.decoder_state = DecoderState::NormalData;
                    self.decoder_buffer.clear();
                    warn!(
                        "Received Unknown or invalid Command during Subnegotiation {:#X}. Aborting",
                        byte
                    );
                    return Ok(Some(TelnetFrame::NoOperation));
                }
            }
        }
        Ok(None)
    }
}

impl Encoder<&str> for TelnetCodec {
    type Error = CodecError;
    fn encode(&mut self, item: &str, dst: &mut BytesMut) -> Result<(), Self::Error> {
        for byte in item.as_bytes() {
            self.encode(TelnetFrame::Data(*byte), dst)?;
        }
        self.encode(TelnetFrame::Data(b'\r'), dst)?;
        self.encode(TelnetFrame::Data(b'\n'), dst)?;
        Ok(())
    }
}

impl Encoder<TelnetFrame> for TelnetCodec {
    type Error = CodecError;

    /// Encodes a `TelnetFrame` into a byte buffer for transmission over the Telnet protocol.
    ///
    /// # Parameters
    ///
    /// - `item`: The `TelnetFrame` variant that represents a specific Telnet command, data, or negotiation to be encoded.
    /// - `dst`: A mutable reference to a [`BytesMut`] buffer where the encoded bytes for the Telnet frame will be appended.
    ///
    /// # Returns
    ///
    /// - `Ok(())`: If the frame was successfully encoded into the destination buffer.
    /// - `Err(Self::Error)`: If an error occurs during the encoding process (handler dependent).
    ///
    /// # Behavior
    ///
    /// The method matches against the specific `TelnetFrame` variants and encodes them
    /// into the required byte sequences for Telnet protocol specifications. Most frames
    /// begin with the `IAC` (Interpret As Command) byte, followed by specific command or
    /// option bytes, and optionally any additional data.
    ///
    /// ## Variants:
    ///
    /// - `TelnetFrame::Data(ch)`: Encodes a single data byte. If the byte is `IAC` (Interpret As Command),
    ///   it is escaped by writing `IAC` twice.
    /// - `TelnetFrame::NoOperation`: Encodes the `NOP` (No Operation) command.
    /// - `TelnetFrame::DataMark`: Encodes the `DM` (Data Mark) command.
    /// - `TelnetFrame::Break`: Encodes the `BRK` (Break) command.
    /// - `TelnetFrame::InterruptProcess`: Encodes the `IP` (Interrupt Process) command.
    /// - `TelnetFrame::AbortOutput`: Encodes the `AO` (Abort Output) command.
    /// - `TelnetFrame::AreYouThere`: Encodes the `AYT` (Are You There?) command.
    /// - `TelnetFrame::EraseCharacter`: Encodes the `EC` (Erase Character) command.
    /// - `TelnetFrame::EraseLine`: Encodes the `EL` (Erase Line) command.
    /// - `TelnetFrame::GoAhead`: Encodes the `GA` (Go Ahead) command.
    /// - `TelnetFrame::Do(option)`: Encodes the `DO` command with the specified `option`.
    /// - `TelnetFrame::Dont(option)`: Encodes the `DONT` command with the specified `option`.
    /// - `TelnetFrame::Will(option)`: Encodes the `WILL` command with the specified `option`.
    /// - `TelnetFrame::Wont(option)`: Encodes the `WONT` command with the specified `option`.
    /// - `TelnetFrame::Subnegotiate(option, arguments)`: Encodes a subnegotiation sequence, consisting of:
    ///     - `IAC SB` prefix (Subnegotiation start).
    ///     - `option`: A byte indicating the option for the subnegotiation.
    ///     - `arguments`: The subnegotiation payload.
    ///     - `IAC SE` suffix (Subnegotiation end).
    ///
    /// # Encoding Buffer
    ///
    /// The method uses an internal buffer `encoder_buffer`, which is first cleared, then used to construct
    /// the encoded frame. Space is reserved in the buffer prior to encoding to ensure efficiency. After
    /// encoding, the content of `encoder_buffer` is appended to the `dst` buffer.
    ///
    /// # Errors
    ///
    /// This method generally does not produce errors unless there's a fault introduced by the implementing
    /// context (e.g., `Self::Error` defined by the encoder implementation).
    fn encode(&mut self, item: TelnetFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            TelnetFrame::Data(ch) => {
                dst.reserve(2);
                if ch == consts::IAC {
                    dst.put_u8(consts::IAC);
                }
                dst.put_u8(ch);
            }
            TelnetFrame::Line(line) => {
                dst.reserve(line.len());
                dst.put_slice(line.as_bytes());
            }
            TelnetFrame::NoOperation => {
                dst.reserve(2);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::NOP);
            }
            TelnetFrame::DataMark => {
                dst.reserve(2);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::DM);
            }
            TelnetFrame::Break => {
                dst.reserve(2);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::BRK);
            }
            TelnetFrame::InterruptProcess => {
                dst.reserve(2);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::IP);
            }
            TelnetFrame::AbortOutput => {
                dst.reserve(2);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::AO);
            }
            TelnetFrame::AreYouThere => {
                dst.reserve(2);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::AYT);
            }
            TelnetFrame::EraseCharacter => {
                dst.reserve(2);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::EC);
            }
            TelnetFrame::EraseLine => {
                dst.reserve(2);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::EL);
            }
            TelnetFrame::GoAhead => {
                dst.reserve(2);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::GA);
            }
            TelnetFrame::Do(option) => {
                dst.reserve(3);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::DO);
                dst.put_u8(option.into());
            }
            TelnetFrame::Dont(option) => {
                dst.reserve(3);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::DONT);
                dst.put_u8(option.into());
            }
            TelnetFrame::Will(option) => {
                dst.reserve(3);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::WILL);
                dst.put_u8(option.into());
            }
            TelnetFrame::Wont(option) => {
                dst.reserve(3);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::WONT);
                dst.put_u8(option.into());
            }
            TelnetFrame::Subnegotiate(option, argument) => {
                dst.reserve(5 + argument.encoded_len());
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::SB);
                dst.put_u8(option.into());
                argument.encode(dst)?;
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::SE);
            }
        }
        Ok(())
    }
}

///
/// Represents the internal state of a Telnet decoder.
/// This enumeration is used to track the current decoding context while processing Telnet protocol
/// messages.
///
/// # Variants
///
/// - `NormalData`:
///   This state indicates that the decoder is in a normal data mode, processing raw incoming data.
///
/// - `InterpretAsCommand`:
///   This state is entered when an IAC (Interpret As Command) byte is received.
///   The next byte is expected to encode a Telnet command.
///
/// - `NegotiateDo`:
///   This state is entered when a DO command is received. The next byte will indicate the option
///   being requested for negotiation.
///
/// - `NegotiateDont`:
///   This state indicates that a DONT command has been received. The next byte will specify the
///   option to cease negotiation.
///
/// - `NegotiateWill`:
///   This state occurs when a WILL command is received. The next byte is expected to specify the
///   option that the sender will negotiate.
///
/// - `NegotiateWont`:
///   This state is entered upon receiving a WONT command. The subsequent byte specifies which
///   option the sender will not negotiate.
///
/// - `Subnegotiate`:
///   This state indicates the beginning of subnegotiation, where the next byte will specify the
///   option being negotiated.
///
/// - `SubnegotiateArgument(u8)`:
///   Represents the sub-negotiation argument state.
///   Contains the option identifier (as u8) being negotiated. Subsequent bytes comprise the
///   argument data for the subnegotiation.
///
/// - `SubnegotiateArgumentIAC(u8)`:
///   Represents the sub-negotiation state when an IAC (Interpret As Command) byte is received
///   during subnegotiation. Contains the option identifier (as u8). The next byte is expected to
///   indicate a Telnet command or other subnegotiation-specific action.
///
/// # Usage
///
/// This enum is designed to guide the processing logic in a Telnet decoder, ensuring proper
/// handling of each state and transition. By maintaining an appropriate state, the decoder can
/// accurately interpret commands, negotiate options, and process subnegotiation arguments.
/// ```
#[derive(Clone, Copy, Debug)]
enum DecoderState {
    /// Normal Data
    NormalData,
    /// Received IAC, Next byte is Command
    InterpretAsCommand,
    /// Received DO Command, Next Byte is arguments
    NegotiateDo,
    /// Received DONT Command, Next Byte is arguments
    NegotiateDont,
    /// Received WILL Command, Next Byte is arguments
    NegotiateWill,
    /// Received WONT Command, Next Byte is arguments
    NegotiateWont,
    /// Received Subnegotiate Command, Next Byte is arguments
    Subnegotiate,
    /// Received Subnegotiate Option, Next Bytes are arguments
    SubnegotiateArgument(u8),
    /// Received IAC during Subnegotiation, Next Byte is command
    SubnegotiateArgumentIAC(u8),
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // ============================================================================
    // Helper Functions
    // ============================================================================

    fn collect_all(codec: &mut TelnetCodec, mut src: BytesMut) -> Vec<TelnetFrame> {
        let mut out = Vec::new();
        loop {
            match codec.decode(&mut src).expect("decode should not error") {
                Some(frame) => out.push(frame),
                None => break,
            }
        }
        out
    }

    fn encode_frame(frame: TelnetFrame) -> BytesMut {
        let mut codec = TelnetCodec::new();
        let mut dst = BytesMut::new();
        codec.encode(frame, &mut dst).expect("encode ok");
        dst
    }

    fn encode_frames(frames: Vec<TelnetFrame>) -> BytesMut {
        let mut codec = TelnetCodec::new();
        let mut dst = BytesMut::new();
        for frame in frames {
            codec.encode(frame, &mut dst).expect("encode ok");
        }
        dst
    }

    // ============================================================================
    // Encoding Tests - Basic Data
    // ============================================================================

    #[test]
    fn encode_single_data_byte() {
        let dst = encode_frame(TelnetFrame::Data(b'A'));
        assert_eq!(&dst[..], &[b'A']);
    }

    #[test]
    fn encode_data_iac_is_escaped() {
        let dst = encode_frame(TelnetFrame::Data(consts::IAC));
        // IAC as data must be doubled (IAC IAC)
        assert_eq!(&dst[..], &[consts::IAC, consts::IAC]);
    }

    #[test]
    fn encode_multiple_data_bytes() {
        let frames = vec![
            TelnetFrame::Data(b'H'),
            TelnetFrame::Data(b'e'),
            TelnetFrame::Data(b'l'),
            TelnetFrame::Data(b'l'),
            TelnetFrame::Data(b'o'),
        ];
        let dst = encode_frames(frames);
        assert_eq!(&dst[..], b"Hello");
    }

    #[test]
    fn encode_data_with_cr_lf() {
        let frames = vec![
            TelnetFrame::Data(b'H'),
            TelnetFrame::Data(b'i'),
            TelnetFrame::Data(consts::CR),
            TelnetFrame::Data(consts::LF),
        ];
        let dst = encode_frames(frames);
        assert_eq!(&dst[..], b"Hi\r\n");
    }

    // ============================================================================
    // Encoding Tests - Control Commands
    // ============================================================================

    #[test]
    fn encode_no_operation() {
        let dst = encode_frame(TelnetFrame::NoOperation);
        assert_eq!(&dst[..], &[consts::IAC, consts::NOP]);
    }

    #[test]
    fn encode_data_mark() {
        let dst = encode_frame(TelnetFrame::DataMark);
        assert_eq!(&dst[..], &[consts::IAC, consts::DM]);
    }

    #[test]
    fn encode_break() {
        let dst = encode_frame(TelnetFrame::Break);
        assert_eq!(&dst[..], &[consts::IAC, consts::BRK]);
    }

    #[test]
    fn encode_interrupt_process() {
        let dst = encode_frame(TelnetFrame::InterruptProcess);
        assert_eq!(&dst[..], &[consts::IAC, consts::IP]);
    }

    #[test]
    fn encode_abort_output() {
        let dst = encode_frame(TelnetFrame::AbortOutput);
        assert_eq!(&dst[..], &[consts::IAC, consts::AO]);
    }

    #[test]
    fn encode_are_you_there() {
        let dst = encode_frame(TelnetFrame::AreYouThere);
        assert_eq!(&dst[..], &[consts::IAC, consts::AYT]);
    }

    #[test]
    fn encode_erase_character() {
        let dst = encode_frame(TelnetFrame::EraseCharacter);
        assert_eq!(&dst[..], &[consts::IAC, consts::EC]);
    }

    #[test]
    fn encode_erase_line() {
        let dst = encode_frame(TelnetFrame::EraseLine);
        assert_eq!(&dst[..], &[consts::IAC, consts::EL]);
    }

    #[test]
    fn encode_go_ahead() {
        let dst = encode_frame(TelnetFrame::GoAhead);
        assert_eq!(&dst[..], &[consts::IAC, consts::GA]);
    }

    // ============================================================================
    // Encoding Tests - Negotiation Commands
    // ============================================================================

    #[test]
    fn encode_do_binary() {
        let dst = encode_frame(TelnetFrame::Do(TelnetOption::TransmitBinary));
        assert_eq!(&dst[..], &[consts::IAC, consts::DO, consts::option::BINARY]);
    }

    #[test]
    fn encode_dont_echo() {
        let dst = encode_frame(TelnetFrame::Dont(TelnetOption::Echo));
        assert_eq!(&dst[..], &[consts::IAC, consts::DONT, consts::option::ECHO]);
    }

    #[test]
    fn encode_will_sga() {
        let dst = encode_frame(TelnetFrame::Will(TelnetOption::SuppressGoAhead));
        assert_eq!(&dst[..], &[consts::IAC, consts::WILL, consts::option::SGA]);
    }

    #[test]
    fn encode_wont_binary() {
        let dst = encode_frame(TelnetFrame::Wont(TelnetOption::TransmitBinary));
        assert_eq!(
            &dst[..],
            &[consts::IAC, consts::WONT, consts::option::BINARY]
        );
    }

    #[test]
    fn encode_multiple_negotiation_commands() {
        let frames = vec![
            TelnetFrame::Do(TelnetOption::Echo),
            TelnetFrame::Will(TelnetOption::SuppressGoAhead),
            TelnetFrame::Dont(TelnetOption::TransmitBinary),
        ];
        let dst = encode_frames(frames);
        assert_eq!(
            &dst[..],
            &[
                consts::IAC,
                consts::DO,
                consts::option::ECHO,
                consts::IAC,
                consts::WILL,
                consts::option::SGA,
                consts::IAC,
                consts::DONT,
                consts::option::BINARY,
            ]
        );
    }

    // ============================================================================
    // Encoding Tests - Subnegotiation
    // ============================================================================

    #[test]
    fn encode_subnegotiation_empty() {
        let dst = encode_frame(TelnetFrame::Subnegotiate(
            TelnetOption::TransmitBinary,
            TelnetArgument::Unknown(BytesMut::new()),
        ));
        assert_eq!(
            &dst[..],
            &[
                consts::IAC,
                consts::SB,
                consts::option::BINARY,
                consts::IAC,
                consts::SE,
            ]
        );
    }

    #[test]
    fn encode_subnegotiation_with_args() {
        let args = BytesMut::from(&[0x01, 0x02, 0x03][..]);
        let dst = encode_frame(TelnetFrame::Subnegotiate(
            TelnetOption::TransmitBinary,
            TelnetArgument::Unknown(args),
        ));
        assert_eq!(
            &dst[..],
            &[
                consts::IAC,
                consts::SB,
                consts::option::BINARY,
                0x01,
                0x02,
                0x03,
                consts::IAC,
                consts::SE,
            ]
        );
    }

    #[test]
    fn encode_subnegotiation_with_iac_in_args() {
        // Note: The encoder does NOT escape IAC in subnegotiation args
        // This may be a bug or intentional - documenting current behavior
        let args = BytesMut::from(&[0x01, consts::IAC, 0x03][..]);
        let dst = encode_frame(TelnetFrame::Subnegotiate(
            TelnetOption::TransmitBinary,
            TelnetArgument::Unknown(args),
        ));
        assert_eq!(
            &dst[..],
            &[
                consts::IAC,
                consts::SB,
                consts::option::BINARY,
                0x01,
                consts::IAC,
                0x03,
                consts::IAC,
                consts::SE,
            ]
        );
    }

    // ============================================================================
    // Encoding Tests - String Encoder
    // ============================================================================

    #[test]
    fn encode_string_simple() {
        let mut codec = TelnetCodec::new();
        let mut dst = BytesMut::new();
        codec.encode("Hello", &mut dst).expect("encode ok");
        assert_eq!(&dst[..], b"Hello\r\n");
    }

    #[test]
    fn encode_string_empty() {
        let mut codec = TelnetCodec::new();
        let mut dst = BytesMut::new();
        codec.encode("", &mut dst).expect("encode ok");
        assert_eq!(&dst[..], b"\r\n");
    }

    // ============================================================================
    // Decoding Tests - Basic Data
    // ============================================================================

    #[test]
    fn decode_single_data_byte() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&b"A"[..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::Data(b'A')]);
    }

    #[test]
    fn decode_multiple_data_bytes() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&b"Hello"[..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![
                TelnetFrame::Data(b'H'),
                TelnetFrame::Data(b'e'),
                TelnetFrame::Data(b'l'),
                TelnetFrame::Data(b'l'),
                TelnetFrame::Data(b'o'),
            ]
        );
    }

    #[test]
    fn decode_data_with_cr_lf() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&b"Line\r\n"[..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![
                TelnetFrame::Data(b'L'),
                TelnetFrame::Data(b'i'),
                TelnetFrame::Data(b'n'),
                TelnetFrame::Data(b'e'),
                TelnetFrame::Data(consts::CR),
                TelnetFrame::Data(consts::LF),
            ]
        );
    }

    #[test]
    fn decode_empty_buffer() {
        let mut codec = TelnetCodec::new();
        let mut src = BytesMut::new();
        let result = codec.decode(&mut src).expect("decode ok");
        assert!(result.is_none());
    }

    // ============================================================================
    // Decoding Tests - IAC Handling
    // ============================================================================

    #[test]
    fn decode_iac_iac_as_data() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::IAC][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::Data(consts::IAC)]);
    }

    #[test]
    fn decode_iac_nop() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::NOP][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::NoOperation]);
    }

    #[test]
    fn decode_iac_dm() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::DM][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::DataMark]);
    }

    #[test]
    fn decode_iac_brk() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::BRK][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::Break]);
    }

    #[test]
    fn decode_iac_ip() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::IP][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::InterruptProcess]);
    }

    #[test]
    fn decode_iac_ao() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::AO][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::AbortOutput]);
    }

    #[test]
    fn decode_iac_ayt() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::AYT][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::AreYouThere]);
    }

    #[test]
    fn decode_iac_ec() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::EC][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::EraseCharacter]);
    }

    #[test]
    fn decode_iac_el() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::EL][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::EraseLine]);
    }

    #[test]
    fn decode_iac_ga() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::GA][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::GoAhead]);
    }

    #[test]
    fn decode_unknown_iac_command_yields_noop() {
        let mut codec = TelnetCodec::new();
        // 0x00 is not a valid Telnet command
        let src = BytesMut::from(&[consts::IAC, 0x00][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::NoOperation]);
    }

    // ============================================================================
    // Decoding Tests - Negotiation Commands
    // ============================================================================

    #[test]
    fn decode_do_binary() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::BINARY][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::Do(TelnetOption::TransmitBinary)]);
    }

    #[test]
    fn decode_dont_echo() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::DONT, consts::option::ECHO][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::Dont(TelnetOption::Echo)]);
    }

    #[test]
    fn decode_will_sga() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::WILL, consts::option::SGA][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![TelnetFrame::Will(TelnetOption::SuppressGoAhead)]
        );
    }

    #[test]
    fn decode_wont_binary() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::WONT, consts::option::BINARY][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![TelnetFrame::Wont(TelnetOption::TransmitBinary)]
        );
    }

    #[test]
    fn decode_unknown_option() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::EXOPL][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::Do(TelnetOption::EXOPL)]);
    }

    // ============================================================================
    // Decoding Tests - Subnegotiation
    // ============================================================================

    #[test]
    fn decode_subnegotiation_empty() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(
            &[
                consts::IAC,
                consts::SB,
                consts::option::BINARY,
                consts::IAC,
                consts::SE,
            ][..],
        );
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![TelnetFrame::Subnegotiate(
                TelnetOption::TransmitBinary,
                TelnetArgument::Unknown(BytesMut::new())
            )]
        );
    }

    #[test]
    fn decode_subnegotiation_with_args() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(
            &[
                consts::IAC,
                consts::SB,
                consts::option::BINARY,
                0x01,
                0x02,
                0x03,
                consts::IAC,
                consts::SE,
            ][..],
        );
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![TelnetFrame::Subnegotiate(
                TelnetOption::TransmitBinary,
                TelnetArgument::Unknown(BytesMut::from(&[0x01, 0x02, 0x03][..]))
            )]
        );
    }

    #[test]
    fn decode_subnegotiation_with_escaped_iac() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(
            &[
                consts::IAC,
                consts::SB,
                consts::option::BINARY,
                0x01,
                consts::IAC,
                consts::IAC, // escaped IAC in subnegotiation
                0x03,
                consts::IAC,
                consts::SE,
            ][..],
        );
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![TelnetFrame::Subnegotiate(
                TelnetOption::TransmitBinary,
                TelnetArgument::Unknown(BytesMut::from(&[0x01, consts::IAC, 0x03][..]))
            )]
        );
    }

    #[test]
    fn decode_subnegotiation_invalid_command_aborts() {
        let mut codec = TelnetCodec::new();
        // Invalid: IAC followed by non-SE during subnegotiation
        let src = BytesMut::from(
            &[
                consts::IAC,
                consts::SB,
                consts::option::BINARY,
                0x01,
                consts::IAC,
                0x00, // Invalid command
            ][..],
        );
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetFrame::NoOperation]);
    }

    // ============================================================================
    // Decoding Tests - Mixed Data and Commands
    // ============================================================================

    #[test]
    fn decode_data_with_interspersed_commands() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(
            &[
                b'H',
                b'i',
                consts::IAC,
                consts::NOP,
                b'!',
                consts::IAC,
                consts::DO,
                consts::option::ECHO,
                b'B',
                b'y',
                b'e',
            ][..],
        );
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![
                TelnetFrame::Data(b'H'),
                TelnetFrame::Data(b'i'),
                TelnetFrame::NoOperation,
                TelnetFrame::Data(b'!'),
                TelnetFrame::Do(TelnetOption::Echo),
                TelnetFrame::Data(b'B'),
                TelnetFrame::Data(b'y'),
                TelnetFrame::Data(b'e'),
            ]
        );
    }

    #[test]
    fn decode_login_sequence_from_rfc() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(
            &[
                b'L',
                b'o',
                b'g',
                b'i',
                b'n',
                b':',
                consts::CR,
                consts::LF,
                consts::IAC,
                consts::DO,
                consts::option::BINARY,
                b'P',
                b'a',
                b's',
                b's',
                b'w',
                b'o',
                b'r',
                b'd',
                b':',
                consts::CR,
                consts::LF,
                consts::IAC,
                consts::WILL,
                consts::option::BINARY,
                b'H',
                b'e',
                b'l',
                b'l',
                b'o',
                b'!',
                consts::CR,
                consts::LF,
            ][..],
        );
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![
                TelnetFrame::Data(b'L'),
                TelnetFrame::Data(b'o'),
                TelnetFrame::Data(b'g'),
                TelnetFrame::Data(b'i'),
                TelnetFrame::Data(b'n'),
                TelnetFrame::Data(b':'),
                TelnetFrame::Data(consts::CR),
                TelnetFrame::Data(consts::LF),
                TelnetFrame::Do(TelnetOption::TransmitBinary),
                TelnetFrame::Data(b'P'),
                TelnetFrame::Data(b'a'),
                TelnetFrame::Data(b's'),
                TelnetFrame::Data(b's'),
                TelnetFrame::Data(b'w'),
                TelnetFrame::Data(b'o'),
                TelnetFrame::Data(b'r'),
                TelnetFrame::Data(b'd'),
                TelnetFrame::Data(b':'),
                TelnetFrame::Data(consts::CR),
                TelnetFrame::Data(consts::LF),
                TelnetFrame::Will(TelnetOption::TransmitBinary),
                TelnetFrame::Data(b'H'),
                TelnetFrame::Data(b'e'),
                TelnetFrame::Data(b'l'),
                TelnetFrame::Data(b'l'),
                TelnetFrame::Data(b'o'),
                TelnetFrame::Data(b'!'),
                TelnetFrame::Data(consts::CR),
                TelnetFrame::Data(consts::LF),
            ]
        );
    }

    // ============================================================================
    // Round-trip Tests (Encode then Decode)
    // ============================================================================

    #[test]
    fn roundtrip_simple_data() {
        let original = vec![
            TelnetFrame::Data(b'H'),
            TelnetFrame::Data(b'e'),
            TelnetFrame::Data(b'l'),
            TelnetFrame::Data(b'l'),
            TelnetFrame::Data(b'o'),
        ];
        let encoded = encode_frames(original.clone());
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn roundtrip_iac_data() {
        let original = vec![TelnetFrame::Data(consts::IAC)];
        let encoded = encode_frames(original.clone());
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn roundtrip_negotiation() {
        let original = vec![
            TelnetFrame::Do(TelnetOption::Echo),
            TelnetFrame::Will(TelnetOption::SuppressGoAhead),
            TelnetFrame::Dont(TelnetOption::TransmitBinary),
            TelnetFrame::Wont(TelnetOption::Echo),
        ];
        let encoded = encode_frames(original.clone());
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn roundtrip_subnegotiation() {
        let original = vec![TelnetFrame::Subnegotiate(
            TelnetOption::TransmitBinary,
            TelnetArgument::Unknown(BytesMut::from(&[0x01, 0x02, 0x03][..])),
        )];
        let encoded = encode_frames(original.clone());
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn roundtrip_control_commands() {
        let original = vec![
            TelnetFrame::NoOperation,
            TelnetFrame::DataMark,
            TelnetFrame::Break,
            TelnetFrame::InterruptProcess,
            TelnetFrame::AbortOutput,
            TelnetFrame::AreYouThere,
            TelnetFrame::EraseCharacter,
            TelnetFrame::EraseLine,
            TelnetFrame::GoAhead,
        ];
        let encoded = encode_frames(original.clone());
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(original, decoded);
    }

    #[test]
    fn roundtrip_mixed_content() {
        let original = vec![
            TelnetFrame::Data(b'H'),
            TelnetFrame::Data(b'i'),
            TelnetFrame::NoOperation,
            TelnetFrame::Do(TelnetOption::Echo),
            TelnetFrame::Data(b'!'),
        ];
        let encoded = encode_frames(original.clone());
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(original, decoded);
    }

    // ============================================================================
    // State Machine Tests - Partial Reads
    // ============================================================================

    #[test]
    fn decode_partial_iac_command() {
        let mut codec = TelnetCodec::new();
        let mut src = BytesMut::from(&[consts::IAC][..]);

        // First read: incomplete
        let result = codec.decode(&mut src).expect("decode ok");
        assert!(result.is_none());

        // Complete the command
        src.put_u8(consts::NOP);
        let result = codec.decode(&mut src).expect("decode ok");
        assert_eq!(result, Some(TelnetFrame::NoOperation));
    }

    #[test]
    fn decode_partial_negotiation() {
        let mut codec = TelnetCodec::new();
        let mut src = BytesMut::from(&[consts::IAC, consts::DO][..]);

        // Incomplete negotiation
        let result = codec.decode(&mut src).expect("decode ok");
        assert!(result.is_none());

        // Complete it
        src.put_u8(consts::option::ECHO);
        let result = codec.decode(&mut src).expect("decode ok");
        assert_eq!(result, Some(TelnetFrame::Do(TelnetOption::Echo)));
    }

    #[test]
    fn decode_partial_subnegotiation() {
        let mut codec = TelnetCodec::new();
        let mut src = BytesMut::from(&[consts::IAC, consts::SB, consts::option::BINARY, 0x01][..]);

        // Incomplete subnegotiation
        let result = codec.decode(&mut src).expect("decode ok");
        assert!(result.is_none());

        // Complete it
        src.extend_from_slice(&[consts::IAC, consts::SE]);
        let result = codec.decode(&mut src).expect("decode ok");
        assert_eq!(
            result,
            Some(TelnetFrame::Subnegotiate(
                TelnetOption::TransmitBinary,
                TelnetArgument::Unknown(BytesMut::from(&[0x01][..]))
            ))
        );
    }

    // ============================================================================
    // Edge Cases
    // ============================================================================

    #[test]
    fn decode_multiple_iac_sequences() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(
            &[
                consts::IAC,
                consts::IAC, // data 0xFF
                consts::IAC,
                consts::IAC, // data 0xFF
                consts::IAC,
                consts::IAC, // data 0xFF
            ][..],
        );
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![
                TelnetFrame::Data(consts::IAC),
                TelnetFrame::Data(consts::IAC),
                TelnetFrame::Data(consts::IAC),
            ]
        );
    }

    #[test]
    fn decode_bad_iac_data() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[0x80, 0xFF, 0x7F][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![TelnetFrame::Data(0x80), TelnetFrame::NoOperation,]
        );
    }

    #[test]
    fn encode_decode_all_options() {
        // Test a sample of different option values
        let options = vec![
            TelnetOption::TransmitBinary,
            TelnetOption::Echo,
            TelnetOption::SuppressGoAhead,
            TelnetOption::NAWS,
            TelnetOption::GMCP,
            TelnetOption::Unknown(200),
        ];

        for option in options {
            let frames = vec![
                TelnetFrame::Do(option),
                TelnetFrame::Dont(option),
                TelnetFrame::Will(option),
                TelnetFrame::Wont(option),
            ];
            let encoded = encode_frames(frames.clone());
            let mut codec = TelnetCodec::new();
            let decoded = collect_all(&mut codec, encoded);
            assert_eq!(frames, decoded);
        }
    }

    #[test]
    fn codec_default_creates_new_instance() {
        let codec1 = TelnetCodec::default();
        let codec2 = TelnetCodec::new();

        // Both should work identically
        let mut dst1 = BytesMut::new();
        let mut dst2 = BytesMut::new();

        let mut c1 = codec1;
        let mut c2 = codec2;

        c1.encode(TelnetFrame::Data(b'A'), &mut dst1).unwrap();
        c2.encode(TelnetFrame::Data(b'A'), &mut dst2).unwrap();

        assert_eq!(dst1, dst2);
    }
}
