//! Document test of the migration `0003_grants.sql` (design D11, `data-model.md` section 5.2).

use crate::support::{document_sql, migration_sql};

const MIGRATION: &str = "0003_grants.sql";

#[test]
fn migration_0003_equals_the_sql_block_of_section_5_2() {
    let document = document_sql(&["5.2"], &[]);
    let file = migration_sql(MIGRATION);

    assert!(document.starts_with("REVOKE ALL ON SCHEMA public FROM PUBLIC;"));
    assert!(document.ends_with("TO recipe_app;"));
    assert_eq!(file, document);
}
