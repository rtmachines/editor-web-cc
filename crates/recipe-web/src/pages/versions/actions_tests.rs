//! Tests of the action area of the version page and of `POST .../retire` and `POST .../discard`
//! (task 10.3, ADR-0049 (f), (i), (j), and (k), ADR-0051 (g), design D7 to D10).
//!
//! Specs "Version page", "Second request of a form", "Danger actions of a version", "Notice of a
//! danger action", and "Pages of a deleted recipe". `R-100` has the retired version 1, the approved
//! version 2, and the live draft 3. `R-300` is deleted.

use axum::http::StatusCode;
use recipe_domain::audit::AuditAction;

use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{TOAST_CLEAR, assert_clean_html, assert_error, assert_htmx_error};
use crate::test_support::plant::{HTMX, TARGET_CONTENT, World, main_of};
use crate::test_support::{Reply, TestSession};

/// The opening tag of the retire form of the approved version 2 of `R-100`.
const RETIRE_FORM_TAG: &str = "<form method=\"post\" action=\"/recipes/R-100/versions/2/retire\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\">";
/// The opening tag of the discard form of the draft 3 of `R-100`.
const DISCARD_FORM_TAG: &str = "<form method=\"post\" action=\"/recipes/R-100/versions/3/discard\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\" hx-sync=\"closest form:drop\">";
/// The container of the links of the action area. Its `hx-*` attributes are literals.
pub(super) const LINKS_TAG: &str =
    "<p class=\"actions\" hx-boost=\"true\" hx-target=\"#content\" hx-push-url=\"true\">";
/// The link to the approval preview of the draft 3 of `R-100`.
const APPROVE_LINK: &str =
    "<a class=\"target-link\" href=\"/recipes/R-100/versions/3/approve\">Approve version 3</a>";
/// The link to the history of `R-100` (ADR-0051 (g)).
const HISTORY_LINK: &str = "<a class=\"target-link\" href=\"/recipes/R-100/history\">History</a>";
/// The address of the export of the approved version of `R-100` (ADR-0052 (b)).
const APPROVED_EXPORT_HREF: &str = "href=\"/export/v1/recipes/R-100/approved\"";
/// The address of the export of the retired version 1 of `R-100` (ADR-0052 (b)).
const RETIRED_EXPORT_HREF: &str = "href=\"/export/v1/recipes/R-100/versions/1\"";
/// The opening tag of a danger action.
pub(super) const DANGER: &str = "<details class=\"danger\">";

/// Return the markup of the form that starts with `tag`, up to `</form>`.
fn form_of<'a>(main: &'a str, tag: &str) -> &'a str {
    let (_before, rest) = main.split_once(tag).unwrap();
    rest.split("</form>").next().unwrap()
}

/// The opening of the export row of an Approved or a Retired version (ADR-0052 (c)).
pub(super) const EXPORT_ROW: &str =
    "<p class=\"actions\"><a class=\"btn btn-secondary\" href=\"/export/v1/";

/// Return the text after the header `dl`, and after the export row when the page has one.
///
/// The export row stands directly after `</dl>`, with the notice of a Retired version before it.
/// The boosted row of links follows it (ADR-0052 (c) and (e)).
pub(super) fn links_area(main: &str) -> &str {
    let after_header = main.split_once("</dl>").unwrap().1.trim_start();
    if after_header.starts_with(LINKS_TAG) {
        return after_header;
    }
    let export = after_header.split_once("</p>").unwrap();
    assert!(export.0.contains(EXPORT_ROW), "{main}");
    export.1.trim_start()
}

/// Assert the row of links after the header `dl` and the export row, with the history link in it.
fn assert_action_area(main: &str) {
    let area = links_area(main).strip_prefix(LINKS_TAG).expect(main);
    let links = area.split("</p>").next().unwrap();
    assert!(links.contains(HISTORY_LINK), "{main}");
}

