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

//! Benchmarks for telnetcodec performance

use bytes::BytesMut;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use termionix_telnetcodec::{
    TelnetArgument, TelnetCodec, TelnetEvent, TelnetFrame, TelnetOption, naws,
};
use tokio_util::codec::{Decoder, Encoder};

// ============================================================================
// Encoding Benchmarks
// ============================================================================

fn bench_encode_single_byte(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_single_byte");

    group.bench_function("data_byte", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer = BytesMut::with_capacity(1024);

        b.iter(|| {
            buffer.clear();
            codec
                .encode(black_box(TelnetFrame::Data(b'A')), &mut buffer)
                .unwrap();
        });
    });

    group.bench_function("iac_byte", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer = BytesMut::with_capacity(1024);

        b.iter(|| {
            buffer.clear();
            codec
                .encode(black_box(TelnetFrame::Data(0xFF)), &mut buffer)
                .unwrap();
        });
    });

    group.finish();
}

fn bench_encode_data_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_data_sizes");

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut codec = TelnetCodec::new();
            let mut buffer = BytesMut::with_capacity(size * 2);
            let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

            b.iter(|| {
                buffer.clear();
                for &byte in &data {
                    codec
                        .encode(black_box(TelnetFrame::Data(byte)), &mut buffer)
                        .unwrap();
                }
            });
        });
    }

    group.finish();
}

fn bench_encode_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_string");

    for size in [10, 100, 1000].iter() {
        let text: String = "A".repeat(*size);
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &text, |b, text| {
            let mut codec = TelnetCodec::new();
            let mut buffer = BytesMut::with_capacity(text.len() * 2);

            b.iter(|| {
                buffer.clear();
                codec.encode(black_box(text.as_str()), &mut buffer).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_encode_commands(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_commands");

    group.bench_function("nop", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer = BytesMut::with_capacity(1024);

        b.iter(|| {
            buffer.clear();
            codec
                .encode(black_box(TelnetFrame::NoOperation), &mut buffer)
                .unwrap();
        });
    });

    group.bench_function("break", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer = BytesMut::with_capacity(1024);

        b.iter(|| {
            buffer.clear();
            codec
                .encode(black_box(TelnetFrame::Break), &mut buffer)
                .unwrap();
        });
    });

    group.bench_function("interrupt_process", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer = BytesMut::with_capacity(1024);

        b.iter(|| {
            buffer.clear();
            codec
                .encode(black_box(TelnetFrame::InterruptProcess), &mut buffer)
                .unwrap();
        });
    });

    group.finish();
}

fn bench_encode_negotiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_negotiation");

    group.bench_function("do_echo", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer = BytesMut::with_capacity(1024);

        b.iter(|| {
            buffer.clear();
            codec
                .encode(black_box(TelnetFrame::Do(TelnetOption::Echo)), &mut buffer)
                .unwrap();
        });
    });

    group.bench_function("will_binary", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer = BytesMut::with_capacity(1024);

        b.iter(|| {
            buffer.clear();
            codec
                .encode(
                    black_box(TelnetFrame::Will(TelnetOption::TransmitBinary)),
                    &mut buffer,
                )
                .unwrap();
        });
    });

    group.finish();
}

fn bench_encode_subnegotiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_subnegotiation");

    group.bench_function("naws", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer = BytesMut::with_capacity(1024);
        let arg = TelnetArgument::NAWSWindowSize(naws::WindowSize::new(80, 24));

        b.iter(|| {
            buffer.clear();
            codec
                .encode(
                    black_box(TelnetFrame::Subnegotiate(arg.clone())),
                    &mut buffer,
                )
                .unwrap();
        });
    });

    group.bench_function("unknown_small", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer = BytesMut::with_capacity(1024);
        let arg = TelnetArgument::Unknown(TelnetOption::Echo, BytesMut::from(&[1, 2, 3, 4][..]));

        b.iter(|| {
            buffer.clear();
            codec
                .encode(
                    black_box(TelnetFrame::Subnegotiate(arg.clone())),
                    &mut buffer,
                )
                .unwrap();
        });
    });

    group.bench_function("unknown_large", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer = BytesMut::with_capacity(2048);
        let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let arg = TelnetArgument::Unknown(TelnetOption::Echo, BytesMut::from(&data[..]));

        b.iter(|| {
            buffer.clear();
            codec
                .encode(
                    black_box(TelnetFrame::Subnegotiate(arg.clone())),
                    &mut buffer,
                )
                .unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Decoding Benchmarks
// ============================================================================

fn bench_decode_single_byte(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_single_byte");

    group.bench_function("data_byte", |b| {
        let mut codec = TelnetCodec::new();

        b.iter(|| {
            let mut buffer = BytesMut::from(&b"A"[..]);
            codec.decode(black_box(&mut buffer)).unwrap();
        });
    });

    group.bench_function("iac_escaped", |b| {
        let mut codec = TelnetCodec::new();

        b.iter(|| {
            let mut buffer = BytesMut::from(&[0xFF, 0xFF][..]);
            codec.decode(black_box(&mut buffer)).unwrap();
        });
    });

    group.finish();
}

fn bench_decode_data_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_data_sizes");

    for size in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let mut codec = TelnetCodec::new();
            let data: Vec<u8> = (0..size).map(|i| (i % 255) as u8).collect(); // Avoid 0xFF

            b.iter(|| {
                let mut buffer = BytesMut::from(&data[..]);
                while codec.decode(black_box(&mut buffer)).unwrap().is_some() {}
            });
        });
    }

    group.finish();
}

