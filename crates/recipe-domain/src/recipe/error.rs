//! The error type of the Recipe Management context.

use crate::error_codes::ErrorCode;
use crate::identity::Login;
use crate::plant::{LeafPath, PlantError, PlcPlatform, S7Address, S7Width, TypeName};
use crate::recipe::{
    Expected, LangCode, ParameterKey, ParameterTag, RebindTargetReason, ReferenceNumber,
    SnapshotItem, VersionStatus,
};

use std::fmt;

/// The counted thing of a size budget (`reliability.md` section 7.11).
///
/// The type solves one problem: the message `E-RCP-021` names what the version has too many of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Budget {
    /// The parameters of a version.
    Parameters,
    /// The scalar leaves of a version.
    Leaves,
    /// The limit rows of a version.
    LimitRows,
    /// The version numbers of a recipe.
    VersionNumbers,
}

impl fmt::Display for Budget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Parameters => "parameters",
            Self::Leaves => "leaves",
            Self::LimitRows => "limit rows",
            Self::VersionNumbers => "version numbers",
        })
    }
}

/// Error of the Recipe Management context (`reliability.md` section 1.1).
///
/// The enum has one variant for each code in the range `E-RCP-001` to
/// `E-RCP-069` that the domain returns. Later tasks add more variants.
#[derive(Debug, thiserror::Error)]
pub enum RecipeError {
    /// A new recipe asks for a reference number that another recipe holds (`E-RCP-001`).
    #[error("Reference number {reference} is already in use.")]
    DuplicateReference {
        /// The reference number that another recipe holds.
        reference: ReferenceNumber,
    },
    /// A text is not a language code (`E-RCP-027`).
    #[error("{lang} is not a language code. Use a code such as pl or de-AT.")]
    InvalidLangCode {
        /// The rejected input text.
        lang: String,
    },
    /// A recipe is deleted (`E-RCP-012`).
    #[error("Recipe {reference} is deleted. An administrator can restore it.")]
    RecipeDeleted {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
    },
    /// A recipe has an approved version and cannot be deleted (`E-RCP-010`).
    #[error("Recipe {reference} has an approved version. An Administrator can retire the version.")]
    RecipeHasApprovedVersion {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
    },
    /// A recipe has a retired version and only an administrator can delete it (`E-RCP-026`).
    #[error("Recipe {reference} has a retired version. Only an administrator can delete it.")]
    RecipeHasRetiredVersion {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
    },
    /// The approver created or edited the draft (`E-RCP-015`, I16).
    #[error(
        "User {login} created or edited draft {number} of recipe {reference}. A different user must approve it."
    )]
    ApproverEditedDraft {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the draft.
        number: u32,
        /// The login of the approver.
        login: Login,
    },
    /// A version is not a live draft (`E-RCP-007`).
    #[error("Version {number} of recipe {reference} is not a draft.")]
    VersionNotDraft {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the version.
        number: u32,
    },
    /// A version is not approved (`E-RCP-009`).
    #[error("Version {number} of recipe {reference} is not approved.")]
    VersionNotApproved {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the version.
        number: u32,
    },
    /// A recipe already has a live draft (`E-RCP-011`).
    #[error("Recipe {reference} already has draft version {number}. Edit or discard that draft.")]
    DraftAlreadyExists {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the live draft.
        number: u32,
    },
    /// A version exceeds a size budget (`E-RCP-021`).
    #[error("The version exceeds the limit of {limit} {budget}. Current count: {actual}.")]
    SizeBudgetExceeded {
        /// The counted thing.
        budget: Budget,
        /// The largest allowed count.
        limit: u64,
        /// The count that the version reached.
        actual: u64,
    },
    /// A recipe is not deleted and cannot be restored (`E-RCP-025`).
    #[error("Recipe {reference} is not deleted.")]
    RecipeNotDeleted {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
    },
    /// A version is not a live draft and cannot change (`E-RCP-002`).
    #[error(
        "Version {number} of recipe {reference} is {status}. Create a new draft to make changes."
    )]
    VersionNotEditable {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the version.
        number: u32,
        /// The status of the version.
        status: VersionStatus,
    },
    /// Two parameters of one version have one tag (`E-RCP-014`).
    #[error("Parameter tag {tag} is already used in version {number}.")]
    DuplicateTag {
        /// The tag that two parameters use.
        tag: ParameterTag,
        /// The number of the version.
        number: u32,
    },
    /// A parameter on an S7-300 machine has no address (`E-RCP-003`).
    #[error("Parameter {tag} needs a PLC address. The machine uses an S7-300 controller.")]
    S7AddressRequired {
        /// The tag of the parameter.
        tag: ParameterTag,
    },
    /// The address of a parameter overlaps the address of another parameter (`E-RCP-022`).
    #[error("Address {address} of {tag} overlaps with {other_tag}.")]
    S7AddressOverlap {
        /// The tag of the parameter that the check reports.
        tag: ParameterTag,
        /// The tag of the parameter that holds the same bytes.
        other_tag: ParameterTag,
        /// The address of `tag`.
        address: S7Address,
    },
    /// The address of a value wider than one byte starts at an odd byte (`E-RCP-023`).
    #[error("Address {address} of {tag} must start at an even byte.")]
    S7AddressMisaligned {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The address of the parameter.
        address: S7Address,
    },
    /// A parameter has an address on a platform that is not S7 (`E-RCP-053`).
    #[error(
        "Parameter {tag} cannot have a PLC address on platform {platform}. Delete the address."
    )]
    S7AddressNotAllowed {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The platform of the machine.
        platform: PlcPlatform,
    },
    /// The width of an address does not match the type of the parameter (`E-RCP-054`).
    #[error("Address {address} of {tag} does not match the type of {tag}. Use the width {width}.")]
    S7AddressWidthMismatch {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The address of the parameter.
        address: S7Address,
        /// The width that the type needs.
        width: S7Width,
    },
    /// A value is outside the process limit of its leaf (`E-RCP-004`).
    #[error("Value {value} of {tag}{leaf} is outside the process limit {min} to {max}.")]
    ValueOutOfLimit {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The path of the leaf. The empty path names the whole value.
        leaf: LeafPath,
        /// The value as message text.
        value: String,
        /// The lower bound as message text.
        min: String,
        /// The upper bound as message text.
        max: String,
    },
    /// A value does not have the form of its type (`E-RCP-005`).
    #[error("Value of {tag}{leaf} does not match the type. Expected {expected}.")]
    ValueShapeMismatch {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The path of the part that does not match. The empty path names the whole value.
        leaf: LeafPath,
        /// The form that the type needs. The web layer maps it to a key of the bundles.
        expected: Expected,
    },
    /// A bound or a length of a limit is outside the range of the type (`E-RCP-006`).
    #[error("Process limit of {tag}{leaf} is outside the range of the data type.")]
    LimitOutsideTypeRange {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The path of the limited leaf.
        leaf: LeafPath,
    },
    /// The type of a parameter has no leaf at the path of a limit (`E-RCP-017`).
    #[error("Path {leaf} does not exist in the type of {tag}.")]
    LeafPathNotFound {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The path of the limit row.
        leaf: LeafPath,
    },
    /// The lower bound of a limit is greater than the upper bound (`E-RCP-018`).
    #[error("The minimum {min} of {tag}{leaf} is greater than the maximum {max}.")]
    MinGreaterThanMax {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The path of the limited leaf.
        leaf: LeafPath,
        /// The lower bound as message text.
        min: String,
        /// The upper bound as message text.
        max: String,
    },
    /// A leaf does not accept the field of a limit (`E-RCP-019`).
    #[error(
        "{tag}{leaf} does not accept this limit. A BOOL, CHAR, or WCHAR leaf has no limit. A string leaf has only a maximum length."
    )]
    LimitNotApplicable {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The path of the limited leaf.
        leaf: LeafPath,
    },
    /// The type of a stored parameter is no longer on the platform (`E-RCP-013`).
    ///
    /// Only an item of [`RecipeError::ValidationFailed`] carries this error.
    #[error("Parameter {tag} uses type {type_name}. This type does not exist on {platform}.")]
    TypeNotOnPlatform {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The label of the type, for example `WSTRING(10)`.
        type_name: String,
        /// The platform of the machine.
        platform: PlcPlatform,
    },
    /// A draft has no parameters and cannot be approved (`E-RCP-008`).
    #[error(
        "Version {number} of recipe {reference} has no parameters. Add at least one parameter before approval."
    )]
    DraftHasNoParameters {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the version.
        number: u32,
    },
    /// A leaf of a parameter has no process limit at approval (`E-RCP-016`).
    #[error("{tag}{leaf} has no process limit. Set a limit before approval.")]
    LimitMissing {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The concrete path of the leaf. The empty path names the whole value.
        leaf: LeafPath,
    },
    /// A parameter uses an older revision of a struct (`E-RCP-024`).
    #[error(
        "{tag} uses revision {current} of {type_name}. The newest revision is {newest}. Rebind the parameter."
    )]
    TypeRevisionOutdated {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The name of the struct.
        type_name: TypeName,
        /// The revision that the parameter uses.
        current: u32,
        /// The newest revision of the struct.
        newest: u32,
    },
    /// The snapshot check of a version found problems (`E-RCP-020`).
    #[error("{} parameters are not valid. See the list.", distinct_tags(.items))]
    ValidationFailed {
        /// Every problem of the snapshot, parameter items first, in tag order.
        items: Vec<SnapshotItem>,
    },
    /// A parameter has no translation for a language (`E-RCP-050`).
    #[error(
        "Parameter {tag} in version {number} of recipe {reference} has no translation for {lang}."
    )]
    TranslationNotFound {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the version.
        number: u32,
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The language that the parameter has no translation for.
        lang: LangCode,
    },
    /// A draft has no parameter with a key (`E-RCP-051`).
    #[error("Parameter {key} does not exist in version {number} of recipe {reference}.")]
    ParameterNotFound {
        /// The reference number of the recipe.
        reference: ReferenceNumber,
        /// The number of the version.
        number: u32,
        /// The key that no parameter of the draft has.
        key: ParameterKey,
    },
    /// A rebind adds a member and the request gives it no value (`E-RCP-028`).
    #[error("{tag}{leaf} is a new member. Give it a value.")]
    RebindNeedsValue {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The concrete path of the new scalar leaf.
        leaf: LeafPath,
    },
    /// A rebind removes or retypes a member that has a limit (`E-RCP-029`).
    #[error(
        "{tag}{leaf} has a limit, but the new revision has no such member or gives the member another type. Delete the limit first. After the rebind, set the limit again."
    )]
    RebindDropsLimit {
        /// The tag of the parameter.
        tag: ParameterTag,
        /// The path of the limit row.
        leaf: LeafPath,
    },
    /// The target of a rebind is not a newer revision of the source struct (`E-RCP-052`).
    #[error("The rebind of {type_name} is not possible: {reason}.")]
    RebindTargetInvalid {
        /// The label of the source type, such as `ST_Zone@2`.
        type_name: String,
        /// The rule that the request breaks.
        reason: RebindTargetReason,
    },
}

