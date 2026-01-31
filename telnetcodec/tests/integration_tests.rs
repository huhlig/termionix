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

//! Integration tests for telnetcodec
//!
//! These tests verify end-to-end functionality and interactions between components.

use bytes::BytesMut;
use termionix_telnetcodec::{
    TelnetArgument, TelnetCodec, TelnetEvent, TelnetFrame, TelnetOption, TelnetSide, naws,
};
use tokio_util::codec::{Decoder, Encoder};

// ============================================================================
// Helper Functions
// ============================================================================

fn create_client_server_pair() -> (TelnetCodec, TelnetCodec) {
    (TelnetCodec::new(), TelnetCodec::new())
}

fn encode_frames(codec: &mut TelnetCodec, frames: Vec<TelnetFrame>) -> BytesMut {
    let mut buffer = BytesMut::new();
    for frame in frames {
        codec.encode(frame, &mut buffer).unwrap();
    }
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
// Client-Server Negotiation Tests
// ============================================================================

#[test]
fn client_server_echo_negotiation() {
    let (mut client, mut server) = create_client_server_pair();

    // Server requests client to enable Echo
    let mut buffer = BytesMut::new();
    server
        .encode(TelnetFrame::Do(TelnetOption::Echo), &mut buffer)
        .unwrap();

    // Client receives DO Echo and responds with WILL Echo
    let events = decode_all(&mut client, &mut buffer);
    assert_eq!(
        events,
        vec![TelnetEvent::OptionStatus(
            TelnetOption::Echo,
            TelnetSide::Local,
            true
        )]
    );
    assert!(client.is_enabled_local(TelnetOption::Echo));
}

#[test]
fn client_server_binary_negotiation() {
    let (mut client, mut server) = create_client_server_pair();

    // Client offers to enable Binary Transmission
    let mut buffer = BytesMut::new();
    client
        .encode(TelnetFrame::Will(TelnetOption::TransmitBinary), &mut buffer)
        .unwrap();

    // Server receives WILL Binary and responds with DO Binary
    let events = decode_all(&mut server, &mut buffer);
    assert_eq!(
        events,
        vec![TelnetEvent::OptionStatus(
            TelnetOption::TransmitBinary,
            TelnetSide::Remote,
            true
        )]
    );
    assert!(server.is_enabled_remote(TelnetOption::TransmitBinary));
}

#[test]
fn client_server_mutual_binary_negotiation() {
    let (mut client, mut server) = create_client_server_pair();

    // Both sides enable binary transmission
    let mut client_buffer = BytesMut::new();
    client
        .encode(
            TelnetFrame::Will(TelnetOption::TransmitBinary),
            &mut client_buffer,
        )
        .unwrap();

    let mut server_buffer = BytesMut::new();
    server
        .encode(
            TelnetFrame::Will(TelnetOption::TransmitBinary),
            &mut server_buffer,
        )
        .unwrap();

    // Server receives client's WILL
    let events = decode_all(&mut server, &mut client_buffer);
    assert!(server.is_enabled_remote(TelnetOption::TransmitBinary));

    // Client receives server's WILL
    let events = decode_all(&mut client, &mut server_buffer);
    assert!(client.is_enabled_remote(TelnetOption::TransmitBinary));
}

#[test]
fn client_server_reject_unsupported_option() {
    let (mut client, mut server) = create_client_server_pair();

    // Server requests an unsupported option
    let mut buffer = BytesMut::new();
    server
        .encode(TelnetFrame::Do(TelnetOption::Unknown(200)), &mut buffer)
        .unwrap();

    // Client should reject it (no OptionStatus event for unsupported options)
    let events = decode_all(&mut client, &mut buffer);
    // Unsupported options may not generate events or may generate rejection
    assert!(!client.is_enabled_local(TelnetOption::Unknown(200)));
}

// ============================================================================
// Data Transfer Tests
// ============================================================================

#[test]
fn client_server_simple_data_transfer() {
    let (mut client, mut server) = create_client_server_pair();

    // Client sends data to server
    let message = "Hello, Server!";
    let mut buffer = BytesMut::new();
    for byte in message.bytes() {
        client.encode(TelnetFrame::Data(byte), &mut buffer).unwrap();
    }

    // Server receives the data
    let events = decode_all(&mut server, &mut buffer);
    let received: Vec<u8> = events
        .iter()
        .filter_map(|e| match e {
            TelnetEvent::Data(b) => Some(*b),
            _ => None,
        })
        .collect();

    assert_eq!(received, message.as_bytes());
}

#[test]
fn client_server_binary_data_transfer() {
    let (mut client, mut server) = create_client_server_pair();

    // Enable binary mode
    let mut buffer = BytesMut::new();
    client
        .encode(TelnetFrame::Will(TelnetOption::TransmitBinary), &mut buffer)
        .unwrap();
    decode_all(&mut server, &mut buffer);

    // Send binary data including IAC byte
    let binary_data: Vec<u8> = vec![0x00, 0xFF, 0x80, 0x7F, 0xFF];
    let mut buffer = BytesMut::new();
    for byte in &binary_data {
        client
            .encode(TelnetFrame::Data(*byte), &mut buffer)
            .unwrap();
    }

    // Server receives the binary data
    let events = decode_all(&mut server, &mut buffer);
    let received: Vec<u8> = events
        .iter()
        .filter_map(|e| match e {
            TelnetEvent::Data(b) => Some(*b),
            _ => None,
        })
        .collect();

    assert_eq!(received, binary_data);
}

#[test]
fn client_server_data_with_interspersed_commands() {
    let (mut client, mut server) = create_client_server_pair();

    // Send data with commands interspersed
    let mut buffer = BytesMut::new();
    client.encode(TelnetFrame::Data(b'H'), &mut buffer).unwrap();
    client.encode(TelnetFrame::Data(b'i'), &mut buffer).unwrap();
    client
        .encode(TelnetFrame::NoOperation, &mut buffer)
        .unwrap();
    client.encode(TelnetFrame::Data(b'!'), &mut buffer).unwrap();

    let events = decode_all(&mut server, &mut buffer);
    assert_eq!(
        events,
        vec![
            TelnetEvent::Data(b'H'),
            TelnetEvent::Data(b'i'),
            TelnetEvent::NoOperation,
            TelnetEvent::Data(b'!'),
        ]
    );
}

// ============================================================================
// Subnegotiation Tests
// ============================================================================

#[test]
fn client_server_naws_subnegotiation() {
    let (mut client, mut server) = create_client_server_pair();

    // Client sends window size
    let window_size = naws::WindowSize::new(120, 40);
    let arg = TelnetArgument::NAWSWindowSize(window_size);

    let mut buffer = BytesMut::new();
    client
        .encode(TelnetFrame::Subnegotiate(arg), &mut buffer)
        .unwrap();

    // Server receives the window size
    let events = decode_all(&mut server, &mut buffer);
    assert_eq!(events.len(), 1);

    match &events[0] {
        TelnetEvent::Subnegotiate(TelnetArgument::NAWSWindowSize(size)) => {
            assert_eq!(size.cols, 120);
            assert_eq!(size.rows, 40);
        }
        _ => panic!("Expected NAWS subnegotiation event"),
    }
}

#[test]
fn client_server_unknown_subnegotiation() {
    let (mut client, mut server) = create_client_server_pair();

    // Client sends unknown subnegotiation
    let data = BytesMut::from(&[1, 2, 3, 4][..]);
    let arg = TelnetArgument::Unknown(TelnetOption::Echo, data.clone());

    let mut buffer = BytesMut::new();
    client
        .encode(TelnetFrame::Subnegotiate(arg), &mut buffer)
        .unwrap();

    // Server receives the subnegotiation
    let events = decode_all(&mut server, &mut buffer);
    assert_eq!(events.len(), 1);

    match &events[0] {
        TelnetEvent::Subnegotiate(TelnetArgument::Unknown(opt, payload)) => {
            assert_eq!(*opt, TelnetOption::Echo);
            assert_eq!(payload, &data);
        }
        _ => panic!("Expected Unknown subnegotiation event"),
    }
}

// ============================================================================
// Control Command Tests
// ============================================================================

#[test]
fn client_server_interrupt_process() {
    let (mut client, mut server) = create_client_server_pair();

    // Client sends interrupt process command
    let mut buffer = BytesMut::new();
    client
        .encode(TelnetFrame::InterruptProcess, &mut buffer)
        .unwrap();

    let events = decode_all(&mut server, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::InterruptProcess]);
}

