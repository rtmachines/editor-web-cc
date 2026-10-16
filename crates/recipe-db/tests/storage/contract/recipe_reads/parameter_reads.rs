//! Contract cases of `parameter_page`, `unit_counts`, and `parameter_by_key` (design D5).

use recipe_app::ports::{ParameterRow, UnitCount};
use recipe_domain::plant::UnitId;
use recipe_domain::recipe::{ParameterKey, RecipeVersionId};
use uuid::Uuid;

use super::fixtures::{WHOLE, begin_read, create_named, in_unit, tree_plant};
use crate::contract::harness::Harness;
use crate::contract::recipe_seed::{
    describe, length, parameter, plant, range, seed, translation, version_id,
};
use crate::contract::seed::{U1_1, U1_1_2, U1_2, U1_10, U2, path, tree_unit_id};

/// Read one page of the parameters in a transaction that nothing commits.
async fn page(
    h: &dyn Harness,
    version: RecipeVersionId,
    unit: Option<UnitId>,
    (offset, limit): (u64, u32),
) -> Vec<ParameterRow> {
    let mut tx = begin_read(h).await;
    tx.recipe_reads()
        .parameter_page(version, unit, offset, limit)
        .await
        .unwrap()
}

/// Return the tags of the rows as text, in the order of the rows.
fn tags(rows: &[ParameterRow]) -> Vec<&str> {
    rows.iter().map(|row| row.tag.as_str()).collect()
}

/// Return the unit paths of the rows as text, in the order of the rows.
fn paths(rows: &[ParameterRow]) -> Vec<String> {
    rows.iter().map(|row| row.unit_path.to_string()).collect()
}

async fn parameter_rows_come_in_the_order_of_unit_numbers_ordinal_and_tag(h: &dyn Harness) {
    let plant = tree_plant(h, 30, "M30").await;
    let recipe = create_named(h, &plant, 30, "REF-30", "Recipe").await;
    let version = recipe.versions()[0].id();
    let unit = |offset| tree_unit_id(30, offset);
    // As text, U1.10 comes before U1.2. By numbers, U1.2 comes first.
    for (n, tag, unit, ordinal) in [
        (1, "B_TAG", unit(U1_10), 0),
        (2, "TAG_A", unit(U1_2), 5),
        (3, "Z_TAG", unit(U1_2), 1),
        (4, "M_TAG", unit(U2), 0),
        (5, "C_TAG", unit(U1_1_2), 0),
        (6, "TAG_B", plant.unit, 0),
        (7, "A_TAG", unit(U1_2), 5),
    ] {
        seed(h, version, &in_unit(&plant, n, tag, unit, ordinal)).await;
    }

    let rows = page(h, version, None, WHOLE).await;

    assert_eq!(
        tags(&rows),
        [
            "TAG_B", "C_TAG", "Z_TAG", "A_TAG", "TAG_A", "B_TAG", "M_TAG"
        ]
    );
    assert_eq!(
        paths(&rows),
        ["U1", "U1.1.2", "U1.2", "U1.2", "U1.2", "U1.10", "U2"]
    );
    let expected = in_unit(&plant, 3, "Z_TAG", unit(U1_2), 1);
    assert_eq!(
        rows[2],
        ParameterRow {
            key: expected.key,
            unit: expected.unit,
            unit_path: path("U1.2"),
            tag: expected.tag,
            name_en: expected.name_en,
            ordinal: expected.ordinal,
            plc_type: plant.dint,
            s7_address: None,
        }
    );
}

async fn the_unit_filter_and_the_page_cut_the_parameter_rows(h: &dyn Harness) {
    let plant = tree_plant(h, 31, "M31").await;
    let recipe = create_named(h, &plant, 31, "REF-31", "Recipe").await;
    let version = recipe.versions()[0].id();
    let second = tree_unit_id(31, U1_2);
    for (n, tag, ordinal) in [(1, "P0", 0), (2, "P1", 1), (3, "P2", 2)] {
        seed(h, version, &in_unit(&plant, n, tag, second, ordinal)).await;
    }
    seed(
        h,
        version,
        &in_unit(&plant, 4, "Q0", tree_unit_id(31, U2), 0),
    )
    .await;

    let of_unit = page(h, version, Some(second), WHOLE).await;
    let middle = page(h, version, Some(second), (1, 1)).await;
    let empty_unit = page(h, version, Some(tree_unit_id(31, U1_1)), WHOLE).await;
    let tail = page(h, version, None, (2, 10)).await;
    let other_version = page(h, version_id(99), None, WHOLE).await;

    assert_eq!(tags(&of_unit), ["P0", "P1", "P2"]);
    assert_eq!(tags(&middle), ["P1"]);
    assert!(empty_unit.is_empty(), "{empty_unit:?}");
    assert_eq!(tags(&tail), ["P2", "Q0"]);
    assert!(other_version.is_empty(), "{other_version:?}");
}

