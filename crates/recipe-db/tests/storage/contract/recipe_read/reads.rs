//! Contract cases of the three reads of `RecipeRepository`.

use recipe_domain::recipe::{ReferenceNumber, VersionState};

use crate::contract::harness::{Harness, begin};
use crate::contract::recipe_seed::{
    approve, create_recipe, create_version, describe, length, parameter, plant, range, read_recipe,
    recipe_id, seed, translation, version_id,
};

async fn recipe_by_reference_and_by_id_read_the_same_recipe(h: &dyn Harness) {
    let plant = plant(h, 1, "M1").await;
    let created = create_recipe(h, &plant, 1, "REF-1").await;
    let mut tx = begin(h).await;

    let by_id = tx.recipes().recipe_by_id(created.id()).await.unwrap();
    let by_reference = tx
        .recipes()
        .recipe_by_reference(created.reference())
        .await
        .unwrap();

    assert_eq!(by_id.as_ref(), Some(&created));
    assert_eq!(by_reference, by_id);
    let recipe = by_id.unwrap();
    assert_eq!(recipe.machine(), plant.machine);
    assert_eq!(recipe.versions().len(), 1);
    assert_eq!(recipe.versions()[0].number(), 1);
    assert_eq!(recipe.versions()[0].state(), VersionState::LiveDraft);
    assert_eq!(recipe.versions()[0].row_version(), 1);
}

async fn a_missing_recipe_gives_none(h: &dyn Harness) {
    let mut tx = begin(h).await;

    let by_id = tx.recipes().recipe_by_id(recipe_id(99)).await.unwrap();
    let by_reference = tx
        .recipes()
        .recipe_by_reference(&ReferenceNumber::parse("NONE").unwrap())
        .await
        .unwrap();

    assert!(by_id.is_none());
    assert!(by_reference.is_none());
}

async fn the_recipe_holds_every_version_header_in_number_order(h: &dyn Harness) {
    let plant = plant(h, 2, "M2").await;
    let created = create_recipe(h, &plant, 2, "REF-2").await;
    let first = created.versions()[0].id();
    approve(h, first).await;
    // The second version has the smaller identifier, so the id order differs from the number order.
    create_version(h, (created.id(), version_id(1)), 2, Some(first)).await;

    let recipe = read_recipe(h, created.id()).await.unwrap();

    let numbers: Vec<u32> = recipe.versions().iter().map(|v| v.number()).collect();
    let states: Vec<VersionState> = recipe.versions().iter().map(|v| v.state()).collect();
    assert_eq!(numbers, vec![1, 2]);
    assert_eq!(
        states,
        vec![VersionState::Approved, VersionState::LiveDraft]
    );
    assert_eq!(recipe.versions()[0].row_version(), 2);
    assert_eq!(recipe.versions()[1].id(), version_id(1));
}

async fn version_parameters_are_ordered_by_tag_in_byte_order(h: &dyn Harness) {
    let plant = plant(h, 4, "M4").await;
    let created = create_recipe(h, &plant, 4, "REF-4").await;
    let version = created.versions()[0].id();
    // Byte order: AB (0x42) < A_B (0x5F), TAG_10 < TAG_2 ('1' < '2'), Z9 ('9') < Z_ ('_').
    for (n, tag) in [
        (1, "Z_"),
        (2, "TAG_2"),
        (3, "A_B"),
        (4, "Z9"),
        (5, "AB"),
        (6, "TAG_10"),
    ] {
        seed(h, version, &parameter(&plant, n, tag)).await;
    }
    let mut tx = begin(h).await;

    let read = tx.recipes().version_parameters(version).await.unwrap();

    let tags: Vec<&str> = read.iter().map(|p| p.tag.as_str()).collect();
    assert_eq!(tags, vec!["AB", "A_B", "TAG_10", "TAG_2", "Z9", "Z_"]);
}

