//! Tests of `deploy/db/check-s7-layout-rules.sql` (ADR-0147, ADR-0148, design D8).
//!
//! The query lists five kinds of stored rows. A `nested_array` row is an array of an array on a
//! machine with `S7_300`, `S7_1200`, or `S7_1500`, or a parameter whose type tree holds one. A
//! `layout_changed` row is a parameter with an S7 address whose type tree holds a struct with a
//! direct `SINT` or `USINT` member, or an array of `BOOL` with more than one dimension. An
//! `array_bound` row is a parameter with an S7 address whose type tree holds an array bound outside
//! `-32768..=32767` (ADR-0178). A `bool_array_3d` row is a parameter with an S7 address whose type
//! tree holds an array of `BOOL` with three dimensions (ADR-0187). A `nesting` row is a struct row
//! on an S7 machine, or a parameter with an S7 platform, whose type has more than 8 levels of the
//! S7 count (ADR-0179). The tests compare the exact rows of the query, run it as `recipe_app`, and
//! show that it writes nothing.

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
const BOUND: &str = "array_bound";
const CUBE: &str = "bool_array_3d";
const NESTING: &str = "nesting";
/// The largest S7 count of a type that the rule accepts (`type.s7_max_nesting`, ADR-0179 (a)).
const MAX_NESTING: usize = 8;

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

/// Insert a chain of `levels` structs `ST_N1` to `ST_N<levels>`. Each struct holds the one before
/// it, and `ST_N1` holds an `INT`. The type `ST_N<n>` has `n` levels. The call returns the ids.
async fn struct_chain(conn: &mut PgConnection, plant: &Plant, levels: usize) -> Vec<Uuid> {
    let mut member = fixtures::primitive(conn, "INT").await;
    let mut chain = Vec::new();
    for level in 1..=levels {
        member = struct_of(conn, plant, &format!("ST_N{level}"), member).await;
        chain.push(member);
    }
    chain
}

/// Seed the rows of the part `nesting` and fill `expected`.
async fn seed_nesting_hits(conn: &mut PgConnection, expected: &mut Expected) {
    // S7_1500: a chain of 9 structs. The struct row with 9 levels and a parameter of it are hits.
    // An array of a struct with 7 levels counts 7 + 2 = 9 levels. Only its parameter is a hit.
    // The struct with 8 levels is not a hit.
    let f = plant(conn, S7_1500).await;
    let chain = struct_chain(conn, &f, MAX_NESTING + 1).await;
    let deepest = chain[MAX_NESTING];
    let eighth = chain[MAX_NESTING - 1];
    let seventh = chain[MAX_NESTING - 2];
    expected.array_row(&f, NESTING, "ST_N9@1");
    parameter(conn, &f, deepest, "DEEP", Some("DB5.DBB0")).await;
    expected.parameter_row(&f, 1, "DEEP", NESTING, "ST_N9@1");
    let wide = array_type(conn, &f, seventh, &[1], &[2]).await;
    parameter(conn, &f, wide, "DEEP_ARRAY", None).await;
    expected.parameter_row(&f, 1, "DEEP_ARRAY", NESTING, "ARRAY[1..2] OF ST_N7@1");
    parameter(conn, &f, eighth, "EDGE", Some("DB5.DBB10")).await;
}

/// Seed the rows of the part `nesting` that the query must not list.
async fn seed_nesting_non_hits(conn: &mut PgConnection) {
    // S7_1200: a chain of 8 structs and an array of a struct with 6 levels, 8 levels in all.
    let g = plant(conn, S7_1200).await;
    let chain = struct_chain(conn, &g, MAX_NESTING).await;
    parameter(conn, &g, chain[MAX_NESTING - 1], "EIGHT", Some("DB1.DBB0")).await;
    let array = array_type(conn, &g, chain[MAX_NESTING - 3], &[1], &[2]).await;
    parameter(conn, &g, array, "EIGHT_ARRAY", Some("DB1.DBB10")).await;

    // TwinCat3 has no limit on the levels.
    let h = plant(conn, TWIN_CAT).await;
    let chain = struct_chain(conn, &h, MAX_NESTING + 1).await;
    parameter(conn, &h, chain[MAX_NESTING], "DEEP", None).await;
}

