//! Tests of the migration `0011_plc_type_rules.sql` (ADR-0143, ADR-0145, design D6 and D10).
//!
//! The statements of `0011` equal the SQL blocks of `data-model.md` sections 3.3 and 3.4. Each
//! test compares one piece of the file with the document after the same normalization. The other
//! tests run the file on a schema that is put back to the state of `0001`.

use std::cell::Cell;
use std::time::Duration;

use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;

use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{Role, TestDatabase, document_sql, migration_sql};

const MIGRATION: &str = "0011_plc_type_rules.sql";
const FIRST_MIGRATION: &str = "0001_schema.sql";
const FUNCTION_HEAD: &str = "CREATE FUNCTION dims_valid";
const REPLACING_HEAD: &str = "CREATE OR REPLACE FUNCTION dims_valid";
const DOLLAR_QUOTE: &str = "$$";
const STRUCT_INDEX_HEAD: &str = "CREATE UNIQUE INDEX plc_type_struct_name_ci_unique";
const STRUCT_INDEX_NAME: &str = "plc_type_struct_name_ci_unique";
const MEMBER_INDEX_HEAD: &str = "CREATE UNIQUE INDEX plc_type_member_name_ci_unique";
const MEMBER_INDEX_NAME: &str = "plc_type_member_name_ci_unique";
const DIMS_VALID_REGPROC: &str = "dims_valid(integer[], integer[])";
const RULE_CHECK_REGPROC: &str = "plc_type_rule_check()";
/// The text that step 2 of D6 raises for 2 array rows, 1 group of struct names, and 1 group of
/// member names.
const STOP_MESSAGE: &str = "migration 0011 stopped: 2 array rows start at another subscript \
    than 1, 1 groups of struct names and 1 groups of member names differ only in case. Run \
    deploy/db/check-identifier-rules.sql (docs/ops/runbook.md).";

/// The statement of a function in normalized SQL: from `head` to the `;` after the closing `$$`.
fn function_statement(sql: &str, head: &str) -> String {
    let found = sql.find(head);
    assert!(found.is_some(), "no statement that starts with '{head}'");
    let start = found.expect("presence asserted above");
    let tail = sql.get(start..).expect("a find index is a char boundary");
    let (signature, rest) = tail.split_once(DOLLAR_QUOTE).expect("an opening $$");
    let (body, after) = rest.split_once(DOLLAR_QUOTE).expect("a closing $$");
    let (gap, _) = after.split_once(';').expect("a ; after the closing $$");
    format!("{signature}{DOLLAR_QUOTE}{body}{DOLLAR_QUOTE}{gap};")
}

#[test]
fn migration_0011_replaces_dims_valid_with_the_function_of_section_3_3() {
    let document = document_sql(&["3.3"], &[]);
    let file = migration_sql(MIGRATION);

    let in_document =
        function_statement(&document, FUNCTION_HEAD).replacen(FUNCTION_HEAD, REPLACING_HEAD, 1);
    let in_file = function_statement(&file, REPLACING_HEAD);

    assert!(
        in_document.contains("array_lower(lower_b, 1) = 1")
            && in_document.contains("array_lower(upper_b, 1) = 1"),
        "section 3.3 must hold both start subscript checks: {in_document}"
    );
    assert_eq!(in_file, in_document);
}

/// The statement of an index in normalized SQL: from `head` to the first `;`.
fn index_statement(sql: &str, head: &str) -> String {
    let found = sql.find(head);
    assert!(found.is_some(), "no statement that starts with '{head}'");
    let start = found.expect("presence asserted above");
    let tail = sql.get(start..).expect("a find index is a char boundary");
    let (statement, _) = tail.split_once(';').expect("a ; after the index");
    format!("{statement};")
}

#[test]
fn migration_0011_builds_the_struct_name_index_of_section_3_3() {
    let document = document_sql(&["3.3"], &[]);
    let file = migration_sql(MIGRATION);

    let in_document = index_statement(&document, STRUCT_INDEX_HEAD);
    let in_file = index_statement(&file, STRUCT_INDEX_HEAD);

    assert!(
        in_document.contains("lower(name)"),
        "section 3.3 must compare the struct name without case: {in_document}"
    );
    assert_eq!(in_file, in_document);
}

#[test]
fn migration_0011_builds_the_member_name_index_of_section_3_4() {
    let document = document_sql(&["3.4"], &[]);
    let file = migration_sql(MIGRATION);

    let in_document = index_statement(&document, MEMBER_INDEX_HEAD);
    let in_file = index_statement(&file, MEMBER_INDEX_HEAD);

    assert!(
        in_document.contains("lower(member_name)"),
        "section 3.4 must compare the member name without case: {in_document}"
    );
    assert_eq!(in_file, in_document);
}

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    TestDatabase::create()
        .await
        .into_session(Role::Migrator)
        .await
}

/// Return the text of a function as `pg_get_functiondef` prints it.
async fn function_definition(conn: &mut PgConnection, regproc: &str) -> String {
    sqlx::query_scalar("SELECT pg_get_functiondef($1::regprocedure)")
        .bind(regproc)
        .fetch_one(conn)
        .await
        .unwrap()
}

