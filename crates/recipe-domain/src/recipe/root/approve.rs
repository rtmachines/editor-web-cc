//! Approval of a version (design D4, D5, ADR-0022, ADR-0028).

use super::*;
use crate::identity::{Login, UserId};
use crate::recipe::ApprovalReady;

/// The rule of an approval that the settings control (A34).
///
/// The type solves one problem: the domain knows no settings, so the use case passes the rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApprovalPolicy {
    /// When `true`, a user who created or edited the draft cannot approve it (I16).
    pub separation_of_duties: bool,
}

impl Default for ApprovalPolicy {
    /// Return the policy with separation of duties on (`approval.separation_of_duties`).
    fn default() -> Self {
        Self {
            separation_of_duties: true,
        }
    }
}

/// The user who approves a version.
///
/// The login is for the message of `E-RCP-015`. The domain compares only the identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Approver {
    /// The identifier of the user.
    pub user: UserId,
    /// The login of the user.
    pub login: Login,
}

/// The result of an approval that did not conflict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApprovalOutcome {
    /// The draft is now approved. The previous approved version is now retired.
    Approved {
        /// The identifier of the new approved version.
        approved: RecipeVersionId,
        /// The identifier of the version that this approval retired, if there was one.
        retired: Option<RecipeVersionId>,
    },
    /// An earlier request already approved the version. This request changed nothing.
    AlreadyApproved,
}

/// Return true when the header is approved by the request that sent `expected` (`api.md` section 4).
fn is_replay(header: &VersionHeader, expected: i64) -> bool {
    header.state == VersionState::Approved && expected.checked_add(1) == Some(header.row_version)
}

impl Recipe {
    /// Return `AlreadyApproved` when an earlier request approved this version (`api.md` section 4).
    ///
    /// The header must be approved and must have the row version `expected_row_version + 1`.
    /// The use case calls this method before it opens the transaction.
    #[must_use]
    pub fn approval_replay(
        &self,
        number: u32,
        expected_row_version: i64,
    ) -> Option<ApprovalOutcome> {
        let header = self.version(number)?;
        is_replay(header, expected_row_version).then_some(ApprovalOutcome::AlreadyApproved)
    }

    /// Check that this version is a live draft that an approval can take (ADR-0028).
    ///
    /// The use case calls this method before `Draft::load`, so an approval of a retired version
    /// gives `E-RCP-007` and not `E-RCP-002`.
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError::RecipeDeleted`] (`E-RCP-012`) when the recipe is deleted.
    /// Returns [`RecipeError::VersionNotDraft`] (`E-RCP-007`) for an unknown number
    /// and for a version that is not a live draft.
    pub fn check_approvable(&self, number: u32) -> Result<(), RecipeError> {
        self.require_not_deleted()?;
        let forbidden = || forbidden_step(&self.reference, number, Transition::Approve);
        let header = self.version(number).ok_or_else(forbidden)?;
        header
            .state
            .apply(Transition::Approve)
            .map(|_| ())
            .map_err(|_| forbidden())
    }

    /// Approve a checked draft and retire the approved version (I1, I2, I16, ADR-0022, ADR-0029).
    ///
    /// The method consumes the token, and the token owns the checked draft. The SQL statements write the times and the row versions.
    /// A replay and a stale row version are values and change nothing.
    ///
    /// # Errors
    ///
    /// Returns [`RecipeError::RecipeDeleted`] (`E-RCP-012`) when the recipe is deleted.
    /// Returns [`RecipeError::VersionNotDraft`] (`E-RCP-007`) for an unknown version
    /// and for a version that is not a live draft.
    /// Returns [`RecipeError::ApproverEditedDraft`] (`E-RCP-015`) when the policy requires
    /// separation of duties and the approver created or edited the draft.
    pub fn approve(
        &mut self,
        ready: ApprovalReady,
        expected_row_version: i64,
        approver: &Approver,
        policy: &ApprovalPolicy,
    ) -> Result<Optimistic<ApprovalOutcome>, RecipeError> {
        self.require_not_deleted()?;
        let not_draft = |number| forbidden_step(&self.reference, number, Transition::Approve);
        let header = self
            .versions
            .iter()
            .find(|v| v.id == ready.version())
            .ok_or_else(|| not_draft(ready.number()))?;
        if is_replay(header, expected_row_version) {
            return Ok(Optimistic::Applied(ApprovalOutcome::AlreadyApproved));
        }
        if header.row_version != expected_row_version {
            return Ok(Optimistic::Conflict {
                expected: expected_row_version,
                actual: header.row_version,
            });
        }
        let approved_state = header
            .state
            .apply(Transition::Approve)
            .map_err(|_| not_draft(header.number))?;
        if ready.row_version() != header.row_version {
            return Ok(Optimistic::Conflict {
                expected: ready.row_version(),
                actual: header.row_version,
            });
        }
        if policy.separation_of_duties
            && (approver.user == ready.creator() || ready.editors().contains(&approver.user))
        {
            return Err(RecipeError::ApproverEditedDraft {
                reference: self.reference.clone(),
                number: header.number,
                login: approver.login.clone(),
            });
        }
        let approved = header.id;
        let mut retired = None;
        for other in &mut self.versions {
            if other.id == approved {
                other.state = approved_state;
            } else if let Ok(next) = other.state.apply(Transition::Retire) {
                other.state = next;
                retired = Some(other.id);
            }
        }
        Ok(Optimistic::Applied(ApprovalOutcome::Approved {
            approved,
            retired,
        }))
    }
}

#[cfg(test)]
mod tests;
