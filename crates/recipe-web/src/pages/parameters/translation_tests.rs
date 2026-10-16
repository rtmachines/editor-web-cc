//! Tests of the translation rows and their routes (task 11.3).
//!
//! Spec "Translation rows", ADR-0040 (a), (b), (f), ADR-0043 (f), `api.md` sections 1 and 2.6.

use axum::http::StatusCode;

use super::test_kit::{assert_panel_error, parameter_path, post_htmx, seed_translation};
use crate::pages::draft_reply::refusal_tests::assert_operator_refused;
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, field_value};
use crate::test_support::plant::{ENGLISH, HTMX, main_of};
use crate::test_support::recipes::{LINE_KEY, SPEED_KEY};

/// The opening tag of the boosted form of the translation `pl` of `SPEED`.
fn pl_form_tag(key: uuid::Uuid) -> String {
    format!(
        "<form id=\"translation-pl-form\" method=\"post\" action=\"{}\" hx-boost=\"true\" hx-target=\"closest .translation-row\" hx-swap=\"outerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\"",
        parameter_path(key, "/translations/pl")
    )
}

#[tokio::test]
async fn parameters_page_shows_one_translation_row_per_tag_of_the_list() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, &parameter_path(LINE_KEY, ""))
        .await;
    let main = main_of(&reply.body);
    assert!(main.contains("<div id=\"translations\">"), "{main}");
    assert!(main.contains(&pl_form_tag(LINE_KEY)), "{main}");
    assert!(main.contains("id=\"translation-pl-name\""), "{main}");
    assert!(main.contains("value=\"Linia\""), "{main}");
    assert!(main.contains("Delete translation pl"), "{main}");
    let speed = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert!(
        speed.body.contains(&pl_form_tag(SPEED_KEY)),
        "{}",
        speed.body
    );
    assert!(
        !speed.body.contains("Delete translation pl"),
        "{}",
        speed.body
    );
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn parameters_translation_save_answers_the_row_with_the_new_row_version() {
    let world = recipe_world();
    let pairs = [
        ("row_version", "1"),
        ("name", "Prędkość"),
        ("description", "Prędkość prasy"),
    ];
    let path = parameter_path(SPEED_KEY, "/translations/pl");
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 2}");
    let body = reply.body.trim_start();
    assert!(
        body.starts_with("<div class=\"translation-row\" id=\"translation-pl\">"),
        "{body}"
    );
    assert!(body.contains("value=\"Prędkość\""), "{body}");
    assert_eq!(field_value(body, "row_version"), "2");
    assert!(body.contains("The translation pl is saved."), "{body}");
    assert!(
        body.contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>"),
        "{body}"
    );
}

#[tokio::test]
async fn parameters_translation_errors_keep_the_url() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let good = [("row_version", "1"), ("name", "Nazwa"), ("description", "")];
    let cases = [
        (
            "/translations/de",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-WEB-008",
        ),
        (
            "/translations/x_y",
            StatusCode::UNPROCESSABLE_ENTITY,
            "E-RCP-027",
        ),
    ];
    for (rest, status, code) in cases {
        let reply = post_htmx(&world, &parameter_path(SPEED_KEY, rest), &good).await;
        assert_panel_error(&reply, status, code);
    }
    let missing = post_htmx(
        &world,
        &parameter_path(uuid::Uuid::from_u128(0xdead), "/translations/pl"),
        &good,
    )
    .await;
    assert_panel_error(&missing, StatusCode::NOT_FOUND, "E-RCP-051");
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn parameters_translation_with_an_empty_name_shows_the_line_under_the_input() {
    let world = recipe_world();
    let pairs = [("row_version", "1"), ("name", ""), ("description", "Opis")];
    let reply = post_htmx(
        &world,
        &parameter_path(SPEED_KEY, "/translations/pl"),
        &pairs,
    )
    .await;
    assert_panel_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    let form = reply.body.split(&pl_form_tag(SPEED_KEY)).nth(1).unwrap();
    assert!(form.starts_with(" hx-swap-oob=\"true\">"), "{form}");
    assert!(form.contains("id=\"translation-pl-name-error\""), "{form}");
    assert!(form.contains("value=\"Opis\""), "{form}");
}

