// reliability.md section 2.3: test code may use `unwrap`, `expect`, and indexing. The options of
// `clippy.toml` do not cover the helpers of an integration test crate.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

//! Export documents with one parameter for each primitive type of a platform (design D4 of the
//! change `plc-type-test-coverage`).
//!
//! The type table below is written by hand from `domain-model.md` section 2.1 and `data-model.md`
//! section 7. The expected texts do not come from production code. The tests build each value with
//! `ParameterValue::parse` and `to_stored`, write the document, and compare the parsed output with
//! the table.

use std::collections::{BTreeMap, BTreeSet};

use jsonschema::Validator;
use recipe_domain::export::{
    DocumentFacts, ExportHeader, ExportStatus, HashedBody, check_version_parameters,
};
use recipe_domain::identity::Login;
use recipe_domain::plant::{
    LeafPath, LiveUnits, Machine, MachineCode, MachineId, PlatformTypes, PlcPlatform, PlcType,
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, RuleSet, S7Address, StoredMachine,
    StoredUnit, TypeTrees, UnitId, UnitPath,
};
use recipe_domain::recipe::{
    Ordinal, Parameter, ParameterId, ParameterKey, ParameterTag, ParameterValue, ReferenceNumber,
    StoredLimit, StoredParameter, StoredValue,
};
use recipe_domain::text::{LongText, Text};
use recipe_domain::timestamp::Timestamp;
use recipe_export::write_document;
use rust_decimal::Decimal;
use serde_json::{Value, json};
use time::{Date, Month};
use uuid::Uuid;

/// The directory of the golden files.
const GOLDEN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/");
/// The path of the schema file of the document v1.
const SCHEMA_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/schema/export-v1.schema.json");

/// The id of the row `STRING(16)` (design D4).
const STRING_16: &str = "00000000-0000-4000-8000-000000000016";
/// The id of the row `WSTRING(16)` (design D4).
const WSTRING_16: &str = "00000000-0000-4000-8000-000000000116";
/// The length of the string rows of the documents.
const STRING_LENGTH: u16 = 16;

#[path = "golden_types/table.rs"]
mod table;
use table::{
    CODESYS_TAGS, Limit, S7_300_ADDRESSES, S7_300_TAGS, S7_1200_TAGS, S7_1500_TAGS,
    codesys_type_table, platform_type_table, type_table,
};

fn type_id(text: &str) -> PlcTypeId {
    PlcTypeId::from(Uuid::parse_str(text).unwrap())
}

fn machine_id() -> MachineId {
    MachineId::from(Uuid::from_u128(0xAA))
}

fn unit_id() -> UnitId {
    UnitId::from(Uuid::from_u128(1))
}

/// The row of a tag. A fixed-size primitive gets an id that no string row uses. The index is the
/// position of the tag in the S7 table, or after it for a tag that only CODESYS has.
fn type_row_id(tag: &str) -> String {
    let index = type_table()
        .iter()
        .chain(codesys_type_table().iter())
        .position(|case| case.tag == tag)
        .unwrap();
    match tag {
        "STRING" => STRING_16.to_owned(),
        "WSTRING" => WSTRING_16.to_owned(),
        _ => format!("00000000-0000-4000-8000-1{index:011x}"),
    }
}

fn type_row(platform: PlcPlatform, tag: &str) -> PlcType {
    let primitive = Primitive::parse(tag).unwrap();
    let length = u32::from(STRING_LENGTH);
    // `STRING(n)` takes n + 2 bytes. `WSTRING(n)` takes 2n + 4 bytes (`domain-model.md` section 2.1).
    let (owner, string_length, size_bytes) = match tag {
        "STRING" => (
            Some(machine_id()),
            Some(STRING_LENGTH),
            length.checked_add(2).unwrap(),
        ),
        "WSTRING" => (
            Some(machine_id()),
            Some(STRING_LENGTH),
            length.checked_mul(2).unwrap().checked_add(4).unwrap(),
        ),
        _ => (None, None, primitive.fixed_size_bytes().unwrap()),
    };
    PlcType::try_from(PlcTypeRecord {
        id: type_id(&type_row_id(tag)),
        machine: owner,
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes,
            string_length,
        },
    })
    .unwrap()
}

