//! Tests of `app_user`, `session`, `login_attempt`, and `security_event`
//! (`data-model.md` sections 3.5 to 3.8, task 7.3).
//!
//! `security_event` belongs to `audit_owner` after `0003`, so its tests run as the superuser of the
//! harness. The other tables run as `recipe_migrator`, the owner of the tables.

use sqlx::PgConnection;
use uuid::Uuid;

use crate::support::fixtures;
use crate::support::{
    CHECK_VIOLATION, FOREIGN_KEY_VIOLATION, Role, TestDatabase, UNIQUE_VIOLATION, assert_constraint,
};

/// The constraints and unique indexes that the tests of this module violate.
pub const COVERED: &[&str] = &[
    "app_user_pkey",
    "app_user_login_format",
    "app_user_role_value",
    "app_user_hash_format",
    "app_user_active_consistent",
    "app_user_epoch_nonnegative",
    "app_user_login_unique",
    "session_pkey",
    "session_id_format",
    "session_csrf_format",
    "session_user_id_fkey",
    "login_attempt_pkey",
    "login_attempt_login_length",
    "security_event_pkey",
    "security_event_kind_value",
    "security_event_subject",
    "security_event_client_ip_present",
    "security_event_user_agent_length",
    "security_event_count_positive",
    "security_event_count_aggregated",
    "security_event_user_id_fkey",
    "security_event_aggregate_unique",
];

/// A password hash that satisfies `app_user_hash_format`.
const HASH: &str = "$argon2id$v=19$m=65536,t=3,p=4$c2FsdA$aGFzaA";

/// Return a migrated database connection as the owner of the tables.
async fn migrator() -> PgConnection {
    let database = TestDatabase::create().await;
    database.connect(Role::Migrator).await
}

/// Return a migrated database connection as the superuser of the harness.
async fn superuser() -> PgConnection {
    let database = TestDatabase::create().await;
    database.connect(Role::Superuser).await
}

/// A 64-character string of one digit, which satisfies the hex format of `session`.
fn hex(digit: char) -> String {
    digit.to_string().repeat(64)
}

// ---- app_user ----

/// Insert an active account and return its id.
async fn insert_user(
    conn: &mut PgConnection,
    login: &str,
    role: &str,
    hash: &str,
    epoch: i64,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO app_user (login, password_hash, role, session_epoch) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(login)
    .bind(hash)
    .bind(role)
    .bind(epoch)
    .fetch_one(conn)
    .await
}

#[tokio::test]
async fn app_user_row_with_valid_values_inserts() {
    let mut conn = migrator().await;

    insert_user(&mut conn, "ops.user", "operator", HASH, 0)
        .await
        .unwrap();
}

#[tokio::test]
async fn app_user_pkey_rejects_a_second_row_with_the_same_id() {
    let mut conn = migrator().await;
    let id = insert_user(&mut conn, "pk.first", "operator", HASH, 0)
        .await
        .unwrap();

    let result = sqlx::query(
        "INSERT INTO app_user (id, login, password_hash, role) VALUES ($1, 'pk.copy', $2, 'operator')",
    )
    .bind(id)
    .bind(HASH)
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "app_user_pkey");
}

#[tokio::test]
async fn app_user_login_format_rejects_an_uppercase_login() {
    let mut conn = migrator().await;

    let result = insert_user(&mut conn, "Bad.Login", "operator", HASH, 0).await;

    assert_constraint(result, CHECK_VIOLATION, "app_user_login_format");
}

#[tokio::test]
async fn app_user_role_value_rejects_an_unknown_role() {
    let mut conn = migrator().await;

    let result = insert_user(&mut conn, "role.test", "root", HASH, 0).await;

    assert_constraint(result, CHECK_VIOLATION, "app_user_role_value");
}

#[tokio::test]
async fn app_user_hash_format_rejects_a_plain_password() {
    let mut conn = migrator().await;

    let result = insert_user(&mut conn, "hash.test", "operator", "plain", 0).await;

    assert_constraint(result, CHECK_VIOLATION, "app_user_hash_format");
}

#[tokio::test]
async fn app_user_active_consistent_rejects_an_inactive_user_without_a_deactivation_time() {
    let mut conn = migrator().await;

    let result = sqlx::query(
        "INSERT INTO app_user (login, password_hash, role, is_active) \
         VALUES ('active.test', $1, 'operator', false)",
    )
    .bind(HASH)
    .execute(&mut conn)
    .await;

    assert_constraint(result, CHECK_VIOLATION, "app_user_active_consistent");
}

#[tokio::test]
async fn app_user_epoch_nonnegative_rejects_a_negative_epoch() {
    let mut conn = migrator().await;

    let result = insert_user(&mut conn, "epoch.test", "operator", HASH, -1).await;

    assert_constraint(result, CHECK_VIOLATION, "app_user_epoch_nonnegative");
}

