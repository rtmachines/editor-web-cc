//! The use case `SetMachineStringEncoding` (`domain-model.md` section 5.1, ADR-0202 (d)).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{
    MachineCode, MachineId, PlantError, StringEncoding, check_string_encoding,
};

use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, Locked, NonEmpty};

/// The parsed request of `SetMachineStringEncoding`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetMachineStringEncodingInput {
    /// The code of the live machine in the URL.
    pub code: MachineCode,
    /// The new `STRING` encoding of the machine.
    pub string_encoding: StringEncoding,
}

/// The result of `SetMachineStringEncoding`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetMachineStringEncodingOutput {
    /// The identifier of the changed machine.
    pub machine: MachineId,
    /// The encoding that the machine holds after the command.
    pub string_encoding: StringEncoding,
}

/// Sets the `STRING` encoding of a machine that has no recipe version.
///
/// The use case solves one problem: a version stores `STRING` values for one encoding, so the
/// encoding must not change after a version row exists, and no version row may appear while the
/// change runs. It reads the platform without a lock, because the platform never changes, and
/// checks the encoding against it before it opens the command transaction (`reliability.md`
/// section 7.4). The transaction locks the machine `FOR NO KEY UPDATE`. That lock conflicts with
/// the `FOR SHARE` lock of the commands that create a version row (ADR-0202 (d3)).
#[derive(Clone)]
pub struct SetMachineStringEncoding {
    ctx: AppContext,
}

impl UseCase for SetMachineStringEncoding {
    const OPERATION: Operation = Operation::SetMachineStringEncoding;
}

impl SetMachineStringEncoding {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Set the `STRING` encoding of the machine.
    ///
    /// The command writes one audit entry also when the encoding does not change, as
    /// `SetMachineDataBlock` does for an unchanged number.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `MachineManage`.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code.
    /// Returns `PlantError::StringEncodingNotOnPlatform` (`E-PLT-053`) before the transaction opens.
    /// Returns `PlantError::MachineArchived` (`E-PLT-010`) when the machine is archived under the lock.
    /// Returns `PlantError::StringEncodingFrozen` (`E-PLT-054`) when a recipe of the machine has a version row.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: SetMachineStringEncodingInput,
    ) -> Result<SetMachineStringEncodingOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let machine_id = self.checked_machine(input).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let Locked {
            state: machine,
            before,
        } = tx
            .plant()
            .lock_machine_for_no_key_update(machine_id)
            .await?
            .ok_or(AppError::NotFound {
                entity: Missing::Machine(input.code),
            })?;
        if !machine.is_live() {
            return Err(PlantError::MachineArchived { code: input.code }.into());
        }
        if tx.plant().machine_has_version(machine_id).await? {
            return Err(PlantError::StringEncodingFrozen { code: input.code }.into());
        }
        let after = tx
            .plant()
            .set_machine_string_encoding(machine_id, input.string_encoding)
            .await?;
        let entries = NonEmpty::new(AuditEntry::row(
            RowAction::MachineStringEncodingSet,
            AuditTarget::machine(machine_id),
            RowChange::Update {
                before: before.into_value(),
                after: after.into_value(),
            },
            audit_context(user, meta),
        ));
        self.ctx.uow.commit(tx, entries).await?;
        Ok(SetMachineStringEncodingOutput {
            machine: machine_id,
            string_encoding: input.string_encoding,
        })
    }

    /// Read the live machine without a lock and check the encoding against its platform.
    async fn checked_machine(
        &self,
        input: SetMachineStringEncodingInput,
    ) -> Result<MachineId, AppError> {
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let machine = tx
            .plant()
            .machine_by_code(input.code)
            .await?
            .ok_or(AppError::NotFound {
                entity: Missing::Machine(input.code),
            })?;
        check_string_encoding(input.code, machine.platform(), input.string_encoding)?;
        Ok(machine.id())
    }
}

#[cfg(test)]
mod tests;