/// Count the distinct parameter tags of the items of a snapshot check.
///
/// An item of the version has no tag and does not count. One parameter with many problems counts
/// once.
fn distinct_tags(items: &[SnapshotItem]) -> usize {
    items
        .iter()
        .filter_map(|item| match item {
            SnapshotItem::Parameter { tag, .. } | SnapshotItem::UnknownUnit { tag, .. } => {
                Some(tag)
            }
            SnapshotItem::Version { .. } => None,
        })
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

impl RecipeError {
    /// Return the stable code of the error (`reliability.md` section 1.2).
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::DuplicateReference { .. } => ErrorCode::RCP_001,
            Self::InvalidLangCode { .. } => ErrorCode::RCP_027,
            Self::RecipeDeleted { .. } => ErrorCode::RCP_012,
            Self::VersionNotDraft { .. } => ErrorCode::RCP_007,
            Self::VersionNotApproved { .. } => ErrorCode::RCP_009,
            Self::DraftAlreadyExists { .. } => ErrorCode::RCP_011,
            Self::SizeBudgetExceeded { .. } => ErrorCode::RCP_021,
            Self::RecipeHasApprovedVersion { .. } => ErrorCode::RCP_010,
            Self::RecipeNotDeleted { .. } => ErrorCode::RCP_025,
            Self::RecipeHasRetiredVersion { .. } => ErrorCode::RCP_026,
            Self::VersionNotEditable { .. } => ErrorCode::RCP_002,
            Self::DuplicateTag { .. } => ErrorCode::RCP_014,
            Self::ValueOutOfLimit { .. } => ErrorCode::RCP_004,
            Self::ValueShapeMismatch { .. } => ErrorCode::RCP_005,
            Self::LimitOutsideTypeRange { .. } => ErrorCode::RCP_006,
            Self::LeafPathNotFound { .. } => ErrorCode::RCP_017,
            Self::MinGreaterThanMax { .. } => ErrorCode::RCP_018,
            Self::LimitNotApplicable { .. } => ErrorCode::RCP_019,
            Self::DraftHasNoParameters { .. } => ErrorCode::RCP_008,
            Self::ApproverEditedDraft { .. } => ErrorCode::RCP_015,
            Self::LimitMissing { .. } => ErrorCode::RCP_016,
            Self::TypeRevisionOutdated { .. } => ErrorCode::RCP_024,
            Self::TypeNotOnPlatform { .. } => ErrorCode::RCP_013,
            Self::ValidationFailed { .. } => ErrorCode::RCP_020,
            Self::TranslationNotFound { .. } => ErrorCode::RCP_050,
            Self::ParameterNotFound { .. } => ErrorCode::RCP_051,
            Self::RebindNeedsValue { .. } => ErrorCode::RCP_028,
            Self::RebindDropsLimit { .. } => ErrorCode::RCP_029,
            Self::RebindTargetInvalid { .. } => ErrorCode::RCP_052,
            Self::S7AddressRequired { .. } => ErrorCode::RCP_003,
            Self::S7AddressOverlap { .. } => ErrorCode::RCP_022,
            Self::S7AddressMisaligned { .. } => ErrorCode::RCP_023,
            Self::S7AddressNotAllowed { .. } => ErrorCode::RCP_053,
            Self::S7AddressWidthMismatch { .. } => ErrorCode::RCP_054,
        }
    }
}

