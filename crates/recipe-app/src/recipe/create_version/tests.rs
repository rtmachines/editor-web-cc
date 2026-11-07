use std::collections::BTreeSet;

use recipe_domain::audit::{AggregateType, AuditAction, SecurityEventKind};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    LeafPath, Machine, MemberName, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive,
    RecordShape, S7Address, TypeMember, TypeName, UnitPath, UnitUsage,
};
use recipe_domain::recipe::{
    Budget, Ordinal, ParameterTag, RecipeDeleter, RecipeError, SnapshotItem, StoredLimit,
    StoredParameter, StoredValue, VersionState,
};
use serde_json::{Value, json};

use super::{CreateVersion, CreateVersionInput};
use crate::auth::Operation;
use crate::blocking::{AppSettings, Blocking};
use crate::context::UseCase;
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, field, lock_line, long_text, meta, mid, param, parameter_on, pid, recipe_with,
    reference, rid, tid, timestamp, uid, user, vid,
};
use crate::memory::{Fault, JournalEntry, LockMode, seed_type_id};
use crate::ports::{Isolation, Transaction};
use crate::recipe::test_support::seed_deleted;

fn input(reference_text: &str, based_on: Option<u32>) -> CreateVersionInput {
    CreateVersionInput {
        reference: reference(reference_text),
        based_on,
        change_note: long_text("Next version"),
    }
}

/// Store recipe 1 with the retired version 1 (id 101) and the approved version 2.
///
/// Version 1 holds the parameters `parameters`.
fn seed_base(fixture: &Fixture, machine: &Machine, parameters: &[StoredParameter]) {
    fixture.seed_retired(machine, 1, "R-1");
    for parameter in parameters {
        fixture.store.seed_parameter(vid(101), parameter);
    }
}

/// Archive the unit at `path` of `machine` and store the machine again.
fn archive_unit(fixture: &Fixture, machine: &mut Machine, path: &str) {
    machine
        .archive_unit(
            &UnitPath::parse(path).unwrap(),
            timestamp(),
            &UnitUsage::default(),
        )
        .unwrap();
    fixture.store.seed_machine(machine);
}

/// Return the codes of the items of an `E-RCP-020` error.
fn item_codes(error: &AppError) -> Vec<ErrorCode> {
    match error {
        AppError::Recipe(RecipeError::ValidationFailed { items }) => {
            items.iter().map(SnapshotItem::code).collect()
        }
        _ => Vec::new(),
    }
}

fn only_the_read() -> Vec<JournalEntry> {
    vec![JournalEntry::Begin(Isolation::ReadOnly)]
}

fn settings(blocking: Blocking) -> AppSettings {
    AppSettings {
        blocking,
        ..AppSettings::default()
    }
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(CreateVersion::OPERATION, Operation::CreateVersion);
}

#[tokio::test]
async fn a_deleted_recipe_gives_rcp_012_and_not_a_snapshot_item() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let error = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert_eq!(fixture.store.journal(), only_the_read());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_recipe_with_a_live_draft_gives_rcp_011() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let error = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_011);
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn an_archived_machine_gives_plt_010() {
    let fixture = Fixture::new();
    let mut machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_base(&fixture, &machine, &[]);
    machine.archive(timestamp(), None).unwrap();
    fixture.store.seed_machine(&machine);
    let error = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_010);
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn a_parameter_on_an_archived_unit_gives_rcp_020_with_a_plt_009_item() {
    let fixture = Fixture::new();
    let mut machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let platform = PlcPlatform::TwinCat3;
    seed_base(
        &fixture,
        &machine,
        &[parameter_on(platform, 1, "T1", uid(103), 0)],
    );
    archive_unit(&fixture, &mut machine, "U1.2.1");
    let error = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_020);
    assert_eq!(item_codes(&error), vec![ErrorCode::PLT_009]);
    assert_eq!(fixture.store.journal(), only_the_read());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_type_that_is_not_on_the_platform_gives_rcp_020_with_an_rcp_013_item() {
    let fixture = Fixture::new();
    let platform = PlcPlatform::S7_300;
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    // `LREAL` has no row for the platform `S7_300`.
    let parameter = StoredParameter {
        plc_type: seed_type_id(Primitive::Lreal, None).unwrap(),
        ..parameter_on(platform, 1, "T1", uid(103), 0)
    };
    seed_base(&fixture, &machine, &[parameter]);
    let error = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_020);
    assert_eq!(item_codes(&error), vec![ErrorCode::RCP_013]);
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn a_validation_timeout_gives_web_009_and_opens_no_write_transaction() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_base(&fixture, &machine, &[]);
    let context = fixture.context_with(settings(Blocking::elapsed()));
    let error = CreateVersion::new(context)
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn a_panicked_validation_gives_web_011_and_opens_no_write_transaction() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_base(&fixture, &machine, &[]);
    let context = fixture.context_with(settings(Blocking::panicked()));
    let error = CreateVersion::new(context)
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn an_unknown_base_number_or_reference_gives_web_005() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_base(&fixture, &machine, &[]);
    let use_case = CreateVersion::new(fixture.context());
    let user = actor(Role::ProcessEngineer);
    let number = use_case
        .execute(&user, &meta(), input("R-1", Some(9)))
        .await
        .unwrap_err();
    assert_eq!(number.code(), ErrorCode::WEB_005);
    let unknown = use_case
        .execute(&user, &meta(), input("R-9", None))
        .await
        .unwrap_err();
    assert_eq!(unknown.code(), ErrorCode::WEB_005);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_base(&fixture, &machine, &[]);
    let error = CreateVersion::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input("R-1", Some(1)))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
}

