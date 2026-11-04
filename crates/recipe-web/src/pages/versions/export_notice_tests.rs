//! Tests of the notice of an Approved version that is not loadable (task 10.2, ADR-0119 (l), (m),
//! design D9).
//!
//! `R-400` is on the `S7_1500` machine `M30`. Its version 1 holds `DUE : DATE` with a stored count.
//! The count 72700 passes the earlier rule set and breaks the current one.

use axum::http::StatusCode;
use recipe_domain::recipe::VersionState;

use super::parameter_table_value_tests::{DATE_PAGE, EARLIER_DATE_COUNT, seed_date_world};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, tags};
use crate::test_support::plant::{POLISH, main_of};

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
