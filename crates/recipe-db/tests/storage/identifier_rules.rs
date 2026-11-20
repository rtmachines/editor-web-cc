//! Tests of `deploy/db/check-identifier-rules.sql` (ADR-0143, ADR-0145, design D9).
//!
//! The query lists the stored type rows that the identifier rules reject: the rows of a struct
//! name or a member name that differ only in case, an array row with a bound array that starts at
//! another subscript than 1, a struct with a reserved name, a struct with a Siemens reserved name
//! on an S7 machine (ADR-0186), and a struct with a reserved name or a member with a keyword name
//! on a CODESYS or TwinCAT machine (ADR-0203), and a struct with one member on a CODESYS machine
//! (ADR-0204). The tests drop the two indexes of
//! ADR-0143 (j) and the array shape constraint, so the rows can exist. They compare the exact rows
//! of the query, run it as `recipe_app`, and show that it writes nothing.

use std::collections::BTreeSet;

use recipe_domain::plant::{
    IEC_EXTRA_KEYWORDS, IEC_RESERVED_STRUCT_NAMES, Primitive, RESERVED_KEYWORDS,
    SIEMENS_RESERVED_NAMES, is_siemens_reserved_name,
};
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
const SIEMENS_BEGIN: &str = "-- siemens names: begin";
const SIEMENS_END: &str = "-- siemens names: end";
const PATTERN_BEGIN: &str = "-- siemens telegram pattern: begin";
const PATTERN_END: &str = "-- siemens telegram pattern: end";
const IEC_STRUCT_BEGIN: &str = "-- iec struct names: begin";
const IEC_STRUCT_END: &str = "-- iec struct names: end";
const PRIMITIVE_BEGIN: &str = "-- primitive names: begin";
const PRIMITIVE_END: &str = "-- primitive names: end";
const IEC_EXTRA_BEGIN: &str = "-- iec extra keywords: begin";
const IEC_EXTRA_END: &str = "-- iec extra keywords: end";
/// The part of a stored struct with a reserved struct name on CODESYS or TwinCAT (ADR-0203 (m)).
const IEC_STRUCT_PART: &str = "iec_reserved_struct_name";
/// The part of a stored member with a reserved keyword name on CODESYS or TwinCAT (ADR-0203 (m)).
const IEC_MEMBER_PART: &str = "iec_keyword_member_name";
/// The part of a stored struct with one member on CODESYS (ADR-0204 (h)).
const ONE_MEMBER_PART: &str = "codesys_one_member_struct";
/// The S7 platforms of ADR-0186 (a).
const S7_1500: &str = "S7_1500";
const CODESYS: &str = "CodesysIec";
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

/// A machine, its code, and its platform.
struct Site {
    id: Uuid,
    code: String,
    platform: &'static str,
}

async fn site(conn: &mut PgConnection) -> Site {
    site_on(conn, TWIN_CAT).await
}

async fn site_on(conn: &mut PgConnection, platform: &'static str) -> Site {
    let id = fixtures::machine(conn, platform).await;
    let code = sqlx::query_scalar("SELECT code FROM machine WHERE id = $1")
        .bind(id)
        .fetch_one(conn)
        .await
        .unwrap();
    Site { id, code, platform }
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
         VALUES ('struct', $1, ARRAY[$4], $2, $3) RETURNING id",
    )
    .bind(name)
    .bind(site.id)
    .bind(revision)
    .bind(site.platform)
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

    // A Siemens reserved name is a hit on S7 only (ADR-0186). The name `PD_TEL3_IN` is no hit.
    let s7 = site_on(conn, S7_1500).await;
    let codesys = site_on(conn, CODESYS).await;
    for name in ["PID_Compact", "PD_TEL3", "pd_tel03"] {
        insert_struct(conn, &s7, name, 1, &["speed"]).await;
        insert_struct(conn, &codesys, name, 1, &["speed", "limit"]).await;
        expected.hit(&s7.code, name, 1, "s7_reserved_struct_name", name);
    }
    insert_struct(conn, &s7, "PD_TEL3_IN", 1, &["speed"]).await;

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

