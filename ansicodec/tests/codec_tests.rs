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

//! Comprehensive tests for AnsiCodec

use termionix_ansicodec::ansi::{
    AnsiControlCode, AnsiControlSequenceIntroducer, AnsiDeviceControlString,
    AnsiOperatingSystemCommand, AnsiSelectGraphicRendition, AnsiSequence,
};
use termionix_ansicodec::{AnsiCodec, AnsiConfig, ColorMode};
use termionix_telnetcodec::TelnetCodec;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

fn create_codec() -> AnsiCodec<TelnetCodec> {
    let telnet_codec = TelnetCodec::new();
    AnsiCodec::new(AnsiConfig::default(), telnet_codec)
}

#[test]
fn test_encode_plain_text() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    codec.encode("Hello, World!", &mut buffer).unwrap();
    assert_eq!(&buffer[..], b"Hello, World!");
}

#[test]
fn test_encode_char() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    codec.encode('A', &mut buffer).unwrap();
    assert_eq!(&buffer[..], b"A");
}

#[test]
fn test_encode_bytes() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let data: &[u8] = b"Test data";
    codec.encode(data, &mut buffer).unwrap();
    assert_eq!(&buffer[..], b"Test data");
}

#[test]
fn test_encode_control_code_bel() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    codec.encode(AnsiControlCode::BEL, &mut buffer).unwrap();
    assert_eq!(&buffer[..], b"\x07");
}

#[test]
fn test_encode_multiple_control_codes() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    codec.encode(AnsiControlCode::BEL, &mut buffer).unwrap();
    codec.encode(AnsiControlCode::BS, &mut buffer).unwrap();
    codec.encode(AnsiControlCode::HT, &mut buffer).unwrap();

    assert_eq!(&buffer[..], b"\x07\x08\x09");
}

#[test]
fn test_encode_csi_cursor_up() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let csi = AnsiControlSequenceIntroducer::CursorUp(5);
    codec.encode(csi, &mut buffer).unwrap();

    assert!(buffer.starts_with(b"\x1B["));
    assert!(buffer.ends_with(b"A"));
}

#[test]
fn test_encode_csi_cursor_position() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let csi = AnsiControlSequenceIntroducer::CursorPosition { row: 10, col: 20 };
    codec.encode(csi, &mut buffer).unwrap();

    assert!(buffer.starts_with(b"\x1B["));
    assert!(buffer.ends_with(b"H"));
}

#[test]
fn test_encode_sgr_default() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let sgr = AnsiSelectGraphicRendition::default();
    codec.encode(sgr, &mut buffer).unwrap();

    // Default SGR (with all fields None) should produce no output
    assert!(buffer.is_empty());
}

#[test]
fn test_encode_osc_unknown() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let osc = AnsiOperatingSystemCommand::Unknown(vec![b'0', b';', b'T', b'e', b's', b't']);
    codec.encode(osc, &mut buffer).unwrap();

    assert!(buffer.starts_with(b"\x1B]"));
}

#[test]
fn test_encode_sequence_character() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let seq = AnsiSequence::Character('X');
    codec.encode(seq, &mut buffer).unwrap();

    assert_eq!(&buffer[..], b"X");
}

#[test]
fn test_encode_sequence_unicode() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let seq = AnsiSequence::Unicode('€');
    codec.encode(seq, &mut buffer).unwrap();

    assert_eq!(&buffer[..], "€".as_bytes());
}

#[test]
fn test_encode_sequence_control() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let seq = AnsiSequence::Control(AnsiControlCode::LF);
    codec.encode(seq, &mut buffer).unwrap();

    assert_eq!(&buffer[..], b"\n");
}

#[test]
fn test_encode_sequence_escape() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let seq = AnsiSequence::AnsiEscape;
    codec.encode(seq, &mut buffer).unwrap();

    assert_eq!(&buffer[..], b"\x1B");
}

#[test]
fn test_decode_simple_text() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::from("Hello");

    let mut results = Vec::new();
    while let Some(seq) = codec.decode(&mut buffer).unwrap() {
        results.push(seq);
    }

    assert_eq!(results.len(), 5);
    assert!(matches!(results[0], AnsiSequence::Character('H')));
    assert!(matches!(results[4], AnsiSequence::Character('o')));
}

#[test]
fn test_decode_with_control_codes() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::from("Hi\n");

    let mut results = Vec::new();
    while let Some(seq) = codec.decode(&mut buffer).unwrap() {
        results.push(seq);
    }

    assert_eq!(results.len(), 3);
    assert!(matches!(
        results[2],
        AnsiSequence::Control(AnsiControlCode::LF)
    ));
}

#[test]
fn test_roundtrip_text() {
    let mut codec = create_codec();
    let original = "Test message";

    // Encode
    let mut encode_buffer = BytesMut::new();
    codec.encode(original, &mut encode_buffer).unwrap();

    // Decode
    let mut decode_buffer = encode_buffer.clone();
    let mut results = Vec::new();
    while let Some(seq) = codec.decode(&mut decode_buffer).unwrap() {
        results.push(seq);
    }

    assert_eq!(results.len(), original.len());
}

