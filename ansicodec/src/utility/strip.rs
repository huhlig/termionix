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

use std::borrow::Cow;

/// Removes ANSI escape sequences from a string.
///
/// This function strips ANSI Control Sequence Introducer (CSI) codes from the input string,
/// which are commonly used for terminal formatting, colors, and cursor control. The function
/// detects sequences that begin with the escape character (`\x1b`) followed by `[` and removes
/// them along with their parameters and terminal character.
///
/// # ANSI Escape Sequence Format
///
/// ANSI CSI sequences follow the pattern: `ESC [ <parameters> <command>`
/// - `ESC`: The escape character (`\x1b` or decimal 27)
/// - `[`: Opening square bracket (Control Sequence Introducer)
/// - `<parameters>`: Optional numeric parameters separated by semicolons
/// - `<command>`: A letter (A-Z, a-z) or specific symbol (like 'm') that terminates the sequence
///
/// # Performance
///
/// The function performs a quick check for the presence of the escape character before
/// processing. If no ANSI codes are found, it returns a borrowed reference to the original
/// string (zero-copy). Otherwise, it allocates a new `String` with the codes removed.
///
/// # Arguments
///
/// * `str` - A string slice that may contain ANSI escape sequences
///
/// # Returns
///
/// Returns a `Cow<'_, str>`:
/// - `Cow::Borrowed(str)` if the input contains no ANSI escape sequences
/// - `Cow::Owned(String)` if ANSI codes were found and removed
///
/// # Examples
///
/// ```
/// use std::borrow::Cow;
/// # use termionix_ansicodec::strip_ansi_codes;
///
/// // String with ANSI color codes
/// let colored = "\x1b[1;31mRed Text\x1b[0m";
/// let clean = strip_ansi_codes(colored);
/// assert_eq!(clean, "Red Text");
///
/// // String without ANSI codes (zero-copy)
/// let plain = "Plain Text";
/// let result = strip_ansi_codes(plain);
/// assert!(matches!(result, Cow::Borrowed(_)));
/// assert_eq!(result, "Plain Text");
///
/// // Complex formatting
/// let formatted = "\x1b[1mBold\x1b[0m and \x1b[4mUnderlined\x1b[0m";
/// assert_eq!(strip_ansi_codes(formatted), "Bold and Underlined");
/// ```
///
/// # Supported Sequences
///
/// This function handles CSI sequences (ESC `[`) which include:
/// - Color codes (foreground/background)
/// - Text styling (bold, italic, underline, etc.)
/// - Cursor positioning and movement
/// - Screen clearing and erasing
///
/// # Limitations
///
/// Currently only strips CSI sequences (ESC `[`). Does not remove:
/// - OSC sequences (ESC `]`) - Operating System Commands
/// - DCS sequences (ESC `P`) - Device Control Strings
/// - Other escape sequences that don't use the `[` delimiter
///
/// # See Also
///
/// For detailed information about ANSI escape codes, see the ANSI specification
/// or the project's `ansi.md` documentation file.
pub fn strip_ansi_codes(str: &str) -> Cow<'_, str> {
    // Check if the string contains any ANSI escape sequences
    if !str.contains('\x1b') {
        // No ANSI codes, return borrowed string
        return Cow::Borrowed(str);
    }

    let mut result = String::with_capacity(str.len());
    let chars: Vec<char> = str.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check for ANSI escape sequence start
        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
            // Skip the escape sequence
            i += 2; // Skip '\x1b['

            // Skip until we find the terminal character (typically 'm', but could be others)
            while i < chars.len() {
                let ch = chars[i];
                i += 1;
                // ANSI escape sequences end with a letter (A-Z, a-z) or certain symbols
                if ch.is_ascii_alphabetic() || ch == 'm' {
                    break;
                }
            }
        } else {
            // Regular character, add to result
            result.push(chars[i]);
            i += 1;
        }
    }

    Cow::Owned(result)
}
