//! The version state machine of `domain-model.md` section 4 (ADR-0028, design D3).

use std::fmt;

/// Status of a recipe version, as stored in the SQL column `status` (`data-model.md` section 1.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionStatus {
    /// The version is a draft. A discarded draft also has this status.
    Draft,
    /// The version is approved and immutable.
    Approved,
    /// The version is retired and immutable.
    Retired,
}

impl VersionStatus {
    /// Return the SQL text of the status.
    #[must_use]
    pub const fn as_sql(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Approved => "approved",
            Self::Retired => "retired",
        }
    }

    /// Read the SQL text of a status. Any other text gives [`VersionStatusReason::Unknown`].
    ///
    /// # Errors
    ///
    /// Returns [`VersionStatusReason::Unknown`] for text that is not a status.
    pub fn parse(text: &str) -> Result<Self, VersionStatusReason> {
        match text {
            "draft" => Ok(Self::Draft),
            "approved" => Ok(Self::Approved),
            "retired" => Ok(Self::Retired),
            _ => Err(VersionStatusReason::Unknown),
        }
    }
}

impl fmt::Display for VersionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_sql())
    }
}

/// Reason why a text is not a version status. It is not an error type and has no code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionStatusReason {
    /// The text is not `draft`, `approved`, or `retired`.
    Unknown,
}

impl fmt::Display for VersionStatusReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the status is not draft, approved, or retired")
    }
}

/// Reason why a stored status and deletion flag do not form a state.
///
/// It is not an error type and has no code. Only a corrupt row gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VersionStateReason {
    /// An approved or retired version has a deletion mark (`data-model.md` section 3.10).
    DeletedNotDraft,
}

impl fmt::Display for VersionStateReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("only a draft can be deleted")
    }
}

/// State of a recipe version. It has four values, and `status` gives the SQL status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VersionState {
    /// A draft that is not deleted. It can be edited, approved, or discarded.
    LiveDraft,
    /// A draft with a deletion mark. It is read only.
    Discarded,
    /// An approved version. It can be retired.
    Approved,
    /// A retired version. It is read only.
    Retired,
}

impl VersionState {
    /// Return the SQL status of the state. Both draft states give `draft`.
    #[must_use]
    pub const fn status(self) -> VersionStatus {
        match self {
            Self::LiveDraft | Self::Discarded => VersionStatus::Draft,
            Self::Approved => VersionStatus::Approved,
            Self::Retired => VersionStatus::Retired,
        }
    }

    /// Build the state from the stored status and deletion mark.
    ///
    /// # Errors
    ///
    /// Returns [`VersionStateReason::DeletedNotDraft`] for a deleted approved or retired version.
    pub const fn from_stored(
        status: VersionStatus,
        is_deleted: bool,
    ) -> Result<Self, VersionStateReason> {
        match (status, is_deleted) {
            (VersionStatus::Draft, false) => Ok(Self::LiveDraft),
            (VersionStatus::Draft, true) => Ok(Self::Discarded),
            (VersionStatus::Approved, false) => Ok(Self::Approved),
            (VersionStatus::Retired, false) => Ok(Self::Retired),
            (VersionStatus::Approved, true) | (VersionStatus::Retired, true) => {
                Err(VersionStateReason::DeletedNotDraft)
            }
        }
    }

    /// Apply a step to the state. This is the one transition function (ADR-0016).
    ///
    /// # Errors
    ///
    /// Returns [`ForbiddenTransition`] for each pair that is not one of the four allowed pairs.
    pub const fn apply(self, step: Transition) -> Result<Self, ForbiddenTransition> {
        match (self, step) {
            (Self::LiveDraft, Transition::Approve) => Ok(Self::Approved),
            (Self::LiveDraft, Transition::Discard) => Ok(Self::Discarded),
            (Self::LiveDraft, Transition::Edit) => Ok(Self::LiveDraft),
            (Self::Approved, Transition::Retire) => Ok(Self::Retired),
            (Self::LiveDraft, Transition::Retire)
            | (
                Self::Discarded,
                Transition::Approve | Transition::Retire | Transition::Discard | Transition::Edit,
            )
            | (Self::Approved, Transition::Approve | Transition::Discard | Transition::Edit)
            | (
                Self::Retired,
                Transition::Approve | Transition::Retire | Transition::Discard | Transition::Edit,
            ) => Err(ForbiddenTransition { from: self, step }),
        }
    }
}

/// A step of the version state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    /// Approve a live draft.
    Approve,
    /// Retire an approved version.
    Retire,
    /// Discard a live draft.
    Discard,
    /// Edit a live draft. The state does not change.
    Edit,
}

/// Reason why a step is not allowed in a state. It is not an error type and has no code.
///
/// The caller maps it to the code of its command (design D3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForbiddenTransition {
    from: VersionState,
    step: Transition,
}

impl ForbiddenTransition {
    /// Return the state in which the step was tried.
    #[must_use]
    pub const fn state(self) -> VersionState {
        self.from
    }

    /// Return the step that was tried.
    #[must_use]
    pub const fn step(self) -> Transition {
        self.step
    }
}

impl fmt::Display for ForbiddenTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cannot {} a version that is {}.",
            step_word(self.step),
            state_word(self.from)
        )
    }
}

/// Return the verb of a step for a message.
const fn step_word(step: Transition) -> &'static str {
    match step {
        Transition::Approve => "approve",
        Transition::Retire => "retire",
        Transition::Discard => "discard",
        Transition::Edit => "edit",
    }
}

/// Return the state words of a state for a message.
const fn state_word(state: VersionState) -> &'static str {
    match state {
        VersionState::LiveDraft => "a live draft",
        VersionState::Discarded => "discarded",
        VersionState::Approved => "approved",
        VersionState::Retired => "retired",
    }
}

#[cfg(test)]
mod tests;
