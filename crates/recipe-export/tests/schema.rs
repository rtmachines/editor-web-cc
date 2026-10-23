// reliability.md section 2.3: test code may use `unwrap`, `expect`, and indexing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

//! Tests of the schema file of the export document v1 (design D4, test plan row T3).
//!
//! The valid document is the example of `api.md` section 6.2 with the real hash.
//! Each bad case changes one field and expects one keyword at one instance path.

use jsonschema::Validator;
use serde_json::{Value, json};

/// Path of the schema file under test.
const SCHEMA_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/schema/export-v1.schema.json");

/// The example of `api.md` section 6.2. The hash is the pinned value of
/// `recipe-domain/src/export/hash/tests.rs`, not the placeholder of the example.
const EXAMPLE: &str = r#"{
  "schema_version": "1",
  "exported_at": "2026-10-12T09:30:00Z",
  "exported_by": "jkowalski",
  "loadable": true,
  "content_sha256": "885d9a7d767af484f1c1824b5d493b9ddc19604db05eff1fa4fbded25bad7497",
  "machine": { "code": "M12", "name": "Filler 2", "plc_platform": "S7_300" },
  "recipe": { "reference_number": "R-100", "name": "Bottle 0.5 l" },
  "version": {
    "number": 3, "status": "Approved",
    "approved_at": "2026-10-11T14:02:10Z", "approved_by": "anowak"
  },
  "types": [
    {
      "id": "5d2c7b1e-3f4a-4b6c-8d9e-1a2b3c4d5e6f",
      "name": "STRING", "revision": 1, "kind": "Primitive", "string_length": 16
    },
    {
      "id": "7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f",
      "name": "ZoneTemp", "revision": 2, "kind": "Struct", "string_length": null,
      "members": [
        { "name": "setpoint", "type_id": "REAL" },
        { "name": "label", "type_id": "5d2c7b1e-3f4a-4b6c-8d9e-1a2b3c4d5e6f" }
      ]
    }
  ],
  "parameters": [
    {
      "unit_path": "U1", "tag": "SPEED",
      "key": "1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718",
      "name_en": "Line speed", "type_id": "INT", "s7_address": "DB1.DBW0",
      "limits": [ { "leaf_path": "", "min": "10", "max": "250", "max_length": null } ],
      "value": "120"
    },
    {
      "unit_path": "U1.2", "tag": "ZONE1",
      "key": "9a8b7c6d-5e4f-4321-b0a9-8f7e6d5c4b3a",
      "name_en": "Zone 1", "type_id": "7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f",
      "s7_address": "DB1.DBB2",
      "limits": [
        { "leaf_path": ".label", "min": null, "max": null, "max_length": "16" },
        { "leaf_path": ".setpoint", "min": "20", "max": "95", "max_length": null }
      ],
      "value": { "setpoint": "72.5", "label": "Z1" }
    }
  ]
}"#;

/// One bad case: a name, a change of the document, the instance path, and the keyword.
type Case = (&'static str, fn(&mut Value), &'static str, &'static str);

fn schema() -> Value {
    let text = std::fs::read_to_string(SCHEMA_PATH).expect("read the schema file");
    serde_json::from_str(&text).expect("parse the schema file")
}

/// Compile the schema as draft 2020-12 with the format checks on (design D4).
fn validator() -> Validator {
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema())
        .expect("compile the schema file")
}

fn example() -> Value {
    serde_json::from_str(EXAMPLE).unwrap()
}

/// The example with a third type entry of the kind `Array`.
fn example_with_array() -> Value {
    changed(example(), |doc| {
        doc["types"].as_array_mut().unwrap().push(json!({
            "id": "9e8d7c6b-5a4f-4e3d-8c2b-1a0f9e8d7c6b",
            "name": null, "revision": 1, "kind": "Array", "string_length": null,
            "element_type_id": "INT",
            "dimensions": [ { "lower": 1, "upper": 3 } ]
        }));
    })
}

fn changed(mut doc: Value, change: impl FnOnce(&mut Value)) -> Value {
    change(&mut doc);
    doc
}

/// Each failure as the pair of instance path and keyword.
fn failures(doc: &Value) -> Vec<(String, String)> {
    validator()
        .iter_errors(doc)
        .map(|error| {
            let path = error.instance_path().as_str().to_owned();
            (path, error.kind().keyword().to_owned())
        })
        .collect()
}

fn assert_fails_with(doc: &Value, path: &str, keyword: &str) {
    let found = failures(doc);
    let expected = (path.to_owned(), keyword.to_owned());
    assert!(
        found.contains(&expected),
        "expected {expected:?}, found {found:?}"
    );
}

