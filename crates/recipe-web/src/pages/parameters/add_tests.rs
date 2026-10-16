//! Tests of `POST .../parameters`: `AddParameter`, `fields=1`, and `limit_mode` (task 11.1).
//!
//! Specs "Form of a new parameter" and "Form rendered again keeps the input", design D7,
//! ADR-0039 (h), (i), and (k), ADR-0043 (i).

use axum::http::StatusCode;

use super::new_tests::{FORM_TAG, PAGE_FORM_TAG};
use super::test_kit::{ADD, INT_FIELDS, assert_panel_error, changed, form, position, post_htmx};
use super::value_tests::assert_operator_refused;
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, assert_error, field_value};
use crate::test_support::Reply;
use crate::test_support::plant::{HTMX, TARGET_CONTENT, World};

/// The value of `HX-Retarget` of an answer that renders the form of a new parameter again.
const FORM_RETARGET: &str = "#parameter-new-form";
/// The opening tag of the out-of-band form of an error answer of the form in the version page.
const OOB_FORM_TAG: &str = "<form id=\"parameter-new-form\" method=\"post\" action=\"/recipes/R-100/versions/3/parameters\" hx-boost=\"true\" hx-target=\"#parameter-rows\" hx-swap=\"beforeend show:none focus-scroll:true\" hx-push-url=\"false\" hx-sync=\"closest form:drop\" hx-swap-oob=\"true\">";
/// The opening tag of the out-of-band form of an error answer of the stand-alone page form.
const PAGE_OOB_FORM_TAG: &str = "<form id=\"parameter-new-form\" method=\"post\" action=\"/recipes/R-100/versions/3/parameters\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\" hx-swap-oob=\"true\">";

/// Send the form `pairs` from the stand-alone page form: an HTMX request with `HX-Target: content`.
async fn post_page_form(world: &World, pairs: &[(&str, &str)]) -> Reply {
    world
        .post_with(&world.engineer, ADD, &form(pairs), &[HTMX, TARGET_CONTENT])
        .await
}

#[tokio::test]
async fn parameters_add_of_the_page_form_with_fields_or_limit_mode_keeps_the_page_form() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let cases = [
        changed(&INT_FIELDS, &[("fields", Some("1"))]),
        changed(
            &INT_FIELDS,
            &[
                ("type", Some("ARRAY[1..4] OF INT")),
                ("value_type", Some("ARRAY[1..4] OF INT")),
                ("value", None),
                ("limit_mode", Some("[*]")),
            ],
        ),
    ];
    for pairs in cases {
        let reply = post_page_form(&world, &pairs).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_eq!(reply.header("hx-push-url"), "false", "{}", reply.body);
        assert_eq!(reply.header("hx-retarget"), "", "{}", reply.body);
        assert!(!reply.body.contains("<main"), "{}", reply.body);
        assert!(reply.body.contains("New parameter</h1>"), "{}", reply.body);
        assert!(reply.body.contains(PAGE_FORM_TAG), "{}", reply.body);
        assert!(!reply.body.contains("#parameter-rows"), "{}", reply.body);
        assert!(reply.body.contains("value=\"NEW_P\""), "{}", reply.body);
        assert_clean_html(&reply.body);
    }
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_add_refused_by_the_page_form_swaps_the_page_form_out_of_band() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let cases = [
        ("bad tag", StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008"),
        ("SPEED", StatusCode::CONFLICT, "E-RCP-014"),
    ];
    for (tag, status, code) in cases {
        let reply = post_page_form(&world, &changed(&INT_FIELDS, &[("tag", Some(tag))])).await;
        assert_panel_error(&reply, status, code);
        assert!(
            reply.body.contains(PAGE_OOB_FORM_TAG),
            "{code}: {}",
            reply.body
        );
        assert!(
            !reply.body.contains("#parameter-rows"),
            "{code}: {}",
            reply.body
        );
        assert!(
            !reply.body.contains("New parameter</h2>"),
            "{code}: {}",
            reply.body
        );
    }
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_add_of_the_form_in_the_version_page_keeps_its_attributes() {
    let world = recipe_world();
    let shown = post_htmx(&world, ADD, &changed(&INT_FIELDS, &[("fields", Some("1"))])).await;
    assert_eq!(shown.status, StatusCode::OK, "{}", shown.body);
    assert!(
        shown.body.trim_start().starts_with(FORM_TAG),
        "{}",
        shown.body
    );
    let refused = post_htmx(
        &world,
        ADD,
        &changed(&INT_FIELDS, &[("tag", Some("bad tag"))]),
    )
    .await;
    assert_panel_error(&refused, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert!(refused.body.contains(OOB_FORM_TAG), "{}", refused.body);
    assert!(!refused.body.contains("#content"), "{}", refused.body);
}

#[tokio::test]
async fn parameters_add_of_the_page_form_goes_to_the_version_page() {
    let world = recipe_world();
    let audit = world.audit_count();
    let reply = post_page_form(&world, &INT_FIELDS).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes/R-100/versions/3");
    assert!(world.audit_count() > audit);
}

#[tokio::test]
async fn parameters_add_with_fields_shows_the_rows_of_the_sent_type_and_writes_nothing() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let pairs = changed(
        &INT_FIELDS,
        &[
            ("type", Some("ARRAY[1..8] OF INT")),
            ("value_type", None),
            ("value", None),
            ("row_version", Some("7")),
            ("fields", Some("1")),
        ],
    );
    let reply = post_htmx(&world, ADD, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), FORM_RETARGET);
    assert_eq!(reply.header("hx-reswap"), "outerHTML");
    let body = reply.body.trim_start();
    assert!(
        body.starts_with("<form id=\"parameter-new-form\""),
        "{body}"
    );
    for index in 1..=8 {
        let name = format!("name=\"value[{index}]\"");
        assert!(body.contains(&name), "{name}: {body}");
    }
    assert_eq!(body.matches("class=\"row-all\"").count(), 1, "{body}");
    assert_eq!(field_value(body, "row_version"), "7");
    assert_eq!(field_value(body, "value_type"), "ARRAY[1..8] OF INT");
    assert!(body.contains("value=\"NEW_P\""), "{body}");
    assert_clean_html(body);
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_add_with_limit_mode_copies_the_limit_to_each_element() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let pairs = changed(
        &INT_FIELDS,
        &[
            ("type", Some("ARRAY[1..4] OF INT")),
            ("value_type", Some("ARRAY[1..4] OF INT")),
            ("value", None),
            ("limits[0].leaf_path", Some("[*]")),
            ("limits[0].min", Some("11")),
            ("limits[0].max", Some("99")),
            ("limit_mode", Some("[*]")),
        ],
    );
    let reply = post_htmx(&world, ADD, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), FORM_RETARGET);
    let body = &reply.body;
    assert!(!body.contains("class=\"row-all\""), "{body}");
    assert_eq!(body.matches("value=\"11\"").count(), 4, "{body}");
    assert_eq!(body.matches("value=\"99\"").count(), 4, "{body}");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_add_without_value_type_shows_the_value_fields_with_a_notice() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let pairs = changed(&INT_FIELDS, &[("value_type", None), ("value", None)]);
    let reply = post_htmx(&world, ADD, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-retarget"), FORM_RETARGET);
    assert!(
        reply
            .body
            .contains("The type changed. Enter the values of the new type."),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("id=\"value-0\""), "{}", reply.body);
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_add_answers_the_new_row_with_the_new_row_version() {
    let world = recipe_world();
    let audit = world.audit_count();
    let reply = post_htmx(&world, ADD, &INT_FIELDS).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 2}");
    let row = position(&reply.body, "<tr>");
    let link = position(
        &reply.body,
        "<a href=\"/recipes/R-100/versions/3/parameters/",
    );
    assert!(row < link, "{}", reply.body);
    assert!(
        reply.body.contains(" autofocus>NEW_P</a>"),
        "{}",
        reply.body
    );
    assert!(
        reply.body.contains("Parameter NEW_P is added."),
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .contains("<div id=\"parameter-form\" hx-swap-oob=\"innerHTML\"></div>"),
        "{}",
        reply.body
    );
    assert!(world.audit_count() > audit);
}

