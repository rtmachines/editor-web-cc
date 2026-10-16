//! The arguments of the messages of the plant errors (design D20 U1, `reliability.md` 1.2).
//!
//! A code, a path, a name, a reference, and a typed text are data and stay as they are. The PLC
//! platform, the reason of a type, and the size limit are words of the UI. Each one is a key of
//! the bundles, so a Polish page shows a Polish sentence with no hole.

use fluent::FluentArgs;
use recipe_domain::plant::limits::{MAX_DEPTH, MAX_LEAVES, MAX_MEMBERS};
use recipe_domain::plant::{DerivedTypeReason, PlantError, TypeLimit};

use super::LineArg;
use crate::i18n::{Bundles, Lang};
use crate::pages::machines::platform_key;

/// Return the key of the message of the reason that a type is not valid.
const fn reason_key(reason: DerivedTypeReason) -> &'static str {
    match reason {
        DerivedTypeReason::Grammar => "type-reason-grammar",
        DerivedTypeReason::UnknownStruct => "type-reason-unknown-struct",
        DerivedTypeReason::NotOnPlatform => "type-reason-not-on-platform",
        DerivedTypeReason::OutOfRange => "type-reason-out-of-range",
        DerivedTypeReason::RowShape => "type-reason-row-shape",
    }
}

/// Return the word `{limit}` of `E-PLT-018`, for example "1024 members".
///
/// The number goes in as text from the constants of `reliability.md` section 7.11, so the bundle
/// adds no group separator.
fn limit_word(limit: TypeLimit) -> LineArg {
    let (key, count) = match limit {
        TypeLimit::Leaves => ("type-limit-leaves", MAX_LEAVES.to_string()),
        TypeLimit::Depth => ("type-limit-depth", MAX_DEPTH.to_string()),
        TypeLimit::Members => ("type-limit-members", MAX_MEMBERS.to_string()),
    };
    LineArg::Word {
        key: key.to_owned(),
        args: vec![("count", count)],
        fallback: limit.to_string(),
    }
}

/// Return the fields of the error that are data: the name of the argument and its text.
///
/// The match has no wildcard, so a new variant of `PlantError` stops the build here.
fn data_fields(error: &PlantError) -> Vec<(&'static str, String)> {
    match error {
        PlantError::UnitDepthExceeded { path } => vec![("path", path.clone())],
        PlantError::ParentNotFound { path }
        | PlantError::DuplicateUnitPath { path }
        | PlantError::UnitArchived { path }
        | PlantError::UnitNotFound { path } => vec![("path", path.to_string())],
        PlantError::DuplicateMachineCode { code }
        | PlantError::MachineArchived { code }
        | PlantError::MachineNotFound { code } => vec![("code", code.to_string())],
        PlantError::LastMainUnit { code, path } => {
            vec![("code", code.to_string()), ("path", path.to_string())]
        }
        PlantError::MachineHasDraft { code, reference } => {
            vec![
                ("code", code.to_string()),
                ("reference", reference.to_string()),
            ]
        }
        PlantError::UnitReferencedByDraft { path, reference }
        | PlantError::UnitReferencedByApproved { path, reference } => {
            vec![
                ("path", path.to_string()),
                ("reference", reference.to_string()),
            ]
        }
        PlantError::TypeCycle { type_name }
        | PlantError::DuplicateTypeName { type_name }
        | PlantError::StructHasNoMembers { type_name } => {
            vec![("type_name", type_name.to_string())]
        }
        PlantError::MemberTypeNotOnPlatform {
            type_name, member, ..
        }
        | PlantError::DuplicateMemberName { type_name, member } => {
            vec![
                ("type_name", type_name.to_string()),
                ("member", member.to_string()),
            ]
        }
        PlantError::TypeTooLarge { type_name, .. } => vec![("type_name", type_name.clone())],
        PlantError::DerivedTypeInvalid { type_expr, .. } => vec![("type_expr", type_expr.clone())],
        PlantError::InvalidUnitPath { text } => vec![("text", text.clone())],
    }
}

/// Return the arguments of a plant error: its data, and then the words of the UI.
pub(super) fn plant_line_args(error: &PlantError) -> Vec<(&'static str, LineArg)> {
    let mut args: Vec<(&'static str, LineArg)> = data_fields(error)
        .into_iter()
        .map(|(name, value)| (name, LineArg::Data(value)))
        .collect();
    match error {
        PlantError::MemberTypeNotOnPlatform { platform, .. } => {
            args.push(("platform", LineArg::word(platform_key(*platform), platform)));
        }
        PlantError::TypeTooLarge { limit, .. } => args.push(("limit", limit_word(*limit))),
        PlantError::DerivedTypeInvalid { reason, .. } => {
            args.push(("reason", LineArg::word(reason_key(*reason), reason)));
        }
        _other => {}
    }
    args
}

/// Add the arguments of a plant error in the UI language: its data, and then its words.
pub(super) fn plant_args(
    args: &mut FluentArgs<'static>,
    error: &PlantError,
    bundles: &Bundles,
    lang: Lang,
) {
    for (name, arg) in plant_line_args(error) {
        args.set(name, arg.text(bundles, lang));
    }
}
