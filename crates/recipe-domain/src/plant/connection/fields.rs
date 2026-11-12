//! Parsed fields of a connection request (ADR-0154, design decision D3).

use super::{CommMode, PlcAddress, PlcHost, PlcLogin, PlcPassword, Rack, Slot, TcpPort};

/// The parsed fields of a connection request.
///
/// `recipe-web` turns an empty field text into `None` before it parses a value.
/// The request holds the fields of both shapes. `PlcConnection::for_platform` refuses
/// a value that the platform does not use.
/// The type has no `Clone`, because [`PlcPassword`] has none.
#[derive(Debug, Default)]
pub struct ConnectionFields {
    /// Host of the PLC. Every platform uses it.
    pub host: Option<PlcHost>,
    /// Rack of an S7 PLC.
    pub rack: Option<Rack>,
    /// Slot of an S7 PLC.
    pub slot: Option<Slot>,
    /// TCP port of an IEC PLC.
    pub port: Option<TcpPort>,
    /// Address of an IEC PLC.
    pub address: Option<PlcAddress>,
    /// Communication mode of an IEC PLC.
    pub mode: Option<CommMode>,
    /// Login of an IEC PLC.
    pub login: Option<PlcLogin>,
    /// New password of an IEC PLC. `None` means that the field is empty.
    pub password: Option<PlcPassword>,
}
