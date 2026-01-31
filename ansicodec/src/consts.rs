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

/// Maximum length of an ANSI escape sequence in bytes.
///
/// This limit prevents unbounded buffer growth from malicious or malformed input.
/// Standard ANSI sequences are typically under 20 bytes, but we allow up to 256
/// to accommodate complex sequences with many parameters.
///
/// Sequences exceeding this length will trigger a `SequenceTooLong` error and
/// cause the parser to reset to a clean state.
pub const MAX_SEQUENCE_LENGTH: usize = 256;

/// Maximum number of parameters in a CSI sequence.
///
/// This prevents excessive parameter parsing and potential DoS attacks.
/// Standard CSI sequences rarely have more than 5-10 parameters.
#[allow(dead_code)]
pub const MAX_PARAMETER_COUNT: usize = 16;
