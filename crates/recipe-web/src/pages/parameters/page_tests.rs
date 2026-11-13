//! Tests of `GET .../parameters/{key}`: the parameter page (task 11.2).
//!
//! Specs "One edit form of a parameter" and "Leaf grid", design D7 and D8, ADR-0039 (i),
//! ADR-0043 (k).

use axum::http::StatusCode;

use super::test_kit::{
    SPEED_FIELDS, changed, parameter_path, position, post_htmx, seed_translation,
};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{
    assert_clean_html, assert_error, assert_ids_unique_and_references_resolve, attribute_values,
    field_value,
};
use crate::test_support::plant::{ENGLISH, HTMX, POLISH, main_of};
use crate::test_support::recipes::{HOLD_KEY, LINE_A_KEY, SPEED_KEY};

// --- Read-only mode (ADR-0113 (b), (d)) ---------------------------------------------------------

/// Spec "Read-only pages", ADR-0113: in read-only mode the page of a draft parameter shows the
/// read-only grid and the translation texts, and no form of the command.
#[tokio::test]
async fn parameters_page_of_a_draft_in_read_only_mode_shows_the_grid_and_the_translations() {
    let world = recipe_world();
    seed_translation(&world, SPEED_KEY, ("de", 1)).await;
    world.fixture.state.read_only().set(true);
    for session in [&world.admin, &world.engineer] {
        let reply = world.get_en(session, &parameter_path(SPEED_KEY, "")).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        assert!(main.contains("<td class=\"data\">1200</td>"), "{main}");
        assert!(main.contains("id=\"translation-de\""), "{main}");
        assert!(!main.contains("parameter-edit-form"), "{main}");
        assert!(!main.contains("parameter-delete-submit"), "{main}");
        assert!(!main.contains("translation-de-delete"), "{main}");
        assert_clean_html(&reply.body);
    }
}

/// ADR-0113 (b): the page holds the forms of the layout only, the logout and the language switch.
#[tokio::test]
async fn parameters_page_in_read_only_mode_holds_only_the_logout_and_language_forms() {
    let world = recipe_world();
    world.fixture.state.read_only().set(true);
    let reply = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = reply.body.as_str();
    assert_eq!(body.matches("<form").count(), 2, "{body}");
    assert_eq!(body.matches("method=\"post\"").count(), 2, "{body}");
    assert!(body.contains("action=\"/logout\""), "{body}");
    assert!(body.contains("action=\"/lang\""), "{body}");
}

/// ADR-0113 (b): `GET .../translations` holds no form in read-only mode.
#[tokio::test]
async fn parameters_translations_in_read_only_mode_hold_no_form() {
    let world = recipe_world();
    seed_translation(&world, SPEED_KEY, ("de", 1)).await;
    world.fixture.state.read_only().set(true);
    let path = parameter_path(SPEED_KEY, "/translations");
    let reply = world.get(&world.engineer, &path, &[ENGLISH, HTMX]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("id=\"translation-de\""),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("<form"), "{}", reply.body);
}

#[tokio::test]
async fn parameters_page_of_a_draft_holds_one_edit_form_with_the_stored_value() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains("Parameter SPEED, version 3 of recipe R-100</h1>"),
        "{main}"
    );
    assert!(main.contains("<div id=\"parameter-summary\">"), "{main}");
    let tag = format!(
        "<form id=\"parameter-edit-form\" method=\"post\" action=\"{}\" hx-boost=\"true\" hx-target=\"#parameter-summary\" hx-swap=\"innerHTML show:none focus-scroll:true\" hx-push-url=\"false\" hx-sync=\"closest form:drop\">",
        parameter_path(SPEED_KEY, "/definition")
    );
    assert_eq!(main.matches("<form id=\"parameter-").count(), 1, "{main}");
    let form = main.split(&tag).nth(1).unwrap();
    let save = position(form, "id=\"parameter-save-submit\"");
    assert!(save < position(form, "id=\"unit_path\""), "{form}");
    assert!(form.contains("id=\"value-0\" name=\"value\""), "{form}");
    assert!(form.contains("value=\"1200\""), "{form}");
    assert!(form.contains("value=\"3000\""), "{form}");
    assert_eq!(field_value(form, "value_type"), "INT");
    assert_eq!(field_value(form, "row_version"), "1");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn parameters_page_shows_the_leaves_of_a_struct_in_a_struct_and_a_time_literal() {
    let world = recipe_world();
    let line = world
        .get_en(&world.engineer, &parameter_path(LINE_A_KEY, ""))
        .await;
    for name in ["value.head.a", "value.head.b", "value.count"] {
        let field = format!("name=\"{name}\"");
        assert!(line.body.contains(&field), "{name}: {}", line.body);
    }
    let hold = world
        .get_en(&world.engineer, &parameter_path(HOLD_KEY, ""))
        .await;
    assert!(hold.body.contains("value=\"T#1m30s\""), "{}", hold.body);
}

