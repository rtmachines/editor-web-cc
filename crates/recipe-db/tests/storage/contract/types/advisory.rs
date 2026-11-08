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

/// The backends of `recipe_app` that wait inside a transaction.
async fn app_backends_in_transaction(session: &mut PgConnection) -> Vec<i32> {
    sqlx::query_scalar(
        "SELECT pid FROM pg_stat_activity WHERE usename = $1 AND xact_start IS NOT NULL \
         AND datname = current_database()",
    )
    .bind(Role::App.name())
    .fetch_all(session)
    .await
    .unwrap()
}

/// Postgres only: an empty key list sends no statement.
///
/// A second session ends the backend of the transaction. A statement on that connection fails.
/// The call with no key must still succeed, and a call with one key must fail.
#[tokio::test]
async fn postgres_only_an_empty_key_list_sends_no_statement() {
    let h = PgHarness::create().await;
    let built = machine(154, "M154");
    insert(&h, &built).await;
    let mut session = h.database.connect(Role::Superuser).await;
    let mut tx = begin(&h).await;
    let pids = app_backends_in_transaction(&mut session).await;
    let [pid] = <[i32; 1]>::try_from(pids).expect("one transaction of recipe_app waits");
    sqlx::query("SELECT pg_terminate_backend($1)")
        .bind(pid)
        .execute(&mut session)
        .await
        .unwrap();
    for _ in 0..POLL_LIMIT {
        let gone: bool =
            sqlx::query_scalar("SELECT NOT EXISTS (SELECT 1 FROM pg_stat_activity WHERE pid = $1)")
                .bind(pid)
                .fetch_one(&mut session)
                .await
                .unwrap();
        if gone {
            break;
        }
        tokio::time::sleep(POLL_PAUSE).await;
    }

    let empty = tx.types().lock_derived_types(built.id(), &[]).await;
    let one = tx
        .types()
        .lock_derived_types(built.id(), &[DerivedTypeLock::Array])
        .await;

    assert!(one.is_err(), "the ended backend refuses a statement");
    assert!(
        empty.is_ok(),
        "an empty key list sent a statement: {empty:?}"
    );
}

/// The second key of the advisory lock as the signed value that `hashtext` gives.
async fn key_value(session: &mut PgConnection, machine: MachineId, name: &str) -> i32 {
    sqlx::query_scalar("SELECT hashtext($1::text || ':' || $2)")
        .bind(machine.to_string())
        .bind(name)
        .fetch_one(session)
        .await
        .unwrap()
}

/// The key as `pg_locks.objid` shows it: the unsigned view of the signed value.
fn objid(value: i32) -> i64 {
    i64::from(u32::from_ne_bytes(value.to_ne_bytes()))
}

/// Postgres only: the adapter takes the keys in the ascending order of their values.
///
/// The caller passes the key with the higher value first. A second session holds that key.
/// The adapter then holds the key with the lower value and waits for the higher one.
/// `pg_locks` shows both facts for one backend.
#[tokio::test]
async fn postgres_only_the_keys_are_taken_in_the_ascending_order_of_their_values() {
    let h = PgHarness::create().await;
    let built = machine(152, "M152");
    insert(&h, &built).await;
    let mut blocker_session = h.database.connect(Role::Migrator).await;
    let mut watcher = h.database.connect(Role::Migrator).await;
    let mut keyed = Vec::new();
    for key in [struct_key("Pump"), DerivedTypeLock::Array] {
        let value = key_value(&mut watcher, built.id(), key.key_text()).await;
        keyed.push((value, key));
    }
    keyed.sort_by_key(|(value, _)| *value);
    let [(low_value, low_key), (high_value, high_key)] = <[_; 2]>::try_from(keyed).unwrap();
    assert!(low_value < high_value, "the two keys differ");
    let (first, last) = (objid(low_value), objid(high_value));
    let mut blocker = blocker_session.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock($1, hashtext($2::text || ':' || $3))")
        .bind(CLASS_2)
        .bind(built.id().to_string())
        .bind(high_key.key_text())
        .execute(&mut *blocker)
        .await
        .unwrap();
    let mut tx = begin(&h).await;
    let keys = [high_key, low_key];

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
                    "the waiting backend holds the key with the lower value: {rows:?}"
                );
                seen = true;
                break;
            }
            tokio::time::sleep(POLL_PAUSE).await;
        }
        blocker.rollback().await.unwrap();
        assert!(
            seen,
            "the adapter never waited for the key with the higher value"
        );
    };
    tokio::join!(take, watch);
}

/// The number of texts that the search of two texts with one `hashtext` value scans.
const COLLISION_SEARCH_LIMIT: i32 = 300_000;

/// Two texts of the form `c<n>` with the same second key on `machine`, found with `generate_series`.
async fn colliding_texts(session: &mut PgConnection, machine: MachineId) -> (String, String) {
    let texts: Vec<String> = sqlx::query_scalar(
        "SELECT (array_agg(t ORDER BY t))[1:2] FROM \
         (SELECT 'c' || n AS t, hashtext($1::text || ':c' || n) AS key \
          FROM generate_series(1, $2::int) AS n) AS s \
         GROUP BY key HAVING count(*) > 1 ORDER BY key LIMIT 1",
    )
    .bind(machine.to_string())
    .bind(COLLISION_SEARCH_LIMIT)
    .fetch_one(session)
    .await
    .unwrap();
    let [first, second] = <[String; 2]>::try_from(texts).unwrap();
    (first, second)
}

/// Postgres only: two texts with one `hashtext` value give one key and one lock.
///
/// The statement `DERIVED_LOCK_KEYS` returns each value once and in ascending order.
/// The adapter takes one lock for the two texts.
#[tokio::test]
async fn postgres_only_two_texts_with_one_hashtext_value_give_one_key_and_one_lock() {
    let h = PgHarness::create().await;
    let built = machine(153, "M153");
    insert(&h, &built).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let (first, second) = colliding_texts(&mut session, built.id()).await;
    assert_ne!(first, second);

    let keys: Vec<i32> = sqlx::query_scalar(recipe_db::statements::DERIVED_LOCK_KEYS)
        .bind(built.id().to_string())
        .bind(vec![
            "STRING".to_owned(),
            first.clone(),
            "ARRAY".to_owned(),
            second.clone(),
        ])
        .fetch_all(&mut session)
        .await
        .unwrap();

    assert_eq!(keys.len(), 3, "the colliding texts give one key: {keys:?}");
    assert!(
        keys.windows(2).all(|pair| pair[0] < pair[1]),
        "the keys ascend: {keys:?}"
    );
    let mut tx = begin(&h).await;
    tx.types()
        .lock_derived_types(
            built.id(),
            &[
                DerivedTypeLock::Struct(TypeName::parse(&first).unwrap()),
                DerivedTypeLock::String,
                DerivedTypeLock::Struct(TypeName::parse(&second).unwrap()),
                DerivedTypeLock::Array,
            ],
        )
        .await
        .unwrap();
    let held = held_keys(&mut session).await;
    assert_eq!(held, keys.iter().map(|key| objid(*key)).collect());
}
