//! Tests of the S7 layout table of the type page (spec "S7 layout of a type page", design D5,
//! ADR-0108 g and h).

use axum::http::StatusCode;
use recipe_domain::plant::{PlcPlatform, PlcTypeId, Primitive};

use super::test_kit::{
    RowOwner, between, get_as, get_in, primitive_id, seed_array_for, seed_machine_on,
    seed_struct_for,
};
use crate::test_support::plant::{ENGLISH, POLISH, World, world};

/// The caption id of the table. The test cuts the table out of the page with it.
const CAPTION: &str = "id=\"type-s7-caption\"";

/// Seed the machine `M<number>` on `platform` and return the owner of its rows.
fn machine(world: &World, number: u32, platform: PlcPlatform) -> RowOwner {
    let (_code, id) = seed_machine_on(&world.fixture, number, platform);
    (id, platform)
}

/// Seed `ST_Zone@2` of the spec: two `BOOL`, a `REAL`, and an `INT`.
fn seed_zone(world: &World, owner: RowOwner) -> PlcTypeId {
    let bool_id = primitive_id(Primitive::Bool);
    let members = [
        ("active", bool_id),
        ("ready", bool_id),
        ("setpoint", primitive_id(Primitive::Real)),
        ("count", primitive_id(Primitive::Int)),
    ];
    seed_struct_for(world, owner, 1, ("ST_Zone", 2), &members)
}

/// Seed `ST_Line@1` of the spec: the struct `ST_Zone@2` and an `ARRAY[1..3] OF INT`.
fn seed_line(world: &World, owner: RowOwner) {
    let zone = seed_zone(world, owner);
    let ramps = seed_array_for(world, owner, 2, primitive_id(Primitive::Int), &[(1, 3)]);
    seed_struct_for(
        world,
        owner,
        3,
        ("ST_Line", 1),
        &[("zone", zone), ("ramps", ramps)],
    );
}

/// Return the cells of the body rows of the S7 table: the path, the type, and the offset.
fn table_rows(body: &str) -> Vec<[String; 3]> {
    let table = between(body, CAPTION, "</table>");
    let rows = between(table, "<tbody>", "</tbody>");
    rows.split("<tr>")
        .skip(1)
        .map(|row| {
            let cells: Vec<String> = row
                .split("<td class=\"")
                .skip(1)
                .map(|cell| {
                    let (_class, rest) = cell.split_once("\">").unwrap();
                    rest.split_once("</td>").unwrap().0.to_owned()
                })
                .collect();
            <[String; 3]>::try_from(cells).unwrap()
        })
        .collect()
}

fn row(path: &str, type_text: &str, offset: &str) -> [String; 3] {
    [path.to_owned(), type_text.to_owned(), offset.to_owned()]
}

