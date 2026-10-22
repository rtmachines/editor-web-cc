//! Tests of `GetUdtForm` on `MemoryStore` (spec "Form of a new revision", design D3).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, PlcPlatform, PlcTypeId, Primitive, TypeExpr, TypeName};

use super::types::{GetUdtForm, UdtForm, UdtFormInput};
use super::world::{array_row, machine_of, string_row, struct_row};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{Fixture, actor, meta, tid};
use crate::memory::{JournalEntry, seed_type_id};
use crate::ports::Isolation;

/// The number of the machine `M12`.
const MACHINE: u128 = 12;
/// The platform of the machine `M12`.
const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;
/// The struct `ST_Zone@1`.
const ZONE_1: u128 = 901;
/// The struct `ST_Zone@2`.
const ZONE_2: u128 = 902;
/// The type `STRING(16)` of the machine.
const NAME_STRING: u128 = 950;
/// The type `ARRAY[1..4] OF STRING(16)`.
const NAME_ARRAY: u128 = 951;
/// The struct `ST_Axis@1`.
const AXIS_1: u128 = 910;

fn code(value: &str) -> MachineCode {
    MachineCode::parse(value).unwrap()
}

fn name(value: &str) -> TypeName {
    TypeName::parse(value).unwrap()
}

fn primitive(primitive: Primitive) -> PlcTypeId {
    seed_type_id(primitive, None).unwrap()
}

const fn owner() -> (u128, PlcPlatform) {
    (MACHINE, PLATFORM)
}

/// The machine `M12` with `ST_Zone@1`, `ST_Zone@2`, and `ST_Axis@1`.
///
/// `ST_Zone@2` has the members `temp_max` of the type `REAL` and `names` of the type
/// `ARRAY[1..4] OF STRING(16)`.
fn world() -> Fixture {
    let fixture = Fixture::new();
    fixture
        .store
        .seed_machine(&machine_of(PLATFORM, MACHINE, "M12", "Press"));
    let members_2 = [
        ("temp_max", primitive(Primitive::Real)),
        ("names", tid(NAME_ARRAY)),
    ];
    for row in [
        string_row(owner(), NAME_STRING, 16),
        array_row(owner(), NAME_ARRAY, NAME_STRING),
        struct_row(
            owner(),
            ZONE_1,
            ("ST_Zone", 1),
            &[("old", primitive(Primitive::Int))],
            None,
        ),
        struct_row(owner(), ZONE_2, ("ST_Zone", 2), &members_2, Some(ZONE_1)),
        struct_row(
            owner(),
            AXIS_1,
            ("ST_Axis", 1),
            &[("pos", primitive(Primitive::Int))],
            None,
        ),
    ] {
        fixture.store.seed_type(&row);
    }
    fixture
}

async fn form_as(
    fixture: &Fixture,
    role: Role,
    machine_code: &str,
    from: Option<&str>,
) -> Result<UdtForm, AppError> {
    let input = UdtFormInput {
        purpose: crate::auth::UdtFormPurpose::DefineUdt,
        code: code(machine_code),
        from: from.map(name),
    };
    GetUdtForm::new(fixture.context())
        .execute(&actor(role), &meta(), input)
        .await
}

async fn form(fixture: &Fixture, from: Option<&str>) -> Result<UdtForm, AppError> {
    form_as(fixture, Role::ProcessEngineer, "M12", from).await
}

fn member_texts(form: &UdtForm) -> Vec<(String, String)> {
    form.members
        .iter()
        .map(|(member, text)| (member.to_string(), text.clone()))
        .collect()
}

/// The proposals of the form for a machine of `platform` that has only `ST_Zone@1`.
async fn choices_on(platform: PlcPlatform) -> Vec<String> {
    let fixture = Fixture::new();
    let machine = machine_of(platform, MACHINE, "M12", "Press");
    fixture.store.seed_machine(&machine);
    let members = [("temp", primitive(Primitive::Int))];
    let zone = struct_row((MACHINE, platform), ZONE_1, ("ST_Zone", 1), &members, None);
    fixture.store.seed_type(&zone);
    form(&fixture, None).await.unwrap().choices
}

#[test]
fn the_query_declares_its_operation() {
    assert_eq!(GetUdtForm::OPERATION, Operation::GetUdtForm);
}

