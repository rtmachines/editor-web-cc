//! Tests of `deploy/db/check-s7-layout-rules.sql` (ADR-0147, ADR-0148, design D8).
//!
//! The query lists two kinds of stored rows. A `nested_array` row is an array of an array on a
//! machine with `S7_1200` or `S7_1500`, or a parameter whose type tree holds one. A
//! `layout_changed` row is a parameter with an S7 address whose type tree holds a struct with a
//! direct `SINT` or `USINT` member, or an array of `BOOL` with more than one dimension. The tests
//! compare the exact rows of the query, run it as `recipe_app`, and show that it writes nothing.

use std::collections::BTreeSet;

use sqlx::{PgConnection, Row as _};
use uuid::Uuid;

use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{Role, TestDatabase};

const QUERY_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/db/check-s7-layout-rules.sql"
);
/// The SQLSTATE of a write in a read-only transaction.
const READ_ONLY_SQL_TRANSACTION: &str = "25006";
const S7_300: &str = "S7_300";
const S7_1200: &str = "S7_1200";
const S7_1500: &str = "S7_1500";
const NESTED: &str = "nested_array";
const CHANGED: &str = "layout_changed";

/// One row of the query: machine code, reference number, version number, tag, part, and type text.
/// An array row has no reference number, no version number, and no tag.
type Row = (
    String,
    Option<String>,
    Option<i32>,
    Option<String>,
    String,
    String,
);

/// The rows that the query must list.
#[derive(Default)]
struct Expected(BTreeSet<Row>);

impl Expected {
    /// An array row of the type level.
    fn array_row(&mut self, plant: &Plant, part: &str, type_text: &str) {
        self.0.insert((
            plant.code.clone(),
            None,
            None,
            None,
            part.to_owned(),
            type_text.to_owned(),
        ));
    }

    /// A parameter row.
    fn parameter_row(&mut self, plant: &Plant, version: i32, tag: &str, part: &str, text: &str) {
        self.0.insert((
            plant.code.clone(),
            Some(plant.reference.clone()),
            Some(version),
            Some(tag.to_owned()),
            part.to_owned(),
            text.to_owned(),
        ));
    }
}

/// A machine with a unit, a recipe, a live draft of version 1, and the data to fill it.
struct Plant {
    platform: &'static str,
    machine: Uuid,
    code: String,
    unit: Uuid,
    user: Uuid,
    recipe: Uuid,
    reference: String,
    version: Uuid,
}

async fn plant(conn: &mut PgConnection, platform: &'static str) -> Plant {
    let user = fixtures::user(conn).await;
    let machine = fixtures::machine(conn, platform).await;
    let unit = fixtures::unit(conn, machine, "U1").await;
    let recipe = fixtures::recipe(conn, machine, platform, user).await;
    let version = fixtures::draft_version(conn, recipe, platform, 1, user).await;
    let code = sqlx::query_scalar("SELECT code FROM machine WHERE id = $1")
        .bind(machine)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    let reference = sqlx::query_scalar("SELECT reference_number FROM recipe WHERE id = $1")
        .bind(recipe)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    Plant {
        platform,
        machine,
        code,
        unit,
        user,
        recipe,
        reference,
        version,
    }
}

/// Insert an array row of the platform of the plant with one-based bounds.
async fn array_type(
    conn: &mut PgConnection,
    plant: &Plant,
    element: Uuid,
    lower: &[i32],
    upper: &[i32],
) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO plc_type (kind, platforms, element_type_id, dim_lower, dim_upper, machine_id) \
         VALUES ('array', ARRAY[$1], $2, $3, $4, $5) RETURNING id",
    )
    .bind(plant.platform)
    .bind(element)
    .bind(lower.to_vec())
    .bind(upper.to_vec())
    .bind(plant.machine)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Insert a struct with one member of the given type.
async fn struct_of(conn: &mut PgConnection, plant: &Plant, name: &str, member: Uuid) -> Uuid {
    fixtures::complete_struct(conn, plant.machine, plant.platform, name, member).await
}

/// Insert a parameter into the first version of the plant.
async fn parameter(
    conn: &mut PgConnection,
    plant: &Plant,
    type_id: Uuid,
    tag: &str,
    address: Option<&str>,
) {
    fixtures::parameter_with_address(
        conn,
        plant.version,
        plant.platform,
        plant.unit,
        type_id,
        tag,
        address,
    )
    .await;
}

/// The types of the plant that the seed reuses: an array of an array of `INT`.
async fn nested_array(conn: &mut PgConnection, plant: &Plant) -> Uuid {
    let int = fixtures::primitive(conn, "INT").await;
    let inner = array_type(conn, plant, int, &[1], &[2]).await;
    array_type(conn, plant, inner, &[1], &[3]).await
}

