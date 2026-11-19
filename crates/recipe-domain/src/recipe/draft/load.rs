//! Loading of a draft and lookup of its parameters (design D6).

use std::collections::{BTreeMap, BTreeSet};

use super::{Draft, Parameter, StoredDraft};
use crate::recipe::read_check::StoredDefect;
use crate::recipe::{DraftError, ParameterKey, RecipeError, Transition};

impl Draft {
    /// Build a draft from its stored form.
    ///
    /// The method trusts the stored values and limits. It does not parse them and needs no type
    /// tree, so a parameter whose type no longer resolves stays in the draft (design D6).
    ///
    /// # Errors
    ///
    /// Returns `DraftError::Recipe` with `E-RCP-002` when the version is not a live draft.
    /// Returns `DraftError::Stored` with `E-RCP-014` and the key of the second parameter when two
    /// parameters share a tag, a key, or an identifier (ADR-0195).
    pub fn load(stored: StoredDraft) -> Result<Self, DraftError> {
        if stored.state.apply(Transition::Edit).is_err() {
            return Err(RecipeError::VersionNotEditable {
                reference: stored.reference,
                number: stored.number,
                status: stored.state.status(),
            }
            .into());
        }
        let mut tags = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut parameters = BTreeMap::new();
        for row in stored.parameters {
            let parameter = Parameter::from(row);
            let is_new_tag = tags.insert(parameter.tag().clone());
            let is_new_id = ids.insert(parameter.id());
            if !is_new_tag || !is_new_id || parameters.contains_key(&parameter.key()) {
                let duplicate = RecipeError::DuplicateTag {
                    tag: parameter.tag().clone(),
                    number: stored.number,
                };
                let defect = StoredDefect::of_parameter(parameter.key(), duplicate);
                return Err(DraftError::Stored(Box::new(defect)));
            }
            parameters.insert(parameter.key(), parameter);
        }
        Ok(Self {
            reference: stored.reference,
            machine: stored.machine,
            platform: stored.platform,
            version: stored.version,
            number: stored.number,
            row_version: stored.row_version,
            created_by: stored.created_by,
            editors: stored.editors.into_iter().collect(),
            parameters,
        })
    }

    /// Return the parameter with `key`.
    ///
    /// # Errors
    ///
    /// Returns `E-RCP-051` when the draft has no parameter with this key.
    pub fn parameter(&self, key: ParameterKey) -> Result<&Parameter, RecipeError> {
        self.parameters
            .get(&key)
            .ok_or_else(|| RecipeError::ParameterNotFound {
                reference: self.reference.clone(),
                number: self.number,
                key,
            })
    }
}

#[cfg(test)]
mod tests;
