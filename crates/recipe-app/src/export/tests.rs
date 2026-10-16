//! Tests of the use cases `ExportApprovedVersion` and `ExportVersionByNumber`.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use recipe_domain::audit::{AuditAction, AuditEntry, AuditTarget, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{Machine, PlcPlatform};
use recipe_domain::recipe::{RecipeDeleter, VersionState, VersionStatus};
use serde_json::json;

use super::{
    ExportApprovedInput, ExportApprovedVersion, ExportByNumberInput, ExportVersionByNumber,
    ExportedVersion,
};
use crate::auth::Operation;
use crate::blocking::{AppSettings, Blocking};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, StorageError, TimeoutStage};
use crate::memory::fixtures::{
    Fixture, actor, field, lock_line, meta, parameter_on, recipe_with, reference, rid,
    second_engineer, text, timestamp, uid, user, vid,
};
use crate::memory::{JournalEntry, LockMode, MemoryStore};
use crate::ports::{
    AttemptRepository, AuditWritten, DraftRepository, Isolation, NonEmpty, PlantRepository,
    RecipeReads, RecipeRepository, SessionRepository, Transaction, TypeCatalog, UnitLookup,
    UnitOfWork, UserRepository,
};
use crate::recipe::{
    ApproveVersion, ApproveVersionInput, RenameRecipe, RenameRecipeInput, RetireVersion,
    RetireVersionInput,
};

const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;

fn approved_input(reference_text: &str) -> ExportApprovedInput {
    ExportApprovedInput {
        reference: reference(reference_text),
    }
}

fn number_input(reference_text: &str, number: u32) -> ExportByNumberInput {
    ExportByNumberInput {
        reference: reference(reference_text),
        number,
    }
}

async fn export_approved(fixture: &Fixture, role: Role) -> Result<ExportedVersion, AppError> {
    ExportApprovedVersion::new(fixture.context())
        .execute(&actor(role), &meta(), approved_input("R-1"))
        .await
}

async fn export_number(
    fixture: &Fixture,
    role: Role,
    number: u32,
) -> Result<ExportedVersion, AppError> {
    ExportVersionByNumber::new(fixture.context())
        .execute(&actor(role), &meta(), number_input("R-1", number))
        .await
}

/// Store the machine `M1`, the draft 101 of `R-1`, and the parameter `T1`. Then approve it.
async fn approved_fixture() -> (Fixture, Machine) {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let parameter = parameter_on(PLATFORM, 1, "T1", uid(102), 0);
    fixture.store.seed_parameter(vid(101), &parameter);
    ApproveVersion::new(fixture.context())
        .execute(
            &second_engineer(),
            &meta(),
            ApproveVersionInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: 1,
                allow_outdated: false,
            },
        )
        .await
        .unwrap();
    (fixture, machine)
}

/// Store recipe `R-1` of `machine` as a deleted recipe whose draft is discarded.
fn seed_deleted(fixture: &Fixture, machine: &Machine) {
    let (mut recipe, _) = fixture.seed_draft(machine, 1, "R-1");
    recipe
        .delete(RecipeDeleter::Administrator, timestamp())
        .unwrap();
    fixture
        .store
        .seed_recipe(&recipe, machine.platform(), user(2));
}

/// Return the hash text that the `VersionApproved` entry stored.
fn stored_hash(store: &MemoryStore) -> serde_json::Value {
    let entries = store.audit_entries();
    let approved = entries
        .iter()
        .find(|entry| entry.action() == AuditAction::VersionApproved)
        .unwrap();
    field(approved.after().unwrap(), "content_sha256")
}

#[test]
fn the_use_cases_declare_their_operations() {
    assert_eq!(
        ExportApprovedVersion::OPERATION,
        Operation::ExportApprovedVersion
    );
    assert_eq!(
        ExportVersionByNumber::OPERATION,
        Operation::ExportVersionByNumber
    );
}

