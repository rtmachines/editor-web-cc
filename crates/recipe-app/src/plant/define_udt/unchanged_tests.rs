//! Tests of the refusal of an unchanged revision (ADR-0159, `E-PLT-052`).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{PlantError, PlcPlatform, Primitive};

use super::DefineUdtMode;
use super::tests::{fixture, input, own_rows, real, run, stored_struct};
use crate::error::AppError;
use crate::memory::fixtures::Fixture;
use crate::memory::seed_type_id;

/// A fixture with `ST_Zone` revision 1 (older) and revision 2 (latest, members `a: REAL`, `b: INT`).
fn fixture_with_latest_zone() -> Fixture {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let int = seed_type_id(Primitive::Int, None).unwrap();
    fixture.store.seed_type(&stored_struct(
        901,
        ("ST_Zone", 1),
        None,
        &[("old", real())],
    ));
    fixture.store.seed_type(&stored_struct(
        902,
        ("ST_Zone", 2),
        Some(901),
        &[("a", real()), ("b", int)],
    ));
    fixture
}

#[tokio::test]
async fn a_revision_with_the_members_of_the_latest_revision_gives_plt_052_and_writes_nothing() {
    let fixture = fixture_with_latest_zone();
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::Revision,
            "ST_Zone",
            &[("a", "REAL"), ("b", "INT")],
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_052);
    assert!(matches!(
        &error,
        AppError::Plant(PlantError::RevisionUnchanged { type_name, revision: 2 })
            if type_name.as_str() == "ST_Zone"
    ));
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(own_rows(&fixture).len(), 2);
}

#[tokio::test]
async fn an_unchanged_revision_with_an_array_member_creates_no_type_row() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let first = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::New,
            "ST_Zone",
            &[("temps", "ARRAY[1..2] OF REAL")],
        ),
    )
    .await
    .unwrap();
    assert_eq!(first.revision, 1);
    let before = own_rows(&fixture).len();
    let entries = fixture.store.audit_entries().len();
    let error = run(
        &fixture,
        Role::ProcessEngineer,
        input(
            DefineUdtMode::Revision,
            "ST_Zone",
            &[("temps", "ARRAY[1..2] OF REAL")],
        ),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_052);
    assert_eq!(own_rows(&fixture).len(), before);
    assert_eq!(fixture.store.audit_entries().len(), entries);
}

async fn assert_next_revision(members: &[(&str, &str)]) {
    let fixture = fixture_with_latest_zone();
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::Revision, "ST_Zone", members),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 3);
}

#[tokio::test]
async fn a_revision_with_one_changed_member_type_gives_the_next_revision() {
    assert_next_revision(&[("a", "REAL"), ("b", "DINT")]).await;
}

#[tokio::test]
async fn a_revision_with_one_changed_letter_case_of_a_member_name_gives_the_next_revision() {
    assert_next_revision(&[("a", "REAL"), ("B", "INT")]).await;
}

#[tokio::test]
async fn a_revision_with_one_more_member_gives_the_next_revision() {
    assert_next_revision(&[("a", "REAL"), ("b", "INT"), ("c", "REAL")]).await;
}

#[tokio::test]
async fn a_revision_with_one_member_less_gives_the_next_revision() {
    assert_next_revision(&[("a", "REAL")]).await;
}

#[tokio::test]
async fn a_revision_with_another_order_of_the_members_gives_the_next_revision() {
    assert_next_revision(&[("b", "INT"), ("a", "REAL")]).await;
}

#[tokio::test]
async fn a_new_struct_is_not_compared_with_any_revision() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Zone", &[("a", "REAL")]),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}
