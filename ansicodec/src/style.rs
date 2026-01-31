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

use crate::{AnsiResult, ColorMode};
use bytes::BufMut;

/// Represents a text style with various formatting attributes and colors for terminal output.
///
/// `Style` encapsulates all ANSI terminal styling properties, including text attributes
/// (bold, italic, underline, etc.) and foreground/background colors. It provides methods
/// to write ANSI escape sequences to apply these styles in terminal environments.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```rust
/// use termionix_ansicodec::{Style, Color, ColorMode, Intensity, AnsiConfig};
/// use std::fmt::Write;
///
/// let config = AnsiConfig::enabled();
/// let mut style = Style::default();
/// style.intensity = Some(Intensity::Bold);
/// style.foreground = Some(Color::Red);
///
/// let mut output = String::new();
/// style.write_style(&mut output, Some(&config)).unwrap();
/// write!(&mut output, "Bold Red Text").unwrap();
/// Style::write_reset(&mut output, Some(&config)).unwrap();
/// ```
///
/// ## Complex Styling
///
/// ```rust
/// use termionix_ansicodec::{Underline,Style, Color, ColorMode, Intensity};
///
/// let style = Style {
///     intensity: Some(Intensity::Bold),
///     italic: Some(true),
///     underline: Some(Underline::Single),
///     foreground: Some(Color::RGB(255, 100, 50)),
///     background: Some(Color::Blue),
///     ..Default::default()
/// };
/// ```
///
/// # ANSI Escape Codes
///
/// The `Style` struct generates ANSI escape sequences in the format `\x1b[<codes>m`,
/// where `<codes>` is a semicolon-separated list of numeric codes:
///
/// - Text attributes: `1` (bold), `2` (dim), `3` (italic), `4` (underline),
///   `5` (blink), `7` (reverse), `8` (hidden), `9` (strikethrough)
/// - Foreground colors: `30-37` (basic), `38;5;<n>` (256-color), `38;2;<r>;<g>;<b>` (RGB)
/// - Background colors: `40-47` (basic), `48;5;<n>` (256-color), `48;2;<r>;<g>;<b>` (RGB)
/// - Reset: `0` (clear all styles)
///
/// # Color Mode Support
///
/// Style rendering respects the `ColorMode` setting:
/// - `ColorMode::None`: No ANSI codes are written
/// - `ColorMode::Basic`: Basic 8-color support (codes 30-37, 40-47)
/// - `ColorMode::FixedColor`: 256-color support
/// - `ColorMode::TrueColor`: Full 24-bit RGB color support
///
/// # Default Values
///
/// The default style has no formatting attributes enabled and no colors set:
///
/// ```rust
/// use termionix_ansicodec::{Color, Style};
///
/// let style = Style::default();
/// assert_eq!(style.intensity, None);
/// assert_eq!(style.italic, None);
/// assert_eq!(style.foreground, None);
/// assert_eq!(style.background, None);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct AnsiSelectGraphicRendition {
    /// Intensity of Text
    ///
    /// | Code | Description |
    /// |------|-------------|
    /// | `1`  | Bold        |
    /// | `2`  | Dim         |
    /// | `22` | Normal      |
    ///
    /// See: [`Intensity`]
    pub intensity: Option<Intensity>,
    /// Whether this style is italic or oblique.
    ///
    /// | Code | Description |
    /// |------|-------------|
    /// | `3`  | Italic      |
    /// | `23` | Oblique     |
    pub italic: Option<bool>,
    /// Whether this style is underlined. (Code: `4`)
    ///
    /// | Code | Description      |
    /// |------|------------------|
    /// | `4`  | Underline        |
    /// | `21` | Double Underline |
    /// | `24` | No Underline     |
    ///
    /// See: [`Intensity`]
    pub underline: Option<Underline>,
    /// Whether this style is blinking.
    ///
    /// | Code | Description |
    /// |------|-------------|
    /// | `5`  | Blink       |
    /// | `6`  | Rapid Blink |
    /// | `25` | No Blink    |
    pub blink: Option<Blink>,
    /// Whether this style has reverse colors.
    ///
    /// | Code | Description     |
    /// |------|-----------------|
    /// | `7`  | Reverse/Inverse |
    /// | `27` | Normal          |
    pub reverse: Option<bool>,
    /// Whether this style is hidden.
    ///
    /// | Code | Description      |
    /// |------|------------------|
    /// | `8`  | Hidden/Concealed |
    /// | `28` | Normal/Revealed  |
    pub hidden: Option<bool>,
    /// Whether this style is struckthrough.
    ///
    /// | Code | Description   |
    /// |------|---------------|
    /// | `9`  | Strikethrough |
    /// | `29` | Normal        |
    pub strike: Option<bool>,
    /// Weather this style is super or subscript.
    ///
    /// | Code | Description   |
    /// |------|---------------|
    /// | `73` | Superscript   |
    /// | `74` | Subscript     |
    /// | `75` | Normal        |
    pub script: Option<Script>,
    /// Weather this style is super or subscript.
    ///
    /// | Code | Description          |
    /// |------|----------------------|
    /// | `60` | Underline            |
    /// | `61` | DoubleUnderline      |
    /// | `62` | Overline             |
    /// | `63` | DoubleOverline       |
    /// | `64` | StressMarking        |
    /// | `65` | NoIdeogramAttributes |
    pub ideogram: Option<Ideogram>,
    /// The font to use for the text.
    ///
    /// | Code | Description      |
    /// |------|------------------|
    /// | `10` | Primary Font     |
    /// | `11` | Alternate Font 1 |
    /// | `12` | Alternate Font 2 |
    /// | `13` | Alternate Font 3 |
    /// | `14` | Alternate Font 4 |
    /// | `15` | Alternate Font 5 |
    /// | `16` | Alternate Font 6 |
    /// | `17` | Alternate Font 7 |
    /// | `18` | Alternate Font 8 |
    /// | `19` | Alternate Font 9 |
    /// | `20` | Fraktur Font     |
    pub font: Option<Font>,
    /// The foreground color of the text.
    ///
    /// | Code         | Description    |
    /// |--------------|----------------|
    /// | `30`         | Black          |
    /// | `31`         | Red            |
    /// | `32`         | Green          |
    /// | `33`         | Yellow         |
    /// | `34`         | Blue           |
    /// | `35`         | Magenta        |
    /// | `36`         | Cyan           |
    /// | `37`         | White          |
    /// | `38;5;n`     | 8-bit Fixed    |
    /// | `38;2;r;g;b` | 24-bit RGB     |
    /// | `90`         | Bright Black   |
    /// | `91`         | Bright Red     |
    /// | `92`         | Bright Green   |
    /// | `93`         | Bright Yellow  |
    /// | `94`         | Bright Blue    |
    /// | `95`         | Bright Magenta |
    /// | `96`         | Bright Cyan    |
    /// | `97`         | Bright White   |
    pub foreground: Option<Color>,
    /// The background color of the text.
    ///
    /// | Code  | Description    |
    /// |-------|----------------|
    /// | Code         | Description    |
    /// |--------------|----------------|
    /// | `40`         | Black          |
    /// | `41`         | Red            |
    /// | `42`         | Green          |
    /// | `43`         | Yellow         |
    /// | `44`         | Blue           |
    /// | `45`         | Magenta        |
    /// | `46`         | Cyan           |
    /// | `47`         | White          |
    /// | `48;5;n`     | 8 bit Fixed    |
    /// | `48;2;r;g;b` | 24-bit RGB     |
    /// | `100`        | Bright Black   |
    /// | `101`        | Bright Red     |
    /// | `102`        | Bright Green   |
    /// | `103`        | Bright Yellow  |
    /// | `104`        | Bright Blue    |
    /// | `105`        | Bright Magenta |
    /// | `106`        | Bright Cyan    |
    /// | `107`        | Bright White   |
    pub background: Option<Color>,
    /// Remaining SGR Bytes
    pub unknown: Vec<SGRParameter>,
}

impl AnsiSelectGraphicRendition {
    /// Length of Style Control
    /// TODO: Use color_mode parameter
    pub fn len(&self, _color_mode: Option<ColorMode>) -> usize {
        let mut length = 0;
        let mut code_count = 0;

        // Count Intensity (Bold `1` or Dim/Faint `2` or Normal `22`)
        match self.intensity {
            Some(intensity) => match intensity {
                Intensity::Bold | Intensity::Dim => {
                    code_count += 1;
                    length += 1;
                }
                Intensity::Normal => {}
            },
            None => {}
        }

        // Count Italic (Enabled `3` or Disabled `23`)
        match self.italic {
            Some(italic) => match italic {
                true => {
                    code_count += 1;
                    length += 1;
                }
                false => {
                    code_count += 1;
                    length += 2;
                }
            },
            None => {}
        }

        // Count Underline (Single `4`, Double `21`, or Disabled `24`)
        match self.underline {
            Some(underline) => match underline {
                Underline::Single => {
                    code_count += 1;
                    length += 1;
                }
                Underline::Double => {
                    code_count += 1;
                    length += 2;
                }
                Underline::Disabled => {
                    code_count += 1;
                    length += 2;
                }
            },
            None => {}
        }

        // Count Blink (slow, rapid, or reset to none)
        match self.blink {
            Some(blink) => match blink {
                Blink::Slow => {
                    code_count += 1;
                    length += 1;
                }
                Blink::Rapid => {
                    code_count += 1;
                    length += 1;
                }
                Blink::Off => {
                    code_count += 1;
                    length += 2;
                }
            },
            None => {}
        }

        // Count Reverse (Enabled `7` or Disabled `27`)
        match self.reverse {
            Some(reverse) => match reverse {
                true => {
                    code_count += 1;
                    length += 1;
                }
                false => {
                    code_count += 1;
                    length += 2;
                }
            },
            None => {}
        }

        // Count Hidden (Enabled `8` or Disabled `28`)
        match self.hidden {
            Some(hidden) => match hidden {
                true => {
                    code_count += 1;
                    length += 1;
                }
                false => {
                    code_count += 1;
                    length += 2;
                }
            },
            None => {}
        }

        // Count Strike (Enabled `9` or Disabled `29`)
        match self.hidden {
            Some(hidden) => match hidden {
                true => {
                    code_count += 1;
                    length += 1;
                }
                false => {
                    code_count += 1;
                    length += 2;
                }
            },
            None => {}
        }

        // Count font
        if self.font.is_some() {
            code_count += 1;
            length += 2; // "10"-"20" (all are 2 digits)
        }

        // Calculate foreground color codes
        if let Some(fg) = &self.foreground {
            match fg {
                Color::Black
                | Color::Red
                | Color::Green
                | Color::Yellow
                | Color::Blue
                | Color::Purple
                | Color::Cyan
                | Color::White => {
                    code_count += 1;
                    length += 2; // "30"-"37"
                }
                Color::BrightBlack
                | Color::BrightRed
                | Color::BrightGreen
                | Color::BrightYellow
                | Color::BrightBlue
                | Color::BrightPurple
                | Color::BrightCyan
                | Color::BrightWhite => {
                    code_count += 1;
                    length += 2; // "90"-"97"
                }
                Color::Fixed(n) => {
                    code_count += 3; // "38", "5", and the number
                    length += 2 + 1 + n.to_string().len(); // "38" + "5" + number
                }
                Color::RGB(r, g, b) => {
                    code_count += 5; // "38", "2", r, g, b
                    length +=
                        2 + 1 + r.to_string().len() + g.to_string().len() + b.to_string().len(); // "38" + "2" + r + g + b
                }
            }
        }

        // Calculate background color codes
        if let Some(bg) = &self.background {
            match bg {
                Color::Black
                | Color::Red
                | Color::Green
                | Color::Yellow
                | Color::Blue
                | Color::Purple
                | Color::Cyan
                | Color::White => {
                    code_count += 1;
                    length += 2; // "40"-"47"
                }
                Color::BrightBlack
                | Color::BrightRed
                | Color::BrightGreen
                | Color::BrightYellow
                | Color::BrightBlue
                | Color::BrightPurple
                | Color::BrightCyan
                | Color::BrightWhite => {
                    code_count += 1;
                    length += 3; // "100"-"107"
                }
                Color::Fixed(n) => {
                    code_count += 3; // "48", "5", and the number
                    length += 2 + 1 + n.to_string().len(); // "48" + "5" + number
                }
                Color::RGB(r, g, b) => {
                    code_count += 5; // "48", "2", r, g, b
                    length +=
                        2 + 1 + r.to_string().len() + g.to_string().len() + b.to_string().len(); // "48" + "2" + r + g + b
                }
            }
        }

        // Add unknown SGR bytes
        for byte in &self.unknown {
            code_count += 1;
            length += byte.to_u8().to_string().len();
        }

        if code_count == 0 {
            return 0; // No style codes, no escape sequence
        }

        // Calculate total length:
        // - "\x1b[" = 2 bytes
        // - "m" = 1 byte
        // - Semicolons between codes = (code_count - 1) bytes
        // - Plus the actual length of all codes calculated above

        let semicolons = if code_count > 1 { code_count - 1 } else { 0 };

        2 + length + semicolons + 1
    }

    pub fn encode<T: BufMut>(
        &self,
        dst: &mut T,
        color_mode: Option<ColorMode>,
    ) -> AnsiResult<usize> {
        Ok(self.write(&mut dst.writer(), color_mode)?)
    }

    ///
    /// Resets terminal color attributes by writing an ANSI reset escape code (`\x1b[0m`) to the provided writer.
    ///
    /// # Parameters
    /// - `mode`: A reference to a `ColorMode` object that determines if ANSI color codes should be used.
    /// - `writer`: A mutable reference to a writer implementing the `std::fmt::Write` trait,
    ///   where the reset escape code will be written if applicable.
    ///
    /// # Returns
    /// - `std::fmt::Result`: Returns `Ok(())` if successful, or an error if writing to the writer fails.
    ///
    /// # Behavior
    /// - If the provided `ColorMode` does not support ANSI (i.e., `mode.is_ansi()` is `false`),
    ///   the function does nothing and exits early with `Ok(())`.
    /// - If ANSI color is enabled, the function writes the ANSI reset escape code (`\x1b[0m`)
    ///   to the given writer.
    ///
    /// # Example
    ///
    /// ```rust
    /// use termionix_ansicodec::{AnsiConfig, ColorMode, Style};
    /// use std::fmt::Write;
    /// use std::io::BufWriter;
    ///
    /// let config = AnsiConfig::enabled();
    /// let mut output = String::new();
    ///
    /// Style::write_reset(&mut output, Some(&config)).unwrap();
    /// assert_eq!(output, "\x1b[0m"); // Check that the ANSI reset code is written
    ///
    /// ```
    pub fn write_reset<W: std::fmt::Write>(writer: &mut W) -> std::fmt::Result {
        write!(writer, "\x1b[0m")
    }

    /// Writes ANSI style escape codes to the given writer based on the style attributes
    /// and colors defined in the instance. This method is used to apply text formatting
    /// when ANSI color output is enabled.
    ///
    /// # Parameters
    ///
    /// - `mode`: A reference to a `ColorMode` instance that determines if ANSI colors
    ///   are supported. If ANSI is not supported, no styling is applied, and the function
    ///   returns early.
    /// - `writer`: A mutable reference to a type that implements the `std::fmt::Write`
    ///   trait, where the ANSI escape sequences will be written.
    ///
    /// # Returns
    ///
    /// - `std::fmt::Result`: Returns `Ok(())` if the escape codes are successfully written
    ///   or if no styling/formatting is needed. Returns an error if writing to the writer fails.
    ///
    /// # Behavior
    ///
    /// - When ANSI coloring is disabled (e.g., `ColorMode::is_ansi` is `false`), the method
    ///   returns immediately without writing any styles.
    /// - For each enabled text attribute (e.g., bold, italic, underline, etc.), the corresponding
    ///   escape code is added to a sequence of codes.
    /// - If foreground or background colors are specified, they are included in the ANSI escape
    ///   sequence. Supports standard, fixed 256-color, and RGB color codes.
    /// - If no attributes or colors are specified, no output is written, and the function exits
    ///   normally.
    ///
    /// # Example
    ///
    /// ```rust
    /// use termionix_ansicodec::{Style, Color, ColorMode, Intensity, Underline, Blink, AnsiConfig};
    /// use std::fmt::Write;
    ///
    /// let config = AnsiConfig::enabled();
    /// let style = Style {
    ///     intensity: Some(Intensity::Bold),
    ///     italic: None,
    ///     underline: Some(Underline::Single),
    ///     blink: Some(Blink::Off),
    ///     reverse: None,
    ///     hidden: None,
    ///     strike: None,
    ///     font: None,
    ///     script: None,
    ///     ideogram: None,
    ///     foreground: Some(Color::Red),
    ///     background: Some(Color::RGB(10, 20, 30)),
    ///     unknown: Vec::new(),
    /// };
    ///
    /// let mut output = String::new();
    /// style.write_style(&mut output, Some(&config)).unwrap();
    /// assert_eq!(output, "\x1b[1;4;25;31;48;2;10;20;30m");
    /// ```
    ///
    /// # Notes
    ///
    /// - This method assumes an implementation of a `Style` struct with text attributes
    ///   such as bold or underline, as well as `foreground` and `background` fields
    ///   for colors.
    /// - The `Color` enum is expected to support standard colors, fixed 256-color values,
    ///   and custom RGB values.
    /// ```
    pub fn write<W: std::io::Write>(
        &self,
        writer: &mut W,
        color_mode: Option<ColorMode>,
    ) -> std::io::Result<usize> {
        let codes = self.codes(color_mode);

        if !codes.is_empty() {
            write!(writer, "\x1b[{}m", codes.join(";"))?;
            Ok(0)
        } else {
            Ok(0)
        }
    }

    pub fn write_str<W: std::fmt::Write>(
        &self,
        writer: &mut W,
        color_mode: Option<ColorMode>,
    ) -> std::fmt::Result {
        let codes = self.codes(color_mode);

        if !codes.is_empty() {
            write!(writer, "\x1b[{}m", codes.join(";"))?;
            Ok(())
        } else {
            Ok(())
        }
    }

