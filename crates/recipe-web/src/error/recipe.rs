//! The arguments of the messages of the recipe errors (design D13, `reliability.md` 1.2).
//!
//! A tag, a leaf path, a value, a reference, and a number are data and stay as they are. A state,
//! a budget, a platform, a rebind reason, and an expected form are words of the UI. Each one is a
//! key of the bundles, so a Polish page never shows the English `Display` (design D13).

use std::collections::BTreeSet;

use recipe_domain::plant::LeafPath;
use recipe_domain::recipe::{
    Budget, Expected, RebindTargetReason, RecipeError, SnapshotItem, VersionStatus,
};

use super::LineArg;
use crate::pages::machines::platform_key;

/// Return the leaf path of an error, or `None` when the error names no leaf.
pub(crate) const fn leaf_of(error: &RecipeError) -> Option<&LeafPath> {
    match error {
        RecipeError::ValueOutOfLimit { leaf, .. }
        | RecipeError::ValueShapeMismatch { leaf, .. }
        | RecipeError::LimitOutsideTypeRange { leaf, .. }
        | RecipeError::LeafPathNotFound { leaf, .. }
        | RecipeError::MinGreaterThanMax { leaf, .. }
        | RecipeError::LimitNotApplicable { leaf, .. }
        | RecipeError::LimitMissing { leaf, .. }
        | RecipeError::RebindNeedsValue { leaf, .. }
        | RecipeError::RebindDropsLimit { leaf, .. } => Some(leaf),
        _other => None,
    }
}

