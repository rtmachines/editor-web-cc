//! Tests of `GetApprovalPreview` (task 6.3, ADR-0049 (c), (d), ADR-0050 (f), (h), design D3).

use std::collections::BTreeMap;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{LeafPath, PlcPlatform, Primitive};
use recipe_domain::recipe::{
    DiffKind, DiffPart, ParameterKey, ParameterTag, Recipe, StoredLimit, StoredParameter,
    StoredRecipe, StoredValue, StoredVersionHeader, VersionState,
};
use serde_json::json;

use super::approval_preview::{ApprovalPreview, ApprovalPreviewInput, GetApprovalPreview};
use super::diff::DiffChoice;
use super::diff_tests::{copy, kinds, seed, world, zone};
use crate::auth::CurrentUser;
use crate::blocking::{AppSettings, DiffSettings};
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, param, pkey, reference, rid, second_engineer, text, tid, user, vid,
};
use crate::memory::{JournalEntry, seed_type_id};
use crate::ports::Isolation;
use crate::recipe::{ApproveVersion, ApproveVersionInput};

/// The live draft version 2 of `R-1`.
const NEW: u128 = 102;

pub(super) fn input(number: u32) -> ApprovalPreviewInput {
    ApprovalPreviewInput {
        reference: reference("R-1"),
        number,
        choice: DiffChoice {
            kind: None,
            page: 0,
        },
    }
}

pub(super) async fn preview_as(
    fixture: &Fixture,
    user: &CurrentUser,
    number: u32,
) -> Result<ApprovalPreview, AppError> {
    GetApprovalPreview::new(fixture.context())
        .execute(user, &meta(), input(number))
        .await
}

/// The administrator, who created and edited nothing.
pub(super) fn admin() -> CurrentUser {
    actor(Role::Administrator)
}

/// The approved version 1 with `A` = 4 and the draft version 2 with `A` = 5, both with a limit.
pub(super) fn valid_world() -> Fixture {
    let fixture = world();
    let old = StoredParameter {
        value: StoredValue::from(json!(4)),
        ..param(1, 101, "A")
    };
    let new = with_value(&copy(&old), 5);
    seed(&fixture, &[old], &[new]);
    fixture
}

fn with_value(parameter: &StoredParameter, value: i64) -> StoredParameter {
    StoredParameter {
        value: StoredValue::from(json!(value)),
        ..parameter.clone()
    }
}

/// Return the new texts of the value rows of the page.
fn new_value_texts(preview: &ApprovalPreview) -> Vec<Option<String>> {
    preview
        .diff
        .as_ref()
        .unwrap()
        .sections
        .iter()
        .filter(|section| section.kind == DiffKind::Values)
        .flat_map(|section| section.rows.iter())
        .filter_map(|row| match row.part() {
            DiffPart::Value { new, .. } => Some(new.clone()),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn the_preview_reads_one_snapshot_and_writes_nothing() {
    let fixture = valid_world();

    let preview = preview_as(&fixture, &admin(), 2).await.unwrap();

    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::RepeatableRead)]
    );
    assert!(fixture.store.audit_entries().is_empty());
    assert!(preview.may_approve);
    assert!(preview.approver_blocker.is_none());
    assert!(preview.draft_blocker.is_none());
    assert_eq!(preview.retired, Some(1));
    assert_eq!(preview.row_version, 1);
    assert_eq!(new_value_texts(&preview), vec![Some("5".to_owned())]);
}

#[tokio::test]
async fn preview_of_an_editor_holds_the_blocker_rcp_015_and_writes_nothing() {
    let fixture = valid_world();
    fixture.store.seed_editor(vid(NEW), user(3));

    let preview = preview_as(&fixture, &second_engineer(), 2).await.unwrap();

    let blocker = preview.approver_blocker.as_ref().map(|error| error.code());
    assert_eq!(blocker, Some(ErrorCode::RCP_015));
    assert!(preview.draft_blocker.is_none());
    assert!(!preview.may_approve);
    assert!(fixture.store.audit_entries().is_empty());
    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::RepeatableRead)]
    );
}

