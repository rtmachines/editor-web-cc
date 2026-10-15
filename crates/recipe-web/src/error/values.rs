//! The values that an error message shows, in the UI language (design D13).
//!
//! Each value is a key of the bundles. A key that the bundle does not hold gives the English
//! `Display` of the value, so the sentence never has a hole.

use fluent::FluentArgs;
use recipe_app::auth::Operation;
use recipe_app::error::{Missing, TaskStage};
use recipe_domain::identity::PasswordReason;

use super::reason::FieldReason;
use crate::i18n::{Bundles, Lang};

/// Return the text of `key` in `lang`, or `fallback` when the bundle has no such message.
pub(super) fn text_or(
    bundles: &Bundles,
    lang: Lang,
    key: &str,
    args: Option<&FluentArgs<'_>>,
    fallback: String,
) -> String {
    bundles.message(lang, key, args).unwrap_or(fallback)
}

/// Return the key of the message of a missing entity.
pub(super) const fn missing_key(missing: &Missing) -> &'static str {
    match missing {
        Missing::Machine(_) => "missing-machine",
        Missing::MachineId(_) => "missing-machine-id",
        Missing::Unit { .. } => "missing-unit",
        Missing::Type { .. } => "missing-type",
        Missing::Recipe(_) => "missing-recipe",
        Missing::Version { .. } => "missing-version",
        Missing::User(_) => "missing-user",
        Missing::MalformedLogin => "missing-malformed-login",
        Missing::MalformedMachine => "missing-malformed-machine",
        Missing::MalformedType => "missing-malformed-type",
        Missing::TypeRevision { .. } => "missing-type-revision",
    }
}

/// Return the data of a missing entity as arguments of its message. A number goes in as text.
fn missing_args(missing: &Missing) -> FluentArgs<'static> {
    let mut args = FluentArgs::new();
    match missing {
        Missing::Machine(code) => args.set("code", code.to_string()),
        Missing::MachineId(id) => args.set("id", id.to_string()),
        Missing::Unit { code, path } => {
            args.set("code", code.to_string());
            args.set("path", path.to_string());
        }
        Missing::Type { code, name } => {
            args.set("code", code.to_string());
            args.set("name", name.to_string());
        }
        Missing::Recipe(reference) => args.set("reference", reference.to_string()),
        Missing::Version { reference, number } => {
            args.set("reference", reference.to_string());
            args.set("number", number.to_string());
        }
        Missing::User(login) => args.set("login", login.to_string()),
        Missing::MalformedLogin | Missing::MalformedMachine | Missing::MalformedType => {}
        Missing::TypeRevision {
            code,
            name,
            revision,
        } => {
            args.set("code", code.to_string());
            args.set("name", name.to_string());
            args.set("revision", revision.to_string());
        }
    }
    args
}

/// Return the text of `{entity}` in `E-WEB-005`.
pub(super) fn missing_text(bundles: &Bundles, lang: Lang, missing: &Missing) -> String {
    let args = missing_args(missing);
    text_or(
        bundles,
        lang,
        missing_key(missing),
        Some(&args),
        missing.to_string(),
    )
}

/// Return the key of the message of a password reason.
pub(super) const fn password_reason_key(reason: PasswordReason) -> &'static str {
    match reason {
        PasswordReason::Empty => "password-reason-empty",
        PasswordReason::TooShort => "password-reason-too-short",
        PasswordReason::TooLong => "password-reason-too-long",
        PasswordReason::Breached => "password-reason-breached",
    }
}

/// Return the text of `{reason}` in `E-IDN-006`.
pub(super) fn password_reason_text(
    bundles: &Bundles,
    lang: Lang,
    reason: PasswordReason,
) -> String {
    text_or(
        bundles,
        lang,
        password_reason_key(reason),
        None,
        reason.to_string(),
    )
}

/// Return the key of the message of a task stage.
pub(super) const fn task_stage_key(stage: TaskStage) -> &'static str {
    match stage {
        TaskStage::PasswordHash => "task-stage-password-hash",
        TaskStage::Validation => "task-stage-validation",
        TaskStage::Export => "task-stage-export",
        TaskStage::Token => "task-stage-token",
    }
}

/// Return the text of `{stage}` in `E-WEB-011`.
pub(super) fn task_stage_text(bundles: &Bundles, lang: Lang, stage: TaskStage) -> String {
    text_or(
        bundles,
        lang,
        task_stage_key(stage),
        None,
        stage.to_string(),
    )
}

/// Return the key of an operation: `operation-` and its name in kebab case.
pub(super) fn operation_key(operation: Operation) -> String {
    let mut key = String::from("operation-");
    for (index, c) in operation.name().chars().enumerate() {
        if index > 0 && c.is_ascii_uppercase() {
            key.push('-');
        }
        key.push(c.to_ascii_lowercase());
    }
    key
}

/// Return the text of `{operation}` in `E-WEB-002`.
pub(super) fn operation_text(bundles: &Bundles, lang: Lang, operation: Operation) -> String {
    text_or(
        bundles,
        lang,
        &operation_key(operation),
        None,
        operation.name().to_owned(),
    )
}

/// Return the text of `{reason}` in `E-WEB-008`.
pub(super) fn field_reason_text(bundles: &Bundles, lang: Lang, reason: FieldReason) -> String {
    text_or(bundles, lang, reason.key(), None, reason.to_string())
}
