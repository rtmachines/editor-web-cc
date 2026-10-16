//! The query `DiffVersions` (`api.md` section 2.5, ADR-0050, design D4).
//!
//! The query takes a permit of the diff gate before its transaction. It reads the change flags
//! and the pairs of one page in one `RepeatableRead` transaction. The domain diff runs after the
//! transaction ends, on a blocking thread. The permit and the blocking task share one deadline.

use recipe_domain::plant::{Machine, PlatformTypes, PlcType, TypeTrees};
use recipe_domain::recipe::{Parameter, Recipe, ReferenceNumber, VersionHeader};

pub use super::diff_page::{DiffChoice, DiffPage, DiffSection, DiffSummary};
use super::diff_page::{PageRead, build_page, read_page};
use super::{read_machine, read_recipe, require_visible};
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::blocking::DiffPermit;
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, Transaction};

/// The parsed request of `DiffVersions` (`GET /recipes/{ref}/versions/{n}/diff/{m}`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffVersionsInput {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The number `n` of the new side.
    pub number: u32,
    /// The number `m` of the old side.
    pub old_number: u32,
    /// The section and the page of the request.
    pub choice: DiffChoice,
}

/// The result of `DiffVersions`.
///
/// The result holds the permit of the diff gate. The holder drops the result after the page
/// renders, and the drop gives the permit back (ADR-0050 (g)).
#[derive(Debug)]
pub struct VersionDiff {
    /// The recipe with the headers of all its versions. It can be deleted.
    pub recipe: Recipe,
    /// The machine of the recipe.
    pub machine: Machine,
    /// The header of the new side `n`.
    pub new: VersionHeader,
    /// The header of the old side `m`.
    pub old: VersionHeader,
    /// The summary and the rows of the page.
    pub diff: DiffPage,
    /// The permit of the diff gate.
    pub permit: DiffPermit,
}

/// What the transaction of `DiffVersions` reads.
struct VersionsRead {
    recipe: Recipe,
    machine: Machine,
    new: VersionHeader,
    old: VersionHeader,
    page: PageRead,
    type_rows: Vec<PlcType>,
}

/// Return the header of version `number`, or `NotFound` (`E-WEB-005`).
fn header_of(recipe: &Recipe, number: u32) -> Result<VersionHeader, AppError> {
    recipe
        .version(number)
        .cloned()
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::version(recipe.reference(), number),
        })
}

/// Read the recipe, the two headers, the machine, one page of pairs, and the type rows.
///
/// # Errors
///
/// Returns `NotFound` (`E-WEB-005`) for an unknown recipe or version number. Returns
/// `RecipeError::RecipeDeleted` (`E-RCP-012`) for a deleted recipe and a role that cannot
/// restore it (`api.md` section 2.4). Returns `AppError::Storage` when the database fails.
async fn read_versions(
    tx: &mut dyn Transaction,
    (user, input): (&CurrentUser, &DiffVersionsInput),
    size: u32,
) -> Result<VersionsRead, AppError> {
    let recipe = read_recipe(tx, &input.reference).await?;
    require_visible(user, &recipe)?;
    let new = header_of(&recipe, input.number)?;
    let old = header_of(&recipe, input.old_number)?;
    let machine = read_machine(tx.plant(), &recipe).await?;
    let sides = (Some(old.id()), new.id());
    let page = read_page(tx.diff_reads(), sides, true, (input.choice, size)).await?;
    let type_rows = tx.types().type_rows(machine.id()).await?;
    Ok(VersionsRead {
        recipe,
        machine,
        new,
        old,
        page,
        type_rows,
    })
}

/// Compare two versions of one recipe, one page at a time (ADR-0050).
///
/// The query solves one problem: a version can hold 4 000 parameters with large values. The
/// database compares the versions and gives the change flags. The query loads only the pairs of
/// one page, and at most `diff.permits` diffs run at the same time.
#[derive(Clone)]
pub struct DiffVersions {
    ctx: AppContext,
}

impl UseCase for DiffVersions {
    const OPERATION: Operation = Operation::DiffVersions;
}

impl DiffVersions {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return one page of the diff of version `number` against version `old_number`.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or a version number does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted and the role cannot restore it.
    /// Returns `AppError::Timeout` (`E-WEB-009`) when the permit wait and the diff pass `timeout.validation`.
    /// Returns `AppError::TaskFailed` (`E-WEB-011`) with the stage `Diff` when the diff task fails.
    /// Returns `AppError::Plant` when a type of a pair does not resolve.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: DiffVersionsInput,
    ) -> Result<VersionDiff, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let blocking = self.ctx.settings.blocking;
        let deadline = blocking.diff_deadline()?;
        let permit = self.ctx.diff_gate.acquire(deadline).await?;
        let mut tx = self.ctx.uow.begin(Isolation::RepeatableRead).await?;
        let size = self.ctx.settings.pages.diff;
        let read = read_versions(tx.as_mut(), (user, &input), size).await?;
        drop(tx);
        let VersionsRead {
            recipe,
            machine,
            new,
            old,
            page,
            type_rows,
        } = read;
        let leaf_rows = self.ctx.settings.diff.leaf_rows;
        let (compared, permit) = blocking
            .diff(deadline, permit, move || {
                compare(machine, page, type_rows, leaf_rows)
            })
            .await?;
        let (machine, diff) = compared?;
        Ok(VersionDiff {
            recipe,
            machine,
            new,
            old,
            diff,
            permit,
        })
    }
}

/// Run the domain diff of one page on a blocking thread and give the machine back.
fn compare(
    machine: Machine,
    page: PageRead,
    type_rows: Vec<PlcType>,
    leaf_rows: u32,
) -> Result<(Machine, DiffPage), AppError> {
    let types = PlatformTypes::collect(machine.id(), machine.platform(), type_rows)?;
    let PageRead {
        plan,
        summary,
        sides,
    } = page;
    let old: Vec<Parameter> = sides.old.into_iter().map(Parameter::from).collect();
    let new: Vec<Parameter> = sides.new.into_iter().map(Parameter::from).collect();
    let ids = old.iter().chain(new.iter()).map(Parameter::plc_type);
    let trees = TypeTrees::resolve(ids, &types);
    let context = (&machine, &types, &trees);
    let diff = build_page(plan, summary, (&old, &new), context, leaf_rows)?;
    Ok((machine, diff))
}
