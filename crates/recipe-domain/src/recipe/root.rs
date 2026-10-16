//! The recipe root of `domain-model.md` section 2.2 (design D4).

use crate::plant::MachineId;
use crate::text::Text;
use crate::timestamp::Timestamp;

use super::{
    Budget, Optimistic, RecipeError, RecipeId, RecipeVersionId, ReferenceNumber, Transition,
    VersionState,
};

/// Largest version number. A number is stored as SQL `INTEGER` (`data-model.md` section 1.2).
const VERSION_NUMBER_MAX: u32 = i32::MAX.cast_unsigned();

/// The header of one version of a recipe, without its parameters.
///
/// Only [`Recipe`] changes a header. This keeps the version states valid (I1, A7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionHeader {
    id: RecipeVersionId,
    number: u32,
    state: VersionState,
    row_version: i64,
}

impl VersionHeader {
    /// Return the identifier of the version.
    #[must_use]
    pub const fn id(&self) -> RecipeVersionId {
        self.id
    }

    /// Return the number of the version in its recipe. The first version has the number 1.
    #[must_use]
    pub const fn number(&self) -> u32 {
        self.number
    }

    /// Return the state of the version.
    #[must_use]
    pub const fn state(&self) -> VersionState {
        self.state
    }

    /// Return the row version of the version (ADR-0008).
    #[must_use]
    pub const fn row_version(&self) -> i64 {
        self.row_version
    }
}

/// The role that deletes a recipe (ADR-0021).
///
/// The type solves one problem: the delete rule depends on the role, but the domain knows no `Role`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeDeleter {
    /// An administrator can delete a recipe with a retired version.
    Administrator,
    /// A process engineer cannot delete a recipe with a retired version.
    ProcessEngineer,
}

/// A recipe with its version headers.
///
/// The root keeps the version states valid: at most one approved version (I1) and at most one live draft (A7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recipe {
    id: RecipeId,
    machine: MachineId,
    reference: ReferenceNumber,
    name: Text,
    deleted_at: Option<Timestamp>,
    versions: Vec<VersionHeader>,
}

impl Recipe {
    /// Create a recipe with its first version, a live draft with the number 1.
    #[must_use]
    pub fn create(
        id: RecipeId,
        machine: MachineId,
        reference: ReferenceNumber,
        name: Text,
        first_version: RecipeVersionId,
    ) -> Self {
        Self {
            id,
            machine,
            reference,
            name,
            deleted_at: None,
            versions: vec![VersionHeader {
                id: first_version,
                number: 1,
                state: VersionState::LiveDraft,
                row_version: 1,
            }],
        }
    }

    /// Return the identifier of the recipe.
    #[must_use]
    pub const fn id(&self) -> RecipeId {
        self.id
    }

    /// Return the machine of the recipe.
    #[must_use]
    pub const fn machine(&self) -> MachineId {
        self.machine
    }

    /// Return the reference number of the recipe.
    #[must_use]
    pub const fn reference(&self) -> &ReferenceNumber {
        &self.reference
    }

    /// Return the name of the recipe.
    #[must_use]
    pub const fn name(&self) -> &Text {
        &self.name
    }

    /// Return the instant of deletion, or `None` for a recipe that is not deleted.
    #[must_use]
    pub const fn deleted_at(&self) -> Option<Timestamp> {
        self.deleted_at
    }

    /// Return true when the recipe is deleted.
    #[must_use]
    pub const fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Return the version headers in ascending order of their numbers.
    #[must_use]
    pub fn versions(&self) -> &[VersionHeader] {
        &self.versions
    }

    /// Return the header with this number, or `None` when the recipe has no such version.
    #[must_use]
    pub fn version(&self, number: u32) -> Option<&VersionHeader> {
        self.versions.iter().find(|v| v.number == number)
    }

