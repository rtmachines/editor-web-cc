//! The use case `AddUnit` (`domain-model.md` section 5.1, `data-model.md` section 6).

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{MachineCode, PlantError, UnitId, UnitPath};
use recipe_domain::text::Text;
use uuid::Uuid;

use super::resolve::live_machine_id;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, Missing, UNIT_PARENT_EXISTS, UNIT_PATH_UNIQUE, map_storage};
use crate::ports::{Isolation, Locked, NonEmpty};

/// The parsed request of `AddUnit`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddUnitInput {
    /// The code of the live machine in the URL.
    pub code: MachineCode,
    /// The path of the parent unit. A main unit has none.
    pub parent_path: Option<UnitPath>,
    /// The last segment of the new path. The domain parses it.
    pub segment: String,
    /// The name of the new unit.
    pub name: Text,
}

/// The result of `AddUnit`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddUnitOutput {
    /// The identifier of the new unit.
    pub unit: UnitId,
    /// The path of the new unit.
    pub path: UnitPath,
}

/// Adds a unit under a live parent, or a main unit.
///
/// The use case solves one problem: a unit must not appear under a parent that a concurrent
/// command archives. It locks the machine `FOR SHARE` and then the parent unit `FOR SHARE`
/// (ADR-0015). The domain rule runs on the locked machine (design D10).
#[derive(Clone)]
pub struct AddUnit {
    ctx: AppContext,
}

impl UseCase for AddUnit {
    const OPERATION: Operation = Operation::AddUnit;
}

impl AddUnit {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Add the unit.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `UnitManage`.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code.
    /// Returns `PlantError::MachineArchived` (`E-PLT-010`) when the machine is archived under the lock.
    /// Returns `PlantError::InvalidUnitPath` (`E-PLT-017`) or `UnitDepthExceeded` (`E-PLT-002`) for a bad segment.
    /// Returns `PlantError::ParentNotFound` (`E-PLT-003`) when the parent does not exist.
    /// Returns `PlantError::UnitArchived` (`E-PLT-009`) when the parent is archived.
    /// Returns `PlantError::DuplicateUnitPath` (`E-PLT-004`) when the path exists.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: AddUnitInput,
    ) -> Result<AddUnitOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let machine_id = live_machine_id(&self.ctx, input.code).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let Locked {
            state: mut machine, ..
        } = tx
            .plant()
            .lock_machine_shared(machine_id)
            .await?
            .ok_or(AppError::NotFound {
                entity: Missing::Machine(input.code),
            })?;
        if let Some(parent) = &input.parent_path {
            // A missing parent takes no lock. The domain method then returns `E-PLT-003`.
            tx.units().lock_unit_shared(machine_id, parent).await?;
        }
        let unit = machine
            .add_unit(
                UnitId::from(Uuid::new_v4()),
                input.parent_path.as_ref(),
                &input.segment,
                input.name,
            )?
            .clone();
        let path = unit.path().clone();
        let after = tx
            .plant()
            .insert_unit(machine_id, &unit)
            .await
            .map_err(|error| {
                map_storage(error, |constraint| match constraint {
                    UNIT_PATH_UNIQUE => {
                        Some(PlantError::DuplicateUnitPath { path: path.clone() }.into())
                    }
                    UNIT_PARENT_EXISTS => {
                        Some(PlantError::ParentNotFound { path: path.clone() }.into())
                    }
                    _ => None,
                })
            })?;
        let entries = NonEmpty::new(AuditEntry::row(
            RowAction::UnitAdded,
            AuditTarget::unit(machine_id, unit.id()),
            RowChange::Create {
                after: after.into_value(),
            },
            audit_context(user, meta),
        ));
        self.ctx.uow.commit(tx, entries).await?;
        Ok(AddUnitOutput {
            unit: unit.id(),
            path,
        })
    }
}

#[cfg(test)]
mod tests;
