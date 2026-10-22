//! Tests of the error answers of `POST /recipes/{ref}/versions/{n}/approve`: one HTMX test for
//! each code of design D9 that the test data can cause (task 10.2, ADR-0047 (b), spec "Errors of
//! the approval").

use std::time::Duration;

use axum::http::StatusCode;
use recipe_app::blocking::{AppSettings, Blocking, DiffSettings};
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineId, UnitPath};
use recipe_domain::recipe::{RecipeVersionId, VersionState};

use super::approve_tests::{R500_APPROVE, approvals, approve, assert_refusal};
use super::preview_tests::part_of;
use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::recipes::test_world::{
    seed_parameter_on_unit, seed_parameter_on_unknown_unit, seed_recipe_of_m20,
};
use crate::test_support::recipes::M20_ID;
use crate::test_support::{Fixture, Reply, capture_logs};

/// The `timeout.validation` of a world whose preview read fails. The wait for a diff permit
/// ends at this limit, and the snapshot check of the approval still completes in it.
const SHORT_VALIDATION: Duration = Duration::from_millis(500);
/// The `timeout.export` of that world. The approval does not use it.
const EXPORT_LIMIT: Duration = Duration::from_secs(10);
/// The number of the lines that a summary shows (ADR-0041 (d)).
const SHOWN_LINES: usize = 20;
/// The number of the parameters without a limit: 5 more than the shown lines.
const UNLIMITED_COUNT: u32 = 25;

#[tokio::test]
async fn approve_answers_the_codes_of_the_recipe_and_the_version() {
    let world = audit_world().await;
    let cases = [
        (
            "/recipes/R-999/versions/2/approve",
            StatusCode::NOT_FOUND,
            "E-WEB-005",
        ),
        (
            "/recipes/R-600/versions/1/approve",
            StatusCode::CONFLICT,
            "E-RCP-007",
        ),
        (
            "/recipes/R-700/versions/1/approve",
            StatusCode::CONFLICT,
            "E-RCP-012",
        ),
    ];
    for (path, status, code) in cases {
        let reply = approve(&world, &world.data.admin, path, "row_version=1").await;
        assert_refusal(&reply, status, code);
    }
}

#[tokio::test]
async fn approve_of_the_creator_answers_the_separation_of_duties_and_writes_nothing() {
    let world = audit_world().await;
    let entries = world.fixture.store.audit_entries().len();
    let body = format!("row_version={}", world.data.r500_v2_row_version);
    let reply = approve(&world, &world.data.anowak, R500_APPROVE, &body).await;
    assert_refusal(&reply, StatusCode::CONFLICT, "E-RCP-015");
    assert_eq!(world.fixture.store.audit_entries().len(), entries);
}

#[tokio::test]
async fn approve_of_an_editor_of_the_draft_answers_the_separation_of_duties() {
    let world = audit_world().await;
    let jsmith = world.data.jsmith_user();
    let rv = world
        .fixture
        .add_zone_to_r500(&jsmith, world.data.r500_v2_row_version)
        .await;
    let body = format!("row_version={rv}&allow_outdated=true");
    let reply = approve(&world, &world.data.jsmith, R500_APPROVE, &body).await;
    assert_refusal(&reply, StatusCode::CONFLICT, "E-RCP-015");
}

/// Return the number of the summary lines of `code` in `body`.
fn lines_of(body: &str, code: &str) -> usize {
    body.matches(&format!("<li><span class=\"alert-code\">{code}</span>"))
        .count()
}

#[tokio::test]
async fn approve_lists_each_missing_limit_on_its_own_line() {
    let world = audit_world().await;
    let path = "/recipes/R-100/versions/3/approve";
    let reply = approve(&world, &world.data.jsmith, path, "row_version=1").await;
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-016");
    let shown = lines_of(&reply.body, "E-RCP-016");
    assert!(shown > 1, "{}", reply.body);
    // The summary shows the first 20 lines and counts the other lines (ADR-0041 (d)).
    let more = reply
        .body
        .split_once("Other errors: ")
        .map_or(0, |(_before, rest)| {
            rest.split('.').next().unwrap().parse::<usize>().unwrap()
        });
    let total = shown.checked_add(more).unwrap();
    assert!(
        reply.body.contains(&format!("Errors: {total}.")),
        "{}",
        reply.body
    );
}