#[tokio::test]
async fn parameters_page_of_an_operator_shows_the_grid_without_a_form() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.operator, &parameter_path(SPEED_KEY, ""))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(!main.contains("<form"), "{main}");
    assert!(main.contains("<td class=\"data\">1200</td>"), "{main}");
    assert!(main.contains("<div id=\"parameter-summary\">"), "{main}");
}

#[tokio::test]
async fn parameters_page_of_a_missing_or_malformed_key_gives_e_web_005() {
    let world = recipe_world();
    let unknown = parameter_path(uuid::Uuid::from_u128(0xdead), "");
    for path in [unknown.as_str(), "/recipes/R-100/versions/3/parameters/x"] {
        let reply = world.get_en(&world.engineer, path).await;
        assert_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005", path);
    }
}

/// Return the text of the `<div id="...">` block `id` up to its first closing tag.
fn block_of<'a>(body: &'a str, id: &str) -> &'a str {
    let open = format!("<div id=\"{id}\">");
    let start = body.split(&open).nth(1).unwrap();
    start.split("</div>").next().unwrap()
}

#[tokio::test]
async fn parameters_page_marks_the_english_definition_texts_with_lang_en_in_both_ui_languages() {
    let world = recipe_world();
    for (ui_lang, header) in [("pl", POLISH), ("en", ENGLISH)] {
        let reply = world
            .get(&world.engineer, &parameter_path(SPEED_KEY, ""), &[header])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{ui_lang}: {}", reply.body);
        for id in ["parameter-summary", "translations"] {
            let block = block_of(&reply.body, id);
            assert!(
                block.contains("<dd lang=\"en\">SPEED</dd>"),
                "{ui_lang} {id}: {block}"
            );
            assert!(
                block.contains(
                    "<dd lang=\"en\" class=\"text-lines\">A parameter of the press line</dd>"
                ),
                "{ui_lang} {id}: {block}"
            );
        }
    }
}

/// Store the description `Line 1`, a line end, and `Line 2` for `SPEED` and return the page of it.
async fn page_with_two_line_description(world: &crate::test_support::plant::World) -> String {
    let pairs = changed(
        &SPEED_FIELDS,
        &[("description_en", Some("Line 1\r\nLine 2"))],
    );
    let path = parameter_path(SPEED_KEY, "/definition");
    let saved = post_htmx(world, &path, &pairs).await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    let page = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.body);
    page.body
}

/// ADR-0112 (a), (d): the edit form holds a `textarea` with the stored line end.
#[tokio::test]
async fn parameters_page_edit_form_holds_a_textarea_description_with_the_stored_line_end() {
    let world = recipe_world();
    let body = page_with_two_line_description(&world).await;
    let form = main_of(&body)
        .split("<form id=\"parameter-edit-form\"")
        .nth(1)
        .unwrap();
    let start =
        "<textarea id=\"description_en\" name=\"description_en\" rows=\"4\" maxlength=\"4000\"";
    assert!(form.contains(start), "{form}");
    assert!(form.contains(">\nLine 1\nLine 2</textarea>"), "{form}");
    assert!(!form.contains("<input id=\"description_en\""), "{form}");
}

/// ADR-0112 (h): the summary and the English base text keep the line end and use `text-lines`.
#[tokio::test]
async fn parameters_page_shows_the_description_in_text_lines_elements() {
    let world = recipe_world();
    let body = page_with_two_line_description(&world).await;
    let shown = "<dd lang=\"en\" class=\"text-lines\">Line 1\nLine 2</dd>";
    for id in ["parameter-summary", "translations"] {
        let block = block_of(&body, id);
        assert!(block.contains(shown), "{id}: {block}");
    }
}

/// Return the text between `<h1 ...>` and `</h1>`.
fn h1_of(body: &str) -> &str {
    let start = body.split("<h1").nth(1).unwrap();
    let text = start.split_once('>').unwrap().1;
    text.split("</h1>").next().unwrap()
}

/// Return the text of the `<title>` element.
fn title_of(body: &str) -> &str {
    body.split("<title>")
        .nth(1)
        .unwrap()
        .split("</title>")
        .next()
        .unwrap()
}

#[tokio::test]
async fn parameters_page_heading_and_title_name_the_tag_the_recipe_and_the_version() {
    let world = recipe_world();
    let expected = [
        (POLISH, "Parametr SPEED, wersja 3 receptury R-100"),
        (ENGLISH, "Parameter SPEED, version 3 of recipe R-100"),
    ];
    for (header, heading) in expected {
        let reply = world
            .get(&world.engineer, &parameter_path(SPEED_KEY, ""), &[header])
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_eq!(h1_of(&reply.body), heading, "{}", reply.body);
        assert!(title_of(&reply.body).starts_with(heading), "{}", reply.body);
        assert!(!reply.body.contains("parameter-title"), "{}", reply.body);
    }
}

