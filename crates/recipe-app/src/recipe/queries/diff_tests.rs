//! Tests of `DiffVersions` (task 6.2, ADR-0050, design D4).

use std::collections::BTreeSet;
use std::time::Duration;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    LeafPath, MemberName, PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape, TypeMember,
    TypeName, UnitPath,
};
use recipe_domain::recipe::{
    DiffKind, DiffPart, ParameterKey, ParameterTag, StoredLimit, StoredParameter, StoredValue,
    Translation,
};
use serde_json::json;

use super::diff::{
    DiffChoice, DiffPage, DiffSummary, DiffVersions, DiffVersionsInput, VersionDiff,
};
use super::test_world::seed_deleted;
use crate::blocking::tests::{one_blocking_thread, wait_for_free};
use crate::blocking::{AppSettings, Blocking, DiffSettings, PageSizes};
use crate::context::AppContext;
use crate::error::{AppError, TaskStage, TimeoutStage};
use crate::memory::fixtures::{
    Fixture, actor, limit, meta, mid, param, pid, pkey, reference, text, tid, translation_pl, uid,
    vid,
};
use crate::memory::{JournalEntry, seed_type_id};
use crate::ports::Isolation;

/// The approved version 1 of `R-1`: the old side.
const OLD: u128 = 101;
/// The live draft version 2 of `R-1`: the new side.
const NEW: u128 = 102;
/// The offset of the identifier of the new copy of a parameter.
const COPY: u128 = 500;

mod stored_tests;

/// A fixture with the machine `M1`, the approved version 1, and the draft version 2 of `R-1`.
pub(super) fn world() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    fixture
}

/// Return a copy of `parameter` with another identifier and the same key.
pub(super) fn copy(parameter: &StoredParameter) -> StoredParameter {
    let id = parameter.id.as_uuid().as_u128().checked_add(COPY).unwrap();
    StoredParameter {
        id: pid(id),
        ..parameter.clone()
    }
}

/// Seed `old` into version 1 and `new` into version 2.
pub(super) fn seed(fixture: &Fixture, old: &[StoredParameter], new: &[StoredParameter]) {
    for parameter in old {
        fixture.store.seed_parameter(vid(OLD), parameter);
    }
    for parameter in new {
        fixture.store.seed_parameter(vid(NEW), parameter);
    }
}

/// Return `parameter` with the upper bound 20 on its limit row.
pub(super) fn with_new_limit(parameter: &StoredParameter) -> StoredParameter {
    StoredParameter {
        limits: vec![StoredLimit {
            max: Some("20".parse().unwrap()),
            ..limit()
        }],
        ..copy(parameter)
    }
}

/// Return `parameter` with the value 6.
pub(super) fn with_new_value(parameter: &StoredParameter) -> StoredParameter {
    StoredParameter {
        value: StoredValue::from(json!(6)),
        ..copy(parameter)
    }
}

fn input(choice: DiffChoice) -> DiffVersionsInput {
    DiffVersionsInput {
        reference: reference("R-1"),
        number: 2,
        old_number: 1,
        choice,
    }
}

const fn first_page() -> DiffChoice {
    DiffChoice {
        kind: None,
        page: 0,
    }
}

async fn run_with(
    context: AppContext,
    role: Role,
    input: DiffVersionsInput,
) -> Result<VersionDiff, AppError> {
    DiffVersions::new(context)
        .execute(&actor(role), &meta(), input)
        .await
}

async fn run(fixture: &Fixture, input: DiffVersionsInput) -> Result<VersionDiff, AppError> {
    run_with(fixture.context(), Role::ProcessEngineer, input).await
}

/// Return the distinct keys of the rows of section `kind`, in row order.
pub(super) fn section_keys(diff: &DiffPage, kind: DiffKind) -> Vec<ParameterKey> {
    let mut keys = Vec::new();
    let rows = diff
        .sections
        .iter()
        .filter(|section| section.kind == kind)
        .flat_map(|section| section.rows.iter());
    for row in rows {
        if !keys.contains(&row.key()) {
            keys.push(row.key());
        }
    }
    keys
}

