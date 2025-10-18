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

use crate::{CodecError, TelnetFrame, TelnetOption};
use crate::result::CodecResult;

/// A structure representing the configuration and state of Telnet options.
///
/// The `TelnetOptions` struct is designed to manage the options available within
/// a Telnet protocol implementation. It tracks the support state of each option
/// as well as the current negotiation state for each option.
///
/// # Fields
///
/// * `config` - An array of `SupportState` values with a fixed size of 255, representing
///              the support state of each Telnet option. Each index corresponds to the
///              associated Telnet option code, determining whether the option is supported
///              and in what manner.
///
/// * `state` - An array of `OptionState` values with a fixed size of 255, representing
///             the negotiation state of each Telnet option. Each index corresponds to the
///             associated Telnet option code, tracking the current state of the option
///             in terms of enabling, disabling, and negotiation progress.
///
/// # Notes
///
/// The Telnet protocol uses option codes ranging from 0 to 254, which makes the size
/// of both arrays precisely 255 to represent all potential options. The fields ensure
/// the ability to handle and manage all standard Telnet options.
#[derive(Debug)]
pub struct TelnetOptions {
    config: [SupportState; 255],
    state: [OptionState; 255],
}

impl TelnetOptions {
    /// Creates a new instance of the struct using the default values.
    ///
    /// This method relies on the implementation of the `Default` trait for the struct
    /// to provide the initial values. It is a convenient way to initialize an instance
    /// without manually specifying values for its fields.
    ///
    /// # Returns
    ///
    /// A new instance of the struct with default values.
    ///
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks if we support the given option locally
    fn is_supported_local(&self, option: TelnetOption) -> bool {
        self.config[option.as_u8() as usize].local
    }

    /// Checks if we support the given option remotely
    fn is_supported_remote(&self, option: TelnetOption) -> bool {
        self.config[option.as_u8() as usize].remote
    }

    /// Checks if the specified Telnet option is enabled locally.
    ///
    /// # Parameters
    /// - `option`: The `TelnetOption` to check for local enablement status.
    ///
    /// # Returns
    /// - `true` if the specified option is enabled locally (i.e., the local state
    ///   of the option is `QState::Yes`),
    /// - `false` otherwise.
    ///
    ///
    /// # Notes
    /// - The function checks the internal state of the Telnet option and determines
    ///   if its local state is enabled.
    pub fn local_enabled(&self, option: TelnetOption) -> bool {
        matches!(
            self.state[option.as_u8() as usize].local,
            QState::Yes | QState::WantNo | QState::WantNoOpposite
        )
    }

    /// Determines if the remote side has enabled a specific Telnet option.
    ///
    /// This function checks if the given Telnet option is currently enabled
    /// for the remote side by examining the internal state.
    ///
    /// # Arguments
    ///
    /// * `option` - A `TelnetOption` representing the Telnet option to check.
    ///
    /// # Returns
    ///
    /// * `true` if the specified Telnet option is enabled for the remote side.
    /// * `false` otherwise.
    ///
    pub fn remote_enabled(&self, option: TelnetOption) -> bool {
        matches!(
            self.state[option.as_u8() as usize].remote,
            QState::Yes | QState::WantNo | QState::WantNoOpposite
        )
    }

    /// Enables a specified Telnet option on the local side of the connection.
    ///
    /// This function requests to enable the provided `TelnetOption` for the local side
    /// by initiating a WILL command. It checks the state of the option and generates an appropriate
    /// `TelnetFrame` if a negotiation message needs to be sent to the remote side.
    ///
    /// # Parameters
    /// - `option`: The `TelnetOption` to be enabled on the local side.
    ///
    /// # Returns
    /// - `Some(TelnetFrame)`: If a negotiation message is required to enable the option with the remote side.
    /// - `None`: If no further action or negotiation is required.
    ///
    ///
    /// # Notes
    /// - The caller should handle the returned `TelnetFrame`, if any, to complete the option negotiation.
    /// - If the option is already in the desired state, no frame will be returned.
    pub fn enable_local(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        self.request_will(option)
    }

