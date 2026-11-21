//! Golden tests of the string encoding in the unit documents (ADR-0202 (j1), (j2), (k), (l)).
//!
//! The module belongs to `golden.rs`. It reuses the private helpers of that file.

use std::collections::BTreeSet;

use recipe_domain::export::{
    ExportHeader, ExportStatus, HashedBody, ScopeBody, ScopeFacts, check_version_parameters,
};
use recipe_domain::plant::{
    MachineCode, MachineId, MemberName, PlatformTypes, PlcPlatform, PlcType, PlcTypeRecord,
    Primitive, RecordShape, RuleSet, StringEncoding, TypeMember, TypeName, TypeTrees, UnitPath,
};
use recipe_domain::recipe::{
    Ordinal, Parameter, ParameterId, ParameterKey, ParameterTag, ParameterValue, ReferenceNumber,
    StoredParameter, StoredValue,
};
use recipe_domain::text::{LongText, Text};
use recipe_export::write_scoped;
use serde_json::Value;
use uuid::Uuid;

use super::{
    INT, SCHEMA_V2_PATH, Spec, assert_passes_schema, consumer_scope_sha256, example, facts, golden,
    machine_id, parameter, primitive_row, text_member, type_id, unit_id, units, utc, validator_of,
    written,
};

/// The golden file of the `Latin1` `CodesysIec` unit document (ADR-0202 (j2)).
const LATIN1_FILE: &str = "recipe_R-200_v1_U1_CodesysIec.json";
/// The type rows of the stored rows of the matrix in `design.md`.
const BOOL_ROW: &str = "00000000-0000-4000-8000-000000000011";
const STRING_4_ROW: &str = "00000000-0000-4000-8000-000000000012";
const ST_COIL_ROW: &str = "00000000-0000-4000-8000-000000000013";
const ST_ONE_ROW: &str = "00000000-0000-4000-8000-000000000014";

/// A type row under the earlier rule set, as the adapter of `recipe-db` reads a stored row.
fn stored_row(owner: Option<MachineId>, id: &str, shape: RecordShape) -> PlcType {
    PlcType::from_stored(
        PlcTypeRecord {
            id: type_id(id),
            machine: owner,
            platforms: BTreeSet::from([PlcPlatform::CodesysIec]),
            shape,
        },
        RuleSet::Earlier,
    )
    .unwrap()
}

fn stored_primitive(id: &str, primitive: Primitive, string_length: Option<u16>) -> PlcType {
    let size_bytes = match string_length {
        Some(length) => u32::from(length).checked_add(2).unwrap(),
        None => primitive.fixed_size_bytes().unwrap(),
    };
    let shape = RecordShape::Primitive {
        primitive,
        leaf_class: primitive.leaf_class(),
        size_bytes,
        string_length,
    };
    stored_row(string_length.map(|_| machine_id()), id, shape)
}

fn stored_struct(id: &str, name: &str, members: &[(&str, &str)]) -> PlcType {
    let members = members
        .iter()
        .map(|(member, type_text)| TypeMember {
            name: MemberName::parse(member).unwrap(),
            type_id: type_id(type_text),
        })
        .collect();
    let shape = RecordShape::Struct {
        name: TypeName::from_stored(name, RuleSet::Earlier).unwrap(),
        revision: 1,
        supersedes: None,
        members,
    };
    stored_row(Some(machine_id()), id, shape)
}

/// The type trees of a `CodesysIec` machine: `STRING(4)`, `ST_Coil@1`, and `ST_One@1`.
fn latin1_trees() -> TypeTrees {
    let platform = PlcPlatform::CodesysIec;
    let rows = vec![
        stored_primitive(INT, Primitive::Int, None),
        stored_primitive(BOOL_ROW, Primitive::Bool, None),
        stored_primitive(STRING_4_ROW, Primitive::String, Some(4)),
        stored_struct(
            ST_COIL_ROW,
            "ST_Coil",
            &[("END_IF", BOOL_ROW), ("S", BOOL_ROW)],
        ),
        stored_struct(ST_ONE_ROW, "ST_One", &[("value", INT)]),
    ];
    let catalog =
        PlatformTypes::collect(machine_id(), platform, StringEncoding::Latin1, rows).unwrap();
    TypeTrees::resolve(
        [STRING_4_ROW, ST_COIL_ROW, ST_ONE_ROW].map(type_id),
        &catalog,
    )
}