#[tokio::test]
async fn the_form_of_a_new_revision_holds_the_members_of_the_latest_revision() {
    let fixture = world();

    let result = form(&fixture, Some("ST_Zone")).await.unwrap();

    let expected = [
        ("temp_max".to_owned(), "REAL".to_owned()),
        ("names".to_owned(), "ARRAY[1..4] OF STRING(16)".to_owned()),
    ];
    assert_eq!(member_texts(&result), expected);
    assert_eq!(result.from, Some((name("ST_Zone"), 2)));
    assert_eq!(result.machine.code(), code("M12"));
}

#[tokio::test]
async fn the_text_of_each_member_parses_to_the_same_expression() {
    let fixture = world();

    let result = form(&fixture, Some("ST_Zone")).await.unwrap();

    for (member, text) in &result.members {
        let parsed = TypeExpr::parse(text).unwrap();
        assert_eq!(&parsed.to_string(), text, "member {member}");
    }
}

#[tokio::test]
async fn a_form_without_from_has_no_member_and_no_origin() {
    let fixture = world();

    let result = form(&fixture, None).await.unwrap();

    assert!(result.members.is_empty(), "{:?}", result.members);
    assert_eq!(result.from, None);
}

#[tokio::test]
async fn an_unknown_struct_is_not_found() {
    let fixture = world();

    let error = form(&fixture, Some("ST_Missing")).await.unwrap_err();

    let expected = Missing::Type {
        code: code("M12"),
        name: name("ST_Missing"),
    };
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(
        matches!(&error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_unknown_machine_is_not_found() {
    let fixture = world();

    let error = form_as(&fixture, Role::ProcessEngineer, "M99", None)
        .await
        .unwrap_err();

    let expected = Missing::Machine(code("M99"));
    assert!(
        matches!(&error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn the_proposals_start_with_the_primitives_then_strings_then_the_latest_structs() {
    let fixture = world();

    let result = form(&fixture, None).await.unwrap();

    let expected = [
        "BOOL",
        "INT",
        "DINT",
        "REAL",
        "LREAL",
        "TIME",
        "LTIME",
        "STRING",
        "WSTRING",
        "ST_Axis@1",
        "ST_Zone@2",
    ];
    assert_eq!(result.choices, expected);
}

#[tokio::test]
async fn proposals_leave_out_every_revision_of_the_revised_struct() {
    let fixture = world();

    let result = form(&fixture, Some("ST_Zone")).await.unwrap();

    let expected = [
        "BOOL",
        "INT",
        "DINT",
        "REAL",
        "LREAL",
        "TIME",
        "LTIME",
        "STRING",
        "WSTRING",
        "ST_Axis@1",
    ];
    assert_eq!(result.choices, expected);
}

#[tokio::test]
async fn s7_300_has_no_wstring_and_no_lreal() {
    let choices = choices_on(PlcPlatform::S7_300).await;

    let expected = ["BOOL", "INT", "DINT", "REAL", "TIME", "STRING", "ST_Zone@1"];
    assert_eq!(choices, expected);
}

#[tokio::test]
async fn s7_1500_has_wstring_and_lreal() {
    let choices = choices_on(PlcPlatform::S7_1500).await;

    let expected = [
        "BOOL",
        "INT",
        "DINT",
        "REAL",
        "LREAL",
        "TIME",
        "LTIME",
        "STRING",
        "WSTRING",
        "ST_Zone@1",
    ];
    assert_eq!(choices, expected);
}

#[tokio::test]
async fn the_proposals_hold_no_struct_of_another_machine() {
    let fixture = world();
    let other = machine_of(PLATFORM, 13, "M13", "Other");
    fixture.store.seed_machine(&other);
    let members = [("x", primitive(Primitive::Int))];
    let foreign = struct_row((13, PLATFORM), 930, ("ST_Other", 1), &members, None);
    fixture.store.seed_type(&foreign);

    let result = form(&fixture, None).await.unwrap();

    assert!(
        !result
            .choices
            .iter()
            .any(|choice| choice.contains("ST_Other")),
        "{:?}",
        result.choices
    );
}

#[tokio::test]
async fn the_form_opens_one_read_only_transaction_and_writes_nothing() {
    let fixture = world();

    form(&fixture, Some("ST_Zone")).await.unwrap();

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_operator_gets_forbidden_and_the_store_stays_unread() {
    let fixture = world();

    let error = form_as(&fixture, Role::Operator, "M12", Some("ST_Zone"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(fixture.store.journal().is_empty());
}
