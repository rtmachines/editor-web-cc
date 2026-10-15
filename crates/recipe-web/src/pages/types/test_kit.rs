//! Helpers of the tests of the type pages (design D12 and D16).
//!
//! The module solves one problem: the tests of the list, the form, the definition, the tree, and the
//! usage page need the same world. It has the three roles, the plant data of `seed_plant`, and a
//! few extra machines and types. One module keeps these helpers in one place.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use recipe_app::memory::seed_type_id;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, Machine, MachineCode, MachineId, MemberName, PlcPlatform,
    PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, TypeMember, TypeName, UnitId,
};
use recipe_domain::text::Text;
use uuid::Uuid;

use crate::i18n::Lang;
use crate::pages::test_helpers::encode;
use crate::test_support::plant::{ENGLISH, HTMX_CONTENT, World};
use crate::test_support::{Fixture, Reply, TestSession, text};

/// Send `GET path` in English as `session`, with the extra headers.
pub(super) async fn get_as(
    world: &World,
    session: &TestSession,
    path: &str,
    extra: &[(&str, &str)],
) -> Reply {
    let mut headers = vec![ENGLISH];
    headers.extend_from_slice(extra);
    world.get(session, path, &headers).await
}

/// Send `GET path` in the language of `language` as `session`.
pub(super) async fn get_in(
    world: &World,
    session: &TestSession,
    path: &str,
    language: (&str, &str),
) -> Reply {
    world.get(session, path, &[language]).await
}

/// Send the form `body` to `path` in English as `session`, with the extra headers.
pub(super) async fn post_as(
    world: &World,
    session: &TestSession,
    path: &str,
    body: &str,
    extra: &[(&str, &str)],
) -> Reply {
    world.post_with(session, path, body, extra).await
}

/// Send the form `body` to `path` as an HTMX request that targets `main`.
pub(super) async fn post_htmx(
    world: &World,
    session: &TestSession,
    path: &str,
    body: &str,
) -> Reply {
    post_as(world, session, path, body, &HTMX_CONTENT).await
}

/// Build the body of the UDT form as a browser sends it.
///
/// The body holds `name` when there is one, then `members[i].name` and `members[i].type` for each
/// row, then the extra fields. Every key and value is percent-encoded.
pub(super) fn form_body(
    name: Option<&str>,
    rows: &[(&str, &str)],
    extra: &[(&str, &str)],
) -> String {
    let mut pairs: Vec<(String, String)> = Vec::new();
    if let Some(name) = name {
        pairs.push(("name".to_owned(), name.to_owned()));
    }
    for (index, (member, type_text)) in rows.iter().enumerate() {
        pairs.push((format!("members[{index}].name"), (*member).to_owned()));
        pairs.push((format!("members[{index}].type"), (*type_text).to_owned()));
    }
    pairs.extend(
        extra
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned())),
    );
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", encode(key), encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Parse a unit identifier from a number. The tests use fixed identifiers.
fn unit_id(number: u128) -> UnitId {
    UnitId::from(Uuid::from_u128(number))
}

/// Seed a live S7-300 machine without a struct and return its code.
///
/// `number` is the number of the code, for example 3 for `M3`.
pub(super) fn seed_empty_machine(fixture: &Fixture, number: u32) -> MachineCode {
    let code = MachineCode::parse(&format!("M{number}")).unwrap();
    let machine = Machine::create(
        MachineId::from(Uuid::from_u128(
            0x7357_0f00_0000_0000_0000_0000_0000_0000 | u128::from(number),
        )),
        code,
        Text::parse("Empty line").unwrap(),
        PlcPlatform::S7_300,
        (
            unit_id(0x7357_0f01_0000_0000_0000_0000_0000_0000 | u128::from(number)),
            Text::parse("Infeed").unwrap(),
        ),
    );
    fixture.store.seed_machine(&machine);
    code
}

/// The identifier of the struct row that a test seeds with the number `number`.
pub(super) fn struct_id(number: u128) -> PlcTypeId {
    PlcTypeId::from(Uuid::from_u128(
        0x7357_0e00_0000_0000_0000_0000_0000_0000 | number,
    ))
}