/// One or more errors of the Recipe Management context, in a fixed order (ADR-0027).
///
/// The first entry is stored apart, so the list cannot be empty.
/// The code of the list is the code of its first entry.
/// The first entry is boxed, so a [`DraftError`] stays small (`clippy::result_large_err`).
#[derive(Debug)]
pub struct RecipeErrors {
    first: Box<RecipeError>,
    rest: Vec<RecipeError>,
}

impl RecipeErrors {
    /// Append an error after the existing entries.
    pub fn push(&mut self, error: RecipeError) {
        self.rest.push(error);
    }

    /// Return the first entry of the list.
    #[must_use]
    pub fn first(&self) -> &RecipeError {
        &self.first
    }

    /// Return the number of entries. The count is never zero.
    #[must_use]
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Return `false`. A list always holds at least one entry.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// Iterate over the entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &RecipeError> {
        std::iter::once(&*self.first).chain(self.rest.iter())
    }

    /// Convert the list into a vector of its entries in insertion order.
    #[must_use]
    pub fn into_vec(self) -> Vec<RecipeError> {
        std::iter::once(*self.first).chain(self.rest).collect()
    }

    /// Build a list from a vector. Return `None` for an empty vector.
    #[must_use]
    pub fn from_vec(errors: Vec<RecipeError>) -> Option<Self> {
        let mut entries = errors.into_iter();
        let first = entries.next()?;
        Some(Self {
            first: Box::new(first),
            rest: entries.collect(),
        })
    }

    /// Return the code of the first entry (`reliability.md` section 1.2).
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        self.first.code()
    }
}