#[test]
fn client_server_break_command() {
    let (mut client, mut server) = create_client_server_pair();

    // Client sends break command
    let mut buffer = BytesMut::new();
    client.encode(TelnetFrame::Break, &mut buffer).unwrap();

    let events = decode_all(&mut server, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::Break]);
}

#[test]
fn client_server_are_you_there() {
    let (mut client, mut server) = create_client_server_pair();

    // Client sends AYT command
    let mut buffer = BytesMut::new();
    client
        .encode(TelnetFrame::AreYouThere, &mut buffer)
        .unwrap();

    let events = decode_all(&mut server, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::AreYouThere]);
}

// ============================================================================
// Complex Scenario Tests
// ============================================================================

#[test]
fn client_server_login_sequence() {
    let (mut client, mut server) = create_client_server_pair();

    // Server sends login prompt
    let mut server_buffer = BytesMut::new();
    server.encode("Login: ", &mut server_buffer).unwrap();

    // Client receives prompt
    let events = decode_all(&mut client, &mut server_buffer);
    let prompt: String = events
        .iter()
        .filter_map(|e| match e {
            TelnetEvent::Data(b) => Some(*b as char),
            _ => None,
        })
        .collect();
    assert_eq!(prompt, "Login: ");

    // Client sends username
    let mut client_buffer = BytesMut::new();
    client.encode("alice\r\n", &mut client_buffer).unwrap();

    // Server receives username
    let events = decode_all(&mut server, &mut client_buffer);
    let username: String = events
        .iter()
        .filter_map(|e| match e {
            TelnetEvent::Data(b) => Some(*b as char),
            _ => None,
        })
        .collect();
    assert_eq!(username, "alice\r\n");
}

