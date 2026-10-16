//! Tests of `ListAuditEntries` (spec "Audit list query", "Scope of the audit queries", "Filters").

use recipe_domain::audit::AuditAction;
use recipe_domain::identity::Role;

use super::test_world::{
    HOLD, M12, R100, R300, SPEED_TABLE, context_with_pages, entry_id, ids, login, october, world,
};
use super::{AuditListInput, ListAuditEntries};
use crate::blocking::PageSizes;
use crate::memory::fixtures::{Fixture, actor, meta, mid, pid, rid};
use crate::ports::{
    AggregateId, AuditEntityType, AuditFilter, AuditRow, CursorPage, EntityFilter, EntityId,
    EntityLabel, TimeRange,
};

/// Run `ListAuditEntries` as `role` with `filter` and `before`.
async fn list(
    fixture: &Fixture,
    role: Role,
    filter: AuditFilter,
    before: Option<u64>,
) -> CursorPage<AuditRow> {
    let input = AuditListInput {
        filter,
        before: before.map(entry_id),
    };
    ListAuditEntries::new(fixture.context())
        .execute(&actor(role), &meta(), input)
        .await
        .unwrap()
        .page
}

/// Run `ListAuditEntries` as the Administrator with `filter`.
async fn admin_list(fixture: &Fixture, filter: AuditFilter) -> Vec<u64> {
    ids(&list(fixture, Role::Administrator, filter, None).await.rows)
}

#[tokio::test]
async fn the_list_holds_the_newest_entries_first() {
    let fixture = world().await;
    let page = list(&fixture, Role::Administrator, AuditFilter::default(), None).await;
    assert_eq!(ids(&page.rows), [8, 7, 6, 5, 4, 3, 2, 1]);
    assert!(!page.has_older);
}

#[tokio::test]
async fn the_cursor_gives_the_older_entries_and_tells_whether_more_exist() {
    let fixture = world().await;
    let pages = PageSizes {
        audit: 3,
        ..PageSizes::default()
    };
    let query = ListAuditEntries::new(context_with_pages(&fixture, pages));
    let first = AuditListInput {
        filter: AuditFilter::default(),
        before: None,
    };
    let second = AuditListInput {
        before: Some(entry_id(6)),
        ..first.clone()
    };
    let third = AuditListInput {
        before: Some(entry_id(3)),
        ..first.clone()
    };
    let admin = actor(Role::Administrator);
    let pages = [
        query.execute(&admin, &meta(), first).await.unwrap(),
        query.execute(&admin, &meta(), second).await.unwrap(),
        query.execute(&admin, &meta(), third).await.unwrap(),
    ];
    let seen: Vec<(Vec<u64>, bool)> = pages
        .iter()
        .map(|p| (ids(&p.page.rows), p.page.has_older))
        .collect();
    assert_eq!(
        seen,
        [
            (vec![8, 7, 6], true),
            (vec![5, 4, 3], true),
            (vec![2, 1], false)
        ]
    );
}

#[tokio::test]
async fn a_last_page_that_fills_the_page_size_tells_that_no_older_entry_exists() {
    let fixture = world().await;
    let pages = PageSizes {
        audit: 3,
        ..PageSizes::default()
    };
    let input = AuditListInput {
        filter: AuditFilter::default(),
        before: Some(entry_id(4)),
    };
    let page = ListAuditEntries::new(context_with_pages(&fixture, pages))
        .execute(&actor(Role::Administrator), &meta(), input)
        .await
        .unwrap()
        .page;
    assert_eq!(ids(&page.rows), [3, 2, 1]);
    assert!(!page.has_older);
}

#[tokio::test]
async fn the_filter_aggregate_keeps_the_entries_of_one_recipe() {
    let fixture = world().await;
    let filter = AuditFilter {
        aggregate: Some(AggregateId::from(*rid(R100).as_uuid())),
        ..AuditFilter::default()
    };
    assert_eq!(admin_list(&fixture, filter).await, [8, 5, 4, 3, 2]);
}

