//! Tests of the migration `0002_triggers.sql` (design D11, `data-model.md` section 4).
//!
//! A trigger test uses a session of the table owner `recipe_migrator`. `recipe_app` has no grant
//! before `0003_grants.sql`. The grant tests repeat the forbidden transitions as `recipe_app`.

mod budget;
mod frozen;
pub mod hash_chain;
mod immutable_types;
mod transition;
mod version_insert;
mod version_mutable;

use crate::support::{
    Role, TestDatabase, created_names, document_sql, function_statement, migration_sql,
    missing_names, without_function_statement,
};

/// The sections that `0002_triggers.sql` holds so far, in the order of the document.
const SECTIONS: [&str; 11] = [
    "4.1", "4.2", "4.3", "4.4", "4.5", "4.6", "4.7", "4.8", "4.9", "4.10", "4.12",
];
/// Blocks of the listed sections that are not part of the migration.
/// The second block of 4.9 holds the statements of the nightly check.
const SKIPPED_BLOCKS: [(&str, usize); 1] = [("4.9", 1)];
const MIGRATION: &str = "0002_triggers.sql";
const REPLACING_MIGRATION: &str = "0019_struct_member_transaction.sql";
const SHIPPED_HEAD: &str = "CREATE FUNCTION plc_type_immutable";
const REPLACING_HEAD: &str = "CREATE OR REPLACE FUNCTION plc_type_immutable";

/// Migration 0019 replaces `plc_type_immutable` (ADR-0212), so section 4.6 shows the 0019 text.
/// This test leaves that function out on both sides.
/// The test `migration_0019_equals_the_function_of_section_4_6` compares it.
/// The other statements of 4.6, such as the triggers, stay in the comparison.
#[test]
fn migration_0002_equals_the_sql_blocks_of_its_sections() {
    let document = document_sql(&SECTIONS, &SKIPPED_BLOCKS);
    let file = migration_sql(MIGRATION);

    assert!(document.starts_with("CREATE FUNCTION enforce_version_mutable()"));
    assert_eq!(
        without_function_statement(&file, SHIPPED_HEAD),
        without_function_statement(&document, REPLACING_HEAD)
    );
}

#[test]
fn migration_0019_equals_the_function_of_section_4_6() {
    let document = document_sql(&["4.6"], &[]);
    let file = migration_sql(REPLACING_MIGRATION);

    let in_document = function_statement(&document, REPLACING_HEAD);

    assert!(
        in_document.contains("pg_current_xact_id()"),
        "section 4.6 must test the creating transaction: {in_document}"
    );
    assert_eq!(function_statement(&file, REPLACING_HEAD), in_document);
    assert_eq!(
        file, in_document,
        "0019 holds the function and nothing else"
    );
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