    /// Disables a specified Telnet option locally (on the client-side).
    ///
    /// This method sends a "WONT" command for the given TelnetOption to indicate that the client
    /// will not enable the specified option. If the frame generation is successful, it returns
    /// an optional `TelnetFrame` containing the corresponding command.
    ///
    /// # Arguments
    /// * `option` - The `TelnetOption` to be disabled locally.
    ///
    /// # Returns
    /// * `Option<TelnetFrame>` - A `TelnetFrame` representing the "WONT" command for the specified
    ///   TelnetOption, or `None` if the frame generation fails.
    pub fn disable_local(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        self.request_wont(option)
    }

    /// Enables the specified Telnet option for remote negotiation.
    ///
    /// This method sends a request to enable a Telnet option for the remote side
    /// of the connection. It communicates the desire for the remote side to
    /// perform a specific feature or capability defined by the Telnet protocol.
    ///
    /// # Parameters
    /// - `option`: The `TelnetOption` specifying the feature or capability to be enabled
    ///   on the remote side.
    ///
    /// # Returns
    /// - `Option<TelnetFrame>`: A Telnet frame representing the request, or `None` if no
    ///   frame is produced.
    ///
    /// # Notes
    /// - The behavior of the method depends on the implementation of `self.request_do()`,
    ///   which constructs and optionally returns a Telnet negotiation frame.
    /// - Use this method to negotiate features that are intended to be enabled for the
    ///   remote side of the connection.
    pub fn enable_remote(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        self.request_do(option)
    }

    /// Disables a remote Telnet option.
    ///
    /// This function sends a "DON'T" command for the specified Telnet option to the remote peer,
    /// indicating the local side does not want the remote side to enable this option.
    ///
    /// # Parameters
    /// - `option`: The `TelnetOption` that should be disabled on the remote side.
    ///
    /// # Returns
    /// - `Option<TelnetFrame>`: Returns a `TelnetFrame` containing the "DON'T" command if the frame
    ///   was successfully created, or `None` if the operation could not produce a frame.
    ///
    /// # Notes
    /// This function is a shorthand wrapper around the `request_dont` method for disabling a remote option.
    pub fn disable_remote(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        self.request_dont(option)
    }

    /// Handles a received Telnet frame and processes it based on its type.
    ///
    /// This method takes a `TelnetFrame` as input and determines the appropriate
    /// response based on the type of frame:
    /// - `TelnetFrame::Do`: Calls `recv_do` to process the frame.
    /// - `TelnetFrame::Dont`: Calls `recv_dont` to process the frame.
    /// - `TelnetFrame::Will`: Calls `recv_will` to process the frame.
    /// - `TelnetFrame::Wont`: Calls `recv_wont` to process the frame.
    /// - Other frame types result in a `TerminalError::NegotationError`.
    ///
    /// # Arguments
    ///
    /// * `frame` - A `TelnetFrame` that represents the received Telnet command or negotiation option.
    ///
    /// # Returns
    ///
    /// Returns a `TerminalResult<Option<TelnetFrame>>`, which can be one of:
    /// - `Ok(Some(TelnetFrame))`: Indicates a successful processing of the received frame, possibly
    ///   including a response frame to be sent.
    /// - `Ok(None)`: Indicates a successful processing of the received frame without requiring a response.
    /// - `Err(TerminalError)`: Indicates a failure during processing, such as an unexpected frame type.
    ///
    /// # Errors
    ///
    /// This function returns `Err(TerminalError::NegotationError)` if the received `frame`
    /// is of an unsupported or invalid type.
    ///
    pub fn handle_received(&mut self, frame: TelnetFrame) -> CodecResult<Option<TelnetFrame>> {
        match frame {
            TelnetFrame::Do(option) => Ok(self.recv_do(option)),
            TelnetFrame::Dont(option) => Ok(self.recv_dont(option)),
            TelnetFrame::Will(option) => Ok(self.recv_will(option)),
            TelnetFrame::Wont(option) => Ok(self.recv_wont(option)),
            _ => Err(CodecError::NegotiationError(String::from(
                "Unsupported frame type",
            ))),
        }
    }

    // #### Outgoing requests (what we initiate) ################################