/// Assert one danger action with the form `tag`, its summary, its notice, and its button.
fn assert_danger_form(main: &str, tag: &str, (summary, button): (&str, &str)) {
    assert_eq!(main.matches(DANGER).count(), 1, "{main}");
    assert_eq!(main.matches("id=\"danger-notice\"").count(), 1, "{main}");
    assert!(main.contains(&format!("<span>{summary}</span>")), "{main}");
    let form = form_of(main, tag);
    assert!(form.contains("name=\"csrf_token\""), "{form}");
    assert!(
        form.contains("<input type=\"hidden\" name=\"row_version\" value=\"1\">"),
        "{form}"
    );
    let named = format!("aria-describedby=\"danger-notice\">{button}</button>");
    assert!(form.contains(&named), "{form}");
    assert!(
        form.contains("class=\"htmx-indicator\">The server processes the request."),
        "{form}"
    );
}

/// Send the form `body` to `path` as an HTMX request that targets `main`, in English.
async fn post_content(world: &World, session: &TestSession, path: &str, body: &str) -> Reply {
    world
        .post_with(session, path, body, &[HTMX, TARGET_CONTENT])
        .await
}

/// Assert an HTMX error answer that keeps the address and shows the code and the request id.
/// The answer ends with the empty `#toast` out of band, and this is the only element out of band
/// (ADR-0049 (k)). So no earlier success text stays on the page.
fn assert_panel(reply: &Reply, status: StatusCode, code: &str) {
    assert_htmx_error(reply, status, code);
    let reference = format!("Reference: {}.", reply.header("x-request-id"));
    assert!(reply.body.contains(&reference), "{code}: {}", reply.body);
    assert!(reply.body.ends_with(TOAST_CLEAR), "{code}: {}", reply.body);
    assert_eq!(reply.body.matches("hx-swap-oob").count(), 1, "{code}");
}

/// Assert the conflict answer for the version `number` of `R-100`. The answer shows `current` as
/// the current change number (ADR-0008, design D10). It sends no `HX-Trigger`, so the form keeps
/// the old number (`api.md` section 4).
fn assert_conflict(reply: &Reply, number: u32, current: u32) {
    assert_panel(reply, StatusCode::CONFLICT, "E-WEB-001");
    assert!(reply.header("hx-trigger").is_empty(), "{:?}", reply.headers);
    let body = &reply.body;
    assert!(
        body.contains("<span class=\"alert-code\">E-WEB-001</span>"),
        "{body}"
    );
    let message = format!("Another user changed version {number} of recipe R-100.");
    assert!(body.contains(&message), "{body}");
    let line = format!("<li>Current change number: {current}</li>");
    assert!(body.contains(&line), "{body}");
    let href = format!("/recipes/R-100/versions/{number}");
    let link =
        format!("<a class=\"target-link\" href=\"{href}\" hx-boost=\"true\">Reload version</a>");
    assert!(body.contains(&link), "{body}");
}

/// Return the actions of the audit entries that the store gained after its first `before` entries.
fn actions_since(world: &World, before: usize) -> Vec<AuditAction> {
    let entries = world.fixture.store.audit_entries();
    entries
        .iter()
        .skip(before)
        .map(|entry| entry.action())
        .collect()
}

#[tokio::test]
async fn actions_draft_shows_the_approve_link_the_discard_form_and_the_history_link() {
    let world = recipe_world();
    for session in [&world.admin, &world.engineer] {
        let reply = world.get_en(session, "/recipes/R-100/versions/3").await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        assert_action_area(main);
        assert!(main.contains(APPROVE_LINK), "{main}");
        let texts = ("Discard this draft", "Discard draft 3");
        assert_danger_form(main, DISCARD_FORM_TAG, texts);
        assert!(!main.contains("/retire\""), "{main}");
        assert_clean_html(&reply.body);
    }
}

#[tokio::test]
async fn actions_approved_version_shows_the_retire_form_only_to_the_administrator() {
    let world = recipe_world();
    let admin = world
        .get_en(&world.admin, "/recipes/R-100/versions/2")
        .await;
    let main = main_of(&admin.body);
    assert_action_area(main);
    let texts = ("Retire this version", "Retire version 2");
    assert_danger_form(main, RETIRE_FORM_TAG, texts);
    assert!(!main.contains("/approve\""), "{main}");
    assert!(!main.contains("/discard\""), "{main}");
    assert!(main.contains(APPROVED_EXPORT_HREF), "{main}");
    let engineer = world
        .get_en(&world.engineer, "/recipes/R-100/versions/2")
        .await;
    let main = main_of(&engineer.body);
    assert_action_area(main);
    assert!(!main.contains("<form"), "{main}");
    assert!(!main.contains(DANGER), "{main}");
    assert!(main.contains(APPROVED_EXPORT_HREF), "{main}");
}

