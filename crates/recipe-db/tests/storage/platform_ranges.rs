//! Tests of `deploy/db/check-platform-ranges.sql` (ADR-0139 risks, ADR-0140 risks).
//!
//! The query lists stored data that the current rule set rejects: an `LDATE` or `LDT` value above
//! `9223372036854775807` on `CodesysIec` or `TwinCat3`, a limit above it, and an S7 data block above
//! 59999 on `S7_1200` or `S7_1500`. The tests seed hits and non-hits on the harness database. They
//! compare the exact rows of the query, run it as `recipe_app`, and show that it writes nothing.

use std::collections::BTreeSet;

use sqlx::{Connection as _, PgConnection, Row as _};
use uuid::Uuid;

use crate::support::fixtures::{self, TWIN_CAT};
use crate::support::{Role, TestDatabase};

const QUERY_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/db/check-platform-ranges.sql"
);
const CODESYS: &str = "CodesysIec";
const S7_1200: &str = "S7_1200";
const S7_1500: &str = "S7_1500";
const S7_300: &str = "S7_300";

/// The first count above the new end of `LDT` (ADR-0139 (a)).
const LDT_ABOVE: &str = "9223372036854775808";
/// The new end of `LDT` and `LDATE` (ADR-0139 (a)).
const END: &str = "9223372036854775807";
/// 2262-04-12 as a count of nanoseconds, the first day above the new end of `LDATE`.
const LDATE_ABOVE: &str = "9223372800000000000";
/// The count of 2262-04-11, the last day of the new range of `LDATE`.
const LDATE_LAST: &str = "9223286400000000000";
/// A count far above the new end of `LDT`.
const LDT_FAR_ABOVE: &str = "10413792000000000000";
/// `u64::MAX`, the end of `ULINT`.
const ULINT_END: &str = "18446744073709551615";
/// The SQLSTATE of a write in a read-only transaction.
const READ_ONLY_SQL_TRANSACTION: &str = "25006";

/// The identity of one row of the query: machine code, reference number, version number, status,
/// tag, part, and stored text.
type Row = (String, String, i32, String, String, String, String);

/// What happens to a version after the seed has filled it.
#[derive(Clone, Copy)]
enum Fate {
    Draft,
    DeletedDraft,
    Approved,
    Retired,
}

/// A machine with a unit, a recipe, and a live draft with number 1.
struct Site {
    platform: &'static str,
    machine: Uuid,
    unit: Uuid,
    user: Uuid,
    recipe: Uuid,
    version: Uuid,
}

/// The place of a version in the output: machine code, reference number, number, and status.
struct Place {
    code: String,
    reference: String,
    number: i32,
    status: String,
}

/// The rows that the query must list, in the form of the output.
#[derive(Default)]
struct Expected(BTreeSet<Row>);

impl Expected {
    fn hit(&mut self, place: &Place, tag: &str, part: &str, text: &str) {
        self.0.insert((
            place.code.clone(),
            place.reference.clone(),
            place.number,
            place.status.clone(),
            tag.to_owned(),
            part.to_owned(),
            text.to_owned(),
        ));
    }
}

async fn site(conn: &mut PgConnection, platform: &'static str) -> Site {
    let user = fixtures::user(conn).await;
    let machine = fixtures::machine(conn, platform).await;
    let unit = fixtures::unit(conn, machine, "U1").await;
    let recipe = fixtures::recipe(conn, machine, platform, user).await;
    let version = fixtures::draft_version(conn, recipe, platform, 1, user).await;
    Site {
        platform,
        machine,
        unit,
        user,
        recipe,
        version,
    }
}

/// Insert a parameter of a type with a stored value and an optional S7 address.
async fn add(
    conn: &mut PgConnection,
    site: &Site,
    plc_type: Uuid,
    tag: &str,
    address: Option<&str>,
    value: &str,
) -> Uuid {
    let id = fixtures::parameter_with_address(
        conn,
        site.version,
        site.platform,
        site.unit,
        plc_type,
        tag,
        address,
    )
    .await;
    sqlx::query("UPDATE parameter SET value = $2::text::jsonb WHERE id = $1")
        .bind(id)
        .bind(value)
        .execute(conn)
        .await
        .unwrap();
    id
}