fn bench_decode_commands(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_commands");

    group.bench_function("nop", |b| {
        let mut codec = TelnetCodec::new();

        b.iter(|| {
            let mut buffer = BytesMut::from(&[0xFF, 0xF1][..]);
            codec.decode(black_box(&mut buffer)).unwrap();
        });
    });

    group.bench_function("break", |b| {
        let mut codec = TelnetCodec::new();

        b.iter(|| {
            let mut buffer = BytesMut::from(&[0xFF, 0xF3][..]);
            codec.decode(black_box(&mut buffer)).unwrap();
        });
    });

    group.finish();
}

fn bench_decode_negotiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_negotiation");

    group.bench_function("do_echo", |b| {
        let mut codec = TelnetCodec::new();

        b.iter(|| {
            let mut buffer = BytesMut::from(&[0xFF, 0xFD, 0x01][..]);
            codec.decode(black_box(&mut buffer)).unwrap();
        });
    });

    group.bench_function("will_binary", |b| {
        let mut codec = TelnetCodec::new();

        b.iter(|| {
            let mut buffer = BytesMut::from(&[0xFF, 0xFB, 0x00][..]);
            codec.decode(black_box(&mut buffer)).unwrap();
        });
    });

    group.finish();
}

fn bench_decode_subnegotiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_subnegotiation");

    group.bench_function("naws", |b| {
        let mut codec = TelnetCodec::new();

        b.iter(|| {
            let mut buffer =
                BytesMut::from(&[0xFF, 0xFA, 0x1F, 0x00, 0x50, 0x00, 0x18, 0xFF, 0xF0][..]);
            codec.decode(black_box(&mut buffer)).unwrap();
        });
    });

    group.bench_function("unknown_small", |b| {
        let mut codec = TelnetCodec::new();

        b.iter(|| {
            let mut buffer = BytesMut::from(&[0xFF, 0xFA, 0x01, 1, 2, 3, 4, 0xFF, 0xF0][..]);
            codec.decode(black_box(&mut buffer)).unwrap();
        });
    });

    group.finish();
}

fn bench_decode_mixed_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_mixed_content");

    group.bench_function("data_with_commands", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer_template = BytesMut::new();
        buffer_template.extend_from_slice(b"Hello");
        buffer_template.extend_from_slice(&[0xFF, 0xF1]); // NOP
        buffer_template.extend_from_slice(b"World");

        b.iter(|| {
            let mut buffer = buffer_template.clone();
            while codec.decode(black_box(&mut buffer)).unwrap().is_some() {}
        });
    });

    group.bench_function("data_with_negotiation", |b| {
        let mut codec = TelnetCodec::new();
        let mut buffer_template = BytesMut::new();
        buffer_template.extend_from_slice(b"Test");
        buffer_template.extend_from_slice(&[0xFF, 0xFD, 0x01]); // DO Echo
        buffer_template.extend_from_slice(b"Data");

        b.iter(|| {
            let mut buffer = buffer_template.clone();
            while codec.decode(black_box(&mut buffer)).unwrap().is_some() {}
        });
    });

    group.finish();
}

