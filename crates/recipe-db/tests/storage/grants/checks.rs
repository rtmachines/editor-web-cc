//! The state checks of `data-model.md` section 5.3, as functions that several tests reuse.
//!
//! The test of `0003_grants.sql` runs them once. The test of the five steps of section 5.1 runs
//! them again after a migration that changes an audit object.

use std::collections::BTreeSet;

use sqlx::{Connection as _, PgConnection};

use crate::support::{Role, TestDatabase, assert_privilege_denied};

const GRANTS: &str = include_str!("../../grants.expected");
const AUDIT_TABLES: [&str; 3] = ["audit_log", "audit_chain_head", "security_event"];
/// The audit functions with the argument types that identify them.
const AUDIT_FUNCTIONS: [&str; 4] = [
    "audit_canonical(audit_log)",
    "audit_hash_chain()",
    "security_event_purge()",
    "security_event_guard()",
];
const TABLE_PRIVILEGES: [&str; 7] = [
    "SELECT",
    "INSERT",
    "UPDATE",
    "DELETE",
    "TRUNCATE",
    "REFERENCES",
    "TRIGGER",
];

const TABLE_GRANTS: &str = "SELECT table_name::text || '|' || privilege_type::text \
     FROM information_schema.role_table_grants \
     WHERE grantee = 'recipe_app' AND table_schema = 'public'";
const COLUMN_GRANTS: &str = "SELECT c.relname::text || '.' || a.attname::text || '|' \
         || x.privilege_type \
     FROM pg_attribute a \
     JOIN pg_class c ON c.oid = a.attrelid \
     CROSS JOIN LATERAL aclexplode(a.attacl) x \
     WHERE x.grantee = 'recipe_app'::regrole \
       AND c.relnamespace = 'public'::regnamespace AND NOT a.attisdropped";