/// The words between two marker lines of the file, one word on each line.
fn words_between(begin: &str, end: &str) -> Vec<String> {
    let text = query();
    let (_, after_begin) = text.split_once(begin).expect("the begin marker line");
    let (list, _) = after_begin
        .split_once(end)
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

    let words = words_between(LIST_BEGIN, LIST_END);
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
    assert_eq!(expected.0.len(), 11, "the seed holds 11 hits");
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
    assert_eq!(count("s7_reserved_struct_name"), 3);
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

#[test]
fn adr_0186_query_lists_the_siemens_names_of_the_domain() {
    let expected: BTreeSet<String> = SIEMENS_RESERVED_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();

    let words = words_between(SIEMENS_BEGIN, SIEMENS_END);
    let listed: BTreeSet<String> = words.iter().cloned().collect();

    assert_eq!(expected.len(), 269, "the domain list is short");
    assert_eq!(
        words.len(),
        listed.len(),
        "a name appears twice in the file"
    );
    assert_eq!(listed, expected);
}

#[tokio::test]
async fn adr_0186_query_pattern_matches_the_telegram_family_of_the_domain() {
    let lines = words_between(PATTERN_BEGIN, PATTERN_END);
    assert_eq!(
        lines.len(),
        1,
        "one pattern between the marker lines: {lines:?}"
    );
    let pattern = lines.first().expect("the pattern line");
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::App).await;
    let probes = [
        "PD_TEL3",
        "pd_tel3",
        "Pd_Tel12",
        "PD_TEL0",
        "PD_TEL03",
        "PD_TEL003",
        "PD_TELx",
        "PD_TEL",
        "PD_TEL3_IN",
        "XPD_TEL3",
        "PD_TEL3 ",
        "PD_TEL\u{663}",
    ];

    for probe in probes {
        let in_pattern: bool = sqlx::query_scalar("SELECT $1 ~* $2")
            .bind(probe)
            .bind(pattern)
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(
            in_pattern,
            is_siemens_reserved_name(probe),
            "the pattern and the domain differ on {probe:?}"
        );
    }
}

/// The rows of the two parts of ADR-0203 (m), as a set.
fn iec_rows(found: Vec<Row>) -> BTreeSet<Row> {
    found
        .into_iter()
        .filter(|row| row.3 == IEC_STRUCT_PART || row.3 == IEC_MEMBER_PART)
        .collect()
}

/// Insert a struct revision with the given member names.
async fn insert_struct_with(
    conn: &mut PgConnection,
    site: &Site,
    name: &str,
    revision: i32,
    members: &[String],
) {
    let names: Vec<&str> = members.iter().map(String::as_str).collect();
    insert_struct(conn, site, name, revision, &names).await;
}

#[tokio::test]
async fn adr_0203_query_lists_the_iec_rows_and_no_s7_row() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let twincat = site_on(&mut conn, TWIN_CAT).await;
    let s7 = site_on(&mut conn, S7_1500).await;
    let codesys = site_on(&mut conn, CODESYS).await;
    insert_struct(&mut conn, &twincat, "VERSION", 1, &["speed"]).await;
    insert_struct(&mut conn, &s7, "VERSION", 1, &["speed"]).await;
    insert_struct(&mut conn, &codesys, "ST_Keyed", 1, &["END_IF", "speed"]).await;
    insert_struct(&mut conn, &s7, "ST_Keyed", 1, &["END_IF"]).await;
    let mut expected = Expected::default();
    expected.hit(&twincat.code, "VERSION", 1, IEC_STRUCT_PART, "VERSION");
    expected.hit(&codesys.code, "ST_Keyed", 1, IEC_MEMBER_PART, "END_IF");

    let mut conn = database.connect(Role::App).await;
    let found = rows(&mut conn).await;

    assert_eq!(found.len(), 2, "rows found: {found:#?}");
    assert_eq!(iec_rows(found), expected.0);
}