fn trees(platform: PlcPlatform, tags: &[&str]) -> TypeTrees {
    let rows: Vec<PlcType> = tags.iter().map(|tag| type_row(platform, tag)).collect();
    let catalog = PlatformTypes::collect(machine_id(), platform, rows).unwrap();
    TypeTrees::resolve(tags.iter().map(|tag| type_id(&type_row_id(tag))), &catalog)
}

/// The machine `M12` "Filler 2" with the unit `U1`.
fn units(platform: PlcPlatform) -> LiveUnits {
    let unit = StoredUnit {
        id: unit_id(),
        path: UnitPath::parse("U1").unwrap(),
        name: Text::parse("U1").unwrap(),
        archived_at: None,
    };
    Machine::try_from(StoredMachine {
        id: machine_id(),
        code: MachineCode::parse("M12").unwrap(),
        name: Text::parse("Filler 2").unwrap(),
        platform,
        db_number: platform
            .max_data_block()
            .map(|_| recipe_domain::plant::DataBlockNumber::parse("1").unwrap()),
        archived_at: None,
        units: vec![unit],
    })
    .unwrap()
    .live_units()
}

fn limit_of(limit: &Limit) -> Option<StoredLimit> {
    let (min, max, max_length) = match limit {
        Limit::None => return None,
        Limit::Range(min, max) => (
            Some(min.parse::<Decimal>().unwrap()),
            Some(max.parse::<Decimal>().unwrap()),
            None,
        ),
        Limit::Length(length) => (None, None, Some(*length)),
    };
    Some(StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min,
        max,
        max_length,
    })
}

/// The number of a parameter, counted from 1 in the order of the tags.
fn number(index: usize) -> u32 {
    u32::try_from(index).unwrap().checked_add(1).unwrap()
}

fn parameter(
    platform: PlcPlatform,
    index: usize,
    tag: &str,
    address: Option<&str>,
    trees: &TypeTrees,
) -> Parameter {
    let case = platform_type_table(platform)
        .into_iter()
        .find(|case| case.tag == tag)
        .unwrap();
    parameter_with_request(index, tag, case.request, &case.limit, address, trees)
}
/// A parameter of one tag whose value comes from `request` and whose limit is `limit`.
fn parameter_with_request(
    index: usize,
    tag: &str,
    request: &str,
    limit: &Limit,
    address: Option<&str>,
    trees: &TypeTrees,
) -> Parameter {
    let root = type_id(&type_row_id(tag));
    let tree = trees.tree(root).unwrap();
    let value = ParameterValue::parse(request, tree).unwrap().to_stored();
    parameter_with_stored_value(index, tag, value, limit, address)
}

/// A parameter of one tag whose stored value is `value` and whose limit is `limit`.
fn parameter_with_stored_value(
    index: usize,
    tag: &str,
    value: StoredValue,
    limit: &Limit,
    address: Option<&str>,
) -> Parameter {
    let root = type_id(&type_row_id(tag));
    Parameter::from(StoredParameter {
        id: ParameterId::from(Uuid::from_u128(u128::from(number(index)))),
        key: ParameterKey::parse(&format!("00000000-0000-4000-8000-2{index:011x}")).unwrap(),
        unit: unit_id(),
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: Text::parse(tag).unwrap(),
        description_en: LongText::parse("Not in the document").unwrap(),
        translations: Vec::new(),
        plc_type: root,
        s7_address: address.map(|text| S7Address::parse(text).unwrap()),
        limits: limit_of(limit).into_iter().collect(),
        value,
        ordinal: Ordinal::parse(number(index)).unwrap(),
    })
}

/// The instant 2026-10-12 09:30:00 UTC.
fn exported_at() -> Timestamp {
    let date = Date::from_calendar_date(2026, Month::October, 12).unwrap();
    Timestamp::from(date.with_hms(9, 30, 0).unwrap().assume_utc())
}

