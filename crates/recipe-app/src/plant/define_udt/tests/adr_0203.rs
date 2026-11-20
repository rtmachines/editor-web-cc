//! Tests of the rule "Reserved names on CODESYS and TwinCAT" in `DefineUdt` (ADR-0203).

use std::collections::BTreeSet;

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    DerivedTypeReason, IEC_RESERVED_STRUCT_NAMES, MemberName, PlantError, PlcPlatform, PlcType,
    PlcTypeId, PlcTypeRecord, RecordShape, TypeMember, TypeName,
};

use super::{DefineUdtMode, fixture, input, real, run};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, mid, tid};

const IEC_PLATFORMS: [PlcPlatform; 2] = [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3];

/// The reason of an `E-PLT-013` error, or `None` for another error.
fn reason_of(error: &AppError) -> Option<DerivedTypeReason> {
    match error {
        AppError::Plant(PlantError::DerivedTypeInvalid { reason, .. }) => Some(*reason),
        _ => None,
    }
}

/// Store the struct `name@1` on machine 1 for `platform`.
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
async fn assert_refused(
    fixture: &Fixture,
    request: super::DefineUdtInput,
    reason: DerivedTypeReason,
    context: &str,
) {
    let before = fixture.store.state().types.len();
    let error = run(fixture, Role::ProcessEngineer, request)
        .await
        .unwrap_err();
    assert_eq!(reason_of(&error), Some(reason), "{context}");
    assert_eq!(fixture.store.state().types.len(), before, "{context}");
    assert!(fixture.store.audit_entries().is_empty(), "{context}");
}

#[tokio::test]
async fn adr_0203_a_each_reserved_struct_name_gives_plt_013_in_any_case() {
    for platform in IEC_PLATFORMS {
        for reserved in IEC_RESERVED_STRUCT_NAMES {
            for name in [reserved.to_owned(), reserved.to_lowercase()] {
                let fixture = fixture(platform);
                let request = input(DefineUdtMode::New, &name, &[("a", "INT"), ("b", "INT")]);
                assert_refused(
                    &fixture,
                    request,
                    DerivedTypeReason::ReservedName,
                    &format!("{platform:?} {name}"),
                )
                .await;
            }
        }
    }
}

#[tokio::test]
async fn adr_0203_a_a_library_type_name_on_codesys_gives_the_reserved_name_reason() {
    let fixture = fixture(PlcPlatform::CodesysIec);
    let request = input(
        DefineUdtMode::New,
        "st_amsaddr",
        &[("port", "INT"), ("net", "INT")],
    );
    assert_refused(
        &fixture,
        request,
        DerivedTypeReason::ReservedName,
        "st_amsaddr",
    )
    .await;
}

#[tokio::test]
async fn adr_0203_d_a_keyword_member_gives_the_reserved_member_name_reason() {
    for (platform, struct_name, members) in [
        (
            PlcPlatform::CodesysIec,
            "ST_Coil",
            [("END_IF", "BOOL"), ("S", "BOOL")],
        ),
        (
            PlcPlatform::TwinCat3,
            "ST_Pair",
            [("union", "INT"), ("b", "INT")],
        ),
        (
            PlcPlatform::TwinCat3,
            "ST_Pair",
            [("a", "INT"), ("END_UNION", "INT")],
        ),
    ] {
        let fixture = fixture(platform);
        let request = input(DefineUdtMode::New, struct_name, &members);
        assert_refused(
            &fixture,
            request,
            DerivedTypeReason::ReservedMemberName,
            &format!("{platform:?} {members:?}"),
        )
        .await;
    }
}

#[tokio::test]
async fn adr_0203_d_a_type_name_as_a_member_name_creates_the_struct() {
    let fixture = fixture(PlcPlatform::CodesysIec);
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Flags",
            &[("BIT", "BOOL"), ("XWORD", "BOOL")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}

#[tokio::test]
async fn adr_0203_e_a_keyword_member_and_a_reserved_name_on_s7_create_the_struct() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        for (name, members) in [
            ("ST_Coil", [("END_IF", "BOOL"), ("S", "BOOL")]),
            ("VERSION", [("a", "BOOL"), ("b", "BOOL")]),
        ] {
            let fixture = fixture(platform);
            let output = run(
                &fixture,
                Role::ProcessEngineer,
                input(DefineUdtMode::New, name, &members),
            )
            .await
            .unwrap();
            assert_eq!(output.revision, 1, "{platform:?} {name}");
        }
    }
}

#[tokio::test]
async fn adr_0203_g_a_new_revision_is_checked_like_a_new_struct() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    seed_struct(&fixture, PlcPlatform::TwinCat3, "VERSION", &["a"]);
    let request = input(
        DefineUdtMode::Revision,
        "VERSION",
        &[("a", "REAL"), ("b", "REAL")],
    );
    assert_refused(&fixture, request, DerivedTypeReason::ReservedName, "name").await;

    let fixture = self::fixture(PlcPlatform::CodesysIec);
    seed_struct(&fixture, PlcPlatform::CodesysIec, "ST_Zone", &["a"]);
    let request = input(
        DefineUdtMode::Revision,
        "ST_Zone",
        &[("a", "REAL"), ("END_IF", "REAL")],
    );
    assert_refused(
        &fixture,
        request,
        DerivedTypeReason::ReservedMemberName,
        "member",
    )
    .await;
}

#[tokio::test]
async fn adr_0203_i1_a_member_type_that_names_a_stored_reserved_struct_gives_plt_013() {
    for member_type in ["VERSION@1", "ARRAY[1..2] OF VERSION@1"] {
        let fixture = fixture(PlcPlatform::TwinCat3);
        seed_struct(&fixture, PlcPlatform::TwinCat3, "VERSION", &["a"]);
        let request = input(
            DefineUdtMode::New,
            "ST_Build",
            &[("ver", member_type), ("count", "INT")],
        );
        assert_refused(
            &fixture,
            request,
            DerivedTypeReason::ReservedName,
            member_type,
        )
        .await;
    }
}

#[tokio::test]
async fn adr_0203_i1_a_member_type_with_a_stored_keyword_member_gives_plt_013() {
    for member_type in ["ST_Coil@1", "ARRAY[1..2] OF ST_Coil@1"] {
        let fixture = fixture(PlcPlatform::CodesysIec);
        seed_struct(
            &fixture,
            PlcPlatform::CodesysIec,
            "ST_Coil",
            &["END_IF", "S"],
        );
        let request = input(
            DefineUdtMode::New,
            "ST_Outer",
            &[("coil", member_type), ("count", "INT")],
        );
        assert_refused(
            &fixture,
            request,
            DerivedTypeReason::ReservedMemberName,
            member_type,
        )
        .await;
    }
}

#[tokio::test]
async fn adr_0203_e_a_member_type_with_a_stored_reserved_struct_on_s7_creates_the_struct() {
    let fixture = fixture(PlcPlatform::S7_1500);
    seed_struct(&fixture, PlcPlatform::S7_1500, "VERSION", &["a"]);
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Build",
            &[("ver", "VERSION@1"), ("count", "INT")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}
