//! Tests of the blocker lines of the approval preview (task 10.1, ADR-0049 (c), design D3).

use std::collections::BTreeMap;

use recipe_app::error::{AppError, StorageError};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{LeafPath, MachineCode, PlantError, S7Address, UnitId};
use recipe_domain::recipe::{
    ParameterKey, ParameterTag, RecipeError, RecipeErrors, ReferenceNumber, SnapshotError,
    SnapshotItem,
};
use uuid::Uuid;

use super::blockers::{BlockerLines, blocker_lines};
use crate::error::WebError;
use crate::i18n::{Bundles, Lang};

/// The address of the version of the tests.
const BASE: &str = "/recipes/R-500/versions/2";

fn tag(text: &str) -> ParameterTag {
    ParameterTag::parse(text).unwrap()
}

fn key(n: u128) -> ParameterKey {
    ParameterKey::from(Uuid::from_u128(n))
}

/// The keys of the tags `ZONE_A` (key 1) and `ZONE_B` (key 2).
fn keys() -> BTreeMap<ParameterTag, ParameterKey> {
    BTreeMap::from([(tag("ZONE_A"), key(1)), (tag("ZONE_B"), key(2))])
}

fn href(n: u128) -> Option<String> {
    Some(format!("{BASE}/parameters/{}", key(n)))
}

fn misaligned(text: &str) -> RecipeError {
    RecipeError::S7AddressMisaligned {
        tag: tag(text),
        address: S7Address::parse("DB1.DBW1").unwrap(),
    }
}

fn codes_and_links(lines: &BlockerLines) -> Vec<(ErrorCode, Option<String>)> {
    lines
        .shown
        .iter()
        .map(|line| (line.error.code(), line.href.clone()))
        .collect()
}

#[test]
fn approve_preview_blocker_line_of_a_draft_without_parameters_has_the_code_and_no_link() {
    let reference = ReferenceNumber::parse("R-500").unwrap();
    let error = AppError::Recipe(RecipeError::DraftHasNoParameters {
        reference,
        number: 2,
    });
    let lines = blocker_lines(error, &BTreeMap::new(), "/recipes/R-500/versions/2").unwrap();
    assert_eq!(lines.total, 1);
    let line = lines.shown.first().unwrap();
    assert_eq!(line.error.code(), ErrorCode::RCP_008);
    assert_eq!(line.href, None);
}

#[test]
fn approve_preview_blocker_lines_of_a_failed_snapshot_keep_each_item_in_order() {
    let archived = PlantError::MachineArchived {
        code: MachineCode::parse("M7").unwrap(),
    };
    let items = vec![
        SnapshotItem::Parameter {
            tag: tag("ZONE_A"),
            error: SnapshotError::Recipe(misaligned("ZONE_A")),
        },
        SnapshotItem::UnknownUnit {
            tag: tag("ZONE_B"),
            unit: UnitId::from(Uuid::nil()),
        },
        SnapshotItem::Version {
            error: SnapshotError::Plant(archived),
        },
    ];
    let error = AppError::Recipe(RecipeError::ValidationFailed { items });
    let lines = blocker_lines(error, &keys(), BASE).unwrap();
    assert_eq!(
        codes_and_links(&lines),
        vec![
            (ErrorCode::RCP_023, href(1)),
            (ErrorCode::PLT_020, href(2)),
            (ErrorCode::PLT_010, None),
        ]
    );
    let views = lines.into_list(&Bundles::load().unwrap(), Lang::En).views;
    let shown: Vec<(&str, &str)> = views
        .iter()
        .map(|view| (view.code, view.href.as_str()))
        .collect();
    let first = href(1).unwrap();
    let second = href(2).unwrap();
    assert_eq!(
        shown,
        vec![
            ("E-RCP-023", first.as_str()),
            ("E-PLT-020", second.as_str()),
            ("E-PLT-010", ""),
        ]
    );
    assert!(views.iter().all(|view| !view.message.is_empty()));
}

#[test]
fn approve_preview_blocker_lines_of_a_list_give_one_line_for_each_entry() {
    let missing = |text: &str, leaf: &str| RecipeError::LimitMissing {
        tag: tag(text),
        leaf: LeafPath::parse(leaf).unwrap(),
    };
    let errors =
        RecipeErrors::from_vec(vec![missing("ZONE_A", ""), missing("ZONE_B", "[1]")]).unwrap();
    let lines = blocker_lines(AppError::Recipes(errors), &keys(), BASE).unwrap();
    assert_eq!(
        codes_and_links(&lines),
        vec![(ErrorCode::RCP_016, href(1)), (ErrorCode::RCP_016, href(2))]
    );
    let views = lines.into_list(&Bundles::load().unwrap(), Lang::En).views;
    assert_eq!(views.len(), 2);
}

