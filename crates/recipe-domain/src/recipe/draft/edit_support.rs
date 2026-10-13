//! Shared builders of the tests of the commands that change a parameter.

use rust_decimal::Decimal;
use serde_json::json;

use super::add::support::{INT, draft_with};
use super::support::stored_parameter;
use crate::plant::fixtures::id;
use crate::plant::{LeafPath, PlcPlatform};
use crate::recipe::{Draft, LangCode, StoredLimit, Translation};
use crate::text::Text;

/// A stored limit on the whole value from 0 to `max`.
pub(super) fn stored_limit_to(max: i64) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some(Decimal::from(0)),
        max: Some(Decimal::from(max)),
        max_length: None,
    }
}

/// A translation into `lang` with the name `name` and no description.
pub(super) fn translation(lang: &str, name: &str) -> Translation {
    Translation {
        lang: LangCode::parse(lang).unwrap(),
        name: Text::parse(name).unwrap(),
        description: None,
    }
}

/// A draft with `SPEED` (INT, value 120, limit 0 to 200, translation `pl`) and `TEMP`.
///
/// The key of `SPEED` derives from 1 and the key of `TEMP` from 2.
pub(super) fn draft_with_content() -> Draft {
    let mut speed = stored_parameter(1, "SPEED", id(INT), None);
    speed.value = json!(120).into();
    speed.limits = vec![stored_limit_to(200)];
    speed.translations = vec![translation("pl", "Predkosc")];
    let temp = stored_parameter(2, "TEMP", id(INT), None);
    draft_with(PlcPlatform::TwinCat3, vec![speed, temp])
}
