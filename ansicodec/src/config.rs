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

///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnsiConfig {
    /// Strip Ansi C0/C1 Control Codes Bytes
    pub strip_ctrl: bool,
    /// Strip Ansi Control Sequence (CSI) Commands (Except SGR) Sequences
    pub strip_csi: bool,
    /// Strip Select Graphics Rendition (SGR) Command Sequences
    pub strip_sgr: bool,
    /// SGR Color Mode Sequence Conversion Settings
    pub color_mode: ColorMode,
    /// Strip Ansi Operating System Command (OSC) Sequences
    pub strip_osc: bool,
    /// Strip Ansi Device Control String (DCS) Sequences
    pub strip_dcs: bool,
    /// Strip Ansi Start of String (SOS) and String Terminator (ST) Sequences
    pub strip_sos_st: bool,
    /// Strip Ansi Privacy Message (PM) Sequences
    pub strip_pm: bool,
    /// Strip Ansi Application Program Command (APC) Sequences
    pub strip_apc: bool,
    /// Strip Telnet Command Sequences
    pub strip_telnet: bool,
}

impl AnsiConfig {
    /// Strip all Ansi Codes
    pub fn strip_all() -> AnsiConfig {
        AnsiConfig {
            strip_ctrl: true,
            strip_csi: true,
            strip_sgr: true,
            color_mode: ColorMode::None,
            strip_osc: true,
            strip_dcs: true,
            strip_sos_st: true,
            strip_pm: true,
            strip_apc: true,
            strip_telnet: true,
        }
    }
    /// Strip all but basic color
    pub fn basic_color_only() -> AnsiConfig {
        AnsiConfig {
            strip_ctrl: true,
            strip_csi: true,
            strip_sgr: false,
            color_mode: ColorMode::Basic,
            strip_osc: true,
            strip_dcs: true,
            strip_sos_st: true,
            strip_pm: true,
            strip_apc: true,
            strip_telnet: true,
        }
    }
    /// Strip all but Fixed color
    pub fn fixed_color_only() -> AnsiConfig {
        AnsiConfig {
            strip_ctrl: true,
            strip_csi: true,
            strip_sgr: false,
            color_mode: ColorMode::FixedColor,
            strip_osc: true,
            strip_dcs: true,
            strip_sos_st: true,
            strip_pm: true,
            strip_apc: true,
            strip_telnet: true,
        }
    }
    /// Strip all but True color
    pub fn true_color_only() -> AnsiConfig {
        AnsiConfig {
            strip_ctrl: true,
            strip_csi: true,
            strip_sgr: false,
            color_mode: ColorMode::TrueColor,
            strip_osc: true,
            strip_dcs: true,
            strip_sos_st: true,
            strip_pm: true,
            strip_apc: true,
            strip_telnet: true,
        }
    }
    /// Enable All Ansi
    pub fn enabled() -> AnsiConfig {
        AnsiConfig {
            strip_ctrl: false,
            strip_csi: false,
            strip_sgr: false,
            color_mode: ColorMode::FixedColor,
            strip_osc: false,
            strip_dcs: false,
            strip_sos_st: false,
            strip_pm: false,
            strip_apc: false,
            strip_telnet: false,
        }
    }
}

impl Default for AnsiConfig {
    fn default() -> Self {
        Self::enabled()
    }
}

