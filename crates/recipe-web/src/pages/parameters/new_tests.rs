//! Tests of `GET .../parameters/new` (task 11.1).
//!
//! Specs "Form of a new parameter" and "S7 address field per platform", design D6 to D8,
//! ADR-0039 (h) and (i), ADR-0043 (g) and (i).

use axum::http::StatusCode;

use super::test_kit::{FORM_TARGET, NEW_FORM, get_form_fragment, position};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, assert_error, field_value};
use crate::test_support::plant::{ENGLISH, HTMX, main_of};

/// The opening tag of the boosted form of a new parameter of the draft 3 of `R-100`.
pub(super) const FORM_TAG: &str = "<form id=\"parameter-new-form\" method=\"post\" action=\"/recipes/R-100/versions/3/parameters\" hx-boost=\"true\" hx-target=\"#parameter-rows\" hx-swap=\"beforeend show:none focus-scroll:true\" hx-push-url=\"false\" hx-sync=\"closest form:drop\">";
/// The opening tag of the stand-alone form of the page of a new parameter (design D7).
pub(super) const PAGE_FORM_TAG: &str = "<form id=\"parameter-new-form\" method=\"post\" action=\"/recipes/R-100/versions/3/parameters\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\">";

#[tokio::test]
async fn parameters_new_form_fragment_starts_with_the_save_button_and_holds_the_fields() {
    let world = recipe_world();
    let reply = get_form_fragment(&world, NEW_FORM).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = reply.body.trim_start();
    assert!(body.starts_with(FORM_TAG), "{body}");
    assert!(!body.contains("<main"), "{body}");
    assert_eq!(field_value(body, "csrf_token"), world.engineer.csrf);
    assert_eq!(field_value(body, "row_version"), "1");
    let save = position(body, "id=\"parameter-add-submit\"");
    let fields = position(body, "id=\"parameter-show-fields\"");
    let unit = position(body, "id=\"unit_path\"");
    assert!(save < fields && fields < unit, "{body}");
    for id in [
        "tag",
        "name_en",
        "description_en",
        "type",
        "s7_address",
        "ordinal",
    ] {
        assert!(body.contains(&format!("id=\"{id}\"")), "{id}: {body}");
    }
    assert!(
        body.contains("<label for=\"s7_address\">PLC address <span class=\"field-required\">required</span></label>"),
        "{body}"
    );
    assert!(body.contains("<option value=\"INT\">"), "{body}");
    assert!(
        body.contains("Enter the type and then show the value fields."),
        "{body}"
    );
    assert!(!body.contains("name=\"value_type\""), "{body}");
    assert_clean_html(body);
}

#[tokio::test]
async fn parameters_new_form_of_a_twincat_machine_has_no_s7_address() {
    let world = recipe_world();
    let reply = get_form_fragment(&world, "/recipes/R-200/versions/1/parameters/new").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("s7_address"), "{}", reply.body);
}

#[tokio::test]
async fn parameters_new_form_selects_the_unit_of_the_query() {
    let world = recipe_world();
    let path = format!("{NEW_FORM}?unit=U1.3");
    let reply = get_form_fragment(&world, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("<option value=\"U1.3\" selected>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn parameters_new_form_without_htmx_is_a_page_with_a_stand_alone_form() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, NEW_FORM).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("New parameter</h1>"), "{main}");
    assert!(main.contains(PAGE_FORM_TAG), "{main}");
    assert!(!main.contains("#parameter-rows"), "{main}");
    assert!(
        main.contains("href=\"/recipes/R-100/versions/3\""),
        "{main}"
    );
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn parameters_new_form_as_content_has_the_stand_alone_form() {
    let world = recipe_world();
    let reply = world.get_content(&world.engineer, NEW_FORM).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("<main"), "{}", reply.body);
    assert!(reply.body.contains(PAGE_FORM_TAG), "{}", reply.body);
    assert!(!reply.body.contains("#parameter-rows"), "{}", reply.body);
}

#[tokio::test]
async fn parameters_new_form_refuses_an_operator_and_a_version_that_is_not_a_draft() {
    let world = recipe_world();
    let headers = [ENGLISH, HTMX, FORM_TARGET];
    let operator = world.get(&world.operator, NEW_FORM, &headers).await;
    assert_error(&operator, StatusCode::FORBIDDEN, "E-WEB-002", "operator");
    let approved = world
        .get(
            &world.engineer,
            "/recipes/R-100/versions/2/parameters/new",
            &headers,
        )
        .await;
    assert_error(&approved, StatusCode::CONFLICT, "E-RCP-002", "approved");
}

#[tokio::test]
async fn parameters_new_form_works_in_read_only_mode() {
    let world = recipe_world();
    world.fixture.state.read_only().set(true);
    let reply = get_form_fragment(&world, NEW_FORM).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}
