// reliability.md section 2.3: test code may use `unwrap`, `expect`, and indexing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

//! Golden tests of the export document v1 (design D11 row T5, design D14).
//!
//! The tests build two hashed bodies through the public constructors of `recipe-domain`. The
//! writer output must equal the golden file byte for byte. The golden files are the test vector
//! of the how-to guide `docs/user/check-export-document.md`.

use std::collections::{BTreeMap, BTreeSet};

use jsonschema::Validator;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::{DocumentFacts, ExportError, ExportHeader, ExportStatus, HashedBody};
use recipe_domain::identity::Login;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, LeafPath, LiveUnits, Machine, MachineCode, MachineId,
    MemberName, PlatformTypes, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive,
    RecordShape, S7Address, StoredMachine, StoredUnit, TypeMember, TypeName, TypeTrees, UnitId,
    UnitPath,
};
use recipe_domain::recipe::{
    Ordinal, Parameter, ParameterId, ParameterKey, ParameterTag, ReferenceNumber, StoredLimit,
    StoredParameter, StoredValue,
};
use recipe_domain::text::{LongText, Text};
use recipe_domain::timestamp::Timestamp;
use recipe_export::write_document;
use rust_decimal::Decimal;
use serde_json::Value;
use time::{Date, Month, OffsetDateTime, UtcOffset};
use uuid::Uuid;

/// Directory of the golden files.
const GOLDEN_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden/");
/// Path of the schema file.
const SCHEMA_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/schema/export-v1.schema.json");
/// Path of `api.md`. Section 6.2 holds the example document.
const API_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/architecture/api.md"
);
/// The golden file of the example of `api.md` section 6.2.
const APPROVED_FILE: &str = "recipe_R-100_v3.json";
/// The golden file of the retired version 2 with an array parameter.
const RETIRED_FILE: &str = "RETIRED_recipe_R-100_v2.json";
/// The pinned hash of the example (`recipe-domain/src/export/hash/tests.rs`).
const EXAMPLE_HASH: &str = "885d9a7d767af484f1c1824b5d493b9ddc19604db05eff1fa4fbded25bad7497";

const PLATFORM: PlcPlatform = PlcPlatform::S7_300;
const INT: &str = "00000000-0000-4000-8000-000000000001";
const REAL: &str = "00000000-0000-4000-8000-000000000002";
const STRING_16: &str = "5d2c7b1e-3f4a-4b6c-8d9e-1a2b3c4d5e6f";
const ZONE_TEMP: &str = "7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f";
const INT_ARRAY: &str = "9e8d7c6b-5a4f-4e3d-8c2b-1a0f9e8d7c6b";

fn type_id(text: &str) -> PlcTypeId {
    PlcTypeId::from(Uuid::parse_str(text).unwrap())
}

fn machine_id() -> MachineId {
    MachineId::from(Uuid::from_u128(0xAA))
}

fn row(platform: PlcPlatform, id: &str, owner: Option<MachineId>, shape: RecordShape) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: type_id(id),
        machine: owner,
        platforms: BTreeSet::from([platform]),
        shape,
    })
    .unwrap()
}

fn primitive_row(
    platform: PlcPlatform,
    id: &str,
    primitive: Primitive,
    string_length: Option<u16>,
) -> PlcType {
    let size_bytes = match string_length {
        Some(length) => u32::from(length).checked_add(2).unwrap(),
        None => primitive.fixed_size_bytes().unwrap(),
    };
    let owner = string_length.map(|_| machine_id());
    let shape = RecordShape::Primitive {
        primitive,
        leaf_class: primitive.leaf_class(),
        size_bytes,
        string_length,
    };
    row(platform, id, owner, shape)
}

fn array_row(
    platform: PlcPlatform,
    id: &str,
    element: &str,
    (lower, upper): (i32, i32),
) -> PlcType {
    let dimension = ArrayDimension::parse(lower, upper).unwrap();
    let shape = RecordShape::Array {
        element: type_id(element),
        dimensions: ArrayDimensions::parse(vec![dimension]).unwrap(),
    };
    row(platform, id, Some(machine_id()), shape)
}

fn struct_row(
    platform: PlcPlatform,
    id: &str,
    (name, revision): (&str, u32),
    members: &[(&str, &str)],
) -> PlcType {
    let members = members
        .iter()
        .map(|(member, type_text)| TypeMember {
            name: MemberName::parse(member).unwrap(),
            type_id: type_id(type_text),
        })
        .collect();
    let shape = RecordShape::Struct {
        name: TypeName::parse(name).unwrap(),
        revision,
        supersedes: None,
        members,
    };
    row(platform, id, Some(machine_id()), shape)
}

