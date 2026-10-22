//! Tests of the diff of the parameters, the definitions, and the translations (ADR-0050).

use serde_json::json;
use uuid::Uuid;

use super::fixtures::{INT, ROWS, diff_of, translation};
use crate::plant::fixtures::id;
use crate::plant::{LeafPath, UnitId};
use crate::recipe::snapshot::fixtures::{LIVE_UNIT, limited, param, range};
use crate::recipe::{
    DefinitionChange, DiffBounds, DiffKind, DiffPair, DiffPart, DiffRows, Ordinal, Parameter,
    ParameterKey, StoredParameter,
};
use crate::text::Text;

/// `INT` with the value 5 and the limit `0` to `100`.
fn speed(n: u128, tag: &str) -> StoredParameter {
    limited(param(n, tag, INT, json!(5)), vec![range("", 0, 100)])
}

/// The tag text and the part of each row of one section.
fn section(rows: &DiffRows, kind: DiffKind) -> Vec<(String, DiffPart)> {
    rows.section(kind)
        .iter()
        .map(|row| (row.tag().to_string(), row.part().clone()))
        .collect()
}

/// The bounds `min` 0 and `max` 100 of [`speed`].
fn speed_bounds() -> DiffBounds {
    DiffBounds {
        min: Some("0".to_owned()),
        max: Some("100".to_owned()),
        max_length: None,
    }
}

/// The row of an added or removed `INT` parameter with the value 5.
fn speed_row() -> DiffPart {
    DiffPart::Parameter {
        name_en: Text::parse("Name").unwrap(),
        unit: UnitId::from(Uuid::from_u128(LIVE_UNIT)),
        plc_type: id(INT),
        scalar: Some("5".to_owned()),
        leaves: 1,
    }
}

/// The kinds of the sections that hold a change of a parameter on both sides.
const CHANGE_KINDS: [DiffKind; 4] = [
    DiffKind::Limits,
    DiffKind::Values,
    DiffKind::Definitions,
    DiffKind::Translations,
];

#[test]
fn parameter_added_in_the_new_version() {
    let rows = diff_of(
        vec![speed(1, "SPEED")],
        vec![speed(1, "SPEED"), speed(2, "HOLD")],
        ROWS,
    );
    assert_eq!(
        section(&rows, DiffKind::Added),
        vec![("HOLD".to_owned(), speed_row())]
    );
    let limit = DiffPart::Limit {
        path: LeafPath::parse("").unwrap(),
        old: None,
        new: Some(speed_bounds()),
    };
    assert_eq!(
        section(&rows, DiffKind::Limits),
        vec![("HOLD".to_owned(), limit)]
    );
    for kind in [DiffKind::Values, DiffKind::Definitions, DiffKind::Removed] {
        assert!(rows.section(kind).is_empty(), "{kind:?}");
    }
    assert_eq!(rows.unchanged(), 1);
}

#[test]
fn a_first_approval_gives_every_parameter_as_added() {
    let rows = diff_of(Vec::new(), vec![speed(2, "HOLD"), speed(1, "SPEED")], ROWS);
    let tags = |kind| -> Vec<String> {
        section(&rows, kind)
            .into_iter()
            .map(|(tag, _)| tag)
            .collect()
    };
    assert_eq!(tags(DiffKind::Added), ["HOLD", "SPEED"]);
    assert_eq!(tags(DiffKind::Limits), ["HOLD", "SPEED"]);
    assert_eq!(rows.unchanged(), 0);
}

#[test]
fn parameter_only_in_the_old_version() {
    let rows = diff_of(
        vec![speed(1, "SPEED"), speed(2, "HOLD")],
        vec![speed(1, "SPEED")],
        ROWS,
    );
    assert_eq!(
        section(&rows, DiffKind::Removed),
        vec![("HOLD".to_owned(), speed_row())]
    );
    let limit = DiffPart::Limit {
        path: LeafPath::parse("").unwrap(),
        old: Some(speed_bounds()),
        new: None,
    };
    assert_eq!(
        section(&rows, DiffKind::Limits),
        vec![("HOLD".to_owned(), limit)]
    );
    assert!(rows.section(DiffKind::Added).is_empty());
}

#[test]
fn a_changed_definition_field_gives_one_row_for_each_field() {
    let mut changed = speed(1, "SPEED");
    changed.name_en = Text::parse("Speed").unwrap();
    changed.ordinal = Ordinal::parse(4).unwrap();
    let rows = diff_of(vec![speed(1, "SPEED")], vec![changed], ROWS);
    let ordinal = DefinitionChange::Ordinal {
        old: Ordinal::parse(1).unwrap(),
        new: Ordinal::parse(4).unwrap(),
    };
    let name = DefinitionChange::NameEn {
        old: Text::parse("Name").unwrap(),
        new: Text::parse("Speed").unwrap(),
    };
    assert_eq!(
        section(&rows, DiffKind::Definitions),
        vec![
            ("SPEED".to_owned(), DiffPart::Definition(ordinal)),
            ("SPEED".to_owned(), DiffPart::Definition(name)),
        ]
    );
    for kind in [DiffKind::Limits, DiffKind::Values, DiffKind::Translations] {
        assert!(rows.section(kind).is_empty(), "{kind:?}");
    }
    assert_eq!(rows.unchanged(), 0);
}

#[test]
fn a_changed_translation_gives_a_row_for_each_language() {
    let mut old = speed(1, "SPEED");
    old.translations = vec![translation("pl", "Prędkość")];
    let mut new = speed(1, "SPEED");
    new.translations = vec![translation("pl", "Szybkość"), translation("de", "Tempo")];
    let rows = diff_of(vec![old], vec![new], ROWS);
    let de = DiffPart::Translation {
        lang: translation("de", "Tempo").lang,
        old: None,
        new: Some(translation("de", "Tempo")),
    };
    let pl = DiffPart::Translation {
        lang: translation("pl", "Tempo").lang,
        old: Some(translation("pl", "Prędkość")),
        new: Some(translation("pl", "Szybkość")),
    };
    assert_eq!(
        section(&rows, DiffKind::Translations),
        vec![("SPEED".to_owned(), de), ("SPEED".to_owned(), pl)]
    );
}

#[test]
fn parameter_without_a_change() {
    let rows = diff_of(vec![speed(1, "SPEED")], vec![speed(1, "SPEED")], ROWS);
    for kind in CHANGE_KINDS
        .into_iter()
        .chain([DiffKind::Added, DiffKind::Removed])
    {
        assert!(rows.section(kind).is_empty(), "{kind:?}");
    }
    assert!(rows.is_empty());
    assert_eq!(rows.unchanged(), 1);
}

#[test]
fn match_by_key_follows_the_order_of_the_keys_and_skips_an_unknown_key() {
    let old: Vec<Parameter> = vec![Parameter::from(speed(1, "SPEED"))];
    let new: Vec<Parameter> = vec![
        Parameter::from(speed(1, "SPEED")),
        Parameter::from(speed(2, "HOLD")),
    ];
    let key = |n| ParameterKey::from(Uuid::from_u128(n));
    let pairs = DiffPair::match_by_key(&[key(2), key(9), key(1)], &old, &new);
    let sides: Vec<(ParameterKey, bool, bool)> = pairs
        .iter()
        .map(|pair| {
            (
                pair.key(),
                pair.old_side().is_some(),
                pair.new_side().is_some(),
            )
        })
        .collect();
    assert_eq!(sides, vec![(key(2), false, true), (key(1), true, true)]);
}