#[tokio::test]
async fn app_user_login_unique_rejects_a_second_user_with_the_same_login() {
    let mut conn = migrator().await;
    insert_user(&mut conn, "same.login", "operator", HASH, 0)
        .await
        .unwrap();

    let result = insert_user(&mut conn, "same.login", "administrator", HASH, 0).await;

    assert_constraint(result, UNIQUE_VIOLATION, "app_user_login_unique");
}

// ---- session ----

/// Insert a session row with the given id and CSRF token.
async fn insert_session(
    conn: &mut PgConnection,
    id: &str,
    csrf: &str,
    user: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO session (id, user_id, epoch, csrf_token, expires_at) \
         VALUES ($1, $2, 0, $3, now() + interval '1 hour')",
    )
    .bind(id)
    .bind(user)
    .bind(csrf)
    .execute(conn)
    .await
    .map(|_| ())
}

#[tokio::test]
async fn session_row_with_valid_values_inserts() {
    let mut conn = migrator().await;
    let user = fixtures::user(&mut conn).await;

    insert_session(&mut conn, &hex('a'), &hex('b'), user)
        .await
        .unwrap();
}

#[tokio::test]
async fn session_pkey_rejects_a_second_session_with_the_same_id() {
    let mut conn = migrator().await;
    let user = fixtures::user(&mut conn).await;
    insert_session(&mut conn, &hex('a'), &hex('b'), user)
        .await
        .unwrap();

    let result = insert_session(&mut conn, &hex('a'), &hex('c'), user).await;

    assert_constraint(result, UNIQUE_VIOLATION, "session_pkey");
}

#[tokio::test]
async fn session_id_format_rejects_a_short_id() {
    let mut conn = migrator().await;
    let user = fixtures::user(&mut conn).await;

    let result = insert_session(&mut conn, "XYZ", &hex('b'), user).await;

    assert_constraint(result, CHECK_VIOLATION, "session_id_format");
}

#[tokio::test]
async fn session_csrf_format_rejects_a_short_token() {
    let mut conn = migrator().await;
    let user = fixtures::user(&mut conn).await;

    let result = insert_session(&mut conn, &hex('a'), "XYZ", user).await;

    assert_constraint(result, CHECK_VIOLATION, "session_csrf_format");
}

#[tokio::test]
async fn session_user_id_fkey_rejects_an_unknown_user() {
    let mut conn = migrator().await;

    let result = insert_session(&mut conn, &hex('a'), &hex('b'), Uuid::nil()).await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "session_user_id_fkey");
}

// ---- login_attempt ----

/// Insert a failed login attempt and return its id.
async fn insert_attempt(conn: &mut PgConnection, login: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO login_attempt (login, client_ip, succeeded) \
         VALUES ($1, '192.0.2.1'::inet, false) RETURNING id",
    )
    .bind(login)
    .fetch_one(conn)
    .await
}

#[tokio::test]
async fn login_attempt_row_with_valid_values_inserts() {
    let mut conn = migrator().await;

    insert_attempt(&mut conn, "ops.user").await.unwrap();
}

#[tokio::test]
async fn login_attempt_pkey_rejects_a_second_row_with_the_same_id() {
    let mut conn = migrator().await;
    let id = insert_attempt(&mut conn, "ops.user").await.unwrap();

    let result = sqlx::query(
        "INSERT INTO login_attempt (id, login, client_ip, succeeded) \
         OVERRIDING SYSTEM VALUE VALUES ($1, 'ops.user', '192.0.2.1'::inet, false)",
    )
    .bind(id)
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "login_attempt_pkey");
}

#[tokio::test]
async fn login_attempt_login_length_rejects_an_empty_login() {
    let mut conn = migrator().await;

    let result = insert_attempt(&mut conn, "").await;

    assert_constraint(result, CHECK_VIOLATION, "login_attempt_login_length");
}

// ---- security_event (superuser of the harness) ----

/// The columns of one `security_event` row. A test changes one field of a valid row.
#[derive(Debug, Clone)]
struct EventRow {
    kind: &'static str,
    login_hmac: Option<Vec<u8>>,
    user_id: Option<Uuid>,
    client_ip: Option<&'static str>,
    user_agent: Option<String>,
    count: i32,
    occurred_at: &'static str,
}

impl EventRow {
    /// A `login_failed` row of one address with a login hash, in the minute of 10:00 UTC.
    fn valid() -> Self {
        Self {
            kind: "login_failed",
            login_hmac: Some(vec![7; 32]),
            user_id: None,
            client_ip: Some("192.0.2.10"),
            user_agent: None,
            count: 1,
            occurred_at: "2026-01-01T10:00:10Z",
        }
    }
}

