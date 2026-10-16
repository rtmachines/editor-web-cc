//! Tests of the values in error messages (task 8.2, design D13).
//!
//! Every value that a message shows as text comes from the bundle of the UI language.

use std::num::NonZeroU64;

use axum::http::StatusCode;
use recipe_app::auth::Operation;
use recipe_app::ports::AuditEntryId;
use recipe_domain::identity::Role;

use recipe_app::error::{Missing, TaskStage};
use recipe_domain::identity::{Login, PasswordReason};
use recipe_domain::plant::{MachineCode, MachineId, TypeName, UnitPath};
use recipe_domain::recipe::{ParameterKey, ReferenceNumber};
use uuid::Uuid;

use super::reason::FieldReason;
use super::values::{missing_key, operation_key, password_reason_key, task_stage_key};
use super::values::{missing_text, text_or};
use crate::i18n::{Bundles, Lang};
use crate::test_support::{Fixture, Reply, TestSession, get_with, post_fields};

/// Return one `Missing` for each variant.
fn every_missing() -> Vec<Missing> {
    let code = MachineCode::parse("M1").unwrap();
    let reference = ReferenceNumber::parse("R-7").unwrap();
    vec![
        Missing::Machine(code),
        Missing::MachineId(MachineId::from(Uuid::nil())),
        Missing::Unit {
            code,
            path: UnitPath::parse("U1.2").unwrap(),
        },
        Missing::Type {
            code,
            name: TypeName::parse("Motor").unwrap(),
        },
        Missing::Recipe(reference.clone()),
        Missing::version(&reference, 1234),
        Missing::User(Login::parse("anna").unwrap()),
        Missing::MalformedLogin,
        Missing::MalformedMachine,
        Missing::MalformedType,
        Missing::TypeRevision {
            code,
            name: TypeName::parse("ST_Zone").unwrap(),
            revision: 9,
        },
        Missing::MalformedRecipe,
        Missing::MalformedVersion,
        Missing::MalformedParameter,
        Missing::Parameter {
            reference,
            number: 1234,
            key: ParameterKey::from(Uuid::nil()),
        },
        Missing::AuditEntry(AuditEntryId::from(NonZeroU64::new(1234).unwrap())),
        Missing::MalformedAuditEntry,
    ]
}

/// The nine reasons that the plant pages add (design D5), with their keys, in the order of `ALL`.
const PLANT_REASON_KEYS: [(FieldReason, &str); 9] = [
    (FieldReason::MachineCode, "reason-machine-code"),
    (FieldReason::Name, "reason-name"),
    (FieldReason::Platform, "reason-platform"),
    (FieldReason::TypeName, "reason-type-name"),
    (FieldReason::MemberName, "reason-member-name"),
    (FieldReason::MemberRows, "reason-member-rows"),
    (FieldReason::MemberCount, "reason-member-count"),
    (FieldReason::Flag, "reason-flag"),
    (FieldReason::SearchText, "reason-search-text"),
];

/// Assert that `key` exists in both bundles.
fn assert_in_both(bundles: &Bundles, key: &str) {
    for lang in [Lang::En, Lang::Pl] {
        assert!(bundles.has_message(lang, key), "{key} in {}", lang.as_str());
    }
}

/// The number of characters that is one more than a password may have.
const TOO_LONG: usize = 129;

/// Return the key of an operation: `operation-` and the kebab case of its name.
fn kebab_key(operation: Operation) -> String {
    let mut key = String::from("operation-");
    for (index, c) in operation.name().chars().enumerate() {
        if c.is_ascii_uppercase() && index > 0 {
            key.push('-');
        }
        key.push(c.to_ascii_lowercase());
    }
    key
}

/// Return the text that a user reads: the markup is gone.
fn visible_text(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _other if !in_tag => text.push(c),
            _skipped => {}
        }
    }
    text
}

/// Send an HTMX request in the language `code` with the session.
async fn htmx_get(fixture: &Fixture, session: &TestSession, code: &str, path: &str) -> Reply {
    let cookie = format!("{}; recipe_lang={code}", session.cookie);
    let headers = [("cookie", cookie.as_str()), ("hx-request", "true")];
    get_with(&fixture.router(), path, &headers).await
}