const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;

/// Store recipe 1 with the retired version 1, the approved version 2, and the discarded draft 3.
///
/// Version 1 holds `T1` on `U1.2.1` (unit 103) and `T2` on `U1.2` (unit 102).
fn seed_base_and_discarded_draft(fixture: &Fixture) -> Machine {
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    let recipe = recipe_with(
        (1, 1),
        "R-1",
        &[
            (101, VersionState::Retired),
            (102, VersionState::Approved),
            (103, VersionState::Discarded),
        ],
    );
    fixture.store.seed_recipe(&recipe, PLATFORM, user(2));
    let first = parameter_on(PLATFORM, 1, "T1", uid(103), 0);
    let second = parameter_on(PLATFORM, 2, "T2", uid(102), 2);
    fixture.store.seed_parameter(vid(101), &first);
    fixture.store.seed_parameter(vid(101), &second);
    machine
}

/// Read the parameters of `version` in a new `ReadOnly` transaction.
pub(super) async fn parameters_of(
    fixture: &Fixture,
    version: recipe_domain::recipe::RecipeVersionId,
) -> Vec<StoredParameter> {
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    tx.recipes().version_parameters(version).await.unwrap()
}

pub(super) fn write_journal(unit_locks: &[u128]) -> Vec<JournalEntry> {
    let mut journal = vec![
        JournalEntry::Begin(Isolation::ReadOnly),
        JournalEntry::Begin(Isolation::ReadCommitted),
        lock_line("machine", LockMode::ForShare, mid(1).to_string()),
        lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
    ];
    journal.extend(
        unit_locks
            .iter()
            .map(|unit| lock_line("unit", LockMode::ForShare, uid(*unit).to_string())),
    );
    journal
}

#[tokio::test]
async fn success_from_a_retired_base_copies_the_parameters_and_writes_one_entry() {
    let fixture = Fixture::new();
    seed_base_and_discarded_draft(&fixture);
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    let output = CreateVersion::new(fixture.context())
        .execute(&user, &request, input("R-1", Some(1)))
        .await
        .unwrap();
    assert_eq!(output.number, 4);
    assert!(output.outdated.is_empty());
    assert_eq!(fixture.store.journal(), write_journal(&[102, 103]));
    let state = fixture.store.state();
    let stored = state.recipe(rid(1)).unwrap();
    let header = stored.version(4).unwrap();
    assert_eq!(header.id(), output.version);
    assert_eq!(header.state(), VersionState::LiveDraft);
    let base = parameters_of(&fixture, vid(101)).await;
    let copy = parameters_of(&fixture, output.version).await;
    assert_eq!(copy.len(), 2);
    for (original, copied) in base.iter().zip(&copy) {
        assert_eq!(copied.key, original.key);
        assert_eq!(copied.tag, original.tag);
        assert_eq!(copied.unit, original.unit);
        assert_eq!(copied.limits, original.limits);
        assert_eq!(copied.translations.len(), original.translations.len());
        assert_ne!(copied.id, original.id);
    }
    assert!(copy.iter().all(|p| p.id != pid(1) && p.id != pid(2)));
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::VersionCreated);
    assert_eq!(entry.target().entity_type(), "recipe_version");
    assert_eq!(entry.target().entity_id(), output.version.as_uuid());
    assert_eq!(entry.target().aggregate_type(), AggregateType::Recipe);
    assert_eq!(entry.target().aggregate_id(), rid(1).as_uuid());
    assert!(entry.before().is_none());
    assert_eq!(field(entry.after().unwrap(), "number"), json!(4));
    assert_eq!(
        field(entry.after().unwrap(), "based_on_version_id"),
        json!(vid(101).to_string())
    );
    assert_eq!(entry.actor(), Some(user.user()));
    assert_eq!(entry.request_id(), request.request_id());
}

