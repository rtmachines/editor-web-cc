//! Tests of the size limits of `resolve` (`reliability.md` section 7.11).

use super::fixtures::{array_row, id, primitive_row, string_row, struct_row, types};
use super::*;
use crate::error_codes::ErrorCode;
use crate::plant::limits::{MAX_DEPTH, MAX_LEAVES, MAX_S7_NESTING};

const INT: u128 = 1;

fn with_int(mut rows: Vec<PlcType>) -> PlatformTypes {
    rows.push(primitive_row(INT, Primitive::Int));
    types(rows)
}

fn resolved(root: u128, rows: Vec<PlcType>) -> Result<TypeTree, PlantError> {
    TypeTree::resolve(id(root), &with_int(rows))
}

/// Return the name and the limit of an `E-PLT-018` error.
fn too_large(result: Result<TypeTree, PlantError>) -> Option<(String, TypeLimit)> {
    let error = result.err()?;
    assert_eq!(error.code(), ErrorCode::PLT_018);
    match error {
        PlantError::TypeTooLarge { type_name, limit } => Some((type_name, limit)),
        _ => None,
    }
}

/// Struct `ST_1` has a member of `ST_2` and so on. The last struct holds an `INT`.
fn struct_chain(levels: u128) -> Vec<PlcType> {
    (1..=levels)
        .map(|level| {
            let next = if level == levels {
                INT
            } else {
                level.checked_add(101).unwrap()
            };
            struct_row(
                level.checked_add(100).unwrap(),
                &format!("ST_{level}"),
                &[("m", next)],
            )
        })
        .collect()
}

/// Array rows from the inside out. Each prefix wraps the rows before it.
fn array_chain(prefixes: &[&[(i32, i32)]]) -> (u128, Vec<PlcType>) {
    let mut element = INT;
    let mut rows = Vec::new();
    for (next_id, bounds) in (200..).zip(prefixes.iter().rev()) {
        rows.push(array_row(next_id, element, bounds));
        element = next_id;
    }
    (element, rows)
}

#[test]
fn limits_equal_the_values_of_section_7_11() {
    assert_eq!(MAX_LEAVES, 16_384);
    assert_eq!(MAX_DEPTH, 32);
}

#[test]
fn array_with_16384_leaves_passes() {
    let tree = resolved(10, vec![array_row(10, INT, &[(1, 16_384)])]).unwrap();
    assert_eq!(tree.leaf_count(), 16_384);
}

#[test]
fn array_with_16385_leaves_fails_with_leaves() {
    let result = resolved(10, vec![array_row(10, INT, &[(1, 16_385)])]);
    assert_eq!(
        too_large(result),
        Some(("ARRAY[1..16385] OF INT".to_owned(), TypeLimit::Leaves))
    );
}

#[test]
fn two_dimensional_array_with_16384_leaves_passes() {
    let tree = resolved(10, vec![array_row(10, INT, &[(1, 128), (1, 128)])]).unwrap();
    assert_eq!(tree.leaf_count(), 16_384);
}

#[test]
fn struct_of_two_large_arrays_fails_with_leaves_and_the_struct_name() {
    let rows = vec![
        array_row(10, INT, &[(1, 8_193)]),
        struct_row(11, "ST_Big", &[("a", 10), ("b", 10)]),
    ];
    let result = resolved(11, rows);
    assert_eq!(
        too_large(result),
        Some(("ST_Big@1".to_owned(), TypeLimit::Leaves))
    );
}

#[test]
fn struct_chain_of_32_levels_passes() {
    let tree = resolved(101, struct_chain(32)).unwrap();
    assert_eq!(tree.depth(), 32);
    assert_eq!(tree.leaf_count(), 1);
}

#[test]
fn struct_chain_of_33_levels_fails_with_depth() {
    let result = resolved(101, struct_chain(33));
    assert_eq!(
        too_large(result),
        Some(("ST_1@1".to_owned(), TypeLimit::Depth))
    );
}

#[test]
fn array_levels_count_one_per_dimension() {
    let three = &[(1, 1), (1, 1), (1, 1)][..];
    let two = &[(1, 1), (1, 1)][..];
    let mut passing = vec![two];
    passing.extend(std::iter::repeat_n(three, 10));
    let (root, rows) = array_chain(&passing);
    assert_eq!(resolved(root, rows).unwrap().depth(), 32);

    let failing = vec![three; 11];
    let (root, rows) = array_chain(&failing);
    let result = resolved(root, rows);
    assert_eq!(
        too_large(result).map(|(_, limit)| limit),
        Some(TypeLimit::Depth)
    );
}