/// Return the kinds of the sections of the page, in page order.
pub(super) fn kinds(diff: &DiffPage) -> Vec<DiffKind> {
    diff.sections.iter().map(|section| section.kind).collect()
}

/// One parameter for each section and one parameter without a change.
fn one_of_each(fixture: &Fixture) {
    let old = [
        param(1, 101, "A"),
        param(2, 101, "B"),
        param(3, 101, "C"),
        param(4, 101, "D"),
        param(6, 101, "F"),
        param(7, 101, "G"),
    ];
    let translated = StoredParameter {
        translations: vec![Translation {
            name: text("Inna"),
            ..translation_pl()
        }],
        ..copy(&old[3])
    };
    let renamed = StoredParameter {
        name_en: text("Other"),
        ..copy(&old[2])
    };
    let new = [
        with_new_limit(&old[0]),
        with_new_value(&old[1]),
        renamed,
        translated,
        param(5, 101, "E"),
        copy(&old[5]),
    ];
    seed(fixture, &old, &new);
}

#[tokio::test]
async fn diff_versions_reads_in_one_repeatable_read_transaction() {
    let fixture = world();
    one_of_each(&fixture);

    run(&fixture, input(first_page())).await.unwrap();

    assert_eq!(
        fixture.store.journal(),
        vec![JournalEntry::Begin(Isolation::RepeatableRead)]
    );
}

#[tokio::test]
async fn the_sections_come_in_the_order_of_a_page_with_the_counts_of_the_flags() {
    let fixture = world();
    one_of_each(&fixture);

    let diff = run(&fixture, input(first_page())).await.unwrap().diff;

    assert_eq!(kinds(&diff), DiffKind::ALL.to_vec());
    assert_eq!(
        section_keys(&diff, DiffKind::Limits),
        vec![pkey(1), pkey(5), pkey(6)]
    );
    assert_eq!(section_keys(&diff, DiffKind::Values), vec![pkey(2)]);
    assert_eq!(section_keys(&diff, DiffKind::Added), vec![pkey(5)]);
    assert_eq!(section_keys(&diff, DiffKind::Removed), vec![pkey(6)]);
    let expected = DiffSummary {
        limits: 3,
        values: 1,
        definitions: 1,
        translations: 1,
        added: 1,
        removed: 1,
        limit_rows: 3,
        definition_rows: 1,
        translation_rows: 1,
        unchanged: 1,
    };
    assert_eq!(diff.summary, expected);
    assert!(!diff.has_next);
}

/// Seed 40 parameters with a changed limit and 30 parameters with a changed value.
fn forty_limits_and_thirty_values(fixture: &Fixture) {
    let old: Vec<StoredParameter> = (1_u128..=70)
        .map(|n| param(n, 101, &format!("P{n:02}")))
        .collect();
    let new: Vec<StoredParameter> = old
        .iter()
        .zip(1_u128..)
        .map(|(parameter, n)| {
            if n <= 40 {
                with_new_limit(parameter)
            } else {
                with_new_value(parameter)
            }
        })
        .collect();
    seed(fixture, &old, &new);
}

fn keys(range: std::ops::RangeInclusive<u128>) -> Vec<ParameterKey> {
    range.map(pkey).collect()
}

#[tokio::test]
async fn a_page_without_a_section_fills_the_sections_up_to_page_diff_parameters() {
    let fixture = world();
    forty_limits_and_thirty_values(&fixture);

    let diff = run(&fixture, input(first_page())).await.unwrap().diff;

    assert_eq!(section_keys(&diff, DiffKind::Limits), keys(1..=40));
    assert_eq!(section_keys(&diff, DiffKind::Values), keys(41..=50));
    assert_eq!(diff.cut, vec![DiffKind::Values]);
    assert!(diff.has_next);
    assert_eq!(diff.summary.values, 30);
}

