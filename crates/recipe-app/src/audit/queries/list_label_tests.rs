//! Tests of the label of the filter `aggregate` of `ListAuditEntries` (ADR-0051 (d), design D5).

use recipe_domain::audit::AggregateType;
use recipe_domain::identity::Role;
use recipe_domain::plant::MachineCode;

use super::test_world::{JSMITH, M12, R100, R300, entry_id, ids, login, world};
use super::{AggregateLabel, AuditList, AuditListInput, ListAuditEntries};
use crate::memory::fixtures::{Fixture, actor, meta, mid, reference, rid, user};
use crate::ports::{AggregateId, AuditFilter, EntityLabel};

/// Run `ListAuditEntries` as `role` with the filter `aggregate` set to `aggregate`.
async fn list_of(
    fixture: &Fixture,
    role: Role,
    aggregate: Option<AggregateId>,
    before: Option<u64>,
) -> AuditList {
    let input = AuditListInput {
        filter: AuditFilter {
            aggregate,
            ..AuditFilter::default()
        },
        before: before.map(entry_id),
    };
    ListAuditEntries::new(fixture.context())
        .execute(&actor(role), &meta(), input)
        .await
        .unwrap()
}

#[tokio::test]
async fn the_filter_aggregate_names_the_root_also_when_the_newest_row_is_a_child() {
    let fixture = world().await;
    let recipe = AggregateId::from(*rid(R100).as_uuid());

    let list = list_of(&fixture, Role::Administrator, Some(recipe), Some(6)).await;

    assert_eq!(ids(&list.page.rows), [5, 4, 3, 2]);
    assert_eq!(list.page.rows[0].label, EntityLabel::Missing);
    assert_eq!(
        list.aggregate,
        Some(AggregateLabel {
            aggregate_type: AggregateType::Recipe,
            label: EntityLabel::Recipe(reference("R-100")),
        })
    );
}

#[tokio::test]
async fn the_filter_aggregate_names_a_machine_by_its_code() {
    let fixture = world().await;
    let machine = AggregateId::from(*mid(M12).as_uuid());

    let list = list_of(&fixture, Role::Administrator, Some(machine), None).await;

    assert_eq!(ids(&list.page.rows), [1]);
    assert_eq!(
        list.aggregate,
        Some(AggregateLabel {
            aggregate_type: AggregateType::Machine,
            label: EntityLabel::Machine(MachineCode::parse("M12").unwrap()),
        })
    );
}

#[tokio::test]
async fn the_filter_aggregate_names_a_user_by_its_login() {
    let fixture = world().await;
    let jsmith = AggregateId::from(*user(JSMITH).as_uuid());

    let list = list_of(&fixture, Role::Administrator, Some(jsmith), None).await;

    assert_eq!(ids(&list.page.rows), [7]);
    assert_eq!(
        list.aggregate,
        Some(AggregateLabel {
            aggregate_type: AggregateType::AppUser,
            label: EntityLabel::User(login("jsmith")),
        })
    );
}

#[tokio::test]
async fn the_filter_aggregate_of_a_process_engineer_names_the_recipe() {
    let fixture = world().await;
    let recipe = AggregateId::from(*rid(R100).as_uuid());

    let list = list_of(&fixture, Role::ProcessEngineer, Some(recipe), None).await;

    assert_eq!(ids(&list.page.rows), [8, 5, 4, 3, 2]);
    assert_eq!(
        list.aggregate,
        Some(AggregateLabel {
            aggregate_type: AggregateType::Recipe,
            label: EntityLabel::Recipe(reference("R-100")),
        })
    );
}

#[tokio::test]
async fn the_filter_aggregate_of_a_recipe_without_a_row_gives_missing() {
    let fixture = world().await;
    let without_row = AggregateId::from(*rid(R300).as_uuid());

    let list = list_of(&fixture, Role::Administrator, Some(without_row), None).await;

    assert_eq!(ids(&list.page.rows), [6]);
    assert_eq!(
        list.aggregate,
        Some(AggregateLabel {
            aggregate_type: AggregateType::Recipe,
            label: EntityLabel::Missing,
        })
    );
}

#[tokio::test]
async fn an_empty_page_gives_no_aggregate_label() {
    let fixture = world().await;
    let machine = AggregateId::from(*mid(M12).as_uuid());

    let outside_scope = list_of(&fixture, Role::ProcessEngineer, Some(machine), None).await;
    let below_first = list_of(&fixture, Role::Administrator, Some(machine), Some(1)).await;

    for list in [outside_scope, below_first] {
        assert!(list.page.rows.is_empty());
        assert_eq!(list.aggregate, None);
    }
}

#[tokio::test]
async fn a_list_without_the_filter_aggregate_gives_no_aggregate_label() {
    let fixture = world().await;

    let list = list_of(&fixture, Role::Administrator, None, None).await;

    assert_eq!(list.page.rows.len(), 8);
    assert_eq!(list.aggregate, None);
}
