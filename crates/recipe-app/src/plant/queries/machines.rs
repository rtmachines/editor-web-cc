//! The queries `ListMachines`, `GetMachine`, and `GetMachineForm` (design D3, D19).
//!
//! The specs are "Machine list", "Archive message", and "Unit tree of a machine".

use recipe_domain::plant::{Machine, MachineCode, MachineId, PlcPlatform, UnitPath};

use super::{live_machine, page_window};
use crate::auth::{
    CurrentUser, FormPurpose, MachineFormPurpose, Operation, RequestMeta, authorize,
};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing, StorageError};
use crate::ports::{Isolation, MachineFilter, MachineSummary, PlantRepository};

/// The parsed request of `ListMachines`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListMachinesInput {
    /// The search text and the choice to show archived machines.
    pub filter: MachineFilter,
    /// The page number. The first page is page 0.
    pub page: u32,
    /// The machine code of the last archive, from the field `archived` of the URL.
    pub archived: Option<MachineCode>,
}

/// The result of `ListMachines`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineList {
    /// The rows of the page. The page holds at most `page.machines` rows.
    pub rows: Vec<MachineSummary>,
    /// The page number. The first page is page 0.
    pub page: u32,
    /// Whether a page after this page holds a row.
    pub has_next: bool,
    /// The archive message. It holds the code only when an archived machine has it and no live machine has it.
    pub archived: Option<MachineCode>,
}

/// Lists the machines, one page at a time.
///
/// The query solves one problem: an engineer must find a machine among the machines of the line.
/// The rows come in the order of the code number. An archived machine shows only when the filter
/// asks for it. The query also confirms the archive message, so a stale or forged URL shows no message.
#[derive(Clone)]
pub struct ListMachines {
    ctx: AppContext,
}

impl UseCase for ListMachines {
    const OPERATION: Operation = Operation::ListMachines;
}

impl ListMachines {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return one page of the machines.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `AppError::Storage` when the database fails or when the page size cannot give a window.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ListMachinesInput,
    ) -> Result<MachineList, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let size = self.ctx.settings.pages.machines;
        let (offset, limit) = page_window(input.page, size)?;
        let (mut rows, archived) = self.read(&input, offset, limit).await?;
        let size = usize::try_from(size).unwrap_or(usize::MAX);
        let has_next = rows.len() > size;
        rows.truncate(size);
        Ok(MachineList {
            rows,
            page: input.page,
            has_next,
            archived,
        })
    }

    /// Read the rows of the window and confirm the archive message in one transaction.
    ///
    /// The transaction ends when the function returns.
    async fn read(
        &self,
        input: &ListMachinesInput,
        offset: u64,
        limit: u32,
    ) -> Result<(Vec<MachineSummary>, Option<MachineCode>), AppError> {
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let rows = tx
            .plant()
            .list_machines(&input.filter, offset, limit)
            .await?;
        let archived = match input.archived {
            Some(code) => archive_message(tx.plant(), code).await?,
            None => None,
        };
        Ok((rows, archived))
    }
}

/// Return `code` when an archived machine has it and no live machine has it.
async fn archive_message(
    plant: &mut dyn PlantRepository,
    code: MachineCode,
) -> Result<Option<MachineCode>, StorageError> {
    if plant.archived_machine_by_code(code).await?.is_none() {
        return Ok(None);
    }
    let live = plant.machine_by_code(code).await?;
    Ok(live.is_none().then_some(code))
}

/// The machine that a request names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineRef {
    /// The live machine with this code.
    Code(MachineCode),
    /// The live or archived machine with this identifier.
    Id(MachineId),
}

/// The parsed request of `GetMachine`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetMachineInput {
    /// The machine that the URL names.
    pub machine: MachineRef,
    /// The unit path of the last archive, from the field `archived` of the URL.
    pub archived: Option<UnitPath>,
}

/// The result of `GetMachine`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MachineView {
    /// The machine with every unit as the port returns it, live or archived.
    /// The web layer chooses the live units or every unit.
    pub machine: Machine,
    /// The archive message. It holds the path only when the machine holds that unit archived.
    pub archived: Option<UnitPath>,
}

/// Shows one machine with its unit tree.
///
/// The query solves one problem: the machine page needs a live machine by its code, and the
/// history page needs a live or an archived machine by its identifier. A code can belong to a
/// new machine after an archive (A32), so only the identifier names an archived machine.
#[derive(Clone)]
pub struct GetMachine {
    ctx: AppContext,
}

impl UseCase for GetMachine {
    const OPERATION: Operation = Operation::GetMachine;
}

impl GetMachine {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the machine and the confirmed archive message.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code or when no machine has the identifier.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: GetMachineInput,
    ) -> Result<MachineView, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let machine = self.read(input.machine).await?;
        let archived = input.archived.filter(|path| {
            machine
                .unit(path)
                .is_some_and(|unit| unit.archived_at().is_some())
        });
        Ok(MachineView { machine, archived })
    }

    /// Read the machine in one transaction. The transaction ends when the function returns.
    async fn read(&self, reference: MachineRef) -> Result<Machine, AppError> {
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        find_machine(tx.plant(), reference).await
    }
}

/// Read the machine that `reference` names, or return `NotFound`.
async fn find_machine(
    plant: &mut dyn PlantRepository,
    reference: MachineRef,
) -> Result<Machine, AppError> {
    match reference {
        MachineRef::Code(code) => live_machine(plant, code).await,
        MachineRef::Id(id) => {
            let found = plant.machine_by_id(id).await?;
            found.ok_or(AppError::NotFound {
                entity: Missing::MachineId(id),
            })
        }
    }
}

/// The parsed request of `GetMachineForm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MachineFormInput {
    /// The command that the form serves. The query checks its permission (ADR-0046 (b)).
    pub purpose: MachineFormPurpose,
}

/// The data of the page `New machine`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MachineForm {
    /// The PLC platforms that the form offers.
    pub platforms: [PlcPlatform; 5],
}

/// Gives the data of the form that creates a machine.
///
/// The query solves one problem: the form must offer exactly the platforms of the domain, and
/// only a role that may run the command of the purpose may open it. The query reads no port.
#[derive(Clone)]
pub struct GetMachineForm {
    ctx: AppContext,
}

impl UseCase for GetMachineForm {
    const OPERATION: Operation = Operation::GetMachineForm;
}

impl GetMachineForm {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the platforms of the form.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) with the command of the purpose when the role lacks its
    /// permission.
    /// Returns `AppError::Storage` when the sink cannot write the `forbidden` event.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: MachineFormInput,
    ) -> Result<MachineForm, AppError> {
        authorize(
            input.purpose.operation(),
            user,
            meta,
            self.ctx.security.as_ref(),
        )
        .await?;
        Ok(MachineForm {
            platforms: PlcPlatform::ALL,
        })
    }
}
