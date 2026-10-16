//! Contract cases of `RecipeReads::list_recipes` (design D5).

use recipe_app::error::StorageError;
use recipe_app::ports::{RecipeFilter, RecipeSummary};
use recipe_domain::recipe::{ReferenceNumber, VersionState};
use recipe_domain::text::Text;

use super::fixtures::{WHOLE, begin_read, create_named, delete_recipe, discard, list, references};
use crate::contract::harness::Harness;
use crate::contract::recipe_seed::{approve, create_version, plant, retire, version_id};
use crate::contract::seed::{archive, code};

/// A filter with a search text only.
fn search(text: &str) -> RecipeFilter {
    RecipeFilter {
        text: Some(Text::parse(text).unwrap()),
        ..RecipeFilter::default()
    }
}

/// A filter with a state only.
fn in_state(state: VersionState) -> RecipeFilter {
    RecipeFilter {
        status: Some(state),
        ..RecipeFilter::default()
    }
}

async fn recipes_come_in_the_byte_order_of_the_reference_number(h: &dyn Harness) {
    let plant = plant(h, 1, "M1").await;
    // Byte order: '-' (0x2D) < '1' (0x31) < '2' (0x32), so R-2 < R10 < R2.
    for (n, reference) in [(1, "R2"), (2, "R10"), (3, "R-2"), (4, "A1")] {
        create_named(h, &plant, n, reference, "Recipe").await;
    }

    let rows = list(h, &RecipeFilter::default(), WHOLE).await;

    assert_eq!(references(&rows), ["A1", "R-2", "R10", "R2"]);
    assert_eq!(
        rows[0],
        RecipeSummary {
            reference: ReferenceNumber::parse("A1").unwrap(),
            name: Text::parse("Recipe").unwrap(),
            machine_code: code("M1"),
            deleted: false,
            approved: None,
            draft: Some(1),
        }
    );
}

async fn a_search_matches_a_part_of_the_name_or_the_reference_in_any_case(h: &dyn Harness) {
    let plant = plant(h, 5, "M5").await;
    create_named(h, &plant, 5, "REF-5", "Łódź").await;
    create_named(h, &plant, 6, "ZZ-6", "Press").await;
    create_named(h, &plant, 7, "REF-7", "Line").await;

    let upper_lower = list(h, &search("łÓDŹ"), WHOLE).await;
    let all_upper = list(h, &search("ŁÓDŹ"), WHOLE).await;
    let reference_part = list(h, &search("ref"), WHOLE).await;
    let reference_tail = list(h, &search("f-7"), WHOLE).await;
    let name_part = list(h, &search("RESS"), WHOLE).await;
    let nothing = list(h, &search("none"), WHOLE).await;

    assert_eq!(references(&upper_lower), ["REF-5"]);
    assert_eq!(references(&all_upper), ["REF-5"]);
    assert_eq!(references(&reference_part), ["REF-5", "REF-7"]);
    assert_eq!(references(&reference_tail), ["REF-7"]);
    assert_eq!(references(&name_part), ["ZZ-6"]);
    assert!(nothing.is_empty(), "{nothing:?}");
}

async fn the_machine_filter_keeps_the_recipes_of_one_machine(h: &dyn Harness) {
    let first = plant(h, 8, "M8").await;
    let second = plant(h, 9, "M9").await;
    create_named(h, &first, 8, "REF-8", "Recipe").await;
    create_named(h, &second, 9, "REF-9", "Recipe").await;
    let of = |machine| RecipeFilter {
        machine: Some(code(machine)),
        ..RecipeFilter::default()
    };

    let on_first = list(h, &of("M8"), WHOLE).await;
    let on_second = list(h, &of("M9"), WHOLE).await;
    let on_none = list(h, &of("M77"), WHOLE).await;

    assert_eq!(references(&on_first), ["REF-8"]);
    assert_eq!(references(&on_second), ["REF-9"]);
    assert!(on_none.is_empty(), "{on_none:?}");
}

async fn the_machine_filter_keeps_only_the_recipes_of_the_live_machine_of_the_code(
    h: &dyn Harness,
) {
    let old = plant(h, 40, "M40").await;
    let old_recipe = create_named(h, &old, 40, "REF-OLD", "Recipe").await;
    approve(h, old_recipe.versions()[0].id()).await;
    archive(h, old.machine).await;
    let live = plant(h, 41, "M40").await;
    create_named(h, &live, 41, "REF-LIVE", "Recipe").await;
    let of_code = RecipeFilter {
        machine: Some(code("M40")),
        ..RecipeFilter::default()
    };

    let filtered = list(h, &of_code, WHOLE).await;
    let every = list(h, &RecipeFilter::default(), WHOLE).await;

    assert_eq!(references(&filtered), ["REF-LIVE"]);
    assert_eq!(references(&every), ["REF-LIVE", "REF-OLD"]);
}

