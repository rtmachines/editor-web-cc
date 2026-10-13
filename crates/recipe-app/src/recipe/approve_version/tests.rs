use std::collections::BTreeSet;

use recipe_domain::audit::SecurityEventKind;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    LeafPath, Machine, MemberName, PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape,
    TypeMember, TypeName, UnitPath, UnitUsage,
};
use recipe_domain::recipe::{
    ApprovalOutcome, Ordinal, ParameterTag, Recipe, RecipeError, StoredLimit, StoredParameter,
    StoredRecipe, StoredValue, StoredVersionHeader, VersionState,
};
use serde_json::json;

use super::{ApproveVersion, ApproveVersionInput};
use crate::auth::{CurrentUser, Operation};
use crate::blocking::{AppSettings, Blocking};
use crate::context::UseCase;
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, parameter_on, reference, rid, text, tid, timestamp, uid, user, vid,
};
use crate::memory::{JournalEntry, MemoryStore, seed_type_id};
use crate::ports::Isolation;
use crate::recipe::test_support::seed_deleted;

mod write;

const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;

fn input(number: u32, expected_row_version: i64, allow_outdated: bool) -> ApproveVersionInput {
    ApproveVersionInput {
        reference: reference("R-1"),
        number,
        expected_row_version,
        allow_outdated,
    }
}

/// The approver of the tests: `anowak` created and edited nothing.
fn approver() -> CurrentUser {
    crate::memory::fixtures::second_engineer()
}

fn settings(blocking: Blocking) -> AppSettings {
    AppSettings {
        blocking,
        ..AppSettings::default()
    }
}

fn only_the_read() -> Vec<JournalEntry> {
    vec![JournalEntry::Begin(Isolation::ReadOnly)]
}

/// Build recipe 1 of machine 1 with the given versions: identifier, state, and row version.
///
/// The headers have the numbers 1 to the length of `versions`.
fn recipe_at(versions: &[(u128, VersionState, i64)]) -> Recipe {
    let headers = (1_u32..)
        .zip(versions)
        .map(|(number, (v, state, row_version))| StoredVersionHeader {
            id: vid(*v),
            number,
            state: *state,
            row_version: *row_version,
        })
        .collect();
    Recipe::try_from(StoredRecipe {
        id: rid(1),
        machine: mid(1),
        reference: reference("R-1"),
        name: text("Recipe"),
        deleted_at: None,
        versions: headers,
    })
    .unwrap()
}

fn store_recipe(store: &MemoryStore, platform: PlcPlatform, recipe: &Recipe) {
    store.seed_recipe(recipe, platform, user(2));
}

/// Store a machine of `platform` and the live draft 101 of recipe `R-1` with `parameters`.
fn seed_with(fixture: &Fixture, platform: PlcPlatform, parameters: &[StoredParameter]) -> Machine {
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    for parameter in parameters {
        fixture.store.seed_parameter(vid(101), parameter);
    }
    machine
}

/// The limit row of a leaf with the bounds 0 and 500.
fn leaf_limit(path: &str) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse(path).unwrap(),
        min: Some("0".parse().unwrap()),
        max: Some("500".parse().unwrap()),
        max_length: None,
    }
}

/// The struct `ST_Zone` with the members `temp_sp` (`REAL`) and `setpoint` (`INT`).
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

/// A parameter of `ST_Zone@1` on the unit 102. `with_limits` adds a limit for each member.
fn zone_parameter(n: u128, tag: &str, with_limits: bool) -> StoredParameter {
    let limits = if with_limits {
        vec![leaf_limit(".temp_sp"), leaf_limit(".setpoint")]
    } else {
        Vec::new()
    };
    StoredParameter {
        plc_type: tid(900),
        value: StoredValue::from(json!({"temp_sp": 85.5, "setpoint": 10})),
        limits,
        ordinal: Ordinal::parse(0).unwrap(),
        ..parameter_on(PLATFORM, n, tag, uid(102), 0)
    }
}

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

async fn run(
    fixture: &Fixture,
    number: u32,
    expected: i64,
    allow_outdated: bool,
) -> Result<ApprovalOutcome, AppError> {
    ApproveVersion::new(fixture.context())
        .execute(
            &approver(),
            &meta(),
            input(number, expected, allow_outdated),
        )
        .await
}

#[test]
fn the_use_case_declares_its_operation() {
    assert_eq!(ApproveVersion::OPERATION, Operation::ApproveVersion);
}

