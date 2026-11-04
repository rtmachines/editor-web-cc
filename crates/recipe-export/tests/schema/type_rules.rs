//! Schema tests of the rules of each type kind and of the grammar of codes and paths.

use super::{
    Case, Value, assert_cases, assert_fails_with, changed, example, example_with_array, failures,
    json,
};

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
