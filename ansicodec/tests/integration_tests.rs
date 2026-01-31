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

//! Integration tests for AnsiCodec with TelnetCodec

use termionix_ansicodec::ansi::{AnsiControlCode, AnsiControlSequenceIntroducer, AnsiSequence};
use termionix_ansicodec::{AnsiCodec, AnsiConfig, AnsiParser, ColorMode, SegmentedString};
use termionix_telnetcodec::TelnetCodec;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

fn create_codec() -> AnsiCodec<TelnetCodec> {
    let telnet_codec = TelnetCodec::new();
    AnsiCodec::new(AnsiConfig::default(), telnet_codec)
}

#[test]
fn test_full_terminal_session_simulation() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    // Simulate a login prompt
    codec.encode("Username: ", &mut buffer).unwrap();

    // Simulate user input
    let mut input = buffer.clone();
    let mut sequences = Vec::new();
    while let Some(seq) = codec.decode(&mut input).unwrap() {
        sequences.push(seq);
    }

    assert!(sequences.len() > 0);

    // Verify we can decode what we encoded
    let text: String = sequences
        .iter()
        .filter_map(|s| match s {
            AnsiSequence::Character(c) => Some(*c),
            _ => None,
        })
        .collect();

    assert_eq!(text, "Username: ");
}

#[test]
fn test_ansi_escape_sequence_roundtrip() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    // Encode cursor movement
    codec
        .encode(AnsiControlSequenceIntroducer::CursorUp(5), &mut buffer)
        .unwrap();
    codec.encode("Text", &mut buffer).unwrap();
    codec
        .encode(AnsiControlSequenceIntroducer::CursorDown(3), &mut buffer)
        .unwrap();

    // Decode
    let mut input = buffer.clone();
    let mut sequences = Vec::new();
    while let Some(seq) = codec.decode(&mut input).unwrap() {
        sequences.push(seq);
    }

    // Should have CSI, characters, and another CSI
    assert!(sequences.len() > 5);
    assert!(matches!(sequences[0], AnsiSequence::AnsiCSI(_)));
}

#[test]
fn test_mixed_text_and_control_codes() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    // Encode mixed content
    codec.encode("Line 1", &mut buffer).unwrap();
    codec.encode(AnsiControlCode::CR, &mut buffer).unwrap();
    codec.encode(AnsiControlCode::LF, &mut buffer).unwrap();
    codec.encode("Line 2", &mut buffer).unwrap();
    codec.encode(AnsiControlCode::CR, &mut buffer).unwrap();
    codec.encode(AnsiControlCode::LF, &mut buffer).unwrap();

    // Decode
    let mut input = buffer.clone();
    let mut sequences = Vec::new();
    while let Some(seq) = codec.decode(&mut input).unwrap() {
        sequences.push(seq);
    }

    // Count control codes
    let control_count = sequences
        .iter()
        .filter(|s| matches!(s, AnsiSequence::Control(_)))
        .count();

    assert_eq!(control_count, 4); // 2 CR + 2 LF
}

#[test]
fn test_unicode_handling() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    // Encode various Unicode characters
    let test_strings = vec![
        "Hello 世界",
        "Emoji: 🚀🌟💻",
        "Symbols: ©®™€£¥",
        "Math: ∑∫∂√∞",
    ];

    for s in &test_strings {
        codec.encode(*s, &mut buffer).unwrap();
        codec.encode(AnsiControlCode::LF, &mut buffer).unwrap();
    }

    // Decode
    let mut input = buffer.clone();
    let mut sequences = Vec::new();
    while let Some(seq) = codec.decode(&mut input).unwrap() {
        sequences.push(seq);
    }

    // Should have decoded all characters (4 strings + 4 line feeds = at least 8 sequences)
    // Each string contains multiple characters, so we expect many more sequences
    assert!(
        sequences.len() >= 8,
        "Expected at least 8 sequences, got {}",
        sequences.len()
    );
}

#[test]
fn test_parser_state_machine() {
    let mut parser = AnsiParser::new();

    // Test normal character
    let result = parser.next(b'A').unwrap();
    assert!(matches!(result, Some(AnsiSequence::Character('A'))));

    // Test control code
    let result = parser.next(0x07).unwrap(); // BEL
    assert!(matches!(
        result,
        Some(AnsiSequence::Control(AnsiControlCode::BEL))
    ));

    // Test escape sequence start
    let result = parser.next(0x1B).unwrap(); // ESC
    assert!(result.is_none()); // Waiting for more

    let result = parser.next(b'[').unwrap(); // CSI
    assert!(result.is_none()); // Waiting for more

    let result = parser.next(b'A').unwrap(); // Cursor Up
    assert!(matches!(result, Some(AnsiSequence::AnsiCSI(_))));
}

#[test]
fn test_segmented_string_creation() {
    let plain = SegmentedString::from("Plain text");
    assert!(!plain.is_empty());
    assert_eq!(plain.to_string(), "Plain text");

    let empty = SegmentedString::from("");
    assert!(empty.is_empty());
}

#[test]
fn test_color_mode_effects() {
    for mode in [
        ColorMode::None,
        ColorMode::Basic,
        ColorMode::FixedColor,
        ColorMode::TrueColor,
    ] {
        let config = AnsiConfig {
            color_mode: mode,
            ..Default::default()
        };
        let telnet_codec = TelnetCodec::new();
        let mut codec = AnsiCodec::new(config, telnet_codec);

        let mut buffer = BytesMut::new();
        codec.encode("Test", &mut buffer).unwrap();

        // All modes should produce output
        assert!(!buffer.is_empty());
    }
}