/// Seed `count` `INT` parameters `P01` and up on the unit `U1` of `M20`. None has a limit.
pub(super) fn seed_unlimited_ints(fixture: &Fixture, version: RecipeVersionId, count: u32) {
    let unit = {
        let state = fixture.store.state();
        let machine = state.machine(MachineId::from(M20_ID)).unwrap();
        machine.unit(&UnitPath::parse("U1").unwrap()).unwrap().id()
    };
    for n in 1..=count {
        seed_parameter_on_unit(fixture, version, unit, (n, &format!("P{n:02}")));
    }
}

/// Design D9 and ADR-0041 (d): the summary of a list shows 20 lines and the line of the others.
#[tokio::test]
async fn approve_caps_the_list_of_missing_limits_at_20_lines_and_counts_the_other_five() {
    let world = audit_world().await;
    let ids = seed_recipe_of_m20(&world.fixture, (802, "R-802"), &[VersionState::LiveDraft]);
    let draft = *ids.first().unwrap();
    seed_unlimited_ints(&world.fixture, draft, UNLIMITED_COUNT);
    let count = approvals(&world).len();
    let path = "/recipes/R-802/versions/1/approve";
    let reply = approve(&world, &world.data.jsmith, path, "row_version=1").await;
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-016");
    let body = &reply.body;
    assert_eq!(lines_of(body, "E-RCP-016"), SHOWN_LINES, "{body}");
    assert!(body.contains("Errors: 25."), "{body}");
    let more = "<li>Other errors: 5. Each one stands under its field.</li>";
    assert_eq!(body.matches(more).count(), 1, "{body}");
    assert!(body.contains("P20 has no process limit."), "{body}");
    assert!(!body.contains("P21 has no process limit."), "{body}");
    assert_eq!(approvals(&world).len(), count);
}

#[tokio::test]
async fn approve_lists_each_item_of_a_failed_snapshot_check_on_its_own_line() {
    let world = audit_world().await;
    let ids = seed_recipe_of_m20(&world.fixture, (801, "R-801"), &[VersionState::LiveDraft]);
    let draft = *ids.first().unwrap();
    seed_parameter_on_unknown_unit(&world.fixture, draft, (1, "P_ONE"));
    seed_parameter_on_unknown_unit(&world.fixture, draft, (2, "P_TWO"));
    let path = "/recipes/R-801/versions/1/approve";
    let reply = approve(&world, &world.data.admin, path, "row_version=1").await;
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-020");
    assert!(reply.body.contains("Errors: 2."), "{}", reply.body);
    for tag in ["P_ONE", "P_TWO"] {
        let line = format!(
            "<span class=\"alert-code\">E-PLT-020</span> <span class=\"data\">{tag}</span>"
        );
        assert!(reply.body.contains(&line), "{tag}: {}", reply.body);
    }
}

/// Build the approval world with no permit of the diff gate (`diff.permits` 0).
///
/// The second read of `GetApprovalPreview` then fails with `E-WEB-009` at the deadline.
async fn world_without_diff_permits() -> AuditWorld {
    let defaults = AppSettings::default();
    let fixture = Fixture::with_app_settings(AppSettings {
        blocking: Blocking::new(SHORT_VALIDATION, EXPORT_LIMIT),
        diff: DiffSettings {
            permits: 0,
            ..defaults.diff
        },
        ..defaults
    });
    let data = fixture.seed_approval_recipes().await;
    let operator = fixture.seed_session(3, Role::Operator, false);
    AuditWorld {
        fixture,
        data,
        operator,
    }
}

