//! The scope hashed body of an export document v2 (ADR-0109 (s) to (w), `api.md` section 6.1).

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::body::{HashedBody, MachineBody, RecipeBody, VersionBody};
use super::hash::{canonical_text_of, sha256_of};
use super::parameters::ParameterEntry;
use super::type_reference::TypeReference;
use super::types::TypeEntry;
use super::{ExportError, Sha256};
use crate::plant::{MachineCode, PlcPlatform, PlcTypeId, UnitPath};
use crate::recipe::ReferenceNumber;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct ScopeMarker {
    #[serde(serialize_with = "super::display_text")]
    unit_path: UnitPath,
}

/// The JSON object that `scope_sha256` covers.
///
/// The type solves one problem: a client that loads one unit needs a hash of exactly the
/// parameters that it gets, and the hash of the whole version (`content_sha256`) cannot cover a
/// subset. The body holds the identifying values of the version, the unit of the scope, the
/// parameters of that unit and its descendants, and the types that those parameters use. It holds
/// no name and no ordinal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScopeBody {
    machine: MachineBody,
    recipe: RecipeBody,
    version: VersionBody,
    scope: ScopeMarker,
    types: Vec<TypeEntry>,
    parameters: Vec<ParameterEntry>,
}

/// Collect the entries of the types that the given parameters use.
///
/// The walk follows the element type of an array and the member types of a struct, so a type
/// enters the subset when a parameter uses it directly or through another type. The walk uses an
/// explicit stack and never recurses (`reliability.md` section 2.2). The entries keep the order of
/// `types`, which is the order by `id`. A reference that `types` does not hold adds nothing,
/// because the whole body already holds the same reference.
fn type_closure(types: &[TypeEntry], parameters: &[ParameterEntry]) -> Vec<TypeEntry> {
    let by_id: BTreeMap<PlcTypeId, &TypeEntry> =
        types.iter().map(|entry| (entry.id(), entry)).collect();
    let mut wanted: BTreeSet<PlcTypeId> = BTreeSet::new();
    let mut stack: Vec<TypeReference> = parameters.iter().map(ParameterEntry::type_id).collect();
    while let Some(reference) = stack.pop() {
        let TypeReference::Row(id) = reference else {
            continue;
        };
        if !wanted.insert(id) {
            continue;
        }
        if let Some(entry) = by_id.get(&id) {
            stack.extend(entry.references());
        }
    }
    types
        .iter()
        .filter(|entry| wanted.contains(&entry.id()))
        .cloned()
        .collect()
}

impl ScopeBody {
    /// Build the scope body of the unit `scope` from the hashed body of the whole version.
    ///
    /// The body keeps each parameter whose unit is `scope` or below it. The test compares unit
    /// path segments, not text, so `U10` is not below `U1`. The arrays keep the orders of
    /// `api.md` section 6.3 because the filter keeps the order of the whole body. The result can
    /// be empty. The caller refuses an empty scope with `E-EXP-005` (ADR-0109 (x)).
    #[must_use]
    pub fn build(whole: &HashedBody, scope: &UnitPath) -> Self {
        let parameters: Vec<ParameterEntry> = whole
            .parameters()
            .iter()
            .filter(|entry| entry.unit_path().is_self_or_descendant_of(scope))
            .cloned()
            .collect();
        let types = type_closure(whole.types(), &parameters);
        Self {
            machine: whole.machine.clone(),
            recipe: whole.recipe.clone(),
            version: whole.version.clone(),
            scope: ScopeMarker {
                unit_path: scope.clone(),
            },
            types,
            parameters,
        }
    }

    /// Return the code of the machine, the value of `machine.code`.
    #[must_use]
    pub const fn machine_code(&self) -> MachineCode {
        self.machine.code
    }

    /// Return the PLC platform of the machine, the value of `machine.plc_platform`.
    #[must_use]
    pub const fn platform(&self) -> PlcPlatform {
        self.machine.plc_platform
    }

    /// Return the reference number of the recipe, the value of `recipe.reference_number`.
    #[must_use]
    pub const fn reference_number(&self) -> &ReferenceNumber {
        &self.recipe.reference_number
    }

    /// Return the number of the version, the value of `version.number`.
    #[must_use]
    pub const fn version_number(&self) -> u32 {
        self.version.number
    }

    /// Return the unit of the scope, the value of `scope.unit_path`.
    #[must_use]
    pub const fn unit_path(&self) -> &UnitPath {
        &self.scope.unit_path
    }

    /// Return the entries of `types[]`, ordered by `id`.
    #[must_use]
    pub fn types(&self) -> &[TypeEntry] {
        &self.types
    }

    /// Return the entries of `parameters[]`, ordered by `unit_path` and then by `tag`.
    #[must_use]
    pub fn parameters(&self) -> &[ParameterEntry] {
        &self.parameters
    }

    /// Return `true` when no parameter of the version is in the scope.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
    }

    /// Return the RFC 8785 (JCS) canonical form of the body.
    ///
    /// # Errors
    ///
    /// Returns `E-EXP-004` when the serializer fails.
    pub fn canonical_text(&self) -> Result<String, ExportError> {
        canonical_text_of(self)
    }

    /// Return the SHA-256 hash of the canonical form, the value of `scope_sha256`.
    ///
    /// # Errors
    ///
    /// Returns `E-EXP-004` when the serializer fails.
    pub fn sha256(&self) -> Result<Sha256, ExportError> {
        sha256_of(self)
    }
}

#[cfg(test)]
mod tests;
