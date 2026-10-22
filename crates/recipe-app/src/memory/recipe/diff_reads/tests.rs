//! Tests of the in-memory `DiffReads` (task 6.1, design D4).

use recipe_domain::plant::{PlcPlatform, Primitive, UnitPath};
use recipe_domain::recipe::{
    Ordinal, ParameterKey, ParameterTag, StoredLimit, StoredParameter, StoredValue, Translation,
};
use serde_json::json;

use crate::memory::fixtures::{
    Fixture, lang, limit, param, pid, pkey, text, translation_pl, uid, vid,
};
use crate::memory::{MemoryTransaction, seed_type_id};
use crate::ports::{ChangeFlags, Isolation, Transaction};

/// The approved version 1 of `R-1`: the old side.
const OLD: u128 = 101;
/// The live draft version 2 of `R-1`: the new side.
const NEW: u128 = 102;
/// The offset of the identifier of the new copy of a parameter.
const COPY: u128 = 500;

/// A fixture with the machine `M1`, the approved version 1, and the draft version 2 of `R-1`.
fn fixture() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    fixture
}

/// Return a copy of `parameter` with another identifier and the same key.
fn copy(parameter: &StoredParameter) -> StoredParameter {
    let id = parameter.id.as_uuid().as_u128().checked_add(COPY).unwrap();
    StoredParameter {
        id: pid(id),
        ..parameter.clone()
    }
}

/// Return the type identifier of a primitive.
fn primitive(primitive: Primitive) -> recipe_domain::plant::PlcTypeId {
    seed_type_id(primitive, None).unwrap()
}

/// Open a `RepeatableRead` transaction on the store of `fixture`.
fn begin(fixture: &Fixture) -> MemoryTransaction {
    fixture
        .store
        .begin_memory(Isolation::RepeatableRead)
        .unwrap()
}

/// Seed `old` into version 1 and `new` into version 2.
fn seed(fixture: &Fixture, old: &[StoredParameter], new: &[StoredParameter]) {
    for parameter in old {
        fixture.store.seed_parameter(vid(OLD), parameter);
    }
    for parameter in new {
        fixture.store.seed_parameter(vid(NEW), parameter);
    }
}

/// The flags, the sides, and the counts of one row as a compact tuple.
type Compact = (String, [bool; 6], (u32, u32, u32));

fn compact(row: &ChangeFlags) -> Compact {
    (
        row.tag.to_string(),
        [
            row.definition,
            row.value,
            row.limits,
            row.translations,
            row.only_old,
            row.only_new,
        ],
        (row.limit_rows, row.definition_rows, row.translation_rows),
    )
}

/// The old side of the case list: `A` to `E`, `G` (old only), and `H` and `I` (retyped).
fn old_side() -> Vec<StoredParameter> {
    vec![
        param(1, 101, "A"),
        param(2, 101, "B"),
        param(3, 101, "C"),
        param(4, 101, "D"),
        param(5, 101, "E"),
        param(7, 101, "G"),
        param(8, 101, "H"),
        StoredParameter {
            limits: Vec::new(),
            ..param(9, 101, "I")
        },
    ]
}

/// Return a copy of `parameter` with a changed Polish name and an added German translation.
fn translated(parameter: &StoredParameter) -> StoredParameter {
    let changed = Translation {
        name: text("Inna"),
        ..translation_pl()
    };
    let added = Translation {
        lang: lang("de"),
        ..translation_pl()
    };
    StoredParameter {
        translations: vec![changed, added],
        ..copy(parameter)
    }
}

/// The new side of the case list: one change for each key of `old`, and `F` (new only).
fn new_side(old: &[StoredParameter]) -> Vec<StoredParameter> {
    let dint = primitive(Primitive::Dint);
    vec![
        copy(&old[0]),
        StoredParameter {
            value: StoredValue::from(json!(6)),
            ..copy(&old[1])
        },
        StoredParameter {
            limits: vec![StoredLimit {
                max: Some("20".parse().unwrap()),
                ..limit()
            }],
            ..copy(&old[2])
        },
        StoredParameter {
            tag: ParameterTag::parse("D2").unwrap(),
            name_en: text("Other"),
            ..copy(&old[3])
        },
        translated(&old[4]),
        param(6, 101, "F"),
        StoredParameter {
            plc_type: dint,
            ..copy(&old[6])
        },
        StoredParameter {
            plc_type: dint,
            ..copy(&old[7])
        },
    ]
}

/// The two sides of the case list of `change_flags_give_one_row_per_key_and_part`.
fn case_sides() -> (Vec<StoredParameter>, Vec<StoredParameter>) {
    let old = old_side();
    let new = new_side(&old);
    (old, new)
}