/// Seed the rows that the query must list, and fill `expected`.
async fn seed_hits(conn: &mut PgConnection, expected: &mut Expected) {
    let sint = fixtures::primitive(conn, "SINT").await;
    let boolean = fixtures::primitive(conn, "BOOL").await;

    // S7_1500: an array of an array, a struct that holds it, and the three affected shapes.
    let a = plant(conn, S7_1500).await;
    let nested = nested_array(conn, &a).await;
    expected.array_row(&a, NESTED, "ARRAY[1..3] OF ARRAY[1..2] OF INT");
    parameter(conn, &a, nested, "NESTED", Some("DB1.DBB0")).await;
    expected.parameter_row(&a, 1, "NESTED", NESTED, "ARRAY[1..3] OF ARRAY[1..2] OF INT");
    let holder = struct_of(conn, &a, "ST_Hold", nested).await;
    parameter(conn, &a, holder, "HOLDER", Some("DB2.DBB0")).await;
    expected.parameter_row(&a, 1, "HOLDER", NESTED, "ST_Hold");

    let small = struct_of(conn, &a, "ST_Small", sint).await;
    parameter(conn, &a, small, "SMALL", Some("DB1.DBB10")).await;
    expected.parameter_row(&a, 1, "SMALL", CHANGED, "ST_Small");
    let grid = array_type(conn, &a, boolean, &[1, 1], &[3, 3]).await;
    parameter(conn, &a, grid, "GRID", Some("DB1.DBX20.0")).await;
    expected.parameter_row(&a, 1, "GRID", CHANGED, "ARRAY[1..3,1..3] OF BOOL");
    let smalls = array_type(conn, &a, small, &[1], &[2]).await;
    parameter(conn, &a, smalls, "SMALLS", Some("DB1.DBB30")).await;
    expected.parameter_row(&a, 1, "SMALLS", CHANGED, "ARRAY[1..2] OF ST_Small");

    // An approved version and a later draft of the same recipe both count.
    fixtures::approve(conn, a.version, a.user).await;
    let second = fixtures::draft_version(conn, a.recipe, S7_1500, 2, a.user).await;
    fixtures::parameter_with_address(
        conn,
        second,
        S7_1500,
        a.unit,
        small,
        "SMALL_V2",
        Some("DB1.DBB40"),
    )
    .await;
    expected.parameter_row(&a, 2, "SMALL_V2", CHANGED, "ST_Small");

    // S7_300: the address is required, so the affected array is a hit.
    let b = plant(conn, S7_300).await;
    let grid = array_type(conn, &b, boolean, &[1, 1], &[3, 3]).await;
    parameter(conn, &b, grid, "GRID", Some("DB3.DBB0")).await;
    expected.parameter_row(&b, 1, "GRID", CHANGED, "ARRAY[1..3,1..3] OF BOOL");

    // S7_1200 and an archived machine: the array row is a hit, and no parameter uses it.
    let d = plant(conn, S7_1200).await;
    nested_array(conn, &d).await;
    expected.array_row(&d, NESTED, "ARRAY[1..3] OF ARRAY[1..2] OF INT");
    sqlx::query("UPDATE machine SET archived_at = now() WHERE id = $1")
        .bind(d.machine)
        .execute(&mut *conn)
        .await
        .unwrap();
}

/// Seed the rows that the change does not affect. The query must not list them.
async fn seed_non_hits(conn: &mut PgConnection) {
    let int = fixtures::primitive(conn, "INT").await;
    let sint = fixtures::primitive(conn, "SINT").await;
    let boolean = fixtures::primitive(conn, "BOOL").await;

    // S7_1500: shapes that the change leaves alone, also with an address.
    let a = plant(conn, S7_1500).await;
    let small = struct_of(conn, &a, "ST_Small", sint).await;
    parameter(conn, &a, small, "NO_ADDRESS", None).await;
    let flags = array_type(conn, &a, boolean, &[1], &[16]).await;
    parameter(conn, &a, flags, "FLAGS", Some("DB1.DBX0.0")).await;
    let plain = struct_of(conn, &a, "ST_Plain", int).await;
    parameter(conn, &a, plain, "PLAIN", Some("DB1.DBB10")).await;
    let bytes = array_type(conn, &a, sint, &[1], &[4]).await;
    parameter(conn, &a, bytes, "BYTES", Some("DB1.DBB20")).await;

    // S7_300 allows an array of an array.
    let b = plant(conn, S7_300).await;
    let nested = nested_array(conn, &b).await;
    parameter(conn, &b, nested, "NESTED", Some("DB3.DBB0")).await;

    // TwinCat3 allows an array of an array and has no address.
    let c = plant(conn, TWIN_CAT).await;
    let nested = nested_array(conn, &c).await;
    parameter(conn, &c, nested, "NESTED", None).await;
    let small = struct_of(conn, &c, "ST_Small", sint).await;
    parameter(conn, &c, small, "SMALL", None).await;

    // A discarded draft keeps its parameter, but the query skips it.
    let e = plant(conn, S7_1500).await;
    let small = struct_of(conn, &e, "ST_Small", sint).await;
    parameter(conn, &e, small, "SMALL", Some("DB1.DBB0")).await;
    sqlx::query("UPDATE recipe_version SET deleted_at = now() WHERE id = $1")
        .bind(e.version)
        .execute(&mut *conn)
        .await
        .unwrap();
}

