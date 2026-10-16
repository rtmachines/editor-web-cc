//! Tests of `GetTypeUsage` on `MemoryStore` (spec "Type usage", design D3).

use std::collections::BTreeSet;

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    ArrayDimension, ArrayDimensions, MachineCode, MemberName, PlcPlatform, PlcType, PlcTypeId,
    PlcTypeRecord, RecordShape, TypeMember, TypeName,
};
use recipe_domain::recipe::{
    ParameterTag, RecipeDeleter, ReferenceNumber, StoredParameter, VersionState, VersionStatus,
};

use super::TypeRef;
use super::usage::{GetTypeUsage, TypeUsage, TypeUsageInput, TypeUsageLine};
use crate::blocking::{AppSettings, PageSizes};
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, parameter_on, recipe_with, tid, timestamp, uid, user, vid,
};
use crate::memory::seed_type_id;
use crate::ports::TypeUsageRow;
use recipe_domain::plant::Primitive;

/// The platform of the machine `M12`.
const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;
/// The number of the machine `M12` in the fixtures.
const MACHINE: u128 = 12;
/// The type `ST_Zone@1`.
const ZONE_1: u128 = 901;
/// The type `ST_Zone@2`.
const ZONE_2: u128 = 902;
/// The type `ARRAY[1..4] OF ST_Zone@2`.
const ZONES: u128 = 903;
/// The type `ST_Line@1`, with the member `zones` of the type `ZONES`.
const LINE: u128 = 904;
/// The type `ST_Pair@1`, with the members `a` and `b` of the type `ZONE_2`.
const PAIR: u128 = 905;

fn struct_row(id: u128, name: &str, revision: u32, members: &[(&str, PlcTypeId)]) -> PlcType {
    let members = members
        .iter()
        .map(|(member, type_id)| TypeMember {
            name: MemberName::parse(member).unwrap(),
            type_id: *type_id,
        })
        .collect();
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(MACHINE)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision,
            supersedes: None,
            members,
        },
    })
    .unwrap()
}

fn array_row(id: u128, element: u128) -> PlcType {
    let dimensions = vec![ArrayDimension::parse(1, 4).unwrap()];
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(MACHINE)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Array {
            element: tid(element),
            dimensions: ArrayDimensions::parse(dimensions).unwrap(),
        },
    })
    .unwrap()
}

/// The machine `M12` with the types of the constants above.
fn world() -> Fixture {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PLATFORM, MACHINE, "M12");
    let temp = [("temp", seed_type_id(Primitive::Int, None).unwrap())];
    let zone_2 = tid(ZONE_2);
    for row in [
        struct_row(ZONE_1, "ST_Zone", 1, &temp),
        struct_row(ZONE_2, "ST_Zone", 2, &temp),
        array_row(ZONES, ZONE_2),
        struct_row(LINE, "ST_Line", 1, &[("zones", tid(ZONES))]),
        struct_row(PAIR, "ST_Pair", 1, &[("a", zone_2), ("b", zone_2)]),
    ] {
        fixture.store.seed_type(&row);
    }
    fixture
}

/// Store the recipe `n` of `M12` with the versions `(identifier, state)`, numbered from 1.
fn seed_recipe(fixture: &Fixture, n: u128, reference: &str, versions: &[(u128, VersionState)]) {
    let recipe = recipe_with((n, MACHINE), reference, versions);
    fixture.store.seed_recipe(&recipe, PLATFORM, user(2));
}

/// Store a recipe of `M12` that an administrator deleted. The versions keep their state.
fn seed_deleted_recipe(fixture: &Fixture, n: u128, reference: &str, version: (u128, VersionState)) {
    let mut recipe = recipe_with((n, MACHINE), reference, &[version]);
    recipe
        .delete(RecipeDeleter::Administrator, timestamp())
        .unwrap();
    fixture.store.seed_recipe(&recipe, PLATFORM, user(2));
}

/// Store the parameter `(number, tag)` of the type `plc_type` in the version `version`.
fn seed_parameter(fixture: &Fixture, version: u128, (n, tag): (u128, &str), plc_type: u128) {
    let parameter = StoredParameter {
        plc_type: tid(plc_type),
        ..parameter_on(PLATFORM, n, tag, uid(1201), 0)
    };
    fixture.store.seed_parameter(vid(version), &parameter);
}