/// The type trees of the machine: `INT`, `REAL`, `STRING(16)`, `ZoneTemp@2`, and
/// `ARRAY[1..3] OF INT`.
fn trees(platform: PlcPlatform) -> TypeTrees {
    let rows = vec![
        primitive_row(platform, INT, Primitive::Int, None),
        primitive_row(platform, REAL, Primitive::Real, None),
        primitive_row(platform, STRING_16, Primitive::String, Some(16)),
        struct_row(
            platform,
            ZONE_TEMP,
            ("ZoneTemp", 2),
            &[("setpoint", REAL), ("label", STRING_16)],
        ),
        array_row(platform, INT_ARRAY, INT, (1, 3)),
    ];
    let catalog = PlatformTypes::collect(machine_id(), platform, rows).unwrap();
    let ids = [INT, REAL, STRING_16, ZONE_TEMP, INT_ARRAY].map(type_id);
    TypeTrees::resolve(ids, &catalog)
}

fn unit_id(path: &str) -> UnitId {
    let n = if path == "U1" { 1 } else { 2 };
    UnitId::from(Uuid::from_u128(n))
}

/// The machine `M12` "Filler 2" with the units `U1` and `U1.2`.
fn units(platform: PlcPlatform) -> LiveUnits {
    let unit = |path: &str| StoredUnit {
        id: unit_id(path),
        path: UnitPath::parse(path).unwrap(),
        name: Text::parse(path).unwrap(),
        archived_at: None,
    };
    Machine::try_from(StoredMachine {
        id: machine_id(),
        code: MachineCode::parse("M12").unwrap(),
        name: Text::parse("Filler 2").unwrap(),
        platform,
        archived_at: None,
        units: ["U1", "U1.2"].map(unit).to_vec(),
    })
    .unwrap()
    .live_units()
}

fn range(path: &str, min: i64, max: i64) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: Some(Decimal::from(min)),
        max: Some(Decimal::from(max)),
        max_length: None,
    }
}

fn length(path: &str, max_length: u16) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: None,
        max: None,
        max_length: Some(max_length),
    }
}

/// The input row of one parameter. `value` is the stored JSON as text.
struct Spec {
    n: u32,
    key: &'static str,
    unit_tag: (&'static str, &'static str),
    name: &'static str,
    plc_type: &'static str,
    address: Option<&'static str>,
    limits: Vec<StoredLimit>,
    value: &'static str,
}

fn parameter(spec: Spec) -> Parameter {
    let (unit, tag) = spec.unit_tag;
    let json: Value = serde_json::from_str(spec.value).unwrap();
    Parameter::from(StoredParameter {
        id: ParameterId::from(Uuid::from_u128(u128::from(spec.n))),
        key: ParameterKey::parse(spec.key).unwrap(),
        unit: unit_id(unit),
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: Text::parse(spec.name).unwrap(),
        description_en: LongText::parse("Not in the document").unwrap(),
        translations: Vec::new(),
        plc_type: type_id(spec.plc_type),
        s7_address: spec
            .address
            .map(|address| S7Address::parse(address).unwrap()),
        limits: spec.limits,
        value: StoredValue::from(json),
        ordinal: Ordinal::parse(spec.n).unwrap(),
    })
}

fn body(platform: PlcPlatform, version: u32, parameters: &[Parameter]) -> HashedBody {
    let header = ExportHeader {
        machine: MachineCode::parse("M12").unwrap(),
        platform,
        reference: ReferenceNumber::parse("R-100").unwrap(),
        version,
    };
    HashedBody::build(&header, parameters, &units(platform), &trees(platform)).unwrap()
}

fn names_of(parameters: &[Parameter]) -> BTreeMap<ParameterKey, Text> {
    parameters
        .iter()
        .map(|parameter| (parameter.key(), parameter.name_en().clone()))
        .collect()
}

/// An instant in UTC.
fn utc(year: i32, (month, day): (u8, u8), (hour, minute, second): (u8, u8, u8)) -> Timestamp {
    let date = Date::from_calendar_date(year, Month::try_from(month).unwrap(), day).unwrap();
    Timestamp::from(date.with_hms(hour, minute, second).unwrap().assume_utc())
}

