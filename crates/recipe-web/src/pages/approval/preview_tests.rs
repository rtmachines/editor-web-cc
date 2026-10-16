//! Tests of `GET /recipes/{ref}/versions/{n}/approve`: the roles, the sections, the summary, and
//! the approve form (task 10.1, ADR-0049 (a) and (b), ADR-0050 (d), spec "Approval preview page",
//! "Layout of the approval preview").

use axum::http::StatusCode;

use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::{assert_clean_html, assert_error, field_value};
use crate::test_support::icon_tag;
use crate::test_support::plant::main_of;

/// The preview of the draft version 2 of `R-500`.
const R500_PREVIEW: &str = "/recipes/R-500/versions/2/approve";

/// Return the part of `main` from the first `start` to the next `end`, or to the end.
pub(super) fn part_of<'a>(main: &'a str, start: &str, end: &str) -> &'a str {
    assert!(main.contains(start), "{start} is not in {main}");
    let rest = &main[main.find(start).unwrap()..];
    let to = rest[start.len()..]
        .find(end)
        .map_or(rest.len(), |at| at.checked_add(start.len()).unwrap());
    &rest[..to]
}

/// Return the position of `needle` in `main`, or fail with `main`.
fn position(main: &str, needle: &str) -> usize {
    assert!(main.contains(needle), "{needle} is not in {main}");
    main.find(needle).unwrap()
}

/// Return the section `kind` of the diff in `main`.
pub(super) fn section<'a>(main: &'a str, kind: &str) -> &'a str {
    part_of(
        main,
        &format!("<section class=\"diff-section\" id=\"diff-{kind}\""),
        "</section>",
    )
}

/// Send the preview of `R-500` version 2 as the Administrator `admin`. Return `main`.
async fn admin_preview(world: &AuditWorld) -> String {
    let reply = world.get_en(&world.data.admin, R500_PREVIEW).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    main_of(&reply.body).to_owned()
}

#[tokio::test]
async fn approve_preview_answers_each_role_with_the_permission_and_refuses_an_operator() {
    let world = audit_world().await;
    for session in [&world.data.admin, &world.data.anowak, &world.data.jsmith] {
        let reply = world.get_en(session, R500_PREVIEW).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(
            main_of(&reply.body).contains("<h1>Approval of version 2 of recipe R-500</h1>"),
            "{}",
            reply.body
        );
    }
    let reply = world.get_en(&world.operator, R500_PREVIEW).await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", "operator");
}

#[tokio::test]
async fn approve_preview_shows_the_sections_in_the_order_of_the_kinds() {
    let world = audit_world().await;
    let main = admin_preview(&world).await;
    let positions: Vec<usize> = ["limits", "values", "added", "removed"]
        .iter()
        .map(|kind| position(&main, &format!("id=\"diff-{kind}\"")))
        .collect();
    let mut sorted = positions.clone();
    sorted.sort_unstable();
    assert_eq!(positions, sorted, "{main}");
    assert!(!main.contains("id=\"diff-definitions\""), "{main}");
    assert_eq!(
        main.matches("class=\"table-wrap diff-limits\"").count(),
        1,
        "{main}"
    );
    assert!(
        main.contains("The approval of version 2 retires version 1."),
        "{main}"
    );
}

#[tokio::test]
async fn approve_preview_limits_section_shows_the_changed_limit_with_its_state() {
    let world = audit_world().await;
    let main = admin_preview(&world).await;
    let limits = section(&main, "limits");
    assert!(limits.contains(&icon_tag("diff-limit")), "{limits}");
    assert!(limits.contains("Limit changed"), "{limits}");
    assert!(
        limits.contains("<caption id=\"diff-limits-caption\">"),
        "{limits}"
    );
    let row = part_of(limits, ">SPEED_TABLE<", "</tr>");
    assert!(row.contains("[*]"), "{row}");
    assert!(!row.contains("Minimum"), "{row}");
    assert!(
        row.contains("<td class=\"diff-value\">0 to 3000</td>"),
        "{row}"
    );
    assert!(
        row.contains("<td class=\"diff-value\">10 to 3000</td>"),
        "{row}"
    );
}

