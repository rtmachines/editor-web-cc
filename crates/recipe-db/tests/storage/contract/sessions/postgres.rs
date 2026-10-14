//! Postgres only: the stored columns of `session`, the time columns set as `recipe_migrator`,
//! and the error text of a `NOT NULL` failure.

use recipe_app::error::StorageError;
use recipe_domain::identity::Role;
use time::OffsetDateTime;

use super::{INSERT_IDLE, LIMITS, new_session, store};
use crate::contract::chain::display_chain;
use crate::contract::harness::{Harness, begin, new_user, seed_user};
use crate::contract::postgres_harness::PgHarness;
use crate::support::{Role as DbRole, probe};

/// Seed a user and a session on a fresh harness. Return the harness and the session values.
async fn seeded() -> (PgHarness, recipe_domain::identity::SessionToken, String) {
    let h = PgHarness::create().await;
    let user = new_user("akowal", Role::Operator);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    let (token, session) = new_session(user.id, 0, 7, INSERT_IDLE);
    store(&h, &session).await;
    let id = session.id.as_str().to_owned();
    (h, token, id)
}

/// The session row holds the SHA-256 of the token and no column holds the token itself.
#[tokio::test]
async fn postgres_only_the_session_row_holds_only_the_sha256_of_the_token() {
    let (h, token, id) = seeded().await;
    let mut session = h.database.connect(DbRole::Migrator).await;

    let row: String = sqlx::query_scalar("SELECT to_jsonb(s)::text FROM session s")
        .fetch_one(&mut session)
        .await
        .unwrap();

    assert!(!row.contains(&token.to_hex()), "{row}");
    assert!(row.contains(&id), "{row}");
    assert_eq!(id.len(), 64);
}

/// Run an `UPDATE` with one time as `recipe_migrator`. The statement is a constant of the test.
async fn set_time(h: &PgHarness, statement: &'static str, value: OffsetDateTime) {
    let mut session = h.database.connect(DbRole::Migrator).await;
    sqlx::query(statement)
        .bind(value)
        .execute(&mut session)
        .await
        .unwrap();
}

/// A session whose `expires_at` lies in the past gives `None`.
#[tokio::test]
async fn postgres_only_an_expired_idle_deadline_gives_none() {
    let (h, _, id) = seeded().await;
    set_time(
        &h,
        "UPDATE session SET expires_at = $1",
        OffsetDateTime::UNIX_EPOCH,
    )
    .await;
    let id = recipe_domain::identity::SessionId::parse(&id).unwrap();

    let row = h.session_store().authenticate(&id, &LIMITS).await;

    assert!(matches!(row, Ok(None)), "{row:?}");
}

/// A session created 13 hours ago gives `None` under the limit of 12 hours.
#[tokio::test]
async fn postgres_only_a_session_created_13_hours_ago_gives_none() {
    let (h, _, id) = seeded().await;
    let created = OffsetDateTime::now_utc()
        .checked_sub(time::Duration::hours(13))
        .unwrap();
    set_time(&h, "UPDATE session SET created_at = $1", created).await;
    let id = recipe_domain::identity::SessionId::parse(&id).unwrap();

    let row = h.session_store().authenticate(&id, &LIMITS).await;

    assert!(matches!(row, Ok(None)), "{row:?}");
}

/// A valid call changes `last_seen_at` and `expires_at` in the table.
#[tokio::test]
async fn postgres_only_a_valid_call_writes_the_new_times_to_the_row() {
    let (h, _, id) = seeded().await;
    let mut session = h.database.connect(DbRole::Migrator).await;
    let before: OffsetDateTime = sqlx::query_scalar("SELECT expires_at FROM session")
        .fetch_one(&mut session)
        .await
        .unwrap();
    let id = recipe_domain::identity::SessionId::parse(&id).unwrap();

    let row = h
        .session_store()
        .authenticate(&id, &LIMITS)
        .await
        .unwrap()
        .unwrap();

    let after: OffsetDateTime = sqlx::query_scalar("SELECT expires_at FROM session")
        .fetch_one(&mut session)
        .await
        .unwrap();
    assert!(after > before);
    assert_eq!(after, *row.expires_at.as_offset_date_time());
}

/// A `NOT NULL` failure on `session` shows neither the session id nor the CSRF token.
#[tokio::test]
async fn postgres_only_a_not_null_failure_on_session_leaks_no_secret() {
    let h = PgHarness::create().await;
    let user = new_user("akowal", Role::Operator);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    probe::install(
        &h.database,
        "session",
        "BEFORE",
        "INSERT",
        "NEW.csrf_token := NULL;",
    )
    .await;
    let (token, session) = new_session(user.id, 0, 7, INSERT_IDLE);
    let mut tx = begin(&h).await;

    let error = tx.sessions().insert(&session).await.unwrap_err();

    assert!(
        matches!(&error, StorageError::Other { sqlstate, .. } if sqlstate.as_deref() == Some("23502")),
        "{error:?}"
    );
    let chain = display_chain(&error);
    assert!(chain.contains("not-null"), "{chain}");
    assert!(!chain.contains(session.id.as_str()), "{chain}");
    assert!(!chain.contains(&session.csrf_token.to_hex()), "{chain}");
    assert!(!chain.contains(&token.to_hex()), "{chain}");
}
