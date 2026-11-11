//! Use cases of the Export context (`domain-model.md` section 5.4).
//!
//! The command `ExportVersion` has two routes with two permissions (`api.md` section 2.7).
//! Each route is a thin use case over one shared flow.

mod document;
mod flow;
mod list_exportable_recipes;
mod output;
mod plc_connection;
mod scope;

use std::sync::Arc;

use recipe_domain::plant::MachineCode;
use recipe_domain::recipe::ReferenceNumber;

use self::flow::{Target, export_version};
pub use self::list_exportable_recipes::{ExportableRecipesInput, ListExportableRecipes};
pub use self::output::ExportedVersion;
pub use self::plc_connection::{
    ExportPlcConnection, ExportPlcConnectionInput, ExportedPlcConnection,
};
pub use self::scope::ExportScope;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::AppError;
use crate::ports::DocumentWriter;

/// The parsed request of `ExportApprovedVersion`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportApprovedInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The code of the machine in the URL of the Basic route (ADR-0109 (h)), or `None` for the
    /// session route. A recipe of another machine gives `NotFound`.
    pub machine: Option<MachineCode>,
    /// The part of the version to export. [`ExportScope::Version`] exports the whole version.
    pub scope: ExportScope,
}

/// The parsed request of `ExportVersionByNumber`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportByNumberInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
    /// The number of the version in the URL.
    pub number: u32,
}

/// Exports the approved version of a recipe (`/export/v1/recipes/{ref}/approved`).
///
/// The use case solves one problem: the transfer system must load the approved version and
/// no other. It locks the version `FOR SHARE`, rebuilds the hashed body, and compares its hash
/// with the stored hash. Then the port `DocumentWriter` writes the document. Every Operator can
/// run it.
#[derive(Clone)]
pub struct ExportApprovedVersion {
    ctx: AppContext,
    writer: Arc<dyn DocumentWriter>,
}

impl UseCase for ExportApprovedVersion {
    const OPERATION: Operation = Operation::ExportApprovedVersion;
}

impl ExportApprovedVersion {
    /// Build the use case over a context and the writer of the export document.
    #[must_use]
    pub fn new(ctx: AppContext, writer: Arc<dyn DocumentWriter>) -> Self {
        Self { ctx, writer }
    }

    /// Export the approved version.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `VersionExport`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe does not exist or belongs to another
    /// machine than `input.machine`, and when the unit of `input.scope` names no unit of the
    /// machine (ADR-0109 (ja)).
    /// Returns `ExportError::EmptyScope` (`E-EXP-005`) when the unit has no parameter in its scope.
    /// Returns `ExportError::NoApprovedVersion` (`E-EXP-001`) when the recipe has no approved
    /// version, also when it is deleted.
    /// Returns `ExportError::ContentHashMismatch` (`E-EXP-002`) when the hash differs or the
    /// server cannot build the body.
    /// Returns `ExportError::Serialize` (`E-EXP-004`) when the writer fails.
    /// Returns `AppError::Timeout` (`E-WEB-009`) when `timeout.export` elapses.
    /// Returns `AppError::TaskFailed` (`E-WEB-011`) when the blocking task panics.
    /// Returns `AppError::Storage` when the database fails or the approval data is missing.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ExportApprovedInput,
    ) -> Result<ExportedVersion, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = Target::Approved(&input.reference);
        export_version(
            &self.ctx,
            &self.writer,
            (user, meta),
            (target, input.machine, &input.scope),
        )
        .await
    }
}

/// Exports an approved or a retired version by number (`/export/v1/recipes/{ref}/versions/{n}`).
///
/// The use case solves one problem: an engineer must reload an earlier version. It runs the
/// same flow as `ExportApprovedVersion`. The permission is `VersionExportHistory`, so an
/// Operator gets `E-WEB-002`.
#[derive(Clone)]
pub struct ExportVersionByNumber {
    ctx: AppContext,
    writer: Arc<dyn DocumentWriter>,
}

impl UseCase for ExportVersionByNumber {
    const OPERATION: Operation = Operation::ExportVersionByNumber;
}

impl ExportVersionByNumber {
    /// Build the use case over a context and the writer of the export document.
    #[must_use]
    pub fn new(ctx: AppContext, writer: Arc<dyn DocumentWriter>) -> Self {
        Self { ctx, writer }
    }

    /// Export the version.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `VersionExportHistory`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `ExportError::VersionNotExportable` (`E-EXP-003`) for a live or discarded draft.
    /// Returns `ExportError::ContentHashMismatch` (`E-EXP-002`) when the hash differs or the
    /// server cannot build the body.
    /// Returns `ExportError::Serialize` (`E-EXP-004`) when the writer fails.
    /// Returns `AppError::Timeout` (`E-WEB-009`) when `timeout.export` elapses.
    /// Returns `AppError::TaskFailed` (`E-WEB-011`) when the blocking task panics.
    /// Returns `AppError::Storage` when the database fails or the approval data is missing.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ExportByNumberInput,
    ) -> Result<ExportedVersion, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = Target::Number(&input.reference, input.number);
        export_version(
            &self.ctx,
            &self.writer,
            (user, meta),
            (target, None, &ExportScope::Version),
        )
        .await
    }
}

#[cfg(test)]
mod list_exportable_recipes_tests;
#[cfg(test)]
mod machine_filter_tests;
#[cfg(test)]
mod missing_tree_tests;
#[cfg(test)]
mod plc_connection_tests;
#[cfg(test)]
mod rule_check_tests;
#[cfg(test)]
mod scope_tests;
#[cfg(test)]
mod test_doubles;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timeout_tests;
