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

use super::{CodecError, TelnetEvent, TelnetFrame, TelnetOption, consts};
use crate::args::TelnetArgument;
use crate::args::gmcp::GmcpMessage;
use crate::options::{TelnetOptions, TelnetSide};
use bytes::{Buf, BufMut, BytesMut};
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
    decoder_buffer: BytesMut,
    decoder_state: DecoderState,
    options: TelnetOptions,
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
    /// use termionix_telnetcodec::TelnetCodec;
    ///
    /// let codec = TelnetCodec::new();
    /// ```
    pub fn new() -> TelnetCodec {
        TelnetCodec::default()
    }

    /// Checks if we support the given option locally
    pub fn is_supported_local(&self, option: TelnetOption) -> bool {
        self.options.is_supported_local(option)
    }

    /// Checks if we support the given option remotely
    pub fn is_supported_remote(&self, option: TelnetOption) -> bool {
        self.options.is_supported_remote(option)
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
    /// use termionix_telnetcodec::{TelnetCodec, TelnetOption};
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
    /// use termionix_telnetcodec::{TelnetCodec, TelnetOption};
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

    /// Request to enable a Telnet option locally (we will send WILL).
    ///
    /// This initiates the negotiation process using the Q-method state machine.
    /// If negotiation is needed, returns a frame that should be sent to the remote side.
    ///
    /// # Parameters
    /// - `option`: The `TelnetOption` to enable locally.
    ///
    /// # Returns
    /// - `Some(TelnetFrame)`: A negotiation frame to send to the remote side.
    /// - `None`: No negotiation needed (option already enabled or not supported).
    ///
    /// # Example
    /// ```
    /// use termionix_telnetcodec::{TelnetCodec, TelnetOption};
    ///
    /// let mut codec = TelnetCodec::new();
    /// if let Some(frame) = codec.enable_local(TelnetOption::Echo) {
    ///     // Send frame to remote side
    /// }
    /// ```
    pub fn enable_local(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        self.options.enable_local(option)
    }

    /// Request to disable a Telnet option locally (we will send WONT).
    ///
    /// This initiates the negotiation process using the Q-method state machine.
    /// If negotiation is needed, returns a frame that should be sent to the remote side.
    ///
    /// # Parameters
    /// - `option`: The `TelnetOption` to disable locally.
    ///
    /// # Returns
    /// - `Some(TelnetFrame)`: A negotiation frame to send to the remote side.
    /// - `None`: No negotiation needed (option already disabled).
    ///
    /// # Example
    /// ```
    /// use termionix_telnetcodec::{TelnetCodec, TelnetOption};
    ///
    /// let mut codec = TelnetCodec::new();
    /// if let Some(frame) = codec.disable_local(TelnetOption::Echo) {
    ///     // Send frame to remote side
    /// }
    /// ```
    pub fn disable_local(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        self.options.disable_local(option)
    }

    /// Request to enable a Telnet option on the remote side (we will send DO).
    ///
    /// This initiates the negotiation process using the Q-method state machine.
    /// If negotiation is needed, returns a frame that should be sent to the remote side.
    ///
    /// # Parameters
    /// - `option`: The `TelnetOption` to enable on the remote side.
    ///
    /// # Returns
    /// - `Some(TelnetFrame)`: A negotiation frame to send to the remote side.
    /// - `None`: No negotiation needed (option already enabled or not supported).
    ///
    /// # Example
    /// ```
    /// use termionix_telnetcodec::{TelnetCodec, TelnetOption};
    ///
    /// let mut codec = TelnetCodec::new();
    /// if let Some(frame) = codec.enable_remote(TelnetOption::SuppressGoAhead) {
    ///     // Send frame to remote side
    /// }
    /// ```
    pub fn enable_remote(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        self.options.enable_remote(option)
    }

    /// Request to disable a Telnet option on the remote side (we will send DONT).
    ///
    /// This initiates the negotiation process using the Q-method state machine.
    /// If negotiation is needed, returns a frame that should be sent to the remote side.
    ///
    /// # Parameters
    /// - `option`: The `TelnetOption` to disable on the remote side.
    ///
    /// # Returns
    /// - `Some(TelnetFrame)`: A negotiation frame to send to the remote side.
    /// - `None`: No negotiation needed (option already disabled).
    ///
    /// # Example
    /// ```
    /// use termionix_telnetcodec::{TelnetCodec, TelnetOption};
    ///
    /// let mut codec = TelnetCodec::new();
    /// if let Some(frame) = codec.disable_remote(TelnetOption::Echo) {
    ///     // Send frame to remote side
    /// }
    /// ```
    pub fn disable_remote(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        self.options.disable_remote(option)
    }
}

impl Default for TelnetCodec {
    fn default() -> Self {
        TelnetCodec {
            decoder_buffer: BytesMut::new(),
            decoder_state: DecoderState::NormalData,
            options: TelnetOptions::default(),
        }
    }
}

impl Decoder for TelnetCodec {
    type Item = TelnetEvent;
    type Error = CodecError;

