//! Schema tests of closed objects, required fields, and fields of the other type kinds.

use super::{
    Case, Value, assert_cases, assert_fails_with, changed, example, example_with_array, failures,
    json,
};

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
