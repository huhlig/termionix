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

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use termionix_terminal::{AnsiCodec, TelnetCodec, TerminalBuffer, TerminalCodec};
use tokio_util::bytes::BytesMut;
use tokio_util::codec::{Decoder, Encoder};

fn create_test_codec() -> TerminalCodec<AnsiCodec<TelnetCodec>> {
    let telnet_codec = TelnetCodec::new();
    let ansi_codec = AnsiCodec::new(termionix_ansicodec::AnsiConfig::default(), telnet_codec);
    TerminalCodec::new(ansi_codec)
}

// ===== Terminal Buffer Benchmarks =====

fn bench_buffer_creation(c: &mut Criterion) {
    c.bench_function("buffer_new", |b| {
        b.iter(|| {
            let buffer = TerminalBuffer::new();
            black_box(buffer);
        });
    });

    c.bench_function("buffer_new_with_size", |b| {
        b.iter(|| {
            let buffer = TerminalBuffer::new_with_size(120, 40);
            black_box(buffer);
        });
    });
}

fn bench_buffer_append_char(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_append_char");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut buffer = TerminalBuffer::new();
                for _ in 0..size {
                    buffer.append_char(black_box('A'));
                }
                black_box(buffer);
            });
        });
    }

    group.finish();
}

fn bench_buffer_append_line(c: &mut Criterion) {
    let mut group = c.benchmark_group("buffer_append_line");

    for line_count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*line_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(line_count),
            line_count,
            |b, &line_count| {
                b.iter(|| {
                    let mut buffer = TerminalBuffer::new();
                    for i in 0..line_count {
                        buffer.append_line(black_box(&format!("Line {}", i)));
                    }
                    black_box(buffer);
                });
            },
        );
    }

    group.finish();
}

fn bench_buffer_complete_line(c: &mut Criterion) {
    c.bench_function("buffer_complete_line", |b| {
        b.iter(|| {
            let mut buffer = TerminalBuffer::new();
            buffer.append_char('H');
            buffer.append_char('e');
            buffer.append_char('l');
            buffer.append_char('l');
            buffer.append_char('o');
            buffer.complete_line();
            black_box(buffer);
        });
    });
}