/// A disabled button loses the focus. So the forms of the version actions have no
/// `hx-disabled-elt` (ADR-0049 (j), `design-system.md` section 10.6, QA-1).
#[tokio::test]
async fn actions_forms_of_retire_and_discard_have_no_hx_disabled_elt() {
    let world = recipe_world();
    for path in ["/recipes/R-100/versions/2", "/recipes/R-100/versions/3"] {
        let reply = world.get_en(&world.admin, path).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        assert_eq!(main.matches(DANGER).count(), 1, "{path}: {main}");
        assert!(!main.contains("hx-disabled-elt"), "{path}: {main}");
    }
}

#[tokio::test]
async fn actions_retired_version_and_operator_draft_show_only_the_history_link() {
    let world = recipe_world();
    let retired = world
        .get_en(&world.admin, "/recipes/R-100/versions/1")
        .await;
    let operator = world
        .get_en(&world.operator, "/recipes/R-100/versions/3")
        .await;
    for reply in [&retired, &operator] {
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        let main = main_of(&reply.body);
        assert_action_area(main);
        assert!(!main.contains("<form"), "{main}");
        assert!(!main.contains(DANGER), "{main}");
        assert!(!main.contains("/approve\""), "{main}");
    }
    // The Administrator also sees the export link of the retired version. A draft has none.
    assert!(
        retired.body.contains(RETIRED_EXPORT_HREF),
        "{}",
        retired.body
    );
    assert!(!operator.body.contains("/export/"), "{}", operator.body);
}

#[tokio::test]
async fn actions_deleted_recipe_shows_no_form_to_the_administrator_and_e_rcp_012_to_others() {
    let world = recipe_world();
    let path = "/recipes/R-300/versions/1";
    let admin = world.get_en(&world.admin, path).await;
    assert_eq!(admin.status, StatusCode::OK, "{}", admin.body);
    let main = main_of(&admin.body);
    assert!(main.contains("Deleted"), "{main}");
    assert!(main.contains("href=\"/recipes/R-300/history\""), "{main}");
    assert!(!main.contains("<form"), "{main}");
    assert!(!main.contains(DANGER), "{main}");
    for session in [&world.engineer, &world.operator] {
        let reply = world.get_en(session, path).await;
        assert_error(&reply, StatusCode::CONFLICT, "E-RCP-012", "deleted");
    }
}

#[tokio::test]
async fn actions_retire_answers_the_version_page_with_the_toast_and_the_address() {
    let world = recipe_world();
    let before = world.audit_count();
    let path = "/recipes/R-100/versions/2/retire";
    let reply = post_content(&world, &world.admin, path, "row_version=1").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    assert_eq!(reply.header("hx-push-url"), "/recipes/R-100/versions/2");
    // The retire raises the change number of the version from 1 to 2.
    assert_eq!(reply.header("hx-trigger"), "{\"rowVersion\": 2}");
    assert!(
        reply.body.contains("Version 2 is retired."),
        "{}",
        reply.body
    );
    let (_before, h1) = reply.body.split_once("<h1").unwrap();
    let h1 = h1.split('>').next().unwrap();
    assert!(h1.contains("aria-describedby=\"toast\""), "{h1}");
    assert!(!reply.body.contains(RETIRE_FORM_TAG), "{}", reply.body);
    assert_eq!(actions_since(&world, before), [AuditAction::VersionRetired]);
}

#[tokio::test]
async fn actions_retire_trigger_equals_the_change_number_that_a_stale_request_reads() {
    let world = recipe_world();
    let before = world.audit_count();
    let path = "/recipes/R-100/versions/2/retire";
    let done = post_content(&world, &world.admin, path, "row_version=1").await;
    assert_eq!(done.status, StatusCode::OK, "{}", done.body);
    assert_eq!(done.header("hx-trigger"), "{\"rowVersion\": 2}");
    // The second request holds the old number. The conflict reports the number after the commit.
    let again = post_content(&world, &world.admin, path, "row_version=1").await;
    assert_conflict(&again, 2, 2);
    // The second request writes no audit entry, so the pair adds exactly one entry.
    assert_eq!(actions_since(&world, before), [AuditAction::VersionRetired]);
}

