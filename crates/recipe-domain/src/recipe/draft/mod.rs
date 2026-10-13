//! The draft aggregate of `domain-model.md` section 2.2 (design D6).

use std::collections::{BTreeMap, BTreeSet};

use crate::identity::UserId;
use crate::plant::{MachineId, PlcPlatform};
use crate::recipe::{ParameterKey, RecipeVersionId, ReferenceNumber, VersionState};

mod add;
mod approval;
mod budget;
mod definition;
#[cfg(test)]
mod edit_support;
mod guard;
mod load;
mod parameter;
mod replace;
mod s7;
#[cfg(test)]
pub(in crate::recipe) mod support;
mod translation;
mod value;

pub use add::NewParameter;
pub(in crate::recipe) use add::step_content::{check_value, stored_limits, stored_value};
pub use approval::ApprovalReady;
pub use budget::{
    BudgetCounts, LEAVES_PER_VERSION, LIMIT_ROWS_PER_VERSION, PARAMETERS_PER_VERSION, check_budgets,
};
pub use definition::{DefinitionUpdate, Retype};
pub(crate) use guard::not_on_platform;
pub use parameter::{Parameter, StoredParameter};

/// The live draft of a version as storage returns it, with its parameters.
///
/// The data is not checked. [`Draft::load`] checks the state and the identity of the parameters.
#[derive(Clone, Debug)]
pub struct StoredDraft {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The machine of the recipe.
    pub machine: MachineId,
    /// The PLC platform of the machine.
    pub platform: PlcPlatform,
    /// The identifier of the version.
    pub version: RecipeVersionId,
    /// The number of the version in its recipe.
    pub number: u32,
    /// The state of the version.
    pub state: VersionState,
    /// The row version of the draft for the optimistic check.
    pub row_version: i64,
    /// The user who created the draft.
    pub created_by: UserId,
    /// The users who changed the draft.
    pub editors: Vec<UserId>,
    /// The parameters of the draft.
    pub parameters: Vec<StoredParameter>,
}

/// The live draft of a recipe version with its parameters.
///
/// The type solves one problem: every change of a parameter passes the checks of the draft,
/// so a parameter cannot change on its own.
#[derive(Clone, Debug)]
pub struct Draft {
    reference: ReferenceNumber,
    machine: MachineId,
    platform: PlcPlatform,
    version: RecipeVersionId,
    number: u32,
    row_version: i64,
    created_by: UserId,
    editors: BTreeSet<UserId>,
    parameters: BTreeMap<ParameterKey, Parameter>,
}

impl Draft {
    /// Return the reference number of the recipe.
    #[must_use]
    pub const fn reference(&self) -> &ReferenceNumber {
        &self.reference
    }

    /// Return the number of the version.
    #[must_use]
    pub const fn number(&self) -> u32 {
        self.number
    }

    /// Return the identifier of the version.
    #[must_use]
    pub const fn version(&self) -> RecipeVersionId {
        self.version
    }

    /// Return the row version of the draft.
    #[must_use]
    pub const fn row_version(&self) -> i64 {
        self.row_version
    }

    /// Return the machine of the recipe.
    #[must_use]
    pub const fn machine(&self) -> MachineId {
        self.machine
    }

    /// Return the PLC platform of the machine.
    #[must_use]
    pub const fn platform(&self) -> PlcPlatform {
        self.platform
    }

    /// Return the user who created the draft.
    #[must_use]
    pub const fn created_by(&self) -> UserId {
        self.created_by
    }

    /// Return the users who changed the draft.
    #[must_use]
    pub const fn editors(&self) -> &BTreeSet<UserId> {
        &self.editors
    }

    /// Return the parameters ordered by tag.
    #[must_use]
    pub fn parameters(&self) -> Vec<&Parameter> {
        let mut ordered: Vec<&Parameter> = self.parameters.values().collect();
        ordered.sort_by(|a, b| a.tag().cmp(b.tag()));
        ordered
    }
}

/// The old and the new form of one parameter, for the audit diff.
///
/// The type solves one problem: a command returns what it changed, so the use case writes the
/// audit entry without a second read. `before` is `None` for a new parameter and `after` is
/// `None` for a deleted one.
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterChange {
    before: Option<Parameter>,
    after: Option<Parameter>,
}

impl ParameterChange {
    /// Build a change from the old and the new parameter.
    #[must_use]
    pub(crate) const fn new(before: Option<Parameter>, after: Option<Parameter>) -> Self {
        Self { before, after }
    }

    /// Return the parameter before the change, or `None` for a new parameter.
    #[must_use]
    pub const fn before(&self) -> Option<&Parameter> {
        self.before.as_ref()
    }

    /// Return the parameter after the change, or `None` for a deleted parameter.
    #[must_use]
    pub const fn after(&self) -> Option<&Parameter> {
        self.after.as_ref()
    }
}
