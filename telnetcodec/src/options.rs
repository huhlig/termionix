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

use crate::{TelnetCodecError, TelnetCodecResult, TelnetFrame, consts};
use std::fmt::Formatter;

///
/// [Telnet Terminal Options](https://www.iana.org/assignments/telnet-options/telnet-options.xhtml)
///
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TelnetOption {
    /// [`consts::option::BINARY`] Telnet Binary Transmission [RFC856](https://tools.ietf.org/html/rfc856)
    TransmitBinary,
    /// [`consts::option::ECHO`] Telnet Echo Option [RFC857](https://tools.ietf.org/html/rfc857)
    Echo,
    /// [`consts::option::RCP`] Telnet Reconnection Option [DDN Protocol Handbook, "Telnet Reconnection Option", NIC 50005, December 1985.]()
    /// Note: Prepare to reconnect
    Reconnection,
    /// [`consts::option::SGA`] Suppress Go ahead [RFC858](https://tools.ietf.org/html/rfc858)
    SuppressGoAhead,
    /// [`consts::option::NAMS`] Negotiate Approximate Message Size
    NegotiateApproxMessageSize,
    /// [`consts::option::STATUS`] Telnet Status Option [RFC859](http://www.iana.org/go/rfc859)
    Status,
    /// [`consts::option::TM`] Telnet Timing Mark Option [RFC860](http://www.iana.org/go/rfc860)
    TimingMark,
    /// [`consts::option::RCTE`] Remote-Controlled Transmission and Echo [RFC726](http://www.iana.org/go/rfc726)
    RCTE,
    /// [`consts::option::NAOL`] Output Line Width [DDN Protocol Handbook, "Telnet Output Line Width Option", NIC 50005, December 1985.]()
    OutLineWidth,
    /// [`consts::option::NAOP`] Output Page Size [DDN Protocol Handbook, "Telnet Output Page Size Option", NIC 50005, December 1985.]()
    OutPageSize,
    /// [`consts::option::NAOCRD`] Output Carriage-Return Disposition [RFC652](http://www.iana.org/go/rfc652)
    NAOCRD,
    /// [`consts::option::NAOHTS`] Output Horizontal Tab Stops [RFC653](http://www.iana.org/go/rfc653)
    NAOHTS,
    /// [`consts::option::NAOHTD`] Output Horizontal Tab Disposition [RFC654](http://www.iana.org/go/rfc654)
    NAOHTD,
    /// [`consts::option::NAOFFD`] Output Form Feed Disposition [RFC655](http://www.iana.org/go/rfc655)
    NAOFFD,
    /// [`consts::option::NAOVTS`] Output Vertical Tab Stops [RFC656](http://www.iana.org/go/rfc656)
    NAOVTS,
    /// [`consts::option::NAOVTD`] Output Vertical Tab Disposition [RFC657](http://www.iana.org/go/rfc657)
    NAOVTD,
    /// [`consts::option::NAOLFD`] Output Linefeed Disposition [RFC658](http://www.iana.org/go/rfc658)
    NAOLFD,
    /// [`consts::option::XASCII`] Extended ASCII [RFC698](http://www.iana.org/go/rfc698)
    XASCII,
    /// [`consts::option::LOGOUT`] Logout Option [RFC727](http://www.iana.org/go/rfc727)
    Logout,
    /// [`consts::option::BM`] Byte Macro [RFC735](http://www.iana.org/go/rfc735)
    ByteMacro,
    /// [`consts::option::DET`] Data Entry Terminal [RFC1043](http://www.iana.org/go/rfc1043) [RFC732](http://www.iana.org/go/rfc732)
    DET,
    /// [`consts::option::SUPDUP`] SUPDUP [RFC736](http://www.iana.org/go/rfc736) [RFC734](http://www.iana.org/go/rfc734)
    SUPDUP,
    /// [`consts::option::SUPDUP_OUTPUT`] SUPDUP Output [RFC749](http://www.iana.org/go/rfc749)
    SUPDUPOutput,
    /// [`consts::option::SNDLOC`] Send Location [RFC779](http://www.iana.org/go/rfc779)
    SNDLOC,
    /// [`consts::option::TTYPE`] Terminal Type [RFC1091](http://www.iana.org/go/rfc1091)
    TTYPE,
    /// [`consts::option::EOR`] End of Record [RFC885](http://www.iana.org/go/rfc885)
    EOR,
    /// [`consts::option::TUID`] TACACS User Identification [RFC927](http://www.iana.org/go/rfc927)
    TUID,
    /// [`consts::option::OUTMRK`] Output Marking [RFC933](http://www.iana.org/go/rfc933)
    OUTMRK,
    /// [`consts::option::TTYLOC`] Terminal Location Number [RFC946](http://www.iana.org/go/rfc946)
    TTYLOC,
    /// [`consts::option::OPT3270REGIME`] Telnet 3270 Regime [RFC1041](http://www.iana.org/go/rfc1041)
    OPT3270Regime,
    /// [`consts::option::X3PAD`] X.3 PAD [RFC1053](http://www.iana.org/go/rfc1053)
    X3PAD,
    /// [`consts::option::NAWS`] Negotiate About Window Size [RFC1073](http://www.iana.org/go/rfc1073)
    NAWS,
    /// [`consts::option::TSPEED`] Terminal Speed [RFC1079](http://www.iana.org/go/rfc1079)
    TSPEED,
    /// [`consts::option::LFLOW`] Remote Flow Control [RFC1372](http://www.iana.org/go/rfc1372)
    LFLOW,
    /// [`consts::option::LINEMODE`] Linemode [RFC1184](http://www.iana.org/go/rfc1184)
    Linemode,
    /// [`consts::option::XDISPLOC`] X Display Location [RFC1096](http://www.iana.org/go/rfc1096)
    XDISPLOC,
    /// [`consts::option::OLD_ENVIRONMENT`] Environment Option [RFC1408](http://www.iana.org/go/rfc1408)
    Environment,
    /// [`consts::option::AUTHENTICATION`] Authentication Option [RFC2941](http://www.iana.org/go/rfc2941)
    Authentication,
    /// [`consts::option::ENCRYPTION`] Encryption Option [RFC2946](http://www.iana.org/go/rfc2946)
    Encryption,
    /// [`consts::option::NEW_ENVIRONMENT`] New Environment Option [RFC1572](http://www.iana.org/go/rfc1572)
    NewEnvironment,
    /// [`consts::option::TN3270E`] TN3270E [RFC2355](http://www.iana.org/go/rfc2355)
    TN3270E,
    /// [`consts::option::XAUTH`] XAUTH [Rob_Earhart](https://www.iana.org/assignments/telnet-options/telnet-options.xhtml#Rob_Earhart)
    XAUTH,
    /// [`consts::option::CHARSET`] Charset [RFC2066](http://www.iana.org/go/rfc2066)
    Charset,
    /// [`consts::option::TRSP`] Telnet Remote Serial Port (RSP)	[Robert_Barnes](https://www.iana.org/assignments/telnet-options/telnet-options.xhtml#Robert_Barnes)
    TRSP,
    /// [`consts::option::CPCO`] Com Port Control Option	[RFC2217](http://www.iana.org/go/rfc2217)
    CPCO,
    /// [`consts::option::TSLE`] Telnet Suppress Local Echo	[Wirt_Atmar](https://www.iana.org/assignments/telnet-options/telnet-options.xhtml#Wirt_Atmar)
    TSLE,
    /// [`consts::option::START_TLS`] Telnet Start TLS [Michael_Boe](https://www.iana.org/assignments/telnet-options/telnet-options.xhtml#Michael_Boe)
    StartTLS,
    /// [`consts::option::KERMIT`] Kermit [RFC2840](http://www.iana.org/go/rfc2840)
    Kermit,
    /// [`consts::option::SENDURL`] SEND-URL [David_Croft](https://www.iana.org/assignments/telnet-options/telnet-options.xhtml#David_Croft)
    SendUrl,
    /// [`consts::option::FORWARDX`] FORWARD_X [Jeffrey_Altman](https://www.iana.org/assignments/telnet-options/telnet-options.xhtml#Jeffrey_Altman)
    ForwardX,
    /// [`consts::option::MSDP`] Mud Server Data Protocol [MSDP](https://tintin.sourceforge.io/protocols/msdp/)
    MSDP,
    /// [`consts::option::MSSP`] Mud Server Status Protocol [MSSP](https://tintin.sourceforge.io/protocols/mssp/)
    MSSP,
    /// [`consts::option::COMPRESS1`] Mud Client Compression Protocol version 1 [MCCPv1](http://www.gammon.com.au/mccp/protocol.html)
    Compress1,
    /// [`consts::option::COMPRESS2`] Mud Client Compression Protocol version 2 [MCCPv2](https://tintin.sourceforge.io/protocols/mccp/)
    Compress2,
    /// [`consts::option::ZMP`] Zenith Mud Protocol [ZMP](http://discworld.starturtle.net/external/protocols/zmp.html)
    ZMP,
    /// [`consts::option::PRAGMA_LOGIN`] Telnet Option Pragma Logon [Steve_McGregory](https://www.iana.org/assignments/telnet-options/telnet-options.xhtml#Steve_McGregory)
    PragmaLogon,
    /// [`consts::option::SSPI_LOGIN`] Telnet Option SSPI Logon [Steve_McGregory](https://www.iana.org/assignments/telnet-options/telnet-options.xhtml#Steve_McGregory)
    SSPILogon,
    /// [`consts::option::PRAGMA_HEARTBEAT`] Telnet Option Pragma Heartbeat [Steve_McGregory](https://www.iana.org/assignments/telnet-options/telnet-options.xhtml#Steve_McGregory)
    PragmaHeartbeat,
    /// [`consts::option::GMCP`] Generic Mud Communication Protocol [GMCP Protocol](https://www.gammon.com.au/gmcp)
    GMCP,
    /// [`consts::option::EXOPL`] Extended-Options-List [RFC861](http://www.iana.org/go/rfc861)
    EXOPL,
    /// Unknown Option
    Unknown(u8),
}

