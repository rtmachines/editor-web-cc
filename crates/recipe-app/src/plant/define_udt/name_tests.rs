//! Tests of the case-insensitive name indexes in `DefineUdt` (ADR-0143 (j), design D4).

use std::collections::BTreeSet;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    DerivedTypeReason, MachineCode, MemberName, PlantError, PlcPlatform, PlcType, PlcTypeRecord,
    RecordShape, RuleSet, TypeExpr, TypeMember, TypeName,
};

use super::tests::{fixture, input, own_rows, real, run};
use super::{DefineUdtInput, DefineUdtMode};
use crate::error::{AppError, Missing};
use crate::memory::Fault;
use crate::memory::fixtures::{Fixture, mid, tid};

#[tokio::test]
async fn the_index_plc_type_struct_name_ci_unique_gives_plt_015_and_writes_nothing() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    fixture
        .store
        .fail_insert_struct(Fault::DuplicateTypeNameAnyCase);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "st_zone", &[("setpoint", "REAL")]),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_015);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::DuplicateTypeName { type_name })
            if type_name.as_str() == "st_zone"
    ));
    assert!(fixture.store.audit_entries().is_empty());
    assert!(own_rows(&fixture).is_empty());
}

#[tokio::test]
async fn the_index_plc_type_member_name_ci_unique_gives_plt_021_for_the_first_member() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    fixture
        .store
        .fail_insert_struct(Fault::DuplicateMemberNameAnyCase);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Zone",
            &[("speed", "INT"), ("label", "REAL")],
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_021);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::DuplicateMemberName { type_name, member })
            if type_name.as_str() == "ST_Zone" && member.as_str() == "speed"
    ));
    assert!(fixture.store.audit_entries().is_empty());
    assert!(own_rows(&fixture).is_empty());
}

/// The number of the machine `M12` of the spec scenarios. It holds the stored struct.
const STORED_MACHINE: u128 = 12;
/// The number of the machine `M7` of the spec scenarios. It holds no struct.
const EMPTY_MACHINE: u128 = 7;

/// Seed the machines `M12` and `M7` on `TwinCat3`.
fn two_machines() -> Fixture {
    let fixture = fixture(PlcPlatform::TwinCat3);
    fixture.seed_machine_on(PlcPlatform::TwinCat3, STORED_MACHINE, "M12");
    fixture.seed_machine_on(PlcPlatform::TwinCat3, EMPTY_MACHINE, "M7");
    fixture
}

/// A stored struct `name@1` of machine `machine` with the one member `setpoint` of `REAL`.
///
/// The row passes `RuleSet::Earlier`, so a name such as `Int` is possible.
fn stored_on(machine: u128, id: u128, name: &str) -> PlcType {
    PlcType::from_stored(
        PlcTypeRecord {
            id: tid(id),
            machine: Some(mid(machine)),
            platforms: BTreeSet::from([PlcPlatform::TwinCat3]),
            shape: RecordShape::Struct {
                name: TypeName::from_stored(name, RuleSet::Earlier).unwrap(),
                revision: 1,
                supersedes: None,
                members: vec![TypeMember {
                    name: MemberName::parse("setpoint").unwrap(),
                    type_id: real(),
                }],
            },
        },
        RuleSet::Earlier,
    )
    .unwrap()
}

/// The input of `DefineUdt` for the machine with the code `code`.
fn input_on(
    code: &str,
    mode: DefineUdtMode,
    name: &str,
    members: &[(&str, &str)],
) -> DefineUdtInput {
    DefineUdtInput {
        code: MachineCode::parse(code).unwrap(),
        ..input(mode, name, members)
    }
}

/// The `(name, revision)` of each struct row of machine `machine` in the committed state.
fn structs_of(fixture: &Fixture, machine: u128) -> Vec<(String, u32)> {
    fixture
        .store
        .state()
        .types
        .values()
        .filter(|row| row.plc_type.machine() == Some(mid(machine)))
        .filter_map(|row| match row.plc_type.shape() {
            RecordShape::Struct { name, revision, .. } => Some((name.to_string(), *revision)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_new_struct_whose_name_differs_only_in_case_gives_plt_015_for_the_stored_spelling() {
    let fixture = two_machines();
    fixture
        .store
        .seed_type(&stored_on(STORED_MACHINE, 901, "ST_Zone"));
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input_on("M12", DefineUdtMode::New, "st_zone", &[("label", "INT")]),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_015);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::DuplicateTypeName { type_name })
            if type_name.as_str() == "ST_Zone"
    ));
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(
        structs_of(&fixture, STORED_MACHINE),
        vec![("ST_Zone".to_owned(), 1)]
    );
}