/// Return the arguments of the message of a recipe error: its data, and then its words.
pub(super) fn recipe_line_args(error: &RecipeError) -> Vec<(&'static str, LineArg)> {
    let mut args: Vec<(&'static str, LineArg)> = data_fields(error)
        .into_iter()
        .map(|(name, value)| (name, LineArg::Data(value)))
        .collect();
    if let Some(word) = word_arg(error) {
        args.push(word);
    }
    args
}

/// Return the fields of an error of a recipe or a version that are data.
///
/// An error of a parameter goes on to `parameter_fields`, and then to `tag_fields`. A new variant
/// gets no data there, so its message falls back to English. The test of each code shows it.
fn data_fields(error: &RecipeError) -> Vec<(&'static str, String)> {
    match error {
        RecipeError::DuplicateReference { reference }
        | RecipeError::RecipeDeleted { reference }
        | RecipeError::RecipeHasApprovedVersion { reference }
        | RecipeError::RecipeHasRetiredVersion { reference }
        | RecipeError::RecipeNotDeleted { reference } => vec![("reference", reference.to_string())],
        RecipeError::VersionNotDraft { reference, number }
        | RecipeError::VersionNotApproved { reference, number }
        | RecipeError::DraftAlreadyExists { reference, number }
        | RecipeError::DraftHasNoParameters { reference, number }
        | RecipeError::VersionNotEditable {
            reference, number, ..
        } => vec![
            ("reference", reference.to_string()),
            ("number", number.to_string()),
        ],
        RecipeError::ApproverEditedDraft {
            reference,
            number,
            login,
        } => vec![
            ("reference", reference.to_string()),
            ("number", number.to_string()),
            ("login", login.to_string()),
        ],
        RecipeError::ParameterNotFound {
            reference,
            number,
            key,
        } => vec![
            ("reference", reference.to_string()),
            ("number", number.to_string()),
            ("key", key.to_string()),
        ],
        RecipeError::InvalidLangCode { lang } => vec![("lang", lang.clone())],
        RecipeError::SizeBudgetExceeded { limit, actual, .. } => {
            vec![("limit", limit.to_string()), ("actual", actual.to_string())]
        }
        RecipeError::ValidationFailed { items } => vec![("count", distinct_tags(items))],
        RecipeError::RebindTargetInvalid { type_name, .. } => {
            vec![("type_name", type_name.clone())]
        }
        other => parameter_fields(other),
    }
}

/// Return the fields of an error of one parameter that are data.
fn parameter_fields(error: &RecipeError) -> Vec<(&'static str, String)> {
    match error {
        RecipeError::ValueOutOfLimit {
            tag,
            leaf,
            value,
            min,
            max,
        } => vec![
            ("tag", tag.to_string()),
            ("leaf", leaf.to_string()),
            ("value", value.clone()),
            ("min", min.clone()),
            ("max", max.clone()),
        ],
        RecipeError::MinGreaterThanMax {
            tag,
            leaf,
            min,
            max,
        } => vec![
            ("tag", tag.to_string()),
            ("leaf", leaf.to_string()),
            ("min", min.clone()),
            ("max", max.clone()),
        ],
        RecipeError::ValueShapeMismatch { tag, leaf, .. }
        | RecipeError::LimitOutsideTypeRange { tag, leaf }
        | RecipeError::LeafPathNotFound { tag, leaf }
        | RecipeError::LimitNotApplicable { tag, leaf }
        | RecipeError::LimitMissing { tag, leaf }
        | RecipeError::RebindNeedsValue { tag, leaf }
        | RecipeError::RebindDropsLimit { tag, leaf } => {
            vec![("tag", tag.to_string()), ("leaf", leaf.to_string())]
        }
        other => tag_fields(other),
    }
}

/// Return the fields of an error of the tag, the type, or a translation.
fn tag_fields(error: &RecipeError) -> Vec<(&'static str, String)> {
    match error {
        RecipeError::DuplicateTag { tag, number } => {
            vec![("tag", tag.to_string()), ("number", number.to_string())]
        }
        RecipeError::TypeNotOnPlatform { tag, type_name, .. } => {
            vec![("tag", tag.to_string()), ("type_name", type_name.clone())]
        }
        RecipeError::TypeRevisionOutdated {
            tag,
            type_name,
            current,
            newest,
        } => vec![
            ("tag", tag.to_string()),
            ("type_name", type_name.to_string()),
            ("current", current.to_string()),
            ("newest", newest.to_string()),
        ],
        RecipeError::TranslationNotFound {
            reference,
            number,
            tag,
            lang,
        } => vec![
            ("reference", reference.to_string()),
            ("number", number.to_string()),
            ("tag", tag.to_string()),
            ("lang", lang.to_string()),
        ],
        other => address_fields(other),
    }
}

/// Return the fields of an error of the PLC address of a parameter.
fn address_fields(error: &RecipeError) -> Vec<(&'static str, String)> {
    match error {
        RecipeError::S7AddressRequired { tag } | RecipeError::S7AddressNotAllowed { tag, .. } => {
            vec![("tag", tag.to_string())]
        }
        RecipeError::S7AddressOverlap {
            tag,
            other_tag,
            address,
        } => vec![
            ("tag", tag.to_string()),
            ("other_tag", other_tag.to_string()),
            ("address", address.to_string()),
        ],
        RecipeError::S7AddressMisaligned { tag, address } => {
            vec![("tag", tag.to_string()), ("address", address.to_string())]
        }
        RecipeError::S7AddressWidthMismatch {
            tag,
            address,
            width,
        } => vec![
            ("tag", tag.to_string()),
            ("address", address.to_string()),
            ("width", width.to_string()),
        ],
        _handled => Vec::new(),
    }
}

/// Count the distinct tags of the items of `E-RCP-020`, as the `Display` of the error does.
///
/// An item of the version has no tag and does not count.
fn distinct_tags(items: &[SnapshotItem]) -> String {
    items
        .iter()
        .filter_map(|item| match item {
            SnapshotItem::Parameter { tag, .. } | SnapshotItem::UnknownUnit { tag, .. } => {
                Some(tag)
            }
            SnapshotItem::Version { .. } => None,
        })
        .collect::<BTreeSet<_>>()
        .len()
        .to_string()
}

/// Return the argument of an error that is a word of the UI, if the error has one.
fn word_arg(error: &RecipeError) -> Option<(&'static str, LineArg)> {
    match error {
        RecipeError::VersionNotEditable { status, .. } => {
            Some(("status", LineArg::word(status_key(*status), status)))
        }
        RecipeError::SizeBudgetExceeded { budget, .. } => {
            Some(("budget", LineArg::word(budget_key(*budget), budget)))
        }
        RecipeError::ValueShapeMismatch { expected, .. } => {
            Some(("expected", expected_word(expected)))
        }
        RecipeError::TypeNotOnPlatform { platform, .. }
        | RecipeError::S7AddressNotAllowed { platform, .. } => {
            Some(("platform", LineArg::word(platform_key(*platform), platform)))
        }
        RecipeError::RebindTargetInvalid { reason, .. } => {
            Some(("reason", LineArg::word(rebind_key(*reason), reason)))
        }
        _other => None,
    }
}

/// Return the key of the word of a version state (`design-system.md` section 9).
const fn status_key(status: VersionStatus) -> &'static str {
    match status {
        VersionStatus::Draft => "status-draft",
        VersionStatus::Approved => "status-approved",
        VersionStatus::Retired => "status-retired",
    }
}