/// Insert a limit of the whole parameter.
async fn add_limit(
    conn: &mut PgConnection,
    site: &Site,
    parameter: Uuid,
    min: Option<&str>,
    max: Option<&str>,
) {
    sqlx::query(
        "INSERT INTO parameter_limit (parameter_id, recipe_version_id, min_value, max_value) \
         VALUES ($1, $2, $3::text::numeric, $4::text::numeric)",
    )
    .bind(parameter)
    .bind(site.version)
    .bind(min)
    .bind(max)
    .execute(conn)
    .await
    .unwrap();
}

/// Insert an array type of one dimension with the bounds 1 to `upper`.
async fn array_of(conn: &mut PgConnection, site: &Site, element: Uuid, upper: i32) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO plc_type (kind, platforms, element_type_id, dim_lower, dim_upper, machine_id) \
         VALUES ('array', ARRAY[$1], $2, ARRAY[1], ARRAY[$3::integer], $4) RETURNING id",
    )
    .bind(site.platform)
    .bind(element)
    .bind(upper)
    .bind(site.machine)
    .fetch_one(conn)
    .await
    .unwrap()
}

/// Insert a struct `MIXED` with the members `m1` `STRING`, `m2` `BOOL`, and `m3` `LDT`.
async fn mixed_struct(conn: &mut PgConnection, site: &Site) -> Uuid {
    let string = fixtures::machine_string(conn, site.machine, site.platform, 20).await;
    let boolean = fixtures::primitive(conn, "BOOL").await;
    let ldt = fixtures::primitive(conn, "LDT").await;
    let mut tx = conn.begin().await.unwrap();
    let id = fixtures::struct_type(&mut tx, site.machine, site.platform, "MIXED").await;
    fixtures::member(&mut tx, id, 1, string).await;
    fixtures::member(&mut tx, id, 2, boolean).await;
    fixtures::member(&mut tx, id, 3, ldt).await;
    tx.commit().await.unwrap();
    id
}

/// Change the version as the fate says, and return its place in the output.
async fn settle(conn: &mut PgConnection, site: &Site, fate: Fate) -> Place {
    match fate {
        Fate::Draft => {}
        Fate::DeletedDraft => {
            sqlx::query("UPDATE recipe_version SET deleted_at = now() WHERE id = $1")
                .bind(site.version)
                .execute(&mut *conn)
                .await
                .unwrap();
        }
        Fate::Approved => fixtures::approve(conn, site.version, site.user).await,
        Fate::Retired => {
            fixtures::approve(conn, site.version, site.user).await;
            fixtures::retire(conn, site.version).await;
        }
    }
    let row = sqlx::query(
        "SELECT m.code, r.reference_number, v.number, v.status \
         FROM recipe_version v JOIN recipe r ON r.id = v.recipe_id \
         JOIN machine m ON m.id = r.machine_id WHERE v.id = $1",
    )
    .bind(site.version)
    .fetch_one(conn)
    .await
    .unwrap();
    Place {
        code: row.get(0),
        reference: row.get(1),
        number: row.get(2),
        status: row.get(3),
    }
}

/// Seed the value hits, the nested hits, and the soft-deleted hits.
async fn seed_value_hits(conn: &mut PgConnection, expected: &mut Expected) {
    seed_scalar_and_array_hits(conn, expected).await;
    seed_nested_hit(conn, expected).await;
    seed_deleted_and_archived_hits(conn, expected).await;
}