#[tokio::test]
async fn a_page_names_each_section_with_an_entry_outside_the_page() {
    let fixture = world();
    forty_limits_and_thirty_values(&fixture);
    let old: Vec<StoredParameter> = (71_u128..=90)
        .map(|n| param(n, 101, &format!("P{n:02}")))
        .collect();
    let renamed: Vec<StoredParameter> = old
        .iter()
        .map(|parameter| StoredParameter {
            name_en: text("Other"),
            ..copy(parameter)
        })
        .collect();
    seed(&fixture, &old, &renamed);

    let diff = run(&fixture, input(first_page())).await.unwrap().diff;

    assert_eq!(kinds(&diff), vec![DiffKind::Limits, DiffKind::Values]);
    assert_eq!(diff.cut, vec![DiffKind::Values, DiffKind::Definitions]);
    assert_eq!(diff.summary.definitions, 20);
}

#[tokio::test]
async fn a_page_of_one_section_holds_the_entries_of_that_section_only() {
    let fixture = world();
    forty_limits_and_thirty_values(&fixture);
    let values = |page| DiffChoice {
        kind: Some(DiffKind::Values),
        page,
    };

    let first = run(&fixture, input(values(0))).await.unwrap().diff;
    let after = run(&fixture, input(values(1))).await.unwrap().diff;

    assert_eq!(kinds(&first), vec![DiffKind::Values]);
    assert_eq!(section_keys(&first, DiffKind::Values), keys(41..=70));
    assert!(!first.has_next);
    assert!(first.cut.is_empty());
    assert!(after.sections.is_empty());
    assert!(!after.has_next);
    assert_eq!(after.kind, Some(DiffKind::Values));
    assert_eq!(after.page, 1);
}

#[tokio::test]
async fn a_page_after_the_last_page_is_an_empty_page_with_the_summary() {
    let fixture = world();
    forty_limits_and_thirty_values(&fixture);
    let choice = DiffChoice {
        kind: None,
        page: 2,
    };

    let diff = run(&fixture, input(choice)).await.unwrap().diff;

    assert!(diff.sections.is_empty());
    assert!(!diff.has_next);
    assert_eq!(diff.summary.limits, 40);
}

#[tokio::test]
async fn a_key_in_two_sections_counts_twice_on_the_pages() {
    let fixture = world();
    let old = param(1, 101, "A");
    let both = StoredParameter {
        value: StoredValue::from(json!(6)),
        ..with_new_limit(&old)
    };
    seed(&fixture, &[old], &[both]);
    let settings = AppSettings {
        pages: PageSizes {
            diff: 1,
            ..PageSizes::default()
        },
        ..AppSettings::default()
    };
    let page = |page| input(DiffChoice { kind: None, page });

    let context = fixture.context_with(settings);
    let first = run_with(context.clone(), Role::ProcessEngineer, page(0)).await;
    let second = run_with(context, Role::ProcessEngineer, page(1)).await;

    let (first, second) = (first.unwrap().diff, second.unwrap().diff);
    assert_eq!(kinds(&first), vec![DiffKind::Limits]);
    assert!(first.has_next);
    assert_eq!(kinds(&second), vec![DiffKind::Values]);
    assert!(!second.has_next);
}

#[tokio::test]
async fn a_version_against_itself_gives_the_empty_state() {
    let fixture = world();
    one_of_each(&fixture);
    let same = DiffVersionsInput {
        old_number: 2,
        ..input(first_page())
    };

    let diff = run(&fixture, same).await.unwrap().diff;

    assert!(diff.sections.is_empty());
    assert_eq!(
        diff.summary,
        DiffSummary {
            unchanged: 6,
            ..DiffSummary::default()
        }
    );
}

