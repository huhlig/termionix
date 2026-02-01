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

use crate::ansi::{
    AnsiApplicationProgramCommand, AnsiControlCode, AnsiControlSequenceIntroducer,
    AnsiDeviceControlString, AnsiOperatingSystemCommand, AnsiPrivacyMessage,
    AnsiSelectGraphicRendition, AnsiSequence, AnsiStartOfString, TelnetCommand,
};
use crate::{AnsiConfig, AnsiError, AnsiParser, AnsiResult};
use termionix_telnetcodec::TelnetEvent;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};
use tracing::instrument;

/// ANSI codec for encoding and decoding ANSI sequences over a Telnet connection.
///
/// This codec wraps a `TelnetCodec` and uses an `AnsiMapper` to parse ANSI escape
/// sequences from the byte stream. It implements both `Decoder` and `Encoder` traits
/// from tokio_util for use with tokio's framed I/O.
pub struct AnsiCodec<I> {
    config: AnsiConfig,
    parser: AnsiParser,
    inner: I,
}

impl<I> AnsiCodec<I> {
    /// Creates a new ANSI codec with the given configuration.
    pub fn new(config: AnsiConfig, codec: I) -> Self {
        Self {
            config,
            inner: codec,
            parser: AnsiParser::new(),
        }
    }

    /// Get a reference to the inner codec
    pub fn inner(&self) -> &I {
        &self.inner
    }

    /// Get a mutable reference to the inner codec
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }
}

impl<I> Decoder for AnsiCodec<I>
where
    I: Decoder<Item = TelnetEvent>,
    AnsiError: From<I::Error>,
{
    type Item = AnsiSequence;
    type Error = AnsiError;

    #[instrument(skip_all)]
    fn decode(&mut self, src: &mut BytesMut) -> AnsiResult<Option<Self::Item>> {
        if let Some(event) = self.inner.decode(src)? {
            match event {
                TelnetEvent::Data(byte) => {
                    // Process the byte through the ANSI mapper
                    if let Some(sequence) = self.parser.next(byte)? {
                        return Ok(Some(sequence));
                    }
                    // If we got no complete sequence, continue decoding
                    self.decode(src)
                }
                TelnetEvent::NoOperation => Ok(Some(AnsiSequence::TelnetCommand(
                    TelnetCommand::NoOperation,
                ))),
                TelnetEvent::DataMark => {
                    Ok(Some(AnsiSequence::TelnetCommand(TelnetCommand::DataMark)))
                }
                TelnetEvent::Break => Ok(Some(AnsiSequence::TelnetCommand(TelnetCommand::Break))),
                TelnetEvent::InterruptProcess => Ok(Some(AnsiSequence::TelnetCommand(
                    TelnetCommand::InterruptProcess,
                ))),
                TelnetEvent::AbortOutput => Ok(Some(AnsiSequence::TelnetCommand(
                    TelnetCommand::AbortOutput,
                ))),
                TelnetEvent::AreYouThere => Ok(Some(AnsiSequence::TelnetCommand(
                    TelnetCommand::AreYouThere,
                ))),
                TelnetEvent::EraseCharacter => Ok(Some(AnsiSequence::TelnetCommand(
                    TelnetCommand::EraseCharacter,
                ))),
                TelnetEvent::EraseLine => {
                    Ok(Some(AnsiSequence::TelnetCommand(TelnetCommand::EraseLine)))
                }
                TelnetEvent::GoAhead => {
                    Ok(Some(AnsiSequence::TelnetCommand(TelnetCommand::GoAhead)))
                }
                TelnetEvent::EndOfRecord => Ok(Some(AnsiSequence::TelnetCommand(
                    TelnetCommand::EndOfRecord,
                ))),
                TelnetEvent::OptionStatus(option, side, enabled) => Ok(Some(
                    AnsiSequence::TelnetCommand(TelnetCommand::OptionStatus(option, side, enabled)),
                )),
                TelnetEvent::Subnegotiate(arg) => Ok(Some(AnsiSequence::TelnetCommand(
                    TelnetCommand::Subnegotiation(arg),
                ))),
            }
        } else {
            Ok(None)
        }
    }
}

impl<I> Encoder<char> for AnsiCodec<I>
where
    I: Encoder<char>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(&mut self, item: char, dst: &mut BytesMut) -> AnsiResult<()> {
        // Encode plain text as telnet data
        self.inner.encode(item, dst)?;
        Ok(())
    }
}

impl<I> Encoder<&str> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(&mut self, item: &str, dst: &mut BytesMut) -> AnsiResult<()> {
        for byte in item.as_bytes() {
            self.inner.encode(*byte, dst)?;
        }
        Ok(())
    }
}

impl<I> Encoder<&[u8]> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(&mut self, item: &[u8], dst: &mut BytesMut) -> AnsiResult<()> {
        // Encode plain text as telnet data
        for byte in item {
            self.inner.encode(*byte, dst)?;
        }
        Ok(())
    }
}

impl<I> Encoder<AnsiControlCode> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(&mut self, item: AnsiControlCode, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Encode control code as a single byte
        self.inner.encode(item.to_byte(), dst)?;
        Ok(())
    }
}

impl<I> Encoder<AnsiControlSequenceIntroducer> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(
        &mut self,
        item: AnsiControlSequenceIntroducer,
        dst: &mut BytesMut,
    ) -> AnsiResult<()> {
        let mut buf = BytesMut::new();
        item.encode(&mut buf)?;
        for byte in buf.iter() {
            self.inner.encode(*byte, dst)?;
        }
        Ok(())
    }
}

