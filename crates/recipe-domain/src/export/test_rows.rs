//! Row builders of the tests of the hashed body: units, limits, and parameters.

use serde_json::Value;
use uuid::Uuid;

use super::test_support::{machine_id, type_id};
use crate::plant::{
    LeafPath, LiveUnits, Machine, MachineCode, PlcPlatform, S7Address, StoredMachine, StoredUnit,
    UnitId, UnitPath,
};
use crate::recipe::{
    Ordinal, Parameter, ParameterId, ParameterKey, ParameterTag, StoredLimit, StoredParameter,
};
use crate::text::{LongText, Text};

/// The unit identifier of the unit with the given path text.
pub(crate) fn unit_id(path: &str) -> UnitId {
    let n = UnitPath::parse(path)
        .unwrap()
        .segments()
        .fold(0_u128, |acc, segment| {
            acc.checked_mul(100)
                .and_then(|shifted| shifted.checked_add(u128::from(segment)))
                .unwrap()
        });
    UnitId::from(Uuid::from_u128(n.checked_add(1_000_000).unwrap()))
}

/// The machine `M12` on `platform` with the units `U1`, `U1.1`, `U1.2`, `U2`, and `U10`.
pub(crate) fn units(platform: PlcPlatform) -> LiveUnits {
    let unit = |path: &str| StoredUnit {
        id: unit_id(path),
        path: UnitPath::parse(path).unwrap(),
        name: Text::parse(path).unwrap(),
        archived_at: None,
    };
    Machine::try_from(StoredMachine {
        id: machine_id(),
        code: MachineCode::parse("M12").unwrap(),
        name: Text::parse("Filler 2").unwrap(),
        platform,
        archived_at: None,
        units: ["U1", "U1.1", "U1.2", "U2", "U10"].map(unit).to_vec(),
    })
    .unwrap()
    .live_units()
}

/// A limit row for a range.
pub(crate) fn range_limit(path: &str, min: Option<i64>, max: Option<i64>) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: min.map(rust_decimal::Decimal::from),
        max: max.map(rust_decimal::Decimal::from),
        max_length: None,
    }
}

/// A limit row for a string length.
pub(crate) fn length_limit(path: &str, max_length: u16) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: None,
        max: None,
        max_length: Some(max_length),
    }
}

/// The address `DB1.DBW0`.
pub(crate) fn word_address() -> S7Address {
    S7Address::parse("DB1.DBW0").unwrap()
}

/// The row of a parameter. The identifier and the key derive from `n`.
pub(crate) fn stored_parameter(
    n: u128,
    (unit, tag): (&str, &str),
    plc_type: &str,
    value: Value,
) -> StoredParameter {
    StoredParameter {
        id: ParameterId::from(Uuid::from_u128(n)),
        key: ParameterKey::from(Uuid::from_u128(n)),
        unit: unit_id(unit),
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: Text::parse("Name").unwrap(),
        description_en: LongText::parse("Description").unwrap(),
        translations: Vec::new(),
        plc_type: type_id(plc_type),
        s7_address: None,
        limits: Vec::new(),
        value: value.into(),
        ordinal: Ordinal::parse(u32::try_from(n).unwrap()).unwrap(),
    }
}

/// A parameter without an address and without limits.
pub(crate) fn parameter(
    n: u128,
    (unit, tag): (&str, &str),
    plc_type: &str,
    value: Value,
) -> Parameter {
    Parameter::from(stored_parameter(n, (unit, tag), plc_type, value))
}
