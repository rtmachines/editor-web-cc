//! Tests of the rule "Two members of a struct on CODESYS" in `DefineUdt` (ADR-0204).

use std::collections::BTreeSet;

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    DerivedTypeReason, MemberName, PlantError, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord,
    RecordShape, TypeMember, TypeName,
};

use super::{DefineUdtInput, DefineUdtMode, fixture, input, real, run};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, mid, tid};

/// The reason of an `E-PLT-013` error, or `None` for another error.
fn reason_of(error: &AppError) -> Option<DerivedTypeReason> {
    match error {
        AppError::Plant(PlantError::DerivedTypeInvalid { reason, .. }) => Some(*reason),
        _ => None,
    }
}

/// Store the struct `name@1` with the given members on machine 1 for `platform`.
fn seed_struct(fixture: &Fixture, platform: PlcPlatform, name: &str, members: &[&str]) {
    let type_id: PlcTypeId = real();
    let record = PlcTypeRecord {
        id: tid(990),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision: 1,
            supersedes: None,
            members: members
                .iter()
                .map(|member| TypeMember {
                    name: MemberName::parse(member).unwrap(),
                    type_id,
                })
                .collect(),
        },
    };
    fixture.store.seed_type(&PlcType::try_from(record).unwrap());
}

/// Run `DefineUdt` and expect `reason`. Assert that the command created no type.
async fn assert_too_few(fixture: &Fixture, request: DefineUdtInput, context: &str) {
    let before = fixture.store.state().types.len();
    let error = run(fixture, Role::ProcessEngineer, request)
        .await
        .unwrap_err();
    assert_eq!(
        reason_of(&error),
        Some(DerivedTypeReason::TooFewMembers),
        "{context}: {error:?}"
    );
    assert_eq!(fixture.store.state().types.len(), before, "{context}");
    assert!(fixture.store.audit_entries().is_empty(), "{context}");
}

#[tokio::test]
async fn adr_0204_b_one_member_on_codesys_gives_plt_013_with_too_few_members() {
    let fixture = fixture(PlcPlatform::CodesysIec);
    let request = input(DefineUdtMode::New, "ST_One", &[("value", "INT")]);
    assert_too_few(&fixture, request, "new").await;
}

#[tokio::test]
async fn adr_0204_b_one_member_on_twincat_and_s7_creates_the_struct() {
    for platform in [
        PlcPlatform::TwinCat3,
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        let fixture = fixture(platform);
        let output = run(
            &fixture,
            Role::ProcessEngineer,
            input(DefineUdtMode::New, "ST_One", &[("value", "INT")]),
        )
        .await
        .unwrap();
        assert_eq!(output.revision, 1, "{platform:?}");
    }
}

#[tokio::test]
async fn adr_0204_b_two_members_on_codesys_create_the_struct() {
    let fixture = fixture(PlcPlatform::CodesysIec);
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Pair",
            &[("first", "INT"), ("second", "INT")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}

#[tokio::test]
async fn adr_0204_b_no_member_still_gives_plt_016_on_every_platform() {
    for platform in [
        PlcPlatform::CodesysIec,
        PlcPlatform::TwinCat3,
        PlcPlatform::S7_1500,
    ] {
        let fixture = fixture(platform);
        let error = run(
            &fixture,
            Role::ProcessEngineer,
            input(DefineUdtMode::New, "ST_Empty", &[]),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(
                error,
                AppError::Plant(PlantError::StructHasNoMembers { .. })
            ),
            "{platform:?}: {error:?}"
        );
    }
}

#[tokio::test]
async fn adr_0204_a_a_new_revision_with_one_member_is_refused_on_codesys() {
    let fixture = fixture(PlcPlatform::CodesysIec);
    seed_struct(&fixture, PlcPlatform::CodesysIec, "ST_Zone", &["a", "b"]);
    let request = input(DefineUdtMode::Revision, "ST_Zone", &[("a", "REAL")]);
    assert_too_few(&fixture, request, "revision").await;
}

#[tokio::test]
async fn adr_0204_a_the_name_check_runs_before_the_count_check() {
    let fixture = fixture(PlcPlatform::CodesysIec);
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Coil", &[("END_IF", "BOOL")]),
    )
    .await
    .unwrap_err();
    assert_eq!(
        reason_of(&error),
        Some(DerivedTypeReason::ReservedMemberName),
        "{error:?}"
    );
}

#[tokio::test]
async fn adr_0204_c1_a_member_type_that_names_a_stored_one_member_struct_gives_plt_013() {
    for member_type in ["ST_One@1", "ARRAY[1..2] OF ST_One@1"] {
        let fixture = fixture(PlcPlatform::CodesysIec);
        seed_struct(&fixture, PlcPlatform::CodesysIec, "ST_One", &["a"]);
        let request = input(
            DefineUdtMode::New,
            "ST_Pair",
            &[("first", member_type), ("second", "INT")],
        );
        assert_too_few(&fixture, request, member_type).await;
    }
}

#[tokio::test]
async fn adr_0204_c1_a_member_type_with_a_stored_one_member_struct_on_twincat_creates_the_struct() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    seed_struct(&fixture, PlcPlatform::TwinCat3, "ST_One", &["a"]);
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Pair",
            &[("first", "ST_One@1"), ("second", "INT")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}

#[tokio::test]
async fn adr_0204_c1_a_member_type_with_a_stored_two_member_struct_on_codesys_creates_the_struct() {
    let fixture = fixture(PlcPlatform::CodesysIec);
    seed_struct(&fixture, PlcPlatform::CodesysIec, "ST_Zone", &["a", "b"]);
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Outer",
            &[("zone", "ST_Zone@1"), ("count", "INT")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}