#[tokio::test]
async fn parameters_add_wraps_the_out_of_band_table_body_in_a_template() {
    // HTMX 2 parses the answer as table content after the row. A bare `<tbody>` after a `<tr>`
    // is dropped by the HTML parser, so the out-of-band body stands in a `<template>`.
    let world = recipe_world();
    let reply = post_htmx(&world, ADD, &INT_FIELDS).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let wrapped = "</tr><template><tbody id=\"parameter-empty\" hx-swap-oob=\"true\"></tbody></template><div id=\"parameter-form\"";
    assert!(reply.body.contains(wrapped), "{}", reply.body);
    assert_eq!(reply.body.matches("<tbody").count(), 1, "{}", reply.body);
}

#[tokio::test]
async fn parameters_add_of_an_operator_with_a_bad_field_gives_e_web_002() {
    let world = recipe_world();
    let body = form(&changed(&INT_FIELDS, &[("bogus", Some("1"))]));
    assert_operator_refused(&world, ADD, &body).await;
}

#[tokio::test]
async fn parameters_add_without_htmx_goes_to_the_version_page() {
    let world = recipe_world();
    let reply = world
        .post(&world.engineer, ADD, &form(&INT_FIELDS), false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes/R-100/versions/3");
}

#[tokio::test]
async fn parameters_add_in_read_only_mode_gives_e_web_006_also_for_fields_and_limit_mode() {
    let world = recipe_world();
    world.fixture.state.read_only().set(true);
    let before = world.fixture.snapshot();
    let cases = [
        changed(&INT_FIELDS, &[]),
        changed(&INT_FIELDS, &[("fields", Some("1"))]),
        changed(&INT_FIELDS, &[("limit_mode", Some("[*]"))]),
    ];
    for pairs in cases {
        let reply = post_htmx(&world, ADD, &pairs).await;
        assert_error(
            &reply,
            StatusCode::SERVICE_UNAVAILABLE,
            "E-WEB-006",
            "read-only",
        );
    }
    assert_eq!(world.fixture.snapshot(), before);
}
