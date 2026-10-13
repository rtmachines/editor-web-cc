//! The `INT` parameter fixture.

use recipe_domain::plant::{LeafPath, PlcPlatform, Primitive, S7Address, UnitId};
use recipe_domain::recipe::{Ordinal, ParameterTag, StoredLimit, StoredParameter, StoredValue};
use serde_json::json;

use super::{long_text, pid, pkey, text, translation_pl};
use crate::memory::seed_type_id;

/// The limit row of an `INT` parameter with the bounds 10 and 250.
fn int_limit() -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some("10".parse().unwrap()),
        max: Some("250".parse().unwrap()),
        max_length: None,
    }
}

/// An `INT` parameter with the value 120 on the unit `unit`.
///
/// The parameter has the S7 address `DB1.DBW<offset>` only on `S7_300` (A31). Other platforms
/// have no address. The identifier and the key come from `n`.
pub(crate) fn parameter_on(
    platform: PlcPlatform,
    n: u128,
    tag: &str,
    unit: UnitId,
    offset: u16,
) -> StoredParameter {
    let s7_address = (platform == PlcPlatform::S7_300)
        .then(|| S7Address::parse(&format!("DB1.DBW{offset}")).unwrap());
    StoredParameter {
        id: pid(n),
        key: pkey(n),
        unit,
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: text("Name"),
        description_en: long_text("Description"),
        translations: vec![translation_pl()],
        plc_type: seed_type_id(Primitive::Int, None).unwrap(),
        s7_address,
        limits: vec![int_limit()],
        value: StoredValue::from(json!(120)),
        ordinal: Ordinal::parse(0).unwrap(),
    }
}
