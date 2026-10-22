//! Tests of three answers of `POST /recipes/{ref}/versions/{n}/approve` that the other test files
//! do not cover (ADR-0049 (b) and (e), `api.md` section 5.3, design D3 and D9):
//!
//! - the success answer without HTMX,
//! - `E-RCP-022` and `E-RCP-023` of a draft that the store seeds,
//! - `E-RCP-024` from the second check in the transaction of `ApproveVersion`.

use axum::http::StatusCode;
use recipe_app::memory::{JournalEntry, Lock, LockMode};

use super::approve_tests::{R500_APPROVE, approvals, approve, assert_refusal};
use super::preview_tests::{assert_approve_button_id, part_of};
use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::{assert_clean_html, field_value};
use crate::test_support::plant::ENGLISH;
use crate::test_support::{Reply, post_fields};

/// The toast of the first approval of `R-500` version 2.
const APPROVED_TOAST: &str = "Version 2 is approved. Version 1 is retired.";

/// The start of the status region `#toast` with the toast inside it and no gap between them.
const TOAST_START: &str =
    "<div id=\"toast\" role=\"status\" aria-live=\"polite\"><div class=\"toast\">";

/// Return the number of the audit entries of `world`.
fn entry_count(world: &AuditWorld) -> usize {
    world.fixture.store.audit_entries().len()
}

/// ADR-0049 (e), `api.md` section 5.3, and `design-system.md` section 10.1: without HTMX, the answer
/// is the full version page. The toast stands in the status region `#toast`. The `h1` names the
/// toast. A full page gives the `h1` no `autofocus`.
#[tokio::test]
async fn approve_without_htmx_answers_the_full_page_with_the_toast_in_the_status_region() {
    let world = audit_world().await;
    let before = approvals(&world).len();
    let body = format!("row_version={}", world.data.r500_v2_row_version);
    let router = world.fixture.router();
    let session = Some(&world.data.admin);
    let reply = post_fields(&router, R500_APPROVE, session, &[ENGLISH], &body).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_clean_html(&reply.body);
    assert!(reply.body.contains("<html"), "{}", reply.body);
    let region = part_of(&reply.body, TOAST_START, "</span>");
    assert!(region.ends_with(APPROVED_TOAST), "{region}");
    assert_eq!(
        reply.body.matches(APPROVED_TOAST).count(),
        1,
        "{}",
        reply.body
    );
    let heading = "<h1 aria-describedby=\"toast\">Version 2 of recipe R-500</h1>";
    assert!(reply.body.contains(heading), "{}", reply.body);
    assert_eq!(
        approvals(&world).len(),
        before.checked_add(1).unwrap(),
        "the approval must write one entry"
    );
}

/// Seed the `INT` parameter `(n, tag, address)` on the draft `R-500` version 2 and send its
/// approval as `admin`. Return the answer and the number of the audit entries before it.
async fn approve_with_seeded_address(
    world: &AuditWorld,
    parameter: (u32, &'static str, &'static str),
) -> (Reply, usize) {
    world.fixture.seed_int_on_r500(&world.data, parameter);
    let entries = entry_count(world);
    let rv = world.row_version_of("R-500", 2).await;
    let body = format!("row_version={rv}");
    let reply = approve(world, &world.data.admin, R500_APPROVE, &body).await;
    (reply, entries)
}

/// Design D9: an overlap of two S7 addresses gives `422 E-RCP-022` with its line.
#[tokio::test]
async fn approve_of_a_draft_with_an_address_overlap_answers_e_rcp_022_and_writes_nothing() {
    let world = audit_world().await;
    // `SPEED_TABLE` holds the bytes 10 to 25 of `DB10`, so `DB10.DBW24` overlaps it.
    let (reply, entries) = approve_with_seeded_address(&world, (1, "CLASH", "DB10.DBW24")).await;
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-022");
    let line = "Address DB10.DBW24 of CLASH overlaps with SPEED_TABLE.";
    assert!(reply.body.contains(line), "{}", reply.body);
    assert_eq!(entry_count(&world), entries);
}

/// Design D9: an `INT` at an odd byte gives `422 E-RCP-023` with its line.
#[tokio::test]
async fn approve_of_a_draft_with_a_misaligned_address_answers_e_rcp_023_and_writes_nothing() {
    let world = audit_world().await;
    let (reply, entries) = approve_with_seeded_address(&world, (2, "ODD", "DB13.DBW1")).await;
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-023");
    let line = "Address DB13.DBW1 of ODD must start at an even byte.";
    assert!(reply.body.contains(line), "{}", reply.body);
    assert_eq!(entry_count(&world), entries);
}

/// Return true when `entries` hold the `FOR UPDATE` lock of the version row `version`.
fn locks_version(entries: &[JournalEntry], version: &str) -> bool {
    entries.iter().any(|entry| {
        matches!(
            entry,
            JournalEntry::Lock(Lock { table: "recipe_version", mode: LockMode::ForUpdate, key })
                if key == version
        )
    })
}

/// Step 4 of design D12 of the archived change `app-use-cases-and-ports`: a struct revision commits
/// after the first check and before the locks. Only the second check in the transaction sees it.
/// The answer equals the answer of the first check: the line in the summary, and the form out of
/// band with the line under the checkbox (design D3).
#[tokio::test]
async fn approve_answers_e_rcp_024_of_the_second_check_like_that_of_the_first_check() {
    let world = audit_world().await;
    let entries = entry_count(&world);
    let sent = world.data.r500_v2_row_version;
    world
        .fixture
        .supersede_cell_at_the_next_draft_read(&world.data.recipes.plant);
    let journal = world.fixture.store.journal().len();
    let body = format!("row_version={sent}");
    let reply = approve(&world, &world.data.admin, R500_APPROVE, &body).await;
    let new_lines: Vec<JournalEntry> = world
        .fixture
        .store
        .journal()
        .into_iter()
        .skip(journal)
        .collect();
    // The first check passed, so the transaction opened and locked the draft row.
    let draft = world.data.r500_v2.to_string();
    assert!(locks_version(&new_lines, &draft), "{new_lines:?}");
    assert_refusal(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-RCP-024");
    let link = part_of(
        &reply.body,
        "<a class=\"target-link\" href=\"#allow_outdated\">",
        "</a>",
    );
    assert!(link.contains("E-RCP-024"), "{link}");
    assert!(link.contains("Approval with outdated parameters"), "{link}");
    let (_before, form) = reply
        .body
        .split_once("<form id=\"approve-fields\"")
        .unwrap();
    let (tag, _rest) = form.split_once('>').unwrap();
    assert!(tag.contains("hx-swap-oob=\"true\""), "{tag}");
    assert_eq!(field_value(form, "row_version"), sent.to_string(), "{form}");
    let line = part_of(
        form,
        "<div class=\"field-error\" id=\"allow_outdated-error\">",
        "</div>",
    );
    assert!(line.contains("E-RCP-024"), "{line}");
    assert!(line.contains("LINE_A uses revision 1 of ST_Cell"), "{line}");
    // The button keeps its fixed id in the answer, once in the whole body (ADR-0049 (j)).
    let whole_form = part_of(&reply.body, "<form id=\"approve-fields\"", "</form>");
    assert_approve_button_id(&reply.body, whole_form);
    assert_eq!(entry_count(&world), entries);
}
