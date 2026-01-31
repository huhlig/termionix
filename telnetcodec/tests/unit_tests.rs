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

//! Unit tests for telnetcodec components

use bytes::BytesMut;
use termionix_telnetcodec::{
    TelnetArgument, TelnetCodec, TelnetEvent, TelnetFrame, TelnetOption, TelnetSide, msdp, mssp,
    naws, status,
};
use tokio_util::codec::{Decoder, Encoder};

// ============================================================================
// Helper Functions
// ============================================================================

fn encode_frame(codec: &mut TelnetCodec, frame: TelnetFrame) -> BytesMut {
    let mut buffer = BytesMut::new();
    codec.encode(frame, &mut buffer).unwrap();
    buffer
}

fn decode_all(codec: &mut TelnetCodec, buffer: &mut BytesMut) -> Vec<TelnetEvent> {
    let mut events = Vec::new();
    while let Some(event) = codec.decode(buffer).unwrap() {
        events.push(event);
    }
    events
}

// ============================================================================
// TelnetOption Tests
// ============================================================================

#[test]
fn telnet_option_from_u8() {
    assert_eq!(TelnetOption::from(0), TelnetOption::TransmitBinary);
    assert_eq!(TelnetOption::from(1), TelnetOption::Echo);
    assert_eq!(TelnetOption::from(3), TelnetOption::SuppressGoAhead);
    assert_eq!(TelnetOption::from(255), TelnetOption::Unknown(255));
}

#[test]
fn telnet_option_to_u8() {
    assert_eq!(u8::from(TelnetOption::TransmitBinary), 0);
    assert_eq!(u8::from(TelnetOption::Echo), 1);
    assert_eq!(u8::from(TelnetOption::SuppressGoAhead), 3);
    assert_eq!(u8::from(TelnetOption::Unknown(255)), 255);
}

#[test]
fn telnet_option_display() {
    assert_eq!(format!("{}", TelnetOption::Echo), "Echo");
    assert_eq!(
        format!("{}", TelnetOption::TransmitBinary),
        "TransmitBinary"
    );
    assert_eq!(format!("{}", TelnetOption::Unknown(99)), "Unknown(99)");
}

#[test]
fn telnet_option_debug() {
    assert_eq!(format!("{:?}", TelnetOption::Echo), "Echo");
    assert_eq!(format!("{:?}", TelnetOption::GMCP), "GMCP");
}

// ============================================================================
// TelnetFrame Tests
// ============================================================================

#[test]
fn telnet_frame_data() {
    let frame = TelnetFrame::Data(b'A');
    assert_eq!(frame, TelnetFrame::Data(b'A'));
}

#[test]
fn telnet_frame_commands() {
    assert_eq!(TelnetFrame::NoOperation, TelnetFrame::NoOperation);
    assert_eq!(TelnetFrame::Break, TelnetFrame::Break);
    assert_eq!(TelnetFrame::InterruptProcess, TelnetFrame::InterruptProcess);
}

#[test]
fn telnet_frame_negotiation() {
    let frame = TelnetFrame::Do(TelnetOption::Echo);
    assert_eq!(frame, TelnetFrame::Do(TelnetOption::Echo));

    let frame = TelnetFrame::Will(TelnetOption::SuppressGoAhead);
    assert_eq!(frame, TelnetFrame::Will(TelnetOption::SuppressGoAhead));
}

#[test]
fn telnet_frame_clone() {
    let frame = TelnetFrame::Do(TelnetOption::Echo);
    let cloned = frame.clone();
    assert_eq!(frame, cloned);
}

// ============================================================================
// TelnetEvent Tests
// ============================================================================

#[test]
fn telnet_event_data() {
    let event = TelnetEvent::Data(b'X');
    assert_eq!(event, TelnetEvent::Data(b'X'));
}

#[test]
fn telnet_event_option_status() {
    let event = TelnetEvent::OptionStatus(TelnetOption::Echo, TelnetSide::Local, true);
    assert_eq!(
        event,
        TelnetEvent::OptionStatus(TelnetOption::Echo, TelnetSide::Local, true)
    );
}

