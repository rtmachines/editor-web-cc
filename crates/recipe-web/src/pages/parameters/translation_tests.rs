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
    assert!(form.contains(">\nOpis</textarea>"), "{form}");
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

/// The opening tag of the boosted delete form of the translation `lang` of `key` (ADR-0111 (a)).
fn delete_form_tag(key: uuid::Uuid, lang: &str) -> String {
    format!(
        "<form method=\"post\" action=\"{}\" hx-boost=\"true\" hx-target=\"closest .translation-row\" hx-swap=\"outerHTML\" hx-push-url=\"false\" hx-sync=\"closest form:drop\">",
        parameter_path(key, &format!("/translations/{lang}/delete"))
    )
}

/// ADR-0111 (a): the delete form swaps only its own row.
#[tokio::test]
async fn parameters_translation_delete_form_targets_its_own_row() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, &parameter_path(LINE_KEY, ""))
        .await;
    let tag = delete_form_tag(LINE_KEY, "pl");
    assert!(reply.body.contains(&tag), "{tag}: {}", reply.body);
    assert!(
        !reply.body.contains("hx-target=\"#translations\""),
        "{}",
        reply.body
    );
}

/// ADR-0111 (b), (c), (d), (e): the answer is the row of the deleted tag with an empty form.
#[tokio::test]
async fn parameters_translation_delete_answers_one_row_with_an_empty_form() {
    let world = recipe_world();
    let path = parameter_path(LINE_KEY, "/translations/pl/delete");
    let reply = post_htmx(&world, &path, &[("row_version", "1")]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 2}");
    let body = reply.body.trim_start();
    assert!(
        body.starts_with("<div class=\"translation-row\" id=\"translation-pl\">"),
        "{body}"
    );
    assert_eq!(
        body.matches("class=\"translation-row\"").count(),
        1,
        "{body}"
    );
    assert!(
        body.contains("<h3 tabindex=\"-1\" autofocus>pl</h3>"),
        "{body}"
    );
    assert!(body.contains(&pl_form_tag(LINE_KEY)), "{body}");
    assert!(!body.contains("value=\"Linia\""), "{body}");
    assert_eq!(field_value(body, "row_version"), "2");
    // The last stored translation is gone, so the row holds no delete disclosure.
    for mark in ["<details", "translation-pl-delete"] {
        assert!(!body.contains(mark), "{mark}: {body}");
    }
    assert!(
        body.contains("<div id=\"toast\" hx-swap-oob=\"innerHTML\">"),
        "{body}"
    );
    assert!(body.contains("The translation pl is deleted."), "{body}");
    assert!(
        body.contains("<div id=\"error-panel\" hx-swap-oob=\"innerHTML\"></div>"),
        "{body}"
    );
}

/// ADR-0111 (b): the other rows stay out of the answer, so their unsaved input stays.
#[tokio::test]
async fn parameters_translation_delete_leaves_the_other_rows_out_of_the_answer() {
    let world = recipe_world();
    seed_translation(&world, LINE_KEY, ("de", 1)).await;
    let path = parameter_path(LINE_KEY, "/translations/de/delete");
    let reply = post_htmx(&world, &path, &[("row_version", "2")]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 3}");
    let body = reply.body.trim_start();
    assert!(!body.contains("id=\"translation-pl\""), "{body}");
    assert!(!body.contains("Linia"), "{body}");
    assert_eq!(
        body.matches("class=\"translation-row\"").count(),
        1,
        "{body}"
    );
}

/// ADR-0111 (b): a tag outside the list gets its row with `translation-none` and no form.
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
    let body = reply.body.trim_start();
    assert!(
        body.starts_with("<div class=\"translation-row\" id=\"translation-de\">"),
        "{body}"
    );
    assert!(
        body.contains("<h3 tabindex=\"-1\" autofocus>de</h3>"),
        "{body}"
    );
    assert!(body.contains("<p>No translation</p>"), "{body}");
    for mark in ["<form id=", "<details", "translation-de-delete"] {
        assert!(!body.contains(mark), "{mark}: {body}");
    }
    assert!(body.contains("The translation de is deleted."), "{body}");
}

/// ADR-0111 (f): a `409` shows the conflict fragment and swaps no row.
#[tokio::test]
async fn parameters_translation_delete_with_a_stale_row_version_swaps_no_row() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let path = parameter_path(LINE_KEY, "/translations/pl/delete");
    let reply = post_htmx(&world, &path, &[("row_version", "7")]).await;
    assert_panel_error(&reply, StatusCode::CONFLICT, "E-WEB-001");
    assert!(!reply.body.contains("translation-row"), "{}", reply.body);
    assert_eq!(world.fixture.snapshot(), before);
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
    assert!(row.contains("<dd lang=\"de\">Linie</dd>"), "{row}");
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
        operator.body.contains("<dd lang=\"de\">Linie</dd>"),
        "{}",
        operator.body
    );
    assert!(
        !operator.body.contains("translation-de-delete"),
        "{}",
        operator.body
    );
}

