//! Postgres only: the flags and the pairs of one `RepeatableRead` transaction show one state
//! (ADR-0050 (f), design D10).
//!
//! A draft command on a second connection commits between the read of the flags and the read of
//! the pairs. Both reads show the state of the start of the transaction.
//!
//! The case runs on PostgreSQL only, because the memory snapshot has its own test:
//! `a_repeatable_read_transaction_reads_the_draft_of_its_snapshot` in
//! `recipe-app/src/memory/core/transaction/tests.rs`.

use recipe_domain::recipe::{ParameterKey, RecipeVersionId, StoredParameter, StoredValue};
use serde_json::json;

use super::fixtures::{
    UNCHANGED, VALUE, begin_read, compact, diff_plant, on_new_side, read_pairs, seed_versions,
    typed,
};
use crate::contract::harness::{Harness, begin, commit};
use crate::contract::postgres_harness::PgHarness;
use crate::contract::recipe_seed::Plant;

/// Change the value of `key` in the draft `version` on a second connection, and commit.
async fn commit_a_value_change(
    h: &PgHarness,
    plant: &Plant,
    version: RecipeVersionId,
    key: ParameterKey,
) {
    let mut writer = begin(h).await;
    writer.drafts().lock_draft(version).await.unwrap().unwrap();
    writer
        .drafts()
        .touch_draft(version, h.user())
        .await
        .unwrap();
    let value = StoredValue::from(json!("9"));
    writer
        .drafts()
        .set_value(version, key, &value)
        .await
        .unwrap();
    commit(h, writer, plant.machine).await;
}

/// A draft command commits between the two reads of one `RepeatableRead` transaction. The pairs
/// show the value of the start, as the flags do.
#[tokio::test]
async fn postgres_only_the_flags_and_the_pairs_keep_the_snapshot_across_a_commit() {
    let h = PgHarness::create().await;
    let plant = diff_plant(&h, 84, "M84").await;
    let old = vec![typed(&plant, (41, "S"), plant.int, json!("5"))];
    let new: Vec<StoredParameter> = old.iter().map(on_new_side).collect();
    let versions = seed_versions(&h, &plant.plant, 84, (&old, &new)).await;
    let key = old[0].key;
    let mut reader = begin_read(&h).await;

    let flags = reader
        .diff_reads()
        .change_flags(Some(versions.old), versions.new)
        .await
        .unwrap();
    commit_a_value_change(&h, &plant.plant, versions.new, key).await;
    let sides = reader
        .diff_reads()
        .pairs(Some(versions.old), Some(versions.new), &[key])
        .await
        .unwrap();
    drop(reader);
    let mut fresh = begin_read(&h).await;
    let committed = fresh
        .diff_reads()
        .change_flags(Some(versions.old), versions.new)
        .await
        .unwrap();
    drop(fresh);
    let committed_sides = read_pairs(&h, None, Some(versions.new), &[key]).await;

    assert_eq!(
        compact(&flags),
        vec![("S".to_string(), UNCHANGED, (0, 0, 0))]
    );
    assert_eq!(sides.new[0].value, StoredValue::from(json!("5")));
    assert_eq!(
        compact(&committed),
        vec![("S".to_string(), VALUE, (0, 0, 0))]
    );
    assert_eq!(committed_sides.new[0].value, StoredValue::from(json!("9")));
}
