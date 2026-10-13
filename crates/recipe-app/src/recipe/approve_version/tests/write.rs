//! Tests of the transaction of `ApproveVersion` (design D12 steps 4 to 6).

use recipe_domain::audit::AuditAction;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::{ExportHeader, HashedBody};
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlatformTypes, TypeTrees};
use recipe_domain::recipe::{ApprovalOutcome, ApprovalPolicy, Draft, VersionState};
use serde_json::{Value, json};

use super::{PLATFORM, approver, input, recipe_at, seed_with, store_recipe, zone, zone_parameter};
use crate::blocking::AppSettings;
use crate::memory::fixtures::{
    Fixture, actor, field, lock_line, meta, parameter_on, reference, rid, uid, user, vid,
};
use crate::memory::{JournalEntry, LockMode, MemoryStore};
use crate::ports::Isolation;
use crate::recipe::ApproveVersion;

/// The journal of an approval of the draft `draft` after its commit or its failure.
fn locks_of(draft: u128, retired: Option<u128>) -> Vec<JournalEntry> {
    let mut journal = vec![
        JournalEntry::Begin(Isolation::ReadOnly),
        JournalEntry::Begin(Isolation::ReadCommitted),
        lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
        lock_line(
            "recipe_version",
            LockMode::ForUpdate,
            vid(draft).to_string(),
        ),
    ];
    journal.extend(
        retired.map(|old| lock_line("recipe_version", LockMode::ForUpdate, vid(old).to_string())),
    );
    journal
}

/// Rebuild the hashed body of the draft 101 in the store and return its hash as text.
async fn expected_hash(store: &MemoryStore) -> String {
    use crate::ports::Transaction as _;
    let mut tx = store.begin_memory(Isolation::ReadOnly).unwrap();
    let stored = tx.drafts().read_draft(vid(101)).await.unwrap().unwrap();
    let draft = Draft::load(stored).unwrap();
    let units = tx
        .units()
        .live_units(draft.machine())
        .await
        .unwrap()
        .unwrap();
    let rows = tx.types().type_rows(draft.machine()).await.unwrap();
    let types = PlatformTypes::collect(draft.machine(), PLATFORM, rows).unwrap();
    let trees = TypeTrees::resolve(draft.parameters().iter().map(|p| p.plc_type()), &types);
    let header = ExportHeader {
        machine: MachineCode::parse("M1").unwrap(),
        platform: PLATFORM,
        reference: reference("R-1"),
        version: draft.number(),
    };
    HashedBody::build(&header, draft.parameters(), &units, &trees)
        .unwrap()
        .sha256()
        .unwrap()
        .to_string()
}

fn version_state(store: &MemoryStore, number: u32) -> VersionState {
    let state = store.state();
    state
        .recipe(rid(1))
        .unwrap()
        .version(number)
        .unwrap()
        .state()
}

fn simple_draft(fixture: &Fixture) {
    seed_with(
        fixture,
        PLATFORM,
        &[parameter_on(PLATFORM, 1, "T1", uid(102), 0)],
    );
}