fn assert_cases(base: fn() -> Value, cases: &[Case]) {
    for (name, change, path, keyword) in cases {
        let found = failures(&changed(base(), change));
        let expected = ((*path).to_owned(), (*keyword).to_owned());
        assert!(
            found.contains(&expected),
            "case {name}: expected {expected:?}, found {found:?}"
        );
    }
}

#[test]
fn schema_file_is_a_valid_draft_2020_12_schema() {
    assert!(jsonschema::draft202012::meta::validate(&schema()).is_ok());
}

#[test]
fn example_of_api_6_2_with_the_real_hash_passes() {
    assert_eq!(failures(&example()), Vec::new());
}

#[test]
fn example_with_an_array_type_passes() {
    assert_eq!(failures(&example_with_array()), Vec::new());
}

#[test]
fn retired_version_with_loadable_false_passes() {
    let doc = changed(example(), |doc| {
        doc["version"]["status"] = json!("Retired");
        doc["loadable"] = json!(false);
    });
    assert_eq!(failures(&doc), Vec::new());
}

#[test]
fn extra_field_fails() {
    let cases: [Case; 4] = [
        (
            "top level",
            |d| d["signature"] = json!("x"),
            "",
            "additionalProperties",
        ),
        (
            "machine",
            |d| d["machine"]["id"] = json!("x"),
            "/machine",
            "additionalProperties",
        ),
        (
            "type entry",
            |d| d["types"][0]["size"] = json!(2),
            "/types/0",
            "additionalProperties",
        ),
        (
            "limit",
            |d| d["parameters"][0]["limits"][0]["step"] = json!("1"),
            "/parameters/0/limits/0",
            "additionalProperties",
        ),
    ];
    assert_cases(example, &cases);
}

#[test]
fn extra_field_of_the_other_closed_objects_fails() {
    let cases: [Case; 5] = [
        (
            "parameter",
            |d| d["parameters"][0]["note"] = json!("x"),
            "/parameters/0",
            "additionalProperties",
        ),
        (
            "version",
            |d| d["version"]["comment"] = json!("x"),
            "/version",
            "additionalProperties",
        ),
        (
            "recipe",
            |d| d["recipe"]["id"] = json!("x"),
            "/recipe",
            "additionalProperties",
        ),
        (
            "member",
            |d| d["types"][1]["members"][0]["default"] = json!("0"),
            "/types/1/members/0",
            "additionalProperties",
        ),
        (
            "dimension",
            |d| d["types"][2]["dimensions"][0]["step"] = json!(1),
            "/types/2/dimensions/0",
            "additionalProperties",
        ),
    ];
    assert_cases(example_with_array, &cases);
}

#[test]
fn missing_required_field_of_each_kind_fails() {
    let cases: [Case; 4] = [
        (
            "Primitive",
            |d| remove(&mut d["types"][0], "string_length"),
            "/types/0",
            "required",
        ),
        (
            "Struct",
            |d| remove(&mut d["types"][1], "members"),
            "/types/1",
            "required",
        ),
        (
            "Array element",
            |d| remove(&mut d["types"][2], "element_type_id"),
            "/types/2",
            "required",
        ),
        (
            "Array dimensions",
            |d| remove(&mut d["types"][2], "dimensions"),
            "/types/2",
            "required",
        ),
    ];
    assert_cases(example_with_array, &cases);
}

fn remove(object: &mut Value, field: &str) {
    object.as_object_mut().unwrap().remove(field).unwrap();
}

#[test]
fn missing_required_field_of_the_document_fails() {
    let doc = changed(example(), |d| remove(d, "loadable"));
    assert_fails_with(&doc, "", "required");
}

/// Remove each field at its object and expect `required` at that object (api.md 6.1).
fn assert_required_fields(rows: &[(&str, &str)]) {
    for (path, field) in rows {
        let doc = changed(example_with_array(), |d| {
            remove(d.pointer_mut(path).unwrap(), field);
        });
        let found = failures(&doc);
        let expected = ((*path).to_owned(), "required".to_owned());
        assert!(
            found.contains(&expected),
            "{path} without {field}: expected {expected:?}, found {found:?}"
        );
    }
}

