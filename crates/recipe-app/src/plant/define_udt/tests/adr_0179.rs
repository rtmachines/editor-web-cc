//! Tests of the rule "nesting limit of a type on S7" in `DefineUdt` (ADR-0179).

use recipe_domain::identity::Role;
use recipe_domain::plant::{PlantError, PlcPlatform, TypeLimit};

use super::{DefineUdtMode, fixture, input, run};
use crate::error::AppError;
use crate::memory::fixtures::seed_struct_chain;

/// Assert that the error is `E-PLT-018` with the S7 nesting limit for `type_name`.
fn assert_nesting_refused(error: &AppError, type_name: &str) {
    assert!(
        matches!(
            error,
            AppError::Plant(PlantError::TypeTooLarge { type_name: named, limit: TypeLimit::S7Nesting })
                if named == type_name
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn adr_0179_a_struct_with_nine_levels_on_s7_gives_plt_018_and_creates_no_type() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        let fixture = fixture(platform);
        seed_struct_chain(&fixture, platform, ("ST_Deep", 8));
        let before = fixture.store.state().types.len();
        let error = run(
            &fixture,
            Role::ProcessEngineer,
            input(DefineUdtMode::New, "ST_Top", &[("inner", "ST_Deep@1")]),
        )
        .await
        .unwrap_err();
        assert_nesting_refused(&error, "ST_Top@1");
        assert_eq!(fixture.store.state().types.len(), before, "{platform:?}");
        assert!(fixture.store.audit_entries().is_empty());
    }
}

#[tokio::test]
async fn adr_0179_a_struct_with_eight_levels_on_s7_succeeds() {
    let fixture = fixture(PlcPlatform::S7_1500);
    seed_struct_chain(&fixture, PlcPlatform::S7_1500, ("ST_Deep", 7));
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Top", &[("inner", "ST_Deep@1")]),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}

#[tokio::test]
async fn adr_0179_a_struct_with_nine_levels_on_twincat_succeeds() {
    let fixture = fixture(PlcPlatform::TwinCat3);
    seed_struct_chain(&fixture, PlcPlatform::TwinCat3, ("ST_Deep", 8));
    let output = run(
        &fixture,
        Role::ProcessEngineer,
        input(DefineUdtMode::New, "ST_Top", &[("inner", "ST_Deep@1")]),
    )
    .await
    .unwrap();
    assert_eq!(output.revision, 1);
}
