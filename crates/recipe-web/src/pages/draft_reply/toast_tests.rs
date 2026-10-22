//! Tests of the place of the status region `#toast` (task 17.3, DR-5 (b)).
//!
//! A save at the foot of a long page must show its toast. HTMX 2 applies no `show` modifier to an
//! out-of-band swap, so `#toast` stays in view with `position: sticky` at the start of `main`
//! (`design-system.md` sections 10.1 and 10.7). No script moves the page.
//!
//! The scroll padding of `html` has its tests in `style_tests/rules/scroll_padding.rs` (QA15-1, ADR-0049 (m)).

/// The base style sheet. It holds the page layout (`design-system.md` section 1).
const APP_CSS: &str = include_str!("../../../static/app.css");

/// Return the declarations of the one rule of `APP_CSS` with exactly the selector `selector`.
fn declarations(selector: &str) -> Vec<String> {
    let start = format!("\n{selector} {{\n");
    assert_eq!(APP_CSS.matches(&start).count(), 1, "{selector}");
    let (_before, rest) = APP_CSS.split_once(&start).unwrap();
    let (body, _after) = rest.split_once('}').unwrap();
    body.lines().map(|line| line.trim().to_owned()).collect()
}

#[test]
fn toast_region_sticks_at_the_top_of_the_view_above_the_sticky_grid_cells() {
    let rule = declarations("#toast");
    for declaration in ["position: sticky;", "top: 0;", "z-index: 1;"] {
        assert!(
            rule.iter().any(|line| line == declaration),
            "{declaration}: {rule:?}"
        );
    }
}

#[tokio::test]
async fn toast_region_of_a_full_page_is_the_live_region_of_the_status() {
    let world = crate::test_support::plant::world();
    let reply = world.get_en(&world.engineer, "/machines").await;
    assert!(
        reply
            .body
            .contains("<div id=\"toast\" role=\"status\" aria-live=\"polite\">"),
        "{}",
        reply.body
    );
}
