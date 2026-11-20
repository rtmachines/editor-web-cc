//! Tests of the migration `0017_machine_string_encoding.sql` (ADR-0202 (n), (o)).
//!
//! The first tests compare the column, the checks, and the grant with `data-model.md` sections 3.1
//! and 5.2. The other tests run the file on a schema that is put back to the state of `0016`.

use sqlx::PgConnection;

use crate::support::{
    CHECK_VIOLATION, Role, TestDatabase, assert_constraint, check_constraint, document_sql,
    migration_sql,
};

const MIGRATION: &str = "0017_machine_string_encoding.sql";
const CHECKS: [&str; 2] = [
    "machine_string_encoding_value",
    "machine_string_encoding_platform",
];
/// One stored machine of each platform: the code, the platform, and the data block number.
const STORED: [(&str, &str, Option<i32>); 5] = [
    ("M7300", "S7_300", Some(10)),
    ("M7301", "S7_1200", Some(10)),
    ("M7302", "S7_1500", Some(10)),
    ("M7303", "CodesysIec", None),
    ("M7304", "TwinCat3", None),
];

#[test]
fn adr_0202_migration_0017_adds_the_checks_of_section_3_1_as_not_valid() {
    let document = document_sql(&["3.1"], &[]);
    let file = migration_sql(MIGRATION);

    for name in CHECKS {
        let in_document = check_constraint(&document, name);
        let in_file = check_constraint(&file, name);
        assert_eq!(in_file, in_document, "constraint {name}");
        assert!(
            file.contains(&format!("{in_document} NOT VALID")),
            "constraint {name} must be NOT VALID in 0017"
        );
    }
}

#[test]
fn adr_0202_migration_0017_adds_the_column_of_section_3_1_and_the_grant_of_section_5_2() {
    let document = document_sql(&["3.1"], &[]);
    let file = migration_sql(MIGRATION);

    assert!(
        document.contains(" string_encoding TEXT NOT NULL DEFAULT 'Latin1',"),
        "{document}"
    );
    assert!(
        file.contains(
            "ALTER TABLE machine ADD COLUMN string_encoding TEXT NOT NULL DEFAULT 'Latin1';"
        ),
        "0017 lacks the column"
    );
    assert!(
        file.contains("GRANT UPDATE (string_encoding) ON machine TO recipe_app;"),
        "0017 lacks the grant"
    );
}

/// Return a migrated database connection as the owner of the tables.
async fn connection() -> PgConnection {
    TestDatabase::create()
        .await
        .into_session(Role::Migrator)
        .await
}

/// Put the schema back to the state before `0017`: no column and no constraint.
async fn reset_to_the_state_of_0016(conn: &mut PgConnection) {
    sqlx::raw_sql(
        "ALTER TABLE machine \
         DROP CONSTRAINT IF EXISTS machine_string_encoding_value, \
         DROP CONSTRAINT IF EXISTS machine_string_encoding_platform, \
         DROP COLUMN IF EXISTS string_encoding",
    )
    .execute(conn)
    .await
    .unwrap();
}

/// Run the whole text of `0017` as one simple query, so it commits or rolls back as one unit.
async fn run_0017(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/0017_machine_string_encoding.sql"
    ))
    .unwrap();
    sqlx::raw_sql(&text).execute(conn).await.map(|_| ())
}

/// Insert a stored machine as the previous release writes it: no `string_encoding` column.
async fn insert_stored_machine(
    conn: &mut PgConnection,
    code: &str,
    platform: &str,
    db_number: Option<i32>,
) {
    sqlx::query(
        "INSERT INTO machine (code, name, plc_platform, db_number) VALUES ($1, 'Press', $2, $3)",
    )
    .bind(code)
    .bind(platform)
    .bind(db_number)
    .execute(conn)
    .await
    .unwrap();
}

#[tokio::test]
async fn adr_0202_migration_0017_gives_a_stored_machine_of_each_platform_latin1() {
    let mut conn = connection().await;
    reset_to_the_state_of_0016(&mut conn).await;
    for (code, platform, number) in STORED {
        insert_stored_machine(&mut conn, code, platform, number).await;
    }

    run_0017(&mut conn).await.expect("0017 runs");

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT plc_platform, string_encoding FROM machine WHERE code = ANY($1) ORDER BY code",
    )
    .bind(STORED.map(|(code, _, _)| code).to_vec())
    .fetch_all(&mut conn)
    .await
    .unwrap();
    let expected: Vec<(String, String)> = STORED
        .iter()
        .map(|(_, platform, _)| ((*platform).to_owned(), "Latin1".to_owned()))
        .collect();
    assert_eq!(rows, expected);
}

#[tokio::test]
async fn adr_0202_migration_0017_adds_both_checks_as_not_valid() {
    let mut conn = connection().await;
    reset_to_the_state_of_0016(&mut conn).await;

    run_0017(&mut conn).await.expect("0017 runs");

    let states: Vec<(String, bool)> = sqlx::query_as(
        "SELECT conname::text, convalidated FROM pg_constraint \
         WHERE conrelid = 'machine'::regclass \
           AND conname IN ('machine_string_encoding_value', 'machine_string_encoding_platform') \
         ORDER BY conname",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();
    assert_eq!(
        states,
        vec![
            ("machine_string_encoding_platform".to_owned(), false),
            ("machine_string_encoding_value".to_owned(), false),
        ]
    );
}

#[tokio::test]
async fn adr_0202_migration_0017_grants_update_of_the_column_to_recipe_app() {
    let mut conn = connection().await;
    reset_to_the_state_of_0016(&mut conn).await;

    run_0017(&mut conn).await.expect("0017 runs");

    let granted: bool = sqlx::query_scalar(
        "SELECT has_column_privilege('recipe_app', 'machine', 'string_encoding', 'UPDATE')",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();
    assert!(granted, "recipe_app has UPDATE (string_encoding)");
}

#[tokio::test]
async fn adr_0202_an_insert_without_string_encoding_gets_latin1_after_0017() {
    let mut conn = connection().await;
    reset_to_the_state_of_0016(&mut conn).await;
    run_0017(&mut conn).await.expect("0017 runs");

    // The previous release names no `string_encoding` column.
    insert_stored_machine(&mut conn, "M7305", "TwinCat3", None).await;
    insert_stored_machine(&mut conn, "M7306", "S7_1500", Some(10)).await;

    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT string_encoding FROM machine WHERE code IN ('M7305', 'M7306') ORDER BY code",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();
    assert_eq!(rows, vec!["Latin1".to_owned(), "Latin1".to_owned()]);
}

#[tokio::test]
async fn adr_0202_the_not_valid_checks_of_0017_refuse_a_new_bad_row() {
    let mut conn = connection().await;
    reset_to_the_state_of_0016(&mut conn).await;
    run_0017(&mut conn).await.expect("0017 runs");

    let result = sqlx::query(
        "INSERT INTO machine (code, name, plc_platform, db_number, string_encoding) \
         VALUES ('M7307', 'Press', 'S7_1500', 10, 'Utf8')",
    )
    .execute(&mut conn)
    .await;

    assert_constraint(result, CHECK_VIOLATION, "machine_string_encoding_platform");
}
