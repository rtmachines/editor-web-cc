//! The blockers of `GetApprovalPreview` against the errors of `ApproveVersion` (ADR-0049 (c), design D3).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{PlcPlatform, UnitPath, UnitUsage};
use recipe_domain::recipe::StoredParameter;

use super::approval_preview::ApprovalPreview;
use super::approval_preview_tests::{admin, keys, preview_as, valid_world};
use crate::error::AppError;
use crate::memory::fixtures::{
    Fixture, meta, param, parameter_on, reference, tid, timestamp, uid, vid,
};
use crate::recipe::{ApproveVersion, ApproveVersionInput};

/// The approved version 1 of `R-1` in [`valid_world`].
const OLD: u128 = 101;
/// The live draft version 2 of `R-1` in [`valid_world`].
const NEW: u128 = 102;
/// A type identifier without a type row.
const NO_TYPE: u128 = 9_999;

/// Return the error of `ApproveVersion` for draft `number` on the state of `fixture`.
async fn approve_error(fixture: &Fixture, number: u32) -> AppError {
    let approve = ApproveVersionInput {
        reference: reference("R-1"),
        number,
        expected_row_version: 1,
        allow_outdated: true,
    };
    ApproveVersion::new(fixture.context())
        .execute(&admin(), &meta(), approve)
        .await
        .unwrap_err()
}

/// Assert that the draft blocker of the preview equals the error of `ApproveVersion`.
async fn assert_same_fault(fixture: &Fixture, number: u32, code: ErrorCode) -> ApprovalPreview {
    let preview = preview_as(fixture, &admin(), number).await.unwrap();
    let error = approve_error(fixture, number).await;

    let blocker = preview.draft_blocker.as_ref().unwrap();
    assert_eq!(error.code(), code, "{error:?}");
    assert_eq!(blocker.code(), code, "{blocker:?}");
    assert_eq!(blocker.to_string(), error.to_string());
    assert!(!preview.may_approve);
    preview
}

/// Store a machine of `platform` and the live draft 1 of `R-1` with `parameters`.
fn first_draft(platform: PlcPlatform, parameters: &[StoredParameter]) -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(platform, 1, "M1");
    let (_, draft) = fixture.seed_draft(&machine, 1, "R-1");
    for parameter in parameters {
        fixture.store.seed_parameter(draft, parameter);
    }
    fixture
}

/// The draft of [`first_draft`] with one parameter on the archived unit `U1.2.1`.
fn archived_unit() -> Fixture {
    archived_unit_with(&["T1"])
}

/// The draft of [`first_draft`] with one parameter for each tag on the archived unit `U1.2.1`.
///
/// The parameter of the tag at position `i` has the fixture number `i + 1`.
fn archived_unit_with(tags: &[&str]) -> Fixture {
    let platform = PlcPlatform::TwinCat3;
    let fixture = Fixture::new();
    let mut machine = fixture.seed_machine_on(platform, 1, "M1");
    let (_, draft) = fixture.seed_draft(&machine, 1, "R-1");
    for (n, tag) in (1..).zip(tags) {
        let on_unit = parameter_on(platform, n, tag, uid(103), 0);
        fixture.store.seed_parameter(draft, &on_unit);
    }
    machine
        .archive_unit(
            &UnitPath::parse("U1.2.1").unwrap(),
            timestamp(),
            &UnitUsage::default(),
        )
        .unwrap();
    fixture.store.seed_machine(&machine);
    fixture
}

#[tokio::test]
async fn the_blockers_equal_the_errors_of_approve_version_for_each_draft_fault() {
    let s7 = PlcPlatform::S7_300;
    let cases = [
        (first_draft(PlcPlatform::TwinCat3, &[]), ErrorCode::RCP_008),
        (archived_unit(), ErrorCode::RCP_020),
        (
            first_draft(
                s7,
                &[
                    parameter_on(s7, 1, "T1", uid(102), 0),
                    parameter_on(s7, 2, "T2", uid(102), 0),
                ],
            ),
            ErrorCode::RCP_022,
        ),
        (
            first_draft(s7, &[parameter_on(s7, 1, "T1", uid(102), 1)]),
            ErrorCode::RCP_023,
        ),
    ];

    for (fixture, code) in cases {
        let preview = assert_same_fault(&fixture, 1, code).await;
        // A diff that succeeds stays on the page next to the blockers (design D3).
        assert!(preview.diff.is_some(), "{code:?}");
    }
}

#[tokio::test]
async fn the_blocker_keys_hold_the_key_of_each_tag_that_the_draft_blocker_names() {
    let s7 = PlcPlatform::S7_300;
    let overlap = first_draft(
        s7,
        &[
            parameter_on(s7, 1, "T1", uid(102), 0),
            parameter_on(s7, 2, "T2", uid(102), 0),
        ],
    );
    let cases = [
        (overlap, ErrorCode::RCP_022, keys(&[("T1", 1), ("T2", 2)])),
        (
            archived_unit_with(&["T1", "T2"]),
            ErrorCode::RCP_020,
            keys(&[("T1", 1), ("T2", 2)]),
        ),
        (
            first_draft(PlcPlatform::TwinCat3, &[]),
            ErrorCode::RCP_008,
            keys(&[]),
        ),
    ];

    for (fixture, code, expected) in cases {
        let preview = assert_same_fault(&fixture, 1, code).await;
        assert_eq!(preview.blocker_keys, expected, "{code:?}");
    }
}

#[tokio::test]
async fn a_draft_type_that_does_not_resolve_gives_the_blockers_and_no_diff() {
    let fixture = valid_world();
    let broken = StoredParameter {
        plc_type: tid(NO_TYPE),
        ..param(2, 101, "BROKEN")
    };
    fixture.store.seed_parameter(vid(NEW), &broken);

    let preview = assert_same_fault(&fixture, 2, ErrorCode::RCP_020).await;

    assert!(preview.diff.is_none());
    assert!(preview.outdated.is_empty());
}

#[tokio::test]
async fn a_unit_outside_the_machine_gives_the_blockers_and_no_diff() {
    let fixture = valid_world();
    // The unit 201 is the root unit of the machine M2.
    fixture.seed_machine_on(PlcPlatform::TwinCat3, 2, "M2");
    fixture
        .store
        .seed_parameter(vid(NEW), &param(2, 201, "STRAY"));

    let preview = assert_same_fault(&fixture, 2, ErrorCode::RCP_020).await;

    assert!(preview.diff.is_none());
    assert!(preview.outdated.is_empty());
}

#[tokio::test]
async fn a_failed_diff_without_a_blocker_gives_the_error_of_the_diff() {
    let fixture = valid_world();
    let gone = StoredParameter {
        plc_type: tid(NO_TYPE),
        ..param(2, 101, "GONE")
    };
    fixture.store.seed_parameter(vid(OLD), &gone);

    let error = preview_as(&fixture, &admin(), 2).await.unwrap_err();

    assert!(matches!(error, AppError::Plant(_)), "{error:?}");
}
