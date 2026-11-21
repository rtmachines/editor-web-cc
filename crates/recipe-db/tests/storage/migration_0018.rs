//! Tests of the migration `0018_machine_string_encoding_validate.sql` (ADR-0202 (o)).
//!
//! The file validates the two checks that `0017` added as `NOT VALID`.

use crate::support::{Role, TestDatabase, migration_sql};

const MIGRATION: &str = "0018_machine_string_encoding_validate.sql";
const CHECKS: [&str; 2] = [
    "machine_string_encoding_value",
    "machine_string_encoding_platform",
];

#[test]
fn adr_0202_migration_0018_validates_both_checks_of_0017() {
    let file = migration_sql(MIGRATION);

    for name in CHECKS {
        assert!(
            file.contains(&format!("VALIDATE CONSTRAINT {name};")),
            "{MIGRATION} must validate {name}"
        );
    }
}

#[tokio::test]
async fn adr_0202_the_string_encoding_checks_of_machine_are_validated_after_migrate() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;

    let rows: Vec<(String, bool)> = sqlx::query_as(
        "SELECT conname::text, convalidated FROM pg_constraint \
         WHERE conrelid = 'public.machine'::regclass \
           AND conname IN ('machine_string_encoding_value', 'machine_string_encoding_platform') \
         ORDER BY conname",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();

    assert_eq!(
        rows,
        vec![
            ("machine_string_encoding_platform".to_owned(), true),
            ("machine_string_encoding_value".to_owned(), true),
        ]
    );
}