#[tokio::test]
async fn actions_discard_answers_303_to_the_recipe_page() {
    let world = recipe_world();
    let before = world.audit_count();
    let path = "/recipes/R-100/versions/3/discard";
    let reply = world
        .post(&world.engineer, path, "row_version=1", false)
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes/R-100");
    assert_eq!(actions_since(&world, before), [AuditAction::DraftDiscarded]);
    let page = world
        .get_en(&world.admin, "/recipes/R-100/versions/3")
        .await;
    assert_eq!(page.status, StatusCode::OK, "{}", page.body);
    assert!(!page.body.contains(DISCARD_FORM_TAG), "{}", page.body);
    let main = main_of(&page.body);
    assert!(main.contains("<span>Discarded</span>"), "{main}");
}

#[tokio::test]
async fn actions_discard_by_htmx_answers_303_to_the_recipe_page() {
    let world = recipe_world();
    let before = world.audit_count();
    let path = "/recipes/R-100/versions/3/discard";
    let reply = post_content(&world, &world.engineer, path, "row_version=1").await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/recipes/R-100");
    // The browser follows the redirect, so the headers of this answer do not reach HTMX.
    assert!(
        reply.header("hx-push-url").is_empty(),
        "{:?}",
        reply.headers
    );
    assert!(
        reply.header("hx-redirect").is_empty(),
        "{:?}",
        reply.headers
    );
    assert_eq!(actions_since(&world, before), [AuditAction::DraftDiscarded]);
}

#[tokio::test]
async fn actions_retire_errors_by_htmx_retarget_the_error_panel_and_keep_the_address() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let cases = [
        (
            "/recipes/R-100/versions/9/retire",
            "row_version=1",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/recipes/R-100/versions/3/retire",
            "row_version=1",
            StatusCode::CONFLICT,
            "E-RCP-009",
        ),
        (
            "/recipes/R-300/versions/1/retire",
            "row_version=1",
            StatusCode::CONFLICT,
            "E-RCP-012",
        ),
        (
            "/recipes/R-100/versions/2/retire",
            "row_version=9",
            StatusCode::CONFLICT,
            "E-WEB-001",
        ),
    ];
    for (path, body, status, code) in cases {
        let reply = post_content(&world, &world.admin, path, body).await;
        assert_panel(&reply, status, code);
    }
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn actions_discard_errors_by_htmx_retarget_the_error_panel_and_keep_the_address() {
    let world = recipe_world();
    let before = world.fixture.snapshot();
    let cases = [
        (
            "/recipes/R-100/versions/9/discard",
            "row_version=1",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/recipes/R-100/versions/2/discard",
            "row_version=1",
            StatusCode::CONFLICT,
            "E-RCP-007",
        ),
        (
            "/recipes/R-300/versions/1/discard",
            "row_version=1",
            StatusCode::CONFLICT,
            "E-RCP-012",
        ),
        (
            "/recipes/R-100/versions/3/discard",
            "row_version=9",
            StatusCode::CONFLICT,
            "E-WEB-001",
        ),
    ];
    for (path, body, status, code) in cases {
        let reply = post_content(&world, &world.engineer, path, body).await;
        assert_panel(&reply, status, code);
    }
    assert_eq!(world.fixture.snapshot(), before);
}

#[tokio::test]
async fn actions_stale_retire_shows_the_conflict_with_the_link_to_the_version_page() {
    let world = recipe_world();
    let path = "/recipes/R-100/versions/2/retire";
    let reply = post_content(&world, &world.admin, path, "row_version=8").await;
    assert_conflict(&reply, 2, 1);
}

#[tokio::test]
async fn actions_stale_discard_shows_the_conflict_with_the_link_to_the_version_page() {
    let world = recipe_world();
    let path = "/recipes/R-100/versions/3/discard";
    let reply = post_content(&world, &world.engineer, path, "row_version=8").await;
    assert_conflict(&reply, 3, 1);
}