#[tokio::test]
async fn the_filter_entity_keeps_the_two_entries_of_speed_table() {
    let fixture = world().await;
    let filter = AuditFilter {
        entity: Some(EntityFilter {
            entity_type: AuditEntityType::Parameter,
            entity_id: Some(EntityId::from(*pid(SPEED_TABLE).as_uuid())),
        }),
        ..AuditFilter::default()
    };
    assert_eq!(admin_list(&fixture, filter).await, [4, 3]);
    let only_type = AuditFilter {
        entity: Some(EntityFilter {
            entity_type: AuditEntityType::Parameter,
            entity_id: None,
        }),
        ..AuditFilter::default()
    };
    assert_eq!(admin_list(&fixture, only_type).await, [5, 4, 3]);
}

#[tokio::test]
async fn the_filter_actor_keeps_the_entries_of_one_user() {
    let fixture = world().await;
    let filter = AuditFilter {
        actor: Some(login("admin")),
        ..AuditFilter::default()
    };
    assert_eq!(admin_list(&fixture, filter).await, [7, 6, 1]);
}

#[tokio::test]
async fn an_actor_that_names_no_user_gives_the_empty_list() {
    let fixture = world().await;
    let filter = AuditFilter {
        actor: Some(login("ghost")),
        ..AuditFilter::default()
    };
    let page = list(&fixture, Role::ProcessEngineer, filter, None).await;
    assert!(page.rows.is_empty());
    assert!(!page.has_older);
}

#[tokio::test]
async fn the_filter_action_keeps_the_entries_of_one_action() {
    let fixture = world().await;
    let filter = AuditFilter {
        action: Some(AuditAction::ParameterValueSet),
        ..AuditFilter::default()
    };
    assert_eq!(admin_list(&fixture, filter).await, [5, 3]);
}

#[tokio::test]
async fn the_range_of_one_day_keeps_both_ends_of_the_day() {
    let fixture = world().await;
    let filter = AuditFilter {
        range: TimeRange {
            from: Some(october(16, 0, 0)),
            until: Some(october(17, 0, 0)),
        },
        ..AuditFilter::default()
    };
    assert_eq!(admin_list(&fixture, filter).await, [7, 6, 5, 4, 3, 2]);
}

#[tokio::test]
async fn a_process_engineer_sees_only_the_recipe_entries() {
    let fixture = world().await;
    let page = list(
        &fixture,
        Role::ProcessEngineer,
        AuditFilter::default(),
        None,
    )
    .await;
    assert_eq!(ids(&page.rows), [8, 6, 5, 4, 3, 2]);
}

#[tokio::test]
async fn a_process_engineer_lists_the_entries_of_a_deleted_recipe() {
    let fixture = world().await;
    let filter = AuditFilter {
        action: Some(AuditAction::RecipeDeleted),
        ..AuditFilter::default()
    };
    let page = list(&fixture, Role::ProcessEngineer, filter, None).await;
    let entities: Vec<EntityId> = page.rows.iter().map(|row| row.entity_id).collect();
    assert_eq!(entities, [EntityId::from(*rid(R300).as_uuid())]);
}

#[tokio::test]
async fn a_filter_value_outside_the_recipe_scope_gives_the_empty_list_without_a_read() {
    let fixture = world().await;
    let filters = [
        AuditFilter {
            action: Some(AuditAction::MachineRenamed),
            ..AuditFilter::default()
        },
        AuditFilter {
            entity: Some(EntityFilter {
                entity_type: AuditEntityType::Machine,
                entity_id: Some(EntityId::from(*mid(M12).as_uuid())),
            }),
            ..AuditFilter::default()
        },
    ];
    for filter in filters {
        let before = fixture.store.journal().len();
        let page = list(&fixture, Role::ProcessEngineer, filter, None).await;
        assert!(page.rows.is_empty());
        assert_eq!(
            fixture.store.journal().len(),
            before,
            "no transaction is opened"
        );
    }
}

#[tokio::test]
async fn a_row_names_the_actor_and_the_label_of_the_current_row() {
    let fixture = world().await;
    let page = list(&fixture, Role::Administrator, AuditFilter::default(), None).await;
    let renamed = page.rows.iter().find(|row| row.id == entry_id(2)).unwrap();
    assert_eq!(renamed.actor, Some(login("jkowalski")));
    assert_eq!(
        renamed.label,
        EntityLabel::Recipe(crate::memory::fixtures::reference("R-100"))
    );
    let hold = page.rows.iter().find(|row| row.id == entry_id(5)).unwrap();
    assert_eq!(hold.entity_id, EntityId::from(*pid(HOLD).as_uuid()));
    assert_eq!(hold.label, EntityLabel::Missing);
}
