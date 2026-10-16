//! Fixtures of the contract cases of `AuditReads` (ADR-0051 (a), design D5).
//!
//! Every seed goes through the ports and `UnitOfWork::commit` on both adapters (design D13). The
//! reads open a `ReadOnly` transaction, as the audit queries do (ADR-0042 (c)).

use std::net::{IpAddr, Ipv4Addr};

use recipe_app::identity::new_request_id;
use recipe_app::ports::{
    AuditEntryId, AuditFilter, AuditRecord, AuditRow, AuditScope, CursorPage, EntityId, Isolation,
    NonEmpty, Transaction,
};
use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange, SessionRef,
};
use recipe_domain::identity::{Role, UserId};
use recipe_domain::recipe::{Recipe, RecipeId, RecipeVersionId};
use recipe_domain::timestamp::Timestamp;
use serde_json::json;
use time::{Duration, Time};
use uuid::Uuid;

use crate::contract::harness::{Harness, begin, commit, new_user, seed_user};

/// The page size that holds every row of a case.
pub const WHOLE: u32 = 100;

/// The base of the aggregate ids that no seed row uses. A case adds its own number.
const FREE_AGGREGATE_BASE: u128 = 0xa0d1_0000_0000_0000_0000_0000_0000_0000;

/// Return an id that no row of the store uses. A larger `n` gives a larger id.
pub fn free_id(n: u128) -> Uuid {
    Uuid::from_u128(FREE_AGGREGATE_BASE | n)
}

/// Return the client address `10.0.0.n`.
pub fn address(n: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(10, 0, 0, n))
}

/// Open the transaction of an audit query.
pub async fn begin_read(h: &dyn Harness) -> Box<dyn Transaction> {
    h.unit_of_work().begin(Isolation::ReadOnly).await.unwrap()
}

/// The context of an entry with `actor` and no trace fields.
pub fn context(actor: Option<UserId>) -> AuditContext {
    AuditContext {
        actor,
        request_id: new_request_id().unwrap(),
        client_ip: None,
        session_ref: None,
    }
}

/// The context of an entry with `actor`, the request `request`, a client address, and a session.
pub fn traced_context(actor: UserId, request: RequestId) -> AuditContext {
    AuditContext {
        actor: Some(actor),
        request_id: request,
        client_ip: Some(address(7)),
        session_ref: Some(SessionRef::parse("0123456789abcdef").unwrap()),
    }
}

/// An entry of `action` on `target` with a create state `{"n": n}`.
pub fn entry(action: RowAction, target: AuditTarget, actor: Option<UserId>, n: u64) -> AuditEntry {
    entry_in(action, target, context(actor), n)
}

/// An entry of `action` on `target` in `context` with a create state `{"n": n}`.
pub fn entry_in(
    action: RowAction,
    target: AuditTarget,
    context: AuditContext,
    n: u64,
) -> AuditEntry {
    AuditEntry::row(
        action,
        target,
        RowChange::Create {
            after: json!({ "n": n }),
        },
        context,
    )
}

/// Commit `entries` in one transaction without another write.
pub async fn write(h: &dyn Harness, entries: Vec<AuditEntry>) {
    let mut items = entries.into_iter();
    let mut list = NonEmpty::new(items.next().expect("a case writes an entry"));
    for item in items {
        list.push(item);
    }
    let tx = begin(h).await;
    h.unit_of_work().commit(tx, list).await.unwrap();
}

/// Read one page of the audit list in a transaction that nothing commits.
pub async fn list(
    h: &dyn Harness,
    filter: &AuditFilter,
    scope: AuditScope,
    before: Option<AuditEntryId>,
    limit: u32,
) -> CursorPage<AuditRow> {
    let mut tx = begin_read(h).await;
    tx.audit_reads()
        .entries(filter, scope, before, limit)
        .await
        .unwrap()
}

/// Read the entry `id` in a transaction that nothing commits.
pub async fn read_entry(
    h: &dyn Harness,
    id: AuditEntryId,
    scope: AuditScope,
) -> Option<AuditRecord> {
    let mut tx = begin_read(h).await;
    tx.audit_reads().entry(id, scope).await.unwrap()
}

/// Read one page of the history of `recipe` in a transaction that nothing commits.
pub async fn history(
    h: &dyn Harness,
    recipe: RecipeId,
    before: Option<AuditEntryId>,
    limit: u32,
) -> CursorPage<AuditRow> {
    let mut tx = begin_read(h).await;
    tx.audit_reads()
        .history(recipe, before, limit)
        .await
        .unwrap()
}

/// Return the ids of the rows, in the order of the rows.
pub fn ids(rows: &[AuditRow]) -> Vec<AuditEntryId> {
    rows.iter().map(|row| row.id).collect()
}

/// The bit that separates the version ids of a case from its recipe ids.
const VERSION_BIT: u128 = 0x1_0000;

/// The target of the version number `n` of `recipe`. The id of the version comes from `n`.
pub fn version_target(recipe: RecipeId, n: u128) -> AuditTarget {
    AuditTarget::recipe_version(recipe, RecipeVersionId::from(free_id(VERSION_BIT | n)))
}

/// The id of the version number `n` as the filter `entity_id` takes it.
pub fn version_entity(n: u128) -> EntityId {
    EntityId::from(free_id(VERSION_BIT | n))
}

/// Return the version numbers of the rows of `version_target`, in the order of the rows.
pub fn numbers(rows: &[AuditRow]) -> Vec<u128> {
    rows.iter()
        .map(|row| row.entity_id.as_uuid().as_u128() & 0xffff)
        .collect()
}

/// The recipe id of a case. No recipe row has it.
pub fn recipe(n: u128) -> RecipeId {
    RecipeId::from(free_id(n))
}

/// Return the instant `at` moved by `micros` microseconds.
pub fn shifted(at: Timestamp, micros: i64) -> Timestamp {
    Timestamp::from(
        at.as_offset_date_time()
            .checked_add(Duration::microseconds(micros))
            .unwrap(),
    )
}

/// Return the start of the day of `at` in UTC, moved by `days` days.
pub fn day_start(at: Timestamp, days: i64) -> Timestamp {
    let midnight = at.as_offset_date_time().replace_time(Time::MIDNIGHT);
    Timestamp::from(midnight.checked_add(Duration::days(days)).unwrap())
}

/// The filter of one aggregate.
pub fn of_aggregate(aggregate: Uuid) -> AuditFilter {
    AuditFilter {
        aggregate: Some(recipe_app::ports::AggregateId::from(aggregate)),
        ..AuditFilter::default()
    }
}

/// Insert a second user through `UserRepository::insert` and return its identifier.
pub async fn second_user(h: &dyn Harness, login: &str) -> UserId {
    let user = new_user(login, Role::ProcessEngineer);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    user.id
}

/// Mark a recipe deleted with the lock of `DeleteRecipe`.
pub async fn delete_recipe(h: &dyn Harness, recipe: &Recipe) {
    let mut tx = begin(h).await;
    tx.recipes()
        .lock_recipe(recipe.id())
        .await
        .unwrap()
        .expect("the recipe exists");
    tx.recipes().mark_recipe_deleted(recipe.id()).await.unwrap();
    commit(h, tx, recipe.machine()).await;
}