#[test]
fn telnet_event_clone() {
    let event = TelnetEvent::OptionStatus(TelnetOption::Echo, TelnetSide::Remote, false);
    let cloned = event.clone();
    assert_eq!(event, cloned);
}

// ============================================================================
// TelnetCodec Basic Tests
// ============================================================================

#[test]
fn codec_new() {
    let codec = TelnetCodec::new();
    assert!(!codec.is_enabled_local(TelnetOption::Echo));
    assert!(!codec.is_enabled_remote(TelnetOption::Echo));
}

#[test]
fn codec_default() {
    let codec = TelnetCodec::default();
    assert!(!codec.is_enabled_local(TelnetOption::Echo));
    assert!(!codec.is_enabled_remote(TelnetOption::Echo));
}

#[test]
fn codec_supported_options() {
    let codec = TelnetCodec::new();

    // Standard options should be supported
    assert!(codec.is_supported_local(TelnetOption::Echo));
    assert!(codec.is_supported_local(TelnetOption::TransmitBinary));
    assert!(codec.is_supported_local(TelnetOption::SuppressGoAhead));

    assert!(codec.is_supported_remote(TelnetOption::Echo));
    assert!(codec.is_supported_remote(TelnetOption::TransmitBinary));
    assert!(codec.is_supported_remote(TelnetOption::SuppressGoAhead));
}

// ============================================================================
// Encoding Tests
// ============================================================================

#[test]
fn encode_single_byte() {
    let mut codec = TelnetCodec::new();
    let buffer = encode_frame(&mut codec, TelnetFrame::Data(b'A'));
    assert_eq!(buffer.as_ref(), b"A");
}

#[test]
fn encode_multiple_bytes() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::new();

    codec.encode(TelnetFrame::Data(b'H'), &mut buffer).unwrap();
    codec.encode(TelnetFrame::Data(b'i'), &mut buffer).unwrap();

    assert_eq!(buffer.as_ref(), b"Hi");
}

#[test]
fn encode_iac_escape() {
    let mut codec = TelnetCodec::new();
    let buffer = encode_frame(&mut codec, TelnetFrame::Data(0xFF));
    assert_eq!(buffer.as_ref(), &[0xFF, 0xFF]);
}

#[test]
fn encode_string() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::new();
    codec.encode("Hello", &mut buffer).unwrap();
    assert_eq!(buffer.as_ref(), b"Hello");
}

#[test]
fn encode_char() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::new();
    codec.encode('A', &mut buffer).unwrap();
    assert_eq!(buffer.as_ref(), b"A");
}

#[test]
fn encode_u8() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::new();
    codec.encode(65u8, &mut buffer).unwrap();
    assert_eq!(buffer.as_ref(), b"A");
}

#[test]
fn encode_control_commands() {
    let mut codec = TelnetCodec::new();

    let nop = encode_frame(&mut codec, TelnetFrame::NoOperation);
    assert_eq!(nop.as_ref(), &[0xFF, 0xF1]);

    let brk = encode_frame(&mut codec, TelnetFrame::Break);
    assert_eq!(brk.as_ref(), &[0xFF, 0xF3]);

    let ip = encode_frame(&mut codec, TelnetFrame::InterruptProcess);
    assert_eq!(ip.as_ref(), &[0xFF, 0xF4]);
}

#[test]
fn encode_negotiation_commands() {
    let mut codec = TelnetCodec::new();

    let do_echo = encode_frame(&mut codec, TelnetFrame::Do(TelnetOption::Echo));
    assert_eq!(do_echo.as_ref(), &[0xFF, 0xFD, 0x01]);

    let will_sga = encode_frame(&mut codec, TelnetFrame::Will(TelnetOption::SuppressGoAhead));
    assert_eq!(will_sga.as_ref(), &[0xFF, 0xFB, 0x03]);

    let dont_binary = encode_frame(&mut codec, TelnetFrame::Dont(TelnetOption::TransmitBinary));
    assert_eq!(dont_binary.as_ref(), &[0xFF, 0xFE, 0x00]);

    let wont_echo = encode_frame(&mut codec, TelnetFrame::Wont(TelnetOption::Echo));
    assert_eq!(wont_echo.as_ref(), &[0xFF, 0xFC, 0x01]);
}

