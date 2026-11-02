//! Tests of the machine filter of `ExportApprovedVersion` (design D5, ADR-0109 (h) and (j)).

use recipe_domain::audit::{AuditAction, RequestId};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlcPlatform};

use super::test_doubles::{RecordingWriter, port};
use super::tests::approved_fixture;
use super::{ExportApprovedInput, ExportApprovedVersion, ExportScope, ExportedVersion};
use crate::auth::RequestMeta;
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, actor, mid, reference, uuid};

fn code(text: &str) -> MachineCode {
    MachineCode::parse(text).unwrap()
}

/// Export `R-1` as the technical account on the route of the machine `wanted`.
async fn export_for(
    fixture: &Fixture,
    wanted: &str,
    meta: &RequestMeta,
) -> Result<ExportedVersion, AppError> {
    let input = ExportApprovedInput {
        reference: reference("R-1"),
        machine: Some(code(wanted)),
        scope: ExportScope::Version,
    };
    ExportApprovedVersion::new(fixture.context(), port(&RecordingWriter::shared()))
        .execute(&actor(Role::ExportClient), meta, input)
        .await
}

/// Return the request data of a Basic request: no session reference.
fn basic_meta() -> RequestMeta {
    RequestMeta::new(RequestId::from(uuid(0x0b)), None, None, None)
}

/// Mark the machine `n` as archived by the clock of the store.
fn archive(fixture: &Fixture, n: u128) {
    fixture.store.update_state(|state, now| {
        if let Some(row) = state.machines.get_mut(&mid(n)) {
            row.archived_at = Some(now);
        }
    });
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
async fn a_recipe_of_another_machine_gives_not_found_and_writes_no_entry() {
    let (fixture, _machine) = approved_fixture().await;
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");

    let error = export_for(&fixture, "M2", &basic_meta()).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert_eq!(exported_entries(&fixture), 0);
}

#[tokio::test]
async fn an_unknown_machine_code_gives_the_same_not_found() {
    let (fixture, _machine) = approved_fixture().await;

    let error = export_for(&fixture, "M9", &basic_meta()).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert_eq!(exported_entries(&fixture), 0);
}

#[tokio::test]
async fn a_recipe_of_the_named_machine_exports() {
    let (fixture, _machine) = approved_fixture().await;

    let exported = export_for(&fixture, "M1", &basic_meta()).await.unwrap();

    assert_eq!(exported.number, 1);
    assert_eq!(exported_entries(&fixture), 1);
}

#[tokio::test]
async fn a_recipe_of_an_archived_machine_exports() {
    let (fixture, _machine) = approved_fixture().await;
    archive(&fixture, 1);

    let exported = export_for(&fixture, "M1", &basic_meta()).await.unwrap();

    assert_eq!(exported.number, 1);
    assert_eq!(exported_entries(&fixture), 1);
}

#[tokio::test]
async fn a_reused_code_hides_the_recipes_of_the_older_archived_machine() {
    let (fixture, _machine) = approved_fixture().await;
    archive(&fixture, 1);
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M1");

    let error = export_for(&fixture, "M1", &basic_meta()).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert_eq!(exported_entries(&fixture), 0);
}

#[tokio::test]
async fn a_request_without_a_session_reference_gives_an_entry_without_one() {
    let (fixture, _machine) = approved_fixture().await;

    export_for(&fixture, "M1", &basic_meta()).await.unwrap();

    let entries = fixture.store.audit_entries();
    let entry = entries
        .iter()
        .find(|entry| entry.action() == AuditAction::VersionExported)
        .unwrap();
    assert_eq!(entry.session_ref(), None);
}
