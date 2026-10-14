//! Tests of the migration `0001_schema.sql` (design D11, `data-model.md` section 3).

use crate::support::{
    Role, TestDatabase, created_names, document_sql, migration_sql, missing_names,
};

const SECTIONS: [&str; 18] = [
    "3.0", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7", "3.8", "3.9", "3.10", "3.11", "3.12",
    "3.13", "3.14", "3.15", "3.16", "3.17",
];
const MIGRATION: &str = "0001_schema.sql";
const TABLE_COUNT: usize = 17;

#[test]
fn migration_0001_equals_the_sql_blocks_of_sections_3_0_to_3_17() {
    let document = document_sql(&SECTIONS, &[]);
    let file = migration_sql(MIGRATION);

    assert!(document.starts_with("CREATE FUNCTION set_updated_at()"));
    assert!(document.ends_with("EXECUTE FUNCTION set_updated_at();"));
    assert_eq!(file, document);
}

#[test]
fn the_document_names_the_seventeen_tables_of_section_3() {
    let names = created_names(&document_sql(&SECTIONS, &[]));

    assert_eq!(names.tables.len(), TABLE_COUNT, "{:?}", names.tables);
    assert!(names.constraints.len() > 80, "{:?}", names.constraints);
}

#[tokio::test]
async fn every_name_of_section_3_exists_in_the_catalog() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let names = created_names(&document_sql(&SECTIONS, &[]));

    let missing = missing_names(&mut conn, &names).await;

    assert!(missing.is_empty(), "missing: {missing:?}");
}

#[tokio::test]
async fn the_audit_chain_head_has_its_one_row_before_the_first_entry() {
    let database = TestDatabase::create().await;
    // `audit_owner` owns the table after 0003. `recipe_migrator` cannot read it.
    let mut conn = database.connect(Role::Superuser).await;

    let rows: Vec<(bool, i64, Option<Vec<u8>>)> =
        sqlx::query_as("SELECT singleton, last_seq, last_hash FROM audit_chain_head")
            .fetch_all(&mut conn)
            .await
            .unwrap();

    assert_eq!(rows, vec![(true, 0, None)]);
}
