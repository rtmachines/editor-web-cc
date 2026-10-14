//! Postgres only: the cascade of `delete_parameter` removes the child rows.
//!
//! The contract case reads the draft through the port. This case counts the rows in the tables.

use super::fixtures::{full_parameter, live_draft};
use crate::contract::harness::{begin, commit};
use crate::contract::postgres_harness::PgHarness;
use crate::contract::recipe_seed::seed;
use crate::support::Role;

async fn count(session: &mut sqlx::PgConnection, table: &'static str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(session)
        .await
        .unwrap()
}

#[tokio::test]
async fn postgres_only_delete_parameter_removes_the_limit_and_translation_rows() {
    let h = PgHarness::create().await;
    let d = live_draft(&h, 60).await;
    let doomed = full_parameter(&d.plant, 1, "GONE");
    seed(&h, d.version, &doomed).await;
    let mut session = h.database.connect(Role::Migrator).await;
    assert_eq!(count(&mut session, "parameter_limit").await, 2);
    assert_eq!(count(&mut session, "parameter_translation").await, 2);
    let mut tx = begin(&h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    tx.drafts()
        .delete_parameter(d.version, doomed.key)
        .await
        .unwrap();
    commit(&h, tx, d.plant.machine).await;

    assert_eq!(count(&mut session, "parameter").await, 0);
    assert_eq!(count(&mut session, "parameter_limit").await, 0);
    assert_eq!(count(&mut session, "parameter_translation").await, 0);
}