#[test]
fn values_every_operation_has_its_key_in_both_bundles() {
    let bundles = Bundles::load().unwrap();
    for operation in Operation::ALL {
        let key = kebab_key(operation);
        assert_eq!(operation_key(operation), key);
        assert_in_both(&bundles, &key);
    }
}

#[test]
fn values_every_field_reason_has_its_key_and_a_log_text() {
    let bundles = Bundles::load().unwrap();
    for reason in FieldReason::ALL {
        assert_in_both(&bundles, reason.key());
        assert!(!reason.to_string().is_empty(), "{}", reason.key());
    }
}

#[test]
fn values_field_reason_all_lists_34_reasons_with_the_plant_reasons_at_11_to_19() {
    assert_eq!(FieldReason::ALL.len(), 34);
    let tail = FieldReason::ALL.get(11..20).unwrap();
    let expected: Vec<FieldReason> = PLANT_REASON_KEYS
        .iter()
        .map(|(reason, _)| *reason)
        .collect();
    assert_eq!(tail, expected.as_slice());
}

#[test]
fn values_each_plant_reason_has_its_key_and_the_english_text_of_its_bundle() {
    let bundles = Bundles::load().unwrap();
    for (reason, key) in PLANT_REASON_KEYS {
        assert_eq!(reason.key(), key);
        assert_in_both(&bundles, key);
        let english = bundles.message(Lang::En, key, None).unwrap();
        assert_eq!(english, reason.to_string(), "{key}");
    }
}

#[test]
fn values_every_password_reason_and_task_stage_has_its_key() {
    let bundles = Bundles::load().unwrap();
    for reason in [
        PasswordReason::Empty,
        PasswordReason::TooShort,
        PasswordReason::TooLong,
        PasswordReason::Breached,
    ] {
        assert_in_both(&bundles, password_reason_key(reason));
        let english = bundles
            .message(Lang::En, password_reason_key(reason), None)
            .unwrap();
        assert_eq!(english, reason.to_string());
    }
    for stage in [
        TaskStage::PasswordHash,
        TaskStage::Validation,
        TaskStage::Export,
        TaskStage::Token,
        TaskStage::Diff,
    ] {
        assert_in_both(&bundles, task_stage_key(stage));
        let english = bundles
            .message(Lang::En, task_stage_key(stage), None)
            .unwrap();
        assert_eq!(english, stage.to_string());
    }
}

#[test]
fn values_every_missing_variant_has_its_key_and_the_english_display() {
    let bundles = Bundles::load().unwrap();
    for missing in every_missing() {
        assert_in_both(&bundles, missing_key(&missing));
        let english = missing_text(&bundles, Lang::En, &missing);
        assert_eq!(english, missing.to_string());
    }
}

#[test]
fn values_missing_unit_names_the_machine_and_the_path_in_polish() {
    let bundles = Bundles::load().unwrap();
    let missing = every_missing().remove(2);
    assert_eq!(
        missing_text(&bundles, Lang::Pl, &missing),
        "Jednostka U1.2 maszyny M1"
    );
}

#[test]
fn values_missing_version_shows_the_number_without_a_group_separator() {
    let bundles = Bundles::load().unwrap();
    let missing = every_missing().remove(5);
    assert_eq!(
        missing_text(&bundles, Lang::Pl, &missing),
        "Wersja 1234 receptury R-7"
    );
}

#[test]
fn values_missing_malformed_values_use_their_own_keys() {
    assert_eq!(
        missing_key(&Missing::MalformedMachine),
        "missing-malformed-machine"
    );
    assert_eq!(
        missing_key(&Missing::MalformedType),
        "missing-malformed-type"
    );
    let bundles = Bundles::load().unwrap();
    assert_eq!(
        missing_text(&bundles, Lang::Pl, &Missing::MalformedMachine),
        "Maszyna"
    );
    assert_eq!(
        missing_text(&bundles, Lang::Pl, &Missing::MalformedType),
        "Typ"
    );
}

