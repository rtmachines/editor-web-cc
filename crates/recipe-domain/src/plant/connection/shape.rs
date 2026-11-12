//! Platform shape of a PLC connection (ADR-0154, design decision D3).

use super::{
    AmsNetId, CommMode, ConnectionFields, PasswordPlan, PasswordState, PlcAddress, PlcHost,
    PlcLogin, PlcPassword, Rack, Slot, TcpPort,
};
use crate::plant::{MachineCode, PlantError, PlantErrors, PlcPlatform};

/// The connection settings of a PLC. Each platform family has one shape.
#[derive(Debug)]
pub enum PlcConnection {
    /// A Siemens S7 PLC.
    S7 {
        /// Host of the PLC.
        host: PlcHost,
        /// Rack of the PLC.
        rack: Rack,
        /// Slot of the PLC.
        slot: Slot,
    },
    /// A PLC with an IEC 61131-3 runtime.
    Iec {
        /// Host of the PLC.
        host: PlcHost,
        /// TCP port of the PLC.
        port: TcpPort,
        /// Address of the PLC.
        address: PlcAddress,
        /// Communication mode.
        mode: CommMode,
        /// Login. The password is not part of the value, see [`PasswordPlan`].
        login: Option<PlcLogin>,
    },
}

/// The result of [`PlcConnection::for_platform`].
#[derive(Debug)]
pub struct ConnectionPlan {
    /// The checked connection settings.
    pub connection: PlcConnection,
    /// The action on the stored password. An S7 record never holds a password, so it has no plan.
    pub password: Option<PasswordPlan>,
}

impl PlcConnection {
    /// Check the fields of a request against a platform and build the connection settings.
    ///
    /// The function lists every fault in one [`PlantErrors`] (ADR-0041).
    /// `machine` names the machine in each error. It is no part of the settings.
    ///
    /// # Errors
    ///
    /// Returns the faults of the request in the order of `domain-model.md` section 2.1.
    pub fn for_platform(
        machine: MachineCode,
        platform: PlcPlatform,
        fields: ConnectionFields,
        stored: PasswordState,
    ) -> Result<ConnectionPlan, PlantErrors> {
        let mut faults = Faults::new(machine, platform);
        let built = if platform.is_s7() {
            s7_plan(&mut faults, fields)
        } else {
            iec_plan(&mut faults, fields, stored)
        };
        faults.finish(built)
    }
}

/// Collects the faults of one request in the order that the checks find them.
struct Faults {
    machine: MachineCode,
    platform: PlcPlatform,
    errors: Vec<PlantError>,
}

impl Faults {
    fn new(machine: MachineCode, platform: PlcPlatform) -> Self {
        Self {
            machine,
            platform,
            errors: Vec::new(),
        }
    }

