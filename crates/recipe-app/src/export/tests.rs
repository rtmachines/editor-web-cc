//! Tests of the use cases `ExportApprovedVersion` and `ExportVersionByNumber`.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use recipe_domain::audit::{AuditAction, AuditEntry, AuditTarget, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::{DocumentFacts, ExportError, ExportStatus, HashedBody};
use recipe_domain::identity::Role;
use recipe_domain::plant::{Machine, PlcPlatform};
use recipe_domain::recipe::{RecipeDeleter, VersionState};
use serde_json::json;

use super::test_doubles::{RecordingWriter, port};
use super::{
    ExportApprovedInput, ExportApprovedVersion, ExportByNumberInput, ExportVersionByNumber,
    ExportedVersion,
};
use crate::auth::Operation;
use crate::blocking::{AppSettings, Blocking, ExportSettings};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, StorageError, TimeoutStage};
use crate::memory::fixtures::{
    Fixture, actor, field, lock_line, meta, parameter_on, recipe_with, reference, rid,
    second_engineer, text, timestamp, uid, user, vid,
};
use crate::memory::{JournalEntry, LockMode, MemoryStore};
use crate::ports::{
    AttemptRepository, AuditReads, AuditWritten, DiffReads, DocumentWriter, DraftRepository,
    Isolation, NonEmpty, PlantRepository, RecipeReads, RecipeRepository, SessionRepository,
    SettingRepository, Transaction, TypeCatalog, UnitLookup, UnitOfWork, UserRepository,
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

/// Return the port of a new recording writer that no test reads.
fn writer() -> Arc<dyn DocumentWriter> {
    port(&RecordingWriter::shared())
}

async fn export_approved(fixture: &Fixture, role: Role) -> Result<ExportedVersion, AppError> {
    export_approved_to(fixture, role, &RecordingWriter::shared()).await
}

/// Export the approved version of `R-1` through `writer`.
async fn export_approved_to(
    fixture: &Fixture,
    role: Role,
    writer: &Arc<RecordingWriter>,
) -> Result<ExportedVersion, AppError> {
    ExportApprovedVersion::new(fixture.context(), port(writer))
        .execute(&actor(role), &meta(), approved_input("R-1"))
        .await
}

async fn export_number(
    fixture: &Fixture,
    role: Role,
    number: u32,
) -> Result<ExportedVersion, AppError> {
    export_number_to(fixture, role, number, &RecordingWriter::shared()).await
}

/// Export version `number` of `R-1` through `writer`.
async fn export_number_to(
    fixture: &Fixture,
    role: Role,
    number: u32,
    writer: &Arc<RecordingWriter>,
) -> Result<ExportedVersion, AppError> {
    ExportVersionByNumber::new(fixture.context(), port(writer))
        .execute(&actor(role), &meta(), number_input("R-1", number))
        .await
}

/// Store the machine `M1`, the draft 101 of `R-1`, and the parameter `T1`. Then approve it.
///
/// The fixture holds the user rows of the fixed actors, so `version_detail` finds each login.
async fn approved_fixture() -> (Fixture, Machine) {
    let fixture = Fixture::new();
    fixture.seed_actors();
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

/// Return the journal lines of `store` after the first `skip` lines.
fn journal_after(store: &MemoryStore, skip: usize) -> Vec<JournalEntry> {
    store.journal().into_iter().skip(skip).collect()
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
    let writer = RecordingWriter::shared();
    let exported = export_approved_to(&fixture, Role::Operator, &writer)
        .await
        .unwrap();
    assert!(exported.loadable());
    assert_eq!(exported.status, ExportStatus::Approved);
    assert_eq!(
        json!(exported.content_sha256.to_string()),
        stored_hash(&fixture.store)
    );
    let (body, facts) = writer.last();
    assert_eq!(body.sha256().unwrap(), exported.content_sha256);
    assert_eq!(&facts.machine_name, machine.name());
    assert_eq!(facts.recipe_name, text("Recipe"));
    assert_eq!(facts.approved_by, *second_engineer().login());
    assert_eq!(facts.approved_at, timestamp());
    assert_eq!(facts.exported_by, *actor(Role::Operator).login());
    assert_eq!(
        journal_after(&fixture.store, own_lines),
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
    let writer = RecordingWriter::shared();
    let approved = export_approved_to(&fixture, Role::Operator, &writer)
        .await
        .unwrap();
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
    let retired = export_number_to(&fixture, Role::ProcessEngineer, 1, &writer)
        .await
        .unwrap();
    assert!(!retired.loadable());
    assert_eq!(retired.status, ExportStatus::Retired);
    assert_eq!(retired.content_sha256, approved.content_sha256);
    let calls = writer.calls();
    let (first_body, _) = calls.first().unwrap();
    let (last_body, last_facts) = calls.last().unwrap();
    assert_eq!(last_body, first_body);
    assert!(!last_facts.loadable());
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
    let writer = RecordingWriter::shared();
    let after = export_approved_to(&fixture, Role::Operator, &writer)
        .await
        .unwrap();
    assert_eq!(after.content_sha256, before.content_sha256);
    assert_eq!(writer.last().1.recipe_name, text("Bottle 0.5 litre"));
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
    let error = ExportVersionByNumber::new(context, writer())
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
    let writer = RecordingWriter::shared();
    let after = ExportApprovedVersion::new(context, port(&writer))
        .execute(&actor(Role::Operator), &meta(), approved_input("R-1"))
        .await
        .unwrap();
    assert_eq!(writer.last().1.recipe_name, text("Renamed late"));
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
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let writer = RecordingWriter::shared();
    let error = export_approved_to(&fixture, Role::Operator, &writer)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::EXP_002);
    assert!(fixture.store.audit_entries().is_empty());
    let by_number = export_number_to(&fixture, Role::Administrator, 1, &writer)
        .await
        .unwrap_err();
    assert_eq!(by_number.code(), ErrorCode::EXP_002);
    // Catches a flow that calls the writer before the hash compare (D6 (c)): E-EXP-004 would hide E-EXP-002.
    assert!(writer.calls().is_empty());
}

#[tokio::test]
async fn an_unknown_reference_or_number_gives_web_005() {
    let (fixture, _machine) = approved_fixture().await;
    let number = export_number(&fixture, Role::Administrator, 9)
        .await
        .unwrap_err();
    assert_eq!(number.code(), ErrorCode::WEB_005);
    let by_number = ExportVersionByNumber::new(fixture.context(), writer())
        .execute(&actor(Role::Administrator), &meta(), number_input("R-9", 1))
        .await
        .unwrap_err();
    assert_eq!(by_number.code(), ErrorCode::WEB_005);
    let approved = ExportApprovedVersion::new(fixture.context(), writer())
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

    fn audit_reads(&mut self) -> &mut dyn AuditReads {
        self.inner.audit_reads()
    }

    fn diff_reads(&mut self) -> &mut dyn DiffReads {
        self.inner.diff_reads()
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

    fn settings(&mut self) -> &mut dyn SettingRepository {
        self.inner.settings()
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
    let exported = ExportApprovedVersion::new(context, writer())
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
    let error = ExportApprovedVersion::new(slow_context(&fixture), writer())
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
    let error = ExportApprovedVersion::new(slow_context(&fixture), writer())
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
    let by_number = ExportVersionByNumber::new(slow_context(&fixture), writer())
        .execute(&actor(Role::Administrator), &meta(), number_input("R-1", 1))
        .await
        .unwrap_err();
    assert_eq!(by_number.code(), ErrorCode::WEB_009);
    assert!(fixture.store.audit_entries().is_empty());
}

/// Bound of a wait for an export permit in the tests below. The clock is paused, so the bound
/// is virtual time. A blocking task that lives stops the auto-advance of the clock, so a slow
/// machine cannot reach the bound before the task ends.
const PERMIT_WAIT_BOUND: Duration = Duration::from_secs(10);

/// Return the settings with the limit `export` of `timeout.export` and one export permit.
fn one_export_permit(export: Duration) -> AppSettings {
    AppSettings {
        blocking: Blocking::new(Duration::from_secs(10), export),
        export: ExportSettings { permits: 1 },
        ..AppSettings::default()
    }
}

#[tokio::test(start_paused = true)]
async fn a_busy_export_gate_beyond_the_limit_gives_web_009_before_the_transaction() {
    let (fixture, _machine) = approved_fixture().await;
    let own_lines = fixture.store.journal().len();
    let before = fixture.store.audit_entries().len();
    let context = fixture.context_with(one_export_permit(Duration::from_secs(1)));
    let held = context.export_gate.acquire().await.unwrap();
    let export = ExportApprovedVersion::new(context, writer());
    let (user, request) = (actor(Role::Operator), meta());
    let error = tokio::time::timeout(
        PERMIT_WAIT_BOUND,
        export.execute(&user, &request, approved_input("R-1")),
    )
    .await
    .expect("the wait for the permit did not end inside timeout.export")
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Export
        }
    ));
    assert_eq!(fixture.store.journal().len(), own_lines);
    assert_eq!(fixture.store.audit_entries().len(), before);
    drop(held);
}

#[tokio::test(start_paused = true)]
async fn a_second_export_waits_for_the_permit_and_then_writes_its_entry() {
    let (fixture, _machine) = approved_fixture().await;
    let own_lines = fixture.store.journal().len();
    let before = fixture.store.audit_entries().len();
    let context = fixture.context_with(one_export_permit(Duration::from_secs(10)));
    let held = context.export_gate.acquire().await.unwrap();
    let export = ExportApprovedVersion::new(context, writer());
    let (user, request) = (actor(Role::Operator), meta());
    let free_the_permit = async {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let lines_while_waiting = fixture.store.journal().len();
        drop(held);
        lines_while_waiting
    };
    let (exported, lines_while_waiting) = tokio::join!(
        export.execute(&user, &request, approved_input("R-1")),
        free_the_permit
    );
    assert!(exported.unwrap().loadable());
    assert_eq!(lines_while_waiting, own_lines);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), before.checked_add(1).unwrap());
    assert_eq!(
        entries.last().unwrap().action(),
        AuditAction::VersionExported
    );
}

