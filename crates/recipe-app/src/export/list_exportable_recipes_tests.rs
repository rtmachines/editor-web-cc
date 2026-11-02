//! Tests of the query `ListExportableRecipes` (design D4 of the change `export-api-for-transfer-clients`).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::MachineCode;

use super::tests::approved_fixture;
use super::{ExportableRecipesInput, ListExportableRecipes};
use crate::auth::Operation;
use crate::blocking::{AppSettings, Blocking};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing, StorageError, TimeoutStage};
use crate::memory::MemoryStore;
use crate::memory::fixtures::{Fixture, actor, meta};
use crate::ports::{Isolation, Transaction, UnitOfWork};

fn input(wanted: &str) -> ExportableRecipesInput {
    ExportableRecipesInput {
        machine: MachineCode::parse(wanted).unwrap(),
    }
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

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(
        ListExportableRecipes::OPERATION,
        Operation::ListExportableRecipes
    );
}

#[tokio::test]
async fn every_role_with_version_export_lists_the_approved_recipes() {
    let (fixture, _machine) = approved_fixture().await;
    let events = fixture.sink.events().len();
    for role in [
        Role::Administrator,
        Role::ProcessEngineer,
        Role::Operator,
        Role::ExportClient,
    ] {
        let listed = ListExportableRecipes::new(fixture.context())
            .execute(&actor(role), &meta(), input("M1"))
            .await
            .unwrap();
        assert_eq!(listed.code.to_string(), "M1", "{role:?}");
        let references: Vec<String> = listed
            .recipes
            .iter()
            .map(|row| row.reference.to_string())
            .collect();
        assert_eq!(references, ["R-1"], "{role:?}");
        assert_eq!(
            listed.recipes.first().map(|row| row.version_number),
            Some(1)
        );
    }
    assert_eq!(fixture.sink.events().len(), events);
}

#[tokio::test]
async fn an_unknown_machine_gives_web_005() {
    let (fixture, _machine) = approved_fixture().await;
    let error = ListExportableRecipes::new(fixture.context())
        .execute(&actor(Role::ExportClient), &meta(), input("M9"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        error,
        AppError::NotFound { entity: Missing::Machine(wanted) } if wanted.to_string() == "M9"
    ));
}

#[tokio::test(start_paused = true)]
async fn a_slow_read_gives_web_009_with_the_export_stage() {
    let (fixture, _machine) = approved_fixture().await;
    let error = ListExportableRecipes::new(slow_context(&fixture))
        .execute(&actor(Role::ExportClient), &meta(), input("M1"))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Export
        }
    ));
}

#[tokio::test]
async fn the_query_writes_no_audit_entry() {
    let (fixture, _machine) = approved_fixture().await;
    let entries = fixture.store.audit_entries().len();
    let query = ListExportableRecipes::new(fixture.context());
    query
        .execute(&actor(Role::ExportClient), &meta(), input("M1"))
        .await
        .unwrap();
    query
        .execute(&actor(Role::ExportClient), &meta(), input("M9"))
        .await
        .unwrap_err();
    assert_eq!(fixture.store.audit_entries().len(), entries);
}
