//! A stored type that does not resolve gives `E-DB-008` or opens the repair form
//! (design D7, ADR-0195 (z) to (ab)).

use std::collections::BTreeSet;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, DerivedTypeReason, MemberName, PlantError, PlatformTypes,
    PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape, StringEncoding, TypeMember,
    TypeName, TypeTree,
};
use recipe_domain::recipe::StoredParameter;
use serde_json::json;

use super::parameter_form_tests::{form_as, input, world};
use crate::auth::ParameterFormPurpose;
use crate::error::AppError;
use crate::memory::fixtures::{
    ARRAY_CYCLE_ROOT, Fixture, STRUCT_CYCLE_ROOT, mid, parameter_on, seed_array_cycle,
    seed_oversized_struct, seed_struct_cycle, tid, uid, vid,
};
use crate::memory::seed_type_id;
use crate::recipe::test_support::DRAFT_VERSION;
use recipe_domain::identity::Role;

const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;
const CASE_KEY: u128 = 5;
/// A struct with the member `deep`, a member of an array-only cycle.
const CYCLE_MEMBER_STRUCT: u128 = 960;
const ABSENT_ROOT: u128 = 970;
const ARRAY_WITHOUT_ELEMENT: u128 = 971;
const ABSENT_ELEMENT: u128 = 972;
const STRUCT_WITH_ABSENT_MEMBER: u128 = 973;
const ABSENT_MEMBER: u128 = 974;

fn row(id: u128, shape: RecordShape) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape,
    })
    .unwrap()
}

fn struct_with(id: u128, name: &str, member_type: u128) -> PlcType {
    row(
        id,
        RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision: 1,
            supersedes: None,
            members: vec![TypeMember {
                name: MemberName::parse("deep").unwrap(),
                type_id: tid(member_type),
            }],
        },
    )
}

/// Store a type of the case on `M1` and return the row number of its root.
type Seed = fn(&Fixture) -> u128;

fn struct_cycle(fixture: &Fixture) -> u128 {
    seed_struct_cycle(fixture, (1, PLATFORM));
    STRUCT_CYCLE_ROOT
}

fn oversized_struct(fixture: &Fixture) -> u128 {
    let id = seed_oversized_struct(fixture, PLATFORM, "ST_Wide");
    id.as_uuid().as_u128()
}

fn struct_over_array_cycle(fixture: &Fixture) -> u128 {
    seed_array_cycle(fixture, (1, PLATFORM));
    fixture.store.seed_type(&struct_with(
        CYCLE_MEMBER_STRUCT,
        "ST_Deep",
        ARRAY_CYCLE_ROOT,
    ));
    CYCLE_MEMBER_STRUCT
}

fn array_cycle(fixture: &Fixture) -> u128 {
    seed_array_cycle(fixture, (1, PLATFORM));
    ARRAY_CYCLE_ROOT
}

/// The seeded `INT` row. It resolves, so the form opens until a hook breaks the rows.
fn resolving_int(_fixture: &Fixture) -> u128 {
    let id = seed_type_id(Primitive::Int, None).unwrap();
    id.as_uuid().as_u128()
}

fn absent_root(_fixture: &Fixture) -> u128 {
    ABSENT_ROOT
}

fn array_without_element(fixture: &Fixture) -> u128 {
    let dimensions = ArrayDimensions::parse(vec![ArrayDimension::parse(1, 4).unwrap()]).unwrap();
    fixture.store.seed_type(&row(
        ARRAY_WITHOUT_ELEMENT,
        RecordShape::Array {
            element: tid(ABSENT_ELEMENT),
            dimensions,
        },
    ));
    ARRAY_WITHOUT_ELEMENT
}

fn struct_with_absent_member(fixture: &Fixture) -> u128 {
    fixture.store.seed_type(&struct_with(
        STRUCT_WITH_ABSENT_MEMBER,
        "ST_Absent",
        ABSENT_MEMBER,
    ));
    STRUCT_WITH_ABSENT_MEMBER
}