#[tokio::test]
async fn success_without_a_base_creates_an_empty_draft() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_retired(&machine, 1, "R-1");
    fixture
        .store
        .seed_parameter(vid(101), &parameter_on(PLATFORM, 1, "T1", uid(102), 0));
    let output = CreateVersion::new(fixture.context())
        .execute(&actor(Role::Administrator), &meta(), input("R-1", None))
        .await
        .unwrap();
    assert_eq!(output.number, 3);
    assert!(output.outdated.is_empty());
    assert_eq!(fixture.store.journal(), write_journal(&[]));
    assert!(parameters_of(&fixture, output.version).await.is_empty());
    let state = fixture.store.state();
    let stored = state.recipe(rid(1)).unwrap();
    assert_eq!(stored.version(3).unwrap().state(), VersionState::LiveDraft);
    let entries = fixture.store.audit_entries();
    assert_eq!(entries.len(), 1);
    let entry = entries.first().unwrap();
    assert_eq!(entry.action(), AuditAction::VersionCreated);
    assert_eq!(
        field(entry.after().unwrap(), "based_on_version_id"),
        Value::Null
    );
}

#[tokio::test]
async fn a_unit_archived_between_the_check_and_the_locks_gives_rcp_020_and_no_version() {
    let fixture = Fixture::new();
    let machine = seed_base_and_discarded_draft(&fixture);
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived
            .archive_unit(
                &UnitPath::parse("U1.2.1").unwrap(),
                timestamp(),
                &UnitUsage::default(),
            )
            .unwrap();
        store.seed_machine(&archived);
    });
    let error = CreateVersion::new(context)
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_020);
    assert_eq!(item_codes(&error), vec![ErrorCode::PLT_009]);
    assert!(matches!(
        &error,
        AppError::Recipe(RecipeError::ValidationFailed { items })
            if matches!(items.as_slice(), [SnapshotItem::Parameter { tag, .. }]
                if *tag == ParameterTag::parse("T1").unwrap())
    ));
    assert_eq!(fixture.store.journal(), write_journal(&[102, 103]));
    assert!(fixture.store.audit_entries().is_empty());
    let state = fixture.store.state();
    assert!(state.recipe(rid(1)).unwrap().version(4).is_none());
}

#[tokio::test]
async fn a_recipe_deleted_between_the_read_and_the_locks_gives_rcp_012_before_the_unit_recheck() {
    let fixture = Fixture::new();
    let machine = seed_base_and_discarded_draft(&fixture);
    let context = fixture.context_racing(move |store| {
        let mut archived = machine;
        archived
            .archive_unit(
                &UnitPath::parse("U1.2.1").unwrap(),
                timestamp(),
                &UnitUsage::default(),
            )
            .unwrap();
        store.seed_machine(&archived);
        let mut deleted = recipe_with(
            (1, 1),
            "R-1",
            &[
                (101, VersionState::Retired),
                (102, VersionState::Retired),
                (103, VersionState::Discarded),
            ],
        );
        deleted
            .delete(RecipeDeleter::Administrator, timestamp())
            .unwrap();
        store.seed_recipe(&deleted, PLATFORM, user(2));
    });
    let error = CreateVersion::new(context)
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert!(fixture.store.audit_entries().is_empty());
    let state = fixture.store.state();
    assert!(state.recipe(rid(1)).unwrap().version(4).is_none());
}

#[tokio::test]
async fn a_draft_created_between_the_read_and_the_locks_gives_rcp_011() {
    let fixture = Fixture::new();
    seed_base_and_discarded_draft(&fixture);
    let context = fixture.context_racing(|store| {
        let recipe = recipe_with(
            (1, 1),
            "R-1",
            &[
                (101, VersionState::Retired),
                (102, VersionState::Approved),
                (103, VersionState::LiveDraft),
            ],
        );
        store.seed_recipe(&recipe, PLATFORM, user(2));
    });
    let error = CreateVersion::new(context)
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_011);
    assert!(fixture.store.audit_entries().is_empty());
    let state = fixture.store.state();
    assert!(state.recipe(rid(1)).unwrap().version(4).is_none());
}