#[test]
fn huge_array_fails_with_leaves_without_building_nodes() {
    let bounds = &[(0, 65_535), (0, 65_535), (0, 65_535)];
    let result = resolved(10, vec![array_row(10, INT, bounds)]);
    assert_eq!(
        too_large(result),
        Some((
            "ARRAY[0..65535,0..65535,0..65535] OF INT".to_owned(),
            TypeLimit::Leaves
        ))
    );
}

#[test]
fn nested_array_name_shows_every_array_level() {
    let (root, rows) = array_chain(&[&[(1, 20_000)], &[(1, 2)]]);
    let result = resolved(root, rows);
    assert_eq!(
        too_large(result),
        Some((
            "ARRAY[1..20000] OF ARRAY[1..2] OF INT".to_owned(),
            TypeLimit::Leaves
        ))
    );
}

#[test]
fn string_array_over_the_leaf_limit_names_the_string_length() {
    // `E-PLT-018` names the root type in the text form of the `TypeExpr` grammar.
    let rows = vec![string_row(2, 10), array_row(10, 2, &[(1, 16_385)])];
    assert_eq!(
        too_large(resolved(10, rows)),
        Some((
            "ARRAY[1..16385] OF STRING(10)".to_owned(),
            TypeLimit::Leaves
        ))
    );
}

/// Metrics of a finished child row, built by hand to reach the overflow arms.
fn finished(child: u128, leaves: u64, depth: u32) -> BTreeMap<PlcTypeId, Metrics> {
    BTreeMap::from([(
        id(child),
        Metrics {
            leaves,
            depth,
            s7_nesting: 0,
            is_struct: false,
        },
    )])
}

#[test]
fn measure_reports_leaves_when_an_array_overflows_u64() {
    // Fault injection: `check_limits` keeps every real child at 16384 leaves or fewer.
    let row = array_row(10, 2, &[(1, 2)]);
    let error = measure(&row, &finished(2, u64::MAX, 0)).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_018);
    assert!(matches!(
        error,
        PlantError::TypeTooLarge {
            limit: TypeLimit::Leaves,
            ..
        }
    ));
}

#[test]
fn measure_reports_depth_when_an_array_overflows_u32() {
    // Fault injection: `check_limits` keeps every real child at depth 32 or less.
    let row = array_row(10, 2, &[(1, 2)]);
    let error = measure(&row, &finished(2, 1, u32::MAX)).unwrap_err();
    assert!(matches!(
        error,
        PlantError::TypeTooLarge {
            limit: TypeLimit::Depth,
            ..
        }
    ));
}

#[test]
fn measure_reports_depth_when_a_struct_overflows_u32() {
    let row = struct_row(10, "ST_A", &[("x", 2)]);
    let error = measure(&row, &finished(2, 1, u32::MAX)).unwrap_err();
    assert!(matches!(
        error,
        PlantError::TypeTooLarge {
            limit: TypeLimit::Depth,
            ..
        }
    ));
}

#[test]
fn measure_reports_leaves_when_a_struct_overflows_u64() {
    let row = struct_row(10, "ST_A", &[("x", 2), ("y", 2)]);
    let error = measure(&row, &finished(2, u64::MAX, 0)).unwrap_err();
    assert!(matches!(
        error,
        PlantError::TypeTooLarge {
            limit: TypeLimit::Leaves,
            ..
        }
    ));
}

#[test]
fn render_root_of_a_missing_row_is_its_identifier() {
    // Fault injection: `walk` enters every row of the root chain before it renders the root.
    assert_eq!(render_root(id(99), &types(Vec::new())), id(99).to_string());
}

/// The S7 platforms of ADR-0179 (a).
const S7_PLATFORMS: [PlcPlatform; 3] = [
    PlcPlatform::S7_300,
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
];

/// Resolve `root` and return its count of the S7 levels.
fn s7_count(root: u128, rows: Vec<PlcType>) -> u32 {
    resolved(root, rows).unwrap().s7_nesting
}

#[test]
fn adr_0179_max_s7_nesting_equals_the_value_of_section_7_11() {
    assert_eq!(MAX_S7_NESTING, 8);
}

#[test]
fn adr_0179_struct_counts_one_level() {
    assert_eq!(s7_count(101, struct_chain(1)), 1);
}

#[test]
fn adr_0179_chain_of_structs_counts_one_level_for_each_struct() {
    assert_eq!(s7_count(101, struct_chain(5)), 5);
}

