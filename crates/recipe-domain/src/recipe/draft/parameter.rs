//! The parameter of a draft (`domain-model.md` section 2.2).

use std::collections::BTreeMap;

use super::{DefinitionUpdate, NewParameter};
use crate::plant::{PlcTypeId, S7Address, UnitId};
use crate::recipe::{
    LangCode, Ordinal, ParameterId, ParameterKey, ParameterTag, StoredLimit, StoredValue,
    Translation,
};
use crate::text::{LongText, Text};

/// The row of a parameter as storage returns it (`data-model.md` sections 3.12 to 3.14).
///
/// The row is not checked. The value and the limits stay in their stored form, so a command
/// parses only the parameter that it changes (design D6).
#[derive(Clone, Debug)]
pub struct StoredParameter {
    /// The stable identity of the parameter.
    pub id: ParameterId,
    /// The key of the parameter in a URL.
    pub key: ParameterKey,
    /// The unit that holds the parameter.
    pub unit: UnitId,
    /// The tag of the parameter.
    pub tag: ParameterTag,
    /// The English name.
    pub name_en: Text,
    /// The English description.
    pub description_en: LongText,
    /// The translations. Storage keeps one row for each language.
    pub translations: Vec<Translation>,
    /// The PLC type of the parameter.
    pub plc_type: PlcTypeId,
    /// The S7 address, or `None` when the parameter has none.
    pub s7_address: Option<S7Address>,
    /// The process limits in their stored form.
    pub limits: Vec<StoredLimit>,
    /// The value in its stored form.
    pub value: StoredValue,
    /// The position of the parameter in its draft.
    pub ordinal: Ordinal,
}

/// A parameter of a draft.
///
/// The type solves one problem: only the draft changes a parameter, so the fields are private.
/// The value and the limits keep their stored form until a command parses them.
#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    id: ParameterId,
    key: ParameterKey,
    unit: UnitId,
    tag: ParameterTag,
    name_en: Text,
    description_en: LongText,
    translations: BTreeMap<LangCode, Translation>,
    plc_type: PlcTypeId,
    s7_address: Option<S7Address>,
    limits: Vec<StoredLimit>,
    value: StoredValue,
    ordinal: Ordinal,
}

impl From<StoredParameter> for Parameter {
    /// Build a parameter from its row. A repeated language keeps the last row.
    fn from(stored: StoredParameter) -> Self {
        let translations = stored
            .translations
            .into_iter()
            .map(|translation| (translation.lang.clone(), translation))
            .collect();
        Self {
            id: stored.id,
            key: stored.key,
            unit: stored.unit,
            tag: stored.tag,
            name_en: stored.name_en,
            description_en: stored.description_en,
            translations,
            plc_type: stored.plc_type,
            s7_address: stored.s7_address,
            limits: stored.limits,
            value: stored.value,
            ordinal: stored.ordinal,
        }
    }
}

impl Parameter {
    /// Build the parameter of a request whose value and limits passed their checks.
    ///
    /// The new parameter has no translations.
    pub(super) fn from_new(
        new: NewParameter,
        value: StoredValue,
        limits: Vec<StoredLimit>,
    ) -> Self {
        Self {
            id: new.id,
            key: new.key,
            unit: new.unit.id(),
            tag: new.tag,
            name_en: new.name_en,
            description_en: new.description_en,
            translations: BTreeMap::new(),
            plc_type: new.plc_type,
            s7_address: new.s7_address,
            limits,
            value,
            ordinal: new.ordinal,
        }
    }

    /// Return the parameter with a new definition (design D6).
    ///
    /// The identifier, the key, and the translations stay. `content` is the value and the limits
    /// of a retype. `None` keeps the stored value and limits.
    pub(super) fn redefined(
        &self,
        update: DefinitionUpdate,
        content: Option<(StoredValue, Vec<StoredLimit>)>,
    ) -> Self {
        let (value, limits) = content.unwrap_or_else(|| (self.value.clone(), self.limits.clone()));
        Self {
            unit: update.unit.id(),
            tag: update.tag,
            name_en: update.name_en,
            description_en: update.description_en,
            plc_type: update.plc_type,
            s7_address: update.s7_address,
            ordinal: update.ordinal,
            value,
            limits,
            ..self.clone()
        }
    }

    /// Return the parameter after a rebind (design D13).
    ///
    /// The type, the value, and the limits change. Everything else stays, also the translations.
    pub(in crate::recipe) fn rebound(
        &self,
        plc_type: PlcTypeId,
        value: StoredValue,
        limits: Vec<StoredLimit>,
    ) -> Self {
        Self {
            plc_type,
            value,
            limits,
            ..self.clone()
        }
    }

    /// Return the parameter with a new value. The limits and the translations stay.
    pub(super) fn with_value(&self, value: StoredValue) -> Self {
        Self {
            value,
            ..self.clone()
        }
    }

    /// Return the parameter with new limits. The value and the translations stay.
    pub(super) fn with_limits(&self, limits: Vec<StoredLimit>) -> Self {
        Self {
            limits,
            ..self.clone()
        }
    }

    /// Return the parameter with the translation. It replaces a translation of the same language.
    pub(super) fn with_translation(&self, translation: Translation) -> Self {
        let mut translations = self.translations.clone();
        translations.insert(translation.lang.clone(), translation);
        Self {
            translations,
            ..self.clone()
        }
    }

    /// Return the parameter without the translation of `lang`, or `None` when it has none.
    pub(super) fn without_translation(&self, lang: &LangCode) -> Option<Self> {
        let mut translations = self.translations.clone();
        translations.remove(lang)?;
        Some(Self {
            translations,
            ..self.clone()
        })
    }

    /// Return the identifier of the parameter.
    #[must_use]
    pub const fn id(&self) -> ParameterId {
        self.id
    }

    /// Return the key of the parameter.
    #[must_use]
    pub const fn key(&self) -> ParameterKey {
        self.key
    }

    /// Return the unit that holds the parameter.
    #[must_use]
    pub const fn unit(&self) -> UnitId {
        self.unit
    }

    /// Return the tag of the parameter.
    #[must_use]
    pub const fn tag(&self) -> &ParameterTag {
        &self.tag
    }

    /// Return the English name.
    #[must_use]
    pub const fn name_en(&self) -> &Text {
        &self.name_en
    }

    /// Return the English description.
    #[must_use]
    pub const fn description_en(&self) -> &LongText {
        &self.description_en
    }

    /// Return the translations, one for each language.
    #[must_use]
    pub const fn translations(&self) -> &BTreeMap<LangCode, Translation> {
        &self.translations
    }

    /// Return the PLC type of the parameter.
    #[must_use]
    pub const fn plc_type(&self) -> PlcTypeId {
        self.plc_type
    }

    /// Return the S7 address, or `None` when the parameter has none.
    #[must_use]
    pub const fn s7_address(&self) -> Option<S7Address> {
        self.s7_address
    }

    /// Return the process limits in their stored form.
    #[must_use]
    pub fn limits(&self) -> &[StoredLimit] {
        &self.limits
    }

    /// Return the value in its stored form.
    #[must_use]
    pub const fn value(&self) -> &StoredValue {
        &self.value
    }

    /// Return the position of the parameter in its draft.
    #[must_use]
    pub const fn ordinal(&self) -> Ordinal {
        self.ordinal
    }
}