fn bench_buffer_pop_completed_line(c: &mut Criterion) {
    c.bench_function("buffer_pop_completed_line", |b| {
        b.iter_batched(
            || {
                let mut buffer = TerminalBuffer::new();
                for i in 0..100 {
                    buffer.append_line(&format!("Line {}", i));
                }
                buffer
            },
            |mut buffer| {
                while buffer.pop_completed_line().is_some() {
                    // Pop all lines
                }
                black_box(buffer);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_buffer_cursor_operations(c: &mut Criterion) {
    c.bench_function("buffer_set_cursor_position", |b| {
        b.iter(|| {
            let mut buffer = TerminalBuffer::new();
            for i in 0..100 {
                buffer.set_cursor_position(black_box(i % 80), black_box(i % 24));
            }
            black_box(buffer);
        });
    });

    c.bench_function("buffer_cursor_position", |b| {
        let buffer = TerminalBuffer::new();
        b.iter(|| {
            let pos = buffer.cursor_position();
            black_box(pos);
        });
    });
}

fn bench_buffer_erase_operations(c: &mut Criterion) {
    c.bench_function("buffer_erase_character", |b| {
        b.iter_batched(
            || {
                let mut buffer = TerminalBuffer::new();
                for _ in 0..100 {
                    buffer.append_char('X');
                }
                buffer
            },
            |mut buffer| {
                for _ in 0..50 {
                    buffer.erase_character();
                }
                black_box(buffer);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    c.bench_function("buffer_erase_line", |b| {
        b.iter_batched(
            || {
                let mut buffer = TerminalBuffer::new();
                for _ in 0..100 {
                    buffer.append_char('X');
                }
                buffer
            },
            |mut buffer| {
                buffer.erase_line();
                black_box(buffer);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_buffer_clear(c: &mut Criterion) {
    c.bench_function("buffer_clear", |b| {
        b.iter_batched(
            || {
                let mut buffer = TerminalBuffer::new();
                for i in 0..100 {
                    buffer.append_line(&format!("Line {}", i));
                }
                buffer
            },
            |mut buffer| {
                buffer.clear();
                black_box(buffer);
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_buffer_resize(c: &mut Criterion) {
    c.bench_function("buffer_set_size", |b| {
        b.iter(|| {
            let mut buffer = TerminalBuffer::new();
            buffer.set_size(black_box(120), black_box(40));
            buffer.set_size(black_box(80), black_box(24));
            black_box(buffer);
        });
    });
}

fn bench_buffer_environment(c: &mut Criterion) {
    c.bench_function("buffer_set_environment", |b| {
        b.iter(|| {
            let mut buffer = TerminalBuffer::new();
            buffer.set_environment(black_box("TERM"), black_box("xterm-256color"));
            buffer.set_environment(black_box("USER"), black_box("testuser"));
            buffer.set_environment(black_box("SHELL"), black_box("/bin/bash"));
            black_box(buffer);
        });
    });

    c.bench_function("buffer_get_environment", |b| {
        let mut buffer = TerminalBuffer::new();
        buffer.set_environment("TERM", "xterm-256color");
        buffer.set_environment("USER", "testuser");
        buffer.set_environment("SHELL", "/bin/bash");

        b.iter(|| {
            let term = buffer.get_environment(black_box("TERM"));
            let user = buffer.get_environment(black_box("USER"));
            let shell = buffer.get_environment(black_box("SHELL"));
            black_box((term, user, shell));
        });
    });
}

// ===== Terminal Codec Benchmarks =====

fn bench_codec_creation(c: &mut Criterion) {
    c.bench_function("codec_new", |b| {
        b.iter(|| {
            let codec = create_test_codec();
            black_box(codec);
        });
    });
}

fn bench_codec_decode_ascii(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_decode_ascii");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let text = "A".repeat(size);
            b.iter(|| {
                let mut codec = create_test_codec();
                let mut buffer = BytesMut::from(text.as_str());
                while codec.decode(&mut buffer).unwrap().is_some() {}
                black_box(codec);
            });
        });
    }

    group.finish();
}

fn bench_codec_decode_unicode(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_decode_unicode");

    for size in [10, 100, 1000].iter() {
        let text = "世".repeat(*size);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut codec = create_test_codec();
                let mut buffer = BytesMut::from(text.as_str());
                while codec.decode(&mut buffer).unwrap().is_some() {}
                black_box(codec);
            });
        });
    }

    group.finish();
}

fn bench_codec_decode_mixed(c: &mut Criterion) {
    c.bench_function("codec_decode_mixed_content", |b| {
        let text = "Hello\x07World\x08\nLine2\rLine3";
        b.iter(|| {
            let mut codec = create_test_codec();
            let mut buffer = BytesMut::from(text);
            while codec.decode(&mut buffer).unwrap().is_some() {}
            black_box(codec);
        });
    });
}

fn bench_codec_decode_lines(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_decode_lines");

    for line_count in [10, 100, 1000].iter() {
        let mut text = String::new();
        for i in 0..*line_count {
            text.push_str(&format!("Line {}\n", i));
        }
        group.throughput(Throughput::Elements(*line_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(line_count),
            line_count,
            |b, _| {
                b.iter(|| {
                    let mut codec = create_test_codec();
                    let mut buffer = BytesMut::from(text.as_str());
                    while codec.decode(&mut buffer).unwrap().is_some() {}
                    black_box(codec);
                });
            },
        );
    }

    group.finish();
}

fn bench_codec_encode_char(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_encode_char");

    for size in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter(|| {
                let mut codec = create_test_codec();
                let mut buffer = BytesMut::new();
                for _ in 0..size {
                    codec.encode(black_box('A'), &mut buffer).unwrap();
                }
                black_box((codec, buffer));
            });
        });
    }

    group.finish();
}

fn bench_codec_encode_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("codec_encode_string");

    for size in [10, 100, 1000].iter() {
        let text = "A".repeat(*size);
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut codec = create_test_codec();
                let mut buffer = BytesMut::new();
                for ch in text.chars() {
                    codec.encode(black_box(ch), &mut buffer).unwrap();
                }
                black_box((codec, buffer));
            });
        });
    }

    group.finish();
}

fn bench_codec_roundtrip(c: &mut Criterion) {
    c.bench_function("codec_encode_decode_roundtrip", |b| {
        let text = "Test String with various characters 123!@#";
        b.iter(|| {
            let mut encode_codec = create_test_codec();
            let mut decode_codec = create_test_codec();
            let mut buffer = BytesMut::new();

            // Encode
            for ch in text.chars() {
                encode_codec.encode(ch, &mut buffer).unwrap();
            }

            // Decode
            while decode_codec.decode(&mut buffer).unwrap().is_some() {}

            black_box((encode_codec, decode_codec));
        });
    });
}

fn bench_codec_control_codes(c: &mut Criterion) {
    c.bench_function("codec_decode_control_codes", |b| {
        let data = vec![0x07, 0x08, 0x09, 0x0A, 0x0D, 0x0C, 0x7F];
        b.iter(|| {
            let mut codec = create_test_codec();
            let mut buffer = BytesMut::from(&data[..]);
            while codec.decode(&mut buffer).unwrap().is_some() {}
            black_box(codec);
        });
    });
}

// ===== Integrated Workflow Benchmarks =====

fn bench_full_terminal_workflow(c: &mut Criterion) {
    c.bench_function("full_terminal_workflow", |b| {
        b.iter(|| {
            let mut codec = create_test_codec();
            let mut buffer = BytesMut::from("ls -la\ncd /tmp\npwd\n");

            // Decode all input
            while codec.decode(&mut buffer).unwrap().is_some() {}

            // Access buffer state
            let _size = codec.buffer().size();
            let _cursor = codec.buffer().cursor_position();
            let _line_count = codec.buffer().completed_line_count();

            black_box(codec);
        });
    });
}

fn bench_large_text_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_text_processing");
    group.sample_size(10); // Reduce sample size for large benchmarks

    for size in [1000, 10000].iter() {
        let mut text = String::new();
        for i in 0..*size {
            text.push_str(&format!("Line {} with some content\n", i));
        }
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut codec = create_test_codec();
                let mut buffer = BytesMut::from(text.as_str());
                while codec.decode(&mut buffer).unwrap().is_some() {}
                black_box(codec);
            });
        });
    }

    group.finish();
}

criterion_group!(
    buffer_benches,
    bench_buffer_creation,
    bench_buffer_append_char,
    bench_buffer_append_line,
    bench_buffer_complete_line,
    bench_buffer_pop_completed_line,
    bench_buffer_cursor_operations,
    bench_buffer_erase_operations,
    bench_buffer_clear,
    bench_buffer_resize,
    bench_buffer_environment,
);

criterion_group!(
    codec_benches,
    bench_codec_creation,
    bench_codec_decode_ascii,
    bench_codec_decode_unicode,
    bench_codec_decode_mixed,
    bench_codec_decode_lines,
    bench_codec_encode_char,
    bench_codec_encode_string,
    bench_codec_roundtrip,
    bench_codec_control_codes,
);

criterion_group!(
    workflow_benches,
    bench_full_terminal_workflow,
    bench_large_text_processing,
);

criterion_main!(buffer_benches, codec_benches, workflow_benches);
