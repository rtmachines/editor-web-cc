//! Tests of the notice of an Approved version that is not loadable (task 10.2, ADR-0119 (l), (m),
//! design D9).
//!
//! `R-400` is on the `S7_1500` machine `M30`. Its version 1 holds `DUE : DATE` with a stored count.
//! The count 72700 passes the earlier rule set and breaks the current one.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{
    Machine, MachineCode, MachineId, MemberName, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord,
    Primitive, RecordShape, S7Address, TypeMember, TypeName, UnitId,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, Recipe, RecipeId, RecipeVersionId,
    ReferenceNumber, StoredParameter, StoredRecipe, StoredValue, StoredVersionHeader, VersionState,
};
use recipe_domain::text::{LongText, Text};
use serde_json::json;
use uuid::Uuid;

use super::parameter_table_value_tests::{DATE_PAGE, EARLIER_DATE_COUNT, seed_date_world};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, tags};
use crate::test_support::plant::{POLISH, World, main_of};

/// The text of the notice in English.
const NOTICE_EN: &str = "The current rules refuse parts of this version. The export file marks this version as not loadable into a PLC. Rule codes: E-RCP-005.";
/// The text of the notice in Polish.
const NOTICE_PL: &str = "Aktualne reguły odrzucają części tej wersji. Plik eksportu oznacza, że tej wersji nie można załadować do PLC. Kody reguł: E-RCP-005.";
/// The stored count at the end of the current `DATE` range, 2168-12-31.
const CURRENT_DATE_END: i64 = 72_683;

/// Return the value of `aria-describedby` of the one export link in `html`.
fn describedby_of_export_link(html: &str) -> Option<String> {
    let anchors: Vec<_> = tags(html)
        .into_iter()
        .filter(|tag| tag.name == "a" && !tag.closing)
        .filter(|tag| {
            tag.attribute("href")
                .is_some_and(|href| href.starts_with("/export/"))
        })
        .collect();
    assert_eq!(anchors.len(), 1, "{html}");
    let anchor = anchors.first().unwrap();
    assert_eq!(anchor.attribute("hx-boost"), Some("false"), "{html}");
    anchor.attribute("aria-describedby").map(str::to_owned)
}

