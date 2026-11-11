//! Tests of the time limits, the slow adapters, and the export permits of the export use cases.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use recipe_domain::audit::{AuditAction, AuditEntry};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::{DocumentFacts, ExportError, HashedBody, ScopeBody, ScopeFacts};
use recipe_domain::identity::Role;

use super::plc_connection_tests::{PASSWORD, iec_fixture, input};
use super::tests::{PLATFORM, approved_fixture, approved_input, number_input, writer};
use super::{ExportApprovedVersion, ExportPlcConnection, ExportVersionByNumber};
use crate::blocking::{AppSettings, Blocking, ExportSettings};
use crate::context::AppContext;
use crate::error::{AppError, StorageError, TimeoutStage};
use crate::memory::MemoryCredentialCipher;
use crate::memory::MemoryStore;
use crate::memory::fixtures::{Fixture, actor, meta};
use crate::ports::{
    AttemptRepository, AuditReads, AuditWritten, DiffReads, DocumentWriter, DraftRepository,
    Isolation, NonEmpty, PlantRepository, PlcConnections, RecipeReads, RecipeRepository,
    SessionRepository, SettingRepository, Transaction, TypeCatalog, UnitLookup, UnitOfWork,
    UserRepository,
};

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

    fn plc_connections(&mut self) -> &mut dyn PlcConnections {
        self.inner.plc_connections()
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

    fn write_scoped(&self, _body: &ScopeBody, _facts: &ScopeFacts) -> Result<Vec<u8>, ExportError> {
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

/// Build the connection use case over `context` with the cipher of the fixture.
fn plc_export(context: AppContext, cipher: &MemoryCredentialCipher) -> ExportPlcConnection {
    ExportPlcConnection::new(context, Arc::new(cipher.clone()))
}

#[tokio::test(start_paused = true)]
async fn a_slow_connection_read_gives_web_009_without_the_password_and_no_entry() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    let error = plc_export(slow_context(&fixture), &cipher)
        .execute(&actor(Role::ExportClient), &meta(), input("M1", "R-1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Export
        }
    ));
    let texts = [error.to_string(), format!("{error:?}")];
    assert!(texts.iter().all(|text| !text.contains(PASSWORD)));
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_busy_gate_beyond_the_limit_stops_the_connection_read_before_any_transaction() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    let own_lines = fixture.store.journal().len();
    let calls = cipher.calls().len();
    let context = fixture.context_with(one_export_permit(Duration::from_secs(1)));
    let held = context.export_gate.acquire().await.unwrap();
    let export = plc_export(context, &cipher);
    let (user, request) = (actor(Role::ExportClient), meta());
    let error = tokio::time::timeout(
        PERMIT_WAIT_BOUND,
        export.execute(&user, &request, input("M1", "R-1")),
    )
    .await
    .expect("the wait for the permit did not end inside timeout.export")
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert_eq!(fixture.store.journal().len(), own_lines);
    assert_eq!(cipher.calls().len(), calls);
    assert!(fixture.store.audit_entries().is_empty());
    drop(held);
}

#[tokio::test(start_paused = true)]
async fn a_second_connection_read_waits_for_the_permit_and_then_writes_its_entry() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
    let own_lines = fixture.store.journal().len();
    let context = fixture.context_with(one_export_permit(Duration::from_secs(10)));
    let held = context.export_gate.acquire().await.unwrap();
    let export = plc_export(context.clone(), &cipher);
    let (user, request) = (actor(Role::ExportClient), meta());
    let free_the_permit = async {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let lines_while_waiting = fixture.store.journal().len();
        drop(held);
        lines_while_waiting
    };
    let (exported, lines_while_waiting) = tokio::join!(
        export.execute(&user, &request, input("M1", "R-1")),
        free_the_permit
    );
    assert!(exported.unwrap().password.is_some());
    assert_eq!(lines_while_waiting, own_lines);
    assert_eq!(fixture.store.audit_entries().len(), 1);
    assert_eq!(context.export_gate.available(), 1);
}

#[tokio::test(start_paused = true)]
async fn a_slow_commit_runs_after_the_limit_of_the_connection_read_and_keeps_the_entry() {
    let cipher = MemoryCredentialCipher::default();
    let fixture = iec_fixture(&cipher);
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
    let exported = plc_export(context, &cipher)
        .execute(&actor(Role::ExportClient), &meta(), input("M1", "R-1"))
        .await
        .unwrap();
    assert!(exported.password.is_some());
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries.first().unwrap().action(),
        AuditAction::PlcConnectionExported
    );
}
