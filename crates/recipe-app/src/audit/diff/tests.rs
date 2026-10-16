use std::collections::BTreeSet;

use recipe_domain::plant::{
    LeafPath, MemberName, PlatformTypes, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive,
    RecordShape, S7Address, TypeMember, TypeName, TypeTrees,
};
use recipe_domain::recipe::{
    Draft, Parameter, ParameterKey, StoredDraft, StoredLimit, StoredParameter, StoredValue,
    Translation, VersionState,
};
use serde_json::{Value, json};

use super::*;
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, lang, long_text, mid, param, pkey, reference, text, tid, uid, user, vid,
};
use crate::memory::seed_type_id;

const PLATFORM: PlcPlatform = PlcPlatform::S7_300;

/// The type rows of an `S7_300` machine: a struct `Zone` in two revisions.
struct Rig {
    trees: TypeTrees,
    int: PlcTypeId,
    zone1: PlcTypeId,
    zone2: PlcTypeId,
}

fn zone(id: u128, revision: u32, supersedes: Option<PlcTypeId>, extra: bool) -> PlcType {
    let real = seed_type_id(Primitive::Real, None).unwrap();
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let bool_type = seed_type_id(Primitive::Bool, None).unwrap();
    let member = |name: &str, type_id| TypeMember {
        name: MemberName::parse(name).unwrap(),
        type_id,
    };
    let mut members = vec![member("temp_sp", real), member("setpoint", int)];
    if extra {
        members.push(member("extra", bool_type));
    }
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Struct {
            name: TypeName::parse("Zone").unwrap(),
            revision,
            supersedes,
            members,
        },
    })
    .unwrap()
}

fn rig() -> Rig {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.store.seed_type(&zone(900, 1, None, false));
    fixture.store.seed_type(&zone(901, 2, Some(tid(900)), true));
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let real = seed_type_id(Primitive::Real, None).unwrap();
    let bool_type = seed_type_id(Primitive::Bool, None).unwrap();
    let state = fixture.store.state();
    let rows: Vec<PlcType> = [int, real, bool_type, tid(900), tid(901)]
        .into_iter()
        .map(|id| state.plc_type(id).unwrap().clone())
        .collect();
    let types = PlatformTypes::collect(machine.id(), PLATFORM, rows).unwrap();
    let trees = TypeTrees::resolve([int, tid(900), tid(901)], &types);
    Rig {
        trees,
        int,
        zone1: tid(900),
        zone2: tid(901),
    }
}

fn limit(path: &str, min: &str, max: &str) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: Some(min.parse().unwrap()),
        max: Some(max.parse().unwrap()),
        max_length: None,
    }
}

/// A `Zone` revision 1 parameter with `temp_sp` 85.5 and a limit on `setpoint`.
fn zone_row(rig: &Rig, tag: &str) -> StoredParameter {
    StoredParameter {
        plc_type: rig.zone1,
        value: StoredValue::from(json!({"temp_sp": 85.5, "setpoint": 10})),
        limits: vec![limit(".setpoint", "0", "10")],
        translations: Vec::new(),
        s7_address: Some(S7Address::parse("DB1.DBW0").unwrap()),
        ..param(1, 101, tag)
    }
}

fn parameter(row: StoredParameter) -> Parameter {
    Parameter::from(row)
}

fn pair(diff: &StateDiff) -> (&Value, &Value) {
    (&diff.before, &diff.after)
}

#[test]
fn the_example_of_section_9_2_gives_one_leaf() {
    let rig = rig();
    let before = parameter(zone_row(&rig, "ZONE_1"));
    let after = parameter(StoredParameter {
        value: StoredValue::from(json!({"temp_sp": 90, "setpoint": 10})),
        ..zone_row(&rig, "ZONE_1")
    });
    let diff = value_diff_of(Some(&before), Some(&after), &rig.trees).unwrap();
    assert_eq!(diff.before, json!({"value": {".temp_sp": 85.5}}));
    assert_eq!(diff.after, json!({"value": {".temp_sp": 90}}));
}

#[test]
fn an_unchanged_parameter_gives_empty_diffs() {
    let rig = rig();
    let one = parameter(zone_row(&rig, "ZONE_1"));
    let two = parameter(zone_row(&rig, "ZONE_1"));
    let value = value_diff_of(Some(&one), Some(&two), &rig.trees).unwrap();
    assert_eq!(pair(&value), (&json!({}), &json!({})));
    let limits = limit_diff_of(Some(&one), Some(&two));
    assert_eq!(pair(&limits), (&json!({}), &json!({})));
    let translations = translation_diff_of(Some(&one), Some(&two));
    assert_eq!(pair(&translations), (&json!({}), &json!({})));
    let definition = definition_diff_of(Some(&one), Some(&two), &rig.trees).unwrap();
    assert_eq!(pair(&definition), (&json!({}), &json!({})));
}