#[test]
fn client_server_option_negotiation_sequence() {
    let (mut client, mut server) = create_client_server_pair();

    // Server initiates multiple option negotiations
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

    // Client processes all negotiations
    let events = decode_all(&mut client, &mut buffer);

    // Verify all options are enabled
    assert!(client.is_enabled_local(TelnetOption::Echo));
    assert!(client.is_enabled_local(TelnetOption::SuppressGoAhead));
    assert!(client.is_enabled_remote(TelnetOption::TransmitBinary));

    // Should have received 3 OptionStatus events
    let option_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, TelnetEvent::OptionStatus(..)))
        .collect();
    assert_eq!(option_events.len(), 3);
}

#[test]
fn client_server_streaming_data() {
    let (mut client, mut server) = create_client_server_pair();

    // Simulate streaming data in chunks
    let message = "This is a long message that will be sent in multiple chunks.";
    let chunk_size = 10;

    for chunk in message.as_bytes().chunks(chunk_size) {
        let mut buffer = BytesMut::new();
        for &byte in chunk {
            client.encode(TelnetFrame::Data(byte), &mut buffer).unwrap();
        }

        // Server receives each chunk
        let events = decode_all(&mut server, &mut buffer);
        assert_eq!(events.len(), chunk.len());
    }
}

#[test]
fn client_server_partial_frame_handling() {
    let (mut client, mut server) = create_client_server_pair();

    // Client sends a negotiation command
    let mut full_buffer = BytesMut::new();
    client
        .encode(TelnetFrame::Do(TelnetOption::Echo), &mut full_buffer)
        .unwrap();

    // Split the buffer into partial frames
    let mut partial1 = full_buffer.split_to(1); // Just IAC
    let mut partial2 = full_buffer.split_to(1); // Just DO
    let mut partial3 = full_buffer; // Option byte

    // Server receives partial frames
    let events1 = decode_all(&mut server, &mut partial1);
    assert_eq!(events1.len(), 0); // Incomplete

    let events2 = decode_all(&mut server, &mut partial2);
    assert_eq!(events2.len(), 0); // Still incomplete

    let events3 = decode_all(&mut server, &mut partial3);
    assert_eq!(events3.len(), 1); // Complete
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[test]
fn client_server_recover_from_invalid_sequence() {
    let (mut client, mut server) = create_client_server_pair();

    // Send valid data, invalid sequence, then more valid data
    let mut buffer = BytesMut::new();
    client.encode(TelnetFrame::Data(b'A'), &mut buffer).unwrap();
    buffer.extend_from_slice(&[0xFF, 0xEE]); // Unknown IAC command
    client.encode(TelnetFrame::Data(b'B'), &mut buffer).unwrap();

    let events = decode_all(&mut server, &mut buffer);

    // Should receive data and handle invalid sequence gracefully
    let data_events: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            TelnetEvent::Data(b) => Some(*b),
            _ => None,
        })
        .collect();

    assert!(data_events.contains(&b'A'));
    assert!(data_events.contains(&b'B'));
}