/// A parameter of the version. The value goes through `from_stored` under the earlier rule set.
fn latin1_parameter(
    (n, key, tag): (u32, &str, &str),
    plc_type: &str,
    value: &str,
    trees: &TypeTrees,
) -> Parameter {
    let json: Value = serde_json::from_str(value).unwrap();
    let tree = trees.tree(type_id(plc_type)).unwrap();
    let stored = ParameterValue::from_stored(&StoredValue::from(json), tree, RuleSet::Earlier)
        .unwrap()
        .to_stored();
    Parameter::from(StoredParameter {
        id: ParameterId::from(Uuid::from_u128(u128::from(n))),
        key: ParameterKey::parse(key).unwrap(),
        unit: unit_id("U1"),
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: Text::parse(tag).unwrap(),
        description_en: LongText::parse("Not in the document").unwrap(),
        translations: Vec::new(),
        plc_type: type_id(plc_type),
        s7_address: None,
        limits: Vec::new(),
        value: stored,
        ordinal: Ordinal::parse(n).unwrap(),
    })
}

/// The scope body and the facts of the unit `U1` of a `Latin1` `CodesysIec` version `R-200` v1.
///
/// A `STRING` value counts characters as Latin-1.
fn latin1_scope() -> (ScopeBody, ScopeFacts) {
    codesys_scope(StringEncoding::Latin1)
}

/// The scope body and the facts of the unit `U1` of a `CodesysIec` version `R-200` v1, with the
/// encoding that the machine row gives.
fn codesys_scope(encoding: StringEncoding) -> (ScopeBody, ScopeFacts) {
    let platform = PlcPlatform::CodesysIec;
    let trees = latin1_trees();
    let parameters = [
        latin1_parameter(
            (1, "3c5e7a90-1b2d-4f3a-8b4c-5d6e7f8091a2", "LABEL"),
            STRING_4_ROW,
            r#""é""#,
            &trees,
        ),
        latin1_parameter(
            (2, "4d6f8ba1-2c3e-4a4b-9c5d-6e7f8091a2b3", "COIL"),
            ST_COIL_ROW,
            r#"{"S": false, "END_IF": true}"#,
            &trees,
        ),
        latin1_parameter(
            (3, "5e7a9cb2-3d4f-4b5c-8d6e-7f8091a2b3c4", "ONE"),
            ST_ONE_ROW,
            r#"{"value": 3}"#,
            &trees,
        ),
    ];
    let header = ExportHeader {
        machine: MachineCode::parse("M12").unwrap(),
        platform,
        reference: ReferenceNumber::parse("R-200").unwrap(),
        version: 1,
    };
    let body = HashedBody::build(&header, &parameters, &units(platform), &trees).unwrap();
    let times = (
        utc(2026, (10, 12), (9, 30, 0)),
        utc(2026, (10, 11), (14, 2, 10)),
    );
    let mut document = facts(&body, &parameters, ExportStatus::Approved, times);
    document.has_rule_fault = !check_version_parameters(&parameters, &trees)
        .unwrap()
        .is_empty();
    let scope = ScopeBody::build(&body, &UnitPath::parse("U1").unwrap(), encoding);
    let facts = ScopeFacts {
        document,
        scope_sha256: scope.sha256().unwrap(),
    };
    (scope, facts)
}

