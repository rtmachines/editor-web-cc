//! Shared builders of the tests of the draft.

use uuid::Uuid;

use crate::identity::UserId;
use crate::plant::fixtures::id;
use crate::plant::{MachineId, PlcPlatform, PlcTypeId, S7Address, UnitId};
use crate::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, RecipeVersionId, ReferenceNumber,
    StoredDraft, StoredParameter, VersionState,
};
use crate::text::{LongText, Text};

pub(super) fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-1").unwrap()
}

pub(super) fn key(n: u128) -> ParameterKey {
    ParameterKey::from(Uuid::from_u128(n))
}

pub(in crate::recipe) fn tag(text: &str) -> ParameterTag {
    ParameterTag::parse(text).unwrap()
}

pub(in crate::recipe) fn user(n: u128) -> UserId {
    UserId::from(Uuid::from_u128(n))
}

pub(super) fn address(text: &str) -> S7Address {
    S7Address::parse(text).unwrap()
}

/// A stored parameter. The identifier and the ordinal derive from `n`.
pub(in crate::recipe) fn stored_parameter(
    n: u128,
    tag_text: &str,
    plc_type: PlcTypeId,
    s7_address: Option<S7Address>,
) -> StoredParameter {
    StoredParameter {
        id: ParameterId::from(Uuid::from_u128(n)),
        key: key(n),
        unit: UnitId::from(Uuid::from_u128(700)),
        tag: tag(tag_text),
        name_en: Text::parse("Name").unwrap(),
        description_en: LongText::parse("Description").unwrap(),
        translations: Vec::new(),
        plc_type,
        s7_address,
        limits: Vec::new(),
        value: serde_json::json!(0).into(),
        ordinal: Ordinal::parse(u32::try_from(n).unwrap()).unwrap(),
    }
}

/// A parameter of the type `n_type` with the tag `tag_text`. The key derives from `n`.
pub(super) fn parameter(n: u128, tag_text: &str, type_n: u128) -> StoredParameter {
    stored_parameter(n, tag_text, id(type_n), None)
}

/// A stored draft of version 3 with the given state and parameters.
pub(in crate::recipe) fn stored_draft_with(
    state: VersionState,
    platform: PlcPlatform,
    parameters: Vec<StoredParameter>,
) -> StoredDraft {
    StoredDraft {
        reference: reference(),
        machine: MachineId::from(Uuid::from_u128(0xAA)),
        platform,
        version: RecipeVersionId::from(Uuid::from_u128(30)),
        number: 3,
        state,
        row_version: 7,
        created_by: user(1),
        editors: vec![user(2), user(3)],
        parameters,
    }
}

/// A live draft on `TwinCat3` with the given parameters.
pub(super) fn stored_draft(parameters: Vec<StoredParameter>) -> StoredDraft {
    stored_draft_with(VersionState::LiveDraft, PlcPlatform::TwinCat3, parameters)
}
