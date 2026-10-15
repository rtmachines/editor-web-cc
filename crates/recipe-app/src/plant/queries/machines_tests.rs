//! Tests of `ListMachines`, `GetMachine`, and `GetMachineForm` on `MemoryStore`.
//!
//! The specs are "Machine list", "Archive message", and "Unit tree of a machine" (design D3).

use recipe_domain::identity::Role;
use recipe_domain::plant::{Machine, MachineCode, PlcPlatform, Unit, UnitPath};

use super::machines::{
    GetMachine, GetMachineForm, GetMachineInput, ListMachines, ListMachinesInput, MachineList,
    MachineRef, MachineView,
};
use super::world::{archived, machine_named, with_archived_unit, with_units};
use crate::auth::Operation;
use crate::blocking::{AppSettings, PageSizes};
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::JournalEntry;
use crate::memory::fixtures::{Fixture, actor, meta, mid, text, timestamp};
use crate::ports::{Isolation, MachineFilter};

fn code(value: &str) -> MachineCode {
    MachineCode::parse(value).unwrap()
}

fn path(value: &str) -> UnitPath {
    UnitPath::parse(value).unwrap()
}

/// The request for the page `page` with no search text and no archive message.
fn plain(page: u32) -> ListMachinesInput {
    ListMachinesInput {
        filter: MachineFilter::default(),
        page,
        archived: None,
    }
}

/// The settings with `size` rows on a page of the machine list.
fn pages_of(size: u32) -> AppSettings {
    AppSettings {
        pages: PageSizes {
            machines: size,
            type_usage: 100,
        },
        ..AppSettings::default()
    }
}

async fn run_list(
    fixture: &Fixture,
    settings: AppSettings,
    role: Role,
    input: ListMachinesInput,
) -> Result<MachineList, AppError> {
    ListMachines::new(fixture.context_with(settings))
        .execute(&actor(role), &meta(), input)
        .await
}

async fn list(fixture: &Fixture, input: ListMachinesInput) -> MachineList {
    run_list(fixture, AppSettings::default(), Role::Operator, input)
        .await
        .unwrap()
}

async fn get(
    fixture: &Fixture,
    machine: MachineRef,
    message: Option<&str>,
) -> Result<MachineView, AppError> {
    let input = GetMachineInput {
        machine,
        archived: message.map(path),
    };
    GetMachine::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input)
        .await
}

fn codes(list: &MachineList) -> Vec<String> {
    list.rows.iter().map(|row| row.code.to_string()).collect()
}

fn unit_paths<'a>(units: impl Iterator<Item = &'a Unit>) -> Vec<String> {
    units.map(|unit| unit.path().to_string()).collect()
}

fn seed(fixture: &Fixture, machines: &[Machine]) {
    for machine in machines {
        fixture.store.seed_machine(machine);
    }
}

/// The live machine `M7` with the live unit `U1` and the archived unit `U2`.
fn machine_with_archived_unit() -> Machine {
    with_archived_unit(with_units(machine_named(7, "M7", "Press"), &["U2"]), "U2")
}

/// The archived machine `M12` with the live unit `U1` and the archived unit `U2`.
fn archived_machine() -> Machine {
    archived(with_archived_unit(
        with_units(machine_named(12, "M12", "Old press"), &["U2"]),
        "U2",
    ))
}

#[test]
fn the_queries_declare_their_operations() {
    assert_eq!(ListMachines::OPERATION, Operation::ListMachines);
    assert_eq!(GetMachine::OPERATION, Operation::GetMachine);
    assert_eq!(GetMachineForm::OPERATION, Operation::GetMachineForm);
}

#[tokio::test]
async fn live_machines_come_in_the_order_of_the_code_number() {
    let fixture = Fixture::new();
    let machines = [(10, "M10"), (2, "M2"), (1, "M1")];
    for (n, machine_code) in machines {
        seed(&fixture, &[machine_named(n, machine_code, "Press")]);
    }

    let result = list(&fixture, plain(0)).await;

    assert_eq!(codes(&result), ["M1", "M2", "M10"]);
    assert_eq!((result.page, result.has_next), (0, false));
    assert_eq!(result.archived, None);
}

#[tokio::test]
async fn a_search_text_matches_the_name_with_no_difference_of_case() {
    let fixture = Fixture::new();
    seed(
        &fixture,
        &[
            machine_named(4, "M4", "Piec hartowniczy"),
            machine_named(5, "M5", "Prasa"),
        ],
    );
    let input = ListMachinesInput {
        filter: MachineFilter {
            text: Some(text("PIEC")),
            include_archived: false,
        },
        ..plain(0)
    };

    let result = run_list(
        &fixture,
        AppSettings::default(),
        Role::ProcessEngineer,
        input,
    )
    .await
    .unwrap();

    assert_eq!(codes(&result), ["M4"]);
}