/// The draft of [`valid_world`] with a second parameter `HOLD` without a limit.
fn world_with_a_missing_limit() -> Fixture {
    let fixture = valid_world();
    let hold = StoredParameter {
        limits: Vec::new(),
        ..param(2, 101, "HOLD")
    };
    fixture.store.seed_parameter(vid(NEW), &hold);
    fixture
}

#[tokio::test]
async fn preview_of_an_editor_with_a_missing_limit_holds_rcp_015_and_rcp_016() {
    let fixture = world_with_a_missing_limit();
    fixture.store.seed_editor(vid(NEW), user(3));

    let preview = preview_as(&fixture, &second_engineer(), 2).await.unwrap();

    let approver = preview.approver_blocker.as_ref().map(|error| error.code());
    let draft = preview.draft_blocker.as_ref().map(AppError::code);
    assert_eq!(approver, Some(ErrorCode::RCP_015));
    assert_eq!(draft, Some(ErrorCode::RCP_016));
    assert!(
        preview
            .draft_blocker
            .as_ref()
            .is_some_and(|error| error.to_string().contains("HOLD"))
    );
}

/// Return the expected `blocker_keys` for pairs of a tag and the fixture number of its key.
pub(super) fn keys(entries: &[(&str, u128)]) -> BTreeMap<ParameterTag, ParameterKey> {
    entries
        .iter()
        .map(|(tag, n)| (ParameterTag::parse(tag).unwrap(), pkey(*n)))
        .collect()
}

#[tokio::test]
async fn the_blocker_keys_hold_the_key_of_the_tag_with_a_missing_limit() {
    let fixture = world_with_a_missing_limit();

    let preview = preview_as(&fixture, &admin(), 2).await.unwrap();

    let draft = preview.draft_blocker.as_ref().map(AppError::code);
    assert_eq!(draft, Some(ErrorCode::RCP_016));
    assert_eq!(preview.blocker_keys, keys(&[("HOLD", 2)]));
}

#[tokio::test]
async fn a_preview_without_a_blocker_has_no_blocker_keys() {
    let fixture = valid_world();

    let preview = preview_as(&fixture, &admin(), 2).await.unwrap();

    assert!(preview.may_approve);
    assert!(preview.blocker_keys.is_empty());
}

#[tokio::test]
async fn a_draft_time_count_that_does_not_fit_gives_the_blockers_and_no_diff() {
    let fixture = valid_world();
    let old = StoredParameter {
        plc_type: seed_type_id(Primitive::Time, None).unwrap(),
        value: StoredValue::from(json!(5_000)),
        limits: Vec::new(),
        ..param(2, 101, "HOLD")
    };
    let broken = with_value(&copy(&old), -1_000);
    seed(&fixture, &[old], &[broken]);

    let preview = preview_as(&fixture, &admin(), 2).await.unwrap();

    let draft = preview.draft_blocker.as_ref().map(AppError::code);
    assert_eq!(draft, Some(ErrorCode::RCP_020));
    assert!(preview.diff.is_none(), "{:?}", preview.diff);
    assert!(preview.outdated.is_empty());
    assert!(!preview.may_approve);
}

/// `HOLD` of type `TIME` with the stored count 5000 and the limit row `min` to 60000.
fn hold_with_min(min: i64) -> StoredParameter {
    StoredParameter {
        plc_type: seed_type_id(Primitive::Time, None).unwrap(),
        value: StoredValue::from(json!(5_000)),
        limits: vec![StoredLimit {
            min: Some(min.into()),
            max: Some(60_000_i64.into()),
            ..crate::memory::fixtures::limit()
        }],
        ..param(2, 101, "HOLD")
    }
}

#[tokio::test]
async fn a_draft_time_bound_that_does_not_fit_gives_the_blockers_and_no_diff() {
    let fixture = valid_world();
    let old = hold_with_min(1_000);
    let broken = StoredParameter {
        limits: hold_with_min(-1_000).limits,
        ..copy(&old)
    };
    seed(&fixture, &[old], &[broken]);

    let preview = preview_as(&fixture, &admin(), 2).await.unwrap();

    let draft = preview.draft_blocker.as_ref().map(AppError::code);
    assert_eq!(draft, Some(ErrorCode::RCP_020));
    assert!(preview.diff.is_none(), "{:?}", preview.diff);
    assert!(preview.outdated.is_empty());
}

