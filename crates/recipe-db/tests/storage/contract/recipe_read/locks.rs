//! Contract cases of the lock methods of `RecipeRepository`.

use recipe_domain::recipe::VersionState;

use crate::contract::harness::{Harness, begin, fixture_hash};
use crate::contract::images::{Stage, assert_shape, field, recipe_shape, text, version_shape};
use crate::contract::recipe_seed::{
    approve, create_recipe, create_version, plant, recipe_id, retire, version_id,
};
use crate::contract::seed::machine_id;

async fn lock_recipe_gives_the_state_and_the_image_before_the_change(h: &dyn Harness) {
    let plant = plant(h, 10, "M10").await;
    let created = create_recipe(h, &plant, 10, "REF-10").await;
    let mut tx = begin(h).await;

    let locked = tx
        .recipes()
        .lock_recipe(created.id())
        .await
        .unwrap()
        .expect("the recipe exists");

    assert_eq!(locked.state, created);
    assert_shape(&locked.before, &recipe_shape(false));
    assert_eq!(text(&locked.before, "id"), created.id().to_string());
    assert_eq!(text(&locked.before, "reference_number"), "REF-10");
    assert_eq!(text(&locked.before, "name"), "Recipe");
    assert_eq!(
        text(&locked.before, "machine_id"),
        plant.machine.to_string()
    );
    assert_eq!(text(&locked.before, "plc_platform"), "TwinCat3");
    assert_eq!(text(&locked.before, "created_by"), h.user().to_string());
}

async fn lock_recipe_of_a_missing_recipe_gives_none(h: &dyn Harness) {
    let mut tx = begin(h).await;

    let locked = tx.recipes().lock_recipe(recipe_id(99)).await.unwrap();

    assert!(locked.is_none());
}

async fn lock_recipes_of_machine_gives_the_recipes_of_that_machine_ordered_by_id(h: &dyn Harness) {
    let plant = plant(h, 11, "M11").await;
    let other = crate::contract::recipe_seed::plant(h, 12, "M12").await;
    // The identifiers do not follow the insert order.
    for (n, reference) in [(30, "REF-30"), (12, "REF-12"), (21, "REF-21")] {
        create_recipe(h, &plant, n, reference).await;
    }
    create_recipe(h, &other, 5, "REF-OTHER").await;
    let mut tx = begin(h).await;

    let recipes = tx
        .recipes()
        .lock_recipes_of_machine(plant.machine)
        .await
        .unwrap();
    let none = tx
        .recipes()
        .lock_recipes_of_machine(machine_id(99))
        .await
        .unwrap();

    let ids: Vec<_> = recipes.iter().map(|r| r.id()).collect();
    assert_eq!(ids, vec![recipe_id(12), recipe_id(21), recipe_id(30)]);
    assert!(recipes.iter().all(|r| r.machine() == plant.machine));
    assert_eq!(recipes[0].versions()[0].state(), VersionState::LiveDraft);
    assert!(none.is_empty());
}

async fn lock_versions_gives_every_version_with_its_image_in_id_order(h: &dyn Harness) {
    let plant = plant(h, 13, "M13").await;
    let created = create_recipe(h, &plant, 13, "REF-13").await;
    let first = created.versions()[0].id();
    approve(h, first).await;
    // The second version has the smaller identifier.
    create_version(h, (created.id(), version_id(1)), 2, Some(first)).await;
    let mut tx = begin(h).await;

    let locked = tx.recipes().lock_versions(created.id()).await.unwrap();
    let none = tx.recipes().lock_versions(recipe_id(99)).await.unwrap();

    let ids: Vec<_> = locked.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, vec![version_id(1), first]);
    assert_shape(&locked[0].1, &version_shape(true, Stage::Draft));
    assert_shape(&locked[1].1, &version_shape(false, Stage::Approved));
    assert_eq!(text(&locked[0].1, "id"), version_id(1).to_string());
    assert_eq!(text(&locked[0].1, "status"), "draft");
    assert_eq!(text(&locked[1].1, "status"), "approved");
    assert_eq!(field(&locked[0].1, "number").as_i64(), Some(2));
    assert_eq!(field(&locked[1].1, "number").as_i64(), Some(1));
    assert_eq!(field(&locked[0].1, "row_version").as_i64(), Some(1));
    assert_eq!(field(&locked[1].1, "row_version").as_i64(), Some(2));
    assert_eq!(text(&locked[0].1, "based_on_version_id"), first.to_string());
    assert!(none.is_empty());
}

