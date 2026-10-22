//! Contract cases of `AuditReads::entries`: cursor, filters, range, and scope (task 5.1).

use recipe_app::ports::{
    AggregateId, AuditEntityType, AuditFilter, AuditScope, EntityFilter, EntityLabel, TimeRange,
};
use recipe_domain::audit::{AggregateType, AuditAction, AuditTarget, RowAction};
use recipe_domain::identity::Login;
use recipe_domain::recipe::ReferenceNumber;
use recipe_domain::timestamp::Timestamp;

use super::fixtures::{
    WHOLE, day_start, delete_recipe, entry, free_id, ids, list, numbers, of_aggregate, recipe,
    second_user, shifted, version_entity, version_target, write,
};
use crate::contract::harness::Harness;
use crate::contract::recipe_seed::{create_recipe, plant};
use crate::contract::seed::machine_id;

async fn the_list_reads_newest_first_below_an_exclusive_cursor(h: &dyn Harness) {
    let entries = (1..=6)
        .map(|n| {
            let target = version_target(recipe(1), n);
            entry(RowAction::VersionCreated, target, Some(h.user()), 1)
        })
        .collect();
    write(h, entries).await;
    let filter = of_aggregate(free_id(1));

    let all = list(h, &filter, AuditScope::All, None, WHOLE).await;
    let first = list(h, &filter, AuditScope::All, None, 3).await;
    let second = list(h, &filter, AuditScope::All, Some(first.rows[2].id), 3).await;
    let tail = list(h, &filter, AuditScope::All, Some(second.rows[0].id), 3).await;

    assert_eq!(numbers(&all.rows), [6, 5, 4, 3, 2, 1]);
    assert!(!all.has_older);
    assert!(ids(&all.rows).windows(2).all(|pair| pair[0] > pair[1]));
    assert_eq!(numbers(&first.rows), [6, 5, 4]);
    assert!(first.has_older);
    assert_eq!(numbers(&second.rows), [3, 2, 1]);
    assert!(!second.has_older, "a last page with exactly `limit` rows");
    assert_eq!(numbers(&tail.rows), [2, 1]);
    assert!(!tail.has_older);
}

contract_case!(
    the_list_reads_newest_first_below_an_exclusive_cursor,
    memory_the_list_reads_newest_first_below_an_exclusive_cursor,
    postgres_the_list_reads_newest_first_below_an_exclusive_cursor
);

/// Write the four entries of the filter case and return the id of the second user.
///
/// 1 and 4 change the version 1 of `recipe(0x20)`. 2 retires the version 2 of the same recipe.
/// 3 changes the version 3 of `recipe(0x21)`. The second user is the actor of 2 and 3.
async fn seed_filter_entries(h: &dyn Harness) -> recipe_domain::identity::UserId {
    let other = second_user(h, "jsmith").await;
    let me = Some(h.user());
    write(
        h,
        vec![
            entry(
                RowAction::VersionCreated,
                version_target(recipe(0x20), 1),
                me,
                1,
            ),
            entry(
                RowAction::VersionRetired,
                version_target(recipe(0x20), 2),
                Some(other),
                2,
            ),
            entry(
                RowAction::VersionCreated,
                version_target(recipe(0x21), 3),
                Some(other),
                3,
            ),
            entry(
                RowAction::VersionCreated,
                version_target(recipe(0x20), 1),
                me,
                4,
            ),
        ],
    )
    .await;
    other
}

/// Read the version numbers of every row that matches `filter` in the scope `All`.
async fn matching(h: &dyn Harness, filter: &AuditFilter) -> Vec<u128> {
    numbers(&list(h, filter, AuditScope::All, None, WHOLE).await.rows)
}

async fn each_filter_keeps_only_its_entries(h: &dyn Harness) {
    seed_filter_entries(h).await;
    let of_version = |id| EntityFilter {
        entity_type: AuditEntityType::RecipeVersion,
        entity_id: id,
    };
    let by = |login: &str| Some(Login::parse(login).unwrap());

    let aggregate = of_aggregate(free_id(0x20));
    let entity = AuditFilter {
        entity: Some(of_version(Some(version_entity(1)))),
        ..AuditFilter::default()
    };
    let entity_type = AuditFilter {
        entity: Some(of_version(None)),
        ..AuditFilter::default()
    };
    let actor = AuditFilter {
        actor: by("jsmith"),
        ..AuditFilter::default()
    };
    let action = AuditFilter {
        action: Some(AuditAction::VersionRetired),
        ..AuditFilter::default()
    };
    let actor_and_aggregate = AuditFilter {
        actor: by("jsmith"),
        ..of_aggregate(free_id(0x20))
    };

    assert_eq!(matching(h, &aggregate).await, [1, 2, 1]);
    assert_eq!(matching(h, &entity).await, [1, 1]);
    assert_eq!(matching(h, &entity_type).await, [1, 3, 2, 1]);
    assert_eq!(matching(h, &actor).await, [3, 2]);
    assert_eq!(matching(h, &action).await, [2]);
    assert_eq!(matching(h, &actor_and_aggregate).await, [2]);
}