impl TelnetOption {
    /// Converts a `TelnetOption` instance into its corresponding `u8` representation.
    ///
    /// # Returns
    ///
    /// This function maps each possible variant of the `TelnetOption` enum to its
    /// associated constant value defined under `consts::option`, or the raw byte value
    /// for the `TelnetOption::Unknown` variant. Each value corresponds to a specific
    /// Telnet sidechannel option code.
    pub fn to_u8(&self) -> u8 {
        match self {
            TelnetOption::TransmitBinary => consts::option::BINARY,
            TelnetOption::Echo => consts::option::ECHO,
            TelnetOption::Reconnection => consts::option::RCP,
            TelnetOption::SuppressGoAhead => consts::option::SGA,
            TelnetOption::NegotiateApproxMessageSize => consts::option::NAMS,
            TelnetOption::Status => consts::option::STATUS,
            TelnetOption::TimingMark => consts::option::TM,
            TelnetOption::RCTE => consts::option::RCTE,
            TelnetOption::OutLineWidth => consts::option::NAOL,
            TelnetOption::OutPageSize => consts::option::NAOP,
            TelnetOption::NAOCRD => consts::option::NAOCRD,
            TelnetOption::NAOHTS => consts::option::NAOHTS,
            TelnetOption::NAOHTD => consts::option::NAOHTD,
            TelnetOption::NAOFFD => consts::option::NAOFFD,
            TelnetOption::NAOVTS => consts::option::NAOVTS,
            TelnetOption::NAOVTD => consts::option::NAOVTD,
            TelnetOption::NAOLFD => consts::option::NAOLFD,
            TelnetOption::XASCII => consts::option::XASCII,
            TelnetOption::Logout => consts::option::LOGOUT,
            TelnetOption::ByteMacro => consts::option::BM,
            TelnetOption::DET => consts::option::DET,
            TelnetOption::SUPDUP => consts::option::SUPDUP,
            TelnetOption::SUPDUPOutput => consts::option::SUPDUP_OUTPUT,
            TelnetOption::SNDLOC => consts::option::SNDLOC,
            TelnetOption::TTYPE => consts::option::TTYPE,
            TelnetOption::EOR => consts::option::EOR,
            TelnetOption::TUID => consts::option::TUID,
            TelnetOption::OUTMRK => consts::option::OUTMRK,
            TelnetOption::TTYLOC => consts::option::TTYLOC,
            TelnetOption::OPT3270Regime => consts::option::OPT3270REGIME,
            TelnetOption::X3PAD => consts::option::X3PAD,
            TelnetOption::NAWS => consts::option::NAWS,
            TelnetOption::TSPEED => consts::option::TSPEED,
            TelnetOption::LFLOW => consts::option::LFLOW,
            TelnetOption::Linemode => consts::option::LINEMODE,
            TelnetOption::XDISPLOC => consts::option::XDISPLOC,
            TelnetOption::Environment => consts::option::OLD_ENVIRONMENT,
            TelnetOption::Authentication => consts::option::AUTHENTICATION,
            TelnetOption::Encryption => consts::option::ENCRYPTION,
            TelnetOption::NewEnvironment => consts::option::NEW_ENVIRONMENT,
            TelnetOption::TN3270E => consts::option::TN3270E,
            TelnetOption::XAUTH => consts::option::XAUTH,
            TelnetOption::Charset => consts::option::CHARSET,
            TelnetOption::TRSP => consts::option::TRSP,
            TelnetOption::CPCO => consts::option::CPCO,
            TelnetOption::TSLE => consts::option::TSLE,
            TelnetOption::StartTLS => consts::option::START_TLS,
            TelnetOption::Kermit => consts::option::KERMIT,
            TelnetOption::SendUrl => consts::option::SENDURL,
            TelnetOption::ForwardX => consts::option::FORWARDX,
            TelnetOption::MSDP => consts::option::MSDP,
            TelnetOption::MSSP => consts::option::MSSP,
            TelnetOption::Compress1 => consts::option::COMPRESS1,
            TelnetOption::Compress2 => consts::option::COMPRESS2,
            TelnetOption::ZMP => consts::option::ZMP,
            TelnetOption::PragmaLogon => consts::option::PRAGMA_LOGIN,
            TelnetOption::SSPILogon => consts::option::SSPI_LOGIN,
            TelnetOption::PragmaHeartbeat => consts::option::PRAGMA_HEARTBEAT,
            TelnetOption::GMCP => consts::option::GMCP,
            TelnetOption::EXOPL => consts::option::EXOPL,
            TelnetOption::Unknown(byte) => *byte,
        }
    }
    /// Converts a `u8` value representing a Telnet option into the corresponding variant of the `TelnetOption` enum.
    ///
    /// # Arguments
    ///
    /// * `byte` - A `u8` value that corresponds to a specific Telnet option as defined by the constants in `consts::option`.
    ///
    /// # Returns
    ///
    /// Returns a variant of the `TelnetOption` enum corresponding to the provided `byte`. If an unknown or unsupported
    /// value is provided, the `TelnetOption::Unknown(byte)` variant is returned containing the original `byte`.
    pub fn from_u8(byte: u8) -> Self {
        match byte {
            consts::option::BINARY => TelnetOption::TransmitBinary,
            consts::option::ECHO => TelnetOption::Echo,
            consts::option::RCP => TelnetOption::Reconnection,
            consts::option::SGA => TelnetOption::SuppressGoAhead,
            consts::option::NAMS => TelnetOption::NegotiateApproxMessageSize,
            consts::option::STATUS => TelnetOption::Status,
            consts::option::TM => TelnetOption::TimingMark,
            consts::option::RCTE => TelnetOption::RCTE,
            consts::option::NAOL => TelnetOption::OutLineWidth,
            consts::option::NAOP => TelnetOption::OutPageSize,
            consts::option::NAOCRD => TelnetOption::NAOCRD,
            consts::option::NAOHTS => TelnetOption::NAOHTS,
            consts::option::NAOHTD => TelnetOption::NAOHTD,
            consts::option::NAOFFD => TelnetOption::NAOFFD,
            consts::option::NAOVTS => TelnetOption::NAOVTS,
            consts::option::NAOVTD => TelnetOption::NAOVTD,
            consts::option::NAOLFD => TelnetOption::NAOLFD,
            consts::option::XASCII => TelnetOption::XASCII,
            consts::option::LOGOUT => TelnetOption::Logout,
            consts::option::BM => TelnetOption::ByteMacro,
            consts::option::DET => TelnetOption::DET,
            consts::option::SUPDUP => TelnetOption::SUPDUP,
            consts::option::SUPDUP_OUTPUT => TelnetOption::SUPDUPOutput,
            consts::option::SNDLOC => TelnetOption::SNDLOC,
            consts::option::TTYPE => TelnetOption::TTYPE,
            consts::option::EOR => TelnetOption::EOR,
            consts::option::TUID => TelnetOption::TUID,
            consts::option::OUTMRK => TelnetOption::OUTMRK,
            consts::option::TTYLOC => TelnetOption::TTYLOC,
            consts::option::OPT3270REGIME => TelnetOption::OPT3270Regime,
            consts::option::X3PAD => TelnetOption::X3PAD,
            consts::option::NAWS => TelnetOption::NAWS,
            consts::option::TSPEED => TelnetOption::TSPEED,
            consts::option::LFLOW => TelnetOption::LFLOW,
            consts::option::LINEMODE => TelnetOption::Linemode,
            consts::option::XDISPLOC => TelnetOption::XDISPLOC,
            consts::option::OLD_ENVIRONMENT => TelnetOption::Environment,
            consts::option::AUTHENTICATION => TelnetOption::Authentication,
            consts::option::ENCRYPTION => TelnetOption::Encryption,
            consts::option::NEW_ENVIRONMENT => TelnetOption::NewEnvironment,
            consts::option::TN3270E => TelnetOption::TN3270E,
            consts::option::XAUTH => TelnetOption::XAUTH,
            consts::option::CHARSET => TelnetOption::Charset,
            consts::option::TRSP => TelnetOption::TRSP,
            consts::option::CPCO => TelnetOption::CPCO,
            consts::option::TSLE => TelnetOption::TSLE,
            consts::option::START_TLS => TelnetOption::StartTLS,
            consts::option::KERMIT => TelnetOption::Kermit,
            consts::option::SENDURL => TelnetOption::SendUrl,
            consts::option::FORWARDX => TelnetOption::ForwardX,
            consts::option::MSDP => TelnetOption::MSDP,
            consts::option::MSSP => TelnetOption::MSSP,
            consts::option::COMPRESS1 => TelnetOption::Compress1,
            consts::option::COMPRESS2 => TelnetOption::Compress2,
            consts::option::ZMP => TelnetOption::ZMP,
            consts::option::PRAGMA_LOGIN => TelnetOption::PragmaLogon,
            consts::option::SSPI_LOGIN => TelnetOption::SSPILogon,
            consts::option::PRAGMA_HEARTBEAT => TelnetOption::PragmaHeartbeat,
            consts::option::GMCP => TelnetOption::GMCP,
            consts::option::EXOPL => TelnetOption::EXOPL,
            byte => TelnetOption::Unknown(byte),
        }
    }
    /// Whether we support this option from us -> them.
    pub fn supported_local(&self) -> bool {
        consts::option::SUPPORT[self.to_u8() as usize].0
    }
    /// Whether we support this option from them -> us.
    pub fn supported_remote(&self) -> bool {
        consts::option::SUPPORT[self.to_u8() as usize].1
    }
}