#[test]
fn encode_subnegotiation_empty() {
    let mut codec = TelnetCodec::new();
    let arg = TelnetArgument::Unknown(TelnetOption::Echo, BytesMut::new());
    let buffer = encode_frame(&mut codec, TelnetFrame::Subnegotiate(arg));
    assert_eq!(buffer.as_ref(), &[0xFF, 0xFA, 0x01, 0xFF, 0xF0]);
}

#[test]
fn encode_subnegotiation_with_data() {
    let mut codec = TelnetCodec::new();
    let arg = TelnetArgument::Unknown(TelnetOption::Echo, BytesMut::from(&[1, 2, 3][..]));
    let buffer = encode_frame(&mut codec, TelnetFrame::Subnegotiate(arg));
    assert_eq!(buffer.as_ref(), &[0xFF, 0xFA, 0x01, 1, 2, 3, 0xFF, 0xF0]);
}

#[test]
fn encode_subnegotiation_with_iac_escape() {
    let mut codec = TelnetCodec::new();
    let arg = TelnetArgument::Unknown(TelnetOption::Echo, BytesMut::from(&[0xFF, 0x01][..]));
    let buffer = encode_frame(&mut codec, TelnetFrame::Subnegotiate(arg));
    // IAC in data should be escaped
    assert_eq!(
        buffer.as_ref(),
        &[0xFF, 0xFA, 0x01, 0xFF, 0xFF, 0x01, 0xFF, 0xF0]
    );
}

// ============================================================================
// Decoding Tests
// ============================================================================

#[test]
fn decode_single_byte() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&b"A"[..]);
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::Data(b'A')]);
}

#[test]
fn decode_multiple_bytes() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&b"Hello"[..]);
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(
        events,
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
fn decode_empty_buffer() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::new();
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events, vec![]);
}

#[test]
fn decode_iac_iac_as_data() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&[0xFF, 0xFF][..]);
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::Data(0xFF)]);
}

#[test]
fn decode_control_commands() {
    let mut codec = TelnetCodec::new();

    let mut buffer = BytesMut::from(&[0xFF, 0xF1][..]); // NOP
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::NoOperation]);

    let mut buffer = BytesMut::from(&[0xFF, 0xF3][..]); // BRK
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::Break]);

    let mut buffer = BytesMut::from(&[0xFF, 0xF4][..]); // IP
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::InterruptProcess]);
}

#[test]
fn decode_negotiation_do() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&[0xFF, 0xFD, 0x01][..]); // DO Echo
    let events = decode_all(&mut codec, &mut buffer);
    // Should respond with WILL and emit OptionStatus
    assert_eq!(
        events,
        vec![TelnetEvent::OptionStatus(
            TelnetOption::Echo,
            TelnetSide::Local,
            true
        )]
    );
    assert!(codec.is_enabled_local(TelnetOption::Echo));
}

#[test]
fn decode_negotiation_will() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&[0xFF, 0xFB, 0x01][..]); // WILL Echo
    let events = decode_all(&mut codec, &mut buffer);
    // Should respond with DO and emit OptionStatus
    assert_eq!(
        events,
        vec![TelnetEvent::OptionStatus(
            TelnetOption::Echo,
            TelnetSide::Remote,
            true
        )]
    );
    assert!(codec.is_enabled_remote(TelnetOption::Echo));
}

#[test]
fn decode_negotiation_dont() {
    let mut codec = TelnetCodec::new();
    // First enable Echo
    codec.enable_local(TelnetOption::Echo);
    let mut buffer = BytesMut::from(&[0xFF, 0xFB, 0x01][..]); // WILL Echo (from remote)
    decode_all(&mut codec, &mut buffer);

    // Now send DONT
    let mut buffer = BytesMut::from(&[0xFF, 0xFE, 0x01][..]); // DONT Echo
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(
        events,
        vec![TelnetEvent::OptionStatus(
            TelnetOption::Echo,
            TelnetSide::Local,
            false
        )]
    );
    assert!(!codec.is_enabled_local(TelnetOption::Echo));
}