/// Represents the color capabilities of a terminal.
///
/// `ColorMode` defines which level of ANSI escape sequence support should be used when
/// rendering styled text. Different terminals support different color capabilities, and
/// this enum allows you to tailor output to match the terminal's capabilities.
///
/// # Color Mode Capabilities
///
/// - **None**: No ANSI codes are generated, resulting in plain text output
/// - **Basic**: 4-bit color supporting 16 colors (8 basic + 8 bright variants)
/// - **FixedColor**: 8-bit color supporting 256 colors
/// - **TrueColor**: 24-bit RGB color supporting 16.7 million colors
///
/// # Usage
///
/// Color modes are typically determined by detecting terminal capabilities at runtime,
/// often by checking environment variables like `COLORTERM`, `TERM`, or `NO_COLOR`.
///
/// # Examples
///
/// Basic usage with styled text:
///
/// ```
/// use termionix_ansicodec::{AnsiConfig, Color, Intensity, AnsiSelectGraphicRendition};
/// use termionix_ansicodec::utility::{StyledString};
///
/// let config = AnsiConfig::enabled();
/// let styled = StyledString::from_string("Hello", Some(AnsiSelectGraphicRendition {
///     intensity: Some(Intensity::Bold),
///     foreground: Some(Color::Red),
///     ..Default::default()
/// }));
///
/// // Output for different terminal capabilities
/// let mut output = String::new();
/// styled.write_str(&mut output, Some(&config)).unwrap();
/// ```
///
/// Detecting terminal capabilities:
///
/// ```
/// use termionix_ansicodec::ColorMode;
///
/// fn detect_color_mode() -> ColorMode {
///     // Check if colors are explicitly disabled
///     if std::env::var("NO_COLOR").is_ok() {
///         return ColorMode::None;
///     }
///
///     // Check for true color support
///     if let Ok(colorterm) = std::env::var("COLORTERM") {
///         if colorterm.contains("truecolor") || colorterm.contains("24bit") {
///             return ColorMode::TrueColor;
///         }
///     }
///
///     // Check for 256 color support
///     if let Ok(term) = std::env::var("TERM") {
///         if term.contains("256") {
///             return ColorMode::FixedColor;
///         }
///     }
///
///     // Default to basic colors
///     ColorMode::Basic
/// }
/// ```
///
/// Conditional styling based on mode:
///
/// ```
/// use termionix_ansicodec::{ColorMode, Color};
///
/// fn get_color_for_mode(mode: &ColorMode) -> Option<Color> {
///     match mode {
///         ColorMode::None => None,
///         ColorMode::Basic => Some(Color::Red),
///         ColorMode::FixedColor => Some(Color::Fixed(196)),
///         ColorMode::TrueColor => Some(Color::RGB(255, 0, 0)),
///     }
/// }
/// ```
///
/// # See Also
///
/// - [`Color`](crate::Color) - Color representations compatible with different modes
/// - [`Style`](crate::Style) - Text styling that respects color modes
/// - [`StyledString::write_str`](crate::StyledString::write_str) - Renders styled text according to color mode
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ColorMode {
    /// No ANSI color codes are generated.
    ///
    /// Use this mode when:
    /// - Writing to non-terminal outputs (files, pipes)
    /// - The `NO_COLOR` environment variable is set
    /// - The terminal doesn't support ANSI escape sequences
    /// - Colors should be explicitly disabled
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::{StyledString, ColorMode, AnsiConfig};
    ///
    /// let config = AnsiConfig::strip_all();
    /// let styled = StyledString::from_string("Plain text", None);
    /// let mut output = String::new();
    /// styled.write_str(&mut output, Some(&config)).unwrap();
    /// // Output contains no ANSI codes, just "Plain text"
    /// ```
    None,

    /// 4-bit color mode supporting 16 colors.
    ///
    /// This mode supports the 8 basic ANSI colors (black, red, green, yellow, blue,
    /// purple, cyan, white) plus their bright variants, for a total of 16 colors.
    ///
    /// Use this mode when:
    /// - Working with older terminals or basic terminal emulators
    /// - Maximum compatibility is required
    /// - Only basic color differentiation is needed
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::{StyledString, ColorMode, Style, Color, AnsiConfig};
    ///
    /// let config = AnsiConfig::enabled();
    /// let styled = StyledString::from_string("Colored text", Some(Style {
    ///     foreground: Some(Color::Red),
    ///     ..Default::default()
    /// }));
    ///
    /// let mut output = String::new();
    /// styled.write_str(&mut output, Some(&config)).unwrap();
    /// // Generates: "\x1b[31mColored text\x1b[0m"
    /// ```
    Basic,

    /// 8-bit color mode supporting 256 colors.
    ///
    /// This mode includes the 16 basic colors plus a 216-color RGB cube and a
    /// 24-shade grayscale ramp, for a total of 256 colors. Colors are specified
    /// using the [`Color::Fixed`](crate::Color::Fixed) variant with values 0-255.
    ///
    /// Use this mode when:
    /// - The terminal supports 256-color mode (most modern terminals)
    /// - More color variety is needed than basic mode provides
    /// - True color is not available or not needed
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::{StyledString, ColorMode, Style, Color, AnsiConfig};
    ///
    /// let config = AnsiConfig::enabled();
    /// let styled = StyledString::from_string("256-color text", Some(Style {
    ///     foreground: Some(Color::Fixed(196)), // Bright red
    ///     background: Some(Color::Fixed(234)), // Dark gray
    ///     ..Default::default()
    /// }));
    ///
    /// let mut output = String::new();
    /// styled.write_str(&mut output, Some(&config)).unwrap();
    /// // Generates: "\x1b[38;5;196;48;5;234m256-color text\x1b[0m"
    /// ```
    FixedColor,

    /// 24-bit true color mode supporting 16.7 million colors.
    ///
    /// This mode supports full RGB color specification with 256 levels each for
    /// red, green, and blue channels. Colors are specified using the
    /// [`Color::RGB`](crate::Color::RGB) variant.
    ///
    /// Use this mode when:
    /// - The terminal supports true color (check `COLORTERM=truecolor`)
    /// - Precise color matching is required
    /// - Creating rich, colorful terminal UIs
    /// - Maximum color fidelity is desired
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::{StyledString, ColorMode, Style, Color, AnsiConfig};
    ///
    /// let config = AnsiConfig::enabled();
    /// let styled = StyledString::from_string("RGB text", Some(Style {
    ///     foreground: Some(Color::RGB(255, 100, 50)), // Custom orange
    ///     background: Some(Color::RGB(20, 20, 40)),   // Dark blue-gray
    ///     ..Default::default()
    /// }));
    ///
    /// let mut output = String::new();
    /// styled.write_str(&mut output, Some(&config)).unwrap();
    /// // Generates: "\x1b[38;2;255;100;50;48;2;20;20;40mRGB text\x1b[0m"
    /// ```
    TrueColor,
}

