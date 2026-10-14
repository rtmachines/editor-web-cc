//! Tests of the migration `0002_triggers.sql` (design D11, `data-model.md` section 4).
//!
//! A trigger test uses a session of the table owner `recipe_migrator`. `recipe_app` has no grant
//! before `0003_grants.sql`. The grant tests repeat the forbidden transitions as `recipe_app`.

mod budget;
mod frozen;
pub mod hash_chain;
mod immutable_types;
mod transition;
mod version_mutable;

use crate::support::{
    Role, TestDatabase, created_names, document_sql, migration_sql, missing_names,
};

/// The sections that `0002_triggers.sql` holds so far, in the order of the document.
const SECTIONS: [&str; 11] = [
    "4.1", "4.2", "4.3", "4.4", "4.5", "4.6", "4.7", "4.8", "4.9", "4.10", "4.12",
];
/// Blocks of the listed sections that are not part of the migration.
/// The second block of 4.9 holds the statements of the nightly check.
const SKIPPED_BLOCKS: [(&str, usize); 1] = [("4.9", 1)];
const MIGRATION: &str = "0002_triggers.sql";

#[test]
fn migration_0002_equals_the_sql_blocks_of_its_sections() {
    let document = document_sql(&SECTIONS, &SKIPPED_BLOCKS);
    let file = migration_sql(MIGRATION);

    assert!(document.starts_with("CREATE FUNCTION enforce_version_mutable()"));
    assert_eq!(file, document);
}

#[tokio::test]
async fn every_function_and_trigger_of_0002_exists_in_the_catalog() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Migrator).await;
    let names = created_names(&document_sql(&SECTIONS, &SKIPPED_BLOCKS));

    let missing = missing_names(&mut conn, &names).await;

    assert!(!names.triggers.is_empty() && !names.functions.is_empty());
    assert!(missing.is_empty(), "missing: {missing:?}");
}
