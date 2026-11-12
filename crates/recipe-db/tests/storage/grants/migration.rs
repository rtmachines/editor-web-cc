//! Document test of the migrations `0003`, `0012`, and `0013` against the SQL blocks of
//! `data-model.md` section 5.2 (design D11). Block 0 is `0003`, block 1 is `0012`, block 2 is `0013`.

use crate::support::{document_sql, migration_sql};

const MIGRATION: &str = "0003_grants.sql";
const PLC_CONNECTION_MIGRATION: &str = "0012_plc_connection.sql";
const MACHINE_DB_NUMBER_MIGRATION: &str = "0013_machine_db_number.sql";
const GUARD_MIGRATION: &str = "0005_security_event_guard.sql";
const INSERT_COLUMNS_MIGRATION: &str = "0008_audit_log_insert_columns.sql";

#[test]
fn migration_0003_equals_the_sql_block_of_section_5_2() {
    let document = document_sql(&["5.2"], &[("5.2", 1), ("5.2", 2)]);
    let file = migration_sql(MIGRATION);

    assert!(document.starts_with("REVOKE ALL ON SCHEMA public FROM PUBLIC;"));
    assert!(document.ends_with("TO recipe_app;"));
    assert_eq!(file, document);
}

#[test]
fn migration_0012_holds_the_second_sql_block_of_section_5_2() {
    let document = document_sql(&["5.2"], &[("5.2", 0), ("5.2", 2)]);
    let file = migration_sql(PLC_CONNECTION_MIGRATION);

    assert!(!document.is_empty());
    assert!(
        file.contains(&document),
        "0012_plc_connection.sql lacks the second SQL block of section 5.2"
    );
}

#[test]
fn migration_0013_holds_the_third_sql_block_of_section_5_2() {
    let document = document_sql(&["5.2"], &[("5.2", 0), ("5.2", 1)]);
    let file = migration_sql(MACHINE_DB_NUMBER_MIGRATION);

    assert_eq!(
        document,
        "GRANT UPDATE (db_number) ON machine TO recipe_app;"
    );
    assert!(
        file.contains(&document),
        "0013_machine_db_number.sql lacks the third SQL block of section 5.2"
    );
}

#[test]
fn migration_0005_equals_the_sql_block_of_section_5_4() {
    let document = document_sql(&["5.4"], &[]);
    let file = migration_sql(GUARD_MIGRATION);

    assert!(document.starts_with("GRANT CREATE ON SCHEMA public TO audit_owner;"));
    assert!(document.ends_with("REVOKE CREATE ON SCHEMA public FROM audit_owner;"));
    assert_eq!(file, document);
}

#[test]
fn migration_0008_equals_the_sql_block_of_section_5_7() {
    let document = document_sql(&["5.7"], &[]);
    let file = migration_sql(INSERT_COLUMNS_MIGRATION);

    assert!(document.starts_with("GRANT CREATE ON SCHEMA public TO audit_owner;"));
    assert!(document.ends_with("REVOKE CREATE ON SCHEMA public FROM audit_owner;"));
    assert_eq!(file, document);
}