#[test]
fn missing_required_field_of_a_small_object_fails() {
    assert_required_fields(&[
        ("", "schema_version"),
        ("", "exported_at"),
        ("", "exported_by"),
        ("", "content_sha256"),
        ("", "machine"),
        ("", "recipe"),
        ("", "version"),
        ("", "types"),
        ("", "parameters"),
        ("/machine", "code"),
        ("/machine", "name"),
        ("/machine", "plc_platform"),
        ("/recipe", "reference_number"),
        ("/recipe", "name"),
        ("/version", "number"),
        ("/version", "status"),
        ("/version", "approved_at"),
        ("/version", "approved_by"),
        ("/types/1/members/0", "name"),
        ("/types/1/members/0", "type_id"),
        ("/types/2/dimensions/0", "lower"),
        ("/types/2/dimensions/0", "upper"),
    ]);
}

#[test]
fn missing_required_field_of_a_parameter_or_a_limit_fails() {
    assert_required_fields(&[
        ("/parameters/0", "unit_path"),
        ("/parameters/0", "tag"),
        ("/parameters/0", "key"),
        ("/parameters/0", "name_en"),
        ("/parameters/0", "type_id"),
        ("/parameters/0", "s7_address"),
        ("/parameters/0", "limits"),
        ("/parameters/0", "value"),
        ("/parameters/0/limits/0", "leaf_path"),
        ("/parameters/0/limits/0", "min"),
        ("/parameters/0/limits/0", "max"),
        ("/parameters/0/limits/0", "max_length"),
    ]);
}

#[test]
fn missing_base_field_of_a_type_entry_fails() {
    assert_required_fields(&[
        ("/types/0", "id"),
        ("/types/0", "name"),
        ("/types/0", "revision"),
        ("/types/0", "kind"),
        ("/types/1", "id"),
        ("/types/1", "name"),
        ("/types/1", "kind"),
        ("/types/2", "id"),
        ("/types/2", "name"),
        ("/types/2", "revision"),
        ("/types/2", "kind"),
        ("/types/2", "string_length"),
    ]);
}

#[test]
fn members_on_a_primitive_fails() {
    let doc = changed(example(), |d| {
        d["types"][0]["members"] = json!([{ "name": "a", "type_id": "INT" }]);
    });
    assert_fails_with(&doc, "/types/0", "not");
}

#[test]
fn field_of_another_kind_fails() {
    let cases: [Case; 5] = [
        (
            "element_type_id on Primitive",
            |d| d["types"][0]["element_type_id"] = json!("INT"),
            "/types/0",
            "not",
        ),
        (
            "dimensions on Primitive",
            |d| d["types"][0]["dimensions"] = json!([{ "lower": 1, "upper": 3 }]),
            "/types/0",
            "not",
        ),
        (
            "element_type_id on Struct",
            |d| d["types"][1]["element_type_id"] = json!("INT"),
            "/types/1",
            "not",
        ),
        (
            "dimensions on Struct",
            |d| d["types"][1]["dimensions"] = json!([{ "lower": 1, "upper": 3 }]),
            "/types/1",
            "not",
        ),
        (
            "members on Array",
            |d| d["types"][2]["members"] = json!([{ "name": "a", "type_id": "INT" }]),
            "/types/2",
            "not",
        ),
    ];
    assert_cases(example_with_array, &cases);
}

#[test]
fn null_element_type_id_fails() {
    let doc = changed(example_with_array(), |d| {
        d["types"][2]["element_type_id"] = Value::Null;
    });
    assert_fails_with(&doc, "/types/2/element_type_id", "anyOf");
}

#[test]
fn json_number_or_null_in_value_fails() {
    let cases: [Case; 4] = [
        (
            "number leaf",
            |d| d["parameters"][0]["value"] = json!(120),
            "/parameters/0/value",
            "anyOf",
        ),
        (
            "null leaf",
            |d| d["parameters"][0]["value"] = Value::Null,
            "/parameters/0/value",
            "anyOf",
        ),
        (
            "number member",
            |d| d["parameters"][1]["value"]["setpoint"] = json!(72.5),
            "/parameters/1/value",
            "anyOf",
        ),
        (
            "null element",
            |d| d["parameters"][0]["value"] = json!(["1", null]),
            "/parameters/0/value",
            "anyOf",
        ),
    ];
    assert_cases(example, &cases);
}

#[test]
fn bad_decimal_text_fails() {
    for text in ["010", "1.50", "-0", "+5", ".5", "1e3", "1.", "0.0", " 1"] {
        let doc = changed(example(), |d| {
            d["parameters"][0]["limits"][0]["min"] = json!(text);
        });
        assert_fails_with(&doc, "/parameters/0/limits/0/min", "pattern");
    }
}

