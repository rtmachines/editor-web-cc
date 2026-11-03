//! Tests of `GET .../parameters/{key}`: the parameter page (task 11.2).
//!
//! Specs "One edit form of a parameter" and "Leaf grid", design D7 and D8, ADR-0039 (i),
//! ADR-0043 (k).

use axum::http::StatusCode;

use super::test_kit::{parameter_path, position, seed_translation};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{
    assert_clean_html, assert_error, assert_ids_unique_and_references_resolve, attribute_values,
    field_value,
};
use crate::test_support::plant::{ENGLISH, POLISH, main_of};
use crate::test_support::recipes::{HOLD_KEY, LINE_A_KEY, SPEED_KEY};

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
                block.contains("<dd lang=\"en\">A parameter of the press line</dd>"),
                "{ui_lang} {id}: {block}"
            );
        }
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
