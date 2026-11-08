//! Tests of `GetVersion` (design D4, ADR-0042, ADR-0043 (m)).

use std::collections::BTreeSet;
use std::time::Duration as StdDuration;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::Sha256;
use recipe_domain::identity::{Login, Role};
use recipe_domain::plant::{
    PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, S7Address,
};
use recipe_domain::recipe::{Ordinal, ParameterTag, StoredParameter, StoredValue, VersionState};
use serde_json::json;
use time::Duration;

use super::approval_preview_tests::{
    CURRENT_DATE_END, DATE_TYPE, EARLIER_DATE, date_parameter, date_world, seed_date_type,
};
use super::outdated::OutdatedParameter;
use super::test_world::seed_users;
use super::versions::{GetVersion, GetVersionInput, LastChange, VersionView};
use crate::auth::Operation;
use crate::blocking::{AppSettings, Blocking, ExportSettings};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing, TaskStage, TimeoutStage};
use crate::memory::JournalEntry;
use crate::memory::fixtures::pid;
use crate::memory::fixtures::{
    Fixture, actor, meta, parameter_on, pkey, recipe_with, reference, rid, scenarios,
    second_engineer, tid, uid, user, vid,
};
use crate::ports::Isolation;
use crate::recipe::AddParameter;
use crate::recipe::rebind::fixtures::typed_struct_row;

/// A type identifier that no type row of the tests holds.
const UNRESOLVABLE_TYPE: u128 = 9_999;

async fn get(
    fixture: &Fixture,
    role: Role,
    (name, number): (&str, u32),
) -> Result<VersionView, AppError> {
    GetVersion::new(fixture.context())
        .execute(
            &actor(role),
            &meta(),
            GetVersionInput {
                reference: reference(name),
                number,
                rebind_bound: 16_384,
            },
        )
        .await
}

#[test]
fn the_query_declares_its_operation() {
    assert_eq!(GetVersion::OPERATION, Operation::GetVersion);
}

#[tokio::test]
async fn get_version_returns_the_header_the_logins_and_the_machine() {
    let fixture = scenarios::approved_and_draft();
    seed_users(&fixture);

    let view = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();

    let detail = &view.detail;
    assert_eq!(detail.id, vid(101));
    assert_eq!(detail.recipe, rid(1));
    assert_eq!((detail.number, detail.state), (1, VersionState::Approved));
    assert_eq!(detail.created_by.as_str(), "jkowalski");
    assert_eq!(
        detail.approved_by.as_ref().map(Login::as_str),
        Some("jkowalski")
    );
    assert_eq!(detail.row_version, 1);
    assert_eq!(view.recipe.reference().to_string(), "R-1");
    assert_eq!(view.machine.platform(), PlcPlatform::TwinCat3);
    assert_eq!(view.last_change, None);
    assert!(view.outdated.is_empty());
}

#[tokio::test]
async fn get_version_returns_the_content_hash_of_an_approved_version_and_none_for_a_draft() {
    let fixture = scenarios::approved_and_draft();
    seed_users(&fixture);

    let approved = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();
    let draft = get(&fixture, Role::Operator, ("R-1", 2)).await.unwrap();

    assert_eq!(approved.detail.state, VersionState::Approved);
    assert_eq!(
        approved.detail.content_sha256,
        Some(Sha256::from([0_u8; 32]))
    );
    assert_eq!(draft.detail.state, VersionState::LiveDraft);
    assert_eq!(draft.detail.content_sha256, None);
}

#[tokio::test]
async fn get_version_reads_in_one_repeatable_read_transaction_and_writes_nothing() {
    let fixture = scenarios::draft();
    seed_users(&fixture);

    get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::RepeatableRead)]
    );
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn last_change_names_the_actor_of_the_last_draft_command_and_its_time() {
    let fixture = scenarios::draft();
    seed_users(&fixture);
    fixture.clock.advance(Duration::seconds(60));
    AddParameter::new(fixture.context())
        .execute(
            &second_engineer(),
            &meta(),
            scenarios::add_parameter_input(),
        )
        .await
        .unwrap();
    fixture.clock.advance(Duration::seconds(60));

    let view = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();

    assert_eq!(view.detail.row_version, 2);
    assert_eq!(
        view.last_change,
        Some(LastChange {
            login: Login::parse("anowak").unwrap(),
            at: view.detail.updated_at,
        })
    );
}

