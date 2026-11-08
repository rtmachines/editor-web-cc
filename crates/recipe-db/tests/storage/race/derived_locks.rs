//! Race of two commands that take the class-2 locks of the same two keys in opposite order
//! (ADR-0144, `design.md` D5).
//!
//! A third connection holds the lock `ARRAY`. Command B asks for `[ARRAY, STRING]` and waits.
//! Command A asks for `[STRING, ARRAY]` and waits. The adapter sorts both lists by the key
//! value, so the two commands queue behind one lock and neither one waits in a circle.

use std::sync::Arc;
use std::time::{Duration, Instant};

use recipe_app::error::StorageError;
use recipe_app::ports::DerivedTypeLock;
use recipe_domain::plant::MachineId;
use sqlx::Connection as _;
use sqlx::PgConnection;
use tokio::task::JoinHandle;

use crate::contract::harness::{Harness, begin, commit};
use crate::contract::postgres_harness::PgHarness;
use crate::contract::seed::{insert, machine};
use crate::support::{Role, gate};

/// The class of the advisory locks of derived types (`data-model.md` section 1.4).
const CLASS_2: i32 = 2;
/// The pause between two looks at `pg_locks`.
const POLL_PAUSE: Duration = Duration::from_millis(20);
/// The longest time that a look at `pg_locks` waits for its condition.
const POLL_BOUND: Duration = Duration::from_secs(20);
/// The longest time that a command takes to end after the third connection commits.
const COMMIT_BOUND: Duration = Duration::from_secs(15);

/// Class-2 lock requests that wait in this database.
const WAITING_LOCKS: &str = "SELECT count(*) FROM pg_locks \
    WHERE locktype = 'advisory' AND classid = $1 AND objsubid = 2 AND NOT granted \
    AND database = (SELECT oid FROM pg_database WHERE datname = current_database())";

/// Poll until at least `at_least` class-2 lock requests wait.
async fn wait_for_waiting_locks(watcher: &mut PgConnection, at_least: i64, what: &str) {
    let start = Instant::now();
    loop {
        let waiting: i64 = sqlx::query_scalar(WAITING_LOCKS)
            .bind(CLASS_2)
            .fetch_one(&mut *watcher)
            .await
            .unwrap();
        if waiting >= at_least {
            return;
        }
        assert!(start.elapsed() < POLL_BOUND, "timeout: {what}");
        tokio::time::sleep(POLL_PAUSE).await;
    }
}

/// Take the keys in one transaction and commit it. The result is the result of the lock call.
fn take_and_commit(
    h: &Arc<PgHarness>,
    machine: MachineId,
    keys: [DerivedTypeLock; 2],
) -> JoinHandle<Result<(), StorageError>> {
    let h = Arc::clone(h);
    tokio::spawn(async move {
        let mut tx = begin(h.as_ref()).await;
        let taken = tx.types().lock_derived_types(machine, &keys).await;
        if taken.is_ok() {
            commit(h.as_ref() as &dyn Harness, tx, machine).await;
        }
        taken
    })
}

/// Two commands that ask for the same two keys in opposite order both commit.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_commands_with_opposite_key_orders_commit_without_a_deadlock() {
    let h = Arc::new(PgHarness::create_with(&gate::pool()).await);
    let built = machine(160, "M160");
    insert(h.as_ref(), &built).await;
    let id = built.id();
    let mut holder_session = h.database.connect(Role::Migrator).await;
    let mut watcher = h.database.connect(Role::Migrator).await;
    let mut holder = holder_session.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text || ':' || $3))")
        .bind(CLASS_2)
        .bind(id.to_string())
        .bind("ARRAY")
        .execute(&mut *holder)
        .await
        .unwrap();

    let b = take_and_commit(&h, id, [DerivedTypeLock::Array, DerivedTypeLock::String]);
    wait_for_waiting_locks(&mut watcher, 1, "command B waits").await;
    let a = take_and_commit(&h, id, [DerivedTypeLock::String, DerivedTypeLock::Array]);
    wait_for_waiting_locks(&mut watcher, 2, "command A waits").await;
    holder.commit().await.unwrap();

    let both = tokio::time::timeout(COMMIT_BOUND, async { (a.await, b.await) })
        .await
        .expect("both commands end within the lock timeout");
    let (a, b) = (
        both.0.expect("command A ends"),
        both.1.expect("command B ends"),
    );
    assert!(a.is_ok(), "command A gives no deadlock: {a:?}");
    assert!(b.is_ok(), "command B gives no deadlock: {b:?}");
}
