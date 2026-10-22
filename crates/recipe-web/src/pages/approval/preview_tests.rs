//! Tests of `GET /recipes/{ref}/versions/{n}/approve`: the roles, the sections, the summary, and
//! the approve form (task 10.1, ADR-0049 (a) and (b), ADR-0050 (d), spec "Approval preview page",
//! "Layout of the approval preview").

use axum::http::StatusCode;

pub(super) use crate::pages::test_helpers::{assert_before, part_of, section};

use super::approve_tests::{R500_APPROVE, approve, assert_refusal};
use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::{
    assert_clean_html, assert_error, field_value, position, seed_unedited_version_3,
};
use crate::test_support::icon_tag;
use crate::test_support::plant::main_of;

/// The preview of the draft version 2 of `R-500`.
const R500_PREVIEW: &str = "/recipes/R-500/versions/2/approve";

/// Assert that the header of version 1 comes before the header of version 2 in `table`.
///
/// `value` is true for a table whose version columns have the class `diff-value`. The check
/// fails when a view swaps the old and the new column header.
pub(super) fn assert_old_header_first(table: &str, value: bool) {
    let class = if value { " class=\"diff-value\"" } else { "" };
    assert_before(
        table,
        &format!("<th scope=\"col\"{class}>Version 1</th>"),
        &format!("<th scope=\"col\"{class}>Version 2</th>"),
    );
}

/// The fixed id of the approve button. HTMX 2 keeps the focus over a swap for an element with an id.
const APPROVE_BUTTON_ID: &str = "id=\"approve-submit\"";

/// Assert that `whole` holds the fixed id of the approve button exactly once.
///
/// `whole` is the full `main` of a page or the full body of an answer. A second element with the
/// same id outside the form breaks the focus rule of HTMX 2 and the structure of the page.
fn assert_approve_id_once(whole: &str) {
    assert_eq!(whole.matches(APPROVE_BUTTON_ID).count(), 1, "{whole}");
}

/// Assert that `whole` holds the approve id once and that `form` holds one submit button with the
/// fixed id and the primary class.
pub(super) fn assert_approve_button_id(whole: &str, form: &str) {
    assert_approve_id_once(whole);
    assert_eq!(form.matches(APPROVE_BUTTON_ID).count(), 1, "{form}");
    let button = part_of(form, "<button", ">");
    assert!(button.contains(APPROVE_BUTTON_ID), "{button}");
    assert!(button.contains("type=\"submit\""), "{button}");
    assert!(button.contains("class=\"btn btn-primary\""), "{button}");
}

/// Send the preview of `R-500` version 2 as the Administrator `admin`. Return `main`.
pub(super) async fn admin_preview(world: &AuditWorld) -> String {
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
    // The old limit comes first and the new limit second (spec "Preview of a changed limit").
    assert_before(
        row,
        "<td class=\"diff-value\">0 to 3000</td>",
        "<td class=\"diff-value\">10 to 3000</td>",
    );
    // The header of the old version comes first, as the cells do.
    assert_old_header_first(limits, true);
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
    // The counts follow from the fixture `seed_approval_recipes` (`R-500`, version 1 to version 2):
    // - Limits: 3 parameters with 3 rows. `SPEED_TABLE` changes its limit `[*]` (one row). `ADDED`
    //   and `OLD_ONLY` stand on one side only. Each has the one limit `""`, which counts as a row.
    // - Values: 1 parameter, `LINE_A` (`.head.a`). Added: `ADDED`. Removed: `OLD_ONLY`.
    //   Only limits, definitions, and translations show a row count (design D4).
    // - `SPEED` is on both sides with no change, so the count without a change is 1.
    let counts = [
        (
            "limits",
            "<span>Parameters: 3</span>",
            Some("<span>Rows: 3</span>"),
        ),
        ("values", "<span>Parameters: 1</span>", None),
        ("added", "<span>Parameters: 1</span>", None),
        ("removed", "<span>Parameters: 1</span>", None),
    ];
    for (kind, parameters, rows) in counts {
        let start = format!("<a class=\"target-link\" href=\"#diff-{kind}\">");
        let item = part_of(summary, &start, "</li>");
        assert!(item.contains(parameters), "{kind}: {item}");
        match rows {
            Some(rows) => assert!(item.contains(rows), "{kind}: {item}"),
            None => assert!(!item.contains("Rows: "), "{kind}: {item}"),
        }
    }
    assert_eq!(summary.matches("Rows: ").count(), 1, "{summary}");
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
        "<span class=\"htmx-indicator\">The server processes the request.</span>",
        "Approve version 2",
    ] {
        assert!(form.contains(literal), "{literal}: {form}");
    }
    // The button keeps the focus after a refusal (ADR-0049 (j), design-system.md 10.6).
    assert!(!form.contains("hx-disabled-elt"), "{form}");
    assert_approve_button_id(&main, form);
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

/// The preview of a draft without a change shows the text `diff-empty` in place of the sections,
/// before the approve form (design D4, ADR-0050 (d)).
#[tokio::test]
async fn approve_preview_of_a_draft_without_a_change_shows_the_empty_state_before_the_form() {
    let world = audit_world().await;
    seed_unedited_version_3(&world.fixture, &world.data).await;
    let reply = world
        .get_en(&world.data.admin, "/recipes/R-500/versions/3/approve")
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    let main = main_of(&reply.body);
    let empty = "<p class=\"empty\">Versions 3 and 2 have the same parameters.</p>";
    assert!(main.contains(empty), "{main}");
    assert_before(main, empty, "<form id=\"approve-fields\"");
    assert!(!main.contains("id=\"diff-limits\""), "{main}");
    assert!(!main.contains("class=\"pager\""), "{main}");
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

/// The answer `E-RCP-024` swaps the form out of band. The button needs the same fixed id as on the
/// page, so HTMX 2 keeps the focus on it (ADR-0049 (j), QA-1).
#[tokio::test]
async fn approve_refused_with_outdated_parameters_rebuilds_the_form_with_the_button_id() {
    let world = audit_world().await;
    let anowak = world.data.anowak_user();
    let sent = world
        .fixture
        .add_zone_to_r500(&anowak, world.data.r500_v2_row_version)
        .await;
    let body = format!("row_version={sent}");
    let reply = approve(&world, &world.data.admin, R500_APPROVE, &body).await;
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-024");
    let form = part_of(&reply.body, "<form id=\"approve-fields\"", "</form>");
    assert!(form.contains("hx-swap-oob=\"true\""), "{form}");
    assert!(!form.contains("hx-disabled-elt"), "{form}");
    assert_approve_button_id(&reply.body, form);
}
