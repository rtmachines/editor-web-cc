//! Postgres only: one `RepeatableRead` transaction sees one snapshot (ADR-0042).
//!
//! The memory adapter reads one locked state, so the case has no meaning there.

use recipe_app::ports::{Isolation, ParameterRow, Transaction, VersionDetail};
use recipe_domain::recipe::{Parameter, RecipeVersionId};

use super::fixtures::WHOLE;
use crate::contract::harness::{Harness, begin, commit};
use crate::contract::postgres_harness::PgHarness;
use crate::contract::recipe_seed::{Plant, create_recipe, parameter, plant};

/// Read the header and the parameter rows of `version` on `tx`.
async fn header_and_rows(
    tx: &mut dyn Transaction,
    version: RecipeVersionId,
) -> (VersionDetail, Vec<ParameterRow>) {
    let header = tx
        .recipe_reads()
        .version_detail(version)
        .await
        .unwrap()
        .unwrap();
    let rows = tx
        .recipe_reads()
        .parameter_page(version, None, WHOLE.0, WHOLE.1)
        .await
        .unwrap();
    (header, rows)
}

/// Touch the draft and add one parameter on a second connection, and commit.
async fn commit_a_draft_change(h: &PgHarness, plant: &Plant, version: RecipeVersionId) {
    let mut writer = begin(h).await;
    writer.drafts().lock_draft(version).await.unwrap().unwrap();
    writer
        .drafts()
        .touch_draft(version, h.user())
        .await
        .unwrap();
    writer
        .drafts()
        .insert_parameter(version, &Parameter::from(parameter(plant, 1, "NEW")))
        .await
        .unwrap();
    commit(h, writer, plant.machine).await;
}

/// A draft command on a second connection commits between two reads of one `RepeatableRead`
/// transaction. The second read sees the state of the first read.
#[tokio::test]
async fn postgres_only_a_repeatable_read_transaction_keeps_its_snapshot_across_a_commit() {
    let h = PgHarness::create().await;
    let plant = plant(&h, 40, "M40").await;
    let recipe = create_recipe(&h, &plant, 40, "REF-40").await;
    let version = recipe.versions()[0].id();
    let mut reader = h
        .unit_of_work()
        .begin(Isolation::RepeatableRead)
        .await
        .unwrap();
    let (before, _) = header_and_rows(reader.as_mut(), version).await;

    commit_a_draft_change(&h, &plant, version).await;
    let (after, rows) = header_and_rows(reader.as_mut(), version).await;
    drop(reader);
    let mut fresh = begin(&h).await;
    let (committed, committed_rows) = header_and_rows(fresh.as_mut(), version).await;

    assert_eq!(before.row_version, 1);
    assert_eq!(after.row_version, 1);
    assert_eq!(after.updated_at, before.updated_at);
    assert!(rows.is_empty(), "{rows:?}");
    assert_eq!(committed.row_version, 2);
    assert_eq!(committed_rows.len(), 1);
}