impl From<RecipeError> for RecipeErrors {
    fn from(error: RecipeError) -> Self {
        Self {
            first: Box::new(error),
            rest: Vec::new(),
        }
    }
}

impl Extend<RecipeError> for RecipeErrors {
    /// Append errors after the existing entries, in their order.
    fn extend<I: IntoIterator<Item = RecipeError>>(&mut self, errors: I) {
        self.rest.extend(errors);
    }
}

impl fmt::Display for RecipeErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, error) in self.iter().enumerate() {
            if index > 0 {
                f.write_str("\n")?;
            }
            write!(f, "{error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for RecipeErrors {}

/// Error of a draft command that can also fail with a Plant Model code (ADR-0027).
///
/// The enum owns no code. [`DraftError::code`] returns the code of the inner error.
#[derive(Debug, thiserror::Error)]
pub enum DraftError {
    /// An error of the Recipe Management context.
    #[error(transparent)]
    Recipe(#[from] RecipeError),
    /// An error of the Plant Model context.
    #[error(transparent)]
    Plant(#[from] PlantError),
    /// A list of errors of the Recipe Management context.
    #[error(transparent)]
    List(#[from] RecipeErrors),
}

impl DraftError {
    /// Return the stable code of the inner error. A list gives the code of its first entry.
    #[must_use]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Recipe(error) => error.code(),
            Self::Plant(error) => error.code(),
            Self::List(errors) => errors.code(),
        }
    }
}

#[cfg(test)]
mod registry_tests;

#[cfg(test)]
mod tests;
