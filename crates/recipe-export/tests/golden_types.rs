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
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, S7Address, StoredMachine, StoredUnit,
    TypeTrees, UnitId, UnitPath,
};
use recipe_domain::recipe::{
    Ordinal, Parameter, ParameterId, ParameterKey, ParameterTag, ParameterValue, ReferenceNumber,
    StoredLimit, StoredParameter,
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

/// The limit of one type in the document.
enum Limit {
    /// The type has no limit (A36).
    None,
    /// The lower and the upper bound as canonical decimal text.
    Range(&'static str, &'static str),
    /// The maximum length of a string.
    Length(u16),
}

/// One row of the type table.
struct TypeCase {
    tag: &'static str,
    /// The value as JSON text, as the request sends it.
    request: &'static str,
    /// The value in the document as JSON text.
    value: &'static str,
    limit: Limit,
}

const fn case(
    tag: &'static str,
    request: &'static str,
    value: &'static str,
    limit: Limit,
) -> TypeCase {
    TypeCase {
        tag,
        request,
        value,
        limit,
    }
}

/// The 28 primitives of the S7 matrix without `LDATE`. The time counts follow the base units of
/// `domain-model.md` section 2.1. They were computed by hand from 1970-01-01 = day 0 and
/// 2026-10-12 = day 20738.
fn type_table() -> [TypeCase; 28] {
    use Limit::{Length, None, Range};
    [
        case("BOOL", "true", "true", None),
        case("BYTE", "255", r#""255""#, Range("0", "255")),
        case("WORD", "65535", r#""65535""#, Range("0", "65535")),
        case(
            "DWORD",
            "4294967295",
            r#""4294967295""#,
            Range("0", "4294967295"),
        ),
        case(
            "LWORD",
            "18446744073709551615",
            r#""18446744073709551615""#,
            Range("0", "18446744073709551615"),
        ),
        case("SINT", "-128", r#""-128""#, Range("-128", "127")),
        case("USINT", "255", r#""255""#, Range("0", "255")),
        case("INT", "-32768", r#""-32768""#, Range("-32768", "32767")),
        case("UINT", "65535", r#""65535""#, Range("0", "65535")),
        case(
            "DINT",
            "-2147483648",
            r#""-2147483648""#,
            Range("-2147483648", "2147483647"),
        ),
        case(
            "UDINT",
            "4294967295",
            r#""4294967295""#,
            Range("0", "4294967295"),
        ),
        case(
            "LINT",
            "-9223372036854775808",
            r#""-9223372036854775808""#,
            Range("-9223372036854775808", "9223372036854775807"),
        ),
        case(
            "ULINT",
            "18446744073709551615",
            r#""18446744073709551615""#,
            Range("0", "18446744073709551615"),
        ),
        // 3 significant digits.
        case("REAL", "1.5e2", r#""150""#, Range("0", "200")),
        case("LREAL", "-0.1250", r#""-0.125""#, Range("-1", "1")),
        // The `CHAR` value is U+00E9, a character of two UTF-8 bytes. The test pins review Q06.
        case("CHAR", "\"\u{e9}\"", "\"\u{e9}\"", None),
        case("WCHAR", "\"\u{3a9}\"", "\"\u{3a9}\"", None),
        case(
            "STRING",
            "\"Zone \u{e9}\"",
            "\"Zone \u{e9}\"",
            Length(STRING_LENGTH),
        ),
        // The `WSTRING` value holds U+1D11E, a character outside the BMP. The test pins review Q07.
        case(
            "WSTRING",
            "\"Z\u{1d11e}\"",
            "\"Z\u{1d11e}\"",
            Length(STRING_LENGTH),
        ),
        case("S5TIME", r#""S5T#1s""#, r#""1000""#, Range("0", "9990000")),
        // S7 `TIME` is signed.
        case(
            "TIME",
            r#""T#-5s""#,
            r#""-5000""#,
            Range("-2147483648", "2147483647"),
        ),
        case(
            "LTIME",
            r#""LT#1m30s""#,
            r#""90000000000""#,
            Range("-9223372036854775808", "9223372036854775807"),
        ),
        // The limit is 2026-01-01 to 2026-12-31.
        case(
            "DATE",
            r#""D#2026-10-12""#,
            r#""20738""#,
            Range("20454", "20818"),
        ),
        case(
            "TIME_OF_DAY",
            r#""TOD#12:30:00.5""#,
            r#""45000500""#,
            Range("0", "86399999"),
        ),
        case(
            "LTOD",
            r#""LTOD#12:30:00.000000001""#,
            r#""45000000000001""#,
            Range("0", "86399999999999"),
        ),
        // The limit is 2026-01-01 to 2026-12-31-23:59:59.
        case(
            "DATE_AND_TIME",
            r#""DT#2026-10-12-12:30:00""#,
            r#""1791808200""#,
            Range("1767225600", "1798761599"),
        ),
        case(
            "LDT",
            r#""LDT#2026-10-12-12:30:00.000000001""#,
            r#""1791808200000000001""#,
            Range("0", "9223372036854775807"),
        ),
        case(
            "DTL",
            r#""DTL#2026-10-12-12:30:00.5""#,
            r#""1791808200500000000""#,
            Range("0", "9223372036854775807"),
        ),
    ]
}

/// The 14 primitives of the `S7_300` platform (`data-model.md` section 7), written by hand.
const S7_300_TAGS: [&str; 14] = [
    "BOOL",
    "BYTE",
    "WORD",
    "DWORD",
    "INT",
    "DINT",
    "REAL",
    "CHAR",
    "STRING",
    "S5TIME",
    "TIME",
    "DATE",
    "TIME_OF_DAY",
    "DATE_AND_TIME",
];

/// The 20 primitives of the `S7_1200` platform (`data-model.md` section 7), written by hand.
const S7_1200_TAGS: [&str; 20] = [
    "BOOL",
    "BYTE",
    "WORD",
    "DWORD",
    "SINT",
    "USINT",
    "INT",
    "UINT",
    "DINT",
    "UDINT",
    "REAL",
    "LREAL",
    "CHAR",
    "WCHAR",
    "STRING",
    "WSTRING",
    "TIME",
    "DATE",
    "TIME_OF_DAY",
    "DTL",
];

/// The 28 primitives of the `S7_1500` platform without `LDATE` (`data-model.md` section 7),
/// written by hand.
const S7_1500_TAGS: [&str; 28] = [
    "BOOL",
    "BYTE",
    "WORD",
    "DWORD",
    "LWORD",
    "SINT",
    "USINT",
    "INT",
    "UINT",
    "DINT",
    "UDINT",
    "LINT",
    "ULINT",
    "REAL",
    "LREAL",
    "CHAR",
    "WCHAR",
    "STRING",
    "WSTRING",
    "S5TIME",
    "TIME",
    "LTIME",
    "DATE",
    "TIME_OF_DAY",
    "LTOD",
    "DATE_AND_TIME",
    "LDT",
    "DTL",
];

/// The addresses of the `S7_300` parameters in `DB10`. Each address is aligned, no two overlap,
/// and the last byte is 57.
const S7_300_ADDRESSES: [(&str, &str); 14] = [
    ("BOOL", "DB10.DBX0.0"),
    ("BYTE", "DB10.DBB1"),
    ("WORD", "DB10.DBW2"),
    ("DWORD", "DB10.DBD4"),
    ("INT", "DB10.DBW8"),
    ("DINT", "DB10.DBD10"),
    ("REAL", "DB10.DBD14"),
    ("CHAR", "DB10.DBB18"),
    ("STRING", "DB10.DBB20"),
    ("S5TIME", "DB10.DBW38"),
    ("TIME", "DB10.DBD40"),
    ("DATE", "DB10.DBW44"),
    ("TIME_OF_DAY", "DB10.DBD46"),
    ("DATE_AND_TIME", "DB10.DBB50"),
];

fn type_id(text: &str) -> PlcTypeId {
    PlcTypeId::from(Uuid::parse_str(text).unwrap())
}

fn machine_id() -> MachineId {
    MachineId::from(Uuid::from_u128(0xAA))
}

fn unit_id() -> UnitId {
    UnitId::from(Uuid::from_u128(1))
}

/// The row of a tag. A fixed-size primitive gets an id that no string row uses.
fn type_row_id(tag: &str) -> String {
    let index = type_table()
        .iter()
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

fn parameter(index: usize, tag: &str, address: Option<&str>, trees: &TypeTrees) -> Parameter {
    let case = type_table()
        .into_iter()
        .find(|case| case.tag == tag)
        .unwrap();
    let root = type_id(&type_row_id(tag));
    let tree = trees.tree(root).unwrap();
    let value = ParameterValue::parse(case.request, tree)
        .unwrap()
        .to_stored();
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
        limits: limit_of(&case.limit).into_iter().collect(),
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
            parameter(index, tag, address, &trees)
        })
        .collect();
    (trees, parameters)
}

/// The approved version 1 of the recipe `R-200` on the machine `M12`, with one parameter for each
/// tag. `addresses` gives the address of a tag. A tag without an entry has no address.
fn document_text(platform: PlcPlatform, tags: &[&str], addresses: &[(&str, &str)]) -> String {
    let (trees, parameters) = version_parameters(platform, tags, addresses);
    let header = ExportHeader {
        machine: MachineCode::parse("M12").unwrap(),
        platform,
        reference: ReferenceNumber::parse("R-200").unwrap(),
        version: 1,
    };
    let body = HashedBody::build(&header, &parameters, &units(platform), &trees).unwrap();
    let names: BTreeMap<ParameterKey, Text> = parameters
        .iter()
        .map(|parameter| (parameter.key(), parameter.name_en().clone()))
        .collect();
    // `ExportVersion` sets the flag from the read check of every parameter (ADR-0119 (a) to (c)).
    let has_rule_fault = !check_version_parameters(&parameters, &trees)
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

fn platform_cases() -> [PlatformCase; 3] {
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

fn assert_values_and_limits(doc: &Value, tags: &[&str]) {
    let parameters = by_tag(doc);
    for case in type_table().iter().filter(|case| tags.contains(&case.tag)) {
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
        assert_values_and_limits(&doc, case.tags);
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
