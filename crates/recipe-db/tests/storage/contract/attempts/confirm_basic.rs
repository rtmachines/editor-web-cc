//! Contract cases of `LoginAttemptStore::confirm_basic`, T2 of the Basic check (ADR-0109 (m)).
//!
//! Each case seeds an `ExportClient` and one failed attempt. T1 reads the user. Then the case
//! commits a change of the user on another transaction, as a concurrent command would do.
//! The cases with the prefix `postgres_only` need a real row lock.

use std::time::Duration;

use recipe_app::ports::{AttemptId, BasicChange, BasicConfirmation, NonEmpty, Transaction};
use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
};
use recipe_domain::identity::{PasswordHash, Role, UserId};
use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;

use super::{FIRST, rows_of, typed};
use crate::contract::harness::{FIXTURE_PASSWORD_HASH, Harness, begin, new_user, seed_user};
use crate::contract::postgres_harness::PgHarness;
use crate::support::Role as DbRole;

/// The login of the user and of the attempt.
const LOGIN: &str = "exporter";
/// The pause between two reads of `pg_stat_activity`.
const POLL_PAUSE: Duration = Duration::from_millis(20);
/// The number of reads before the case gives up. The bound is about 5 s.
const POLL_LIMIT: u32 = 250;

/// The statements that wait for a row lock, seen from a second session.
const WAITING_LOCKS: &str = "\
SELECT count(*) FROM pg_stat_activity \
WHERE datname = current_database() AND wait_event_type = 'Lock' \
    AND query LIKE '%FOR NO KEY UPDATE%'";

fn original_hash() -> PasswordHash {
    PasswordHash::parse(FIXTURE_PASSWORD_HASH).unwrap()
}

fn changed_hash() -> PasswordHash {
    PasswordHash::parse("$argon2id$changed").unwrap()
}

/// Seed an `ExportClient` and one failed attempt of the same login.
async fn seeded(h: &dyn Harness) -> (UserId, AttemptId) {
    let user = new_user(LOGIN, Role::ExportClient);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    let attempts = h.seed_attempts(&typed(LOGIN), FIRST, &[5], false).await;
    (user.id, attempts[0])
}

fn entry(action: RowAction, user: UserId) -> NonEmpty<AuditEntry> {
    NonEmpty::new(AuditEntry::row(
        action,
        AuditTarget::app_user(user),
        RowChange::Update {
            before: serde_json::json!({}),
            after: serde_json::json!({}),
        },
        AuditContext {
            actor: None,
            request_id: RequestId::from(Uuid::new_v4()),
            client_ip: None,
            session_ref: None,
        },
    ))
}

/// The change that a concurrent command commits.
///
/// `PasswordChange` stands for the reset of a password by an Administrator. The reset of an
/// `ExportClient` keeps `must_change_password` false, as the `CHECK` of that role demands.
#[derive(Clone, Copy)]
enum Concurrent {
    PasswordChange,
    Deactivation,
}

impl Concurrent {
    const fn action(self) -> RowAction {
        match self {
            Self::PasswordChange => RowAction::PasswordReset,
            Self::Deactivation => RowAction::UserDeactivated,
        }
    }

    const fn reason(self) -> BasicChange {
        match self {
            Self::PasswordChange => BasicChange::PasswordChanged,
            Self::Deactivation => BasicChange::Inactive,
        }
    }
}

/// Lock the user and apply the change in a transaction. The caller commits.
async fn open_change(h: &dyn Harness, change: Concurrent) -> Box<dyn Transaction> {
    let mut tx = begin(h).await;
    let locked = tx
        .users()
        .lock_by_login(&new_user(LOGIN, Role::ExportClient).login)
        .await
        .unwrap()
        .unwrap();
    let id = locked.state.id;
    match change {
        Concurrent::PasswordChange => {
            tx.users()
                .reset_password(id, &changed_hash(), false)
                .await
                .unwrap();
        }
        Concurrent::Deactivation => {
            tx.users().deactivate(id).await.unwrap();
        }
    }
    tx
}

async fn commit_change(h: &dyn Harness, user: UserId, change: Concurrent) {
    let tx = open_change(h, change).await;
    h.unit_of_work()
        .commit(tx, entry(change.action(), user))
        .await
        .unwrap();
}

async fn a_success_sets_succeeded(h: &dyn Harness) {
    let (user, attempt) = seeded(h).await;

    let answer = h
        .login_attempts()
        .confirm_basic(attempt, user, &original_hash())
        .await
        .unwrap();

    assert_eq!(answer, BasicConfirmation::Confirmed);
    let rows = rows_of(h, LOGIN).await;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].succeeded);
}

async fn a_password_change_during_the_hash_check_gives_changed(h: &dyn Harness) {
    let (user, attempt) = seeded(h).await;
    commit_change(h, user, Concurrent::PasswordChange).await;

    let answer = h
        .login_attempts()
        .confirm_basic(attempt, user, &original_hash())
        .await
        .unwrap();

    assert_eq!(
        answer,
        BasicConfirmation::Changed {
            reason: BasicChange::PasswordChanged
        }
    );
    assert!(!rows_of(h, LOGIN).await[0].succeeded);
}

