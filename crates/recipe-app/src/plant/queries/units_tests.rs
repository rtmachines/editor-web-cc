//! Tests of `GetUnitTree`, `GetUnitForm`, and `GetUnit` on `MemoryStore`.
//!
//! The specs are "Unit tree of a machine" and "Proposal of a new unit" (design D3).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{Machine, MachineCode, PlantError, Unit, UnitPath};

use super::units::{
    GetUnit, GetUnitForm, GetUnitTree, UnitForm, UnitFormInput, UnitRef, UnitTreeInput, UnitView,
};
use super::world::{archived, machine_named, with_archived_unit, with_units};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::JournalEntry;
use crate::memory::fixtures::{Fixture, actor, meta, timestamp};
use crate::ports::Isolation;

/// The paths of the live units of `M7` in the spec "Tree of five levels".
const FIVE_LEVELS: [&str; 4] = ["U1.2", "U1.2.4", "U1.2.4.5", "U1.2.4.5.6"];

fn code(value: &str) -> MachineCode {
    MachineCode::parse(value).unwrap()
}

fn path(value: &str) -> UnitPath {
    UnitPath::parse(value).unwrap()
}

/// The live machine `M7` with the live units of five levels.
fn deep_machine() -> Machine {
    with_units(machine_named(7, "M7", "Press"), &FIVE_LEVELS)
}

/// A fixture that holds `machine`.
fn world(machine: &Machine) -> Fixture {
    let fixture = Fixture::new();
    fixture.store.seed_machine(machine);
    fixture
}

fn paths<'a>(units: impl IntoIterator<Item = &'a Unit>) -> Vec<String> {
    units
        .into_iter()
        .map(|unit| unit.path().to_string())
        .collect()
}

async fn tree(fixture: &Fixture, machine_code: &str) -> Result<Machine, AppError> {
    let input = UnitTreeInput {
        code: code(machine_code),
    };
    GetUnitTree::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input)
        .await
}

async fn form_as(
    fixture: &Fixture,
    role: Role,
    machine_code: &str,
    parent: Option<&str>,
) -> Result<UnitForm, AppError> {
    let input = UnitFormInput {
        code: code(machine_code),
        parent: parent.map(path),
    };
    GetUnitForm::new(fixture.context())
        .execute(&actor(role), &meta(), input)
        .await
}

async fn form(
    fixture: &Fixture,
    machine_code: &str,
    parent: Option<&str>,
) -> Result<UnitForm, AppError> {
    form_as(fixture, Role::ProcessEngineer, machine_code, parent).await
}

async fn unit(
    fixture: &Fixture,
    machine_code: &str,
    unit_path: &str,
) -> Result<UnitView, AppError> {
    let input = UnitRef {
        code: code(machine_code),
        path: path(unit_path),
    };
    GetUnit::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input)
        .await
}