/// K12, ADR-0041 (e): a refusal of valid fields swaps the row with the sent texts and no stale line.
#[tokio::test]
async fn parameters_translation_refused_with_valid_fields_swaps_the_row_without_a_line() {
    let world = recipe_world();
    let missing = uuid::Uuid::from_u128(0xdead);
    let pairs = [("row_version", "1"), ("name", "Nazwa"), ("description", "")];
    let reply = post_htmx(&world, &parameter_path(missing, "/translations/pl"), &pairs).await;
    assert_panel_error(&reply, StatusCode::NOT_FOUND, "E-RCP-051");
    assert!(reply.body.contains("Errors: 1."), "{}", reply.body);
    let form = reply.body.split(&pl_form_tag(missing)).nth(1).unwrap();
    assert!(form.starts_with(" hx-swap-oob=\"true\">"), "{form}");
    assert!(form.contains("value=\"Nazwa\""), "{form}");
    for mark in ["aria-invalid", "field-error"] {
        assert!(!form.contains(mark), "{mark}: {form}");
    }
}

/// K13, SEC-3, ADR-0013: an Operator with a bad field gets `403 E-WEB-002` and no form.
#[tokio::test]
async fn parameters_translation_of_an_operator_with_a_bad_field_gives_e_web_002() {
    let world = recipe_world();
    let upsert = parameter_path(SPEED_KEY, "/translations/pl");
    let remove = parameter_path(LINE_KEY, "/translations/pl/delete");
    let cases = [
        (upsert.as_str(), "row_version=1&name=&description="),
        (upsert.as_str(), "row_version=1&name=A&bogus=1"),
        (remove.as_str(), "row_version=x"),
    ];
    for (path, body) in cases {
        assert_operator_refused(&world, path, body).await;
    }
}

#[tokio::test]
async fn parameters_translation_delete_answers_the_rows_without_the_translation() {
    let world = recipe_world();
    let path = parameter_path(LINE_KEY, "/translations/pl/delete");
    let reply = post_htmx(&world, &path, &[("row_version", "1")]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 2}");
    assert!(
        reply.body.contains("The translation pl is deleted."),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("value=\"Linia\""), "{}", reply.body);
    assert!(
        !reply.body.contains("Delete translation pl"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn parameters_translation_delete_takes_a_tag_outside_the_list() {
    let world = recipe_world();
    seed_translation(&world, LINE_KEY, ("de", 1)).await;
    let page = world
        .get_en(&world.engineer, &parameter_path(LINE_KEY, ""))
        .await;
    assert!(page.body.contains("id=\"translation-de\""), "{}", page.body);
    let path = parameter_path(LINE_KEY, "/translations/de/delete");
    let reply = post_htmx(&world, &path, &[("row_version", "2")]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        !reply.body.contains("id=\"translation-de\""),
        "{}",
        reply.body
    );
}

/// K8, ADR-0043 (f): a row of a tag outside the list shows its texts and only the removal form.
#[tokio::test]
async fn parameters_translation_row_outside_the_list_is_read_only_with_the_removal() {
    let world = recipe_world();
    seed_translation(&world, LINE_KEY, ("de", 1)).await;
    let page = world
        .get_en(&world.engineer, &parameter_path(LINE_KEY, ""))
        .await;
    let row = page
        .body
        .split("<div class=\"translation-row\" id=\"translation-de\">")
        .nth(1)
        .unwrap();
    assert!(row.contains("<dd>Linie</dd>"), "{row}");
    assert!(row.contains("id=\"translation-de-delete\""), "{row}");
    for mark in [
        "translation-de-form",
        "translation-de-name",
        "translation-de-save",
    ] {
        assert!(!row.contains(mark), "{mark}: {row}");
    }
    let operator = world
        .get_en(&world.operator, &parameter_path(LINE_KEY, ""))
        .await;
    assert!(
        operator.body.contains("<dd>Linie</dd>"),
        "{}",
        operator.body
    );
    assert!(
        !operator.body.contains("translation-de-delete"),
        "{}",
        operator.body
    );
}

#[tokio::test]
async fn parameters_translation_delete_of_a_tag_that_the_draft_lacks_gives_e_rcp_050() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/translations/fr/delete");
    let reply = post_htmx(&world, &path, &[("row_version", "1")]).await;
    assert_panel_error(&reply, StatusCode::NOT_FOUND, "E-RCP-050");
}

#[tokio::test]
async fn parameters_translations_answer_the_rows_or_the_page() {
    let world = recipe_world();
    let path = parameter_path(LINE_KEY, "/translations");
    let fragment = world.get(&world.engineer, &path, &[ENGLISH, HTMX]).await;
    assert_eq!(fragment.status, StatusCode::OK, "{}", fragment.body);
    assert!(!fragment.body.contains("<main"), "{}", fragment.body);
    assert!(
        fragment.body.contains("id=\"translation-pl\""),
        "{}",
        fragment.body
    );
    let page = world.get_en(&world.operator, &path).await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.body);
    let main = main_of(&page.body);
    assert!(main.contains("Parameter LINE</h1>"), "{main}");
    assert!(!main.contains("<form"), "{main}");
    assert!(main.contains("Linia"), "{main}");
}
