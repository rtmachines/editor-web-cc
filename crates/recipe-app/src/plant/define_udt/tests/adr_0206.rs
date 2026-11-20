//! Tests of the rule "Vendor aliases of type names" in `DefineUdt` (ADR-0206).

use recipe_domain::plant::{PlcPlatform, Primitive};

use super::{DefineUdtMode, fixture, input, own_rows, run, struct_parts};
use crate::memory::fixtures::Fixture;
use crate::memory::seed_type_id;
use recipe_domain::identity::Role;

/// A machine 1 on `platform` that has the rows of the time primitives.
fn time_fixture(platform: PlcPlatform) -> Fixture {
    let fixture = fixture(platform);
    fixture.store.seed_time_primitive_types();
    fixture
}

#[tokio::test]
async fn adr_0206_alias_member_types_store_the_catalog_names() {
    let fixture = time_fixture(PlcPlatform::TwinCat3);
    let request = input(
        DefineUdtMode::New,
        "ST_Shift",
        &[("start", "LTIME_OF_DAY"), ("stamp", "LDATE_AND_TIME")],
    );

    let output = run(&fixture, Role::ProcessEngineer, request).await.unwrap();

    assert_eq!(output.revision, 1);
    let rows = own_rows(&fixture);
    let row = rows.first().unwrap();
    assert_eq!(rows.len(), 1);
    let (name, revision, _, members) = struct_parts(row);
    assert_eq!((name.as_str(), revision), ("ST_Shift", 1));
    let types: Vec<_> = members.iter().map(|member| member.type_id).collect();
    assert_eq!(
        types,
        [
            seed_type_id(Primitive::Ltod, None).unwrap(),
            seed_type_id(Primitive::Ldt, None).unwrap(),
        ]
    );
}

#[tokio::test]
async fn adr_0206_the_short_aliases_as_member_types_store_the_catalog_names() {
    let fixture = time_fixture(PlcPlatform::S7_1500);
    let request = input(
        DefineUdtMode::New,
        "ST_Clock",
        &[("start", "TOD"), ("stamp", "DT")],
    );

    run(&fixture, Role::ProcessEngineer, request).await.unwrap();

    let rows = own_rows(&fixture);
    let (_, _, _, members) = struct_parts(rows.first().unwrap());
    let types: Vec<_> = members.iter().map(|member| member.type_id).collect();
    assert_eq!(
        types,
        [
            seed_type_id(Primitive::TimeOfDay, None).unwrap(),
            seed_type_id(Primitive::DateAndTime, None).unwrap(),
        ]
    );
}