/// Design D6, WCAG 3.1.2: the inputs and the stored texts of a row carry the `lang` of the row.
/// The labels and the terms are texts of the UI language and carry none.
#[tokio::test]
async fn parameters_translation_row_marks_its_texts_with_the_language_of_the_row() {
    let world = recipe_world();
    seed_translation(&world, LINE_KEY, ("de", 1)).await;
    let page = world
        .get_en(&world.engineer, &parameter_path(LINE_KEY, ""))
        .await;
    let pl_row = page
        .body
        .split("<div class=\"translation-row\" id=\"translation-pl\">")
        .nth(1)
        .and_then(|rest| rest.split("<div class=\"translation-row\"").next())
        .unwrap();
    assert!(
        pl_row.contains("<input id=\"translation-pl-name\" name=\"name\" type=\"text\" maxlength=\"200\" autocomplete=\"off\" value=\"Linia\" lang=\"pl\""),
        "{pl_row}"
    );
    // ADR-0112: the description is a textarea. One LF follows the start tag.
    assert!(
        pl_row.contains("<textarea id=\"translation-pl-description\" name=\"description\" rows=\"4\" maxlength=\"4000\" lang=\"pl\""),
        "{pl_row}"
    );
    assert!(
        pl_row.contains(">\nStrefy linii prasy</textarea>"),
        "{pl_row}"
    );
    assert!(
        !pl_row.contains("<input id=\"translation-pl-description\""),
        "{pl_row}"
    );
    assert!(
        pl_row.contains("<label for=\"translation-pl-name\">"),
        "{pl_row}"
    );
    assert!(
        pl_row.contains("<label for=\"translation-pl-description\">"),
        "{pl_row}"
    );
    let de_row = page
        .body
        .split("<div class=\"translation-row\" id=\"translation-de\">")
        .nth(1)
        .and_then(|rest| rest.split("<details").next())
        .unwrap();
    assert!(de_row.contains("<dd lang=\"de\">Linie</dd>"), "{de_row}");
    assert_eq!(de_row.matches("<dd lang=\"de\">").count(), 1, "{de_row}");
    // ADR-0112 (h): the description keeps its line ends on a read-only row.
    assert_eq!(
        de_row
            .matches("<dd lang=\"de\" class=\"text-lines\">")
            .count(),
        1,
        "{de_row}"
    );
    assert!(!de_row.contains("<dt lang"), "{de_row}");
    let operator = world
        .get_en(&world.operator, &parameter_path(LINE_KEY, ""))
        .await;
    assert!(
        operator.body.contains("<dd lang=\"pl\">Linia</dd>"),
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
    assert!(
        main.contains("Parameter LINE, version 3 of recipe R-100</h1>"),
        "{main}"
    );
    assert!(!main.contains("<form"), "{main}");
    assert!(main.contains("Linia"), "{main}");
}

/// ADR-0112 (g): a save with CRLF stores LF, and the row shows the stored text in a `textarea`.
#[tokio::test]
async fn parameters_translation_save_with_crlf_stores_lf() {
    let world = recipe_world();
    let pairs = [
        ("row_version", "1"),
        ("name", "Prędkość"),
        ("description", "Linia 1\r\nLinia 2"),
    ];
    let path = parameter_path(SPEED_KEY, "/translations/pl");
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = reply.body.trim_start();
    assert!(body.contains(">\nLinia 1\nLinia 2</textarea>"), "{body}");
    assert!(!body.contains('\r'), "{body}");
    let page = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert!(
        page.body.contains(">\nLinia 1\nLinia 2</textarea>"),
        "{}",
        page.body
    );
    assert!(!page.body.contains("Linia 1\r"), "{}", page.body);
}

/// ADR-0158 (k), design D4: the save form of a translation row holds the indicator of
/// `form-in-progress` after its save button. The delete form of the row has none.
#[tokio::test]
async fn parameters_translation_row_has_the_indicator_in_the_save_form_only() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, &parameter_path(LINE_KEY, ""))
        .await;
    let main = main_of(&reply.body);
    let indicator = "<span class=\"htmx-indicator\">The server processes the request.</span>";
    let save_form = main
        .split("<form id=\"translation-pl-form\"")
        .nth(1)
        .and_then(|rest| rest.split("</form>").next())
        .unwrap();
    assert_eq!(save_form.matches(indicator).count(), 1, "{save_form}");
    let save = save_form.find("id=\"translation-pl-save\"").unwrap();
    assert!(save < save_form.find(indicator).unwrap(), "{save_form}");
    let delete_button = main.find("id=\"translation-pl-delete\"").unwrap();
    let delete_start = main[..delete_button].rfind("<form").unwrap();
    let delete_end = delete_button + main[delete_button..].find("</form>").unwrap();
    let delete_form = &main[delete_start..delete_end];
    assert!(
        delete_form.starts_with("<form method=\"post\""),
        "{delete_form}"
    );
    assert!(!delete_form.contains("htmx-indicator"), "{delete_form}");
}