    /// TODO: Use color_mode
    fn codes(&self, color_mode: Option<ColorMode>) -> Vec<String> {
        let mut codes = Vec::new();

        // Write Intensity (Bold `1` or Dim/Faint `2` or Normal `22`)
        match self.intensity {
            Some(intensity) => match intensity {
                Intensity::Bold => codes.push("1".to_string()),
                Intensity::Dim => codes.push("2".to_string()),
                Intensity::Normal => codes.push("22".to_string()),
            },
            None => {}
        }

        // Write Italic (Enabled `3` or Disabled `23`)
        match self.italic {
            Some(reverse) => match reverse {
                true => codes.push("3".to_string()),
                false => codes.push("23".to_string()),
            },
            None => {}
        }

        // Write Underline (Single `4`,  Double `21`, or Disabled `24`)
        match self.underline {
            Some(underline) => match underline {
                Underline::Single => codes.push("4".to_string()),
                Underline::Double => codes.push("21".to_string()),
                Underline::Disabled => codes.push("24".to_string()),
            },
            None => {}
        }

        // Write Blink (Slow `5`, Rapid `6`, or Off `25`)
        match self.blink {
            Some(blink) => match blink {
                Blink::Slow => codes.push("5".to_string()),
                Blink::Rapid => codes.push("6".to_string()),
                Blink::Off => codes.push("25".to_string()),
            },
            None => {}
        }

        // Write Reverse (Enabled `7` or Disabled `27`)
        match self.reverse {
            Some(reverse) => match reverse {
                true => codes.push("7".to_string()),
                false => codes.push("27".to_string()),
            },
            None => {}
        }

        // Write Hidden (Enabled `8` or Disabled `28`)
        match self.hidden {
            Some(hidden) => match hidden {
                true => codes.push("8".to_string()),
                false => codes.push("28".to_string()),
            },
            None => {}
        }

        // Write Strike (Enabled `9` or Disabled `29`)
        match self.strike {
            Some(hidden) => match hidden {
                true => codes.push("9".to_string()),
                false => codes.push("29".to_string()),
            },
            None => {}
        }

        // Write Font (`10` - `20`)
        if let Some(font) = &self.font {
            codes.push(font.to_u8().to_string());
        }

        // Write Foreground color
        if let Some(fg) = &self.foreground {
            match fg {
                Color::Black => codes.push("30".to_string()),
                Color::Red => codes.push("31".to_string()),
                Color::Green => codes.push("32".to_string()),
                Color::Yellow => codes.push("33".to_string()),
                Color::Blue => codes.push("34".to_string()),
                Color::Purple => codes.push("35".to_string()),
                Color::Cyan => codes.push("36".to_string()),
                Color::White => codes.push("37".to_string()),
                Color::BrightBlack => codes.push("90".to_string()),
                Color::BrightRed => codes.push("91".to_string()),
                Color::BrightGreen => codes.push("92".to_string()),
                Color::BrightYellow => codes.push("93".to_string()),
                Color::BrightBlue => codes.push("94".to_string()),
                Color::BrightPurple => codes.push("95".to_string()),
                Color::BrightCyan => codes.push("96".to_string()),
                Color::BrightWhite => codes.push("97".to_string()),
                Color::Fixed(n) => {
                    codes.push("38".to_string());
                    codes.push("5".to_string());
                    codes.push(n.to_string());
                }
                Color::RGB(r, g, b) => {
                    codes.push("38".to_string());
                    codes.push("2".to_string());
                    codes.push(r.to_string());
                    codes.push(g.to_string());
                    codes.push(b.to_string());
                }
            }
        }

        // Write Background color
        if let Some(bg) = &self.background {
            match bg {
                Color::Black => codes.push("40".to_string()),
                Color::Red => codes.push("41".to_string()),
                Color::Green => codes.push("42".to_string()),
                Color::Yellow => codes.push("43".to_string()),
                Color::Blue => codes.push("44".to_string()),
                Color::Purple => codes.push("45".to_string()),
                Color::Cyan => codes.push("46".to_string()),
                Color::White => codes.push("47".to_string()),
                Color::BrightBlack => codes.push("100".to_string()),
                Color::BrightRed => codes.push("101".to_string()),
                Color::BrightGreen => codes.push("102".to_string()),
                Color::BrightYellow => codes.push("103".to_string()),
                Color::BrightBlue => codes.push("104".to_string()),
                Color::BrightPurple => codes.push("105".to_string()),
                Color::BrightCyan => codes.push("106".to_string()),
                Color::BrightWhite => codes.push("107".to_string()),
                Color::Fixed(n) => {
                    codes.push("48".to_string());
                    codes.push("5".to_string());
                    codes.push(n.to_string());
                }
                Color::RGB(r, g, b) => {
                    codes.push("48".to_string());
                    codes.push("2".to_string());
                    codes.push(r.to_string());
                    codes.push(g.to_string());
                    codes.push(b.to_string());
                }
            }
        }

        // Unknown SGR bytes
        for byte in &self.unknown {
            codes.push(byte.to_u8().to_string());
        }

        codes
    }

    /// Parses SGR (Select Graphic Rendition) parameters into a Style struct.
    ///
    /// This function takes a slice of SGR parameter codes (the numeric values between
    /// `ESC[` and `m` in ANSI escape sequences) and converts them into a `Style` struct
    /// with the appropriate formatting attributes and colors.
    ///
    /// # Arguments
    ///
    /// * `params` - A slice of u8 values representing SGR codes (e.g., `[1, 31]` for bold red)
    ///
    /// # Returns
    ///
    /// A `Style` struct with the parsed attributes applied. If the input is empty or
    /// contains only a reset code (0), returns a default style.
    ///
    /// # SGR Code Support
    ///
    /// ## Text Attributes
    /// - `0` - Reset all attributes to default
    /// - `1` - Bold
    /// - `2` - Dim
    /// - `3` - Italic
    /// - `4` - Underline
    /// - `5` - Slow blink
    /// - `6` - Rapid blink
    /// - `7` - Reverse video
    /// - `8` - Hidden/concealed
    /// - `9` - Strikethrough
    /// - `21` - Double underline
    /// - `22` - Normal intensity (neither bold nor dim)
    /// - `23` - Not italic
    /// - `24` - Not underlined
    /// - `25` - Not blinking
    /// - `27` - Not reversed
    /// - `28` - Not hidden
    /// - `29` - Not strikethrough
    ///
    /// ## Foreground Colors (Basic)
    /// - `30-37` - Black, Red, Green, Yellow, Blue, Purple, Cyan, White
    /// - `39` - Default foreground color
    /// - `90-97` - Bright variants of basic colors
    ///
    /// ## Background Colors (Basic)
    /// - `40-47` - Black, Red, Green, Yellow, Blue, Purple, Cyan, White
    /// - `49` - Default background color
    /// - `100-107` - Bright variants of basic colors
    ///
    /// ## Extended Colors
    /// - `38;5;n` - Set foreground to 256-color palette color n
    /// - `38;2;r;g;b` - Set foreground to RGB color
    /// - `48;5;n` - Set background to 256-color palette color n
    /// - `48;2;r;g;b` - Set background to RGB color
    ///
    /// ## Fonts
    /// - `10-19` - Select font (primary or alternate fonts 1-9)
    /// - `20` - Fraktur font
    ///
    /// # Notes
    ///
    /// - Unknown or unsupported SGR codes are collected in `style.unknown`
    /// - Extended color sequences (38/48 with 5 or 2) consume multiple parameters
    /// - If extended color sequences are incomplete, the codes are stored as unknown
    /// - Reset code (0) clears all attributes and returns a default style
    pub fn parse(params: &[u8]) -> AnsiSelectGraphicRendition {
        let mut style = AnsiSelectGraphicRendition::default();
        let mut i = 0;

        while i < params.len() {
            match params[i] {
                // Reset
                0 => {
                    style = AnsiSelectGraphicRendition::default();
                }

                // Intensity
                1 => style.intensity = Some(Intensity::Bold),
                2 => style.intensity = Some(Intensity::Dim),
                22 => style.intensity = Some(Intensity::Normal),

                // Italic
                3 => style.italic = Some(true),
                23 => style.italic = Some(false),

                // Underline
                4 => style.underline = Some(Underline::Single),
                21 => style.underline = Some(Underline::Double),
                24 => style.underline = Some(Underline::Disabled),

                // Blink
                5 => style.blink = Some(Blink::Slow),
                6 => style.blink = Some(Blink::Rapid),
                25 => style.blink = Some(Blink::Off),

                // Reverse
                7 => style.reverse = Some(true),
                27 => style.reverse = Some(false),

                // Hidden
                8 => style.hidden = Some(true),
                28 => style.hidden = Some(false),

                // Strike
                9 => style.strike = Some(true),
                29 => style.strike = Some(false),

                // Fonts
                10 => style.font = Some(Font::PrimaryFont),
                11 => style.font = Some(Font::AlternateFont1),
                12 => style.font = Some(Font::AlternateFont2),
                13 => style.font = Some(Font::AlternateFont3),
                14 => style.font = Some(Font::AlternateFont4),
                15 => style.font = Some(Font::AlternateFont5),
                16 => style.font = Some(Font::AlternateFont6),
                17 => style.font = Some(Font::AlternateFont7),
                18 => style.font = Some(Font::AlternateFont8),
                19 => style.font = Some(Font::AlternateFont9),
                20 => style.font = Some(Font::Fraktur),

                // Foreground colors (basic)
                30 => style.foreground = Some(Color::Black),
                31 => style.foreground = Some(Color::Red),
                32 => style.foreground = Some(Color::Green),
                33 => style.foreground = Some(Color::Yellow),
                34 => style.foreground = Some(Color::Blue),
                35 => style.foreground = Some(Color::Purple),
                36 => style.foreground = Some(Color::Cyan),
                37 => style.foreground = Some(Color::White),
                39 => style.foreground = None, // Default foreground

                // Background colors (basic)
                40 => style.background = Some(Color::Black),
                41 => style.background = Some(Color::Red),
                42 => style.background = Some(Color::Green),
                43 => style.background = Some(Color::Yellow),
                44 => style.background = Some(Color::Blue),
                45 => style.background = Some(Color::Purple),
                46 => style.background = Some(Color::Cyan),
                47 => style.background = Some(Color::White),
                49 => style.background = None, // Default background

                // Bright foreground colors
                90 => style.foreground = Some(Color::Black),
                91 => style.foreground = Some(Color::Red),
                92 => style.foreground = Some(Color::Green),
                93 => style.foreground = Some(Color::Yellow),
                94 => style.foreground = Some(Color::Blue),
                95 => style.foreground = Some(Color::Purple),
                96 => style.foreground = Some(Color::Cyan),
                97 => style.foreground = Some(Color::White),

                // Bright background colors
                100 => style.background = Some(Color::Black),
                101 => style.background = Some(Color::Red),
                102 => style.background = Some(Color::Green),
                103 => style.background = Some(Color::Yellow),
                104 => style.background = Some(Color::Blue),
                105 => style.background = Some(Color::Purple),
                106 => style.background = Some(Color::Cyan),
                107 => style.background = Some(Color::White),

                // Extended foreground color
                38 => {
                    if i + 2 < params.len() && params[i + 1] == 5 {
                        // 256-color: 38;5;n
                        style.foreground = Some(Color::Fixed(params[i + 2]));
                        i += 2;
                    } else if i + 4 < params.len() && params[i + 1] == 2 {
                        // RGB: 38;2;r;g;b
                        style.foreground =
                            Some(Color::RGB(params[i + 2], params[i + 3], params[i + 4]));
                        i += 4;
                    } else {
                        // Incomplete sequence, store as unknown
                        style.unknown.push(SGRParameter::Unknown(params[i]));
                    }
                }

                // Extended background color
                48 => {
                    if i + 2 < params.len() && params[i + 1] == 5 {
                        // 256-color: 48;5;n
                        style.background = Some(Color::Fixed(params[i + 2]));
                        i += 2;
                    } else if i + 4 < params.len() && params[i + 1] == 2 {
                        // RGB: 48;2;r;g;b
                        style.background =
                            Some(Color::RGB(params[i + 2], params[i + 3], params[i + 4]));
                        i += 4;
                    } else {
                        // Incomplete sequence, store as unknown
                        style.unknown.push(SGRParameter::Unknown(params[i]));
                    }
                }

                // Unknown or unsupported codes
                _ => {
                    style.unknown.push(SGRParameter::Unknown(params[i]));
                }
            }

            i += 1;
        }

        style
    }
}

/// Represents the intensity (weight) of text in ANSI terminal styling.
///
/// Intensity controls whether text appears with normal weight, bold (increased intensity),
/// or dim (decreased intensity). This is one of the fundamental text styling attributes
/// in ANSI escape sequences.
///
/// # ANSI Codes
///
/// - `Normal`: SGR code 22 - Resets to normal intensity
/// - `Bold`: SGR code 1 - Increases intensity (often rendered as bold)
/// - `Dim`: SGR code 2 - Decreases intensity (often rendered as faint)
///
/// Note that `Bold` and `Dim` are mutually exclusive; setting one will override the other.
/// The `Normal` variant resets both bold and dim attributes.
///
/// # Examples
///
/// ```
/// use termionix_ansicodec::{Intensity, Style, StyledString, ColorMode};
///
/// // Create bold text
/// let mut styled = StyledString::empty();
/// styled.concat_with_style("Bold Text", Style {
///     intensity: Some(Intensity::Bold),
///     ..Default::default()
/// });
///
/// // Create dim text
/// let mut styled = StyledString::empty();
/// styled.concat_with_style("Dim Text", Style {
///     intensity: Some(Intensity::Dim),
///     ..Default::default()
/// });
/// ```
///
/// # Conversion
///
/// The `Intensity` enum can be converted to and from u8 values representing ANSI SGR codes:
///
/// ```
/// use termionix_ansicodes::Intensity;
///
/// // Convert to ANSI code
/// assert_eq!(Intensity::Bold.to_u8(), 1);
/// assert_eq!(Intensity::Dim.to_u8(), 2);
/// assert_eq!(Intensity::Normal.to_u8(), 22);
///
/// // Convert from ANSI code
/// assert_eq!(Intensity::from_u8(1), Some(Intensity::Bold));
/// assert_eq!(Intensity::from_u8(2), Some(Intensity::Dim));
/// assert_eq!(Intensity::from_u8(22), Some(Intensity::Normal));
/// assert_eq!(Intensity::from_u8(99), None); // Invalid code
/// ```
///
/// # Default
///
/// The default intensity is `Normal`, which represents standard text weight without
/// bold or dim effects.
#[derive(Clone, Copy, Debug, PartialOrd, PartialEq, Hash, Eq, Default)]
pub enum Intensity {
    /// Normal text intensity (standard weight).
    ///
    /// This is the default intensity level. It corresponds to ANSI SGR code 22,
    /// which explicitly resets bold and dim attributes to normal.
    #[default]
    Normal,

    /// Bold or increased intensity text.
    ///
    /// Corresponds to ANSI SGR code 1. Typically renders text with a heavier weight
    /// or brighter color, depending on the terminal implementation. Some terminals
    /// may also render bold text in a different color (e.g., bright variants of
    /// basic colors).
    Bold,

    /// Dim or decreased intensity text.
    ///
    /// Corresponds to ANSI SGR code 2. Typically renders text with a lighter weight
    /// or less saturated color, making it appear faint or subdued. This is commonly
    /// used for de-emphasized text like comments or secondary information.
    Dim,
}

impl Intensity {
    /// Converts the intensity variant to its corresponding ANSI SGR code.
    ///
    /// This method returns the numeric value used in ANSI escape sequences to represent
    /// this intensity level. The mapping follows the ANSI/ECMA-48 standard:
    ///
    /// # Returns
    ///
    /// The ANSI SGR (Select Graphic Rendition) code as a `u8`:
    /// - `Bold` returns `1` - Sets bold or increased intensity
    /// - `Dim` returns `2` - Sets faint or decreased intensity
    /// - `Normal` returns `22` - Resets to normal intensity (neither bold nor dim)
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodes::Intensity;
    ///
    /// assert_eq!(Intensity::Bold.to_u8(), 1);
    /// assert_eq!(Intensity::Dim.to_u8(), 2);
    /// assert_eq!(Intensity::Normal.to_u8(), 22);
    /// ```
    ///
    /// # Use in ANSI Sequences
    ///
    /// The returned code can be used directly in ANSI escape sequences:
    ///
    /// ```
    /// use termionix_ansicodes::Intensity;
    ///
    /// let code = Intensity::Bold.to_u8();
    /// let ansi_sequence = format!("\x1b[{}m", code);
    /// assert_eq!(ansi_sequence, "\x1b[1m"); // Bold text
    /// ```
    pub fn to_u8(&self) -> u8 {
        match self {
            Intensity::Bold => 1,
            Intensity::Dim => 2,
            Intensity::Normal => 22,
        }
    }

    /// Converts an ANSI SGR code to its corresponding `Intensity` variant.
    ///
    /// This method attempts to parse a numeric ANSI SGR code into an `Intensity` value.
    /// It recognizes the standard codes for bold, dim, and normal intensity.
    ///
    /// # Arguments
    ///
    /// * `value` - The ANSI SGR code to convert
    ///
    /// # Returns
    ///
    /// - `Some(Intensity::Bold)` if `value` is `1`
    /// - `Some(Intensity::Dim)` if `value` is `2`
    /// - `Some(Intensity::Normal)` if `value` is `22`
    /// - `None` for any other value
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use termionix_ansicodes::Intensity;
    ///
    /// assert_eq!(Intensity::from_u8(1), Some(Intensity::Bold));
    /// assert_eq!(Intensity::from_u8(2), Some(Intensity::Dim));
    /// assert_eq!(Intensity::from_u8(22), Some(Intensity::Normal));
    /// ```
    ///
    /// Handling invalid codes:
    ///
    /// ```
    /// use termionix_ansicodes::Intensity;
    ///
    /// assert_eq!(Intensity::from_u8(99), None);
    /// assert_eq!(Intensity::from_u8(0), None);
    /// ```
    ///
    /// Parsing ANSI sequences:
    ///
    /// ```
    /// use termionix_ansicodes::Intensity;
    ///
    /// // Parse a code from an ANSI sequence parameter
    /// let code: u8 = 1; // From "\x1b[1m"
    /// match Intensity::from_u8(code) {
    ///     Some(intensity) => println!("Parsed intensity: {:?}", intensity),
    ///     None => println!("Unknown intensity code: {}", code),
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// - Code `21` (sometimes used for "doubly underlined or not bold") is not recognized
    ///   as it's not universally supported and conflicts with double underline in some terminals
    /// - Code `0` (reset all) is not handled by this method as it affects multiple attributes,
    ///   not just intensity
    pub fn from_u8(value: u8) -> Option<Intensity> {
        match value {
            1 => Some(Intensity::Bold),
            2 => Some(Intensity::Dim),
            22 => Some(Intensity::Normal),
            _ => None,
        }
    }
}

/// Represents the underline style for text in ANSI terminal output.
///
/// This enum defines the different underline formatting options available
/// through ANSI escape sequences. The underline can be disabled, single,
/// or double.
///
/// # ANSI Codes
///
/// - `NoUnderline`: SGR code 24 (disable underline)
/// - `Underline`: SGR code 4 (enable single underline)
/// - `DoubleUnderline`: SGR code 21 (enable double underline)
///
/// # Examples
///
/// ```rust
/// use termionix_ansicodes::{Underline, Style, StyledString, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// // Create a styled string with underline
/// let mut styled = StyledString::empty();
/// styled.concat_with_style("Underlined text", Style {
///     underline: Some(Underline::Single),
///     ..Default::default()
/// });
///
/// // Convert to ANSI string
/// let mut output = String::new();
/// styled.write_str(&mut output, Some(&config)).unwrap();
/// ```
///
/// # Default
///
/// The default value is [`Underline::Disabled`], which represents no underline formatting.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Underline {
    /// No underline formatting applied to text.
    ///
    /// This is the default state and corresponds to ANSI SGR code 24
    /// (reset/disable underline).
    #[default]
    Disabled,

    /// Single underline formatting applied to text.
    ///
    /// This corresponds to ANSI SGR code 4 and produces a single line
    /// beneath the text.
    Single,

    /// Double underline formatting applied to text.
    ///
    /// This corresponds to ANSI SGR code 21 and produces two lines
    /// beneath the text. Note that support for double underline may
    /// vary across different terminal emulators.
    Double,
}

impl Underline {
    /// Converts the underline variant to its corresponding ANSI SGR parameter code.
    ///
    /// This method returns the numeric code used in ANSI escape sequences to
    /// set or reset underline formatting.
    ///
    /// # Returns
    ///
    /// - `4` for [`Underline::Single`] (enable single underline)
    /// - `21` for [`Underline::Double`] (enable double underline)
    /// - `24` for [`Underline::Disabled`] (disable underline)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::Underline;
    ///
    /// assert_eq!(Underline::Single.to_u8(), 4);
    /// assert_eq!(Underline::Double.to_u8(), 21);
    /// assert_eq!(Underline::Disabled.to_u8(), 24);
    /// ```
    pub fn to_u8(&self) -> u8 {
        match self {
            Underline::Single => 4,
            Underline::Double => 21,
            Underline::Disabled => 24,
        }
    }

    /// Converts an ANSI SGR parameter code to the corresponding underline variant.
    ///
    /// This method is used for parsing ANSI escape sequences to determine the
    /// underline formatting state.
    ///
    /// # Arguments
    ///
    /// * `value` - The ANSI SGR parameter code to convert
    ///
    /// # Returns
    ///
    /// - `Some(Underline::Underline)` if `value` is `4`
    /// - `Some(Underline::DoubleUnderline)` if `value` is `21`
    /// - `Some(Underline::NoUnderline)` if `value` is `24`
    /// - `None` if the value doesn't correspond to any underline code
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::Underline;
    ///
    /// assert_eq!(Underline::from_u8(4), Some(Underline::Single));
    /// assert_eq!(Underline::from_u8(21), Some(Underline::Double));
    /// assert_eq!(Underline::from_u8(24), Some(Underline::Disabled));
    /// assert_eq!(Underline::from_u8(99), None);
    /// ```
    pub fn from_u8(value: u8) -> Option<Underline> {
        match value {
            4 => Some(Underline::Single),
            21 => Some(Underline::Double),
            24 => Some(Underline::Disabled),
            _ => None,
        }
    }
}

