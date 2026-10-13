//! Steps 2 to 7 of design D10 for the eight draft commands (`data-model.md` section 6).

use recipe_domain::audit::{AuditEntry, AuditTarget, DiffAction};
use recipe_domain::plant::{PlatformTypes, TypeTrees};
use recipe_domain::recipe::{
    Draft, Optimistic, ParameterId, ParameterKey, RecipeError, RecipeId, RecipeVersionId,
    ReferenceNumber,
};

use super::resolve::{ResolvedVersion, resolve_version};
use crate::audit::StateDiff;
use crate::auth::{CurrentUser, RequestMeta};
use crate::context::{AppContext, audit_context};
use crate::error::{AppError, Missing, conflict};
use crate::ports::{Isolation, Locked, NonEmpty, Transaction};

/// The URL values and the row version that every draft command carries.
///
/// The type solves one problem: the eight draft commands read the same three request fields,
/// and a bare argument list would swap the version number and the row version.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DraftTarget<'a> {
    /// The reference number of the recipe in the URL.
    pub(crate) reference: &'a ReferenceNumber,
    /// The number of the version in the URL.
    pub(crate) number: u32,
    /// The row version that the form sent.
    pub(crate) expected_row_version: i64,
}

/// The machine lock that a draft command takes before the recipe lock.
///
/// The type solves one problem: `AddParameter`, `UpdateParameterDefinition`, and
/// `RebindTypeRevision` take `machine FOR SHARE`, but the other draft commands take no
/// machine lock (`data-model.md` section 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MachineLock {
    /// Take no machine lock.
    None,
    /// Take `machine FOR SHARE`.
    Shared,
}

/// The result of a draft command.
///
/// The type solves one problem: the response header `HX-Trigger` carries the new row version
/// of the draft (`api.md` section 4), so every draft command returns it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DraftOutput {
    /// The identifier of the changed version.
    pub version: RecipeVersionId,
    /// The row version after the touch statement.
    pub row_version: i64,
}

/// A locked live draft inside an open transaction.
///
/// The type solves one problem: every draft command repeats the locks, the load of the draft,
/// the row version compare, and the type trees in the same order (design D10, I12). The struct
/// holds the result of these steps. The command runs its domain method on `draft`, then calls
/// [`OpenDraft::touch`], which consumes the struct and returns a [`TouchedDraft`]. The command
/// writes through the `tx` of the touched draft and calls [`TouchedDraft::commit`]. Only the
/// touched draft can commit, so a command cannot skip the touch statement that writes
/// `draft_editor` (separation of duties, I16). A command that fails before the touch drops the
/// struct, and the transaction rolls back.
pub(crate) struct OpenDraft {
    /// The transaction with the locks of the command.
    pub(crate) tx: Box<dyn Transaction>,
    /// The recipe of the draft.
    pub(crate) recipe: RecipeId,
    /// The draft that the lock statement read. The domain method runs on it.
    pub(crate) draft: Draft,
    /// The trees of the types of every parameter of the draft, read in the transaction.
    pub(crate) trees: TypeTrees,
}

/// A locked live draft after the touch statement.
///
/// The type solves one problem: the type state makes `commit` exist only after the touch, because
/// the touch writes the `draft_editor` row that the separation of duties reads (I16).
pub(crate) struct TouchedDraft {
    /// The transaction with the locks of the command.
    pub(crate) tx: Box<dyn Transaction>,
    /// The recipe of the draft.
    pub(crate) recipe: RecipeId,
    /// The draft that the domain method changed.
    pub(crate) draft: Draft,
    row_version: i64,
}

/// Take `machine FOR SHARE` when the command asks for it.
async fn lock_machine(
    tx: &mut dyn Transaction,
    resolved: ResolvedVersion,
    lock: MachineLock,
) -> Result<(), AppError> {
    if lock == MachineLock::None {
        return Ok(());
    }
    tx.plant()
        .lock_machine_shared(resolved.machine)
        .await?
        .map(|_| ())
        .ok_or(AppError::NotFound {
            entity: Missing::MachineId(resolved.machine),
        })
}

/// Take `recipe FOR UPDATE` and require a recipe that is not deleted (`E-RCP-012`).
async fn lock_live_recipe(
    tx: &mut dyn Transaction,
    resolved: ResolvedVersion,
    target: DraftTarget<'_>,
) -> Result<(), AppError> {
    let Locked { state: recipe, .. } = tx
        .recipes()
        .lock_recipe(resolved.recipe)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::Recipe(target.reference.clone()),
        })?;
    if recipe.is_deleted() {
        return Err(RecipeError::RecipeDeleted {
            reference: recipe.reference().clone(),
        }
        .into());
    }
    Ok(())
}