#[tokio::test]
async fn parameters_page_names_each_danger_notice_in_its_delete_button() {
    let world = recipe_world();
    seed_translation(&world, SPEED_KEY, ("de", 1)).await;
    seed_translation(&world, SPEED_KEY, ("fr", 2)).await;
    let reply = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = reply.body.as_str();
    let pairs = [
        (
            "<button type=\"submit\" id=\"parameter-delete-submit\" class=\"btn-danger\" aria-describedby=\"danger-notice-parameter\">",
            "<div id=\"danger-notice-parameter\">",
        ),
        (
            "<button type=\"submit\" id=\"translation-de-delete\" class=\"btn-danger\" aria-describedby=\"danger-notice-translation-de\">",
            "<div id=\"danger-notice-translation-de\">",
        ),
        (
            "<button type=\"submit\" id=\"translation-fr-delete\" class=\"btn-danger\" aria-describedby=\"danger-notice-translation-fr\">",
            "<div id=\"danger-notice-translation-fr\">",
        ),
    ];
    for (button, notice) in pairs {
        assert!(body.contains(button), "{button}: {body}");
        assert!(body.contains(notice), "{notice}: {body}");
    }
}

#[tokio::test]
async fn parameters_page_has_unique_ids_and_resolved_aria_describedby_references() {
    let world = recipe_world();
    seed_translation(&world, SPEED_KEY, ("de", 1)).await;
    seed_translation(&world, SPEED_KEY, ("fr", 2)).await;
    let reply = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let body = reply.body.as_str();
    for id in [
        "danger-notice-translation-de",
        "danger-notice-translation-fr",
    ] {
        let attribute = format!(" id=\"{id}\"");
        assert_eq!(body.matches(attribute.as_str()).count(), 1, "{id}: {body}");
    }
    assert!(!attribute_values(body, "aria-describedby").is_empty());
    assert_ids_unique_and_references_resolve(body);
}

#[tokio::test]
async fn parameters_page_repeats_the_version_link_right_after_the_heading() {
    let world = recipe_world();
    let reply = world
        .get(&world.engineer, &parameter_path(SPEED_KEY, ""), &[ENGLISH])
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let link = "<p><a class=\"target-link\" href=\"/recipes/R-100/versions/3\">Version 3 of recipe R-100</a></p>";
    let after_heading = reply.body.split_once("</h1>").unwrap().1;
    assert!(
        after_heading.starts_with(&format!("\n{link}\n")),
        "{after_heading}"
    );
}

// --- Language tag of the English base inputs (ADR-0126 (f), design D4) ---------------------------

/// Return the start tag that begins with `open`, up to and including the first `>`.
fn start_tag(form: &str, open: &str) -> String {
    let (_before, rest) = form.split_once(open).unwrap();
    let (attributes, _after) = rest.split_once('>').unwrap();
    format!("{open}{attributes}>")
}

/// Assert that `form` tags the two English base inputs with `lang="en"` and the labels with no `lang`.
fn assert_english_base_inputs_are_tagged(form: &str, context: &str) {
    let input = start_tag(form, "<input id=\"name_en\"");
    assert!(input.contains(" lang=\"en\""), "{context}: {input}");
    let area = start_tag(form, "<textarea id=\"description_en\"");
    assert!(area.contains(" lang=\"en\""), "{context}: {area}");
    for id in ["name_en", "description_en"] {
        let label = format!("<label for=\"{id}\">");
        assert!(form.contains(&label), "{context} {id}: {form}");
    }
}

/// ADR-0126 (f): in the `pl` UI the edit form of a parameter tags the input `name_en` and the text
/// area `description_en` with `lang="en"`. The labels stay in the UI language.
#[tokio::test]
async fn parameters_page_edit_form_tags_the_english_base_inputs_with_lang_en_in_pl() {
    let world = recipe_world();
    let reply = world
        .get(&world.engineer, &parameter_path(SPEED_KEY, ""), &[POLISH])
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let form = main_of(&reply.body)
        .split("<form id=\"parameter-edit-form\"")
        .nth(1)
        .unwrap();
    assert_english_base_inputs_are_tagged(form, "edit");
}

/// ADR-0126 (f): in the `pl` UI the form of a new parameter tags the input `name_en` and the text
/// area `description_en` with `lang="en"`. The labels stay in the UI language.
#[tokio::test]
async fn parameters_new_form_tags_the_english_base_inputs_with_lang_en_in_pl() {
    let world = recipe_world();
    let reply = world
        .get(
            &world.engineer,
            super::test_kit::NEW_FORM,
            &[POLISH, HTMX, super::test_kit::FORM_TARGET],
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_english_base_inputs_are_tagged(&reply.body, "new");
}

/// ADR-0158 (k), design D4: the edit form of a parameter holds the indicator of
/// `form-in-progress` after its save button.
#[tokio::test]
async fn parameters_page_edit_form_has_the_indicator_after_the_save_button() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, &parameter_path(SPEED_KEY, ""))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let form = main_of(&reply.body)
        .split("<form id=\"parameter-edit-form\"")
        .nth(1)
        .and_then(|rest| rest.split("</form>").next())
        .unwrap();
    let indicator = "<span class=\"htmx-indicator\">The server processes the request.</span>";
    assert_eq!(form.matches(indicator).count(), 1, "{form}");
    let save = position(form, "id=\"parameter-save-submit\"");
    assert!(save < position(form, indicator), "{form}");
}
