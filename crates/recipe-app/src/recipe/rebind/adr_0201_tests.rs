//! Tests of `RebindTypeRevision` for the rule "WSTRING characters on CODESYS and TwinCAT"
//! (ADR-0201 (d)): the value of a new `WSTRING` member.

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{PlcPlatform, Primitive};
use serde_json::json;

use super::fixtures::{
    request, run, seed_parameter, stored, typed_struct_row, with_text, with_value,
};
use crate::memory::fixtures::{Fixture, tid};
use crate::recipe::rebind::fixtures::primitive_id;
use crate::recipe::test_support::{
    DraftSnapshot, NOTE_TYPE, expected_forms, iec_wstring, wstring_row,
};

const CFG_1: u128 = 990;
const CFG_2: u128 = 991;
/// The parameter number of `CFG`.
const CFG: u128 = 22;

/// A draft on `platform` with `CFG` of type `ST_Cfg@1`.
///
/// `ST_Cfg@1` has the members `speed` and `mode` of type `INT`. `ST_Cfg@2` adds the member
/// `note` of type `WSTRING(10)`.
fn cfg_draft(platform: PlcPlatform) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let int = primitive_id(Primitive::Int);
    fixture
        .store
        .seed_type(&wstring_row(NOTE_TYPE, platform, 10));
    fixture.store.seed_type(&typed_struct_row(
        platform,
        (CFG_1, "ST_Cfg", 1),
        None,
        &[("speed", int), ("mode", int)],
    ));
    fixture.store.seed_type(&typed_struct_row(
        platform,
        (CFG_2, "ST_Cfg", 2),
        Some(CFG_1),
        &[("speed", int), ("mode", int), ("note", tid(NOTE_TYPE))],
    ));
    seed_parameter(
        &fixture,
        platform,
        (CFG, "CFG"),
        (CFG_1, json!({"speed": 1, "mode": 2})),
        Vec::new(),
    );
    fixture
}

#[tokio::test]
async fn adr_0201_a_new_wstring_member_outside_the_basic_plane_gives_rcp_005_on_codesys() {
    let fixture = cfg_draft(PlcPlatform::CodesysIec);
    let before = DraftSnapshot::capture(&fixture);
    let input = with_value(request(CFG_1, CFG_2), (CFG, ".note"), &json!("😀"));
    let error = run(&fixture, input).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(expected_forms(&error), vec![iec_wstring(10)]);
    assert_eq!(stored(&fixture, CFG).plc_type, tid(CFG_1));
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn adr_0201_a_new_wstring_member_text_outside_the_basic_plane_gives_rcp_005_on_twincat() {
    let fixture = cfg_draft(PlcPlatform::TwinCat3);
    let before = DraftSnapshot::capture(&fixture);
    let input = with_text(request(CFG_1, CFG_2), (CFG, ".note"), "A\u{1F600}");
    let error = run(&fixture, input).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(expected_forms(&error), vec![iec_wstring(10)]);
    assert_eq!(stored(&fixture, CFG).plc_type, tid(CFG_1));
    before.assert_unchanged(&fixture);
}

#[tokio::test]
async fn adr_0201_a_new_wstring_member_of_the_basic_plane_is_rebound() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        let fixture = cfg_draft(platform);
        let input = with_value(request(CFG_1, CFG_2), (CFG, ".note"), &json!("A\u{FFFF}"));
        run(&fixture, input).await.unwrap();
        let after = stored(&fixture, CFG);
        assert_eq!(after.plc_type, tid(CFG_2), "{platform:?}");
        assert_eq!(
            after.value.as_json(),
            &json!({"speed": 1, "mode": 2, "note": "A\u{FFFF}"}),
            "{platform:?}"
        );
    }
}