impl std::fmt::Display for TelnetOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TelnetOption::TransmitBinary => write!(f, "TransmitBinary"),
            TelnetOption::Echo => write!(f, "Echo"),
            TelnetOption::Reconnection => write!(f, "Reconnection"),
            TelnetOption::SuppressGoAhead => write!(f, "SuppressGoAhead"),
            TelnetOption::NegotiateApproxMessageSize => write!(f, "NegotiateApproxMessageSize"),
            TelnetOption::Status => write!(f, "Status"),
            TelnetOption::TimingMark => write!(f, "TimingMark"),
            TelnetOption::RCTE => write!(f, "RCTE"),
            TelnetOption::OutLineWidth => write!(f, "OutLineWidth"),
            TelnetOption::OutPageSize => write!(f, "OutPageSize"),
            TelnetOption::NAOCRD => write!(f, "NAOCRD"),
            TelnetOption::NAOHTS => write!(f, "NAOHTS"),
            TelnetOption::NAOHTD => write!(f, "NAOHTD"),
            TelnetOption::NAOFFD => write!(f, "NAOFFD"),
            TelnetOption::NAOVTS => write!(f, "NAOVTS"),
            TelnetOption::NAOVTD => write!(f, "NAOVTD"),
            TelnetOption::NAOLFD => write!(f, "NAOLFD"),
            TelnetOption::XASCII => write!(f, "XASCII"),
            TelnetOption::Logout => write!(f, "Logout"),
            TelnetOption::ByteMacro => write!(f, "ByteMacro"),
            TelnetOption::DET => write!(f, "DET"),
            TelnetOption::SUPDUP => write!(f, "SUPDUP"),
            TelnetOption::SUPDUPOutput => write!(f, "SUPDUPOutput"),
            TelnetOption::SNDLOC => write!(f, "SNDLOC"),
            TelnetOption::TTYPE => write!(f, "TTYPE"),
            TelnetOption::EOR => write!(f, "EOR"),
            TelnetOption::TUID => write!(f, "TUID"),
            TelnetOption::OUTMRK => write!(f, "OUTMRK"),
            TelnetOption::TTYLOC => write!(f, "TTYLOC"),
            TelnetOption::OPT3270Regime => write!(f, "OPT3270Regime"),
            TelnetOption::X3PAD => write!(f, "X3PAD"),
            TelnetOption::NAWS => write!(f, "NAWS"),
            TelnetOption::TSPEED => write!(f, "TSPEED"),
            TelnetOption::LFLOW => write!(f, "LFLOW"),
            TelnetOption::Linemode => write!(f, "Linemode"),
            TelnetOption::XDISPLOC => write!(f, "XDISPLOC"),
            TelnetOption::Environment => write!(f, "Environment"),
            TelnetOption::Authentication => write!(f, "Authentication"),
            TelnetOption::Encryption => write!(f, "Encryption"),
            TelnetOption::NewEnvironment => write!(f, "NewEnvironment"),
            TelnetOption::TN3270E => write!(f, "TN3270E"),
            TelnetOption::XAUTH => write!(f, "XAUTH"),
            TelnetOption::Charset => write!(f, "Charset"),
            TelnetOption::TRSP => write!(f, "TRSP"),
            TelnetOption::CPCO => write!(f, "CPCO"),
            TelnetOption::TSLE => write!(f, "TSLE"),
            TelnetOption::StartTLS => write!(f, "StartTLS"),
            TelnetOption::Kermit => write!(f, "Kermit"),
            TelnetOption::SendUrl => write!(f, "SendUrl"),
            TelnetOption::ForwardX => write!(f, "ForwardX"),
            TelnetOption::MSDP => write!(f, "MSDP"),
            TelnetOption::MSSP => write!(f, "MSSP"),
            TelnetOption::Compress1 => write!(f, "Compress1"),
            TelnetOption::Compress2 => write!(f, "Compress2"),
            TelnetOption::ZMP => write!(f, "ZMP"),
            TelnetOption::PragmaLogon => write!(f, "PragmaLogon"),
            TelnetOption::SSPILogon => write!(f, "SSPILogon"),
            TelnetOption::PragmaHeartbeat => write!(f, "PragmaHeartbeat"),
            TelnetOption::GMCP => write!(f, "GMCP"),
            TelnetOption::EXOPL => write!(f, "EXOPL"),
            TelnetOption::Unknown(option) => write!(f, "Unknown({option})"),
        }
    }
}

