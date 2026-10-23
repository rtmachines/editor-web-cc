//! Tests of the export link of the version page (task 7.1, ADR-0052, design D8, D9, and D11 row
//! T9).
//!
//! The Administrator and the ProcessEngineer hold `VersionExport` and `VersionExportHistory`. The
//! Operator holds only `VersionExport`. `R-100` has the retired version 1, the approved version 2,
//! and the live draft 3. `R-300` is deleted. Each test compares the exact text from `</dl>` to the
//! end of the export `p`.

use axum::http::StatusCode;

use super::actions_tests::LINKS_TAG;
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::tags;
use crate::test_support::plant::{POLISH, World, main_of};
use crate::test_support::{TestSession, icon_tag};

/// The address of the approved version 2 of `R-100`.
const APPROVED: &str = "/recipes/R-100/versions/2";
/// The address of the retired version 1 of `R-100`.
const RETIRED: &str = "/recipes/R-100/versions/1";
/// The address of the live draft 3 of `R-100`.
const DRAFT: &str = "/recipes/R-100/versions/3";
/// The address of the retired version 1 of the deleted recipe `R-300`.
const DELETED: &str = "/recipes/R-300/versions/1";
/// The text of the notice of a retired version in English.
const NOTICE_EN: &str =
    "The file name starts with RETIRED_. The file marks this version as not loadable into a PLC.";
/// The text of the notice of a retired version in Polish.
const NOTICE_PL: &str =
    "Nazwa pliku zaczyna się od RETIRED_. Plik oznacza, że tej wersji nie można załadować do PLC.";

/// Return the markup of the info notice with the text `text`, as `components/alert.html` writes it.
fn notice_markup(text: &str) -> String {
    let icon = format!(
        "{}<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M12 11v6\"/><path d=\"M12 7.5v.01\"/></svg>",
        icon_tag("alert-info")
    );
    format!(
        "<div id=\"export-notice\"><div class=\"alert alert-info\">{icon}<span>{text}</span></div></div>"
    )
}

/// Return the export `p` with the anchor `href`, the text `text`, and the attributes `describedby`.
fn export_row(href: &str, text: &str, describedby: &str) -> String {
    format!(
        "<p class=\"actions\"><a class=\"btn btn-secondary\" href=\"{href}\" hx-boost=\"false\"{describedby}>{text}</a></p>"
    )
}

/// Return the expected text from `</dl>` to the end of the export `p` of the approved version 2.
fn approved_area(text: &str) -> String {
    let row = export_row("/export/v1/recipes/R-100/approved", text, "");
    format!("\n{row}")
}

/// Return the expected text from `</dl>` to the end of the export `p` of the retired version 1.
fn retired_area(text: &str, notice: &str) -> String {
    let describedby = " aria-describedby=\"export-notice\"";
    let row = export_row("/export/v1/recipes/R-100/versions/1", text, describedby);
    format!("\n{}\n{row}", notice_markup(notice))
}

/// Return the text of `main` after the header `dl` up to the end of the first `p`.
fn area_of(main: &str) -> String {
    let after_header = main.split_once("</dl>").unwrap().1;
    let end = after_header.find("</p>").unwrap().checked_add("</p>".len());
    after_header.get(..end.unwrap()).unwrap().to_owned()
}