fn assert_machine_missing(error: &AppError, machine_code: &str) {
    let expected = Missing::Machine(code(machine_code));
    assert!(
        matches!(error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

fn assert_unit_missing(error: &AppError, machine_code: &str, unit_path: &str) {
    let expected = Missing::Unit {
        code: code(machine_code),
        path: path(unit_path),
    };
    assert!(
        matches!(error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[test]
fn the_queries_declare_their_operations() {
    assert_eq!(GetUnitTree::OPERATION, Operation::GetUnitTree);
    assert_eq!(GetUnitForm::OPERATION, Operation::GetUnitForm);
    assert_eq!(GetUnit::OPERATION, Operation::GetUnit);
}

#[tokio::test]
async fn a_tree_of_five_levels_holds_each_unit_under_its_parent() {
    let fixture = world(&deep_machine());

    let machine = tree(&fixture, "M7").await.unwrap();

    let expected = ["U1", "U1.2", "U1.2.4", "U1.2.4.5", "U1.2.4.5.6"];
    assert_eq!(paths(machine.units()), expected);
    for unit in machine.units() {
        let has_parent = unit
            .path()
            .parent()
            .is_none_or(|parent| machine.unit(&parent).is_some());
        assert!(has_parent, "{}", unit.path());
    }
}

#[tokio::test]
async fn the_tree_keeps_an_archived_unit_marked_for_the_web_layer() {
    let fixture = world(&with_archived_unit(deep_machine(), "U1.2.4.5"));

    let machine = tree(&fixture, "M7").await.unwrap();

    let live = machine.units().filter(|unit| unit.is_live());
    assert_eq!(paths(live), ["U1", "U1.2", "U1.2.4"]);
    let archived_unit = machine.unit(&path("U1.2.4.5.6")).unwrap();
    assert_eq!(archived_unit.archived_at(), Some(timestamp()));
}

#[tokio::test]
async fn an_unknown_machine_code_is_not_found_for_the_tree() {
    let fixture = world(&deep_machine());

    let error = tree(&fixture, "M99").await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert_machine_missing(&error, "M99");
}

#[tokio::test]
async fn the_code_of_an_archived_machine_is_not_found_for_the_tree() {
    let fixture = world(&archived(machine_named(12, "M12", "Old press")));

    let error = tree(&fixture, "M12").await.unwrap_err();

    assert_machine_missing(&error, "M12");
}

#[tokio::test]
async fn the_tree_opens_one_read_only_transaction_and_writes_nothing() {
    let fixture = world(&deep_machine());

    tree(&fixture, "M7").await.unwrap();

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn the_form_proposes_the_segment_after_an_archived_unit() {
    let machine = with_units(machine_named(7, "M7", "Press"), &["U1.1", "U1.2"]);
    let fixture = world(&with_archived_unit(machine, "U1.2"));

    let proposal = form(&fixture, "M7", Some("U1")).await.unwrap();

    assert_eq!(proposal.segment, Some(3));
    assert_eq!(proposal.parent, Some(path("U1")));
}

#[tokio::test]
async fn the_form_proposes_the_lowest_free_segment_in_a_gap() {
    let machine = with_units(machine_named(7, "M7", "Press"), &["U1.1", "U1.3"]);
    let fixture = world(&machine);

    let proposal = form(&fixture, "M7", Some("U1")).await.unwrap();

    assert_eq!(proposal.segment, Some(2));
}

#[tokio::test]
async fn the_form_proposes_no_segment_when_64_units_are_in_use() {
    let owned: Vec<String> = (1..=64).map(|n| format!("U1.{n}")).collect();
    let all: Vec<&str> = owned.iter().map(String::as_str).collect();
    let fixture = world(&with_units(machine_named(7, "M7", "Press"), &all));

    let proposal = form(&fixture, "M7", Some("U1")).await.unwrap();

    assert_eq!(proposal.segment, None);
    assert_eq!(proposal.parent, Some(path("U1")));
}

#[tokio::test]
async fn without_a_parent_the_form_proposes_a_main_unit() {
    let machine = with_units(machine_named(7, "M7", "Press"), &["U3", "U1.1"]);
    let fixture = world(&machine);

    let proposal = form(&fixture, "M7", None).await.unwrap();

    assert_eq!(proposal.parent, None);
    assert_eq!(proposal.segment, Some(2));
}

#[tokio::test]
async fn the_proposal_ignores_the_children_of_other_parents() {
    let machine = with_units(
        machine_named(7, "M7", "Press"),
        &["U1.1", "U1.1.1", "U1.1.2"],
    );
    let fixture = world(&machine);

    let proposal = form(&fixture, "M7", Some("U1")).await.unwrap();

    assert_eq!(proposal.segment, Some(2));
}

#[tokio::test]
async fn the_parents_are_the_live_units_of_depth_one_to_four_in_path_order() {
    let machine = with_units(
        deep_machine(),
        &["U1.10", "U2", "U1.2.4.5.7", "U1.3", "U1.3.1"],
    );
    let fixture = world(&with_archived_unit(machine, "U1.3.1"));

    let proposal = form(&fixture, "M7", None).await.unwrap();

    let expected = ["U1", "U1.2", "U1.2.4", "U1.2.4.5", "U1.3", "U1.10", "U2"];
    assert_eq!(paths(&proposal.parents), expected);
    assert_eq!(proposal.machine.code(), code("M7"));
}

#[tokio::test]
async fn a_live_parent_of_depth_five_gives_the_depth_error_with_the_first_child_path() {
    let fixture = world(&deep_machine());

    let error = form(&fixture, "M7", Some("U1.2.4.5.6")).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::PLT_002);
    assert!(
        matches!(
            &error,
            AppError::Plant(PlantError::UnitDepthExceeded { path }) if path == "U1.2.4.5.6.1"
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_archived_parent_is_not_found() {
    let fixture = world(&with_archived_unit(deep_machine(), "U1.2.4"));

    let error = form(&fixture, "M7", Some("U1.2.4")).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert_unit_missing(&error, "M7", "U1.2.4");
}

#[tokio::test]
async fn an_archived_parent_of_depth_five_is_not_found_and_not_a_depth_error() {
    let fixture = world(&with_archived_unit(deep_machine(), "U1.2.4.5.6"));

    let error = form(&fixture, "M7", Some("U1.2.4.5.6")).await.unwrap_err();

    assert_unit_missing(&error, "M7", "U1.2.4.5.6");
}

#[tokio::test]
async fn an_unknown_parent_is_not_found() {
    let fixture = world(&deep_machine());

    let error = form(&fixture, "M7", Some("U1.9")).await.unwrap_err();

    assert_unit_missing(&error, "M7", "U1.9");
}

#[tokio::test]
async fn an_unknown_machine_code_is_not_found_for_the_form() {
    let fixture = world(&deep_machine());

    let error = form(&fixture, "M99", None).await.unwrap_err();

    assert_machine_missing(&error, "M99");
}

#[tokio::test]
async fn an_operator_gets_forbidden_from_the_form_and_the_store_stays_unread() {
    let fixture = world(&deep_machine());

    let error = form_as(&fixture, Role::Operator, "M7", Some("U1"))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(matches!(
        error,
        AppError::Forbidden {
            operation: Operation::GetUnitForm
        }
    ));
    assert!(fixture.store.journal().is_empty());
}

#[tokio::test]
async fn the_form_opens_one_read_only_transaction_and_writes_nothing() {
    let fixture = world(&deep_machine());

    form(&fixture, "M7", Some("U1")).await.unwrap();

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}

/// `M7` with `U1.2` that has the live children `U1.2.4` and `U1.2.9`, one archived child `U1.2.7`,
/// and the live unit `U1.2.4.5` under `U1.2.4`.
fn branching_machine() -> Machine {
    let units = with_units(
        machine_named(7, "M7", "Press"),
        &["U1.2", "U1.2.4", "U1.2.4.5", "U1.2.7", "U1.2.9"],
    );
    with_archived_unit(units, "U1.2.7")
}

#[tokio::test]
async fn a_unit_has_its_parent_its_live_children_and_the_count_of_live_descendants() {
    let fixture = world(&branching_machine());

    let view = unit(&fixture, "M7", "U1.2").await.unwrap();

    assert_eq!(view.unit.path(), &path("U1.2"));
    assert_eq!(view.parent.as_ref().map(Unit::path), Some(&path("U1")));
    assert_eq!(paths(&view.children), ["U1.2.4", "U1.2.9"]);
    assert_eq!(view.descendants, 3);
    assert_eq!(view.machine.code(), code("M7"));
}

#[tokio::test]
async fn a_main_unit_has_no_parent() {
    let fixture = world(&branching_machine());

    let view = unit(&fixture, "M7", "U1").await.unwrap();

    assert_eq!(view.parent, None);
    assert_eq!(paths(&view.children), ["U1.2"]);
    assert_eq!(view.descendants, 4);
}

#[tokio::test]
async fn an_archived_unit_is_found_with_no_live_children() {
    let fixture = world(&with_archived_unit(branching_machine(), "U1.2.4"));

    let view = unit(&fixture, "M7", "U1.2.4").await.unwrap();

    assert_eq!(view.unit.archived_at(), Some(timestamp()));
    assert_eq!(view.parent.as_ref().map(Unit::path), Some(&path("U1.2")));
    assert!(view.children.is_empty(), "{:?}", view.children);
    assert_eq!(view.descendants, 0);
}

#[tokio::test]
async fn an_unknown_unit_is_not_found() {
    let fixture = world(&branching_machine());

    let error = unit(&fixture, "M7", "U1.8").await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert_unit_missing(&error, "M7", "U1.8");
}

#[tokio::test]
async fn an_unknown_machine_code_is_not_found_for_the_unit() {
    let fixture = world(&archived(machine_named(12, "M12", "Old press")));

    let error = unit(&fixture, "M12", "U1").await.unwrap_err();

    assert_machine_missing(&error, "M12");
}

#[tokio::test]
async fn the_unit_query_opens_one_read_only_transaction_and_writes_nothing() {
    let fixture = world(&branching_machine());

    unit(&fixture, "M7", "U1.2").await.unwrap();

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}
