use serde_json::{Value, json};
use sha2::Digest;

use super::ScopeBody;
use crate::export::test_rows::{length_limit, parameter, range_limit, stored_parameter, units};
use crate::export::test_support::{
    INT, LABEL_BOX, STRING_8, STRING_16, STRING_ARRAY, ZONE_TEMP, trees,
};
use crate::export::{ExportHeader, HashedBody, Sha256};
use crate::plant::{MachineCode, PlcPlatform, S7Address, UnitPath};
use crate::recipe::{Parameter, ParameterKey, ReferenceNumber};

const PLATFORM: PlcPlatform = PlcPlatform::S7_300;

fn header() -> ExportHeader {
    ExportHeader {
        machine: MachineCode::parse("M12").unwrap(),
        platform: PLATFORM,
        reference: ReferenceNumber::parse("R-100").unwrap(),
        version: 3,
    }
}

fn path(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

/// The zone parameter of the example in `api.md` section 6.2, in the unit `U1.2`.
fn zone() -> Parameter {
    let mut zone = stored_parameter(
        2,
        ("U1.2", "ZONE1"),
        ZONE_TEMP,
        json!({ "setpoint": 72.5, "label": "Z1" }),
    );
    zone.key = ParameterKey::parse("9a8b7c6d-5e4f-4321-b0a9-8f7e6d5c4b3a").unwrap();
    zone.s7_address = Some(S7Address::parse("DB1.DBB2").unwrap());
    zone.limits = vec![
        range_limit(".setpoint", Some(20), Some(95)),
        length_limit(".label", 16),
    ];
    Parameter::from(zone)
}

/// Six parameters in five units, given in an order that is not the order of the arrays.
///
/// `U1` holds `SPEED` and `ALPHA`. `U1.1` holds an array of strings. `U1.2` holds a struct. `U2`
/// holds another struct. `U10` holds a `STRING(8)`, and `U10` is not below `U1`.
fn parameters() -> Vec<Parameter> {
    let mut speed = stored_parameter(1, ("U1", "SPEED"), INT, json!(120));
    speed.key = ParameterKey::parse("1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718").unwrap();
    speed.s7_address = Some(S7Address::parse("DB1.DBW0").unwrap());
    speed.limits = vec![range_limit("", Some(10), Some(250))];
    vec![
        parameter(5, ("U10", "TXT"), STRING_8, json!("abc")),
        zone(),
        Parameter::from(speed),
        parameter(4, ("U2", "BOX"), LABEL_BOX, json!({ "text": "x" })),
        parameter(
            3,
            ("U1.1", "NAMES"),
            STRING_ARRAY,
            json!(["a", "b", "c", "d"]),
        ),
        parameter(6, ("U1", "ALPHA"), INT, json!(1)),
    ]
}

fn whole() -> HashedBody {
    HashedBody::build(&header(), &parameters(), &units(PLATFORM), &trees(PLATFORM)).unwrap()
}

fn scope_of(unit: &str) -> ScopeBody {
    ScopeBody::build(&whole(), &path(unit))
}

fn json_of(body: &ScopeBody) -> Value {
    serde_json::to_value(body).unwrap()
}

fn type_ids(body: &ScopeBody) -> Vec<Value> {
    json_of(body)["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["id"].clone())
        .collect()
}

fn tags(body: &ScopeBody) -> Vec<(String, String)> {
    json_of(body)["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            (
                entry["unit_path"].as_str().unwrap().to_owned(),
                entry["tag"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn key_set(value: &Value) -> Vec<&str> {
    value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect()
}

#[test]
fn field_set_is_the_one_of_adr_0109_v() {
    let value = json_of(&scope_of("U1.2"));
    assert_eq!(
        key_set(&value),
        [
            "machine",
            "parameters",
            "recipe",
            "scope",
            "types",
            "version"
        ]
    );
    assert_eq!(
        value["machine"],
        json!({ "code": "M12", "plc_platform": "S7_300" })
    );
    assert_eq!(value["recipe"], json!({ "reference_number": "R-100" }));
    assert_eq!(value["version"], json!({ "number": 3 }));
    assert_eq!(value["scope"], json!({ "unit_path": "U1.2" }));
}

#[test]
fn parameter_entry_has_no_name_and_no_ordinal() {
    let value = json_of(&scope_of("U1.2"));
    let entry = &value["parameters"][0];
    assert_eq!(
        key_set(entry),
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

#[test]
fn identity_accessors_return_the_values_of_the_whole_body() {
    let body = scope_of("U1.2");
    assert_eq!(body.machine_code(), whole().machine_code());
    assert_eq!(body.platform(), PLATFORM);
    assert_eq!(body.reference_number().to_string(), "R-100");
    assert_eq!(body.version_number(), 3);
    assert_eq!(body.unit_path(), &path("U1.2"));
}

#[test]
fn scope_holds_the_unit_and_its_descendants() {
    let expected: Vec<(String, String)> = [
        ("U1", "ALPHA"),
        ("U1", "SPEED"),
        ("U1.1", "NAMES"),
        ("U1.2", "ZONE1"),
    ]
    .map(|(unit, tag)| (unit.to_owned(), tag.to_owned()))
    .to_vec();
    assert_eq!(tags(&scope_of("U1")), expected);
}

#[test]
fn unit_u10_is_not_below_u1() {
    let found = tags(&scope_of("U1"));
    assert!(found.iter().all(|(unit, _)| unit != "U10"));
    assert_eq!(
        tags(&scope_of("U10")),
        vec![("U10".to_owned(), "TXT".to_owned())]
    );
}

#[test]
fn type_closure_of_a_struct_parameter_holds_the_struct_and_its_member_types() {
    // ZoneTemp has the members REAL and STRING(16). REAL is a primitive without a length.
    assert_eq!(
        type_ids(&scope_of("U1.2")),
        [json!(STRING_16), json!(ZONE_TEMP)]
    );
}

#[test]
fn type_closure_of_an_array_parameter_holds_the_array_and_its_element_type() {
    assert_eq!(
        type_ids(&scope_of("U1.1")),
        [json!(STRING_ARRAY), json!(STRING_16)]
    );
}

#[test]
fn types_that_no_filtered_parameter_uses_are_not_in_the_scope() {
    let ids = type_ids(&scope_of("U1"));
    assert!(!ids.contains(&json!(LABEL_BOX)));
    assert!(!ids.contains(&json!(STRING_8)));
    assert_eq!(whole().types().len(), 5, "the whole body holds all five");
    assert_eq!(
        ids,
        [json!(STRING_ARRAY), json!(STRING_16), json!(ZONE_TEMP)]
    );
}

#[test]
fn scope_equals_a_v1_build_of_the_filtered_parameters_only() {
    let all_units = units(PLATFORM);
    let filtered: Vec<Parameter> = parameters()
        .into_iter()
        .filter(|parameter| {
            let unit = all_units.path_of(parameter.unit()).unwrap();
            matches!(unit.to_string().as_str(), "U1" | "U1.1" | "U1.2")
        })
        .collect();
    let v1 = HashedBody::build(&header(), &filtered, &all_units, &trees(PLATFORM)).unwrap();
    let scope = scope_of("U1");
    assert_eq!(
        serde_json::to_value(scope.types()).unwrap(),
        serde_json::to_value(v1.types()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(scope.parameters()).unwrap(),
        serde_json::to_value(v1.parameters()).unwrap()
    );
}

#[test]
fn arrays_follow_the_orders_of_api_section_6_3() {
    let value = json_of(&scope_of("U1"));
    let ids: Vec<&str> = value["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["id"].as_str().unwrap())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "types[] by id, byte by byte");
    let found = tags(&scope_of("U1"));
    let units_in_order: Vec<&str> = found.iter().map(|(unit, _)| unit.as_str()).collect();
    assert_eq!(
        units_in_order,
        ["U1", "U1", "U1.1", "U1.2"],
        "parameters[] by unit_path"
    );
    assert_eq!(found.first().map(|(_, tag)| tag.as_str()), Some("ALPHA"));
    let limits = value["parameters"][3]["limits"].as_array().unwrap();
    assert_eq!(limits[0]["leaf_path"], ".label", "limits[] by leaf_path");
}

#[test]
fn unit_without_a_parameter_gives_an_empty_scope() {
    let body = scope_of("U1.2.9");
    assert!(body.is_empty());
    assert!(body.parameters().is_empty());
    assert!(body.types().is_empty());
    assert!(!scope_of("U1.2").is_empty());
}

/// The canonical form of the scope `U1.2`, written by hand from RFC 8785.
fn expected_canonical_text() -> String {
    [
        r#"{"machine":{"code":"M12","plc_platform":"S7_300"},"#,
        r#""parameters":["#,
        r#"{"key":"9a8b7c6d-5e4f-4321-b0a9-8f7e6d5c4b3a","#,
        r#""limits":[{"leaf_path":".label","max":null,"max_length":"16","min":null},"#,
        r#"{"leaf_path":".setpoint","max":"95","max_length":null,"min":"20"}],"#,
        r#""s7_address":"DB1.DBB2","tag":"ZONE1","#,
        r#""type_id":"7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f","unit_path":"U1.2","#,
        r#""value":{"label":"Z1","setpoint":"72.5"}}],"#,
        r#""recipe":{"reference_number":"R-100"},"#,
        r#""scope":{"unit_path":"U1.2"},"#,
        r#""types":["#,
        r#"{"id":"5d2c7b1e-3f4a-4b6c-8d9e-1a2b3c4d5e6f","kind":"Primitive","name":"STRING","revision":1,"string_length":16},"#,
        r#"{"id":"7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f","kind":"Struct","#,
        r#""members":[{"name":"setpoint","type_id":"REAL"},"#,
        r#"{"name":"label","type_id":"5d2c7b1e-3f4a-4b6c-8d9e-1a2b3c4d5e6f"}],"#,
        r#""name":"ZoneTemp","revision":2,"string_length":null}],"#,
        r#""version":{"number":3}}"#,
    ]
    .concat()
}

#[test]
fn canonical_text_of_the_scope_equals_the_hand_written_text() {
    let body = scope_of("U1.2");
    assert_eq!(body.canonical_text().unwrap(), expected_canonical_text());
}

#[test]
fn hash_of_the_scope_equals_the_sha256_of_the_hand_written_text() {
    let digest: [u8; 32] = sha2::Sha256::digest(expected_canonical_text().as_bytes()).into();
    assert_eq!(scope_of("U1.2").sha256().unwrap(), Sha256::from(digest));
}

#[test]
fn hash_of_the_scope_equals_the_value_that_sha256sum_gave() {
    // Computed outside the code: printf '%s' "<canonical text>" | sha256sum
    let pinned = "49a1d056e18c28cc6f84605db5766b1266e2e83a7b5015cf7dfd19f850de3433";
    assert_eq!(scope_of("U1.2").sha256().unwrap().to_string(), pinned);
}

#[test]
fn hash_of_the_scope_differs_from_the_hash_of_the_whole_body_and_of_other_scopes() {
    assert_ne!(scope_of("U1").sha256().unwrap(), whole().sha256().unwrap());
    assert_ne!(
        scope_of("U1").sha256().unwrap(),
        scope_of("U1.2").sha256().unwrap()
    );
}