#[test]
fn a_tag_change_gives_only_the_tag() {
    let rig = rig();
    let before = parameter(zone_row(&rig, "OLD"));
    let after = parameter(zone_row(&rig, "NEW"));
    let diff = definition_diff_of(Some(&before), Some(&after), &rig.trees).unwrap();
    assert_eq!(diff.before, json!({"tag": "OLD"}));
    assert_eq!(diff.after, json!({"tag": "NEW"}));
}

#[test]
fn a_definition_diff_names_the_changed_columns() {
    let rig = rig();
    let before = parameter(zone_row(&rig, "TAG"));
    let after = parameter(StoredParameter {
        unit: uid(102),
        name_en: text("Other"),
        s7_address: None,
        ..zone_row(&rig, "TAG")
    });
    let diff = definition_diff_of(Some(&before), Some(&after), &rig.trees).unwrap();
    assert_eq!(
        diff.before,
        json!({
            "unit_id": uid(101).to_string(),
            "name_en": "Name",
            "s7_address": "DB1.DBW0",
        })
    );
    assert_eq!(
        diff.after,
        json!({"unit_id": uid(102).to_string(), "name_en": "Other", "s7_address": null})
    );
}

#[test]
fn a_retype_adds_the_value_and_limit_parts_to_the_definition_diff() {
    let rig = rig();
    let before = parameter(StoredParameter {
        plc_type: rig.int,
        value: StoredValue::from(json!(5)),
        limits: vec![limit("", "0", "10")],
        ..zone_row(&rig, "TAG")
    });
    let after = parameter(zone_row(&rig, "TAG"));
    let diff = definition_diff_of(Some(&before), Some(&after), &rig.trees).unwrap();
    assert_eq!(
        diff.before,
        json!({
            "plc_type_id": rig.int.to_string(),
            "value": {"": 5, ".temp_sp": null, ".setpoint": null},
            "limits": {
                "": {"min_value": "0", "max_value": "10", "max_length": null},
                ".setpoint": null,
            },
        })
    );
    assert_eq!(
        diff.after,
        json!({
            "plc_type_id": rig.zone1.to_string(),
            "value": {"": null, ".temp_sp": 85.5, ".setpoint": 10},
            "limits": {
                "": null,
                ".setpoint": {"min_value": "0", "max_value": "10", "max_length": null},
            },
        })
    );
}

#[test]
fn a_limit_change_gives_one_limit_key() {
    let rig = rig();
    let before = parameter(zone_row(&rig, "TAG"));
    let after = parameter(StoredParameter {
        limits: vec![limit(".setpoint", "0", "20")],
        ..zone_row(&rig, "TAG")
    });
    let diff = limit_diff_of(Some(&before), Some(&after));
    assert_eq!(
        diff.before,
        json!({"limits": {".setpoint": {"min_value": "0", "max_value": "10", "max_length": null}}})
    );
    assert_eq!(
        diff.after,
        json!({"limits": {".setpoint": {"min_value": "0", "max_value": "20", "max_length": null}}})
    );
}

#[test]
fn a_removed_limit_is_null_after() {
    let rig = rig();
    let before = parameter(zone_row(&rig, "TAG"));
    let after = parameter(StoredParameter {
        limits: Vec::new(),
        ..zone_row(&rig, "TAG")
    });
    let diff = limit_diff_of(Some(&before), Some(&after));
    assert_eq!(diff.after, json!({"limits": {".setpoint": null}}));
}

#[test]
fn a_new_translation_has_null_before() {
    let rig = rig();
    let before = parameter(zone_row(&rig, "TAG"));
    let after = parameter(StoredParameter {
        translations: vec![Translation {
            lang: lang("pl"),
            name: text("Nazwa"),
            description: Some(long_text("Opis")),
        }],
        ..zone_row(&rig, "TAG")
    });
    let diff = translation_diff_of(Some(&before), Some(&after));
    assert_eq!(diff.before, json!({"translations": {"pl": null}}));
    assert_eq!(
        diff.after,
        json!({"translations": {"pl": {"name": "Nazwa", "description": "Opis"}}})
    );
}

