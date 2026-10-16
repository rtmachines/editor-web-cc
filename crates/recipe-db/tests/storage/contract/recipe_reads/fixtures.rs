//! Fixtures of the contract cases of `RecipeReads` (design D5).
//!
//! Every seed goes through the ports and `UnitOfWork::commit` on both adapters (design D13). The
//! reads open a `RepeatableRead` transaction, as a query of the recipe pages does (ADR-0042).

use recipe_app::identity::new_request_id;
use recipe_app::ports::{Isolation, NonEmpty, RecipeFilter, RecipeSummary, Transaction};
use recipe_domain::audit::{AuditContext, AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::identity::{Role, UserId};
use recipe_domain::plant::{MachineId, Primitive, UnitId};
use recipe_domain::recipe::{
    Ordinal, Recipe, RecipeId, RecipeVersionId, ReferenceNumber, StoredParameter,
};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;
use serde_json::json;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::contract::harness::{Harness, begin, commit, fixture_hash, new_user, seed_user};
use crate::contract::recipe_seed::{Plant, first_version, parameter, recipe_id, version_id};
use crate::contract::seed::{insert, machine_with_tree, seed_type, unit_id};

/// The page that holds every row of a case.
pub const WHOLE: (u64, u32) = (0, 100);

/// Open the transaction of a query of the recipe pages.
pub async fn begin_read(h: &dyn Harness) -> Box<dyn Transaction> {
    h.unit_of_work()
        .begin(Isolation::RepeatableRead)
        .await
        .unwrap()
}

/// The machine that an audit entry names when the seed has no machine at hand.
fn any_machine() -> MachineId {
    MachineId::from(Uuid::nil())
}

/// Read one page of the recipes in a transaction that nothing commits.
pub async fn list(
    h: &dyn Harness,
    filter: &RecipeFilter,
    (offset, limit): (u64, u32),
) -> Vec<RecipeSummary> {
    let mut tx = begin_read(h).await;
    tx.recipe_reads()
        .list_recipes(filter, offset, limit)
        .await
        .unwrap()
}

/// Return the reference numbers of the rows as text, in the order of the rows.
pub fn references(rows: &[RecipeSummary]) -> Vec<String> {
    rows.iter().map(|row| row.reference.to_string()).collect()
}

/// Seed a recipe with its name and its first version through `insert_recipe`.
pub async fn create_named(
    h: &dyn Harness,
    plant: &Plant,
    n: u128,
    reference: &str,
    name: &str,
) -> Recipe {
    let recipe = Recipe::create(
        recipe_id(n),
        plant.machine,
        ReferenceNumber::parse(reference).unwrap(),
        Text::parse(name).unwrap(),
        version_id(n),
    );
    let mut tx = begin(h).await;
    tx.recipes()
        .insert_recipe(&recipe, first_version(h, &recipe))
        .await
        .unwrap();
    commit(h, tx, recipe.machine()).await;
    recipe
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

/// Discard a live draft with the port sequence of `DiscardDraft`.
pub async fn discard(h: &dyn Harness, version: RecipeVersionId) {
    let mut tx = begin(h).await;
    tx.recipes()
        .lock_version(version)
        .await
        .unwrap()
        .expect("the version exists");
    tx.recipes().discard_version(version).await.unwrap();
    commit(h, tx, any_machine()).await;
}

/// Approve a live draft as `approver` with the port sequence of `ApproveVersion`.
pub async fn approve_as(h: &dyn Harness, version: RecipeVersionId, approver: UserId) {
    let mut tx = begin(h).await;
    tx.recipes()
        .lock_version(version)
        .await
        .unwrap()
        .expect("the version exists");
    tx.recipes()
        .approve_version(version, approver, fixture_hash())
        .await
        .unwrap();
    commit(h, tx, any_machine()).await;
}

/// Insert a second user through `UserRepository::insert` and return its identifier.
pub async fn second_user(h: &dyn Harness, login: &str) -> UserId {
    let user = new_user(login, Role::ProcessEngineer);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    user.id
}

/// An audit entry of a change of `version` with `actor`, as a draft command writes it.
fn version_entry(recipe: RecipeId, version: RecipeVersionId, actor: UserId) -> AuditEntry {
    AuditEntry::row(
        RowAction::VersionCreated,
        AuditTarget::recipe_version(recipe, version),
        RowChange::Create { after: json!({}) },
        AuditContext {
            actor: Some(actor),
            request_id: new_request_id().unwrap(),
            client_ip: None,
            session_ref: None,
        },
    )
}

/// Touch a draft and commit one entry for each actor, in the order of `actors`.
///
/// The entries name the recipe as their aggregate, so `last_change` can find them.
pub async fn touch_by(
    h: &dyn Harness,
    (recipe, version): (RecipeId, RecipeVersionId),
    actors: &[UserId],
) {
    let mut tx = begin(h).await;
    tx.drafts()
        .lock_draft(version)
        .await
        .unwrap()
        .expect("the draft exists");
    tx.drafts().touch_draft(version, h.user()).await.unwrap();
    let mut entries = NonEmpty::new(version_entry(recipe, version, h.user()));
    for actor in actors {
        entries.push(version_entry(recipe, version, *actor));
    }
    h.unit_of_work().commit(tx, entries).await.unwrap();
}

/// A whole-second instant in 2001. No seed of the suite writes a row at that time.
pub fn far_past() -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(1_000_000_000).unwrap())
}

/// Insert the machine with the unit tree of `machine_with_tree` and find its `DINT`.
pub async fn tree_plant(h: &dyn Harness, n: u128, code: &str) -> Plant {
    let built = machine_with_tree(n, code);
    insert(h, &built).await;
    Plant {
        machine: built.id(),
        unit: unit_id(n.checked_mul(10).unwrap()),
        dint: seed_type(h, built.id(), Primitive::Dint).await,
    }
}

/// A parameter of the type `DINT` in `unit` with `ordinal`. The number `n` sets the identifiers.
pub fn in_unit(plant: &Plant, n: u128, tag: &str, unit: UnitId, ordinal: u32) -> StoredParameter {
    let mut built = parameter(plant, n, tag);
    built.unit = unit;
    built.ordinal = Ordinal::parse(ordinal).unwrap();
    built
}
