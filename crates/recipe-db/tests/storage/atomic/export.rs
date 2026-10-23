//! Export tests on PostgreSQL (task 6.1, design D11 row T8).
//!
//! The three states: a value changed under the stored hash, the hash after a retirement, and a
//! writer error. Each test checks the audit log. A refused export leaves no entry.

use std::sync::Arc;
use std::sync::mpsc::Receiver;

use recipe_app::error::AppError;
use recipe_app::export::{ExportApprovedVersion, ExportVersionByNumber};
use recipe_app::memory::BodyWriter;
use recipe_app::ports::DocumentWriter;
use recipe_app::recipe::RetireVersion;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::{ExportError, ExportStatus};
use sqlx::Connection;

use super::recipe::{approved_by_use_case, export_by_number_input, export_input, retire_input};
use super::world::{actions_named, status};
use crate::support::app::{App, ChainHead};
use crate::support::assert_trigger;
use crate::support::writers::{Call, FailingWriter, RecordingWriter};

/// The trigger code of a write to a version that is not a live draft.
const NOT_A_LIVE_DRAFT: &str = "E-RCP-031";

/// Change the value of the one parameter of a version. The transaction disables the triggers for
/// this one statement through `SET LOCAL`, so the setting ends with the commit.
async fn change_value_under_the_hash(app: &mut App, version: uuid::Uuid, value: &str) {
    let mut tx = app.admin().begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    let changed =
        sqlx::query("UPDATE parameter SET value = $2::jsonb WHERE recipe_version_id = $1")
            .bind(version)
            .bind(value)
            .execute(&mut *tx)
            .await
            .unwrap();
    assert_eq!(changed.rows_affected(), 1);
    tx.commit().await.unwrap();
}

/// Read the stored value of the one parameter of a version as text.
async fn stored_value(app: &mut App, version: uuid::Uuid) -> String {
    sqlx::query_scalar("SELECT value::text FROM parameter WHERE recipe_version_id = $1")
        .bind(version)
        .fetch_one(app.admin())
        .await
        .unwrap()
}

/// Assert the error of a refused export: `E-EXP-002`, and the writer got no call.
fn assert_hash_mismatch(error: &AppError, calls: &Receiver<Call>) {
    assert!(
        matches!(
            error,
            AppError::Export(ExportError::ContentHashMismatch { .. })
        ),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::EXP_002);
    assert!(calls.try_recv().is_err(), "the writer ran before the check");
}

/// Catches an export that trusts the stored hash and does not rebuild the body. A value that
/// changed under the hash would leave as a valid document. The error code `E-EXP-002` and the
/// unchanged audit chain on both routes prove that the flow rebuilt the body and refused. The
/// plain update shows that the triggers work again after the edit. The restored value shows
/// that the refusal came from the value and not from another build error.
#[tokio::test]
async fn a_changed_value_under_the_stored_hash_gives_exp_002_and_no_entry_on_both_routes() {
    let mut app = App::create().await;
    let world = approved_by_use_case(&mut app).await;
    change_value_under_the_hash(&mut app, world.version, "1").await;
    let plain = sqlx::query("UPDATE parameter SET value = '2'::jsonb WHERE recipe_version_id = $1")
        .bind(world.version)
        .execute(app.admin())
        .await;
    assert_trigger(plain, NOT_A_LIVE_DRAFT);
    assert_eq!(stored_value(&mut app, world.version).await, "1");
    let (writer, calls) = RecordingWriter::port();
    let before = app.head().await;

    let approved = ExportApprovedVersion::new(app.context(), Arc::clone(&writer))
        .execute(&app.user, &app.meta, export_input(&world))
        .await
        .unwrap_err();
    assert_hash_mismatch(&approved, &calls);
    let by_number = ExportVersionByNumber::new(app.context(), Arc::clone(&writer))
        .execute(&app.user, &app.meta, export_by_number_input(&world))
        .await
        .unwrap_err();
    assert_hash_mismatch(&by_number, &calls);

    assert_eq!(actions_named(&mut app, "VersionExported").await, 0);
    app.assert_chain_unchanged(&before).await;
    change_value_under_the_hash(&mut app, world.version, "0").await;
    ExportApprovedVersion::new(app.context(), writer)
        .execute(&app.user, &app.meta, export_input(&world))
        .await
        .unwrap();
    app.assert_chain_moved(&before, &["VersionExported"]).await;
}

