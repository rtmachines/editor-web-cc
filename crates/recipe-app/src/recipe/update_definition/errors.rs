//! Table tests of the domain errors of `UpdateParameterDefinition` (`domain-model.md` section 5.2).
//!
//! Each test runs the real use case with one invalid request. It checks the code, the parameters,
//! the row version of the draft, and the audit log. A rejected command writes nothing.

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{LeafPath, PlcPlatform, S7Address, UnitPath};
use recipe_domain::recipe::{LimitInput, ParameterTag};
use serde_json::{Value, json};

use super::UpdateParameterDefinitionInput;
use super::requests::{run, speed_request, with_retype, with_type};
use crate::memory::fixtures::{Fixture, parameter_on, pkey, uid, vid};
use crate::recipe::rebind::fixtures::near_budget_draft;
use crate::recipe::test_support::{
    DRAFT_VERSION, DraftSnapshot, archive_unit, bounds, s7_draft_with_speed, seed_deleted,
    seed_speed_draft, twincat_draft,
};

/// The same request with the S7 address `address`.
fn with_address(
    request: UpdateParameterDefinitionInput,
    address: &str,
) -> UpdateParameterDefinitionInput {
    UpdateParameterDefinitionInput {
        s7_address: Some(S7Address::parse(address).unwrap()),
        ..request
    }
}

/// A request that replaces the value and the limits of `SPEED` with `120` and `limits`.
fn with_limits(limits: Vec<LimitInput>) -> UpdateParameterDefinitionInput {
    with_retype(speed_request(), "120", limits)
}

/// Run the use case with `request` and assert the code and that nothing was written.
async fn assert_rejected(
    fixture: &Fixture,
    request: UpdateParameterDefinitionInput,
    code: ErrorCode,
) {
    let before = DraftSnapshot::capture(fixture);
    let error = run(fixture, request).await.unwrap_err();
    assert_eq!(error.code(), code);
    before.assert_unchanged(fixture);
}

#[tokio::test]
async fn an_s7_300_parameter_without_address_gives_rcp_003() {
    let fixture = s7_draft_with_speed();
    assert_rejected(&fixture, speed_request(), ErrorCode::RCP_003).await;
}

#[tokio::test]
async fn a_text_value_for_an_int_gives_rcp_005() {
    let fixture = twincat_draft();
    let request = with_retype(speed_request(), "\"abc\"", vec![bounds("10", "250")]);
    assert_rejected(&fixture, request, ErrorCode::RCP_005).await;
}

#[tokio::test]
async fn a_limit_outside_the_range_of_an_int_gives_rcp_006() {
    let fixture = twincat_draft();
    let request = with_limits(vec![bounds("10", "40000")]);
    assert_rejected(&fixture, request, ErrorCode::RCP_006).await;
}

#[tokio::test]
async fn a_limit_on_a_missing_leaf_gives_rcp_017() {
    let fixture = twincat_draft();
    let limit = LimitInput {
        leaf_path: LeafPath::parse(".missing").unwrap(),
        min: Some("10".to_owned()),
        max: Some("250".to_owned()),
        max_length: None,
    };
    assert_rejected(&fixture, with_limits(vec![limit]), ErrorCode::RCP_017).await;
}

#[tokio::test]
async fn a_minimum_above_the_maximum_gives_rcp_018() {
    let fixture = twincat_draft();
    let request = with_limits(vec![bounds("250", "10")]);
    assert_rejected(&fixture, request, ErrorCode::RCP_018).await;
}

#[tokio::test]
async fn a_length_limit_on_an_int_gives_rcp_019() {
    let fixture = twincat_draft();
    let limit = LimitInput {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: None,
        max_length: Some(5),
    };
    assert_rejected(&fixture, with_limits(vec![limit]), ErrorCode::RCP_019).await;
}

#[tokio::test]
async fn an_int_moved_to_the_address_of_another_gives_rcp_022() {
    let fixture = s7_draft_with_speed();
    let other = parameter_on(PlcPlatform::S7_300, 3, "OTHER", uid(101), 2);
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &other);
    let request = with_address(speed_request(), "DB1.DBW2");
    assert_rejected(&fixture, request, ErrorCode::RCP_022).await;
}

#[tokio::test]
async fn an_int_at_an_odd_byte_gives_rcp_023() {
    let fixture = s7_draft_with_speed();
    let request = with_address(speed_request(), "DB1.DBW3");
    assert_rejected(&fixture, request, ErrorCode::RCP_023).await;
}

#[tokio::test]
async fn an_address_on_a_twincat3_machine_gives_rcp_053() {
    let fixture = twincat_draft();
    let request = with_address(speed_request(), "DB1.DBW10");
    assert_rejected(&fixture, request, ErrorCode::RCP_053).await;
}

#[tokio::test]
async fn a_double_word_address_for_an_int_gives_rcp_054() {
    let fixture = s7_draft_with_speed();
    let request = with_address(speed_request(), "DB1.DBD0");
    assert_rejected(&fixture, request, ErrorCode::RCP_054).await;
}

#[tokio::test]
async fn an_archived_unit_gives_plt_009() {
    let fixture = Fixture::new();
    let mut machine = seed_speed_draft(&fixture);
    archive_unit(&mut machine, "U1.2.1");
    fixture.store.seed_machine(&machine);
    let mut request = speed_request();
    request.unit_path = UnitPath::parse("U1.2.1").unwrap();
    assert_rejected(&fixture, request, ErrorCode::PLT_009).await;
}

#[tokio::test]
async fn an_unknown_unit_path_gives_plt_020() {
    let fixture = twincat_draft();
    let mut request = speed_request();
    request.unit_path = UnitPath::parse("U9").unwrap();
    assert_rejected(&fixture, request, ErrorCode::PLT_020).await;
}

#[tokio::test]
async fn an_unknown_struct_revision_gives_plt_013() {
    let fixture = twincat_draft();
    let request = with_retype(with_type(speed_request(), "ST_Missing@1"), "{}", Vec::new());
    assert_rejected(&fixture, request, ErrorCode::PLT_013).await;
}

#[tokio::test]
async fn an_array_above_the_leaf_budget_gives_plt_018() {
    let fixture = twincat_draft();
    let request = with_retype(
        with_type(speed_request(), "ARRAY[1..16385] OF INT"),
        "[]",
        Vec::new(),
    );
    assert_rejected(&fixture, request, ErrorCode::PLT_018).await;
}

#[tokio::test]
async fn a_version_that_is_not_a_live_draft_gives_rcp_002_and_writes_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let error = run(&fixture, speed_request()).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_002);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.store.draft_writes().is_empty());
}

#[tokio::test]
async fn a_deleted_recipe_gives_rcp_012_and_writes_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let error = run(&fixture, speed_request()).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.store.draft_writes().is_empty());
}

#[tokio::test]
async fn a_type_change_above_the_leaf_budget_gives_rcp_021_and_writes_nothing() {
    // The draft holds 253 953 leaves. The new array of 16 384 elements replaces one leaf.
    let fixture = near_budget_draft();
    let request = UpdateParameterDefinitionInput {
        key: pkey(41),
        tag: ParameterTag::parse("SMALL").unwrap(),
        ..with_retype(
            with_type(speed_request(), "ARRAY[1..16384] OF INT"),
            &Value::Array(vec![json!(0); 16_384]).to_string(),
            Vec::new(),
        )
    };
    assert_rejected(&fixture, request, ErrorCode::RCP_021).await;
    assert!(fixture.store.draft_writes().is_empty());
}