#[tokio::test]
async fn an_approved_value_that_does_not_fit_without_a_blocker_gives_db_008() {
    let fixture = valid_world();
    let broken = StoredParameter {
        value: StoredValue::from(json!(-1_000)),
        ..hold_with_min(0)
    };
    let new = StoredParameter {
        value: StoredValue::from(json!(5_000)),
        ..copy(&broken)
    };
    seed(&fixture, &[broken], &[new]);

    let result = preview_as(&fixture, &admin(), 2).await;

    assert_eq!(
        result.as_ref().err().map(AppError::code),
        Some(ErrorCode::DB_008),
        "{result:?}"
    );
}

#[tokio::test]
async fn preview_of_a_retired_version_gives_rcp_007() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_retired(&machine, 1, "R-1");

    let error = preview_as(&fixture, &admin(), 1).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::RCP_007);
}

#[tokio::test]
async fn the_errors_come_in_the_order_of_approve_version() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    super::test_world::seed_deleted(&fixture, 1, "R-1");

    let unknown = preview_as(&fixture, &admin(), 9).await.unwrap_err();
    let deleted = preview_as(&fixture, &admin(), 1).await.unwrap_err();

    assert_eq!(unknown.code(), ErrorCode::WEB_005);
    assert_eq!(deleted.code(), ErrorCode::RCP_012);
}

async fn approve_error(fixture: &Fixture, user: &CurrentUser) -> AppError {
    let approve = ApproveVersionInput {
        reference: reference("R-1"),
        number: 2,
        expected_row_version: 1,
        allow_outdated: true,
    };
    ApproveVersion::new(fixture.context())
        .execute(user, &meta(), approve)
        .await
        .unwrap_err()
}

#[tokio::test]
async fn the_blockers_equal_the_errors_of_approve_version_on_the_same_state() {
    let missing = world_with_a_missing_limit();
    let edited = valid_world();
    edited.store.seed_editor(vid(NEW), user(3));

    let draft = preview_as(&missing, &admin(), 2).await.unwrap();
    let approver = preview_as(&edited, &second_engineer(), 2).await.unwrap();
    let draft_error = approve_error(&missing, &admin()).await;
    let approver_error = approve_error(&edited, &second_engineer()).await;

    let blocker = draft.draft_blocker.unwrap();
    assert_eq!(blocker.code(), draft_error.code());
    assert_eq!(blocker.to_string(), draft_error.to_string());
    let blocker = approver.approver_blocker.unwrap();
    assert_eq!(blocker.code(), approver_error.code());
    assert_eq!(blocker.to_string(), approver_error.to_string());
}

/// Recipe `R-1` with the approved version 1 and the live draft version 2 at `row_version`.
fn recipe_at(row_version: i64) -> Recipe {
    let header = |v: u128, number, state, row_version| StoredVersionHeader {
        id: vid(v),
        number,
        state,
        row_version,
    };
    Recipe::try_from(StoredRecipe {
        id: rid(1),
        machine: mid(1),
        reference: reference("R-1"),
        name: text("Recipe"),
        deleted_at: None,
        versions: vec![
            header(101, 1, VersionState::Approved, 1),
            header(NEW, 2, VersionState::LiveDraft, row_version),
        ],
    })
    .unwrap()
}

#[tokio::test]
async fn a_command_between_two_reads_of_the_preview_keeps_the_snapshot() {
    let fixture = valid_world();
    fixture
        .store
        .seed_recipe(&recipe_at(7), PlcPlatform::TwinCat3, user(2));
    let changed = with_value(&copy(&param(1, 101, "A")), 6);
    fixture.store.on_read_draft(move |store| {
        store.seed_recipe(&recipe_at(8), PlcPlatform::TwinCat3, user(2));
        store.seed_parameter(vid(NEW), &changed);
    });

    let preview = preview_as(&fixture, &admin(), 2).await.unwrap();

    assert_eq!(preview.row_version, 7);
    assert_eq!(new_value_texts(&preview), vec![Some("5".to_owned())]);
    assert_eq!(
        fixture
            .store
            .state()
            .versions
            .get(&vid(NEW))
            .map(|v| v.row_version),
        Some(8)
    );
}