fn input(name: &str, revision: Option<u32>, page: u32) -> TypeUsageInput {
    TypeUsageInput {
        type_ref: TypeRef {
            code: MachineCode::parse("M12").unwrap(),
            name: TypeName::parse(name).unwrap(),
            revision,
        },
        page,
    }
}

async fn run(
    fixture: &Fixture,
    settings: AppSettings,
    input: TypeUsageInput,
) -> Result<TypeUsage, AppError> {
    GetTypeUsage::new(fixture.context_with(settings))
        .execute(&actor(Role::Operator), &meta(), input)
        .await
}

/// The settings with `size` rows on a page of the usage list.
fn pages_of(size: u32) -> AppSettings {
    AppSettings {
        pages: PageSizes {
            machines: 100,
            type_usage: size,
            recipes: 100,
            versions: 100,
            parameters: 100,
        },
        ..AppSettings::default()
    }
}

/// The line that a test expects.
fn line(
    (reference, number, status): (&str, u32, VersionStatus),
    tag: &str,
    plc_type: u128,
    type_text: &str,
) -> TypeUsageLine {
    TypeUsageLine {
        row: TypeUsageRow {
            reference: ReferenceNumber::parse(reference).unwrap(),
            number,
            status,
            tag: ParameterTag::parse(tag).unwrap(),
            parameter_type: tid(plc_type),
        },
        type_text: type_text.to_owned(),
    }
}

fn tags(usage: &TypeUsage) -> Vec<String> {
    usage
        .rows
        .iter()
        .map(|row| row.row.tag.to_string())
        .collect()
}

#[tokio::test]
async fn a_use_through_another_struct_gives_the_row_with_the_text_of_its_type() {
    let fixture = world();
    let versions = [
        (10_001, VersionState::Retired),
        (10_002, VersionState::Approved),
        (10_003, VersionState::LiveDraft),
    ];
    seed_recipe(&fixture, 100, "R-100", &versions);
    seed_parameter(&fixture, 10_003, (1, "LINE"), LINE);
    seed_parameter(&fixture, 10_003, (2, "ZONES"), ZONES);

    let usage = run(
        &fixture,
        AppSettings::default(),
        input("ST_Zone", Some(2), 0),
    )
    .await
    .unwrap();

    let draft = ("R-100", 3, VersionStatus::Draft);
    assert_eq!(
        usage.rows,
        [
            line(draft, "LINE", LINE, "ST_Line@1"),
            line(draft, "ZONES", ZONES, "ARRAY[1..4] OF ST_Zone@2"),
        ]
    );
    assert_eq!(usage.name.as_str(), "ST_Zone");
    assert_eq!(usage.revision, 2);
    assert_eq!(usage.machine.code().to_string(), "M12");
    assert_eq!((usage.page, usage.has_next), (0, false));
}

#[tokio::test]
async fn no_revision_means_the_latest_revision() {
    let fixture = world();
    seed_recipe(&fixture, 100, "R-100", &[(10_001, VersionState::LiveDraft)]);
    seed_parameter(&fixture, 10_001, (1, "LINE"), LINE);

    let latest = run(&fixture, AppSettings::default(), input("ST_Zone", None, 0))
        .await
        .unwrap();
    let first = run(
        &fixture,
        AppSettings::default(),
        input("ST_Zone", Some(1), 0),
    )
    .await
    .unwrap();

    assert_eq!(latest.revision, 2);
    assert_eq!(tags(&latest), ["LINE"]);
    assert_eq!(first.revision, 1);
    assert!(first.rows.is_empty(), "{:?}", first.rows);
}

#[tokio::test]
async fn two_members_of_one_struct_give_one_row() {
    let fixture = world();
    seed_recipe(&fixture, 400, "R-400", &[(40_001, VersionState::LiveDraft)]);
    seed_parameter(&fixture, 40_001, (1, "PAIR"), PAIR);

    let usage = run(
        &fixture,
        AppSettings::default(),
        input("ST_Zone", Some(2), 0),
    )
    .await
    .unwrap();

    let draft = ("R-400", 1, VersionStatus::Draft);
    assert_eq!(usage.rows, [line(draft, "PAIR", PAIR, "ST_Pair@1")]);
}

