//! Tests of `POST /recipes/{ref}/versions/{n}/approve`: the answer of a success, the replay, and
//! the flag `allow_outdated` (task 10.2, ADR-0049 (b) and (e), spec "Answer of the approval" and
//! "Errors of the approval").

use axum::http::StatusCode;
use recipe_domain::audit::AuditAction;

use super::preview_tests::part_of;
use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::{assert_clean_html, assert_htmx_error};
use crate::test_support::plant::{ENGLISH, HTMX, TARGET_CONTENT};
use crate::test_support::{Reply, TestSession, post_fields};

/// The approval route of the draft version 2 of `R-500`.
pub(super) const R500_APPROVE: &str = "/recipes/R-500/versions/2/approve";

/// Send an HTMX approval that targets `main`, in English.
pub(super) async fn approve(
    world: &AuditWorld,
    session: &TestSession,
    path: &str,
    body: &str,
) -> Reply {
    let headers = [ENGLISH, HTMX, TARGET_CONTENT];
    post_fields(&world.fixture.router(), path, Some(session), &headers, body).await
}

/// Return the image after the change of each `VersionApproved` entry as JSON text, in the order of
/// the log.
pub(super) fn approvals(world: &AuditWorld) -> Vec<String> {
    world
        .fixture
        .store
        .audit_entries()
        .into_iter()
        .filter(|entry| entry.action() == AuditAction::VersionApproved)
        .map(|entry| entry.after().map(ToString::to_string).unwrap_or_default())
        .collect()
}

/// Assert an HTMX error answer that keeps the address of the page (ADR-0037 (z)).
pub(super) fn assert_refusal(reply: &Reply, status: StatusCode, code: &str) {
    assert_htmx_error(reply, status, code);
    assert_eq!(reply.header("hx-push-url"), "false", "{code}");
    assert!(reply.body.contains("Reference: "), "{code}: {}", reply.body);
}

/// Return the attributes of the first `h1` of `body`.
fn h1_tag(body: &str) -> &str {
    let (_before, rest) = body.split_once("<h1").unwrap();
    rest.split('>').next().unwrap()
}

/// Send the approval of `R-500` version 2 as `admin` with `body`.
async fn admin_approves(world: &AuditWorld, body: &str) -> Reply {
    approve(world, &world.data.admin, R500_APPROVE, body).await
}

#[tokio::test]
async fn approve_answers_the_version_page_with_the_toast_of_both_versions() {
    let world = audit_world().await;
    let before = approvals(&world).len();
    let body = format!("row_version={}", world.data.r500_v2_row_version);
    let reply = admin_approves(&world, &body).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    assert_eq!(reply.header("hx-push-url"), "/recipes/R-500/versions/2");
    assert!(
        reply.header("hx-trigger").contains("\"rowVersion\""),
        "{:?}",
        reply.headers
    );
    assert!(
        reply
            .body
            .contains("Version 2 is approved. Version 1 is retired."),
        "{}",
        reply.body
    );
    assert!(
        h1_tag(&reply.body).contains("aria-describedby=\"toast\""),
        "{}",
        reply.body
    );
    let entries = approvals(&world);
    assert_eq!(entries.len(), before.checked_add(1).unwrap(), "{entries:?}");
    let entry = entries.last().unwrap();
    assert!(entry.contains("\"content_sha256\":\""), "{entry}");
    assert!(entry.contains("\"allow_outdated\":false"), "{entry}");
}

#[tokio::test]
async fn approve_twice_with_the_same_row_version_answers_already_approved_with_one_entry() {
    let world = audit_world().await;
    let body = format!("row_version={}", world.data.r500_v2_row_version);
    let first = admin_approves(&world, &body).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let count = approvals(&world).len();
    let second = admin_approves(&world, &body).await;
    assert_eq!(second.status, StatusCode::OK, "{}", second.body);
    assert!(
        second.body.contains("Version 2 is already approved."),
        "{}",
        second.body
    );
    assert!(
        h1_tag(&second.body).contains("aria-describedby=\"toast\""),
        "{}",
        second.body
    );
    assert_eq!(second.header("hx-push-url"), "/recipes/R-500/versions/2");
    assert_eq!(approvals(&world).len(), count);
}

#[tokio::test]
async fn approve_with_allow_outdated_writes_the_flag_into_the_audit_entry() {
    let world = audit_world().await;
    let anowak = world.data.anowak_user();
    let rv = world
        .fixture
        .add_zone_to_r500(&anowak, world.data.r500_v2_row_version)
        .await;
    let reply = admin_approves(&world, &format!("row_version={rv}&allow_outdated=true")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let entries = approvals(&world);
    let entry = entries.last().unwrap();
    assert!(entry.contains("\"allow_outdated\":true"), "{entry}");
}

/// Send the approval of a draft with the outdated parameter `ZONE` and no flag.
async fn outdated_refusal(world: &AuditWorld) -> Reply {
    let anowak = world.data.anowak_user();
    let rv = world
        .fixture
        .add_zone_to_r500(&anowak, world.data.r500_v2_row_version)
        .await;
    admin_approves(world, &format!("row_version={rv}")).await
}

#[tokio::test]
async fn approve_without_allow_outdated_puts_the_line_under_the_checkbox() {
    let world = audit_world().await;
    let count = approvals(&world).len();
    let reply = outdated_refusal(&world).await;
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-024");
    let link = part_of(
        &reply.body,
        "<a class=\"target-link\" href=\"#allow_outdated\">",
        "</a>",
    );
    assert!(link.contains("E-RCP-024"), "{link}");
    let (_before, form) = reply
        .body
        .split_once("<form id=\"approve-fields\"")
        .unwrap();
    let (tag, rest) = form.split_once('>').unwrap();
    assert!(tag.contains("hx-swap-oob=\"true\""), "{tag}");
    let checkbox = rest.find("id=\"allow_outdated\"").unwrap();
    let line = part_of(
        rest,
        "<div class=\"field-error\" id=\"allow_outdated-error\">",
        "</div>",
    );
    assert!(line.contains("E-RCP-024"), "{line}");
    assert!(checkbox < rest.find(line).unwrap(), "{rest}");
    assert_eq!(approvals(&world).len(), count);
}

/// The spec asks for the label of `field-allow-outdated` in the summary line. A line of a domain
/// error has no label, so the line gets the label as its subject.
#[tokio::test]
async fn approve_without_allow_outdated_names_the_checkbox_label_in_the_summary() {
    let world = audit_world().await;
    let reply = outdated_refusal(&world).await;
    let link = part_of(
        &reply.body,
        "<a class=\"target-link\" href=\"#allow_outdated\">",
        "</a>",
    );
    assert!(link.contains("Approval with outdated parameters"), "{link}");
}
