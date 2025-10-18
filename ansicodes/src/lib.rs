//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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

//! ANSI Terminal String Processing Library
//!
//! This library provides robust parsing, manipulation, and generation of ANSI escape sequences
//! for terminal applications. It offers multiple representations optimized for different use cases:
//! parsing terminal output, building styled text, and constructing terminal commands.
//!
//! # Overview
//!
//! The library is built around three core string types, each serving a specific purpose:
//!
//! - **[`StyledString`]** - For building and manipulating styled text with automatic ANSI code generation
//! - **[`SegmentedString`]** - For constructing terminal output with explicit control over segments
//! - **[`SpannedString`]** - For parsing ANSI strings and extracting byte ranges of each element
//!
//! Additionally, the library provides low-level primitives for working directly with ANSI codes:
//!
//! - **[`Style`]** - Text styling attributes (colors, bold, underline, etc.)
//! - **[`Color`]** - Color representation (named, 256-color palette, RGB)
//! - **[`ControlCode`]** - ISO 6429 control characters (C0/C1 sets)
//! - **[`CSICommand`]** - Control Sequence Introducer commands
//! - **[`AnsiConfig`]** - Configuration for ANSI parsing and generation
//!
//! # Core Types
//!
//! ## StyledString
//!
//! [`StyledString`] stores text content with styling metadata separately, making it ideal for:
//! - Building user-facing text with colors and formatting
//! - Applying styles to character ranges
//! - Generating ANSI output automatically
//! - Working with text where you care about content more than control codes
//!
//! ```rust
//! use termionix_ansicodes::{StyledString, Style, Color, Intensity};
//!
//! let mut text = StyledString::empty();
//! text.concat_with_style("Error: ", Style {
//!     foreground: Some(Color::Red),
//!     intensity: Some(Intensity::Bold),
//!     ..Default::default()
//! });
//! text.concat("File not found");
//! ```
//!
//! ## SegmentedString
//!
//! [`SegmentedString`] stores raw segments (text, control codes, ANSI sequences) explicitly,
//! perfect for:
//! - Building terminal output incrementally
//! - Precise control over ANSI sequence placement
//! - Preserving the exact structure of terminal commands
//! - Converting between different terminal representations
//!
//! ```rust
//! use termionix_ansicodes::{SegmentedString, Style, Color, ControlCode};
//!
//! let mut segmented = SegmentedString::empty();
//! segmented.push_style(Style {
//!     foreground: Some(Color::Green),
//!     ..Default::default()
//! });
//! segmented.push_str("Success");
//! segmented.push_control(ControlCode::LF);
//! ```
//!
//! ## SpannedString
//!
//! [`SpannedString`] parses ANSI strings and returns byte ranges for each segment,
//! useful for:
//! - Analyzing existing ANSI-formatted text
//! - Extracting specific segments from terminal output
//! - Understanding the structure of ANSI sequences
//! - Converting parsed data to other representations
//!
//! ```rust
//! use termionix_ansicodes::SpannedString;
//!
//! let input = "\x1b[31mRed text\x1b[0m";
//! let spanned = SpannedString::parse(input);
//! println!("Found {} segments", spanned.count());
//! ```
//!
//! # Style and Color System
//!
//! The [`Style`] struct provides comprehensive text styling options:
//!
//! ```rust
//! use termionix_ansicodes::{Style, Color, Intensity, Underline, Blink, Ideogram, Font, Script};
//! use termionix_ansicodes::Color::Cyan;
//!
//! let style = Style {
//!     foreground: Some(Color::RGB(255, 100, 50)),
//!     background: Some(Color::Fixed(0)), // Black
//!     intensity: Some(Intensity::Normal),
//!     underline: Some(Underline::Single),
//!     ideogram: Some(Ideogram::NoIdeogramAttributes),
//!     reverse: Some(false),
//!     font: Some(Font::PrimaryFont),
//!     italic: Some(true),
//!     script: Some(Script::Normal),
//!     blink: Some(Blink::Slow),
//!     hidden: Some(true),
//!     strike: None,
//!     .. Default::default()
//! };
//! ```
//!
//! Colors support multiple formats:
//! - **Named colors** - Standard 16-color palette (0-15)
//! - **256-color palette** - Extended colors (0-255)
//! - **RGB colors** - True color (24-bit)
//!
//! # Control Codes and Commands
//!
//! The library provides complete support for ISO 6429 control codes and CSI commands:
//!
//! ```rust
//! use termionix_ansicodes::{ControlCode, CSICommand, EraseInDisplayMode};
//!
//! // C0 control codes
//! let newline = ControlCode::LF;
//! let bell = ControlCode::BEL;
//!
//! // CSI commands
//! let move_up = CSICommand::CursorUp(5);
//! let clear_screen = CSICommand::EraseInDisplay(EraseInDisplayMode::EraseEntireScreen);
//! ```
//!
//! # Configuration
//!
//! Use [`AnsiConfig`] to control parsing and generation behavior:
//!
//! ```rust
//! use termionix_ansicodes::{AnsiConfig, ColorMode};
//!
//! let config = AnsiConfig::default();
//! // Configure which ANSI features are enabled
//! ```
//!
//! # Feature Highlights
//!
//! - **Zero-copy parsing** - [`SpannedString`] returns byte ranges without copying
//! - **Automatic segment merging** - [`SegmentedString`] intelligently combines compatible segments
//! - **Type-safe ANSI generation** - Compile-time guarantees for valid sequences
//! - **Comprehensive control code support** - Full ISO 6429 C0/C1 coverage
//! - **Multiple color formats** - Named, 256-color, and RGB support
//! - **Flexible styling** - Apply styles to character ranges or entire strings
//!
//! # Examples
//!
//! Building a colorful progress indicator:
//!
//! ```rust
//! use termionix_ansicodes::{StyledString, Style, Color};
//!
//! fn progress_bar(percent: u8) -> StyledString {
//!     let mut bar = StyledString::empty();
//!     let filled = (percent as usize * 20) / 100;
//!
//!     bar.concat_with_style(&"█".repeat(filled), Style {
//!         foreground: Some(Color::Green),
//!         ..Default::default()
//!     });
//!
//!     bar.concat_with_style(&"░".repeat(20 - filled), Style {
//!         foreground: Some(Color::BrightBlack),
//!         ..Default::default()
//!     });
//!
//!     bar.concat(&format!(" {}%", percent));
//!     bar
//! }
//! ```
//!
//! Parsing and analyzing terminal output:
//!
//! ```rust
//! use termionix_ansicodes::{SpannedString, Span};
//!
//! let output = "\x1b[1;32mSuccess\x1b[0m: Operation completed";
//! let spanned = SpannedString::parse(output);
//!
//! for span in spanned.iter() {
//!     match span {
//!         Span::CSI { .. } => println!("Found CSI sequence"),
//!         Span::ASCII { range } => println!("Text: {}", &output[range.clone()]),
//!         _ => {}
//!     }
//! }
//! ```

#![warn(
    clippy::cargo,
    missing_docs,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![allow(
    clippy::option_if_let_else,
    clippy::module_name_repetitions,
    clippy::missing_errors_doc
)]

mod ansi;
mod config;
mod mapper;
mod segment;
mod spanned;
mod string;
mod style;
mod utility;

pub use self::ansi::{CSICommand, ControlCode, EraseInDisplayMode, EraseInLineMode};
pub use self::config::AnsiConfig;
pub use self::mapper::{AnsiMapper, AnsiMapperResult};
pub use self::segment::{Segment, SegmentedString};
pub use self::spanned::{Span, SpannedString};
pub use self::string::StyledString;
pub use self::style::{
    Blink, Color, ColorMode, Font, Ideogram, Intensity, SGRParameter, Script, Style, Underline,
};
pub use self::utility::strip_ansi_codes;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
