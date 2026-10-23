use serde_json::{Value, json};

use crate::export::test_rows::{length_limit, range_limit, stored_parameter, units};
use crate::export::test_support::{INT, ZONE_TEMP, trees, type_id};
use crate::export::{ExportHeader, HashedBody, TypeReference};
use crate::plant::{MachineCode, PlcPlatform, Primitive, S7Address, UnitPath};
use crate::recipe::{Parameter, ParameterKey, ParameterTag, ReferenceNumber};

const PLATFORM: PlcPlatform = PlcPlatform::S7_300;
const SPEED_KEY: &str = "1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718";

/// The body of the example in `api.md` section 6.2.
fn example_body() -> HashedBody {
    let header = ExportHeader {
        machine: MachineCode::parse("M12").unwrap(),
        platform: PLATFORM,
        reference: ReferenceNumber::parse("R-100").unwrap(),
        version: 3,
    };
    let mut speed = stored_parameter(1, ("U1", "SPEED"), INT, json!(120));
    speed.key = ParameterKey::parse(SPEED_KEY).unwrap();
    speed.s7_address = Some(S7Address::parse("DB1.DBW0").unwrap());
    speed.limits = vec![range_limit("", Some(10), Some(250))];
    let mut zone = stored_parameter(
        2,
        ("U1.2", "ZONE1"),
        ZONE_TEMP,
        json!({ "setpoint": 72.5, "label": "Z1" }),
    );
    zone.limits = vec![length_limit(".label", 16)];
    let parameters = [Parameter::from(speed), Parameter::from(zone)];
    HashedBody::build(&header, &parameters, &units(PLATFORM), &trees(PLATFORM)).unwrap()
}

#[test]
fn the_header_accessors_give_the_values_of_the_header() {
    let body = example_body();

    assert_eq!(body.machine_code(), MachineCode::parse("M12").unwrap());
    assert_eq!(body.platform(), PlcPlatform::S7_300);
    assert_eq!(
        body.reference_number(),
        &ReferenceNumber::parse("R-100").unwrap()
    );
    assert_eq!(body.version_number(), 3);
}

#[test]
fn the_type_and_parameter_accessors_give_the_hashed_arrays() {
    let body = example_body();
    let hashed = serde_json::to_value(&body).unwrap();

    assert_eq!(serde_json::to_value(body.types()).unwrap(), hashed["types"]);
    assert_eq!(
        serde_json::to_value(body.parameters()).unwrap(),
        hashed["parameters"]
    );
    let tags: Vec<&ParameterTag> = body.parameters().iter().map(|entry| entry.tag()).collect();
    assert_eq!(
        tags,
        [
            &ParameterTag::parse("SPEED").unwrap(),
            &ParameterTag::parse("ZONE1").unwrap()
        ]
    );
}

#[test]
fn the_parameter_entry_accessors_give_each_field() {
    let body = example_body();
    let speed = &body.parameters()[0];

    assert_eq!(speed.unit_path(), &UnitPath::parse("U1").unwrap());
    assert_eq!(speed.tag(), &ParameterTag::parse("SPEED").unwrap());
    assert_eq!(speed.key(), ParameterKey::parse(SPEED_KEY).unwrap());
    assert_eq!(speed.type_id(), TypeReference::Primitive(Primitive::Int));
    assert_eq!(
        speed.s7_address(),
        Some(S7Address::parse("DB1.DBW0").unwrap())
    );
    assert_eq!(
        serde_json::to_value(speed.limits()).unwrap(),
        json!([{ "leaf_path": "", "min": "10", "max": "250", "max_length": null }])
    );
    assert_eq!(serde_json::to_value(speed.value()).unwrap(), json!("120"));
}

#[test]
fn the_value_accessor_keeps_the_member_order_of_the_struct() {
    let body = example_body();
    let zone = &body.parameters()[1];

    assert_eq!(zone.type_id(), TypeReference::Row(type_id(ZONE_TEMP)));
    assert_eq!(zone.s7_address(), None);
    assert_eq!(
        serde_json::to_string(zone.value()).unwrap(),
        r#"{"setpoint":"72.5","label":"Z1"}"#
    );
    let limits = serde_json::to_value(zone.limits()).unwrap();
    assert_eq!(limits[0]["max_length"], Value::from("16"));
}

#[test]
fn a_type_reference_writes_the_primitive_name_or_the_row_identifier() {
    let primitive = TypeReference::Primitive(Primitive::Int);
    let row = TypeReference::Row(type_id(ZONE_TEMP));

    assert_eq!(primitive.to_string(), "INT");
    assert_eq!(row.to_string(), ZONE_TEMP);
    assert_eq!(serde_json::to_value(primitive).unwrap(), json!("INT"));
    assert_eq!(serde_json::to_value(row).unwrap(), json!(ZONE_TEMP));
}