    /// Request that *we* enable the option (i.e. we want to send WILL).
    /// Returns frames (commands) you should send on the wire as a result.
    fn request_will(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        // First check if we support providing this option
        if !self.config[option.as_u8() as usize].local {
            return None; // Don't try to enable unsupported options
        }
        match self.state[option.as_u8() as usize].local {
            QState::Yes | QState::WantYes | QState::WantYesOpposite => {
                // already enabled or in-progress to enable
                None
            }
            QState::No => {
                // Start negotiation: send WILL -> state WANTYES
                self.state[option.as_u8() as usize].local = QState::WantYes;
                Some(TelnetFrame::Will(option))
            }
            QState::WantNo => {
                // collision: we were trying to disable, but now ask enable -> WantYesOpposite
                self.state[option.as_u8() as usize].local = QState::WantYesOpposite;
                Some(TelnetFrame::Will(option))
            }
            QState::WantNoOpposite => {
                // will -> wantyes (opposite canceled)
                self.state[option.as_u8() as usize].local = QState::WantYes;
                Some(TelnetFrame::Will(option))
            }
        }
    }

    /// Request that *we* disable the option (i.e. we want to send WONT).
    fn request_wont(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        match self.state[option.as_u8() as usize].local {
            QState::No | QState::WantNo | QState::WantNoOpposite => {
                // already disabled or in-progress to disable
                None
            }
            QState::Yes => {
                // start disable: WONT -> WANTNO
                self.state[option.as_u8() as usize].local = QState::WantNo;
                Some(TelnetFrame::Wont(option))
            }
            QState::WantYes => {
                // collision -> WantNoOpposite
                self.state[option.as_u8() as usize].local = QState::WantNoOpposite;
                Some(TelnetFrame::Wont(option))
            }
            QState::WantYesOpposite => {
                self.state[option.as_u8() as usize].local = QState::WantNo;
                Some(TelnetFrame::Wont(option))
            }
        }
    }

    /// Request remote to enable the option (i.e. send DO).
    fn request_do(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        // First check if we support providing this option
        if !self.config[option.as_u8() as usize].remote {
            return None; // Don't try to enable unsupported options
        }
        match self.state[option.as_u8() as usize].remote {
            QState::Yes | QState::WantYes | QState::WantYesOpposite => None,
            QState::No => {
                self.state[option.as_u8() as usize].remote = QState::WantYes;
                Some(TelnetFrame::Do(option))
            }
            QState::WantNo => {
                self.state[option.as_u8() as usize].remote = QState::WantYesOpposite;
                Some(TelnetFrame::Do(option))
            }
            QState::WantNoOpposite => {
                self.state[option.as_u8() as usize].remote = QState::WantYes;
                Some(TelnetFrame::Do(option))
            }
        }
    }

    /// Request remote to disable the option (i.e. send DONT).
    fn request_dont(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        match self.state[option.as_u8() as usize].remote {
            QState::No | QState::WantNo | QState::WantNoOpposite => None,
            QState::Yes => {
                self.state[option.as_u8() as usize].remote = QState::WantNo;
                Some(TelnetFrame::Dont(option))
            }
            QState::WantYes => {
                self.state[option.as_u8() as usize].remote = QState::WantNoOpposite;
                Some(TelnetFrame::Dont(option))
            }
            QState::WantYesOpposite => {
                self.state[option.as_u8() as usize].remote = QState::WantNo;
                Some(TelnetFrame::Dont(option))
            }
        }
    }

    // #### Incoming processing (peer sent us DO/DONT/WILL/WONT) ##################

    /// Process an incoming WILL from remote (they say "I will do option").
    /// Returns frames to send in response (if any).
    fn recv_will(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        // First, check if we support providing this option
        if !self.config[option.as_u8() as usize].remote {
            // If we don't support it, reject with `DONT`
            return Some(TelnetFrame::Dont(option));
        }
        match self.state[option.as_u8() as usize].remote {
            QState::No => {
                // remote offers WILL -> if we accept, send DO and move to YES
                // For a generic engine we accept by default; caller can override by sending DONT.
                // Here we accept. If you want to implement policy, change this branch.
                self.state[option.as_u8() as usize].remote = QState::Yes;
                Some(TelnetFrame::Do(option))
            }
            QState::Yes => {
                // already yes -> no response
                None
            }
            QState::WantNo => {
                // peer is contradicting our previous DONT: move to No (or remain?) RFC1143:
                // WANTNO + WILL => WANTNO-OPPOSITE -> send DONT
                self.state[option.as_u8() as usize].remote = QState::WantNoOpposite;
                Some(TelnetFrame::Dont(option))
            }
            QState::WantNoOpposite => {
                // collision resolved -> YES
                self.state[option.as_u8() as usize].remote = QState::Yes;
                None // no further response
            }
            QState::WantYes => {
                // we asked for it, and peer confirms -> YES
                self.state[option.as_u8() as usize].remote = QState::Yes;
                None
            }
            QState::WantYesOpposite => {
                // double negotiation: move to YES
                self.state[option.as_u8() as usize].remote = QState::Yes;
                None
            }
        }
    }