/// Seed a scalar hit in a draft and an approved version. Seed an array element hit in a retired version.
async fn seed_scalar_and_array_hits(conn: &mut PgConnection, expected: &mut Expected) {
    let ldt = fixtures::primitive(conn, "LDT").await;
    let ldate = fixtures::primitive(conn, "LDATE").await;

    let draft = site(conn, TWIN_CAT).await;
    add(conn, &draft, ldt, "VALUE_LDT_DRAFT", None, LDT_ABOVE).await;
    let place = settle(conn, &draft, Fate::Draft).await;
    expected.hit(&place, "VALUE_LDT_DRAFT", "value", LDT_ABOVE);

    let approved = site(conn, CODESYS).await;
    add(
        conn,
        &approved,
        ldate,
        "VALUE_LDATE_APPROVED",
        None,
        LDATE_ABOVE,
    )
    .await;
    let place = settle(conn, &approved, Fate::Approved).await;
    expected.hit(&place, "VALUE_LDATE_APPROVED", "value", LDATE_ABOVE);

    let retired = site(conn, TWIN_CAT).await;
    let array = array_of(conn, &retired, ldt, 2).await;
    let value = format!("[0,{LDT_FAR_ABOVE}]");
    add(conn, &retired, array, "ARRAY_LDT_RETIRED", None, &value).await;
    let place = settle(conn, &retired, Fate::Retired).await;
    expected.hit(&place, "ARRAY_LDT_RETIRED", "value", LDT_FAR_ABOVE);
}

/// Seed a hit in a struct member that sits two levels deep, in a deleted draft.
async fn seed_nested_hit(conn: &mut PgConnection, expected: &mut Expected) {
    let ldt = fixtures::primitive(conn, "LDT").await;

    let nested = site(conn, TWIN_CAT).await;
    let inner = fixtures::complete_struct(conn, nested.machine, TWIN_CAT, "INNER", ldt).await;
    let outer = fixtures::complete_struct(conn, nested.machine, TWIN_CAT, "OUTER", inner).await;
    let value = format!("{{\"m1\":{{\"m1\":{LDT_ABOVE}}}}}");
    add(
        conn,
        &nested,
        outer,
        "NESTED_LDT_DELETED_DRAFT",
        None,
        &value,
    )
    .await;
    let place = settle(conn, &nested, Fate::DeletedDraft).await;
    expected.hit(&place, "NESTED_LDT_DELETED_DRAFT", "value", LDT_ABOVE);
}

/// Seed a hit under a deleted recipe and a hit on an archived machine.
async fn seed_deleted_and_archived_hits(conn: &mut PgConnection, expected: &mut Expected) {
    let ldt = fixtures::primitive(conn, "LDT").await;

    let mixed = site(conn, TWIN_CAT).await;
    let struct_type = mixed_struct(conn, &mixed).await;
    let value = format!("{{\"m1\":\"text\",\"m2\":true,\"m3\":{LDT_FAR_ABOVE}}}");
    add(
        conn,
        &mixed,
        struct_type,
        "MIXED_LDT_DELETED_RECIPE",
        None,
        &value,
    )
    .await;
    sqlx::query("UPDATE recipe SET deleted_at = now() WHERE id = $1")
        .bind(mixed.recipe)
        .execute(&mut *conn)
        .await
        .unwrap();
    let place = settle(conn, &mixed, Fate::Draft).await;
    expected.hit(&place, "MIXED_LDT_DELETED_RECIPE", "value", LDT_FAR_ABOVE);

    let archived = site(conn, TWIN_CAT).await;
    add(
        conn,
        &archived,
        ldt,
        "VALUE_LDT_ARCHIVED_MACHINE",
        None,
        LDT_FAR_ABOVE,
    )
    .await;
    sqlx::query("UPDATE machine SET archived_at = now() WHERE id = $1")
        .bind(archived.machine)
        .execute(&mut *conn)
        .await
        .unwrap();
    let place = settle(conn, &archived, Fate::Draft).await;
    expected.hit(&place, "VALUE_LDT_ARCHIVED_MACHINE", "value", LDT_FAR_ABOVE);
}