#[test]
fn a_changed_translation_keeps_only_its_language() {
    let rig = rig();
    let make = |name: &str| StoredParameter {
        translations: vec![
            Translation {
                lang: lang("pl"),
                name: text(name),
                description: None,
            },
            Translation {
                lang: lang("de"),
                name: text("Name"),
                description: None,
            },
        ],
        ..zone_row(&rig, "TAG")
    };
    let diff = translation_diff_of(
        Some(&parameter(make("Stary"))),
        Some(&parameter(make("Nowy"))),
    );
    assert_eq!(
        diff.before,
        json!({"translations": {"pl": {"name": "Stary", "description": ""}}})
    );
    assert_eq!(
        diff.after,
        json!({"translations": {"pl": {"name": "Nowy", "description": ""}}})
    );
}

#[test]
fn a_rebind_diff_holds_the_type_and_the_new_leaf() {
    let rig = rig();
    let before = parameter(zone_row(&rig, "TAG"));
    let after = parameter(StoredParameter {
        plc_type: rig.zone2,
        value: StoredValue::from(json!({"temp_sp": 85.5, "setpoint": 10, "extra": false})),
        ..zone_row(&rig, "TAG")
    });
    let diff = rebind_diff_of(Some(&before), Some(&after), &rig.trees).unwrap();
    assert_eq!(
        diff.before,
        json!({"plc_type_id": rig.zone1.to_string(), "value": {".extra": null}})
    );
    assert_eq!(
        diff.after,
        json!({"plc_type_id": rig.zone2.to_string(), "value": {".extra": false}})
    );
}

#[test]
fn an_array_leaf_is_read_by_its_plc_index() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    let int = seed_type_id(Primitive::Int, None).unwrap();
    let array = PlcType::try_from(PlcTypeRecord {
        id: tid(910),
        machine: Some(machine.id()),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Array {
            element: int,
            dimensions: recipe_domain::plant::ArrayDimensions::parse(vec![
                recipe_domain::plant::ArrayDimension::parse(3, 4).unwrap(),
            ])
            .unwrap(),
        },
    })
    .unwrap();
    fixture.store.seed_type(&array);
    let rows = vec![fixture.store.state().plc_type(int).unwrap().clone(), array];
    let types = PlatformTypes::collect(machine.id(), PLATFORM, rows).unwrap();
    let trees = TypeTrees::resolve([tid(910)], &types);
    let make = |first, second| {
        parameter(StoredParameter {
            plc_type: tid(910),
            value: StoredValue::from(json!([first, second])),
            limits: Vec::new(),
            ..param(1, 101, "ARR")
        })
    };
    let diff = value_diff_of(Some(&make(1, 2)), Some(&make(1, 9)), &trees).unwrap();
    assert_eq!(diff.before, json!({"value": {"[4]": 2}}));
    assert_eq!(diff.after, json!({"value": {"[4]": 9}}));
}

#[test]
fn a_type_without_a_tree_gives_a_plant_error() {
    let rig = rig();
    let unknown = parameter(StoredParameter {
        plc_type: tid(999),
        ..zone_row(&rig, "TAG")
    });
    let error = value_diff_of(Some(&unknown), Some(&unknown), &rig.trees).unwrap_err();
    assert!(matches!(error, AppError::Plant(_)));
}

#[test]
fn a_new_parameter_has_nothing_before() {
    let rig = rig();
    let after = parameter(zone_row(&rig, "TAG"));
    let diff = value_diff_of(None, Some(&after), &rig.trees).unwrap();
    assert_eq!(
        diff.before,
        json!({"value": {".temp_sp": null, ".setpoint": null}})
    );
}

#[test]
fn a_change_of_a_real_draft_gives_the_diff_of_its_leaf() {
    let rig = rig();
    let row = StoredParameter {
        key: pkey(1),
        s7_address: Some(S7Address::parse("DB1.DBB0").unwrap()),
        ..zone_row(&rig, "ZONE_1")
    };
    let mut draft = Draft::load(StoredDraft {
        reference: reference("R-1"),
        machine: mid(1),
        platform: PLATFORM,
        version: vid(11),
        number: 1,
        state: VersionState::LiveDraft,
        row_version: 1,
        created_by: user(2),
        editors: Vec::new(),
        parameters: vec![row],
    })
    .unwrap();
    let key: ParameterKey = pkey(1);
    let value =
        recipe_domain::recipe::ValueInput::Json(r#"{"temp_sp": 90, "setpoint": 10}"#.to_owned());
    let change = draft.set_value(key, &value, &rig.trees).unwrap();
    let diff = value_diff(&change, &rig.trees).unwrap();
    let stored = change.after().unwrap().value().as_json().clone();
    assert_eq!(diff.before, json!({"value": {".temp_sp": 85.5}}));
    assert_eq!(
        diff.after,
        json!({"value": {".temp_sp": stored.get("temp_sp").cloned().unwrap()}})
    );
}