#[tokio::test]
async fn export_notice_shows_the_code_in_the_export_area_and_the_link_names_it() {
    let world = recipe_world();
    seed_date_world(&world, EARLIER_DATE_COUNT, VersionState::Approved);
    let reply = world.get_en(&world.engineer, DATE_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let after_header = main.split_once("</dl>").unwrap().1;
    let notice_open = "<div id=\"export-notice\"><div class=\"alert alert-warning\">";
    assert!(after_header.trim_start().starts_with(notice_open), "{main}");
    assert!(
        after_header.contains(&format!("<span>{NOTICE_EN}</span></div></div>\n<p class=\"actions\"><a class=\"btn btn-secondary\" href=\"/export/v1/recipes/R-400/approved\" hx-boost=\"false\" aria-describedby=\"export-notice\">")),
        "{main}"
    );
    assert_eq!(
        describedby_of_export_link(&reply.body).as_deref(),
        Some("export-notice")
    );
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn export_notice_is_polish_in_the_polish_interface() {
    let world = recipe_world();
    seed_date_world(&world, EARLIER_DATE_COUNT, VersionState::Approved);
    let reply = world.get(&world.engineer, DATE_PAGE, &[POLISH]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains(&format!("<span>{NOTICE_PL}</span>")),
        "{main}"
    );
}

#[tokio::test]
async fn export_notice_is_absent_for_a_date_of_the_current_range() {
    let world = recipe_world();
    seed_date_world(&world, CURRENT_DATE_END, VersionState::Approved);
    let reply = world.get_en(&world.engineer, DATE_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(!main.contains("export-notice"), "{main}");
    assert!(!main.contains("alert-warning"), "{main}");
    assert_eq!(describedby_of_export_link(&reply.body), None);
}

#[tokio::test]
async fn export_notice_is_absent_for_a_draft_with_the_earlier_date() {
    let world = recipe_world();
    seed_date_world(&world, EARLIER_DATE_COUNT, VersionState::LiveDraft);
    let reply = world.get_en(&world.engineer, DATE_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!main_of(&reply.body).contains("export-notice"));
}

// --- An overlap after a change of the layout rules (ADR-0149 (g), design D10) --------------------

/// The version page of the approved version 1 of `R-420` on the `S7_1500` machine `M32`.
const OVERLAP_PAGE: &str = "/recipes/R-420/versions/1";
/// The text of the notice for the code `E-RCP-022` in English.
const OVERLAP_NOTICE_EN: &str = "The current rules refuse parts of this version. The export file marks this version as not loadable into a PLC. Rule codes: E-RCP-022.";
/// The first field of the identifier of a type of `M32`.
const OVERLAP_TYPE_BASE: u128 = 0x7357_0013_0000_0000_0000_0000_0000_0f20;
/// The first field of the identifier and of the key of a parameter of `R-420`.
const OVERLAP_PARAMETER_BASE: u128 = 0x7357_0006_0000_0000_0000_0000_0f00_0050;

/// The primitive types of the struct `ST_Bytes@1`, in the order of the members.
const BYTES_MEMBERS: [(&str, Primitive); 4] = [
    ("a", Primitive::Byte),
    ("b", Primitive::Char),
    ("c", Primitive::Sint),
    ("d", Primitive::Usint),
];

/// Return the identifier of the type row number `n` of `M32`.
fn overlap_type(n: u128) -> PlcTypeId {
    PlcTypeId::from(Uuid::from_u128(OVERLAP_TYPE_BASE | n))
}

/// Return the platforms of the global row of `primitive` (`0004_seed_plc_types.sql`).
///
/// `SINT` and `USINT` do not exist on `S7_300`. `CHAR` does not exist on the IEC platforms.
fn byte_family_platforms(primitive: Primitive) -> BTreeSet<PlcPlatform> {
    PlcPlatform::ALL
        .into_iter()
        .filter(|platform| match primitive {
            Primitive::Sint | Primitive::Usint => *platform != PlcPlatform::S7_300,
            Primitive::Char => matches!(
                platform,
                PlcPlatform::S7_1200 | PlcPlatform::S7_1500 | PlcPlatform::S7_300
            ),
            _ => true,
        })
        .collect()
}

/// Build the global primitive row `primitive` with the identifier `id` and the seed platforms.
fn overlap_primitive_row(id: PlcTypeId, primitive: Primitive) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id,
        machine: None,
        platforms: byte_family_platforms(primitive),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: primitive.fixed_size_bytes().unwrap(),
            string_length: None,
        },
    })
    .unwrap()
}

/// Build the stored parameter `tag` of the type `plc_type` at `address`. `n` makes it unique.
fn overlap_parameter(
    (n, tag): (u32, &str),
    plc_type: PlcTypeId,
    address: &str,
    value: serde_json::Value,
    unit: UnitId,
) -> StoredParameter {
    let id = Uuid::from_u128(OVERLAP_PARAMETER_BASE | u128::from(n));
    StoredParameter {
        id: ParameterId::from(id),
        key: ParameterKey::from(id),
        unit,
        tag: ParameterTag::parse(tag).unwrap(),
        name_en: Text::parse(tag).unwrap(),
        description_en: LongText::parse("A seeded parameter").unwrap(),
        translations: Vec::new(),
        plc_type,
        s7_address: Some(S7Address::parse(address).unwrap()),
        limits: Vec::new(),
        value: StoredValue::from(value),
        ordinal: Ordinal::parse(n).unwrap(),
    }
}