/// Represents a color that can be used in terminal output through ANSI escape sequences.
///
/// `Color` provides four levels of color specification, from basic 16-color support
/// to full 24-bit RGB true color. The enum variants correspond to different ANSI
/// color escape sequence formats that terminals support.
///
/// # Color Levels
///
/// - **Basic Colors** (16 colors): The 8 standard ANSI colors plus their bright variants
/// - **Fixed Palette** (256 colors): Extended 8-bit color palette via [`Fixed`](Color::Fixed)
/// - **True Color** (16.7M colors): 24-bit RGB color via [`RGB`](Color::RGB)
///
/// # Variants
///
/// ## Basic Colors (Standard 8)
///
/// The first 8 colors are the standard ANSI colors that have been supported since
/// the earliest terminals. These can typically be customized in terminal emulator
/// settings and correspond to foreground codes 30-37 and background codes 40-47.
///
/// - [`Black`](Color::Black) - Usually rendered as dark gray rather than true black
/// - [`Red`](Color::Red) - Standard red
/// - [`Green`](Color::Green) - Standard green
/// - [`Yellow`](Color::Yellow) - Standard yellow/orange
/// - [`Blue`](Color::Blue) - Standard blue
/// - [`Purple`](Color::Purple) - Magenta/Purple
/// - [`Cyan`](Color::Cyan) - Cyan/Aqua
/// - [`White`](Color::White) - Usually light gray rather than true white
///
/// ## Bright Colors (Bold/Intense 8)
///
/// The bright variants provide more vivid versions of the basic colors. These
/// correspond to the AIXterm specification (foreground codes 90-97, background
/// codes 100-107) and are widely supported in modern terminals.
///
/// - [`BrightBlack`](Color::BrightBlack) - Often used as a gray color
/// - [`BrightRed`](Color::BrightRed) - Vivid red
/// - [`BrightGreen`](Color::BrightGreen) - Vivid green
/// - [`BrightYellow`](Color::BrightYellow) - Vivid yellow
/// - [`BrightBlue`](Color::BrightBlue) - Vivid blue
/// - [`BrightPurple`](Color::BrightPurple) - Vivid magenta/purple
/// - [`BrightCyan`](Color::BrightCyan) - Vivid cyan
/// - [`BrightWhite`](Color::BrightWhite) - True white or near-white
///
/// ## Extended Colors
///
/// - [`Fixed`](Color::Fixed) - 256-color palette (8-bit color)
/// - [`RGB`](Color::RGB) - True color (24-bit RGB)
///
/// # Color Mode Compatibility
///
/// Colors are rendered differently depending on the [`ColorMode`] used:
///
/// - [`ColorMode::None`]: No colors are rendered
/// - [`ColorMode::Basic`]: Only the 16 basic colors are rendered
/// - [`ColorMode::FixedColor`]: 256-color palette is used
/// - [`ColorMode::TrueColor`]: Full RGB colors are used
///
/// The methods [`to_basic()`](Color::to_basic), [`to_fixed()`](Color::to_fixed),
/// and [`to_truecolor()`](Color::to_truecolor) can be used to convert between
/// these representations.
///
/// # Examples
///
/// Using basic colors:
///
/// ```
/// use termionix_ansicodes::{StyledString, Style, Color, ColorMode};
///
/// let styled = StyledString::from_string("Error", Some(Style {
///     foreground: Some(Color::Red),
///     ..Default::default()
/// }));
/// ```
///
/// Using bright colors:
///
/// ```
/// use termionix_ansicodes::{Color, Style};
///
/// let style = Style {
///     foreground: Some(Color::BrightRed),
///     background: Some(Color::BrightBlack),
///     ..Default::default()
/// };
/// ```
///
/// Using 256-color palette:
///
/// ```
/// use termionix_ansicodes::Color;
///
/// // Use color #196 from the 256-color palette (bright red)
/// let bright_red = Color::Fixed(196);
///
/// // Use color #234 from the 256-color palette (dark gray)
/// let dark_gray = Color::Fixed(234);
/// ```
///
/// Using true color (RGB):
///
/// ```
/// use termionix_ansicodes::Color;
///
/// // Create a custom orange color
/// let orange = Color::RGB(255, 165, 0);
///
/// // Create a custom teal color
/// let teal = Color::RGB(0, 128, 128);
/// ```
///
/// Converting between color modes:
///
/// ```
/// use termionix_ansicodes::Color;
///
/// let true_color = Color::RGB(255, 100, 50);
///
/// // Convert to 256-color palette
/// let fixed = true_color.to_fixed();
///
/// // Convert to basic 16-color palette
/// let basic = true_color.to_basic();
/// ```
///
/// # Color Palette Reference
///
/// ## 256-Color Palette Structure
///
/// The 256-color palette (used with [`Fixed`](Color::Fixed)) is structured as:
///
/// - **0-7**: Standard colors (matches basic [`Black`](Color::Black) through [`White`](Color::White))
/// - **8-15**: Bright colors (matches [`BrightBlack`](Color::BrightBlack) through [`BrightWhite`](Color::BrightWhite))
/// - **16-231**: 6×6×6 RGB cube (216 colors)
///   - Formula: `16 + 36×r + 6×g + b` where r, g, b ∈ [0, 5]
/// - **232-255**: Grayscale ramp (24 shades from dark to light)
///
/// See the [XTerm 256 Color Chart](https://upload.wikimedia.org/wikipedia/commons/1/15/Xterm_256color_chart.svg)
/// for a visual reference.
///
/// # ANSI Escape Sequences
///
/// When rendered to terminal output, colors generate different ANSI sequences:
///
/// ## Basic Colors
/// - Foreground: `\x1b[30m` through `\x1b[37m`
/// - Background: `\x1b[40m` through `\x1b[47m`
///
/// ## Bright Colors
/// - Foreground: `\x1b[90m` through `\x1b[97m`
/// - Background: `\x1b[100m` through `\x1b[107m`
///
/// ## 256-Color (Fixed)
/// - Foreground: `\x1b[38;5;<n>m` where n is 0-255
/// - Background: `\x1b[48;5;<n>m` where n is 0-255
///
/// ## True Color (RGB)
/// - Foreground: `\x1b[38;2;<r>;<g>;<b>m`
/// - Background: `\x1b[48;2;<r>;<g>;<b>m`
///
/// # Terminal Compatibility
///
/// Not all terminals support all color modes:
///
/// - **Basic colors**: Universally supported (since VT100 era)
/// - **Bright colors**: Widely supported (AIXterm standard)
/// - **256 colors**: Supported by most modern terminals
/// - **True color**: Requires `COLORTERM=truecolor` or `COLORTERM=24bit`
///
/// Use [`ColorMode`] detection at runtime to adapt to terminal capabilities.
///
/// # See Also
///
/// - [`ColorMode`] - Determines which color capabilities to use
/// - [`AnsiSelectGraphicRendition`] - Container for text styling including colors
/// - [`StyledString`](crate::StyledString) - For building styled text output
#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum Color {
    /// Black - Color #0 (FG `30`, BG `40`).
    Black,
    /// Red - Color #1 (FG `31`, BG `41`).
    Red,
    /// Green - Color #2 (FG `32`, BG `42`).
    Green,
    /// Yellow - Color #3 (FG `33`, BG `43`).
    Yellow,
    /// Blue - Color #4 (FG `34`, BG `44`).
    Blue,
    /// Purple - Color #5 (FG `35`, BG `45`).
    Purple,
    /// Cyan - Color #6 (FG `36`, BG `46`).
    Cyan,
    /// White - Color #7 (FG `37`, BG `47`).
    White,

    /// Black - Color #0 (FG `90`, BG `100`).
    BrightBlack,
    /// Red - Color #1 (FG `91`, BG `101`).
    BrightRed,
    /// Green - Color #2 (FG `92`, BG `102`).
    BrightGreen,
    /// Yellow - Color #3 (FG `93`, BG `103`).
    BrightYellow,
    /// Blue - Color #4 (FG `94`, BG `104`).
    BrightBlue,
    /// Purple - Color #5 (FG `95`, BG `105`).
    BrightPurple,
    /// Cyan - Color #6 (FG `96`, BG `106`).
    BrightCyan,
    /// White - Color #7 (FG `97`, BG `107`).
    BrightWhite,

    /// A color number from 0 to 255, for use in 256-color terminal environments.
    ///
    /// - Colours 0 to 7 are the `Black` to `White` variants respectively. These colors can usually
    ///   be changed in the terminal emulator.
    /// - Colours 8 to 15 are brighter versions of the eight colors above. These can also usually
    ///   be changed in the terminal emulator, or it could be configured to use the original
    ///   colors and show the text in bold instead. It varies depending on the program.
    /// - Colours 16 to 231 contain several palettes of bright colors, arranged in six squares
    ///   measuring six by six each.
    /// - Colours 232 to 255 are shades of gray from black to white.
    ///
    /// Reference [XTerm - Color Chart][https://upload.wikimedia.org/wikipedia/commons/1/15/Xterm_256color_chart.svg].
    Fixed(u8),

    /// A 24-bit RGB color, as specified by ISO-8613-3.
    RGB(u8, u8, u8),
}

impl Color {
    /// Converts this color to its nearest equivalent in the basic 16-color palette.
    ///
    /// This method performs downsampling from higher color modes to the basic ANSI
    /// 16-color palette (8 standard colors + 8 bright variants). This is useful when:
    ///
    /// - Targeting terminals with limited color support
    /// - Ensuring maximum compatibility across different terminal emulators
    /// - Reducing color complexity while maintaining reasonable visual fidelity
    ///
    /// # Conversion Rules
    ///
    /// - **Basic colors** ([`Black`](Color::Black)-[`White`](Color::White)): Returned unchanged
    /// - **Bright colors** ([`BrightBlack`](Color::BrightBlack)-[`BrightWhite`](Color::BrightWhite)): Returned unchanged
    /// - **Fixed palette** ([`Fixed`](Color::Fixed)): Mapped to the corresponding basic/bright color
    ///   - Colors 0-7 map to basic colors
    ///   - Colors 8-15 map to bright colors
    ///   - Colors 16-255 are converted to their nearest basic color equivalent
    /// - **RGB colors** ([`RGB`](Color::RGB)): Mapped to the nearest basic color by calculating
    ///   color distance in RGB space
    ///
    /// # Algorithm
    ///
    /// For RGB colors, the conversion finds the closest basic color by:
    /// 1. Calculating the Euclidean distance in RGB color space
    /// 2. Selecting the basic/bright color with the minimum distance
    /// 3. Choosing bright variants for colors with higher overall intensity
    ///
    /// # Examples
    ///
    /// Basic colors are preserved:
    ///
    /// ```
    /// use termionix_ansicodes::Color;
    ///
    /// let red = Color::Red;
    /// assert_eq!(red.to_basic(), Color::Red);
    ///
    /// let bright_blue = Color::BrightBlue;
    /// assert_eq!(bright_blue.to_basic(), Color::BrightBlue);
    /// ```
    ///
    /// Fixed palette colors are converted:
    ///
    /// ```
    /// use termionix_ansicodes::Color;
    ///
    /// // Color 0-7 map to basic colors
    /// let fixed_red = Color::Fixed(1);
    /// assert_eq!(fixed_red.to_basic(), Color::Red);
    ///
    /// // Color 8-15 map to bright colors
    /// let fixed_bright_red = Color::Fixed(9);
    /// assert_eq!(fixed_bright_red.to_basic(), Color::BrightRed);
    ///
    /// // Higher colors are converted to nearest match
    /// let fixed_196 = Color::Fixed(196); // Bright red in 256-color palette
    /// let basic = fixed_196.to_basic(); // Converts to Red or BrightRed
    /// ```
    ///
    /// RGB colors are approximated:
    ///
    /// ```
    /// use termionix_ansicodes::Color;
    ///
    /// // Bright red RGB
    /// let rgb_red = Color::RGB(255, 0, 0);
    /// let basic = rgb_red.to_basic();
    /// // Results in Color::Red or Color::BrightRed depending on intensity
    ///
    /// // Custom orange color
    /// let orange = Color::RGB(255, 165, 0);
    /// let basic = orange.to_basic();
    /// // Approximated to nearest basic color (likely Yellow or BrightYellow)
    /// ```
    ///
    /// # Use Cases
    ///
    /// Fallback for limited terminals:
    ///
    /// ```
    /// use termionix_ansicodes::{Color, ColorMode};
    ///
    /// fn get_color_for_mode(color: Color, mode: &ColorMode) -> Color {
    ///     match mode {
    ///         ColorMode::None => color, // Not used
    ///         ColorMode::Basic => color.to_basic(),
    ///         ColorMode::FixedColor => color.to_fixed(),
    ///         ColorMode::TrueColor => color.to_truecolor(),
    ///     }
    /// }
    /// ```
    ///
    /// # Performance
    ///
    /// - Basic/Bright colors: O(1) - immediate return
    /// - Fixed colors: O(1) for colors 0-15, O(1) computation for others
    /// - RGB colors: O(1) - fixed number of distance calculations
    ///
    /// # See Also
    ///
    /// - [`to_fixed()`](Color::to_fixed) - Convert to 256-color palette
    /// - [`to_truecolor()`](Color::to_truecolor) - Convert to 24-bit RGB
    /// - [`ColorMode::Basic`] - The color mode that uses this conversion
    pub fn to_basic(&self) -> Color {
        match self {
            Color::Black => Color::Black,
            Color::Red => Color::Red,
            Color::Green => Color::Green,
            Color::Yellow => Color::Yellow,
            Color::Blue => Color::Blue,
            Color::Purple => Color::Purple,
            Color::Cyan => Color::Cyan,
            Color::White => Color::White,

            Color::BrightBlack => Color::BrightBlack,
            Color::BrightRed => Color::BrightRed,
            Color::BrightGreen => Color::BrightGreen,
            Color::BrightYellow => Color::BrightYellow,
            Color::BrightBlue => Color::BrightBlue,
            Color::BrightPurple => Color::BrightPurple,
            Color::BrightCyan => Color::BrightCyan,
            Color::BrightWhite => Color::BrightWhite,

            Color::Fixed(n) => {
                if *n < 8 {
                    // Colors 0-7 map to basic colors
                    match n {
                        0 => Color::Black,
                        1 => Color::Red,
                        2 => Color::Green,
                        3 => Color::Yellow,
                        4 => Color::Blue,
                        5 => Color::Purple,
                        6 => Color::Cyan,
                        7 => Color::White,
                        _ => unreachable!(),
                    }
                } else if *n < 16 {
                    // Colors 8-15 map to bright colors
                    match n {
                        8 => Color::BrightBlack,
                        9 => Color::BrightRed,
                        10 => Color::BrightGreen,
                        11 => Color::BrightYellow,
                        12 => Color::BrightBlue,
                        13 => Color::BrightPurple,
                        14 => Color::BrightCyan,
                        15 => Color::BrightWhite,
                        _ => unreachable!(),
                    }
                } else {
                    // For other fixed colors, approximate to nearest basic color
                    Self::rgb_to_basic(
                        (((*n as u16 - 16) / 36) * 51) as u8,
                        ((((*n as u16 - 16) % 36) / 6) * 51) as u8,
                        (((*n as u16 - 16) % 6) * 51) as u8,
                    )
                }
            }

            Color::RGB(r, g, b) => Self::rgb_to_basic(*r, *g, *b),
        }
    }

    /// Converts this color to the 256-color fixed palette format.
    ///
    /// This method converts any color representation to a [`Fixed`](Color::Fixed) palette
    /// index (0-255). The 256-color palette provides a good balance between color variety
    /// and terminal compatibility, supported by most modern terminal emulators.
    ///
    /// # Conversion Rules
    ///
    /// - **Basic colors** ([`Black`](Color::Black)-[`White`](Color::White)): Mapped to palette indices 0-7
    /// - **Bright colors** ([`BrightBlack`](Color::BrightBlack)-[`BrightWhite`](Color::BrightWhite)): Mapped to palette indices 8-15
    /// - **Fixed palette** ([`Fixed`](Color::Fixed)): Returned unchanged
    /// - **RGB colors** ([`RGB`](Color::RGB)): Mapped to the nearest color in the 256-color palette
    ///
    /// # 256-Color Palette Structure
    ///
    /// The fixed palette is organized as follows:
    ///
    /// - **0-7**: Basic ANSI colors (Black, Red, Green, Yellow, Blue, Purple, Cyan, White)
    /// - **8-15**: Bright ANSI colors (corresponding bright variants)
    /// - **16-231**: 6×6×6 RGB cube with formula: `16 + 36×r + 6×g + b` where r,g,b ∈ [0,5]
    /// - **232-255**: 24-level grayscale ramp from dark to light
    ///
    /// # RGB to Fixed Conversion Algorithm
    ///
    /// For RGB colors, the conversion process:
    /// 1. Determines if the color is a grayscale (R ≈ G ≈ B)
    /// 2. For grayscale: Maps to the grayscale ramp (indices 232-255) or extreme black/white
    /// 3. For color: Converts to the 6×6×6 RGB cube (indices 16-231) by:
    ///    - Quantizing each RGB channel from 0-255 to 0-5
    ///    - Applying the formula: `16 + 36×r + 6×g + b`
    ///
    /// # Examples
    ///
    /// Basic colors to fixed indices:
    ///
    /// ```
    /// use termionix_ansicodes::Color;
    ///
    /// assert_eq!(Color::Black.to_fixed(), Color::Fixed(0));
    /// assert_eq!(Color::Red.to_fixed(), Color::Fixed(1));
    /// assert_eq!(Color::White.to_fixed(), Color::Fixed(7));
    /// ```
    ///
    /// Bright colors to fixed indices:
    ///
    /// ```
    /// use termionix_ansicodes::Color;
    ///
    /// assert_eq!(Color::BrightBlack.to_fixed(), Color::Fixed(8));
    /// assert_eq!(Color::BrightRed.to_fixed(), Color::Fixed(9));
    /// assert_eq!(Color::BrightWhite.to_fixed(), Color::Fixed(15));
    /// ```
    ///
    /// RGB colors are approximated:
    ///
    /// ```
    /// use termionix_ansicodes::Color;
    ///
    /// // Bright red approximated to fixed palette
    /// let red = Color::RGB(255, 0, 0);
    /// let fixed = red.to_fixed();
    /// // Results in a Color::Fixed variant close to red
    ///
    /// // Grayscale values map to grayscale ramp
    /// let gray = Color::RGB(128, 128, 128);
    /// let fixed = gray.to_fixed();
    /// // Results in Color::Fixed in range 232-255
    /// ```
    ///
    /// Converting colors for 256-color terminals:
    ///
    /// ```
    /// use termionix_ansicodes::{Color, Style, ColorMode};
    ///
    /// let style = Style {
    ///     foreground: Some(Color::RGB(255, 100, 50).to_fixed()),
    ///     ..Default::default()
    /// };
    /// ```
    ///
    /// # Use Cases
    ///
    /// Progressive enhancement:
    ///
    /// ```
    /// use termionix_ansicodes::{Color, ColorMode};
    ///
    /// fn adapt_color(color: Color, mode: &ColorMode) -> Color {
    ///     match mode {
    ///         ColorMode::Basic => color.to_basic(),
    ///         ColorMode::FixedColor => color.to_fixed(),
    ///         ColorMode::TrueColor => color,
    ///         ColorMode::None => color,
    ///     }
    /// }
    /// ```
    ///
    /// # Visual Fidelity
    ///
    /// The 256-color palette provides good color reproduction for most use cases:
    /// - The 6×6×6 RGB cube offers 216 distinct colors
    /// - The grayscale ramp provides smooth gray transitions
    /// - Most RGB colors map to visually close approximations
    ///
    /// # Performance
    ///
    /// - Basic/Bright colors: O(1) - direct index mapping
    /// - Fixed colors: O(1) - identity function
    /// - RGB colors: O(1) - constant-time calculation
    ///
    /// # See Also
    ///
    /// - [`to_basic()`](Color::to_basic) - Convert to 16-color palette
    /// - [`to_truecolor()`](Color::to_truecolor) - Convert to 24-bit RGB
    /// - [`ColorMode::FixedColor`] - The color mode that uses this format
    /// - [XTerm 256 Color Chart](https://upload.wikimedia.org/wikipedia/commons/1/15/Xterm_256color_chart.svg)
    pub fn to_fixed(&self) -> Color {
        match self {
            Color::Black => Color::Fixed(0),
            Color::Red => Color::Fixed(1),
            Color::Green => Color::Fixed(2),
            Color::Yellow => Color::Fixed(3),
            Color::Blue => Color::Fixed(4),
            Color::Purple => Color::Fixed(5),
            Color::Cyan => Color::Fixed(6),
            Color::White => Color::Fixed(7),

            Color::BrightBlack => Color::Fixed(8),
            Color::BrightRed => Color::Fixed(9),
            Color::BrightGreen => Color::Fixed(10),
            Color::BrightYellow => Color::Fixed(11),
            Color::BrightBlue => Color::Fixed(12),
            Color::BrightPurple => Color::Fixed(13),
            Color::BrightCyan => Color::Fixed(14),
            Color::BrightWhite => Color::Fixed(15),

            Color::Fixed(n) => Color::Fixed(*n),
            Color::RGB(r, g, b) => Color::Fixed(Self::rgb_to_fixed_index(*r, *g, *b)),
        }
    }

