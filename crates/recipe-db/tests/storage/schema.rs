//! Tests of the migration `0001_schema.sql` (design D11, `data-model.md` section 3).

use crate::support::{
    Role, TestDatabase, as_shipped_in_0001, check_constraint, created_names, document_sql,
    migration_sql, missing_names,
};

const SECTIONS: [&str; 19] = [
    "3.0", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7", "3.8", "3.9", "3.10", "3.11", "3.12",
    "3.13", "3.14", "3.15", "3.16", "3.17", "3.18",
];
/// The sections that migration 0001 creates. Section 3.18 belongs to migration 0012.
const SECTIONS_OF_0001: [&str; 18] = [
    "3.0", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7", "3.8", "3.9", "3.10", "3.11", "3.12",
    "3.13", "3.14", "3.15", "3.16", "3.17",
];
const MIGRATION: &str = "0001_schema.sql";
const EXPORT_CLIENT_MIGRATION: &str = "0009_export_client_role.sql";
const PLC_CONNECTION_MIGRATION: &str = "0012_plc_connection.sql";
const PLC_CONNECTION_SECTION: &str = "3.18";
/// The named constraints of `plc_connection` in section 3.18, each compared with `0012`.
const PLC_CONNECTION_CHECKS: [&str; 13] = [
    "plc_connection_shape",
    "plc_connection_credentials",
    "plc_connection_rack_range",
    "plc_connection_slot_range",
    "plc_connection_db_number_range",
    "plc_connection_port_range",
    "plc_connection_mode_value",
    "plc_connection_host_length",
    "plc_connection_host_format",
    "plc_connection_address_length",
    "plc_connection_login_length",
    "plc_connection_text_no_control",
    "plc_connection_ams_net_id",
];
const TABLE_COUNT: usize = 18;

#[test]
fn migration_0001_equals_the_sql_blocks_of_sections_3_0_to_3_17() {
    // Migration 0001 shipped before ADR-0109. The document shows the current schema.
    // Section 3.18 is the table of migration 0012, so this comparison leaves it out.
    let document = as_shipped_in_0001(&document_sql(&SECTIONS_OF_0001, &[]));
    let file = migration_sql(MIGRATION);

    assert!(document.starts_with("CREATE FUNCTION set_updated_at()"));
    assert!(document.ends_with("EXECUTE FUNCTION set_updated_at();"));
    assert_eq!(file, document);
}

#[test]
fn migration_0009_adds_the_export_client_constraints_of_section_3_5() {
    let document = document_sql(&["3.5"], &[]);
    let file = migration_sql(EXPORT_CLIENT_MIGRATION);

    for name in ["app_user_role_value", "app_user_export_client_password"] {
        let in_document = check_constraint(&document, name);
        let in_file = check_constraint(&file, name);
        assert_eq!(in_file, in_document, "constraint {name}");
        assert!(
            file.contains(&format!("{in_document} NOT VALID")),
            "constraint {name} must be NOT VALID in 0009"
        );
    }
}

#[test]
fn migration_0012_creates_the_table_of_section_3_18() {
    let document = document_sql(&[PLC_CONNECTION_SECTION], &[]);
    let file = migration_sql(PLC_CONNECTION_MIGRATION);

    // The file lists the named constraints in another order than the document. Each part is
    // therefore compared on its own.
    assert_eq!(table_columns_part(&file), table_columns_part(&document));
    for name in PLC_CONNECTION_CHECKS
        .iter()
        .chain(&["plc_connection_row_version_positive"])
    {
        assert_eq!(
            check_constraint(&file, name),
            check_constraint(&document, name),
            "constraint {name}"
        );
    }
    for statement in [
        "CONSTRAINT plc_connection_machine_unique UNIQUE (machine_id)",
        concat!(
            "CONSTRAINT plc_connection_machine_platform FOREIGN KEY (machine_id, plc_platform) ",
            "REFERENCES machine (id, plc_platform)"
        ),
        concat!(
            "CREATE TRIGGER plc_connection_a_bump BEFORE UPDATE ON plc_connection ",
            "FOR EACH ROW EXECUTE FUNCTION bump_row_version();"
        ),
    ] {
        assert!(document.contains(statement), "document lacks: {statement}");
        assert!(file.contains(statement), "0012 lacks: {statement}");
    }
    assert_eq!(
        created_names(&file).constraints.len(),
        created_names(&document).constraints.len()
    );
}

/// The normalized text of the column list: from `CREATE TABLE plc_connection (` up to the
/// first table constraint.
fn table_columns_part(normalized: &str) -> &str {
    let start = normalized
        .find("CREATE TABLE plc_connection (")
        .expect("no CREATE TABLE plc_connection");
    let tail = normalized.get(start..).expect("start is a char boundary");
    let end = tail.find(" CONSTRAINT ").expect("no table constraint");
    tail.get(..end).expect("end is a char boundary")
}

#[test]
fn the_document_names_the_eighteen_tables_of_section_3() {
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