    /// Process incoming WONT from remote.
    fn recv_wont(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        match self.state[option.as_u8() as usize].remote {
            QState::No => None, // already no
            QState::Yes => {
                self.state[option.as_u8() as usize].remote = QState::No;
                // if we expected it, nothing to send; RFC1143: no immediate reply
                None
            }
            QState::WantNo => {
                self.state[option.as_u8() as usize].remote = QState::No;
                None
            }
            QState::WantNoOpposite => {
                // remote confirmed refusal -> NO
                self.state[option.as_u8() as usize].remote = QState::No;
                None
            }
            QState::WantYes => {
                // requested YES, but peer refuses -> NO and maybe clear want
                self.state[option.as_u8() as usize].remote = QState::No;
                None
            }
            QState::WantYesOpposite => {
                self.state[option.as_u8() as usize].remote = QState::No;
                None
            }
        }
    }

    /// Process incoming DO (peer requests we enable option -> they ask us to send WILL/WONT).
    fn recv_do(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        // First, check if we support providing this option
        if !self.config[option.as_u8() as usize].local {
            // If we don't support it, reject with `WONT`
            return Some(TelnetFrame::Wont(option));
        }
        match self.state[option.as_u8() as usize].local {
            QState::No => {
                // peer asks us to enable: we accept by default -> send WILL
                self.state[option.as_u8() as usize].local = QState::Yes;
                Some(TelnetFrame::Will(option))
            }
            QState::Yes => None,
            QState::WantNo => {
                self.state[option.as_u8() as usize].local = QState::WantNoOpposite;
                Some(TelnetFrame::Will(option))
            }
            QState::WantNoOpposite => {
                self.state[option.as_u8() as usize].local = QState::Yes;
                None
            }
            QState::WantYes => {
                self.state[option.as_u8() as usize].local = QState::Yes;
                None
            }
            QState::WantYesOpposite => {
                self.state[option.as_u8() as usize].local = QState::Yes;
                None
            }
        }
    }

    /// Process incoming DONT (peer asks us not to enable option -> they ask we send WONT).
    fn recv_dont(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        match self.state[option.as_u8() as usize].local {
            QState::No => None,
            QState::Yes => {
                self.state[option.as_u8() as usize].local = QState::No;
                None
            }
            QState::WantNo => {
                self.state[option.as_u8() as usize].local = QState::No;
                None
            }
            QState::WantNoOpposite => {
                self.state[option.as_u8() as usize].local = QState::No;
                None
            }
            QState::WantYes => {
                self.state[option.as_u8() as usize].local = QState::No;
                None
            }
            QState::WantYesOpposite => {
                self.state[option.as_u8() as usize].local = QState::No;
                None
            }
        }
    }
}

impl Default for TelnetOptions {
    fn default() -> Self {
        TelnetOptions {
            config: core::array::from_fn(|idx| {
                let option = TelnetOption::from_u8(idx as u8);
                SupportState {
                    local: if option.supported_local() {
                        true
                    } else {
                        false
                    },
                    remote: if option.supported_remote() {
                        true
                    } else {
                        false
                    },
                }
            }),
            state: core::array::from_fn(|_| OptionState::default()),
        }
    }
}

/// Represents the state of options with local and remote `QState`.
///
/// This struct is used to maintain and handle the state of a configuration or
/// options set, specifically keeping track of both local and remote states.
///
/// # Fields
///
/// * `local` - A `QState` representing the state of the local options.
/// * `remote` - A `QState` representing the state of the remote options.
///
/// # Derives
///
/// * `Clone` - Allows for creating a copy of an `OptionState` instance.
/// * `Debug` - Enables formatting the `OptionState` in a user-friendly way for debugging purposes.
/// * `Default` - Provides a default implementation for `OptionState` where the local and remote
///               states are initialized using their respective `Default` implementations.
///
#[derive(Clone, Debug, Default)]
struct OptionState {
    pub local: QState,
    pub remote: QState,
}

