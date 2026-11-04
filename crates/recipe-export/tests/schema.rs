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
    compiled(&schema())
}

fn compiled(schema: &Value) -> Validator {
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .expect("compile the schema file")
}

/// Path of the schema file of the document v2 (the unit scope).
const SCHEMA_V2_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/schema/export-v2.schema.json");

/// Path of the golden document v2 of an `Approved` version.
const GOLDEN_V2_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/golden/recipe_R-100_v3_U1.2.json"
);

/// The golden document v2 after one change.
fn v2_document(change: impl FnOnce(&mut Value)) -> Value {
    let text = std::fs::read_to_string(GOLDEN_V2_PATH).expect("read the golden document v2");
    changed(
        serde_json::from_str(&text).expect("parse the golden document v2"),
        change,
    )
}

/// Each failure of a document v2 as the pair of instance path and keyword.
fn v2_failures(doc: &Value) -> Vec<(String, String)> {
    let text = std::fs::read_to_string(SCHEMA_V2_PATH).expect("read the schema file v2");
    let schema = serde_json::from_str(&text).expect("parse the schema file v2");
    compiled(&schema)
        .iter_errors(doc)
        .map(|error| {
            let path = error.instance_path().as_str().to_owned();
            (path, error.kind().keyword().to_owned())
        })
        .collect()
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
fn approved_version_with_loadable_false_passes() {
    let doc = changed(example(), |doc| doc["loadable"] = json!(false));
    assert_eq!(failures(&doc), Vec::new());
}

#[test]
fn v2_golden_document_passes() {
    assert_eq!(v2_failures(&v2_document(|_| {})), Vec::new());
}

#[test]
fn v2_approved_version_with_loadable_false_passes() {
    let doc = v2_document(|doc| doc["loadable"] = json!(false));
    assert_eq!(v2_failures(&doc), Vec::new());
}

#[test]
fn v2_loadable_true_with_retired_fails() {
    let doc = v2_document(|doc| doc["version"]["status"] = json!("Retired"));
    let expected = ("/loadable".to_owned(), "const".to_owned());
    let found = v2_failures(&doc);
    assert!(
        found.contains(&expected),
        "expected {expected:?}, found {found:?}"
    );
}

// A test target file is a crate root, so `#[path]` places the modules under `tests/schema/`.
#[path = "schema/structure.rs"]
mod structure;
#[path = "schema/type_rules.rs"]
mod type_rules;
#[path = "schema/values.rs"]
mod values;
