//! Table tests of the domain errors of `AddParameter` (`domain-model.md` section 5.2).
//!
//! Each test runs the real use case with one invalid request. It checks the code, the parameter
//! count, the row version of the draft, and the audit log. A rejected command writes nothing.

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{LeafPath, PlcPlatform, S7Address, TypeExpr, UnitPath};
use recipe_domain::recipe::{LimitInput, LimitList, Ordinal, ParameterTag};

use super::{AddParameter, AddParameterInput};
use crate::memory::fixtures::{Fixture, actor, meta, parameter_on, reference, text, uid, vid};
use crate::recipe::test_support::{
    DRAFT_VERSION, DraftSnapshot, bounds, s7_draft, s7_draft_with_speed, stored_parameters,
    twincat_draft,
};

/// The number of parameters that the version budget allows (`reliability.md` section 7.11).
const PARAMETER_BUDGET: u128 = 4_000;

/// The tag of the parameter that the rejected requests add.
const NEW_TAG: &str = "FEED";

/// A request that adds the `INT` parameter `FEED` on the unit `U1.2` with the given value and limits.
fn int_request(value: &str, limits: Vec<LimitInput>) -> AddParameterInput {
    AddParameterInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        unit_path: UnitPath::parse("U1.2").unwrap(),
        tag: ParameterTag::parse(NEW_TAG).unwrap(),
        name_en: text("Feed"),
        description_en: recipe_domain::text::LongText::parse("Feed rate").unwrap(),
        type_expr: TypeExpr::parse("INT").unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(1).unwrap(),
        value_json: value.to_owned(),
        limits: LimitList::parse(limits).unwrap(),
    }
}

/// A request that the domain accepts on a draft without an S7 address.
fn valid_request() -> AddParameterInput {
    int_request("120", vec![bounds("10", "250")])
}

/// The same request with the S7 address `address`.
fn with_address(request: AddParameterInput, address: &str) -> AddParameterInput {
    AddParameterInput {
        s7_address: Some(S7Address::parse(address).unwrap()),
        ..request
    }
}

/// Run the use case with `request` and assert the code and that nothing was written.
async fn assert_rejected(fixture: &Fixture, request: AddParameterInput, code: ErrorCode) {
    let before = DraftSnapshot::capture(fixture);
    let error = AddParameter::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), request)
        .await
        .unwrap_err();
    assert_eq!(error.code(), code);
    before.assert_unchanged(fixture);
}

#[tokio::test]
async fn an_s7_300_parameter_without_address_gives_rcp_003() {
    let fixture = s7_draft();
    assert_rejected(&fixture, valid_request(), ErrorCode::RCP_003).await;
}

#[tokio::test]
async fn a_value_above_the_process_limit_gives_rcp_004() {
    let fixture = twincat_draft();
    let request = int_request("300", vec![bounds("10", "250")]);
    assert_rejected(&fixture, request, ErrorCode::RCP_004).await;
}

#[tokio::test]
async fn a_text_value_for_an_int_gives_rcp_005() {
    let fixture = twincat_draft();
    let request = int_request("\"abc\"", vec![bounds("10", "250")]);
    assert_rejected(&fixture, request, ErrorCode::RCP_005).await;
}

#[tokio::test]
async fn a_limit_outside_the_range_of_an_int_gives_rcp_006() {
    let fixture = twincat_draft();
    let request = int_request("120", vec![bounds("10", "40000")]);
    assert_rejected(&fixture, request, ErrorCode::RCP_006).await;
}

#[tokio::test]
async fn a_tag_that_the_draft_already_holds_gives_rcp_014() {
    let fixture = twincat_draft();
    let request = AddParameterInput {
        tag: ParameterTag::parse("SPEED").unwrap(),
        ..valid_request()
    };
    assert_rejected(&fixture, request, ErrorCode::RCP_014).await;
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
    let request = int_request("120", vec![limit]);
    assert_rejected(&fixture, request, ErrorCode::RCP_017).await;
}

#[tokio::test]
async fn a_minimum_above_the_maximum_gives_rcp_018() {
    let fixture = twincat_draft();
    let request = int_request("120", vec![bounds("250", "10")]);
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
    let request = int_request("120", vec![limit]);
    assert_rejected(&fixture, request, ErrorCode::RCP_019).await;
}

#[tokio::test]
async fn a_version_at_the_parameter_budget_gives_rcp_021() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    for n in 1..=PARAMETER_BUDGET {
        let parameter = parameter_on(PlcPlatform::TwinCat3, n, &format!("P{n}"), uid(102), 0);
        fixture.store.seed_parameter(vid(DRAFT_VERSION), &parameter);
    }
    assert_eq!(
        stored_parameters(&fixture).len(),
        usize::try_from(PARAMETER_BUDGET).unwrap()
    );
    assert_rejected(&fixture, valid_request(), ErrorCode::RCP_021).await;
}

#[tokio::test]
async fn two_ints_at_the_same_address_give_rcp_022() {
    let fixture = s7_draft_with_speed();
    let request = with_address(valid_request(), "DB1.DBW0");
    assert_rejected(&fixture, request, ErrorCode::RCP_022).await;
}

#[tokio::test]
async fn an_int_at_an_odd_byte_gives_rcp_023() {
    let fixture = s7_draft();
    let request = with_address(valid_request(), "DB1.DBW3");
    assert_rejected(&fixture, request, ErrorCode::RCP_023).await;
}

#[tokio::test]
async fn an_address_on_a_twincat3_machine_gives_rcp_053() {
    let fixture = twincat_draft();
    let request = with_address(valid_request(), "DB1.DBW10");
    assert_rejected(&fixture, request, ErrorCode::RCP_053).await;
}

#[tokio::test]
async fn a_double_word_address_for_an_int_gives_rcp_054() {
    let fixture = s7_draft();
    let request = with_address(valid_request(), "DB1.DBD0");
    assert_rejected(&fixture, request, ErrorCode::RCP_054).await;
}