#[tokio::test]
async fn types_s7_layout_struct_with_two_bool_members_shows_the_offsets_and_the_size() {
    let world = world();
    let owner = machine(&world, 20, PlcPlatform::S7_300);
    seed_zone(&world, owner);
    let reply = get_as(&world, &world.operator, "/machines/M20/types/ST_Zone", &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = &reply.body;
    assert_eq!(
        table_rows(body),
        vec![
            row(".active", "BOOL", "0.0"),
            row(".ready", "BOOL", "0.1"),
            row(".setpoint", "REAL", "2.0"),
            row(".count", "INT", "6.0"),
        ]
    );
    assert!(
        body.contains("<p>The type has a size of 8 bytes.</p>"),
        "{body}"
    );
}

#[tokio::test]
async fn types_s7_layout_nested_struct_and_array_show_one_row_for_each_member() {
    let world = world();
    let owner = machine(&world, 21, PlcPlatform::S7_300);
    seed_line(&world, owner);
    let reply = get_as(&world, &world.operator, "/machines/M21/types/ST_Line", &[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(
        table_rows(&reply.body),
        vec![
            row(".zone", "ST_Zone@2", "0.0"),
            row(".zone.active", "BOOL", "0.0"),
            row(".zone.ready", "BOOL", "0.1"),
            row(".zone.setpoint", "REAL", "2.0"),
            row(".zone.count", "INT", "6.0"),
            row(".ramps", "ARRAY[1..3] OF INT", "8.0"),
        ]
    );
    assert!(
        reply
            .body
            .contains("<p>The type has a size of 14 bytes.</p>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn types_s7_layout_is_shown_for_the_s7_1200_and_the_s7_1500() {
    let world = world();
    for (number, platform) in [(22, PlcPlatform::S7_1200), (23, PlcPlatform::S7_1500)] {
        let owner = machine(&world, number, platform);
        seed_zone(&world, owner);
        let path = format!("/machines/M{number}/types/ST_Zone");
        let reply = get_as(&world, &world.operator, &path, &[]).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_eq!(table_rows(&reply.body).len(), 4, "{platform}");
        assert!(
            reply
                .body
                .contains("<p>The type has a size of 8 bytes.</p>"),
            "{platform}"
        );
    }
}

#[tokio::test]
async fn types_s7_layout_is_not_shown_for_the_codesys_and_the_twincat_platforms() {
    let world = world();
    for (number, platform) in [(24, PlcPlatform::CodesysIec), (25, PlcPlatform::TwinCat3)] {
        let owner = machine(&world, number, platform);
        seed_zone(&world, owner);
        let path = format!("/machines/M{number}/types/ST_Zone");
        let reply = get_as(&world, &world.operator, &path, &[]).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let body = &reply.body;
        assert!(body.contains("id=\"type-tree-title\""), "{platform}");
        assert!(!body.contains(CAPTION), "{platform}: {body}");
        assert!(!body.contains("The type has a size of"), "{platform}");
    }
}

/// The sentence of ADR-0108 (h) in the English bundle.
const ACCESS_SENTENCE_EN: &str = "<p>The offsets apply to a data block with standard access.</p>";
/// The same sentence in the Polish bundle.
const ACCESS_SENTENCE_PL: &str =
    "<p>Przesunięcia dotyczą bloku danych z dostępem standardowym.</p>";

#[tokio::test]
async fn types_s7_layout_standard_access_sentence_stands_before_the_table_on_every_s7_platform() {
    let world = world();
    let platforms = [
        (30, PlcPlatform::S7_300),
        (31, PlcPlatform::S7_1200),
        (32, PlcPlatform::S7_1500),
    ];
    for (number, platform) in platforms {
        let owner = machine(&world, number, platform);
        seed_zone(&world, owner);
        let path = format!("/machines/M{number}/types/ST_Zone");
        for (reply, sentence) in [
            (
                get_as(&world, &world.operator, &path, &[]).await,
                ACCESS_SENTENCE_EN,
            ),
            (
                get_in(&world, &world.operator, &path, POLISH).await,
                ACCESS_SENTENCE_PL,
            ),
        ] {
            let body = &reply.body;
            assert_eq!(body.matches(sentence).count(), 1, "{platform}: {body}");
            let note = body.find(sentence).unwrap();
            let table = body.find(CAPTION).unwrap();
            let tree = body.find("id=\"type-tree-title\"").unwrap();
            assert!(tree < note && note < table, "{platform}: {body}");
            let first_table = body.find("<table").unwrap();
            assert!(
                note < first_table && first_table < table,
                "{platform}: {body}"
            );
        }
    }
}

#[tokio::test]
async fn types_s7_layout_standard_access_sentence_is_absent_on_codesys_and_twincat() {
    let world = world();
    for (number, platform) in [(33, PlcPlatform::CodesysIec), (34, PlcPlatform::TwinCat3)] {
        let owner = machine(&world, number, platform);
        seed_zone(&world, owner);
        let path = format!("/machines/M{number}/types/ST_Zone");
        for (reply, sentence) in [
            (
                get_as(&world, &world.operator, &path, &[]).await,
                ACCESS_SENTENCE_EN,
            ),
            (
                get_in(&world, &world.operator, &path, POLISH).await,
                ACCESS_SENTENCE_PL,
            ),
        ] {
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
            assert!(!reply.body.contains(sentence), "{platform}: {}", reply.body);
            assert!(
                !reply.body.contains("standard access")
                    && !reply.body.contains("dostępem standardowym"),
                "{platform}: {}",
                reply.body
            );
        }
    }
}

#[tokio::test]
async fn types_s7_layout_follows_the_type_tree_in_the_page() {
    let world = world();
    let owner = machine(&world, 26, PlcPlatform::S7_1500);
    seed_zone(&world, owner);
    let reply = get_as(&world, &world.operator, "/machines/M26/types/ST_Zone", &[]).await;
    let body = &reply.body;
    let tree = body.find("id=\"type-tree-title\"").unwrap();
    let layout = body.find(CAPTION).unwrap();
    let links = body.find("Show the use of this revision").unwrap();
    assert!(tree < layout && layout < links, "{body}");
    for header in ["Path", "Type", "Offset"] {
        let cell = format!("<th scope=\"col\">{header}</th>");
        assert!(body.contains(&cell), "{header}: {body}");
    }
    assert!(body.contains(">S7 layout</caption>"), "{body}");
}

#[tokio::test]
async fn types_s7_layout_texts_are_polish_in_the_polish_bundle() {
    let world = world();
    let owner = machine(&world, 27, PlcPlatform::S7_1200);
    seed_zone(&world, owner);
    let path = "/machines/M27/types/ST_Zone";
    let reply = get_in(&world, &world.operator, path, POLISH).await;
    let body = &reply.body;
    assert!(body.contains(">Układ S7</caption>"), "{body}");
    for header in ["Ścieżka", "Typ", "Przesunięcie"] {
        let cell = format!("<th scope=\"col\">{header}</th>");
        assert!(body.contains(&cell), "{header}: {body}");
    }
    assert!(body.contains("<p>Rozmiar typu w bajtach: 8.</p>"), "{body}");
}

#[tokio::test]
async fn types_s7_layout_size_sentence_is_plain_text_in_the_english_bundle() {
    let world = world();
    let owner = machine(&world, 28, PlcPlatform::S7_1500);
    seed_zone(&world, owner);
    let path = "/machines/M28/types/ST_Zone";
    let reply = get_in(&world, &world.operator, path, ENGLISH).await;
    let body = &reply.body;
    assert!(
        body.contains("<p>The type has a size of 8 bytes.</p>"),
        "{body}"
    );
    assert!(!body.contains('\u{2068}'), "{body}");
}