    /// Converts this color to 24-bit RGB true color format.
    ///
    /// This method converts any color representation to an [`RGB`](Color::RGB) true color
    /// with full 24-bit color depth (8 bits per channel). True color provides the highest
    /// color fidelity but requires terminal support (check `COLORTERM=truecolor`).
    ///
    /// # Conversion Rules
    ///
    /// - **Basic colors** ([`Black`](Color::Black)-[`White`](Color::White)): Mapped to standard RGB values
    /// - **Bright colors** ([`BrightBlack`](Color::BrightBlack)-[`BrightWhite`](Color::BrightWhite)): Mapped to bright RGB values
    /// - **Fixed palette** ([`Fixed`](Color::Fixed)): Converted to the RGB values of the 256-color palette
    /// - **RGB colors** ([`RGB`](Color::RGB)): Returned unchanged
    ///
    /// # Standard RGB Mappings
    ///
    /// Basic colors are mapped to commonly-used RGB values:
    ///
    /// | Color | RGB Value | Hex |
    /// |-------|-----------|-----|
    /// | Black | (0, 0, 0) | #000000 |
    /// | Red | (128, 0, 0) | #800000 |
    /// | Green | (0, 128, 0) | #008000 |
    /// | Yellow | (128, 128, 0) | #808000 |
    /// | Blue | (0, 0, 128) | #000080 |
    /// | Purple | (128, 0, 128) | #800080 |
    /// | Cyan | (0, 128, 128) | #008080 |
    /// | White | (192, 192, 192) | #C0C0C0 |
    ///
    /// Bright colors use more intense values:
    ///
    /// | Color | RGB Value | Hex |
    /// |-------|-----------|-----|
    /// | BrightBlack | (128, 128, 128) | #808080 |
    /// | BrightRed | (255, 0, 0) | #FF0000 |
    /// | BrightGreen | (0, 255, 0) | #00FF00 |
    /// | BrightYellow | (255, 255, 0) | #FFFF00 |
    /// | BrightBlue | (0, 0, 255) | #0000FF |
    /// | BrightPurple | (255, 0, 255) | #FF00FF |
    /// | BrightCyan | (0, 255, 255) | #00FFFF |
    /// | BrightWhite | (255, 255, 255) | #FFFFFF |
    ///
    /// # Fixed Palette Conversion
    ///
    /// For [`Fixed`](Color::Fixed) colors, the conversion follows the 256-color palette structure:
    ///
    /// - **0-15**: Maps to the basic/bright standard RGB values above
    /// - **16-231**: 6×6×6 RGB cube with values calculated from the cube position
    /// - **232-255**: Grayscale ramp with evenly distributed gray values
    ///
    /// # Examples
    ///
    /// Basic colors to RGB:
    ///
    /// ```
    /// use termionix_ansicodes::Color;
    ///
    /// let red = Color::Red.to_truecolor();
    /// assert_eq!(red, Color::RGB(205, 0, 0));
    ///
    /// let bright_red = Color::BrightRed.to_truecolor();
    /// assert_eq!(bright_red, Color::RGB(255, 0, 0));
    /// ```
    ///
    /// Fixed palette to RGB:
    ///
    /// ```
    /// use termionix_ansicodes::Color;
    ///
    /// // Fixed color 1 (Red) converts to standard red RGB
    /// let fixed_red = Color::Fixed(1).to_truecolor();
    /// assert_eq!(fixed_red, Color::RGB(205, 0, 0));
    ///
    /// // Fixed colors from RGB cube convert to their RGB equivalents
    /// let fixed_color = Color::Fixed(196); // A red from the RGB cube
    /// let rgb = fixed_color.to_truecolor();
    /// // Results in a Color::RGB variant
    /// ```
    ///
    /// RGB colors are preserved:
    ///
    /// ```
    /// use termionix_ansicodes::Color;
    ///
    /// let custom = Color::RGB(123, 45, 67);
    /// assert_eq!(custom.to_truecolor(), custom);
    /// ```
    ///
    /// Using for true color output:
    ///
    /// ```
    /// use termionix_ansicodes::{Color, Style, ColorMode};
    ///
    /// // Ensure all colors are in RGB format for true color terminals
    /// let style = Style {
    ///     foreground: Some(Color::Red.to_truecolor()),
    ///     background: Some(Color::Fixed(234).to_truecolor()),
    ///     ..Default::default()
    /// };
    /// ```
    ///
    /// # Use Cases
    ///
    /// Normalizing colors to RGB:
    ///
    /// ```
    /// use termionix_ansicodes::Color;
    ///
    /// fn get_rgb_values(color: Color) -> (u8, u8, u8) {
    ///     match color.to_truecolor() {
    ///         Color::RGB(r, g, b) => (r, g, b),
    ///         _ => unreachable!("to_truecolor always returns RGB"),
    ///     }
    /// }
    /// ```
    ///
    /// Color manipulation:
    ///
    /// ```
    /// use termionix_ansicodes::Color;
    ///
    /// fn lighten(color: Color, amount: u8) -> Color {
    ///     match color.to_truecolor() {
    ///         Color::RGB(r, g, b) => Color::RGB(
    ///             r.saturating_add(amount),
    ///             g.saturating_add(amount),
    ///             b.saturating_add(amount),
    ///         ),
    ///         _ => unreachable!(),
    ///     }
    /// }
    /// ```
    ///
    /// # Terminal Support
    ///
    /// True color support can be detected by checking:
    /// ```bash
    /// echo $COLORTERM  # Should be "truecolor" or "24bit"
    /// ```
    ///
    /// Most modern terminals support true color:
    /// - iTerm2 (macOS)
    /// - Windows Terminal
    /// - GNOME Terminal (3.x+)
    /// - Konsole (KDE)
    /// - Alacritty
    /// - kitty
    ///
    /// # Performance
    ///
    /// - Basic/Bright colors: O(1) - direct mapping
    /// - Fixed colors: O(1) - lookup or calculation
    /// - RGB colors: O(1) - identity function
    ///
    /// # See Also
    ///
    /// - [`to_basic()`](Color::to_basic) - Convert to 16-color palette
    /// - [`to_fixed()`](Color::to_fixed) - Convert to 256-color palette
    /// - [`ColorMode::TrueColor`] - The color mode that uses RGB format
    pub fn to_truecolor(&self) -> Color {
        match self {
            Color::Black => Color::RGB(0, 0, 0),
            Color::Red => Color::RGB(205, 0, 0),
            Color::Green => Color::RGB(0, 205, 0),
            Color::Yellow => Color::RGB(205, 205, 0),
            Color::Blue => Color::RGB(0, 0, 238),
            Color::Purple => Color::RGB(205, 0, 205),
            Color::Cyan => Color::RGB(0, 205, 205),
            Color::White => Color::RGB(229, 229, 229),

            Color::BrightBlack => Color::RGB(127, 127, 127),
            Color::BrightRed => Color::RGB(255, 0, 0),
            Color::BrightGreen => Color::RGB(0, 255, 0),
            Color::BrightYellow => Color::RGB(255, 255, 0),
            Color::BrightBlue => Color::RGB(92, 92, 255),
            Color::BrightPurple => Color::RGB(255, 0, 255),
            Color::BrightCyan => Color::RGB(0, 255, 255),
            Color::BrightWhite => Color::RGB(255, 255, 255),

            Color::Fixed(n) => {
                if *n < 16 {
                    // Use the basic/bright color mappings
                    match n {
                        0 => Color::RGB(0, 0, 0),
                        1 => Color::RGB(205, 0, 0),
                        2 => Color::RGB(0, 205, 0),
                        3 => Color::RGB(205, 205, 0),
                        4 => Color::RGB(0, 0, 238),
                        5 => Color::RGB(205, 0, 205),
                        6 => Color::RGB(0, 205, 205),
                        7 => Color::RGB(229, 229, 229),
                        8 => Color::RGB(127, 127, 127),
                        9 => Color::RGB(255, 0, 0),
                        10 => Color::RGB(0, 255, 0),
                        11 => Color::RGB(255, 255, 0),
                        12 => Color::RGB(92, 92, 255),
                        13 => Color::RGB(255, 0, 255),
                        14 => Color::RGB(0, 255, 255),
                        15 => Color::RGB(255, 255, 255),
                        _ => unreachable!(),
                    }
                } else if *n < 232 {
                    // 216-color cube (16-231)
                    let idx = *n as u16 - 16;
                    let r = ((idx / 36) * 51) as u8;
                    let g = (((idx % 36) / 6) * 51) as u8;
                    let b = ((idx % 6) * 51) as u8;
                    Color::RGB(r, g, b)
                } else {
                    // Grayscale (232-255)
                    let gray = ((*n as u16 - 232) * 10 + 8) as u8;
                    Color::RGB(gray, gray, gray)
                }
            }

            Color::RGB(r, g, b) => Color::RGB(*r, *g, *b),
        }
    }

    /// Converts RGB values to the nearest basic 16-color palette color.
    ///
    /// This is an internal helper method used by [`to_basic()`](Color::to_basic) to perform
    /// the actual RGB-to-basic color conversion. It calculates the Euclidean distance in
    /// RGB color space to find the closest match from the 16 basic/bright ANSI colors.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel value (0-255)
    /// * `g` - Green channel value (0-255)
    /// * `b` - Blue channel value (0-255)
    ///
    /// # Algorithm
    ///
    /// 1. Computes the distance to each of the 16 basic colors using the formula:
    ///    ```text
    ///    distance = √((r₁-r₂)² + (g₁-g₂)² + (b₁-b₂)²)
    ///    ```
    /// 2. Returns the color with the minimum distance
    /// 3. Prioritizes bright variants for higher-intensity colors
    ///
    /// # Returns
    ///
    /// A [`Color`] variant from the basic 16-color palette that most closely matches
    /// the input RGB values.
    ///
    /// # Examples
    ///
    /// This method is primarily used internally:
    ///
    /// ```ignore
    /// // Internal usage in to_basic()
    /// let rgb_red = Color::RGB(255, 0, 0);
    /// // Internally calls rgb_to_basic(255, 0, 0)
    /// let basic = rgb_red.to_basic();
    /// ```
    ///
    /// # Performance
    ///
    /// O(1) - Fixed number of distance calculations (16 colors)
    ///
    /// # See Also
    ///
    /// - [`to_basic()`](Color::to_basic) - Public method that uses this helper
    /// - [`rgb_to_fixed_index()`](Color::rgb_to_fixed_index) - Similar conversion for 256-color palette
    fn rgb_to_basic(r: u8, g: u8, b: u8) -> Color {
        // Calculate perceived brightness
        let brightness = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000;

        // Determine which color component is dominant
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let saturation = if max > 0 {
            ((max - min) as f32 / max as f32) * 100.0
        } else {
            0.0
        };

        // Low saturation means grayscale
        if saturation < 25.0 {
            if brightness > 127 {
                return Color::BrightWhite;
            } else if brightness > 64 {
                return Color::White;
            } else if brightness > 32 {
                return Color::BrightBlack;
            } else {
                return Color::Black;
            }
        }

        // For saturated colors, choose based on dominant component
        let is_bright = brightness > 127;

        if r > g && r > b {
            // Red dominant
            if is_bright {
                Color::BrightRed
            } else {
                Color::Red
            }
        } else if g > r && g > b {
            // Green dominant
            if is_bright {
                Color::BrightGreen
            } else {
                Color::Green
            }
        } else if b > r && b > g {
            // Blue dominant
            if is_bright {
                Color::BrightBlue
            } else {
                Color::Blue
            }
        } else if r > 0 && g > 0 && b == min {
            // Yellow (red + green)
            if is_bright {
                Color::BrightYellow
            } else {
                Color::Yellow
            }
        } else if r > 0 && b > 0 && g == min {
            // Magenta/Purple (red + blue)
            if is_bright {
                Color::BrightPurple
            } else {
                Color::Purple
            }
        } else if g > 0 && b > 0 && r == min {
            // Cyan (green + blue)
            if is_bright {
                Color::BrightCyan
            } else {
                Color::Cyan
            }
        } else {
            // Fallback to white/black based on brightness
            if is_bright {
                Color::BrightWhite
            } else {
                Color::Black
            }
        }
    }

    /// Converts RGB values to the nearest 256-color palette index.
    ///
    /// This is an internal helper method used by [`to_fixed()`](Color::to_fixed) to convert
    /// RGB colors to the 256-color fixed palette. It intelligently determines whether to
    /// use the grayscale ramp or the RGB cube based on the color characteristics.
    ///
    /// # Arguments
    ///
    /// * `r` - Red channel value (0-255)
    /// * `g` - Green channel value (0-255)
    /// * `b` - Blue channel value (0-255)
    ///
    /// # Algorithm
    ///
    /// 1. **Grayscale Detection**: If R ≈ G ≈ B (within a threshold), maps to grayscale ramp
    ///    - Pure black (0,0,0) → Index 0
    ///    - Pure white (255,255,255) → Index 15
    ///    - Gray values → Indices 232-255 (24-level grayscale ramp)
    ///
    /// 2. **Color Mapping**: Otherwise, maps to the 6×6×6 RGB cube (indices 16-231)
    ///    - Quantizes each RGB channel from 0-255 to 0-5
    ///    - Applies formula: `16 + 36×r + 6×g + b`
    ///
    /// # Quantization
    ///
    /// RGB values are quantized to 6 levels using:
    /// ```text
    /// level = round(channel / 255 × 5)
    /// ```
    ///
    /// This maps:
    /// - 0-25 → 0
    /// - 26-76 → 1
    /// - 77-127 → 2
    /// - 128-178 → 3
    /// - 179-229 → 4
    /// - 230-255 → 5
    ///
    /// # Returns
    ///
    /// A palette index (0-255) representing the closest color in the 256-color palette.
    ///
    /// # Examples
    ///
    /// This method is primarily used internally:
    ///
    /// ```ignore
    /// // Internal usage in to_fixed()
    /// let rgb_color = Color::RGB(128, 64, 192);
    /// // Internally calls rgb_to_fixed_index(128, 64, 192)
    /// let fixed = rgb_color.to_fixed();
    /// ```
    ///
    /// # Color Space Mapping
    ///
    /// The 256-color palette structure:
    ///
    /// ```text
    /// 0-15:   Basic/Bright colors (standard ANSI)
    /// 16-231: 6×6×6 RGB cube (216 colors)
    ///         16 + 36×r + 6×g + b where r,g,b ∈ [0,5]
    /// 232-255: Grayscale ramp (24 shades)
    /// ```
    ///
    /// # Performance
    ///
    /// O(1) - Constant-time calculation with no loops
    ///
    /// # See Also
    ///
    /// - [`to_fixed()`](Color::to_fixed) - Public method that uses this helper
    /// - [`rgb_to_basic()`](Color::rgb_to_basic) - Similar conversion for basic colors
    /// - [XTerm 256 Color Chart](https://upload.wikimedia.org/wikipedia/commons/1/15/Xterm_256color_chart.svg)
    fn rgb_to_fixed_index(r: u8, g: u8, b: u8) -> u8 {
        // Check if it's close to a grayscale value
        let avg = (r as u16 + g as u16 + b as u16) / 3;
        let variance = ((r as i16 - avg as i16).abs()
            + (g as i16 - avg as i16).abs()
            + (b as i16 - avg as i16).abs()) as u16;

        if variance < 15 {
            // Map to grayscale colors (232-255)
            if avg < 8 {
                return 16; // Use color cube black
            } else if avg > 238 {
                return 231; // Use color cube white
            } else {
                return (232 + (avg - 8) / 10) as u8;
            }
        }

        // Map to 216-color cube (16-231)
        let r_idx = ((r as u16 * 5 + 127) / 255) as u8;
        let g_idx = ((g as u16 * 5 + 127) / 255) as u8;
        let b_idx = ((b as u16 * 5 + 127) / 255) as u8;

        16 + 36 * r_idx + 6 * g_idx + b_idx
    }
}

/// Represents the font selection for text in ANSI terminal styling.
///
/// The `Font` enum allows selection between the primary (default) font and up to 10
/// alternate fonts, including a special Fraktur (gothic) font. Font support varies
/// significantly between terminal emulators, and many terminals may ignore these
/// codes entirely or only support a subset of fonts.
///
/// # ANSI SGR Codes
///
/// Each font variant corresponds to a specific SGR (Select Graphic Rendition) code:
///
/// | Variant | SGR Code | Description |
/// |---------|----------|-------------|
/// | `PrimaryFont` | 10 | Default/primary font (resets to normal) |
/// | `AlternateFont1` | 11 | First alternate font |
/// | `AlternateFont2` | 12 | Second alternate font |
/// | `AlternateFont3` | 13 | Third alternate font |
/// | `AlternateFont4` | 14 | Fourth alternate font |
/// | `AlternateFont5` | 15 | Fifth alternate font |
/// | `AlternateFont6` | 16 | Sixth alternate font |
/// | `AlternateFont7` | 17 | Seventh alternate font |
/// | `AlternateFont8` | 18 | Eighth alternate font |
/// | `AlternateFont9` | 19 | Ninth alternate font |
/// | `Fraktur` | 20 | Fraktur (gothic/blackletter) font |
///
/// # Terminal Support
///
/// **Warning**: Font selection codes have very limited support across terminal emulators:
///
/// - Most modern terminals (iTerm2, GNOME Terminal, Windows Terminal, Alacritty, kitty)
///   **do not support** alternate font selection
/// - Some terminals may interpret alternate fonts as font style changes (italic, bold)
/// - The Fraktur font is rarely supported in modern terminals
/// - When unsupported, these codes are typically ignored without affecting output
///
/// # Use Cases
///
/// Due to limited support, font selection is primarily useful for:
/// - Legacy terminal applications
/// - Specialized terminal emulators with custom font configurations
/// - Testing ANSI code parsing and rendering
/// - Historical compatibility
///
/// For modern applications, consider using other styling attributes like
/// [`Intensity::Bold`] or `italic` instead.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use termionix_ansicodes::{Style, Font, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let style = Style {
///     font: Some(Font::AlternateFont1),
///     ..Default::default()
/// };
///
/// let mut output = String::new();
/// style.write_style(&mut output, Some(&config)).unwrap();
/// // Generates: "\x1b[11m"
/// ```
///
/// Using Fraktur font:
///
/// ```
/// use termionix_ansicodes::{Style, Font, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let fraktur_style = Style {
///     font: Some(Font::Fraktur),
///     ..Default::default()
/// };
///
/// let mut output = String::new();
/// fraktur_style.write_style(&mut output, Some(&config)).unwrap();
/// // Generates: "\x1b[20m"
/// ```
///
/// Resetting to primary font:
///
/// ```
/// use termionix_ansicodes::{Style, Font, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let reset_style = Style {
///     font: Some(Font::PrimaryFont),
///     ..Default::default()
/// };
///
/// let mut output = String::new();
/// reset_style.write_style(&mut output, Some(&config)).unwrap();
/// // Generates: "\x1b[10m"
/// ```
///
/// # Default
///
/// The default font is [`PrimaryFont`](Font::PrimaryFont), which represents the
/// terminal's standard font and is used to reset any alternate font selection.
///
/// ```
/// use termionix_ansicodes::Font;
///
/// assert_eq!(Font::default(), Font::PrimaryFont);
/// ```
///
/// # Conversion
///
/// The `Font` enum can be converted to SGR codes:
///
/// ```
/// use termionix_ansicodes::Font;
///
/// assert_eq!(Font::PrimaryFont.to_u8(), 10);
/// assert_eq!(Font::AlternateFont1.to_u8(), 11);
/// assert_eq!(Font::Fraktur.to_u8(), 20);
/// ```
///
/// # Notes
///
/// - Setting `font: None` in a [`AnsiSelectGraphicRendition`] means no font selection code is generated
/// - Setting `font: Some(Font::PrimaryFont)` explicitly resets to the primary font
/// - Font selection does not persist across style resets (`\x1b[0m`)
/// - Multiple font selections override each other; only the last one takes effect
///
/// # Historical Context
///
/// Font selection codes were part of the original ANSI X3.64 standard (now ECMA-48)
/// but were never widely adopted. The Fraktur font code (20) was included for
/// compatibility with older German computer systems that used gothic-style fonts.
///
/// # See Also
///
/// - [`AnsiSelectGraphicRendition`] - Container for text styling including font selection
/// - [`AnsiSelectGraphicRendition::font`] - The font field in a style
/// - [`AnsiSelectGraphicRendition::write`] - Renders font selection as ANSI codes
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Font {
    /// Primary (default) font - SGR code 10.
    ///
    /// This is the terminal's default font. Selecting this font resets any
    /// previous alternate font selection back to normal.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    /// assert_eq!(Font::PrimaryFont.to_u8(), 10);
    /// ```
    #[default]
    PrimaryFont,

    /// First alternate font - SGR code 11.
    ///
    /// Selects the first alternate font if supported by the terminal.
    /// Most terminals do not support this and will ignore the code.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    /// assert_eq!(Font::AlternateFont1.to_u8(), 11);
    /// ```
    AlternateFont1,

    /// Second alternate font - SGR code 12.
    ///
    /// Selects the second alternate font if supported by the terminal.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    /// assert_eq!(Font::AlternateFont2.to_u8(), 12);
    /// ```
    AlternateFont2,

    /// Third alternate font - SGR code 13.
    ///
    /// Selects the third alternate font if supported by the terminal.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    /// assert_eq!(Font::AlternateFont3.to_u8(), 13);
    /// ```
    AlternateFont3,

    /// Fourth alternate font - SGR code 14.
    ///
    /// Selects the fourth alternate font if supported by the terminal.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    /// assert_eq!(Font::AlternateFont4.to_u8(), 14);
    /// ```
    AlternateFont4,

    /// Fifth alternate font - SGR code 15.
    ///
    /// Selects the fifth alternate font if supported by the terminal.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    /// assert_eq!(Font::AlternateFont5.to_u8(), 15);
    /// ```
    AlternateFont5,

    /// Sixth alternate font - SGR code 16.
    ///
    /// Selects the sixth alternate font if supported by the terminal.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    /// assert_eq!(Font::AlternateFont6.to_u8(), 16);
    /// ```
    AlternateFont6,

    /// Seventh alternate font - SGR code 17.
    ///
    /// Selects the seventh alternate font if supported by the terminal.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    /// assert_eq!(Font::AlternateFont7.to_u8(), 17);
    /// ```
    AlternateFont7,

    /// Eighth alternate font - SGR code 18.
    ///
    /// Selects the eighth alternate font if supported by the terminal.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    /// assert_eq!(Font::AlternateFont8.to_u8(), 18);
    /// ```
    AlternateFont8,

    /// Ninth alternate font - SGR code 19.
    ///
    /// Selects the ninth alternate font if supported by the terminal.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    /// assert_eq!(Font::AlternateFont9.to_u8(), 19);
    /// ```
    AlternateFont9,

    /// Fraktur (gothic/blackletter) font - SGR code 20.
    ///
    /// Selects a Fraktur-style gothic font if supported. This was primarily
    /// used on older German computer systems. Modern terminals rarely support
    /// this font style.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    /// assert_eq!(Font::Fraktur.to_u8(), 20);
    /// ```
    Fraktur,
}