/// Return the key of the counted thing of a size budget.
const fn budget_key(budget: Budget) -> &'static str {
    match budget {
        Budget::Parameters => "budget-parameters",
        Budget::Leaves => "budget-leaves",
        Budget::LimitRows => "budget-limit-rows",
        Budget::VersionNumbers => "budget-version-numbers",
    }
}

/// Return the key of the reason of a refused rebind.
const fn rebind_key(reason: RebindTargetReason) -> &'static str {
    match reason {
        RebindTargetReason::SourceNotStruct => "rebind-reason-source-not-struct",
        RebindTargetReason::TargetNotStruct => "rebind-reason-target-not-struct",
        RebindTargetReason::TargetNotFound => "rebind-reason-target-not-found",
        RebindTargetReason::OtherStruct => "rebind-reason-other-struct",
        RebindTargetReason::NotNewer => "rebind-reason-not-newer",
        RebindTargetReason::NotUsed => "rebind-reason-not-used",
        RebindTargetReason::PlanMismatch => "rebind-reason-plan-mismatch",
    }
}

/// Return the phrase `expected-*` of an expected form with its data (design D3).
///
/// The highest scalar of `Expected::Char` is `U+` and at least four upper-case hex digits.
fn expected_word(expected: &Expected) -> LineArg {
    let (key, args) = expected_key(expected);
    LineArg::Word {
        key: key.to_owned(),
        args,
        fallback: expected.to_string(),
    }
}

/// Return a list of one argument of a phrase.
fn one(name: &'static str, value: impl ToString) -> Vec<(&'static str, String)> {
    vec![(name, value.to_string())]
}

/// Return the key of an expected form and the arguments of its phrase.
fn expected_key(expected: &Expected) -> (&'static str, Vec<(&'static str, String)>) {
    match expected {
        Expected::Json => ("expected-json", Vec::new()),
        Expected::Depth { levels } => ("expected-depth", one("levels", levels)),
        Expected::Object { members } => {
            let names: Vec<String> = members.iter().map(ToString::to_string).collect();
            ("expected-object", one("members", names.join(", ")))
        }
        Expected::Array => ("expected-array", Vec::new()),
        Expected::ArrayCount { count } => ("expected-array-count", one("count", count)),
        Expected::Index => ("expected-index", Vec::new()),
        Expected::Scalar => ("expected-scalar", Vec::new()),
        Expected::Wstring { length } => ("expected-wstring", one("length", length)),
        Expected::String { length } => ("expected-string", one("length", length)),
        Expected::Char { max } => ("expected-char", one("max", format!("U+{max:04X}"))),
        Expected::Literal { type_name } => ("expected-literal", one("type_name", type_name)),
        Expected::Count { type_name } => ("expected-count", one("type_name", type_name)),
        Expected::Integer { type_name } => ("expected-integer", one("type_name", type_name)),
        Expected::NumberRange {
            type_name,
            low,
            high,
        } => (
            "expected-number-range",
            vec![
                ("type_name", type_name.to_string()),
                ("low", low.to_string()),
                ("high", high.to_string()),
            ],
        ),
        Expected::NumberDigits { type_name, digits } => (
            "expected-number-digits",
            vec![
                ("type_name", type_name.to_string()),
                ("digits", digits.to_string()),
            ],
        ),
        Expected::ParameterType => ("expected-parameter-type", Vec::new()),
        Expected::NewValue => ("expected-new-value", Vec::new()),
        Expected::Bool => ("expected-bool", Vec::new()),
        Expected::Decimal => ("expected-decimal", Vec::new()),
        Expected::Leaf => ("expected-leaf", Vec::new()),
        Expected::KnownLeaf => ("expected-known-leaf", Vec::new()),
    }
}