/// Seed a discarded draft. Its parameters stay stored. The parts `array_bound`, `bool_array_3d`,
/// and `nesting` list them (ADR-0178 (j), ADR-0187 (i), ADR-0179 (k)). The older parts skip them.
async fn seed_discarded_hits(conn: &mut PgConnection, expected: &mut Expected) {
    let boolean = fixtures::primitive(conn, "BOOL").await;
    let byte = fixtures::primitive(conn, "BYTE").await;
    let f = plant(conn, S7_1500).await;
    let wide = array_type(conn, &f, byte, &[32760], &[32770]).await;
    parameter(conn, &f, wide, "WIDE_DISCARDED", Some("DB1.DBB0")).await;
    expected.parameter_row(
        &f,
        1,
        "WIDE_DISCARDED",
        BOUND,
        "ARRAY[32760..32770] OF BYTE",
    );
    let cube = array_type(conn, &f, boolean, &[1, 1, 1], &[2, 2, 2]).await;
    parameter(conn, &f, cube, "CUBE_DISCARDED", Some("DB1.DBX10.0")).await;
    expected.parameter_row(
        &f,
        1,
        "CUBE_DISCARDED",
        CUBE,
        "ARRAY[1..2,1..2,1..2] OF BOOL",
    );
    let chain = struct_chain(conn, &f, MAX_NESTING + 1).await;
    parameter(
        conn,
        &f,
        chain[MAX_NESTING],
        "DEEP_DISCARDED",
        Some("DB1.DBB20"),
    )
    .await;
    expected.array_row(&f, NESTING, "ST_N9@1");
    expected.parameter_row(&f, 1, "DEEP_DISCARDED", NESTING, "ST_N9@1");
    sqlx::query("UPDATE recipe_version SET deleted_at = now() WHERE id = $1")
        .bind(f.version)
        .execute(&mut *conn)
        .await
        .unwrap();
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

    // ADR-0178 and ADR-0187: a bound of 32770, the same array inside a struct, and a BOOL array
    // with three dimensions. The three dimensions are also a change of the layout.
    let byte = fixtures::primitive(conn, "BYTE").await;
    let wide = array_type(conn, &a, byte, &[32760], &[32770]).await;
    parameter(conn, &a, wide, "WIDE", Some("DB1.DBB100")).await;
    expected.parameter_row(&a, 1, "WIDE", BOUND, "ARRAY[32760..32770] OF BYTE");
    let wide_holder = struct_of(conn, &a, "ST_Wide", wide).await;
    parameter(conn, &a, wide_holder, "WIDE_HOLD", Some("DB1.DBB200")).await;
    expected.parameter_row(&a, 1, "WIDE_HOLD", BOUND, "ST_Wide");
    let cube = array_type(conn, &a, boolean, &[1, 1, 1], &[2, 2, 2]).await;
    parameter(conn, &a, cube, "CUBE", Some("DB1.DBX300.0")).await;
    expected.parameter_row(&a, 1, "CUBE", CUBE, "ARRAY[1..2,1..2,1..2] OF BOOL");
    expected.parameter_row(&a, 1, "CUBE", CHANGED, "ARRAY[1..2,1..2,1..2] OF BOOL");

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

    // S7_300: the address is required, so the affected array is a hit. An array of an array is a
    // hit too (ADR-0177 (g)).
    let b = plant(conn, S7_300).await;
    let nested = nested_array(conn, &b).await;
    expected.array_row(&b, NESTED, "ARRAY[1..3] OF ARRAY[1..2] OF INT");
    parameter(conn, &b, nested, "NESTED", Some("DB3.DBB10")).await;
    expected.parameter_row(&b, 1, "NESTED", NESTED, "ARRAY[1..3] OF ARRAY[1..2] OF INT");
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
    seed_nesting_hits(conn, expected).await;
    seed_discarded_hits(conn, expected).await;
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

    // ADR-0178 and ADR-0187: the same shapes without an address, and the bounds at the limits.
    let byte = fixtures::primitive(conn, "BYTE").await;
    let wide = array_type(conn, &a, byte, &[32760], &[32770]).await;
    parameter(conn, &a, wide, "WIDE_NO_ADDRESS", None).await;
    let cube = array_type(conn, &a, boolean, &[1, 1, 1], &[2, 2, 2]).await;
    parameter(conn, &a, cube, "CUBE_NO_ADDRESS", None).await;
    let limits = array_type(conn, &a, byte, &[-32768], &[32767]).await;
    parameter(conn, &a, limits, "LIMITS", Some("DB1.DBB400")).await;

    // TwinCat3 allows an array of an array and has no address.
    let c = plant(conn, TWIN_CAT).await;
    let nested = nested_array(conn, &c).await;
    parameter(conn, &c, nested, "NESTED", None).await;
    let small = struct_of(conn, &c, "ST_Small", sint).await;
    parameter(conn, &c, small, "SMALL", None).await;
    let wide = array_type(conn, &c, byte, &[32760], &[32770]).await;
    parameter(conn, &c, wide, "WIDE", None).await;

    // A discarded draft keeps its parameter, but the query skips it.
    let e = plant(conn, S7_1500).await;
    let small = struct_of(conn, &e, "ST_Small", sint).await;
    parameter(conn, &e, small, "SMALL", Some("DB1.DBB0")).await;
    sqlx::query("UPDATE recipe_version SET deleted_at = now() WHERE id = $1")
        .bind(e.version)
        .execute(&mut *conn)
        .await
        .unwrap();
    seed_nesting_non_hits(conn).await;
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
    assert_eq!(expected.0.len(), 22, "the seed holds 22 hits");
    assert_eq!(found.len(), expected.0.len(), "rows found: {found:#?}");
    let found: BTreeSet<Row> = found.into_iter().collect();
    assert_eq!(found, expected.0);
}

