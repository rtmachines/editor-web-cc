//! The use case `ArchiveMachine` (`domain-model.md` section 5.1, `data-model.md` section 6).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{MachineCode, MachineId};
use recipe_domain::recipe::{Recipe, ReferenceNumber, VersionState};

use super::resolve::{live_machine_id, require_live};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::AppError;
use crate::ports::{Isolation, Locked, NonEmpty};

/// The parsed request of `ArchiveMachine`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchiveMachineInput {
    /// The code of the live machine in the URL.
    pub code: MachineCode,
}

/// The result of `ArchiveMachine`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArchiveMachineOutput {
    /// The identifier of the archived machine.
    pub machine: MachineId,
}

/// Archives a machine that has no live draft.
///
/// The use case solves one problem: a draft must not appear on a machine while the machine
/// is archived. It locks the machine `FOR NO KEY UPDATE` and then every recipe of the machine
/// `FOR UPDATE` in `id` order (ADR-0015). It finds the live draft in the locked recipes,
/// so a draft that a concurrent command creates after the read is also found (design D10).
#[derive(Clone)]
pub struct ArchiveMachine {
    ctx: AppContext,
}

impl UseCase for ArchiveMachine {
    const OPERATION: Operation = Operation::ArchiveMachine;
}

/// Return the reference number of the first recipe that has a live draft.
fn first_live_draft(recipes: &[Recipe]) -> Option<ReferenceNumber> {
    recipes
        .iter()
        .find(|recipe| {
            recipe
                .versions()
                .iter()
                .any(|version| version.state() == VersionState::LiveDraft)
        })
        .map(|recipe| recipe.reference().clone())
}

impl ArchiveMachine {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Archive the machine.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `MachineManage`.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code.
    /// Returns `PlantError::MachineHasDraft` (`E-PLT-011`) when a recipe of the machine has a live draft.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ArchiveMachineInput,
    ) -> Result<ArchiveMachineOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let machine_id = live_machine_id(&self.ctx, input.code).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let locked = tx
            .plant()
            .lock_machine_for_no_key_update(machine_id)
            .await?;
        let Locked {
            state: mut machine,
            before,
        } = require_live(locked, input.code)?;
        let recipes = tx.recipes().lock_recipes_of_machine(machine_id).await?;
        let live_draft = first_live_draft(&recipes);
        machine.archive(self.ctx.clock.now(), live_draft.as_ref())?;
        let after = tx.plant().archive_machine(machine_id).await?;
        let entries = NonEmpty::new(AuditEntry::row(
            RowAction::MachineArchived,
            AuditTarget::machine(machine_id),
            RowChange::Update {
                before: before.into_value(),
                after: after.into_value(),
            },
            audit_context(user, meta),
        ));
        self.ctx.uow.commit(tx, entries).await?;
        Ok(ArchiveMachineOutput {
            machine: machine_id,
        })
    }
}

#[cfg(test)]
mod tests;
