//! The test of the full scenario: create, edit, approve, version 2, approve, retire, export.

use recipe_domain::recipe::ApprovalOutcome;
use recipe_domain::recipe::RecipeVersionId;
use recipe_domain::recipe::VersionState::{Approved, LiveDraft, Retired};
use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::chain::{
    ChainRow, approvals, assert_actions, assert_chain_recomputes, assert_no_gap,
    assert_one_transaction_per_command, bases, chain_rows, instants,
};
use super::scene::Scene;

/// The engineer creates and edits version 1, and the administrator approves it.
/// Return the identifier of version 1.
async fn version_one(scene: &mut Scene) -> RecipeVersionId {
    let version = scene
        .create_recipe(&["RecipeCreated", "VersionCreated"])
        .await;
    assert_eq!(scene.states().await, [(1, LiveDraft)]);
    scene.add_speed(&["ParameterAdded"]).await;
    assert_eq!(scene.states().await, [(1, LiveDraft)]);

    let approval = scene.approve(1, &["VersionApproved"]).await;

    let expected = ApprovalOutcome::Approved {
        approved: version,
        retired: None,
    };
    assert_eq!((approval.outcome, approval.retired), (expected, None));
    assert_eq!(scene.states().await, [(1, Approved)]);
    version
}

/// The engineer creates version 2 on version 1 and edits it. The administrator approves it,
/// which retires version 1. Then the administrator retires version 2.
/// Return the identifier of version 2.
async fn version_two(scene: &mut Scene, first: RecipeVersionId) -> RecipeVersionId {
    let created = scene.new_version(1, &["VersionCreated"]).await;
    assert_eq!((created.number, created.outdated.len()), (2, 0));
    assert_eq!(scene.states().await, [(2, LiveDraft), (1, Approved)]);
    scene.set_speed(2, "7", &["ParameterValueSet"]).await;
    assert_eq!(scene.states().await, [(2, LiveDraft), (1, Approved)]);

    let approval = scene
        .approve(2, &["VersionRetired", "VersionApproved"])
        .await;

    let expected = ApprovalOutcome::Approved {
        approved: created.version,
        retired: Some(first),
    };
    assert_eq!((approval.outcome, approval.retired), (expected, Some(1)));
    assert_eq!(scene.states().await, [(2, Approved), (1, Retired)]);
    scene.retire(2, &["VersionRetired"]).await;
    assert_eq!(scene.states().await, [(2, Retired), (1, Retired)]);
    created.version
}

/// The entity and the `occurred_at` of each row with `action`, with the instant as an `Option`.
///
/// The instant is an `Option` to match the nullable columns `approved_at` and `retired_at`.
fn optional_instants(rows: &[ChainRow], action: &str) -> Vec<(Uuid, Option<Timestamp>)> {
    instants(rows, action)
        .into_iter()
        .map(|(id, at)| (id, Some(at)))
        .collect()
}

/// Assert that the retirement of each version and its audit row come from the transaction of one
/// command.
///
/// Version 1 retires inside the approval of version 2. The administrator retires version 2 in a
/// later command. `retired_at` and `occurred_at` are each `now()` of their transaction.
async fn assert_retirements_share_the_audit_instants(
    scene: &mut Scene,
    rows: &[ChainRow],
    versions: [RecipeVersionId; 2],
) {
    let mut stored = Vec::new();
    for version in versions {
        let id = *version.as_uuid();
        let retired_at: Option<OffsetDateTime> =
            sqlx::query_scalar("SELECT retired_at FROM recipe_version WHERE id = $1")
                .bind(id)
                .fetch_one(scene.app.admin())
                .await
                .unwrap();
        stored.push((id, retired_at.map(Timestamp::from)));
    }
    assert_eq!(stored, optional_instants(rows, "VersionRetired"));
}

/// Assert that the insert of `SPEED` and its audit row come from the transaction of one command.
///
/// `created_at` of the parameter and `occurred_at` are each `now()` of their transaction. Only
/// `AddParameter` writes a `ParameterAdded` row.
/// `CreateVersion` copies the parameter into version 2 and writes no `ParameterAdded` row.
async fn assert_parameter_shares_the_audit_instant(
    scene: &mut Scene,
    rows: &[ChainRow],
    first: RecipeVersionId,
) {
    let stored: Vec<(Uuid, OffsetDateTime)> = sqlx::query_as(
        "SELECT id, created_at FROM parameter WHERE recipe_version_id = $1 AND tag = 'SPEED'",
    )
    .bind(*first.as_uuid())
    .fetch_all(scene.app.admin())
    .await
    .unwrap();
    let stored: Vec<(Uuid, Timestamp)> = stored
        .into_iter()
        .map(|(id, at)| (id, Timestamp::from(at)))
        .collect();
    assert_eq!(stored, instants(rows, "ParameterAdded"));
}