/// Insert a `security_event` row and return its id.
async fn insert_event(conn: &mut PgConnection, row: &EventRow) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        "INSERT INTO security_event \
             (kind, login_hmac, user_id, client_ip, user_agent, request_id, count, occurred_at) \
         VALUES ($1, $2, $3, $4::inet, $5, $6, $7, $8::timestamptz) RETURNING id",
    )
    .bind(row.kind)
    .bind(row.login_hmac.clone())
    .bind(row.user_id)
    .bind(row.client_ip)
    .bind(row.user_agent.clone())
    .bind(Uuid::nil())
    .bind(row.count)
    .bind(row.occurred_at)
    .fetch_one(conn)
    .await
}

#[tokio::test]
async fn security_event_row_with_valid_values_inserts() {
    let mut conn = superuser().await;

    insert_event(&mut conn, &EventRow::valid()).await.unwrap();
}

#[tokio::test]
async fn security_event_pkey_rejects_a_second_row_with_the_same_id() {
    let mut conn = superuser().await;
    let id = insert_event(&mut conn, &EventRow::valid()).await.unwrap();

    let result = sqlx::query(
        "INSERT INTO security_event \
             (id, kind, login_hmac, client_ip, request_id, count) \
         OVERRIDING SYSTEM VALUE VALUES ($1, 'login_failed', $2, '192.0.2.11'::inet, $3, 1)",
    )
    .bind(id)
    .bind(vec![7_u8; 32])
    .bind(Uuid::nil())
    .execute(&mut conn)
    .await;

    assert_constraint(result, UNIQUE_VIOLATION, "security_event_pkey");
}

#[tokio::test]
async fn security_event_kind_value_rejects_an_unknown_kind() {
    let mut conn = superuser().await;
    let row = EventRow {
        kind: "bogus",
        ..EventRow::valid()
    };

    let result = insert_event(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "security_event_kind_value");
}

#[tokio::test]
async fn security_event_subject_rejects_a_row_without_login_hash_or_user() {
    let mut conn = superuser().await;
    let row = EventRow {
        login_hmac: None,
        user_id: None,
        ..EventRow::valid()
    };

    let result = insert_event(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "security_event_subject");
}

#[tokio::test]
async fn security_event_client_ip_present_rejects_a_row_without_an_address() {
    let mut conn = superuser().await;
    let row = EventRow {
        client_ip: None,
        ..EventRow::valid()
    };

    let result = insert_event(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "security_event_client_ip_present");
}

#[tokio::test]
async fn security_event_user_agent_length_rejects_a_user_agent_above_256_characters() {
    let mut conn = superuser().await;
    let row = EventRow {
        kind: "unlock",
        user_agent: Some("a".repeat(257)),
        ..EventRow::valid()
    };

    let result = insert_event(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "security_event_user_agent_length");
}

#[tokio::test]
async fn security_event_count_positive_rejects_a_count_of_zero() {
    let mut conn = superuser().await;
    let row = EventRow {
        count: 0,
        ..EventRow::valid()
    };

    let result = insert_event(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "security_event_count_positive");
}

#[tokio::test]
async fn security_event_count_aggregated_rejects_a_count_above_one_for_a_single_event_kind() {
    let mut conn = superuser().await;
    let row = EventRow {
        kind: "unlock",
        count: 2,
        ..EventRow::valid()
    };

    let result = insert_event(&mut conn, &row).await;

    assert_constraint(result, CHECK_VIOLATION, "security_event_count_aggregated");
}

#[tokio::test]
async fn security_event_user_id_fkey_rejects_an_unknown_user() {
    let mut conn = superuser().await;
    let row = EventRow {
        user_id: Some(Uuid::nil()),
        ..EventRow::valid()
    };

    let result = insert_event(&mut conn, &row).await;

    assert_constraint(result, FOREIGN_KEY_VIOLATION, "security_event_user_id_fkey");
}

#[tokio::test]
async fn security_event_aggregate_unique_rejects_a_second_row_of_one_kind_and_minute() {
    let mut conn = superuser().await;
    insert_event(&mut conn, &EventRow::valid()).await.unwrap();
    let second = EventRow {
        occurred_at: "2026-01-01T10:00:50Z",
        ..EventRow::valid()
    };

    let result = insert_event(&mut conn, &second).await;

    assert_constraint(result, UNIQUE_VIOLATION, "security_event_aggregate_unique");
}

#[tokio::test]
async fn security_event_aggregate_unique_accepts_a_row_of_the_next_minute() {
    let mut conn = superuser().await;
    insert_event(&mut conn, &EventRow::valid()).await.unwrap();
    let next_minute = EventRow {
        occurred_at: "2026-01-01T10:01:00Z",
        ..EventRow::valid()
    };

    insert_event(&mut conn, &next_minute).await.unwrap();
}