#[tokio::test]
async fn the_live_draft_constraint_gives_rcp_011_and_writes_nothing() {
    let fixture = Fixture::new();
    seed_base_and_discarded_draft(&fixture);
    fixture.store.fail_insert_version(Fault::DuplicateLiveDraft);
    let error = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_011);
    assert!(fixture.store.audit_entries().is_empty());
    let state = fixture.store.state();
    assert!(state.recipe(rid(1)).unwrap().version(4).is_none());
}

#[tokio::test]
async fn a_base_with_too_many_parameters_gives_rcp_021_and_opens_no_write_transaction() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_retired(&machine, 1, "R-1");
    for n in 1..=4_001_u128 {
        let tag = format!("P{n}");
        fixture
            .store
            .seed_parameter(vid(101), &parameter_on(PLATFORM, n, &tag, uid(102), 0));
    }
    let error = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_021);
    assert!(matches!(
        error,
        AppError::Recipe(RecipeError::SizeBudgetExceeded {
            budget: Budget::Parameters,
            limit: 4_000,
            actual: 4_001,
        })
    ));
    assert_eq!(fixture.store.journal(), only_the_read());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_parameter_on_an_unknown_unit_gives_rcp_020_with_a_plt_020_item() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    seed_base(
        &fixture,
        &machine,
        &[parameter_on(PLATFORM, 1, "T1", uid(999), 0)],
    );
    let error = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_020);
    assert_eq!(item_codes(&error), vec![ErrorCode::PLT_020]);
    assert_eq!(fixture.store.journal(), only_the_read());
}

fn zone(id: u128, revision: u32, supersedes: Option<u128>) -> PlcType {
    let member = |name: &str, primitive| TypeMember {
        name: MemberName::parse(name).unwrap(),
        type_id: seed_type_id(primitive, None).unwrap(),
    };
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Struct {
            name: TypeName::parse("ST_Zone").unwrap(),
            revision,
            supersedes: supersedes.map(tid),
            members: vec![
                member("temp_sp", Primitive::Real),
                member("setpoint", Primitive::Int),
            ],
        },
    })
    .unwrap()
}

#[tokio::test]
async fn the_result_lists_the_parameters_of_an_outdated_type_revision() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.store.seed_type(&zone(900, 1, None));
    fixture.store.seed_type(&zone(901, 2, Some(900)));
    fixture.seed_retired(&machine, 1, "R-1");
    let outdated = StoredParameter {
        plc_type: tid(900),
        value: StoredValue::from(json!({"temp_sp": 85.5, "setpoint": 10})),
        limits: Vec::new(),
        translations: Vec::new(),
        ..param(1, 101, "Z1")
    };
    fixture.store.seed_parameter(vid(101), &outdated);
    fixture
        .store
        .seed_parameter(vid(101), &parameter_on(PLATFORM, 2, "T1", uid(102), 0));
    let output = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(1)),
        )
        .await
        .unwrap();
    assert_eq!(output.outdated, vec![ParameterTag::parse("Z1").unwrap()]);
    assert_eq!(parameters_of(&fixture, output.version).await.len(), 2);
}

/// The identifier of the `DATE` row that the tests of the earlier rule set add.
const DATE_TYPE: u128 = 950;

/// Store a `DATE` row for `S7_1500`, because the seed has no `DATE` row.
fn seed_date_type(fixture: &Fixture) -> PlcTypeId {
    fixture.seed_primitive_type(DATE_TYPE, PlcPlatform::S7_1500, Primitive::Date)
}

/// A parameter on `U1.2` of the `S7_1500` machine with the type, value, limits, and address given.
pub(super) fn s7_parameter(
    n: u128,
    tag: &str,
    (plc_type, value): (PlcTypeId, StoredValue),
    (limits, address): (Vec<StoredLimit>, &str),
) -> StoredParameter {
    StoredParameter {
        plc_type,
        value,
        limits,
        s7_address: Some(S7Address::parse(address).unwrap()),
        ordinal: Ordinal::parse(u32::try_from(n).unwrap()).unwrap(),
        ..parameter_on(PlcPlatform::S7_1500, n, tag, uid(102), 0)
    }
}

/// A limit row of the whole parameter with the upper bound `max`.
pub(super) fn max_limit(max: &str) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some(max.parse().unwrap()),
        max_length: None,
    }
}