#[tokio::test]
async fn a_draft_with_an_old_struct_revision_lists_the_outdated_parameter() {
    let fixture = scenarios::zone_draft();
    seed_users(&fixture);

    let view = get(&fixture, Role::ProcessEngineer, ("R-1", 1))
        .await
        .unwrap();

    assert_eq!(view.outdated.len(), 1);
    let found: &OutdatedParameter = view.outdated.first().unwrap();
    assert_eq!(found.key, pkey(11));
    assert_eq!(found.tag.to_string(), "ZONE1");
    assert_eq!(found.unit_path.to_string(), "U1");
    assert_eq!(found.type_name.to_string(), "ST_Zone");
    assert_eq!((found.current, found.newest), (1, 2));
    assert_eq!((found.from, found.to), (tid(901), tid(902)));
}

#[tokio::test]
async fn a_version_that_is_not_a_live_draft_lists_no_outdated_parameter() {
    let fixture = scenarios::zone_draft();
    seed_users(&fixture);
    let approved = recipe_with((1, 1), "R-1", &[(101, VersionState::Approved)]);
    fixture
        .store
        .seed_recipe(&approved, PlcPlatform::TwinCat3, user(2));

    let view = get(&fixture, Role::ProcessEngineer, ("R-1", 1))
        .await
        .unwrap();

    assert_eq!(view.detail.state, VersionState::Approved);
    assert!(view.outdated.is_empty());
}

#[tokio::test]
async fn an_approved_version_with_the_stored_date_72700_gives_the_code_e_rcp_005() {
    let fixture = date_world(EARLIER_DATE, EARLIER_DATE);
    seed_users(&fixture);

    let view = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();

    assert_eq!(view.detail.state, VersionState::Approved);
    assert_eq!(view.rule_codes, vec![ErrorCode::RCP_005]);
    assert_eq!(
        fixture.store.journal(),
        [
            JournalEntry::Begin(Isolation::RepeatableRead),
            JournalEntry::Begin(Isolation::RepeatableRead)
        ]
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_approved_version_without_a_rule_fault_gives_no_code() {
    let fixture = date_world(CURRENT_DATE_END, CURRENT_DATE_END);
    seed_users(&fixture);

    let view = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();

    assert_eq!(view.detail.state, VersionState::Approved);
    assert!(view.rule_codes.is_empty(), "{:?}", view.rule_codes);
}

#[tokio::test]
async fn a_draft_with_the_stored_date_72700_gives_no_code() {
    let fixture = date_world(EARLIER_DATE, EARLIER_DATE);
    seed_users(&fixture);

    let view = get(&fixture, Role::ProcessEngineer, ("R-1", 2))
        .await
        .unwrap();

    assert_eq!(view.detail.state, VersionState::LiveDraft);
    assert!(view.rule_codes.is_empty(), "{:?}", view.rule_codes);
}

#[tokio::test]
async fn a_retired_version_with_the_stored_date_72700_gives_no_code() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_retired(&machine, 1, "R-1");
    let date_type = seed_date_type(&fixture);
    fixture
        .store
        .seed_parameter(vid(101), &date_parameter(date_type, EARLIER_DATE, false));
    seed_users(&fixture);

    let view = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();

    assert_eq!(view.detail.state, VersionState::Retired);
    assert!(view.rule_codes.is_empty(), "{:?}", view.rule_codes);
}

#[tokio::test]
async fn an_approved_version_lists_each_code_once_in_a_stable_order() {
    let fixture = date_world(EARLIER_DATE, EARLIER_DATE);
    seed_users(&fixture);
    let date_type = tid(DATE_TYPE);
    let twin = StoredParameter {
        id: pid(2),
        key: pkey(2),
        tag: ParameterTag::parse("DUE2").unwrap(),
        ordinal: Ordinal::parse(2).unwrap(),
        s7_address: Some(S7Address::parse("DB1.DBW2").unwrap()),
        ..date_parameter(date_type, EARLIER_DATE, false)
    };
    fixture.store.seed_parameter(vid(101), &twin);

    let view = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();

    assert_eq!(view.rule_codes, vec![ErrorCode::RCP_005]);
}

#[tokio::test]
async fn an_approved_version_with_an_unresolvable_type_gives_db_008() {
    let fixture = date_world(CURRENT_DATE_END, CURRENT_DATE_END);
    seed_users(&fixture);
    let lost = StoredParameter {
        id: pid(2),
        key: pkey(2),
        tag: ParameterTag::parse("LOST").unwrap(),
        ordinal: Ordinal::parse(2).unwrap(),
        s7_address: Some(S7Address::parse("DB1.DBW2").unwrap()),
        ..date_parameter(tid(UNRESOLVABLE_TYPE), CURRENT_DATE_END, false)
    };
    fixture.store.seed_parameter(vid(101), &lost);

    let error = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008);
}