/// Seed the limit hits and the address hits.
async fn seed_limit_and_address_hits(conn: &mut PgConnection, expected: &mut Expected) {
    let ldt = fixtures::primitive(conn, "LDT").await;
    let int = fixtures::primitive(conn, "INT").await;

    let max = site(conn, TWIN_CAT).await;
    let parameter = add(conn, &max, ldt, "LIMIT_MAX", None, "0").await;
    add_limit(conn, &max, parameter, None, Some(LDT_FAR_ABOVE)).await;
    let place = settle(conn, &max, Fate::Draft).await;
    expected.hit(&place, "LIMIT_MAX", "limit_max", LDT_FAR_ABOVE);

    let min = site(conn, CODESYS).await;
    let parameter = add(conn, &min, ldt, "LIMIT_MIN_ONLY", None, "0").await;
    add_limit(conn, &min, parameter, Some(LDT_ABOVE), None).await;
    let place = settle(conn, &min, Fate::Draft).await;
    expected.hit(&place, "LIMIT_MIN_ONLY", "limit_min", LDT_ABOVE);

    let s7_1500 = site(conn, S7_1500).await;
    add(
        conn,
        &s7_1500,
        int,
        "BLOCK_60000_S7_1500",
        Some("DB60000.DBW0"),
        "0",
    )
    .await;
    let place = settle(conn, &s7_1500, Fate::Draft).await;
    expected.hit(&place, "BLOCK_60000_S7_1500", "s7_address", "DB60000.DBW0");

    let s7_1200 = site(conn, S7_1200).await;
    add(
        conn,
        &s7_1200,
        int,
        "BLOCK_65535_S7_1200",
        Some("DB65535.DBW0"),
        "0",
    )
    .await;
    let place = settle(conn, &s7_1200, Fate::Draft).await;
    expected.hit(&place, "BLOCK_65535_S7_1200", "s7_address", "DB65535.DBW0");
}

/// Seed the rows that the query must not list.
async fn seed_non_hits(conn: &mut PgConnection) {
    seed_boundary_non_hits(conn).await;
    seed_platform_non_hits(conn).await;
    seed_wide_non_hit(conn).await;
}

/// Seed the values and limits that equal the end of the range. The query must not list them.
async fn seed_boundary_non_hits(conn: &mut PgConnection) {
    let ldt = fixtures::primitive(conn, "LDT").await;
    let ldate = fixtures::primitive(conn, "LDATE").await;

    let at_end = site(conn, TWIN_CAT).await;
    add(conn, &at_end, ldt, "NO_HIT_LDT_END", None, END).await;
    settle(conn, &at_end, Fate::Draft).await;

    let last_day = site(conn, CODESYS).await;
    add(
        conn,
        &last_day,
        ldate,
        "NO_HIT_LDATE_LAST_DAY",
        None,
        LDATE_LAST,
    )
    .await;
    settle(conn, &last_day, Fate::Approved).await;

    let mixed = site(conn, TWIN_CAT).await;
    let struct_type = mixed_struct(conn, &mixed).await;
    let value = format!("{{\"m1\":\"text\",\"m2\":false,\"m3\":{END}}}");
    add(conn, &mixed, struct_type, "NO_HIT_MIXED_END", None, &value).await;
    settle(conn, &mixed, Fate::Draft).await;

    let limit = site(conn, TWIN_CAT).await;
    let parameter = add(conn, &limit, ldt, "NO_HIT_LIMIT_MAX_END", None, "0").await;
    add_limit(conn, &limit, parameter, None, Some(END)).await;
    settle(conn, &limit, Fate::Draft).await;
}

