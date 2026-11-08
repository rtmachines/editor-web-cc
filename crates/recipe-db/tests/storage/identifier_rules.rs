//! Tests of `deploy/db/check-identifier-rules.sql` (ADR-0143, ADR-0145, design D9).
//!
//! The query lists the stored type rows that the identifier rules reject: the rows of a struct
//! name or a member name that differ only in case, an array row with a bound array that starts at
//! another subscript than 1, and a struct with a reserved name. The tests drop the two indexes of
//! ADR-0143 (j) and the array shape constraint, so the rows can exist. They compare the exact rows
//! of the query, run it as `recipe_app`, and show that it writes nothing.

use std::collections::BTreeSet;

use recipe_domain::plant::{Primitive, RESERVED_KEYWORDS};
use sqlx::{Connection as _, PgConnection, Row as _};
use uuid::Uuid;

use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{Role, TestDatabase};

const QUERY_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/db/check-identifier-rules.sql"
);
const LIST_BEGIN: &str = "-- reserved words: begin";
const LIST_END: &str = "-- reserved words: end";
/// The SQLSTATE of a write in a read-only transaction.
const READ_ONLY_SQL_TRANSACTION: &str = "25006";

/// One row of the query: machine code, type name, revision, part, and stored text.
type Row = (String, String, i32, String, String);

/// The rows that the query must list.
#[derive(Default)]
struct Expected(BTreeSet<Row>);

impl Expected {
    fn hit(&mut self, code: &str, type_name: &str, revision: i32, part: &str, text: &str) {
        self.0.insert((
            code.to_owned(),
            type_name.to_owned(),
            revision,
            part.to_owned(),
            text.to_owned(),
        ));
    }
}

/// A machine and its code.
struct Site {
    id: Uuid,
    code: String,
}

async fn site(conn: &mut PgConnection) -> Site {
    let id = fixtures::machine(conn, TWIN_CAT).await;
    let code = sqlx::query_scalar("SELECT code FROM machine WHERE id = $1")
        .bind(id)
        .fetch_one(conn)
        .await
        .unwrap();
    Site { id, code }
}

/// Drop what keeps a stored row from breaking a rule: the two indexes of ADR-0143 (j) and the
/// array shape constraint of ADR-0145.
async fn drop_the_guards(conn: &mut PgConnection) {
    for statement in [
        "DROP INDEX plc_type_struct_name_ci_unique",
        "DROP INDEX plc_type_member_name_ci_unique",
        "ALTER TABLE plc_type DROP CONSTRAINT plc_type_array_shape",
    ] {
        sqlx::query(statement).execute(&mut *conn).await.unwrap();
    }
}