/// The lines of `grants.expected` as a set.
fn expected_lines() -> BTreeSet<String> {
    GRANTS
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The grants that `recipe_app` holds, in the line format of `grants.expected`.
async fn actual_lines(conn: &mut PgConnection) -> BTreeSet<String> {
    let mut lines = BTreeSet::new();
    for query in [TABLE_GRANTS, COLUMN_GRANTS] {
        let rows: Vec<String> = sqlx::query_scalar(query)
            .fetch_all(&mut *conn)
            .await
            .unwrap();
        lines.extend(rows);
    }
    lines
}

/// The grants of `recipe_app` equal `grants.expected`. The message names both directions.
pub async fn assert_grants_equal_file(database: &TestDatabase) {
    let mut conn = database.connect(Role::App).await;
    let expected = expected_lines();
    let actual = actual_lines(&mut conn).await;

    let missing: Vec<&String> = expected.difference(&actual).collect();
    let unexpected: Vec<&String> = actual.difference(&expected).collect();

    assert!(
        missing.is_empty() && unexpected.is_empty(),
        "in the file but not granted: {missing:?}; granted but not in the file: {unexpected:?}"
    );
}

/// Return one value of a query, with no bind.
async fn scalar<T>(conn: &mut PgConnection, query: &str) -> T
where
    T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin,
{
    sqlx::query_scalar(query).fetch_one(conn).await.unwrap()
}

/// `recipe_migrator` is a member of `audit_owner` and does not inherit its privileges.
async fn assert_migrator_membership(conn: &mut PgConnection) {
    let inherit: bool = scalar(
        conn,
        "SELECT inherit_option FROM pg_auth_members \
         WHERE roleid = 'audit_owner'::regrole AND member = 'recipe_migrator'::regrole",
    )
    .await;

    assert!(!inherit, "the membership of recipe_migrator inherits");
}

/// `audit_owner` owns the three audit tables and the four audit functions.
async fn assert_audit_owner_owns(conn: &mut PgConnection) {
    for table in AUDIT_TABLES {
        let owner: String = sqlx::query_scalar(
            "SELECT pg_get_userbyid(relowner)::text FROM pg_class \
             WHERE oid = to_regclass($1)",
        )
        .bind(table)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
        assert_eq!(owner, "audit_owner", "owner of {table}");
    }
    for function in AUDIT_FUNCTIONS {
        let owner: String = sqlx::query_scalar(
            "SELECT pg_get_userbyid(proowner)::text FROM pg_proc \
             WHERE oid = to_regprocedure($1)",
        )
        .bind(function)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
        assert_eq!(owner, "audit_owner", "owner of {function}");
    }
}

/// `recipe_migrator` holds no privilege on an audit table and cannot insert an audit row.
async fn assert_migrator_has_no_audit_privilege(database: &TestDatabase, conn: &mut PgConnection) {
    for table in AUDIT_TABLES {
        for privilege in TABLE_PRIVILEGES {
            let held: bool = sqlx::query_scalar(
                "SELECT has_table_privilege('recipe_migrator'::name, $1::text, $2::text)",
            )
            .bind(table)
            .bind(privilege)
            .fetch_one(&mut *conn)
            .await
            .unwrap();
            assert!(!held, "recipe_migrator holds {privilege} on {table}");
        }
    }
    let mut migrator = database.connect(Role::Migrator).await;
    let result = sqlx::query(
        "INSERT INTO audit_log (action, entity_type, entity_id, aggregate_type, aggregate_id, \
             after_state, request_id) \
         VALUES ('RenameMachine', 'machine', gen_random_uuid(), 'machine', gen_random_uuid(), \
             '{}'::jsonb, gen_random_uuid())",
    )
    .execute(&mut migrator)
    .await;
    assert_privilege_denied(result);
}

/// `recipe_migrator` cannot execute the three audit functions that the document revokes from `PUBLIC`.
///
/// `audit_canonical` keeps the default `PUBLIC` execute right, so this check skips it.
async fn assert_migrator_cannot_execute_audit_functions(conn: &mut PgConnection) {
    for function in [
        "audit_hash_chain()",
        "security_event_purge()",
        "security_event_guard()",
    ] {
        let held: bool = sqlx::query_scalar(
            "SELECT has_function_privilege('recipe_migrator'::name, $1::text, 'EXECUTE')",
        )
        .bind(function)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
        assert!(!held, "recipe_migrator can execute {function}");
    }
}

/// `recipe_app` is not a superuser, owns nothing, and cannot switch to another role.
async fn assert_app_is_confined(database: &TestDatabase, conn: &mut PgConnection) {
    let superuser: bool = scalar(
        conn,
        "SELECT rolsuper FROM pg_roles WHERE rolname = 'recipe_app'",
    )
    .await;
    let owned: i64 = scalar(
        conn,
        "SELECT count(*) FROM pg_class WHERE relowner = 'recipe_app'::regrole",
    )
    .await;
    let switchable: i64 = scalar(
        conn,
        "SELECT count(*) FROM pg_auth_members \
         WHERE member = 'recipe_app'::regrole AND set_option",
    )
    .await;
    let sequence_grants: i64 = scalar(
        conn,
        "SELECT count(*) FROM pg_class c, aclexplode(c.relacl) x \
         WHERE c.relkind = 'S' AND x.grantee = 'recipe_app'::regrole",
    )
    .await;
    let can_purge: bool = scalar(
        conn,
        "SELECT has_function_privilege('recipe_app', 'security_event_purge()', 'EXECUTE')",
    )
    .await;

    assert!(!superuser, "recipe_app is a superuser");
    assert_eq!(owned, 0, "recipe_app owns a relation");
    assert_eq!(switchable, 0, "recipe_app can SET ROLE to a role");
    assert_eq!(sequence_grants, 0, "recipe_app holds a sequence privilege");
    assert!(
        can_purge,
        "recipe_app cannot execute security_event_purge()"
    );

    let mut app = database.connect(Role::App).await;
    for role in ["audit_owner", "pg_read_all_stats"] {
        // The role name comes from the constant list above.
        let result = sqlx::query(&format!("SET ROLE {role}"))
            .execute(&mut app)
            .await;
        assert!(result.is_err(), "recipe_app switched to {role}");
        app.ping().await.unwrap();
    }
}

/// `audit_owner` holds no `CREATE` on the schema `public`.
async fn assert_audit_owner_cannot_create(conn: &mut PgConnection) {
    let can_create: bool = scalar(
        conn,
        "SELECT has_schema_privilege('audit_owner', 'public', 'CREATE')",
    )
    .await;

    assert!(!can_create, "audit_owner holds CREATE on public");
}

/// Run every state check of `data-model.md` section 5.3 on one test database.
pub async fn assert_least_privilege_state(database: &TestDatabase) {
    assert_grants_equal_file(database).await;
    let mut admin = database.connect(Role::Superuser).await;
    assert_migrator_membership(&mut admin).await;
    assert_audit_owner_owns(&mut admin).await;
    assert_migrator_has_no_audit_privilege(database, &mut admin).await;
    assert_migrator_cannot_execute_audit_functions(&mut admin).await;
    assert_app_is_confined(database, &mut admin).await;
    assert_audit_owner_cannot_create(&mut admin).await;
}

#[tokio::test]
async fn the_grants_of_recipe_app_equal_the_file() {
    let database = TestDatabase::create().await;

    assert_grants_equal_file(&database).await;
}

#[tokio::test]
async fn the_state_of_the_roles_after_0003_follows_section_5_3() {
    let database = TestDatabase::create().await;

    assert_least_privilege_state(&database).await;
}
