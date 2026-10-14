//! Fixtures of the contract cases of `DraftRepository`.
//!
//! A draft comes from the harness. The machine of the fixtures is a `TwinCat3` machine, so no
//! parameter has an S7 address.

use recipe_app::error::StorageError;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::recipe::{
    Parameter, ParameterKey, RecipeVersionId, StoredDraft, StoredParameter,
};
use serde_json::Value;
use uuid::Uuid;

use crate::contract::harness::{Harness, begin, commit};
use crate::contract::images::Kind;
use crate::contract::recipe_seed::{
    Plant, create_recipe, length, parameter, plant, range, translation,
};

/// A recipe with one live draft and the plant of its machine.
pub struct Draft {
    /// The machine, its unit, and its type `DINT`.
    pub plant: Plant,
    /// The version that is a live draft.
    pub version: RecipeVersionId,
}

/// Seed the plant `n` and a recipe `n` with a live draft.
pub async fn live_draft(h: &dyn Harness, n: u128) -> Draft {
    let plant = plant(h, n, &format!("M{n}")).await;
    let created = create_recipe(h, &plant, n, &format!("REF-{n}")).await;
    Draft {
        plant,
        version: created.versions()[0].id(),
    }
}

/// A parameter with two limit rows and two translations, in the order that a read gives.
pub fn full_parameter(plant: &Plant, n: u128, tag: &str) -> StoredParameter {
    let mut full = parameter(plant, n, tag);
    full.limits = vec![range("", "0", "10.25"), length(".B", 12)];
    full.translations = vec![
        translation("de", "Name", Some("Beschreibung")),
        translation("pl", "Nazwa", None),
    ];
    full
}

/// The domain parameter of a stored parameter, as a command hands it to the port.
pub fn domain(stored: &StoredParameter) -> Parameter {
    Parameter::from(stored.clone())
}

/// A parameter key that no fixture parameter has.
pub fn missing_key() -> ParameterKey {
    ParameterKey::from(Uuid::from_u128(0xDEAD))
}

/// Read the draft without a lock in a transaction that nothing commits.
pub async fn read_back(h: &dyn Harness, version: RecipeVersionId) -> StoredDraft {
    let mut tx = begin(h).await;
    tx.drafts().read_draft(version).await.unwrap().unwrap()
}

/// Commit a transaction on the machine of the draft.
pub async fn commit_draft(
    h: &dyn Harness,
    draft: &Draft,
    tx: Box<dyn recipe_app::ports::Transaction>,
) {
    commit(h, tx, draft.plant.machine).await;
}

/// Assert that an error is the trigger error `E-RCP-031`.
pub fn assert_not_live_draft(error: &StorageError) {
    assert!(
        matches!(error, StorageError::Constraint { code, .. } if *code == ErrorCode::RCP_031),
        "{error:?}"
    );
}

/// Assert that an error is the failure of a write that finds no row.
pub fn assert_missing_row(error: &StorageError) {
    assert!(matches!(error, StorageError::Other { .. }), "{error:?}");
}

/// Parse a JSON number from its text, so a case compares the digits that storage keeps.
pub fn number(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

/// The keys of a `parameter` row image with the JSON type of each one (`data-model.md` 3.12).
pub fn parameter_shape(value_kind: Kind) -> Vec<(&'static str, Kind)> {
    vec![
        ("id", Kind::String),
        ("recipe_version_id", Kind::String),
        ("plc_platform", Kind::String),
        ("parameter_key", Kind::String),
        ("unit_id", Kind::String),
        ("tag", Kind::String),
        ("name_en", Kind::String),
        ("description_en", Kind::String),
        ("plc_type_id", Kind::String),
        ("s7_address", Kind::Null),
        ("value", value_kind),
        ("ordinal", Kind::Number),
        ("created_at", Kind::String),
        ("limits", Kind::Array),
        ("translations", Kind::Array),
    ]
}

/// The sorted key names of a JSON object.
pub fn keys(object: &Value) -> Vec<String> {
    let mut names: Vec<String> = object.as_object().unwrap().keys().cloned().collect();
    names.sort();
    names
}