    /// Decodes bytes from the provided `src` buffer into a `TelnetEvent` object by interpreting them
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
    ///   - For unknown commands, it logs a warning and returns `TelnetEvent::NoOperation`.
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
    ///     logs a warning, and returns `TelnetEvent::NoOperation`.
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
    /// - `Ok(Some(TelnetEvent::NoOperation))`: Processed a `No-Op (NOP)` or invalid/unknown command.
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
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<TelnetEvent>, Self::Error> {
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

                    return Ok(Some(TelnetEvent::Data(byte)));
                }
                (DecoderState::InterpretAsCommand, consts::NOP) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetEvent::NoOperation));
                }
                (DecoderState::InterpretAsCommand, consts::DM) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetEvent::DataMark));
                }
                (DecoderState::InterpretAsCommand, consts::BRK) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetEvent::Break));
                }
                (DecoderState::InterpretAsCommand, consts::IP) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetEvent::InterruptProcess));
                }
                (DecoderState::InterpretAsCommand, consts::AO) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetEvent::AbortOutput));
                }
                (DecoderState::InterpretAsCommand, consts::AYT) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetEvent::AreYouThere));
                }
                (DecoderState::InterpretAsCommand, consts::EC) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetEvent::EraseCharacter));
                }
                (DecoderState::InterpretAsCommand, consts::EL) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetEvent::EraseLine));
                }
                (DecoderState::InterpretAsCommand, consts::GA) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetEvent::GoAhead));
                }
                (DecoderState::InterpretAsCommand, consts::EOR) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetEvent::EndOfRecord));
                }
                (DecoderState::InterpretAsCommand, consts::IAC) => {
                    self.decoder_state = DecoderState::NormalData;
                    return Ok(Some(TelnetEvent::Data(consts::IAC)));
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
                    return Ok(Some(TelnetEvent::NoOperation));
                }
                (DecoderState::NegotiateDo, _) => {
                    self.decoder_state = DecoderState::NormalData;
                    let option: TelnetOption = byte.into();
                    let frame = TelnetFrame::Do(option);
                    // Check QState before processing - DO affects LOCAL side
                    let was_yes = matches!(
                        self.options.local_qstate(option),
                        crate::options::QState::Yes
                    );
                    // Process through QState machine
                    self.options.handle_received(frame)?;
                    // Check if we transitioned to/from Yes state
                    let is_yes = matches!(
                        self.options.local_qstate(option),
                        crate::options::QState::Yes
                    );
                    if is_yes != was_yes {
                        return Ok(Some(TelnetEvent::OptionStatus(
                            option,
                            TelnetSide::Local,
                            is_yes,
                        )));
                    }
                    continue;
                }
                (DecoderState::NegotiateDont, _) => {
                    self.decoder_state = DecoderState::NormalData;
                    let option: TelnetOption = byte.into();
                    let frame = TelnetFrame::Dont(option);
                    // Check QState before processing - DONT affects LOCAL side
                    let was_yes = matches!(
                        self.options.local_qstate(option),
                        crate::options::QState::Yes
                    );
                    // Process through QState machine
                    self.options.handle_received(frame)?;
                    // Check if we transitioned to/from Yes state
                    let is_yes = matches!(
                        self.options.local_qstate(option),
                        crate::options::QState::Yes
                    );
                    if is_yes != was_yes {
                        return Ok(Some(TelnetEvent::OptionStatus(
                            option,
                            TelnetSide::Local,
                            is_yes,
                        )));
                    }
                    continue;
                }
                (DecoderState::NegotiateWill, _) => {
                    self.decoder_state = DecoderState::NormalData;
                    let option: TelnetOption = byte.into();
                    let frame = TelnetFrame::Will(option);
                    // Check QState before processing - WILL affects REMOTE side
                    let was_yes = matches!(
                        self.options.remote_qstate(option),
                        crate::options::QState::Yes
                    );
                    // Process through QState machine
                    self.options.handle_received(frame)?;
                    // Check if we transitioned to/from Yes state
                    let is_yes = matches!(
                        self.options.remote_qstate(option),
                        crate::options::QState::Yes
                    );
                    if is_yes != was_yes {
                        return Ok(Some(TelnetEvent::OptionStatus(
                            option,
                            TelnetSide::Remote,
                            is_yes,
                        )));
                    }
                    continue;
                }
                (DecoderState::NegotiateWont, _) => {
                    self.decoder_state = DecoderState::NormalData;
                    let option: TelnetOption = byte.into();
                    let frame = TelnetFrame::Wont(option);
                    // Check QState before processing - WONT affects REMOTE side
                    let was_yes = matches!(
                        self.options.remote_qstate(option),
                        crate::options::QState::Yes
                    );
                    // Process through QState machine
                    self.options.handle_received(frame)?;
                    // Check if we transitioned to/from Yes state
                    let is_yes = matches!(
                        self.options.remote_qstate(option),
                        crate::options::QState::Yes
                    );
                    if is_yes != was_yes {
                        return Ok(Some(TelnetEvent::OptionStatus(
                            option,
                            TelnetSide::Remote,
                            is_yes,
                        )));
                    }
                    continue;
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
                        TelnetOption::GMCP => {
                            // Parse GMCP message from buffer
                            if let Some(gmcp_msg) = GmcpMessage::parse(&buffer) {
                                TelnetArgument::GMCP(gmcp_msg)
                            } else {
                                // If parsing fails, treat as unknown
                                warn!("Failed to parse GMCP message, treating as unknown");
                                TelnetArgument::Unknown(option, buffer)
                            }
                        }
                        _ => TelnetArgument::Unknown(option, buffer),
                    };
                    self.decoder_buffer.clear();
                    return Ok(Some(TelnetEvent::Subnegotiate(argument)));
                }
                (DecoderState::SubnegotiateArgumentIAC(_), _) => {
                    // TODO: Evaluate if better to return back to SubnegotiateArgumentIAC state and keep buffer
                    self.decoder_state = DecoderState::NormalData;
                    self.decoder_buffer.clear();
                    warn!(
                        "Received Unknown or invalid Command during Subnegotiation {:#X}. Aborting",
                        byte
                    );
                    return Ok(Some(TelnetEvent::NoOperation));
                }
            }
        }
        Ok(None)
    }
}

impl Encoder<char> for TelnetCodec {
    type Error = CodecError;