#[test]
fn decode_negotiation_wont() {
    let mut codec = TelnetCodec::new();
    // First enable Echo on remote
    codec.enable_remote(TelnetOption::Echo);
    let mut buffer = BytesMut::from(&[0xFF, 0xFD, 0x01][..]); // DO Echo (from remote)
    decode_all(&mut codec, &mut buffer);

    // Now send WONT
    let mut buffer = BytesMut::from(&[0xFF, 0xFC, 0x01][..]); // WONT Echo
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(
        events,
        vec![TelnetEvent::OptionStatus(
            TelnetOption::Echo,
            TelnetSide::Remote,
            false
        )]
    );
    assert!(!codec.is_enabled_remote(TelnetOption::Echo));
}

#[test]
fn decode_subnegotiation_empty() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&[0xFF, 0xFA, 0x01, 0xFF, 0xF0][..]); // SB Echo SE
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events.len(), 1);
    match &events[0] {
        TelnetEvent::Subnegotiate(arg) => match arg {
            TelnetArgument::Unknown(opt, data) => {
                assert_eq!(*opt, TelnetOption::Echo);
                assert_eq!(data.len(), 0);
            }
            _ => panic!("Expected Unknown argument"),
        },
        _ => panic!("Expected Subnegotiate event"),
    }
}

#[test]
fn decode_subnegotiation_with_data() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&[0xFF, 0xFA, 0x01, 1, 2, 3, 0xFF, 0xF0][..]);
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events.len(), 1);
    match &events[0] {
        TelnetEvent::Subnegotiate(arg) => match arg {
            TelnetArgument::Unknown(opt, data) => {
                assert_eq!(*opt, TelnetOption::Echo);
                assert_eq!(data, &vec![1, 2, 3]);
            }
            _ => panic!("Expected Unknown argument"),
        },
        _ => panic!("Expected Subnegotiate event"),
    }
}

#[test]
fn decode_subnegotiation_with_escaped_iac() {
    let mut codec = TelnetCodec::new();
    // SB Echo [0xFF, 0xFF, 0x01] SE - IAC should be unescaped
    let mut buffer = BytesMut::from(&[0xFF, 0xFA, 0x01, 0xFF, 0xFF, 0x01, 0xFF, 0xF0][..]);
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events.len(), 1);
    match &events[0] {
        TelnetEvent::Subnegotiate(arg) => match arg {
            TelnetArgument::Unknown(opt, data) => {
                assert_eq!(*opt, TelnetOption::Echo);
                assert_eq!(data, &vec![0xFF, 0x01]);
            }
            _ => panic!("Expected Unknown argument"),
        },
        _ => panic!("Expected Subnegotiate event"),
    }
}

// ============================================================================
// Partial Frame Tests
// ============================================================================

#[test]
fn decode_partial_iac() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&[0xFF][..]); // Incomplete IAC
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events, vec![]); // Should wait for more data

    // Complete the command
    buffer.extend_from_slice(&[0xF1]); // NOP
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::NoOperation]);
}

#[test]
fn decode_partial_negotiation() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&[0xFF, 0xFD][..]); // Incomplete DO
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events, vec![]); // Should wait for option byte

    // Complete the negotiation
    buffer.extend_from_slice(&[0x01]); // Echo
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(
        events,
        vec![TelnetEvent::OptionStatus(
            TelnetOption::Echo,
            TelnetSide::Local,
            true
        )]
    );
}

#[test]
fn decode_partial_subnegotiation() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&[0xFF, 0xFA, 0x01, 1, 2][..]); // Incomplete SB
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events, vec![]); // Should wait for SE

    // Complete the subnegotiation
    buffer.extend_from_slice(&[3, 0xFF, 0xF0]); // Data + SE
    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(events.len(), 1);
}

// ============================================================================
// Mixed Content Tests
// ============================================================================