async fn a_deactivation_during_the_hash_check_gives_changed(h: &dyn Harness) {
    let (user, attempt) = seeded(h).await;
    commit_change(h, user, Concurrent::Deactivation).await;

    let answer = h
        .login_attempts()
        .confirm_basic(attempt, user, &original_hash())
        .await
        .unwrap();

    assert_eq!(
        answer,
        BasicConfirmation::Changed {
            reason: BasicChange::Inactive
        }
    );
    assert!(!rows_of(h, LOGIN).await[0].succeeded);
}

async fn a_role_change_gives_not_export_client(h: &dyn Harness) {
    let (user, attempt) = seeded(h).await;
    let mut tx = begin(h).await;
    tx.users()
        .lock_by_login(&new_user(LOGIN, Role::ExportClient).login)
        .await
        .unwrap();
    tx.users().set_role(user, Role::Operator).await.unwrap();
    h.unit_of_work()
        .commit(tx, entry(RowAction::UserRoleSet, user))
        .await
        .unwrap();

    let answer = h
        .login_attempts()
        .confirm_basic(attempt, user, &original_hash())
        .await
        .unwrap();

    assert_eq!(
        answer,
        BasicConfirmation::Changed {
            reason: BasicChange::NotExportClient
        }
    );
    assert!(!rows_of(h, LOGIN).await[0].succeeded);
}

async fn an_unknown_user_gives_missing(h: &dyn Harness) {
    let (_, attempt) = seeded(h).await;

    let answer = h
        .login_attempts()
        .confirm_basic(attempt, UserId::from(Uuid::new_v4()), &original_hash())
        .await
        .unwrap();

    assert_eq!(
        answer,
        BasicConfirmation::Changed {
            reason: BasicChange::Missing
        }
    );
    assert!(!rows_of(h, LOGIN).await[0].succeeded);
}

async fn an_attempt_that_an_unlock_deleted_does_not_fail_the_check(h: &dyn Harness) {
    let (user, attempt) = seeded(h).await;
    let mut tx = begin(h).await;
    tx.attempts()
        .delete_for_login(&typed(LOGIN), 10)
        .await
        .unwrap();
    h.unit_of_work()
        .commit(tx, entry(RowAction::LoginUnlocked, user))
        .await
        .unwrap();

    let answer = h
        .login_attempts()
        .confirm_basic(attempt, user, &original_hash())
        .await
        .unwrap();

    assert_eq!(answer, BasicConfirmation::Confirmed);
    assert!(rows_of(h, LOGIN).await.is_empty());
}

/// Poll until one statement waits for a row lock, and fail after the bound.
async fn wait_for_blocked_check(observer: &mut PgConnection) {
    let mut last = 0_i64;
    for _ in 0..POLL_LIMIT {
        last = sqlx::query_scalar(WAITING_LOCKS)
            .fetch_one(&mut *observer)
            .await
            .unwrap();
        if last == 1 {
            return;
        }
        tokio::time::sleep(POLL_PAUSE).await;
    }
    assert_eq!(last, 1, "statements that wait for the user row");
}

/// While a command holds the user row, T2 waits for it. After the commit, T2 sees the change.
async fn blocked_check_gives_changed(change: Concurrent) {
    let h = PgHarness::create().await;
    let (user, attempt) = seeded(&h).await;
    let mut observer = h.database.connect(DbRole::Superuser).await;
    let tx = open_change(&h, change).await;
    let store = h.login_attempts();
    let task =
        tokio::spawn(async move { store.confirm_basic(attempt, user, &original_hash()).await });

    wait_for_blocked_check(&mut observer).await;
    assert!(!task.is_finished());
    h.unit_of_work()
        .commit(tx, entry(change.action(), user))
        .await
        .unwrap();
    let answer = task.await.unwrap().unwrap();

    assert_eq!(
        answer,
        BasicConfirmation::Changed {
            reason: change.reason()
        }
    );
    assert!(!rows_of(&h, LOGIN).await[0].succeeded);
    observer.close().await.unwrap();
}

#[tokio::test]
async fn postgres_only_confirm_basic_waits_for_a_password_change_and_gives_changed() {
    blocked_check_gives_changed(Concurrent::PasswordChange).await;
}

#[tokio::test]
async fn postgres_only_confirm_basic_waits_for_a_deactivation_and_gives_changed() {
    blocked_check_gives_changed(Concurrent::Deactivation).await;
}

contract_case!(
    a_success_sets_succeeded,
    confirm_basic_a_success_sets_succeeded_memory,
    confirm_basic_a_success_sets_succeeded_postgres
);
contract_case!(
    a_password_change_during_the_hash_check_gives_changed,
    confirm_basic_a_password_change_gives_changed_memory,
    confirm_basic_a_password_change_gives_changed_postgres
);
contract_case!(
    a_deactivation_during_the_hash_check_gives_changed,
    confirm_basic_a_deactivation_gives_changed_memory,
    confirm_basic_a_deactivation_gives_changed_postgres
);
contract_case!(
    a_role_change_gives_not_export_client,
    confirm_basic_a_role_change_gives_not_export_client_memory,
    confirm_basic_a_role_change_gives_not_export_client_postgres
);
contract_case!(
    an_unknown_user_gives_missing,
    confirm_basic_an_unknown_user_gives_missing_memory,
    confirm_basic_an_unknown_user_gives_missing_postgres
);
contract_case!(
    an_attempt_that_an_unlock_deleted_does_not_fail_the_check,
    confirm_basic_a_deleted_attempt_does_not_fail_memory,
    confirm_basic_a_deleted_attempt_does_not_fail_postgres
);
