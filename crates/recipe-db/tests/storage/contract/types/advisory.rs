//! Contract cases of `TypeCatalog::lock_derived_types`.

use std::collections::BTreeSet;
use std::time::Duration;

use recipe_app::ports::DerivedTypeLock;
use recipe_domain::plant::{MachineId, TypeName};
use sqlx::Connection as _;
use sqlx::PgConnection;

use crate::contract::harness::{Harness, begin};
use crate::contract::postgres_harness::PgHarness;
use crate::contract::seed::{insert, machine};
use crate::support::Role;

/// The class of the advisory locks of derived types (`data-model.md` section 1.4).
const CLASS_2: i32 = 2;
/// The pause between two looks at `pg_locks`.
const POLL_PAUSE: Duration = Duration::from_millis(20);
/// The number of looks at `pg_locks` before the case gives up. The pool lock timeout is 2 s.
const POLL_LIMIT: u32 = 60;

fn struct_key(name: &str) -> DerivedTypeLock {
    DerivedTypeLock::Struct(TypeName::parse(name).unwrap())
}

async fn lock_derived_types_accepts_the_keys_of_a_command(h: &dyn Harness) {
    let built = machine(150, "M150");
    insert(h, &built).await;
    let keys = [
        struct_key("Pump"),
        DerivedTypeLock::String,
        DerivedTypeLock::Wstring,
        DerivedTypeLock::Array,
    ];
    let mut tx = begin(h).await;

    tx.types()
        .lock_derived_types(built.id(), &keys)
        .await
        .unwrap();
    // A second call in the same transaction takes the same keys again without a wait.
    tx.types()
        .lock_derived_types(built.id(), &keys)
        .await
        .unwrap();
    tx.types()
        .lock_derived_types(built.id(), &[])
        .await
        .unwrap();
}

contract_case!(
    lock_derived_types_accepts_the_keys_of_a_command,
    lock_derived_types_accepts_the_keys_of_a_command_memory,
    lock_derived_types_accepts_the_keys_of_a_command_postgres
);

/// The second key of the advisory lock as `pg_locks.objid` shows it.
async fn key_of(session: &mut PgConnection, machine: MachineId, name: &str) -> i64 {
    sqlx::query_scalar("SELECT hashtext($1::text || ':' || $2)::bigint & 4294967295")
        .bind(machine.to_string())
        .bind(name)
        .fetch_one(session)
        .await
        .unwrap()
}

/// The class-2 keys that sessions hold, as a set.
async fn held_keys(session: &mut PgConnection) -> BTreeSet<i64> {
    let keys: Vec<i64> = sqlx::query_scalar(
        "SELECT objid::bigint FROM pg_locks \
         WHERE locktype = 'advisory' AND classid = $1 AND objsubid = 2 AND granted \
         AND database = (SELECT oid FROM pg_database WHERE datname = current_database())",
    )
    .bind(CLASS_2)
    .fetch_all(session)
    .await
    .unwrap();
    keys.into_iter().collect()
}

/// Postgres only: `pg_locks` shows the class-2 keys until the transaction ends.
#[tokio::test]
async fn postgres_only_pg_locks_shows_the_class_2_keys_of_the_command() {
    let h = PgHarness::create().await;
    let built = machine(151, "M151");
    insert(&h, &built).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let mut tx = begin(&h).await;

    tx.types()
        .lock_derived_types(
            built.id(),
            &[
                struct_key("Pump"),
                DerivedTypeLock::String,
                DerivedTypeLock::Array,
            ],
        )
        .await
        .unwrap();

    let expected = BTreeSet::from([
        key_of(&mut session, built.id(), "Pump").await,
        key_of(&mut session, built.id(), "STRING").await,
        key_of(&mut session, built.id(), "ARRAY").await,
    ]);
    assert_eq!(expected.len(), 3);
    assert_eq!(held_keys(&mut session).await, expected);
    drop(tx);
    for _ in 0..POLL_LIMIT {
        if held_keys(&mut session).await.is_empty() {
            return;
        }
        tokio::time::sleep(POLL_PAUSE).await;
    }
    assert!(
        held_keys(&mut session).await.is_empty(),
        "the keys stay locked"
    );
}

/// Postgres only: the adapter takes the keys in the given order.
///
/// A second session holds the last key. The adapter then holds the first key and waits for
/// the last one. `pg_locks` shows both facts for one backend.
#[tokio::test]
async fn postgres_only_the_keys_are_taken_in_the_given_order() {
    let h = PgHarness::create().await;
    let built = machine(152, "M152");
    insert(&h, &built).await;
    let mut blocker_session = h.database.connect(Role::Migrator).await;
    let mut watcher = h.database.connect(Role::Migrator).await;
    let first = key_of(&mut watcher, built.id(), "Pump").await;
    let last = key_of(&mut watcher, built.id(), "ARRAY").await;
    let mut blocker = blocker_session.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text || ':' || $3))")
        .bind(CLASS_2)
        .bind(built.id().to_string())
        .bind("ARRAY")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let mut tx = begin(&h).await;
    let keys = [struct_key("Pump"), DerivedTypeLock::Array];

    let take = async {
        tx.types()
            .lock_derived_types(built.id(), &keys)
            .await
            .unwrap();
    };
    let watch = async {
        let mut seen = false;
        for _ in 0..POLL_LIMIT {
            let rows: Vec<(i64, bool, i32)> = sqlx::query_as(
                "SELECT objid::bigint, granted, pid FROM pg_locks \
                 WHERE locktype = 'advisory' AND classid = $1 AND objsubid = 2 \
                 AND database = (SELECT oid FROM pg_database \
                                 WHERE datname = current_database())",
            )
            .bind(CLASS_2)
            .fetch_all(&mut watcher)
            .await
            .unwrap();
            let waiting = rows.iter().find(|row| row.0 == last && !row.1);
            if let Some(waiting) = waiting {
                let holds_first = rows
                    .iter()
                    .any(|row| row.0 == first && row.1 && row.2 == waiting.2);
                assert!(
                    holds_first,
                    "the waiting backend holds the first key: {rows:?}"
                );
                seen = true;
                break;
            }
            tokio::time::sleep(POLL_PAUSE).await;
        }
        blocker.rollback().await.unwrap();
        assert!(seen, "the adapter never waited for the last key");
    };
    tokio::join!(take, watch);
}
