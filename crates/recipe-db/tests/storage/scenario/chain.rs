//! Reads of the audit chain after the scenario, and the checks over the rows.
//!
//! The superuser of the harness reads `audit_log`, because `audit_owner` owns the audit tables.

use std::collections::BTreeSet;

use recipe_domain::timestamp::Timestamp;
use time::OffsetDateTime;
use uuid::Uuid;

use super::scene::Step;
use crate::support::app::App;
use crate::triggers::hash_chain::check;

/// The columns of one audit row that the checks compare.
#[derive(Debug, sqlx::FromRow)]
pub struct ChainRow {
    /// The position of the row in the chain.
    pub chain_seq: i64,
    /// The audit action, for example `VersionApproved`.
    pub action: String,
    /// The user who sent the command, or `None` for a system entry.
    pub actor_user_id: Option<Uuid>,
    /// The request identifier of the command.
    pub request_id: Uuid,
    /// The instant of the transaction that wrote the row.
    pub occurred_at: OffsetDateTime,
    /// The identifier of the entity that the row describes.
    pub entity_id: Uuid,
    /// The field `content_sha256` of `after_state`, when the row has it.
    pub content_sha256: Option<String>,
    /// The field `based_on_version_id` of `after_state`, when the row has it.
    pub based_on: Option<Uuid>,
}

/// Read every audit row in the order of the chain.
pub async fn chain_rows(app: &mut App) -> Vec<ChainRow> {
    sqlx::query_as(
        "SELECT chain_seq, action, actor_user_id, request_id, occurred_at, entity_id, \
             after_state ->> 'content_sha256' AS content_sha256, \
             (after_state ->> 'based_on_version_id')::uuid AS based_on \
         FROM audit_log ORDER BY chain_seq",
    )
    .fetch_all(app.admin())
    .await
    .unwrap()
}

/// Assert that the chain holds the actions of the steps, in the order of the steps.
pub fn assert_actions(rows: &[ChainRow], steps: &[Step]) {
    let stored: Vec<&str> = rows.iter().map(|row| row.action.as_str()).collect();
    let expected: Vec<&str> = steps
        .iter()
        .flat_map(|step| step.actions.iter().copied())
        .collect();
    assert_eq!(stored, expected);
}

/// Assert that `chain_seq` starts at 1 and has no gap.
pub fn assert_no_gap(rows: &[ChainRow]) {
    let stored: Vec<i64> = rows.iter().map(|row| row.chain_seq).collect();
    let expected: Vec<i64> = (1..=i64::try_from(rows.len()).unwrap()).collect();
    assert_eq!(stored, expected);
}

/// Split the rows into one group for each step. Call `assert_actions` first.
fn groups<'a>(rows: &'a [ChainRow], steps: &[Step]) -> Vec<&'a [ChainRow]> {
    let mut rest = rows;
    let mut groups = Vec::new();
    for step in steps {
        let (group, tail) = rest.split_at(step.actions.len());
        groups.push(group);
        rest = tail;
    }
    assert!(rest.is_empty(), "rows without a step: {rest:?}");
    groups
}

/// Assert one transaction for each command.
///
/// The rows of one command share its request, its actor, and one `occurred_at`. `occurred_at`
/// is `now()` of the transaction, so two commands never share it. It increases from command to
/// command.
pub fn assert_one_transaction_per_command(rows: &[ChainRow], steps: &[Step]) {
    let mut instants = Vec::new();
    for (step, group) in steps.iter().zip(groups(rows, steps)) {
        let first = group.first().unwrap();
        for row in group {
            assert_eq!(row.request_id, step.request, "{row:?}");
            assert_eq!(row.actor_user_id, Some(step.actor), "{row:?}");
            assert_eq!(row.occurred_at, first.occurred_at, "{row:?}");
        }
        instants.push(first.occurred_at);
    }
    let requests: BTreeSet<Uuid> = steps.iter().map(|step| step.request).collect();
    assert_eq!(requests.len(), steps.len());
    assert!(
        instants.windows(2).all(|pair| pair[0] < pair[1]),
        "{instants:?}"
    );
}

/// Assert that the batch statement of the nightly check recomputes every `entry_hash`.
///
/// The statement also follows each `prev_hash` to the row before it. Its last row must be the
/// head of the chain.
pub async fn assert_chain_recomputes(app: &mut App, rows: usize) {
    let expected = i64::try_from(rows).unwrap();
    let head = app.head().await;
    let (first_break, count, last_seq, last_hash) = check(app.admin()).await;
    assert_eq!(first_break, None);
    assert_eq!(count, expected);
    assert_eq!(last_seq, Some(expected));
    assert_eq!(head.last_seq, expected);
    assert_eq!(last_hash, head.last_hash);
}

/// The version and the `content_sha256` of each `VersionApproved` row, in the order of the chain.
pub fn approvals(rows: &[ChainRow]) -> Vec<(Uuid, Option<String>)> {
    rows.iter()
        .filter(|row| row.action == "VersionApproved")
        .map(|row| (row.entity_id, row.content_sha256.clone()))
        .collect()
}

/// The version and its base of each `VersionCreated` row, in the order of the chain.
pub fn bases(rows: &[ChainRow]) -> Vec<(Uuid, Option<Uuid>)> {
    rows.iter()
        .filter(|row| row.action == "VersionCreated")
        .map(|row| (row.entity_id, row.based_on))
        .collect()
}

/// The entity and the `occurred_at` of each row with `action`, in the order of the chain.
pub fn instants(rows: &[ChainRow], action: &str) -> Vec<(Uuid, Timestamp)> {
    rows.iter()
        .filter(|row| row.action == action)
        .map(|row| (row.entity_id, Timestamp::from(row.occurred_at)))
        .collect()
}
