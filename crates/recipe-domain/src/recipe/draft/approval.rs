//! Approval checks of a draft (design D5, D12, ADR-0027, ADR-0028).

use std::collections::{BTreeSet, HashSet};

use super::{Draft, stored_limits};
use crate::identity::UserId;
use crate::plant::{LeafClass, LeafPath, LiveUnits, TypeTrees};
use crate::recipe::snapshot::check_snapshot;
use crate::recipe::{DraftError, RecipeError, RecipeErrors, RecipeVersionId};

/// The proof that a draft passed the checks of an approval.
///
/// The type solves one problem: a draft with no parameters or with a missing limit cannot
/// become `Approved` (I17). Only [`Draft::check_for_approval`] creates the token. The type has
/// no public constructor and no `Clone`, so one check gives one approval.
///
/// The token owns the checked draft (ADR-0029). While the token lives, no command can change
/// that draft, so the token cannot be stale. The token has no lifetime, so it can leave
/// `spawn_blocking`. A command on the draft after the check is a use after a move. This
/// sequence does not compile:
///
/// ```compile_fail,E0382
/// use recipe_domain::plant::{LiveUnits, TypeTrees};
/// use recipe_domain::recipe::{ApprovalPolicy, Approver, Draft, ParameterKey, Recipe};
///
/// fn stale_token(
///     mut draft: Draft,
///     recipe: &mut Recipe,
///     (units, trees): (&LiveUnits, &TypeTrees),
///     (approver, policy): (&Approver, &ApprovalPolicy),
///     key: ParameterKey,
/// ) {
///     let ready = draft.check_for_approval(units, trees, false).unwrap();
///     let _ = draft.remove_parameter(key);
///     let _ = recipe.approve(ready, 1, approver, policy);
/// }
/// ```
///
/// The same sequence without the command on the draft compiles:
///
/// ```
/// use recipe_domain::plant::{LiveUnits, TypeTrees};
/// use recipe_domain::recipe::{ApprovalPolicy, Approver, Draft, Recipe};
///
/// fn fresh_token(
///     draft: Draft,
///     recipe: &mut Recipe,
///     (units, trees): (&LiveUnits, &TypeTrees),
///     (approver, policy): (&Approver, &ApprovalPolicy),
/// ) {
///     let ready = draft.check_for_approval(units, trees, false).unwrap();
///     let _ = recipe.approve(ready, 1, approver, policy);
/// }
/// ```
#[derive(Debug)]
pub struct ApprovalReady {
    draft: Draft,
    allow_outdated: bool,
}

impl ApprovalReady {
    /// Return the checked draft. The token gives read access only.
    #[must_use]
    pub const fn draft(&self) -> &Draft {
        &self.draft
    }

    /// Return the identifier of the checked version.
    #[must_use]
    pub const fn version(&self) -> RecipeVersionId {
        self.draft.version()
    }

    /// Return the number of the checked version.
    #[must_use]
    pub const fn number(&self) -> u32 {
        self.draft.number()
    }

    /// Return the row version of the draft at the time of the check.
    #[must_use]
    pub const fn row_version(&self) -> i64 {
        self.draft.row_version()
    }

    /// Return the user who created the draft.
    #[must_use]
    pub const fn creator(&self) -> UserId {
        self.draft.created_by()
    }

    /// Return the users who changed the draft.
    #[must_use]
    pub const fn editors(&self) -> &BTreeSet<UserId> {
        self.draft.editors()
    }

    /// Return `true` when the approval accepts outdated parameters (A29).
    #[must_use]
    pub const fn allow_outdated(&self) -> bool {
        self.allow_outdated
    }
}

/// Return `true` when a leaf of this class needs a process limit at approval (I17).
const fn needs_limit(class: LeafClass) -> bool {
    matches!(
        class,
        LeafClass::Numeric | LeafClass::String | LeafClass::Duration | LeafClass::Instant
    )
}

