//! Test that a duplicate type row gives `E-DB-008` in `RebindTypeRevision` (ADR-0195).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::Primitive;

use super::fixtures::{run, zone_draft, zone_request};
use crate::memory::seed_type_id;
use crate::recipe::test_support::DraftSnapshot;

#[tokio::test]
async fn a_duplicate_type_row_gives_e_db_008_in_rebind_type_revision_and_commits_nothing() {
    let fixture = zone_draft();
    let int = seed_type_id(Primitive::Int, None).unwrap();
    // `OpenDraft::open` reads the rows first, and the plan of the rebind reads them second.
    fixture.store.duplicate_type_row_on_call(2, int);
    let before = DraftSnapshot::capture(&fixture);

    let error = run(&fixture, zone_request()).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008, "{error:?}");
    before.assert_unchanged(&fixture);
}