impl Font {
    /// Converts the font variant to its corresponding ANSI SGR code.
    ///
    /// This method returns the numeric SGR parameter code used in ANSI escape
    /// sequences to select this font. The codes range from 10 (primary font)
    /// to 20 (Fraktur font).
    ///
    /// # Returns
    ///
    /// The ANSI SGR code as a `u8`:
    /// - `PrimaryFont` → `10` - Reset to default font
    /// - `AlternateFont1` → `11` - First alternate font
    /// - `AlternateFont2` → `12` - Second alternate font
    /// - `AlternateFont3` → `13` - Third alternate font
    /// - `AlternateFont4` → `14` - Fourth alternate font
    /// - `AlternateFont5` → `15` - Fifth alternate font
    /// - `AlternateFont6` → `16` - Sixth alternate font
    /// - `AlternateFont7` → `17` - Seventh alternate font
    /// - `AlternateFont8` → `18` - Eighth alternate font
    /// - `AlternateFont9` → `19` - Ninth alternate font
    /// - `Fraktur` → `20` - Fraktur/gothic font
    ///
    /// # Examples
    ///
    /// Basic conversion:
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    ///
    /// assert_eq!(Font::PrimaryFont.to_u8(), 10);
    /// assert_eq!(Font::AlternateFont1.to_u8(), 11);
    /// assert_eq!(Font::AlternateFont5.to_u8(), 15);
    /// assert_eq!(Font::Fraktur.to_u8(), 20);
    /// ```
    ///
    /// Building ANSI sequences:
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    ///
    /// let font = Font::AlternateFont3;
    /// let code = font.to_u8();
    /// let ansi_sequence = format!("\x1b[{}m", code);
    /// assert_eq!(ansi_sequence, "\x1b[13m");
    /// ```
    ///
    /// Iterating through all fonts:
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    ///
    /// let fonts = vec![
    ///     Font::PrimaryFont,
    ///     Font::AlternateFont1,
    ///     Font::AlternateFont2,
    ///     Font::AlternateFont3,
    ///     Font::AlternateFont4,
    ///     Font::AlternateFont5,
    ///     Font::AlternateFont6,
    ///     Font::AlternateFont7,
    ///     Font::AlternateFont8,
    ///     Font::AlternateFont9,
    ///     Font::Fraktur,
    /// ];
    ///
    /// for (i, font) in fonts.iter().enumerate() {
    ///     let code = font.to_u8();
    ///     assert_eq!(code, (10 + i) as u8);
    /// }
    /// ```
    ///
    /// # Use in ANSI Sequences
    ///
    /// The returned code is used directly in ANSI escape sequences:
    ///
    /// ```text
    /// \x1b[<code>m
    /// ```
    ///
    /// For example, `Font::AlternateFont1.to_u8()` returns `11`, which generates
    /// the sequence `\x1b[11m`.
    ///
    /// # Performance
    ///
    /// This is a simple constant-time operation that performs a single pattern match.
    ///
    /// # See Also
    ///
    /// - [`AnsiSelectGraphicRendition::write`] - Uses this method when rendering font codes
    /// - [`AnsiSelectGraphicRendition::font`] - The field that stores font selection in a style
    pub fn to_u8(&self) -> u8 {
        match self {
            Font::PrimaryFont => 10,
            Font::AlternateFont1 => 11,
            Font::AlternateFont2 => 12,
            Font::AlternateFont3 => 13,
            Font::AlternateFont4 => 14,
            Font::AlternateFont5 => 15,
            Font::AlternateFont6 => 16,
            Font::AlternateFont7 => 17,
            Font::AlternateFont8 => 18,
            Font::AlternateFont9 => 19,
            Font::Fraktur => 20,
        }
    }

    /// Converts an ANSI SGR code to its corresponding `Font` variant.
    ///
    /// This method attempts to parse a numeric ANSI SGR code into a `Font` value.
    /// It recognizes the standard font selection codes from 10 (primary font) through
    /// 20 (Fraktur font).
    ///
    /// # Arguments
    ///
    /// * `value` - The ANSI SGR code to convert
    ///
    /// # Returns
    ///
    /// - `Some(Font::PrimaryFont)` if `value` is `10`
    /// - `Some(Font::AlternateFont1)` if `value` is `11`
    /// - `Some(Font::AlternateFont2)` if `value` is `12`
    /// - `Some(Font::AlternateFont3)` if `value` is `13`
    /// - `Some(Font::AlternateFont4)` if `value` is `14`
    /// - `Some(Font::AlternateFont5)` if `value` is `15`
    /// - `Some(Font::AlternateFont6)` if `value` is `16`
    /// - `Some(Font::AlternateFont7)` if `value` is `17`
    /// - `Some(Font::AlternateFont8)` if `value` is `18`
    /// - `Some(Font::AlternateFont9)` if `value` is `19`
    /// - `Some(Font::Fraktur)` if `value` is `20`
    /// - `None` for any other value
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    ///
    /// assert_eq!(Font::from_u8(10), Some(Font::PrimaryFont));
    /// assert_eq!(Font::from_u8(11), Some(Font::AlternateFont1));
    /// assert_eq!(Font::from_u8(15), Some(Font::AlternateFont5));
    /// assert_eq!(Font::from_u8(20), Some(Font::Fraktur));
    /// ```
    ///
    /// Handling invalid codes:
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    ///
    /// assert_eq!(Font::from_u8(0), None);
    /// assert_eq!(Font::from_u8(9), None);
    /// assert_eq!(Font::from_u8(21), None);
    /// assert_eq!(Font::from_u8(99), None);
    /// ```
    ///
    /// Parsing ANSI sequences:
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    ///
    /// // Parse a code from an ANSI sequence parameter
    /// let sgr_params: Vec<u8> = vec![1, 11, 31]; // Bold, AlternateFont1, Red
    ///
    /// for param in sgr_params {
    ///     if let Some(font) = Font::from_u8(param) {
    ///         println!("Found font selection: {:?}", font);
    ///     }
    /// }
    /// ```
    ///
    /// Round-trip conversion:
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    ///
    /// let original = Font::AlternateFont3;
    /// let code = original.to_u8();
    /// let parsed = Font::from_u8(code);
    ///
    /// assert_eq!(parsed, Some(original));
    /// ```
    ///
    /// Using in a parser:
    ///
    /// ```
    /// use termionix_ansicodes::Font;
    ///
    /// fn parse_sgr_code(code: u8) -> Option<String> {
    ///     Font::from_u8(code).map(|font| {
    ///         format!("Font selection: {:?}", font)
    ///     })
    /// }
    ///
    /// assert_eq!(
    ///     parse_sgr_code(12),
    ///     Some("Font selection: AlternateFont2".to_string())
    /// );
    /// assert_eq!(parse_sgr_code(99), None);
    /// ```
    ///
    /// # Use in ANSI Parsing
    ///
    /// This method is primarily used when parsing ANSI escape sequences to detect
    /// font selection codes:
    ///
    /// ```text
    /// Input:  "\x1b[11m"
    /// Parsed: [11]
    /// Result: Font::from_u8(11) → Some(Font::AlternateFont1)
    /// ```
    ///
    /// # Performance
    ///
    /// O(1) - Constant-time pattern matching with 11 branches.
    ///
    /// # Notes
    ///
    /// - This method only recognizes font selection codes (10-20)
    /// - Other SGR codes (like 0 for reset, 1 for bold, etc.) return `None`
    /// - The method is used internally by [`AnsiSelectGraphicRendition::parse`] when processing SGR parameters
    ///
    /// # See Also
    ///
    /// - [`to_u8()`](Font::to_u8) - Converts a `Font` to its SGR code
    /// - [`AnsiSelectGraphicRendition::parse`] - Parses complete SGR sequences including font codes
    /// - [`Intensity::from_u8`] - Similar conversion for intensity codes
    /// - [`Underline::from_u8`] - Similar conversion for underline codes
    /// - [`Blink::from_u8`] - Similar conversion for blink codes
    pub fn from_u8(value: u8) -> Option<Font> {
        match value {
            10 => Some(Font::PrimaryFont),
            11 => Some(Font::AlternateFont1),
            12 => Some(Font::AlternateFont2),
            13 => Some(Font::AlternateFont3),
            14 => Some(Font::AlternateFont4),
            15 => Some(Font::AlternateFont5),
            16 => Some(Font::AlternateFont6),
            17 => Some(Font::AlternateFont7),
            18 => Some(Font::AlternateFont8),
            19 => Some(Font::AlternateFont9),
            20 => Some(Font::Fraktur),
            _ => None,
        }
    }
}

/// Text blinking mode for terminal output.
///
/// Controls whether text should blink and at what rate. Blinking text is achieved
/// through ANSI escape sequences and terminal support varies across different
/// terminal emulators. Modern terminals may not support blinking or may allow
/// users to disable it.
///
/// # ANSI Escape Codes
///
/// - **Slow blink**: CSI code 5 (`\x1b[5m`) - Less than 150 blinks per minute
/// - **Rapid blink**: CSI code 6 (`\x1b[6m`) - 150+ blinks per minute
/// - **Off**: No blink code emitted (default state)
///
/// # Terminal Support
///
/// Blinking text support varies significantly:
/// - **Traditional terminals**: Often support slow blink; rapid blink may be treated as slow
/// - **Modern terminals**: Many disable blinking by default or replace it with other emphasis
/// - **Web terminals**: Usually don't support blinking
/// - **Accessibility**: Blinking can be problematic for users with certain conditions
///
/// # Examples
///
/// Basic usage:
///
/// ```rust
/// use termionix_ansicodes::{Style, Blink, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let style = Style {
///     blink: Some(Blink::Slow),
///     ..Default::default()
/// };
///
/// let output = style.to_string(Some(&config));
/// assert_eq!(output, "\x1b[5m"); // Slow blink escape code
/// ```
///
/// Different blink rates:
///
/// ```rust
/// use termionix_ansicodes::{Style, Blink, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let slow = Style {
///     blink: Some(Blink::Slow),
///     ..Default::default()
/// };
///
/// let rapid = Style {
///     blink: Some(Blink::Rapid),
///     ..Default::default()
/// };
///
/// let mut output = String::new();
///
/// slow.write_style(&mut output, Some(&config)).unwrap();
/// assert_eq!(output, "\x1b[5m");
///
/// output.clear();
/// rapid.write_style(&mut output, Some(&config)).unwrap();
/// assert_eq!(output, "\x1b[6m");
/// ```
///
/// No blinking (default):
///
/// ```rust
/// use termionix_ansicodes::{Style, Blink, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let style = Style {
///     blink: Some(Blink::Off),
///     ..Default::default()
/// };
///
/// let output = style.to_string(Some(&config));
/// assert_eq!(output, "\x1b[25m"); // No escape code emitted
/// ```
///
/// # Conversion
///
/// Convert between `Blink` and its numeric representation:
///
/// ```rust
/// use termionix_ansicodes::Blink;
///
/// // To numeric codes
/// assert_eq!(Blink::Off.to_u8(), 25);
/// assert_eq!(Blink::Slow.to_u8(), 5);
/// assert_eq!(Blink::Rapid.to_u8(), 6);
///
/// // From numeric codes
/// assert_eq!(Blink::from_u8(25), Some(Blink::Off));
/// assert_eq!(Blink::from_u8(5), Some(Blink::Slow));
/// assert_eq!(Blink::from_u8(6), Some(Blink::Rapid));
/// assert_eq!(Blink::from_u8(99), None); // Invalid code
/// ```
///
/// # Use Cases
///
/// - **Attention-grabbing**: Highlight critical warnings or alerts
/// - **Status indicators**: Show active/processing states
/// - **Accessibility**: Should be used sparingly and with user control
/// - **Legacy compatibility**: Support older terminal applications
///
/// # Best Practices
///
/// - **Avoid overuse**: Blinking text can be distracting and reduce readability
/// - **Provide alternatives**: Don't rely solely on blinking for important information
/// - **Respect user preferences**: Many users disable blinking for accessibility
/// - **Test compatibility**: Check that your target terminals support blinking
///
/// # See Also
///
/// - [`AnsiSelectGraphicRendition`] - Container for all text styling attributes including blink
/// - [`Intensity`] - Control text boldness/dimness
/// - [`Underline`] - Underline style options
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Blink {
    /// No blinking (default state).
    ///
    /// Text displays normally without any blinking effect. This is the default
    /// state and produces no ANSI escape codes when written.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::{Style, Blink, ColorMode, AnsiConfig};
    ///
    /// let config = AnsiConfig::enabled();
    /// let style = Style {
    ///     blink: Some(Blink::Off),
    ///     ..Default::default()
    /// };
    ///
    /// let output = style.to_string(Some(&config));
    /// assert_eq!(output, "\x1b[25m"); // No escape code
    /// ```
    #[default]
    Off,

    /// Slow blinking text (less than 150 blinks per minute).
    ///
    /// Activates slow text blinking using ANSI CSI code 5. The exact blink rate
    /// depends on terminal implementation but should be less than 150 blinks per
    /// minute according to the standard.
    ///
    /// Most terminals that support blinking implement this variant, though support
    /// varies and many modern terminals allow users to disable it.
    ///
    /// # ANSI Code
    ///
    /// Emits: `\x1b[5m` (CSI 5 m)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::{Style, Blink, ColorMode, AnsiConfig};
    ///
    /// let config = AnsiConfig::enabled();
    /// let style = Style {
    ///     blink: Some(Blink::Slow),
    ///     ..Default::default()
    /// };
    ///
    /// let output = style.to_string(Some(&config));
    /// assert_eq!(output, "\x1b[5m");
    /// ```
    Slow,

    /// Rapid blinking text (150 or more blinks per minute).
    ///
    /// Activates rapid text blinking using ANSI CSI code 6. The exact blink rate
    /// depends on terminal implementation but should be 150 or more blinks per
    /// minute according to the standard.
    ///
    /// Support for rapid blinking is less common than slow blinking. Many terminals
    /// treat this the same as slow blinking or ignore it entirely.
    ///
    /// # ANSI Code
    ///
    /// Emits: `\x1b[6m` (CSI 6 m)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::{Style, Blink, ColorMode, AnsiConfig};
    ///
    /// let config = AnsiConfig::enabled();
    /// let style = Style {
    ///     blink: Some(Blink::Rapid),
    ///     ..Default::default()
    /// };
    ///
    /// let output = style.to_string(Some(&config));
    /// assert_eq!(output, "\x1b[6m");
    /// ```
    ///
    /// # Terminal Support
    ///
    /// Many terminals fall back to slow blinking when rapid blinking is requested,
    /// as rapid blinking support is optional in most terminal implementations.
    Rapid,
}

impl Blink {
    /// Converts this blink mode to its ANSI numeric code.
    ///
    /// Returns the numeric value used in ANSI escape sequences to represent
    /// this blink mode. These values correspond to SGR (Select Graphic Rendition)
    /// parameter codes.
    ///
    /// # Returns
    ///
    /// - `0` for [`Blink::Off`] - No blink code (used internally)
    /// - `5` for [`Blink::Slow`] - ANSI code for slow blink
    /// - `6` for [`Blink::Rapid`] - ANSI code for rapid blink
    ///
    /// # Examples
    ///
    /// ```rust
    /// use termionix_ansicodes::Blink;
    ///
    /// assert_eq!(Blink::Off.to_u8(), 25);
    /// assert_eq!(Blink::Slow.to_u8(), 5);
    /// assert_eq!(Blink::Rapid.to_u8(), 6);
    /// ```
    ///
    /// Building ANSI sequences manually:
    ///
    /// ```rust
    /// use termionix_ansicodes::Blink;
    ///
    /// let blink = Blink::Slow;
    /// if blink.to_u8() != 0 {
    ///     let sequence = format!("\x1b[{}m", blink.to_u8());
    ///     assert_eq!(sequence, "\x1b[5m");
    /// }
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Serialization**: Convert to numeric form for storage or transmission
    /// - **Manual sequence building**: Construct custom ANSI escape sequences
    /// - **Debugging**: Display numeric codes for diagnostics
    /// - **Protocol implementation**: Work with raw ANSI parameter values
    pub fn to_u8(&self) -> u8 {
        match self {
            Blink::Off => 25,
            Blink::Slow => 5,
            Blink::Rapid => 6,
        }
    }

