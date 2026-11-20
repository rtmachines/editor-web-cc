//! Schema tests of the type and value rules of the document v2 (ADR-0208).
//!
//! The platform columns of `data-model.md` section 7 give the primitive names of each platform.
//! `recipe-domain` gives the leaf class, the string length range, and the member limit. Thus the
//! schema file cannot drift from the domain.

use std::collections::BTreeSet;

use recipe_domain::plant::limits::MAX_MEMBERS;
use recipe_domain::plant::{LeafClass, PlcPlatform, Primitive, StringKind, StringLength};

use super::{SCHEMA_V2_PATH, Value, changed, json, v2_failures};

/// Path of `data-model.md`, the home of the primitive matrix (section 7).
const DATA_MODEL_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/architecture/data-model.md"
);

/// The platforms in the order of the columns of the matrix.
const PLATFORMS: [PlcPlatform; 5] = [
    PlcPlatform::CodesysIec,
    PlcPlatform::TwinCat3,
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
    PlcPlatform::S7_300,
];

/// The `STRING` entry of the test document.
const STRING_ID: &str = "5d2c7b1e-3f4a-4b6c-8d9e-1a2b3c4d5e6f";
/// The `Struct` entry of the test document.
const STRUCT_ID: &str = "7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f";
/// The `Array` entry of the test document.
const ARRAY_ID: &str = "9e8d7c6b-5a4f-4e3d-8c2b-1a0f9e8d7c6b";

/// A position of ADR-0208 (a1) where a document names a type.
#[derive(Clone, Copy, Debug)]
enum Position {
    /// `parameters[].type_id`.
    Parameter,
    /// `types[].members[].type_id`.
    Member,
    /// `types[].element_type_id`.
    Element,
}

impl Position {
    const ALL: [Self; 3] = [Self::Parameter, Self::Member, Self::Element];

    /// The JSON pointer of the position in the test document.
    const fn pointer(self) -> &'static str {
        match self {
            Self::Parameter => "/parameters/0/type_id",
            Self::Member => "/types/1/members/0/type_id",
            Self::Element => "/types/2/element_type_id",
        }
    }
}

/// The primitive rows of `data-model.md` section 7 with the flag of each platform.
fn matrix() -> Vec<(Primitive, [bool; 5])> {
    let text = std::fs::read_to_string(DATA_MODEL_PATH).expect("read data-model.md");
    let rows: Vec<_> = text
        .lines()
        .skip_while(|line| !line.starts_with("## 7. "))
        .skip(1)
        .take_while(|line| !line.starts_with("## "))
        .filter_map(matrix_row)
        .collect();
    assert_eq!(
        rows.len(),
        Primitive::ALL.len(),
        "one row for each primitive"
    );
    rows
}

/// One row of the matrix, or `None` for a line that is not a primitive row.
fn matrix_row(line: &str) -> Option<(Primitive, [bool; 5])> {
    let cells: Vec<&str> = line.split('|').map(str::trim).collect();
    let name = cells.get(1)?.strip_prefix('`')?.strip_suffix('`')?;
    let primitive = Primitive::parse(name).ok()?;
    let mut flags = [false; 5];
    for (flag, cell) in flags.iter_mut().zip(cells.get(6..11)?) {
        *flag = match *cell {
            "yes" => true,
            "no" => false,
            _ => return None,
        };
    }
    Some((primitive, flags))
}

/// Return true when the matrix lists the primitive on the platform.
fn is_on_platform(flags: [bool; 5], platform: PlcPlatform) -> bool {
    let index = PLATFORMS
        .iter()
        .position(|candidate| *candidate == platform)
        .expect("the platform is in the list");
    flags[index]
}

/// Return true for `STRING` and `WSTRING`. A document names them by a UUID reference (a2).
const fn has_length(primitive: Primitive) -> bool {
    matches!(primitive, Primitive::String | Primitive::Wstring)
}

fn v2_schema() -> Value {
    let text = std::fs::read_to_string(SCHEMA_V2_PATH).expect("read the schema file v2");
    serde_json::from_str(&text).expect("parse the schema file v2")
}