#[tokio::test]
async fn adr_0203_query_lists_every_stored_struct_and_member_of_the_domain_lists() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let codesys = site_on(&mut conn, CODESYS).await;
    let twincat = site_on(&mut conn, TWIN_CAT).await;
    let s7 = site_on(&mut conn, S7_1500).await;
    let archived = site_on(&mut conn, TWIN_CAT).await;
    let mut expected = Expected::default();

    // Each struct name, in the case of the list on CODESYS and in lower case on TwinCAT.
    // A struct of another revision and a struct of an archived machine are hits too.
    for name in IEC_RESERVED_STRUCT_NAMES {
        let lower = name.to_ascii_lowercase();
        insert_struct(&mut conn, &codesys, name, 1, &["speed"]).await;
        insert_struct(&mut conn, &twincat, &lower, 1, &["speed"]).await;
        insert_struct(&mut conn, &s7, name, 1, &["speed"]).await;
        expected.hit(&codesys.code, name, 1, IEC_STRUCT_PART, name);
        expected.hit(&twincat.code, &lower, 1, IEC_STRUCT_PART, &lower);
    }
    insert_struct(&mut conn, &codesys, "VERSION", 2, &["speed"]).await;
    expected.hit(&codesys.code, "VERSION", 2, IEC_STRUCT_PART, "VERSION");
    insert_struct(&mut conn, &archived, "BIT", 1, &["speed"]).await;
    sqlx::query("UPDATE machine SET archived_at = now() WHERE id = $1")
        .bind(archived.id)
        .execute(&mut conn)
        .await
        .unwrap();
    expected.hit(&archived.code, "BIT", 1, IEC_STRUCT_PART, "BIT");

    // Each keyword as a member name, in the case of the list on CODESYS, in lower case on
    // TwinCAT, and in no row on S7.
    let keywords: Vec<String> = RESERVED_KEYWORDS
        .iter()
        .chain(IEC_EXTRA_KEYWORDS.iter())
        .map(|word| (*word).to_owned())
        .collect();
    let lower: Vec<String> = keywords.iter().map(|word| word.to_lowercase()).collect();
    insert_struct_with(&mut conn, &codesys, "ST_Keywords", 1, &keywords).await;
    insert_struct_with(&mut conn, &twincat, "ST_Keywords", 4, &lower).await;
    insert_struct_with(&mut conn, &s7, "ST_Keywords", 1, &keywords).await;
    for word in &keywords {
        expected.hit(&codesys.code, "ST_Keywords", 1, IEC_MEMBER_PART, word);
    }
    for word in &lower {
        expected.hit(&twincat.code, "ST_Keywords", 4, IEC_MEMBER_PART, word);
    }

    // A primitive name and a type name of ADR-0203 (c2) are no reserved member names.
    let allowed: Vec<String> = Primitive::ALL
        .iter()
        .map(|primitive| primitive.name().to_owned())
        .chain(
            IEC_RESERVED_STRUCT_NAMES
                .iter()
                .skip(2)
                .map(|name| (*name).to_owned()),
        )
        .collect();
    insert_struct_with(&mut conn, &codesys, "ST_Allowed", 1, &allowed).await;
    insert_struct_with(&mut conn, &twincat, "ST_Allowed", 1, &allowed).await;

    let mut conn = database.connect(Role::App).await;
    let found = iec_rows(rows(&mut conn).await);

    assert_eq!(expected.0.len(), 30 + 1 + 1 + 184 * 2);
    assert_eq!(found, expected.0);
}

