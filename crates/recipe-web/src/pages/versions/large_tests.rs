//! Tests of the version page of a draft with 4 000 parameters in 40 units (task 9.4).
//!
//! Spec "Version page", scenario "Large draft". ADR-0043 (j), design D16. The page shows one
//! unit, at most `page.parameters` rows, and one link to each unit with the count of its rows.

use axum::http::StatusCode;

use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::plant::main_of;

/// The largest body of the page of the large draft, in bytes.
///
/// The page with 100 rows and 40 unit links measured 27 276 bytes. The bound of 40 KiB keeps a
/// margin. A page with all 4 000 rows would have more than 700 KB, so the bound fails at once.
const BYTE_BOUND: usize = 40_960;
/// The start of the link of one parameter row of the large draft.
const ROW_LINK: &str = "<tr><td class=\"data\"><a href=\"/recipes/R-400/versions/1/parameters/";

/// Return the link of the unit `path` with its count of 100 parameters.
fn unit_link(path: &str, current: bool) -> String {
    let marker = if current {
        " aria-current=\"page\""
    } else {
        ""
    };
    format!(
        "<a class=\"target-link\" href=\"/recipes/R-400/versions/1?unit={path}\"{marker}>{path} (parameters: 100)</a>"
    )
}

#[tokio::test]
async fn versions_large_draft_shows_one_unit_with_at_most_one_hundred_rows_under_a_byte_bound() {
    let world = recipe_world();
    let large = world.fixture.seed_large_draft();
    assert_eq!(large.parameters, 4000);
    let reply = world
        .get_en(&world.engineer, "/recipes/R-400/versions/1")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert_eq!(main.matches(ROW_LINK).count(), 100, "{main}");
    assert!(main.contains("\">P0000</a></td>"), "{main}");
    assert!(!main.contains("\">P0100</a></td>"), "{main}");
    assert!(
        reply.body.len() < BYTE_BOUND,
        "the page has {} bytes",
        reply.body.len()
    );
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn versions_large_draft_links_each_unit_with_its_count_and_marks_the_shown_unit() {
    let world = recipe_world();
    world.fixture.seed_large_draft();
    let reply = world
        .get_en(&world.engineer, "/recipes/R-400/versions/1")
        .await;
    let main = main_of(&reply.body);
    let nav = main
        .split("<nav aria-label=\"Units of the version\">")
        .nth(1)
        .unwrap();
    let nav = nav.split("</nav>").next().unwrap();
    assert_eq!(nav.matches("<a class=\"target-link\"").count(), 40, "{nav}");
    assert!(nav.contains(&unit_link("U1", true)), "{nav}");
    assert!(nav.contains(&unit_link("U2", false)), "{nav}");
    assert!(nav.contains(&unit_link("U40", false)), "{nav}");
    assert_eq!(nav.matches("aria-current").count(), 1, "{nav}");
}

#[tokio::test]
async fn versions_large_draft_shows_the_unit_of_the_link() {
    let world = recipe_world();
    world.fixture.seed_large_draft();
    let reply = world
        .get_en(&world.engineer, "/recipes/R-400/versions/1?unit=U2")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert_eq!(main.matches(ROW_LINK).count(), 100, "{main}");
    assert!(main.contains("\">P0100</a></td>"), "{main}");
    assert!(!main.contains("\">P0000</a></td>"), "{main}");
    assert!(main.contains(&unit_link("U2", true)), "{main}");
    assert!(main.contains(&unit_link("U1", false)), "{main}");
    let add = "href=\"/recipes/R-400/versions/1/parameters/new?unit=U2\"";
    assert!(main.contains(add), "{main}");
}
