//
// Copyright 2017-2025 Hans W. Uhlig. All Rights Reserved.
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

use crate::ColorMode;

///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnsiConfig {
    /// Allow C0/C1 Control Codes Bytes
    pub control: bool,
    /// Allow Control Sequence (CSI) Commands (Except SGR) Sequences
    pub csi: bool,
    /// Allow Select Graphics Rendition (SGR) Command Sequences
    pub sgr: bool,
    /// SGR Color Mode Sequence
    pub color_mode: ColorMode,
    /// Allow Operating System Command (OSC) Sequences
    pub osc: bool,
    /// Allow (DCS)
    pub dcs: bool,
    /// Allow Start of String (SOS) Sequences
    pub sos: bool,
    /// Allow String Terminator (ST) Sequences
    pub st: bool,
    /// Allow Privacy Message (PM) Sequences
    pub pm: bool,
    /// Allow Application Program Command (APC) Sequences
    pub apc: bool,
}

impl AnsiConfig {
    /// Strip all Ansi Codes
    pub fn strip_all() -> AnsiConfig {
        AnsiConfig {
            control: false,
            csi: false,
            sgr: false,
            color_mode: ColorMode::None,
            osc: false,
            dcs: false,
            sos: false,
            st: false,
            pm: false,
            apc: false,
        }
    }
    /// Strip all but basic color
    pub fn basic_color_only() -> AnsiConfig {
        AnsiConfig {
            control: false,
            csi: false,
            sgr: false,
            color_mode: ColorMode::Basic,
            osc: false,
            dcs: false,
            sos: false,
            st: false,
            pm: false,
            apc: false,
        }
    }
    /// Strip all but Fixed color
    pub fn fixed_color_only() -> AnsiConfig {
        AnsiConfig {
            control: false,
            csi: false,
            sgr: false,
            color_mode: ColorMode::FixedColor,
            osc: false,
            dcs: false,
            sos: false,
            st: false,
            pm: false,
            apc: false,
        }
    }
    /// Strip all but True color
    pub fn true_color_only() -> AnsiConfig {
        AnsiConfig {
            control: false,
            csi: false,
            sgr: false,
            color_mode: ColorMode::TrueColor,
            osc: false,
            dcs: false,
            sos: false,
            st: false,
            pm: false,
            apc: false,
        }
    }
    /// Enable All Ansi
    pub fn enabled() -> AnsiConfig {
        AnsiConfig {
            control: true,
            csi: true,
            sgr: true,
            color_mode: ColorMode::TrueColor,
            osc: true,
            dcs: true,
            sos: true,
            st: true,
            pm: true,
            apc: true,
        }
    }
}

impl Default for AnsiConfig {
    fn default() -> Self {
        Self::enabled()
    }
}
