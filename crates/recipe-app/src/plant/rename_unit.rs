//! The use case `RenameUnit` (`domain-model.md` section 5.1, `data-model.md` section 6).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{MachineCode, UnitId, UnitPath};
use recipe_domain::text::Text;

use super::resolve::{live_machine_id, require_live};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, Locked, NonEmpty};

/// The parsed request of `RenameUnit`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameUnitInput {
    /// The code of the live machine in the URL.
    pub code: MachineCode,
    /// The path of the unit in the URL.
    pub path: UnitPath,
    /// The new name of the unit.
    pub name: Text,
}

/// The result of `RenameUnit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenameUnitOutput {
    /// The identifier of the renamed unit.
    pub unit: UnitId,
}

/// Renames a unit of a machine.
///
/// The use case solves one problem: a rename must not race with an archive of the same unit.
/// It locks the machine `FOR NO KEY UPDATE` and then the unit `FOR UPDATE` (ADR-0015).
/// The domain rule runs on the locked machine (design D10).
#[derive(Clone)]
pub struct RenameUnit {
    ctx: AppContext,
}

impl UseCase for RenameUnit {
    const OPERATION: Operation = Operation::RenameUnit;
}

impl RenameUnit {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Rename the unit.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `UnitManage`.
    /// Returns `NotFound` (`E-WEB-005`) when the machine is archived or gone, or the unit does not exist.
    /// Returns `PlantError::UnitArchived` (`E-PLT-009`) when the unit is archived under the lock.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: RenameUnitInput,
    ) -> Result<RenameUnitOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let machine_id = live_machine_id(&self.ctx, input.code).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let locked = tx
            .plant()
            .lock_machine_for_no_key_update(machine_id)
            .await?;
        let mut machine = require_live(locked, input.code)?.state;
        let Locked {
            state: unit,
            before,
        } = tx
            .plant()
            .lock_unit_for_update(machine_id, &input.path)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::Unit {
                    code: input.code,
                    path: input.path.clone(),
                },
            })?;
        machine.rename_unit(&input.path, input.name.clone())?;
        let after = tx.plant().rename_unit(unit.id(), &input.name).await?;
        let entries = NonEmpty::new(AuditEntry::row(
            RowAction::UnitRenamed,
            AuditTarget::unit(machine_id, unit.id()),
            RowChange::Update {
                before: before.into_value(),
                after: after.into_value(),
            },
            audit_context(user, meta),
        ));
        self.ctx.uow.commit(tx, entries).await?;
        Ok(RenameUnitOutput { unit: unit.id() })
    }
}

#[cfg(test)]
mod tests;
