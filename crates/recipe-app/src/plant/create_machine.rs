//! The use case `CreateMachine` (`domain-model.md` section 5.1, `data-model.md` section 6).

use recipe_domain::audit::{AuditContext, AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{Machine, MachineCode, MachineId, PlantError, PlcPlatform, UnitId};
use recipe_domain::text::Text;
use uuid::Uuid;

use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, MACHINE_CODE_LIVE_UNIQUE, StorageError, map_storage};
use crate::ports::{InsertedMachine, Isolation, NonEmpty};

/// The parsed request of `CreateMachine`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreateMachineInput {
    /// The code of the new machine.
    pub code: MachineCode,
    /// The name of the new machine.
    pub name: Text,
    /// The PLC platform of the new machine. It never changes (I21).
    pub platform: PlcPlatform,
    /// The name of the main unit `U1`.
    pub main_unit_name: Text,
}

/// The result of `CreateMachine`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreateMachineOutput {
    /// The identifier of the new machine.
    pub machine: MachineId,
    /// The identifier of its main unit `U1`.
    pub main_unit: UnitId,
}

/// Creates a machine with its main unit `U1`.
///
/// The use case solves one problem: a machine and its first unit must appear together,
/// with one audit entry for each row, or not at all. It takes no lock (`data-model.md` section 6).
/// The unique index `machine_code_live_unique` is the only guard of the code.
#[derive(Clone)]
pub struct CreateMachine {
    ctx: AppContext,
}

impl UseCase for CreateMachine {
    const OPERATION: Operation = Operation::CreateMachine;
}

impl CreateMachine {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Create the machine.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `MachineManage`.
    /// Returns `PlantError::DuplicateMachineCode` (`E-PLT-001`) when a live machine has the code.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: CreateMachineInput,
    ) -> Result<CreateMachineOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let code = input.code;
        let output = CreateMachineOutput {
            machine: MachineId::from(Uuid::new_v4()),
            main_unit: UnitId::from(Uuid::new_v4()),
        };
        let machine = Machine::create(
            output.machine,
            code,
            input.name,
            input.platform,
            (output.main_unit, input.main_unit_name),
        );
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let inserted = tx.plant().insert_machine(&machine).await.map_err(|error| {
            map_storage(error, |constraint| {
                (constraint == MACHINE_CODE_LIVE_UNIQUE)
                    .then(|| PlantError::DuplicateMachineCode { code }.into())
            })
        })?;
        let entries = creation_entries(&machine, inserted, audit_context(user, meta))?;
        self.ctx.uow.commit(tx, entries).await?;
        Ok(output)
    }
}

/// Build the audit entries of a new machine: one for the machine and one for each unit.
fn creation_entries(
    machine: &Machine,
    inserted: InsertedMachine,
    context: AuditContext,
) -> Result<NonEmpty<AuditEntry>, AppError> {
    if machine.units().count() != inserted.units.len() {
        return Err(StorageError::invariant(
            "insert_machine returned a different number of unit images than units",
        )
        .into());
    }
    let mut entries = NonEmpty::new(AuditEntry::row(
        RowAction::MachineCreated,
        AuditTarget::machine(machine.id()),
        RowChange::Create {
            after: inserted.machine.into_value(),
        },
        context.clone(),
    ));
    for (unit, image) in machine.units().zip(inserted.units) {
        entries.push(AuditEntry::row(
            RowAction::UnitAdded,
            AuditTarget::unit(machine.id(), unit.id()),
            RowChange::Create {
                after: image.into_value(),
            },
            context.clone(),
        ));
    }
    Ok(entries)
}

#[cfg(test)]
mod tests;