#[tokio::test]
async fn show_archived_adds_the_archived_machine_with_its_identifier_and_time() {
    let fixture = Fixture::new();
    seed(
        &fixture,
        &[
            archived(machine_named(12, "M12", "Old press")),
            machine_named(112, "M12", "New press"),
        ],
    );
    let input = ListMachinesInput {
        filter: MachineFilter {
            text: None,
            include_archived: true,
        },
        ..plain(0)
    };

    let result = list(&fixture, input).await;

    assert_eq!(codes(&result), ["M12", "M12"]);
    let old = result.rows.last().unwrap();
    assert_eq!(old.id, mid(12));
    assert_eq!(old.archived_at, Some(timestamp()));
    assert_eq!(result.rows.first().unwrap().archived_at, None);
}

#[tokio::test]
async fn without_show_archived_the_list_holds_only_live_machines() {
    let fixture = Fixture::new();
    seed(
        &fixture,
        &[
            archived(machine_named(12, "M12", "Old press")),
            machine_named(112, "M12", "New press"),
        ],
    );

    let result = list(&fixture, plain(0)).await;

    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows.first().unwrap().id, mid(112));
}

#[tokio::test]
async fn the_second_page_of_101_machines_holds_one_machine_and_no_next_page() {
    let fixture = Fixture::new();
    for n in 1_u128..=101 {
        seed(&fixture, &[machine_named(n, &format!("M{n}"), "Press")]);
    }

    let first = list(&fixture, plain(0)).await;
    let second = list(&fixture, plain(1)).await;

    assert_eq!(first.rows.len(), 100);
    assert!(first.has_next);
    assert_eq!(codes(&second), ["M101"]);
    assert!(!second.has_next);
    assert_eq!(second.page, 1);
}

#[tokio::test]
async fn a_page_after_the_end_is_empty_and_has_no_next_page() {
    let fixture = Fixture::new();
    for n in 1_u128..=3 {
        seed(&fixture, &[machine_named(n, &format!("M{n}"), "Press")]);
    }

    let result = list(&fixture, plain(2)).await;

    assert!(result.rows.is_empty(), "{:?}", result.rows);
    assert!(!result.has_next);
    assert_eq!(result.page, 2);
}

#[tokio::test]
async fn the_page_size_comes_from_the_settings() {
    let fixture = Fixture::new();
    for n in 1_u128..=3 {
        seed(&fixture, &[machine_named(n, &format!("M{n}"), "Press")]);
    }

    let first = run_list(&fixture, pages_of(2), Role::Operator, plain(0))
        .await
        .unwrap();
    let second = run_list(&fixture, pages_of(2), Role::Operator, plain(1))
        .await
        .unwrap();

    assert_eq!(codes(&first), ["M1", "M2"]);
    assert!(first.has_next);
    assert_eq!(codes(&second), ["M3"]);
    assert!(!second.has_next);
}

#[tokio::test]
async fn a_page_size_that_cannot_add_one_is_a_storage_error() {
    let fixture = Fixture::new();

    let error = run_list(&fixture, pages_of(u32::MAX), Role::Operator, plain(0))
        .await
        .unwrap_err();

    assert!(matches!(error, AppError::Storage(_)), "{error:?}");
}

#[tokio::test]
async fn an_archive_message_names_a_code_that_only_an_archived_machine_has() {
    let fixture = Fixture::new();
    seed(&fixture, &[archived(machine_named(7, "M7", "Press"))]);
    let input = ListMachinesInput {
        archived: Some(code("M7")),
        ..plain(0)
    };

    let result = list(&fixture, input).await;

    assert_eq!(result.archived, Some(code("M7")));
}

#[tokio::test]
async fn an_archive_message_for_a_live_machine_is_dropped() {
    let fixture = Fixture::new();
    seed(&fixture, &[machine_named(7, "M7", "Press")]);
    let input = ListMachinesInput {
        archived: Some(code("M7")),
        ..plain(0)
    };

    let result = list(&fixture, input).await;

    assert_eq!(result.archived, None);
}

#[tokio::test]
async fn an_archive_message_is_dropped_when_a_live_machine_replaced_the_archived_one() {
    let fixture = Fixture::new();
    seed(
        &fixture,
        &[
            archived(machine_named(7, "M7", "Old press")),
            machine_named(107, "M7", "New press"),
        ],
    );
    let input = ListMachinesInput {
        archived: Some(code("M7")),
        ..plain(0)
    };

    let result = list(&fixture, input).await;

    assert_eq!(result.archived, None);
}

#[tokio::test]
async fn an_archive_message_for_an_unknown_code_is_dropped() {
    let fixture = Fixture::new();
    let input = ListMachinesInput {
        archived: Some(code("M99")),
        ..plain(0)
    };

    let result = list(&fixture, input).await;

    assert_eq!(result.archived, None);
}

