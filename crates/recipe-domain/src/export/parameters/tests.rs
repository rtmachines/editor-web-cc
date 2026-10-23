use serde_json::{Value, json};

use super::ParameterEntry;
use crate::error_codes::ErrorCode;
use crate::export::test_rows::{
    length_limit, parameter, range_limit, stored_parameter, unit_id, units, word_address,
};
use crate::export::test_support::{
    BOOL, INT, REAL, STRING_16, STRING_ARRAY, TIME, ZONE_TEMP, trees, type_id,
};
use crate::export::{ExportHeader, HashedBody};
use crate::identity::UserId;
use crate::plant::{
    ArrayDimension, ArrayDimensions, MachineCode, MemberName, PlcPlatform, Primitive, TypeName,
    TypeNode, UnitPath,
};
use crate::recipe::{
    Draft, Parameter, RecipeError, ReferenceNumber, SnapshotItem, StoredDraft, StoredParameter,
    VersionSnapshot, VersionState,
};
use uuid::Uuid;

fn header(platform: PlcPlatform) -> ExportHeader {
    ExportHeader {
        machine: MachineCode::parse("M12").unwrap(),
        platform,
        reference: ReferenceNumber::parse("R-100").unwrap(),
        version: 3,
    }
}

fn build(platform: PlcPlatform, parameters: &[Parameter]) -> Result<HashedBody, RecipeError> {
    HashedBody::build(
        &header(platform),
        parameters,
        &units(platform),
        &trees(platform),
    )
}

fn body_json(platform: PlcPlatform, parameters: &[Parameter]) -> Value {
    serde_json::to_value(build(platform, parameters).unwrap()).unwrap()
}