#[test]
fn decode_data_with_commands() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&b"Hello"[..]);
    buffer.extend_from_slice(&[0xFF, 0xF1]); // NOP
    buffer.extend_from_slice(b"World");

    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(
        events,
        vec![
            TelnetEvent::Data(b'H'),
            TelnetEvent::Data(b'e'),
            TelnetEvent::Data(b'l'),
            TelnetEvent::Data(b'l'),
            TelnetEvent::Data(b'o'),
            TelnetEvent::NoOperation,
            TelnetEvent::Data(b'W'),
            TelnetEvent::Data(b'o'),
            TelnetEvent::Data(b'r'),
            TelnetEvent::Data(b'l'),
            TelnetEvent::Data(b'd'),
        ]
    );
}

#[test]
fn decode_data_with_negotiation() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&b"Test"[..]);
    buffer.extend_from_slice(&[0xFF, 0xFD, 0x01]); // DO Echo
    buffer.extend_from_slice(b"Data");

    let events = decode_all(&mut codec, &mut buffer);
    assert_eq!(
        events,
        vec![
            TelnetEvent::Data(b'T'),
            TelnetEvent::Data(b'e'),
            TelnetEvent::Data(b's'),
            TelnetEvent::Data(b't'),
            TelnetEvent::OptionStatus(TelnetOption::Echo, TelnetSide::Local, true),
            TelnetEvent::Data(b'D'),
            TelnetEvent::Data(b'a'),
            TelnetEvent::Data(b't'),
            TelnetEvent::Data(b'a'),
        ]
    );
}

// ============================================================================
// Round-trip Tests
// ============================================================================

#[test]
fn roundtrip_data() {
    let mut encoder = TelnetCodec::new();
    let mut decoder = TelnetCodec::new();

    let mut buffer = BytesMut::new();
    encoder
        .encode(TelnetFrame::Data(b'A'), &mut buffer)
        .unwrap();
    encoder
        .encode(TelnetFrame::Data(b'B'), &mut buffer)
        .unwrap();
    encoder
        .encode(TelnetFrame::Data(b'C'), &mut buffer)
        .unwrap();

    let events = decode_all(&mut decoder, &mut buffer);
    assert_eq!(
        events,
        vec![
            TelnetEvent::Data(b'A'),
            TelnetEvent::Data(b'B'),
            TelnetEvent::Data(b'C'),
        ]
    );
}

#[test]
fn roundtrip_iac_data() {
    let mut encoder = TelnetCodec::new();
    let mut decoder = TelnetCodec::new();

    let mut buffer = BytesMut::new();
    encoder
        .encode(TelnetFrame::Data(0xFF), &mut buffer)
        .unwrap();

    let events = decode_all(&mut decoder, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::Data(0xFF)]);
}

#[test]
fn roundtrip_commands() {
    let mut encoder = TelnetCodec::new();
    let mut decoder = TelnetCodec::new();

    let mut buffer = BytesMut::new();
    encoder
        .encode(TelnetFrame::NoOperation, &mut buffer)
        .unwrap();
    encoder.encode(TelnetFrame::Break, &mut buffer).unwrap();
    encoder
        .encode(TelnetFrame::InterruptProcess, &mut buffer)
        .unwrap();

    let events = decode_all(&mut decoder, &mut buffer);
    assert_eq!(
        events,
        vec![
            TelnetEvent::NoOperation,
            TelnetEvent::Break,
            TelnetEvent::InterruptProcess,
        ]
    );
}

// ============================================================================
// Option State Management Tests
// ============================================================================

#[test]
fn enable_local_option() {
    let mut codec = TelnetCodec::new();
    let frame = codec.enable_local(TelnetOption::Echo);
    assert_eq!(frame, Some(TelnetFrame::Will(TelnetOption::Echo)));
}

#[test]
fn enable_remote_option() {
    let mut codec = TelnetCodec::new();
    let frame = codec.enable_remote(TelnetOption::Echo);
    assert_eq!(frame, Some(TelnetFrame::Do(TelnetOption::Echo)));
}

#[test]
fn disable_local_option() {
    let mut codec = TelnetCodec::new();
    codec.enable_local(TelnetOption::Echo);
    let frame = codec.disable_local(TelnetOption::Echo);
    assert_eq!(frame, Some(TelnetFrame::Wont(TelnetOption::Echo)));
}