/// Seed the struct `name@1` of `M7` with the members `members` and return its identifier.
///
/// `MemoryStore::seed_type` writes the row without a check. The member types may name rows that
/// the test seeds later, so a test can build a tree that no request can make.
pub(super) fn seed_struct(
    world: &World,
    number: u128,
    name: &str,
    members: &[(&str, PlcTypeId)],
) -> PlcTypeId {
    let id = struct_id(number);
    let row = PlcType::try_from(PlcTypeRecord {
        id,
        machine: Some(world.ids.m7),
        platforms: BTreeSet::from([PlcPlatform::S7_300]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision: 1,
            supersedes: None,
            members: members
                .iter()
                .map(|(member, type_id)| TypeMember {
                    name: MemberName::parse(member).unwrap(),
                    type_id: *type_id,
                })
                .collect(),
        },
    })
    .unwrap();
    world.fixture.store.seed_type(&row);
    id
}

/// Seed an array row of `M7` and return its identifier.
///
/// `dimensions` holds the lower and the upper bound of each dimension.
pub(super) fn seed_array(
    world: &World,
    number: u128,
    element: PlcTypeId,
    dimensions: &[(i32, i32)],
) -> PlcTypeId {
    let id = struct_id(number);
    let parsed: Vec<ArrayDimension> = dimensions
        .iter()
        .map(|(lower, upper)| ArrayDimension::parse(*lower, *upper).unwrap())
        .collect();
    let row = PlcType::try_from(PlcTypeRecord {
        id,
        machine: Some(world.ids.m7),
        platforms: BTreeSet::from([PlcPlatform::S7_300]),
        shape: RecordShape::Array {
            element,
            dimensions: ArrayDimensions::parse(parsed).unwrap(),
        },
    })
    .unwrap();
    world.fixture.store.seed_type(&row);
    id
}

/// Return the identifier of the seed primitive `primitive`, for example `INT`.
pub(super) fn primitive_id(primitive: Primitive) -> PlcTypeId {
    seed_type_id(primitive, None).unwrap()
}

/// Return the identifier of the seed row `STRING(254)`, the default string of the S7 platforms.
pub(super) fn string_id() -> PlcTypeId {
    seed_type_id(Primitive::String, Some(254)).unwrap()
}

/// Seed the structs `ST_Alpha@1` and `ST_Beta@1` of `M7`, which contain each other.
///
/// The store then holds a cycle that no request can make (ADR-0037 u).
pub(super) fn seed_cycle(world: &World) {
    let alpha = struct_id(1);
    let beta = struct_id(2);
    seed_struct(world, 1, "ST_Alpha", &[("beta", beta)]);
    seed_struct(world, 2, "ST_Beta", &[("alpha", alpha)]);
}

/// Return the text from `start` up to the first `end` after it.
///
/// The test fails when a marker is missing.
pub(super) fn between<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let (_before, rest) = text.split_once(start).unwrap();
    rest.split_once(end).unwrap().0
}

/// Return the table row `<tr>...</tr>` that holds `needle`. The test fails when no row holds it.
pub(super) fn row_of(body: &str, needle: &str) -> String {
    let (head, tail) = body.split_once(needle).unwrap();
    let start = head.rfind("<tr>").unwrap();
    let end = tail.find("</tr>").unwrap();
    format!("{}{needle}{}", &head[start..], &tail[..end])
}

/// Assert the content answer of design D6 for a page: the title, the regions, the heading that
/// takes the focus, and the field `next` out of band. The answer has no frame of the page.
pub(super) fn assert_content_answer(reply: &Reply, heading: &str, next: &str) {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = &reply.body;
    for frame in [
        "<!DOCTYPE",
        "<html",
        "<head",
        "<body",
        "<main",
        "class=\"topbar\"",
        "class=\"skip-link\"",
        "class=\"lang-switch\"",
    ] {
        assert!(!body.contains(frame), "found {frame}: {body}");
    }
    let title = format!("<title>{heading} · {}</title>", text(Lang::En, "app-name"));
    assert!(body.contains(&title), "{body}");
    assert!(body.contains("<div id=\"error-panel\"></div>"), "{body}");
    assert!(body.contains("<div id=\"toast\" role=\"status\" aria-live=\"polite\"></div>"));
    assert!(
        body.contains(&format!("<h1 tabindex=\"-1\" autofocus>{heading}</h1>")),
        "{body}"
    );
    assert_eq!(body.matches("<h1").count(), 1, "{body}");
    let field = format!(
        "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"{next}\" hx-swap-oob=\"true\">"
    );
    assert!(body.contains(&field), "{body}");
}