/// The identifier of the struct `ST_Bytes@1` in the test of the overlap.
const BYTES_STRUCT: u128 = 970;
/// The first identifier of the global rows `BYTE`, `CHAR`, `SINT`, and `USINT`.
const BYTE_FAMILY_FIRST: u128 = 971;

/// Return the platforms of the global row of `primitive` (`0004_seed_plc_types.sql`).
///
/// `SINT` and `USINT` do not exist on `S7_300`. `CHAR` does not exist on the IEC platforms.
fn byte_family_platforms(primitive: Primitive) -> BTreeSet<PlcPlatform> {
    let all = [
        PlcPlatform::CodesysIec,
        PlcPlatform::TwinCat3,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
        PlcPlatform::S7_300,
    ];
    all.into_iter()
        .filter(|platform| match primitive {
            Primitive::Sint | Primitive::Usint => *platform != PlcPlatform::S7_300,
            Primitive::Char => matches!(
                platform,
                PlcPlatform::S7_1200 | PlcPlatform::S7_1500 | PlcPlatform::S7_300
            ),
            _ => true,
        })
        .collect()
}

/// Store the global row `id` of `primitive` with the platforms of the seed migration.
fn seed_byte_family_row(fixture: &Fixture, id: u128, primitive: Primitive) -> PlcTypeId {
    let row = PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: None,
        platforms: byte_family_platforms(primitive),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: primitive.fixed_size_bytes().unwrap(),
            string_length: None,
        },
    })
    .unwrap();
    fixture.store.seed_type(&row);
    tid(id)
}

/// Return the approved version 1 of `R-1` on `S7_1500` with an overlap of the current layout rules.
///
/// `BYTES` of type `ST_Bytes@1` stands at `DB1.DBB0` and `MODE` of type `BYTE` stands at
/// `DB1.DBB4`. The struct has the members `a` of type `BYTE`, `b` of type `CHAR`, `c` of type
/// `SINT`, and `d` of type `USINT`. The earlier layout gave it the size 4. The current layout
/// gives the size 6 (ADR-0147), so the struct shares the byte 4 with `MODE` (ADR-0149).
fn overlap_world() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let family = [
        ("a", Primitive::Byte),
        ("b", Primitive::Char),
        ("c", Primitive::Sint),
        ("d", Primitive::Usint),
    ];
    let members: Vec<(&str, PlcTypeId)> = (0_u128..)
        .zip(family)
        .map(|(offset, (name, primitive))| {
            let id = BYTE_FAMILY_FIRST.checked_add(offset).unwrap();
            (name, seed_byte_family_row(&fixture, id, primitive))
        })
        .collect();
    let byte = members.first().map(|(_, type_id)| *type_id).unwrap();
    fixture.store.seed_type(&typed_struct_row(
        PlcPlatform::S7_1500,
        (BYTES_STRUCT, "ST_Bytes", 1),
        None,
        &members,
    ));
    let address = |text: &str| Some(S7Address::parse(text).unwrap());
    let bytes_parameter = StoredParameter {
        plc_type: tid(BYTES_STRUCT),
        value: StoredValue::from(json!({"a": 0, "b": "A", "c": 0, "d": 0})),
        s7_address: address("DB1.DBB0"),
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::S7_1500, 1, "BYTES", uid(102), 0)
    };
    let mode_parameter = StoredParameter {
        plc_type: byte,
        value: StoredValue::from(json!(0)),
        s7_address: address("DB1.DBB4"),
        limits: Vec::new(),
        ordinal: Ordinal::parse(1).unwrap(),
        ..parameter_on(PlcPlatform::S7_1500, 2, "MODE", uid(102), 0)
    };
    fixture.store.seed_parameter(vid(101), &bytes_parameter);
    fixture.store.seed_parameter(vid(101), &mode_parameter);
    fixture
}

