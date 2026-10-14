//! Tests of the use case `UnlockLogin` (task 4.5, design D8 i, j, and q).

use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::{AuditAction, AuditTarget, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{Login, Role, TypedLogin};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::UnlockLogin;
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing, StorageError};
use crate::identity::fixtures::{IdentityFixture, actor_id, anna_id};
use crate::memory::Fault;

/// The start of the clock of a fixture, in Unix seconds.
const START_SECONDS: i64 = 1_000_000;

fn admin_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 9))
}

fn failing_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))
}

fn anna() -> Login {
    Login::parse("anna").unwrap()
}

fn typed(text: &str) -> TypedLogin {
    TypedLogin::parse(text).unwrap()
}

fn at(offset: i64) -> Timestamp {
    let seconds = START_SECONDS.checked_add(offset).unwrap();
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

fn fixture() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    fixture
}

/// Seed `count` failed attempts of `login`, one second apart. The newest has the highest key.
fn seed_failures(fixture: &IdentityFixture, login: &str, count: i64) {
    for offset in 0..count {
        fixture
            .store
            .seed_attempt(&typed(login), failing_ip(), at(offset), false);
    }
}

fn attempts_of(fixture: &IdentityFixture, login: &str) -> usize {
    let login = typed(login);
    fixture
        .store
        .state()
        .attempts
        .values()
        .filter(|row| row.login == login)
        .count()
}

#[test]
fn the_operation_is_unlock_login() {
    assert_eq!(<UnlockLogin as UseCase>::OPERATION, Operation::UnlockLogin);
}

#[tokio::test]
async fn a_success_deletes_every_attempt_of_the_login_and_keeps_the_others() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 5);
    seed_failures(&fixture, "berta", 2);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    assert_eq!(attempts_of(&fixture, "anna"), 0);
    assert_eq!(attempts_of(&fixture, "berta"), 2);
}

#[tokio::test]
async fn the_entry_login_unlocked_holds_the_deleted_attempts_and_an_empty_array() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 2);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::LoginUnlocked);
    assert_eq!(entry.target(), &AuditTarget::app_user(anna_id()));
    assert_eq!(entry.actor(), Some(actor_id()));
    let before = entry.before().unwrap();
    assert_eq!(before["login"], "anna");
    assert_eq!(before["login_attempt_count"], 2);
    let listed = before["login_attempts"].as_array().unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed.first().unwrap()["client_ip"], "10.0.0.5");
    assert_eq!(listed.first().unwrap()["succeeded"], false);
    let after = entry.after().unwrap();
    assert_eq!(after["login"], "anna");
    assert_eq!(after["login_attempt_count"], 0);
    assert!(after["login_attempts"].as_array().unwrap().is_empty());
    let shown = format!("{entries:?}");
    assert!(!shown.contains("password_hash"), "{shown}");
    assert!(!shown.contains("argon2id"), "{shown}");
}

#[tokio::test]
async fn the_attempted_at_of_each_listed_attempt_is_rfc_3339_text() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 3);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    let before = entries.first().unwrap().before().unwrap();
    let listed = before["login_attempts"].as_array().unwrap();
    assert_eq!(listed.len(), 3);
    for attempt in listed {
        let text = attempt["attempted_at"].as_str().unwrap();
        assert!(
            OffsetDateTime::parse(text, &Rfc3339).is_ok(),
            "{text:?} is no RFC 3339 text"
        );
    }
}

#[tokio::test]
async fn the_image_holds_the_newest_100_of_250_attempts_and_the_full_count() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 250);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    let before = entries.first().unwrap().before().unwrap();
    assert_eq!(before["login_attempt_count"], 250);
    let listed = before["login_attempts"].as_array().unwrap();
    assert_eq!(listed.len(), 100);
    assert_eq!(listed.first().unwrap()["id"], 250);
    assert_eq!(listed.last().unwrap()["id"], 151);
    assert_eq!(attempts_of(&fixture, "anna"), 0);
}

#[tokio::test]
async fn the_event_unlock_names_the_target_the_actor_and_the_address() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 1);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    let event = events.first().unwrap();
    assert_eq!(event.kind(), SecurityEventKind::Unlock);
    assert_eq!(event.user(), Some(anna_id()));
    assert_eq!(event.client_ip(), Some(admin_ip()));
    assert_eq!(event.request_id(), meta.request_id());
    assert_eq!(
        event.detail().unwrap()["unlocked_by"],
        actor_id().as_uuid().to_string()
    );
}

#[tokio::test]
async fn a_repeated_unlock_deletes_nothing_more_and_commits_a_count_of_0() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 3);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let unlock = UnlockLogin::new(fixture.context.clone());
    unlock
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    unlock
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 2);
    let second = entries.last().unwrap().before().unwrap();
    assert_eq!(second["login_attempt_count"], 0);
}

#[tokio::test]
async fn an_unknown_login_gives_e_web_005_and_writes_nothing() {
    let fixture = fixture();
    seed_failures(&fixture, "nobody", 2);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    let nobody = Login::parse("nobody").unwrap();
    let error = UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &nobody, admin_ip())
        .await
        .unwrap_err();
    assert!(
        matches!(
            &error,
            AppError::NotFound { entity: Missing::User(login) } if login == &nobody
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert_eq!(attempts_of(&fixture, "nobody"), 2);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn the_permission_check_comes_first_for_an_operator_and_a_process_engineer() {
    for role in [Role::Operator, Role::ProcessEngineer] {
        for login in ["anna", "nobody"] {
            let fixture = fixture();
            seed_failures(&fixture, "anna", 3);
            let (auth, meta) = fixture.actor(role).await;
            let journal_before = fixture.store.journal().len();
            let error = UnlockLogin::new(fixture.context.clone())
                .execute(&auth, &meta, &Login::parse(login).unwrap(), admin_ip())
                .await
                .unwrap_err();
            assert!(
                matches!(
                    error,
                    AppError::Forbidden {
                        operation: Operation::UnlockLogin
                    }
                ),
                "{error:?}"
            );
            assert_eq!(error.code(), ErrorCode::WEB_002);
            let events = fixture.sink.events();
            assert_eq!(events.len(), 1, "{role:?}");
            assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
            assert_eq!(fixture.store.journal().len(), journal_before, "{role:?}");
            assert_eq!(attempts_of(&fixture, "anna"), 3, "{role:?}");
        }
    }
}

#[tokio::test]
async fn a_failed_unlock_event_gives_the_storage_error_and_the_unlock_stays_committed() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 4);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture.sink.fail_with(Fault::Unavailable);
    let error = UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap_err();
    assert!(
        matches!(error, AppError::Storage(StorageError::Unavailable)),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert_eq!(attempts_of(&fixture, "anna"), 0);
    assert_eq!(fixture.store.audit_entries().len(), 1);
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn a_failed_commit_deletes_nothing_and_writes_no_event() {
    let fixture = fixture();
    seed_failures(&fixture, "anna", 4);
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    fixture.store.fail_finish(Fault::Unavailable);
    let error = UnlockLogin::new(fixture.context.clone())
        .execute(&auth, &meta, &anna(), admin_ip())
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_005);
    assert_eq!(attempts_of(&fixture, "anna"), 4);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.sink.events().is_empty());
}