    fn encode(&mut self, item: char, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(4);
        if item.is_ascii() {
            let ch = item as u8;
            if ch == consts::IAC {
                dst.put_u8(consts::IAC);
            }
            dst.put_u8(ch);
        } else {
            let mut buf = [0; 4];
            item.encode_utf8(&mut buf);
            dst.put_slice(&buf[0..item.len_utf8()]);
        }
        Ok(())
    }
}

impl Encoder<u8> for TelnetCodec {
    type Error = CodecError;

    fn encode(&mut self, item: u8, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Encode a raw byte, escaping IAC if necessary
        dst.reserve(2);
        if item == consts::IAC {
            dst.put_u8(consts::IAC);
        }
        dst.put_u8(item);
        Ok(())
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
    /// - `TelnetEvent::NoOperation`: Encodes the `NOP` (No Operation) command.
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
            TelnetFrame::EndOfRecord => {
                dst.reserve(2);
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::EOR);
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
            TelnetFrame::Subnegotiate(argument) => {
                dst.reserve(5 + argument.len());
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::SB);
                dst.put_u8(argument.option().to_u8());
                argument.encode(dst)?;
                dst.put_u8(consts::IAC);
                dst.put_u8(consts::SE);
            }
        }
        Ok(())
    }
}

impl Encoder<TelnetEvent> for TelnetCodec {
    type Error = CodecError;

    /// Encodes a `TelnetEvent` into a byte buffer for transmission over the Telnet protocol.
    ///
    /// Note: `OptionStatus` events are informational only and cannot be encoded.
    /// They represent completed negotiations and are emitted by the decoder.
    fn encode(&mut self, item: TelnetEvent, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            TelnetEvent::Data(byte) => self.encode(TelnetFrame::Data(byte), dst),
            TelnetEvent::NoOperation => self.encode(TelnetEvent::NoOperation, dst),
            TelnetEvent::DataMark => self.encode(TelnetFrame::DataMark, dst),
            TelnetEvent::Break => self.encode(TelnetFrame::Break, dst),
            TelnetEvent::InterruptProcess => self.encode(TelnetFrame::InterruptProcess, dst),
            TelnetEvent::AbortOutput => self.encode(TelnetFrame::AbortOutput, dst),
            TelnetEvent::AreYouThere => self.encode(TelnetFrame::AreYouThere, dst),
            TelnetEvent::EraseCharacter => self.encode(TelnetFrame::EraseCharacter, dst),
            TelnetEvent::EraseLine => self.encode(TelnetFrame::EraseLine, dst),
            TelnetEvent::GoAhead => self.encode(TelnetFrame::GoAhead, dst),
            TelnetEvent::EndOfRecord => self.encode(TelnetFrame::EndOfRecord, dst),
            TelnetEvent::Subnegotiate(arg) => self.encode(TelnetFrame::Subnegotiate(arg), dst),
            TelnetEvent::OptionStatus(_option, _side, _enabled) => {
                // OptionStatus events are informational only and cannot be encoded
                // They represent the result of negotiation, not a command to send
                warn!("Attempted to encode OptionStatus event - this is informational only");
                Ok(())
            }
        }
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

    fn collect_all(codec: &mut TelnetCodec, mut src: BytesMut) -> Vec<TelnetEvent> {
        let mut out = Vec::new();
        loop {
            match codec.decode(&mut src).expect("decode should not error") {
                Some(event) => out.push(event),
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
        let dst = encode_frame(TelnetFrame::Subnegotiate(TelnetArgument::Unknown(
            TelnetOption::TransmitBinary,
            BytesMut::new(),
        )));
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
        let dst = encode_frame(TelnetFrame::Subnegotiate(TelnetArgument::Unknown(
            TelnetOption::TransmitBinary,
            args,
        )));
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
        let dst = encode_frame(TelnetFrame::Subnegotiate(TelnetArgument::Unknown(
            TelnetOption::TransmitBinary,
            args,
        )));
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
        assert_eq!(frames, vec![TelnetEvent::Data(b'A')]);
    }

    #[test]
    fn decode_multiple_data_bytes() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&b"Hello"[..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![
                TelnetEvent::Data(b'H'),
                TelnetEvent::Data(b'e'),
                TelnetEvent::Data(b'l'),
                TelnetEvent::Data(b'l'),
                TelnetEvent::Data(b'o'),
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
                TelnetEvent::Data(b'L'),
                TelnetEvent::Data(b'i'),
                TelnetEvent::Data(b'n'),
                TelnetEvent::Data(b'e'),
                TelnetEvent::Data(consts::CR),
                TelnetEvent::Data(consts::LF),
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
        assert_eq!(frames, vec![TelnetEvent::Data(consts::IAC)]);
    }

    #[test]
    fn decode_iac_nop() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::NOP][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetEvent::NoOperation]);
    }

