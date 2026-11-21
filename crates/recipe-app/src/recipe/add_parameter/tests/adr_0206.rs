//! Tests of the rule "Vendor aliases of type names" in `AddParameter` (ADR-0206).

use std::collections::BTreeSet;

use recipe_domain::plant::{
    DerivedTypeReason, MemberName, PlantError, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord,
    Primitive, RecordShape, RuleSet, TypeExpr, TypeMember, TypeName,
};
use recipe_domain::recipe::ValueInput;

use super::{AddParameterInput, assert_nothing_written, empty_draft, input, run};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, mid, tid};
use crate::memory::seed_type_id;
use crate::recipe::rebind::fixtures::primitive_id;
use crate::recipe::test_support::stored_parameters;

/// A draft of a machine on `platform` that has the rows of the time primitives.
fn time_draft(platform: PlcPlatform) -> Fixture {
    let fixture = empty_draft(platform);
    fixture.store.seed_time_primitive_types();
    fixture
}

/// A value of a `TIME_OF_DAY` parameter, as JSON text.
const TOD_VALUE: &str = "\"TOD#06:00:00\"";
/// A value of a `DATE_AND_TIME` parameter, as JSON text.
const DT_VALUE: &str = "\"DT#2026-10-12-12:30:00\"";

/// The reason of an `E-PLT-013` error, or `None` for another error.
fn reason_of(error: &AppError) -> Option<DerivedTypeReason> {
    match error {
        AppError::Plant(PlantError::DerivedTypeInvalid { reason, .. }) => Some(*reason),
        _ => None,
    }
}

/// The only stored parameter of the draft.
fn only_type(fixture: &Fixture) -> PlcTypeId {
    let rows = stored_parameters(fixture);
    assert_eq!(rows.len(), 1);
    rows.first().unwrap().plc_type
}

/// The element type of an array row that the committed state holds.
fn array_element(fixture: &Fixture, array: PlcTypeId) -> PlcTypeId {
    let state = fixture.store.state();
    let row = state.types.get(&array).unwrap();
    match row.plc_type.shape() {
        RecordShape::Array { element, .. } => Some(*element),
        _ => None,
    }
    .unwrap()
}

/// A request of the request type text `type_text` as the request parser builds it after a
/// refusal: with the earlier rule set (ADR-0206 (d3)).
fn earlier_input(type_text: &str) -> AddParameterInput {
    AddParameterInput {
        type_expr: TypeExpr::from_stored(type_text, RuleSet::Earlier).unwrap(),
        ..input("INT", "0", Vec::new())
    }
}

/// Store the struct `DT@1` with two members. It is a row from before ADR-0143, so the fixture
/// builds it with the earlier rule set.
fn seed_struct_dt(fixture: &Fixture) {
    let int = primitive_id(Primitive::Int);
    let record = PlcTypeRecord {
        id: tid(984),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PlcPlatform::CodesysIec]),
        shape: RecordShape::Struct {
            name: TypeName::from_stored("DT", RuleSet::Earlier).unwrap(),
            revision: 1,
            supersedes: None,
            members: ["a", "b"]
                .map(|member| TypeMember {
                    name: MemberName::parse(member).unwrap(),
                    type_id: int,
                })
                .to_vec(),
        },
    };
    fixture
        .store
        .seed_type(&PlcType::from_stored(record, RuleSet::Earlier).unwrap());
}

#[tokio::test]
async fn adr_0206_tod_stores_time_of_day() {
    let fixture = time_draft(PlcPlatform::CodesysIec);
    run(&fixture, input("TOD", TOD_VALUE, Vec::new()))
        .await
        .unwrap();
    assert_eq!(
        only_type(&fixture),
        primitive_id(Primitive::TimeOfDay),
        "the stored type is the catalog name"
    );
}