/// Assert that the first edit of the engineer on version 2 and its audit row come from the
/// transaction of one command.
///
/// `SetParameterValue` is the first edit of the engineer on version 2. `first_edit_at` of the
/// `draft_editor` row and `occurred_at` of the `ParameterValueSet` row are each `now()` of their
/// transaction. `CreateVersion` writes no editor row.
async fn assert_first_edit_shares_the_audit_instant(
    scene: &mut Scene,
    rows: &[ChainRow],
    second: RecipeVersionId,
) {
    let stored: Vec<(Uuid, OffsetDateTime)> = sqlx::query_as(
        "SELECT user_id, first_edit_at FROM draft_editor WHERE recipe_version_id = $1",
    )
    .bind(*second.as_uuid())
    .fetch_all(scene.app.admin())
    .await
    .unwrap();
    let stored: Vec<(Uuid, Timestamp)> = stored
        .into_iter()
        .map(|(user, at)| (user, Timestamp::from(at)))
        .collect();
    let edits: Vec<(Uuid, Timestamp)> = rows
        .iter()
        .filter(|row| row.action == "ParameterValueSet")
        .map(|row| (row.actor_user_id.unwrap(), Timestamp::from(row.occurred_at)))
        .collect();
    assert_eq!(stored, edits);
}

/// Assert that the insert and the approval of each version and their audit rows come from the
/// transaction of one command.
///
/// `created_at`, `approved_at`, and `occurred_at` are each `now()` of their transaction. The
/// instants of two commands differ, so an equal instant ties the version write to the audit write.
async fn assert_versions_share_the_audit_instants(
    scene: &Scene,
    rows: &[ChainRow],
    versions: [RecipeVersionId; 2],
) {
    let summaries: Vec<_> = scene.versions().await.into_iter().rev().collect();
    let numbers: Vec<u32> = summaries.iter().map(|row| row.number).collect();
    assert_eq!(numbers, [1, 2]);
    let ids = versions.map(|version| *version.as_uuid());
    let created: Vec<(Uuid, Timestamp)> = ids
        .into_iter()
        .zip(summaries.iter().map(|row| row.created_at))
        .collect();
    let approved: Vec<(Uuid, Option<Timestamp>)> = ids
        .into_iter()
        .zip(summaries.iter().map(|row| row.approved_at))
        .collect();
    assert_eq!(created, instants(rows, "VersionCreated"));
    assert_eq!(approved, optional_instants(rows, "VersionApproved"));
}

#[tokio::test]
async fn a_recipe_lives_from_create_to_retire_on_one_gapless_audit_chain() {
    let mut scene = Scene::create().await;
    let first = version_one(&mut scene).await;
    let second = version_two(&mut scene, first).await;
    let [first_hash, second_hash] = [
        scene.export(1, &["VersionExported"]).await,
        scene.export(2, &["VersionExported"]).await,
    ];

    let rows = chain_rows(&mut scene.app).await;

    assert_actions(&rows, &scene.steps);
    assert_no_gap(&rows);
    assert_one_transaction_per_command(&rows, &scene.steps);
    assert_chain_recomputes(&mut scene.app, rows.len()).await;
    assert_ne!(first_hash, second_hash);
    assert_eq!(
        approvals(&rows),
        [
            (*first.as_uuid(), Some(first_hash)),
            (*second.as_uuid(), Some(second_hash)),
        ]
    );
    assert_eq!(
        bases(&rows),
        [
            (*first.as_uuid(), None),
            (*second.as_uuid(), Some(*first.as_uuid())),
        ]
    );
    assert_versions_share_the_audit_instants(&scene, &rows, [first, second]).await;
    assert_retirements_share_the_audit_instants(&mut scene, &rows, [first, second]).await;
    assert_parameter_shares_the_audit_instant(&mut scene, &rows, first).await;
    assert_first_edit_shares_the_audit_instant(&mut scene, &rows, second).await;
}