#[test]
fn canonical_decimal_text_passes() {
    for text in [
        "0",
        "7",
        "-12",
        "12.5",
        "-0.25",
        "0.001",
        "123456789012345678901",
    ] {
        let doc = changed(example(), |d| {
            d["parameters"][0]["limits"][0]["max"] = json!(text);
        });
        assert_eq!(failures(&doc), Vec::new(), "text {text:?}");
    }
}

#[test]
fn bad_type_reference_fails() {
    let cases: [Case; 4] = [
        (
            "unknown name",
            |d| d["parameters"][0]["type_id"] = json!("FOO"),
            "/parameters/0/type_id",
            "anyOf",
        ),
        (
            "lower case",
            |d| d["parameters"][0]["type_id"] = json!("int"),
            "/parameters/0/type_id",
            "anyOf",
        ),
        (
            "name of a length-bearing primitive",
            |d| d["types"][1]["members"][1]["type_id"] = json!("STRING"),
            "/types/1/members/1/type_id",
            "anyOf",
        ),
        (
            "upper-case UUID",
            |d| d["types"][2]["element_type_id"] = json!("9E8D7C6B-5A4F-4E3D-8C2B-1A0F9E8D7C6B"),
            "/types/2/element_type_id",
            "anyOf",
        ),
    ];
    assert_cases(example_with_array, &cases);
}

#[test]
fn schema_version_2_fails() {
    let doc = changed(example(), |d| d["schema_version"] = json!("2"));
    assert_fails_with(&doc, "/schema_version", "const");
}

#[test]
fn loadable_true_with_retired_fails() {
    let doc = changed(example(), |d| d["version"]["status"] = json!("Retired"));
    assert_fails_with(&doc, "/loadable", "const");
}

#[test]
fn bad_hash_fails() {
    for text in [
        "885D9A7D767AF484F1C1824B5D493B9DDC19604DB05EFF1FA4FBDED25BAD7497",
        "885d9a7d767af484f1c1824b5d493b9ddc19604db05eff1fa4fbded25bad749",
        "885d9a7d767af484f1c1824b5d493b9ddc19604db05eff1fa4fbded25bad7497a",
        "g85d9a7d767af484f1c1824b5d493b9ddc19604db05eff1fa4fbded25bad7497",
    ] {
        let doc = changed(example(), |d| d["content_sha256"] = json!(text));
        assert_fails_with(&doc, "/content_sha256", "pattern");
    }
}

#[test]
fn bad_time_fails() {
    let cases = [
        ("/exported_at", "2026-10-12T09:30:00+02:00", "pattern"),
        ("/exported_at", "2026-10-12T09:30:00.5Z", "pattern"),
        ("/exported_at", "2026-10-12 09:30:00Z", "pattern"),
        ("/version/approved_at", "2026-10-11T14:02:10z", "pattern"),
        ("/version/approved_at", "2026-13-11T14:02:10Z", "format"),
        ("/version/approved_at", "2026-02-30T14:02:10Z", "format"),
    ];
    for (path, text, keyword) in cases {
        let doc = changed(example(), |d| *d.pointer_mut(path).unwrap() = json!(text));
        assert_fails_with(&doc, path, keyword);
    }
}

#[test]
fn name_and_revision_rule_of_each_type_kind_fails() {
    let cases: [Case; 5] = [
        (
            "Struct name null",
            |d| d["types"][1]["name"] = Value::Null,
            "/types/1/name",
            "type",
        ),
        (
            "Array name",
            |d| d["types"][2]["name"] = json!("A"),
            "/types/2/name",
            "type",
        ),
        (
            "Primitive name",
            |d| d["types"][0]["name"] = json!("INT"),
            "/types/0/name",
            "enum",
        ),
        (
            "Primitive revision",
            |d| d["types"][0]["revision"] = json!(2),
            "/types/0/revision",
            "const",
        ),
        (
            "Array revision",
            |d| d["types"][2]["revision"] = json!(2),
            "/types/2/revision",
            "const",
        ),
    ];
    assert_cases(example_with_array, &cases);
}

#[test]
fn length_members_and_kind_rule_of_each_type_kind_fails() {
    let cases: [Case; 4] = [
        (
            "Primitive length null",
            |d| d["types"][0]["string_length"] = Value::Null,
            "/types/0/string_length",
            "type",
        ),
        (
            "Struct length",
            |d| d["types"][1]["string_length"] = json!(8),
            "/types/1/string_length",
            "type",
        ),
        (
            "Struct no members",
            |d| d["types"][1]["members"] = json!([]),
            "/types/1/members",
            "minItems",
        ),
        (
            "unknown kind",
            |d| d["types"][0]["kind"] = json!("Union"),
            "/types/0/kind",
            "enum",
        ),
    ];
    assert_cases(example_with_array, &cases);
}