#[tokio::test]
async fn an_outdated_parameter_is_listed_and_does_not_block() {
    let fixture = valid_world();
    fixture.store.seed_type(&zone(901, 1, None, &["setpoint"]));
    fixture
        .store
        .seed_type(&zone(902, 2, Some(901), &["setpoint"]));
    let outdated = StoredParameter {
        plc_type: tid(901),
        value: StoredValue::from(json!({"setpoint": 1})),
        limits: vec![StoredLimit {
            leaf_path: LeafPath::parse(".setpoint").unwrap(),
            ..crate::memory::fixtures::limit()
        }],
        ..param(3, 101, "ZONE")
    };
    fixture.store.seed_parameter(vid(NEW), &outdated);

    let preview = preview_as(&fixture, &admin(), 2).await.unwrap();

    let listed: Vec<(String, u32, u32)> = preview
        .outdated
        .iter()
        .map(|item| (item.tag.to_string(), item.current, item.newest))
        .collect();
    assert_eq!(listed, vec![("ZONE".to_owned(), 1, 2)]);
    assert!(preview.draft_blocker.is_none());
    assert!(preview.may_approve);
}

#[tokio::test]
async fn a_first_approval_shows_every_parameter_as_added_and_retires_nothing() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let (_, draft) = fixture.seed_draft(&machine, 1, "R-1");
    fixture.store.seed_parameter(draft, &param(1, 101, "A"));

    let preview = preview_as(&fixture, &admin(), 1).await.unwrap();

    assert_eq!(preview.retired, None);
    assert_eq!(
        kinds(preview.diff.as_ref().unwrap()),
        vec![DiffKind::Limits, DiffKind::Added]
    );
    assert!(preview.may_approve);
}

#[tokio::test]
async fn the_setting_leaf_rows_cuts_the_rows_of_one_parameter_in_the_preview() {
    let fixture = world();
    fixture
        .store
        .seed_type(&zone(901, 1, None, &["setpoint", "extra"]));
    let old = StoredParameter {
        plc_type: tid(901),
        value: StoredValue::from(json!({"setpoint": 1, "extra": 2})),
        limits: Vec::new(),
        ..param(1, 101, "ZONE")
    };
    let new = StoredParameter {
        value: StoredValue::from(json!({"setpoint": 3, "extra": 4})),
        ..copy(&old)
    };
    seed(&fixture, &[old], &[new]);
    let settings = AppSettings {
        diff: DiffSettings {
            leaf_rows: 1,
            ..DiffSettings::default()
        },
        ..AppSettings::default()
    };

    let preview = GetApprovalPreview::new(fixture.context_with(settings))
        .execute(&admin(), &meta(), input(2))
        .await
        .unwrap();

    let parts: Vec<&DiffPart> = preview
        .diff
        .as_ref()
        .unwrap()
        .sections
        .iter()
        .filter(|section| section.kind == DiffKind::Values)
        .flat_map(|section| section.rows.iter().map(|row| row.part()))
        .collect();
    assert_eq!(parts.len(), 2, "{parts:?}");
    assert!(matches!(parts[0], DiffPart::Value { .. }), "{parts:?}");
    assert!(
        matches!(parts[1], DiffPart::Omitted { count: 1 }),
        "{parts:?}"
    );
}

#[tokio::test]
async fn the_preview_holds_the_permit_until_its_holder_drops_it() {
    let fixture = valid_world();
    let settings = AppSettings {
        diff: DiffSettings {
            permits: 1,
            ..DiffSettings::default()
        },
        ..AppSettings::default()
    };
    let context = fixture.context_with(settings);

    let preview = GetApprovalPreview::new(context.clone())
        .execute(&admin(), &meta(), input(2))
        .await
        .unwrap();

    assert_eq!(context.diff_gate.available(), 0);
    drop(preview);
    assert_eq!(context.diff_gate.available(), 1);
}