/// The type trees and the parameters of a version with one parameter for each tag. `addresses`
/// gives the address of a tag. A tag without an entry has no address.
fn version_parameters(
    platform: PlcPlatform,
    tags: &[&str],
    addresses: &[(&str, &str)],
) -> (TypeTrees, Vec<Parameter>) {
    let trees = trees(platform, tags);
    let parameters: Vec<Parameter> = tags
        .iter()
        .enumerate()
        .map(|(index, tag)| {
            let address = addresses
                .iter()
                .find(|(address_tag, _)| address_tag == tag)
                .map(|(_, address)| *address);
            parameter(platform, index, tag, address, &trees)
        })
        .collect();
    (trees, parameters)
}

/// The approved version 1 of the recipe `R-200` on the machine `M12`, with one parameter for each
/// tag. `addresses` gives the address of a tag. A tag without an entry has no address.
fn document_text(platform: PlcPlatform, tags: &[&str], addresses: &[(&str, &str)]) -> String {
    let (trees, parameters) = version_parameters(platform, tags, addresses);
    render_document(platform, &trees, &parameters)
}
/// The approved version 1 of the recipe `R-200` for the given trees and parameters.
fn render_document(platform: PlcPlatform, trees: &TypeTrees, parameters: &[Parameter]) -> String {
    let header = ExportHeader {
        machine: MachineCode::parse("M12").unwrap(),
        platform,
        reference: ReferenceNumber::parse("R-200").unwrap(),
        version: 1,
    };
    let body = HashedBody::build(&header, parameters, &units(platform), trees).unwrap();
    let names: BTreeMap<ParameterKey, Text> = parameters
        .iter()
        .map(|parameter| (parameter.key(), parameter.name_en().clone()))
        .collect();
    // `ExportVersion` sets the flag from the read check of every parameter (ADR-0119 (a) to (c)).
    let has_rule_fault = !check_version_parameters(parameters, trees)
        .unwrap()
        .is_empty();
    let facts = DocumentFacts {
        exported_at: exported_at(),
        exported_by: Login::parse("jkowalski").unwrap(),
        status: ExportStatus::Approved,
        approved_at: exported_at(),
        approved_by: Login::parse("anowak").unwrap(),
        machine_name: Text::parse("Filler 2").unwrap(),
        recipe_name: Text::parse("Types of the platform").unwrap(),
        content_sha256: body.sha256().unwrap(),
        names,
        has_rule_fault,
    };
    String::from_utf8(write_document(&body, &facts).unwrap()).unwrap()
}

/// The same document as a JSON value.
fn document(platform: PlcPlatform, tags: &[&str], addresses: &[(&str, &str)]) -> Value {
    serde_json::from_str(&document_text(platform, tags, addresses)).unwrap()
}

/// One document under test: the platform, its primitives, its addresses, and its `types` entries.
struct PlatformCase {
    platform: PlcPlatform,
    /// The golden file of the platform in `tests/golden/`.
    golden_file: &'static str,
    tags: &'static [&'static str],
    /// Only `S7_300` parameters have an address (A31).
    addresses: &'static [(&'static str, &'static str)],
    types: Value,
}

/// The `types` entry of a string row (design D4).
fn string_entry(id: &str, name: &str) -> Value {
    json!({
        "id": id,
        "name": name,
        "revision": 1,
        "kind": "Primitive",
        "string_length": 16
    })
}

fn platform_cases() -> [PlatformCase; 5] {
    [
        PlatformCase {
            platform: PlcPlatform::S7_300,
            golden_file: "types_S7_300.json",
            tags: &S7_300_TAGS,
            addresses: &S7_300_ADDRESSES,
            types: json!([string_entry(STRING_16, "STRING")]),
        },
        PlatformCase {
            platform: PlcPlatform::S7_1200,
            golden_file: "types_S7_1200.json",
            tags: &S7_1200_TAGS,
            addresses: &[],
            types: json!([
                string_entry(STRING_16, "STRING"),
                string_entry(WSTRING_16, "WSTRING")
            ]),
        },
        PlatformCase {
            platform: PlcPlatform::S7_1500,
            golden_file: "types_S7_1500.json",
            tags: &S7_1500_TAGS,
            addresses: &[],
            types: json!([
                string_entry(STRING_16, "STRING"),
                string_entry(WSTRING_16, "WSTRING")
            ]),
        },
        PlatformCase {
            platform: PlcPlatform::CodesysIec,
            golden_file: "types_CodesysIec.json",
            tags: &CODESYS_TAGS,
            addresses: &[],
            types: json!([
                string_entry(STRING_16, "STRING"),
                string_entry(WSTRING_16, "WSTRING")
            ]),
        },
        PlatformCase {
            platform: PlcPlatform::TwinCat3,
            golden_file: "types_TwinCat3.json",
            tags: &CODESYS_TAGS,
            addresses: &[],
            types: json!([
                string_entry(STRING_16, "STRING"),
                string_entry(WSTRING_16, "WSTRING")
            ]),
        },
    ]
}

