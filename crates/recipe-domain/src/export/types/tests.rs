use serde_json::{Value, json};

use super::collect_types;
use crate::export::test_support::{
    BOOL, INT, LABEL_BOX, REAL, STRING_8, STRING_16, STRING_ARRAY, TIME, ZONE_TEMP, trees, type_id,
};
use crate::plant::PlcPlatform;

const PLATFORM: PlcPlatform = PlcPlatform::S7_300;

/// The entries of the types that the parameters of the given root types use, as JSON.
fn entries(roots: &[&str]) -> Vec<Value> {
    let trees = trees(PLATFORM);
    let nodes: Vec<_> = roots
        .iter()
        .map(|root| trees.tree(type_id(root)).unwrap().root())
        .collect();
    collect_types(nodes)
        .iter()
        .map(|entry| serde_json::to_value(entry).unwrap())
        .collect()
}

#[test]
fn primitives_without_a_length_give_no_entry() {
    assert!(entries(&[INT, REAL, BOOL, TIME]).is_empty());
}

#[test]
fn array_of_strings_gives_the_array_and_the_string() {
    let found = entries(&[STRING_ARRAY]);
    assert_eq!(
        found,
        vec![
            json!({
                "id": "30000000-0000-4000-8000-000000000001",
                "name": null,
                "revision": 1,
                "kind": "Array",
                "string_length": null,
                "element_type_id": STRING_16,
                "dimensions": [{ "lower": 1, "upper": 4 }]
            }),
            json!({
                "id": STRING_16,
                "name": "STRING",
                "revision": 1,
                "kind": "Primitive",
                "string_length": 16
            }),
        ]
    );
}

#[test]
fn struct_gives_its_entry_and_the_entries_of_its_members() {
    let found = entries(&[ZONE_TEMP]);
    assert_eq!(
        found,
        vec![
            json!({
                "id": STRING_16,
                "name": "STRING",
                "revision": 1,
                "kind": "Primitive",
                "string_length": 16
            }),
            json!({
                "id": ZONE_TEMP,
                "name": "ZoneTemp",
                "revision": 2,
                "kind": "Struct",
                "string_length": null,
                "members": [
                    { "name": "setpoint", "type_id": "REAL" },
                    { "name": "label", "type_id": STRING_16 }
                ]
            }),
        ]
    );
}

#[test]
fn a_type_used_by_two_parameters_appears_once() {
    let found = entries(&[ZONE_TEMP, STRING_16, ZONE_TEMP, LABEL_BOX]);
    let ids: Vec<&str> = found.iter().filter_map(|e| e["id"].as_str()).collect();
    assert_eq!(ids, vec![LABEL_BOX, STRING_16, ZONE_TEMP]);
}

#[test]
fn entries_are_ordered_by_id_text() {
    let found = entries(&[ZONE_TEMP, STRING_8, STRING_ARRAY]);
    let ids: Vec<&str> = found.iter().filter_map(|e| e["id"].as_str()).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted);
    assert_eq!(ids.len(), 4);
}

#[test]
fn presence_rule_has_no_null_for_array_and_struct_only_fields() {
    for entry in entries(&[ZONE_TEMP, STRING_ARRAY]) {
        let object = entry.as_object().unwrap();
        let kind = object["kind"].as_str().unwrap();
        assert_eq!(object.contains_key("members"), kind == "Struct");
        assert_eq!(object.contains_key("dimensions"), kind == "Array");
        assert_eq!(object.contains_key("element_type_id"), kind == "Array");
        for key in ["id", "name", "revision", "kind", "string_length"] {
            assert!(object.contains_key(key), "{key} is missing");
        }
    }
}