/// Return whether a function exists.
async fn function_exists(conn: &mut PgConnection, regproc: &str) -> bool {
    sqlx::query_scalar("SELECT to_regprocedure($1) IS NOT NULL")
        .bind(regproc)
        .fetch_one(conn)
        .await
        .unwrap()
}

/// Put the schema back to the state before `0011`: the `dims_valid` of `0001`, no array shape
/// constraint, and no index of ADR-0143 (j).
async fn reset_to_the_state_of_0001(conn: &mut PgConnection) {
    let first = migration_sql(FIRST_MIGRATION);
    let shipped =
        function_statement(&first, FUNCTION_HEAD).replacen(FUNCTION_HEAD, REPLACING_HEAD, 1);
    for statement in [
        shipped.as_str(),
        "ALTER TABLE plc_type DROP CONSTRAINT plc_type_array_shape",
        "DROP INDEX IF EXISTS plc_type_struct_name_ci_unique",
        "DROP INDEX IF EXISTS plc_type_member_name_ci_unique",
    ] {
        sqlx::query(statement).execute(&mut *conn).await.unwrap();
    }
}

/// Run the whole text of `0011` as one simple query, so it commits or rolls back as one unit.
async fn run_0011(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/0011_plc_type_rules.sql"
    ))
    .unwrap();
    sqlx::raw_sql(&text).execute(conn).await.map(|_| ())
}

/// Insert an array row with literal bound arrays.
async fn insert_array(
    conn: &mut PgConnection,
    machine: Uuid,
    element: Uuid,
    lower: &str,
    upper: &str,
) {
    sqlx::query(&format!(
        "INSERT INTO plc_type (kind, platforms, element_type_id, dim_lower, dim_upper, \
             machine_id) \
         VALUES ('array', ARRAY['{TWIN_CAT}'], $1, '{lower}'::integer[], '{upper}'::integer[], $2)"
    ))
    .bind(element)
    .bind(machine)
    .execute(conn)
    .await
    .unwrap();
}

