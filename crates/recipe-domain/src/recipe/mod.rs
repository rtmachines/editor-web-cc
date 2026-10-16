//! The Recipe Management context of `domain-model.md` section 2.2.
//!
//! The module holds the value objects, the identifiers, and the error type of this context.

mod diff;
mod draft;
mod error;
mod ids;
mod lang_code;
mod limit;
mod optimistic;
mod ordinal;
mod rebind;
mod reference_number;
mod root;
mod snapshot;
mod state;
mod tag;
mod value;

pub use diff::{
    DefinitionChange, DiffBounds, DiffKind, DiffKindReason, DiffPair, DiffPart, DiffRow, DiffRows,
    diff_pairs,
};
pub(crate) use draft::not_on_platform;
pub use draft::{
    ApprovalReady, BudgetCounts, DefinitionUpdate, Draft, LEAVES_PER_VERSION,
    LIMIT_ROWS_PER_VERSION, NewParameter, PARAMETERS_PER_VERSION, Parameter, ParameterChange,
    Retype, StoredDraft, StoredParameter, check_budgets,
};
pub use error::{Budget, DraftError, RecipeError, RecipeErrors};
pub use ids::{ParameterId, ParameterKey, ParameterKeyReason, RecipeId, RecipeVersionId};
pub use lang_code::{LangCode, Translation};
pub use limit::{LimitFault, LimitInput, LimitList, LimitListReason, ProcessLimit, StoredLimit};

pub use optimistic::Optimistic;
pub use ordinal::{Ordinal, OrdinalReason};
pub use rebind::{
    ArrayShape, MemberValue, RebindLeaves, RebindPlan, RebindRequest, RebindTargetReason,
    rebind_new_leaves,
};
pub use reference_number::{ReferenceNumber, ReferenceNumberReason};
pub use root::{
    ApprovalOutcome, ApprovalPolicy, Approver, Recipe, RecipeDeleter, StoredRecipe,
    StoredRecipeReason, StoredVersionHeader, VersionHeader, breaks_separation_of_duties,
};
pub use snapshot::{SnapshotError, SnapshotItem, VersionSnapshot};
pub use state::{
    ForbiddenTransition, Transition, VersionState, VersionStateReason, VersionStatus,
    VersionStatusReason,
};
pub use tag::{ParameterTag, ParameterTagReason};
pub use value::{
    Expected, LeafText, LeafTexts, LeafTextsError, ParameterValue, StoredValue, ValueFault,
    ValueFaults, ValueInput,
};
