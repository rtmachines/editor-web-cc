//! Tests of `ExportApprovedVersion` with a unit scope (design D5 and D7, ADR-0109 (w), (x), (ab)).

use std::sync::Arc;

use recipe_domain::audit::AuditAction;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::ExportError;
use recipe_domain::identity::Role;
use recipe_domain::plant::{PlcPlatform, UnitPath};
use serde_json::json;

use super::test_doubles::{FailingWriter, RecordingWriter, port};
use super::tests::approved_fixture;
use super::{ExportApprovedInput, ExportApprovedVersion, ExportScope, ExportedVersion};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, actor, meta, reference, vid};
use crate::ports::DocumentWriter;

fn unit(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

/// Export `R-1` as an Operator with the unit scope `path` through `writer`.
async fn export_unit(
    fixture: &Fixture,
    path: &str,
    writer: Arc<dyn DocumentWriter>,
) -> Result<ExportedVersion, AppError> {
    let input = ExportApprovedInput {
        reference: reference("R-1"),
        machine: None,
        scope: ExportScope::Unit(unit(path)),
    };
    ExportApprovedVersion::new(fixture.context(), writer)
        .execute(&actor(Role::Operator), &meta(), input)
        .await
}

/// Store `R-1` as an approved version without a stored hash.
///
/// A task that starts for this version ends with `E-EXP-002`, so the code shows the order.
fn unhashed_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    fixture
}

fn exported_entries(fixture: &Fixture) -> usize {
    fixture
        .store
        .audit_entries()
        .iter()
        .filter(|entry| entry.action() == AuditAction::VersionExported)
        .count()
}

#[tokio::test]
async fn a_unit_with_a_parameter_writes_the_scope_document_and_the_scope_entry() {
    let (fixture, _machine) = approved_fixture().await;
    let writer = RecordingWriter::shared();

    let exported = export_unit(&fixture, "U1.2", port(&writer)).await.unwrap();

    assert!(writer.calls().is_empty());
    let calls = writer.scoped_calls();
    assert_eq!(calls.len(), 1, "expected one scoped call");
    let Some((body, facts)) = calls.first() else {
        return;
    };
    assert_eq!(body.unit_path(), &unit("U1.2"));
    assert_eq!(body.parameters().len(), 1);
    assert_eq!(facts.scope_sha256, body.sha256().unwrap());
    assert_eq!(facts.document.content_sha256, exported.content_sha256);
    assert!(
        facts
            .document
            .names
            .contains_key(&body.parameters()[0].key())
    );
    let entries = fixture.store.audit_entries();
    let entry = entries.last().unwrap();
    assert_eq!(entry.action(), AuditAction::VersionExported);
    assert_eq!(
        entry.after(),
        Some(&json!({
            "version_id": vid(101).to_string(),
            "schema_version": "2",
            "content_sha256": exported.content_sha256.to_string(),
            "scope_unit_path": "U1.2",
            "scope_sha256": facts.scope_sha256.to_string(),
        }))
    );
}

#[tokio::test]
async fn a_parent_unit_covers_the_parameters_of_its_descendants() {
    let (fixture, _machine) = approved_fixture().await;
    let writer = RecordingWriter::shared();

    export_unit(&fixture, "U1", port(&writer)).await.unwrap();

    let calls = writer.scoped_calls();
    let (body, _facts) = calls.last().unwrap();
    assert_eq!(body.unit_path(), &unit("U1"));
    assert_eq!(body.parameters().len(), 1);
}

#[tokio::test]
async fn an_unknown_unit_gives_not_found_before_the_blocking_task() {
    let fixture = unhashed_fixture();
    let writer = RecordingWriter::shared();

    let error = export_unit(&fixture, "U9", port(&writer))
        .await
        .unwrap_err();

    // A blocking task would end with E-EXP-002, because the version has no stored hash.
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(writer.scoped_calls().is_empty());
    assert_eq!(exported_entries(&fixture), 0);
}

#[tokio::test]
async fn an_archived_unit_of_a_live_machine_gives_not_found() {
    let (fixture, _machine) = approved_fixture().await;
    fixture.store.update_state(|state, now| {
        for row in state
            .units
            .values_mut()
            .filter(|row| row.path == unit("U1.2.1"))
        {
            row.archived_at = Some(now);
        }
    });

    let error = export_unit(&fixture, "U1.2.1", port(&RecordingWriter::shared()))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert_eq!(exported_entries(&fixture), 0);
}

#[tokio::test]
async fn the_whole_hash_check_comes_before_the_scope_filter() {
    let fixture = unhashed_fixture();
    let writer = RecordingWriter::shared();

    // U1.2.1 has no parameter, so a filter before the hash check would give E-EXP-005.
    for path in ["U1.2", "U1.2.1"] {
        let error = export_unit(&fixture, path, port(&writer))
            .await
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::EXP_002, "{path}");
    }
    assert!(writer.scoped_calls().is_empty());
    assert_eq!(exported_entries(&fixture), 0);
}

#[tokio::test]
async fn a_unit_without_a_parameter_gives_empty_scope_and_no_entry() {
    let (fixture, _machine) = approved_fixture().await;
    let writer = RecordingWriter::shared();

    let error = export_unit(&fixture, "U1.2.1", port(&writer))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::EXP_005);
    assert!(matches!(
        error,
        AppError::Export(ExportError::EmptyScope { number: 1, ref path, .. })
            if *path == unit("U1.2.1")
    ));
    assert!(writer.scoped_calls().is_empty());
    assert_eq!(exported_entries(&fixture), 0);
}

#[tokio::test]
async fn a_failing_scope_writer_gives_exp_004_and_no_entry() {
    let (fixture, _machine) = approved_fixture().await;

    let error = export_unit(&fixture, "U1.2", Arc::new(FailingWriter))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::EXP_004);
    assert_eq!(exported_entries(&fixture), 0);
}

#[tokio::test]
async fn every_unit_of_an_archived_machine_counts_as_a_unit() {
    let (fixture, _machine) = approved_fixture().await;
    fixture.store.update_state(|state, now| {
        for row in state.units.values_mut() {
            if row.path == unit("U1.2.1") {
                row.archived_at = Some(now);
            }
        }
        for row in state.machines.values_mut() {
            row.archived_at = Some(now);
        }
    });

    let error = export_unit(&fixture, "U1.2.1", port(&RecordingWriter::shared()))
        .await
        .unwrap_err();

    // The unit check passed, so the empty scope is the next refusal.
    assert_eq!(error.code(), ErrorCode::EXP_005);
}