async fn version_parameters_hold_limits_and_translations_in_byte_order(h: &dyn Harness) {
    let plant = plant(h, 5, "M5").await;
    let created = create_recipe(h, &plant, 5, "REF-5").await;
    let version = created.versions()[0].id();
    let mut full = parameter(&plant, 1, "FULL");
    // Byte order of the paths: "" < ".B" < ".a" < "[*]". Byte order of the languages:
    // "cs" < "de" < "pl" < "pt" < "pt-BR".
    full.limits = vec![
        range("[*]", "-1.50", "9"),
        length(".a", 12),
        range("", "0", "10.25"),
        range(".B", "1", "2"),
    ];
    full.translations = vec![
        translation("pt-BR", "Nome", Some("Descricao")),
        translation("pl", "Nazwa", None),
        translation("de", "Name", Some("Beschreibung")),
        translation("pt", "Nome", None),
        translation("cs", "Jmeno", None),
    ];
    let plain = parameter(&plant, 2, "PLAIN");
    seed(h, version, &full).await;
    seed(h, version, &plain).await;
    let mut expected = full.clone();
    expected.limits = vec![
        range("", "0", "10.25"),
        range(".B", "1", "2"),
        length(".a", 12),
        range("[*]", "-1.50", "9"),
    ];
    expected.translations = vec![
        translation("cs", "Jmeno", None),
        translation("de", "Name", Some("Beschreibung")),
        translation("pl", "Nazwa", None),
        translation("pt", "Nome", None),
        translation("pt-BR", "Nome", Some("Descricao")),
    ];
    let mut tx = begin(h).await;

    let read = tx.recipes().version_parameters(version).await.unwrap();

    assert_eq!(describe(&read), describe(&[expected, plain]));
    assert_eq!(read[0].limits.len(), 4);
    assert_eq!(read[0].translations.len(), 5);
    assert!(read[1].limits.is_empty());
    assert!(read[1].translations.is_empty());
}

async fn version_parameters_read_any_version_and_only_that_version(h: &dyn Harness) {
    let plant = plant(h, 6, "M6").await;
    let created = create_recipe(h, &plant, 6, "REF-6").await;
    let first = created.versions()[0].id();
    seed(h, first, &parameter(&plant, 1, "ONLY_V1")).await;
    approve(h, first).await;
    create_version(h, (created.id(), version_id(7)), 2, None).await;
    seed(h, version_id(7), &parameter(&plant, 2, "ONLY_V2")).await;
    let mut tx = begin(h).await;

    let approved = tx.recipes().version_parameters(first).await.unwrap();
    let draft = tx
        .recipes()
        .version_parameters(version_id(7))
        .await
        .unwrap();
    let missing = tx
        .recipes()
        .version_parameters(version_id(99))
        .await
        .unwrap();

    assert_eq!(approved.len(), 1);
    assert_eq!(approved[0].tag.as_str(), "ONLY_V1");
    assert_eq!(draft.len(), 1);
    assert_eq!(draft[0].tag.as_str(), "ONLY_V2");
    assert!(missing.is_empty());
}

contract_case!(
    recipe_by_reference_and_by_id_read_the_same_recipe,
    recipe_by_reference_and_by_id_read_the_same_recipe_memory,
    recipe_by_reference_and_by_id_read_the_same_recipe_postgres
);
contract_case!(
    a_missing_recipe_gives_none,
    a_missing_recipe_gives_none_memory,
    a_missing_recipe_gives_none_postgres
);
contract_case!(
    the_recipe_holds_every_version_header_in_number_order,
    the_recipe_holds_every_version_header_in_number_order_memory,
    the_recipe_holds_every_version_header_in_number_order_postgres
);
contract_case!(
    version_parameters_are_ordered_by_tag_in_byte_order,
    version_parameters_are_ordered_by_tag_in_byte_order_memory,
    version_parameters_are_ordered_by_tag_in_byte_order_postgres
);
contract_case!(
    version_parameters_hold_limits_and_translations_in_byte_order,
    version_parameters_hold_limits_and_translations_in_byte_order_memory,
    version_parameters_hold_limits_and_translations_in_byte_order_postgres
);
contract_case!(
    version_parameters_read_any_version_and_only_that_version,
    version_parameters_read_any_version_and_only_that_version_memory,
    version_parameters_read_any_version_and_only_that_version_postgres
);
