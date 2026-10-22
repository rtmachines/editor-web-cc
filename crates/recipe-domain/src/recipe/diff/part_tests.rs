//! Tests of single fields of the parts of a diff (A43, ADR-0050 (c)).

use std::str::FromStr;

use rust_decimal::Decimal;
use serde_json::json;
use uuid::Uuid;

use super::fixtures::{DT, INT, NAME, ROWS, diff_of, translation};
use crate::plant::{LeafPath, S7Address, UnitId};
use crate::recipe::snapshot::fixtures::{
    LIVE_UNIT, length, limited, on_unit, param, parameter_tag, range,
};
use crate::recipe::{
    DefinitionChange, DiffBounds, DiffKind, DiffPart, DiffRows, StoredLimit, StoredParameter,
};
use crate::text::LongText;

/// The parts of one section.
fn parts(rows: &DiffRows, kind: DiffKind) -> Vec<DiffPart> {
    rows.section(kind)
        .iter()
        .map(|row| row.part().clone())
        .collect()
}

/// `SPEED` of type `INT` with the value 5 and the limit rows.
fn speed(limits: Vec<StoredLimit>) -> StoredParameter {
    limited(param(1, "SPEED", INT, json!(5)), limits)
}

/// The definition changes of a diff of `SPEED` with `old` and `new`.
fn definitions(old: StoredParameter, new: StoredParameter) -> Vec<DiffPart> {
    parts(&diff_of(vec![old], vec![new], ROWS), DiffKind::Definitions)
}

fn bounds(min: Option<&str>, max: Option<&str>, max_length: Option<&str>) -> Option<DiffBounds> {
    Some(DiffBounds {
        min: min.map(str::to_owned),
        max: max.map(str::to_owned),
        max_length: max_length.map(str::to_owned),
    })
}

#[test]
fn a_bound_that_changes_from_a_value_to_none_gives_a_limit_row() {
    let open = StoredLimit {
        max: None,
        ..range("", 0, 100)
    };
    let rows = diff_of(
        vec![speed(vec![range("", 0, 100)])],
        vec![speed(vec![open])],
        ROWS,
    );
    assert_eq!(
        parts(&rows, DiffKind::Limits),
        vec![DiffPart::Limit {
            path: LeafPath::parse("").unwrap(),
            old: bounds(Some("0"), Some("100"), None),
            new: bounds(Some("0"), None, None),
        }]
    );
}

#[test]
fn a_max_length_change_of_a_string_leaf_gives_a_limit_row() {
    let name = |max| limited(param(9, "NAME", NAME, json!("abc")), vec![length("", max)]);
    let rows = diff_of(vec![name(8)], vec![name(6)], ROWS);
    assert_eq!(
        parts(&rows, DiffKind::Limits),
        vec![DiffPart::Limit {
            path: LeafPath::parse("").unwrap(),
            old: bounds(None, None, Some("8")),
            new: bounds(None, None, Some("6")),
        }]
    );
}

#[test]
fn a_bound_with_another_scale_gives_no_limit_row() {
    let scaled = StoredLimit {
        max: Some(Decimal::from_str("100.0").unwrap()),
        ..range("", 0, 100)
    };
    let rows = diff_of(
        vec![speed(vec![range("", 0, 100)])],
        vec![speed(vec![scaled])],
        ROWS,
    );
    assert!(rows.is_empty(), "{rows:?}");
    assert_eq!(rows.unchanged(), 1);
}

#[test]
fn a_language_only_in_the_old_version_gives_a_row_with_an_empty_new_side() {
    let mut old = speed(Vec::new());
    old.translations = vec![translation("pl", "Prędkość")];
    let rows = diff_of(vec![old], vec![speed(Vec::new())], ROWS);
    assert_eq!(
        parts(&rows, DiffKind::Translations),
        vec![DiffPart::Translation {
            lang: translation("pl", "Prędkość").lang,
            old: Some(translation("pl", "Prędkość")),
            new: None,
        }]
    );
}

#[test]
fn a_changed_tag_gives_a_tag_row() {
    let mut new = speed(Vec::new());
    new.tag = parameter_tag("VELOCITY");
    let change = DefinitionChange::Tag {
        old: parameter_tag("SPEED"),
        new: parameter_tag("VELOCITY"),
    };
    assert_eq!(
        definitions(speed(Vec::new()), new),
        vec![DiffPart::Definition(change)]
    );
}

#[test]
fn a_changed_unit_gives_a_unit_row() {
    let new = on_unit(speed(Vec::new()), 77);
    let change = DefinitionChange::Unit {
        old: UnitId::from(Uuid::from_u128(LIVE_UNIT)),
        new: UnitId::from(Uuid::from_u128(77)),
    };
    assert_eq!(
        definitions(speed(Vec::new()), new),
        vec![DiffPart::Definition(change)]
    );
}

#[test]
fn a_new_s7_address_gives_an_address_row() {
    let mut old = speed(Vec::new());
    old.s7_address = None;
    let mut new = speed(Vec::new());
    new.s7_address = Some(S7Address::parse("DB1.DBW0").unwrap());
    let change = DefinitionChange::S7Address {
        old: None,
        new: Some(S7Address::parse("DB1.DBW0").unwrap()),
    };
    assert_eq!(definitions(old, new), vec![DiffPart::Definition(change)]);
}

#[test]
fn a_changed_english_description_gives_a_description_row() {
    let mut old = speed(Vec::new());
    old.description_en = LongText::parse("Old text").unwrap();
    let mut new = speed(Vec::new());
    new.description_en = LongText::parse("New text").unwrap();
    let change = DefinitionChange::DescriptionEn {
        old: LongText::parse("Old text").unwrap(),
        new: LongText::parse("New text").unwrap(),
    };
    assert_eq!(definitions(old, new), vec![DiffPart::Definition(change)]);
}

#[test]
fn an_instant_leaf_shows_its_iec_literal() {
    let stamp = |count: i64| param(10, "STAMP", DT, json!(count));
    let rows = diff_of(vec![stamp(0)], vec![stamp(86_400)], ROWS);
    assert_eq!(
        parts(&rows, DiffKind::Values),
        vec![DiffPart::Value {
            path: LeafPath::parse("").unwrap(),
            old: Some("DT#1970-01-01-00:00:00".to_owned()),
            new: Some("DT#1970-01-02-00:00:00".to_owned()),
            limit: None,
        }]
    );
}