#[test]
fn disable_remote_option() {
    let mut codec = TelnetCodec::new();
    codec.enable_remote(TelnetOption::Echo);
    let frame = codec.disable_remote(TelnetOption::Echo);
    assert_eq!(frame, Some(TelnetFrame::Dont(TelnetOption::Echo)));
}

#[test]
fn idempotent_enable_local() {
    let mut codec = TelnetCodec::new();
    let frame1 = codec.enable_local(TelnetOption::Echo);
    assert!(frame1.is_some());

    // Second enable should return None (already enabled)
    let frame2 = codec.enable_local(TelnetOption::Echo);
    assert!(frame2.is_none());
}

#[test]
fn idempotent_enable_remote() {
    let mut codec = TelnetCodec::new();
    let frame1 = codec.enable_remote(TelnetOption::Echo);
    assert!(frame1.is_some());

    // Second enable should return None (already enabled)
    let frame2 = codec.enable_remote(TelnetOption::Echo);
    assert!(frame2.is_none());
}

// ============================================================================
// TelnetArgument Tests
// ============================================================================

#[test]
fn telnet_argument_naws() {
    let arg = naws::WindowSize::new(80, 24);
    let telnet_arg = TelnetArgument::NAWSWindowSize(arg);

    match telnet_arg {
        TelnetArgument::NAWSWindowSize(naws) => {
            assert_eq!(naws.cols, 80);
            assert_eq!(naws.rows, 24);
        }
        _ => panic!("Expected NAWS argument"),
    }
}

#[test]
fn telnet_argument_unknown() {
    let data = BytesMut::from(&[1, 2, 3][..]);
    let arg = TelnetArgument::Unknown(TelnetOption::Echo, data.clone());

    match arg {
        TelnetArgument::Unknown(opt, payload) => {
            assert_eq!(opt, TelnetOption::Echo);
            assert_eq!(payload, data);
        }
        _ => panic!("Expected Unknown argument"),
    }
}

#[test]
fn telnet_argument_option() {
    let arg = TelnetArgument::NAWSWindowSize(naws::WindowSize::new(80, 24));
    assert_eq!(arg.option(), TelnetOption::NAWS);

    let arg = TelnetArgument::Unknown(TelnetOption::Echo, BytesMut::new());
    assert_eq!(arg.option(), TelnetOption::Unknown(1));
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn decode_unknown_iac_command() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&[0xFF, 0xEE][..]); // Unknown command
    let events = decode_all(&mut codec, &mut buffer);
    // Should emit NoOperation for unknown commands
    assert_eq!(events, vec![TelnetEvent::NoOperation]);
}

#[test]
fn decode_unknown_option() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::from(&[0xFF, 0xFD, 0xFF][..]); // DO Unknown(255)
    let events = decode_all(&mut codec, &mut buffer);
    // Should handle unknown options gracefully
    assert!(events.len() <= 1); // May emit OptionStatus or nothing
}

#[test]
fn encode_event_data() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::new();
    codec.encode(TelnetEvent::Data(b'X'), &mut buffer).unwrap();
    assert_eq!(buffer.as_ref(), b"X");
}

#[test]
fn encode_event_commands() {
    let mut codec = TelnetCodec::new();
    let mut buffer = BytesMut::new();
    codec.encode(TelnetEvent::NoOperation, &mut buffer).unwrap();
    assert_eq!(buffer.as_ref(), &[0xFF, 0xF1]);
}

#[test]
fn large_data_stream() {
    let mut encoder = TelnetCodec::new();
    let mut decoder = TelnetCodec::new();

    let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
    let mut buffer = BytesMut::new();

    for &byte in &data {
        encoder
            .encode(TelnetFrame::Data(byte), &mut buffer)
            .unwrap();
    }

    let events = decode_all(&mut decoder, &mut buffer);
    assert_eq!(events.len(), 1000);

    for (i, event) in events.iter().enumerate() {
        match event {
            TelnetEvent::Data(byte) => assert_eq!(*byte, data[i]),
            _ => panic!("Expected Data event"),
        }
    }
}