/// The facts of a document of `R-100` on the machine "Filler 2".
fn facts(
    body: &HashedBody,
    parameters: &[Parameter],
    status: ExportStatus,
    (exported_at, approved_at): (Timestamp, Timestamp),
) -> DocumentFacts {
    DocumentFacts {
        exported_at,
        exported_by: Login::parse("jkowalski").unwrap(),
        status,
        approved_at,
        approved_by: Login::parse("anowak").unwrap(),
        machine_name: Text::parse("Filler 2").unwrap(),
        recipe_name: Text::parse("Bottle 0.5 l").unwrap(),
        content_sha256: body.sha256().unwrap(),
        names: names_of(parameters),
    }
}

/// The parameters of the example of `api.md` section 6.2. The limits come unsorted.
fn example_parameters() -> Vec<Parameter> {
    let speed = Spec {
        n: 1,
        key: "1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718",
        unit_tag: ("U1", "SPEED"),
        name: "Line speed",
        plc_type: INT,
        address: Some("DB1.DBW0"),
        limits: vec![range("", 10, 250)],
        value: "120",
    };
    let zone = Spec {
        n: 2,
        key: "9a8b7c6d-5e4f-4321-b0a9-8f7e6d5c4b3a",
        unit_tag: ("U1.2", "ZONE1"),
        name: "Zone 1",
        plc_type: ZONE_TEMP,
        address: Some("DB1.DBB2"),
        limits: vec![range(".setpoint", 20, 95), length(".label", 16)],
        value: r#"{"setpoint": 72.5, "label": "Z1"}"#,
    };
    vec![parameter(zone), parameter(speed)]
}

/// The body and the facts of the example of `api.md` section 6.2.
fn example() -> (HashedBody, DocumentFacts) {
    let parameters = example_parameters();
    let body = body(PLATFORM, 3, &parameters);
    let times = (
        utc(2026, (10, 12), (9, 30, 0)),
        utc(2026, (10, 11), (14, 2, 10)),
    );
    let facts = facts(&body, &parameters, ExportStatus::Approved, times);
    (body, facts)
}

/// The body and the facts of the retired version 2 with an `ARRAY[1..3] OF INT` parameter.
///
/// The export time has the offset `+02:00` and a fraction of a second. The document writes it
/// in UTC with whole seconds.
fn retired() -> (HashedBody, DocumentFacts) {
    let parameters = vec![parameter(Spec {
        n: 3,
        key: "2b4d6f80-1a3c-4e5f-8a9b-0c1d2e3f4a5b",
        unit_tag: ("U1", "FILL_STEPS"),
        name: "Fill steps",
        plc_type: INT_ARRAY,
        address: Some("DB2.DBB0"),
        limits: vec![range("[*]", 0, 100)],
        value: "[10, 20, 30]",
    })];
    let body = body(PLATFORM, 2, &parameters);
    let exported_at: OffsetDateTime = Date::from_calendar_date(2026, Month::October, 20)
        .unwrap()
        .with_hms_milli(9, 15, 42, 750)
        .unwrap()
        .assume_offset(UtcOffset::from_hms(2, 0, 0).unwrap());
    let times = (Timestamp::from(exported_at), utc(2026, (9, 30), (11, 0, 0)));
    let facts = facts(&body, &parameters, ExportStatus::Retired, times);
    (body, facts)
}

/// Read a golden file and drop each `\r`, so a checkout with CRLF line ends gives the same text.
fn golden(file: &str) -> String {
    let path = format!("{GOLDEN_DIR}{file}");
    std::fs::read_to_string(path).unwrap().replace('\r', "")
}

fn written((body, facts): (HashedBody, DocumentFacts)) -> String {
    String::from_utf8(write_document(&body, &facts).unwrap()).unwrap()
}

/// Compile the schema as draft 2020-12 with the format checks on (design D4).
fn validator() -> Validator {
    let text = std::fs::read_to_string(SCHEMA_PATH).unwrap();
    let schema: Value = serde_json::from_str(&text).unwrap();
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .unwrap()
}

/// The JSON block of `api.md` section 6.2.
fn api_example_block() -> String {
    let text = std::fs::read_to_string(API_PATH).unwrap().replace('\r', "");
    let (_, section) = text.split_once("### 6.2 Example").unwrap();
    let (_, block) = section.split_once("```json\n").unwrap();
    let (block, _) = block.split_once("\n```").unwrap();
    block.to_owned()
}