async fn lock_version_gives_the_image_before_the_change(h: &dyn Harness) {
    let plant = plant(h, 14, "M14").await;
    let created = create_recipe(h, &plant, 14, "REF-14").await;
    let draft = created.versions()[0].id();
    let mut tx = begin(h).await;

    let image = tx
        .recipes()
        .lock_version(draft)
        .await
        .unwrap()
        .expect("the version exists");
    let missing = tx.recipes().lock_version(version_id(99)).await.unwrap();

    assert_shape(&image, &version_shape(false, Stage::Draft));
    assert_eq!(text(&image, "id"), draft.to_string());
    assert_eq!(text(&image, "recipe_id"), created.id().to_string());
    assert_eq!(text(&image, "status"), "draft");
    assert_eq!(text(&image, "change_note"), "Fixture note");
    assert_eq!(text(&image, "created_by"), h.user().to_string());
    assert_eq!(field(&image, "number").as_i64(), Some(1));
    assert_eq!(field(&image, "row_version").as_i64(), Some(1));
    assert!(missing.is_none());
}

async fn lock_version_shared_of_a_draft_gives_no_approver(h: &dyn Harness) {
    let plant = plant(h, 15, "M15").await;
    let created = create_recipe(h, &plant, 15, "REF-15").await;
    let draft = created.versions()[0].id();
    let mut tx = begin(h).await;

    let shared = tx
        .recipes()
        .lock_version_shared(draft)
        .await
        .unwrap()
        .expect("the version exists");
    let missing = tx
        .recipes()
        .lock_version_shared(version_id(99))
        .await
        .unwrap();

    assert_eq!(shared.recipe, created.id());
    assert_eq!(shared.number, 1);
    assert_eq!(shared.state, VersionState::LiveDraft);
    assert_eq!(shared.content_sha256, None);
    assert_eq!(shared.approved_by, None);
    assert_eq!(shared.approved_at, None);
    assert!(missing.is_none());
}

async fn lock_version_shared_gives_the_approval_of_an_approved_and_a_retired_version(
    h: &dyn Harness,
) {
    let plant = plant(h, 16, "M16").await;
    let created = create_recipe(h, &plant, 16, "REF-16").await;
    let first = created.versions()[0].id();
    approve(h, first).await;
    let mut tx = begin(h).await;
    let approved = tx
        .recipes()
        .lock_version_shared(first)
        .await
        .unwrap()
        .unwrap();
    drop(tx);
    retire(h, first).await;
    let mut tx = begin(h).await;

    let retired = tx
        .recipes()
        .lock_version_shared(first)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(approved.state, VersionState::Approved);
    assert_eq!(retired.state, VersionState::Retired);
    for shared in [&approved, &retired] {
        assert_eq!(shared.content_sha256, Some(fixture_hash()));
        assert_eq!(shared.approved_by, Some(h.user()));
        assert!(shared.approved_at.is_some());
        assert_eq!(shared.recipe, created.id());
    }
    assert_eq!(approved.approved_at, retired.approved_at);
}

contract_case!(
    lock_recipe_gives_the_state_and_the_image_before_the_change,
    lock_recipe_gives_the_state_and_the_image_before_the_change_memory,
    lock_recipe_gives_the_state_and_the_image_before_the_change_postgres
);
contract_case!(
    lock_recipe_of_a_missing_recipe_gives_none,
    lock_recipe_of_a_missing_recipe_gives_none_memory,
    lock_recipe_of_a_missing_recipe_gives_none_postgres
);
contract_case!(
    lock_recipes_of_machine_gives_the_recipes_of_that_machine_ordered_by_id,
    lock_recipes_of_machine_gives_the_recipes_of_that_machine_ordered_by_id_memory,
    lock_recipes_of_machine_gives_the_recipes_of_that_machine_ordered_by_id_postgres
);
contract_case!(
    lock_versions_gives_every_version_with_its_image_in_id_order,
    lock_versions_gives_every_version_with_its_image_in_id_order_memory,
    lock_versions_gives_every_version_with_its_image_in_id_order_postgres
);
contract_case!(
    lock_version_gives_the_image_before_the_change,
    lock_version_gives_the_image_before_the_change_memory,
    lock_version_gives_the_image_before_the_change_postgres
);
contract_case!(
    lock_version_shared_of_a_draft_gives_no_approver,
    lock_version_shared_of_a_draft_gives_no_approver_memory,
    lock_version_shared_of_a_draft_gives_no_approver_postgres
);
contract_case!(
    lock_version_shared_gives_the_approval_of_an_approved_and_a_retired_version,
    lock_version_shared_gives_the_approval_of_an_approved_and_a_retired_version_memory,
    lock_version_shared_gives_the_approval_of_an_approved_and_a_retired_version_postgres
);
