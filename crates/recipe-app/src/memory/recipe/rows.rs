//! Rows of the in-memory recipe tables (`data-model.md` sections 3.9 to 3.14).

use std::collections::BTreeMap;

use recipe_domain::export::Sha256;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{MachineId, PlcPlatform, PlcTypeId, S7Address, UnitId};
use recipe_domain::recipe::{
    LangCode, Ordinal, ParameterId, ParameterKey, ParameterTag, RecipeId, RecipeVersionId,
    ReferenceNumber, StoredLimit, StoredParameter, StoredValue, Translation, VersionState,
    VersionStateReason, VersionStatus,
};
use recipe_domain::text::{LongText, Text};
use recipe_domain::timestamp::Timestamp;

/// A row of the table `recipe`.
#[derive(Clone, Debug)]
pub(crate) struct RecipeRow {
    pub(crate) id: RecipeId,
    pub(crate) machine: MachineId,
    pub(crate) platform: PlcPlatform,
    pub(crate) reference: ReferenceNumber,
    pub(crate) name: Text,
    pub(crate) deleted_at: Option<Timestamp>,
    pub(crate) created_by: UserId,
    pub(crate) created_at: Timestamp,
    pub(crate) updated_at: Timestamp,
}

/// A row of the table `recipe_version`.
#[derive(Clone, Debug)]
pub(crate) struct VersionRow {
    pub(crate) id: RecipeVersionId,
    pub(crate) recipe: RecipeId,
    pub(crate) platform: PlcPlatform,
    pub(crate) number: u32,
    pub(crate) status: VersionStatus,
    pub(crate) row_version: i64,
    pub(crate) based_on: Option<RecipeVersionId>,
    pub(crate) change_note: LongText,
    pub(crate) content_sha256: Option<Sha256>,
    pub(crate) created_by: UserId,
    pub(crate) created_at: Timestamp,
    pub(crate) updated_at: Timestamp,
    pub(crate) approved_by: Option<UserId>,
    pub(crate) approved_at: Option<Timestamp>,
    pub(crate) retired_at: Option<Timestamp>,
    pub(crate) deleted_at: Option<Timestamp>,
}

impl VersionRow {
    /// Return the state of the version from its status and deletion mark.
    pub(crate) const fn state(&self) -> Result<VersionState, VersionStateReason> {
        VersionState::from_stored(self.status, self.deleted_at.is_some())
    }
}

/// A row of the table `parameter_limit`.
#[derive(Clone, Debug)]
pub(crate) struct LimitRow {
    pub(crate) limit: StoredLimit,
    pub(crate) created_at: Timestamp,
}

/// A row of the table `parameter_translation`.
#[derive(Clone, Debug)]
pub(crate) struct TranslationRow {
    pub(crate) translation: Translation,
    pub(crate) created_at: Timestamp,
}

/// A row of the table `parameter` with its limit rows and translation rows.
///
/// The child rows live inside the parameter row, so a delete cascades without extra code.
#[derive(Clone, Debug)]
pub(crate) struct ParameterRow {
    pub(crate) id: ParameterId,
    pub(crate) version: RecipeVersionId,
    pub(crate) platform: PlcPlatform,
    pub(crate) key: ParameterKey,
    pub(crate) unit: UnitId,
    pub(crate) tag: ParameterTag,
    pub(crate) name_en: Text,
    pub(crate) description_en: LongText,
    pub(crate) plc_type: PlcTypeId,
    pub(crate) s7_address: Option<S7Address>,
    pub(crate) value: StoredValue,
    pub(crate) ordinal: Ordinal,
    pub(crate) created_at: Timestamp,
    /// The limit rows by the text of their `leaf_path`.
    pub(crate) limits: BTreeMap<String, LimitRow>,
    pub(crate) translations: BTreeMap<LangCode, TranslationRow>,
}

impl ParameterRow {
    /// Return the parameter in the form that the domain loads.
    pub(crate) fn stored(&self) -> StoredParameter {
        StoredParameter {
            id: self.id,
            key: self.key,
            unit: self.unit,
            tag: self.tag.clone(),
            name_en: self.name_en.clone(),
            description_en: self.description_en.clone(),
            translations: self
                .translations
                .values()
                .map(|row| row.translation.clone())
                .collect(),
            plc_type: self.plc_type,
            s7_address: self.s7_address,
            limits: self.limits.values().map(|row| row.limit.clone()).collect(),
            value: self.value.clone(),
            ordinal: self.ordinal,
        }
    }
}