/// The seeded parameter `CASE` has the type `root`, and the rows belong to `M1`.
fn world_with_case(seed: Seed) -> (Fixture, u128) {
    let fixture = world();
    let root = seed(&fixture);
    let case = StoredParameter {
        plc_type: tid(root),
        value: json!({"x": 1}).into(),
        limits: Vec::new(),
        ..parameter_on(PLATFORM, CASE_KEY, "CASE", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(DRAFT_VERSION), &case);
    (fixture, root)
}

fn resolution(fixture: &Fixture, root: u128) -> Result<TypeTree, PlantError> {
    let rows: Vec<PlcType> = fixture
        .store
        .state()
        .types
        .values()
        .map(|row| row.plc_type.clone())
        .collect();
    let types = PlatformTypes::collect(mid(1), PLATFORM, StringEncoding::Latin1, rows).unwrap();
    TypeTree::resolve(tid(root), &types)
}

async fn run(
    fixture: &Fixture,
    purpose: ParameterFormPurpose,
    type_text: Option<&str>,
) -> Result<super::ParameterForm, AppError> {
    let request = super::GetParameterFormInput {
        purpose,
        ..input(Some(CASE_KEY), type_text, None)
    };
    form_as(fixture, Role::ProcessEngineer, request).await
}

fn is_resolution_failure(result: &Result<TypeTree, PlantError>, expected: &str) -> bool {
    match expected {
        "cycle" => matches!(result, Err(PlantError::TypeCycle { .. })),
        "large" => matches!(result, Err(PlantError::TypeTooLarge { .. })),
        "row_shape" => matches!(
            result,
            Err(PlantError::DerivedTypeInvalid {
                reason: DerivedTypeReason::RowShape,
                ..
            })
        ),
        _ => matches!(
            result,
            Err(PlantError::DerivedTypeInvalid {
                reason: DerivedTypeReason::NotOnPlatform,
                ..
            })
        ),
    }
}

/// A case with the seed, the resolution of its root, and whether the root has a type text.
const CASES: [(&str, Seed, &str, bool); 7] = [
    ("struct cycle", struct_cycle, "cycle", true),
    ("oversized struct", oversized_struct, "large", true),
    (
        "struct over an array cycle",
        struct_over_array_cycle,
        "row_shape",
        true,
    ),
    ("root array cycle", array_cycle, "row_shape", false),
    ("absent root row", absent_root, "not_on_platform", false),
    (
        "array without element row",
        array_without_element,
        "not_on_platform",
        false,
    ),
    (
        "struct with absent member",
        struct_with_absent_member,
        "not_on_platform",
        true,
    ),
];

#[tokio::test]
async fn set_parameter_value_maps_a_stored_defect_to_db_008() {
    for (name, seed, resolves_as, _) in CASES {
        let (fixture, root) = world_with_case(seed);
        assert!(
            is_resolution_failure(&resolution(&fixture, root), resolves_as),
            "{name}: {:?}",
            resolution(&fixture, root).map(|_| ())
        );

        let result = run(&fixture, ParameterFormPurpose::SetParameterValue, None).await;

        let error = result.map(|_| ()).unwrap_err();
        let expected = if resolves_as == "not_on_platform" && name == "struct with absent member" {
            ErrorCode::PLT_013
        } else {
            ErrorCode::DB_008
        };
        assert_eq!(error.code(), expected, "{name}: {error:?}");
    }
}

#[tokio::test]
async fn update_definition_opens_the_form_for_a_stored_defect() {
    for (name, seed, _, has_text) in CASES {
        let (fixture, _) = world_with_case(seed);

        let result = run(
            &fixture,
            ParameterFormPurpose::UpdateParameterDefinition,
            None,
        )
        .await;

        let opened = result
            .map_err(|error| format!("{name}: {error:?}"))
            .unwrap();
        assert!(opened.rule_faults.is_empty(), "{name}");
        assert_eq!(opened.type_text.is_some(), has_text, "{name}");
    }
}

#[tokio::test]
async fn two_type_rows_with_one_identifier_give_db_008_for_both_purposes() {
    for purpose in [
        ParameterFormPurpose::SetParameterValue,
        ParameterFormPurpose::UpdateParameterDefinition,
    ] {
        let (fixture, root) = world_with_case(resolving_int);
        let int_value = StoredParameter {
            plc_type: tid(root),
            value: json!(1).into(),
            limits: Vec::new(),
            ..parameter_on(PLATFORM, CASE_KEY, "CASE", uid(101), 0)
        };
        fixture.store.seed_parameter(vid(DRAFT_VERSION), &int_value);
        let opened = run(&fixture, purpose, None)
            .await
            .map_err(|error| format!("{purpose:?}: {error:?}"))
            .unwrap();
        assert!(opened.rule_faults.is_empty(), "{purpose:?}");
        fixture.store.duplicate_type_row_on_call(1, tid(root));

        let error = run(&fixture, purpose, None).await.map(|_| ()).unwrap_err();

        assert_eq!(error.code(), ErrorCode::DB_008, "{purpose:?}: {error:?}");
    }
}

#[tokio::test]
async fn the_repair_form_previews_the_sent_type_of_a_stored_cycle() {
    let (fixture, _) = world_with_case(struct_cycle);
    let purpose = ParameterFormPurpose::UpdateParameterDefinition;

    let int = run(&fixture, purpose, Some("INT")).await.unwrap();
    let above_limit = run(&fixture, purpose, Some("ARRAY[1..16385] OF INT")).await;

    assert!(int.preview.is_some());
    assert_eq!(
        above_limit.map(|_| ()).unwrap_err().code(),
        ErrorCode::PLT_018
    );
}