#[tokio::test]
async fn adr_0206_each_alias_stores_its_catalog_name_on_codesys_twincat_and_s7() {
    use PlcPlatform::{CodesysIec, S7_1200, S7_1500, TwinCat3};
    // `DATE_AND_TIME` is not on `S7_1200` (`data-model.md` section 7). `S7_300` needs an address.
    let cases = [
        (CodesysIec, "TOD", TOD_VALUE, Primitive::TimeOfDay),
        (TwinCat3, "TOD", TOD_VALUE, Primitive::TimeOfDay),
        (S7_1200, "TOD", TOD_VALUE, Primitive::TimeOfDay),
        (S7_1500, "TOD", TOD_VALUE, Primitive::TimeOfDay),
        (CodesysIec, "DT", DT_VALUE, Primitive::DateAndTime),
        (TwinCat3, "DT", DT_VALUE, Primitive::DateAndTime),
        (S7_1500, "DT", DT_VALUE, Primitive::DateAndTime),
    ];
    for (platform, alias, value, catalog) in cases {
        let fixture = time_draft(platform);
        let added = run(&fixture, input(alias, value, Vec::new())).await;
        assert!(added.is_ok(), "{platform:?} {alias}: {added:?}");
        assert_eq!(
            only_type(&fixture),
            primitive_id(catalog),
            "{platform:?} {alias}"
        );
    }
}

#[tokio::test]
async fn adr_0206_the_long_aliases_store_ltod_and_ldt() {
    for platform in [
        PlcPlatform::CodesysIec,
        PlcPlatform::TwinCat3,
        PlcPlatform::S7_1500,
    ] {
        for (alias, value, catalog) in [
            ("LTIME_OF_DAY", "\"LTOD#06:00:00\"", Primitive::Ltod),
            (
                "LDATE_AND_TIME",
                "\"LDT#2026-10-12-12:30:00\"",
                Primitive::Ldt,
            ),
        ] {
            let fixture = time_draft(platform);
            let added = run(&fixture, input(alias, value, Vec::new())).await;
            assert!(added.is_ok(), "{platform:?} {alias}: {added:?}");
            assert_eq!(
                only_type(&fixture),
                primitive_id(catalog),
                "{platform:?} {alias}"
            );
        }
    }
}

#[tokio::test]
async fn adr_0206_an_alias_as_the_element_of_an_array_stores_the_catalog_name() {
    let fixture = time_draft(PlcPlatform::S7_1500);
    run(
        &fixture,
        input(
            "ARRAY[1..2] OF DT",
            "[\"DT#2026-10-12-12:30:00\",\"DT#2026-10-13-12:30:00\"]",
            Vec::new(),
        ),
    )
    .await
    .unwrap();
    let array = only_type(&fixture);
    assert_eq!(
        array_element(&fixture, array),
        seed_type_id(Primitive::DateAndTime, None).unwrap()
    );
}

#[tokio::test]
async fn adr_0206_an_alias_with_a_revision_number_gives_plt_013_with_reserved_name() {
    for text in ["DT@1", "TOD@1", "LTIME_OF_DAY@1", "LDATE_AND_TIME@1"] {
        let fixture = time_draft(PlcPlatform::CodesysIec);
        let error = run(&fixture, earlier_input(text)).await.unwrap_err();
        assert_eq!(
            reason_of(&error),
            Some(DerivedTypeReason::ReservedName),
            "{text}: {error:?}"
        );
        assert_nothing_written(&fixture);
        assert!(stored_parameters(&fixture).is_empty(), "{text}");
    }
}

#[tokio::test]
async fn adr_0206_an_alias_with_a_revision_number_stays_refused_beside_a_stored_struct() {
    let fixture = time_draft(PlcPlatform::CodesysIec);
    seed_struct_dt(&fixture);
    let request = AddParameterInput {
        value: ValueInput::Json("{\"a\":1,\"b\":2}".to_owned()),
        ..earlier_input("DT@1")
    };
    let error = run(&fixture, request).await.unwrap_err();
    assert_eq!(
        reason_of(&error),
        Some(DerivedTypeReason::ReservedName),
        "{error:?}"
    );
    assert_nothing_written(&fixture);
    assert!(stored_parameters(&fixture).is_empty());
}

#[test]
fn adr_0206_the_literal_prefixes_t_d_and_ld_stay_grammar_errors() {
    for text in ["T", "D", "LD"] {
        let error = TypeExpr::parse(text).unwrap_err();
        assert!(
            matches!(
                error,
                PlantError::DerivedTypeInvalid {
                    reason: DerivedTypeReason::Grammar,
                    ..
                }
            ),
            "{text}: {error:?}"
        );
    }
}
