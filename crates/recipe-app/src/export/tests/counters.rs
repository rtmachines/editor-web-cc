//! Tests of the counter `E-EXP-002` in the export flow (design D14 row T16, ADR-0073).

use std::sync::Arc;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;

use super::super::test_doubles::FailingWriter;
use super::{PLATFORM, approved_fixture, approved_input, number_input, writer};
use crate::error::AppError;
use crate::export::{ExportApprovedVersion, ExportVersionByNumber, ExportedVersion};
use crate::memory::fixtures::{Fixture, actor, meta, pid, uid};
use crate::operations::AlertCounters;
use crate::ports::DocumentWriter;

/// Store a recipe whose approved version has no stored hash, so every export finds a mismatch.
fn mismatch_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    fixture
}

/// Export the approved version of `R-1` as an Operator in a context that shares `counters`.
async fn export_approved_counted(
    fixture: &Fixture,
    counters: &AlertCounters,
    writer: Arc<dyn DocumentWriter>,
) -> Result<ExportedVersion, AppError> {
    let context = fixture.context().with_counters(counters.clone());
    ExportApprovedVersion::new(context, writer)
        .execute(&actor(Role::Operator), &meta(), approved_input("R-1"))
        .await
}

/// Export version `number` of `R-1` as an Administrator in a context that shares `counters`.
async fn export_number_counted(
    fixture: &Fixture,
    counters: &AlertCounters,
    number: u32,
) -> Result<ExportedVersion, AppError> {
    let context = fixture.context().with_counters(counters.clone());
    ExportVersionByNumber::new(context, writer())
        .execute(
            &actor(Role::Administrator),
            &meta(),
            number_input("R-1", number),
        )
        .await
}

#[tokio::test]
async fn one_export_with_a_hash_mismatch_counts_one() {
    let fixture = mismatch_fixture();
    let counters = AlertCounters::default();

    let error = export_approved_counted(&fixture, &counters, writer())
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::EXP_002);
    assert_eq!(counters.export_hash_mismatches(), 1);
}

#[tokio::test]
async fn two_exports_with_a_hash_mismatch_count_two() {
    let fixture = mismatch_fixture();
    let counters = AlertCounters::default();

    let approved = export_approved_counted(&fixture, &counters, writer())
        .await
        .unwrap_err();
    let by_number = export_number_counted(&fixture, &counters, 1)
        .await
        .unwrap_err();

    assert_eq!(approved.code(), ErrorCode::EXP_002);
    assert_eq!(by_number.code(), ErrorCode::EXP_002);
    assert_eq!(counters.export_hash_mismatches(), 2);
}

#[tokio::test]
async fn a_body_build_failure_gives_exp_002_and_counts_one() {
    let (fixture, _machine) = approved_fixture().await;
    fixture.store.update_state(|state, _| {
        if let Some(row) = state.parameters.get_mut(&pid(1)) {
            row.unit = uid(999);
        }
    });
    let counters = AlertCounters::default();

    let error = export_approved_counted(&fixture, &counters, writer())
        .await
        .unwrap_err();

    // Catches a counter that increments only in the hash compare branch of `write_document`.
    assert_eq!(error.code(), ErrorCode::EXP_002);
    assert_eq!(counters.export_hash_mismatches(), 1);
}

#[tokio::test]
async fn an_export_without_a_mismatch_leaves_the_counter_unchanged() {
    let (fixture, _machine) = approved_fixture().await;
    let counters = AlertCounters::default();

    export_approved_counted(&fixture, &counters, writer())
        .await
        .unwrap();
    export_number_counted(&fixture, &counters, 1).await.unwrap();

    assert_eq!(counters.export_hash_mismatches(), 0);
}

#[tokio::test]
async fn a_writer_failure_gives_exp_004_and_leaves_the_counter_unchanged() {
    let (fixture, _machine) = approved_fixture().await;
    let counters = AlertCounters::default();

    let error = export_approved_counted(&fixture, &counters, Arc::new(FailingWriter))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::EXP_004);
    assert_eq!(counters.export_hash_mismatches(), 0);
}

#[tokio::test]
async fn a_live_draft_gives_exp_003_and_leaves_the_counter_unchanged() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let counters = AlertCounters::default();

    let error = export_number_counted(&fixture, &counters, 1)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::EXP_003);
    assert_eq!(counters.export_hash_mismatches(), 0);
}