/// A writer that signals its start and then waits until the test releases it.
struct GatedWriter {
    started: parking_lot::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    release: parking_lot::Mutex<std::sync::mpsc::Receiver<()>>,
}

impl DocumentWriter for GatedWriter {
    fn write(&self, _body: &HashedBody, _facts: &DocumentFacts) -> Result<Vec<u8>, ExportError> {
        if let Some(started) = self.started.lock().take() {
            started.send(()).unwrap();
        }
        self.release
            .lock()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        Ok(b"{}".to_vec())
    }
}

/// The paused clock moves only through `advance`, which the test calls after the writer starts.
#[tokio::test(start_paused = true)]
async fn an_export_beyond_the_limit_keeps_its_permit_until_the_writer_ends() {
    let (fixture, _machine) = approved_fixture().await;
    let before = fixture.store.audit_entries().len();
    let context = fixture.context_with(one_export_permit(Duration::from_secs(1)));
    let (started_sender, started) = tokio::sync::oneshot::channel();
    let (release, released) = std::sync::mpsc::channel();
    let gated: Arc<dyn DocumentWriter> = Arc::new(GatedWriter {
        started: parking_lot::Mutex::new(Some(started_sender)),
        release: parking_lot::Mutex::new(released),
    });
    let export = ExportApprovedVersion::new(context.clone(), gated);
    let (user, request) = (actor(Role::Operator), meta());
    let pass_the_limit = async {
        tokio::time::timeout(PERMIT_WAIT_BOUND, started)
            .await
            .expect("the export did not start the writer")
            .unwrap();
        tokio::time::advance(Duration::from_millis(1_001)).await;
    };
    let (result, ()) = tokio::join!(
        export.execute(&user, &request, approved_input("R-1")),
        pass_the_limit
    );
    assert_eq!(result.unwrap_err().code(), ErrorCode::WEB_009);
    assert_eq!(context.export_gate.available(), 0);
    release.send(()).unwrap();
    let returned = tokio::time::timeout(PERMIT_WAIT_BOUND, context.export_gate.acquire())
        .await
        .expect("the end of the writer did not free the permit");
    drop(returned.unwrap());
    assert_eq!(context.export_gate.available(), 1);
    assert_eq!(fixture.store.audit_entries().len(), before);
}

mod counters;
mod writer;
