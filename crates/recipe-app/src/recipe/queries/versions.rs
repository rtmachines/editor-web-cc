//! The queries `ListVersions`, `GetVersionForm`, and `GetVersion` (design D4).
//!
//! The specs are "Version list", "Version form", and "Version page" (ADR-0042, ADR-0043).

use std::cmp::Reverse;

use recipe_domain::identity::{Login, Permission};
use recipe_domain::plant::{Machine, PlcType};
use recipe_domain::recipe::{Recipe, RecipeId, ReferenceNumber, VersionHeader, VersionState};
use recipe_domain::timestamp::Timestamp;

use super::outdated::{OutdatedParameter, RebindBudget, outdated_parameters};
use super::{cut_page, page_window, read_machine, read_recipe, require_visible};
use crate::auth::{
    CurrentUser, FormPurpose, Operation, RequestMeta, VersionFormPurpose, authorize,
};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, ParameterRow, Transaction, VersionDetail, VersionSummary};

/// One page of the versions of a recipe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionList {
    /// The rows of the page, the newest number first. The page holds at most `page.versions` rows.
    pub rows: Vec<VersionSummary>,
    /// The page number. The first page is page 0.
    pub page: u32,
    /// Whether a page after this page holds a row.
    pub has_next: bool,
}

/// Read one page of the versions of `recipe` in the transaction `tx`.
///
/// `page` holds the page number and the page size.
pub(super) async fn read_version_list(
    tx: &mut dyn Transaction,
    recipe: RecipeId,
    (page, size): (u32, u32),
) -> Result<VersionList, AppError> {
    let (offset, limit) = page_window(page, size)?;
    let rows = tx
        .recipe_reads()
        .version_page(recipe, offset, limit)
        .await?;
    let (rows, has_next) = cut_page(rows, size);
    Ok(VersionList {
        rows,
        page,
        has_next,
    })
}

/// The parsed request of `ListVersions`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListVersionsInput {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The page number. The first page is page 0.
    pub page: u32,
}

/// Lists the versions of a recipe, one page at a time.
///
/// The query solves one problem: a recipe can have hundreds of versions, and the version table
/// loads one page after another. A deleted recipe is visible only to the role that restores it.
#[derive(Clone)]
pub struct ListVersions {
    ctx: AppContext,
}

impl UseCase for ListVersions {
    const OPERATION: Operation = Operation::ListVersions;
}

impl ListVersions {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return one page of the versions, the newest number first.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when no recipe has the reference number.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted and the role cannot restore it.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ListVersionsInput,
    ) -> Result<VersionList, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let recipe = read_recipe(tx.as_mut(), &input.reference).await?;
        require_visible(user, &recipe)?;
        let size = self.ctx.settings.pages.versions;
        read_version_list(tx.as_mut(), recipe.id(), (input.page, size)).await
    }
}

/// The parsed request of `GetVersionForm`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetVersionFormInput {
    /// The command that the form serves. The query checks its permission (ADR-0046 (b)).
    pub purpose: VersionFormPurpose,
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
}

/// One version that the form of a new version can use as its base.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VersionChoice {
    /// The number of the version.
    pub number: u32,
    /// The state of the version.
    pub state: VersionState,
}

/// The data of the page `New version`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionForm {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// Every version of the recipe, the newest number first.
    pub choices: Vec<VersionChoice>,
    /// The number that the form selects at first (ADR-0043 (h)).
    ///
    /// It is the approved version, else the newest version. `None` means "empty draft".
    pub default_based_on: Option<u32>,
}

/// Gives the data of the form that creates a version.
///
/// The query solves one problem: the form must offer the versions of the recipe as a base and
/// select a good default. A deleted recipe has no form, also for the role that restores it.
#[derive(Clone)]
pub struct GetVersionForm {
    ctx: AppContext,
}

impl UseCase for GetVersionForm {
    const OPERATION: Operation = Operation::GetVersionForm;
}

impl GetVersionForm {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the choices of the form and the default base.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) with the command of the purpose when the role lacks its
    /// permission.
    /// Returns `NotFound` (`E-WEB-005`) when no recipe has the reference number.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: GetVersionFormInput,
    ) -> Result<VersionForm, AppError> {
        authorize(
            input.purpose.operation(),
            user,
            meta,
            self.ctx.security.as_ref(),
        )
        .await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let recipe = read_recipe(tx.as_mut(), &input.reference).await?;
        drop(tx);
        // A form for a deleted recipe is useless, so no role passes `require_visible` here.
        if recipe.is_deleted() {
            return Err(deleted(&recipe));
        }
        let mut choices: Vec<VersionChoice> = recipe
            .versions()
            .iter()
            .map(|header| VersionChoice {
                number: header.number(),
                state: header.state(),
            })
            .collect();
        choices.sort_by_key(|choice| Reverse(choice.number));
        let default_based_on = default_base(&recipe, &choices);
        Ok(VersionForm {
            reference: input.reference,
            choices,
            default_based_on,
        })
    }
}

/// Return the error of a deleted recipe.
fn deleted(recipe: &Recipe) -> AppError {
    AppError::Recipe(recipe_domain::recipe::RecipeError::RecipeDeleted {
        reference: recipe.reference().clone(),
    })
}