/// Return the text of one field of every parameter entry.
fn column(body: &Value, field: &str) -> Vec<String> {
    body["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry[field].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn header_fields_are_in_the_body() {
    let body = body_json(PlcPlatform::S7_300, &[]);
    assert_eq!(
        body["machine"],
        json!({ "code": "M12", "plc_platform": "S7_300" })
    );
    assert_eq!(body["recipe"], json!({ "reference_number": "R-100" }));
    assert_eq!(body["version"], json!({ "number": 3 }));
    assert_eq!(body["types"], json!([]));
    assert_eq!(body["parameters"], json!([]));
}

#[test]
fn unit_paths_order_as_integers_and_a_parent_comes_first() {
    let parameters = [
        parameter(1, ("U10", "A"), INT, json!(1)),
        parameter(2, ("U2", "A"), INT, json!(1)),
        parameter(3, ("U1.1", "A"), INT, json!(1)),
        parameter(4, ("U1", "A"), INT, json!(1)),
    ];
    let body = body_json(PlcPlatform::S7_300, &parameters);
    assert_eq!(column(&body, "unit_path"), ["U1", "U1.1", "U2", "U10"]);
}

#[test]
fn tags_of_one_unit_order_byte_by_byte() {
    let parameters = [
        parameter(1, ("U1", "BETA"), INT, json!(1)),
        parameter(2, ("U1", "ALPHA"), INT, json!(1)),
    ];
    let body = body_json(PlcPlatform::S7_300, &parameters);
    assert_eq!(column(&body, "tag"), ["ALPHA", "BETA"]);
}

#[test]
fn limits_order_by_leaf_path_text_with_canonical_bounds() {
    let mut row = stored_parameter(
        1,
        ("U1.2", "ZONE1"),
        ZONE_TEMP,
        json!({ "setpoint": 72.5, "label": "Z1" }),
    );
    row.limits = vec![
        range_limit(".setpoint", Some(20), Some(95)),
        length_limit(".label", 16),
    ];
    let body = body_json(PlcPlatform::S7_300, &[Parameter::from(row)]);
    assert_eq!(
        body["parameters"][0]["limits"],
        json!([
            { "leaf_path": ".label", "min": null, "max": null, "max_length": "16" },
            { "leaf_path": ".setpoint", "min": "20", "max": "95", "max_length": null }
        ])
    );
}

#[test]
fn a_bound_with_trailing_zeros_is_canonical_text() {
    let mut row = stored_parameter(1, ("U1", "SPEED"), INT, json!(120));
    let mut limit = range_limit("", Some(10), Some(250));
    limit.max = Some(rust_decimal::Decimal::new(25_000, 2));
    row.limits = vec![limit];
    let body = body_json(PlcPlatform::S7_300, &[Parameter::from(row)]);
    assert_eq!(body["parameters"][0]["limits"][0]["max"], json!("250"));
}

#[test]
fn numeric_and_time_leaves_are_strings_and_a_bool_stays_a_bool() {
    let parameters = [
        parameter(1, ("U1", "SPEED"), INT, json!(120)),
        parameter(2, ("U1", "DELAY"), TIME, json!(1500)),
        parameter(3, ("U1", "ACTIVE"), BOOL, json!(true)),
        parameter(4, ("U1", "TEMP"), REAL, json!(72.5)),
        parameter(5, ("U1", "NAME"), STRING_16, json!("Z1")),
        parameter(
            6,
            ("U1", "ZONE"),
            ZONE_TEMP,
            json!({ "setpoint": 72.5, "label": "Z1" }),
        ),
        parameter(
            7,
            ("U1", "LABELS"),
            STRING_ARRAY,
            json!(["a", "b", "c", "d"]),
        ),
    ];
    let body = body_json(PlcPlatform::S7_300, &parameters);
    let value_of = |tag: &str| -> Value {
        body["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["tag"] == tag)
            .unwrap()["value"]
            .clone()
    };
    assert_eq!(value_of("SPEED"), json!("120"));
    assert_eq!(value_of("DELAY"), json!("1500"));
    assert_eq!(value_of("ACTIVE"), json!(true));
    assert_eq!(value_of("TEMP"), json!("72.5"));
    assert_eq!(value_of("NAME"), json!("Z1"));
    assert_eq!(
        value_of("ZONE"),
        json!({ "setpoint": "72.5", "label": "Z1" })
    );
    assert_eq!(value_of("LABELS"), json!(["a", "b", "c", "d"]));
}

#[test]
fn type_ids_are_references() {
    let parameters = [
        parameter(1, ("U1", "SPEED"), INT, json!(1)),
        parameter(
            2,
            ("U1", "ZONE"),
            ZONE_TEMP,
            json!({ "setpoint": 1, "label": "" }),
        ),
    ];
    let body = body_json(PlcPlatform::S7_300, &parameters);
    assert_eq!(column(&body, "type_id"), ["INT", ZONE_TEMP]);
    assert_eq!(body["types"].as_array().unwrap().len(), 2);
}

#[test]
fn s7_address_is_text_on_s7_300_and_null_on_twincat3() {
    let mut row = stored_parameter(1, ("U1", "SPEED"), INT, json!(1));
    row.s7_address = Some(word_address());
    let on_s7 = body_json(PlcPlatform::S7_300, &[Parameter::from(row)]);
    assert_eq!(on_s7["parameters"][0]["s7_address"], json!("DB1.DBW0"));
    let plain = parameter(2, ("U1", "SPEED"), INT, json!(1));
    let on_twincat = body_json(PlcPlatform::TwinCat3, &[plain]);
    assert_eq!(on_twincat["parameters"][0]["s7_address"], Value::Null);
    assert_eq!(on_twincat["machine"]["plc_platform"], json!("TwinCat3"));
}

#[test]
fn the_body_has_no_name_and_no_ordinal() {
    let mut row = stored_parameter(1, ("U1", "SPEED"), INT, json!(1));
    row.limits = vec![range_limit("", Some(1), Some(2))];
    let body = body_json(PlcPlatform::S7_300, &[Parameter::from(row)]);
    let mut stack = vec![&body];
    while let Some(value) = stack.pop() {
        match value {
            Value::Object(map) => {
                for key in map.keys() {
                    assert!(!matches!(key.as_str(), "name_en" | "ordinal"), "{key}");
                }
                stack.extend(map.values());
            }
            Value::Array(items) => stack.extend(items),
            _ => {}
        }
    }
    let keys: Vec<&String> = body["parameters"][0].as_object().unwrap().keys().collect();
    assert_eq!(
        keys,
        [
            "key",
            "limits",
            "s7_address",
            "tag",
            "type_id",
            "unit_path",
            "value"
        ]
    );
}

fn rows() -> Vec<StoredParameter> {
    vec![
        stored_parameter(1, ("U1", "SPEED"), INT, json!(120)),
        stored_parameter(
            2,
            ("U1.2", "ZONE1"),
            ZONE_TEMP,
            json!({ "setpoint": 72.5, "label": "Z1" }),
        ),
    ]
}

#[test]
fn a_draft_and_a_version_snapshot_give_equal_bodies() {
    let platform = PlcPlatform::S7_300;
    let draft = Draft::load(StoredDraft {
        reference: ReferenceNumber::parse("R-100").unwrap(),
        machine: crate::export::test_support::machine_id(),
        platform,
        version: crate::recipe::RecipeVersionId::from(Uuid::from_u128(30)),
        number: 3,
        state: VersionState::LiveDraft,
        row_version: 7,
        created_by: UserId::from(Uuid::from_u128(1)),
        editors: Vec::new(),
        parameters: rows(),
    })
    .unwrap();
    let snapshot = VersionSnapshot::from(rows());
    let (units, trees) = (units(platform), trees(platform));
    let from_draft = HashedBody::build(&header(platform), draft.parameters(), &units, &trees);
    let from_snapshot = HashedBody::build(&header(platform), snapshot.parameters(), &units, &trees);
    assert_eq!(from_draft.unwrap(), from_snapshot.unwrap());
}

fn item_codes(error: RecipeError) -> Vec<ErrorCode> {
    assert_eq!(error.code(), ErrorCode::RCP_020);
    let RecipeError::ValidationFailed { items } = error else {
        return Vec::new();
    };
    items.iter().map(SnapshotItem::code).collect()
}

#[test]
fn the_serialized_value_of_zone1_holds_setpoint_before_label() {
    let body = build(PlcPlatform::S7_300, &[Parameter::from(rows().remove(1))]).unwrap();
    let text = serde_json::to_string(&body).unwrap();
    assert!(
        text.contains(r#""value":{"setpoint":"72.5","label":"Z1"}"#),
        "{text}"
    );
}

/// The identifier of the struct `Pair` that the tests build by hand.
const PAIR: &str = "50000000-0000-4000-8000-000000000001";
/// The identifier of an array of `Pair` that the tests build by hand.
const PAIR_ARRAY: &str = "50000000-0000-4000-8000-000000000002";
/// The identifier of the struct `Outer` that the tests build by hand.
const OUTER: &str = "50000000-0000-4000-8000-000000000003";

fn leaf(id: &str, primitive: Primitive) -> TypeNode {
    TypeNode::Leaf {
        type_id: type_id(id),
        primitive,
        leaf_class: primitive.leaf_class(),
        size_bytes: primitive.fixed_size_bytes().unwrap(),
        string_length: None,
    }
}

/// The struct `Pair` with the members `zeta` and `alpha`, in this order.
fn pair() -> TypeNode {
    TypeNode::Struct {
        type_id: type_id(PAIR),
        name: TypeName::parse("Pair").unwrap(),
        revision: 1,
        members: vec![
            (
                MemberName::parse("zeta").unwrap(),
                leaf(INT, Primitive::Int),
            ),
            (
                MemberName::parse("alpha").unwrap(),
                leaf(BOOL, Primitive::Bool),
            ),
        ],
    }
}

fn array_of(element: TypeNode, bounds: &[(i32, i32)]) -> TypeNode {
    let dimensions = bounds
        .iter()
        .map(|&(lower, upper)| ArrayDimension::parse(lower, upper).unwrap())
        .collect();
    TypeNode::Array {
        type_id: type_id(PAIR_ARRAY),
        dimensions: ArrayDimensions::parse(dimensions).unwrap(),
        element: Box::new(element),
    }
}

/// Return the JSON text of the entry of a parameter of type `root` with `value`.
fn entry_text(root: &TypeNode, value: Value) -> String {
    let pairs = parameter(1, ("U1", "PAIRS"), PAIR, value);
    let unit = UnitPath::parse("U1").unwrap();
    serde_json::to_string(&ParameterEntry::of(&pairs, &unit, root)).unwrap()
}

#[test]
fn a_struct_inside_an_array_keeps_its_member_order() {
    let root = array_of(pair(), &[(1, 2)]);
    let value = json!([{ "zeta": 1, "alpha": true }, { "zeta": 2, "alpha": false }]);
    let text = entry_text(&root, value);
    let expected = r#""value":[{"zeta":"1","alpha":true},{"zeta":"2","alpha":false}]}"#;
    assert!(text.ends_with(expected), "{text}");
}

#[test]
fn a_two_dimensional_array_of_a_struct_keeps_the_member_and_element_order() {
    let root = array_of(pair(), &[(1, 2), (0, 1)]);
    let value = json!([
        [{ "zeta": 1, "alpha": true }, { "zeta": 2, "alpha": false }],
        [{ "zeta": 3, "alpha": true }, { "zeta": 4, "alpha": false }]
    ]);
    let text = entry_text(&root, value);
    let expected = concat!(
        r#""value":[[{"zeta":"1","alpha":true},{"zeta":"2","alpha":false}],"#,
        r#"[{"zeta":"3","alpha":true},{"zeta":"4","alpha":false}]]}"#
    );
    assert!(text.ends_with(expected), "{text}");
}

#[test]
fn a_struct_member_and_an_array_member_of_a_struct_keep_their_order() {
    let outer = TypeNode::Struct {
        type_id: type_id(OUTER),
        name: TypeName::parse("Outer").unwrap(),
        revision: 1,
        members: vec![
            (MemberName::parse("zeta").unwrap(), pair()),
            (
                MemberName::parse("alpha").unwrap(),
                array_of(pair(), &[(1, 2)]),
            ),
        ],
    };
    let value = json!({
        "zeta": { "zeta": 1, "alpha": true },
        "alpha": [{ "zeta": 2, "alpha": false }, { "zeta": 3, "alpha": true }]
    });
    let text = entry_text(&outer, value);
    let expected = concat!(
        r#""value":{"zeta":{"zeta":"1","alpha":true},"#,
        r#""alpha":[{"zeta":"2","alpha":false},{"zeta":"3","alpha":true}]}}"#
    );
    assert!(text.ends_with(expected), "{text}");
}

#[test]
fn a_key_outside_the_members_comes_after_the_members_in_byte_order() {
    let value = json!({ "omega": 9, "zeta": 1, "beta": "b", "alpha": true });
    let text = entry_text(&pair(), value);
    let expected = r#""value":{"zeta":"1","alpha":true,"beta":"b","omega":"9"}}"#;
    assert!(text.ends_with(expected), "{text}");
}

#[test]
fn an_unknown_unit_gives_e_rcp_020_with_an_e_plt_020_item() {
    let parameters = [parameter(1, ("U3", "SPEED"), INT, json!(1))];
    assert_ne!(unit_id("U3"), unit_id("U2"));
    let error = build(PlcPlatform::S7_300, &parameters).unwrap_err();
    assert_eq!(item_codes(error), [ErrorCode::PLT_020]);
}

#[test]
fn a_type_that_the_trees_do_not_hold_gives_e_rcp_020_with_an_e_plt_013_item() {
    let missing = "99999999-0000-4000-8000-000000000000";
    assert!(trees(PlcPlatform::S7_300).get(type_id(missing)).is_none());
    let parameters = [parameter(1, ("U1", "SPEED"), missing, json!(1))];
    let error = build(PlcPlatform::S7_300, &parameters).unwrap_err();
    assert_eq!(item_codes(error), [ErrorCode::PLT_013]);
}