/// A valid document v2 on `platform`. Every type reference is a UUID of its `types` list (a3).
fn base(platform: PlcPlatform) -> Value {
    let address = if platform.is_s7() {
        json!("DB1.DBB0")
    } else {
        Value::Null
    };
    json!({
        "schema_version": "2",
        "exported_at": "2026-10-12T09:30:00Z",
        "exported_by": "jkowalski",
        "loadable": true,
        "content_sha256": "885d9a7d767af484f1c1824b5d493b9ddc19604db05eff1fa4fbded25bad7497",
        "scope_sha256": "49a1d056e18c28cc6f84605db5766b1266e2e83a7b5015cf7dfd19f850de3433",
        "machine": { "code": "M12", "name": "Filler 2", "plc_platform": platform.as_str() },
        "recipe": { "reference_number": "R-100", "name": "Bottle 0.5 l" },
        "version": {
            "number": 3, "status": "Approved",
            "approved_at": "2026-10-11T14:02:10Z", "approved_by": "anowak"
        },
        "scope": { "unit_path": "U1" },
        "types": [
            {
                "id": STRING_ID, "name": "STRING", "revision": 1,
                "kind": "Primitive", "string_length": 16
            },
            {
                "id": STRUCT_ID, "name": "Holder", "revision": 1,
                "kind": "Struct", "string_length": null,
                "members": [
                    { "name": "label", "type_id": STRING_ID },
                    { "name": "note", "type_id": STRING_ID }
                ]
            },
            {
                "id": ARRAY_ID, "name": null, "revision": 1,
                "kind": "Array", "string_length": null,
                "element_type_id": STRING_ID,
                "dimensions": [ { "lower": 1, "upper": 3 } ]
            }
        ],
        "parameters": [
            {
                "unit_path": "U1", "tag": "TEXT",
                "key": "1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718",
                "name_en": "Text", "type_id": STRING_ID, "s7_address": address,
                "limits": [], "value": "Z1"
            },
            {
                "unit_path": "U1", "tag": "HOLDER",
                "key": "9a8b7c6d-5e4f-4321-b0a9-8f7e6d5c4b3a",
                "name_en": "Holder", "type_id": STRUCT_ID, "s7_address": address,
                "limits": [], "value": { "label": "A", "note": "B" }
            },
            {
                "unit_path": "U1", "tag": "LIST",
                "key": "2b4c6d8e-0f1a-4b3c-8d5e-7f9a1b3c5d7e",
                "name_en": "List", "type_id": ARRAY_ID, "s7_address": address,
                "limits": [], "value": ["A", "B", "C"]
            }
        ]
    })
}

/// A value of the JSON kind of the class of `primitive`. The text is canonical when it is a string.
fn matching_value(primitive: Primitive) -> Value {
    match primitive.leaf_class() {
        LeafClass::Bool => json!(true),
        LeafClass::Char => json!("A"),
        LeafClass::String => json!("Z1"),
        LeafClass::Numeric | LeafClass::Duration | LeafClass::Instant => json!("1"),
    }
}

/// A value of the wrong form for the class of `primitive`.
fn wrong_value(primitive: Primitive) -> Value {
    match primitive.leaf_class() {
        LeafClass::Bool => json!("true"),
        LeafClass::Char => json!("AB"),
        LeafClass::String => json!(5),
        LeafClass::Numeric | LeafClass::Duration | LeafClass::Instant => json!("01"),
    }
}

/// The base document with the name `name` at `position`. A parameter gets a matching value, so the
/// only change of a failing document is the type position.
fn named_at(platform: PlcPlatform, position: Position, name: &str) -> Value {
    changed(base(platform), |doc| {
        *doc.pointer_mut(position.pointer()).expect("position") = json!(name);
        if let (Position::Parameter, Ok(primitive)) = (position, Primitive::parse(name)) {
            doc["parameters"][0]["value"] = matching_value(primitive);
        }
    })
}

/// The base document with a primitive in the first parameter and one value.
fn parameter_with(primitive: Primitive, value: Value) -> Value {
    changed(base(PlcPlatform::S7_1500), |doc| {
        doc["parameters"][0]["type_id"] = json!(primitive.name());
        doc["parameters"][0]["value"] = value;
    })
}

fn assert_fails_at(doc: &Value, path: &str, keyword: &str) {
    let found = v2_failures(doc);
    let expected = (path.to_owned(), keyword.to_owned());
    assert!(
        found.contains(&expected),
        "expected {expected:?}, found {found:?}"
    );
}