/// Read the hash of every `VersionExported` entry, the oldest first.
async fn exported_entry_hashes(app: &mut App) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT after_state->>'content_sha256' FROM audit_log \
         WHERE action = 'VersionExported' ORDER BY chain_seq",
    )
    .fetch_all(app.admin())
    .await
    .unwrap()
}

/// Catches a flow or an adapter that gives the writer the status `Approved` for a retired
/// version. The facts of the second export must show `Retired`, so `loadable` is `false` while
/// the hash stays equal. The assertion on the status of the second recorded call fails otherwise.
#[tokio::test]
async fn the_hash_stays_after_a_retirement_and_loadable_becomes_false() {
    let mut app = App::create().await;
    let world = approved_by_use_case(&mut app).await;
    let (writer, calls) = RecordingWriter::port();
    let first = ExportApprovedVersion::new(app.context(), Arc::clone(&writer))
        .execute(&app.user, &app.meta, export_input(&world))
        .await
        .unwrap();
    let input = retire_input(&mut app, &world).await;
    RetireVersion::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();
    assert_eq!(status(&mut app, world.version).await, "retired");

    let second = ExportVersionByNumber::new(app.context(), writer)
        .execute(&app.user, &app.meta, export_by_number_input(&world))
        .await
        .unwrap();

    assert_eq!(second.content_sha256, first.content_sha256);
    let stored: String =
        sqlx::query_scalar("SELECT content_sha256 FROM recipe_version WHERE id = $1")
            .bind(world.version)
            .fetch_one(app.admin())
            .await
            .unwrap();
    assert_eq!(second.content_sha256.to_string(), stored);
    assert_eq!(
        exported_entry_hashes(&mut app).await,
        [stored.clone(), stored]
    );
    let recorded: Vec<Call> = calls.try_iter().collect();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0].1.status, ExportStatus::Approved);
    assert!(recorded[0].1.loadable());
    assert_eq!(recorded[1].1.status, ExportStatus::Retired);
    assert!(!recorded[1].1.loadable());
    assert_eq!(recorded[1].1.content_sha256, first.content_sha256);
    assert_eq!(second.status, ExportStatus::Retired);
    assert!(!second.loadable());
}

/// Assert that a use case failed with `E-EXP-004` and wrote no entry.
async fn assert_writer_refusal(app: &mut App, error: &AppError, before: &ChainHead) {
    assert!(
        matches!(error, AppError::Export(ExportError::Serialize(_))),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::EXP_004);
    assert_eq!(actions_named(app, "VersionExported").await, 0);
    app.assert_chain_unchanged(before).await;
}

/// Catches a flow that commits the `VersionExported` entry before it calls the writer, or that
/// ignores the writer result. A failed document would still leave an entry for a file that no
/// user got. The unchanged chain head on both routes proves that the transaction dropped.
#[tokio::test]
async fn a_writer_error_gives_exp_004_and_writes_no_entry_on_both_routes() {
    let mut app = App::create().await;
    let world = approved_by_use_case(&mut app).await;
    let writer: Arc<dyn DocumentWriter> = FailingWriter::port();
    let before = app.head().await;

    let approved = ExportApprovedVersion::new(app.context(), Arc::clone(&writer))
        .execute(&app.user, &app.meta, export_input(&world))
        .await
        .unwrap_err();
    assert_writer_refusal(&mut app, &approved, &before).await;
    let by_number = ExportVersionByNumber::new(app.context(), writer)
        .execute(&app.user, &app.meta, export_by_number_input(&world))
        .await
        .unwrap_err();
    assert_writer_refusal(&mut app, &by_number, &before).await;

    let ok = ExportApprovedVersion::new(app.context(), Arc::new(BodyWriter))
        .execute(&app.user, &app.meta, export_input(&world))
        .await;
    assert!(ok.is_ok(), "the version stays exportable: {ok:?}");
    app.assert_chain_moved(&before, &["VersionExported"]).await;
}
