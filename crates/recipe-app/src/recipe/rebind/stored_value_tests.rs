//! Test that `RebindTypeRevision` beside a stored value of the wrong shape gives `E-DB-008`
//! (ADR-0195 (l), (n)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::PlcPlatform;
use serde_json::json;

use super::fixtures::{ZONE_1, empty_draft, run, seed_parameter, zone_request};
use crate::recipe::test_support::DraftSnapshot;

#[tokio::test]
async fn a_rebind_of_a_struct_value_that_is_no_object_gives_e_db_008_and_commits_nothing() {
    let fixture = empty_draft();
    seed_parameter(
        &fixture,
        PlcPlatform::TwinCat3,
        (11, "ZONE1"),
        (ZONE_1, json!(5)),
        Vec::new(),
    );
    let before = DraftSnapshot::capture(&fixture);

    let error = run(&fixture, zone_request()).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008, "{error:?}");
    before.assert_unchanged(&fixture);
}