#[tokio::test]
async fn change_flags_give_one_row_per_key_and_part() {
    let fixture = fixture();
    let (old, new) = case_sides();
    seed(&fixture, &old, &new);
    let mut tx = begin(&fixture);

    let rows = tx
        .diff_reads()
        .change_flags(Some(vid(OLD)), vid(NEW))
        .await
        .unwrap();

    let none = [false; 6];
    let flag = |index: usize| {
        let mut flags = none;
        *flags.get_mut(index).unwrap() = true;
        flags
    };
    let expected: Vec<Compact> = vec![
        ("A".into(), none, (0, 0, 0)),
        ("B".into(), flag(1), (0, 0, 0)),
        ("C".into(), flag(2), (1, 0, 0)),
        ("D2".into(), flag(0), (0, 2, 0)),
        ("E".into(), flag(3), (0, 0, 2)),
        (
            "F".into(),
            [false, false, true, false, false, true],
            (1, 0, 0),
        ),
        (
            "G".into(),
            [false, false, true, false, true, false],
            (1, 0, 0),
        ),
        (
            "H".into(),
            [true, true, true, false, false, false],
            (1, 1, 0),
        ),
        (
            "I".into(),
            [true, true, false, false, false, false],
            (0, 1, 0),
        ),
    ];
    assert_eq!(rows.iter().map(compact).collect::<Vec<_>>(), expected);
}

#[tokio::test]
async fn a_first_approval_gives_every_key_only_in_the_new_version() {
    let fixture = fixture();
    seed(
        &fixture,
        &[param(1, 101, "A")],
        &[param(11, 101, "B"), param(12, 101, "C")],
    );
    let mut tx = begin(&fixture);

    let rows = tx.diff_reads().change_flags(None, vid(NEW)).await.unwrap();

    let only_new = [false, false, true, false, false, true];
    assert_eq!(
        rows.iter().map(compact).collect::<Vec<_>>(),
        vec![
            ("B".into(), only_new, (1, 0, 0)),
            ("C".into(), only_new, (1, 0, 0)),
        ]
    );
}

#[tokio::test]
async fn change_flags_come_in_the_order_of_the_parameter_table() {
    let fixture = fixture();
    let ordinal = |n: u32| Ordinal::parse(n).unwrap();
    let new = [
        param(1, 102, "A"),
        StoredParameter {
            ordinal: ordinal(2),
            ..param(2, 101, "B")
        },
        StoredParameter {
            ordinal: ordinal(1),
            ..param(3, 101, "Z")
        },
    ];
    let old_only = param(4, 103, "OLD");
    seed(&fixture, &[old_only], &new);
    let mut tx = begin(&fixture);

    let rows = tx
        .diff_reads()
        .change_flags(Some(vid(OLD)), vid(NEW))
        .await
        .unwrap();

    let order: Vec<(ParameterKey, UnitPath, Ordinal)> = rows
        .iter()
        .map(|row| (row.key, row.unit_path.clone(), row.ordinal))
        .collect();
    let path = |text: &str| UnitPath::parse(text).unwrap();
    assert_eq!(
        order,
        vec![
            (pkey(3), path("U1"), ordinal(1)),
            (pkey(2), path("U1"), ordinal(2)),
            (pkey(1), path("U1.2"), ordinal(0)),
            (pkey(4), path("U1.2.1"), ordinal(0)),
        ]
    );
}

#[tokio::test]
async fn pairs_give_the_stored_parameters_of_the_keys_on_each_side() {
    let fixture = fixture();
    let (old, new) = case_sides();
    seed(&fixture, &old, &new);
    let mut tx = begin(&fixture);
    let keys = [pkey(2), pkey(6), pkey(7), pkey(99)];

    let sides = tx
        .diff_reads()
        .pairs(Some(vid(OLD)), Some(vid(NEW)), &keys)
        .await
        .unwrap();

    let ids = |side: &[StoredParameter]| side.iter().map(|p| p.id).collect::<Vec<_>>();
    assert_eq!(ids(&sides.old), vec![pid(2), pid(7)]);
    assert_eq!(ids(&sides.new), vec![pid(502), pid(6)]);
    assert_eq!(
        sides.new.first().unwrap().value,
        StoredValue::from(json!(6))
    );
}

#[tokio::test]
async fn a_side_without_a_version_gives_no_parameter() {
    let fixture = fixture();
    let (old, new) = case_sides();
    seed(&fixture, &old, &new);
    let mut tx = begin(&fixture);
    let keys = [pkey(2)];

    let first = tx
        .diff_reads()
        .pairs(None, Some(vid(NEW)), &keys)
        .await
        .unwrap();
    let old_only = tx
        .diff_reads()
        .pairs(Some(vid(OLD)), None, &keys)
        .await
        .unwrap();

    assert!(first.old.is_empty());
    assert_eq!(first.new.len(), 1);
    assert_eq!(old_only.old.len(), 1);
    assert!(old_only.new.is_empty());
}

#[tokio::test]
async fn a_unit_of_an_old_parameter_keeps_its_path_after_the_archive() {
    let fixture = fixture();
    seed(&fixture, &[param(4, 103, "OLD")], &[]);
    fixture.store.update_state(|state, now| {
        let unit = state.units.get_mut(&uid(103)).unwrap();
        unit.archived_at = Some(now);
    });
    let mut tx = begin(&fixture);

    let rows = tx
        .diff_reads()
        .change_flags(Some(vid(OLD)), vid(NEW))
        .await
        .unwrap();

    assert_eq!(
        rows.first().map(|row| row.unit_path.clone()),
        Some(UnitPath::parse("U1.2.1").unwrap())
    );
}
