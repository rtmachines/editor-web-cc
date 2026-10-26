//! Tests of ADR-0034: the event trigger `audit_objects_dropped` stops a drop of an audit object
//! by every role except `audit_owner` (`data-model.md` sections 5.1 and 5.3).
//!
//! `recipe_migrator` owns the schema `public` through `pg_database_owner`. Only the event trigger
//! can stop `DROP SCHEMA public CASCADE`. Each test uses its own disposable database.

use std::collections::BTreeSet;

use sqlx::{Connection, PgConnection};

use crate::support::{
    Role, TestDatabase, assert_privilege_denied, document_sql, init_script_sql, migration_texts,
};

const GUARD_TRIGGER: &str = "audit_objects_dropped";
const GUARD_FUNCTION: &str = "audit_guard.audit_objects_dropped()";
const GUARD_SCHEMA: &str = "audit_guard";
/// The end of the function body, after the normalization.
const GUARD_BODY_END: &str = "$$;";
const GUARD_CREATE: &str = "CREATE FUNCTION audit_guard.audit_objects_dropped()";

/// True when the relation exists in the schema `public`.
async fn table_exists(conn: &mut PgConnection, name: &str) -> bool {
    sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
        .bind(format!("public.{name}"))
        .fetch_one(conn)
        .await
        .unwrap()
}

/// Run one statement as `recipe_migrator` and assert SQLSTATE `42501` and an intact table.
async fn assert_drop_is_refused(statement: &str, survivor: &str) {
    let database = TestDatabase::create().await;
    let mut migrator = database.connect(Role::Migrator).await;

    let result = sqlx::query(statement).execute(&mut migrator).await;

    assert_privilege_denied(result);
    let mut admin = database.connect(Role::Superuser).await;
    assert!(
        table_exists(&mut admin, survivor).await,
        "{survivor} is gone after: {statement}"
    );
}

#[test]
fn no_migration_file_holds_a_carriage_return() {
    let files = migration_texts();

    assert!(files.len() >= 4, "migration files: {}", files.len());
    for (name, text) in &files {
        assert!(!text.contains('\r'), "{name} holds a carriage return");
    }
}

#[tokio::test]
async fn the_migrator_cannot_drop_an_audit_table() {
    // Without CASCADE the dependent function audit_canonical stops the drop with 2BP01 first.
    assert_drop_is_refused("DROP TABLE audit_log CASCADE", "audit_log").await;
}

#[tokio::test]
async fn the_migrator_cannot_drop_the_schema_public_with_cascade() {
    assert_drop_is_refused("DROP SCHEMA public CASCADE", "audit_log").await;
}

#[tokio::test]
async fn the_migrator_cannot_drop_an_audit_table_in_a_do_block() {
    assert_drop_is_refused(
        "DO $$ BEGIN EXECUTE 'DROP TABLE audit_chain_head'; END $$",
        "audit_chain_head",
    )
    .await;
}

/// The schema name that the rename test gives to `public`.
const RENAMED_SCHEMA: &str = "moved";

/// ADR-0097 (f): the rename of `public` must not open a path around the drop guard.
///
/// The test renames `public` and drops the renamed schema with `CASCADE` in one transaction.
/// The test commits only when the drop succeeds, so a database error never leaves a rename behind.
#[tokio::test]
async fn the_migrator_cannot_drop_the_schema_after_a_rename() {
    let database = TestDatabase::create().await;
    let mut migrator = database.connect(Role::Migrator).await;

    let mut transaction = migrator.begin().await.unwrap();
    sqlx::query(&format!("ALTER SCHEMA public RENAME TO {RENAMED_SCHEMA}"))
        .execute(&mut *transaction)
        .await
        .unwrap();
    let dropped = sqlx::query(&format!("DROP SCHEMA {RENAMED_SCHEMA} CASCADE"))
        .execute(&mut *transaction)
        .await;
    let drop_succeeded = dropped.is_ok();
    if drop_succeeded {
        transaction.commit().await.unwrap();
    } else {
        transaction.rollback().await.unwrap();
    }

    assert!(
        !drop_succeeded,
        "DROP SCHEMA {RENAMED_SCHEMA} CASCADE succeeded: the guard did not fire for {RENAMED_SCHEMA}.audit_log"
    );
    assert_privilege_denied(dropped);
    let mut admin = database.connect(Role::Superuser).await;
    assert!(
        table_exists(&mut admin, "audit_log").await,
        "audit_log is gone from the schema public"
    );
}

#[tokio::test]
async fn audit_owner_can_drop_an_audit_table() {
    let database = TestDatabase::create().await;
    let mut migrator = database.connect(Role::Migrator).await;

    sqlx::query("SET ROLE audit_owner")
        .execute(&mut migrator)
        .await
        .unwrap();
    sqlx::query("DROP TABLE security_event")
        .execute(&mut migrator)
        .await
        .unwrap();

    let mut admin = database.connect(Role::Superuser).await;
    assert!(!table_exists(&mut admin, "security_event").await);
}