/// Seed the approved version 1 of `R-420` on the `S7_1500` machine `M32`.
///
/// The test world has none of the four primitive rows, so the function stores them with the
/// platforms of the seed migration. The struct `ST_Bytes@1` holds `BYTE`, `CHAR`, `SINT`, and
/// `USINT`. The parameter `BYTES` of that type stands at `DB1.DBB0` and `MODE` of type `BYTE`
/// stands at `DB1.DBB4`. The earlier layout gave
/// `ST_Bytes@1` the size 4, so the version had no overlap when it was approved. The current layout
/// gives the size 6, and the struct then shares the byte 4 with `MODE`.
fn seed_overlap_world(world: &World) {
    let machine_id = MachineId::from(Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0000_0020));
    let unit_id = UnitId::from(Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_0020_0001));
    let machine = Machine::create(
        machine_id,
        MachineCode::parse("M32").unwrap(),
        Text::parse("Byte line").unwrap(),
        PlcPlatform::S7_1500,
        (unit_id, Text::parse("Infeed").unwrap()),
    );
    world.fixture.store.seed_machine(&machine);
    let mut members = Vec::new();
    for (n, (name, primitive)) in (1_u128..).zip(BYTES_MEMBERS) {
        world
            .fixture
            .store
            .seed_type(&overlap_primitive_row(overlap_type(n), primitive));
        members.push(TypeMember {
            name: MemberName::parse(name).unwrap(),
            type_id: overlap_type(n),
        });
    }
    let bytes = overlap_type(5);
    let record = PlcTypeRecord {
        id: bytes,
        machine: Some(machine_id),
        platforms: BTreeSet::from([PlcPlatform::S7_1500]),
        shape: RecordShape::Struct {
            name: TypeName::parse("ST_Bytes").unwrap(),
            revision: 1,
            supersedes: None,
            members,
        },
    };
    world
        .fixture
        .store
        .seed_type(&PlcType::try_from(record).unwrap());
    let recipe = Recipe::try_from(StoredRecipe {
        id: RecipeId::from(Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0000_0420)),
        machine: machine_id,
        reference: ReferenceNumber::parse("R-420").unwrap(),
        name: Text::parse("Byte recipe").unwrap(),
        deleted_at: None,
        versions: vec![StoredVersionHeader {
            id: RecipeVersionId::from(Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0000_0421)),
            number: 1,
            state: VersionState::Approved,
            row_version: 1,
        }],
    })
    .unwrap();
    let creator = UserId::from(Uuid::from_u128(1));
    world
        .fixture
        .store
        .seed_recipe(&recipe, PlcPlatform::S7_1500, creator);
    let version = RecipeVersionId::from(Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0000_0421));
    let stored = [
        overlap_parameter(
            (1, "BYTES"),
            bytes,
            "DB1.DBB0",
            json!({"a": 0, "b": "A", "c": 0, "d": 0}),
            unit_id,
        ),
        overlap_parameter((2, "MODE"), overlap_type(1), "DB1.DBB4", json!(0), unit_id),
    ];
    for parameter in &stored {
        world.fixture.store.seed_parameter(version, parameter);
    }
}

#[tokio::test]
async fn export_notice_shows_e_rcp_022_for_an_approved_version_with_an_overlap() {
    let world = recipe_world();
    seed_overlap_world(&world);
    let reply = world.get_en(&world.engineer, OVERLAP_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let after_header = main.split_once("</dl>").unwrap().1;
    let notice_open = "<div id=\"export-notice\"><div class=\"alert alert-warning\">";
    assert!(after_header.trim_start().starts_with(notice_open), "{main}");
    assert!(
        after_header.contains(&format!("<span>{OVERLAP_NOTICE_EN}</span></div></div>\n<p class=\"actions\"><a class=\"btn btn-secondary\" href=\"/export/v1/recipes/R-420/approved\" hx-boost=\"false\" aria-describedby=\"export-notice\">")),
        "{main}"
    );
    assert_eq!(
        describedby_of_export_link(&reply.body).as_deref(),
        Some("export-notice")
    );
    assert_clean_html(&reply.body);
}
