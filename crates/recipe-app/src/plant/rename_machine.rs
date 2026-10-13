//! The use case `RenameMachine` (`domain-model.md` section 5.1, `data-model.md` section 6).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{MachineCode, MachineId};
use recipe_domain::text::Text;

use super::resolve::live_machine_id;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, Locked, NonEmpty};

/// The parsed request of `RenameMachine`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameMachineInput {
    /// The code of the live machine in the URL.
    pub code: MachineCode,
    /// The new name of the machine.
    pub name: Text,
}

/// The result of `RenameMachine`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenameMachineOutput {
    /// The identifier of the renamed machine.
    pub machine: MachineId,
}

/// Renames a machine.
///
/// The use case solves one problem: a rename must not race with an archive of the same machine.
/// It resolves the code without a lock, locks the machine `FOR NO KEY UPDATE`, and runs the
/// domain rule on the locked state (design D10).
#[derive(Clone)]
pub struct RenameMachine {
    ctx: AppContext,
}

impl UseCase for RenameMachine {
    const OPERATION: Operation = Operation::RenameMachine;
}

impl RenameMachine {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Rename the machine.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `MachineManage`.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code.
    /// Returns `PlantError::MachineArchived` (`E-PLT-010`) when the machine is archived under the lock.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: RenameMachineInput,
    ) -> Result<RenameMachineOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let machine_id = live_machine_id(&self.ctx, input.code).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let Locked {
            state: mut machine,
            before,
        } = tx
            .plant()
            .lock_machine_for_no_key_update(machine_id)
            .await?
            .ok_or(AppError::NotFound {
                entity: Missing::Machine(input.code),
            })?;
        machine.rename(input.name.clone())?;
        let after = tx.plant().rename_machine(machine_id, &input.name).await?;
        let entries = NonEmpty::new(AuditEntry::row(
            RowAction::MachineRenamed,
            AuditTarget::machine(machine_id),
            RowChange::Update {
                before: before.into_value(),
                after: after.into_value(),
            },
            audit_context(user, meta),
        ));
        self.ctx.uow.commit(tx, entries).await?;
        Ok(RenameMachineOutput {
            machine: machine_id,
        })
    }
}

#[cfg(test)]
mod tests;
