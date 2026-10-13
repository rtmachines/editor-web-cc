//! The rebind of a draft to a newer struct revision (design D13, ADR-0018).
//!
//! This module holds the request, the target check, the plan with the mapped values and limits,
//! and the change of the draft.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::plant::{LeafPath, PlcTypeId, TypeTree};
use crate::recipe::{ParameterKey, StoredLimit, StoredValue};
use level::struct_level;
use map::Mapped;

mod apply;
mod build;
mod level;
mod limit;
mod map;
mod shape;
mod target;
pub use shape::ArrayShape;
#[cfg(test)]
mod test_support;
mod value;

/// The request of `RebindTypeRevision` (`domain-model.md` section 5.2).
///
/// The type solves one problem: the use case passes the whole request to the draft. The draft
/// checks the target and maps the parameters in one call.
#[derive(Clone, Debug)]
pub struct RebindRequest {
    /// The struct revision that the parameters use now.
    pub from: PlcTypeId,
    /// The newer revision of the same struct.
    pub to: PlcTypeId,
    /// The values of the new leaves, by parameter and concrete leaf path.
    ///
    /// A value is a decimal text, a string, or a bool in JSON form.
    pub new_member_values: BTreeMap<(ParameterKey, LeafPath), serde_json::Value>,
}

/// The rule that a rebind request breaks (`E-RCP-052`, ADR-0025).
///
/// The enum is the `reason` field of `RecipeError::RebindTargetInvalid`.
/// It is not an error type and has no code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RebindTargetReason {
    /// The source type is not a struct.
    SourceNotStruct,
    /// The target type is not a struct.
    TargetNotStruct,
    /// The target type does not exist on the machine of the draft.
    TargetNotFound,
    /// The target is a revision of another struct.
    OtherStruct,
    /// The target revision is not higher than the source revision.
    NotNewer,
    /// No parameter of the draft uses the source revision.
    NotUsed,
    /// The plan does not match the draft or the trees of the new types.
    PlanMismatch,
}

impl fmt::Display for RebindTargetReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SourceNotStruct => "the source type is not a struct",
            Self::TargetNotStruct => "the target type is not a struct",
            Self::TargetNotFound => {
                "the target revision does not exist on the machine of the draft"
            }
            Self::OtherStruct => "the target is a revision of another struct",
            Self::NotNewer => "the target revision is not newer than the source revision",
            Self::NotUsed => "no parameter of the draft uses the source revision",
            Self::PlanMismatch => "the plan does not match the draft or the new types",
        })
    }
}

/// The checked plan of a rebind.
///
/// The type solves one problem: only `Draft::plan_rebind` builds a plan, so a later step
/// receives a target that passed the check. The plan lists each affected parameter with its
/// planned type tree, its new value, and its new limit rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebindPlan {
    from: PlcTypeId,
    to: PlcTypeId,
    parameters: BTreeMap<ParameterKey, TypeTree>,
    old_types: BTreeMap<ParameterKey, PlcTypeId>,
    values: BTreeMap<ParameterKey, StoredValue>,
    limits: BTreeMap<ParameterKey, Vec<StoredLimit>>,
}

impl RebindPlan {
    /// Build a plan from a checked target, the planned trees, and the mapped content.
    ///
    /// `old_types` holds the type of each affected parameter before the rebind.
    fn new(
        from: PlcTypeId,
        to: PlcTypeId,
        parameters: BTreeMap<ParameterKey, TypeTree>,
        old_types: BTreeMap<ParameterKey, PlcTypeId>,
        mapped: Mapped,
    ) -> Self {
        Self {
            from,
            to,
            parameters,
            old_types,
            values: mapped.values,
            limits: mapped.limits,
        }
    }

    /// List the array shapes that the new types need, each shape once.
    ///
    /// The use case calls `EnsureDerivedType` for each shape. The shapes come in ascending order.
    /// A plan without an array parameter gives an empty list.
    #[must_use]
    pub fn array_shapes(&self) -> Vec<ArrayShape> {
        let shapes: BTreeSet<ArrayShape> = self
            .parameters
            .values()
            .filter_map(struct_level)
            .filter(|level| !level.chain.is_empty())
            .map(|level| {
                ArrayShape::new(level.chain.iter().map(|d| (*d).clone()).collect(), self.to)
            })
            .collect();
        shapes.into_iter().collect()
    }

    /// Return the new limit rows of an affected parameter, or `None` for any other key.
    ///
    /// A kept limit keeps its row. The rows keep the order of the draft.
    #[must_use]
    pub fn limits(&self, key: ParameterKey) -> Option<&[StoredLimit]> {
        self.limits.get(&key).map(Vec::as_slice)
    }

    /// Return the new stored value of an affected parameter, or `None` for any other key.
    #[must_use]
    pub fn value(&self, key: ParameterKey) -> Option<&StoredValue> {
        self.values.get(&key)
    }

    /// Return the struct revision that the affected parameters use now.
    #[must_use]
    pub const fn from(&self) -> PlcTypeId {
        self.from
    }

    /// Return the struct revision that the affected parameters move to.
    #[must_use]
    pub const fn to(&self) -> PlcTypeId {
        self.to
    }

    /// Return the planned type tree of each affected parameter, ordered by key.
    #[must_use]
    pub const fn parameters(&self) -> &BTreeMap<ParameterKey, TypeTree> {
        &self.parameters
    }
}
