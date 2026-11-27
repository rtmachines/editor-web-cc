//! Tests of the limit rows of a sent form and of the switch of a limit mode (ADR-0039 (k)).
//!
//! A forged form can send any leaf path. The set keeps only the rows that the type tree holds, and
//! a switch copies at most one row per leaf of the tree.

use std::collections::BTreeSet;

use recipe_domain::plant::{
    LeafClass, LeafPath, MachineId, PlatformTypes, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord,
    Primitive, RecordShape, RuleSet, StringEncoding, TimeType, TypeTree,
};
use recipe_domain::recipe::StoredLimit;
use uuid::Uuid;

use super::limits::{LimitSet, LimitTexts};
use super::{GridRow, GridSource, GridView, ValueTexts};
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;
use crate::pages::parameters::tree_kit::{
    Kit, array_tree, primitive_tree, string_tree, zones_tree,
};
use crate::settings::LIMIT_TEXT_MAX;

/// Build one limit row of texts.
fn limit(path: &str, min: &str, max: &str) -> LimitTexts {
    LimitTexts {
        path: LeafPath::parse(path).unwrap(),
        min: min.to_owned(),
        max: max.to_owned(),
        max_length: String::new(),
    }
}

/// Build the English grid of `tree` with the sent limit rows and the pressed level `toggle`.
fn grid(tree: &TypeTree, limits: Vec<LimitTexts>, toggle: Option<&str>) -> GridView {
    let bundles = Bundles::load().unwrap();
    let level = toggle.map(|text| LeafPath::parse(text).unwrap());
    let source = GridSource {
        tree,
        values: ValueTexts::default(),
        limits,
        toggle: level.as_ref(),
        chunk: 100,
        root_label: "SPEED".to_owned(),
    };
    GridView::build(source, Texts::new(&bundles, Lang::En))
}

/// Return the current mode sentence of each level of the grid.
fn currents(view: &GridView) -> Vec<String> {
    view.bodies
        .iter()
        .flat_map(|body| &body.rows)
        .filter_map(GridRow::mode)
        .map(|mode| mode.current.clone())
        .collect()
}

/// Return true when the grid shows limit inputs for the shown path `path`.
fn has_limit_inputs(view: &GridView, path: &str) -> bool {
    view.bodies
        .iter()
        .flat_map(|body| &body.rows)
        .filter_map(GridRow::leaf)
        .any(|row| row.path == path && row.min.is_some())
}

/// Resolve `ARRAY[1..1024] OF ST_S3`. `ST_S3.x` is `ARRAY[1..2] OF ST_S2`, `ST_S2.w` is
/// `ARRAY[1..2] OF ST_S1`, and `ST_S1.v` is `ARRAY[1..2] OF INT`. The tree has 8 192 leaves.
fn deep_tree() -> TypeTree {
    let mut kit = Kit::new();
    let int = kit.primitive(Primitive::Int);
    let v = kit.array(int, &[(1, 2)]);
    let s1 = kit.structure("ST_S1", &[("v", v)]);
    let w = kit.array(s1, &[(1, 2)]);
    let s2 = kit.structure("ST_S2", &[("w", w)]);
    let x = kit.array(s2, &[(1, 2)]);
    let s3 = kit.structure("ST_S3", &[("x", x)]);
    let root = kit.array(s3, &[(1, 1024)]);
    kit.tree(root)
}

/// Return the 27 tails under one element of `deep_tree`: each level is `[*]`, `[1]`, or `[2]`.
fn deep_tails() -> Vec<String> {
    const CHOICES: [&str; 3] = ["[*]", "[1]", "[2]"];
    let mut tails = Vec::new();
    for x in CHOICES {
        for w in CHOICES {
            for v in CHOICES {
                tails.push(format!(".x{x}.w{w}.v{v}"));
            }
        }
    }
    tails
}

#[test]
fn grid_limits_a_sent_row_outside_the_tree_is_dropped() {
    let tree = array_tree(Primitive::Int, &[(1, 1000)]);
    let rows = vec![limit("[*]", "0", "10"), limit("[*].x", "1", "2")];
    let set = LimitSet::new(&tree, rows, None);
    assert!(set.get("[*]").is_some());
    assert!(set.get("[*].x").is_none());
}

#[test]
fn grid_limits_a_split_copies_only_the_rows_of_the_tree() {
    let tree = array_tree(Primitive::Int, &[(1, 1000)]);
    let rows = vec![
        limit("[*]", "0", "10"),
        limit("[*].x", "1", "2"),
        limit("[*][1]", "1", "2"),
    ];
    let level = LeafPath::parse("[*]").unwrap();
    let set = LimitSet::new(&tree, rows, Some(&level));
    assert_eq!(set.get("[5]").map(|row| row.max.as_str()), Some("10"));
    assert!(set.get("[5].x").is_none());
    assert!(set.get("[5][1]").is_none());
}

