//! Tests of the migration `0015_machine_db_number_validate.sql` (ADR-0156, design D2 item 3).
//!
//! The file validates the two checks that `0013` added as `NOT VALID`.

use crate::support::{Role, TestDatabase, migration_sql};

const MIGRATION: &str = "0015_machine_db_number_validate.sql";
const CHECKS: [&str; 2] = ["machine_db_number_shape", "machine_db_number_range"];

#[test]
fn migration_0015_validates_both_checks_of_0013() {
    let file = migration_sql(MIGRATION);

    for name in CHECKS {
        assert!(
            file.contains(&format!("VALIDATE CONSTRAINT {name};")),
            "{MIGRATION} must validate {name}"
        );
    }
}

#[tokio::test]
async fn the_db_number_checks_of_machine_are_validated_after_migrate() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;

    let rows: Vec<(String, bool)> = sqlx::query_as(
        "SELECT conname::text, convalidated FROM pg_constraint \
         WHERE conrelid = 'public.machine'::regclass \
           AND conname IN ('machine_db_number_shape', 'machine_db_number_range') \
         ORDER BY conname",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();

    assert_eq!(
        rows,
        vec![
            ("machine_db_number_range".to_owned(), true),
            ("machine_db_number_shape".to_owned(), true),
        ]
    );
}