    #[test]
    fn decode_iac_dm() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::DM][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetEvent::DataMark]);
    }

    #[test]
    fn decode_iac_brk() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::BRK][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetEvent::Break]);
    }

    #[test]
    fn decode_iac_ip() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::IP][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetEvent::InterruptProcess]);
    }

    #[test]
    fn decode_iac_ao() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::AO][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetEvent::AbortOutput]);
    }

    #[test]
    fn decode_iac_ayt() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::AYT][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetEvent::AreYouThere]);
    }

    #[test]
    fn decode_iac_ec() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::EC][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetEvent::EraseCharacter]);
    }

    #[test]
    fn decode_iac_el() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::EL][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetEvent::EraseLine]);
    }

    #[test]
    fn decode_iac_ga() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::GA][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetEvent::GoAhead]);
    }

    #[test]
    fn decode_unknown_iac_command_yields_noop() {
        let mut codec = TelnetCodec::new();
        // 0x00 is not a valid Telnet command
        let src = BytesMut::from(&[consts::IAC, 0x00][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![TelnetEvent::NoOperation]);
    }

    // ============================================================================
    // Decoding Tests - Negotiation Commands
    // ============================================================================

    #[test]
    fn decode_do_binary() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::BINARY][..]);
        let frames = collect_all(&mut codec, src);
        // Receiving DO Binary -> QState accepts and emits OptionStatus for local option enabled
        assert_eq!(
            frames,
            vec![TelnetEvent::OptionStatus(
                TelnetOption::TransmitBinary,
                TelnetSide::Local,
                true
            )]
        );
    }

    #[test]
    fn decode_dont_echo() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::DONT, consts::option::ECHO][..]);
        let frames = collect_all(&mut codec, src);
        // Receiving DONT Echo when already disabled -> no event emitted
        assert_eq!(frames, vec![]);
    }

    #[test]
    fn decode_will_sga() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::WILL, consts::option::SGA][..]);
        let frames = collect_all(&mut codec, src);
        // Receiving WILL SGA -> QState accepts and emits OptionStatus for remote option enabled
        assert_eq!(
            frames,
            vec![TelnetEvent::OptionStatus(
                TelnetOption::SuppressGoAhead,
                TelnetSide::Remote,
                true
            )]
        );
    }

    #[test]
    fn decode_wont_binary() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::WONT, consts::option::BINARY][..]);
        let frames = collect_all(&mut codec, src);
        // Receiving WONT Binary when already disabled -> no event emitted
        assert_eq!(frames, vec![]);
    }

    #[test]
    fn decode_unknown_option() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::EXOPL][..]);
        let frames = collect_all(&mut codec, src);
        // Receiving DO for unsupported option -> rejected, no event emitted
        assert_eq!(frames, vec![]);
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
            vec![TelnetEvent::Subnegotiate(TelnetArgument::Unknown(
                TelnetOption::TransmitBinary,
                BytesMut::new()
            ))]
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
            vec![TelnetEvent::Subnegotiate(TelnetArgument::Unknown(
                TelnetOption::TransmitBinary,
                BytesMut::from(&[0x01, 0x02, 0x03][..])
            ))]
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
            vec![TelnetEvent::Subnegotiate(TelnetArgument::Unknown(
                TelnetOption::TransmitBinary,
                BytesMut::from(&[0x01, consts::IAC, 0x03][..])
            ))]
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
        assert_eq!(frames, vec![TelnetEvent::NoOperation]);
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
                consts::option::KERMIT,
                b'B',
                b'y',
                b'e',
            ][..],
        );
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![
                TelnetEvent::Data(b'H'),
                TelnetEvent::Data(b'i'),
                TelnetEvent::NoOperation,
                TelnetEvent::Data(b'!'),
                // Receiving DO Echo -> QState responds with WONT (Kermit not supported by default)
                // No event emitted since negotiation failed
                TelnetEvent::Data(b'B'),
                TelnetEvent::Data(b'y'),
                TelnetEvent::Data(b'e'),
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
                TelnetEvent::Data(b'L'),
                TelnetEvent::Data(b'o'),
                TelnetEvent::Data(b'g'),
                TelnetEvent::Data(b'i'),
                TelnetEvent::Data(b'n'),
                TelnetEvent::Data(b':'),
                TelnetEvent::Data(consts::CR),
                TelnetEvent::Data(consts::LF),
                // Receiving DO Binary -> QState responds with WILL Binary and emits OptionStatus
                TelnetEvent::OptionStatus(TelnetOption::TransmitBinary, TelnetSide::Local, true),
                TelnetEvent::Data(b'P'),
                TelnetEvent::Data(b'a'),
                TelnetEvent::Data(b's'),
                TelnetEvent::Data(b's'),
                TelnetEvent::Data(b'w'),
                TelnetEvent::Data(b'o'),
                TelnetEvent::Data(b'r'),
                TelnetEvent::Data(b'd'),
                TelnetEvent::Data(b':'),
                TelnetEvent::Data(consts::CR),
                TelnetEvent::Data(consts::LF),
                // Receiving WILL Binary -> QState responds with DO Binary and emits OptionStatus
                TelnetEvent::OptionStatus(TelnetOption::TransmitBinary, TelnetSide::Remote, true),
                TelnetEvent::Data(b'H'),
                TelnetEvent::Data(b'e'),
                TelnetEvent::Data(b'l'),
                TelnetEvent::Data(b'l'),
                TelnetEvent::Data(b'o'),
                TelnetEvent::Data(b'!'),
                TelnetEvent::Data(consts::CR),
                TelnetEvent::Data(consts::LF),
            ]
        );
    }

    // ============================================================================
    // Round-trip Tests (Encode then Decode)
    // ============================================================================

    #[test]
    fn roundtrip_simple_data() {
        let frames = vec![
            TelnetFrame::Data(b'H'),
            TelnetFrame::Data(b'e'),
            TelnetFrame::Data(b'l'),
            TelnetFrame::Data(b'l'),
            TelnetFrame::Data(b'o'),
        ];
        let expected_events = vec![
            TelnetEvent::Data(b'H'),
            TelnetEvent::Data(b'e'),
            TelnetEvent::Data(b'l'),
            TelnetEvent::Data(b'l'),
            TelnetEvent::Data(b'o'),
        ];
        let encoded = encode_frames(frames);
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(expected_events, decoded);
    }

    #[test]
    fn roundtrip_iac_data() {
        let frames = vec![TelnetFrame::Data(consts::IAC)];
        let expected_events = vec![TelnetEvent::Data(consts::IAC)];
        let encoded = encode_frames(frames);
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(expected_events, decoded);
    }

    #[test]
    fn roundtrip_negotiation() {
        let frames = vec![
            TelnetFrame::Do(TelnetOption::Echo),
            TelnetFrame::Will(TelnetOption::SuppressGoAhead),
            TelnetFrame::Dont(TelnetOption::TransmitBinary),
            TelnetFrame::Wont(TelnetOption::Echo),
        ];
        // Negotiation frames produce OptionStatus events when they complete successfully
        let expected_events: Vec<TelnetEvent> = vec![
            TelnetEvent::OptionStatus(TelnetOption::Echo, TelnetSide::Local, true),
            TelnetEvent::OptionStatus(TelnetOption::SuppressGoAhead, TelnetSide::Remote, true),
        ];
        let encoded = encode_frames(frames);
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(expected_events, decoded);
    }

    #[test]
    fn roundtrip_subnegotiation() {
        let frames = vec![TelnetFrame::Subnegotiate(TelnetArgument::Unknown(
            TelnetOption::TransmitBinary,
            BytesMut::from(&[0x01, 0x02, 0x03][..]),
        ))];
        let expected_events = vec![TelnetEvent::Subnegotiate(TelnetArgument::Unknown(
            TelnetOption::TransmitBinary,
            BytesMut::from(&[0x01, 0x02, 0x03][..]),
        ))];
        let encoded = encode_frames(frames);
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(expected_events, decoded);
    }

    #[test]
    fn test_two_codec_negotiation() {
        // Simulate a client-server negotiation between two codecs
        let mut client = TelnetCodec::new();
        let mut server = TelnetCodec::new();

        // Step 1: Client requests to enable Echo locally (client will echo)
        let client_frame = client.enable_local(TelnetOption::Echo);
        assert_eq!(client_frame, Some(TelnetFrame::Will(TelnetOption::Echo)));

        // Step 2: Send WILL Echo to server
        let will_echo_bytes = encode_frame(TelnetFrame::Will(TelnetOption::Echo));
        let server_events = collect_all(&mut server, will_echo_bytes);

        // Server should respond with DO Echo and emit OptionStatus event
        assert_eq!(server_events.len(), 1);
        match &server_events[0] {
            TelnetEvent::OptionStatus(option, side, enabled) => {
                assert_eq!(*option, TelnetOption::Echo);
                assert_eq!(*side, TelnetSide::Remote);
                assert_eq!(*enabled, true);
            }
            _ => panic!("Expected OptionStatus event, got {:?}", server_events[0]),
        }

        // Step 3: Server should have sent DO Echo response - simulate receiving it
        // In a real implementation, we'd capture the server's response frame
        // For this test, we'll manually send DO Echo to client
        let do_echo_bytes = encode_frame(TelnetFrame::Do(TelnetOption::Echo));
        let client_events = collect_all(&mut client, do_echo_bytes);

        // Client should emit OptionStatus event when negotiation completes
        assert_eq!(client_events.len(), 1);
        match &client_events[0] {
            TelnetEvent::OptionStatus(option, side, enabled) => {
                assert_eq!(*option, TelnetOption::Echo);
                assert_eq!(*side, TelnetSide::Local);
                assert_eq!(*enabled, true);
            }
            _ => panic!("Expected OptionStatus event, got {:?}", client_events[0]),
        }

        // Step 4: Verify both sides have Echo enabled correctly
        assert!(client.is_enabled_local(TelnetOption::Echo));
        assert!(!client.is_enabled_remote(TelnetOption::Echo));
        assert!(!server.is_enabled_local(TelnetOption::Echo));
        assert!(server.is_enabled_remote(TelnetOption::Echo));

        // Step 5: Now test server requesting client to enable SuppressGoAhead
        let server_frame = server.enable_remote(TelnetOption::SuppressGoAhead);
        assert_eq!(
            server_frame,
            Some(TelnetFrame::Do(TelnetOption::SuppressGoAhead))
        );

        // Step 6: Send DO SuppressGoAhead to client
        let do_sga_bytes = encode_frame(TelnetFrame::Do(TelnetOption::SuppressGoAhead));
        let client_events = collect_all(&mut client, do_sga_bytes);

        // Client should respond with WILL SuppressGoAhead and emit OptionStatus
        assert_eq!(client_events.len(), 1);
        match &client_events[0] {
            TelnetEvent::OptionStatus(option, side, enabled) => {
                assert_eq!(*option, TelnetOption::SuppressGoAhead);
                assert_eq!(*side, TelnetSide::Local);
                assert_eq!(*enabled, true);
            }
            _ => panic!("Expected OptionStatus event, got {:?}", client_events[0]),
        }

        // Step 7: Send WILL SuppressGoAhead back to server
        let will_sga_bytes = encode_frame(TelnetFrame::Will(TelnetOption::SuppressGoAhead));
        let server_events = collect_all(&mut server, will_sga_bytes);

        // Server should emit OptionStatus event when negotiation completes
        assert_eq!(server_events.len(), 1);
        match &server_events[0] {
            TelnetEvent::OptionStatus(option, side, enabled) => {
                assert_eq!(*option, TelnetOption::SuppressGoAhead);
                assert_eq!(*side, TelnetSide::Remote);
                assert_eq!(*enabled, true);
            }
            _ => panic!("Expected OptionStatus event, got {:?}", server_events[0]),
        }

        // Step 8: Verify final state - both options enabled on both sides
        assert!(client.is_enabled_local(TelnetOption::Echo));
        assert!(client.is_enabled_local(TelnetOption::SuppressGoAhead));
        assert!(server.is_enabled_remote(TelnetOption::Echo));
        assert!(server.is_enabled_remote(TelnetOption::SuppressGoAhead));

        // Step 9: Test disabling - client disables Echo
        let client_frame = client.disable_local(TelnetOption::Echo);
        assert_eq!(client_frame, Some(TelnetFrame::Wont(TelnetOption::Echo)));

        // Step 10: Send WONT Echo to server
        let wont_echo_bytes = encode_frame(TelnetFrame::Wont(TelnetOption::Echo));
        let server_events = collect_all(&mut server, wont_echo_bytes);

        // Server should emit OptionStatus event for disable
        assert_eq!(server_events.len(), 1);
        match &server_events[0] {
            TelnetEvent::OptionStatus(option, side, enabled) => {
                assert_eq!(*option, TelnetOption::Echo);
                assert_eq!(*side, TelnetSide::Remote);
                assert_eq!(*enabled, false);
            }
            _ => panic!("Expected OptionStatus event, got {:?}", server_events[0]),
        }

        // Step 11: Send DONT Echo back to client (server would send this automatically)
        let dont_echo_bytes = encode_frame(TelnetFrame::Dont(TelnetOption::Echo));
        let client_events = collect_all(&mut client, dont_echo_bytes);

        // Client should emit OptionStatus event when disable completes
        // But only if the client was in WantNo state, which it should be after sending WONT
        if !client_events.is_empty() {
            assert_eq!(client_events.len(), 1);
            match &client_events[0] {
                TelnetEvent::OptionStatus(option, side, enabled) => {
                    assert_eq!(*option, TelnetOption::Echo);
                    assert_eq!(*side, TelnetSide::Local);
                    assert_eq!(*enabled, false);
                }
                _ => panic!("Expected OptionStatus event, got {:?}", client_events[0]),
            }
        }

        // Step 12: Verify final state - Echo disabled, SGA still enabled
        assert!(!client.is_enabled_local(TelnetOption::Echo));
        assert!(client.is_enabled_local(TelnetOption::SuppressGoAhead));
        assert!(!server.is_enabled_remote(TelnetOption::Echo));
        assert!(server.is_enabled_remote(TelnetOption::SuppressGoAhead));
    }

    #[test]
    fn roundtrip_control_commands() {
        let frames = vec![
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
        let expected_events = vec![
            TelnetEvent::NoOperation,
            TelnetEvent::DataMark,
            TelnetEvent::Break,
            TelnetEvent::InterruptProcess,
            TelnetEvent::AbortOutput,
            TelnetEvent::AreYouThere,
            TelnetEvent::EraseCharacter,
            TelnetEvent::EraseLine,
            TelnetEvent::GoAhead,
        ];
        let encoded = encode_frames(frames);
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(expected_events, decoded);
    }

    #[test]
    fn roundtrip_mixed_content() {
        let frames = vec![
            TelnetFrame::Data(b'H'),
            TelnetFrame::Data(b'i'),
            TelnetFrame::NoOperation,
            TelnetFrame::Data(b'!'),
        ];
        let expected_events = vec![
            TelnetEvent::Data(b'H'),
            TelnetEvent::Data(b'i'),
            TelnetEvent::NoOperation,
            TelnetEvent::Data(b'!'),
        ];
        let encoded = encode_frames(frames);
        let mut codec = TelnetCodec::new();
        let decoded = collect_all(&mut codec, encoded);
        assert_eq!(expected_events, decoded);
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
        assert_eq!(result, Some(TelnetEvent::NoOperation));
    }

    #[test]
    fn decode_partial_negotiation() {
        let mut codec = TelnetCodec::new();
        let mut src = BytesMut::from(&[consts::IAC, consts::DO][..]);

        // Incomplete negotiation
        let result = codec.decode(&mut src).expect("decode ok");
        assert!(result.is_none());

        // Complete it - DO Echo will trigger negotiation and emit OptionStatus
        src.put_u8(consts::option::ECHO);
        let result = codec.decode(&mut src).expect("decode ok");
        // Since Echo is supported locally, we accept and emit OptionStatus
        assert_eq!(
            result,
            Some(TelnetEvent::OptionStatus(
                TelnetOption::Echo,
                TelnetSide::Local,
                true
            ))
        );
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
            Some(TelnetEvent::Subnegotiate(TelnetArgument::Unknown(
                TelnetOption::TransmitBinary,
                BytesMut::from(&[0x01][..])
            )))
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
                TelnetEvent::Data(consts::IAC),
                TelnetEvent::Data(consts::IAC),
                TelnetEvent::Data(consts::IAC),
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
            vec![TelnetEvent::Data(0x80), TelnetEvent::NoOperation,]
        );
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

    // ============================================================================
    // QState Negotiation Tests
    // ============================================================================

    #[test]
    fn qstate_recv_do_supported_responds_will() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::BINARY][..]);
        let frames = collect_all(&mut codec, src);
        // When we receive DO for a supported option, we emit OptionStatus for local option enabled
        assert_eq!(
            frames,
            vec![TelnetEvent::OptionStatus(
                TelnetOption::TransmitBinary,
                TelnetSide::Local,
                true
            )]
        );
        // And the option should now be enabled locally
        assert!(codec.is_enabled_local(TelnetOption::TransmitBinary));
    }

    #[test]
    fn qstate_recv_dont_when_disabled_no_response() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::DONT, consts::option::ECHO][..]);
        let frames = collect_all(&mut codec, src);
        // When we receive DONT for an already disabled option, no event
        assert_eq!(frames, vec![]);
        assert!(!codec.is_enabled_local(TelnetOption::Echo));
    }

    #[test]
    fn qstate_recv_will_supported_responds_do() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::WILL, consts::option::SGA][..]);
        let frames = collect_all(&mut codec, src);
        // When remote sends WILL for a supported option, we emit OptionStatus for remote option enabled
        assert_eq!(
            frames,
            vec![TelnetEvent::OptionStatus(
                TelnetOption::SuppressGoAhead,
                TelnetSide::Remote,
                true
            )]
        );
        // And the option should now be enabled remotely
        assert!(codec.is_enabled_remote(TelnetOption::SuppressGoAhead));
    }

    #[test]
    fn qstate_recv_wont_when_disabled_no_response() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::WONT, consts::option::BINARY][..]);
        let frames = collect_all(&mut codec, src);
        // When remote sends WONT for an already disabled option, no event
        assert_eq!(frames, vec![]);
        assert!(!codec.is_enabled_remote(TelnetOption::TransmitBinary));
    }

    #[test]
    fn qstate_recv_do_unsupported_responds_wont() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::EXOPL][..]);
        let frames = collect_all(&mut codec, src);
        // When we receive DO for an unsupported option, no event (rejected)
        assert_eq!(frames, vec![]);
        assert!(!codec.is_enabled_local(TelnetOption::EXOPL));
    }

    #[test]
    fn qstate_recv_will_unsupported_responds_dont() {
        let mut codec = TelnetCodec::new();
        let src = BytesMut::from(&[consts::IAC, consts::WILL, consts::option::EXOPL][..]);
        let frames = collect_all(&mut codec, src);
        // When remote sends WILL for an unsupported option, no event (rejected)
        assert_eq!(frames, vec![]);
        assert!(!codec.is_enabled_remote(TelnetOption::EXOPL));
    }

    #[test]
    fn qstate_enable_local_sends_will() {
        let mut codec = TelnetCodec::new();
        let frame = codec.enable_local(TelnetOption::Echo);
        assert_eq!(frame, Some(TelnetFrame::Will(TelnetOption::Echo)));
        // Option is not yet enabled (waiting for DO response)
        assert!(!codec.is_enabled_local(TelnetOption::Echo));
    }

    #[test]
    fn qstate_enable_local_then_recv_do_completes() {
        let mut codec = TelnetCodec::new();
        // Request to enable Echo locally
        let frame = codec.enable_local(TelnetOption::Echo);
        assert_eq!(frame, Some(TelnetFrame::Will(TelnetOption::Echo)));

        // Receive DO Echo from remote
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::ECHO][..]);
        let frames = collect_all(&mut codec, src);
        // Should emit OptionStatus event when negotiation completes
        assert_eq!(
            frames,
            vec![TelnetEvent::OptionStatus(
                TelnetOption::Echo,
                TelnetSide::Local,
                true
            )]
        );
        // Now the option is enabled
        assert!(codec.is_enabled_local(TelnetOption::Echo));
    }

    #[test]
    fn qstate_enable_remote_sends_do() {
        let mut codec = TelnetCodec::new();
        let frame = codec.enable_remote(TelnetOption::SuppressGoAhead);
        assert_eq!(frame, Some(TelnetFrame::Do(TelnetOption::SuppressGoAhead)));
        // Option is not yet enabled (waiting for WILL response)
        assert!(!codec.is_enabled_remote(TelnetOption::SuppressGoAhead));
    }

    #[test]
    fn qstate_enable_remote_then_recv_will_completes() {
        let mut codec = TelnetCodec::new();
        // Request remote to enable SGA
        let frame = codec.enable_remote(TelnetOption::SuppressGoAhead);
        assert_eq!(frame, Some(TelnetFrame::Do(TelnetOption::SuppressGoAhead)));

        // Receive WILL SGA from remote
        let src = BytesMut::from(&[consts::IAC, consts::WILL, consts::option::SGA][..]);
        let frames = collect_all(&mut codec, src);
        // Should emit OptionStatus event when negotiation completes
        assert_eq!(
            frames,
            vec![TelnetEvent::OptionStatus(
                TelnetOption::SuppressGoAhead,
                TelnetSide::Remote,
                true
            )]
        );
        // Now the option is enabled
        assert!(codec.is_enabled_remote(TelnetOption::SuppressGoAhead));
    }

    #[test]
    fn qstate_disable_local_sends_wont() {
        let mut codec = TelnetCodec::new();
        // First enable the option
        codec.enable_local(TelnetOption::Echo);
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::ECHO][..]);
        collect_all(&mut codec, src);
        assert!(codec.is_enabled_local(TelnetOption::Echo));

        // Now disable it
        let frame = codec.disable_local(TelnetOption::Echo);
        assert_eq!(frame, Some(TelnetFrame::Wont(TelnetOption::Echo)));
    }

    #[test]
    fn qstate_disable_remote_sends_dont() {
        let mut codec = TelnetCodec::new();
        // First enable the option
        codec.enable_remote(TelnetOption::SuppressGoAhead);
        let src = BytesMut::from(&[consts::IAC, consts::WILL, consts::option::SGA][..]);
        collect_all(&mut codec, src);
        assert!(codec.is_enabled_remote(TelnetOption::SuppressGoAhead));

        // Now disable it
        let frame = codec.disable_remote(TelnetOption::SuppressGoAhead);
        assert_eq!(
            frame,
            Some(TelnetFrame::Dont(TelnetOption::SuppressGoAhead))
        );
    }

    #[test]
    fn qstate_full_local_enable_disable_cycle() {
        let mut codec = TelnetCodec::new();

        // 1. Request to enable
        let frame = codec.enable_local(TelnetOption::TransmitBinary);
        assert_eq!(frame, Some(TelnetFrame::Will(TelnetOption::TransmitBinary)));

        // 2. Receive DO (accept)
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::BINARY][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![TelnetEvent::OptionStatus(
                TelnetOption::TransmitBinary,
                TelnetSide::Local,
                true
            )]
        );
        assert!(codec.is_enabled_local(TelnetOption::TransmitBinary));

        // 3. Request to disable
        let frame = codec.disable_local(TelnetOption::TransmitBinary);
        assert_eq!(frame, Some(TelnetFrame::Wont(TelnetOption::TransmitBinary)));

        // 4. Receive DONT (accept)
        let src = BytesMut::from(&[consts::IAC, consts::DONT, consts::option::BINARY][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![]);
        assert!(!codec.is_enabled_local(TelnetOption::TransmitBinary));
    }

    #[test]
    fn qstate_full_remote_enable_disable_cycle() {
        let mut codec = TelnetCodec::new();

        // 1. Request remote to enable
        let frame = codec.enable_remote(TelnetOption::Echo);
        assert_eq!(frame, Some(TelnetFrame::Do(TelnetOption::Echo)));

        // 2. Receive WILL (accept)
        let src = BytesMut::from(&[consts::IAC, consts::WILL, consts::option::ECHO][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![TelnetEvent::OptionStatus(
                TelnetOption::Echo,
                TelnetSide::Remote,
                true
            )]
        );
        assert!(codec.is_enabled_remote(TelnetOption::Echo));

        // 3. Request remote to disable
        let frame = codec.disable_remote(TelnetOption::Echo);
        assert_eq!(frame, Some(TelnetFrame::Dont(TelnetOption::Echo)));

        // 4. Receive WONT (accept)
        let src = BytesMut::from(&[consts::IAC, consts::WONT, consts::option::ECHO][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![]);
        assert!(!codec.is_enabled_remote(TelnetOption::Echo));
    }

    #[test]
    fn qstate_idempotent_enable_local() {
        let mut codec = TelnetCodec::new();

        // Enable once
        let frame1 = codec.enable_local(TelnetOption::Echo);
        assert_eq!(frame1, Some(TelnetFrame::Will(TelnetOption::Echo)));

        // Try to enable again - should return None (already in progress)
        let frame2 = codec.enable_local(TelnetOption::Echo);
        assert_eq!(frame2, None);
    }

    #[test]
    fn qstate_idempotent_enable_remote() {
        let mut codec = TelnetCodec::new();

        // Enable once
        let frame1 = codec.enable_remote(TelnetOption::SuppressGoAhead);
        assert_eq!(frame1, Some(TelnetFrame::Do(TelnetOption::SuppressGoAhead)));

        // Try to enable again - should return None (already in progress)
        let frame2 = codec.enable_remote(TelnetOption::SuppressGoAhead);
        assert_eq!(frame2, None);
    }

    #[test]
    fn qstate_recv_do_when_already_enabled_no_response() {
        let mut codec = TelnetCodec::new();

        // Enable and complete negotiation
        codec.enable_local(TelnetOption::Echo);
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::ECHO][..]);
        collect_all(&mut codec, src);
        assert!(codec.is_enabled_local(TelnetOption::Echo));

        // Receive DO again - should not respond
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::ECHO][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![]);
    }

    #[test]
    fn qstate_recv_will_when_already_enabled_no_response() {
        let mut codec = TelnetCodec::new();

        // Enable and complete negotiation
        codec.enable_remote(TelnetOption::SuppressGoAhead);
        let src = BytesMut::from(&[consts::IAC, consts::WILL, consts::option::SGA][..]);
        collect_all(&mut codec, src);
        assert!(codec.is_enabled_remote(TelnetOption::SuppressGoAhead));

        // Receive WILL again - should not respond
        let src = BytesMut::from(&[consts::IAC, consts::WILL, consts::option::SGA][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(frames, vec![]);
    }

    #[test]
    fn qstate_multiple_options_independent() {
        let mut codec = TelnetCodec::new();

        // Enable multiple options
        let frame1 = codec.enable_local(TelnetOption::Echo);
        let frame2 = codec.enable_local(TelnetOption::TransmitBinary);
        let frame3 = codec.enable_remote(TelnetOption::SuppressGoAhead);

        assert_eq!(frame1, Some(TelnetFrame::Will(TelnetOption::Echo)));
        assert_eq!(
            frame2,
            Some(TelnetFrame::Will(TelnetOption::TransmitBinary))
        );
        assert_eq!(frame3, Some(TelnetFrame::Do(TelnetOption::SuppressGoAhead)));

        // Complete Echo negotiation
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::ECHO][..]);
        collect_all(&mut codec, src);
        assert!(codec.is_enabled_local(TelnetOption::Echo));

        // Binary and SGA should still be in negotiation
        assert!(!codec.is_enabled_local(TelnetOption::TransmitBinary));
        assert!(!codec.is_enabled_remote(TelnetOption::SuppressGoAhead));
    }

    #[test]
    fn qstate_recv_dont_disables_enabled_option() {
        let mut codec = TelnetCodec::new();

        // Enable option
        codec.enable_local(TelnetOption::Echo);
        let src = BytesMut::from(&[consts::IAC, consts::DO, consts::option::ECHO][..]);
        collect_all(&mut codec, src);
        assert!(codec.is_enabled_local(TelnetOption::Echo));

        // Remote sends DONT to disable
        let src = BytesMut::from(&[consts::IAC, consts::DONT, consts::option::ECHO][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![TelnetEvent::OptionStatus(
                TelnetOption::Echo,
                TelnetSide::Local,
                false
            )]
        );
        assert!(!codec.is_enabled_local(TelnetOption::Echo));
    }

    #[test]
    fn qstate_recv_wont_disables_enabled_option() {
        let mut codec = TelnetCodec::new();

        // Enable option
        codec.enable_remote(TelnetOption::SuppressGoAhead);
        let src = BytesMut::from(&[consts::IAC, consts::WILL, consts::option::SGA][..]);
        collect_all(&mut codec, src);
        assert!(codec.is_enabled_remote(TelnetOption::SuppressGoAhead));

        // Remote sends WONT to disable
        let src = BytesMut::from(&[consts::IAC, consts::WONT, consts::option::SGA][..]);
        let frames = collect_all(&mut codec, src);
        assert_eq!(
            frames,
            vec![TelnetEvent::OptionStatus(
                TelnetOption::SuppressGoAhead,
                TelnetSide::Remote,
                false
            )]
        );
        assert!(!codec.is_enabled_remote(TelnetOption::SuppressGoAhead));
    }
}
