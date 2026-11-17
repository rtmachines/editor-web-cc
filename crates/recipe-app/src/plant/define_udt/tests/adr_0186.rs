//! Tests of the rule "Siemens reserved struct names on S7" in `DefineUdt` (ADR-0186).

use std::collections::BTreeSet;

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    DerivedTypeReason, MemberName, PlantError, PlcPlatform, PlcType, PlcTypeRecord, RecordShape,
    TypeMember, TypeName,
};

use super::{DefineUdtMode, fixture, input, real, run};
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, tid};

const S7_PLATFORMS: [PlcPlatform; 3] = [
    PlcPlatform::S7_300,
    PlcPlatform::S7_1200,
    PlcPlatform::S7_1500,
];

/// Assert that the error is `E-PLT-013` with the reason "reserved name".
fn assert_reserved_name(error: &AppError) {
    assert!(
        matches!(
            error,
            AppError::Plant(PlantError::DerivedTypeInvalid {
                reason: DerivedTypeReason::ReservedName,
                ..
            })
        ),
        "{error:?}"
    );
}

/// Store the struct `name@1` with the member `a` on machine 1 for `platform`.
fn seed_stored_struct(fixture: &Fixture, platform: PlcPlatform, name: &str) {
    let record = PlcTypeRecord {
        id: tid(960),
        machine: Some(crate::memory::fixtures::mid(1)),
        platforms: BTreeSet::from([platform]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision: 1,
            supersedes: None,
            members: vec![TypeMember {
                name: MemberName::parse("a").unwrap(),
                type_id: real(),
            }],
        },
    };
    fixture.store.seed_type(&PlcType::try_from(record).unwrap());
}

#[tokio::test]
async fn adr_0186_a_new_struct_with_a_siemens_name_on_s7_gives_plt_013_and_creates_no_type() {
    for platform in S7_PLATFORMS {
        for name in ["PID_Compact", "pid_compact", "TON_TIME", "PD_TEL3"] {
            let fixture = fixture(platform);
            let before = fixture.store.state().types.len();
            let error = run(
                &fixture,
                Role::ProcessEngineer,
                input(DefineUdtMode::New, name, &[("a", "REAL")]),
            )
            .await
            .unwrap_err();
            assert_reserved_name(&error);
            assert_eq!(fixture.store.state().types.len(), before, "{platform:?}");
            assert!(fixture.store.audit_entries().is_empty());
        }
    }
}

#[tokio::test]
async fn adr_0186_a_new_revision_with_a_siemens_name_on_s7_gives_plt_013() {
    let fixture = fixture(PlcPlatform::S7_1500);
    seed_stored_struct(&fixture, PlcPlatform::S7_1500, "PID_Compact");
    let before = fixture.store.state().types.len();
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::Revision,
            "PID_Compact",
            &[("a", "REAL"), ("b", "REAL")],
        ),
    )
    .await
    .unwrap_err();
    assert_reserved_name(&error);
    assert_eq!(fixture.store.state().types.len(), before);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn adr_0186_a_member_name_with_a_siemens_name_on_s7_succeeds() {
    let fixture = fixture(PlcPlatform::S7_1500);
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Clock", &[("TIMER", "INT")]),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}

#[tokio::test]
async fn adr_0186_a_member_of_a_stored_struct_with_a_siemens_name_on_s7_gives_plt_013() {
    let fixture = fixture(PlcPlatform::S7_1500);
    seed_stored_struct(&fixture, PlcPlatform::S7_1500, "PID_Compact");
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Outer", &[("pid", "PID_Compact@1")]),
    )
    .await
    .unwrap_err();
    assert_reserved_name(&error);
}

#[tokio::test]
async fn adr_0186_a_siemens_name_on_codesys_iec_creates_the_struct() {
    for name in ["PID_Compact", "PD_TEL3"] {
        let fixture = fixture(PlcPlatform::CodesysIec);
        let output = run(
            &fixture,
            Role::ProcessEngineer,
            input(DefineUdtMode::New, name, &[("a", "REAL")]),
        )
        .await
        .unwrap();
        assert_eq!(output.revision, 1);
    }
}
