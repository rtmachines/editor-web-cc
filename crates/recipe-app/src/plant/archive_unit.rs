//! The use case `ArchiveUnit` (`domain-model.md` section 5.1, `data-model.md` section 6).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{MachineCode, PlantError, Unit, UnitId, UnitPath};

use super::resolve::{live_machine_id, require_live};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, Missing, StorageError};
use crate::ports::{Isolation, Locked, NonEmpty, RowImage};

/// The parsed request of `ArchiveUnit`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveUnitInput {
    /// The code of the live machine in the URL.
    pub code: MachineCode,
    /// The path of the unit in the URL.
    pub path: UnitPath,
}

/// The result of `ArchiveUnit`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveUnitOutput {
    /// The identifiers of the archived units: the unit and its live descendants, in `id` order.
    pub units: Vec<UnitId>,
}

/// Archives a unit and its descendants.
///
/// The use case solves one problem: a unit must not be archived while a draft or an approved
/// version uses it. It locks the machine `FOR NO KEY UPDATE` and then the unit tree
/// `FOR UPDATE` in `id` order (ADR-0015). It reads the use of the units after the locks,
/// so a draft that a concurrent command adds after the first read is also found (design D10).
/// A unit that is already archived gives `E-PLT-009`, because a commit needs one entry (design D21).
#[derive(Clone)]
pub struct ArchiveUnit {
    ctx: AppContext,
}

impl UseCase for ArchiveUnit {
    const OPERATION: Operation = Operation::ArchiveUnit;
}

/// Pair the identifier and the row image before the change of each unit that the domain archived.
fn archived_before_images(
    locked: Vec<Locked<Unit>>,
    archived: &[UnitId],
) -> Vec<(UnitId, RowImage)> {
    locked
        .into_iter()
        .filter(|entry| archived.contains(&entry.state.id()))
        .map(|entry| (entry.state.id(), entry.before))
        .collect()
}

impl ArchiveUnit {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Archive the unit and its descendants.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `UnitManage`.
    /// Returns `NotFound` (`E-WEB-005`) when the machine is archived or gone, or the unit does not exist.
    /// Returns `PlantError::UnitArchived` (`E-PLT-009`) when the unit is already archived.
    /// Returns `PlantError::LastMainUnit` (`E-PLT-008`) for the last live main unit.
    /// Returns `PlantError::UnitReferencedByDraft` (`E-PLT-012`) or `UnitReferencedByApproved` (`E-PLT-014`) for a used unit.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ArchiveUnitInput,
    ) -> Result<ArchiveUnitOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let machine_id = live_machine_id(&self.ctx, input.code).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let locked = tx
            .plant()
            .lock_machine_for_no_key_update(machine_id)
            .await?;
        let mut machine = require_live(locked, input.code)?.state;
        let tree = tx
            .plant()
            .lock_unit_tree_for_update(machine_id, &input.path)
            .await?;
        if tree.is_empty() {
            return Err(AppError::NotFound {
                entity: Missing::Unit {
                    code: input.code,
                    path: input.path,
                },
            });
        }
        let tree_ids: Vec<UnitId> = tree.iter().map(|entry| entry.state.id()).collect();
        let usage = tx.plant().unit_usage(machine_id, &tree_ids).await?;
        let archived = machine.archive_unit(&input.path, self.ctx.clock.now(), &usage)?;
        let already_archived = || PlantError::UnitArchived {
            path: input.path.clone(),
        };
        let pairs = NonEmpty::try_from(archived_before_images(tree, &archived))
            .map_err(|_| already_archived())?;
        let ids: Vec<UnitId> = pairs.iter().map(|(id, _)| *id).collect();
        let afters = tx.plant().archive_units(&ids).await?;
        if afters.len() != ids.len() {
            return Err(StorageError::invariant(
                "archive_units returned a different number of row images than units",
            )
            .into());
        }
        let context = audit_context(user, meta);
        let entries: Vec<AuditEntry> = Vec::from(pairs)
            .into_iter()
            .zip(afters)
            .map(|((id, before), after)| {
                AuditEntry::row(
                    RowAction::UnitArchived,
                    AuditTarget::unit(machine_id, id),
                    RowChange::Update {
                        before: before.into_value(),
                        after: after.into_value(),
                    },
                    context.clone(),
                )
            })
            .collect();
        let entries = NonEmpty::try_from(entries).map_err(|_| already_archived())?;
        self.ctx.uow.commit(tx, entries).await?;
        Ok(ArchiveUnitOutput { units: ids })
    }
}

#[cfg(test)]
mod tests;