/// Pins the bytes of the production writer for a `Latin1` machine before the writer carries the
/// string encoding (ADR-0202 (j2)). The test needs no code change to pass.
#[test]
fn adr_0202_latin1_codesys_unit_document() {
    let (scope, facts) = latin1_scope();
    let text = String::from_utf8(write_scoped(&scope, &facts).unwrap()).unwrap();
    assert_eq!(text, golden(LATIN1_FILE));

    let doc: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(doc["machine"]["plc_platform"], "CodesysIec");
    assert_eq!(doc["loadable"], true);
    assert_eq!(
        consumer_scope_sha256(&doc),
        text_member(&doc, "scope_sha256")
    );
    assert_passes_schema(&validator_of(SCHEMA_V2_PATH), &doc, LATIN1_FILE);

    assert!(doc.get("layout").is_none(), "{text}");
    assert!(doc["machine"].get("string_encoding").is_none(), "{text}");
    let parameters = doc["parameters"].as_array().unwrap();
    assert_eq!(parameters.len(), 3);
    for parameter in parameters {
        assert_eq!(
            parameter.get("s7_address"),
            Some(&Value::Null),
            "{parameter}"
        );
        assert!(parameter.get("layout").is_none(), "{parameter}");
    }
    let label = parameters
        .iter()
        .find(|parameter| parameter["tag"] == "LABEL")
        .unwrap();
    assert_eq!(label["value"], "é");
    let names: BTreeSet<&str> = doc["types"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["name"].as_str())
        .collect();
    assert!(names.contains("ST_Coil"), "{names:?}");
    assert!(names.contains("ST_One"), "{names:?}");
}

/// The object `machine` of a `Utf8` document v2: the encoding follows the platform (ADR-0202 (j1)).
const UTF8_MACHINE_OBJECT: &str = concat!(
    "  \"machine\": {\n",
    "    \"code\": \"M12\",\n",
    "    \"name\": \"Filler 2\",\n",
    "    \"plc_platform\": \"CodesysIec\",\n",
    "    \"string_encoding\": \"Utf8\"\n",
    "  },\n",
);

#[test]
fn adr_0202_utf8_unit_document_has_the_encoding_directly_after_the_platform() {
    let (scope, facts) = codesys_scope(StringEncoding::Utf8);
    let text = String::from_utf8(write_scoped(&scope, &facts).unwrap()).unwrap();
    assert!(text.contains(UTF8_MACHINE_OBJECT), "{text}");

    let doc: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(doc["machine"]["string_encoding"], "Utf8");
    assert_passes_schema(&validator_of(SCHEMA_V2_PATH), &doc, "a Utf8 unit document");
}

#[test]
fn adr_0202_utf8_unit_document_differs_from_the_latin1_document_in_the_encoding_and_the_hash() {
    let (latin1_scope, latin1_facts) = latin1_scope();
    let (utf8_scope, utf8_facts) = codesys_scope(StringEncoding::Utf8);
    let latin1 = String::from_utf8(write_scoped(&latin1_scope, &latin1_facts).unwrap()).unwrap();
    let utf8 = String::from_utf8(write_scoped(&utf8_scope, &utf8_facts).unwrap()).unwrap();
    assert_ne!(latin1_facts.scope_sha256, utf8_facts.scope_sha256);
    let line_of_hash = |text: &str| {
        text.lines()
            .find(|line| line.contains("\"scope_sha256\""))
            .unwrap()
            .to_owned()
    };
    let without_encoding = utf8
        .replace(",\n    \"string_encoding\": \"Utf8\"", "")
        .replace(&line_of_hash(&utf8), &line_of_hash(&latin1));
    assert_eq!(without_encoding, latin1);
}

#[test]
fn adr_0202_v1_document_never_holds_the_encoding() {
    let text = written(example());
    assert!(!text.contains("string_encoding"), "{text}");
}

/// The golden file of the `Utf8` `TwinCat3` unit document (ADR-0202 (j2), (k), (l)).
const UTF8_TWINCAT_FILE: &str = "recipe_R-300_v1_U1_TwinCat3_Utf8.json";
/// The type row of `STRING(10)` of the `Utf8` `TwinCat3` machine.
const STRING_10_ROW: &str = "00000000-0000-4000-8000-000000000021";

