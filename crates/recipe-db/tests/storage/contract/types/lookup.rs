//! Contract cases of `UnitLookup`.

use recipe_domain::plant::{MachineId, UnitId};
use uuid::Uuid;

use crate::contract::harness::{Harness, begin};
use crate::contract::locks::{LOCK_NOT_AVAILABLE, try_row_lock};
use crate::contract::postgres_harness::PgHarness;
use crate::contract::seed::{
    U1_1, U1_1_1, U1_2, U1_10, U2, insert, machine, machine_with_archived_unit, machine_with_tree,
    path, tree_unit_id,
};
use crate::support::Role;

async fn live_units_gives_the_machine_and_the_state_of_every_unit(h: &dyn Harness) {
    let built = machine_with_archived_unit(90, "M90");
    insert(h, &built).await;
    let mut tx = begin(h).await;

    let live = tx
        .units()
        .live_units(built.id())
        .await
        .unwrap()
        .expect("the machine exists");

    assert_eq!(live.machine(), built.id());
    assert_eq!(live.machine_code(), built.code());
    assert!(!live.is_machine_archived());
    for unit in built.units() {
        assert_eq!(
            live.path_of(unit.id()),
            Some(unit.path()),
            "{}",
            unit.path()
        );
        assert_eq!(live.is_live(unit.id()), unit.is_live(), "{}", unit.path());
    }
    assert!(live.require(&path("U1")).is_ok());
    assert!(live.require(&path("U2")).is_err());
}

async fn live_units_of_an_archived_machine_shows_the_archive(h: &dyn Harness) {
    let built = machine(91, "M91");
    insert(h, &built).await;
    crate::contract::seed::archive(h, built.id()).await;
    let mut tx = begin(h).await;

    let live = tx
        .units()
        .live_units(built.id())
        .await
        .unwrap()
        .expect("the machine exists");

    assert!(live.is_machine_archived());
}

async fn live_units_of_a_missing_machine_gives_none(h: &dyn Harness) {
    let mut tx = begin(h).await;

    let found = tx
        .units()
        .live_units(MachineId::from(Uuid::from_u128(0xdead)))
        .await
        .unwrap();

    assert!(found.is_none());
}

async fn lock_unit_shared_gives_the_unit_at_the_path(h: &dyn Harness) {
    let built = machine_with_tree(92, "M92");
    let other = machine_with_tree(93, "M93");
    insert(h, &built).await;
    insert(h, &other).await;
    let mut tx = begin(h).await;

    let found = tx
        .units()
        .lock_unit_shared(built.id(), &path("U1.1"))
        .await
        .unwrap();
    let missing = tx
        .units()
        .lock_unit_shared(built.id(), &path("U3"))
        .await
        .unwrap();

    assert_eq!(found.as_ref(), built.unit(&path("U1.1")));
    assert_eq!(found.map(|unit| unit.id()), Some(tree_unit_id(92, U1_1)));
    assert!(missing.is_none());
}

async fn lock_units_shared_orders_by_id_and_skips_unknown_ids(h: &dyn Harness) {
    let built = machine_with_tree(94, "M94");
    insert(h, &built).await;
    let unknown = UnitId::from(Uuid::from_u128(0xdead));
    let wanted = [
        tree_unit_id(94, U1_1),
        unknown,
        tree_unit_id(94, U1_2),
        tree_unit_id(94, U1_1_1),
    ];
    let mut tx = begin(h).await;

    let units = tx.units().lock_units_shared(&wanted).await.unwrap();

    let ids: Vec<UnitId> = units.iter().map(|unit| unit.id()).collect();
    assert_eq!(
        ids,
        vec![
            tree_unit_id(94, U1_2),
            tree_unit_id(94, U1_1_1),
            tree_unit_id(94, U1_1)
        ]
    );
    for unit in &units {
        assert_eq!(Some(unit), built.unit(unit.path()));
    }
}

async fn lock_units_shared_of_an_empty_list_is_empty(h: &dyn Harness) {
    let mut tx = begin(h).await;

    let units = tx.units().lock_units_shared(&[]).await.unwrap();

    assert!(units.is_empty());
}

contract_case!(
    live_units_gives_the_machine_and_the_state_of_every_unit,
    live_units_gives_the_machine_and_the_state_of_every_unit_memory,
    live_units_gives_the_machine_and_the_state_of_every_unit_postgres
);
contract_case!(
    live_units_of_an_archived_machine_shows_the_archive,
    live_units_of_an_archived_machine_shows_the_archive_memory,
    live_units_of_an_archived_machine_shows_the_archive_postgres
);
contract_case!(
    live_units_of_a_missing_machine_gives_none,
    live_units_of_a_missing_machine_gives_none_memory,
    live_units_of_a_missing_machine_gives_none_postgres
);
contract_case!(
    lock_unit_shared_gives_the_unit_at_the_path,
    lock_unit_shared_gives_the_unit_at_the_path_memory,
    lock_unit_shared_gives_the_unit_at_the_path_postgres
);
contract_case!(
    lock_units_shared_orders_by_id_and_skips_unknown_ids,
    lock_units_shared_orders_by_id_and_skips_unknown_ids_memory,
    lock_units_shared_orders_by_id_and_skips_unknown_ids_postgres
);
contract_case!(
    lock_units_shared_of_an_empty_list_is_empty,
    lock_units_shared_of_an_empty_list_is_empty_memory,
    lock_units_shared_of_an_empty_list_is_empty_postgres
);

/// Postgres only: both unit locks hold `FOR SHARE`.
///
/// `FOR SHARE` lets `FOR SHARE` and `FOR KEY SHARE` pass. It blocks `FOR NO KEY UPDATE`
/// and `FOR UPDATE`.
#[tokio::test]
async fn postgres_only_unit_locks_hold_for_share_on_the_unit_rows() {
    let h = PgHarness::create().await;
    let built = machine_with_tree(95, "M95");
    insert(&h, &built).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let single = *tree_unit_id(95, U1_1).as_uuid();
    let listed = *tree_unit_id(95, U1_10).as_uuid();
    let untouched = *tree_unit_id(95, U2).as_uuid();
    let mut tx = begin(&h).await;

    tx.units()
        .lock_unit_shared(built.id(), &path("U1.1"))
        .await
        .unwrap()
        .expect("the unit exists");
    tx.units()
        .lock_units_shared(&[tree_unit_id(95, U1_10)])
        .await
        .unwrap();

    for id in [single, listed] {
        for mode in ["UPDATE", "NO KEY UPDATE"] {
            assert_eq!(
                try_row_lock(&mut session, "unit", mode, id)
                    .await
                    .as_deref(),
                Some(LOCK_NOT_AVAILABLE),
                "{mode} on {id}"
            );
        }
        for mode in ["SHARE", "KEY SHARE"] {
            assert_eq!(
                try_row_lock(&mut session, "unit", mode, id).await,
                None,
                "{mode} on {id}"
            );
        }
    }
    assert_eq!(
        try_row_lock(&mut session, "unit", "UPDATE", untouched).await,
        None
    );
}

/// Postgres only: `live_units` takes no row lock.
#[tokio::test]
async fn postgres_only_live_units_takes_no_lock() {
    let h = PgHarness::create().await;
    let built = machine_with_tree(96, "M96");
    insert(&h, &built).await;
    let mut session = h.database.connect(Role::Migrator).await;
    let mut tx = begin(&h).await;

    tx.units().live_units(built.id()).await.unwrap();

    let id = *tree_unit_id(96, U1_1).as_uuid();
    assert_eq!(try_row_lock(&mut session, "unit", "UPDATE", id).await, None);
}