impl ColorMode {
    /// Returns `true` if this color mode supports ANSI escape codes.
    ///
    /// This method returns `true` for all modes except [`ColorMode::None`].
    /// It's useful for determining whether ANSI styling should be applied at all.
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::ColorMode;
    ///
    /// assert!(!ColorMode::None.is_ansi());
    /// assert!(ColorMode::Basic.is_ansi());
    /// assert!(ColorMode::FixedColor.is_ansi());
    /// assert!(ColorMode::TrueColor.is_ansi());
    /// ```
    ///
    /// Practical usage:
    ///
    /// ```
    /// use termionix_ansicodec::ColorMode;
    ///
    /// fn should_apply_styling(mode: &ColorMode) -> bool {
    ///     mode.is_ansi()
    /// }
    /// ```
    pub fn is_ansi(&self) -> bool {
        match self {
            ColorMode::None => false,
            ColorMode::Basic | ColorMode::FixedColor | ColorMode::TrueColor => true,
        }
    }

    /// Returns `true` if this color mode is [`ColorMode::TrueColor`].
    ///
    /// This method is useful for code that needs to handle true color specifically,
    /// such as providing RGB color pickers or gradients.
    ///
    /// # Examples
    ///
    /// ```
    /// use termionix_ansicodec::ColorMode;
    ///
    /// assert!(!ColorMode::None.is_true_color());
    /// assert!(!ColorMode::Basic.is_true_color());
    /// assert!(!ColorMode::FixedColor.is_true_color());
    /// assert!(ColorMode::TrueColor.is_true_color());
    /// ```
    ///
    /// Conditional color selection:
    ///
    /// ```
    /// use termionix_ansicodec::{ColorMode, Color};
    ///
    /// fn select_color(mode: &ColorMode) -> Color {
    ///     if mode.is_true_color() {
    ///         Color::RGB(255, 128, 64) // Custom orange
    ///     } else {
    ///         Color::Yellow // Fallback to basic color
    ///     }
    /// }
    /// ```
    pub fn is_true_color(&self) -> bool {
        match self {
            ColorMode::TrueColor => true,
            _ => false,
        }
    }
}