#[test]
fn dimension_count_outside_1_to_3_fails() {
    let four = json!([
        { "lower": 1, "upper": 2 }, { "lower": 1, "upper": 2 },
        { "lower": 1, "upper": 2 }, { "lower": 1, "upper": 2 }
    ]);
    let doc = changed(example_with_array(), |d| d["types"][2]["dimensions"] = four);
    assert_fails_with(&doc, "/types/2/dimensions", "maxItems");
    let doc = changed(example_with_array(), |d| {
        d["types"][2]["dimensions"] = json!([])
    });
    assert_fails_with(&doc, "/types/2/dimensions", "minItems");
}

#[test]
fn s7_address_follows_the_platform_rule_of_a31() {
    let null_on_s7_300 = changed(example(), |d| {
        d["parameters"][0]["s7_address"] = Value::Null;
    });
    assert_fails_with(&null_on_s7_300, "/parameters/0/s7_address", "type");
    for platform in ["CodesysIec", "TwinCat3"] {
        let doc = changed(example(), |d| {
            d["machine"]["plc_platform"] = json!(platform)
        });
        assert_fails_with(&doc, "/parameters/0/s7_address", "type");
    }
    let optional_on_s7_1500 = changed(example(), |d| {
        d["machine"]["plc_platform"] = json!("S7_1500");
        d["parameters"][0]["s7_address"] = Value::Null;
    });
    assert_eq!(failures(&optional_on_s7_1500), Vec::new());
}

#[test]
fn bad_enum_value_fails() {
    let cases: [Case; 2] = [
        (
            "status",
            |d| d["version"]["status"] = json!("Draft"),
            "/version/status",
            "enum",
        ),
        (
            "platform",
            |d| d["machine"]["plc_platform"] = json!("S7_400"),
            "/machine/plc_platform",
            "enum",
        ),
    ];
    assert_cases(example, &cases);
}

#[test]
fn bad_grammar_of_a_code_or_a_name_fails() {
    let cases: [Case; 5] = [
        (
            "machine code",
            |d| d["machine"]["code"] = json!("M012"),
            "/machine/code",
            "pattern",
        ),
        (
            "login",
            |d| d["exported_by"] = json!("J.Kowalski"),
            "/exported_by",
            "pattern",
        ),
        (
            "reference",
            |d| d["recipe"]["reference_number"] = json!("r 100"),
            "/recipe/reference_number",
            "pattern",
        ),
        (
            "tag",
            |d| d["parameters"][0]["tag"] = json!("speed"),
            "/parameters/0/tag",
            "pattern",
        ),
        (
            "leaf path",
            |d| d["parameters"][1]["limits"][0]["leaf_path"] = json!("label"),
            "/parameters/1/limits/0/leaf_path",
            "pattern",
        ),
    ];
    assert_cases(example, &cases);
}

#[test]
fn bad_grammar_of_a_path_or_an_identifier_fails() {
    let cases: [Case; 5] = [
        (
            "unit path zero",
            |d| d["parameters"][0]["unit_path"] = json!("U0"),
            "/parameters/0/unit_path",
            "pattern",
        ),
        (
            "unit path 65",
            |d| d["parameters"][0]["unit_path"] = json!("U1.65"),
            "/parameters/0/unit_path",
            "pattern",
        ),
        (
            "unit path depth",
            |d| d["parameters"][0]["unit_path"] = json!("U1.2.3.4.5.6"),
            "/parameters/0/unit_path",
            "pattern",
        ),
        (
            "key",
            |d| d["parameters"][0]["key"] = json!("1F3A6B2C-8D9E-4F00-A1B2-C3D4E5F60718"),
            "/parameters/0/key",
            "pattern",
        ),
        (
            "type id",
            |d| d["types"][0]["id"] = json!("5d2c7b1e"),
            "/types/0/id",
            "pattern",
        ),
    ];
    assert_cases(example, &cases);
}

#[test]
fn leaf_path_of_the_grammar_passes() {
    for path in [
        "",
        ".setpoint",
        "[3]",
        "[1,2]",
        "[-1,0,7]",
        ".zones[2].temp_max",
        ".zones[*].temp_max",
    ] {
        let doc = changed(example(), |d| {
            d["parameters"][1]["limits"][0]["leaf_path"] = json!(path);
        });
        assert_eq!(failures(&doc), Vec::new(), "path {path:?}");
    }
}
