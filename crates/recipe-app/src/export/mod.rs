//! Use cases of the Export context (`domain-model.md` section 5.4).
//!
//! The command `ExportVersion` has two routes with two permissions (`api.md` section 2.7).
//! Each route is a thin use case over one shared flow.

mod flow;
mod output;

use recipe_domain::recipe::ReferenceNumber;

use self::flow::{Target, export_version};
pub use self::output::ExportedVersion;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::AppError;

/// The parsed request of `ExportApprovedVersion`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportApprovedInput {
    /// The reference number of the recipe in the URL.
    pub reference: ReferenceNumber,
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
/// with the stored hash. Every Operator can run it.
#[derive(Clone)]
pub struct ExportApprovedVersion {
    ctx: AppContext,
}

impl UseCase for ExportApprovedVersion {
    const OPERATION: Operation = Operation::ExportApprovedVersion;
}

impl ExportApprovedVersion {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Export the approved version.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `VersionExport`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe does not exist.
    /// Returns `ExportError::NoApprovedVersion` (`E-EXP-001`) when the recipe has no approved
    /// version, also when it is deleted.
    /// Returns `ExportError::ContentHashMismatch` (`E-EXP-002`) when the hash differs.
    /// Returns `AppError::Timeout` (`E-WEB-009`) when `timeout.export` elapses.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ExportApprovedInput,
    ) -> Result<ExportedVersion, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        export_version(&self.ctx, (user, meta), Target::Approved(&input.reference)).await
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
}

impl UseCase for ExportVersionByNumber {
    const OPERATION: Operation = Operation::ExportVersionByNumber;
}

impl ExportVersionByNumber {
    /// Build the use case over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Export the version.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` when the role lacks `VersionExportHistory`.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `ExportError::VersionNotExportable` (`E-EXP-003`) for a live or discarded draft.
    /// Returns `ExportError::ContentHashMismatch` (`E-EXP-002`) when the hash differs.
    /// Returns `AppError::Timeout` (`E-WEB-009`) when `timeout.export` elapses.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ExportByNumberInput,
    ) -> Result<ExportedVersion, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let target = Target::Number(&input.reference, input.number);
        export_version(&self.ctx, (user, meta), target).await
    }
}

#[cfg(test)]
mod tests;