/// A database with every seeded row, and the rows that the query must list.
async fn seeded() -> (TestDatabase, Expected) {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
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
                row.try_get("reference_number").unwrap(),
                row.try_get("version_number").unwrap(),
                row.try_get("tag").unwrap(),
                row.try_get("part").unwrap(),
                row.try_get("type_text").unwrap(),
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
             UNION ALL SELECT 'version' || x::text FROM recipe_version x \
             UNION ALL SELECT 'parameter' || x::text FROM parameter x \
             UNION ALL SELECT 'audit' || x::text FROM audit_log x) s",
    )
    .fetch_one(conn)
    .await
    .unwrap()
}

#[tokio::test]
async fn s7_layout_query_lists_exactly_the_hit_rows_as_recipe_app() {
    let (database, expected) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let user: String = sqlx::query_scalar("SELECT current_user::text")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    let found = rows(&mut conn).await;

    assert_eq!(user, "recipe_app");
    assert_eq!(expected.0.len(), 9, "the seed holds 9 hits");
    assert_eq!(found.len(), expected.0.len(), "rows found: {found:#?}");
    let found: BTreeSet<Row> = found.into_iter().collect();
    assert_eq!(found, expected.0);
}

#[tokio::test]
async fn s7_layout_query_lists_a_nested_array_only_on_s7_1200_and_s7_1500() {
    let (database, expected) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    let nested: Vec<&Row> = found.iter().filter(|row| row.4 == NESTED).collect();
    let codes: BTreeSet<&str> = nested.iter().map(|row| row.0.as_str()).collect();
    let expected_codes: BTreeSet<&str> = expected
        .0
        .iter()
        .filter(|row| row.4 == NESTED)
        .map(|row| row.0.as_str())
        .collect();
    assert_eq!(nested.len(), 4, "rows found: {nested:#?}");
    assert_eq!(codes, expected_codes);
    assert_eq!(codes.len(), 2, "one S7_1500 and one S7_1200 machine");
    for code in &codes {
        let platform: String =
            sqlx::query_scalar("SELECT plc_platform FROM machine WHERE code = $1")
                .bind(*code)
                .fetch_one(&mut conn)
                .await
                .unwrap();
        assert!(
            [S7_1200, S7_1500].contains(&platform.as_str()),
            "{platform}"
        );
    }
}

#[tokio::test]
async fn s7_layout_query_lists_one_changed_row_for_each_affected_parameter() {
    let (database, _) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    let changed: Vec<&Row> = found.iter().filter(|row| row.4 == CHANGED).collect();
    let tags: BTreeSet<&str> = changed.iter().filter_map(|row| row.3.as_deref()).collect();
    assert_eq!(changed.len(), 5, "rows found: {changed:#?}");
    assert_eq!(
        tags,
        BTreeSet::from(["SMALL", "GRID", "SMALLS", "SMALL_V2"])
    );
}

#[tokio::test]
async fn s7_layout_query_lists_each_hit_once_in_the_order_of_the_output() {
    let (database, _) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    let mut sorted = found.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(found.len(), sorted.len(), "a row appears twice");
    // The output sorts by byte order and puts an empty value first, so it matches `Ord`.
    assert_eq!(found, sorted, "the rows are not in the order of the output");
}

#[tokio::test]
async fn s7_layout_query_writes_nothing() {
    let (database, _) = seeded().await;
    let mut conn = database.connect(Role::App).await;
    let before = fingerprint(&mut conn).await;

    let found = rows(&mut conn).await;
    let after = fingerprint(&mut conn).await;

    assert!(!found.is_empty());
    assert_eq!(before, after);
}

#[tokio::test]
async fn s7_layout_query_runs_in_a_read_only_transaction() {
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
async fn s7_layout_query_lists_no_row_on_a_database_without_hits() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    seed_non_hits(&mut conn).await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    assert!(found.is_empty(), "rows found: {found:#?}");
}
