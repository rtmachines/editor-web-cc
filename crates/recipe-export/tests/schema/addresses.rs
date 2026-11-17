//! Schema tests of the pattern of the S7 address in the document v2 (ADR-0189).
//!
//! Each form runs against the v2 schema file and against `S7Address::parse`, so the two grammars
//! cannot drift apart.

use recipe_domain::plant::S7Address;

use super::{Value, example, json, v2_document, v2_failures};

/// One passing address of each width `X`, `B`, `W`, and `D`, with the edges of the ranges.
const PASSING: [&str; 4] = ["DB1.DBX0.7", "DB2.DBB10", "DB65535.DBW65534", "DB12.DBD4"];

/// The seven failing forms of ADR-0189 (d).
const FAILING: [&str; 7] = [
    "DB1.DBX0.8",
    "DB0.DBW0",
    "DB65536.DBB0",
    "DB1.DBW65536",
    "DB1.DBB0.1",
    "DB1.DBX0",
    "db1.dbw0",
];

/// The golden document v2 with one address in its first parameter.
fn v2_with_address(address: &str) -> Value {
    v2_document(|doc| doc["parameters"][0]["s7_address"] = json!(address))
}

/// The keyword of a failure of the address of the first parameter, if there is one.
fn address_failure(doc: &Value) -> Option<String> {
    v2_failures(doc)
        .into_iter()
        .find(|(path, _)| path == "/parameters/0/s7_address")
        .map(|(_, keyword)| keyword)
}

#[test]
fn adr_0189_each_width_of_a_valid_address_passes_the_v2_file() {
    for address in PASSING {
        assert_eq!(
            v2_failures(&v2_with_address(address)),
            Vec::new(),
            "address {address}"
        );
    }
}

#[test]
fn adr_0189_a_null_address_passes_the_v2_file() {
    let doc = v2_document(|doc| {
        doc["machine"]["plc_platform"] = json!("S7_1500");
        doc["parameters"][0]["s7_address"] = Value::Null;
    });
    assert_eq!(v2_failures(&doc), Vec::new());
}

#[test]
fn adr_0189_each_bad_form_fails_the_v2_file_with_the_keyword_pattern() {
    for address in FAILING {
        assert_eq!(
            address_failure(&v2_with_address(address)).as_deref(),
            Some("pattern"),
            "address {address}"
        );
    }
}

#[test]
fn adr_0189_the_schema_and_the_parser_give_the_same_answer() {
    for address in PASSING.iter().chain(FAILING.iter()) {
        let schema_accepts = address_failure(&v2_with_address(address)).is_none();
        let parser_accepts = S7Address::parse(address).is_ok();
        assert_eq!(schema_accepts, parser_accepts, "address {address}");
    }
}

#[test]
fn adr_0189_a_bit_beyond_seven_still_passes_the_v1_file() {
    let doc = super::changed(example(), |d| {
        d["parameters"][0]["s7_address"] = json!("DB1.DBX0.8");
    });
    assert_eq!(super::failures(&doc), Vec::new());
}