#[test]
fn adr_0203_sql_lists_equal_the_domain_lists() {
    let structs = words_between(IEC_STRUCT_BEGIN, IEC_STRUCT_END);
    let expected: BTreeSet<String> = IEC_RESERVED_STRUCT_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    assert_eq!(expected.len(), 15, "the domain list is short");
    assert_eq!(structs.len(), expected.len(), "a name appears twice");
    assert_eq!(structs.into_iter().collect::<BTreeSet<_>>(), expected);

    let extras = words_between(IEC_EXTRA_BEGIN, IEC_EXTRA_END);
    let expected: BTreeSet<String> = IEC_EXTRA_KEYWORDS
        .iter()
        .map(|word| (*word).to_owned())
        .collect();
    assert_eq!(extras.len(), expected.len(), "a word appears twice");
    assert_eq!(extras.into_iter().collect::<BTreeSet<_>>(), expected);

    // The member keywords are the reserved words without the primitive names, and the extras.
    // The 182 keywords and the 29 primitive names are disjoint, so the 211 reserved words of the
    // file less the primitive names are the 182 keywords of the domain.
    let primitives = words_between(PRIMITIVE_BEGIN, PRIMITIVE_END);
    let expected: BTreeSet<String> = Primitive::ALL
        .iter()
        .map(|primitive| primitive.name().to_owned())
        .collect();
    assert_eq!(primitives.len(), expected.len(), "a name appears twice");
    assert_eq!(primitives.into_iter().collect::<BTreeSet<_>>(), expected);
    let keywords: BTreeSet<&str> = RESERVED_KEYWORDS.iter().copied().collect();
    assert_eq!(keywords.len(), 182);
    assert!(
        expected
            .iter()
            .all(|name| !keywords.contains(name.as_str())),
        "a primitive name is a keyword"
    );
}

/// The rows of the part of ADR-0204 (h), as a set.
fn one_member_rows(found: Vec<Row>) -> BTreeSet<Row> {
    found
        .into_iter()
        .filter(|row| row.3 == ONE_MEMBER_PART)
        .collect()
}

#[tokio::test]
async fn adr_0204_one_member_struct_lists_only_codesys() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let codesys = site_on(&mut conn, CODESYS).await;
    let twincat = site_on(&mut conn, TWIN_CAT).await;
    let s7 = site_on(&mut conn, S7_1500).await;
    let archived = site_on(&mut conn, CODESYS).await;
    let mut expected = Expected::default();

    // A one-member struct on CODESYS is a hit, also in a later revision, also with a name that
    // no parameter uses, and also on an archived machine. The member name is no part of the row.
    insert_struct(&mut conn, &codesys, "ST_One", 1, &["speed"]).await;
    insert_struct(&mut conn, &codesys, "ST_One", 2, &["limit"]).await;
    insert_struct(&mut conn, &codesys, "ST_Unused", 1, &["speed"]).await;
    insert_struct(&mut conn, &archived, "ST_Old", 1, &["speed"]).await;
    sqlx::query("UPDATE machine SET archived_at = now() WHERE id = $1")
        .bind(archived.id)
        .execute(&mut conn)
        .await
        .unwrap();
    for (site, name, revision) in [
        (&codesys, "ST_One", 1),
        (&codesys, "ST_One", 2),
        (&codesys, "ST_Unused", 1),
        (&archived, "ST_Old", 1),
    ] {
        expected.hit(&site.code, name, revision, ONE_MEMBER_PART, name);
    }

    // The same shape is no hit on TwinCAT and on S7. A struct with two members is no hit.
    insert_struct(&mut conn, &twincat, "ST_One", 1, &["speed"]).await;
    insert_struct(&mut conn, &s7, "ST_One", 1, &["speed"]).await;
    insert_struct(&mut conn, &codesys, "ST_Two", 1, &["speed", "limit"]).await;
    insert_struct(&mut conn, &twincat, "ST_Two", 1, &["speed", "limit"]).await;

    let mut conn = database.connect(Role::App).await;
    let found = rows(&mut conn).await;

    assert_eq!(
        one_member_rows(found),
        expected.0,
        "only the one-member structs of CODESYS machines are listed"
    );
}