impl From<u8> for TelnetOption {
    fn from(byte: u8) -> Self {
        Self::from_u8(byte)
    }
}

impl From<TelnetOption> for u8 {
    fn from(option: TelnetOption) -> Self {
        option.to_u8()
    }
}

/// A structure representing the configuration and state of Telnet options.
///
/// The `TelnetOptions` struct is designed to manage the options available within
/// a Telnet sidechannel implementation. It tracks the support state of each option
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
/// The Telnet sidechannel uses option codes ranging from 0 to 254, which makes the size
/// of both arrays precisely 255 to represent all potential options. The fields ensure
/// the ability to handle and manage all standard Telnet options.
#[derive(Clone, Debug)]
pub struct TelnetOptions {
    config: [SupportState; 255],
    state: [OptionState; 255],
}

impl TelnetOptions {
    /// Checks if we support the given option locally
    pub fn is_supported_local(&self, option: TelnetOption) -> bool {
        self.config[option.to_u8() as usize].local
    }

    /// Checks if we support the given option remotely
    pub fn is_supported_remote(&self, option: TelnetOption) -> bool {
        self.config[option.to_u8() as usize].remote
    }

    /// Gets the local QState for an option
    pub(crate) fn local_qstate(&self, option: TelnetOption) -> QState {
        let option_idx = option.to_u8() as usize;
        if option_idx >= self.state.len() {
            return QState::No;
        }
        self.state[option_idx].local
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
        let option_idx = option.to_u8() as usize;
        if option_idx >= self.state.len() {
            return false;
        }
        matches!(
            self.state[option_idx].local,
            QState::Yes | QState::WantNo | QState::WantNoOpposite
        )
    }

