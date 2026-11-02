//! The query `ListExportableRecipes` (`api.md` section 2.7, design D4).

use recipe_domain::plant::MachineCode;

use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing};
use crate::ports::{ExportableRecipes, Isolation};

/// The parsed request of `ListExportableRecipes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportableRecipesInput {
    /// The code of the machine in the URL.
    pub machine: MachineCode,
}

/// Lists the recipes of one machine that have an `Approved` version (`/export/v1/machines/{code}/recipes`).
///
/// The use case solves one problem: the transfer client must find the recipes that it can load,
/// each with the number and the stored hash of its approved version, without a parameter. It
/// reads in one `ReadOnly` transaction inside `timeout.export` (ADR-0109 (g)). It writes no
/// audit entry. The role `ExportClient` and the person roles with `VersionExport` can run it.
#[derive(Clone)]
pub struct ListExportableRecipes {
    ctx: AppContext,
}

impl UseCase for ListExportableRecipes {
    const OPERATION: Operation = Operation::ListExportableRecipes;
}

impl ListExportableRecipes {
    /// Build the use case over a context.
    #[must_use]
    pub fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// List the exportable recipes of the machine.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `VersionExport`.
    /// Returns `NotFound` (`E-WEB-005`) when no machine has the code (ADR-0109 (j)).
    /// Returns `AppError::Timeout` (`E-WEB-009`, stage `Export`) when `timeout.export` elapses.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ExportableRecipesInput,
    ) -> Result<ExportableRecipes, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let code = input.machine;
        self.ctx
            .settings
            .blocking
            .export(async {
                let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
                tx.recipe_reads()
                    .exportable_recipes(code)
                    .await?
                    .ok_or(AppError::NotFound {
                        entity: Missing::Machine(code),
                    })
            })
            .await
    }
}