impl Draft {
    /// Check the draft for an approval and return the token (design D5, D12, ADR-0029).
    ///
    /// The method consumes the draft, and the token owns it. A failed check drops the draft,
    /// so a caller that needs the draft after a failure clones it first.
    /// The order is the snapshot check of the parameters and the machine. Then the check for no
    /// parameters, the limits, the S7 layout, and the outdated parameters. The snapshot check has
    /// no recipe, because a live draft of a deleted recipe cannot exist.
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees belong to another machine or platform.
    /// Returns `E-RCP-020` with every item of the snapshot check. Returns `E-RCP-008` for a draft
    /// without parameters. Returns the list of `E-RCP-016` entries of [`Draft::check_limits`].
    /// Returns the errors of [`Draft::check_s7_layout`]. Returns `E-RCP-024` for the first
    /// outdated parameter when `allow_outdated` is `false`.
    pub fn check_for_approval(
        self,
        units: &LiveUnits,
        trees: &TypeTrees,
        allow_outdated: bool,
    ) -> Result<ApprovalReady, DraftError> {
        self.check_draft_trees(trees)?;
        check_snapshot(self.parameters(), None, units, trees)?;
        if self.parameters.is_empty() {
            return Err(RecipeError::DraftHasNoParameters {
                reference: self.reference.clone(),
                number: self.number,
            }
            .into());
        }
        self.check_limits(trees)?;
        self.check_s7_layout(trees)?;
        self.check_outdated(trees, allow_outdated)?;
        Ok(ApprovalReady {
            draft: self,
            allow_outdated,
        })
    }

    /// Check that every numeric, string, duration, and instant leaf has a limit (I17, A36).
    ///
    /// A limit covers a leaf directly or through `[*]`. A parameter whose type tree did not
    /// resolve is skipped, because the snapshot check reports it. The cost is the number of
    /// leaves plus the number of leaves that the limits cover.
    ///
    /// # Errors
    ///
    /// Returns a list with one `E-RCP-016` entry for each leaf without a limit. The list is
    /// ordered by tag and then by leaf. Returns the error of a stored limit that the tree rejects.
    /// Returns `E-PLT-013` when the trees belong to another machine or platform, or when they
    /// do not hold the type of a parameter.
    pub fn check_limits(&self, trees: &TypeTrees) -> Result<(), DraftError> {
        let mut missing = Vec::new();
        for parameter in self.parameters() {
            let Some(tree) = self.held_tree(trees, parameter.plc_type())? else {
                continue;
            };
            let limits = stored_limits(parameter.tag(), parameter.limits(), tree)?;
            let covered: HashSet<LeafPath> = limits
                .iter()
                .filter_map(|limit| limit.leaf_path().concrete_paths(tree))
                .flatten()
                .collect();
            let leaves = tree.leaves();
            missing.extend(
                leaves
                    .iter()
                    .filter(|leaf| needs_limit(leaf.target().leaf_class()))
                    .filter(|leaf| !covered.contains(leaf.path()))
                    .map(|leaf| RecipeError::LimitMissing {
                        tag: parameter.tag().clone(),
                        leaf: leaf.path().clone(),
                    }),
            );
        }
        match RecipeErrors::from_vec(missing) {
            Some(list) => Err(list.into()),
            None => Ok(()),
        }
    }

    /// Check that no parameter uses an older revision of a struct (A29).
    ///
    /// # Errors
    ///
    /// Returns `E-PLT-013` when the trees belong to another machine or platform, or when they
    /// do not hold the type of a parameter. Returns `E-RCP-024` for the first outdated
    /// parameter in tag order when `allow_outdated` is `false`.
    pub fn check_outdated(
        &self,
        trees: &TypeTrees,
        allow_outdated: bool,
    ) -> Result<(), DraftError> {
        if allow_outdated {
            return Ok(());
        }
        for parameter in self.parameters() {
            self.held_tree(trees, parameter.plc_type())?;
            if let Some(outdated) = trees.outdated(parameter.plc_type()) {
                return Err(RecipeError::TypeRevisionOutdated {
                    tag: parameter.tag().clone(),
                    type_name: outdated.type_name().clone(),
                    current: outdated.current(),
                    newest: outdated.newest(),
                }
                .into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