#[tokio::test]
async fn success_writes_one_version_approved_entry_with_the_content_hash() {
    let fixture = Fixture::new();
    simple_draft(&fixture);
    let hash = expected_hash(&fixture.store).await;
    // The helper opened one transaction of its own. The journal of the use case starts after it.
    let own_lines = fixture.store.journal().len();
    let user = approver();
    let request = meta();
    let outcome = ApproveVersion::new(fixture.context())
        .execute(&user, &request, input(1, 1, false))
        .await
        .unwrap();
    assert_eq!(
        outcome,
        ApprovalOutcome::Approved {
            approved: vid(101),
            retired: None,
        }
    );
    let journal: Vec<JournalEntry> = fixture
        .store
        .journal()
        .into_iter()
        .skip(own_lines)
        .collect();
    assert_eq!(
        journal,
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
            lock_line("recipe_version", LockMode::ForUpdate, vid(101).to_string()),
        ]
    );
    assert_eq!(version_state(&fixture.store, 1), VersionState::Approved);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::VersionApproved);
    assert_eq!(entry.target().entity_type(), "recipe_version");
    assert_eq!(entry.target().entity_id(), vid(101).as_uuid());
    assert_eq!(entry.actor(), Some(user.user()));
    assert_eq!(entry.request_id(), request.request_id());
    let before = entry.before().unwrap();
    assert_eq!(field(before, "status"), json!("draft"));
    let after = entry.after().unwrap();
    assert_eq!(field(after, "status"), json!("approved"));
    assert_eq!(field(after, "allow_outdated"), json!(false));
    assert_eq!(field(after, "approved_by"), json!(user.user().to_string()));
    let stored_hash = field(after, "content_sha256");
    assert_eq!(stored_hash, Value::String(hash.clone()));
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn a_previous_approved_version_is_retired_before_the_approval_is_written() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    fixture
        .store
        .seed_parameter(vid(102), &parameter_on(PLATFORM, 1, "T1", uid(102), 0));
    let outcome = ApproveVersion::new(fixture.context())
        .execute(&approver(), &meta(), input(2, 1, false))
        .await
        .unwrap();
    assert_eq!(
        outcome,
        ApprovalOutcome::Approved {
            approved: vid(102),
            retired: Some(vid(101)),
        }
    );
    assert_eq!(fixture.store.journal(), locks_of(102, Some(101)));
    assert_eq!(version_state(&fixture.store, 1), VersionState::Retired);
    assert_eq!(version_state(&fixture.store, 2), VersionState::Approved);
    let entries = fixture.store.audit_entries();
    let actions: Vec<AuditAction> = entries.iter().map(|entry| entry.action()).collect();
    assert_eq!(
        actions,
        vec![AuditAction::VersionRetired, AuditAction::VersionApproved]
    );
    let retired = entries.first().unwrap();
    assert_eq!(retired.target().entity_id(), vid(101).as_uuid());
    assert_eq!(
        field(retired.before().unwrap(), "status"),
        json!("approved")
    );
    assert_eq!(field(retired.after().unwrap(), "status"), json!("retired"));
    let approved = entries.get(1).unwrap();
    assert_eq!(approved.target().entity_id(), vid(102).as_uuid());
}

#[tokio::test]
async fn a_stale_row_version_gives_web_001_and_writes_nothing() {
    let fixture = Fixture::new();
    simple_draft(&fixture);
    let error = ApproveVersion::new(fixture.context())
        .execute(&approver(), &meta(), input(1, 5, false))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_001);
    assert_eq!(fixture.store.journal(), locks_of(101, None));
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(version_state(&fixture.store, 1), VersionState::LiveDraft);
}

#[tokio::test]
async fn the_creator_gives_rcp_015_and_writes_nothing() {
    let fixture = Fixture::new();
    simple_draft(&fixture);
    // `jkowalski` created the seeded draft.
    let error = ApproveVersion::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta(), input(1, 1, false))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_015);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(version_state(&fixture.store, 1), VersionState::LiveDraft);
}

#[tokio::test]
async fn an_editor_gives_rcp_015_and_a_third_user_passes() {
    let fixture = Fixture::new();
    simple_draft(&fixture);
    fixture.store.seed_editor(vid(101), user(3));
    let use_case = ApproveVersion::new(fixture.context());
    let error = use_case
        .execute(&approver(), &meta(), input(1, 1, false))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_015);
    assert!(fixture.store.audit_entries().is_empty());
    let outcome = use_case
        .execute(&actor(Role::Administrator), &meta(), input(1, 1, false))
        .await
        .unwrap();
    assert!(matches!(outcome, ApprovalOutcome::Approved { .. }));
    assert_eq!(version_state(&fixture.store, 1), VersionState::Approved);
}