fn platform_document(case: &PlatformCase) -> Value {
    document(case.platform, case.tags, case.addresses)
}

/// The limits that the document must hold for one table row.
fn expected_limits(limit: &Limit) -> Value {
    match limit {
        Limit::None => json!([]),
        Limit::Range(min, max) => {
            json!([{ "leaf_path": "", "min": min, "max": max, "max_length": null }])
        }
        Limit::Length(length) => {
            json!([{ "leaf_path": "", "min": null, "max": null, "max_length": length.to_string() }])
        }
    }
}

/// The parameters of a document by tag.
fn by_tag(doc: &Value) -> BTreeMap<String, &Value> {
    doc["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|parameter| (parameter["tag"].as_str().unwrap().to_owned(), parameter))
        .collect()
}

fn assert_values_and_limits(doc: &Value, platform: PlcPlatform, tags: &[&str]) {
    let parameters = by_tag(doc);
    for case in platform_type_table(platform)
        .iter()
        .filter(|case| tags.contains(&case.tag))
    {
        assert!(
            parameters.contains_key(case.tag),
            "{}: no parameter in the document",
            case.tag
        );
        let parameter = parameters[case.tag];
        let expected_value: Value = serde_json::from_str(case.value).unwrap();
        assert_eq!(parameter["value"], expected_value, "{}: value", case.tag);
        assert_eq!(
            parameter["limits"],
            expected_limits(&case.limit),
            "{}: limits",
            case.tag
        );
    }
}

#[test]
fn each_s7_type_document_holds_the_expected_value_and_limit_of_each_type() {
    for case in platform_cases() {
        let doc = platform_document(&case);
        assert_values_and_limits(&doc, case.platform, case.tags);
    }
}

fn assert_one_parameter_for_each_tag(doc: &Value, tags: &[&str]) {
    let parameters = doc["parameters"].as_array().unwrap();
    assert_eq!(parameters.len(), tags.len());
    let found: BTreeMap<String, &Value> = by_tag(doc);
    assert_eq!(found.len(), tags.len(), "a tag appears twice");
    for tag in tags {
        assert!(
            found.contains_key(*tag),
            "{tag}: no parameter in the document"
        );
        let parameter = found[*tag];
        // A string parameter has the id of its row. A fixed-size primitive has its name.
        let expected = match *tag {
            "STRING" => STRING_16,
            "WSTRING" => WSTRING_16,
            _ => tag,
        };
        assert_eq!(parameter["type_id"], *expected, "{tag}: type_id");
    }
}

#[test]
fn each_s7_type_document_has_one_parameter_for_each_primitive_of_its_platform() {
    for case in platform_cases() {
        let platform = case.platform;
        let doc = platform_document(&case);
        assert_one_parameter_for_each_tag(&doc, case.tags);
        assert_eq!(doc["types"], case.types, "{platform:?}: types");
        let parameters = by_tag(&doc);
        for tag in case.tags {
            let expected = case
                .addresses
                .iter()
                .find(|(address_tag, _)| address_tag == tag)
                .map_or(Value::Null, |(_, address)| json!(address));
            assert_eq!(
                parameters[*tag]["s7_address"], expected,
                "{platform:?} {tag}: s7_address"
            );
        }
    }
}

/// Read a golden file and drop each `\r`, so a checkout with CRLF line ends gives the same text.
fn golden(file: &str) -> String {
    let path = format!("{GOLDEN_DIR}{file}");
    std::fs::read_to_string(path).unwrap().replace('\r', "")
}

#[test]
fn writer_output_of_each_s7_type_document_equals_its_golden_file() {
    for case in platform_cases() {
        let written = document_text(case.platform, case.tags, case.addresses);
        assert_eq!(written, golden(case.golden_file), "{}", case.golden_file);
    }
}

/// No value, limit, or address of the type table breaks a current rule (ADR-0114 to ADR-0117), so
/// each document is `loadable` (ADR-0119 (c)).
#[test]
fn each_s7_type_document_has_no_rule_fault_and_is_loadable() {
    for case in platform_cases() {
        let platform = case.platform;
        let (trees, parameters) = version_parameters(platform, case.tags, case.addresses);
        let faults = check_version_parameters(&parameters, &trees).unwrap();
        assert!(faults.is_empty(), "{platform:?}: rule faults {faults:?}");
        let doc = platform_document(&case);
        assert_eq!(doc["loadable"], json!(true), "{platform:?}: loadable");
    }
}

/// Compile the schema as draft 2020-12 with the format checks on.
fn validator() -> Validator {
    let text = std::fs::read_to_string(SCHEMA_PATH).unwrap();
    let schema: Value = serde_json::from_str(&text).unwrap();
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .unwrap()
}

#[test]
fn each_s7_type_golden_file_passes_the_v1_schema() {
    let validator = validator();
    for case in platform_cases() {
        let doc: Value = serde_json::from_str(&golden(case.golden_file)).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&doc)
            .map(|error| format!("{} at {}", error, error.instance_path().as_str()))
            .collect();
        assert!(errors.is_empty(), "{}: {errors:?}", case.golden_file);
    }
}