    /// Gets the remote QState for an option
    pub(crate) fn remote_qstate(&self, option: TelnetOption) -> QState {
        let option_idx = option.to_u8() as usize;
        if option_idx >= self.state.len() {
            return QState::No;
        }
        self.state[option_idx].remote
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
        let option_idx = option.to_u8() as usize;
        if option_idx >= self.state.len() {
            return false;
        }
        matches!(
            self.state[option_idx].remote,
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
    /// perform a specific feature or capability defined by the Telnet sidechannel.
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
    pub fn handle_received(
        &mut self,
        frame: TelnetFrame,
    ) -> TelnetCodecResult<Option<TelnetFrame>> {
        match frame {
            TelnetFrame::Do(option) => Ok(self.recv_do(option)),
            TelnetFrame::Dont(option) => Ok(self.recv_dont(option)),
            TelnetFrame::Will(option) => Ok(self.recv_will(option)),
            TelnetFrame::Wont(option) => Ok(self.recv_wont(option)),
            _ => Err(TelnetCodecError::NegotiationError {
                reason: "Unsupported frame type".into(),
                frame_type: Some(format!("{:?}", frame)),
            }),
        }
    }

    // #### Outgoing requests (what we initiate) ################################

    /// Request that *we* enable the option (i.e. we want to send WILL).
    /// Returns frames (commands) you should send on the wire as a result.
    fn request_will(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        // First check if we support providing this option
        if !self.config[option.to_u8() as usize].local {
            return None; // Don't try to enable unsupported options
        }
        match self.state[option.to_u8() as usize].local {
            QState::Yes | QState::WantYes | QState::WantYesOpposite => {
                // already enabled or in-progress to enable
                None
            }
            QState::No => {
                // Start negotiation: send WILL -> state WANTYES
                self.state[option.to_u8() as usize].local = QState::WantYes;
                Some(TelnetFrame::Will(option))
            }
            QState::WantNo => {
                // collision: we were trying to disable, but now ask enable -> WantYesOpposite
                self.state[option.to_u8() as usize].local = QState::WantYesOpposite;
                Some(TelnetFrame::Will(option))
            }
            QState::WantNoOpposite => {
                // will -> wantyes (opposite canceled)
                self.state[option.to_u8() as usize].local = QState::WantYes;
                Some(TelnetFrame::Will(option))
            }
        }
    }

    /// Request that *we* disable the option (i.e. we want to send WONT).
    fn request_wont(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        match self.state[option.to_u8() as usize].local {
            QState::No | QState::WantNo | QState::WantNoOpposite => {
                // already disabled or in-progress to disable
                None
            }
            QState::Yes => {
                // start disable: WONT -> WANTNO
                self.state[option.to_u8() as usize].local = QState::WantNo;
                Some(TelnetFrame::Wont(option))
            }
            QState::WantYes => {
                // collision -> WantNoOpposite
                self.state[option.to_u8() as usize].local = QState::WantNoOpposite;
                Some(TelnetFrame::Wont(option))
            }
            QState::WantYesOpposite => {
                self.state[option.to_u8() as usize].local = QState::WantNo;
                Some(TelnetFrame::Wont(option))
            }
        }
    }

    /// Request remote to enable the option (i.e. send DO).
    fn request_do(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        // First check if we support providing this option
        if !self.config[option.to_u8() as usize].remote {
            return None; // Don't try to enable unsupported options
        }
        match self.state[option.to_u8() as usize].remote {
            QState::Yes | QState::WantYes | QState::WantYesOpposite => None,
            QState::No => {
                self.state[option.to_u8() as usize].remote = QState::WantYes;
                Some(TelnetFrame::Do(option))
            }
            QState::WantNo => {
                self.state[option.to_u8() as usize].remote = QState::WantYesOpposite;
                Some(TelnetFrame::Do(option))
            }
            QState::WantNoOpposite => {
                self.state[option.to_u8() as usize].remote = QState::WantYes;
                Some(TelnetFrame::Do(option))
            }
        }
    }

    /// Request remote to disable the option (i.e. send DONT).
    fn request_dont(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        match self.state[option.to_u8() as usize].remote {
            QState::No | QState::WantNo | QState::WantNoOpposite => None,
            QState::Yes => {
                self.state[option.to_u8() as usize].remote = QState::WantNo;
                Some(TelnetFrame::Dont(option))
            }
            QState::WantYes => {
                self.state[option.to_u8() as usize].remote = QState::WantNoOpposite;
                Some(TelnetFrame::Dont(option))
            }
            QState::WantYesOpposite => {
                self.state[option.to_u8() as usize].remote = QState::WantNo;
                Some(TelnetFrame::Dont(option))
            }
        }
    }

    // #### Incoming processing (peer sent us DO/DONT/WILL/WONT) ##################

    /// Process an incoming WILL from remote (they say "I will do option").
    /// Returns frames to send in response (if any).
    fn recv_will(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        let option_idx = option.to_u8() as usize;
        // Handle out-of-bounds option (e.g., Unknown option with value 255)
        if option_idx >= self.config.len() {
            return Some(TelnetFrame::Dont(option));
        }
        // First, check if we support providing this option
        if !self.config[option_idx].remote {
            // If we don't support it, reject with `DONT`
            return Some(TelnetFrame::Dont(option));
        }
        match self.state[option_idx].remote {
            QState::No => {
                // remote offers WILL -> if we accept, send DO and move to YES
                // For a generic engine we accept by default; caller can override by sending DONT.
                // Here we accept. If you want to implement policy, change this branch.
                self.state[option_idx].remote = QState::Yes;
                Some(TelnetFrame::Do(option))
            }
            QState::Yes => {
                // already yes -> no response
                None
            }
            QState::WantNo => {
                // peer is contradicting our previous DONT: move to No (or remain?) RFC1143:
                // WANTNO + WILL => WANTNO-OPPOSITE -> send DONT
                self.state[option_idx].remote = QState::WantNoOpposite;
                Some(TelnetFrame::Dont(option))
            }
            QState::WantNoOpposite => {
                // collision resolved -> YES
                self.state[option_idx].remote = QState::Yes;
                None // no further response
            }
            QState::WantYes => {
                // we asked for it, and peer confirms -> YES
                self.state[option_idx].remote = QState::Yes;
                None
            }
            QState::WantYesOpposite => {
                // double negotiation: move to YES
                self.state[option_idx].remote = QState::Yes;
                None
            }
        }
    }

    /// Process incoming WONT from remote.
    fn recv_wont(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        let option_idx = option.to_u8() as usize;
        // Handle out-of-bounds option (e.g., Unknown option with value 255)
        if option_idx >= self.state.len() {
            return None; // No response needed for unknown options
        }
        match self.state[option_idx].remote {
            QState::No => None, // already no
            QState::Yes => {
                self.state[option_idx].remote = QState::No;
                // if we expected it, nothing to send; RFC1143: no immediate reply
                None
            }
            QState::WantNo => {
                self.state[option_idx].remote = QState::No;
                None
            }
            QState::WantNoOpposite => {
                // remote confirmed refusal -> NO
                self.state[option_idx].remote = QState::No;
                None
            }
            QState::WantYes => {
                // requested YES, but peer refuses -> NO and maybe clear want
                self.state[option_idx].remote = QState::No;
                None
            }
            QState::WantYesOpposite => {
                self.state[option_idx].remote = QState::No;
                None
            }
        }
    }

    /// Process incoming DO (peer requests we enable option -> they ask us to send WILL/WONT).
    fn recv_do(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        let option_idx = option.to_u8() as usize;
        // Handle out-of-bounds option (e.g., Unknown option with value 255)
        if option_idx >= self.config.len() {
            return Some(TelnetFrame::Wont(option));
        }
        // First, check if we support providing this option
        if !self.config[option_idx].local {
            // If we don't support it, reject with `WONT`
            return Some(TelnetFrame::Wont(option));
        }
        match self.state[option_idx].local {
            QState::No => {
                // peer asks us to enable: we accept by default -> send WILL
                self.state[option_idx].local = QState::Yes;
                Some(TelnetFrame::Will(option))
            }
            QState::Yes => None,
            QState::WantNo => {
                self.state[option_idx].local = QState::WantNoOpposite;
                Some(TelnetFrame::Will(option))
            }
            QState::WantNoOpposite => {
                self.state[option_idx].local = QState::Yes;
                None
            }
            QState::WantYes => {
                self.state[option_idx].local = QState::Yes;
                None
            }
            QState::WantYesOpposite => {
                self.state[option_idx].local = QState::Yes;
                None
            }
        }
    }

    /// Process incoming DONT (peer asks us not to enable option -> they ask we send WONT).
    fn recv_dont(&mut self, option: TelnetOption) -> Option<TelnetFrame> {
        let option_idx = option.to_u8() as usize;
        // Handle out-of-bounds option (e.g., Unknown option with value 255)
        if option_idx >= self.state.len() {
            return None; // No response needed for unknown options
        }
        match self.state[option_idx].local {
            QState::No => None,
            QState::Yes => {
                self.state[option_idx].local = QState::No;
                None
            }
            QState::WantNo => {
                self.state[option_idx].local = QState::No;
                None
            }
            QState::WantNoOpposite => {
                self.state[option_idx].local = QState::No;
                None
            }
            QState::WantYes => {
                self.state[option_idx].local = QState::No;
                None
            }
            QState::WantYesOpposite => {
                self.state[option_idx].local = QState::No;
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

/// Represents the perspective of a Telnet option in a client-server negotiation.
///
/// In the Telnet sidechannel, option negotiation involves two independent paths:
/// one for local options (what the local side wants to do) and one for remote options
/// (what the remote side wants to do). `OptionSide` disambiguates between these two
/// perspectives when managing option state.
///
/// # Variants
///
/// ## `Local`
///
/// Represents the local side of the connection. This is the perspective of the endpoint
/// that is currently executing this code.
///
/// When applied to option negotiation:
/// - Used when *we* want to enable or disable an option
/// - We send `WILL` (agreement to perform) or `WONT` (refusal to perform) commands
/// - The remote side responds with `DO` (request us to perform) or `DONT` (request us not to perform)
///
/// # Examples
///
/// Requesting that *we* start performing an option:
/// ```text
/// Local: WILL <option>  →  Remote
/// Remote: DO <option>   →  Local
/// ```
///
/// ## `Remote`
///
/// Represents the remote side of the connection. This is the perspective of the peer
/// endpoint that we are communicating with.
///
/// When applied to option negotiation:
/// - Used when *we* want the remote side to enable or disable an option
/// - We send `DO` (request them to perform) or `DONT` (request them not to perform) commands
/// - The remote side responds with `WILL` (agreement to perform) or `WONT` (refusal to perform)
///
/// # Examples
///
/// Requesting that the *remote side* start performing an option:
/// ```text
/// Local: DO <option>    →  Remote
/// Remote: WILL <option> →  Local
/// ```
///
/// # Usage in Option State Machine
///
/// Each Telnet option maintains two independent state machines (using the Q-method defined in RFC 1143):
/// one for the local side and one for the remote side. `OptionSide` is used to determine which
/// state machine path to follow:
///
/// - **Local path**: Tracks whether *we* are performing an option
///   - Initial commands: `WILL` / `WONT`
///   - Expected responses: `DO` / `DONT`
///   - Final states: enabled (performing) or disabled (not performing)
///
/// - **Remote path**: Tracks whether the *remote side* is performing an option
///   - Initial commands: `DO` / `DONT`
///   - Expected responses: `WILL` / `WONT`
///   - Final states: enabled (remote is performing) or disabled (remote not performing)
///
/// # See Also
///
/// - [`TelnetOption`]: The specific Telnet option being negotiated
/// - [`TelnetOptions`]: Manages both local and remote state for all options
/// - [`QState`]: The negotiation state machine states
///
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TelnetSide {
    /// The local side of the Telnet connection (what we want to do)
    Local,
    /// The remote side of the Telnet connection (what the peer wants to do)
    Remote,
}

impl std::fmt::Display for TelnetSide {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TelnetSide::Local => write!(f, "Local"),
            TelnetSide::Remote => write!(f, "Remote"),
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
pub(crate) enum QState {
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

impl std::fmt::Display for QState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QState::No => write!(f, "No"),
            QState::WantNo => write!(f, "WantNo"),
            QState::WantNoOpposite => write!(f, "WantNoOpposite"),
            QState::Yes => write!(f, "Yes"),
            QState::WantYes => write!(f, "WantYes"),
            QState::WantYesOpposite => write!(f, "WantYesOpposite"),
        }
    }
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
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Initial state should be No
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::No);
        assert!(!opts.local_enabled(opt));

        // Request to enable local option
        let frame = opts.enable_local(opt);
        assert_eq!(frame, Some(TelnetFrame::Will(opt)));
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::WantYes);
        assert!(!opts.local_enabled(opt)); // Still not enabled until confirmed
    }

    #[test]
    fn test_local_enable_recv_do_completes_to_yes() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Move to WantYes state
        opts.enable_local(opt);
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::WantYes);

        // Receive DO from remote
        let response = opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(response, None); // No response needed
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::Yes);
        assert!(opts.local_enabled(opt));
    }

    #[test]
    fn test_local_enable_idempotent_when_yes() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes state
        opts.enable_local(opt);
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::Yes);

        // Try to enable again
        let frame = opts.enable_local(opt);
        assert_eq!(frame, None); // No frame sent
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::Yes);
    }

    #[test]
    fn test_local_enable_idempotent_when_wantyes() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Move to WantYes
        opts.enable_local(opt);
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::WantYes);

        // Try to enable again
        let frame = opts.enable_local(opt);
        assert_eq!(frame, None);
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::WantYes);
    }

    // ============================================================================
    // Local Option Disable Tests (We send WONT, they send DONT)
    // ============================================================================

    #[test]
    fn test_local_disable_from_yes_to_wantno() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes state first
        opts.enable_local(opt);
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::Yes);

        // Disable
        let frame = opts.disable_local(opt);
        assert_eq!(frame, Some(TelnetFrame::Wont(opt)));
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::WantNo);
        assert!(opts.local_enabled(opt)); // Still enabled until confirmed
    }

    #[test]
    fn test_local_disable_recv_dont_completes_to_no() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes, then WantNo
        opts.enable_local(opt);
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        opts.disable_local(opt);
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::WantNo);

        // Receive DONT from remote
        let response = opts.handle_received(TelnetFrame::Dont(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::No);
        assert!(!opts.local_enabled(opt));
    }

    #[test]
    fn test_local_disable_idempotent_when_no() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Already at No
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::No);

        // Try to disable
        let frame = opts.disable_local(opt);
        assert_eq!(frame, None);
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::No);
    }

    // ============================================================================
    // Remote Option Enable Tests (We send DO, they send WILL)
    // ============================================================================

    #[test]
    fn test_remote_enable_from_no_to_wantyes() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Initial state should be No
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::No);
        assert!(!opts.remote_enabled(opt));

        // Request to enable remote option
        let frame = opts.enable_remote(opt);
        assert_eq!(frame, Some(TelnetFrame::Do(opt)));
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::WantYes);
        assert!(!opts.remote_enabled(opt));
    }

    #[test]
    fn test_remote_enable_recv_will_completes_to_yes() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Move to WantYes state
        opts.enable_remote(opt);
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::WantYes);

        // Receive WILL from remote
        let response = opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::Yes);
        assert!(opts.remote_enabled(opt));
    }

    #[test]
    fn test_remote_enable_idempotent_when_yes() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes state
        opts.enable_remote(opt);
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::Yes);

        // Try to enable again
        let frame = opts.enable_remote(opt);
        assert_eq!(frame, None);
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::Yes);
    }

    // ============================================================================
    // Remote Option Disable Tests (We send DONT, they send WONT)
    // ============================================================================

    #[test]
    fn test_remote_disable_from_yes_to_wantno() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes state first
        opts.enable_remote(opt);
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::Yes);

        // Disable
        let frame = opts.disable_remote(opt);
        assert_eq!(frame, Some(TelnetFrame::Dont(opt)));
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::WantNo);
        assert!(opts.remote_enabled(opt)); // Still enabled until confirmed
    }

    #[test]
    fn test_remote_disable_recv_wont_completes_to_no() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes, then WantNo
        opts.enable_remote(opt);
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        opts.disable_remote(opt);
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::WantNo);

        // Receive WONT from remote
        let response = opts.handle_received(TelnetFrame::Wont(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::No);
        assert!(!opts.remote_enabled(opt));
    }

    #[test]
    fn test_remote_disable_idempotent_when_no() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Already at No
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::No);

        // Try to disable
        let frame = opts.disable_remote(opt);
        assert_eq!(frame, None);
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::No);
    }

    // ============================================================================
    // Unsolicited Remote Requests (They initiate)
    // ============================================================================

    #[test]
    fn test_recv_will_from_no_accepts_to_yes() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Start at No
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::No);

        // Remote sends WILL
        let response = opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(response, Some(TelnetFrame::Do(opt)));
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::Yes);
        assert!(opts.remote_enabled(opt));
    }

    #[test]
    fn test_recv_will_when_yes_no_response() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::Yes);

        // Remote sends WILL again
        let response = opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::Yes);
    }

    #[test]
    fn test_recv_do_from_no_accepts_to_yes() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Start at No
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::No);

        // Remote sends DO
        let response = opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(response, Some(TelnetFrame::Will(opt)));
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::Yes);
        assert!(opts.local_enabled(opt));
    }

    #[test]
    fn test_recv_do_when_yes_no_response() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::Yes);

        // Remote sends DO again
        let response = opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::Yes);
    }

    #[test]
    fn test_recv_wont_from_yes_to_no() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::Yes);

        // Remote sends WONT
        let response = opts.handle_received(TelnetFrame::Wont(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::No);
        assert!(!opts.remote_enabled(opt));
    }

    #[test]
    fn test_recv_dont_from_yes_to_no() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::Yes);

        // Remote sends DONT
        let response = opts.handle_received(TelnetFrame::Dont(opt)).unwrap();
        assert_eq!(response, None);
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::No);
        assert!(!opts.local_enabled(opt));
    }

    // ============================================================================
    // Collision Tests (Both sides negotiate simultaneously)
    // ============================================================================

    #[test]
    fn test_collision_enable_local_while_wantno() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes then start disabling
        opts.enable_local(opt);
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        opts.disable_local(opt);
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::WantNo);

        // Try to enable again (collision)
        let frame = opts.enable_local(opt);
        assert_eq!(frame, Some(TelnetFrame::Will(opt)));
        assert_eq!(
            opts.state[opt.to_u8() as usize].local,
            QState::WantYesOpposite
        );
    }

    #[test]
    fn test_collision_disable_local_while_wantyes() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Start enabling
        opts.enable_local(opt);
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::WantYes);

        // Try to disable (collision)
        let frame = opts.disable_local(opt);
        assert_eq!(frame, Some(TelnetFrame::Wont(opt)));
        assert_eq!(
            opts.state[opt.to_u8() as usize].local,
            QState::WantNoOpposite
        );
    }

    #[test]
    fn test_recv_do_while_wantno_stays_wantno_opposite() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes, then WantNo
        opts.enable_local(opt);
        opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        opts.disable_local(opt);
        assert_eq!(opts.state[opt.to_u8() as usize].local, QState::WantNo);

        // Remote sends DO (collision)
        let response = opts.handle_received(TelnetFrame::Do(opt)).unwrap();
        assert_eq!(response, Some(TelnetFrame::Will(opt)));
        assert_eq!(
            opts.state[opt.to_u8() as usize].local,
            QState::WantNoOpposite
        );
    }

    #[test]
    fn test_recv_will_while_wantno_stays_wantno_opposite() {
        let mut opts = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Get to Yes, then WantNo
        opts.enable_remote(opt);
        opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        opts.disable_remote(opt);
        assert_eq!(opts.state[opt.to_u8() as usize].remote, QState::WantNo);

        // Remote sends WILL again (collision)
        let response = opts.handle_received(TelnetFrame::Will(opt)).unwrap();
        assert_eq!(response, Some(TelnetFrame::Dont(opt)));
        assert_eq!(
            opts.state[opt.to_u8() as usize].remote,
            QState::WantNoOpposite
        );
    }

    // ============================================================================
    // Full Handshake Integration Tests
    // ============================================================================

    #[test]
    fn test_full_local_enable_disable_handshake() {
        let mut client = TelnetOptions::default();
        let mut server = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Enable: Client sends WILL
        let will = client.enable_local(opt).unwrap();
        assert_eq!(will, TelnetFrame::Will(opt));
        assert_eq!(client.state[opt.to_u8() as usize].local, QState::WantYes);

        // Server receives WILL, responds with DO
        let do_frame = server.handle_received(will).unwrap().unwrap();
        assert_eq!(do_frame, TelnetFrame::Do(opt));
        assert_eq!(server.state[opt.to_u8() as usize].remote, QState::Yes);
        assert!(server.remote_enabled(opt));

        // Client receives DO, completes to Yes
        let none = client.handle_received(do_frame).unwrap();
        assert_eq!(none, None);
        assert_eq!(client.state[opt.to_u8() as usize].local, QState::Yes);
        assert!(client.local_enabled(opt));

        // Disable: Client sends WONT
        let wont = client.disable_local(opt).unwrap();
        assert_eq!(wont, TelnetFrame::Wont(opt));
        assert_eq!(client.state[opt.to_u8() as usize].local, QState::WantNo);

        // Server receives WONT, moves to No
        let none = server.handle_received(wont).unwrap();
        assert_eq!(none, None);
        assert_eq!(server.state[opt.to_u8() as usize].remote, QState::No);
        assert!(!server.remote_enabled(opt));
        assert_eq!(client.state[opt.to_u8() as usize].local, QState::WantNo);

        // Client state is still WantNo (waiting for explicit DONT)
        // In practice, the server could send DONT to confirm
    }

    #[test]
    fn test_full_remote_enable_disable_handshake() {
        let mut client = TelnetOptions::default();
        let mut server = TelnetOptions::default();
        let opt = TelnetOption::TransmitBinary;

        // Enable: Client sends DO
        let do_frame = client.enable_remote(opt).unwrap();
        assert_eq!(do_frame, TelnetFrame::Do(opt));
        assert_eq!(client.state[opt.to_u8() as usize].remote, QState::WantYes);

        // Server receives DO, responds with WILL
        let will = server.handle_received(do_frame).unwrap().unwrap();
        assert_eq!(will, TelnetFrame::Will(opt));
        assert_eq!(server.state[opt.to_u8() as usize].local, QState::Yes);
        assert!(server.local_enabled(opt));

        // Client receives WILL, completes to Yes
        let none = client.handle_received(will).unwrap();
        assert_eq!(none, None);
        assert_eq!(client.state[opt.to_u8() as usize].remote, QState::Yes);
        assert!(client.remote_enabled(opt));

        // Disable: Client sends DONT
        let dont = client.disable_remote(opt).unwrap();
        assert_eq!(dont, TelnetFrame::Dont(opt));
        assert_eq!(client.state[opt.to_u8() as usize].remote, QState::WantNo);

        // Server receives DONT, moves to No
        let none = server.handle_received(dont).unwrap();
        assert_eq!(none, None);
        assert_eq!(server.state[opt.to_u8() as usize].local, QState::No);
        assert!(!server.local_enabled(opt));
    }
}
