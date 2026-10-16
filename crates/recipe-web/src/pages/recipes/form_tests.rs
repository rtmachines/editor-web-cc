//! Tests of `GET /recipes/new` and `POST /recipes` (task 9.2).
//!
//! Spec "Recipe form and creation", ADR-0041 (a) and (e), design D6 and D7.

use axum::http::StatusCode;

use super::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, assert_error, assert_htmx_error};
use crate::test_support::plant::{ENGLISH, POLISH, main_of};

/// The opening tag of the form of a new recipe on the page.
const FORM_TAG: &str = "<form id=\"recipe-new-form\" method=\"post\" action=\"/recipes\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\">";
/// The opening tag of the same form when an answer swaps it out of band.
const OOB_FORM_TAG: &str = "<form id=\"recipe-new-form\" method=\"post\" action=\"/recipes\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\" hx-swap-oob=\"true\">";

#[tokio::test]
async fn recipes_new_form_offers_the_live_machines_and_the_four_fields() {
    let world = recipe_world();
    let reply = world.get_en(&world.engineer, "/recipes/new").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains(FORM_TAG), "{main}");
    assert!(
        main.contains("<select id=\"machine_code\" name=\"machine_code\" aria-required=\"true\">"),
        "{main}"
    );
    assert!(
        main.contains("<option value=\"\" selected>Choose a machine</option>"),
        "{main}"
    );
    assert!(
        main.contains("<option value=\"M7\">M7 Press line</option>"),
        "{main}"
    );
    assert!(
        main.contains("<option value=\"M20\">M20 Packing line</option>"),
        "{main}"
    );
    assert!(!main.contains("M12"), "{main}");
    assert!(
        main.contains(
            "<input id=\"reference_number\" name=\"reference_number\" type=\"text\" class=\"data\""
        ),
        "{main}"
    );
    assert!(
        main.contains("<input id=\"recipe_name\" name=\"name\" type=\"text\""),
        "{main}"
    );
    assert!(
        main.contains("<input id=\"change_note\" name=\"change_note\" type=\"text\""),
        "{main}"
    );
    let token = format!(
        "<input type=\"hidden\" name=\"csrf_token\" value=\"{}\">",
        world.engineer.csrf
    );
    assert!(main.contains(&token), "{main}");
    assert!(
        main.contains("<button type=\"submit\" id=\"recipe-create-submit\""),
        "{main}"
    );
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn recipes_new_form_empty_machine_option_names_the_choice_in_both_languages() {
    let world = recipe_world();
    for (cookie, word) in [(ENGLISH, "Choose a machine"), (POLISH, "Wybierz maszynę")] {
        let reply = world.get(&world.engineer, "/recipes/new", &[cookie]).await;
        let option = format!("<option value=\"\" selected>{word}</option>");
        assert!(main_of(&reply.body).contains(&option), "{}", reply.body);
    }
}

#[tokio::test]
async fn recipes_new_form_of_an_operator_gives_e_web_002() {
    let world = recipe_world();
    let reply = world.get_en(&world.operator, "/recipes/new").await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", "form");
}

#[tokio::test]
async fn recipes_create_folds_the_reference_and_goes_to_version_1() {
    let world = recipe_world();
    let body = "machine_code=M7&reference_number=r-500&name=New+recipe&change_note=First+version";
    let reply = world.post(&world.engineer, "/recipes", body, false).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes/R-500/versions/1");
    let list = world.get_en(&world.engineer, "/recipes").await;
    assert!(
        list.body.contains("<a href=\"/recipes/R-500\">R-500</a>"),
        "{}",
        list.body
    );
}

#[tokio::test]
async fn recipes_create_lists_every_bad_field_and_swaps_the_form_with_the_sent_text() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let body = "machine_code=&reference_number=-bad%3C&name=&change_note=";
    let reply = world.post(&world.engineer, "/recipes", body, true).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(reply.header("hx-push-url"), "false");
    assert!(reply.body.contains("Errors: 4."), "{}", reply.body);
    let form = reply.body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert!(form.contains("value=\"-bad&#60;\""), "{form}");
    assert_eq!(form.matches("aria-invalid=\"true\"").count(), 4, "{form}");
    for id in [
        "machine_code",
        "reference_number",
        "recipe_name",
        "change_note",
    ] {
        let line = format!("<div class=\"field-error\" id=\"{id}-error\">");
        assert!(form.contains(&line), "{id}: {form}");
    }
    assert_eq!(world.fixture.snapshot(), before);
}

/// Assert that the out-of-band form holds one line, under `id`, with `code` (ADR-0041 (e)).
fn assert_one_line_under(body: &str, id: &str, code: &str) {
    assert!(body.contains("Errors: 1."), "{code}: {body}");
    assert!(body.contains(&format!("href=\"#{id}\"")), "{code}: {body}");
    let form = body.split(OOB_FORM_TAG).nth(1).unwrap();
    assert!(form.contains("value=\"Note\""), "{form}");
    assert_eq!(form.matches("aria-invalid=\"true\"").count(), 1, "{form}");
    assert_eq!(form.matches("class=\"field-error\"").count(), 1, "{form}");
    let line = format!("<div class=\"field-error\" id=\"{id}-error\">");
    let (_input, below) = form.split_once(&line).unwrap();
    assert!(below.contains(code), "{code}: {form}");
}

/// K12: the refused use case swaps the form with its line under the machine list.
#[tokio::test]
async fn recipes_create_with_an_unknown_machine_gives_e_plt_019() {
    let world = recipe_world();
    let body = "machine_code=M99&reference_number=R-501&name=New&change_note=Note";
    let reply = world.post(&world.engineer, "/recipes", body, true).await;
    assert_htmx_error(&reply, StatusCode::NOT_FOUND, "E-PLT-019");
    assert_one_line_under(&reply.body, "machine_code", "E-PLT-019");
}

/// K12: the refused use case swaps the form with its line under the reference number.
#[tokio::test]
async fn recipes_create_with_a_used_reference_gives_e_rcp_001() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let body = "machine_code=M7&reference_number=R-100&name=Copy&change_note=Note";
    let reply = world.post(&world.engineer, "/recipes", body, true).await;
    assert_htmx_error(&reply, StatusCode::CONFLICT, "E-RCP-001");
    assert_one_line_under(&reply.body, "reference_number", "E-RCP-001");
    assert!(reply.body.contains("value=\"R-100\""), "{}", reply.body);
    assert_clean_html(&reply.body);
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn recipes_create_of_an_operator_gives_e_web_002() {
    let world = recipe_world();
    let body = "machine_code=M7&reference_number=R-502&name=New&change_note=Note";
    let reply = world.post(&world.operator, "/recipes", body, true).await;
    assert_htmx_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002");
}
