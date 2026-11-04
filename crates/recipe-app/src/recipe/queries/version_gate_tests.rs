//! Tests of the order of `GetVersion` for an `Approved` version: permit first, bulk read second
//! (ADR-0056 (b), ADR-0119 (l), (m)).

use std::sync::Arc;
use std::time::Duration as StdDuration;

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;

use super::approval_preview_tests::{EARLIER_DATE, date_world};
use super::test_world::seed_users;
use super::versions::{GetVersion, GetVersionInput, VersionView};
use crate::blocking::{AppSettings, Blocking, ExportGate, ExportSettings};
use crate::context::AppContext;
use crate::error::{AppError, TimeoutStage};
use crate::memory::MemoryStore;
use crate::memory::fixtures::{actor, meta, reference};
use crate::ports::{Isolation, Transaction, UnitOfWork};

/// A unit of work that notes the number of free export permits at each `begin`.
///
/// The type solves one problem: a test must show that the bulk read of the rows opens its
/// transaction only after the page holds a permit.
struct PermitProbe {
    inner: MemoryStore,
    gate: ExportGate,
    free_at_begin: Mutex<Vec<usize>>,
}

#[async_trait]
impl UnitOfWork for PermitProbe {
    async fn begin(
        &self,
        isolation: Isolation,
    ) -> Result<Box<dyn Transaction>, crate::error::StorageError> {
        self.free_at_begin.lock().push(self.gate.available());
        self.inner.begin(isolation).await
    }
}

/// Return the settings with `blocking` and an export gate of one permit.
fn one_permit(blocking: Blocking) -> AppSettings {
    AppSettings {
        blocking,
        export: ExportSettings { permits: 1 },
        ..AppSettings::default()
    }
}

/// Return a context over the probe, and the probe.
fn probed(blocking: Blocking) -> (AppContext, Arc<PermitProbe>) {
    let fixture = date_world(EARLIER_DATE, EARLIER_DATE);
    seed_users(&fixture);
    let context = fixture.context_with(one_permit(blocking));
    let probe = Arc::new(PermitProbe {
        inner: fixture.store.clone(),
        gate: context.export_gate.clone(),
        free_at_begin: Mutex::new(Vec::new()),
    });
    let context = AppContext {
        uow: probe.clone(),
        ..context
    };
    (context, probe)
}

async fn get_approved(context: AppContext) -> Result<VersionView, AppError> {
    GetVersion::new(context)
        .execute(
            &actor(Role::Operator),
            &meta(),
            GetVersionInput {
                reference: reference("R-1"),
                number: 1,
                rebind_bound: 16_384,
            },
        )
        .await
}

#[tokio::test]
async fn the_bulk_read_of_an_approved_version_opens_its_transaction_after_the_permit() {
    let (context, probe) = probed(Blocking::default());

    let view = get_approved(context.clone()).await.unwrap();

    assert_eq!(view.rule_codes, vec![ErrorCode::RCP_005]);
    // The header read sees the free permit. The bulk read sees none, because the page holds it.
    assert_eq!(*probe.free_at_begin.lock(), [1, 0]);
    assert_eq!(context.export_gate.available(), 1);
}

#[tokio::test]
async fn a_busy_gate_stops_an_approved_version_before_the_bulk_read() {
    let short = Blocking::new(StdDuration::from_secs(10), StdDuration::from_millis(50));
    let (context, probe) = probed(short);
    let held = context.export_gate.acquire().await.unwrap();

    let error = get_approved(context).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Export
        }
    ));
    // Only the header read ran. The bulk read did not open a transaction.
    assert_eq!(*probe.free_at_begin.lock(), [0]);
    drop(held);
}