#[tokio::test]
async fn a_deleted_recipe_gives_rcp_012_except_for_the_administrator() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    seed_deleted(&fixture, 1, "R-1");
    let own = DiffVersionsInput {
        number: 1,
        ..input(first_page())
    };

    let engineer = run_with(fixture.context(), Role::ProcessEngineer, own.clone()).await;
    let admin = run_with(fixture.context(), Role::Administrator, own).await;

    assert_eq!(engineer.unwrap_err().code(), ErrorCode::RCP_012);
    assert!(admin.unwrap().diff.sections.is_empty());
}

#[tokio::test]
async fn an_unknown_version_number_gives_web_005() {
    let fixture = world();
    let unknown = DiffVersionsInput {
        old_number: 9,
        ..input(first_page())
    };

    let error = run(&fixture, unknown).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_005);
}

/// Return the settings with `blocking` and a gate of one permit.
pub(super) fn one_permit(blocking: Blocking) -> AppSettings {
    AppSettings {
        blocking,
        diff: DiffSettings {
            permits: 1,
            ..DiffSettings::default()
        },
        ..AppSettings::default()
    }
}

#[tokio::test]
async fn the_result_holds_the_permit_until_its_holder_drops_it() {
    let fixture = world();
    one_of_each(&fixture);
    let context = fixture.context_with(one_permit(Blocking::default()));

    let result = run_with(context.clone(), Role::Operator, input(first_page())).await;

    assert_eq!(context.diff_gate.available(), 0);
    drop(result.unwrap());
    assert_eq!(context.diff_gate.available(), 1);
}

#[tokio::test]
async fn a_busy_gate_beyond_the_deadline_gives_web_009_before_the_transaction() {
    let fixture = world();
    let short = Blocking::new(Duration::from_millis(20), Duration::from_secs(10));
    let context = fixture.context_with(one_permit(short));
    let far = Blocking::default().diff_deadline().unwrap();
    let held = context.diff_gate.acquire(far).await.unwrap();

    let error = run_with(context, Role::ProcessEngineer, input(first_page()))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Diff
        }
    ));
    assert!(fixture.store.journal().is_empty());
    drop(held);
}

#[tokio::test]
async fn a_failed_diff_task_gives_web_011_with_the_diff_stage() {
    let fixture = world();
    one_of_each(&fixture);
    let context = fixture.context_with(one_permit(Blocking::panicked()));

    let error = run_with(context.clone(), Role::ProcessEngineer, input(first_page()))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert!(matches!(
        error,
        AppError::TaskFailed {
            stage: TaskStage::Diff
        }
    ));
    assert_eq!(context.diff_gate.available(), 1);
}

/// The `timeout.validation` of the test of the rest of the deadline.
const LIMIT: Duration = Duration::from_millis(300);
/// The time that the gate holds its one permit: half of `LIMIT`.
const GATE_WAIT: Duration = Duration::from_millis(150);
/// The time after `GATE_WAIT` that the pool stays busy. The sum is 1.3 times `LIMIT`.
const POOL_WAIT: Duration = Duration::from_millis(240);

#[test]
fn the_diff_task_gets_only_the_rest_of_the_deadline() {
    one_blocking_thread().block_on(async {
        let fixture = world();
        one_of_each(&fixture);
        let context = fixture.context_with(one_permit(Blocking::new(LIMIT, LIMIT)));
        let far = Blocking::default().diff_deadline().unwrap();
        let held = context.diff_gate.acquire(far).await.unwrap();
        let (release, released) = std::sync::mpsc::channel::<()>();
        let busy = tokio::task::spawn_blocking(move || {
            released.recv_timeout(Duration::from_secs(5)).is_ok()
        });
        // A fresh deadline after the gate would end after the pool is free again.
        tokio::spawn(async move {
            tokio::time::sleep(GATE_WAIT).await;
            drop(held);
            tokio::time::sleep(POOL_WAIT).await;
            release.send(()).unwrap();
        });

        let error = run_with(context.clone(), Role::ProcessEngineer, input(first_page()))
            .await
            .unwrap_err();
        assert!(busy.await.unwrap());
        wait_for_free(&context.diff_gate, 1).await;

        assert_eq!(error.code(), ErrorCode::WEB_009);
        assert!(matches!(
            error,
            AppError::Timeout {
                stage: TimeoutStage::Diff
            }
        ));
        assert_eq!(context.diff_gate.available(), 1);
    });
}

