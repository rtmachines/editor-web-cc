//! Schema tests of the value shapes, the document fields, and their formats.

use super::{
    Case, Value, assert_cases, assert_fails_with, changed, example, example_with_array, failures,
    json,
};

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