/// Remove the white space outside JSON strings. The text inside a string stays as it is.
fn without_white_space(text: &str) -> String {
    let mut compact = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for c in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            compact.push(c);
        } else if c == '"' {
            in_string = true;
            compact.push(c);
        } else if !c.is_ascii_whitespace() {
            compact.push(c);
        }
    }
    compact
}

/// Replace the value of `content_sha256` in a compact text with a fixed mark.
fn without_hash(compact: &str) -> String {
    let field = "\"content_sha256\":\"";
    let (before, rest) = compact.split_once(field).unwrap();
    let (_, after) = rest.split_once('"').unwrap();
    format!("{before}{field}<hash>\"{after}")
}

#[test]
fn writer_output_of_the_api_example_equals_its_golden_file() {
    assert_eq!(written(example()), golden(APPROVED_FILE));
}

#[test]
fn golden_file_of_the_api_example_holds_the_pinned_hash() {
    let (body, facts) = example();
    assert_eq!(body.sha256().unwrap().to_string(), EXAMPLE_HASH);
    assert_eq!(facts.content_sha256.to_string(), EXAMPLE_HASH);
    let doc: Value = serde_json::from_str(&golden(APPROVED_FILE)).unwrap();
    assert_eq!(doc["content_sha256"], EXAMPLE_HASH);
}

#[test]
fn writer_output_of_a_retired_array_version_equals_its_golden_file() {
    assert_eq!(written(retired()), golden(RETIRED_FILE));
}

/// Assert that a document passes the schema. `label` names the document in a failure message.
fn assert_passes_schema(validator: &Validator, doc: &Value, label: &str) {
    let errors: Vec<String> = validator
        .iter_errors(doc)
        .map(|error| format!("{} at {}", error, error.instance_path().as_str()))
        .collect();
    assert!(errors.is_empty(), "{label}: {errors:?}");
}

#[test]
fn each_golden_file_passes_the_schema() {
    let validator = validator();
    for file in [APPROVED_FILE, RETIRED_FILE] {
        let doc: Value = serde_json::from_str(&golden(file)).unwrap();
        assert_passes_schema(&validator, &doc, file);
    }
}

/// The body and the facts of a `TwinCat3` version with one parameter that has no S7 address.
fn without_s7_address() -> (HashedBody, DocumentFacts) {
    let parameters = vec![parameter(Spec {
        n: 1,
        key: "1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718",
        unit_tag: ("U1", "SPEED"),
        name: "Line speed",
        plc_type: INT,
        address: None,
        limits: vec![range("", 10, 250)],
        value: "120",
    })];
    let body = body(PlcPlatform::TwinCat3, 1, &parameters);
    let times = (
        utc(2026, (10, 12), (9, 30, 0)),
        utc(2026, (10, 11), (14, 2, 10)),
    );
    let facts = facts(&body, &parameters, ExportStatus::Approved, times);
    (body, facts)
}

/// Catches a `skip_serializing_if` on `s7_address` in the writer. The hashed body keeps the key
/// as `null` on `CodesysIec` and `TwinCat3` (`api.md` section 6.1). A writer that drops the key
/// gives a document whose hash does not match the body, and the schema requires the key.
#[test]
fn parameter_without_an_s7_address_writes_null_and_passes_the_schema() {
    let text = written(without_s7_address());
    assert!(text.contains("\"s7_address\": null"), "{text}");
    let doc: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(doc["machine"]["plc_platform"], "TwinCat3");
    assert_eq!(doc["parameters"][0].get("s7_address"), Some(&Value::Null));
    assert_passes_schema(&validator(), &doc, "TwinCat3 without s7_address");
}

#[test]
fn json_block_of_api_6_2_equals_the_first_golden_file_except_the_hash() {
    let example = without_hash(&without_white_space(&api_example_block()));
    let golden = without_hash(&without_white_space(&golden(APPROVED_FILE)));
    assert_eq!(example, golden);
}

#[test]
fn parameter_key_without_a_name_gives_e_exp_004() {
    let (body, mut facts) = example();
    facts.names.retain(|_, name| name.as_str() != "Zone 1");
    let error = write_document(&body, &facts).unwrap_err();
    assert!(matches!(error, ExportError::Serialize(_)), "{error:?}");
    assert_eq!(error.code(), ErrorCode::EXP_004);
}
