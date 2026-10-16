//! Contract cases of `AuditReads::entry` (task 5.2, ADR-0051 (c), (f)).

use std::num::NonZeroU64;

use recipe_app::identity::new_request_id;
use recipe_app::ports::{AuditEntryId, AuditFilter, AuditRow, AuditScope, EntityLabel};
use recipe_domain::audit::RequestId;
use recipe_domain::audit::{
    AuditAction, AuditEntry, AuditTarget, RowAction, RowChange, SessionRef,
};
use recipe_domain::identity::Login;
use recipe_domain::recipe::ReferenceNumber;
use serde_json::json;

use super::fixtures::{
    WHOLE, address, delete_recipe, entry, free_id, list, of_aggregate, read_entry, recipe,
    traced_context, write,
};
use crate::contract::harness::Harness;
use crate::contract::recipe_seed::{create_recipe, plant};
use crate::contract::seed::machine_id;

/// Write a traced entry and then an entry without trace fields on `recipe(0x60)`.
///
/// The function returns the request of the traced entry and the two rows, newest first.
async fn seed_two_entries(h: &dyn Harness) -> (RequestId, Vec<AuditRow>) {
    let request = new_request_id().unwrap();
    let traced = AuditEntry::row(
        RowAction::RecipeRenamed,
        AuditTarget::recipe(recipe(0x60)),
        RowChange::Update {
            before: json!({ "name": "Old" }),
            after: json!({ "name": "New" }),
        },
        traced_context(h.user(), request),
    );
    write(h, vec![traced]).await;
    write(
        h,
        vec![entry(
            RowAction::RecipeRenamed,
            AuditTarget::recipe(recipe(0x60)),
            None,
            2,
        )],
    )
    .await;
    let rows = list(
        h,
        &of_aggregate(free_id(0x60)),
        AuditScope::All,
        None,
        WHOLE,
    )
    .await
    .rows;
    (request, rows)
}

async fn an_entry_holds_its_states_with_and_without_trace_fields(h: &dyn Harness) {
    let (request, rows) = seed_two_entries(h).await;

    let first = read_entry(h, rows[1].id, AuditScope::All).await.unwrap();
    let second = read_entry(h, rows[0].id, AuditScope::All).await.unwrap();

    assert_eq!(first.row, rows[1]);
    assert_eq!(
        first.row.actor,
        Some(Login::parse("harness-admin").unwrap())
    );
    assert_eq!(first.request_id, request);
    assert_eq!(first.before, Some(json!({ "name": "Old" })));
    assert_eq!(first.after, Some(json!({ "name": "New" })));
    assert_eq!(first.client_ip, Some(address(7)));
    assert_eq!(
        first.session_ref,
        Some(SessionRef::parse("0123456789abcdef").unwrap())
    );
    assert!(first.chain_seq >= 1);
    assert_eq!(second.row, rows[0]);
    assert_eq!(second.row.actor, None);
    assert_eq!(second.before, None);
    assert_eq!(second.after, Some(json!({ "n": 2 })));
    assert_eq!(second.client_ip, None);
    assert_eq!(second.session_ref, None);
    assert!(second.chain_seq > first.chain_seq);
}

contract_case!(
    an_entry_holds_its_states_with_and_without_trace_fields,
    memory_an_entry_holds_its_states_with_and_without_trace_fields,
    postgres_an_entry_holds_its_states_with_and_without_trace_fields
);

/// Write a `MachineRenamed` entry and a `RecipeDeleted` entry of the deleted recipe `R-610`.
///
/// The function returns the ids of the two entries, in this order.
async fn seed_scope_entries(h: &dyn Harness) -> (AuditEntryId, AuditEntryId) {
    let plant = plant(h, 61, "M61").await;
    let deleted = create_recipe(h, &plant, 61, "R-610").await;
    delete_recipe(h, &deleted).await;
    let me = Some(h.user());
    write(
        h,
        vec![
            entry(
                RowAction::MachineRenamed,
                AuditTarget::machine(machine_id(61)),
                me,
                1,
            ),
            entry(
                RowAction::RecipeDeleted,
                AuditTarget::recipe(deleted.id()),
                me,
                2,
            ),
        ],
    )
    .await;
    let rows = list(h, &AuditFilter::default(), AuditScope::All, None, WHOLE)
        .await
        .rows;
    let id_of = |action| rows.iter().find(|row| row.action == action).unwrap().id;
    (
        id_of(AuditAction::MachineRenamed),
        id_of(AuditAction::RecipeDeleted),
    )
}

async fn an_entry_outside_the_scope_equals_an_absent_entry(h: &dyn Harness) {
    let (machine_entry, recipe_entry) = seed_scope_entries(h).await;
    let largest = NonZeroU64::new(u64::try_from(i64::MAX).unwrap()).unwrap();
    let beyond = NonZeroU64::new(u64::MAX).unwrap();

    for absent in [largest, beyond].map(AuditEntryId::from) {
        assert_eq!(read_entry(h, absent, AuditScope::Recipes).await, None);
        assert_eq!(read_entry(h, absent, AuditScope::All).await, None);
    }
    assert_eq!(
        read_entry(h, machine_entry, AuditScope::Recipes).await,
        None
    );
    assert!(
        read_entry(h, machine_entry, AuditScope::All)
            .await
            .is_some()
    );
    let in_scope = read_entry(h, recipe_entry, AuditScope::Recipes)
        .await
        .unwrap();
    assert_eq!(
        in_scope.row.label,
        EntityLabel::Recipe(ReferenceNumber::parse("R-610").unwrap())
    );
}

contract_case!(
    an_entry_outside_the_scope_equals_an_absent_entry,
    memory_an_entry_outside_the_scope_equals_an_absent_entry,
    postgres_an_entry_outside_the_scope_equals_an_absent_entry
);
