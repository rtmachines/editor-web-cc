//! Tests of the port `DocumentWriter` in the export flow (design D11 rows T6 and T7).

use std::collections::BTreeMap;
use std::sync::Arc;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::ExportStatus;
use recipe_domain::identity::Role;
use serde_json::json;
use time::Duration;

use super::super::test_doubles::{DOCUMENT, FailingWriter, PanickingWriter, RecordingWriter, port};
use super::{approved_fixture, approved_input, export_approved_to, stored_hash};
use crate::error::{AppError, TaskStage};
use crate::export::{ExportApprovedVersion, ExportedVersion};
use crate::memory::BodyWriter;
use crate::memory::fixtures::{
    Fixture, actor, meta, pid, pkey, reference, second_engineer, text, timestamp, uid, vid,
};
use crate::ports::{Clock, DocumentWriter};

/// The time between the approval and the export.
const LATER: Duration = Duration::minutes(5);

/// Export the approved version of `R-1` as an Operator through `writer`.
async fn export_with(
    fixture: &Fixture,
    writer: Arc<dyn DocumentWriter>,
) -> Result<ExportedVersion, AppError> {
    ExportApprovedVersion::new(fixture.context(), writer)
        .execute(&actor(Role::Operator), &meta(), approved_input("R-1"))
        .await
}

/// Export through `writer` and return the error and whether the audit log stayed the same.
async fn refused_with(fixture: &Fixture, writer: Arc<dyn DocumentWriter>) -> (AppError, bool) {
    let before = fixture.store.audit_entries().len();
    let error = export_with(fixture, writer).await.unwrap_err();
    (error, fixture.store.audit_entries().len() == before)
}

#[tokio::test]
async fn the_writer_gets_the_body_and_the_facts_and_returns_the_document() {
    let (fixture, machine) = approved_fixture().await;
    fixture.clock.advance(LATER);
    let writer = RecordingWriter::shared();
    let exported = export_approved_to(&fixture, Role::Operator, &writer)
        .await
        .unwrap();
    assert_eq!(exported.document, DOCUMENT);
    assert_eq!(exported.reference, reference("R-1"));
    assert_eq!(exported.number, 1);
    assert_eq!(exported.status, ExportStatus::Approved);
    assert_eq!(writer.calls().len(), 1);
    let (body, facts) = writer.last();
    assert_eq!(body.reference_number(), &reference("R-1"));
    assert_eq!(body.version_number(), 1);
    assert_eq!(body.sha256().unwrap(), exported.content_sha256);
    assert_eq!(facts.content_sha256, exported.content_sha256);
    assert_eq!(
        json!(facts.content_sha256.to_string()),
        stored_hash(&fixture.store)
    );
    assert_eq!(facts.status, ExportStatus::Approved);
    assert!(facts.loadable());
    assert_eq!(facts.exported_at, fixture.clock.now());
    assert_eq!(facts.exported_by, *actor(Role::Operator).login());
    assert_eq!(facts.approved_at, timestamp());
    assert_eq!(facts.approved_by, *second_engineer().login());
    assert_eq!(&facts.machine_name, machine.name());
    assert_eq!(facts.recipe_name, text("Recipe"));
    assert_eq!(facts.names, BTreeMap::from([(pkey(1), text("Name"))]));
}

#[tokio::test]
async fn the_body_writer_of_the_memory_adapters_writes_the_hashed_body_as_json() {
    let (fixture, _machine) = approved_fixture().await;
    let exported = export_with(&fixture, Arc::new(BodyWriter)).await.unwrap();
    let document: serde_json::Value = serde_json::from_slice(&exported.document).unwrap();
    assert_eq!(document["recipe"], json!({"reference_number": "R-1"}));
    assert_eq!(document["version"], json!({"number": 1}));
    assert_eq!(document["parameters"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn two_exports_give_the_same_hash_and_the_same_body() {
    let (fixture, _machine) = approved_fixture().await;
    let writer = RecordingWriter::shared();
    let first = export_approved_to(&fixture, Role::Operator, &writer)
        .await
        .unwrap();
    let second = export_approved_to(&fixture, Role::Operator, &writer)
        .await
        .unwrap();
    assert_eq!(second.content_sha256, first.content_sha256);
    let calls = writer.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls.first().unwrap().0, calls.last().unwrap().0);
}

#[tokio::test]
async fn a_writer_error_gives_exp_004_and_no_entry() {
    let (fixture, _machine) = approved_fixture().await;
    let (error, unchanged) = refused_with(&fixture, Arc::new(FailingWriter)).await;
    assert_eq!(error.code(), ErrorCode::EXP_004);
    assert!(unchanged);
}

#[tokio::test]
async fn a_writer_that_panics_fails_the_task_with_web_011_and_no_entry() {
    let (fixture, _machine) = approved_fixture().await;
    let (error, unchanged) = refused_with(&fixture, Arc::new(PanickingWriter)).await;
    assert!(matches!(
        error,
        AppError::TaskFailed {
            stage: TaskStage::Export
        }
    ));
    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert!(unchanged);
}

#[tokio::test]
async fn a_missing_approval_time_gives_db_008_and_no_entry() {
    let (fixture, _machine) = approved_fixture().await;
    fixture.store.update_state(|state, _| {
        if let Some(row) = state.versions.get_mut(&vid(101)) {
            row.approved_at = None;
        }
    });
    let (error, unchanged) = refused_with(&fixture, Arc::new(RecordingWriter::default())).await;
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(unchanged);
}

#[tokio::test]
async fn a_missing_approver_login_gives_db_008_and_no_entry() {
    let (fixture, _machine) = approved_fixture().await;
    fixture.store.update_state(|state, _| {
        if let Some(row) = state.versions.get_mut(&vid(101)) {
            row.approved_by = None;
        }
    });
    let (error, unchanged) = refused_with(&fixture, Arc::new(RecordingWriter::default())).await;
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(unchanged);
}

#[tokio::test]
async fn a_body_that_the_server_cannot_build_gives_exp_002_and_no_entry() {
    let (fixture, _machine) = approved_fixture().await;
    fixture.store.update_state(|state, _| {
        if let Some(row) = state.parameters.get_mut(&pid(1)) {
            row.unit = uid(999);
        }
    });
    let writer = RecordingWriter::shared();
    let (error, unchanged) = refused_with(&fixture, port(&writer)).await;
    assert_eq!(error.code(), ErrorCode::EXP_002);
    assert!(unchanged);
    // Catches a flow that writes a fallback document after a build error instead of stopping.
    assert!(writer.calls().is_empty());
}
