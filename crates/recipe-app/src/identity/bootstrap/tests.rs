//! Tests of `bootstrap_administrator` (task 4.6, design D17, ADR-0035 h).

use recipe_domain::audit::{AuditAction, AuditTarget, RequestId};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{IdentityError, Login, NewPassword, Role};
use uuid::Uuid;

use super::{BootstrapOutcome, bootstrap_administrator};
use crate::error::AppError;
use crate::identity::fixtures::{IdentityFixture, fake_hash, serializable_runs};
use crate::memory::{Fault, FaultPoint};
use crate::ports::UserRow;

const PASSWORD: &str = "bootstrap-secret-phrase-42";

fn admin() -> Login {
    Login::parse("admin").unwrap()
}

fn request_id() -> RequestId {
    RequestId::from(Uuid::from_u128(0x77))
}

async fn run(fixture: &IdentityFixture, login: Login) -> Result<BootstrapOutcome, AppError> {
    bootstrap_administrator(
        fixture.context.app.uow.as_ref(),
        fixture.context.gate(),
        login,
        NewPassword::parse(PASSWORD).unwrap(),
        request_id(),
    )
    .await
}

fn users(fixture: &IdentityFixture) -> Vec<UserRow> {
    fixture.store.state().users.values().cloned().collect()
}

#[tokio::test]
async fn without_an_active_administrator_it_creates_one_with_must_change_password() {
    let fixture = IdentityFixture::new();
    let outcome = run(&fixture, admin()).await.unwrap();
    assert_eq!(outcome, BootstrapOutcome::Created);
    let rows = users(&fixture);
    assert_eq!(rows.len(), 1);
    let row = rows.first().unwrap();
    assert_eq!(row.login, admin());
    assert_eq!(row.role, Role::Administrator);
    assert!(row.is_active);
    assert!(row.must_change_password);
    assert_eq!(row.password_hash, fake_hash(PASSWORD));
    assert_eq!(fixture.hasher.hash_calls(), 1);
    assert_eq!(fixture.context.gate().available_permits(), 4);
    assert_eq!(serializable_runs(&fixture.store), 1);
}

#[tokio::test]
async fn the_id_of_the_bootstrap_administrator_is_a_version_4_uuid() {
    let fixture = IdentityFixture::new();
    run(&fixture, admin()).await.unwrap();
    let rows = users(&fixture);
    let row = rows.first().unwrap();
    assert_eq!(row.id.as_uuid().get_version_num(), 4);
}

#[tokio::test]
async fn it_writes_one_user_created_entry_with_no_actor_no_address_and_no_session() {
    let fixture = IdentityFixture::new();
    run(&fixture, admin()).await.unwrap();
    let row = users(&fixture).into_iter().next().unwrap();
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::UserCreated);
    assert_eq!(entry.target(), &AuditTarget::app_user(row.id));
    assert_eq!(entry.actor(), None);
    assert_eq!(entry.client_ip(), None);
    assert_eq!(entry.session_ref(), None);
    assert_eq!(entry.request_id(), request_id());
    assert_eq!(entry.before(), None);
    let after = entry.after().unwrap();
    assert_eq!(after["login"], "admin");
    assert_eq!(after["role"], "administrator");
    assert_eq!(after["must_change_password"], true);
    let shown = format!("{entries:?}");
    assert!(!shown.contains(PASSWORD), "{shown}");
    assert!(!shown.contains("argon2id"), "{shown}");
    assert!(!shown.contains("password_hash"), "{shown}");
}

#[tokio::test]
async fn an_active_administrator_changes_nothing() {
    let fixture = IdentityFixture::new();
    let _ = fixture.actor(Role::Administrator).await;
    let before = users(&fixture);
    let outcome = run(&fixture, admin()).await.unwrap();
    assert_eq!(outcome, BootstrapOutcome::AdministratorExists);
    assert_eq!(users(&fixture), before);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn an_inactive_administrator_does_not_count() {
    let fixture = IdentityFixture::new();
    let _ = fixture.actor(Role::Administrator).await;
    fixture.store.update_state(|state, now| {
        for row in state.users.values_mut() {
            row.is_active = false;
            row.deactivated_at = Some(now);
        }
    });
    let outcome = run(&fixture, admin()).await.unwrap();
    assert_eq!(outcome, BootstrapOutcome::Created);
    assert_eq!(users(&fixture).len(), 2);
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn an_active_operator_does_not_count() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    let outcome = run(&fixture, admin()).await.unwrap();
    assert_eq!(outcome, BootstrapOutcome::Created);
}

#[tokio::test]
async fn one_serialization_failure_gives_one_more_run_and_one_hash() {
    for point in [
        FaultPoint::Begin,
        FaultPoint::AppendAudit,
        FaultPoint::Finish,
    ] {
        let fixture = IdentityFixture::new();
        fixture.store.fail_next(point, Fault::Serialization, 1);
        let outcome = run(&fixture, admin()).await.unwrap();
        assert_eq!(outcome, BootstrapOutcome::Created, "{point:?}");
        // A failed `begin` leaves no journal line, so the first run of that case is not counted.
        let journaled_runs = if point == FaultPoint::Begin { 1 } else { 2 };
        assert_eq!(
            serializable_runs(&fixture.store),
            journaled_runs,
            "{point:?}"
        );
        assert_eq!(users(&fixture).len(), 1, "{point:?}");
        assert_eq!(fixture.store.audit_entries().len(), 1, "{point:?}");
        assert_eq!(fixture.hasher.hash_calls(), 1, "{point:?}");
    }
}

#[tokio::test]
async fn two_serialization_failures_give_e_db_001_and_create_no_user() {
    let fixture = IdentityFixture::new();
    fixture
        .store
        .fail_next(FaultPoint::Finish, Fault::Serialization, 2);
    let error = run(&fixture, admin()).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert_eq!(serializable_runs(&fixture.store), 2);
    assert!(users(&fixture).is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(fixture.context.gate().available_permits(), 4);
}

#[tokio::test]
async fn a_login_in_use_by_another_role_gives_e_idn_001_and_writes_nothing() {
    let fixture = IdentityFixture::new();
    fixture.seed_anna("secret-phrase-one", false);
    let anna = Login::parse("anna").unwrap();
    let error = run(&fixture, anna.clone()).await.unwrap_err();
    assert!(
        matches!(
            &error,
            AppError::Identity(IdentityError::DuplicateLogin { login }) if login == &anna
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::IDN_001);
    assert_eq!(users(&fixture).len(), 1);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_wait_for_a_permit_that_times_out_gives_e_web_009_before_any_transaction() {
    let fixture = IdentityFixture::new();
    let mut held = Vec::new();
    for _ in 0..4 {
        held.push(fixture.context.gate().acquire().await.unwrap());
    }
    let journal_before = fixture.store.journal().len();
    let error = run(&fixture, admin()).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert_eq!(fixture.store.journal().len(), journal_before);
    drop(held);
}