#[test]
fn grid_limits_a_forged_index_row_outside_the_tree_keeps_the_mode() {
    let rows = vec![
        limit("[*].setpoint", "0", "10"),
        limit("[3].nothing", "1", "2"),
    ];
    let view = grid(&zones_tree(), rows, None);
    assert_eq!(currents(&view), ["[*]: one limit for all elements"]);
}

#[test]
fn grid_limits_a_split_makes_at_most_one_row_per_leaf() {
    let tree = deep_tree();
    let rows = deep_tails()
        .iter()
        .map(|tail| limit(&format!("[*]{tail}"), "0", "1"))
        .collect();
    let level = LeafPath::parse("[*]").unwrap();
    let set = LimitSet::new(&tree, rows, Some(&level));
    let tails = deep_tails();
    let kept = (1..=1024)
        .flat_map(|index| tails.iter().map(move |tail| format!("[{index}]{tail}")))
        .filter(|path| set.get(path).is_some())
        .count();
    assert!(kept > 0);
    assert!(kept <= 8192, "{kept}");
}

#[test]
fn grid_limits_a_join_after_an_empty_split_gives_one_limit_for_all() {
    let rows = (1..=4)
        .flat_map(|index| {
            [
                limit(&format!("[{index}].setpoint"), "", ""),
                limit(&format!("[{index}].hold"), " ", ""),
            ]
        })
        .collect();
    let view = grid(&zones_tree(), rows, Some("[*]"));
    assert_eq!(currents(&view), ["[*]: one limit for all elements"]);
    assert!(has_limit_inputs(&view, "[*].setpoint"));
    assert!(!has_limit_inputs(&view, "[2].setpoint"));
}

#[test]
fn grid_limits_empty_element_rows_keep_a_limit_per_element() {
    let rows = (1..=4)
        .map(|index| limit(&format!("[{index}].setpoint"), "", ""))
        .collect();
    let view = grid(&zones_tree(), rows, None);
    assert_eq!(currents(&view), ["[*]: a limit per element"]);
    assert!(has_limit_inputs(&view, "[2].setpoint"));
}

/// Both ends of each integer type, as the two texts that the page shows (`data-model.md` section 7).
const INTEGER_ENDS: [(Primitive, &str, &str); 12] = [
    (Primitive::Sint, "-128", "127"),
    (Primitive::Usint, "0", "255"),
    (Primitive::Byte, "0", "255"),
    (Primitive::Int, "-32768", "32767"),
    (Primitive::Uint, "0", "65535"),
    (Primitive::Word, "0", "65535"),
    (Primitive::Dint, "-2147483648", "2147483647"),
    (Primitive::Udint, "0", "4294967295"),
    (Primitive::Dword, "0", "4294967295"),
    (
        Primitive::Lint,
        "-9223372036854775808",
        "9223372036854775807",
    ),
    (Primitive::Ulint, "0", "18446744073709551615"),
    (Primitive::Lword, "0", "18446744073709551615"),
];

/// A `REAL` or `LREAL` bound as the page stores it: the digit limits of ADR-0117 (a) with a scale
/// of 28, a bound of 28 significant digits for each type, and the ends of `Decimal`. Each text
/// has at most 28 fractional digits, so `Decimal` keeps it exactly.
const REAL_BOUNDS: [(Primitive, &str, &str); 5] = [
    (
        Primitive::Real,
        "-0.0000000000000000000001234567",
        "0.0000000000000000000007654321",
    ),
    (
        Primitive::Lreal,
        "-0.0000000000000123456789012345",
        "0.0000000000000987654321098765",
    ),
    (
        Primitive::Real,
        "-0.1234567890123456789012345678",
        "0.9999999999999999999999999999",
    ),
    (
        Primitive::Lreal,
        "-7922816251426433759354395033",
        "7922816251426433759354395033",
    ),
    (
        Primitive::Lreal,
        "-79228162514264337593543950335",
        "79228162514264337593543950335",
    ),
];

/// Build a stored row on the whole value with the bounds `min` and `max`.
fn stored_range(min: &str, max: &str) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: Some(min.parse().unwrap()),
        max: Some(max.parse().unwrap()),
        max_length: None,
    }
}

/// Resolve the tree of one primitive on `platform`. The kit of the tests has only `TwinCat3`.
fn primitive_tree_on(primitive: Primitive, platform: PlcPlatform) -> TypeTree {
    let id = PlcTypeId::from(Uuid::from_u128(1));
    let row = PlcType::try_from(PlcTypeRecord {
        id,
        machine: None,
        platforms: BTreeSet::from(PlcPlatform::ALL),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: primitive.fixed_size_bytes().unwrap(),
            string_length: None,
        },
    })
    .unwrap();
    let types = PlatformTypes::collect(
        MachineId::from(Uuid::from_u128(2)),
        platform,
        StringEncoding::Latin1,
        [row],
    )
    .unwrap();
    TypeTree::resolve(id, &types).unwrap()
}