fn assert_passes(doc: &Value) {
    assert_eq!(v2_failures(doc), Vec::new());
}

#[test]
fn adr_0208_schema_file_v2_is_a_valid_draft_2020_12_schema() {
    assert!(jsonschema::draft202012::meta::validate(&v2_schema()).is_ok());
}

#[test]
fn adr_0208_a3_base_document_with_uuid_references_passes_on_every_platform() {
    for platform in PLATFORMS {
        assert_passes(&base(platform));
    }
}

#[test]
fn adr_0208_a1_primitive_off_the_platform_fails_at_each_type_position() {
    for platform in PLATFORMS {
        for (primitive, flags) in matrix() {
            if has_length(primitive) || is_on_platform(flags, platform) {
                continue;
            }
            for position in Position::ALL {
                let doc = named_at(platform, position, primitive.name());
                assert_fails_at(&doc, position.pointer(), "anyOf");
            }
        }
    }
}

#[test]
fn adr_0208_a1_primitive_on_the_platform_passes_at_each_type_position() {
    for platform in PLATFORMS {
        for (primitive, flags) in matrix() {
            if has_length(primitive) || !is_on_platform(flags, platform) {
                continue;
            }
            for position in Position::ALL {
                assert_passes(&named_at(platform, position, primitive.name()));
            }
        }
    }
}

#[test]
fn adr_0208_a2_bare_string_and_wstring_fail_on_every_platform() {
    for platform in PLATFORMS {
        for name in ["STRING", "WSTRING"] {
            for position in Position::ALL {
                let doc = named_at(platform, position, name);
                assert_fails_at(&doc, position.pointer(), "anyOf");
            }
        }
    }
}

#[test]
fn adr_0208_a_enum_of_each_platform_equals_the_catalog() {
    let schema = v2_schema();
    for platform in PLATFORMS {
        let name = format!("type_ref_{}", platform.as_str());
        let listed = schema["$defs"][&name]["anyOf"][0]["enum"]
            .as_array()
            .expect("the enum list of the platform type_ref");
        let in_schema: BTreeSet<&str> = listed.iter().filter_map(Value::as_str).collect();
        assert_eq!(in_schema.len(), listed.len(), "{name} has no duplicate");
        let in_catalog: BTreeSet<&str> = matrix()
            .into_iter()
            .filter(|(primitive, flags)| {
                !has_length(*primitive) && is_on_platform(*flags, platform)
            })
            .map(|(primitive, _)| primitive.name())
            .collect();
        assert_eq!(in_schema, in_catalog, "{name}");
    }
}

#[test]
fn adr_0208_b_string_length_follows_the_range_of_the_platform() {
    for platform in PLATFORMS {
        for kind in [StringKind::String, StringKind::Wstring] {
            let name = match kind {
                StringKind::String => "STRING",
                StringKind::Wstring => "WSTRING",
            };
            let largest = (1..=16_383_u32)
                .filter(|length| StringLength::parse(kind, platform, *length).is_ok())
                .max();
            let with_length = |length: u32| {
                changed(base(platform), |doc| {
                    doc["types"][0]["name"] = json!(name);
                    doc["types"][0]["string_length"] = json!(length);
                })
            };
            let Some(largest) = largest else {
                assert_fails_at(&with_length(1), "/types/0/name", "const");
                continue;
            };
            assert_passes(&with_length(1));
            assert_passes(&with_length(largest));
            let beyond = largest.checked_add(1).expect("a small length");
            assert_fails_at(&with_length(beyond), "/types/0/string_length", "maximum");
        }
    }
}

#[test]
fn adr_0208_c_members_have_at_most_the_member_limit() {
    let with_members = |count: usize| {
        changed(base(PlcPlatform::S7_1500), |doc| {
            let members: Vec<Value> = (0..count)
                .map(|index| json!({ "name": format!("m{index}"), "type_id": STRING_ID }))
                .collect();
            doc["types"][1]["members"] = Value::Array(members);
        })
    };
    assert_passes(&with_members(MAX_MEMBERS));
    let beyond = MAX_MEMBERS.checked_add(1).expect("a small count");
    assert_fails_at(&with_members(beyond), "/types/1/members", "maxItems");
}

