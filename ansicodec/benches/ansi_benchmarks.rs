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

//! Benchmarks for AnsiCodec performance

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use termionix_ansicodec::ansi::{AnsiControlCode, AnsiControlSequenceIntroducer, AnsiSequence};
use termionix_ansicodec::{AnsiCodec, AnsiConfig, AnsiParser, ColorMode};
use termionix_telnetcodec::TelnetCodec;
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

fn create_codec() -> AnsiCodec<TelnetCodec> {
    let telnet_codec = TelnetCodec::new();
    AnsiCodec::new(AnsiConfig::default(), telnet_codec)
}

// Benchmark encoding plain text
fn bench_encode_plain_text(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_plain_text");

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let text = "A".repeat(size);
            let mut codec = create_codec();

            b.iter(|| {
                let mut buffer = BytesMut::new();
                codec.encode(black_box(text.as_str()), &mut buffer).unwrap();
                black_box(buffer);
            });
        });
    }
    group.finish();
}

// Benchmark decoding plain text
fn bench_decode_plain_text(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_plain_text");

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let text = "A".repeat(size);
            let mut codec = create_codec();

            b.iter(|| {
                let mut buffer = BytesMut::from(black_box(text.as_str()));
                let mut count = 0;
                while codec.decode(&mut buffer).unwrap().is_some() {
                    count += 1;
                }
                black_box(count);
            });
        });
    }
    group.finish();
}

// Benchmark encoding control codes
fn bench_encode_control_codes(c: &mut Criterion) {
    c.bench_function("encode_control_codes", |b| {
        let mut codec = create_codec();
        let codes = vec![
            AnsiControlCode::BEL,
            AnsiControlCode::BS,
            AnsiControlCode::HT,
            AnsiControlCode::LF,
            AnsiControlCode::CR,
        ];

        b.iter(|| {
            let mut buffer = BytesMut::new();
            for code in &codes {
                codec.encode(black_box(*code), &mut buffer).unwrap();
            }
            black_box(buffer);
        });
    });
}

// Benchmark encoding CSI sequences
fn bench_encode_csi_sequences(c: &mut Criterion) {
    c.bench_function("encode_csi_sequences", |b| {
        let mut codec = create_codec();
        let sequences = vec![
            AnsiControlSequenceIntroducer::CursorUp(5),
            AnsiControlSequenceIntroducer::CursorDown(3),
            AnsiControlSequenceIntroducer::CursorForward(10),
            AnsiControlSequenceIntroducer::CursorBack(2),
            AnsiControlSequenceIntroducer::CursorPosition { row: 10, col: 20 },
        ];

        b.iter(|| {
            let mut buffer = BytesMut::new();
            for seq in &sequences {
                codec.encode(black_box(seq.clone()), &mut buffer).unwrap();
            }
            black_box(buffer);
        });
    });
}

// Benchmark parser state machine
fn bench_parser_characters(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_characters");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let data: Vec<u8> = (0..size).map(|i| b'A' + (i % 26) as u8).collect();

            b.iter(|| {
                let mut parser = AnsiParser::new();
                let mut count = 0;
                for &byte in black_box(&data) {
                    if parser.next(byte).unwrap().is_some() {
                        count += 1;
                    }
                }
                black_box(count);
            });
        });
    }
    group.finish();
}

// Benchmark parser with escape sequences
fn bench_parser_with_escapes(c: &mut Criterion) {
    c.bench_function("parser_with_escapes", |b| {
        // Mix of text and escape sequences
        let data = b"Hello\x1B[2JWorld\x1B[H\x1B[31mRed\x1B[0m";

        b.iter(|| {
            let mut parser = AnsiParser::new();
            let mut count = 0;
            for &byte in black_box(data) {
                if parser.next(byte).unwrap().is_some() {
                    count += 1;
                }
            }
            black_box(count);
        });
    });
}

// Benchmark Unicode handling
fn bench_unicode_encoding(c: &mut Criterion) {
    c.bench_function("unicode_encoding", |b| {
        let mut codec = create_codec();
        let text = "Hello 世界 🌍 Emoji 🚀";

        b.iter(|| {
            let mut buffer = BytesMut::new();
            codec.encode(black_box(text), &mut buffer).unwrap();
            black_box(buffer);
        });
    });
}

// Benchmark Unicode decoding
fn bench_unicode_decoding(c: &mut Criterion) {
    c.bench_function("unicode_decoding", |b| {
        let text = "Hello 世界 🌍 Emoji 🚀";
        let mut codec = create_codec();

        b.iter(|| {
            let mut buffer = BytesMut::from(black_box(text));
            let mut count = 0;
            while codec.decode(&mut buffer).unwrap().is_some() {
                count += 1;
            }
            black_box(count);
        });
    });
}