/// The sort key of a `unit_path`: the segments as integers, the first one without its `U`.
fn unit_path_key(path: &str) -> Vec<u64> {
    path.trim_start_matches('U')
        .split('.')
        .map(|segment| segment.parse().unwrap())
        .collect()
}

/// Return the text of a string member of a JSON object.
fn text_member<'a>(object: &'a Value, key: &str) -> &'a str {
    object[key].as_str().unwrap()
}

/// Rebuild the hashed body from a parsed v1 document by the rules of `api.md` sections 6.1, 6.3.
///
/// The body holds the fields that the table marks "yes". The function sorts `types`,
/// `parameters`, and each `limits` array by the keys of section 6.3. The member order inside
/// a type stays as it is. The function drops `name_en` from each parameter.
fn consumer_hashed_body(doc: &Value) -> Value {
    let mut types = doc["types"].as_array().unwrap().clone();
    types.sort_by(|a, b| text_member(a, "id").cmp(text_member(b, "id")));
    let mut parameters = doc["parameters"].as_array().unwrap().clone();
    parameters.sort_by_key(|parameter| {
        (
            unit_path_key(text_member(parameter, "unit_path")),
            text_member(parameter, "tag").to_owned(),
        )
    });
    for parameter in &mut parameters {
        let object = parameter.as_object_mut().unwrap();
        object.remove("name_en");
        let limits = object.get_mut("limits").unwrap().as_array_mut().unwrap();
        limits.sort_by(|a, b| text_member(a, "leaf_path").cmp(text_member(b, "leaf_path")));
    }
    json!({
        "machine": {
            "code": doc["machine"]["code"],
            "plc_platform": doc["machine"]["plc_platform"],
        },
        "recipe": { "reference_number": doc["recipe"]["reference_number"] },
        "version": { "number": doc["version"]["number"] },
        "types": types,
        "parameters": parameters,
    })
}

