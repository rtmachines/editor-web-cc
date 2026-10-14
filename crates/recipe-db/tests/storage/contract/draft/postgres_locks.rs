//! Postgres only: the row lock of `lock_draft` and the missing lock of `read_draft`.
//!
//! A probe tries a lock with `NOWAIT` from a second session. `55P03` shows a conflicting lock.

use super::fixtures::live_draft;
use crate::contract::harness::begin;
use crate::contract::locks::{LOCK_NOT_AVAILABLE, try_row_lock};
use crate::contract::postgres_harness::PgHarness;
use crate::support::Role;

/// `lock_draft` holds `FOR UPDATE` on the version row and no lock on the recipe row.
#[tokio::test]
async fn postgres_only_lock_draft_holds_for_update_on_the_version() {
    let h = PgHarness::create().await;
    let d = live_draft(&h, 30).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let version = *d.version.as_uuid();
    let recipe = *crate::contract::recipe_seed::recipe_id(30).as_uuid();
    let mut tx = begin(&h).await;

    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    for mode in ["UPDATE", "NO KEY UPDATE", "SHARE", "KEY SHARE"] {
        assert_eq!(
            try_row_lock(&mut session, "recipe_version", mode, version)
                .await
                .as_deref(),
            Some(LOCK_NOT_AVAILABLE),
            "{mode}"
        );
    }
    assert_eq!(
        try_row_lock(&mut session, "recipe", "UPDATE", recipe).await,
        None
    );
    drop(tx);
    crate::contract::locks::wait_until_released(&mut session, "recipe_version", version).await;
}

/// `read_draft` and a missing version take no lock.
#[tokio::test]
async fn postgres_only_read_draft_takes_no_lock() {
    let h = PgHarness::create().await;
    let d = live_draft(&h, 31).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let mut tx = begin(&h).await;

    tx.drafts().read_draft(d.version).await.unwrap().unwrap();

    assert_eq!(
        try_row_lock(
            &mut session,
            "recipe_version",
            "UPDATE",
            *d.version.as_uuid()
        )
        .await,
        None
    );
}