#[test]
fn values_missing_recipe_values_use_their_own_keys_and_polish_texts() {
    let bundles = Bundles::load().unwrap();
    let cases = [
        (
            Missing::MalformedRecipe,
            "missing-malformed-recipe",
            "Receptura",
        ),
        (
            Missing::MalformedVersion,
            "missing-malformed-version",
            "Wersja",
        ),
        (
            Missing::MalformedParameter,
            "missing-malformed-parameter",
            "Parametr",
        ),
    ];
    for (missing, key, polish) in cases {
        assert_eq!(missing_key(&missing), key);
        assert_eq!(missing_text(&bundles, Lang::Pl, &missing), polish);
    }
    let parameter = every_missing().remove(14);
    assert_eq!(missing_key(&parameter), "missing-parameter");
    assert_eq!(
        missing_text(&bundles, Lang::Pl, &parameter),
        "Parametr 00000000-0000-0000-0000-000000000000 wersji 1234 receptury R-7"
    );
}

#[test]
fn values_missing_audit_entry_names_the_full_id_in_polish_and_a_malformed_id_names_none() {
    let bundles = Bundles::load().unwrap();
    let missing = every_missing().remove(15);
    assert_eq!(missing_key(&missing), "missing-audit-entry");
    assert_eq!(
        missing_text(&bundles, Lang::Pl, &missing),
        "Wpis audytu 1234"
    );
    assert_eq!(
        missing_key(&Missing::MalformedAuditEntry),
        "missing-malformed-audit-entry"
    );
    assert_eq!(
        missing_text(&bundles, Lang::Pl, &Missing::MalformedAuditEntry),
        "Wpis audytu"
    );
}

#[test]
fn values_missing_type_revision_names_revision_type_and_machine_in_polish() {
    let bundles = Bundles::load().unwrap();
    let missing = Missing::TypeRevision {
        code: MachineCode::parse("M1").unwrap(),
        name: TypeName::parse("ST_Zone").unwrap(),
        revision: 1234,
    };
    assert_eq!(missing_key(&missing), "missing-type-revision");
    assert_eq!(
        missing_text(&bundles, Lang::Pl, &missing),
        "Rewizja 1234 typu ST_Zone maszyny M1"
    );
    assert_eq!(
        missing_text(&bundles, Lang::En, &missing),
        "Revision 1234 of type ST_Zone of machine M1"
    );
}

#[test]
fn values_a_missing_key_falls_back_to_the_given_text() {
    let bundles = Bundles::load().unwrap();
    let shown = text_or(
        &bundles,
        Lang::Pl,
        "no-such-key",
        None,
        "fallback".to_owned(),
    );
    assert_eq!(shown, "fallback");
}

#[tokio::test]
async fn values_missing_user_is_polish_on_a_polish_page() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(1, Role::Administrator, false);
    let reply = htmx_get(&fixture, &session, "pl", "/users/nobody").await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND, "{}", reply.body);
    let text = visible_text(&reply.body);
    assert!(text.contains("Użytkownik nobody"), "{text}");
    assert!(!text.contains("User"), "{text}");
}

#[tokio::test]
async fn values_password_reason_is_polish_on_a_polish_page() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);
    let cookie = format!("{}; recipe_lang=pl", session.cookie);
    let headers = [("cookie", cookie.as_str()), ("hx-request", "true")];
    let body = format!("current_password={}&password=x", "a".repeat(TOO_LONG));
    let reply = post_fields(
        &fixture.router(),
        "/me/password",
        Some(&session),
        &headers,
        &body,
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let text = visible_text(&reply.body);
    assert!(text.contains("hasło ma od 1 do 128 znaków"), "{text}");
}

#[tokio::test]
async fn values_forbidden_operation_is_named_in_polish() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);
    let reply = htmx_get(&fixture, &session, "pl", "/users").await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    let text = visible_text(&reply.body);
    assert!(text.contains("Lista użytkowników"), "{text}");
}
