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

use super::blockers::{BlockerLine, blocker_lines, blocker_views};
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

fn codes_and_links(lines: &[BlockerLine]) -> Vec<(ErrorCode, Option<String>)> {
    lines
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
    assert_eq!(lines.len(), 1);
    let line = lines.first().unwrap();
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
    let views = blocker_views(&lines, &Bundles::load().unwrap(), Lang::En);
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
    let views = blocker_views(&lines, &Bundles::load().unwrap(), Lang::En);
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
    let views = blocker_views(&lines, &Bundles::load().unwrap(), Lang::En);
    let view = views.first().unwrap();
    let text = format!("{} {}", view.subject, view.message);
    assert!(text.contains("ZONE_A"), "{text}");
    assert!(text.contains("ZONE_B"), "{text}");
}

#[test]
fn approve_preview_blocker_line_of_a_misaligned_address_links_its_parameter() {
    let lines = blocker_lines(AppError::Recipe(misaligned("ZONE_B")), &keys(), BASE).unwrap();
    assert_eq!(codes_and_links(&lines), vec![(ErrorCode::RCP_023, href(2))]);
    let views = blocker_views(&lines, &Bundles::load().unwrap(), Lang::En);
    assert_eq!(views.len(), 1);
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