// Benchmark mixed content (text + control codes + CSI)
fn bench_mixed_content(c: &mut Criterion) {
    c.bench_function("mixed_content_encode", |b| {
        let mut codec = create_codec();

        b.iter(|| {
            let mut buffer = BytesMut::new();
            codec.encode(black_box("Line 1"), &mut buffer).unwrap();
            codec
                .encode(black_box(AnsiControlCode::CR), &mut buffer)
                .unwrap();
            codec
                .encode(black_box(AnsiControlCode::LF), &mut buffer)
                .unwrap();
            codec
                .encode(
                    black_box(AnsiControlSequenceIntroducer::CursorUp(1)),
                    &mut buffer,
                )
                .unwrap();
            codec.encode(black_box("Line 2"), &mut buffer).unwrap();
            black_box(buffer);
        });
    });
}

// Benchmark roundtrip (encode + decode)
fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let text = "A".repeat(size);

            b.iter(|| {
                let mut codec = create_codec();

                // Encode
                let mut encode_buffer = BytesMut::new();
                codec
                    .encode(black_box(text.as_str()), &mut encode_buffer)
                    .unwrap();

                // Decode
                let mut decode_buffer = encode_buffer.clone();
                let mut count = 0;
                while codec.decode(&mut decode_buffer).unwrap().is_some() {
                    count += 1;
                }
                black_box(count);
            });
        });
    }
    group.finish();
}

// Benchmark different color modes
fn bench_color_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("color_modes");

    for mode in [
        ColorMode::None,
        ColorMode::Basic,
        ColorMode::FixedColor,
        ColorMode::TrueColor,
    ] {
        group.bench_function(format!("{:?}", mode), |b| {
            let config = AnsiConfig {
                color_mode: mode,
                ..Default::default()
            };
            let telnet_codec = TelnetCodec::new();
            let mut codec = AnsiCodec::new(config, telnet_codec);

            b.iter(|| {
                let mut buffer = BytesMut::new();
                codec.encode(black_box("Test text"), &mut buffer).unwrap();
                black_box(buffer);
            });
        });
    }
    group.finish();
}

// Benchmark sequence type encoding
fn bench_sequence_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequence_types");

    group.bench_function("character", |b| {
        let mut codec = create_codec();
        let seq = AnsiSequence::Character('A');

        b.iter(|| {
            let mut buffer = BytesMut::new();
            codec.encode(black_box(seq.clone()), &mut buffer).unwrap();
            black_box(buffer);
        });
    });

    group.bench_function("unicode", |b| {
        let mut codec = create_codec();
        let seq = AnsiSequence::Unicode('世');

        b.iter(|| {
            let mut buffer = BytesMut::new();
            codec.encode(black_box(seq.clone()), &mut buffer).unwrap();
            black_box(buffer);
        });
    });

    group.bench_function("control", |b| {
        let mut codec = create_codec();
        let seq = AnsiSequence::Control(AnsiControlCode::LF);

        b.iter(|| {
            let mut buffer = BytesMut::new();
            codec.encode(black_box(seq.clone()), &mut buffer).unwrap();
            black_box(buffer);
        });
    });

    group.bench_function("escape", |b| {
        let mut codec = create_codec();
        let seq = AnsiSequence::AnsiEscape;

        b.iter(|| {
            let mut buffer = BytesMut::new();
            codec.encode(black_box(seq.clone()), &mut buffer).unwrap();
            black_box(buffer);
        });
    });

    group.finish();
}

// Benchmark large data streams
fn bench_large_stream(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_stream");
    group.sample_size(10); // Reduce sample size for large benchmarks

    for size in [10000, 50000, 100000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let text = "A".repeat(size);
            let mut codec = create_codec();

            b.iter(|| {
                let mut buffer = BytesMut::new();
                codec.encode(black_box(text.as_str()), &mut buffer).unwrap();
                black_box(buffer);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_encode_plain_text,
    bench_decode_plain_text,
    bench_encode_control_codes,
    bench_encode_csi_sequences,
    bench_parser_characters,
    bench_parser_with_escapes,
    bench_unicode_encoding,
    bench_unicode_decoding,
    bench_mixed_content,
    bench_roundtrip,
    bench_color_modes,
    bench_sequence_types,
    bench_large_stream,
);

criterion_main!(benches);