impl<I> Encoder<AnsiSelectGraphicRendition> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(&mut self, item: AnsiSelectGraphicRendition, dst: &mut BytesMut) -> AnsiResult<()> {
        let mut buf = BytesMut::new();
        item.encode(&mut buf, Some(self.config.color_mode))?;
        for byte in buf.iter() {
            self.inner.encode(*byte, dst)?;
        }
        Ok(())
    }
}

impl<I> Encoder<AnsiOperatingSystemCommand> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(&mut self, item: AnsiOperatingSystemCommand, dst: &mut BytesMut) -> AnsiResult<()> {
        let mut buf = BytesMut::new();
        item.encode(&mut buf)?;
        for byte in buf.iter() {
            self.inner.encode(*byte, dst)?;
        }
        Ok(())
    }
}

impl<I> Encoder<AnsiDeviceControlString> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(&mut self, item: AnsiDeviceControlString, dst: &mut BytesMut) -> AnsiResult<()> {
        let mut buf = BytesMut::new();
        item.encode(&mut buf)?;
        for byte in buf.iter() {
            self.inner.encode(*byte, dst)?;
        }
        Ok(())
    }
}

impl<I> Encoder<AnsiStartOfString> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(&mut self, item: AnsiStartOfString, dst: &mut BytesMut) -> AnsiResult<()> {
        let mut buf = BytesMut::new();
        item.encode(&mut buf)?;
        for byte in buf.iter() {
            self.inner.encode(*byte, dst)?;
        }
        Ok(())
    }
}

impl<I> Encoder<AnsiPrivacyMessage> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(&mut self, item: AnsiPrivacyMessage, dst: &mut BytesMut) -> AnsiResult<()> {
        let mut buf = BytesMut::new();
        item.encode(&mut buf)?;
        for byte in buf.iter() {
            self.inner.encode(*byte, dst)?;
        }
        Ok(())
    }
}

impl<I> Encoder<AnsiApplicationProgramCommand> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(
        &mut self,
        item: AnsiApplicationProgramCommand,
        dst: &mut BytesMut,
    ) -> AnsiResult<()> {
        let mut buf = BytesMut::new();
        item.encode(&mut buf)?;
        for byte in buf.iter() {
            self.inner.encode(*byte, dst)?;
        }
        Ok(())
    }
}

impl<I> Encoder<TelnetCommand> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(&mut self, item: TelnetCommand, dst: &mut BytesMut) -> AnsiResult<()> {
        let mut buf = BytesMut::new();
        item.encode(&mut buf)?;
        for byte in buf.iter() {
            self.inner.encode(*byte, dst)?;
        }
        Ok(())
    }
}

impl<I> Encoder<AnsiSequence> for AnsiCodec<I>
where
    I: Encoder<u8>,
    AnsiError: From<I::Error>,
{
    type Error = AnsiError;

    fn encode(&mut self, item: AnsiSequence, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            AnsiSequence::Character(ch) => {
                let mut buf = [0u8; 4];
                let s = ch.encode_utf8(&mut buf);
                for byte in s.as_bytes() {
                    self.inner.encode(*byte, dst)?;
                }
            }
            AnsiSequence::Unicode(ch) => {
                let mut buf = [0u8; 4];
                let s = ch.encode_utf8(&mut buf);
                for byte in s.as_bytes() {
                    self.inner.encode(*byte, dst)?;
                }
            }
            AnsiSequence::Control(code) => {
                self.inner.encode(code.to_byte(), dst)?;
            }
            AnsiSequence::AnsiEscape => {
                self.inner.encode(0x1B, dst)?;
            }
            AnsiSequence::AnsiCSI(csi) => {
                let mut buf = BytesMut::new();
                csi.encode(&mut buf)?;
                for byte in buf.iter() {
                    self.inner.encode(*byte, dst)?;
                }
            }
            AnsiSequence::AnsiSGR(sgr) => {
                let mut buf = BytesMut::new();
                sgr.encode(&mut buf, Some(self.config.color_mode))?;
                for byte in buf.iter() {
                    self.inner.encode(*byte, dst)?;
                }
            }
            AnsiSequence::AnsiOSC(osc) => {
                let mut buf = BytesMut::new();
                osc.encode(&mut buf)?;
                for byte in buf.iter() {
                    self.inner.encode(*byte, dst)?;
                }
            }
            AnsiSequence::AnsiDCS(dcs) => {
                let mut buf = BytesMut::new();
                dcs.encode(&mut buf)?;
                for byte in buf.iter() {
                    self.inner.encode(*byte, dst)?;
                }
            }
            AnsiSequence::AnsiSOS(sos) => {
                let mut buf = BytesMut::new();
                sos.encode(&mut buf)?;
                for byte in buf.iter() {
                    self.inner.encode(*byte, dst)?;
                }
            }
            AnsiSequence::AnsiST => {
                self.inner.encode(0x1B, dst)?;
                self.inner.encode(b'\\', dst)?;
            }
            AnsiSequence::AnsiPM(pm) => {
                let mut buf = BytesMut::new();
                pm.encode(&mut buf)?;
                for byte in buf.iter() {
                    self.inner.encode(*byte, dst)?;
                }
            }
            AnsiSequence::AnsiAPC(apc) => {
                let mut buf = BytesMut::new();
                apc.encode(&mut buf)?;
                for byte in buf.iter() {
                    self.inner.encode(*byte, dst)?;
                }
            }
            AnsiSequence::TelnetCommand(cmd) => {
                let mut buf = BytesMut::new();
                cmd.encode(&mut buf)?;
                for byte in buf.iter() {
                    self.inner.encode(*byte, dst)?;
                }
            }
        }
        Ok(())
    }
}