/// Represents the state of a negotiation process with possible outcomes and desires.
///
/// The `NegotiationState` enum is used to define the different states
/// that can occur during a negotiation, including the current agreement
/// (or lack thereof) and any expressed desires for alternative outcomes.
///
/// # Variants
///
/// - `No`:
///     Indicates that the answer or agreement in the negotiation is a firm "No."
///
/// - `WantNo`:
///     Represents a state where one party currently agrees with "Yes" but desires a "No."
///
/// - `WantNoOpposite`:
///     Indicates that one party's current agreement is "No," but the other party desires "Yes."
///
/// - `Yes`:
///     Indicates that the answer or agreement in the negotiation is a firm "Yes."
///
/// - `WantYes`:
///     Represents a state where one party currently agrees with "No" but desires a "Yes."
///
/// - `WantYesOpposite`:
///     Indicates that one party's current agreement is "Yes," but the other party desires "No."
///
#[derive(Copy, Clone, Debug, Default, Ord, PartialOrd, Eq, PartialEq, Hash)]
enum QState {
    ///
    #[default]
    No,
    ///
    WantNo,
    ///
    WantNoOpposite,
    ///
    Yes,
    ///
    WantYes,
    ///
    WantYesOpposite,
}

/// The `SupportState` struct represents the state of a support entity,
/// containing both local and remote state information.
///
#[derive(Clone, Debug, Default)]
struct SupportState {
    /// Whether we support this option from us -> them.
    pub local: bool,
    /// Whether we support this option from them -> us.
    pub remote: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_option_state_default() {
        let state = OptionState::default();
        assert_eq!(state.local, QState::No);
        assert_eq!(state.remote, QState::No);
    }

    // ============================================================================
    // Local Option Enable Tests (We send WILL, they send DO)
    // ============================================================================