/// Send the approval of `R-500` version 2 with the outdated parameter `ZONE` and no flag.
async fn outdated_approval(world: &AuditWorld) -> Reply {
    let anowak = world.data.anowak_user();
    let rv = world
        .fixture
        .add_zone_to_r500(&anowak, world.data.r500_v2_row_version)
        .await;
    let body = format!("row_version={rv}");
    approve(world, &world.data.admin, R500_APPROVE, &body).await
}

#[tokio::test]
async fn approve_without_allow_outdated_keeps_the_line_when_the_preview_read_fails() {
    let world = world_without_diff_permits().await;
    let reply = outdated_approval(&world).await;
    assert_outdated_line_without_form(&reply);
    assert!(!reply.body.contains("E-WEB-009"), "{}", reply.body);
}

/// The log pattern of `conflict_after_a_failed_read_is_still_e_web_001_and_logs_the_failure_once`.
#[tokio::test]
async fn approve_without_allow_outdated_logs_the_failed_preview_read_once() {
    let world = world_without_diff_permits().await;
    let (logs, _guard) = capture_logs();
    let reply = outdated_approval(&world).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let text = logs.text();
    assert_eq!(text.matches("request failed").count(), 1, "{text}");
    assert!(text.contains("E-RCP-024"), "{text}");
    assert!(text.contains("the read of the form failed"), "{text}");
    // A second log call of the failure, with any message, writes its text a second time.
    assert_eq!(
        text.matches("The request took too long").count(),
        1,
        "{text}"
    );
    assert!(text.contains("ERROR"), "{text}");
}

/// Assert the answer `E-RCP-024` without a form: the line and its label in the summary with no
/// link, no out-of-band swap of the approve form, and no approve button (design D3).
///
/// The answer holds no checkbox, so a link to `#allow_outdated` has no target (step-7 item S7-10).
fn assert_outdated_line_without_form(reply: &Reply) {
    assert_refusal(reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-024");
    assert!(
        !reply.body.contains("href=\"#allow_outdated\""),
        "{}",
        reply.body
    );
    let line = part_of(
        &reply.body,
        "<li><span class=\"alert-code\">E-RCP-024</span>",
        "</li>",
    );
    assert!(line.contains("Approval with outdated parameters"), "{line}");
    assert!(
        !reply.body.contains("id=\"approve-fields\""),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("<form"), "{}", reply.body);
    assert!(!reply.body.contains("Approve version 2"), "{}", reply.body);
}

/// The preview shows no form for a stale `row_version`, so the answer swaps no form either.
#[tokio::test]
async fn approve_with_a_stale_row_version_and_an_outdated_parameter_swaps_no_form() {
    let world = audit_world().await;
    let anowak = world.data.anowak_user();
    let stale = world.data.r500_v2_row_version;
    let fresh = world.fixture.add_zone_to_r500(&anowak, stale).await;
    assert_ne!(fresh, stale);
    let body = format!("row_version={stale}");
    let reply = approve(&world, &world.data.admin, R500_APPROVE, &body).await;
    assert_outdated_line_without_form(&reply);
}

/// The preview of an editor of the draft has `may_approve` false (`E-RCP-015`), so the answer
/// swaps no form.
#[tokio::test]
async fn approve_of_an_editor_of_the_draft_with_an_outdated_parameter_swaps_no_form() {
    let world = audit_world().await;
    let jsmith = world.data.jsmith_user();
    let rv = world
        .fixture
        .add_zone_to_r500(&jsmith, world.data.r500_v2_row_version)
        .await;
    let body = format!("row_version={rv}");
    let reply = approve(&world, &world.data.jsmith, R500_APPROVE, &body).await;
    assert_outdated_line_without_form(&reply);
}

#[tokio::test]
async fn approve_of_a_draft_without_parameters_answers_its_code() {
    let world = audit_world().await;
    seed_recipe_of_m20(&world.fixture, (800, "R-800"), &[VersionState::LiveDraft]);
    let count = approvals(&world).len();
    let path = "/recipes/R-800/versions/1/approve";
    let reply = approve(&world, &world.data.admin, path, "row_version=1").await;
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-008");
    assert_eq!(approvals(&world).len(), count);
}