/// Insert a struct revision with the listed members in one transaction.
async fn insert_struct(
    conn: &mut PgConnection,
    site: &Site,
    name: &str,
    revision: i32,
    members: &[&str],
) {
    let int = fixtures::primitive(conn, "INT").await;
    let mut tx = conn.begin().await.unwrap();
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO plc_type (kind, name, platforms, machine_id, revision) \
         VALUES ('struct', $1, ARRAY['TwinCat3'], $2, $3) RETURNING id",
    )
    .bind(name)
    .bind(site.id)
    .bind(revision)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    for (ordinal, member) in (1_i16..).zip(members) {
        sqlx::query(
            "INSERT INTO plc_type_member (struct_type_id, ordinal, member_name, member_type_id) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(ordinal)
        .bind(member)
        .bind(int)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
}

/// Insert an array row of `INT` with literal bound arrays.
async fn insert_array(conn: &mut PgConnection, site: &Site, lower: &str, upper: &str) {
    let int = fixtures::primitive(conn, "INT").await;
    sqlx::query(&format!(
        "INSERT INTO plc_type (kind, platforms, element_type_id, dim_lower, dim_upper, \
             machine_id) \
         VALUES ('array', ARRAY['{TWIN_CAT}'], $1, '{lower}'::integer[], '{upper}'::integer[], $2)"
    ))
    .bind(int)
    .bind(site.id)
    .execute(conn)
    .await
    .unwrap();
}

/// Seed the rows that the query must list.
async fn seed_hits(conn: &mut PgConnection, expected: &mut Expected) {
    let plant = site(conn).await;
    let code = plant.code.as_str();

    insert_struct(conn, &plant, "ST_Zone", 1, &["speed"]).await;
    insert_struct(conn, &plant, "st_zone", 1, &["speed"]).await;
    for name in ["ST_Zone", "st_zone"] {
        expected.hit(code, name, 1, "struct_name_case", name);
    }

    insert_struct(conn, &plant, "ST_Pair", 3, &["speed", "Speed"]).await;
    for member in ["speed", "Speed"] {
        expected.hit(code, "ST_Pair", 3, "member_name_case", member);
    }

    // The lower bounds start at 0 in the first row. The upper bounds start at 0 in the second.
    insert_array(conn, &plant, "[0:0]={1}", "{5}").await;
    expected.hit(code, "ARRAY OF INT", 1, "array_subscript", "[0:0]={1} {5}");
    insert_array(conn, &plant, "{1}", "[0:0]={5}").await;
    expected.hit(code, "ARRAY OF INT", 1, "array_subscript", "{1} [0:0]={5}");

    insert_struct(conn, &plant, "Int", 1, &["speed"]).await;
    expected.hit(code, "Int", 1, "reserved_struct_name", "Int");

    // An archived machine and a reserved word in lower case.
    let archived = site(conn).await;
    insert_struct(conn, &archived, "end_struct", 2, &["speed"]).await;
    sqlx::query("UPDATE machine SET archived_at = now() WHERE id = $1")
        .bind(archived.id)
        .execute(&mut *conn)
        .await
        .unwrap();
    expected.hit(
        &archived.code,
        "end_struct",
        2,
        "reserved_struct_name",
        "end_struct",
    );
}

/// Seed the rows that follow the rules. The query must not list them.
async fn seed_non_hits(conn: &mut PgConnection) {
    let plant = site(conn).await;
    let other = site(conn).await;

    insert_struct(conn, &plant, "ST_Valid", 1, &["speed", "limit"]).await;
    // The same name in another case on another machine and in another revision.
    insert_struct(conn, &other, "st_valid", 1, &["speed"]).await;
    insert_struct(conn, &plant, "st_valid", 2, &["speed"]).await;
    // The same member name in another case in another struct.
    insert_struct(conn, &plant, "ST_Other", 1, &["Speed"]).await;
    // A name that holds a reserved word but is not one.
    insert_struct(conn, &plant, "ST_Int", 1, &["speed"]).await;
    insert_array(conn, &plant, "{1}", "{5}").await;
}

/// A database with every seeded row, and the rows that the query must list.
async fn seeded() -> (TestDatabase, Expected) {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    drop_the_guards(&mut conn).await;
    let mut expected = Expected::default();
    seed_hits(&mut conn, &mut expected).await;
    seed_non_hits(&mut conn).await;
    (database, expected)
}

fn query() -> String {
    std::fs::read_to_string(QUERY_PATH)
        .map_err(|error| format!("cannot read {QUERY_PATH}: {error}"))
        .unwrap()
}

/// Run the file on one connection and return its rows.
async fn rows(conn: &mut PgConnection) -> Vec<Row> {
    let found = sqlx::raw_sql(&query()).fetch_all(&mut *conn).await.unwrap();
    found
        .iter()
        .map(|row| {
            (
                row.try_get("machine_code").unwrap(),
                row.try_get("type_name").unwrap(),
                row.try_get("revision").unwrap(),
                row.try_get("part").unwrap(),
                row.try_get("stored_text").unwrap(),
            )
        })
        .collect()
}

/// One digest of every table that the seed fills, to see that a run changes no row.
async fn fingerprint(conn: &mut PgConnection) -> String {
    sqlx::query_scalar(
        "SELECT md5(string_agg(row_text, ',' ORDER BY row_text)) FROM ( \
             SELECT 'machine' || x::text AS row_text FROM machine x \
             UNION ALL SELECT 'type' || x::text FROM plc_type x \
             UNION ALL SELECT 'member' || x::text FROM plc_type_member x \
             UNION ALL SELECT 'audit' || x::text FROM audit_log x) s",
    )
    .fetch_one(conn)
    .await
    .unwrap()
}

/// The words between the marker lines of the file, one word on each line.
fn words_of_the_file() -> Vec<String> {
    let text = query();
    let (_, after_begin) = text.split_once(LIST_BEGIN).expect("the begin marker line");
    let (list, _) = after_begin
        .split_once(LIST_END)
        .expect("the end marker line after the begin marker line");
    list.lines()
        .map(|line| line.trim().trim_end_matches(',').trim_matches(['(', ')']))
        .filter(|word| !word.is_empty())
        .map(|word| word.trim_matches('\'').to_owned())
        .collect()
}

#[test]
fn identifier_rules_query_lists_the_reserved_words_of_the_domain() {
    let mut expected: BTreeSet<String> = RESERVED_KEYWORDS
        .iter()
        .map(|word| (*word).to_owned())
        .collect();
    expected.extend(
        Primitive::ALL
            .iter()
            .map(|primitive| primitive.name().to_owned()),
    );

    let words = words_of_the_file();
    let listed: BTreeSet<String> = words.iter().cloned().collect();

    assert!(RESERVED_KEYWORDS.len() > 100, "the keyword list is short");
    assert_eq!(
        words.len(),
        listed.len(),
        "a word appears twice in the file"
    );
    assert_eq!(listed, expected);
}

#[tokio::test]
async fn identifier_rules_query_lists_exactly_the_hit_rows_as_recipe_app() {
    let (database, expected) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let user: String = sqlx::query_scalar("SELECT current_user::text")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    let found = rows(&mut conn).await;

    assert_eq!(user, "recipe_app");
    assert_eq!(expected.0.len(), 8, "the seed holds 8 hits");
    assert_eq!(found.len(), expected.0.len(), "rows found: {found:#?}");
    let found: BTreeSet<Row> = found.into_iter().collect();
    assert_eq!(found, expected.0);
}

#[tokio::test]
async fn identifier_rules_query_lists_two_rows_for_each_case_pair() {
    let (database, _) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    let count = |part: &str| found.iter().filter(|row| row.3 == part).count();
    assert_eq!(count("struct_name_case"), 2);
    assert_eq!(count("member_name_case"), 2);
    assert_eq!(count("array_subscript"), 2);
    assert_eq!(count("reserved_struct_name"), 2);
}

#[tokio::test]
async fn identifier_rules_query_lists_each_hit_once_in_the_order_of_the_output() {
    let (database, _) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    let mut sorted = found.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(found.len(), sorted.len(), "a row appears twice");
    // The output sorts by byte order, so the order does not depend on the collation.
    assert_eq!(found, sorted, "the rows are not in the order of the output");
}

#[tokio::test]
async fn identifier_rules_query_writes_nothing() {
    let (database, _) = seeded().await;
    let mut conn = database.connect(Role::App).await;
    let before = fingerprint(&mut conn).await;

    let found = rows(&mut conn).await;
    let after = fingerprint(&mut conn).await;

    assert!(!found.is_empty());
    assert_eq!(before, after);
}

#[tokio::test]
async fn identifier_rules_query_runs_in_a_read_only_transaction() {
    let (database, _) = seeded().await;
    let mut conn = database.connect(Role::App).await;
    let text = query();
    // The file ends with COMMIT. A write that follows the query before COMMIT shows the mode.
    let end = text.rfind("COMMIT").expect("the file ends with COMMIT");
    let probe = format!(
        "{}INSERT INTO machine (code, name, plc_platform) VALUES ('M9999999', 'probe', '{}');",
        &text[..end],
        TWIN_CAT
    );

    let result = sqlx::raw_sql(&probe).execute(&mut conn).await;

    let error = result.expect_err("a write in the transaction of the file fails");
    let code = error
        .as_database_error()
        .and_then(|database_error| database_error.code().map(|code| code.into_owned()));
    assert_eq!(code.as_deref(), Some(READ_ONLY_SQL_TRANSACTION), "{error}");
    sqlx::raw_sql("ROLLBACK").execute(&mut conn).await.unwrap();
    let probes: i64 = sqlx::query_scalar("SELECT count(*) FROM machine WHERE code = 'M9999999'")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    assert_eq!(probes, 0);
}

#[tokio::test]
async fn identifier_rules_query_lists_no_row_on_a_database_without_hits() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let plant = site(&mut conn).await;
    insert_struct(&mut conn, &plant, "ST_Valid", 1, &["speed", "limit"]).await;
    insert_array(&mut conn, &plant, "{1}", "{5}").await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    assert!(found.is_empty(), "rows found: {found:#?}");
}