/// Seed the rows of the platforms that the check excludes. The query must not list them.
async fn seed_platform_non_hits(conn: &mut PgConnection) {
    let ldt = fixtures::primitive(conn, "LDT").await;
    let int = fixtures::primitive(conn, "INT").await;

    // The check is for CodesysIec and TwinCat3. S7_1500 has LDT, and its end is another rule.
    let other = site(conn, S7_1500).await;
    add(conn, &other, ldt, "NO_HIT_LDT_S7_1500", None, LDT_FAR_ABOVE).await;
    settle(conn, &other, Fate::Draft).await;

    let block_59999 = site(conn, S7_1200).await;
    add(
        conn,
        &block_59999,
        int,
        "NO_HIT_BLOCK_59999",
        Some("DB59999.DBW0"),
        "0",
    )
    .await;
    settle(conn, &block_59999, Fate::Draft).await;

    let s7_300 = site(conn, S7_300).await;
    add(
        conn,
        &s7_300,
        int,
        "NO_HIT_BLOCK_S7_300",
        Some("DB60000.DBW0"),
        "0",
    )
    .await;
    settle(conn, &s7_300, Fate::Draft).await;
}

/// Seed a ULINT value at the end of its own range. The query must not list it.
async fn seed_wide_non_hit(conn: &mut PgConnection) {
    let ulint = fixtures::primitive(conn, "ULINT").await;

    let wide = site(conn, TWIN_CAT).await;
    add(conn, &wide, ulint, "NO_HIT_ULINT_END", None, ULINT_END).await;
    settle(conn, &wide, Fate::Draft).await;
}

/// A database with every seeded row, and the rows that the query must list.
async fn seeded() -> (TestDatabase, Expected) {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let mut expected = Expected::default();
    seed_value_hits(&mut conn, &mut expected).await;
    seed_limit_and_address_hits(&mut conn, &mut expected).await;
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
                row.try_get("version_status").unwrap(),
                row.try_get("tag").unwrap(),
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
             UNION ALL SELECT 'recipe' || x::text FROM recipe x \
             UNION ALL SELECT 'version' || x::text FROM recipe_version x \
             UNION ALL SELECT 'parameter' || x::text FROM parameter x \
             UNION ALL SELECT 'limit' || x::text FROM parameter_limit x \
             UNION ALL SELECT 'type' || x::text FROM plc_type x \
             UNION ALL SELECT 'audit' || x::text FROM audit_log x) s",
    )
    .fetch_one(conn)
    .await
    .unwrap()
}

#[tokio::test]
async fn platform_ranges_query_lists_exactly_the_hit_rows_as_recipe_app() {
    let (database, expected) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let user: String = sqlx::query_scalar("SELECT current_user::text")
        .fetch_one(&mut conn)
        .await
        .unwrap();
    let found = rows(&mut conn).await;

    assert_eq!(user, "recipe_app");
    assert_eq!(expected.0.len(), 10, "the seed holds 10 hits");
    assert_eq!(found.len(), expected.0.len(), "rows found: {found:#?}");
    let found: BTreeSet<Row> = found.into_iter().collect();
    assert_eq!(found, expected.0);
}

#[tokio::test]
async fn platform_ranges_query_lists_each_hit_once_in_the_order_of_the_output() {
    let (database, _) = seeded().await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    let mut sorted = found.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(found.len(), sorted.len(), "a row appears twice");
    let keys = |row: &Row| {
        (
            row.0.clone(),
            row.1.clone(),
            row.2,
            row.4.clone(),
            row.5.clone(),
        )
    };
    let mut by_key = found.clone();
    by_key.sort_by_key(keys);
    assert_eq!(found, by_key, "the rows are not in the order of the output");
}

#[tokio::test]
async fn platform_ranges_query_writes_nothing() {
    let (database, _) = seeded().await;
    let mut conn = database.connect(Role::App).await;
    let before = fingerprint(&mut conn).await;

    let found = rows(&mut conn).await;
    let after = fingerprint(&mut conn).await;

    assert!(!found.is_empty());
    assert_eq!(before, after);
}

#[tokio::test]
async fn platform_ranges_query_runs_in_a_read_only_transaction() {
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
async fn platform_ranges_query_lists_no_row_on_a_database_without_hits() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::App).await;

    let found = rows(&mut conn).await;

    assert!(found.is_empty(), "rows found: {found:#?}");
}
