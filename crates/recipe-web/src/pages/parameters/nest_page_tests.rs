//! Tests of the mode rows on the page of a nested array (ADR-0223, design D2).
//!
//! The parameter `NEST` has the type `ARRAY[1..2] OF ARRAY[1..3] OF INT`. The page shows one mode
//! row for each type level, and the read-only grid shows its mode texts without a button.
use super::test_kit::post_htmx;
use crate::test_support::plant::{World, main_of, world};
use axum::http::StatusCode;
use recipe_domain::recipe::{ParameterKey, RecipeVersionId};
/// The path of the draft of `R-200`. It is a draft of the `TwinCat3` machine `M20`.
const DRAFT: &str = "/recipes/R-200/versions/1";
/// The text that starts a mode row of the grid.
const MODE_ROW: &str = "<tr><td colspan=\"6\"><span class=\"grid-note\">";
/// The text that starts a leaf row of the grid.
const LEAF_ROW: &str = "<tr><th scope=\"row\"";
/// The leaf paths of `NEST`.
const LEAVES: [&str; 6] = ["[1][1]", "[1][2]", "[1][3]", "[2][1]", "[2][2]", "[2][3]"];
/// The limit paths of a limit per element at both type levels.
const EACH_LIMITS: [&str; 6] = LEAVES;
/// The limit paths of a mixed type level: the limit of `[1][*]` and a limit for each `[2][i]`.
const MIXED_LIMITS: [&str; 4] = ["[1][*]", "[2][1]", "[2][2]", "[2][3]"];
/// Add `NEST` to the draft of `R-200` with a limit on each path of `limits`. Return its page.
async fn nest_page(world: &World, limits: &[&str]) -> String {
    let ids = world.fixture.seed_recipes();
    let mut pairs: Vec<(String, String)> = [
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "NEST"),
        ("name_en", "Nest"),
        ("description_en", "A nested array"),
        ("type", "ARRAY[1..2] OF ARRAY[1..3] OF INT"),
        ("value_type", "ARRAY[1..2] OF ARRAY[1..3] OF INT"),
        ("ordinal", "7"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect();
    for leaf in LEAVES {
        pairs.push((format!("value{leaf}"), "7".to_owned()));
    }
    for (index, path) in limits.iter().enumerate() {
        pairs.push((format!("limits[{index}].leaf_path"), (*path).to_owned()));
        pairs.push((format!("limits[{index}].min"), "0".to_owned()));
        pairs.push((format!("limits[{index}].max"), "100".to_owned()));
    }
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let reply = post_htmx(world, &format!("{DRAFT}/parameters"), &refs).await;
    assert!(
        reply.status.is_success(),
        "{}: {}",
        reply.status,
        reply.body
    );
    let key = stored_key(world, ids.r200_draft);
    format!("{DRAFT}/parameters/{key}")
}
/// Return the key of the parameter `NEST` of the version `version`.
fn stored_key(world: &World, version: RecipeVersionId) -> ParameterKey {
    let stored = world.fixture.store.state().parameters_of(version);
    let found = stored.iter().find(|row| row.tag.as_str() == "NEST");
    found.map(|row| row.key).unwrap()
}
/// Count the mode rows and the leaf rows of the grid in `body`.
fn rows_of(body: &str) -> (usize, usize) {
    (
        body.matches(MODE_ROW).count(),
        body.matches(LEAF_ROW).count(),
    )
}
#[tokio::test]
async fn nest_page_of_an_engineer_shows_one_mode_button_for_each_type_level() {
    let world = world();
    let path = nest_page(&world, &EACH_LIMITS).await;
    let reply = world.get_en(&world.engineer, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let buttons = reply.body.matches("name=\"limit_mode\"").count();
    assert_eq!(buttons, 2, "{}", reply.body);
    assert!(reply.body.contains("name=\"limit_mode\" value=\"[*]\""));
    assert!(reply.body.contains("name=\"limit_mode\" value=\"[*][*]\""));
    assert_eq!(rows_of(&reply.body), (2, LEAVES.len()), "{}", reply.body);
    for leaf in LEAVES {
        assert!(reply.body.contains(&format!(">{leaf}</th>")), "{leaf}");
    }
}
#[tokio::test]
async fn nest_page_of_an_operator_shows_the_mode_texts_of_a_mixed_type_level_and_no_button() {
    let world = world();
    let path = nest_page(&world, &MIXED_LIMITS).await;
    let reply = world.get_en(&world.operator, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        !reply.body.contains("name=\"limit_mode\""),
        "{}",
        reply.body
    );
    assert!(!main_of(&reply.body).contains("<form"), "{}", reply.body);
    assert_eq!(rows_of(&reply.body).0, 2, "{}", reply.body);
    for level in ["[*]", "[*][*]"] {
        let text = format!("{MODE_ROW}{level}: ");
        assert_eq!(reply.body.matches(&text).count(), 1, "{level}");
    }
}
/// The text that starts the notice of a type level with mixed modes.
const MIXED_NOTICE: &str = "had both modes";
/// Return the HTML of the read-only row of the leaf path `leaf` in `body`.
fn read_row_of<'a>(body: &'a str, leaf: &str) -> &'a str {
    let start = body
        .find(&format!("<tr><th scope=\"row\">{leaf}</th>"))
        .unwrap();
    let rest = &body[start..];
    &rest[..rest.find("</tr>").unwrap()]
}
#[tokio::test]
async fn nest_page_mixed_modes_of_an_engineer_copy_the_limit_and_show_one_notice() {
    let world = world();
    let path = nest_page(&world, &MIXED_LIMITS).await;
    let reply = world.get_en(&world.engineer, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(
        reply.body.matches(MIXED_NOTICE).count(),
        1,
        "{}",
        reply.body
    );
    assert_eq!(rows_of(&reply.body), (2, LEAVES.len()), "{}", reply.body);
    for leaf in LEAVES {
        let hidden = format!(
            "name=\"limits[{}].leaf_path\" value=\"{leaf}\"",
            index_of(leaf)
        );
        assert!(reply.body.contains(&hidden), "{leaf}");
    }
    assert!(!reply.body.contains("value=\"[1][*]\""), "{}", reply.body);
}
/// Return the position of the leaf path `leaf` in `LEAVES`: the number of its limit row.
fn index_of(leaf: &str) -> usize {
    LEAVES.iter().position(|path| *path == leaf).unwrap()
}
#[tokio::test]
async fn nest_page_mixed_modes_of_an_operator_show_the_copied_limits_as_text_and_no_notice() {
    let world = world();
    let path = nest_page(&world, &MIXED_LIMITS).await;
    let reply = world.get_en(&world.operator, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    for leaf in ["[1][1]", "[1][2]", "[1][3]"] {
        let row = read_row_of(&reply.body, leaf);
        assert!(
            row.contains("<td class=\"data\">0</td><td class=\"data\">100</td>"),
            "{leaf}: {row}"
        );
    }
    assert!(!reply.body.contains(MIXED_NOTICE), "{}", reply.body);
}

/// The limit rows of a save of `STRING` leaves: `[1][*]` and a bad maximum length on `[2][1]`.
const LENGTH_ROWS: [(&str, &str); 2] = [("[1][*]", "5"), ("[2][1]", "x")];

/// Send the add form of `NEST_S` with the types `type_text` and the limit rows `rows`.
///
/// Each row is a leaf path and a maximum length. The values of `leaves` are one letter.
async fn add_string_reply(
    world: &World,
    type_text: &str,
    leaves: &[&str],
    rows: &[(&str, &str)],
) -> crate::test_support::Reply {
    world.fixture.seed_recipes();
    let mut pairs: Vec<(String, String)> = [
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "NEST_S"),
        ("name_en", "Nest"),
        ("description_en", "A nested array"),
        ("type", type_text),
        ("value_type", type_text),
        ("ordinal", "7"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect();
    for leaf in leaves {
        pairs.push((format!("value{leaf}"), "a".to_owned()));
    }
    for (index, (path, length)) in rows.iter().enumerate() {
        pairs.push((format!("limits[{index}].leaf_path"), (*path).to_owned()));
        pairs.push((format!("limits[{index}].max_length"), (*length).to_owned()));
    }
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    post_htmx(world, &format!("{DRAFT}/parameters"), &refs).await
}

#[tokio::test]
async fn limit_line_of_a_sent_row_names_the_input_of_its_leaf_path_after_the_copy() {
    let world = world();
    let leaves = ["[1][1]", "[1][2]", "[1][3]", "[2][1]", "[2][2]", "[2][3]"];
    let type_text = "ARRAY[1..2] OF ARRAY[1..3] OF STRING(10)";
    let reply = add_string_reply(&world, type_text, &leaves, &LENGTH_ROWS).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    // The copy of `[1][*]` puts the rows `[1][1]` to `[1][3]` before `[2][1]`.
    let input = "limits-3-max_length";
    let hidden = "name=\"limits[3].leaf_path\" value=\"[2][1]\"";
    assert!(reply.body.contains(hidden), "{}", reply.body);
    let line = format!("<div class=\"field-error\" id=\"{input}-error\">");
    assert!(reply.body.contains(&line), "{}", reply.body);
    assert!(
        reply.body.contains(&format!("href=\"#{input}\"")),
        "{}",
        reply.body
    );
    assert!(
        !reply.body.contains("limits-1-max_length-error"),
        "{}",
        reply.body
    );
    assert!(
        !reply.body.contains("href=\"#limits-1-max_length\""),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn limit_line_of_a_maximum_length_on_a_row_without_that_input_keeps_its_sent_id() {
    let world = world();
    let reply = add_string_reply(&world, "LDT", &[""], &[("", "x")]).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert!(
        reply.body.contains("href=\"#limits-0-max_length\""),
        "{}",
        reply.body
    );
    assert!(
        !reply.body.contains("id=\"limits-0-max_length-error\""),
        "{}",
        reply.body
    );
    assert!(
        !reply.body.contains("id=\"limits-0-min-error\""),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn limit_line_of_a_length_on_a_row_that_the_copy_removed_stays_off_the_input_of_another_leaf()
{
    let world = world();
    let type_text = "ARRAY[1..3] OF ARRAY[1..2] OF STRING(10)";
    let rows = [("[1][*]", "5"), ("[2][*]", "x"), ("[3][1]", "5")];
    let reply = add_string_reply(&world, type_text, &["[3][1]"], &rows).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    // The copy of the mixed level renumbers the rows, so `limits-1-max_length` is the input of
    // `[1][2]`. The line stays in the summary and has no link and no line under that input.
    let body = &reply.body;
    assert!(
        body.contains("name=\"limits[1].leaf_path\" value=\"[1][2]\""),
        "{body}"
    );
    assert!(body.contains("id=\"limits-1-max_length\""), "{body}");
    assert!(!body.contains("id=\"limits-1-max_length-error\""), "{body}");
    assert!(!body.contains("href=\"#limits-1-max_length\""), "{body}");
    assert!(body.contains("[2][*]"), "{body}");
    assert!(body.contains("<ul class=\"field-lines\"><li>"), "{body}");
    assert!(!body.contains("aria-invalid"), "{body}");
}
