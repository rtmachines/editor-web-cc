//! Contract cases of `AuditReads::history` (task 5.2, ADR-0051 (g)).

use recipe_app::ports::AggregateId;
use recipe_domain::audit::{AggregateType, AuditTarget, RowAction};
use recipe_domain::plant::MachineId;

use super::fixtures::{WHOLE, entry, free_id, history, numbers, recipe, version_target, write};
use crate::contract::harness::Harness;

async fn the_history_holds_the_entries_of_one_recipe_newest_first(h: &dyn Harness) {
    let (own, other) = (recipe(0x70), recipe(0x71));
    let me = Some(h.user());
    let same_id_machine = AuditTarget::machine(MachineId::from(free_id(0x70)));
    write(
        h,
        vec![
            entry(RowAction::VersionCreated, version_target(own, 1), me, 1),
            entry(RowAction::VersionCreated, version_target(other, 2), me, 2),
            entry(RowAction::VersionApproved, version_target(own, 3), me, 3),
            entry(RowAction::MachineRenamed, same_id_machine, me, 4),
            entry(RowAction::VersionRetired, version_target(own, 5), me, 5),
        ],
    )
    .await;

    let all = history(h, own, None, WHOLE).await;
    let first = history(h, own, None, 2).await;
    let rest = history(h, own, Some(first.rows[1].id), 2).await;

    assert_eq!(numbers(&all.rows), [5, 3, 1]);
    assert!(!all.has_older);
    assert!(all.rows.iter().all(|row| {
        row.aggregate_type == AggregateType::Recipe
            && row.aggregate_id == AggregateId::from(free_id(0x70))
    }));
    assert_eq!(numbers(&first.rows), [5, 3]);
    assert!(first.has_older);
    assert_eq!(numbers(&rest.rows), [1]);
    assert!(!rest.has_older);
}

contract_case!(
    the_history_holds_the_entries_of_one_recipe_newest_first,
    memory_the_history_holds_the_entries_of_one_recipe_newest_first,
    postgres_the_history_holds_the_entries_of_one_recipe_newest_first
);
