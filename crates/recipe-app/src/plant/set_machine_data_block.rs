//! The use case `SetMachineDataBlock` (`domain-model.md` section 5.1, ADR-0156, design D6).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{
    DataBlockNumber, MachineCode, MachineId, PlantError, PlcPlatform, check_data_block,
};

use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, Locked, NonEmpty};

/// The parsed request of `SetMachineDataBlock`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetMachineDataBlockInput {
    /// The code of the live machine in the URL.
    pub code: MachineCode,
    /// The new data block number. An empty field is `None`.
    pub db_number: Option<DataBlockNumber>,
}

/// The result of `SetMachineDataBlock`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetMachineDataBlockOutput {
    /// The identifier of the changed machine.
    pub machine: MachineId,
    /// The platform of the machine. The caller derives the range hint of the form from it.
    pub platform: PlcPlatform,
}

/// Sets the data block number of a machine.
///
/// The use case solves one problem: the number needs the same rules as at creation, and a change
/// must not race with an archive of the same machine. It reads the platform without a lock,
/// because the platform never changes, and checks the number before it opens the command
/// transaction (`reliability.md` section 7.4). The transaction locks the machine `FOR NO KEY UPDATE`
/// and runs the domain rule on the locked state (ADR-0156, design D6).
#[derive(Clone)]
pub struct SetMachineDataBlock {
    ctx: AppContext,
}

impl UseCase for SetMachineDataBlock {
    const OPERATION: Operation = Operation::SetMachineDataBlock;
}

impl SetMachineDataBlock {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Set the data block number of the machine.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `MachineManage`.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code.
    /// Returns `PlantError::DataBlockMissing` (`E-PLT-050`), `DataBlockNotOnPlatform` (`E-PLT-051`),
    /// or `DataBlockOutOfRange` (`E-PLT-024`) before the transaction opens.
    /// Returns `PlantError::MachineArchived` (`E-PLT-010`) when the machine is archived under the lock.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: SetMachineDataBlockInput,
    ) -> Result<SetMachineDataBlockOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let (machine_id, platform, number) = self.checked_number(input).await?;
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
        machine.set_data_block(Some(number))?;
        let after = tx
            .plant()
            .set_machine_data_block(machine_id, number)
            .await?;
        let entries = NonEmpty::new(AuditEntry::row(
            RowAction::MachineDataBlockSet,
            AuditTarget::machine(machine_id),
            RowChange::Update {
                before: before.into_value(),
                after: after.into_value(),
            },
            audit_context(user, meta),
        ));
        self.ctx.uow.commit(tx, entries).await?;
        Ok(SetMachineDataBlockOutput {
            machine: machine_id,
            platform,
        })
    }

    /// Read the live machine without a lock and check the request against its platform.
    ///
    /// The method solves one problem: the port takes a number and not an option. After the
    /// check, an S7 machine always has a number. An IEC machine with no number has nothing to
    /// set, so it gets the same error as an IEC machine with a number.
    async fn checked_number(
        &self,
        input: SetMachineDataBlockInput,
    ) -> Result<(MachineId, PlcPlatform, DataBlockNumber), AppError> {
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let machine = tx
            .plant()
            .machine_by_code(input.code)
            .await?
            .ok_or(AppError::NotFound {
                entity: Missing::Machine(input.code),
            })?;
        check_data_block(input.code, machine.platform(), input.db_number)?;
        let Some(number) = input.db_number else {
            return Err(PlantError::DataBlockNotOnPlatform {
                code: input.code,
                platform: machine.platform(),
            }
            .into());
        };
        Ok((machine.id(), machine.platform(), number))
    }
}

#[cfg(test)]
mod tests;