#[tokio::test]
async fn approve_preview_values_section_shows_the_leaf_with_the_new_limit() {
    let world = audit_world().await;
    let main = admin_preview(&world).await;
    let values = section(&main, "values");
    let row = part_of(values, ">LINE_A<", "</tr>");
    assert!(row.contains(".head.a"), "{row}");
    assert!(row.contains("<td class=\"diff-value\">1</td>"), "{row}");
    assert!(row.contains("<td class=\"diff-value\">2</td>"), "{row}");
    assert!(row.contains("0 to 100"), "{row}");
    assert!(row.contains("Value changed"), "{row}");
}

#[tokio::test]
async fn approve_preview_summary_counts_link_to_their_sections() {
    let world = audit_world().await;
    let main = admin_preview(&world).await;
    let summary = part_of(&main, "<ul class=\"diff-summary\">", "</ul>");
    for kind in ["limits", "values", "added", "removed"] {
        assert!(
            summary.contains(&format!("href=\"#diff-{kind}\"")),
            "{summary}"
        );
    }
    assert!(summary.contains("Parameters: 1"), "{summary}");
    assert!(summary.contains("Rows: "), "{summary}");
    assert!(
        summary.contains("Parameters without a change: 1"),
        "{summary}"
    );
    assert!(
        summary.contains("Parameters only in version 1"),
        "{summary}"
    );
    assert!(summary.contains(&icon_tag("diff-added")), "{summary}");
    let before_summary = &main[..position(&main, "<ul class=\"diff-summary\">")];
    assert!(before_summary.contains("Summary of the changes"), "{main}");
}

#[tokio::test]
async fn approve_preview_first_summary_item_links_to_the_heading_of_the_form() {
    let world = audit_world().await;
    let main = admin_preview(&world).await;
    let summary = part_of(&main, "<ul class=\"diff-summary\">", "</ul>");
    let first = part_of(summary, "<li>", "</li>");
    assert!(first.contains("href=\"#approve-form\""), "{first}");
    assert!(first.contains("Go to the approval form"), "{first}");
    assert!(
        main.contains("<h2 id=\"approve-form\">Approval</h2>"),
        "{main}"
    );
}

#[tokio::test]
async fn approve_preview_links_only_the_first_row_of_a_parameter_in_a_section() {
    let world = audit_world().await;
    let main = admin_preview(&world).await;
    let limits = section(&main, "limits");
    // One limit row for each leaf path (design D4). `preview_rows_tests` covers two rows.
    assert_eq!(limits.matches(">ADDED<").count(), 1, "{limits}");
    assert_eq!(limits.matches(">ADDED</a>").count(), 1, "{limits}");
    let added = section(&main, "added");
    assert_eq!(added.matches(">ADDED</a>").count(), 1, "{added}");
    let removed = section(&main, "removed");
    let link = part_of(
        removed,
        "href=\"/recipes/R-500/versions/1/parameters/",
        "\"",
    );
    assert!(link.len() > 40, "{removed}");
}

#[tokio::test]
async fn approve_preview_form_sends_the_row_version_of_the_snapshot_with_the_token() {
    let world = audit_world().await;
    let main = admin_preview(&world).await;
    let form = part_of(
        &main,
        "<form id=\"approve-fields\" method=\"post\" action=\"/recipes/R-500/versions/2/approve\"",
        "</form>",
    );
    for literal in [
        "hx-boost=\"true\"",
        "hx-target=\"#content\"",
        "hx-push-url=\"true\"",
        "hx-sync=\"closest form:drop\"",
        "hx-disabled-elt=\"find button\"",
        "<span class=\"htmx-indicator\">The server processes the request.</span>",
        "Approve version 2",
    ] {
        assert!(form.contains(literal), "{literal}: {form}");
    }
    let row_version = world.data.r500_v2_row_version.to_string();
    assert_eq!(field_value(form, "row_version"), row_version, "{form}");
    assert_eq!(
        field_value(form, "csrf_token"),
        world.data.admin.csrf,
        "{form}"
    );
    assert!(!form.contains("allow_outdated"), "{form}");
    assert!(
        main.contains("href=\"/recipes/R-500/versions/2\""),
        "{main}"
    );
    assert!(main.contains("Back to version 2"), "{main}");
}

#[tokio::test]
async fn approve_preview_content_answer_focuses_the_heading() {
    let world = audit_world().await;
    let reply = world.get_content(&world.data.admin, R500_PREVIEW).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("<html"), "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<h1 tabindex=\"-1\" autofocus>Approval of version 2"),
        "{}",
        reply.body
    );
}