/// Return the three texts that the page shows for the stored row `limit` on `tree`.
fn shown_texts(tree: &TypeTree, limit: StoredLimit) -> [String; 3] {
    let rows = LimitTexts::from_stored(tree, &[limit]);
    assert_eq!(rows.len(), 1);
    let row = rows.first().unwrap();
    [row.min.clone(), row.max.clone(), row.max_length.clone()]
}

/// The nanoseconds of one day. An `LDATE` count is a whole number of days.
const NANOS_PER_DAY: i128 = 86_400_000_000_000;

/// Return the last count of `primitive` up to `high` that the page shows as a literal.
///
/// Only `LDATE` has gaps: its literal needs a whole day, but the raw range maximum of each rule
/// set falls inside a day. Its last literal is the last midnight below the maximum.
fn last_stored_count(primitive: Primitive, high: i128) -> i128 {
    if primitive == Primitive::Ldate {
        high.checked_div(NANOS_PER_DAY)
            .and_then(|days| days.checked_mul(NANOS_PER_DAY))
            .unwrap()
    } else {
        high
    }
}

/// Pair each of the three shown texts with its label.
fn labelled(label: &str, texts: [String; 3]) -> [(String, String); 3] {
    texts.map(|text| (label.to_owned(), text))
}

/// Return the shown texts of the integer, real, and string bounds that the page stores.
fn stored_bound_texts() -> Vec<(String, String)> {
    let mut shown = Vec::new();
    for (primitive, min, max) in INTEGER_ENDS {
        let tree = primitive_tree(primitive);
        let texts = shown_texts(&tree, stored_range(min, max));
        shown.extend(labelled(&format!("{primitive:?}"), texts));
    }
    for (primitive, min, max) in REAL_BOUNDS {
        let tree = primitive_tree(primitive);
        let texts = shown_texts(&tree, stored_range(min, max));
        // `Decimal` keeps the stored bound exactly, so the page shows the sent text unchanged.
        assert_eq!(texts, [min, max, ""], "{primitive:?} {min}");
        shown.extend(labelled(&format!("{primitive:?} {min}"), texts));
    }
    let string = string_tree(Primitive::String, 20);
    let length = StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: None,
        max_length: Some(u16::MAX),
    };
    shown.extend(labelled("max_length", shown_texts(&string, length)));
    shown
}

/// Return the shown texts of the stored time ranges and the count of rows that it checked. Each
/// text must be a literal and not the fallback decimal count.
fn stored_time_texts() -> (usize, Vec<(String, String)>) {
    let mut shown = Vec::new();
    let mut time_rows = 0_usize;
    for primitive in Primitive::ALL {
        if !matches!(
            primitive.leaf_class(),
            LeafClass::Duration | LeafClass::Instant
        ) {
            continue;
        }
        for platform in PlcPlatform::ALL {
            let Some(ty) = TimeType::of(primitive, platform) else {
                continue;
            };
            let tree = primitive_tree_on(primitive, platform);
            for rules in [RuleSet::Current, RuleSet::Earlier] {
                let Some((low, high)) = ty.range_under(rules) else {
                    continue;
                };
                time_rows = time_rows.saturating_add(1);
                let high = last_stored_count(primitive, high);
                let label = format!("{primitive:?} {platform:?} {rules:?}");
                let texts = shown_texts(&tree, stored_range(&low.to_string(), &high.to_string()));
                // A text that equals its decimal count is the fallback and not a literal.
                for (text, count) in texts.iter().zip([low, high]) {
                    assert!(text.contains('#'), "{label}: {text}");
                    assert_ne!(*text, count.to_string(), "{label}");
                }
                shown.extend(labelled(&label, texts));
            }
        }
    }
    (time_rows, shown)
}

#[test]
fn stored_bound_text_fits_the_cap() {
    let mut shown = stored_bound_texts();
    let (time_rows, time_shown) = stored_time_texts();
    shown.extend(time_shown);
    // 34 pairs of a time type and a platform, each under two rule sets.
    assert_eq!(time_rows, 68);

    for (label, text) in &shown {
        assert!(text.len() <= LIMIT_TEXT_MAX, "{label}: {text}");
    }
    let (_label, longest) = shown
        .iter()
        .max_by_key(|(_label, text)| text.len())
        .unwrap();
    // The longest text is the canonical `LTIME` literal of 35 bytes (`design.md`, C1).
    assert_eq!(longest.len(), 35, "{longest}");
}