/// The type trees of a `Utf8` `TwinCat3` machine: `STRING(10)`.
fn utf8_twincat_trees() -> TypeTrees {
    let platform = PlcPlatform::TwinCat3;
    let rows = vec![primitive_row(
        platform,
        STRING_10_ROW,
        Primitive::String,
        Some(10),
    )];
    let catalog =
        PlatformTypes::collect(machine_id(), platform, StringEncoding::Utf8, rows).unwrap();
    TypeTrees::resolve([STRING_10_ROW].map(type_id), &catalog)
}

/// The scope body and the facts of the unit `U1` of a `Utf8` `TwinCat3` version `R-300` v1.
///
/// The parameter holds `"Łódź"`: four characters and seven bytes.
fn utf8_twincat_scope() -> (ScopeBody, ScopeFacts) {
    let platform = PlcPlatform::TwinCat3;
    let trees = utf8_twincat_trees();
    let parameters = [parameter(Spec {
        n: 1,
        key: "6f8a0dc3-4e50-4c6d-9e7f-8091a2b3c4d5",
        unit_tag: ("U1", "LABEL"),
        name: "Label",
        plc_type: STRING_10_ROW,
        address: None,
        limits: Vec::new(),
        value: r#""Łódź""#,
    })];
    let header = ExportHeader {
        machine: MachineCode::parse("M10").unwrap(),
        platform,
        reference: ReferenceNumber::parse("R-300").unwrap(),
        version: 1,
    };
    let body = HashedBody::build(&header, &parameters, &units(platform), &trees).unwrap();
    let times = (
        utc(2026, (10, 12), (9, 30, 0)),
        utc(2026, (10, 11), (14, 2, 10)),
    );
    let mut document = facts(&body, &parameters, ExportStatus::Approved, times);
    document.machine_name = Text::parse("Packer 10").unwrap();
    document.recipe_name = Text::parse("Jar 0.3 l").unwrap();
    document.string_encoding = StringEncoding::Utf8;
    document.has_rule_fault = !check_version_parameters(&parameters, &trees)
        .unwrap()
        .is_empty();
    let scope = ScopeBody::build(&body, &UnitPath::parse("U1").unwrap(), StringEncoding::Utf8);
    let facts = ScopeFacts {
        document,
        scope_sha256: scope.sha256().unwrap(),
    };
    (scope, facts)
}

/// Pins the bytes of the production writer for a `Utf8` `TwinCat3` machine (ADR-0202 (j2)).
#[test]
fn adr_0202_utf8_twincat_unit_document() {
    let (scope, facts) = utf8_twincat_scope();
    let text = String::from_utf8(write_scoped(&scope, &facts).unwrap()).unwrap();
    assert_eq!(text, golden(UTF8_TWINCAT_FILE));

    let doc: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        consumer_scope_sha256(&doc),
        text_member(&doc, "scope_sha256")
    );
    assert_passes_schema(&validator_of(SCHEMA_V2_PATH), &doc, UTF8_TWINCAT_FILE);
    assert_eq!(doc["machine"]["string_encoding"], "Utf8");
    assert_eq!(doc["machine"]["plc_platform"], "TwinCat3");
    assert_eq!(doc["loadable"], true);

    assert!(doc.get("layout").is_none(), "{text}");
    let parameters = doc["parameters"].as_array().unwrap();
    assert_eq!(parameters.len(), 1);
    for parameter in parameters {
        assert_eq!(
            parameter.get("s7_address"),
            Some(&Value::Null),
            "{parameter}"
        );
        assert!(parameter.get("layout").is_none(), "{parameter}");
    }
    let label = parameters
        .iter()
        .find(|parameter| parameter["type_id"] == STRING_10_ROW)
        .unwrap();
    assert_eq!(label["value"], "Łódź");
    assert_eq!("Łódź".len(), 7);
}
