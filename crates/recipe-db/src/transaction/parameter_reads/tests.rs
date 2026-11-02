//! Tests of the SQL text of the parameter reads.

use super::SELECT_PAGE;

/// A fast check that needs no database. It cannot see `p.*`.
/// The storage test `postgres_only_the_page_statement_returns_exactly_the_row_columns` reads the
/// real output columns.
#[test]
fn parameter_reads_page_statement_selects_no_value() {
    // `GetVersion` calls `parameter_page` for a draft with no row limit (design D1 step 1).
    // `ParameterRow` has no value field, so the statement must not read the column either.
    assert!(!SELECT_PAGE.contains("value"), "{SELECT_PAGE}");
}