// ============================================================================
// Performance and Stress Tests
// ============================================================================

#[test]
fn client_server_large_data_transfer() {
    let (mut client, mut server) = create_client_server_pair();

    // Send 10KB of data
    let data: Vec<u8> = (0..10240).map(|i| (i % 256) as u8).collect();
    let mut buffer = BytesMut::new();

    for &byte in &data {
        client.encode(TelnetFrame::Data(byte), &mut buffer).unwrap();
    }

    // Server receives all data
    let events = decode_all(&mut server, &mut buffer);
    assert_eq!(events.len(), data.len());

    let received: Vec<u8> = events
        .iter()
        .filter_map(|e| match e {
            TelnetEvent::Data(b) => Some(*b),
            _ => None,
        })
        .collect();

    assert_eq!(received, data);
}

#[test]
fn client_server_many_negotiations() {
    let (mut client, mut server) = create_client_server_pair();

    // Negotiate many options
    let options = vec![
        TelnetOption::Echo,
        TelnetOption::SuppressGoAhead,
        TelnetOption::TransmitBinary,
        TelnetOption::NAWS,
        TelnetOption::TTYPE,
    ];

    for option in &options {
        let mut buffer = BytesMut::new();
        server
            .encode(TelnetFrame::Do(*option), &mut buffer)
            .unwrap();
        decode_all(&mut client, &mut buffer);
    }

    // Verify all supported options are enabled
    for option in &options {
        if client.is_supported_local(*option) {
            assert!(client.is_enabled_local(*option));
        }
    }
}

// ============================================================================
// State Consistency Tests
// ============================================================================

#[test]
fn client_server_state_consistency_after_disable() {
    let (mut client, mut server) = create_client_server_pair();

    // Enable Echo
    let mut buffer = BytesMut::new();
    server
        .encode(TelnetFrame::Do(TelnetOption::Echo), &mut buffer)
        .unwrap();
    decode_all(&mut client, &mut buffer);
    assert!(client.is_enabled_local(TelnetOption::Echo));

    // Disable Echo
    let mut buffer = BytesMut::new();
    server
        .encode(TelnetFrame::Dont(TelnetOption::Echo), &mut buffer)
        .unwrap();
    decode_all(&mut client, &mut buffer);
    assert!(!client.is_enabled_local(TelnetOption::Echo));
}

#[test]
fn client_server_independent_option_states() {
    let (mut client, mut server) = create_client_server_pair();

    // Enable Echo on client (local)
    let mut buffer = BytesMut::new();
    server
        .encode(TelnetFrame::Do(TelnetOption::Echo), &mut buffer)
        .unwrap();
    decode_all(&mut client, &mut buffer);

    // Enable Binary on server (remote from client's perspective)
    let mut buffer = BytesMut::new();
    server
        .encode(TelnetFrame::Will(TelnetOption::TransmitBinary), &mut buffer)
        .unwrap();
    decode_all(&mut client, &mut buffer);

    // Verify independent states
    assert!(client.is_enabled_local(TelnetOption::Echo));
    assert!(!client.is_enabled_remote(TelnetOption::Echo));
    assert!(!client.is_enabled_local(TelnetOption::TransmitBinary));
    assert!(client.is_enabled_remote(TelnetOption::TransmitBinary));
}

// ============================================================================
// RFC Compliance Tests
// ============================================================================

#[test]
fn rfc854_example_1_simple_communication() {
    let (mut client, mut server) = create_client_server_pair();

    // RFC 854 Example: Simple data exchange
    let mut buffer = BytesMut::new();
    client.encode("Hello\r\n", &mut buffer).unwrap();

    let events = decode_all(&mut server, &mut buffer);
    let received: String = events
        .iter()
        .filter_map(|e| match e {
            TelnetEvent::Data(b) => Some(*b as char),
            _ => None,
        })
        .collect();

    assert_eq!(received, "Hello\r\n");
}

