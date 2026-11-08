//! Tests of the kept members of a rebind: a member keeps its value when the type shape is equal.
//!
//! Two rows can have one shape. A `STRING(80)` row of the machine and the seed row `STRING` of
//! `CodesysIec` are an example (ADR-0146 (a) to (f)). The seed rows of the tests follow
//! `migrations/0004_seed_plc_types.sql`.

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::rebind_new_leaves;
use super::test_support::{draft_on, key, param, planned_json, request};
use crate::error_codes::ErrorCode;
use crate::plant::fixtures::{
    array_row, id, machine, on_platform, primitive_row, struct_row_revision, types_for,
};
use crate::plant::{
    LeafPath, MachineId, PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape, TypeTrees,
};
use crate::recipe::{Draft, DraftError, RecipeError, StoredLimit};

const OLD_ROW: u128 = 1;
const NEW_ROW: u128 = 2;
const OLD_ARRAY: u128 = 3;
const NEW_ARRAY: u128 = 4;
const INT: u128 = 5;
const TEXT_1: u128 = 100;
const TEXT_2: u128 = 101;
const ZONE_2: u128 = 110;
const ZONE_3: u128 = 111;
const PLANT_1: u128 = 112;
const PLANT_2: u128 = 113;

/// The id of the seed row `STRING` on `CodesysIec` and `TwinCat3`.
const SEED_STRING_80: u128 = 0x5eed_0000_0000_4000_8000_0000_0000_001e;
/// The id of the seed row `WSTRING` on `CodesysIec` and `TwinCat3`.
const SEED_WSTRING_80: u128 = 0x5eed_0000_0000_4000_8000_0000_0000_001f;
/// The id of the seed row `STRING` on the S7 platforms.
const SEED_STRING_254: u128 = 0x5eed_0000_0000_4000_8000_0000_0000_0010;

const S7_STRING_PLATFORMS: [PlcPlatform; 3] = [
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
    PlcPlatform::S7_300,
];
const IEC_PLATFORMS: [PlcPlatform; 2] = [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3];

/// The length and the size in bytes of a string row.
#[derive(Clone, Copy)]
struct Sized {
    length: u16,
    size_bytes: u32,
}

const STRING_80: Sized = Sized {
    length: 80,
    size_bytes: 82,
};
const STRING_81: Sized = Sized {
    length: 81,
    size_bytes: 83,
};
const STRING_254: Sized = Sized {
    length: 254,
    size_bytes: 256,
};
const WSTRING_80: Sized = Sized {
    length: 80,
    size_bytes: 164,
};

fn text_row(
    (n, owner): (u128, Option<MachineId>),
    platforms: &[PlcPlatform],
    primitive: Primitive,
    sized: Sized,
) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: id(n),
        machine: owner,
        platforms: BTreeSet::from_iter(platforms.iter().copied()),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: sized.size_bytes,
            string_length: Some(sized.length),
        },
    })
    .unwrap()
}

/// A string row of the machine on one platform.
fn machine_text(n: u128, platform: PlcPlatform, primitive: Primitive, sized: Sized) -> PlcType {
    text_row((n, Some(machine())), &[platform], primitive, sized)
}

/// A seed string row.
fn seed_text(n: u128, platforms: &[PlcPlatform], primitive: Primitive, sized: Sized) -> PlcType {
    text_row((n, None), platforms, primitive, sized)
}

/// The array row `ARRAY[1..2]` of the machine on `platform`.
fn array_on(platform: PlcPlatform, (n, element): (u128, u128)) -> PlcType {
    on_platform(platform, vec![array_row(n, element, &[(1, 2)])]).remove(0)
}

/// The trees of `ST_Text@1` and `ST_Text@2`. Both have one member `member`.
///
/// `rows` hold the member types. The member of revision 1 has the type `old`. The member of
/// revision 2 has the type `new`.
fn text_trees(
    platform: PlcPlatform,
    member: &str,
    mut rows: Vec<PlcType>,
    (old, new): (u128, u128),
) -> TypeTrees {
    rows.extend(on_platform(
        platform,
        vec![
            struct_row_revision(TEXT_1, "ST_Text", 1, &[(member, old)]),
            struct_row_revision(TEXT_2, "ST_Text", 2, &[(member, new)]),
        ],
    ));
    TypeTrees::resolve(
        [TEXT_1, TEXT_2].map(id),
        &types_for(machine(), platform, rows),
    )
}

