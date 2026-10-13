//! The stored form of a recipe and its check (design D4).

use std::collections::BTreeSet;
use std::fmt;

use super::{Recipe, VERSION_NUMBER_MAX, VersionHeader};
use crate::plant::MachineId;
use crate::recipe::{RecipeId, RecipeVersionId, ReferenceNumber, VersionState};
use crate::text::Text;
use crate::timestamp::Timestamp;

/// A version header as storage loads it.
///
/// Storage builds this record. `TryFrom<StoredRecipe>` checks it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredVersionHeader {
    /// The identifier of the version.
    pub id: RecipeVersionId,
    /// The number of the version in its recipe.
    pub number: u32,
    /// The state of the version.
    pub state: VersionState,
    /// The row version of the version.
    pub row_version: i64,
}

/// A recipe row with its version headers as storage loads it.
///
/// Storage builds this record. `TryFrom<StoredRecipe>` checks it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRecipe {
    /// The identifier of the recipe.
    pub id: RecipeId,
    /// The machine of the recipe.
    pub machine: MachineId,
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The name of the recipe.
    pub name: Text,
    /// The instant of deletion, or `None` for a recipe that is not deleted.
    pub deleted_at: Option<Timestamp>,
    /// The version headers of the recipe, in any order.
    pub versions: Vec<StoredVersionHeader>,
}

/// Reason why stored rows do not form a recipe. It is not an error type and has no code.
///
/// Only a corrupt row gives it (ADR-0025).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoredRecipeReason {
    /// A version has the number 0.
    NumberZero,
    /// A version number is above the range of the SQL type `INTEGER`.
    NumberTooLarge,
    /// Two versions have the same number.
    DuplicateNumber,
    /// Two versions have the same identifier.
    DuplicateId,
    /// A version has a row version below 1.
    RowVersionBelowOne,
    /// Two versions are approved (I1).
    TwoApproved,
    /// Two versions are live drafts (A7).
    TwoLiveDrafts,
    /// A deleted recipe has an approved version (ADR-0021).
    DeletedWithApproved,
    /// A deleted recipe has a live draft (ADR-0021).
    DeletedWithLiveDraft,
    /// The version numbers do not run from 1 to the number of versions.
    NumberGap,
}

impl fmt::Display for StoredRecipeReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NumberZero => "a version number is at least 1",
            Self::NumberTooLarge => "a version number is at most 2147483647",
            Self::DuplicateNumber => "two versions of a recipe have the same number",
            Self::DuplicateId => "two versions of a recipe have the same identifier",
            Self::RowVersionBelowOne => "a row version is at least 1",
            Self::TwoApproved => "a recipe has at most one approved version",
            Self::TwoLiveDrafts => "a recipe has at most one live draft",
            Self::DeletedWithApproved => "a deleted recipe has no approved version",
            Self::DeletedWithLiveDraft => "a deleted recipe has no live draft",
            Self::NumberGap => "the version numbers of a recipe run from 1 without a gap",
        })
    }
}

impl TryFrom<StoredRecipe> for Recipe {
    type Error = StoredRecipeReason;

    fn try_from(stored: StoredRecipe) -> Result<Self, Self::Error> {
        check_headers(&stored.versions, stored.deleted_at.is_some())?;
        let mut versions: Vec<VersionHeader> = stored
            .versions
            .into_iter()
            .map(|h| VersionHeader {
                id: h.id,
                number: h.number,
                state: h.state,
                row_version: h.row_version,
            })
            .collect();
        versions.sort_by_key(VersionHeader::number);
        Ok(Self {
            id: stored.id,
            machine: stored.machine,
            reference: stored.reference,
            name: stored.name,
            deleted_at: stored.deleted_at,
            versions,
        })
    }
}

/// Check the stored headers of one recipe against the rules of design D4.
///
/// A version is never deleted, so the numbers are exactly 1 to the number of headers. A deleted
/// recipe has no approved version and no live draft (ADR-0021).
fn check_headers(
    headers: &[StoredVersionHeader],
    is_deleted: bool,
) -> Result<(), StoredRecipeReason> {
    let mut numbers = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut has_approved = false;
    let mut has_live_draft = false;
    for header in headers {
        if header.number == 0 {
            return Err(StoredRecipeReason::NumberZero);
        }
        if header.number > VERSION_NUMBER_MAX {
            return Err(StoredRecipeReason::NumberTooLarge);
        }
        if header.row_version < 1 {
            return Err(StoredRecipeReason::RowVersionBelowOne);
        }
        if !numbers.insert(header.number) {
            return Err(StoredRecipeReason::DuplicateNumber);
        }
        if !ids.insert(header.id) {
            return Err(StoredRecipeReason::DuplicateId);
        }
        match header.state {
            VersionState::Approved if std::mem::replace(&mut has_approved, true) => {
                return Err(StoredRecipeReason::TwoApproved);
            }
            VersionState::LiveDraft if std::mem::replace(&mut has_live_draft, true) => {
                return Err(StoredRecipeReason::TwoLiveDrafts);
            }
            _ => {}
        }
    }
    if is_deleted && has_approved {
        return Err(StoredRecipeReason::DeletedWithApproved);
    }
    if is_deleted && has_live_draft {
        return Err(StoredRecipeReason::DeletedWithLiveDraft);
    }
    let count = u32::try_from(headers.len()).map_err(|_| StoredRecipeReason::NumberGap)?;
    if numbers.last().copied().unwrap_or(0) != count {
        return Err(StoredRecipeReason::NumberGap);
    }
    Ok(())
}