/// Single-quoted literals of the guard function that are not names of audit objects.
/// They are the two object types (ADR-0097 (b)), the role that may drop an audit object, the
/// two schemas of the search path, and the SQLSTATE condition name of the error.
const NON_NAME_LITERALS: [&str; 6] = [
    "table",
    "function",
    "audit_owner",
    "pg_catalog",
    "pg_temp",
    "insufficient_privilege",
];

/// The bare names in the guard list of a function text (ADR-0097 (a) to (c)).
///
/// A bare name is a single-quoted literal of lower-case letters and underscores. The function
/// drops the object types and the role name. The message text has spaces, so it never matches.
fn guard_list_names(definition: &str) -> BTreeSet<String> {
    definition
        .split('\'')
        .skip(1)
        .step_by(2)
        .filter(|text| !text.is_empty() && text.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        .filter(|text| !NON_NAME_LITERALS.contains(text))
        .map(str::to_owned)
        .collect()
}

/// The statement `CREATE FUNCTION audit_guard.audit_objects_dropped()` in normalized SQL.
///
/// The statement ends at the first `$$;`, the end of the function body.
fn guard_function_statement(normalized: &str) -> String {
    let start = normalized
        .find(GUARD_CREATE)
        .expect("the text holds the CREATE FUNCTION statement of the guard");
    let (body, _) = normalized
        .get(start..)
        .unwrap()
        .split_once(GUARD_BODY_END)
        .expect("the guard function has an end");
    format!("{body}{GUARD_BODY_END}")
}

#[test]
fn the_guard_function_of_section_5_1_equals_the_function_of_the_init_script() {
    let document = guard_function_statement(&document_sql(&["5.1"], &[]));
    let script = guard_function_statement(&init_script_sql());

    assert_eq!(script, document);
}

#[tokio::test]
async fn the_guard_list_equals_the_objects_that_audit_owner_owns() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;

    let definition: String = sqlx::query_scalar("SELECT pg_get_functiondef($1::regprocedure)")
        .bind(GUARD_FUNCTION)
        .fetch_one(&mut admin)
        .await
        .unwrap();
    let owned: Vec<String> = sqlx::query_scalar(
        "SELECT c.relname::text FROM pg_class c \
         WHERE c.relnamespace = 'public'::regnamespace AND c.relkind IN ('r', 'p') \
           AND c.relowner = 'audit_owner'::regrole \
         UNION ALL \
         SELECT p.proname::text FROM pg_proc p \
         WHERE p.pronamespace = 'public'::regnamespace \
           AND p.proowner = 'audit_owner'::regrole",
    )
    .fetch_all(&mut admin)
    .await
    .unwrap();

    let owned: BTreeSet<String> = owned.into_iter().collect();
    // ADR-0069 (o): 0006 adds replication_state() to the seven objects of 0003 and 0005.
    // ADR-0097 (e): the list holds bare names, so the comparison ignores the schema.
    assert_eq!(owned.len(), 8, "{owned:?}");
    assert_eq!(guard_list_names(&definition), owned);
}

#[tokio::test]
async fn the_event_trigger_is_enabled_and_belongs_to_the_superuser() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;

    let row: (String, String, bool) = sqlx::query_as(
        "SELECT t.evtevent::text, t.evtenabled::text, r.rolsuper \
         FROM pg_event_trigger t JOIN pg_roles r ON r.oid = t.evtowner \
         WHERE t.evtname = $1",
    )
    .bind(GUARD_TRIGGER)
    .fetch_one(&mut admin)
    .await
    .unwrap();

    assert_eq!(row, ("sql_drop".to_owned(), "O".to_owned(), true));
}

#[tokio::test]
async fn no_login_role_has_a_privilege_on_the_schema_audit_guard() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;

    // recipe_backup is outside this list: pg_read_all_data grants it USAGE on every schema.
    for role in [Role::App, Role::Migrator, Role::Replicator] {
        for privilege in ["USAGE", "CREATE"] {
            let held: bool = sqlx::query_scalar("SELECT has_schema_privilege($1, $2, $3)")
                .bind(role.name())
                .bind(GUARD_SCHEMA)
                .bind(privilege)
                .fetch_one(&mut admin)
                .await
                .unwrap();
            assert!(!held, "{} holds {privilege} on audit_guard", role.name());
        }
        let can_execute: bool = sqlx::query_scalar("SELECT has_function_privilege($1, $2, $3)")
            .bind(role.name())
            .bind(GUARD_FUNCTION)
            .bind("EXECUTE")
            .fetch_one(&mut admin)
            .await
            .unwrap();
        assert!(!can_execute, "{} can execute the guard", role.name());
    }
}
