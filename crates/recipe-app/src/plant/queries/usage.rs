//! The query `GetTypeUsage` (design D3, spec "Type usage").

use recipe_domain::plant::{Machine, PlatformTypes, PlcType, TypeName};

use super::type_text::type_text;
use super::{TypeRef, live_machine, page_window, pick_revision, revisions_of};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, StorageError};
use crate::ports::{Isolation, TypeUsageRow};

/// The parsed request of `GetTypeUsage`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeUsageInput {
    /// The struct revision. A `None` revision names the latest revision.
    pub type_ref: TypeRef,
    /// The page number. The first page is page 0.
    pub page: u32,
}

/// One parameter that uses the struct revision, with the text of its type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeUsageLine {
    /// The recipe, the version, the tag, and the type of the parameter.
    pub row: TypeUsageRow,
    /// The type expression of the type of the parameter, for example `ST_Line@1`.
    pub type_text: String,
}

/// The result of `GetTypeUsage`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeUsage {
    /// The live machine of the request.
    pub machine: Machine,
    /// The name of the struct.
    pub name: TypeName,
    /// The revision that the page lists. It is never the `None` of the request.
    pub revision: u32,
    /// The rows of the page. The page holds at most `page.type_usage` rows.
    pub rows: Vec<TypeUsageLine>,
    /// The page number. The first page is page 0.
    pub page: u32,
    /// Whether a page after this page holds a row.
    pub has_next: bool,
}

/// Lists the parameters that use a struct revision, one page at a time.
///
/// The query solves one problem: before an engineer changes a struct, the engineer must see
/// which parameters depend on it. A parameter can depend on it through another struct or an
/// array. A parameter of a deleted recipe or of a discarded draft is not a use.
#[derive(Clone)]
pub struct GetTypeUsage {
    ctx: AppContext,
}

impl UseCase for GetTypeUsage {
    const OPERATION: Operation = Operation::GetTypeUsage;
}

/// What the transaction of `GetTypeUsage` reads.
struct UsageRead {
    machine: Machine,
    revision: u32,
    rows: Vec<TypeUsageRow>,
    types: Vec<PlcType>,
}

impl GetTypeUsage {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return one page of the parameters that use the struct revision.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code, when the machine has
    /// no struct with the name, or when the struct has no such revision.
    /// Returns `AppError::Storage` when the database fails or when a parameter has a type that
    /// the catalog does not hold.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: TypeUsageInput,
    ) -> Result<TypeUsage, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let size = self.ctx.settings.pages.type_usage;
        let (offset, limit) = page_window(input.page, size)?;
        let read = self.read(&input.type_ref, offset, limit).await?;
        let size = usize::try_from(size).unwrap_or(usize::MAX);
        let has_next = read.rows.len() > size;
        let types = PlatformTypes::collect(read.machine.id(), read.machine.platform(), read.types)?;
        let rows = read
            .rows
            .into_iter()
            .take(size)
            .map(|row| line_of(&types, row))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(TypeUsage {
            machine: read.machine,
            name: input.type_ref.name,
            revision: read.revision,
            rows,
            page: input.page,
            has_next,
        })
    }

    /// Read the machine, the revision, the usage rows, and the type rows in one transaction.
    ///
    /// The transaction ends when the function returns. Type rows are append-only (ADR-0006), so
    /// the second read of the type rows holds the type of every usage row.
    async fn read(
        &self,
        type_ref: &TypeRef,
        offset: u64,
        limit: u32,
    ) -> Result<UsageRead, AppError> {
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let machine = live_machine(tx.plant(), type_ref.code).await?;
        let known = tx.types().type_rows(machine.id()).await?;
        let known = PlatformTypes::collect(machine.id(), machine.platform(), known)?;
        let (revision, type_id) = pick_revision(&revisions_of(&known, &type_ref.name), type_ref)?;
        let rows = tx
            .types()
            .type_usage(machine.id(), type_id, offset, limit)
            .await?;
        let types = tx.types().type_rows(machine.id()).await?;
        Ok(UsageRead {
            machine,
            revision,
            rows,
            types,
        })
    }
}

/// Add the text of the parameter type to a usage row.
fn line_of(types: &PlatformTypes, row: TypeUsageRow) -> Result<TypeUsageLine, StorageError> {
    let text = type_text(types, row.parameter_type).ok_or_else(|| {
        StorageError::invariant("a parameter has a type that the catalog does not hold")
    })?;
    Ok(TypeUsageLine {
        row,
        type_text: text,
    })
}