// ============================================================================
// Round-trip Benchmarks
// ============================================================================

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    group.bench_function("single_byte", |b| {
        let mut encoder = TelnetCodec::new();
        let mut decoder = TelnetCodec::new();

        b.iter(|| {
            let mut buffer = BytesMut::new();
            encoder
                .encode(black_box(TelnetFrame::Data(b'A')), &mut buffer)
                .unwrap();
            decoder.decode(black_box(&mut buffer)).unwrap();
        });
    });

    group.bench_function("negotiation", |b| {
        let mut encoder = TelnetCodec::new();
        let mut decoder = TelnetCodec::new();

        b.iter(|| {
            let mut buffer = BytesMut::new();
            encoder
                .encode(black_box(TelnetFrame::Do(TelnetOption::Echo)), &mut buffer)
                .unwrap();
            decoder.decode(black_box(&mut buffer)).unwrap();
        });
    });

    group.bench_function("subnegotiation", |b| {
        let mut encoder = TelnetCodec::new();
        let mut decoder = TelnetCodec::new();
        let arg = TelnetArgument::NAWSWindowSize(naws::WindowSize::new(80, 24));

        b.iter(|| {
            let mut buffer = BytesMut::new();
            encoder
                .encode(
                    black_box(TelnetFrame::Subnegotiate(arg.clone())),
                    &mut buffer,
                )
                .unwrap();
            decoder.decode(black_box(&mut buffer)).unwrap();
        });
    });

    group.finish();
}

// ============================================================================
// Option State Management Benchmarks
// ============================================================================

fn bench_option_state(c: &mut Criterion) {
    let mut group = c.benchmark_group("option_state");

    group.bench_function("enable_local", |b| {
        b.iter(|| {
            let mut codec = TelnetCodec::new();
            codec.enable_local(black_box(TelnetOption::Echo));
        });
    });

    group.bench_function("enable_remote", |b| {
        b.iter(|| {
            let mut codec = TelnetCodec::new();
            codec.enable_remote(black_box(TelnetOption::Echo));
        });
    });

    group.bench_function("is_enabled_local", |b| {
        let codec = TelnetCodec::new();

        b.iter(|| {
            codec.is_enabled_local(black_box(TelnetOption::Echo));
        });
    });

    group.bench_function("is_enabled_remote", |b| {
        let codec = TelnetCodec::new();

        b.iter(|| {
            codec.is_enabled_remote(black_box(TelnetOption::Echo));
        });
    });

    group.finish();
}

// ============================================================================
// Realistic Scenario Benchmarks
// ============================================================================

fn bench_realistic_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic_scenarios");

    group.bench_function("login_prompt", |b| {
        let mut encoder = TelnetCodec::new();
        let mut decoder = TelnetCodec::new();

        b.iter(|| {
            // Server sends prompt
            let mut buffer = BytesMut::new();
            encoder.encode("Username: ", &mut buffer).unwrap();

            // Client receives and processes
            while decoder.decode(black_box(&mut buffer)).unwrap().is_some() {}

            // Client sends response
            buffer.clear();
            encoder.encode("alice\r\n", &mut buffer).unwrap();

            // Server receives
            while decoder.decode(black_box(&mut buffer)).unwrap().is_some() {}
        });
    });

    group.bench_function("option_negotiation_sequence", |b| {
        let mut server = TelnetCodec::new();
        let mut client = TelnetCodec::new();

        b.iter(|| {
            // Server initiates negotiations
            let mut buffer = BytesMut::new();
            server
                .encode(TelnetFrame::Do(TelnetOption::Echo), &mut buffer)
                .unwrap();
            server
                .encode(TelnetFrame::Do(TelnetOption::SuppressGoAhead), &mut buffer)
                .unwrap();
            server
                .encode(TelnetFrame::Will(TelnetOption::TransmitBinary), &mut buffer)
                .unwrap();

            // Client processes
            while client.decode(black_box(&mut buffer)).unwrap().is_some() {}
        });
    });

    group.bench_function("streaming_text", |b| {
        let mut encoder = TelnetCodec::new();
        let mut decoder = TelnetCodec::new();
        let text = "The quick brown fox jumps over the lazy dog. ";

        b.iter(|| {
            let mut buffer = BytesMut::new();

            // Encode text in chunks
            for _ in 0..10 {
                encoder.encode(black_box(text), &mut buffer).unwrap();
            }

            // Decode all
            while decoder.decode(black_box(&mut buffer)).unwrap().is_some() {}
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(
    encoding_benches,
    bench_encode_single_byte,
    bench_encode_data_sizes,
    bench_encode_string,
    bench_encode_commands,
    bench_encode_negotiation,
    bench_encode_subnegotiation
);

criterion_group!(
    decoding_benches,
    bench_decode_single_byte,
    bench_decode_data_sizes,
    bench_decode_commands,
    bench_decode_negotiation,
    bench_decode_subnegotiation,
    bench_decode_mixed_content
);

criterion_group!(roundtrip_benches, bench_roundtrip);

criterion_group!(state_benches, bench_option_state);

criterion_group!(scenario_benches, bench_realistic_scenarios);

criterion_main!(
    encoding_benches,
    decoding_benches,
    roundtrip_benches,
    state_benches,
    scenario_benches
);