/// Insert a struct revision with the listed members in one transaction.
async fn insert_struct(
    conn: &mut PgConnection,
    machine: Uuid,
    name: &str,
    revision: i32,
    members: &[(&str, Uuid)],
) {
    let mut tx = conn.begin().await.unwrap();
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO plc_type (kind, name, platforms, machine_id, revision) \
         VALUES ('struct', $1, ARRAY['TwinCat3'], $2, $3) RETURNING id",
    )
    .bind(name)
    .bind(machine)
    .bind(revision)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    for (ordinal, (member, member_type)) in (1_i16..).zip(members) {
        sqlx::query(
            "INSERT INTO plc_type_member (struct_type_id, ordinal, member_name, member_type_id) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(ordinal)
        .bind(member)
        .bind(member_type)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn migration_0011_stops_on_rows_that_break_the_new_rules_and_changes_nothing() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    let int = fixtures::primitive(&mut conn, "INT").await;
    reset_to_the_state_of_0001(&mut conn).await;
    let before = function_definition(&mut conn, DIMS_VALID_REGPROC).await;
    // Two array rows: the lower bounds and the upper bounds start at the subscript 0.
    insert_array(&mut conn, machine, int, "[0:0]={1}", "{5}").await;
    insert_array(&mut conn, machine, int, "{1}", "[0:0]={5}").await;
    // One group of struct names and one group of member names that differ only in case.
    insert_struct(&mut conn, machine, "st_zone", 1, &[("a", int)]).await;
    insert_struct(&mut conn, machine, "ST_Zone", 1, &[("a", int)]).await;
    insert_struct(
        &mut conn,
        machine,
        "ST_Pair",
        1,
        &[("speed", int), ("Speed", int)],
    )
    .await;

    let error = run_0011(&mut conn)
        .await
        .expect_err("the precheck stops the file");

    let message = error
        .as_database_error()
        .expect("a database error")
        .message();
    assert_eq!(message, STOP_MESSAGE);
    assert_eq!(
        function_definition(&mut conn, DIMS_VALID_REGPROC).await,
        before,
        "dims_valid keeps the text of 0001"
    );
    assert!(!before.contains("array_lower"), "{before}");
    assert!(
        !function_exists(&mut conn, RULE_CHECK_REGPROC).await,
        "the check function rolled back with the file"
    );
    let indexes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_indexes \
         WHERE indexname IN ('plc_type_struct_name_ci_unique', 'plc_type_member_name_ci_unique')",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert_eq!(indexes, 0, "neither index exists");
}

#[tokio::test]
async fn migration_0011_runs_on_rows_that_follow_the_rules() {
    let mut conn = connection().await;
    let machine = fixtures::machine(&mut conn, TWIN_CAT).await;
    let other = fixtures::machine(&mut conn, TWIN_CAT).await;
    let int = fixtures::primitive(&mut conn, "INT").await;
    reset_to_the_state_of_0001(&mut conn).await;
    let before = function_definition(&mut conn, DIMS_VALID_REGPROC).await;
    insert_array(&mut conn, machine, int, "{1}", "{5}").await;
    insert_struct(
        &mut conn,
        machine,
        "ST_Zone",
        1,
        &[("speed", int), ("limit", int)],
    )
    .await;
    // The same name in another case on another machine and in another revision.
    insert_struct(&mut conn, other, "st_zone", 1, &[("speed", int)]).await;
    insert_struct(&mut conn, machine, "st_zone", 2, &[("speed", int)]).await;

    run_0011(&mut conn)
        .await
        .expect("rows that follow the rules pass the check");

    assert_dims_valid_replaced(&mut conn, &before).await;
    assert!(
        !function_exists(&mut conn, RULE_CHECK_REGPROC).await,
        "step 4 drops the check function"
    );
    let definition = index_definition(&mut conn, STRUCT_INDEX_NAME).await;
    assert!(definition.contains("UNIQUE"), "{definition}");
    assert!(definition.contains("lower(name"), "{definition}");
    let member_definition = index_definition(&mut conn, MEMBER_INDEX_NAME).await;
    assert!(member_definition.contains("UNIQUE"), "{member_definition}");
    assert!(
        member_definition.contains("lower(member_name"),
        "{member_definition}"
    );
}

/// Assert that `dims_valid` has a new text with the start subscript check and rejects a start of 0.
async fn assert_dims_valid_replaced(conn: &mut PgConnection, before: &str) {
    let after = function_definition(conn, DIMS_VALID_REGPROC).await;
    assert_ne!(after, before, "dims_valid has the new text");
    assert!(after.contains("array_lower(lower_b, 1) = 1"), "{after}");
    let accepted: bool = sqlx::query_scalar("SELECT dims_valid('[0:0]={5}', '{0}')")
        .fetch_one(conn)
        .await
        .unwrap();
    assert!(!accepted);
}

/// Return the text of an index definition that the migration built.
async fn index_definition(conn: &mut PgConnection, index_name: &str) -> String {
    sqlx::query_scalar("SELECT indexdef FROM pg_indexes WHERE indexname = $1")
        .bind(index_name)
        .fetch_one(conn)
        .await
        .expect("the migration builds the index")
}

/// The pause between two looks at `pg_locks`.
const POLL_PAUSE: Duration = Duration::from_millis(20);
/// The number of looks at `pg_locks` before the test gives up.
const POLL_LIMIT: u32 = 150;
/// The first text of the message that the row check raises (design D6, step 2).
const STOP_PREFIX: &str = "migration 0011 stopped: 1 array rows start at another subscript than 1";

/// Open three sessions of the same database: the writer, the migrator, and the watcher.
async fn three_sessions() -> (PgConnection, PgConnection, PgConnection) {
    let database = TestDatabase::create().await;
    let writer = database.connect(Role::Migrator).await;
    let migrator = database.connect(Role::Migrator).await;
    let watcher = database.connect(Role::Migrator).await;
    database.keep_until_thread_ends();
    (writer, migrator, watcher)
}

/// Return whether a session waits for a lock on `plc_type`.
async fn waits_for_plc_type(watcher: &mut PgConnection) -> bool {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_locks \
         WHERE locktype = 'relation' AND NOT granted AND relation = 'plc_type'::regclass)",
    )
    .fetch_one(watcher)
    .await
    .unwrap()
}

/// Look at `pg_locks` until a session waits for `plc_type` or the migration has ended.
async fn wait_for_the_table_lock(watcher: &mut PgConnection, finished: &Cell<bool>) -> bool {
    for _ in 0..POLL_LIMIT {
        if waits_for_plc_type(watcher).await {
            return true;
        }
        if finished.get() {
            return false;
        }
        tokio::time::sleep(POLL_PAUSE).await;
    }
    false
}

/// `0011` locks both tables first (ADR-0143 (k)), so the check sees the row of a writer that
/// is still open. The old application keeps running during `migrate`.
#[tokio::test]
async fn migration_0011_waits_for_an_open_writer_and_then_checks_its_row() {
    let (mut writer, mut migrator, mut watcher) = three_sessions().await;
    let machine = fixtures::machine(&mut writer, TWIN_CAT).await;
    let int = fixtures::primitive(&mut writer, "INT").await;
    reset_to_the_state_of_0001(&mut writer).await;
    let mut open = writer.begin().await.unwrap();
    insert_array(&mut open, machine, int, "[0:0]={5}", "[0:0]={5}").await;

    let finished = Cell::new(false);
    let migrate = async {
        let outcome = run_0011(&mut migrator).await;
        finished.set(true);
        outcome
    };
    let release = async {
        let waited = wait_for_the_table_lock(&mut watcher, &finished).await;
        open.commit().await.unwrap();
        waited
    };
    let (outcome, waited) = tokio::join!(migrate, release);

    assert!(waited, "0011 never waited for the open writer: {outcome:?}");
    let error = outcome.expect_err("the row check stops the file");
    let message = error
        .as_database_error()
        .expect("a database error")
        .message()
        .to_owned();
    assert!(message.starts_with(STOP_PREFIX), "{message}");
}