#[tokio::test]
async fn an_export_after_the_approval_finds_the_stored_hash_and_writes_one_entry() {
    let (fixture, machine) = approved_fixture().await;
    let own_lines = fixture.store.journal().len();
    let before = fixture.store.audit_entries().len();
    let exported = export_approved(&fixture, Role::Operator).await.unwrap();
    assert!(exported.loadable());
    assert_eq!(exported.status, VersionStatus::Approved);
    assert_eq!(
        json!(exported.content_sha256.to_string()),
        stored_hash(&fixture.store)
    );
    assert_eq!(exported.body.sha256().unwrap(), exported.content_sha256);
    assert_eq!(&exported.machine_name, machine.name());
    assert_eq!(exported.recipe_name, text("Recipe"));
    assert_eq!(exported.approved_by, Some(second_engineer().user()));
    assert!(exported.approved_at.is_some());
    assert_eq!(exported.exported_by, *actor(Role::Operator).login());
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
            lock_line("recipe_version", LockMode::ForShare, vid(101).to_string()),
        ]
    );
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), before.checked_add(1).unwrap());
    let entry = entries.last().unwrap();
    assert_eq!(entry.action(), AuditAction::VersionExported);
    assert_eq!(
        entry.target(),
        &AuditTarget::recipe_version(rid(1), vid(101))
    );
    assert_eq!(entry.before(), None);
    assert_eq!(
        entry.after(),
        Some(&json!({
            "version_id": vid(101).to_string(),
            "schema_version": "1",
            "content_sha256": exported.content_sha256.to_string(),
        }))
    );
}

#[tokio::test]
async fn a_retired_version_keeps_its_hash_and_is_not_loadable() {
    let (fixture, _machine) = approved_fixture().await;
    let approved = export_approved(&fixture, Role::Operator).await.unwrap();
    let row_version = fixture.store.state().version_row_version(vid(101)).unwrap();
    RetireVersion::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            RetireVersionInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: row_version,
            },
        )
        .await
        .unwrap();
    let retired = export_number(&fixture, Role::ProcessEngineer, 1)
        .await
        .unwrap();
    assert!(!retired.loadable());
    assert_eq!(retired.status, VersionStatus::Retired);
    assert_eq!(retired.content_sha256, approved.content_sha256);
    assert_eq!(retired.body, approved.body);
    let error = export_approved(&fixture, Role::Operator).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::EXP_001);
}

#[tokio::test]
async fn a_rename_of_the_recipe_keeps_the_hash_and_changes_the_name() {
    let (fixture, _machine) = approved_fixture().await;
    let before = export_approved(&fixture, Role::Operator).await.unwrap();
    RenameRecipe::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            RenameRecipeInput {
                reference: reference("R-1"),
                name: text("Bottle 0.5 litre"),
            },
        )
        .await
        .unwrap();
    let after = export_approved(&fixture, Role::Operator).await.unwrap();
    assert_eq!(after.content_sha256, before.content_sha256);
    assert_eq!(after.recipe_name, text("Bottle 0.5 litre"));
}

#[tokio::test]
async fn a_recipe_without_an_approved_version_gives_exp_001() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let error = export_approved(&fixture, Role::Operator).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::EXP_001);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn the_approved_route_of_a_deleted_recipe_gives_exp_001() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    seed_deleted(&fixture, &machine);
    let error = export_approved(&fixture, Role::Operator).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::EXP_001);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn the_route_by_number_of_a_deleted_recipe_gives_rcp_012() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    seed_deleted(&fixture, &machine);
    let error = export_number(&fixture, Role::Administrator, 1)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert!(fixture.store.audit_entries().is_empty());
}

/// Retire version 1 of the approved fixture.
async fn retire_version_one(fixture: &Fixture) {
    let row_version = fixture.store.state().version_row_version(vid(101)).unwrap();
    RetireVersion::new(fixture.context())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            RetireVersionInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: row_version,
            },
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn a_recipe_deleted_between_the_read_and_the_lock_gives_rcp_012_on_the_route_by_number() {
    let (fixture, _machine) = approved_fixture().await;
    retire_version_one(&fixture).await;
    let entries_before = fixture.store.audit_entries().len();
    let context = fixture.context_racing(|store| {
        store.update_state(|state, now| {
            if let Some(row) = state.recipes.get_mut(&rid(1)) {
                row.deleted_at = Some(now);
            }
        });
    });
    let error = ExportVersionByNumber::new(context)
        .execute(&actor(Role::Administrator), &meta(), number_input("R-1", 1))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), entries_before);
    assert!(
        entries
            .iter()
            .all(|entry| entry.action() != AuditAction::VersionExported)
    );
}