/// Return the lower-case hex SHA-256 of the RFC 8785 (JCS) canonical form of a JSON value.
///
/// A transfer client has only the document and a JCS library. The test therefore does not use
/// the hash function of `recipe-domain`.
fn jcs_sha256_hex(value: &Value) -> String {
    use sha2::{Digest, Sha256};
    let text = serde_json_canonicalizer::to_string(value).unwrap();
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn consumer_rebuilds_content_sha256_of_each_s7_type_golden_file() {
    for case in platform_cases() {
        let doc: Value = serde_json::from_str(&golden(case.golden_file)).unwrap();
        let expected = doc["content_sha256"].as_str().unwrap();
        assert_eq!(
            jcs_sha256_hex(&consumer_hashed_body(&doc)),
            expected,
            "{}",
            case.golden_file
        );
    }
}

#[test]
fn consumer_rebuilds_content_sha256_when_the_arrays_arrive_in_reverse_order() {
    for case in platform_cases() {
        let mut doc: Value = serde_json::from_str(&golden(case.golden_file)).unwrap();
        let expected = text_member(&doc, "content_sha256").to_owned();
        doc["types"].as_array_mut().unwrap().reverse();
        doc["parameters"].as_array_mut().unwrap().reverse();
        for parameter in doc["parameters"].as_array_mut().unwrap() {
            parameter["limits"].as_array_mut().unwrap().reverse();
        }
        assert_eq!(
            jcs_sha256_hex(&consumer_hashed_body(&doc)),
            expected,
            "{}",
            case.golden_file
        );
    }
}

/// The `WSTRING` value of the non-BMP golden file: U+1D11E needs a surrogate pair in UTF-16.
const NON_BMP_VALUE: &str = "Z\u{1d11e}";
/// The golden file of the `TwinCat3` document with a character outside the BMP.
const NON_BMP_GOLDEN_FILE: &str = "types_TwinCat3_wstring_non_bmp.json";

/// The document of one `TwinCat3` `WSTRING` parameter that holds U+1D11E.
///
/// `ParameterValue::parse` refuses the value (ADR-0201 (a)). The stored value of an approved
/// version still holds it, so the test reads it under `RuleSet::Earlier` (ADR-0201 (i)).
fn non_bmp_document_text() -> String {
    let platform = PlcPlatform::TwinCat3;
    let trees = trees(platform, &["WSTRING"]);
    let tree = trees.tree(type_id(&type_row_id("WSTRING"))).unwrap();
    let stored = StoredValue::from(json!(NON_BMP_VALUE));
    let value = ParameterValue::from_stored(&stored, tree, RuleSet::Earlier)
        .unwrap()
        .to_stored();
    let limit = Limit::Length(STRING_LENGTH);
    let parameters = [parameter_with_stored_value(
        0, "WSTRING", value, &limit, None,
    )];
    render_document(platform, &trees, &parameters)
}

/// A stored `WSTRING` value outside the BMP on `TwinCat3` makes `loadable` `false` (ADR-0201 (i)).
/// The hashed body and `content_sha256` do not change (ADR-0202 (j2)). Only `loadable` differs
/// from the golden file of before ADR-0201. The file also pins the escape of the JSON writer.
#[test]
fn adr_0201_writer_output_of_a_stored_twincat3_wstring_with_a_non_bmp_character_equals_its_golden_file()
 {
    let written = non_bmp_document_text();
    assert_eq!(
        written,
        golden(NON_BMP_GOLDEN_FILE),
        "{NON_BMP_GOLDEN_FILE}"
    );
    let doc: Value = serde_json::from_str(&written).unwrap();
    assert_eq!(doc["parameters"][0]["value"], json!(NON_BMP_VALUE));
    assert_eq!(doc["loadable"], json!(false));
    assert_eq!(
        doc["content_sha256"],
        json!(NON_BMP_CONTENT_SHA256),
        "content_sha256 stays as before ADR-0201"
    );
}

/// The `content_sha256` of the non-BMP golden file before ADR-0201. Written by hand from the
/// committed file of task 1.7.
const NON_BMP_CONTENT_SHA256: &str =
    "8fb8791f92e30030784f52d26428fa571336f88949ad1b8190298a44404b2018";

#[test]
fn consumer_rebuilds_content_sha256_of_the_non_bmp_golden_file() {
    let doc: Value = serde_json::from_str(&golden(NON_BMP_GOLDEN_FILE)).unwrap();
    let expected = doc["content_sha256"].as_str().unwrap();
    assert_eq!(jcs_sha256_hex(&consumer_hashed_body(&doc)), expected);
}

/// The platforms of the CODESYS and TwinCAT 3 golden files with their file names.
const CODESYS_GOLDEN_FILES: [(PlcPlatform, &str, &str); 2] = [
    (
        PlcPlatform::CodesysIec,
        "types_CodesysIec.json",
        "CodesysIec",
    ),
    (PlcPlatform::TwinCat3, "types_TwinCat3.json", "TwinCat3"),
];

/// The ends and the `LREAL` value that the golden files of `CodesysIec` and `TwinCat3` must hold.
/// The texts are written by hand from `domain-model.md` section 2.1 and ADR-0117. They do not come
/// from the type table.
const CODESYS_EXPECTED_VALUES: [(&str, &str); 14] = [
    ("LWORD", "18446744073709551615"),
    ("LINT", "-9223372036854775808"),
    ("ULINT", "18446744073709551615"),
    ("LREAL", "123456789.012345"),
    ("TIME", "4294967295"),
    ("LTIME", "18446744073709551615"),
    ("DATE", "49710"),
    ("LDATE", "9223286400000000000"),
    ("TIME_OF_DAY", "45000500"),
    ("LTOD", "86399999999999"),
    ("DATE_AND_TIME", "4294967295"),
    ("LDT", "9223372036854775807"),
    ("DWORD", "4294967295"),
    ("UDINT", "4294967295"),
];

/// The tags that the matrix gives to S7 only (`data-model.md` section 7).
const NOT_ON_CODESYS: [&str; 4] = ["CHAR", "WCHAR", "DTL", "S5TIME"];

/// Each golden file of `CodesysIec` and `TwinCat3` holds the 25 primitives of the matrix, the
/// 64-bit ends, and an `LREAL` number of 15 digits (ADR-0199, review item R09). The platform is the
/// only difference between the two files, so the content hashes differ.
#[test]
fn adr_0199_golden_types_of_codesys_and_twincat() {
    let mut hashes = Vec::new();
    for (platform, file, name) in CODESYS_GOLDEN_FILES {
        let doc: Value = serde_json::from_str(&golden(file)).unwrap();
        assert_eq!(doc["machine"]["plc_platform"], json!(name), "{file}");
        assert_eq!(doc["loadable"], json!(true), "{file}: loadable");
        let parameters = by_tag(&doc);
        let found: BTreeSet<&str> = parameters.keys().map(String::as_str).collect();
        let expected: BTreeSet<&str> = CODESYS_TAGS.iter().copied().collect();
        assert_eq!(found, expected, "{file}: tags");
        for tag in NOT_ON_CODESYS {
            assert!(!parameters.contains_key(tag), "{file}: {tag} is not here");
        }
        for (tag, text) in CODESYS_EXPECTED_VALUES {
            assert_eq!(parameters[tag]["value"], json!(text), "{file}: {tag}");
        }
        for parameter in parameters.values() {
            assert_eq!(parameter["s7_address"], Value::Null, "{file}: no address");
        }
        assert_eq!(
            document_text(platform, &CODESYS_TAGS, &[]),
            golden(file),
            "{file}: writer output"
        );
        hashes.push(doc["content_sha256"].as_str().unwrap().to_owned());
    }
    assert_ne!(hashes[0], hashes[1], "the platform is part of the hash");
}

/// The SHA-256 of the file `types_TwinCat3.json` of task 1.7, without `\r`. Computed from the
/// committed file with `sha256sum`.
const TWINCAT3_GOLDEN_FILE_SHA256: &str =
    "6a4b5e05264401513da92a6af2135d994f931d3ba5f826107fa6141d15a68113";

/// ADR-0201 changes only the file with a character outside the BMP (ADR-0202 (j2)). The file
/// `types_TwinCat3.json` keeps its exact bytes, its `content_sha256`, and `loadable` `true`.
#[test]
fn adr_0201_unaffected_golden_is_byte_identical() {
    use sha2::{Digest, Sha256};
    let text = golden("types_TwinCat3.json");
    let file_hash: String = Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(file_hash, TWINCAT3_GOLDEN_FILE_SHA256, "file bytes");
    let written = document_text(PlcPlatform::TwinCat3, &CODESYS_TAGS, &[]);
    assert_eq!(written, text, "writer output");
    let doc: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(doc["loadable"], json!(true));
    assert_eq!(
        doc["content_sha256"],
        json!("5e7ec68c2aca5122b15c115b1c76f9e159af1fb35196dac1a97212962a3fb144")
    );
}
