use recipe_domain::error_codes::ErrorCode;

use super::{DraftTarget, MachineLock, OpenDraft};
use crate::memory::fixtures::{Fixture, lock_line, mid, reference, rid, vid};
use crate::memory::{JournalEntry, LockMode};
use crate::ports::Isolation;
use crate::recipe::test_support::{DRAFT_VERSION, seed_speed_draft};

fn target(expected: i64) -> (recipe_domain::recipe::ReferenceNumber, i64) {
    (reference("R-1"), expected)
}

#[tokio::test]
async fn a_shared_machine_lock_comes_before_the_recipe_lock() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let (reference, expected) = target(1);
    let draft_target = DraftTarget {
        reference: &reference,
        number: 1,
        expected_row_version: expected,
    };
    let open = OpenDraft::open(&fixture.context(), draft_target, MachineLock::Shared)
        .await
        .unwrap();
    assert_eq!(open.recipe, rid(1));
    assert_eq!(open.draft.row_version(), 1);
    assert_eq!(
        fixture.store.journal(),
        vec![
            JournalEntry::Begin(Isolation::ReadOnly),
            JournalEntry::Begin(Isolation::ReadCommitted),
            lock_line("machine", LockMode::ForShare, mid(1).to_string()),
            lock_line("recipe", LockMode::ForUpdate, rid(1).to_string()),
            lock_line(
                "recipe_version",
                LockMode::ForUpdate,
                vid(DRAFT_VERSION).to_string()
            ),
        ]
    );
}

#[tokio::test]
async fn the_trees_cover_the_type_of_every_parameter() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let (reference, expected) = target(1);
    let draft_target = DraftTarget {
        reference: &reference,
        number: 1,
        expected_row_version: expected,
    };
    let open = OpenDraft::open(&fixture.context(), draft_target, MachineLock::None)
        .await
        .unwrap();
    let types = open.draft.parameters();
    let parameter = types.first().unwrap();
    assert!(open.trees.tree(parameter.plc_type()).is_some());
}

#[tokio::test]
async fn a_touched_draft_commits_with_the_row_version_of_the_touch() {
    use recipe_domain::audit::DiffAction;

    use super::parameter_diff_entry;
    use crate::audit::StateDiff;
    use crate::memory::fixtures::{meta, second_engineer};
    use crate::ports::NonEmpty;

    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let (reference, expected) = target(1);
    let draft_target = DraftTarget {
        reference: &reference,
        number: 1,
        expected_row_version: expected,
    };
    let context = fixture.context();
    let user = second_engineer();
    let open = OpenDraft::open(&context, draft_target, MachineLock::None)
        .await
        .unwrap();
    let parameter = open.parameter_id(crate::recipe::test_support::speed_key());
    let parameter = parameter.unwrap();
    let recipe = open.recipe;
    let touched = open.touch(&user).await.unwrap();
    let entry = parameter_diff_entry(
        DiffAction::ParameterValueSet,
        (recipe, parameter),
        StateDiff {
            before: serde_json::json!({"value": 1}),
            after: serde_json::json!({"value": 2}),
        },
        (&user, &meta()),
    );
    let output = touched
        .commit(&context, NonEmpty::new(entry))
        .await
        .unwrap();
    assert_eq!(output.row_version, 2);
    assert_eq!(output.version, vid(DRAFT_VERSION));
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn a_stale_row_version_fails_before_any_write() {
    let fixture = Fixture::new();
    seed_speed_draft(&fixture);
    let (reference, _) = target(1);
    let draft_target = DraftTarget {
        reference: &reference,
        number: 1,
        expected_row_version: 4,
    };
    let error = OpenDraft::open(&fixture.context(), draft_target, MachineLock::None)
        .await
        .err()
        .unwrap();
    assert_eq!(error.code(), ErrorCode::WEB_001);
}