async fn unit_counts_count_the_parameters_of_each_unit_in_the_order_of_the_path(h: &dyn Harness) {
    let plant = tree_plant(h, 32, "M32").await;
    let recipe = create_named(h, &plant, 32, "REF-32", "Recipe").await;
    let version = recipe.versions()[0].id();
    let unit = |offset| tree_unit_id(32, offset);
    for (n, tag, unit) in [
        (1, "TEN", unit(U1_10)),
        (2, "TWO_A", unit(U1_2)),
        (3, "TWO_B", unit(U1_2)),
        (4, "MAIN_TWO", unit(U2)),
    ] {
        seed(h, version, &in_unit(&plant, n, tag, unit, 0)).await;
    }
    let mut tx = begin_read(h).await;

    let counts = tx.recipe_reads().unit_counts(version).await.unwrap();
    let none = tx.recipe_reads().unit_counts(version_id(99)).await.unwrap();

    assert_eq!(
        counts,
        [
            UnitCount {
                unit: unit(U1_2),
                path: path("U1.2"),
                count: 2,
            },
            UnitCount {
                unit: unit(U1_10),
                path: path("U1.10"),
                count: 1,
            },
            UnitCount {
                unit: unit(U2),
                path: path("U2"),
                count: 1,
            },
        ]
    );
    assert!(none.is_empty(), "{none:?}");
}

async fn parameter_by_key_gives_the_parameter_with_its_limits_and_translations(h: &dyn Harness) {
    let plant = plant(h, 33, "M33").await;
    let recipe = create_named(h, &plant, 33, "REF-33", "Recipe").await;
    let version = recipe.versions()[0].id();
    let mut full = parameter(&plant, 1, "FULL");
    full.limits = vec![range("", "0", "10.25"), length(".B", 12)];
    full.translations = vec![
        translation("de", "Name", Some("Beschreibung")),
        translation("pl", "Nazwa", None),
    ];
    let plain = parameter(&plant, 2, "PLAIN");
    seed(h, version, &full).await;
    seed(h, version, &plain).await;
    let mut tx = begin_read(h).await;

    let read = tx
        .recipe_reads()
        .parameter_by_key(version, full.key)
        .await
        .unwrap();
    let missing = tx
        .recipe_reads()
        .parameter_by_key(version, ParameterKey::from(Uuid::from_u128(0xDEAD)))
        .await
        .unwrap();
    let other_version = tx
        .recipe_reads()
        .parameter_by_key(version_id(99), full.key)
        .await
        .unwrap();

    assert_eq!(describe(&[read.unwrap()]), describe(&[full]));
    assert!(missing.is_none());
    assert!(other_version.is_none());
}

contract_case!(
    parameter_rows_come_in_the_order_of_unit_numbers_ordinal_and_tag,
    parameter_rows_come_in_the_order_of_unit_numbers_ordinal_and_tag_memory,
    parameter_rows_come_in_the_order_of_unit_numbers_ordinal_and_tag_postgres
);
contract_case!(
    the_unit_filter_and_the_page_cut_the_parameter_rows,
    the_unit_filter_and_the_page_cut_the_parameter_rows_memory,
    the_unit_filter_and_the_page_cut_the_parameter_rows_postgres
);
contract_case!(
    unit_counts_count_the_parameters_of_each_unit_in_the_order_of_the_path,
    unit_counts_count_the_parameters_of_each_unit_in_the_order_of_the_path_memory,
    unit_counts_count_the_parameters_of_each_unit_in_the_order_of_the_path_postgres
);
contract_case!(
    parameter_by_key_gives_the_parameter_with_its_limits_and_translations,
    parameter_by_key_gives_the_parameter_with_its_limits_and_translations_memory,
    parameter_by_key_gives_the_parameter_with_its_limits_and_translations_postgres
);