    /// Replace the name of the recipe.
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError::RecipeDeleted`] (`E-RCP-012`) when the recipe is deleted.
    pub fn rename(&mut self, name: Text) -> Result<(), RecipeError> {
        self.require_not_deleted()?;
        self.name = name;
        Ok(())
    }

    /// Delete the recipe and discard its live draft (ADR-0021).
    ///
    /// Return the identifier of the discarded draft, or `None` when the recipe had no live draft.
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError::RecipeDeleted`] (`E-RCP-012`) when the recipe is deleted.
    /// Returns [`RecipeError::RecipeHasApprovedVersion`] (`E-RCP-010`) for an approved version.
    /// Returns [`RecipeError::RecipeHasRetiredVersion`] (`E-RCP-026`) when a process engineer
    /// deletes a recipe with a retired version.
    pub fn delete(
        &mut self,
        deleter: RecipeDeleter,
        now: Timestamp,
    ) -> Result<Option<RecipeVersionId>, RecipeError> {
        self.require_not_deleted()?;
        if self.has_state(VersionState::Approved) {
            return Err(RecipeError::RecipeHasApprovedVersion {
                reference: self.reference.clone(),
            });
        }
        if deleter == RecipeDeleter::ProcessEngineer && self.has_state(VersionState::Retired) {
            return Err(RecipeError::RecipeHasRetiredVersion {
                reference: self.reference.clone(),
            });
        }
        let discarded = self.versions.iter_mut().find_map(|header| {
            let next = header.state.apply(Transition::Discard).ok()?;
            header.state = next;
            Some(header.id)
        });
        self.deleted_at = Some(now);
        Ok(discarded)
    }

    /// Clear the deletion mark of the recipe. A discarded draft stays discarded.
    ///
    /// The use case checks that the machine is live (`E-PLT-010`).
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError::RecipeNotDeleted`] (`E-RCP-025`) when the recipe is not deleted.
    pub fn restore(&mut self) -> Result<(), RecipeError> {
        if !self.is_deleted() {
            return Err(RecipeError::RecipeNotDeleted {
                reference: self.reference.clone(),
            });
        }
        self.deleted_at = None;
        Ok(())
    }

    /// Add a live draft with the next version number and return its header.
    ///
    /// The number is the largest number of all headers plus one. A discarded draft counts.
    /// The SQL statements copy the parameters of the base version (`data-model.md` section 6).
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError::RecipeDeleted`] (`E-RCP-012`) when the recipe is deleted.
    /// Returns [`RecipeError::DraftAlreadyExists`] (`E-RCP-011`) when a live draft exists.
    /// Returns [`RecipeError::SizeBudgetExceeded`] (`E-RCP-021`) when the next number
    /// does not fit the SQL type `INTEGER`.
    pub fn create_version(&mut self, id: RecipeVersionId) -> Result<&VersionHeader, RecipeError> {
        self.require_not_deleted()?;
        if let Some(draft) = self
            .versions
            .iter()
            .find(|v| v.state == VersionState::LiveDraft)
        {
            return Err(RecipeError::DraftAlreadyExists {
                reference: self.reference.clone(),
                number: draft.number,
            });
        }
        let highest = self.versions.iter().map(|v| v.number).max().unwrap_or(0);
        let number = highest
            .checked_add(1)
            .filter(|next| *next <= VERSION_NUMBER_MAX)
            .ok_or(RecipeError::SizeBudgetExceeded {
                budget: Budget::VersionNumbers,
                limit: u64::from(VERSION_NUMBER_MAX),
                actual: u64::from(highest),
            })?;
        Ok(self.versions.push_mut(VersionHeader {
            id,
            number,
            state: VersionState::LiveDraft,
            row_version: 1,
        }))
    }

    /// Retire an approved version (ADR-0028). The SQL statement writes the time.
    ///
    /// The row version is compared before the state (`api.md` section 4).
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError::RecipeDeleted`] (`E-RCP-012`) when the recipe is deleted.
    /// Returns [`RecipeError::VersionNotApproved`] (`E-RCP-009`) for an unknown number
    /// and for a version that is not approved.
    pub fn retire(
        &mut self,
        number: u32,
        expected_row_version: i64,
    ) -> Result<Optimistic<RecipeVersionId>, RecipeError> {
        self.apply_step(number, expected_row_version, Transition::Retire)
    }

    /// Discard a live draft (ADR-0028). The SQL statement writes the time.
    ///
    /// The row version is compared before the state (`api.md` section 4).
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError::RecipeDeleted`] (`E-RCP-012`) when the recipe is deleted.
    /// Returns [`RecipeError::VersionNotDraft`] (`E-RCP-007`) for an unknown number
    /// and for a version that is not a live draft.
    pub fn discard(
        &mut self,
        number: u32,
        expected_row_version: i64,
    ) -> Result<Optimistic<RecipeVersionId>, RecipeError> {
        self.apply_step(number, expected_row_version, Transition::Discard)
    }

    /// Apply a step to one version in the order of design D4.
    fn apply_step(
        &mut self,
        number: u32,
        expected: i64,
        step: Transition,
    ) -> Result<Optimistic<RecipeVersionId>, RecipeError> {
        self.require_not_deleted()?;
        let reference = &self.reference;
        let forbidden = || forbidden_step(reference, number, step);
        let header = self
            .versions
            .iter_mut()
            .find(|v| v.number == number)
            .ok_or_else(forbidden)?;
        if header.row_version != expected {
            return Ok(Optimistic::Conflict {
                expected,
                actual: header.row_version,
            });
        }
        header.state = header.state.apply(step).map_err(|_| forbidden())?;
        Ok(Optimistic::Applied(header.id))
    }

    /// Return true when a header has this state.
    fn has_state(&self, state: VersionState) -> bool {
        self.versions.iter().any(|v| v.state == state)
    }

    /// Return `E-RCP-012` when the recipe is deleted.
    fn require_not_deleted(&self) -> Result<(), RecipeError> {
        if self.is_deleted() {
            return Err(RecipeError::RecipeDeleted {
                reference: self.reference.clone(),
            });
        }
        Ok(())
    }
}

/// Map a forbidden step to the error of its command (design D3).
fn forbidden_step(reference: &ReferenceNumber, number: u32, step: Transition) -> RecipeError {
    let reference = reference.clone();
    match step {
        Transition::Retire => RecipeError::VersionNotApproved { reference, number },
        Transition::Approve | Transition::Discard | Transition::Edit => {
            RecipeError::VersionNotDraft { reference, number }
        }
    }
}

mod approve;
mod stored;
pub use approve::{ApprovalOutcome, ApprovalPolicy, Approver, breaks_separation_of_duties};
pub use stored::{StoredRecipe, StoredRecipeReason, StoredVersionHeader};
#[cfg(test)]
mod build;
#[cfg(test)]
mod create_version;
#[cfg(test)]
mod delete;
#[cfg(test)]
mod retire;
#[cfg(test)]
mod support;
