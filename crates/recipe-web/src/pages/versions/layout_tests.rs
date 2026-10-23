//! Tests of the place of the action area on the version page (task 10.3, ADR-0049 (i), design D7).
//!
//! The row of links stands right after the header `dl`, with no heading. The `h2` of
//! `version-actions-heading` and the danger action stand at the end of `main`. A page without a
//! danger action shows no such `h2`. `R-100` has the retired version 1, the approved version 2, and
//! the live draft 3.
//!
//! The parameter table has its own `h2` right before `#parameter-table`, outside that element (step-7
//! item S7-17).

use super::actions_tests::{DANGER, EXPORT_ROW, LINKS_TAG, links_area};
use crate::i18n::Lang;
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::position;
use crate::test_support::plant::{POLISH, main_of};
use crate::test_support::text;

/// The heading of the danger action.
const ACTIONS_HEADING: &str = "<h2>Actions</h2>";

/// Return the levels of the headings of `main`, in document order.
fn heading_levels(main: &str) -> Vec<u32> {
    main.split("<h")
        .skip(1)
        .filter_map(|piece| {
            let mut chars = piece.chars();
            let level = chars.next()?.to_digit(10)?;
            matches!(chars.next(), Some('>' | ' ')).then_some(level)
        })
        .collect()
}

/// Assert that the danger action stands under its heading at the end of `main`.
fn assert_danger_at_the_end(main: &str, route: &str) {
    assert_eq!(main.matches(ACTIONS_HEADING).count(), 1, "{main}");
    let heading = position(main, ACTIONS_HEADING);
    assert!(heading > position(main, "id=\"parameter-table\""), "{main}");
    assert!(heading > position(main, "id=\"parameter-form\""), "{main}");
    let after_heading = main.split_once(ACTIONS_HEADING).unwrap().1;
    assert!(after_heading.trim_start().starts_with(DANGER), "{main}");
    assert!(after_heading.contains(route), "{main}");
    let after_danger = after_heading.split_once("</details>").unwrap().1;
    assert!(!after_danger.contains("<h"), "{main}");
}

/// Assert one `h1` and no skipped heading level in `main`.
fn assert_outline(main: &str) {
    let levels = heading_levels(main);
    assert_eq!(levels.first(), Some(&1), "{levels:?} in {main}");
    assert_eq!(levels.iter().filter(|level| **level == 1).count(), 1);
    for pair in levels.windows(2) {
        let next = pair.first().unwrap().checked_add(1).unwrap();
        assert!(pair.last().unwrap() <= &next, "{levels:?} in {main}");
    }
}

#[tokio::test]
async fn layout_draft_row_of_links_stands_right_after_the_header_without_a_heading() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.admin, "/recipes/R-100/versions/3")
        .await;
    let main = main_of(&reply.body);
    let after_header = main.split_once("</dl>").unwrap().1;
    assert!(after_header.trim_start().starts_with(LINKS_TAG), "{main}");
    let row = after_header.split_once("</p>").unwrap().0;
    assert!(row.contains("/recipes/R-100/versions/3/approve"), "{main}");
    assert!(row.contains("/recipes/R-100/history"), "{main}");
    assert!(!row.contains("<h"), "{row}");
    let before_table = main.split_once("id=\"parameter-table\"").unwrap().0;
    assert!(!before_table.contains(ACTIONS_HEADING), "{main}");
}

#[tokio::test]
async fn layout_draft_danger_action_stands_under_its_heading_at_the_end_of_main() {
    let world = recipe_world();
    for session in [&world.admin, &world.engineer] {
        let reply = world.get_en(session, "/recipes/R-100/versions/3").await;
        let main = main_of(&reply.body);
        assert_danger_at_the_end(main, "/recipes/R-100/versions/3/discard");
    }
}

#[tokio::test]
async fn layout_approved_version_retire_action_stands_under_its_heading_at_the_end_of_main() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.admin, "/recipes/R-100/versions/2")
        .await;
    let main = main_of(&reply.body);
    // The export row follows `</dl>` (ADR-0052 (c)). Then the boosted row of links.
    let after_header = main.split_once("</dl>").unwrap().1;
    assert!(after_header.trim_start().starts_with(EXPORT_ROW), "{main}");
    assert!(links_area(main).starts_with(LINKS_TAG), "{main}");
    assert_danger_at_the_end(main, "/recipes/R-100/versions/2/retire");
}

#[tokio::test]
async fn layout_page_without_a_danger_action_shows_no_actions_heading() {
    let world = recipe_world();
    let pages = [
        (&world.operator, "/recipes/R-100/versions/3"),
        (&world.engineer, "/recipes/R-100/versions/2"),
        (&world.admin, "/recipes/R-100/versions/1"),
        (&world.admin, "/recipes/R-300/versions/1"),
    ];
    for (session, path) in pages {
        let reply = world.get_en(session, path).await;
        let main = main_of(&reply.body);
        assert!(!main.contains(ACTIONS_HEADING), "{path}: {main}");
        assert!(!main.contains(DANGER), "{path}: {main}");
        assert!(main.contains(LINKS_TAG), "{path}: {main}");
    }
}

/// Assert that the last `h2` before `#parameter-table` is the `h2` of the table, with `caption`.
fn assert_table_has_its_own_heading(main: &str, caption: &str) {
    let before_table = main.split_once("<div id=\"parameter-table\">").unwrap().0;
    let last_h2 = before_table.rsplit_once("<h2");
    assert!(last_h2.is_some(), "no h2 before the table in {main}");
    let heading = last_h2.unwrap().1;
    let expected = format!(">{caption}</h2>");
    assert!(heading.starts_with(&expected), "{heading} in {main}");
    assert!(!heading.contains("<form"), "{heading} in {main}");
}

#[tokio::test]
async fn layout_parameter_table_of_a_draft_with_a_rebind_form_has_its_own_heading() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, "/recipes/R-100/versions/3")
        .await;
    let main = main_of(&reply.body);
    assert!(main.contains("Rebind ST_Zone@2 to ST_Zone@3"), "{main}");
    assert_table_has_its_own_heading(main, &text(Lang::En, "parameters-caption"));
    let reply = world
        .get(&world.engineer, "/recipes/R-100/versions/3", &[POLISH])
        .await;
    let main = main_of(&reply.body);
    assert!(main.contains("<form id=\"rebind-"), "{main}");
    assert_table_has_its_own_heading(main, &text(Lang::Pl, "parameters-caption"));
    assert_outline(main);
}

#[tokio::test]
async fn layout_parameter_table_of_an_approved_version_has_its_own_heading() {
    let world = recipe_world();
    let reply = world
        .get_en(&world.engineer, "/recipes/R-100/versions/2")
        .await;
    let main = main_of(&reply.body);
    assert!(main.contains("<div id=\"outdated-list\"></div>"), "{main}");
    assert_table_has_its_own_heading(main, &text(Lang::En, "parameters-caption"));
}

#[tokio::test]
async fn layout_headings_have_one_h1_and_no_skipped_level() {
    let world = recipe_world();
    let pages = [
        (&world.admin, "/recipes/R-100/versions/3"),
        (&world.admin, "/recipes/R-100/versions/2"),
        (&world.engineer, "/recipes/R-100/versions/2"),
        (&world.operator, "/recipes/R-100/versions/3"),
    ];
    for (session, path) in pages {
        let reply = world.get_en(session, path).await;
        assert_outline(main_of(&reply.body));
    }
}