#[tokio::test]
async fn the_machine_list_opens_one_read_only_transaction_and_writes_nothing() {
    let fixture = Fixture::new();
    seed(&fixture, &[machine_named(1, "M1", "Press")]);
    let input = ListMachinesInput {
        archived: Some(code("M1")),
        ..plain(0)
    };

    list(&fixture, input).await;

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_live_machine_by_code_returns_the_machine_with_its_units() {
    let fixture = Fixture::new();
    let machine = with_units(machine_named(7, "M7", "Press"), &["U1.2"]);
    seed(&fixture, std::slice::from_ref(&machine));

    let view = get(&fixture, MachineRef::Code(code("M7")), None)
        .await
        .unwrap();

    assert_eq!(view.machine, machine);
    assert_eq!(view.archived, None);
    assert_eq!(unit_paths(view.machine.units()), ["U1", "U1.2"]);
}

#[tokio::test]
async fn a_live_machine_keeps_its_archived_unit_marked_for_the_web_layer() {
    let fixture = Fixture::new();
    seed(&fixture, &[machine_with_archived_unit()]);

    let view = get(&fixture, MachineRef::Code(code("M7")), None)
        .await
        .unwrap();

    let live = view.machine.units().filter(|unit| unit.is_live());
    assert_eq!(unit_paths(live), ["U1"]);
    let second = view.machine.unit(&path("U2")).unwrap();
    assert_eq!(second.archived_at(), Some(timestamp()));
}

#[tokio::test]
async fn an_archived_machine_by_identifier_returns_every_unit() {
    let fixture = Fixture::new();
    seed(&fixture, &[archived_machine()]);

    let view = get(&fixture, MachineRef::Id(mid(12)), None).await.unwrap();

    assert_eq!(view.machine.archived_at(), Some(timestamp()));
    assert_eq!(unit_paths(view.machine.units()), ["U1", "U2"]);
    let second = view.machine.unit(&path("U2")).unwrap();
    assert_eq!(second.archived_at(), Some(timestamp()));
}

#[tokio::test]
async fn a_live_machine_by_identifier_is_the_live_machine() {
    let fixture = Fixture::new();
    seed(&fixture, &[machine_named(7, "M7", "Press")]);

    let by_id = get(&fixture, MachineRef::Id(mid(7)), None).await.unwrap();
    let by_code = get(&fixture, MachineRef::Code(code("M7")), None)
        .await
        .unwrap();

    assert_eq!(by_id.machine, by_code.machine);
    assert!(by_id.machine.is_live());
}

#[tokio::test]
async fn the_code_of_an_archived_machine_finds_no_machine() {
    let fixture = Fixture::new();
    seed(&fixture, &[archived_machine()]);

    let error = get(&fixture, MachineRef::Code(code("M12")), None)
        .await
        .unwrap_err();

    let expected = Missing::Machine(code("M12"));
    assert!(
        matches!(&error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_unknown_identifier_is_not_found_with_the_identifier() {
    let fixture = Fixture::new();

    let error = get(&fixture, MachineRef::Id(mid(99)), None)
        .await
        .unwrap_err();

    let expected = Missing::MachineId(mid(99));
    assert!(
        matches!(&error, AppError::NotFound { entity } if *entity == expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn the_path_of_a_live_unit_is_dropped_from_the_archive_message() {
    let fixture = Fixture::new();
    seed(
        &fixture,
        &[with_units(machine_named(7, "M7", "Press"), &["U1.2"])],
    );

    let view = get(&fixture, MachineRef::Code(code("M7")), Some("U1.2"))
        .await
        .unwrap();

    assert_eq!(view.archived, None);
}

#[tokio::test]
async fn the_path_of_an_archived_unit_is_the_archive_message() {
    let fixture = Fixture::new();
    seed(&fixture, &[machine_with_archived_unit()]);

    let view = get(&fixture, MachineRef::Code(code("M7")), Some("U2"))
        .await
        .unwrap();

    assert_eq!(view.archived, Some(path("U2")));
}

#[tokio::test]
async fn the_path_of_an_unknown_unit_is_dropped_from_the_archive_message() {
    let fixture = Fixture::new();
    seed(&fixture, &[machine_named(7, "M7", "Press")]);

    let view = get(&fixture, MachineRef::Code(code("M7")), Some("U9"))
        .await
        .unwrap();

    assert_eq!(view.archived, None);
}

#[tokio::test]
async fn get_machine_opens_one_read_only_transaction_and_writes_nothing() {
    let fixture = Fixture::new();
    seed(&fixture, &[machine_named(7, "M7", "Press")]);

    get(&fixture, MachineRef::Code(code("M7")), None)
        .await
        .unwrap();

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::ReadOnly)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn the_machine_form_offers_the_five_platforms_and_reads_no_port() {
    let fixture = Fixture::new();

    let form = GetMachineForm::new(fixture.context())
        .execute(&actor(Role::ProcessEngineer), &meta())
        .await
        .unwrap();

    assert_eq!(form.platforms, PlcPlatform::ALL);
    assert!(fixture.store.journal().is_empty());
    assert!(fixture.sink.events().is_empty());
}