#[test]
fn adr_0208_d_bool_value_is_a_boolean() {
    assert_passes(&parameter_with(Primitive::Bool, json!(true)));
    let doc = parameter_with(Primitive::Bool, json!("true"));
    assert_fails_at(&doc, "/parameters/0/value", "type");
}

#[test]
fn adr_0208_d_char_and_wchar_value_is_a_string_with_one_character() {
    for primitive in [Primitive::Char, Primitive::Wchar] {
        assert_passes(&parameter_with(primitive, json!("A")));
        assert_passes(&parameter_with(primitive, json!("\u{00e9}")));
        let two = parameter_with(primitive, json!("AB"));
        assert_fails_at(&two, "/parameters/0/value", "maxLength");
        let none = parameter_with(primitive, json!(""));
        assert_fails_at(&none, "/parameters/0/value", "minLength");
        let boolean = parameter_with(primitive, json!(true));
        assert_fails_at(&boolean, "/parameters/0/value", "type");
    }
}

#[test]
fn adr_0208_d_numeric_duration_and_instant_value_is_canonical_decimal_text() {
    for (primitive, good, bad) in [
        (Primitive::Int, "-12", "012"),
        (Primitive::Real, "72.5", "72.50"),
        (Primitive::Time, "1500", "1.5e3"),
        (Primitive::Date, "20738", "+20738"),
    ] {
        assert_passes(&parameter_with(primitive, json!(good)));
        let doc = parameter_with(primitive, json!(bad));
        assert_fails_at(&doc, "/parameters/0/value", "pattern");
        let number = parameter_with(primitive, json!(12));
        assert_fails_at(&number, "/parameters/0/value", "type");
    }
}

#[test]
fn adr_0208_d_every_primitive_on_a_platform_follows_the_form_of_its_class() {
    for (primitive, flags) in matrix() {
        if has_length(primitive) || !is_on_platform(flags, PlcPlatform::S7_1500) {
            continue;
        }
        assert_passes(&parameter_with(primitive, matching_value(primitive)));
        let doc = parameter_with(primitive, wrong_value(primitive));
        assert!(
            !v2_failures(&doc).is_empty(),
            "{} accepts a value of the wrong form",
            primitive.name()
        );
    }
}

#[test]
fn adr_0208_d_name_lists_of_the_value_rule_equal_the_classes_of_the_domain() {
    let schema = v2_schema();
    let listed = |def: &str| -> BTreeSet<String> {
        schema["$defs"][def]["enum"]
            .as_array()
            .expect("the enum list of the definition")
            .iter()
            .filter_map(|name| name.as_str().map(str::to_owned))
            .collect()
    };
    let of_classes = |classes: &[LeafClass]| -> BTreeSet<String> {
        Primitive::ALL
            .into_iter()
            .filter(|primitive| classes.contains(&primitive.leaf_class()))
            .map(|primitive| primitive.name().to_owned())
            .collect()
    };
    assert_eq!(
        listed("primitive_names_decimal"),
        of_classes(&[LeafClass::Numeric, LeafClass::Duration, LeafClass::Instant])
    );
    assert_eq!(
        listed("primitive_names_char"),
        of_classes(&[LeafClass::Char])
    );
}

#[test]
fn adr_0208_e_leaf_below_a_uuid_reference_is_not_checked() {
    let doc = changed(base(PlcPlatform::S7_1500), |doc| {
        doc["parameters"][0]["value"] = json!(true);
        doc["parameters"][1]["value"] = json!({ "label": true, "note": "300000" });
    });
    assert_passes(&doc);
}

#[test]
fn adr_0208_i_every_golden_document_v2_passes() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden");
    let mut checked = 0_u32;
    for entry in std::fs::read_dir(dir).expect("read the golden directory") {
        let path = entry.expect("directory entry").path();
        let text = std::fs::read_to_string(&path).expect("read a golden document");
        let doc: Value = serde_json::from_str(&text).expect("parse a golden document");
        if doc["schema_version"] == json!("2") {
            assert_eq!(v2_failures(&doc), Vec::new(), "{}", path.display());
            checked = checked.checked_add(1).expect("a small count");
        }
    }
    assert!(checked >= 1, "at least one golden document v2");
}