#[tokio::test]
async fn a_rename_between_the_read_and_the_lock_gives_the_new_name_and_the_same_hash() {
    let (fixture, _machine) = approved_fixture().await;
    let before = export_approved(&fixture, Role::Operator).await.unwrap();
    let context = fixture.context_racing(|store| {
        store.update_state(|state, _| {
            if let Some(row) = state.recipes.get_mut(&rid(1)) {
                row.name = text("Renamed late");
            }
        });
    });
    let after = ExportApprovedVersion::new(context)
        .execute(&actor(Role::Operator), &meta(), approved_input("R-1"))
        .await
        .unwrap();
    assert_eq!(after.recipe_name, text("Renamed late"));
    assert_eq!(after.content_sha256, before.content_sha256);
}

#[tokio::test]
async fn a_live_draft_gives_exp_003() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let error = export_number(&fixture, Role::ProcessEngineer, 1)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::EXP_003);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_discarded_draft_gives_exp_003() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    let recipe = recipe_with((1, 1), "R-1", &[(101, VersionState::Discarded)]);
    fixture.store.seed_recipe(&recipe, PLATFORM, user(2));
    let error = export_number(&fixture, Role::ProcessEngineer, 1)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::EXP_003);
    assert_eq!(machine.platform(), PLATFORM);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_stored_hash_that_differs_gives_exp_002_and_no_entry() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let error = export_approved(&fixture, Role::Operator).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::EXP_002);
    assert!(fixture.store.audit_entries().is_empty());
    let by_number = export_number(&fixture, Role::Administrator, 1)
        .await
        .unwrap_err();
    assert_eq!(by_number.code(), ErrorCode::EXP_002);
}

#[tokio::test]
async fn an_unknown_reference_or_number_gives_web_005() {
    let (fixture, _machine) = approved_fixture().await;
    let number = export_number(&fixture, Role::Administrator, 9)
        .await
        .unwrap_err();
    assert_eq!(number.code(), ErrorCode::WEB_005);
    let by_number = ExportVersionByNumber::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), number_input("R-9", 1))
        .await
        .unwrap_err();
    assert_eq!(by_number.code(), ErrorCode::WEB_005);
    let approved = ExportApprovedVersion::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), approved_input("R-9"))
        .await
        .unwrap_err();
    assert_eq!(approved.code(), ErrorCode::WEB_005);
}

#[tokio::test]
async fn an_operator_is_forbidden_on_the_route_by_number_and_allowed_on_the_approved_route() {
    let (fixture, _machine) = approved_fixture().await;
    let own_lines = fixture.store.journal().len();
    let events_before = fixture.sink.events().len();
    let before = fixture.store.audit_entries().len();
    let error = export_number(&fixture, Role::Operator, 1)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), events_before.checked_add(1).unwrap());
    assert_eq!(events.last().unwrap().kind(), SecurityEventKind::Forbidden);
    assert_eq!(fixture.store.journal().len(), own_lines);
    assert_eq!(fixture.store.audit_entries().len(), before);
    export_approved(&fixture, Role::Operator).await.unwrap();
}

/// A unit of work whose transactions wait much longer than any limit of the tests in `finish`.
struct SlowCommitStore {
    inner: MemoryStore,
}

#[async_trait]
impl UnitOfWork for SlowCommitStore {
    async fn begin(&self, isolation: Isolation) -> Result<Box<dyn Transaction>, StorageError> {
        let inner = self.inner.begin(isolation).await?;
        Ok(Box::new(SlowCommitTransaction { inner }))
    }
}

/// The transaction of [`SlowCommitStore`]. It forwards every call except the delay of `finish`.
struct SlowCommitTransaction {
    inner: Box<dyn Transaction>,
}