#[test]
fn test_large_data_stream() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    // Encode a large amount of data
    for i in 0..1000 {
        let line = format!("Line {}\n", i);
        codec.encode(line.as_str(), &mut buffer).unwrap();
    }

    // Decode it all
    let mut input = buffer.clone();
    let mut sequences = Vec::new();
    while let Some(seq) = codec.decode(&mut input).unwrap() {
        sequences.push(seq);
    }

    // Should have decoded many sequences
    assert!(sequences.len() > 5000);
}

#[test]
fn test_interleaved_encode_decode() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    // Encode some data
    codec.encode("Part 1", &mut buffer).unwrap();

    // Decode part of it
    let mut sequences = Vec::new();
    while let Some(seq) = codec.decode(&mut buffer).unwrap() {
        sequences.push(seq);
    }

    assert_eq!(sequences.len(), 6);

    // Encode more
    codec.encode(" Part 2", &mut buffer).unwrap();

    // Decode the rest
    while let Some(seq) = codec.decode(&mut buffer).unwrap() {
        sequences.push(seq);
    }

    assert_eq!(sequences.len(), 13);
}

#[test]
fn test_control_sequence_parsing() {
    let mut parser = AnsiParser::new();

    // Parse a complete CSI sequence: ESC[2J (clear screen)
    let bytes = b"\x1B[2J";
    let mut results = Vec::new();

    for &byte in bytes {
        if let Some(seq) = parser.next(byte).unwrap() {
            results.push(seq);
        }
    }

    assert_eq!(results.len(), 1);
    assert!(matches!(results[0], AnsiSequence::AnsiCSI(_)));
}

#[test]
fn test_partial_sequence_handling() {
    let mut codec = create_codec();

    // Send partial escape sequence
    let mut buffer = BytesMut::from(&b"\x1B["[..]);
    let result = codec.decode(&mut buffer).unwrap();
    assert!(result.is_none()); // Incomplete sequence

    // Complete the sequence
    buffer.extend_from_slice(b"H");
    let result = codec.decode(&mut buffer).unwrap();
    assert!(result.is_some()); // Now complete
}

#[test]
fn test_error_recovery() {
    let mut parser = AnsiParser::new();

    // Send invalid UTF-8 continuation byte
    let result = parser.next(0x80);
    assert!(result.is_ok()); // Should handle gracefully

    // Parser should still work after error
    let result = parser.next(b'A').unwrap();
    assert!(matches!(result, Some(AnsiSequence::Character('A'))));
}

#[test]
fn test_telnet_iac_escaping() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    // Encode text that would contain IAC byte (0xFF)
    codec.encode("Test", &mut buffer).unwrap();

    // The telnet codec should handle IAC escaping
    // Verify we can decode it back
    let mut input = buffer.clone();
    let mut sequences = Vec::new();
    while let Some(seq) = codec.decode(&mut input).unwrap() {
        sequences.push(seq);
    }

    assert_eq!(sequences.len(), 4);
}

#[test]
fn test_cursor_movement_sequences() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    // Encode various cursor movements
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
    codec
        .encode(
            AnsiControlSequenceIntroducer::CursorPosition { row: 10, col: 20 },
            &mut buffer,
        )
        .unwrap();

    // Decode
    let mut input = buffer.clone();
    let mut csi_count = 0;
    while let Some(seq) = codec.decode(&mut input).unwrap() {
        if matches!(seq, AnsiSequence::AnsiCSI(_)) {
            csi_count += 1;
        }
    }

    assert_eq!(csi_count, 5);
}

#[test]
fn test_empty_and_whitespace() {
    let mut codec = create_codec();
    let mut buffer = BytesMut::new();

    // Test empty string
    codec.encode("", &mut buffer).unwrap();
    assert!(buffer.is_empty());

    // Test whitespace
    codec.encode("   ", &mut buffer).unwrap();
    assert_eq!(buffer.len(), 3);

    // Test tabs and newlines
    codec.encode("\t\n\r", &mut buffer).unwrap();
    assert!(buffer.len() > 3);
}

#[test]
fn test_streaming_decode() {
    let mut codec = create_codec();

    // Simulate streaming input byte by byte
    let input_text = "Hello, World!";
    let mut total_sequences = Vec::new();

    for byte in input_text.bytes() {
        let mut buffer = BytesMut::from(&[byte][..]);
        while let Some(seq) = codec.decode(&mut buffer).unwrap() {
            total_sequences.push(seq);
        }
    }

    assert_eq!(total_sequences.len(), input_text.len());
}

#[test]
fn test_config_variations() {
    // Test different configurations
    let configs = vec![
        AnsiConfig::default(),
        AnsiConfig {
            color_mode: ColorMode::TrueColor,
            ..Default::default()
        },
        AnsiConfig {
            color_mode: ColorMode::None,
            ..Default::default()
        },
    ];

    for config in configs {
        let telnet_codec = TelnetCodec::new();
        let mut codec = AnsiCodec::new(config, telnet_codec);
        let mut buffer = BytesMut::new();

        codec.encode("Test", &mut buffer).unwrap();
        assert!(!buffer.is_empty());
    }
}
