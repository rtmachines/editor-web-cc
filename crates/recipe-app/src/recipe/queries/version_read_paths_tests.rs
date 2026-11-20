//! Tests of the version read paths that take the STRING encoding of the machine (ADR-0202 (m)).
//!
//! The machine `M1` holds `Utf8`. The version 1 is approved and the version 2 is a live draft.
//! Both hold the parameter `LABEL` of type `STRING(10)` with a value outside Latin-1.

use recipe_domain::identity::Role;

use super::approval_preview::GetApprovalPreview;
use super::approval_preview_tests::{admin, input as preview_input};
use super::diff::{DiffVersions, DiffVersionsInput};
use super::diff_page::DiffChoice;
use super::test_world::seed_users;
use super::versions::{GetVersion, GetVersionInput};
use crate::memory::fixtures::{Fixture, actor, meta, reference};
use crate::recipe::test_support::{UTF8_LABEL, seed_utf8_label, utf8_label_fixture};

/// A `Utf8` world with the approved version 1 (key 101) and the live draft 2 (key 102).
fn utf8_world(draft_label: &str) -> Fixture {
    let (fixture, machine) = utf8_label_fixture();
    seed_users(&fixture);
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    seed_utf8_label(&fixture, 101, UTF8_LABEL);
    seed_utf8_label(&fixture, 102, draft_label);
    fixture
}

#[tokio::test]
async fn adr_0202_version_read_paths_get_version_checks_an_approved_version_as_utf8() {
    let fixture = utf8_world(UTF8_LABEL);

    let view = GetVersion::new(fixture.context())
        .execute(
            &actor(Role::Operator),
            &meta(),
            GetVersionInput {
                reference: reference("R-1"),
                number: 1,
                rebind_bound: 0,
            },
        )
        .await
        .unwrap();

    assert!(view.rule_codes.is_empty(), "{:?}", view.rule_codes);
}

#[tokio::test]
async fn adr_0202_version_read_paths_diff_resolves_the_types_as_utf8() {
    let fixture = utf8_world("Łódź!");

    let diff = DiffVersions::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            DiffVersionsInput {
                reference: reference("R-1"),
                number: 2,
                old_number: 1,
                choice: DiffChoice {
                    kind: None,
                    page: 0,
                },
            },
        )
        .await
        .unwrap();

    assert_eq!(diff.diff.summary.values, 1, "{:?}", diff.diff.summary);
}

#[tokio::test]
async fn adr_0202_version_read_paths_approval_preview_checks_the_draft_as_utf8() {
    let fixture = utf8_world(UTF8_LABEL);

    let preview = GetApprovalPreview::new(fixture.context())
        .execute(&admin(), &meta(), preview_input(2))
        .await
        .unwrap();

    assert!(
        preview.draft_blocker.is_none(),
        "{:?}",
        preview.draft_blocker
    );
    assert!(preview.may_approve);
}