#[tokio::test]
async fn a_revision_with_the_name_in_another_case_finds_no_struct() {
    let fixture = two_machines();
    fixture
        .store
        .seed_type(&stored_on(STORED_MACHINE, 901, "ST_Zone"));
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input_on(
            "M12",
            DefineUdtMode::Revision,
            "st_zone",
            &[("label", "INT")],
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(matches!(
        &error,
        AppError::NotFound { entity: Missing::Type { name, .. } } if name.as_str() == "st_zone"
    ));
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(
        structs_of(&fixture, STORED_MACHINE),
        vec![("ST_Zone".to_owned(), 1)]
    );
}

#[tokio::test]
async fn a_new_struct_with_the_name_of_another_machine_in_another_case_gets_revision_1() {
    let fixture = two_machines();
    fixture
        .store
        .seed_type(&stored_on(STORED_MACHINE, 901, "ST_Zone"));
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input_on("M7", DefineUdtMode::New, "st_zone", &[("label", "INT")]),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
    assert_eq!(
        structs_of(&fixture, EMPTY_MACHINE),
        vec![("st_zone".to_owned(), 1)]
    );
}

#[tokio::test]
async fn two_members_in_two_cases_give_plt_021_for_the_second_spelling_and_write_nothing() {
    let fixture = two_machines();
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input_on(
            "M12",
            DefineUdtMode::New,
            "ST_Zone",
            &[("speed", "INT"), ("Speed", "REAL")],
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_021);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::DuplicateMemberName { member, .. })
            if member.as_str() == "Speed"
    ));
    assert!(fixture.store.audit_entries().is_empty());
    assert!(structs_of(&fixture, STORED_MACHINE).is_empty());
}

#[tokio::test]
async fn a_member_named_like_a_primitive_creates_the_struct() {
    let fixture = two_machines();
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input_on("M12", DefineUdtMode::New, "ST_Text", &[("REAL", "INT")]),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
    assert_eq!(
        structs_of(&fixture, STORED_MACHINE),
        vec![("ST_Text".to_owned(), 1)]
    );
}

#[tokio::test]
async fn a_revision_of_a_stored_struct_with_a_reserved_name_gives_plt_013_and_writes_nothing() {
    let fixture = two_machines();
    fixture
        .store
        .seed_type(&stored_on(STORED_MACHINE, 901, "Int"));
    let revision = DefineUdtInput {
        name: TypeName::from_stored("Int", RuleSet::Earlier).unwrap(),
        ..input_on(
            "M12",
            DefineUdtMode::Revision,
            "ST_Zone",
            &[("label", "INT")],
        )
    };
    let error = run(&fixture, Role::ProcessEngineer, revision)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::ReservedName,
            ..
        })
    ));
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(
        structs_of(&fixture, STORED_MACHINE),
        vec![("Int".to_owned(), 1)]
    );
}

#[tokio::test]
async fn a_member_type_with_a_reserved_struct_name_gives_plt_013_and_takes_no_lock() {
    let fixture = two_machines();
    fixture
        .store
        .seed_type(&stored_on(STORED_MACHINE, 901, "Int"));
    let holder = DefineUdtInput {
        members: vec![(
            MemberName::parse("inner").unwrap(),
            TypeExpr::from_stored("Int@1", RuleSet::Earlier).unwrap(),
        )],
        ..input_on("M12", DefineUdtMode::New, "ST_Holder", &[])
    };
    let error = run(&fixture, Role::ProcessEngineer, holder)
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::DerivedTypeInvalid {
            reason: DerivedTypeReason::ReservedName,
            ..
        })
    ));
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(
        structs_of(&fixture, STORED_MACHINE),
        vec![("Int".to_owned(), 1)]
    );
}