    #[test]
    fn test_local_enable_from_no_to_wantyes() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Initial state should be No
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::No);
        assert!(!opts.local_enabled(opt));

        // Request to enable local option
        let frame = opts.enable_local(opt);
        assert_eq!(frame, Some(TelnetFrame::Will(opt)));
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::WantYes);
        assert!(!opts.local_enabled(opt)); // Still not enabled until confirmed
    }

    #[test]
    fn test_local_enable_recv_do_completes_to_yes() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Move to WantYes state
        opts.enable_local(opt);
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::WantYes);

        // Receive DO from remote
        let response = opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(response, None); // No response needed
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::Yes);
        assert!(opts.local_enabled(opt));
    }

    #[test]
    fn test_local_enable_idempotent_when_yes() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes state
        opts.enable_local(opt);
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::Yes);

        // Try to enable again
        let frame = opts.enable_local(opt);
        assert_eq!(frame, None); // No frame sent
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::Yes);
    }

    #[test]
    fn test_local_enable_idempotent_when_wantyes() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Move to WantYes
        opts.enable_local(opt);
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::WantYes);

        // Try to enable again
        let frame = opts.enable_local(opt);
        assert_eq!(frame, None);
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::WantYes);
    }

    // ============================================================================
    // Local Option Disable Tests (We send WONT, they send DONT)
    // ============================================================================

    #[test]
    fn test_local_disable_from_yes_to_wantno() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes state first
        opts.enable_local(opt);
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::Yes);

        // Disable
        let frame = opts.disable_local(opt);
        assert_eq!(frame, Some(TelnetFrame::Wont(opt)));
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::WantNo);
        assert!(opts.local_enabled(opt)); // Still enabled until confirmed
    }

    #[test]
    fn test_local_disable_recv_dont_completes_to_no() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes, then WantNo
        opts.enable_local(opt);
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        opts.disable_local(opt);
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::WantNo);

        // Receive DONT from remote
        let response = opts.handle_received(TelnetFrame::Dont(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::No);
        assert!(!opts.local_enabled(opt));
    }

    #[test]
    fn test_local_disable_idempotent_when_no() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Already at No
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::No);

        // Try to disable
        let frame = opts.disable_local(opt);
        assert_eq!(frame, None);
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::No);
    }

    // ============================================================================
    // Remote Option Enable Tests (We send DO, they send WILL)
    // ============================================================================

    #[test]
    fn test_remote_enable_from_no_to_wantyes() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Initial state should be No
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::No);
        assert!(!opts.remote_enabled(opt));

        // Request to enable remote option
        let frame = opts.enable_remote(opt);
        assert_eq!(frame, Some(TelnetFrame::Do(opt)));
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::WantYes);
        assert!(!opts.remote_enabled(opt));
    }

    #[test]
    fn test_remote_enable_recv_will_completes_to_yes() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Move to WantYes state
        opts.enable_remote(opt);
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::WantYes);

        // Receive WILL from remote
        let response = opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::Yes);
        assert!(opts.remote_enabled(opt));
    }

    #[test]
    fn test_remote_enable_idempotent_when_yes() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes state
        opts.enable_remote(opt);
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::Yes);

        // Try to enable again
        let frame = opts.enable_remote(opt);
        assert_eq!(frame, None);
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::Yes);
    }

    // ============================================================================
    // Remote Option Disable Tests (We send DONT, they send WONT)
    // ============================================================================

    #[test]
    fn test_remote_disable_from_yes_to_wantno() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes state first
        opts.enable_remote(opt);
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::Yes);

        // Disable
        let frame = opts.disable_remote(opt);
        assert_eq!(frame, Some(TelnetFrame::Dont(opt)));
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::WantNo);
        assert!(opts.remote_enabled(opt)); // Still enabled until confirmed
    }

    #[test]
    fn test_remote_disable_recv_wont_completes_to_no() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes, then WantNo
        opts.enable_remote(opt);
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        opts.disable_remote(opt);
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::WantNo);

        // Receive WONT from remote
        let response = opts.handle_received(TelnetFrame::Wont(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::No);
        assert!(!opts.remote_enabled(opt));
    }

    #[test]
    fn test_remote_disable_idempotent_when_no() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Already at No
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::No);

        // Try to disable
        let frame = opts.disable_remote(opt);
        assert_eq!(frame, None);
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::No);
    }

    // ============================================================================
    // Unsolicited Remote Requests (They initiate)
    // ============================================================================

    #[test]
    fn test_recv_will_from_no_accepts_to_yes() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Start at No
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::No);

        // Remote sends WILL
        let response = opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(response, Some(TelnetFrame::Do(opt)));
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::Yes);
        assert!(opts.remote_enabled(opt));
    }

    #[test]
    fn test_recv_will_when_yes_no_response() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::Yes);

        // Remote sends WILL again
        let response = opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::Yes);
    }

    #[test]
    fn test_recv_do_from_no_accepts_to_yes() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Start at No
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::No);

        // Remote sends DO
        let response = opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(response, Some(TelnetFrame::Will(opt)));
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::Yes);
        assert!(opts.local_enabled(opt));
    }

    #[test]
    fn test_recv_do_when_yes_no_response() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::Yes);

        // Remote sends DO again
        let response = opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::Yes);
    }

    #[test]
    fn test_recv_wont_from_yes_to_no() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::Yes);

        // Remote sends WONT
        let response = opts.handle_received(TelnetFrame::Wont(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::No);
        assert!(!opts.remote_enabled(opt));
    }

    #[test]
    fn test_recv_dont_from_yes_to_no() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::Yes);

        // Remote sends DONT
        let response = opts.handle_received(TelnetFrame::Dont(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::No);
        assert!(!opts.local_enabled(opt));
    }

    // ============================================================================
    // Collision Tests (Both sides negotiate simultaneously)
    // ============================================================================

    #[test]
    fn test_collision_enable_local_while_wantno() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes then start disabling
        opts.enable_local(opt);
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        opts.disable_local(opt);
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::WantNo);

        // Try to enable again (collision)
        let frame = opts.enable_local(opt);
        assert_eq!(frame, Some(TelnetFrame::Will(opt)));
        assert_eq!(
            opts.state[opt.as_u8() as usize].local,
            QState::WantYesOpposite
        );
    }

    #[test]
    fn test_collision_disable_local_while_wantyes() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Start enabling
        opts.enable_local(opt);
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::WantYes);

        // Try to disable (collision)
        let frame = opts.disable_local(opt);
        assert_eq!(frame, Some(TelnetFrame::Wont(opt)));
        assert_eq!(
            opts.state[opt.as_u8() as usize].local,
            QState::WantNoOpposite
        );
    }

    #[test]
    fn test_recv_do_while_wantno_stays_wantno_opposite() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes, then WantNo
        opts.enable_local(opt);
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        opts.disable_local(opt);
        assert_eq!(opts.state[opt.as_u8() as usize].local, QState::WantNo);

        // Remote sends DO (collision)
        let response = opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(response, Some(TelnetFrame::Will(opt)));
        assert_eq!(
            opts.state[opt.as_u8() as usize].local,
            QState::WantNoOpposite
        );
    }

    #[test]
    fn test_recv_will_while_wantno_stays_wantno_opposite() {
        let mut opts = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes, then WantNo
        opts.enable_remote(opt);
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        opts.disable_remote(opt);
        assert_eq!(opts.state[opt.as_u8() as usize].remote, QState::WantNo);

        // Remote sends WILL again (collision)
        let response = opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(response, Some(TelnetFrame::Dont(opt)));
        assert_eq!(
            opts.state[opt.as_u8() as usize].remote,
            QState::WantNoOpposite
        );
    }

    // ============================================================================
    // Full Handshake Integration Tests
    // ============================================================================

    #[test]
    fn test_full_local_enable_disable_handshake() {
        let mut client = TelnetOptions::new();
        let mut server = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Enable: Client sends WILL
        let will = client.enable_local(opt).unwrap();
        assert_eq!(will, TelnetFrame::Will(opt));
        assert_eq!(client.state[opt.as_u8() as usize].local, QState::WantYes);

        // Server receives WILL, responds with DO
        let do_frame = server.handle_received(will).unwrap().unwrap();
        assert_eq!(do_frame, TelnetFrame::Do(opt));
        assert_eq!(server.state[opt.as_u8() as usize].remote, QState::Yes);
        assert!(server.remote_enabled(opt));

        // Client receives DO, completes to Yes
        let none = client.handle_received(do_frame).unwrap();
        assert_eq!(none, None);
        assert_eq!(client.state[opt.as_u8() as usize].local, QState::Yes);
        assert!(client.local_enabled(opt));

        // Disable: Client sends WONT
        let wont = client.disable_local(opt).unwrap();
        assert_eq!(wont, TelnetFrame::Wont(opt));
        assert_eq!(client.state[opt.as_u8() as usize].local, QState::WantNo);

        // Server receives WONT, moves to No
        let none = server.handle_received(wont).unwrap();
        assert_eq!(none, None);
        assert_eq!(server.state[opt.as_u8() as usize].remote, QState::No);
        assert!(!server.remote_enabled(opt));
        assert_eq!(client.state[opt.as_u8() as usize].local, QState::WantNo);

        // Client state is still WantNo (waiting for explicit DONT)
        // In practice, the server could send DONT to confirm
    }

    #[test]
    fn test_full_remote_enable_disable_handshake() {
        let mut client = TelnetOptions::new();
        let mut server = TelnetOptions::new();
        let opt = TelnetOption::TransmitBinary;

        // Enable: Client sends DO
        let do_frame = client.enable_remote(opt).unwrap();
        assert_eq!(do_frame, TelnetFrame::Do(opt));
        assert_eq!(client.state[opt.as_u8() as usize].remote, QState::WantYes);

        // Server receives DO, responds with WILL
        let will = server.handle_received(do_frame).unwrap().unwrap();
        assert_eq!(will, TelnetFrame::Will(opt));
        assert_eq!(server.state[opt.as_u8() as usize].local, QState::Yes);
        assert!(server.local_enabled(opt));

        // Client receives WILL, completes to Yes
        let none = client.handle_received(will).unwrap();
        assert_eq!(none, None);
        assert_eq!(client.state[opt.as_u8() as usize].remote, QState::Yes);
        assert!(client.remote_enabled(opt));

        // Disable: Client sends DONT
        let dont = client.disable_remote(opt).unwrap();
        assert_eq!(dont, TelnetFrame::Dont(opt));
        assert_eq!(client.state[opt.as_u8() as usize].remote, QState::WantNo);

        // Server receives DONT, moves to No
        let none = server.handle_received(dont).unwrap();
        assert_eq!(none, None);
        assert_eq!(server.state[opt.as_u8() as usize].local, QState::No);
        assert!(!server.local_enabled(opt));
    }
}