fn max_length(path: &str, length: u16) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: None,
        max: None,
        max_length: Some(length),
    }
}

/// A draft on `platform` with `LABEL_1` of type `ST_Text@1`, a value, and one limit.
fn label_draft(platform: PlcPlatform, value: Value, limit: StoredLimit) -> Draft {
    draft_on(
        platform,
        vec![param(1, "LABEL_1", TEXT_1, value, vec![limit])],
    )
}

/// The new leaves of `LABEL_1` as paths.
fn new_leaf_paths(trees: &TypeTrees) -> Vec<String> {
    let tree = trees.tree(id(TEXT_1)).unwrap();
    let target = trees.tree(id(TEXT_2)).unwrap();
    rebind_new_leaves(tree, id(TEXT_1), target)
        .unwrap()
        .map(|found| {
            found
                .leaves()
                .iter()
                .map(|l| l.path().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Assert that the rebind of `LABEL_1` without new values keeps its value and its limit.
fn assert_rebind_keeps(
    platform: PlcPlatform,
    member: &str,
    rows: Vec<PlcType>,
    rows_pair: (u128, u128),
) {
    let trees = text_trees(platform, member, rows, rows_pair);
    let value = if member == "lines" {
        json!({"lines": ["A", "B"]})
    } else {
        json!({"label": "ABC"})
    };
    let path = if member == "lines" {
        ".lines[*]"
    } else {
        ".label"
    };
    let limit = max_length(path, 40);
    let draft = label_draft(platform, value.clone(), limit.clone());

    let plan = draft.plan_rebind(&request(TEXT_1, TEXT_2), &trees).unwrap();

    assert_eq!(planned_json(&plan, 1), value);
    assert_eq!(plan.limits(key(1)), Some(&[limit][..]));
    assert_eq!(new_leaf_paths(&trees), Vec::<String>::new());
}

/// The code and the leaf of each entry of a rejected plan, in the order of the list.
fn entries(result: Result<crate::recipe::RebindPlan, DraftError>) -> Vec<(ErrorCode, String)> {
    let Some(DraftError::List(list)) = result.err() else {
        return Vec::new();
    };
    list.into_vec()
        .into_iter()
        .map(|entry| {
            let leaf = match &entry {
                RecipeError::RebindNeedsValue { leaf, .. }
                | RecipeError::RebindDropsLimit { leaf, .. } => leaf.to_string(),
                other => format!("unexpected {other}"),
            };
            (entry.code(), leaf)
        })
        .collect()
}

#[test]
fn string_80_to_the_seed_string_keeps_the_value_and_the_limit_on_codesys() {
    let rows = vec![
        machine_text(
            OLD_ROW,
            PlcPlatform::CodesysIec,
            Primitive::String,
            STRING_80,
        ),
        seed_text(SEED_STRING_80, &IEC_PLATFORMS, Primitive::String, STRING_80),
    ];

    assert_rebind_keeps(
        PlcPlatform::CodesysIec,
        "label",
        rows,
        (OLD_ROW, SEED_STRING_80),
    );
}

#[test]
fn the_seed_string_to_string_80_keeps_the_value_and_the_limit_on_codesys() {
    let rows = vec![
        seed_text(SEED_STRING_80, &IEC_PLATFORMS, Primitive::String, STRING_80),
        machine_text(
            NEW_ROW,
            PlcPlatform::CodesysIec,
            Primitive::String,
            STRING_80,
        ),
    ];

    assert_rebind_keeps(
        PlcPlatform::CodesysIec,
        "label",
        rows,
        (SEED_STRING_80, NEW_ROW),
    );
}

#[test]
fn string_254_to_the_seed_string_keeps_the_value_and_the_limit_on_s7_1500() {
    let rows = vec![
        machine_text(OLD_ROW, PlcPlatform::S7_1500, Primitive::String, STRING_254),
        seed_text(
            SEED_STRING_254,
            &S7_STRING_PLATFORMS,
            Primitive::String,
            STRING_254,
        ),
    ];

    assert_rebind_keeps(
        PlcPlatform::S7_1500,
        "label",
        rows,
        (OLD_ROW, SEED_STRING_254),
    );
}

#[test]
fn wstring_80_to_the_seed_wstring_keeps_the_value_and_the_limit_on_twincat3() {
    let rows = vec![
        machine_text(
            OLD_ROW,
            PlcPlatform::TwinCat3,
            Primitive::Wstring,
            WSTRING_80,
        ),
        seed_text(
            SEED_WSTRING_80,
            &IEC_PLATFORMS,
            Primitive::Wstring,
            WSTRING_80,
        ),
    ];

    assert_rebind_keeps(
        PlcPlatform::TwinCat3,
        "label",
        rows,
        (OLD_ROW, SEED_WSTRING_80),
    );
}

#[test]
fn an_array_of_string_80_to_an_array_of_the_seed_string_keeps_the_value_on_codesys() {
    let platform = PlcPlatform::CodesysIec;
    let rows = vec![
        machine_text(OLD_ROW, platform, Primitive::String, STRING_80),
        seed_text(SEED_STRING_80, &IEC_PLATFORMS, Primitive::String, STRING_80),
        array_on(platform, (OLD_ARRAY, OLD_ROW)),
        array_on(platform, (NEW_ARRAY, SEED_STRING_80)),
    ];

    assert_rebind_keeps(platform, "lines", rows, (OLD_ARRAY, NEW_ARRAY));
}

#[test]
fn string_80_to_string_81_asks_for_a_value_and_drops_the_limit() {
    let platform = PlcPlatform::CodesysIec;
    let rows = vec![
        machine_text(OLD_ROW, platform, Primitive::String, STRING_80),
        machine_text(NEW_ROW, platform, Primitive::String, STRING_81),
    ];
    let trees = text_trees(platform, "label", rows, (OLD_ROW, NEW_ROW));
    let draft = label_draft(platform, json!({"label": "ABC"}), max_length(".label", 40));

    let result = draft.plan_rebind(&request(TEXT_1, TEXT_2), &trees);

    assert_eq!(
        entries(result),
        vec![
            (ErrorCode::RCP_028, ".label".to_owned()),
            (ErrorCode::RCP_029, ".label".to_owned()),
        ]
    );
    assert_eq!(new_leaf_paths(&trees), vec![".label".to_owned()]);
}

#[test]
fn a_member_of_a_newer_struct_revision_is_new_for_each_leaf() {
    let platform = PlcPlatform::CodesysIec;
    let rows = on_platform(
        platform,
        vec![
            primitive_row(INT, Primitive::Int),
            struct_row_revision(
                ZONE_2,
                "ST_Zone",
                2,
                &[("setpoint", INT), ("temp_max", INT)],
            ),
            struct_row_revision(
                ZONE_3,
                "ST_Zone",
                3,
                &[("setpoint", INT), ("temp_max", INT)],
            ),
            struct_row_revision(PLANT_1, "ST_Plant", 1, &[("zone", ZONE_2)]),
            struct_row_revision(PLANT_2, "ST_Plant", 2, &[("zone", ZONE_3)]),
        ],
    );
    let trees = TypeTrees::resolve(
        [PLANT_1, PLANT_2].map(id),
        &types_for(machine(), platform, rows),
    );
    let value = json!({"zone": {"setpoint": 1, "temp_max": 2}});
    let draft = draft_on(
        platform,
        vec![param(1, "PLANT_1", PLANT_1, value, Vec::new())],
    );

    let result = draft.plan_rebind(&request(PLANT_1, PLANT_2), &trees);

    assert_eq!(
        entries(result),
        vec![
            (ErrorCode::RCP_028, ".zone.setpoint".to_owned()),
            (ErrorCode::RCP_028, ".zone.temp_max".to_owned()),
        ]
    );
}