#[tokio::test]
async fn a_replay_gives_already_approved_with_no_write_transaction_and_no_entry() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    let recipe = recipe_at(&[(101, VersionState::Approved, 2)]);
    store_recipe(&fixture.store, machine.platform(), &recipe);
    let outcome = run(&fixture, 1, 1, false).await.unwrap();
    assert_eq!(outcome, ApprovalOutcome::AlreadyApproved);
    assert_eq!(fixture.store.journal(), only_the_read());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_approval_that_commits_before_the_draft_read_gives_already_approved() {
    let fixture = Fixture::new();
    seed_with(
        &fixture,
        PLATFORM,
        &[parameter_on(PLATFORM, 1, "T1", uid(102), 0)],
    );
    // Another request approves the version after this request read the root.
    fixture.store.on_read_draft(|store| {
        let approved = recipe_at(&[(101, VersionState::Approved, 2)]);
        store_recipe(store, PLATFORM, &approved);
    });
    let outcome = run(&fixture, 1, 1, false).await.unwrap();
    assert_eq!(outcome, ApprovalOutcome::AlreadyApproved);
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadOnly),
        ]
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_draft_discarded_before_the_draft_read_gives_rcp_007() {
    let fixture = Fixture::new();
    seed_with(
        &fixture,
        PLATFORM,
        &[parameter_on(PLATFORM, 1, "T1", uid(102), 0)],
    );
    fixture.store.on_read_draft(|store| {
        let discarded = recipe_at(&[(101, VersionState::Discarded, 2)]);
        store_recipe(store, PLATFORM, &discarded);
    });
    let error = run(&fixture, 1, 1, false).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_007);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_retired_version_gives_rcp_007_and_opens_no_write_transaction() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_retired(&machine, 1, "R-1");
    let error = run(&fixture, 1, 1, false).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_007);
    assert_eq!(fixture.store.journal(), only_the_read());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_deleted_recipe_gives_rcp_012() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    seed_deleted(&fixture, &machine, 1, "R-1");
    let error = run(&fixture, 1, 1, false).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn an_unknown_reference_or_number_gives_web_005() {
    let fixture = Fixture::new();
    seed_with(
        &fixture,
        PLATFORM,
        &[parameter_on(PLATFORM, 1, "T1", uid(102), 0)],
    );
    let number = run(&fixture, 9, 1, false).await.unwrap_err();
    assert_eq!(number.code(), ErrorCode::WEB_005);
    let unknown = ApproveVersion::new(fixture.context())
        .execute(
            &approver(),
            &meta(),
            ApproveVersionInput {
                reference: reference("R-9"),
                ..input(1, 1, false)
            },
        )
        .await
        .unwrap_err();
    assert_eq!(unknown.code(), ErrorCode::WEB_005);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_draft_without_parameters_gives_rcp_008() {
    let fixture = Fixture::new();
    seed_with(&fixture, PLATFORM, &[]);
    let error = run(&fixture, 1, 1, false).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_008);
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn every_leaf_without_a_limit_gives_one_rcp_016_entry() {
    let fixture = Fixture::new();
    fixture.store.seed_type(&zone(900, 1, None));
    seed_with(&fixture, PLATFORM, &[zone_parameter(1, "Z1", false)]);
    let error = run(&fixture, 1, 1, false).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_016);
    let tag = ParameterTag::parse("Z1").unwrap();
    assert!(
        matches!(&error, AppError::Recipes(list)
        if list.len() == 2
            && list.iter().all(|entry| matches!(
                entry,
                RecipeError::LimitMissing { tag: missing, .. } if *missing == tag
            ))),
        "{error:?}"
    );
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn a_parameter_on_an_archived_unit_gives_rcp_020() {
    let fixture = Fixture::new();
    let mut machine = seed_with(
        &fixture,
        PLATFORM,
        &[parameter_on(PLATFORM, 1, "T1", uid(103), 0)],
    );
    archive_unit(&fixture, &mut machine, "U1.2.1");
    let error = run(&fixture, 1, 1, false).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_020);
    assert_eq!(fixture.store.journal(), only_the_read());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn two_parameters_on_one_s7_address_give_rcp_022() {
    let fixture = Fixture::new();
    let platform = PlcPlatform::S7_300;
    seed_with(
        &fixture,
        platform,
        &[
            parameter_on(platform, 1, "T1", uid(102), 0),
            parameter_on(platform, 2, "T2", uid(102), 0),
        ],
    );
    let error = run(&fixture, 1, 1, false).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_022);
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn an_int_on_an_odd_s7_offset_gives_rcp_023() {
    let fixture = Fixture::new();
    let platform = PlcPlatform::S7_300;
    seed_with(
        &fixture,
        platform,
        &[parameter_on(platform, 1, "T1", uid(102), 1)],
    );
    let error = run(&fixture, 1, 1, false).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_023);
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn an_outdated_struct_revision_gives_rcp_024() {
    let fixture = Fixture::new();
    fixture.store.seed_type(&zone(900, 1, None));
    fixture.store.seed_type(&zone(901, 2, Some(900)));
    seed_with(&fixture, PLATFORM, &[zone_parameter(1, "Z1", true)]);
    let error = run(&fixture, 1, 1, false).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::RCP_024);
    assert_eq!(fixture.store.journal(), only_the_read());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_validation_timeout_gives_web_009_and_opens_no_write_transaction() {
    let fixture = Fixture::new();
    seed_with(
        &fixture,
        PLATFORM,
        &[parameter_on(PLATFORM, 1, "T1", uid(102), 0)],
    );
    let error = ApproveVersion::new(fixture.context_with(settings(Blocking::elapsed())))
        .execute(&approver(), &meta(), input(1, 1, false))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn a_panicked_validation_gives_web_011_and_opens_no_write_transaction() {
    let fixture = Fixture::new();
    seed_with(
        &fixture,
        PLATFORM,
        &[parameter_on(PLATFORM, 1, "T1", uid(102), 0)],
    );
    let error = ApproveVersion::new(fixture.context_with(settings(Blocking::panicked())))
        .execute(&approver(), &meta(), input(1, 1, false))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert_eq!(fixture.store.journal(), only_the_read());
}

#[tokio::test]
async fn an_operator_is_forbidden_with_one_event_and_no_transaction() {
    let fixture = Fixture::new();
    seed_with(
        &fixture,
        PLATFORM,
        &[parameter_on(PLATFORM, 1, "T1", uid(102), 0)],
    );
    let error = ApproveVersion::new(fixture.context())
        .execute(&actor(Role::Operator), &meta(), input(1, 1, false))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_002);
    let events = fixture.sink.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events.first().unwrap().kind(), SecurityEventKind::Forbidden);
    assert!(fixture.store.journal().is_empty());
}
