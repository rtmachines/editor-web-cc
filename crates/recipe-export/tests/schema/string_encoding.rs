//! Schema tests of the optional field `machine.string_encoding` of the document v2 (ADR-0202 (j3)).
//!
//! The field has the enum `["Utf8"]`. The schema permits it only on `CodesysIec` and `TwinCat3`.

use recipe_domain::plant::PlcPlatform;

use super::platform_rules::{assert_fails_at, assert_passes, base};
use super::{Value, changed, json};

/// The platforms that permit the field.
const CODESYS_FAMILY: [PlcPlatform; 2] = [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3];
/// The platforms that refuse the field.
const S7_FAMILY: [PlcPlatform; 3] = [
    PlcPlatform::S7_300,
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
];

/// The valid document of `platform` with `machine.string_encoding` set to `encoding`.
fn with_encoding(platform: PlcPlatform, encoding: &str) -> Value {
    changed(base(platform), |doc| {
        doc["machine"]["string_encoding"] = json!(encoding);
    })
}

#[test]
fn adr_0202_document_without_the_field_passes_on_each_platform() {
    for platform in CODESYS_FAMILY.into_iter().chain(S7_FAMILY) {
        let doc = base(platform);
        assert!(doc["machine"].get("string_encoding").is_none());
        assert_passes(&doc);
    }
}

#[test]
fn adr_0202_utf8_passes_on_codesys_and_twincat3() {
    for platform in CODESYS_FAMILY {
        assert_passes(&with_encoding(platform, "Utf8"));
    }
}

#[test]
fn adr_0202_utf8_fails_on_each_s7_platform() {
    for platform in S7_FAMILY {
        let doc = with_encoding(platform, "Utf8");
        let failures = super::v2_failures(&doc);
        assert!(
            failures
                .iter()
                .any(|(path, _)| path == "/machine/string_encoding"),
            "{}: {failures:?}",
            platform.as_str()
        );
    }
}

#[test]
fn adr_0202_explicit_latin1_fails_on_every_platform() {
    for platform in CODESYS_FAMILY.into_iter().chain(S7_FAMILY) {
        assert_fails_at(
            &with_encoding(platform, "Latin1"),
            "/machine/string_encoding",
            "enum",
        );
    }
}

#[test]
fn adr_0202_unknown_encoding_fails() {
    assert_fails_at(
        &with_encoding(PlcPlatform::CodesysIec, "Ucs2"),
        "/machine/string_encoding",
        "enum",
    );
}