#[tokio::test]
async fn an_approved_version_with_an_overlap_of_the_current_layout_gives_the_code_e_rcp_022_once() {
    // ADR-0149 (g): the struct of size 6 at DB1.DBB0 covers MODE at DB1.DBB4. The overlap
    // affects two parameters. The page lists the code once.
    let fixture = overlap_world();
    seed_users(&fixture);

    let view = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();

    assert_eq!(view.detail.state, VersionState::Approved);
    assert_eq!(view.rule_codes, vec![ErrorCode::RCP_022]);
    assert!(fixture.store.audit_entries().is_empty());
}

/// Return the settings with `blocking` and an export gate of one permit.
fn one_export_permit(blocking: Blocking) -> AppSettings {
    AppSettings {
        blocking,
        export: ExportSettings { permits: 1 },
        ..AppSettings::default()
    }
}

/// Return a runner with a `timeout.export` of 50 milliseconds.
fn short_export() -> Blocking {
    Blocking::new(StdDuration::from_secs(10), StdDuration::from_millis(50))
}

async fn get_in(context: AppContext, role: Role, number: u32) -> Result<VersionView, AppError> {
    GetVersion::new(context)
        .execute(
            &actor(role),
            &meta(),
            GetVersionInput {
                reference: reference("R-1"),
                number,
                rebind_bound: 16_384,
            },
        )
        .await
}

#[tokio::test]
async fn the_rule_check_of_an_approved_version_waits_for_an_export_permit_and_returns_it() {
    let fixture = date_world(EARLIER_DATE, EARLIER_DATE);
    seed_users(&fixture);
    let context = fixture.context_with(one_export_permit(Blocking::default()));

    let view = get_in(context.clone(), Role::Operator, 1).await.unwrap();

    assert_eq!(view.rule_codes, vec![ErrorCode::RCP_005]);
    assert_eq!(context.export_gate.available(), 1);
}

#[tokio::test]
async fn a_busy_export_gate_beyond_timeout_export_gives_web_009_and_no_codes() {
    let fixture = date_world(EARLIER_DATE, EARLIER_DATE);
    seed_users(&fixture);
    let context = fixture.context_with(one_export_permit(short_export()));
    let held = context.export_gate.acquire().await.unwrap();

    let error = get_in(context, Role::Operator, 1).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Export
        }
    ));
    drop(held);
}

#[tokio::test]
async fn a_failed_rule_check_task_gives_web_011_with_the_export_stage_and_no_codes() {
    let fixture = date_world(EARLIER_DATE, EARLIER_DATE);
    seed_users(&fixture);
    let context = fixture.context_with(one_export_permit(Blocking::panicked()));

    let error = get_in(context.clone(), Role::Operator, 1)
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert!(matches!(
        error,
        AppError::TaskFailed {
            stage: TaskStage::Export
        }
    ));
    assert_eq!(context.export_gate.available(), 1);
}

#[tokio::test]
async fn a_draft_page_takes_no_export_permit() {
    let fixture = date_world(EARLIER_DATE, EARLIER_DATE);
    seed_users(&fixture);
    let context = fixture.context_with(one_export_permit(short_export()));
    let held = context.export_gate.acquire().await.unwrap();

    let view = get_in(context, Role::ProcessEngineer, 2).await.unwrap();

    assert_eq!(view.detail.state, VersionState::LiveDraft);
    drop(held);
}

#[tokio::test]
async fn an_unknown_recipe_or_version_is_not_found() {
    let fixture = scenarios::draft();
    seed_users(&fixture);

    let recipe = get(&fixture, Role::Operator, ("R-9", 1)).await.unwrap_err();
    let version = get(&fixture, Role::Operator, ("R-1", 7)).await.unwrap_err();

    assert!(matches!(
        recipe,
        AppError::NotFound {
            entity: Missing::Recipe(_)
        }
    ));
    assert!(matches!(
        version,
        AppError::NotFound {
            entity: Missing::Version { number: 7, .. }
        }
    ));
}

#[tokio::test]
async fn a_version_of_a_deleted_recipe_is_refused_except_for_the_role_that_restores() {
    let fixture = scenarios::deleted();
    seed_users(&fixture);

    let operator = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap_err();
    let engineer = get(&fixture, Role::ProcessEngineer, ("R-1", 1))
        .await
        .unwrap_err();
    let administrator = get(&fixture, Role::Administrator, ("R-1", 1))
        .await
        .unwrap();

    assert_eq!(operator.code(), ErrorCode::RCP_012);
    assert_eq!(engineer.code(), ErrorCode::RCP_012);
    assert_eq!(administrator.detail.state, VersionState::Discarded);
    assert!(administrator.recipe.is_deleted());
}