#[test]
fn test_color_mode_configuration() {
    let config = AnsiConfig {
        color_mode: ColorMode::TrueColor,
        ..Default::default()
    };
    let telnet_codec = TelnetCodec::new();
    let _codec = AnsiCodec::new(config, telnet_codec);

    // Codec should be created with TrueColor mode
}

#[test]
fn test_encode_empty_string() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    codec.encode("", &mut buffer).unwrap();
    assert!(buffer.is_empty());
}

#[test]
fn test_encode_empty_bytes() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let data: &[u8] = b"";
    codec.encode(data, &mut buffer).unwrap();
    assert!(buffer.is_empty());
}

#[test]
fn test_decode_empty_buffer() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let result = codec.decode(&mut buffer).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_encode_unicode_characters() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    codec.encode("Hello 世界 🌍", &mut buffer).unwrap();
    assert!(buffer.len() > 0);

    let decoded = String::from_utf8_lossy(&buffer);
    assert!(decoded.contains("世界"));
    assert!(decoded.contains("🌍"));
}

#[test]
fn test_encode_mixed_content() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    codec.encode("Text", &mut buffer).unwrap();
    codec.encode(AnsiControlCode::BEL, &mut buffer).unwrap();
    codec.encode("More", &mut buffer).unwrap();

    assert!(buffer.len() > 0);
    assert!(buffer.contains(&0x07)); // Bell character
}

#[test]
fn test_encode_long_text() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let long_text = "A".repeat(10000);
    codec.encode(long_text.as_str(), &mut buffer).unwrap();

    assert_eq!(buffer.len(), 10000);
}

#[test]
fn test_encode_special_characters() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    codec.encode("\r\n\t", &mut buffer).unwrap();
    assert_eq!(&buffer[..], b"\r\n\t");
}

#[test]
fn test_multiple_encodes_same_buffer() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    for i in 0..10 {
        let line = format!("Line {}\n", i);
        codec.encode(line.as_str(), &mut buffer).unwrap();
    }

    assert!(buffer.len() > 0);
    let text = String::from_utf8_lossy(&buffer);
    assert!(text.contains("Line 0"));
    assert!(text.contains("Line 9"));
}

#[test]
fn test_encode_all_c0_control_codes() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let codes = vec![
        AnsiControlCode::NUL,
        AnsiControlCode::SOH,
        AnsiControlCode::STX,
        AnsiControlCode::ETX,
        AnsiControlCode::EOT,
        AnsiControlCode::ENQ,
        AnsiControlCode::ACK,
        AnsiControlCode::BEL,
        AnsiControlCode::BS,
        AnsiControlCode::HT,
        AnsiControlCode::LF,
        AnsiControlCode::VT,
        AnsiControlCode::FF,
        AnsiControlCode::CR,
    ];

    for code in codes {
        codec.encode(code, &mut buffer).unwrap();
    }

    assert_eq!(buffer.len(), 14);
}

#[test]
fn test_encode_csi_variants() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    codec
        .encode(AnsiControlSequenceIntroducer::CursorUp(1), &mut buffer)
        .unwrap();
    codec
        .encode(AnsiControlSequenceIntroducer::CursorDown(2), &mut buffer)
        .unwrap();
    codec
        .encode(AnsiControlSequenceIntroducer::CursorForward(3), &mut buffer)
        .unwrap();
    codec
        .encode(AnsiControlSequenceIntroducer::CursorBack(4), &mut buffer)
        .unwrap();

    assert!(buffer.len() > 0);
}

#[test]
fn test_encode_dcs() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    let dcs = AnsiDeviceControlString::Unknown(vec![b't', b'e', b's', b't']);
    codec.encode(dcs, &mut buffer).unwrap();

    assert!(buffer.starts_with(b"\x1BP"));
}

#[test]
fn test_decode_partial_sequence() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::from("Hel");

    let mut results = Vec::new();
    while let Some(seq) = codec.decode(&mut buffer).unwrap() {
        results.push(seq);
    }

    assert_eq!(results.len(), 3);

    // Add more data
    buffer.extend_from_slice(b"lo");
    while let Some(seq) = codec.decode(&mut buffer).unwrap() {
        results.push(seq);
    }

    assert_eq!(results.len(), 5);
}

#[test]
fn test_encode_sequence_types() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    // Test various sequence types
    codec
        .encode(AnsiSequence::Character('A'), &mut buffer)
        .unwrap();
    codec
        .encode(AnsiSequence::Unicode('€'), &mut buffer)
        .unwrap();
    codec
        .encode(AnsiSequence::Control(AnsiControlCode::LF), &mut buffer)
        .unwrap();
    codec.encode(AnsiSequence::AnsiEscape, &mut buffer).unwrap();

    assert!(buffer.len() > 0);
}

#[test]
fn test_codec_with_different_color_modes() {
    for color_mode in [
        ColorMode::None,
        ColorMode::Basic,
        ColorMode::FixedColor,
        ColorMode::TrueColor,
    ] {
        let config = AnsiConfig {
            color_mode,
            ..Default::default()
        };
        let telnet_codec = TelnetCodec::new();
        let mut codec = AnsiCodec::new(config, telnet_codec);

        let mut buffer = BytesMut::new();
        codec.encode("Test", &mut buffer).unwrap();
        assert!(!buffer.is_empty());
    }
}