/// Store recipe 1 on an `S7_1500` machine, and give its approved version 2 the parameters
/// that the earlier rule set permits and the current rule set refuses (ADR-0120 (g), (h)).
/// `S7_1500` is the platform, because `S7_300` keeps the `DATE` end 72683 in both rule sets.
///
/// `date` is the stored `DATE` count. The other parameters are a `REAL` with 8 significant
/// digits, a `REAL` limit `max` with 7 decimals, and a `DINT` that ends after its data block.
fn seed_earlier_rule_base(fixture: &Fixture, date: i64) -> Vec<StoredParameter> {
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let date_type = seed_date_type(fixture);
    let real = seed_type_id(Primitive::Real, None).unwrap();
    let dint = seed_type_id(Primitive::Dint, None).unwrap();
    let parameters = vec![
        s7_parameter(
            1,
            "DUE",
            (date_type, StoredValue::from(json!(date))),
            (Vec::new(), "DB1.DBW0"),
        ),
        s7_parameter(
            2,
            "BIG",
            (real, StoredValue::from(json!(16_777_217))),
            (Vec::new(), "DB1.DBD4"),
        ),
        s7_parameter(
            3,
            "CAP",
            (real, StoredValue::from(json!(50))),
            (vec![max_limit("100.00001")], "DB1.DBD8"),
        ),
        s7_parameter(
            4,
            "TAIL",
            (dint, StoredValue::from(json!(7))),
            (Vec::new(), "DB1.DBD65534"),
        ),
    ];
    let (_, _, approved) = fixture.seed_retired(&machine, 1, "R-1");
    for parameter in &parameters {
        fixture.store.seed_parameter(approved, parameter);
    }
    parameters
}

#[tokio::test]
async fn an_approved_base_with_the_earlier_rule_set_parts_is_copied_unchanged() {
    // ADR-0120 (g), (h): the base holds the DATE count 72700, the REAL 16777217, the REAL
    // limit max 100.00001, and the DINT at DB1.DBD65534. The current rules refuse each part.
    let fixture = Fixture::new();
    let base = seed_earlier_rule_base(&fixture, 72_700);
    let output = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(2)),
        )
        .await
        .unwrap();
    assert_eq!(output.number, 3);
    assert_eq!(fixture.store.journal(), write_journal(&[102]));
    let copy = parameters_of(&fixture, output.version).await;
    assert_eq!(copy.len(), base.len());
    for original in &base {
        // The copy lists the parameters by tag, so the test finds each one by its key.
        let copied = copy.iter().find(|p| p.key == original.key).unwrap();
        assert_eq!(copied.plc_type, original.plc_type);
        assert_eq!(copied.value, original.value);
        assert_eq!(copied.limits, original.limits);
        assert_eq!(copied.s7_address, original.s7_address);
        assert_ne!(copied.id, original.id);
    }
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn a_base_with_a_limit_without_max_is_checked_against_the_earlier_date_end() {
    // ADR-0120 (e), (g): the limit has a min only, so the earlier end 72840 bounds the count.
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    let limits = vec![StoredLimit {
        min: Some(70_000.into()),
        max: None,
        ..max_limit("0")
    }];
    let date = (seed_date_type(&fixture), StoredValue::from(json!(72_700)));
    let parameter = s7_parameter(1, "DUE", date, (limits, "DB1.DBW0"));
    let (_, _, approved) = fixture.seed_retired(&machine, 1, "R-1");
    fixture.store.seed_parameter(approved, &parameter);
    let output = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(2)),
        )
        .await
        .unwrap();
    assert_eq!(output.number, 3);
    assert_eq!(parameters_of(&fixture, output.version).await.len(), 1);
}

#[tokio::test]
async fn a_date_count_above_the_earlier_end_gives_rcp_020_and_no_draft() {
    // ADR-0120 risk: the earlier rule set ends at the count 72840, so 72900 still fails.
    let fixture = Fixture::new();
    seed_earlier_rule_base(&fixture, 72_900);
    let error = CreateVersion::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            input("R-1", Some(2)),
        )
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_020);
    assert_eq!(item_codes(&error), vec![ErrorCode::RCP_005]);
    assert_eq!(fixture.store.journal(), only_the_read());
    assert!(fixture.store.audit_entries().is_empty());
    let state = fixture.store.state();
    assert!(state.recipe(rid(1)).unwrap().version(3).is_none());
}
