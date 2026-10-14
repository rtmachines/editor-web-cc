//! Contract cases of `insert_recipe`, `rename_recipe`, `mark_recipe_deleted`, and `restore_recipe`.

use recipe_app::error::{RECIPE_REFERENCE_UNIQUE, StorageError};
use recipe_domain::recipe::VersionState;
use recipe_domain::text::Text;

use crate::contract::harness::{Harness, begin, commit};
use crate::contract::images::{Stage, assert_shape, field, recipe_shape, text, version_shape};
use crate::contract::recipe_seed::{
    build_recipe, create_recipe, first_version, plant, read_recipe, version_id,
};
use crate::contract::seed::machine_id;

async fn insert_recipe_gives_the_images_of_the_recipe_and_its_first_version(h: &dyn Harness) {
    let plant = plant(h, 1, "M1").await;
    let recipe = build_recipe(&plant, 1, "REF-1");
    let mut tx = begin(h).await;

    let inserted = tx
        .recipes()
        .insert_recipe(&recipe, first_version(h, &recipe))
        .await
        .unwrap();
    commit(h, tx, plant.machine).await;

    assert_shape(&inserted.recipe, &recipe_shape(false));
    assert_eq!(text(&inserted.recipe, "id"), recipe.id().to_string());
    assert_eq!(text(&inserted.recipe, "reference_number"), "REF-1");
    assert_eq!(text(&inserted.recipe, "name"), "Recipe");
    assert_eq!(text(&inserted.recipe, "plc_platform"), "TwinCat3");
    assert_eq!(
        text(&inserted.recipe, "machine_id"),
        plant.machine.to_string()
    );
    assert_eq!(text(&inserted.recipe, "created_by"), h.user().to_string());
    assert_shape(&inserted.version, &version_shape(false, Stage::Draft));
    assert_eq!(text(&inserted.version, "id"), version_id(1).to_string());
    assert_eq!(
        text(&inserted.version, "recipe_id"),
        recipe.id().to_string()
    );
    assert_eq!(text(&inserted.version, "status"), "draft");
    assert_eq!(text(&inserted.version, "change_note"), "Fixture note");
    assert_eq!(text(&inserted.version, "plc_platform"), "TwinCat3");
    assert_eq!(field(&inserted.version, "number").as_i64(), Some(1));
    assert_eq!(field(&inserted.version, "row_version").as_i64(), Some(1));
    assert_eq!(read_recipe(h, recipe.id()).await, Some(recipe));
}

async fn a_known_reference_number_gives_the_constraint_of_the_unique_index(h: &dyn Harness) {
    let plant = plant(h, 2, "M2").await;
    create_recipe(h, &plant, 2, "REF-2").await;
    let other = build_recipe(&plant, 3, "REF-2");
    let mut tx = begin(h).await;

    let error = tx
        .recipes()
        .insert_recipe(&other, first_version(h, &other))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Duplicate { constraint }
            if constraint == RECIPE_REFERENCE_UNIQUE),
        "{error:?}"
    );
}

async fn a_recipe_of_a_missing_machine_gives_a_missing_reference(h: &dyn Harness) {
    let mut plant = plant(h, 4, "M4").await;
    plant.machine = machine_id(999);
    let recipe = build_recipe(&plant, 4, "REF-4");
    let mut tx = begin(h).await;

    let error = tx
        .recipes()
        .insert_recipe(&recipe, first_version(h, &recipe))
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::MissingReference { constraint }
            if constraint == "recipe_machine_platform"),
        "{error:?}"
    );
}

async fn rename_recipe_gives_the_image_after_the_change(h: &dyn Harness) {
    let plant = plant(h, 5, "M5").await;
    let created = create_recipe(h, &plant, 5, "REF-5").await;
    let mut tx = begin(h).await;
    let locked = tx
        .recipes()
        .lock_recipe(created.id())
        .await
        .unwrap()
        .unwrap();

    let image = tx
        .recipes()
        .rename_recipe(created.id(), &Text::parse("Renamed").unwrap())
        .await
        .unwrap();
    commit(h, tx, plant.machine).await;

    assert_shape(&image, &recipe_shape(false));
    assert_eq!(text(&image, "name"), "Renamed");
    assert_eq!(text(&image, "reference_number"), "REF-5");
    assert_eq!(text(&locked.before, "name"), "Recipe");
    let reloaded = read_recipe(h, created.id()).await.unwrap();
    assert_eq!(reloaded.name().as_str(), "Renamed");
}

async fn delete_and_restore_set_and_clear_the_deletion_mark(h: &dyn Harness) {
    let plant = plant(h, 6, "M6").await;
    let created = create_recipe(h, &plant, 6, "REF-6").await;
    let draft = created.versions()[0].id();
    let mut tx = begin(h).await;
    tx.recipes()
        .lock_recipe(created.id())
        .await
        .unwrap()
        .unwrap();
    tx.recipes().lock_versions(created.id()).await.unwrap();
    tx.recipes().discard_version(draft).await.unwrap();

    let deleted = tx
        .recipes()
        .mark_recipe_deleted(created.id())
        .await
        .unwrap();
    commit(h, tx, plant.machine).await;

    assert_shape(&deleted, &recipe_shape(true));
    let after_delete = read_recipe(h, created.id()).await.unwrap();
    assert!(after_delete.is_deleted());
    assert_eq!(after_delete.versions()[0].state(), VersionState::Discarded);
    let mut tx = begin(h).await;
    tx.recipes()
        .lock_recipe(created.id())
        .await
        .unwrap()
        .unwrap();

    let restored = tx.recipes().restore_recipe(created.id()).await.unwrap();
    commit(h, tx, plant.machine).await;

    assert_shape(&restored, &recipe_shape(false));
    let after_restore = read_recipe(h, created.id()).await.unwrap();
    assert!(!after_restore.is_deleted());
    assert_eq!(after_restore.versions()[0].state(), VersionState::Discarded);
}

contract_case!(
    insert_recipe_gives_the_images_of_the_recipe_and_its_first_version,
    insert_recipe_gives_the_images_of_the_recipe_and_its_first_version_memory,
    insert_recipe_gives_the_images_of_the_recipe_and_its_first_version_postgres
);
contract_case!(
    a_known_reference_number_gives_the_constraint_of_the_unique_index,
    a_known_reference_number_gives_the_constraint_of_the_unique_index_memory,
    a_known_reference_number_gives_the_constraint_of_the_unique_index_postgres
);
contract_case!(
    a_recipe_of_a_missing_machine_gives_a_missing_reference,
    a_recipe_of_a_missing_machine_gives_a_missing_reference_memory,
    a_recipe_of_a_missing_machine_gives_a_missing_reference_postgres
);
contract_case!(
    rename_recipe_gives_the_image_after_the_change,
    rename_recipe_gives_the_image_after_the_change_memory,
    rename_recipe_gives_the_image_after_the_change_postgres
);
contract_case!(
    delete_and_restore_set_and_clear_the_deletion_mark,
    delete_and_restore_set_and_clear_the_deletion_mark_memory,
    delete_and_restore_set_and_clear_the_deletion_mark_postgres
);
