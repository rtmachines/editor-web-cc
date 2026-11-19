//! Test that `RebindTypeRevision` beside a stored S7 width defect gives `E-DB-008` (ADR-0195).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{PlcPlatform, Primitive, S7Address};
use recipe_domain::recipe::{StoredParameter, StoredValue};
use serde_json::json;

use super::fixtures::{run, s7_zones_draft, zones_request};
use crate::memory::fixtures::{parameter_on, uid, vid};
use crate::memory::seed_type_id;
use crate::recipe::test_support::{DRAFT_VERSION, DraftSnapshot};

#[tokio::test]
async fn a_rebind_beside_a_stored_width_defect_gives_e_db_008_and_commits_nothing() {
    let fixture = s7_zones_draft();
    let speed = StoredParameter {
        plc_type: seed_type_id(Primitive::Dint, None).unwrap(),
        value: StoredValue::from(json!(5)),
        limits: Vec::new(),
        s7_address: Some(S7Address::parse("DB2.DBW0").unwrap()),
        ..parameter_on(PlcPlatform::S7_300, 14, "SPEED", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &speed);
    let before = DraftSnapshot::capture(&fixture);

    let error = run(&fixture, zones_request()).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008, "{error:?}");
    before.assert_unchanged(&fixture);
}