#[tokio::test]
async fn without_separation_of_duties_the_creator_passes() {
    let fixture = Fixture::new();
    simple_draft(&fixture);
    let policy = AppSettings {
        approval_policy: ApprovalPolicy {
            separation_of_duties: false,
        },
        ..AppSettings::default()
    };
    let outcome = ApproveVersion::new(fixture.context_with(policy))
        .execute(&actor(Role::ProcessEngineer), &meta(), input(1, 1, false))
        .await
        .unwrap();
    assert!(matches!(outcome, ApprovalOutcome::Approved { .. }));
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn a_revision_that_commits_after_the_check_gives_rcp_024() {
    let fixture = Fixture::new();
    fixture.store.seed_type(&zone(900, 1, None));
    seed_with(&fixture, PLATFORM, &[zone_parameter(1, "Z1", true)]);
    let context = fixture.context_racing(|store| store.seed_type(&zone(901, 2, Some(900))));
    let error = ApproveVersion::new(context)
        .execute(&approver(), &meta(), input(1, 1, false))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_024);
    assert_eq!(fixture.store.journal(), locks_of(101, None));
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(version_state(&fixture.store, 1), VersionState::LiveDraft);
}

#[tokio::test]
async fn a_racing_edit_and_a_racing_revision_give_web_001_and_not_a_limit_error() {
    let fixture = Fixture::new();
    fixture.store.seed_type(&zone(900, 1, None));
    seed_with(&fixture, PLATFORM, &[zone_parameter(1, "Z1", true)]);
    // Another request edits the draft (row version 2) and a struct revision commits.
    let context = fixture.context_racing(|store| {
        let edited = recipe_at(&[(101, VersionState::LiveDraft, 2)]);
        store_recipe(store, PLATFORM, &edited);
        store.seed_type(&zone(901, 2, Some(900)));
    });
    let error = ApproveVersion::new(context)
        .execute(&approver(), &meta(), input(1, 1, false))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_001);
    assert_eq!(fixture.store.journal(), locks_of(101, None));
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(version_state(&fixture.store, 1), VersionState::LiveDraft);
}

#[tokio::test]
async fn a_racing_approval_and_a_racing_revision_give_already_approved() {
    let fixture = Fixture::new();
    fixture.store.seed_type(&zone(900, 1, None));
    seed_with(&fixture, PLATFORM, &[zone_parameter(1, "Z1", true)]);
    let context = fixture.context_racing(|store| {
        let approved = recipe_at(&[(101, VersionState::Approved, 2)]);
        store_recipe(store, PLATFORM, &approved);
        store.seed_type(&zone(901, 2, Some(900)));
    });
    let outcome = ApproveVersion::new(context)
        .execute(&approver(), &meta(), input(1, 1, false))
        .await
        .unwrap();
    assert_eq!(outcome, ApprovalOutcome::AlreadyApproved);
    assert_eq!(fixture.store.journal(), locks_of(101, None));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn allow_outdated_approves_and_records_the_flag() {
    let fixture = Fixture::new();
    fixture.store.seed_type(&zone(900, 1, None));
    fixture.store.seed_type(&zone(901, 2, Some(900)));
    seed_with(&fixture, PLATFORM, &[zone_parameter(1, "Z1", true)]);
    let outcome = ApproveVersion::new(fixture.context())
        .execute(&approver(), &meta(), input(1, 1, true))
        .await
        .unwrap();
    assert!(matches!(outcome, ApprovalOutcome::Approved { .. }));
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let after = entries.first().unwrap().after().unwrap();
    assert_eq!(field(after, "allow_outdated"), json!(true));
}

#[tokio::test]
async fn an_approval_by_another_request_before_the_locks_gives_already_approved() {
    let fixture = Fixture::new();
    simple_draft(&fixture);
    let context = fixture.context_racing(|store| {
        let approved = recipe_at(&[(101, VersionState::Approved, 2)]);
        store_recipe(store, PLATFORM, &approved);
    });
    let outcome = ApproveVersion::new(context)
        .execute(&approver(), &meta(), input(1, 1, false))
        .await
        .unwrap();
    assert_eq!(outcome, ApprovalOutcome::AlreadyApproved);
    assert_eq!(fixture.store.journal(), locks_of(101, None));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_failed_commit_keeps_the_draft() {
    let fixture = Fixture::new();
    simple_draft(&fixture);
    fixture
        .store
        .fail_append_audit(crate::memory::Fault::Unavailable);
    let error = ApproveVersion::new(fixture.context())
        .execute(&approver(), &meta(), input(1, 1, false))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::AUD_001);
    assert_eq!(version_state(&fixture.store, 1), VersionState::LiveDraft);
    assert!(fixture.store.audit_entries().is_empty());
}
