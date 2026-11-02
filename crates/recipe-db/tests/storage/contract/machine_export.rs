//! Contract cases of `PlantRepository::export_machine_by_code` (task 3.9, ADR-0109 (j)).
//!
//! The memory adapter and the statement pick the same machine. The case that needs two different
//! archive instants carries the prefix `postgres_only`, because the memory clock stands still.

use recipe_domain::plant::MachineId;

use super::harness::{Harness, begin, commit};
use super::seed::{archive, code, insert, machine, machine_id};

/// Read the machine that `export_machine_by_code` picks for `wanted`, and return its id.
async fn exported_id(h: &dyn Harness, wanted: &str) -> Option<MachineId> {
    let mut tx = begin(h).await;
    tx.plant()
        .export_machine_by_code(code(wanted))
        .await
        .unwrap()
        .map(|found| found.id())
}

/// Insert the machine `n` with the code `wanted`.
async fn seed(h: &dyn Harness, n: u128, wanted: &str) {
    insert(h, &machine(n, wanted)).await;
}

/// Archive the machines `ids` in one transaction. The transaction gives them one `archived_at`.
async fn archive_together(h: &dyn Harness, ids: [MachineId; 2]) {
    let mut tx = begin(h).await;
    for id in ids {
        tx.plant().archive_machine(id).await.unwrap();
    }
    commit(h, tx, ids[0]).await;
}

async fn export_machine_gives_none_for_an_unknown_code(h: &dyn Harness) {
    seed(h, 1, "M1").await;

    assert_eq!(exported_id(h, "M2").await, None);
}

async fn export_machine_prefers_the_live_machine_over_archived_machines(h: &dyn Harness) {
    seed(h, 20, "M20").await;
    archive(h, machine_id(20)).await;
    seed(h, 10, "M20").await;

    assert_eq!(exported_id(h, "M20").await, Some(machine_id(10)));
}

async fn export_machine_gives_the_only_archived_machine(h: &dyn Harness) {
    seed(h, 30, "M30").await;
    archive(h, machine_id(30)).await;

    assert_eq!(exported_id(h, "M30").await, Some(machine_id(30)));
}

async fn export_machine_breaks_an_archive_tie_with_the_higher_id(h: &dyn Harness) {
    // A live code is unique, so the first machine leaves the live state before the second enters.
    // The shared transaction then archives both again and gives them one instant.
    seed(h, 42, "M40").await;
    archive(h, machine_id(42)).await;
    seed(h, 41, "M40").await;
    archive_together(h, [machine_id(41), machine_id(42)]).await;

    assert_eq!(exported_id(h, "M40").await, Some(machine_id(42)));
}

contract_case!(
    export_machine_gives_none_for_an_unknown_code,
    export_machine_gives_none_for_an_unknown_code_memory,
    export_machine_gives_none_for_an_unknown_code_postgres
);
contract_case!(
    export_machine_prefers_the_live_machine_over_archived_machines,
    export_machine_prefers_the_live_machine_over_archived_machines_memory,
    export_machine_prefers_the_live_machine_over_archived_machines_postgres
);
contract_case!(
    export_machine_gives_the_only_archived_machine,
    export_machine_gives_the_only_archived_machine_memory,
    export_machine_gives_the_only_archived_machine_postgres
);
contract_case!(
    export_machine_breaks_an_archive_tie_with_the_higher_id,
    export_machine_breaks_an_archive_tie_with_the_higher_id_memory,
    export_machine_breaks_an_archive_tie_with_the_higher_id_postgres
);

/// Postgres only: the memory clock stands still, so two archives never differ there.
///
/// The machine that the system archived last wins, even when its id is the lower one.
#[tokio::test]
async fn postgres_only_export_machine_takes_the_machine_archived_last() {
    let h = crate::contract::PgHarness::create().await;
    seed(&h, 52, "M50").await;
    archive(&h, machine_id(52)).await;
    seed(&h, 51, "M50").await;
    archive(&h, machine_id(51)).await;

    assert_eq!(exported_id(&h, "M50").await, Some(machine_id(51)));
}