contract_case!(
    each_filter_keeps_only_its_entries,
    memory_each_filter_keeps_only_its_entries,
    postgres_each_filter_keeps_only_its_entries
);

async fn an_actor_that_names_no_user_gives_the_empty_page(h: &dyn Harness) {
    seed_filter_entries(h).await;
    let ghost = AuditFilter {
        actor: Some(Login::parse("ghost").unwrap()),
        ..AuditFilter::default()
    };

    for scope in [AuditScope::All, AuditScope::Recipes] {
        let page = list(h, &ghost, scope, None, WHOLE).await;
        assert!(page.rows.is_empty(), "{scope:?}");
        assert!(!page.has_older, "{scope:?}");
    }
}

contract_case!(
    an_actor_that_names_no_user_gives_the_empty_page,
    memory_an_actor_that_names_no_user_gives_the_empty_page,
    postgres_an_actor_that_names_no_user_gives_the_empty_page
);

async fn the_range_keeps_from_and_drops_until(h: &dyn Harness) {
    let target = version_target(recipe(0x30), 1);
    write(h, vec![entry(RowAction::VersionCreated, target, None, 1)]).await;
    let filter = of_aggregate(free_id(0x30));
    let at = list(h, &filter, AuditScope::All, None, WHOLE).await.rows[0].occurred_at;
    let count = |from: Option<Timestamp>, until: Option<Timestamp>| {
        let filter = AuditFilter {
            range: TimeRange { from, until },
            ..filter.clone()
        };
        async move {
            list(h, &filter, AuditScope::All, None, WHOLE)
                .await
                .rows
                .len()
        }
    };

    assert_eq!(count(Some(at), None).await, 1);
    assert_eq!(count(Some(shifted(at, 1)), None).await, 0);
    assert_eq!(count(None, Some(at)).await, 0);
    assert_eq!(count(None, Some(shifted(at, 1))).await, 1);
    assert_eq!(
        count(Some(day_start(at, 0)), Some(day_start(at, 1))).await,
        1
    );
    assert_eq!(
        count(Some(day_start(at, 1)), Some(day_start(at, 2))).await,
        0
    );
    assert_eq!(
        count(Some(day_start(at, -1)), Some(day_start(at, 0))).await,
        0
    );
}

contract_case!(
    the_range_keeps_from_and_drops_until,
    memory_the_range_keeps_from_and_drops_until,
    postgres_the_range_keeps_from_and_drops_until
);

/// Read every row of `action` in `scope`.
async fn of_action(
    h: &dyn Harness,
    action: AuditAction,
    scope: AuditScope,
) -> recipe_app::ports::CursorPage<recipe_app::ports::AuditRow> {
    let filter = AuditFilter {
        action: Some(action),
        ..AuditFilter::default()
    };
    list(h, &filter, scope, None, WHOLE).await
}

async fn the_recipe_scope_keeps_only_recipe_entries_also_of_a_deleted_recipe(h: &dyn Harness) {
    let plant = plant(h, 40, "M40").await;
    let deleted = create_recipe(h, &plant, 40, "R-300").await;
    delete_recipe(h, &deleted).await;
    let machine = AuditTarget::machine(machine_id(40));
    write(
        h,
        vec![
            entry(RowAction::MachineRenamed, machine, Some(h.user()), 1),
            entry(
                RowAction::RecipeDeleted,
                AuditTarget::recipe(deleted.id()),
                Some(h.user()),
                2,
            ),
        ],
    )
    .await;
    let recipes = list(h, &AuditFilter::default(), AuditScope::Recipes, None, WHOLE).await;
    let machine_rows = of_action(h, AuditAction::MachineRenamed, AuditScope::Recipes).await;
    let deleted_rows = of_action(h, AuditAction::RecipeDeleted, AuditScope::Recipes).await;
    let all_machine = of_action(h, AuditAction::MachineRenamed, AuditScope::All).await;

    assert!(!recipes.rows.is_empty());
    assert!(
        recipes
            .rows
            .iter()
            .all(|row| row.aggregate_type == AggregateType::Recipe)
    );
    assert!(machine_rows.rows.is_empty());
    assert_eq!(deleted_rows.rows.len(), 1);
    let row = &deleted_rows.rows[0];
    assert_eq!(row.aggregate_id, AggregateId::from(*deleted.id().as_uuid()));
    assert_eq!(
        row.label,
        EntityLabel::Recipe(ReferenceNumber::parse("R-300").unwrap())
    );
    assert_eq!(all_machine.rows.len(), 1);
}

contract_case!(
    the_recipe_scope_keeps_only_recipe_entries_also_of_a_deleted_recipe,
    memory_the_recipe_scope_keeps_only_recipe_entries_also_of_a_deleted_recipe,
    postgres_the_recipe_scope_keeps_only_recipe_entries_also_of_a_deleted_recipe
);