    /// Converts an ANSI numeric code to a blink mode.
    ///
    /// Parses a numeric SGR parameter code and returns the corresponding blink
    /// mode if the value is valid. Returns `None` for unrecognized or invalid
    /// blink codes.
    ///
    /// # Arguments
    ///
    /// * `value` - The numeric ANSI code to parse
    ///
    /// # Returns
    ///
    /// - `Some(Blink::Off)` for value `0`
    /// - `Some(Blink::Slow)` for value `5`
    /// - `Some(Blink::Rapid)` for value `6`
    /// - `None` for any other value
    ///
    /// # Examples
    ///
    /// Valid blink codes:
    ///
    /// ```rust
    /// use termionix_ansicodes::Blink;
    ///
    /// assert_eq!(Blink::from_u8(25), Some(Blink::Off));
    /// assert_eq!(Blink::from_u8(5), Some(Blink::Slow));
    /// assert_eq!(Blink::from_u8(6), Some(Blink::Rapid));
    /// ```
    ///
    /// Invalid blink codes:
    ///
    /// ```rust
    /// use termionix_ansicodes::Blink;
    ///
    /// assert_eq!(Blink::from_u8(1), None);
    /// assert_eq!(Blink::from_u8(7), None);
    /// assert_eq!(Blink::from_u8(255), None);
    /// ```
    ///
    /// Parsing ANSI sequences:
    ///
    /// ```rust
    /// use termionix_ansicodes::Blink;
    ///
    /// let ansi_params = vec![5, 31]; // Slow blink + red foreground
    ///
    /// for &param in &ansi_params {
    ///     if let Some(blink) = Blink::from_u8(param) {
    ///         println!("Found blink mode: {:?}", blink);
    ///     }
    /// }
    /// ```
    ///
    /// Safe parsing with fallback:
    ///
    /// ```rust
    /// use termionix_ansicodes::Blink;
    ///
    /// let code = 5;
    /// let blink = Blink::from_u8(code).unwrap_or(Blink::Off);
    /// assert_eq!(blink, Blink::Slow);
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **ANSI parsing**: Convert parsed numeric codes to strongly-typed values
    /// - **Deserialization**: Reconstruct blink modes from stored numeric values
    /// - **Validation**: Check if a numeric code represents a valid blink mode
    /// - **Protocol handling**: Process received ANSI parameter sequences
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            25 => Some(Blink::Off),
            5 => Some(Blink::Slow),
            6 => Some(Blink::Rapid),
            _ => None,
        }
    }
}

/// Text position mode for subscript and superscript styling.
///
/// Controls vertical positioning of text characters relative to the baseline, allowing
/// for scientific notation, mathematical expressions, and typographic effects. Support
/// for these features varies significantly across terminal emulators.
///
/// # ANSI SGR Codes
///
/// Each variant corresponds to a specific SGR (Select Graphic Rendition) code:
///
/// | Variant | SGR Code | Description |
/// |---------|----------|-------------|
/// | `Normal` | 75 | Normal baseline position (resets subscript/superscript) |
/// | `Superscript` | 73 | Raised text above baseline |
/// | `Subscript` | 74 | Lowered text below baseline |
///
/// # Terminal Support
///
/// **Warning**: Script positioning codes have very limited support across terminal emulators:
///
/// - Most modern terminals (iTerm2, GNOME Terminal, Windows Terminal, Alacritty, kitty)
///   **do not support** superscript/subscript positioning
/// - Some terminals may ignore these codes entirely
/// - When supported, the exact positioning and scaling behavior varies by terminal
/// - These codes are rarely implemented in practice
///
/// # Use Cases
///
/// Due to limited support, script positioning is primarily useful for:
/// - Mathematical formulas (x², H₂O)
/// - Scientific notation (10³, CO₂)
/// - Footnote references¹²³
/// - Chemical formulas
/// - Legacy terminal applications
/// - Testing ANSI code parsing and rendering
///
/// For modern applications, consider using Unicode subscript/superscript characters
/// instead, which have better support across terminals and text systems.
///
/// # Unicode Alternative
///
/// For better compatibility, consider using Unicode subscript and superscript characters:
///
/// **Superscripts**: ⁰ ¹ ² ³ ⁴ ⁵ ⁶ ⁷ ⁸ ⁹ ⁺ ⁻ ⁼ ⁽ ⁾ ⁿ
///
/// **Subscripts**: ₀ ₁ ₂ ₃ ₄ ₅ ₆ ₇ ₈ ₉ ₊ ₋ ₌ ₍ ₎
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use termionix_ansicodes::{Style, Script, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let superscript_style = Style {
///     script: Some(Script::Superscript),
///     ..Default::default()
/// };
///
/// let mut output = String::new();
/// superscript_style.write_style(&mut output, Some(&config)).unwrap();
/// // Generates: "\x1b[73m"
/// ```
///
/// Subscript usage:
///
/// ```
/// use termionix_ansicodes::{Style, Script, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let subscript_style = Style {
///     script: Some(Script::Subscript),
///     ..Default::default()
/// };
///
/// let mut output = String::new();
/// subscript_style.write_style(&mut output, Some(&config)).unwrap();
/// // Generates: "\x1b[74m"
/// ```
///
/// Resetting to normal position:
///
/// ```
/// use termionix_ansicodes::{Style, Script, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let normal_style = Style {
///     script: Some(Script::Normal),
///     ..Default::default()
/// };
///
/// let mut output = String::new();
/// normal_style.write_style(&mut output, Some(&config)).unwrap();
/// // Generates: "\x1b[75m"
/// ```
///
/// Mathematical expression example (conceptual):
///
/// ```
/// use termionix_ansicodes::{StyledString, Style, Script};
///
/// // Creating "x²" with superscript (if terminal supports it)
/// let mut expr = StyledString::empty();
/// expr.concat_with_style("x", Style::default());
/// expr.concat_with_style("2", Style {
///     script: Some(Script::Superscript),
///     ..Default::default()
/// });
/// ```
///
/// # Default
///
/// The default script position is [`Normal`](Script::Normal), which represents text
/// at the standard baseline position.
///
/// ```
/// use termionix_ansicodes::Script;
///
/// assert_eq!(Script::default(), Script::Normal);
/// ```
///
/// # Conversion
///
/// The `Script` enum can be converted to and from SGR codes:
///
/// ```
/// use termionix_ansicodes::Script;
///
/// // To SGR codes
/// assert_eq!(Script::Superscript.to_u8(), 73);
/// assert_eq!(Script::Subscript.to_u8(), 74);
/// assert_eq!(Script::Normal.to_u8(), 75);
///
/// // From SGR codes
/// assert_eq!(Script::from_u8(73), Some(Script::Superscript));
/// assert_eq!(Script::from_u8(74), Some(Script::Subscript));
/// assert_eq!(Script::from_u8(75), Some(Script::Normal));
/// assert_eq!(Script::from_u8(99), None); // Invalid code
/// ```
///
/// # Notes
///
/// - Script positioning does not persist across style resets (`\x1b[0m`)
/// - Multiple script selections override each other; only the last one takes effect
/// - The exact visual appearance (size, position) depends on terminal implementation
/// - Script codes are part of the ECMA-48 standard but rarely implemented
///
/// # Historical Context
///
/// Script positioning codes were included in the ANSI/ECMA-48 standard to support
/// advanced text formatting capabilities, but they never gained widespread adoption
/// in terminal emulators. Most modern terminals focus on color and basic styling
/// support instead.
///
/// # Recommendations
///
/// For production code:
/// 1. **Prefer Unicode characters** for subscript/superscript when possible
/// 2. **Test thoroughly** if using ANSI script codes with your target terminals
/// 3. **Provide fallbacks** for terminals that don't support these codes
/// 4. **Document requirements** clearly for users
///
/// # See Also
///
/// - [`AnsiSelectGraphicRendition`] - Container for text styling including script positioning
/// - [`Font`] - Font selection (also has limited terminal support)
/// - [ECMA-48 Standard](https://www.ecma-international.org/publications-and-standards/standards/ecma-48/)
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum Script {
    /// Normal baseline position (default) - SGR code 75.
    ///
    /// Text is positioned at the standard baseline, neither raised nor lowered.
    /// This is the default position and is used to reset any superscript or
    /// subscript positioning.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Script;
    /// assert_eq!(Script::Normal.to_u8(), 75);
    /// ```
    #[default]
    Normal,

    /// Superscript (raised) position - SGR code 73.
    ///
    /// Text is positioned above the normal baseline, typically rendered smaller
    /// and higher than regular text. Commonly used for exponents, ordinal
    /// indicators, and footnote references.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Script;
    /// assert_eq!(Script::Superscript.to_u8(), 73);
    /// ```
    ///
    /// # Common Uses
    ///
    /// - Mathematical exponents: x², 10³
    /// - Ordinal indicators: 1ˢᵗ, 2ⁿᵈ
    /// - Footnote references: text¹
    /// - Trademark symbols: TM™
    Superscript,

    /// Subscript (lowered) position - SGR code 74.
    ///
    /// Text is positioned below the normal baseline, typically rendered smaller
    /// and lower than regular text. Commonly used for chemical formulas, array
    /// indices, and mathematical notation.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Script;
    /// assert_eq!(Script::Subscript.to_u8(), 74);
    /// ```
    ///
    /// # Common Uses
    ///
    /// - Chemical formulas: H₂O, CO₂
    /// - Array indices: a₁, a₂, a₃
    /// - Mathematical notation: log₂, x₀
    /// - Variable subscripts: vₘₐₓ
    Subscript,
}

impl Script {
    /// Converts the script variant to its corresponding ANSI SGR code.
    ///
    /// This method returns the numeric SGR parameter code used in ANSI escape
    /// sequences to select this script positioning mode.
    ///
    /// # Returns
    ///
    /// The ANSI SGR code as a `u8`:
    /// - `Superscript` → `73` - Raised text (superscript)
    /// - `Subscript` → `74` - Lowered text (subscript)
    /// - `Normal` → `75` - Normal baseline position (reset)
    ///
    /// # Examples
    ///
    /// Basic conversion:
    ///
    /// ```
    /// use termionix_ansicodes::Script;
    ///
    /// assert_eq!(Script::Superscript.to_u8(), 73);
    /// assert_eq!(Script::Subscript.to_u8(), 74);
    /// assert_eq!(Script::Normal.to_u8(), 75);
    /// ```
    ///
    /// Building ANSI sequences:
    ///
    /// ```
    /// use termionix_ansicodes::Script;
    ///
    /// let script = Script::Superscript;
    /// let code = script.to_u8();
    /// let ansi_sequence = format!("\x1b[{}m", code);
    /// assert_eq!(ansi_sequence, "\x1b[73m");
    /// ```
    ///
    /// Iterating through all script modes:
    ///
    /// ```
    /// use termionix_ansicodes::Script;
    ///
    /// let scripts = vec![
    ///     (Script::Superscript, 73),
    ///     (Script::Subscript, 74),
    ///     (Script::Normal, 75),
    /// ];
    ///
    /// for (script, expected_code) in scripts {
    ///     assert_eq!(script.to_u8(), expected_code);
    /// }
    /// ```
    ///
    /// # Use in ANSI Sequences
    ///
    /// The returned code is used directly in ANSI escape sequences:
    ///
    /// ```text
    /// \x1b[<code>m
    /// ```
    ///
    /// For example, `Script::Superscript.to_u8()` returns `73`, which generates
    /// the sequence `\x1b[73m`.
    ///
    /// # Performance
    ///
    /// This is a simple constant-time operation that performs a single pattern match.
    ///
    /// # See Also
    ///
    /// - [`from_u8`](Script::from_u8) - Converts SGR code to `Script` variant
    /// - [`AnsiSelectGraphicRendition::write`] - Uses this method when rendering script codes
    pub fn to_u8(&self) -> u8 {
        match self {
            Script::Superscript => 73,
            Script::Subscript => 74,
            Script::Normal => 75,
        }
    }

    /// Converts an ANSI SGR code to its corresponding `Script` variant.
    ///
    /// This method attempts to parse a numeric ANSI SGR code into a `Script` value.
    /// It recognizes the standard script positioning codes: 73 (superscript),
    /// 74 (subscript), and 75 (normal).
    ///
    /// # Arguments
    ///
    /// * `value` - The ANSI SGR code to convert
    ///
    /// # Returns
    ///
    /// - `Some(Script::Superscript)` if `value` is `73`
    /// - `Some(Script::Subscript)` if `value` is `74`
    /// - `Some(Script::Normal)` if `value` is `75`
    /// - `None` for any other value
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use termionix_ansicodes::Script;
    ///
    /// assert_eq!(Script::from_u8(73), Some(Script::Superscript));
    /// assert_eq!(Script::from_u8(74), Some(Script::Subscript));
    /// assert_eq!(Script::from_u8(75), Some(Script::Normal));
    /// ```
    ///
    /// Handling invalid codes:
    ///
    /// ```
    /// use termionix_ansicodes::Script;
    ///
    /// assert_eq!(Script::from_u8(0), None);
    /// assert_eq!(Script::from_u8(72), None);
    /// assert_eq!(Script::from_u8(76), None);
    /// assert_eq!(Script::from_u8(99), None);
    /// ```
    ///
    /// Parsing ANSI sequences:
    ///
    /// ```
    /// use termionix_ansicodes::Script;
    ///
    /// // Parse a code from an ANSI sequence parameter
    /// let sgr_params: Vec<u8> = vec![1, 73, 31]; // Bold, Superscript, Red
    ///
    /// for param in sgr_params {
    ///     if let Some(script) = Script::from_u8(param) {
    ///         println!("Found script positioning: {:?}", script);
    ///     }
    /// }
    /// ```
    ///
    /// Round-trip conversion:
    ///
    /// ```
    /// use termionix_ansicodes::Script;
    ///
    /// let original = Script::Subscript;
    /// let code = original.to_u8();
    /// let parsed = Script::from_u8(code);
    ///
    /// assert_eq!(parsed, Some(original));
    /// ```
    ///
    /// Using in a parser:
    ///
    /// ```
    /// use termionix_ansicodes::Script;
    ///
    /// fn parse_sgr_code(code: u8) -> Option<String> {
    ///     Script::from_u8(code).map(|script| {
    ///         format!("Script positioning: {:?}", script)
    ///     })
    /// }
    ///
    /// assert_eq!(
    ///     parse_sgr_code(73),
    ///     Some("Script positioning: Superscript".to_string())
    /// );
    /// assert_eq!(parse_sgr_code(99), None);
    /// ```
    ///
    /// # Use in ANSI Parsing
    ///
    /// This method is primarily used when parsing ANSI escape sequences to detect
    /// script positioning codes:
    ///
    /// ```text
    /// Input:  "\x1b[73m"
    /// Parsed: [73]
    /// Result: Script::from_u8(73) → Some(Script::Superscript)
    /// ```
    ///
    /// # Performance
    ///
    /// O(1) - Constant-time pattern matching with 3 branches.
    ///
    /// # Notes
    ///
    /// - This method only recognizes script positioning codes (73-75)
    /// - Other SGR codes (like 0 for reset, 1 for bold, etc.) return `None`
    /// - The method is used internally by [`AnsiSelectGraphicRendition::parse`] when processing SGR parameters
    ///
    /// # See Also
    ///
    /// - [`to_u8()`](Script::to_u8) - Converts a `Script` to its SGR code
    /// - [`AnsiSelectGraphicRendition::parse`] - Parses complete SGR sequences including script codes
    /// - [`Intensity::from_u8`] - Similar conversion for intensity codes
    /// - [`Font::from_u8`] - Similar conversion for font codes
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            73 => Some(Script::Superscript),
            74 => Some(Script::Subscript),
            75 => Some(Script::Normal),
            _ => None,
        }
    }
}