/// Assert that the page of `path` shows `expected` after the header `dl` for `session`.
async fn assert_area(world: &World, session: &TestSession, path: &str, expected: &str) {
    let reply = world.get_en(session, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
    let main = main_of(&reply.body);
    assert_eq!(area_of(main), expected, "{path}: {main}");
    assert_eq!(main.matches("/export/").count(), 1, "{path}: {main}");
}

/// Assert that the page of `path` shows no export link and starts its row of links after `</dl>`.
async fn assert_no_link(world: &World, session: &TestSession, path: &str) {
    let reply = world.get_en(session, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
    let main = main_of(&reply.body);
    let after_header = main.split_once("</dl>").unwrap().1;
    assert!(after_header.trim_start().starts_with(LINKS_TAG), "{main}");
    assert!(!main.contains("/export/"), "{path}: {main}");
    assert!(!main.contains("export-notice"), "{path}: {main}");
}

#[tokio::test]
async fn export_link_approved_version_shows_the_link_to_every_role() {
    let world = recipe_world();
    let expected = approved_area("Export version 2 as JSON");
    for session in [&world.admin, &world.engineer, &world.operator] {
        assert_area(&world, session, APPROVED, &expected).await;
    }
}

#[tokio::test]
async fn export_link_retired_version_shows_the_link_and_the_notice_to_administrator_and_engineer() {
    let world = recipe_world();
    let expected = retired_area("Export version 1 as JSON", NOTICE_EN);
    for session in [&world.admin, &world.engineer] {
        assert_area(&world, session, RETIRED, &expected).await;
    }
}

#[tokio::test]
async fn export_link_retired_version_shows_no_link_to_the_operator() {
    let world = recipe_world();
    assert_no_link(&world, &world.operator, RETIRED).await;
}

#[tokio::test]
async fn export_link_draft_shows_no_link_to_any_role() {
    let world = recipe_world();
    for session in [&world.admin, &world.engineer, &world.operator] {
        assert_no_link(&world, session, DRAFT).await;
    }
}

#[tokio::test]
async fn export_link_discarded_draft_shows_no_link() {
    let world = recipe_world();
    let path = "/recipes/R-100/versions/3/discard";
    let discarded = world
        .post(&world.engineer, path, "row_version=1", false)
        .await;
    assert_eq!(
        discarded.status,
        StatusCode::SEE_OTHER,
        "{}",
        discarded.body
    );
    for session in [&world.admin, &world.engineer, &world.operator] {
        let reply = world.get_en(session, DRAFT).await;
        assert!(
            reply.body.contains("<span>Discarded</span>"),
            "{}",
            reply.body
        );
        assert_no_link(&world, session, DRAFT).await;
    }
}

#[tokio::test]
async fn export_link_version_of_a_deleted_recipe_shows_no_link() {
    let world = recipe_world();
    assert_no_link(&world, &world.admin, DELETED).await;
}

#[tokio::test]
async fn export_link_stays_in_read_only_mode() {
    let world = recipe_world();
    world.fixture.state.read_only().set(true);
    let approved = approved_area("Export version 2 as JSON");
    assert_area(&world, &world.operator, APPROVED, &approved).await;
    let retired = retired_area("Export version 1 as JSON", NOTICE_EN);
    assert_area(&world, &world.engineer, RETIRED, &retired).await;
    assert_no_link(&world, &world.operator, RETIRED).await;
    assert_no_link(&world, &world.admin, DRAFT).await;
}

#[tokio::test]
async fn export_link_is_a_plain_link_without_download_target_or_label() {
    let world = recipe_world();
    for (path, describedby) in [(APPROVED, None), (RETIRED, Some("export-notice"))] {
        let reply = world.get_en(&world.engineer, path).await;
        let anchors: Vec<_> = tags(&reply.body)
            .into_iter()
            .filter(|tag| tag.name == "a" && !tag.closing)
            .filter(|tag| {
                tag.attribute("href")
                    .is_some_and(|h| h.starts_with("/export/"))
            })
            .collect();
        assert_eq!(anchors.len(), 1, "{path}");
        let anchor = anchors.first().unwrap();
        assert_eq!(anchor.attribute("hx-boost"), Some("false"), "{path}");
        assert_eq!(
            anchor.attribute("class"),
            Some("btn btn-secondary"),
            "{path}"
        );
        assert_eq!(anchor.attribute("aria-describedby"), describedby, "{path}");
        for name in ["download", "target", "aria-label", "hx-get", "hx-target"] {
            assert!(anchor.attribute(name).is_none(), "{path}: {name}");
        }
    }
}

#[tokio::test]
async fn export_link_has_no_boosted_ancestor() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, RETIRED).await;
    for tag in tags(&reply.body) {
        let boosted = !tag.closing && tag.attribute("hx-boost") == Some("true");
        let is_boost_holder = matches!(tag.name.as_str(), "p" | "a" | "form");
        assert!(!boosted || is_boost_holder, "boosted <{}>", tag.name);
    }
    let main = main_of(&reply.body);
    let before_header = main.split_once("</dl>").unwrap().0;
    assert!(!before_header.contains("hx-boost"), "{before_header}");
}

#[tokio::test]
async fn export_link_texts_are_polish_in_the_polish_interface() {
    let world = recipe_world();
    let reply = world.get(&world.engineer, RETIRED, &[POLISH]).await;
    let main = main_of(&reply.body);
    assert_eq!(
        area_of(main),
        retired_area("Eksportuj wersję 1 do pliku JSON", NOTICE_PL),
        "{main}"
    );
    let reply = world.get(&world.operator, APPROVED, &[POLISH]).await;
    let main = main_of(&reply.body);
    assert_eq!(
        area_of(main),
        approved_area("Eksportuj wersję 2 do pliku JSON"),
        "{main}"
    );
}