#[test]
fn approve_preview_blocker_line_of_an_overlap_names_both_tags_and_links_the_first() {
    let overlap = RecipeError::S7AddressOverlap {
        tag: tag("ZONE_A"),
        other_tag: tag("ZONE_B"),
        address: S7Address::parse("DB1.DBW2").unwrap(),
    };
    let lines = blocker_lines(AppError::Recipe(overlap), &keys(), BASE).unwrap();
    assert_eq!(codes_and_links(&lines), vec![(ErrorCode::RCP_022, href(1))]);
    let views = lines.into_list(&Bundles::load().unwrap(), Lang::En).views;
    let view = views.first().unwrap();
    let text = format!("{} {}", view.subject, view.message);
    assert!(text.contains("ZONE_A"), "{text}");
    assert!(text.contains("ZONE_B"), "{text}");
}

#[test]
fn approve_preview_blocker_line_of_a_misaligned_address_links_its_parameter() {
    let lines = blocker_lines(AppError::Recipe(misaligned("ZONE_B")), &keys(), BASE).unwrap();
    assert_eq!(codes_and_links(&lines), vec![(ErrorCode::RCP_023, href(2))]);
    let views = lines.into_list(&Bundles::load().unwrap(), Lang::En).views;
    assert_eq!(views.len(), 1);
}

#[test]
fn approve_preview_blocker_of_the_plant_model_is_a_line_with_its_code_and_no_link() {
    let archived = PlantError::MachineArchived {
        code: MachineCode::parse("M7").unwrap(),
    };
    let lines = blocker_lines(AppError::Plant(archived), &keys(), BASE).unwrap();
    assert_eq!(codes_and_links(&lines), vec![(ErrorCode::PLT_010, None)]);
    let views = lines.into_list(&Bundles::load().unwrap(), Lang::En).views;
    let view = views.first().unwrap();
    assert_eq!(view.code, "E-PLT-010");
    assert_eq!(view.href, "");
    assert!(!view.message.is_empty());
}

/// Return a failed snapshot check with `count` items.
fn long_snapshot(count: u128) -> AppError {
    let item = |n: u128| SnapshotItem::UnknownUnit {
        tag: tag("ZONE_A"),
        unit: UnitId::from(Uuid::from_u128(n)),
    };
    AppError::Recipe(RecipeError::ValidationFailed {
        items: (1..=count).map(item).collect(),
    })
}

/// Return a list of `count` missing limits. The tags `ZONE_A` and `ZONE_B` alternate.
fn long_list(count: usize) -> AppError {
    let tags = ["ZONE_A", "ZONE_B"].into_iter().cycle();
    let errors = tags
        .take(count)
        .map(|text| RecipeError::LimitMissing {
            tag: tag(text),
            leaf: LeafPath::parse("").unwrap(),
        })
        .collect();
    AppError::Recipes(RecipeErrors::from_vec(errors).unwrap())
}

#[test]
fn approve_preview_blocker_lines_of_a_long_list_build_only_the_first_20_and_count_all() {
    let lines = blocker_lines(long_list(25), &keys(), BASE).unwrap();
    assert_eq!(lines.shown.len(), 20);
    assert_eq!(lines.total, 25);
    let expected: Vec<Option<String>> = [1, 2].into_iter().cycle().take(20).map(href).collect();
    let links: Vec<Option<String>> = lines.shown.iter().map(|line| line.href.clone()).collect();
    assert_eq!(links, expected);
}

#[test]
fn approve_preview_blocker_lines_of_a_long_snapshot_build_only_the_first_20_and_count_all() {
    let lines = blocker_lines(long_snapshot(24), &keys(), BASE).unwrap();
    assert_eq!(lines.shown.len(), 20);
    assert_eq!(lines.total, 24);
    let list = lines.into_list(&Bundles::load().unwrap(), Lang::En);
    assert_eq!(list.views.len(), 20);
    assert!(list.views.iter().all(|view| view.code == "E-PLT-020"));
    assert_eq!(
        list.more,
        "Other errors: 4. Each one stands under its field."
    );
}

#[test]
fn approve_preview_blocker_list_of_25_entries_gives_20_views_and_the_sentence_for_five() {
    let bundles = Bundles::load().unwrap();
    let lines = blocker_lines(long_list(25), &keys(), BASE).unwrap();
    let list = lines.into_list(&bundles, Lang::En);
    assert_eq!(list.views.len(), 20);
    assert_eq!(
        list.more,
        "Other errors: 5. Each one stands under its field."
    );
}

#[test]
fn approve_preview_blocker_list_counts_the_lines_beyond_the_shown_ones() {
    let bundles = Bundles::load().unwrap();
    let counted = |total: usize, lang: Lang| {
        let lines = BlockerLines {
            shown: Vec::new(),
            total,
        };
        lines.into_list(&bundles, lang).more
    };
    assert_eq!(
        counted(21, Lang::Pl),
        "Pozostałe błędy: 1. Każdy stoi pod swoim polem."
    );
    for total in [0, 1, 20] {
        assert_eq!(counted(total, Lang::En), "", "{total}");
    }
}

#[test]
fn approve_preview_blocker_of_the_storage_is_an_error_of_the_page() {
    let error = AppError::Storage(StorageError::Constraint {
        code: ErrorCode::RCP_030,
        message: "The rule of the database stopped the change.".to_owned(),
    });
    let result = blocker_lines(error, &keys(), BASE);
    assert!(matches!(
        result,
        Err(WebError::App(AppError::Storage(
            StorageError::Constraint { .. }
        )))
    ));
}