#[tokio::test]
async fn a_type_that_does_not_resolve_gives_an_error_and_no_raw_text() {
    let fixture = world();
    let old = param(1, 101, "A");
    let broken = StoredParameter {
        plc_type: tid(9_999),
        ..with_new_value(&old)
    };
    seed(&fixture, &[old], &[broken]);

    let error = run(&fixture, input(first_page())).await.unwrap_err();

    assert!(matches!(error, AppError::Plant(_)), "{error:?}");
}

/// The struct `ST_Zone` in `revision` with the `INT` members `members`.
pub(super) fn zone(id: u128, revision: u32, supersedes: Option<u128>, members: &[&str]) -> PlcType {
    let members = members
        .iter()
        .map(|name| TypeMember {
            name: MemberName::parse(name).unwrap(),
            type_id: seed_type_id(Primitive::Int, None).unwrap(),
        })
        .collect();
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PlcPlatform::TwinCat3]),
        shape: RecordShape::Struct {
            name: TypeName::parse("ST_Zone").unwrap(),
            revision,
            supersedes: supersedes.map(tid),
            members,
        },
    })
    .unwrap()
}

#[tokio::test]
async fn each_side_resolves_its_own_struct_revision_and_names_its_type_and_unit() {
    let fixture = world();
    fixture.store.seed_type(&zone(901, 1, None, &["setpoint"]));
    fixture
        .store
        .seed_type(&zone(902, 2, Some(901), &["setpoint", "extra"]));
    let old = StoredParameter {
        plc_type: tid(901),
        value: StoredValue::from(json!({"setpoint": 1})),
        limits: Vec::new(),
        ..param(1, 101, "ZONE")
    };
    let new = StoredParameter {
        plc_type: tid(902),
        value: StoredValue::from(json!({"setpoint": 1, "extra": 2})),
        ..copy(&old)
    };
    seed(&fixture, &[old], &[new]);

    let diff = run(&fixture, input(first_page())).await.unwrap().diff;

    let paths: Vec<LeafPath> = diff
        .sections
        .iter()
        .filter(|section| section.kind == DiffKind::Values)
        .flat_map(|section| section.rows.iter())
        .filter_map(|row| match row.part() {
            DiffPart::Value { path, .. } => Some(path.clone()),
            _ => None,
        })
        .collect();
    let leaf = |text: &str| LeafPath::parse(text).unwrap();
    assert_eq!(paths, vec![leaf(".setpoint"), leaf(".extra")]);
    assert_eq!(
        diff.type_texts.get(&tid(901)).map(String::as_str),
        Some("ST_Zone@1")
    );
    assert_eq!(
        diff.type_texts.get(&tid(902)).map(String::as_str),
        Some("ST_Zone@2")
    );
    assert_eq!(
        diff.unit_paths.get(&uid(101)),
        Some(&UnitPath::parse("U1").unwrap())
    );
    assert_eq!(
        ParameterTag::parse("ZONE").unwrap(),
        diff.sections[0].rows[0].tag().clone()
    );
}

#[tokio::test]
async fn the_setting_leaf_rows_cuts_the_rows_of_one_parameter() {
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

    let context = fixture.context_with(settings);
    let diff = run_with(context, Role::ProcessEngineer, input(first_page()))
        .await
        .unwrap()
        .diff;

    let parts: Vec<&DiffPart> = diff
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