    /// Record `E-PLT-022` for each named field that holds a value.
    fn refuse_on_platform(&mut self, fields: &[(&'static str, bool)]) {
        let refused = fields
            .iter()
            .filter(|(_, has_value)| *has_value)
            .map(|(field, _)| PlantError::ConnectionFieldNotOnPlatform {
                code: self.machine,
                field,
                platform: self.platform,
            });
        self.errors.extend(refused);
    }

    /// Return the value of a required field. Record `E-PLT-023` when the field has no value.
    fn require<T>(&mut self, field: &'static str, value: Option<T>) -> Option<T> {
        if value.is_none() {
            self.errors.push(PlantError::ConnectionFieldMissing {
                code: self.machine,
                field,
            });
        }
        value
    }

    /// Record `E-PLT-025` when the platform does not permit the mode.
    fn check_mode(&mut self, mode: CommMode) {
        let permitted = match self.platform {
            PlcPlatform::CodesysIec => mode != CommMode::Ads,
            PlcPlatform::TwinCat3 => mode != CommMode::CodesysV3,
            PlcPlatform::S7_300 | PlcPlatform::S7_1200 | PlcPlatform::S7_1500 => false,
        };
        if !permitted {
            self.errors.push(PlantError::CommModeNotOnPlatform {
                code: self.machine,
                mode,
                platform: self.platform,
            });
        }
    }

    /// Record `E-PLT-026` when the address of an `Ads` request is not an AMS Net ID.
    fn check_ams_net_id(&mut self, address: &PlcAddress) {
        if AmsNetId::parse(address.as_str()).is_err() {
            self.errors.push(PlantError::AddressNotAmsNetId {
                code: self.machine,
                address: address.clone(),
            });
        }
    }

    /// Record `E-PLT-023` or `E-PLT-028` for the login and the password of the mode.
    ///
    /// `OpcUa` and `CodesysV3` need a login and a password. `Ads` needs both or neither.
    /// `ModbusTcp` refuses both. A stored password stands for the password of the request.
    fn check_credentials(
        &mut self,
        mode: CommMode,
        has_login: bool,
        has_new_password: bool,
        stored: PasswordState,
    ) {
        let has_password = has_new_password || stored == PasswordState::Stored;
        match mode {
            CommMode::ModbusTcp => self.refuse_in_mode(
                mode,
                &[("login", has_login), ("password", has_new_password)],
            ),
            CommMode::OpcUa | CommMode::CodesysV3 => {
                self.require_flag("login", has_login);
                self.require_flag("password", has_password);
            }
            // A password without a login is a fault on the login. A login without a password
            // is a fault on the password. The two cases cannot occur together.
            CommMode::Ads if has_login => self.require_flag("password", has_password),
            CommMode::Ads => self.require_flag("login", !has_new_password),
        }
    }

    /// Record `E-PLT-028` for each named field that holds a value.
    fn refuse_in_mode(&mut self, mode: CommMode, fields: &[(&'static str, bool)]) {
        let refused = fields
            .iter()
            .filter(|(_, has_value)| *has_value)
            .map(|(field, _)| PlantError::ConnectionFieldNotInMode {
                code: self.machine,
                field,
                mode,
            });
        self.errors.extend(refused);
    }

    /// Record `E-PLT-023` when a required field has no value.
    fn require_flag(&mut self, field: &'static str, is_present: bool) {
        if !is_present {
            self.errors.push(PlantError::ConnectionFieldMissing {
                code: self.machine,
                field,
            });
        }
    }

    /// Return the plan when no check found a fault. Return every fault otherwise.
    fn finish(self, built: Option<ConnectionPlan>) -> Result<ConnectionPlan, PlantErrors> {
        let code = self.machine;
        match (PlantErrors::from_vec(self.errors), built) {
            (None, Some(plan)) => Ok(plan),
            (Some(errors), _) => Err(errors),
            // A branch builds no plan only after it recorded a fault, so this arm is unreachable.
            // The arm gives a defined error in place of a panic (`reliability.md` section 2).
            (None, None) => Err(PlantError::ConnectionNotConfigured { code }.into()),
        }
    }
}

/// Check the fields of an S7 request. The S7 shape has no password.
fn s7_plan(faults: &mut Faults, fields: ConnectionFields) -> Option<ConnectionPlan> {
    faults.refuse_on_platform(&[
        ("port", fields.port.is_some()),
        ("address", fields.address.is_some()),
        ("comm_mode", fields.mode.is_some()),
        ("login", fields.login.is_some()),
        ("password", fields.password.is_some()),
    ]);
    let host = faults.require("host", fields.host);
    let rack = faults.require("rack", fields.rack);
    let slot = faults.require("slot", fields.slot);
    let (Some(host), Some(rack), Some(slot)) = (host, rack, slot) else {
        return None;
    };
    Some(ConnectionPlan {
        connection: PlcConnection::S7 { host, rack, slot },
        password: None,
    })
}

/// Derive the action on the stored password from the mode, the login, and the request.
///
/// A mode without credentials gives `Clear`. This covers `ModbusTcp` and `Ads` without a login.
/// A mode with credentials gives `Replace` for a new password and `Keep` for a stored one.
/// The case of a mode with credentials, no new password, and no stored password is a fault
/// that `Faults::check_credentials` records. Its plan is `Clear` and the caller never uses it.
fn password_plan(
    mode: CommMode,
    has_login: bool,
    new: Option<PlcPassword>,
    stored: PasswordState,
) -> PasswordPlan {
    let uses_password = match mode {
        CommMode::OpcUa | CommMode::CodesysV3 => true,
        CommMode::Ads => has_login,
        CommMode::ModbusTcp => false,
    };
    match (uses_password, new, stored) {
        (true, Some(new), _) => PasswordPlan::Replace(new),
        (true, None, PasswordState::Stored) => PasswordPlan::Keep,
        (true, None, PasswordState::Absent) | (false, _, _) => PasswordPlan::Clear,
    }
}

/// Check the fields of an IEC request.
///
/// The function checks the credentials of the mode, keeps the login, and derives a password
/// plan from the request and the stored state.
fn iec_plan(
    faults: &mut Faults,
    fields: ConnectionFields,
    stored: PasswordState,
) -> Option<ConnectionPlan> {
    faults.refuse_on_platform(&[
        ("rack", fields.rack.is_some()),
        ("slot", fields.slot.is_some()),
    ]);
    let host = faults.require("host", fields.host);
    let port = faults.require("port", fields.port);
    let address = faults.require("address", fields.address);
    let mode = faults.require("comm_mode", fields.mode);
    if let Some(mode) = mode {
        faults.check_mode(mode);
    }
    if let (Some(CommMode::Ads), Some(address)) = (mode, &address) {
        faults.check_ams_net_id(address);
    }
    if let Some(mode) = mode {
        faults.check_credentials(
            mode,
            fields.login.is_some(),
            fields.password.is_some(),
            stored,
        );
    }
    let (Some(host), Some(port), Some(address), Some(mode)) = (host, port, address, mode) else {
        return None;
    };
    let password = password_plan(mode, fields.login.is_some(), fields.password, stored);
    Some(ConnectionPlan {
        connection: PlcConnection::Iec {
            host,
            port,
            address,
            mode,
            login: fields.login,
        },
        password: Some(password),
    })
}

#[cfg(test)]
mod tests;