#[test]
fn adr_0179_array_of_a_struct_counts_three_levels() {
    let mut rows = struct_chain(1);
    rows.push(array_row(10, 101, &[(1, 2)]));
    assert_eq!(s7_count(10, rows), 3);
}

#[test]
fn adr_0179_array_of_an_array_of_a_struct_counts_three_levels() {
    let mut rows = struct_chain(1);
    rows.push(array_row(10, 101, &[(1, 2)]));
    rows.push(array_row(11, 10, &[(1, 2)]));
    assert_eq!(s7_count(11, rows), 3);
}

#[test]
fn adr_0179_array_of_a_primitive_or_a_string_counts_zero_levels() {
    assert_eq!(s7_count(10, vec![array_row(10, INT, &[(1, 2)])]), 0);
    let rows = vec![string_row(2, 10), array_row(10, 2, &[(1, 2)])];
    assert_eq!(s7_count(10, rows), 0);
    assert_eq!(s7_count(INT, Vec::new()), 0);
}

#[test]
fn adr_0179_struct_with_an_array_of_a_struct_counts_one_and_three_levels() {
    let rows = vec![
        struct_row(101, "ST_Inner", &[("m", INT)]),
        array_row(10, 101, &[(1, 2)]),
        struct_row(102, "ST_Top", &[("a", 10)]),
    ];
    assert_eq!(s7_count(102, rows), 4);
}

#[test]
fn adr_0179_count_is_the_largest_sum_on_one_path() {
    let rows = vec![
        struct_row(101, "ST_Leaf", &[("m", INT)]),
        struct_row(102, "ST_Mid", &[("m", 101)]),
        array_row(10, 101, &[(1, 2)]),
        struct_row(
            103,
            "ST_Top",
            &[("shallow", INT), ("deep", 102), ("wide", 10)],
        ),
    ];
    // The path through `wide` counts 1 + 3. The path through `deep` counts 1 + 1 + 1.
    assert_eq!(s7_count(103, rows), 4);
}

#[test]
fn adr_0179_eight_levels_pass_on_s7_1500() {
    let tree = resolved(101, struct_chain(8)).unwrap();
    assert!(tree.check_s7_nesting(PlcPlatform::S7_1500).is_ok());
}

#[test]
fn adr_0179_nine_levels_fail_with_s7_nesting_on_the_three_s7_platforms() {
    let tree = resolved(101, struct_chain(9)).unwrap();
    for platform in S7_PLATFORMS {
        let result = tree.check_s7_nesting(platform).map(|()| tree.clone());
        assert_eq!(
            too_large(result),
            Some(("ST_1@1".to_owned(), TypeLimit::S7Nesting)),
            "{platform:?}"
        );
    }
}

#[test]
fn adr_0179_nine_levels_of_one_array_of_a_struct_name_the_array_type() {
    let mut rows = struct_chain(7);
    rows.push(array_row(10, 101, &[(1, 2)]));
    let tree = resolved(10, rows).unwrap();
    assert_eq!(tree.s7_nesting, 9);
    let result = tree
        .check_s7_nesting(PlcPlatform::S7_1500)
        .map(|()| tree.clone());
    assert_eq!(
        too_large(result),
        Some(("ARRAY[1..2] OF ST_1@1".to_owned(), TypeLimit::S7Nesting))
    );
}

#[test]
fn adr_0179_nine_levels_pass_on_twincat3_and_codesys() {
    let tree = resolved(101, struct_chain(9)).unwrap();
    assert!(tree.check_s7_nesting(PlcPlatform::TwinCat3).is_ok());
    assert!(tree.check_s7_nesting(PlcPlatform::CodesysIec).is_ok());
}

#[test]
fn adr_0179_resolve_does_not_apply_the_s7_limit() {
    assert!(resolved(101, struct_chain(9)).is_ok());
}

#[test]
fn adr_0179_measure_reports_depth_when_the_s7_count_overflows_u32() {
    // Fault injection: `check_limits` keeps every real child at depth 32 or less.
    let row = struct_row(10, "ST_A", &[("x", 2)]);
    let mut done = finished(2, 1, 0);
    if let Some(metrics) = done.get_mut(&id(2)) {
        metrics.s7_nesting = u32::MAX;
    }
    let error = measure(&row, &done).unwrap_err();
    assert!(matches!(
        error,
        PlantError::TypeTooLarge {
            limit: TypeLimit::Depth,
            ..
        }
    ));
}

#[test]
fn adr_0179_render_of_a_string_leaf_root_shows_its_length() {
    let rows = vec![string_row(2, 10)];
    let tree = resolved(2, rows).unwrap();
    assert_eq!(render_node(tree.root()), "STRING(10)");
}