/// Take `recipe_version FOR UPDATE`, load the live draft, and compare the row version (I12).
async fn lock_current_draft(
    tx: &mut dyn Transaction,
    resolved: ResolvedVersion,
    target: DraftTarget<'_>,
) -> Result<Draft, AppError> {
    let stored = tx
        .drafts()
        .lock_draft(resolved.version)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::version(target.reference, target.number),
        })?;
    let draft = Draft::load(stored)?;
    let actual = draft.row_version();
    let outcome = if actual == target.expected_row_version {
        Optimistic::Applied(())
    } else {
        Optimistic::Conflict {
            expected: target.expected_row_version,
            actual,
        }
    };
    conflict(outcome, target.reference.clone(), target.number)?;
    Ok(draft)
}

/// Read the type rows of the machine and resolve the tree of every parameter type.
async fn resolve_trees(tx: &mut dyn Transaction, draft: &Draft) -> Result<TypeTrees, AppError> {
    let rows = tx.types().type_rows(draft.machine()).await?;
    let types = PlatformTypes::collect(draft.machine(), draft.platform(), rows)?;
    Ok(TypeTrees::resolve(
        draft
            .parameters()
            .iter()
            .map(|parameter| parameter.plc_type()),
        &types,
    ))
}

impl OpenDraft {
    /// Resolve the URL values, open the transaction, take the locks, and load the draft.
    ///
    /// The order is `ReadOnly` read, `Begin(ReadCommitted)`, the optional machine lock,
    /// `recipe FOR UPDATE`, and `recipe_version FOR UPDATE` (`data-model.md` section 6).
    /// The domain method of the command runs on the locked draft, never on the state of the read.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` (`E-WEB-005`) when the recipe, the version, or the machine is missing.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `RecipeError::VersionNotEditable` (`E-RCP-002`) when the version is not a live draft.
    /// Returns `AppError::ConcurrentEdit` (`E-WEB-001`) when the row version is stale.
    /// Returns a `PlantError` when the type rows break a rule of the catalog.
    /// Returns `AppError::Storage` when the database fails.
    pub(crate) async fn open(
        ctx: &AppContext,
        target: DraftTarget<'_>,
        lock: MachineLock,
    ) -> Result<Self, AppError> {
        let resolved = resolve_version(ctx, target.reference, target.number).await?;
        let mut tx = ctx.uow.begin(Isolation::ReadCommitted).await?;
        lock_machine(tx.as_mut(), resolved, lock).await?;
        lock_live_recipe(tx.as_mut(), resolved, target).await?;
        let draft = lock_current_draft(tx.as_mut(), resolved, target).await?;
        let trees = resolve_trees(tx.as_mut(), &draft).await?;
        Ok(Self {
            tx,
            recipe: resolved.recipe,
            draft,
            trees,
        })
    }

    /// Return the identifier of the parameter with `key`.
    ///
    /// The audit target of a parameter change needs it.
    ///
    /// # Errors
    ///
    /// Returns `RecipeError::ParameterNotFound` (`E-RCP-051`) for an unknown key.
    pub(crate) fn parameter_id(&self, key: ParameterKey) -> Result<ParameterId, AppError> {
        Ok(self.draft.parameter(key)?.id())
    }

    /// Run the touch statement and the `draft_editor` insert, and return the touched draft.
    ///
    /// A command calls it after the domain method succeeded and before its own write.
    /// When the touch fails, the struct drops and the transaction rolls back.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Storage` when the database fails.
    pub(crate) async fn touch(mut self, user: &CurrentUser) -> Result<TouchedDraft, AppError> {
        let version = self.draft.version();
        let row_version = self.tx.drafts().touch_draft(version, user.user()).await?;
        Ok(TouchedDraft {
            tx: self.tx,
            recipe: self.recipe,
            draft: self.draft,
            row_version,
        })
    }
}

impl TouchedDraft {
    /// Commit the transaction with the audit entries and return the output of the command.
    ///
    /// # Errors
    ///
    /// Returns `AppError::AuditWriteFailed` or `AppError::Storage` when the commit fails.
    pub(crate) async fn commit(
        self,
        ctx: &AppContext,
        entries: NonEmpty<AuditEntry>,
    ) -> Result<DraftOutput, AppError> {
        let version = self.draft.version();
        ctx.uow.commit(self.tx, entries).await?;
        Ok(DraftOutput {
            version,
            row_version: self.row_version,
        })
    }
}

/// Build the audit entry of a diff action on a parameter.
///
/// The function solves one problem: `SetParameterValue`, `ReplaceLimits`, `UpsertTranslation`,
/// and the next draft commands build the same entry with the target `parameter`. A change of a
/// limit row or of a translation row names the parameter (`data-model.md` section 10).
#[must_use]
pub(crate) fn parameter_diff_entry(
    action: DiffAction,
    (recipe, parameter): (RecipeId, ParameterId),
    diff: StateDiff,
    (user, meta): (&CurrentUser, &RequestMeta),
) -> AuditEntry {
    AuditEntry::diff(
        action,
        AuditTarget::parameter(recipe, parameter),
        diff.before,
        diff.after,
        audit_context(user, meta),
    )
}

#[cfg(test)]
mod tests;
