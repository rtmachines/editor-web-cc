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
        DerivedTypeReason::ReservedName => "type-reason-reserved-name",
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

/// Pair an argument name with the text of its value.
fn kv(name: &'static str, value: &impl std::fmt::Display) -> (&'static str, String) {
    (name, value.to_string())
}

/// Return the fields of the error that are data: the name of the argument and its text.
///
/// The match has no wildcard, so a new variant of `PlantError` stops the build here.
fn data_fields(error: &PlantError) -> Vec<(&'static str, String)> {
    use PlantError as E;
    match error {
        E::UnitDepthExceeded { path } => vec![kv("path", path)],
        E::ParentNotFound { path }
        | E::DuplicateUnitPath { path }
        | E::UnitArchived { path }
        | E::UnitNotFound { path } => vec![kv("path", path)],
        E::DuplicateMachineCode { code }
        | E::MachineArchived { code }
        | E::MachineNotFound { code }
        | E::ConnectionNotConfigured { code }
        | E::DataBlockMissing { code, .. }
        | E::DataBlockNotOnPlatform { code, .. } => vec![kv("code", code)],
        E::LastMainUnit { code, path } => vec![kv("code", code), kv("path", path)],
        E::MachineHasDraft { code, reference }
        | E::ConnectionRecipeMismatch { code, reference } => {
            vec![kv("code", code), kv("reference", reference)]
        }
        E::UnitReferencedByDraft { path, reference }
        | E::UnitReferencedByApproved { path, reference } => {
            vec![kv("path", path), kv("reference", reference)]
        }
        E::TypeCycle { type_name }
        | E::DuplicateTypeName { type_name }
        | E::StructHasNoMembers { type_name } => vec![kv("type_name", type_name)],
        E::TypeTooLarge { type_name, .. } => vec![kv("type_name", type_name)],
        E::MemberTypeNotOnPlatform {
            type_name, member, ..
        }
        | E::DuplicateMemberName { type_name, member } => {
            vec![kv("type_name", type_name), kv("member", member)]
        }
        E::DerivedTypeInvalid { type_expr, .. } => vec![kv("type_expr", type_expr)],
        E::InvalidUnitPath { text } => vec![kv("text", text)],
        E::ConnectionFieldNotOnPlatform { code, field, .. }
        | E::ConnectionFieldMissing { code, field } => {
            vec![kv("code", code), kv("field", field)]
        }
        E::DataBlockOutOfRange { code, number, max } => {
            vec![kv("code", code), kv("number", number), kv("max", max)]
        }
        E::CommModeNotOnPlatform { code, mode, .. } => vec![kv("code", code), kv("mode", mode)],
        E::AddressNotAmsNetId { code, address } => vec![kv("code", code), kv("address", address)],
        E::ConnectionFieldNotInMode { code, field, mode } => {
            vec![kv("code", code), kv("field", field), kv("mode", mode)]
        }
    }
}

/// Return the arguments of a plant error: its data, and then the words of the UI.
pub(super) fn plant_line_args(error: &PlantError) -> Vec<(&'static str, LineArg)> {
    let mut args: Vec<(&'static str, LineArg)> = data_fields(error)
        .into_iter()
        .map(|(name, value)| (name, LineArg::Data(value)))
        .collect();
    match error {
        PlantError::MemberTypeNotOnPlatform { platform, .. }
        | PlantError::ConnectionFieldNotOnPlatform { platform, .. }
        | PlantError::CommModeNotOnPlatform { platform, .. }
        | PlantError::DataBlockMissing { platform, .. }
        | PlantError::DataBlockNotOnPlatform { platform, .. } => {
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