#[tokio::test]
async fn adr_0177_s7_layout_query_lists_a_nested_array_on_each_s7_platform() {
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
    assert_eq!(nested.len(), 6, "rows found: {nested:#?}");
    assert_eq!(codes, expected_codes);
    assert_eq!(
        codes.len(),
        3,
        "one S7_1500, one S7_1200, and one S7_300 machine"
    );
    for code in &codes {
        let platform: String =
            sqlx::query_scalar("SELECT plc_platform FROM machine WHERE code = $1")
                .bind(*code)
                .fetch_one(&mut conn)
                .await
                .unwrap();
        assert!(
            [S7_300, S7_1200, S7_1500].contains(&platform.as_str()),
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
    assert_eq!(changed.len(), 6, "rows found: {changed:#?}");
    assert_eq!(
        tags,
        BTreeSet::from(["SMALL", "GRID", "SMALLS", "SMALL_V2", "CUBE"])
    );
}

#[tokio::test]
async fn adr_0178_s7_layout_query_lists_a_parameter_with_an_address_and_a_wide_bound() {
    let (database, expected) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    let bound: Vec<&Row> = found.iter().filter(|row| row.4 == BOUND).collect();
    let tags: BTreeSet<&str> = bound.iter().filter_map(|row| row.3.as_deref()).collect();
    assert_eq!(bound.len(), 3, "rows found: {bound:#?}");
    assert_eq!(
        tags,
        BTreeSet::from(["WIDE", "WIDE_HOLD", "WIDE_DISCARDED"]),
        "a discarded draft stays in the inventory"
    );
    let expected_bound = expected.0.iter().filter(|row| row.4 == BOUND).count();
    assert_eq!(bound.len(), expected_bound);
}

#[tokio::test]
async fn adr_0187_s7_layout_query_lists_a_parameter_with_an_address_and_a_three_dimension_bool() {
    let (database, _) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    let cube: Vec<&Row> = found.iter().filter(|row| row.4 == CUBE).collect();
    let tags: BTreeSet<&str> = cube.iter().filter_map(|row| row.3.as_deref()).collect();
    assert_eq!(cube.len(), 2, "rows found: {cube:#?}");
    assert_eq!(
        tags,
        BTreeSet::from(["CUBE", "CUBE_DISCARDED"]),
        "a discarded draft stays in the inventory"
    );
    assert!(
        cube.iter()
            .all(|row| row.5 == "ARRAY[1..2,1..2,1..2] OF BOOL")
    );
}

#[tokio::test]
async fn adr_0179_s7_layout_query_lists_a_struct_row_and_a_parameter_with_nine_levels() {
    let (database, expected) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    let nesting: BTreeSet<&Row> = found.iter().filter(|row| row.4 == NESTING).collect();
    let expected_nesting: BTreeSet<&Row> =
        expected.0.iter().filter(|row| row.4 == NESTING).collect();
    assert_eq!(nesting.len(), 5, "rows found: {nesting:#?}");
    assert_eq!(nesting, expected_nesting);
    let tags: BTreeSet<Option<&str>> = nesting.iter().map(|row| row.3.as_deref()).collect();
    assert_eq!(
        tags,
        BTreeSet::from([
            None,
            Some("DEEP"),
            Some("DEEP_ARRAY"),
            Some("DEEP_DISCARDED")
        ]),
        "the struct row has no tag"
    );
}

#[tokio::test]
async fn adr_0179_s7_layout_query_tells_two_revisions_of_one_struct_apart() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let f = plant(&mut conn, S7_1500).await;
    let chain = struct_chain(&mut conn, &f, MAX_NESTING + 1).await;
    let eighth = chain[MAX_NESTING - 1];
    let mut tx = sqlx::Connection::begin(&mut conn).await.unwrap();
    let second_revision: Uuid = sqlx::query_scalar(
        "INSERT INTO plc_type (kind, name, platforms, machine_id, revision) \
         VALUES ('struct', 'ST_N9', ARRAY[$1], $2, 2) RETURNING id",
    )
    .bind(f.platform)
    .bind(f.machine)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    fixtures::member(&mut tx, second_revision, 1, eighth).await;
    tx.commit().await.unwrap();
    let array = array_type(&mut conn, &f, second_revision, &[1], &[2]).await;
    parameter(&mut conn, &f, array, "TWO", None).await;
    let mut expected = Expected::default();
    expected.array_row(&f, NESTING, "ST_N9@1");
    expected.array_row(&f, NESTING, "ST_N9@2");
    expected.parameter_row(&f, 1, "TWO", NESTING, "ARRAY[1..2] OF ST_N9@2");

    let found = rows(&mut database.connect(Role::App).await).await;

    let nesting: BTreeSet<Row> = found.into_iter().filter(|row| row.4 == NESTING).collect();
    assert_eq!(nesting, expected.0);
}

#[tokio::test]
async fn adr_0179_s7_layout_query_skips_eight_levels_and_a_platform_without_the_limit() {
    let (database, _) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    let texts: BTreeSet<&str> = found
        .iter()
        .filter(|row| row.4 == NESTING)
        .map(|row| row.5.as_str())
        .collect();
    assert_eq!(texts, BTreeSet::from(["ST_N9@1", "ARRAY[1..2] OF ST_N7@1"]));
    let tags: BTreeSet<&str> = found.iter().filter_map(|row| row.3.as_deref()).collect();
    assert!(!tags.contains("EDGE"), "a type with 8 levels is listed");
    assert!(!tags.contains("EIGHT"), "a type with 8 levels is listed");
    assert!(
        !tags.contains("EIGHT_ARRAY"),
        "a type with 8 levels is listed"
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