#[test]
fn rfc854_iac_escaping() {
    let (mut client, mut server) = create_client_server_pair();

    // RFC 854: IAC must be escaped as IAC IAC
    let mut buffer = BytesMut::new();
    client.encode(TelnetFrame::Data(0xFF), &mut buffer).unwrap();

    // Verify IAC is escaped in the buffer
    assert_eq!(buffer.as_ref(), &[0xFF, 0xFF]);

    // Server should receive single 0xFF
    let events = decode_all(&mut server, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::Data(0xFF)]);
}


// ============================================================================
// EOR (End of Record) Tests
// ============================================================================

#[test]
fn client_server_eor_negotiation() {
    let (mut client, mut server) = create_client_server_pair();

    // Server requests client to enable EOR
    let mut buffer = BytesMut::new();
    server
        .encode(TelnetFrame::Do(TelnetOption::EOR), &mut buffer)
        .unwrap();

    // Client receives DO EOR and responds with WILL EOR
    let events = decode_all(&mut client, &mut buffer);
    assert_eq!(
        events,
        vec![TelnetEvent::OptionStatus(
            TelnetOption::EOR,
            TelnetSide::Local,
            true
        )]
    );
    assert!(client.is_enabled_local(TelnetOption::EOR));
}

#[test]
fn client_server_eor_command() {
    let (mut client, mut server) = create_client_server_pair();

    // Send EOR command
    let mut buffer = BytesMut::new();
    client
        .encode(TelnetFrame::EndOfRecord, &mut buffer)
        .unwrap();

    // Verify EOR is encoded correctly (IAC EOR = 0xFF 0xEF)
    assert_eq!(buffer.as_ref(), &[0xFF, 0xEF]);

    // Server should receive EndOfRecord event
    let events = decode_all(&mut server, &mut buffer);
    assert_eq!(events, vec![TelnetEvent::EndOfRecord]);
}

#[test]
fn client_server_prompt_with_eor() {
    let (mut client, mut server) = create_client_server_pair();

    // Simulate a MUD server sending a prompt with EOR marker
    let mut buffer = BytesMut::new();
    
    // Send prompt text without \r\n
    for &byte in b"HP:100 MP:50> " {
        server.encode(TelnetFrame::Data(byte), &mut buffer).unwrap();
    }
    
    // Mark end of prompt with EOR
    server
        .encode(TelnetFrame::EndOfRecord, &mut buffer)
        .unwrap();

    // Client receives prompt data followed by EOR
    let events = decode_all(&mut client, &mut buffer);
    
    // Extract data bytes
    let data: Vec<u8> = events
        .iter()
        .filter_map(|e| match e {
            TelnetEvent::Data(b) => Some(*b),
            _ => None,
        })
        .collect();
    
    assert_eq!(data, b"HP:100 MP:50> ");
    
    // Verify EOR event is present
    assert!(events.contains(&TelnetEvent::EndOfRecord));
}

#[test]
fn client_server_eor_with_regular_output() {
    let (mut client, mut server) = create_client_server_pair();

    // Send regular output (with \r\n) followed by prompt (with EOR)
    let mut buffer = BytesMut::new();
    
    // Regular output line
    for &byte in b"You enter the room.\r\n" {
        server.encode(TelnetFrame::Data(byte), &mut buffer).unwrap();
    }
    
    // Prompt without \r\n, marked with EOR
    for &byte in b"> " {
        server.encode(TelnetFrame::Data(byte), &mut buffer).unwrap();
    }
    server
        .encode(TelnetFrame::EndOfRecord, &mut buffer)
        .unwrap();

    let events = decode_all(&mut client, &mut buffer);
    
    // Extract all data
    let data: Vec<u8> = events
        .iter()
        .filter_map(|e| match e {
            TelnetEvent::Data(b) => Some(*b),
            _ => None,
        })
        .collect();
    
    assert_eq!(data, b"You enter the room.\r\n> ");
    
    // Verify EOR is at the end
    assert_eq!(events.last(), Some(&TelnetEvent::EndOfRecord));
}

#[test]
fn encode_decode_eor_roundtrip() {
    let mut codec = TelnetCodec::new();
    
    // Encode EOR
    let mut buffer = BytesMut::new();
    codec.encode(TelnetFrame::EndOfRecord, &mut buffer).unwrap();
    
    // Decode EOR
    let event = codec.decode(&mut buffer).unwrap();
    assert_eq!(event, Some(TelnetEvent::EndOfRecord));
    
    // Buffer should be empty
    assert!(codec.decode(&mut buffer).unwrap().is_none());
}