/// Text decoration modes for ideographic (CJK) characters.
///
/// The `Ideogram` enum provides special text decoration options specifically designed
/// for ideographic writing systems such as Chinese, Japanese, and Korean (CJK) characters.
/// These decorations include underlines, overlines, and stress marking that can be
/// positioned differently than standard Latin text decorations.
///
/// # ANSI SGR Codes
///
/// Each variant corresponds to a specific SGR (Select Graphic Rendition) code:
///
/// | Variant | SGR Code | Description |
/// |---------|----------|-------------|
/// | `Underline` | 60 | Underline or right side line |
/// | `DoubleUnderline` | 61 | Double underline or double right side line |
/// | `Overline` | 62 | Overline or left side line |
/// | `DoubleOverline` | 63 | Double overline or double left side line |
/// | `StressMarking` | 64 | Stress marking emphasis |
/// | `NoIdeogramAttributes` | 65 | Reset ideogram decorations to none |
///
/// # Terminal Support
///
/// **Warning**: Ideogram decoration codes have extremely limited support across terminal emulators:
///
/// - Most modern terminals (iTerm2, GNOME Terminal, Windows Terminal, Alacritty, kitty)
///   **do not support** ideogram-specific decorations
/// - These codes were designed for specialized CJK terminal environments
/// - When unsupported, terminals typically ignore these codes without error
/// - Even terminals with CJK support often don't implement these decorations
///
/// # Ideographic Writing Systems
///
/// These decorations are specifically designed for:
/// - **Chinese** (Hanzi characters)
/// - **Japanese** (Kanji and Kana)
/// - **Korean** (Hangul and Hanja)
/// - Other East Asian character systems
///
/// In vertical text layouts, "underline" may appear as a right side line, and "overline"
/// as a left side line, which is why the variants have alternative descriptions.
///
/// # Use Cases
///
/// Ideogram decorations are primarily used for:
/// - Emphasis in CJK text (similar to bold or italic in Latin text)
/// - Ruby text annotations (furigana in Japanese)
/// - Phonetic guides (pinyin in Chinese, romaji in Japanese)
/// - Semantic emphasis or stress marking
/// - Traditional text formatting in Asian typography
/// - Legacy terminal applications supporting CJK
///
/// # Modern Alternatives
///
/// For modern CJK text emphasis, consider:
/// - Standard [`Underline`] styles (more widely supported)
/// - [`Intensity::Bold`] for emphasis
/// - Unicode combining characters for diacritics
/// - HTML ruby annotations for web display
/// - Terminal-specific rich text formats
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use termionix_ansicodes::{Style, Ideogram, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let ideogram_style = Style {
///     ideogram: Some(Ideogram::Underline),
///     ..Default::default()
/// };
///
/// let mut output = String::new();
/// ideogram_style.write_style(&mut output, Some(&config)).unwrap();
/// // Generates: "\x1b[60m" (rarely supported)
/// ```
///
/// Double overline for emphasis:
///
/// ```
/// use termionix_ansicodes::{Ideogram, Style, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let emphasis_style = Style {
///     ideogram: Some(Ideogram::DoubleOverline),
///     ..Default::default()
/// };
///
/// let mut output = String::new();
/// emphasis_style.write_style(&mut output, Some(&config)).unwrap();
/// // Generates: "\x1b[63m"
/// ```
///
/// Stress marking:
///
/// ```
/// use termionix_ansicodes::{Ideogram, Style, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let stress_style = Style {
///     ideogram: Some(Ideogram::StressMarking),
///     ..Default::default()
/// };
///
/// let mut output = String::new();
/// stress_style.write_style(&mut output, Some(&config)).unwrap();
/// // Generates: "\x1b[64m"
/// ```
///
/// Resetting ideogram attributes:
///
/// ```
/// use termionix_ansicodes::{Ideogram, Style, ColorMode, AnsiConfig};
///
/// let config = AnsiConfig::enabled();
/// let reset_style = Style {
///     ideogram: Some(Ideogram::NoIdeogramAttributes),
///     ..Default::default()
/// };
///
/// let mut output = String::new();
/// reset_style.write_style(&mut output, Some(&config)).unwrap();
/// // Generates: "\x1b[65m"
/// ```
///
/// # Default
///
/// There is no default value for `Ideogram`. The `Style` struct uses
/// `Option<Ideogram>` where `None` means no ideogram decoration is applied.
///
/// # Conversion
///
/// The `Ideogram` enum can be converted to and from SGR codes:
///
/// ```
/// use termionix_ansicodes::Ideogram;
///
/// // To SGR codes
/// assert_eq!(Ideogram::Underline.to_u8(), 60);
/// assert_eq!(Ideogram::DoubleUnderline.to_u8(), 61);
/// assert_eq!(Ideogram::Overline.to_u8(), 62);
/// assert_eq!(Ideogram::DoubleOverline.to_u8(), 63);
/// assert_eq!(Ideogram::StressMarking.to_u8(), 64);
/// assert_eq!(Ideogram::NoIdeogramAttributes.to_u8(), 65);
///
/// // From SGR codes
/// assert_eq!(Ideogram::from_u8(60), Some(Ideogram::Underline));
/// assert_eq!(Ideogram::from_u8(64), Some(Ideogram::StressMarking));
/// assert_eq!(Ideogram::from_u8(99), None); // Invalid code
/// ```
///
/// # Notes
///
/// - Ideogram decorations do not persist across style resets (`\x1b[0m`)
/// - Multiple ideogram selections override each other; only the last one takes effect
/// - The exact visual appearance depends on terminal implementation and font support
/// - In vertical text, "underline" and "overline" become side lines
/// - Setting `ideogram: None` in a [`AnsiSelectGraphicRendition`] means no ideogram code is generated
///
/// # Historical Context
///
/// Ideogram decoration codes were included in the ECMA-48 standard to support
/// advanced text formatting in CJK computing environments, particularly for
/// Japanese terminals in the 1980s and 1990s. However, they never gained
/// widespread adoption outside of specialized systems.
///
/// # Recommendations
///
/// For production code:
/// 1. **Avoid relying on ideogram codes** unless targeting specific legacy systems
/// 2. **Test thoroughly** with target CJK terminals before deployment
/// 3. **Provide fallbacks** using standard underline or bold styling
/// 4. **Document limitations** clearly for users
/// 5. **Consider Unicode alternatives** for better cross-platform support
///
/// # See Also
///
/// - [`AnsiSelectGraphicRendition`] - Container for text styling including ideogram decorations
/// - [`Underline`] - Standard underline styles (more widely supported)
/// - [`Font`] - Font selection (similarly has limited support)
/// - [ECMA-48 Standard](https://www.ecma-international.org/publications-and-standards/standards/ecma-48/)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Ideogram {
    /// Ideogram underline or right side line - SGR code 60.
    ///
    /// In horizontal text layouts, this applies an underline beneath ideographic
    /// characters. In vertical text layouts, this may appear as a line on the
    /// right side of the characters.
    ///
    /// This decoration is rarely supported in modern terminal emulators.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    /// assert_eq!(Ideogram::Underline.to_u8(), 60);
    /// ```
    Underline,

    /// Ideogram double underline or double right side line - SGR code 61.
    ///
    /// Similar to [`Underline`](Ideogram::Underline), but with two parallel lines
    /// instead of one. Provides stronger emphasis than a single underline.
    ///
    /// In horizontal layouts: double line beneath characters
    /// In vertical layouts: double line on the right side of characters
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    /// assert_eq!(Ideogram::DoubleUnderline.to_u8(), 61);
    /// ```
    DoubleUnderline,

    /// Ideogram overline or left side line - SGR code 62.
    ///
    /// In horizontal text layouts, this applies a line above ideographic characters.
    /// In vertical text layouts, this may appear as a line on the left side of
    /// the characters.
    ///
    /// Commonly used for emphasis or to indicate readings/annotations in CJK text.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    /// assert_eq!(Ideogram::Overline.to_u8(), 62);
    /// ```
    Overline,

    /// Ideogram double overline or double left side line - SGR code 63.
    ///
    /// Similar to [`Overline`](Ideogram::Overline), but with two parallel lines
    /// instead of one. Provides stronger emphasis than a single overline.
    ///
    /// In horizontal layouts: double line above characters
    /// In vertical layouts: double line on the left side of characters
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    /// assert_eq!(Ideogram::DoubleOverline.to_u8(), 63);
    /// ```
    DoubleOverline,

    /// Ideogram stress marking - SGR code 64.
    ///
    /// Applies stress or emphasis marking to ideographic characters. The exact
    /// visual representation depends on the terminal implementation and may
    /// include dots, marks, or other indicators positioned around the characters
    /// to show emphasis.
    ///
    /// In Japanese typography, this might correspond to 傍点 (bōten) or emphasis dots.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    /// assert_eq!(Ideogram::StressMarking.to_u8(), 64);
    /// ```
    StressMarking,

    /// No ideogram attributes (reset) - SGR code 65.
    ///
    /// Resets all ideogram-specific decorations to none, returning to normal
    /// character display. This doesn't affect other text attributes like color
    /// or standard underlines.
    ///
    /// Use this to explicitly clear any previously set ideogram decorations.
    ///
    /// # Example
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    /// assert_eq!(Ideogram::NoIdeogramAttributes.to_u8(), 65);
    /// ```
    #[default]
    NoIdeogramAttributes,
}

impl Ideogram {
    /// Converts the ideogram variant to its corresponding ANSI SGR code.
    ///
    /// This method returns the numeric SGR parameter code used in ANSI escape
    /// sequences to select this ideogram decoration mode. The codes range from
    /// 60 (underline) to 65 (no attributes).
    ///
    /// # Returns
    ///
    /// The ANSI SGR code as a `u8`:
    /// - `Underline` → `60` - Underline or right side line
    /// - `DoubleUnderline` → `61` - Double underline or double right side line
    /// - `Overline` → `62` - Overline or left side line
    /// - `DoubleOverline` → `63` - Double overline or double left side line
    /// - `StressMarking` → `64` - Stress marking
    /// - `NoIdeogramAttributes` → `65` - Reset ideogram attributes
    ///
    /// # Examples
    ///
    /// Basic conversion:
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    ///
    /// assert_eq!(Ideogram::Underline.to_u8(), 60);
    /// assert_eq!(Ideogram::DoubleUnderline.to_u8(), 61);
    /// assert_eq!(Ideogram::Overline.to_u8(), 62);
    /// assert_eq!(Ideogram::DoubleOverline.to_u8(), 63);
    /// assert_eq!(Ideogram::StressMarking.to_u8(), 64);
    /// assert_eq!(Ideogram::NoIdeogramAttributes.to_u8(), 65);
    /// ```
    ///
    /// Building ANSI sequences:
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    ///
    /// let ideogram = Ideogram::StressMarking;
    /// let code = ideogram.to_u8();
    /// let ansi_sequence = format!("\x1b[{}m", code);
    /// assert_eq!(ansi_sequence, "\x1b[64m");
    /// ```
    ///
    /// # Use in ANSI Sequences
    ///
    /// The returned code is used directly in ANSI escape sequences:
    ///
    /// ```text
    /// \x1b[<code>m
    /// ```
    ///
    /// For example, `Ideogram::Underline.to_u8()` returns `60`, which generates
    /// the sequence `\x1b[60m`.
    ///
    /// # Performance
    ///
    /// This is a simple constant-time operation that performs a single pattern match.
    ///
    /// # See Also
    ///
    /// - [`from_u8`](Ideogram::from_u8) - Converts SGR code to `Ideogram` variant
    /// - [`AnsiSelectGraphicRendition::write`] - Uses this method when rendering ideogram codes
    pub fn to_u8(&self) -> u8 {
        match self {
            Ideogram::Underline => 60,
            Ideogram::DoubleUnderline => 61,
            Ideogram::Overline => 62,
            Ideogram::DoubleOverline => 63,
            Ideogram::StressMarking => 64,
            Ideogram::NoIdeogramAttributes => 65,
        }
    }

    /// Converts an ANSI SGR code to its corresponding `Ideogram` variant.
    ///
    /// This method attempts to parse a numeric ANSI SGR code into an `Ideogram` value.
    /// It recognizes the standard ideogram decoration codes from 60 through 65.
    ///
    /// # Arguments
    ///
    /// * `value` - The ANSI SGR code to convert
    ///
    /// # Returns
    ///
    /// - `Some(Ideogram::Underline)` if `value` is `60`
    /// - `Some(Ideogram::DoubleUnderline)` if `value` is `61`
    /// - `Some(Ideogram::Overline)` if `value` is `62`
    /// - `Some(Ideogram::DoubleOverline)` if `value` is `63`
    /// - `Some(Ideogram::StressMarking)` if `value` is `64`
    /// - `Some(Ideogram::NoIdeogramAttributes)` if `value` is `65`
    /// - `None` for any other value
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    ///
    /// assert_eq!(Ideogram::from_u8(60), Some(Ideogram::Underline));
    /// assert_eq!(Ideogram::from_u8(61), Some(Ideogram::DoubleUnderline));
    /// assert_eq!(Ideogram::from_u8(62), Some(Ideogram::Overline));
    /// assert_eq!(Ideogram::from_u8(63), Some(Ideogram::DoubleOverline));
    /// assert_eq!(Ideogram::from_u8(64), Some(Ideogram::StressMarking));
    /// assert_eq!(Ideogram::from_u8(65), Some(Ideogram::NoIdeogramAttributes));
    /// ```
    ///
    /// Handling invalid codes:
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    ///
    /// assert_eq!(Ideogram::from_u8(0), None);
    /// assert_eq!(Ideogram::from_u8(59), None);
    /// assert_eq!(Ideogram::from_u8(66), None);
    /// assert_eq!(Ideogram::from_u8(99), None);
    /// ```
    ///
    /// Parsing ANSI sequences:
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    ///
    /// // Parse a code from an ANSI sequence parameter
    /// let sgr_params: Vec<u8> = vec![1, 64, 31]; // Bold, StressMarking, Red
    ///
    /// for param in sgr_params {
    ///     if let Some(ideogram) = Ideogram::from_u8(param) {
    ///         println!("Found ideogram decoration: {:?}", ideogram);
    ///     }
    /// }
    /// ```
    ///
    /// Round-trip conversion:
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    ///
    /// let original = Ideogram::StressMarking;
    /// let code = original.to_u8();
    /// let parsed = Ideogram::from_u8(code);
    ///
    /// assert_eq!(parsed, Some(original));
    /// ```
    ///
    /// Using in a parser:
    ///
    /// ```
    /// use termionix_ansicodes::Ideogram;
    ///
    /// fn parse_sgr_code(code: u8) -> Option<String> {
    ///     Ideogram::from_u8(code).map(|ideogram| {
    ///         format!("Ideogram decoration: {:?}", ideogram)
    ///     })
    /// }
    ///
    /// assert_eq!(
    ///     parse_sgr_code(64),
    ///     Some("Ideogram decoration: StressMarking".to_string())
    /// );
    /// assert_eq!(parse_sgr_code(99), None);
    /// ```
    ///
    /// # Use in ANSI Parsing
    ///
    /// This method is primarily used when parsing ANSI escape sequences to detect
    /// ideogram decoration codes:
    ///
    /// ```text
    /// Input:  "\x1b[64m"
    /// Parsed: [64]
    /// Result: Ideogram::from_u8(64) → Some(Ideogram::StressMarking)
    /// ```
    ///
    /// # Performance
    ///
    /// O(1) - Constant-time pattern matching with 6 branches.
    ///
    /// # Notes
    ///
    /// - This method only recognizes ideogram decoration codes (60-65)
    /// - Other SGR codes (like 0 for reset, 1 for bold, etc.) return `None`
    /// - The method is used internally by [`AnsiSelectGraphicRendition::parse`] when processing SGR parameters
    ///
    /// # See Also
    ///
    /// - [`to_u8()`](Ideogram::to_u8) - Converts an `Ideogram` to its SGR code
    /// - [`AnsiSelectGraphicRendition::parse`] - Parses complete SGR sequences including ideogram codes
    /// - [`Font::from_u8`] - Similar conversion for font codes
    /// - [`Script::from_u8`] - Similar conversion for script positioning codes
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            60 => Some(Ideogram::Underline),
            61 => Some(Ideogram::DoubleUnderline),
            62 => Some(Ideogram::Overline),
            63 => Some(Ideogram::DoubleOverline),
            64 => Some(Ideogram::StressMarking),
            65 => Some(Ideogram::NoIdeogramAttributes),
            _ => None,
        }
    }
}

/// SGR (Select Graphic Rendition) Parameter
///
/// Represents additional ANSI escape sequence SGR parameters that control text
/// rendering attributes beyond the basic style properties like intensity, italic,
/// underline, etc. These are less commonly used SGR codes that don't fit into
/// the main `Style` struct fields.
///
/// # SGR Codes
///
/// SGR codes are part of ANSI escape sequences in the format `CSI n m` where
/// `n` is the numeric parameter. This enum represents SGR codes that are not
/// covered by the primary `Style` struct fields.
///
/// # Variants
///
/// ## Color Reset
/// - `DefaultForegroundColor` - Resets foreground color to terminal default (SGR 39)
/// - `DefaultBackgroundColor` - Resets background color to terminal default (SGR 49)
///
/// ## Text Effects
/// - `Framed` - Adds a frame around text (SGR 51)
/// - `Encircled` - Draws a circle around text (SGR 52)
/// - `Overlined` - Draws a line above text (SGR 53)
/// - `NotFramedNotEncircled` - Removes frame or encircle effects (SGR 54)
/// - `NotOverlined` - Removes overline effect (SGR 55)
///
/// ## Spacing
/// - `DisableProportionalSpacing` - Disables proportional character spacing (SGR 50)
///
/// ## Underline Color
/// - `SetUnderlineColor` - Sets the color of underlines independently from text color (SGR 58)
/// - `DefaultUnderlineColor` - Resets underline color to default (SGR 59)
///
/// ## Unknown
/// - `Unknown` - Represents unrecognized or unsupported SGR parameters
///
/// # Terminal Support
///
/// Note that many of these parameters have limited support across different terminal
/// emulators. The most widely supported are the color reset parameters
/// (`DefaultForegroundColor` and `DefaultBackgroundColor`). Effects like `Framed`,
/// `Encircled`, and `Overlined` are rarely implemented.
///
/// # Examples
///
/// ```rust
/// use termionix_ansicodes::{SGRParameter, Color};
///
/// // Reset foreground to default
/// let param = SGRParameter::DefaultForegroundColor;
///
/// // Set a custom underline color
/// let underline_param = SGRParameter::SetUnderlineColor(Color::Red);
///
/// // Handle unknown parameter
/// let unknown = SGRParameter::Unknown(99);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SGRParameter {
    /// 39 - Default foreground color
    ///
    /// Resets the foreground (text) color to the terminal's default color.
    /// This is equivalent to "unset" rather than setting to a specific color.
    DefaultForegroundColor,

    /// 49 - Default background color
    ///
    /// Resets the background color to the terminal's default color.
    /// This is equivalent to "unset" rather than setting to a specific color.
    DefaultBackgroundColor,

    /// 50 - Disable proportional spacing
    ///
    /// Disables proportional spacing and returns to monospace rendering.
    /// This SGR code is rarely supported in modern terminal emulators.
    DisableProportionalSpacing,

    /// 51 - Framed
    ///
    /// Draws a frame (border) around the text. This effect is rarely
    /// supported in terminal emulators and its exact rendering is
    /// implementation-dependent.
    Framed,

    /// 52 - Encircled
    ///
    /// Draws a circle around the text. This effect is rarely supported
    /// in terminal emulators and its exact rendering is implementation-dependent.
    Encircled,

    /// 53 - Overlined
    ///
    /// Draws a line above the text, similar to underline but on top.
    /// Support varies across terminal emulators.
    Overlined,

    /// 54 - Neither framed nor encircled
    ///
    /// Removes both framed (SGR 51) and encircled (SGR 52) effects.
    NotFramedNotEncircled,

    /// 55 - Not overlined
    ///
    /// Removes the overline effect (SGR 53).
    NotOverlined,

    /// 58 - Set underline color (extended color)
    ///
    /// Sets the color of underlines independently from the text color.
    /// This allows for colored underlines while maintaining different text colors.
    ///
    /// The color can be specified using any `Color` variant:
    /// - Basic colors (e.g., `Color::Red`)
    /// - 256-color palette (`Color::Fixed(n)`)
    /// - True color RGB (`Color::RGB(r, g, b)`)
    ///
    /// # ANSI Format
    /// - 58;5;n for 256-color mode
    /// - 58;2;r;g;b for RGB/true color mode
    ///
    /// # Example
    /// ```rust
    /// use termionix_ansicodes::{SGRParameter, Color};
    ///
    /// // Red underline
    /// let red_underline = SGRParameter::SetUnderlineColor(Color::Red);
    ///
    /// // Custom RGB underline
    /// let purple = SGRParameter::SetUnderlineColor(Color::RGB(128, 0, 128));
    /// ```
    SetUnderlineColor(Color),

    /// 59 - Default underline color
    ///
    /// Resets the underline color to the terminal's default, typically
    /// matching the text color.
    DefaultUnderlineColor,

    /// Unknown SGR parameter
    ///
    /// Represents an SGR code that is not recognized or not explicitly
    /// supported by this library. The numeric value of the unrecognized
    /// code is stored for debugging or pass-through purposes.
    ///
    /// # Example
    /// ```rust
    /// use termionix_ansicodes::SGRParameter;
    ///
    /// let unknown = SGRParameter::Unknown(99);
    /// ```
    Unknown(u8),
}

impl SGRParameter {
    /// Converts the SGR parameter to its corresponding ANSI SGR code.
    ///
    /// This method returns the numeric SGR parameter code used in ANSI escape
    /// sequences for this parameter type.
    ///
    /// # Returns
    ///
    /// The ANSI SGR code as a `u8`:
    /// - `DefaultForegroundColor` → `39`
    /// - `DefaultBackgroundColor` → `49`
    /// - `DisableProportionalSpacing` → `50`
    /// - `Framed` → `51`
    /// - `Encircled` → `52`
    /// - `Overlined` → `53`
    /// - `NotFramedNotEncircled` → `54`
    /// - `NotOverlined` → `55`
    /// - `SetUnderlineColor(_)` → `58` (note: color data requires additional parameters)
    /// - `DefaultUnderlineColor` → `59`
    /// - `Unknown(n)` → `n` (returns the stored value)
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodes::{SGRParameter, Color};
    ///
    /// assert_eq!(SGRParameter::DefaultForegroundColor.to_u8(), 39);
    /// assert_eq!(SGRParameter::Overlined.to_u8(), 53);
    /// assert_eq!(SGRParameter::SetUnderlineColor(Color::Red).to_u8(), 58);
    /// assert_eq!(SGRParameter::Unknown(99).to_u8(), 99);
    /// ```
    pub fn to_u8(&self) -> u8 {
        match self {
            SGRParameter::DefaultForegroundColor => 39,
            SGRParameter::DefaultBackgroundColor => 49,
            SGRParameter::DisableProportionalSpacing => 50,
            SGRParameter::Framed => 51,
            SGRParameter::Encircled => 52,
            SGRParameter::Overlined => 53,
            SGRParameter::NotFramedNotEncircled => 54,
            SGRParameter::NotOverlined => 55,
            SGRParameter::SetUnderlineColor(_) => 58,
            SGRParameter::DefaultUnderlineColor => 59,
            SGRParameter::Unknown(value) => *value,
        }
    }

    /// Converts an ANSI SGR code to its corresponding `SGRParameter` variant.
    ///
    /// This method attempts to parse a numeric ANSI SGR code into an `SGRParameter` value.
    /// Note that for `SetUnderlineColor`, this only recognizes the base code (58) and
    /// cannot parse the color data, so it returns `None` for that code.
    ///
    /// # Arguments
    ///
    /// * `value` - The ANSI SGR code to convert
    ///
    /// # Returns
    ///
    /// - `Some(SGRParameter::DefaultForegroundColor)` if `value` is `39`
    /// - `Some(SGRParameter::DefaultBackgroundColor)` if `value` is `49`
    /// - `Some(SGRParameter::DisableProportionalSpacing)` if `value` is `50`
    /// - `Some(SGRParameter::Framed)` if `value` is `51`
    /// - `Some(SGRParameter::Encircled)` if `value` is `52`
    /// - `Some(SGRParameter::Overlined)` if `value` is `53`
    /// - `Some(SGRParameter::NotFramedNotEncircled)` if `value` is `54`
    /// - `Some(SGRParameter::NotOverlined)` if `value` is `55`
    /// - `Some(SGRParameter::DefaultUnderlineColor)` if `value` is `59`
    /// - `None` for `58` (requires additional color parameters to parse)
    /// - `None` for any other value
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodes::SGRParameter;
    ///
    /// assert_eq!(
    ///     SGRParameter::from_u8(39),
    ///     Some(SGRParameter::DefaultForegroundColor)
    /// );
    /// assert_eq!(
    ///     SGRParameter::from_u8(53),
    ///     Some(SGRParameter::Overlined)
    /// );
    /// assert_eq!(SGRParameter::from_u8(58), None); // Requires color parameters
    /// assert_eq!(SGRParameter::from_u8(99), None);
    /// ```
    pub fn from_u8(value: u8) -> Option<SGRParameter> {
        match value {
            39 => Some(SGRParameter::DefaultForegroundColor),
            49 => Some(SGRParameter::DefaultBackgroundColor),
            50 => Some(SGRParameter::DisableProportionalSpacing),
            51 => Some(SGRParameter::Framed),
            52 => Some(SGRParameter::Encircled),
            53 => Some(SGRParameter::Overlined),
            54 => Some(SGRParameter::NotFramedNotEncircled),
            55 => Some(SGRParameter::NotOverlined),
            58 => None, // SetUnderlineColor requires additional parameters
            59 => Some(SGRParameter::DefaultUnderlineColor),
            _ => None,
        }
    }
}