/// Choose the default base: the approved version, else the newest version.
///
/// `choices` come with the newest number first.
fn default_base(recipe: &Recipe, choices: &[VersionChoice]) -> Option<u32> {
    recipe
        .approved_version()
        .map(VersionHeader::number)
        .or_else(|| choices.first().map(|choice| choice.number))
}

/// The parsed request of `GetVersion`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetVersionInput {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The number of the version.
    pub number: u32,
    /// The bound `form.rebind_leaves` on the sum of the new member leaves of the page (ADR-0044).
    ///
    /// A caller that shows no rebind form can give 0. The query then computes at most the leaves
    /// of one parameter.
    pub rebind_bound: usize,
}

/// The last change of a version row: who made it and when (ADR-0043 (m)).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LastChange {
    /// The login of the actor of the change.
    pub login: Login,
    /// The instant of the change: `updated_at` of the version.
    pub at: Timestamp,
}

/// The result of `GetVersion`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionView {
    /// The recipe with the headers of all its versions. It can be deleted.
    pub recipe: Recipe,
    /// The machine of the recipe. It gives the PLC platform of the version.
    pub machine: Machine,
    /// The header of the version with the logins of its actors and its `row_version`.
    pub detail: VersionDetail,
    /// The last change, or `None` when no audit entry belongs to `updated_at`.
    pub last_change: Option<LastChange>,
    /// The outdated parameters. Only a live draft has them.
    pub outdated: Vec<OutdatedParameter>,
    /// The sum of the new member leaves that the query computed (ADR-0044 (b)).
    ///
    /// Over the bound, the sum is a lower limit, because the query stops at the first parameter
    /// that passes the bound.
    pub rebind_sum: usize,
    /// True when `rebind_sum` passed `rebind_bound`. No parameter then holds leaves.
    pub rebind_over_bound: bool,
    /// The bound of the input. The notice `rebind-too-large` shows it.
    pub rebind_bound: usize,
}

/// What `GetVersion` reads in its transaction.
struct Snapshot {
    recipe: Recipe,
    machine: Machine,
    detail: VersionDetail,
    last_change: Option<LastChange>,
    parameters: Vec<ParameterRow>,
    types: Vec<PlcType>,
}

/// Shows the header of one version.
///
/// The query solves one problem: the version page must show the header, the last editor, and the
/// `row_version` of one moment, so a save from the page cannot overwrite a change that the user
/// never saw (ADR-0042). It reads in one `RepeatableRead` transaction and ends it before it
/// resolves the type trees of the outdated check.
#[derive(Clone)]
pub struct GetVersion {
    ctx: AppContext,
}

impl UseCase for GetVersion {
    const OPERATION: Operation = Operation::GetVersion;
}

impl GetVersion {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the header, the last change, and the outdated parameters of the version.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when the recipe or the version does not exist.
    /// Returns `RecipeError::RecipeDeleted` (`E-RCP-012`) when the recipe is deleted and the role cannot restore it.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: GetVersionInput,
    ) -> Result<VersionView, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let snapshot = self.read(user, &input).await?;
        // ADR-0044 (a): only a role with `ParameterEdit` gets the rebind work. The parameter rows
        // exist only for a live draft. Another role gets no error and no security event.
        let budget = RebindBudget {
            computes: user.role().has(Permission::ParameterEdit)
                && snapshot.detail.state == VersionState::LiveDraft,
            bound: input.rebind_bound,
        };
        let outdated = outdated_parameters(
            &snapshot.machine,
            &snapshot.parameters,
            snapshot.types,
            budget,
        )?;
        Ok(VersionView {
            recipe: snapshot.recipe,
            machine: snapshot.machine,
            detail: snapshot.detail,
            last_change: snapshot.last_change,
            outdated: outdated.parameters,
            rebind_sum: outdated.sum,
            rebind_over_bound: outdated.over_bound,
            rebind_bound: input.rebind_bound,
        })
    }

    /// Read everything of the version page in one `RepeatableRead` transaction.
    ///
    /// The parameter rows and the type rows are read only for a live draft.
    async fn read(
        &self,
        user: &CurrentUser,
        input: &GetVersionInput,
    ) -> Result<Snapshot, AppError> {
        let mut tx = self.ctx.uow.begin(Isolation::RepeatableRead).await?;
        let recipe = read_recipe(tx.as_mut(), &input.reference).await?;
        require_visible(user, &recipe)?;
        let not_found = || AppError::NotFound {
            entity: Missing::version(&input.reference, input.number),
        };
        let header = recipe.version(input.number).ok_or_else(not_found)?;
        let machine = read_machine(tx.plant(), &recipe).await?;
        let detail = tx
            .recipe_reads()
            .version_detail(header.id())
            .await?
            .ok_or_else(not_found)?;
        let at = detail.updated_at;
        let last_change = tx
            .recipe_reads()
            .last_change(recipe.id(), at)
            .await?
            .map(|login| LastChange { login, at });
        let is_draft = detail.state == VersionState::LiveDraft;
        let parameters = if is_draft {
            tx.recipe_reads()
                .parameter_page(detail.id, None, 0, u32::MAX)
                .await?
        } else {
            Vec::new()
        };
        let types = if is_draft {
            tx.types().type_rows(machine.id()).await?
        } else {
            Vec::new()
        };
        Ok(Snapshot {
            recipe,
            machine,
            detail,
            last_change,
            parameters,
            types,
        })
    }
}
