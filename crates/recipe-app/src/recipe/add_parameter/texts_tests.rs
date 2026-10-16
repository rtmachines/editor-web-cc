//! Tests of `AddParameter` with the texts of a form (ADR-0039 (c), ADR-0041 (b)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{LeafPath, PlcPlatform, Primitive, TypeExpr, UnitPath};
use recipe_domain::recipe::{LimitInput, LimitList, Ordinal, ParameterTag, ValueInput};
use serde_json::json;

use super::{AddParameter, AddParameterInput};
use crate::memory::fixtures::{Fixture, actor, meta, reference, text};
use crate::recipe::rebind::fixtures::{error_lines, leaf_texts, primitive_id, typed_struct_row};
use crate::recipe::test_support::{DraftSnapshot, seed_speed_draft, stored_parameters};

/// The identifier of `ST_Hold@1` with the members `setpoint` (`REAL`) and `hold` (`TIME`).
const HOLD_1: u128 = 940;

/// A draft with the parameter `SPEED` and the struct row `ST_Hold@1`.
fn hold_draft() -> Fixture {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let members = [
        ("setpoint", primitive_id(Primitive::Real)),
        ("hold", primitive_id(Primitive::Time)),
    ];
    let row = typed_struct_row(
        PlcPlatform::TwinCat3,
        (HOLD_1, "ST_Hold", 1),
        None,
        &members,
    );
    fixture.store.seed_type(&row);
    fixture
}

/// A limit row on `leaf` with the given bounds.
fn row(leaf: &str, min: &str, max: &str) -> LimitInput {
    LimitInput {
        leaf_path: LeafPath::parse(leaf).unwrap(),
        min: Some(min.to_owned()),
        max: Some(max.to_owned()),
        max_length: None,
    }
}

/// A request that adds the `ST_Hold@1` parameter `HOLD` with the given value and limits.
fn hold_request(value: ValueInput, limits: Vec<LimitInput>) -> AddParameterInput {
    AddParameterInput {
        reference: reference("R-1"),
        number: 1,
        expected_row_version: 1,
        unit_path: UnitPath::parse("U1.2").unwrap(),
        tag: ParameterTag::parse("HOLD").unwrap(),
        name_en: text("Hold"),
        description_en: recipe_domain::text::LongText::parse("Hold time").unwrap(),
        type_expr: TypeExpr::parse("ST_Hold@1").unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(1).unwrap(),
        value,
        limits: LimitList::parse(limits).unwrap(),
    }
}

#[tokio::test]
async fn texts_store_the_canonical_decimal_and_the_count_of_the_time_literal() {
    let fixture = hold_draft();
    let value = leaf_texts(&[(".setpoint", "182.50"), (".hold", "T#1m30s")]);
    AddParameter::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            hold_request(value, Vec::new()),
        )
        .await
        .unwrap();
    let added = stored_parameters(&fixture)
        .into_iter()
        .find(|parameter| parameter.tag.as_str() == "HOLD")
        .unwrap();
    let stored = added.value.as_json();
    assert_eq!(stored["setpoint"].to_string(), "182.5");
    assert_eq!(stored["hold"], json!(90_000));
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn two_bad_leaves_and_two_bad_rows_give_one_list_in_order_and_write_nothing() {
    let fixture = hold_draft();
    let before = DraftSnapshot::capture(&fixture);
    let value = leaf_texts(&[(".setpoint", "1,5"), (".hold", "90000")]);
    let limits = vec![row(".missing", "0", "1"), row(".setpoint", "9", "1")];
    let error = AddParameter::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            hold_request(value, limits),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_005);
    assert_eq!(
        error_lines(&error),
        [
            "E-RCP-005 '.setpoint'",
            "E-RCP-005 '.hold'",
            "E-RCP-017 '.missing'",
            "E-RCP-018 '.setpoint'",
        ]
    );
    before.assert_unchanged(&fixture);
}
