use serde_json::{Value, json};
use sha2::Digest;

use crate::export::test_rows::{length_limit, range_limit, stored_parameter, units};
use crate::export::test_support::{INT, ZONE_TEMP, machine_id, trees};
use crate::export::{ExportHeader, HashedBody, Sha256};
use crate::plant::{
    Machine, MachineCode, PlcPlatform, S7Address, StoredMachine, StoredUnit, UnitPath,
};
use crate::recipe::{Ordinal, Parameter, ParameterKey, ReferenceNumber};
use crate::text::Text;

const PLATFORM: PlcPlatform = PlcPlatform::S7_300;

fn header() -> ExportHeader {
    ExportHeader {
        machine: MachineCode::parse("M12").unwrap(),
        platform: PLATFORM,
        reference: ReferenceNumber::parse("R-100").unwrap(),
        version: 3,
    }
}

/// The parameters of the example in `api.md` section 6.2.
fn example_parameters() -> Vec<Parameter> {
    let mut speed = stored_parameter(1, ("U1", "SPEED"), INT, json!(120));
    speed.key = ParameterKey::parse("1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718").unwrap();
    speed.s7_address = Some(S7Address::parse("DB1.DBW0").unwrap());
    speed.limits = vec![range_limit("", Some(10), Some(250))];
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
    vec![Parameter::from(speed), Parameter::from(zone)]
}

fn body_of(parameters: &[Parameter]) -> HashedBody {
    HashedBody::build(&header(), parameters, &units(PLATFORM), &trees(PLATFORM)).unwrap()
}

/// The canonical form of the example, written by hand from RFC 8785.
///
/// The keys of every object are sorted and the text has no white space.
fn expected_canonical_text() -> String {
    [
        r#"{"machine":{"code":"M12","plc_platform":"S7_300"},"#,
        r#""parameters":["#,
        r#"{"key":"1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718","#,
        r#""limits":[{"leaf_path":"","max":"250","max_length":null,"min":"10"}],"#,
        r#""s7_address":"DB1.DBW0","tag":"SPEED","type_id":"INT","unit_path":"U1","value":"120"},"#,
        r#"{"key":"9a8b7c6d-5e4f-4321-b0a9-8f7e6d5c4b3a","#,
        r#""limits":[{"leaf_path":".label","max":null,"max_length":"16","min":null},"#,
        r#"{"leaf_path":".setpoint","max":"95","max_length":null,"min":"20"}],"#,
        r#""s7_address":"DB1.DBB2","tag":"ZONE1","#,
        r#""type_id":"7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f","unit_path":"U1.2","#,
        r#""value":{"label":"Z1","setpoint":"72.5"}}],"#,
        r#""recipe":{"reference_number":"R-100"},"#,
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
fn canonical_text_of_the_example_equals_the_hand_written_text() {
    let body = body_of(&example_parameters());
    assert_eq!(body.canonical_text().unwrap(), expected_canonical_text());
}

#[test]
fn hash_of_the_example_equals_the_sha256_of_the_hand_written_text() {
    let body = body_of(&example_parameters());
    let digest: [u8; 32] = sha2::Sha256::digest(expected_canonical_text().as_bytes()).into();
    assert_eq!(body.sha256().unwrap(), Sha256::from(digest));
}

#[test]
fn hash_of_the_example_equals_the_value_that_sha256sum_gave() {
    // Computed outside the code: printf '%s' "<canonical text>" | sha256sum
    let pinned = "885d9a7d767af484f1c1824b5d493b9ddc19604db05eff1fa4fbded25bad7497";
    let body = body_of(&example_parameters());
    assert_eq!(body.sha256().unwrap().to_string(), pinned);
}

#[test]
fn hash_displays_as_64_lower_case_hex_characters() {
    let text = body_of(&example_parameters()).sha256().unwrap().to_string();
    assert_eq!(text.len(), 64);
    assert!(text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
}

#[test]
fn names_and_ordinals_are_not_inputs_of_the_hash() {
    let plain = body_of(&example_parameters());
    let renamed: Vec<Parameter> = example_parameters()
        .into_iter()
        .map(|parameter| {
            let mut row = crate::recipe::StoredParameter {
                id: parameter.id(),
                key: parameter.key(),
                unit: parameter.unit(),
                tag: parameter.tag().clone(),
                name_en: Text::parse("Another name").unwrap(),
                description_en: parameter.description_en().clone(),
                translations: Vec::new(),
                plc_type: parameter.plc_type(),
                s7_address: parameter.s7_address(),
                limits: parameter.limits().to_vec(),
                value: parameter.value().clone(),
                ordinal: Ordinal::parse(99).unwrap(),
            };
            row.name_en = Text::parse("Another name").unwrap();
            Parameter::from(row)
        })
        .collect();
    let other_machine = Machine::try_from(StoredMachine {
        id: machine_id(),
        code: MachineCode::parse("M12").unwrap(),
        name: Text::parse("Renamed filler").unwrap(),
        platform: PLATFORM,
        archived_at: None,
        units: ["U1", "U1.2"]
            .map(|path| StoredUnit {
                id: crate::export::test_rows::unit_id(path),
                path: UnitPath::parse(path).unwrap(),
                name: Text::parse("Renamed unit").unwrap(),
                archived_at: None,
            })
            .to_vec(),
    })
    .unwrap()
    .live_units();
    let rebuilt = HashedBody::build(&header(), &renamed, &other_machine, &trees(PLATFORM)).unwrap();
    assert_eq!(plain.sha256().unwrap(), rebuilt.sha256().unwrap());
}

#[test]
fn a_changed_limit_gives_another_hash() {
    let before = body_of(&example_parameters());
    let mut speed = stored_parameter(1, ("U1", "SPEED"), INT, json!(120));
    speed.key = ParameterKey::parse("1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718").unwrap();
    speed.limits = vec![range_limit("", Some(10), Some(251))];
    let after = body_of(&[Parameter::from(speed)]);
    assert_ne!(before.sha256().unwrap(), after.sha256().unwrap());
}

#[test]
fn every_number_in_the_body_is_an_integer_below_2_pow_53() {
    let body = serde_json::to_value(body_of(&example_parameters())).unwrap();
    let mut stack = vec![&body];
    let mut numbers = 0_u32;
    while let Some(value) = stack.pop() {
        match value {
            Value::Number(number) => {
                numbers = numbers.checked_add(1).unwrap();
                let integer = number.as_u64().unwrap();
                assert!(integer < (1_u64 << 53), "{number}");
            }
            Value::Object(map) => stack.extend(map.values()),
            Value::Array(items) => stack.extend(items),
            _ => {}
        }
    }
    assert!(
        numbers >= 3,
        "the example holds a length, a revision, and a version"
    );
}
