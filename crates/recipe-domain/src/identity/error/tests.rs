//! Registry test of task 2.1: the `E-IDN` rows of `reliability.md` section 1.2 match the
//! variants of `IdentityError`, their display texts, and `REGISTRY`.

use super::*;
use crate::error_codes::REGISTRY;

/// The number of `E-IDN` rows: `E-IDN-001..010`.
const DOC_ROW_COUNT: usize = 10;

/// One `E-IDN` row of `reliability.md` section 1.2 that an owner `IdentityError::` names.
struct DocRow {
    code: String,
    variant: String,
    message: String,
}

/// One sample of a variant, its code text, and the display text of each of its fields.
type Sample = (IdentityError, &'static str, Vec<(&'static str, String)>);

/// Return the message of a table cell without its trailing note in parentheses.
fn message_of(cell: &str) -> &str {
    let text = cell.trim();
    match text.strip_suffix(')').and_then(|head| head.rfind(" (")) {
        Some(index) => text.get(..index).unwrap_or(text),
        None => text,
    }
}

/// Return the `E-IDN` rows of `reliability.md` section 1.2 in document order.
fn doc_rows() -> Vec<DocRow> {
    const DOC: &str = include_str!("../../../../../docs/architecture/reliability.md");
    DOC.lines()
        .filter(|line| line.starts_with("| `E-IDN-"))
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let code = cells.get(1)?.trim_matches('`');
            let owner = cells.get(2)?.trim_matches('`');
            let variant = owner
                .strip_prefix("IdentityError::")?
                .split([' ', '{', '('])
                .next()?;
            let message = message_of(cells.get(4)?);
            Some(DocRow {
                code: code.to_owned(),
                variant: variant.to_owned(),
                message: message.to_owned(),
            })
        })
        .collect()
}

/// Replace each `{field}` of a template with the text of its value.
fn fill(template: &str, fields: &[(&str, String)]) -> String {
    fields
        .iter()
        .fold(template.to_owned(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}

/// Return the name of the variant of an error, from its `Debug` text.
fn variant_name(error: &IdentityError) -> String {
    format!("{error:?}")
        .split([' ', '{', '('])
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn anna() -> Login {
    Login::parse("anna").unwrap()
}

/// One sample for each variant, in the order of the rows.
fn samples() -> Vec<Sample> {
    vec![
        (
            IdentityError::DuplicateLogin { login: anna() },
            "E-IDN-001",
            vec![("login", "anna".to_owned())],
        ),
        (
            IdentityError::LastAdministrator { login: anna() },
            "E-IDN-002",
            vec![("login", "anna".to_owned())],
        ),
        (
            IdentityError::InvalidCredentials {
                cause: CredentialFailure::WrongPassword,
            },
            "E-IDN-003",
            vec![],
        ),
        (IdentityError::UserInactive, "E-IDN-004", vec![]),
        (
            IdentityError::LoginThrottled { retry_after: 7 },
            "E-IDN-005",
            vec![("retry_after", "7".to_owned())],
        ),
        (
            IdentityError::PasswordPolicy {
                reason: PasswordReason::Breached,
            },
            "E-IDN-006",
            vec![("reason", PasswordReason::Breached.to_string())],
        ),
        (IdentityError::MustChangePassword, "E-IDN-007", vec![]),
        (IdentityError::InvalidCurrentPassword, "E-IDN-008", vec![]),
        (
            IdentityError::UserAlreadyActive { login: anna() },
            "E-IDN-009",
            vec![("login", "anna".to_owned())],
        ),
        (
            IdentityError::UserAlreadyInactive { login: anna() },
            "E-IDN-010",
            vec![("login", "anna".to_owned())],
        ),
    ]
}

#[test]
fn the_doc_has_one_row_for_each_variant() {
    assert_eq!(doc_rows().len(), DOC_ROW_COUNT);
    assert_eq!(samples().len(), DOC_ROW_COUNT);
}

#[test]
fn each_sample_matches_its_doc_row_by_code_and_variant() {
    for (row, (error, code, _)) in doc_rows().iter().zip(samples()) {
        assert_eq!(row.code, code);
        assert_eq!(row.variant, variant_name(&error), "{}", row.code);
        assert_eq!(error.code().as_str(), row.code);
    }
}

#[test]
fn display_text_equals_the_message_of_the_doc_row() {
    // Rows 004 and 007 hold a note and not a message. Two tests below cover them.
    let skipped = ["E-IDN-004", "E-IDN-007"];
    for (row, (error, _, fields)) in doc_rows().iter().zip(samples()) {
        if skipped.contains(&row.code.as_str()) {
            continue;
        }
        assert_eq!(
            error.to_string(),
            fill(&row.message, &fields),
            "{}",
            row.code
        );
    }
}

#[test]
fn every_idn_row_and_the_guard_row_are_in_the_registry() {
    for row in doc_rows() {
        let code = ErrorCode::find(&row.code).unwrap();
        assert!(REGISTRY.contains(&code), "{}", row.code);
    }
    assert_eq!(ErrorCode::find("E-AUD-003"), Some(ErrorCode::AUD_003));
    assert_eq!(ErrorCode::find("E-IDN-010"), Some(ErrorCode::IDN_010));
}

#[test]
fn user_inactive_shows_the_message_of_invalid_credentials() {
    let inactive = IdentityError::UserInactive;
    let invalid = IdentityError::InvalidCredentials {
        cause: CredentialFailure::UnknownLogin,
    };
    assert_eq!(inactive.to_string(), invalid.to_string());
    assert_eq!(inactive.code(), ErrorCode::IDN_004);
}

#[test]
fn must_change_password_displays_a_sentence_and_has_its_code() {
    let error = IdentityError::MustChangePassword;
    assert_eq!(error.code(), ErrorCode::IDN_007);
    assert_eq!(error.to_string(), "Change your password first.");
}

#[test]
fn the_cause_of_invalid_credentials_does_not_change_the_message_or_the_code() {
    let unknown = IdentityError::InvalidCredentials {
        cause: CredentialFailure::UnknownLogin,
    };
    let wrong = IdentityError::InvalidCredentials {
        cause: CredentialFailure::WrongPassword,
    };
    assert_eq!(unknown.to_string(), "Login or password is not correct.");
    assert_eq!(unknown.to_string(), wrong.to_string());
    assert_eq!(unknown.code(), ErrorCode::IDN_003);
    assert_eq!(wrong.code(), ErrorCode::IDN_003);
    assert_ne!(unknown, wrong);
}

#[test]
fn the_causes_have_distinct_names_for_the_event_log() {
    assert_eq!(CredentialFailure::UnknownLogin.as_str(), "unknown_login");
    assert_eq!(CredentialFailure::WrongPassword.as_str(), "wrong_password");
}

#[test]
fn password_reasons_display_a_short_phrase_each() {
    let texts: Vec<String> = [
        PasswordReason::Empty,
        PasswordReason::TooShort,
        PasswordReason::TooLong,
        PasswordReason::Breached,
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    assert_eq!(
        texts,
        [
            "it is empty",
            "it has fewer than 12 characters",
            "it has more than 128 characters",
            "it is in the breached-password list",
        ]
    );
}

#[test]
fn no_variant_carries_a_secret() {
    // Every field is a login, a number, or an enum of this module. A debug text of a sample
    // therefore holds none of the words that mark a secret.
    for (error, _, _) in samples() {
        let debug = format!("{error:?}").to_ascii_lowercase();
        for needle in ["$argon2", "hash", "token", "cookie", "secret"] {
            assert!(!debug.contains(needle), "{debug}");
        }
    }
}
