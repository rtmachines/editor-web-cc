//! Tests of the notice of a v1 document of a `Utf8` machine on the version page (task 11.19,
//! ADR-0202 (l), ADR-0119 (l)).
//!
//! Four cases cover the export area of an Approved version: healthy and faulty, each on a `Latin1`
//! machine and on a `Utf8` machine. A fault is a stored part that breaks a current rule. The rule
//! notice stands first and the encoding notice second. The link names the notices in the same order.

use axum::http::StatusCode;
use recipe_domain::recipe::VersionState;

use super::export_notice_tests::{CURRENT_DATE_END, describedby_of_export_link};
use super::parameter_table_value_tests::{DATE_PAGE, EARLIER_DATE_COUNT, seed_date_world};
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::{POLISH, World, main_of, world};
use crate::test_support::recipes::utf8_world::{
    APPROVED_REFERENCE, utf8_approved_faulty_world, utf8_approved_world,
};

/// The id of the rule notice and of the retired notice.
const RULE_ID: &str = "export-notice";
/// The id of the encoding notice.
const ENCODING_ID: &str = "export-encoding-notice";

/// The encoding notice in English.
const ENCODING_EN: &str = "This machine uses UTF-8 for STRING values. The v1 export file marks this version as not loadable into a PLC. Use the v2 export of each unit.";
/// The encoding notice in Polish.
const ENCODING_PL: &str = "Ta maszyna używa UTF-8 dla wartości STRING. Plik eksportu v1 oznacza, że tej wersji nie można załadować do PLC. Użyj eksportu v2 każdej jednostki.";
/// The rule notice of the code `E-RCP-005` in English.
const RULE_EN: &str = "The current rules refuse parts of this version. The export file marks this version as not loadable into a PLC. Rule codes: E-RCP-005.";

/// The version page of the approved version 1 of `R-300` on the `Utf8` machine `M10`.
fn utf8_page() -> String {
    format!("/recipes/{APPROVED_REFERENCE}/versions/1")
}

/// Return the export area of `main`: the markup from the end of the header list to the link.
fn export_area(main: &str) -> &str {
    let after_header = main.split_once("</dl>").unwrap().1;
    let end = after_header.find("</a>").unwrap();
    after_header.get(..end).unwrap()
}

/// Return the ids of the notice blocks of `area`, in the order of the page.
fn notice_ids(area: &str) -> Vec<&str> {
    area.split("<div id=\"")
        .skip(1)
        .map(|rest| rest.split_once('"').unwrap().0)
        .collect()
}

/// Assert that the export area of `html` holds the notices `ids` in this order and that the link
/// names the same ids, separated by one space. An empty `ids` means no notice and no attribute.
fn assert_notices(html: &str, ids: &[&str]) {
    let main = main_of(html);
    assert_eq!(notice_ids(export_area(main)), ids, "{main}");
    let expected = (!ids.is_empty()).then(|| ids.join(" "));
    assert_eq!(describedby_of_export_link(html), expected, "{main}");
    assert_clean_html(html);
}

/// Seed the approved version 1 of `R-400` on the `Latin1` machine `M30`, with the count `count`.
fn latin1_world(count: i64) -> World {
    let world = world();
    seed_date_world(&world, count, VersionState::Approved);
    world
}

#[tokio::test]
async fn adr_0202_version_notice_latin1_healthy_shows_no_notice() {
    let world = latin1_world(CURRENT_DATE_END);
    let reply = world.get_en(&world.engineer, DATE_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_notices(&reply.body, &[]);
}

#[tokio::test]
async fn adr_0202_version_notice_latin1_faulty_shows_only_the_rule_notice() {
    let world = latin1_world(EARLIER_DATE_COUNT);
    let reply = world.get_en(&world.engineer, DATE_PAGE).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("Rule codes: E-RCP-005."), "{main}");
    assert!(!main.contains(ENCODING_ID), "{main}");
    assert_notices(&reply.body, &[RULE_ID]);
}

#[tokio::test]
async fn adr_0202_version_notice_utf8_healthy_shows_only_the_encoding_notice() {
    let world = utf8_approved_world().await;
    let reply = world.get_en(&world.operator, &utf8_page()).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let area = export_area(main);
    assert!(
        area.trim_start().starts_with(&format!(
            "<div id=\"{ENCODING_ID}\"><div class=\"alert alert-info\">"
        )),
        "{area}"
    );
    assert!(
        area.contains(&format!("<span>{ENCODING_EN}</span>")),
        "{area}"
    );
    assert!(!main.contains("alert-warning"), "{main}");
    assert_notices(&reply.body, &[ENCODING_ID]);
}

#[tokio::test]
async fn adr_0202_version_notice_utf8_faulty_shows_the_rule_notice_then_the_encoding_notice() {
    let world = utf8_approved_faulty_world().await;
    let reply = world.get_en(&world.operator, &utf8_page()).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    let area = export_area(main);
    assert!(
        area.contains(&format!(
            "<div id=\"{RULE_ID}\"><div class=\"alert alert-warning\">"
        )),
        "{area}"
    );
    assert!(area.contains(&format!("<span>{RULE_EN}</span>")), "{area}");
    assert!(
        area.contains(&format!(
            "<div id=\"{ENCODING_ID}\"><div class=\"alert alert-info\">"
        )),
        "{area}"
    );
    assert!(
        area.contains(&format!("<span>{ENCODING_EN}</span>")),
        "{area}"
    );
    assert_notices(&reply.body, &[RULE_ID, ENCODING_ID]);
}

#[tokio::test]
async fn adr_0202_version_notice_text_is_polish_in_the_polish_interface() {
    let world = utf8_approved_world().await;
    let reply = world.get(&world.operator, &utf8_page(), &[POLISH]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains(&format!("<span>{ENCODING_PL}</span>")),
        "{main}"
    );
    assert_notices(&reply.body, &[ENCODING_ID]);
}