#[tokio::test]
async fn a_deleted_recipe_and_a_discarded_draft_give_no_row() {
    let fixture = world();
    seed_deleted_recipe(&fixture, 200, "R-200", (20_001, VersionState::Retired));
    seed_parameter(&fixture, 20_001, (1, "OLD"), ZONE_2);
    seed_recipe(&fixture, 300, "R-300", &[(30_001, VersionState::Discarded)]);
    seed_parameter(&fixture, 30_001, (2, "GONE"), ZONE_2);
    seed_recipe(&fixture, 400, "R-400", &[(40_001, VersionState::LiveDraft)]);
    seed_parameter(&fixture, 40_001, (3, "LIVE"), ZONE_2);

    let usage = run(
        &fixture,
        AppSettings::default(),
        input("ST_Zone", Some(2), 0),
    )
    .await
    .unwrap();

    let draft = ("R-400", 1, VersionStatus::Draft);
    assert_eq!(usage.rows, [line(draft, "LIVE", ZONE_2, "ST_Zone@2")]);
}

#[tokio::test]
async fn a_revision_without_use_gives_an_empty_list() {
    let fixture = world();

    let usage = run(
        &fixture,
        AppSettings::default(),
        input("ST_Zone", Some(1), 0),
    )
    .await
    .unwrap();

    assert!(usage.rows.is_empty(), "{:?}", usage.rows);
    assert!(!usage.has_next);
}

#[tokio::test]
async fn a_page_of_two_rows_has_a_next_page_until_the_last_row() {
    let fixture = world();
    seed_recipe(&fixture, 100, "R-100", &[(10_001, VersionState::LiveDraft)]);
    for (n, tag) in [(1, "TAG_A"), (2, "TAG_B"), (3, "TAG_C")] {
        seed_parameter(&fixture, 10_001, (n, tag), ZONE_2);
    }

    let first = run(&fixture, pages_of(2), input("ST_Zone", Some(2), 0))
        .await
        .unwrap();
    let second = run(&fixture, pages_of(2), input("ST_Zone", Some(2), 1))
        .await
        .unwrap();
    let third = run(&fixture, pages_of(2), input("ST_Zone", Some(2), 2))
        .await
        .unwrap();

    assert_eq!(tags(&first), ["TAG_A", "TAG_B"]);
    assert!(first.has_next);
    assert_eq!(tags(&second), ["TAG_C"]);
    assert!(!second.has_next);
    assert_eq!((second.page, third.page), (1, 2));
    assert!(third.rows.is_empty(), "{:?}", third.rows);
    assert!(!third.has_next);
}

#[tokio::test]
async fn a_full_last_page_has_no_next_page() {
    let fixture = world();
    seed_recipe(&fixture, 100, "R-100", &[(10_001, VersionState::LiveDraft)]);
    for (n, tag) in [(1, "TAG_A"), (2, "TAG_B")] {
        seed_parameter(&fixture, 10_001, (n, tag), ZONE_2);
    }

    let usage = run(&fixture, pages_of(2), input("ST_Zone", Some(2), 0))
        .await
        .unwrap();

    assert_eq!(tags(&usage), ["TAG_A", "TAG_B"]);
    assert!(!usage.has_next);
}

#[tokio::test]
async fn an_unknown_struct_name_is_not_found() {
    let fixture = world();

    let error = run(&fixture, AppSettings::default(), input("ST_Axis", None, 0))
        .await
        .unwrap_err();

    let expected = Missing::Type {
        code: MachineCode::parse("M12").unwrap(),
        name: TypeName::parse("ST_Axis").unwrap(),
    };
    assert!(
        matches!(&error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_unknown_revision_is_not_found_with_the_revision() {
    let fixture = world();

    let error = run(
        &fixture,
        AppSettings::default(),
        input("ST_Zone", Some(9), 0),
    )
    .await
    .unwrap_err();

    let expected = Missing::TypeRevision {
        code: MachineCode::parse("M12").unwrap(),
        name: TypeName::parse("ST_Zone").unwrap(),
        revision: 9,
    };
    assert!(
        matches!(&error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_unknown_machine_is_not_found() {
    let fixture = world();
    let missing = TypeUsageInput {
        type_ref: TypeRef {
            code: MachineCode::parse("M99").unwrap(),
            name: TypeName::parse("ST_Zone").unwrap(),
            revision: None,
        },
        page: 0,
    };

    let error = run(&fixture, AppSettings::default(), missing)
        .await
        .unwrap_err();

    let expected = Missing::Machine(MachineCode::parse("M99").unwrap());
    assert!(
        matches!(&error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_page_size_that_cannot_add_one_is_a_storage_error() {
    let fixture = world();

    let error = run(&fixture, pages_of(u32::MAX), input("ST_Zone", None, 0))
        .await
        .unwrap_err();

    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
}