impl std::fmt::Display for AnsiSelectGraphicRendition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        AnsiSelectGraphicRendition::write_str(self, f, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorMode;

    #[test]
    fn test_style_default() {
        let style = AnsiSelectGraphicRendition::default();
        assert_eq!(style.foreground, None);
        assert_eq!(style.background, None);
        assert_eq!(style.intensity, None);
        assert_eq!(style.italic, None);
        assert_eq!(style.underline, None);
        assert_eq!(style.blink, None);
        assert_eq!(style.reverse, None);
        assert_eq!(style.hidden, None);
        assert_eq!(style.strike, None);
        assert_eq!(style.font, None);
    }

    #[test]
    fn test_write_style_empty_style() {
        let style = AnsiSelectGraphicRendition::default();

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::Basic))
            .unwrap();
        assert_eq!(output, "");
    }

    #[test]
    fn test_write_style_no_color_mode() {
        let style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            foreground: Some(Color::Red),
            ..Default::default()
        };

        let mut output = String::new();

        style.write_str(&mut output, Some(ColorMode::None)).unwrap();
        assert_eq!(output, "");
    }

    #[test]
    fn test_write_style_bold() {
        let style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            ..Default::default()
        };

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::Basic))
            .unwrap();
        assert_eq!(output, "\x1b[1m");
    }

    #[test]
    fn test_write_style_dim() {
        let style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Dim),
            ..Default::default()
        };

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::Basic))
            .unwrap();
        assert_eq!(output, "\x1b[2m");
    }

    #[test]
    fn test_write_style_all_attributes() {
        let style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            italic: Some(true),
            underline: Some(Underline::Single),
            blink: Some(Blink::Slow),
            reverse: Some(true),
            hidden: Some(true),
            strike: Some(true),
            ..Default::default()
        };

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::TrueColor))
            .unwrap();
        assert_eq!(output, "\x1b[1;3;4;5;7;8;9m");
    }

    #[test]
    fn test_write_style_underline_variants() {
        let test_cases = vec![
            (Underline::Single, "\x1b[4m"),
            (Underline::Double, "\x1b[21m"),
            (Underline::Disabled, "\x1b[24m"),
        ];

        for (underline, expected) in test_cases {
            let style = AnsiSelectGraphicRendition {
                underline: Some(underline),
                ..Default::default()
            };
            let mut output = String::new();

            style
                .write_str(&mut output, Some(ColorMode::Basic))
                .unwrap();
            assert_eq!(output, expected, "Failed for underline: {:?}", underline);
        }
    }

    #[test]
    fn test_write_style_blink_variants() {
        let test_cases = vec![
            (Blink::Slow, "\x1b[5m"),
            (Blink::Rapid, "\x1b[6m"),
            (Blink::Off, "\x1b[25m"),
        ];

        for (blink, expected) in test_cases {
            let style = AnsiSelectGraphicRendition {
                blink: Some(blink),
                ..Default::default()
            };

            let mut output = String::new();

            style
                .write_str(&mut output, Some(ColorMode::Basic))
                .unwrap();
            assert_eq!(output, expected, "Failed for blink: {:?}", blink);
        }
    }

    #[test]
    fn test_write_style_font() {
        let style = AnsiSelectGraphicRendition {
            font: Some(Font::AlternateFont1),
            ..Default::default()
        };

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::Basic))
            .unwrap();
        assert_eq!(output, "\x1b[11m");
    }

    #[test]
    fn test_write_style_foreground_basic_colors() {
        let test_cases = vec![
            (Color::Black, "\x1b[30m"),
            (Color::Red, "\x1b[31m"),
            (Color::Green, "\x1b[32m"),
            (Color::Yellow, "\x1b[33m"),
            (Color::Blue, "\x1b[34m"),
            (Color::Purple, "\x1b[35m"),
            (Color::Cyan, "\x1b[36m"),
            (Color::White, "\x1b[37m"),
        ];

        for (color, expected) in test_cases {
            let style = AnsiSelectGraphicRendition {
                foreground: Some(color),
                ..Default::default()
            };

            let mut output = String::new();

            style
                .write_str(&mut output, Some(ColorMode::Basic))
                .unwrap();
            assert_eq!(output, expected, "Failed for color: {:?}", color);
        }
    }

    #[test]
    fn test_write_style_background_basic_colors() {
        let test_cases = vec![
            (Color::Black, "\x1b[40m"),
            (Color::Red, "\x1b[41m"),
            (Color::Green, "\x1b[42m"),
            (Color::Yellow, "\x1b[43m"),
            (Color::Blue, "\x1b[44m"),
            (Color::Purple, "\x1b[45m"),
            (Color::Cyan, "\x1b[46m"),
            (Color::White, "\x1b[47m"),
        ];

        for (color, expected) in test_cases {
            let style = AnsiSelectGraphicRendition {
                background: Some(color),
                ..Default::default()
            };

            let mut output = String::new();

            style
                .write_str(&mut output, Some(ColorMode::Basic))
                .unwrap();
            assert_eq!(output, expected, "Failed for color: {:?}", color);
        }
    }

    #[test]
    fn test_write_style_foreground_fixed_color() {
        let style = AnsiSelectGraphicRendition {
            foreground: Some(Color::Fixed(123)),
            ..Default::default()
        };

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::FixedColor))
            .unwrap();
        assert_eq!(output, "\x1b[38;5;123m");
    }

    #[test]
    fn test_write_style_background_fixed_color() {
        let style = AnsiSelectGraphicRendition {
            background: Some(Color::Fixed(200)),
            ..Default::default()
        };

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::FixedColor))
            .unwrap();
        assert_eq!(output, "\x1b[48;5;200m");
    }

    #[test]
    fn test_write_style_foreground_rgb_color() {
        let style = AnsiSelectGraphicRendition {
            foreground: Some(Color::RGB(255, 128, 64)),
            ..Default::default()
        };

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::TrueColor))
            .unwrap();
        assert_eq!(output, "\x1b[38;2;255;128;64m");
    }

    #[test]
    fn test_write_style_background_rgb_color() {
        let style = AnsiSelectGraphicRendition {
            background: Some(Color::RGB(10, 20, 30)),
            ..Default::default()
        };

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::TrueColor))
            .unwrap();
        assert_eq!(output, "\x1b[48;2;10;20;30m");
    }

    #[test]
    fn test_write_style_combined_attributes_and_colors() {
        let style = AnsiSelectGraphicRendition {
            foreground: Some(Color::Red),
            background: Some(Color::Blue),
            intensity: Some(Intensity::Bold),
            underline: Some(Underline::Single),
            ..Default::default()
        };

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::Basic))
            .unwrap();
        assert_eq!(output, "\x1b[1;4;31;44m");
    }

    #[test]
    fn test_write_style_complex_style() {
        let style = AnsiSelectGraphicRendition {
            foreground: Some(Color::RGB(255, 100, 50)),
            background: Some(Color::Fixed(234)),
            intensity: Some(Intensity::Bold),
            italic: Some(true),
            strike: Some(true),
            ..Default::default()
        };

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::TrueColor))
            .unwrap();
        assert_eq!(output, "\x1b[1;3;9;38;2;255;100;50;48;5;234m");
    }

    #[test]
    fn test_write_style_individual_attributes() {
        let attributes = vec![
            (
                "italic",
                AnsiSelectGraphicRendition {
                    italic: Some(true),
                    ..Default::default()
                },
                "\x1b[3m",
            ),
            (
                "underline",
                AnsiSelectGraphicRendition {
                    underline: Some(Underline::Single),
                    ..Default::default()
                },
                "\x1b[4m",
            ),
            (
                "double_underline",
                AnsiSelectGraphicRendition {
                    underline: Some(Underline::Double),
                    ..Default::default()
                },
                "\x1b[21m",
            ),
            (
                "blink_slow",
                AnsiSelectGraphicRendition {
                    blink: Some(Blink::Slow),
                    ..Default::default()
                },
                "\x1b[5m",
            ),
            (
                "blink_rapid",
                AnsiSelectGraphicRendition {
                    blink: Some(Blink::Rapid),
                    ..Default::default()
                },
                "\x1b[6m",
            ),
            (
                "reverse",
                AnsiSelectGraphicRendition {
                    reverse: Some(true),
                    ..Default::default()
                },
                "\x1b[7m",
            ),
            (
                "hidden",
                AnsiSelectGraphicRendition {
                    hidden: Some(true),
                    ..Default::default()
                },
                "\x1b[8m",
            ),
            (
                "strike",
                AnsiSelectGraphicRendition {
                    strike: Some(true),
                    ..Default::default()
                },
                "\x1b[9m",
            ),
        ];

        for (name, style, expected) in attributes {
            let mut output = String::new();
            style
                .write_str(&mut output, Some(ColorMode::Basic))
                .unwrap();
            assert_eq!(output, expected, "Failed for attribute: {}", name);
        }
    }

    #[test]
    fn test_write_style_with_different_color_modes() {
        let style = AnsiSelectGraphicRendition {
            foreground: Some(Color::Red),
            ..Default::default()
        };

        // Test with Basic mode
        let mut output = String::new();
        style
            .write_str(&mut output, Some(ColorMode::Basic))
            .unwrap();
        assert_eq!(output, "\x1b[31m");

        // Test with FixedColor mode
        output.clear();
        style
            .write_str(&mut output, Some(ColorMode::FixedColor))
            .unwrap();
        assert_eq!(output, "\x1b[31m");

        // Test with TrueColor mode
        output.clear();
        style
            .write_str(&mut output, Some(ColorMode::TrueColor))
            .unwrap();
        assert_eq!(output, "\x1b[31m");

        // Test with None mode
        output.clear();
        style.write_str(&mut output, Some(ColorMode::None)).unwrap();
        assert_eq!(output, "");
    }

    #[test]
    fn test_style_clone() {
        let style1 = AnsiSelectGraphicRendition {
            foreground: Some(Color::Red),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        };
        let style2 = style1.clone();

        assert_eq!(style1, style2);
    }

    #[test]
    fn test_style_equality() {
        let style1 = AnsiSelectGraphicRendition {
            foreground: Some(Color::Green),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        };
        let style2 = AnsiSelectGraphicRendition {
            foreground: Some(Color::Green),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        };
        let style3 = AnsiSelectGraphicRendition {
            foreground: Some(Color::Red),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        };

        assert_eq!(style1, style2);
        assert_ne!(style1, style3);
    }

    #[test]
    fn test_style_hash() {
        use std::collections::HashSet;

        let style1 = AnsiSelectGraphicRendition {
            foreground: Some(Color::Red),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        };
        let style2 = AnsiSelectGraphicRendition {
            foreground: Some(Color::Red),
            intensity: Some(Intensity::Bold),
            ..Default::default()
        };

        let mut set = HashSet::new();
        set.insert(style1.clone());
        assert!(set.contains(&style2));
    }

    #[test]
    fn test_color_variants() {
        let colors = vec![
            Color::Black,
            Color::Red,
            Color::Green,
            Color::Yellow,
            Color::Blue,
            Color::Purple,
            Color::Cyan,
            Color::White,
            Color::Fixed(42),
            Color::RGB(100, 150, 200),
        ];

        for (i, color1) in colors.iter().enumerate() {
            for (j, color2) in colors.iter().enumerate() {
                if i == j {
                    assert_eq!(color1, color2);
                } else {
                    assert_ne!(color1, color2);
                }
            }
        }
    }

    #[test]
    fn test_color_clone() {
        let color1 = Color::RGB(128, 64, 32);
        let color2 = color1.clone();
        assert_eq!(color1, color2);
    }

    #[test]
    fn test_color_ordering() {
        assert!(Color::Black < Color::Red);
        assert!(Color::Fixed(10) < Color::Fixed(20));
        assert!(Color::RGB(0, 0, 0) < Color::RGB(255, 255, 255));
    }

    #[test]
    fn test_color_fixed_boundary_values() {
        let color_0 = Color::Fixed(0);
        let color_255 = Color::Fixed(255);

        let style_0 = AnsiSelectGraphicRendition {
            foreground: Some(color_0),
            ..Default::default()
        };
        let style_255 = AnsiSelectGraphicRendition {
            foreground: Some(color_255),
            ..Default::default()
        };

        let mut output_0 = String::new();
        let mut output_255 = String::new();

        style_0
            .write_str(&mut output_0, Some(ColorMode::FixedColor))
            .unwrap();
        style_255
            .write_str(&mut output_255, Some(ColorMode::FixedColor))
            .unwrap();

        assert_eq!(output_0, "\x1b[38;5;0m");
        assert_eq!(output_255, "\x1b[38;5;255m");
    }

    #[test]
    fn test_color_rgb_boundary_values() {
        let color_min = Color::RGB(0, 0, 0);
        let color_max = Color::RGB(255, 255, 255);

        let style_min = AnsiSelectGraphicRendition {
            foreground: Some(color_min),
            ..Default::default()
        };
        let style_max = AnsiSelectGraphicRendition {
            foreground: Some(color_max),
            ..Default::default()
        };

        let mut output_min = String::new();
        let mut output_max = String::new();

        style_min
            .write_str(&mut output_min, Some(ColorMode::FixedColor))
            .unwrap();
        style_max
            .write_str(&mut output_max, Some(ColorMode::FixedColor))
            .unwrap();

        assert_eq!(output_min, "\x1b[38;2;0;0;0m");
        assert_eq!(output_max, "\x1b[38;2;255;255;255m");
    }

    #[test]
    fn test_write_style_both_foreground_and_background() {
        let style = AnsiSelectGraphicRendition {
            foreground: Some(Color::Green),
            background: Some(Color::Yellow),
            ..Default::default()
        };

        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::Basic))
            .unwrap();
        assert_eq!(output, "\x1b[32;43m");
    }

    #[test]
    fn test_write_style_mixed_color_types() {
        let style = AnsiSelectGraphicRendition {
            foreground: Some(Color::Fixed(100)),
            background: Some(Color::RGB(50, 100, 150)),
            intensity: Some(Intensity::Bold),
            underline: Some(Underline::Single),
            ..Default::default()
        };
        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::TrueColor))
            .unwrap();
        assert_eq!(output, "\x1b[1;4;38;5;100;48;2;50;100;150m");
    }

    #[test]
    fn test_style_debug_format() {
        let style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            foreground: Some(Color::Red),
            ..Default::default()
        };
        let debug_str = format!("{:?}", style);
        assert!(debug_str.contains("intensity"));
        assert!(debug_str.contains("Bold"));
        assert!(debug_str.contains("Red"));
    }

    #[test]
    fn test_color_debug_format() {
        let color = Color::RGB(10, 20, 30);
        let debug_str = format!("{:?}", color);
        assert!(debug_str.contains("RGB"));
        assert!(debug_str.contains("10"));
        assert!(debug_str.contains("20"));
        assert!(debug_str.contains("30"));
    }

    #[test]
    fn test_intensity_variants() {
        assert_eq!(Intensity::Bold.to_u8(), 1);
        assert_eq!(Intensity::Dim.to_u8(), 2);
        assert_eq!(Intensity::Normal.to_u8(), 22);

        assert_eq!(Intensity::from_u8(1), Some(Intensity::Bold));
        assert_eq!(Intensity::from_u8(2), Some(Intensity::Dim));
        assert_eq!(Intensity::from_u8(22), Some(Intensity::Normal));
        assert_eq!(Intensity::from_u8(99), None);
    }

    #[test]
    fn test_underline_variants() {
        assert_eq!(Underline::Single.to_u8(), 4);
        assert_eq!(Underline::Double.to_u8(), 21);
        assert_eq!(Underline::Disabled.to_u8(), 24);

        assert_eq!(Underline::from_u8(4), Some(Underline::Single));
        assert_eq!(Underline::from_u8(21), Some(Underline::Double));
        assert_eq!(Underline::from_u8(24), Some(Underline::Disabled));
        assert_eq!(Underline::from_u8(99), None);
    }

    #[test]
    fn test_blink_variants() {
        assert_eq!(Blink::Off.to_u8(), 25);
        assert_eq!(Blink::Slow.to_u8(), 5);
        assert_eq!(Blink::Rapid.to_u8(), 6);

        assert_eq!(Blink::from_u8(25), Some(Blink::Off));
        assert_eq!(Blink::from_u8(5), Some(Blink::Slow));
        assert_eq!(Blink::from_u8(6), Some(Blink::Rapid));
        assert_eq!(Blink::from_u8(99), None);
    }

    #[test]
    fn test_font_variants() {
        assert_eq!(Font::PrimaryFont.to_u8(), 10);
        assert_eq!(Font::AlternateFont1.to_u8(), 11);
        assert_eq!(Font::AlternateFont2.to_u8(), 12);
        assert_eq!(Font::AlternateFont3.to_u8(), 13);
        assert_eq!(Font::AlternateFont4.to_u8(), 14);
        assert_eq!(Font::AlternateFont5.to_u8(), 15);
        assert_eq!(Font::AlternateFont6.to_u8(), 16);
        assert_eq!(Font::AlternateFont7.to_u8(), 17);
        assert_eq!(Font::AlternateFont8.to_u8(), 18);
        assert_eq!(Font::AlternateFont9.to_u8(), 19);
        assert_eq!(Font::Fraktur.to_u8(), 20);
    }

    #[test]
    fn test_color_to_basic() {
        assert_eq!(Color::Red.to_basic(), Color::Red);
        assert_eq!(Color::Fixed(1).to_basic(), Color::Red);
        assert_eq!(Color::Fixed(9).to_basic(), Color::BrightRed);
        assert_eq!(Color::RGB(255, 0, 0).to_basic(), Color::Red);
    }

    #[test]
    fn test_color_to_fixed() {
        assert_eq!(Color::Red.to_fixed(), Color::Fixed(1));
        assert_eq!(Color::Fixed(42).to_fixed(), Color::Fixed(42));
        assert_eq!(Color::RGB(255, 0, 0).to_fixed(), Color::Fixed(196));
    }

    #[test]
    fn test_color_to_truecolor() {
        assert_eq!(Color::Red.to_truecolor(), Color::RGB(205, 0, 0));
        assert_eq!(Color::Fixed(1).to_truecolor(), Color::RGB(205, 0, 0));
        assert_eq!(
            Color::RGB(100, 150, 200).to_truecolor(),
            Color::RGB(100, 150, 200)
        );
    }

    #[test]
    fn test_color_mode_is_ansi() {
        assert!(!ColorMode::None.is_ansi());
        assert!(ColorMode::Basic.is_ansi());
        assert!(ColorMode::FixedColor.is_ansi());
        assert!(ColorMode::TrueColor.is_ansi());
    }

    #[test]
    fn test_color_mode_is_true_color() {
        assert!(!ColorMode::None.is_true_color());
        assert!(!ColorMode::Basic.is_true_color());
        assert!(!ColorMode::FixedColor.is_true_color());
        assert!(ColorMode::TrueColor.is_true_color());
    }

    #[test]
    fn test_style_len() {
        let color_mode = ColorMode::Basic;

        // Empty style should have length 0
        let style = AnsiSelectGraphicRendition::default();
        assert_eq!(style.len(Some(color_mode)), 0);

        // Style with bold only
        let style = AnsiSelectGraphicRendition {
            intensity: Some(Intensity::Bold),
            ..Default::default()
        };
        // \x1b[ (2) + 1 (1) + m (1) = 4
        assert_eq!(style.len(Some(color_mode)), 4);

        // Style with foreground color
        let style = AnsiSelectGraphicRendition {
            foreground: Some(Color::Red),
            ..Default::default()
        };
        // \x1b[ (2) + 31 (2) + m (1) = 5
        assert_eq!(style.len(Some(color_mode)), 5);

        // Style with RGB color
        let style = AnsiSelectGraphicRendition {
            foreground: Some(Color::RGB(255, 128, 64)),
            ..Default::default()
        };
        // \x1b[ (2) + 38;2;255;128;64 (17) + m (1) = 20
        assert_eq!(style.len(Some(color_mode)), 18);
    }

    #[test]
    fn test_style_with_unknown_sgr() {
        let style = AnsiSelectGraphicRendition {
            unknown: vec![SGRParameter::Unknown(50), SGRParameter::Unknown(51)],
            ..Default::default()
        };
        let mut output = String::new();

        style
            .write_str(&mut output, Some(ColorMode::Basic))
            .unwrap();
        assert_eq!(output, "\x1b[50;51m");
    }
}
