//! Postgres only: the class-1 advisory lock of T1 (design D6).
//!
//! The case observes `pg_locks` from a second session, filtered by the database of the test.
//! A blocker holds T1 at a known statement, so the observer sees the lock while T1 runs.

use std::time::Duration;

use recipe_app::ports::AttemptKey;
use recipe_domain::identity::ThrottlePolicy;
use sqlx::{Connection as _, PgConnection};

use super::{FIRST, typed};
use crate::contract::harness::Harness;
use crate::contract::postgres_harness::PgHarness;
use crate::support::Role;

/// The pause between two reads of `pg_locks`.
const POLL_PAUSE: Duration = Duration::from_millis(20);
/// The number of reads before the case gives up. The bound is about 5 s.
const POLL_LIMIT: u32 = 250;

/// The granted and the waiting class-1 locks on the key of `login` in this database.
const CLASS_1_LOCKS: &str = "\
SELECT count(*) FILTER (WHERE granted), count(*) FILTER (WHERE NOT granted) FROM pg_locks \
WHERE locktype = 'advisory' \
    AND database = (SELECT oid FROM pg_database WHERE datname = current_database()) \
    AND classid = 1::oid AND objsubid = 2 \
    AND objid = (hashtext($1)::bigint & 4294967295)::oid";

/// Return the number of granted and of waiting class-1 locks on the key of `login`.
async fn class_1_locks(observer: &mut PgConnection, login: &str) -> (i64, i64) {
    sqlx::query_as(CLASS_1_LOCKS)
        .bind(login)
        .fetch_one(observer)
        .await
        .unwrap()
}

/// Poll until the locks equal `expected`, and fail after the bound.
async fn wait_for_locks(observer: &mut PgConnection, login: &str, expected: (i64, i64)) {
    let mut last = (-1, -1);
    for _ in 0..POLL_LIMIT {
        last = class_1_locks(observer, login).await;
        if last == expected {
            return;
        }
        tokio::time::sleep(POLL_PAUSE).await;
    }
    assert_eq!(last, expected, "the class-1 locks of {login}");
}

fn key() -> AttemptKey {
    AttemptKey {
        login: typed("anna"),
        client_ip: FIRST,
    }
}

/// While T1 counts, it holds the class-1 key. After the commit, nobody holds it.
#[tokio::test]
async fn postgres_only_t1_holds_the_class_1_key_while_it_counts_and_frees_it_at_the_commit() {
    let h = PgHarness::create().await;
    let mut observer = h.database.connect(Role::Migrator).await;
    let mut blocker = h.database.connect(Role::Migrator).await;
    let mut hold = blocker.begin().await.unwrap();
    // T1 takes the key, then waits for this lock at the statement that counts.
    sqlx::query("LOCK TABLE login_attempt IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let store = h.login_attempts();
    let task = tokio::spawn(async move {
        store
            .begin_attempt(&key(), &ThrottlePolicy::default())
            .await
    });

    wait_for_locks(&mut observer, "anna", (1, 0)).await;
    hold.rollback().await.unwrap();
    let outcome = task.await.unwrap().unwrap();

    assert!(outcome.attempt.is_some());
    wait_for_locks(&mut observer, "anna", (0, 0)).await;
}

/// The lock is its own statement: a failure that commits while T1 waits for the key is counted.
#[tokio::test]
async fn postgres_only_the_count_sees_a_row_that_commits_while_t1_waits_for_the_key() {
    let h = PgHarness::create().await;
    let mut observer = h.database.connect(Role::Migrator).await;
    let mut holder = h.database.connect(Role::Migrator).await;
    sqlx::query("SELECT pg_advisory_lock(1, hashtext($1))")
        .bind("anna")
        .execute(&mut holder)
        .await
        .unwrap();
    let store = h.login_attempts();
    let task = tokio::spawn(async move {
        store
            .begin_attempt(&key(), &ThrottlePolicy::default())
            .await
    });

    wait_for_locks(&mut observer, "anna", (1, 1)).await;
    h.seed_attempts(&typed("anna"), FIRST, &[5], false).await;
    sqlx::query("SELECT pg_advisory_unlock(1, hashtext($1))")
        .bind("anna")
        .execute(&mut holder)
        .await
        .unwrap();
    let outcome = task.await.unwrap().unwrap();

    assert_eq!(outcome.count.pair_failures, 1);
    assert!(outcome.attempt.is_some());
    wait_for_locks(&mut observer, "anna", (0, 0)).await;
}