async fn the_status_filter_keeps_recipes_with_a_version_in_that_state(h: &dyn Harness) {
    let plant = plant(h, 10, "M10").await;
    create_named(h, &plant, 10, "R-DRAFT", "Recipe").await;
    let approved = create_named(h, &plant, 11, "R-APPROVED", "Recipe").await;
    approve(h, approved.versions()[0].id()).await;
    let retired = create_named(h, &plant, 12, "R-RETIRED", "Recipe").await;
    approve(h, retired.versions()[0].id()).await;
    retire(h, retired.versions()[0].id()).await;
    create_version(h, (retired.id(), version_id(112)), 2, None).await;
    approve(h, version_id(112)).await;
    create_version(h, (retired.id(), version_id(212)), 3, None).await;
    let discarded = create_named(h, &plant, 13, "R-DISCARDED", "Recipe").await;
    discard(h, discarded.versions()[0].id()).await;

    let live = list(h, &in_state(VersionState::LiveDraft), WHOLE).await;
    let approved = list(h, &in_state(VersionState::Approved), WHOLE).await;
    let retired = list(h, &in_state(VersionState::Retired), WHOLE).await;
    let discarded = list(h, &in_state(VersionState::Discarded), WHOLE).await;

    assert_eq!(references(&live), ["R-DRAFT", "R-RETIRED"]);
    assert_eq!(references(&approved), ["R-APPROVED", "R-RETIRED"]);
    assert_eq!(references(&retired), ["R-RETIRED"]);
    assert_eq!(references(&discarded), ["R-DISCARDED"]);
    assert_eq!((retired[0].approved, retired[0].draft), (Some(2), Some(3)));
    assert_eq!((discarded[0].approved, discarded[0].draft), (None, None));
}

async fn deleted_recipes_show_only_with_the_flag(h: &dyn Harness) {
    let plant = plant(h, 14, "M14").await;
    create_named(h, &plant, 14, "REF-14", "Recipe").await;
    let deleted = create_named(h, &plant, 15, "REF-15", "Recipe").await;
    delete_recipe(h, &deleted).await;
    let with_deleted = RecipeFilter {
        include_deleted: true,
        ..RecipeFilter::default()
    };

    let live_only = list(h, &RecipeFilter::default(), WHOLE).await;
    let all = list(h, &with_deleted, WHOLE).await;

    assert_eq!(references(&live_only), ["REF-14"]);
    assert_eq!(references(&all), ["REF-14", "REF-15"]);
    assert_eq!((all[0].deleted, all[1].deleted), (false, true));
}

async fn offset_and_limit_cut_the_page(h: &dyn Harness) {
    let plant = plant(h, 16, "M16").await;
    for (n, reference) in [(16, "R1"), (17, "R2"), (18, "R3"), (19, "R4"), (20, "R5")] {
        create_named(h, &plant, n, reference, "Recipe").await;
    }
    let all = RecipeFilter::default();

    let first = list(h, &all, (0, 3)).await;
    let middle = list(h, &all, (1, 2)).await;
    let tail = list(h, &all, (4, 10)).await;
    let beyond = list(h, &all, (5, 10)).await;
    let none = list(h, &all, (0, 0)).await;

    assert_eq!(references(&first), ["R1", "R2", "R3"]);
    assert_eq!(references(&middle), ["R2", "R3"]);
    assert_eq!(references(&tail), ["R5"]);
    assert!(beyond.is_empty(), "{beyond:?}");
    assert!(none.is_empty(), "{none:?}");
}

async fn an_offset_above_bigint_is_a_storage_error_not_an_empty_list(h: &dyn Harness) {
    let plant = plant(h, 21, "M21").await;
    create_named(h, &plant, 21, "REF-21", "Recipe").await;
    let largest = u64::try_from(i64::MAX).unwrap();
    let mut tx = begin_read(h).await;

    let at_limit = tx
        .recipe_reads()
        .list_recipes(&RecipeFilter::default(), largest, 10)
        .await;
    let beyond = tx
        .recipe_reads()
        .list_recipes(
            &RecipeFilter::default(),
            largest.checked_add(1).unwrap(),
            10,
        )
        .await;

    assert!(at_limit.unwrap().is_empty());
    assert!(matches!(
        beyond.unwrap_err(),
        StorageError::Other { sqlstate: None, .. }
    ));
}

contract_case!(
    recipes_come_in_the_byte_order_of_the_reference_number,
    recipes_come_in_the_byte_order_of_the_reference_number_memory,
    recipes_come_in_the_byte_order_of_the_reference_number_postgres
);
contract_case!(
    a_search_matches_a_part_of_the_name_or_the_reference_in_any_case,
    a_search_matches_a_part_of_the_name_or_the_reference_in_any_case_memory,
    a_search_matches_a_part_of_the_name_or_the_reference_in_any_case_postgres
);
contract_case!(
    the_machine_filter_keeps_the_recipes_of_one_machine,
    the_machine_filter_keeps_the_recipes_of_one_machine_memory,
    the_machine_filter_keeps_the_recipes_of_one_machine_postgres
);
contract_case!(
    the_machine_filter_keeps_only_the_recipes_of_the_live_machine_of_the_code,
    the_machine_filter_keeps_only_the_recipes_of_the_live_machine_of_the_code_memory,
    the_machine_filter_keeps_only_the_recipes_of_the_live_machine_of_the_code_postgres
);
contract_case!(
    the_status_filter_keeps_recipes_with_a_version_in_that_state,
    the_status_filter_keeps_recipes_with_a_version_in_that_state_memory,
    the_status_filter_keeps_recipes_with_a_version_in_that_state_postgres
);
contract_case!(
    deleted_recipes_show_only_with_the_flag,
    deleted_recipes_show_only_with_the_flag_memory,
    deleted_recipes_show_only_with_the_flag_postgres
);
contract_case!(
    offset_and_limit_cut_the_page,
    offset_and_limit_cut_the_page_memory,
    offset_and_limit_cut_the_page_postgres
);
contract_case!(
    an_offset_above_bigint_is_a_storage_error_not_an_empty_list,
    an_offset_above_bigint_is_a_storage_error_not_an_empty_list_memory,
    an_offset_above_bigint_is_a_storage_error_not_an_empty_list_postgres
);