#[async_trait]
impl Transaction for SlowCommitTransaction {
    fn recipes(&mut self) -> &mut dyn RecipeRepository {
        self.inner.recipes()
    }

    fn recipe_reads(&mut self) -> &mut dyn RecipeReads {
        self.inner.recipe_reads()
    }

    fn drafts(&mut self) -> &mut dyn DraftRepository {
        self.inner.drafts()
    }

    fn plant(&mut self) -> &mut dyn PlantRepository {
        self.inner.plant()
    }

    fn units(&mut self) -> &mut dyn UnitLookup {
        self.inner.units()
    }

    fn types(&mut self) -> &mut dyn TypeCatalog {
        self.inner.types()
    }

    fn users(&mut self) -> &mut dyn UserRepository {
        self.inner.users()
    }

    fn sessions(&mut self) -> &mut dyn SessionRepository {
        self.inner.sessions()
    }

    fn attempts(&mut self) -> &mut dyn AttemptRepository {
        self.inner.attempts()
    }

    async fn append_audit(&mut self, entries: &NonEmpty<AuditEntry>) -> Result<(), StorageError> {
        self.inner.append_audit(entries).await
    }

    async fn finish(self: Box<Self>, written: AuditWritten) -> Result<(), StorageError> {
        tokio::time::sleep(Duration::from_secs(60)).await;
        self.inner.finish(written).await
    }
}

#[tokio::test(start_paused = true)]
async fn a_slow_commit_runs_after_the_export_limit_and_keeps_the_entry() {
    let (fixture, _machine) = approved_fixture().await;
    let before = fixture.store.audit_entries().len();
    let settings = AppSettings {
        blocking: Blocking::new(Duration::from_secs(10), Duration::from_secs(1)),
        ..AppSettings::default()
    };
    let context = AppContext {
        uow: Arc::new(SlowCommitStore {
            inner: fixture.store.clone(),
        }),
        ..fixture.context_with(settings)
    };
    let exported = ExportApprovedVersion::new(context)
        .execute(&actor(Role::Operator), &meta(), approved_input("R-1"))
        .await
        .unwrap();
    assert!(exported.loadable());
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), before.checked_add(1).unwrap());
    assert_eq!(
        entries.last().unwrap().action(),
        AuditAction::VersionExported
    );
}

#[tokio::test(start_paused = true)]
async fn a_slow_read_of_an_approved_version_gives_web_009_and_no_entry() {
    let (fixture, _machine) = approved_fixture().await;
    let before = fixture.store.audit_entries().len();
    let error = ExportApprovedVersion::new(slow_context(&fixture))
        .execute(&actor(Role::Operator), &meta(), approved_input("R-1"))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Export
        }
    ));
    assert_eq!(fixture.store.audit_entries().len(), before);
}

/// A unit of work whose `begin` takes much longer than any limit of the tests.
struct SlowStore {
    inner: MemoryStore,
}

#[async_trait]
impl UnitOfWork for SlowStore {
    async fn begin(&self, isolation: Isolation) -> Result<Box<dyn Transaction>, StorageError> {
        tokio::time::sleep(Duration::from_secs(60)).await;
        self.inner.begin(isolation).await
    }
}

fn slow_context(fixture: &Fixture) -> AppContext {
    let settings = AppSettings {
        blocking: Blocking::new(Duration::from_secs(10), Duration::from_secs(1)),
        ..AppSettings::default()
    };
    AppContext {
        uow: Arc::new(SlowStore {
            inner: fixture.store.clone(),
        }),
        ..fixture.context_with(settings)
    }
}

#[tokio::test(start_paused = true)]
async fn a_slow_adapter_gives_web_009_with_the_export_stage() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let error = ExportApprovedVersion::new(slow_context(&fixture))
        .execute(&actor(Role::Operator), &meta(), approved_input("R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Export
        }
    ));
    let by_number = ExportVersionByNumber::new(slow_context(&fixture))
        .execute(&actor(Role::Administrator), &meta(), number_input("R-1", 1))
        .await
        .unwrap_err();
    assert_eq!(by_number.code(), ErrorCode::WEB_009);
    assert!(fixture.store.audit_entries().is_empty());
}
