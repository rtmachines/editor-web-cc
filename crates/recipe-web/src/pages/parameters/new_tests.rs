//! Tests of `GET .../parameters/new` (task 11.1).
//!
//! Specs "Form of a new parameter" and "S7 address field per platform", design D6 to D8,
//! ADR-0039 (h) and (i), ADR-0043 (g) and (i).

use axum::http::StatusCode;

use super::test_kit::{
    ADD, FORM_TARGET, INT_FIELDS, NEW_FORM, changed, form, get_form_fragment, position,
};
use crate::i18n::Lang;
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, assert_error, field_value};
use crate::test_support::plant::{ENGLISH, HTMX, POLISH, TARGET_CONTENT, main_of};
use crate::test_support::text;

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

/// Guard of the change `ui-form-fixes`, design D1: the form that the link of the second page of
/// the version page loads stays the stand-alone page form, and its save goes to the version page.
///
/// The link sends `HX-Request: true` and `HX-Target: content`. The test passes before the change.
#[tokio::test]
async fn parameters_new_form_of_a_content_request_saves_with_303_to_the_version_page() {
    let world = recipe_world();
    let reply = world
        .get(&world.engineer, NEW_FORM, &[ENGLISH, HTMX, TARGET_CONTENT])
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains(PAGE_FORM_TAG), "{}", reply.body);
    assert!(
        !reply.body.contains("id=\"parameter-rows\""),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("#parameter-rows"), "{}", reply.body);
    let row_version = field_value(&reply.body, "row_version");
    let pairs = changed(&INT_FIELDS, &[("row_version", Some(row_version.as_str()))]);
    let audit = world.audit_count();
    let saved = world
        .post_with(&world.engineer, ADD, &form(&pairs), &[HTMX, TARGET_CONTENT])
        .await;
    assert_eq!(saved.status, StatusCode::SEE_OTHER, "{}", saved.body);
    assert_eq!(saved.header("location"), "/recipes/R-100/versions/3");
    assert_eq!(saved.header("hx-redirect"), "", "{:?}", saved.headers);
    assert_eq!(saved.header("hx-location"), "", "{:?}", saved.headers);
    assert!(world.audit_count() > audit);
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

/// Spec "Read-only pages", ADR-0113 (d): in read-only mode the form route answers `200` with the
/// notice in place of the form, as a page, as `HX-Target: content`, and as `HX-Target:
/// parameter-form`.
#[tokio::test]
async fn parameters_new_form_in_read_only_mode_shows_the_notice_in_place_of_the_form() {
    let world = recipe_world();
    world.fixture.state.read_only().set(true);
    for (lang, header) in [(Lang::En, ENGLISH), (Lang::Pl, POLISH)] {
        let notice = text(lang, "readonly-form-unavailable");
        let page = world.get(&world.engineer, NEW_FORM, &[header]).await;
        let content = world
            .get(
                &world.engineer,
                NEW_FORM,
                &[header, HTMX, ("hx-target", "content")],
            )
            .await;
        let fragment = world
            .get(&world.engineer, NEW_FORM, &[header, HTMX, FORM_TARGET])
            .await;
        for (kind, reply) in [
            ("page", &page),
            ("content", &content),
            ("fragment", &fragment),
        ] {
            let body = &reply.body;
            assert_eq!(reply.status, StatusCode::OK, "{lang} {kind}: {body}");
            assert!(body.contains(&notice), "{lang} {kind}: {body}");
            for forbidden in [
                "id=\"parameter-new-form\"",
                "id=\"parameter-add-submit\"",
                "action=\"/recipes/R-100/versions/3/parameters\"",
            ] {
                assert!(
                    !body.contains(forbidden),
                    "{lang} {kind} {forbidden}: {body}"
                );
            }
        }
        assert!(page.body.contains("<main"), "{lang}: {}", page.body);
        assert!(
            !fragment.body.contains("<main"),
            "{lang}: {}",
            fragment.body
        );
    }
}

/// Regression, ADR-0113 (d): the permission check runs before the notice, so an Operator gets
/// `403` in read-only mode.
#[tokio::test]
async fn parameters_new_form_in_read_only_mode_still_refuses_an_operator() {
    let world = recipe_world();
    world.fixture.state.read_only().set(true);
    let reply = world
        .get(&world.operator, NEW_FORM, &[ENGLISH, HTMX, FORM_TARGET])
        .await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", "operator");
}

/// ADR-0158 (k), design D4: the new form of a parameter holds the indicator of `form-in-progress`
/// after its save button.
#[tokio::test]
async fn parameters_new_form_has_the_indicator_after_the_save_button() {
    let world = recipe_world();
    let reply = get_form_fragment(&world, NEW_FORM).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = reply.body.as_str();
    let indicator = "<span class=\"htmx-indicator\">The server processes the request.</span>";
    assert_eq!(body.matches(indicator).count(), 1, "{body}");
    let save = position(body, "id=\"parameter-add-submit\"");
    assert!(save < position(body, indicator), "{body}");
    assert!(
        position(body, indicator) < position(body, "</form>"),
        "{body}"
    );
}
