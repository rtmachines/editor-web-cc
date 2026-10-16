//! The queries `ListRecipes`, `ListDeletedRecipes`, `GetRecipe`, and `GetRecipeForm` (design D4).
//!
//! The specs are "Recipe list", "Recipe page", and "Deleted recipe" (ADR-0043).

use recipe_domain::plant::{Machine, MachineCode};
use recipe_domain::recipe::{Recipe, ReferenceNumber, VersionState};
use recipe_domain::text::Text;

use super::versions::{VersionList, read_version_list};
use super::{cut_page, page_window, read_machine, read_recipe, require_visible};
use crate::auth::{CurrentUser, FormPurpose, Operation, RecipeFormPurpose, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::{AppError, Missing};
use crate::ports::{Isolation, MachineFilter, MachineSummary, RecipeFilter, RecipeSummary};

/// The parsed request of `ListRecipes` and `ListDeletedRecipes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeListInput {
    /// A text that the reference number or the name must contain, with no difference of case.
    pub text: Option<Text>,
    /// The code of the machine of the recipes.
    pub machine: Option<MachineCode>,
    /// A state that at least one version of the recipe must have.
    pub status: Option<VersionState>,
    /// The page number. The first page is page 0.
    pub page: u32,
}

/// The result of `ListRecipes` and `ListDeletedRecipes`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeList {
    /// The rows of the page. The page holds at most `page.recipes` rows.
    pub rows: Vec<RecipeSummary>,
    /// The page number. The first page is page 0.
    pub page: u32,
    /// Whether a page after this page holds a row.
    pub has_next: bool,
}

/// Read one page of the recipes in one `ReadOnly` transaction.
///
/// A machine code follows the rule of the path value `{code}` (ADR-0043 (b)): it names the live
/// machine with that code. The transaction ends when the function returns.
///
/// # Errors
///
/// Returns `NotFound` (`E-WEB-005`) when no live machine has the code of the filter.
async fn read_list(
    ctx: &AppContext,
    input: RecipeListInput,
    include_deleted: bool,
) -> Result<RecipeList, AppError> {
    let size = ctx.settings.pages.recipes;
    let (offset, limit) = page_window(input.page, size)?;
    let filter = RecipeFilter {
        text: input.text,
        machine: input.machine,
        status: input.status,
        include_deleted,
    };
    let mut tx = ctx.uow.begin(Isolation::ReadOnly).await?;
    if let Some(code) = filter.machine
        && tx.plant().machine_by_code(code).await?.is_none()
    {
        return Err(AppError::NotFound {
            entity: Missing::Machine(code),
        });
    }
    let rows = tx
        .recipe_reads()
        .list_recipes(&filter, offset, limit)
        .await?;
    let (rows, has_next) = cut_page(rows, size);
    Ok(RecipeList {
        rows,
        page: input.page,
        has_next,
    })
}

/// Lists the recipes that are not deleted, one page at a time.
///
/// The query solves one problem: an engineer must find a recipe among the recipes of the line.
/// The rows come in the byte order of the reference number. Every role may run it, also in
/// read-only mode.
#[derive(Clone)]
pub struct ListRecipes {
    ctx: AppContext,
}

impl UseCase for ListRecipes {
    const OPERATION: Operation = Operation::ListRecipes;
}

impl ListRecipes {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return one page of the recipes that are not deleted.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code of the filter.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: RecipeListInput,
    ) -> Result<RecipeList, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        read_list(&self.ctx, input, false).await
    }
}

/// Lists the recipes with the deleted recipes, one page at a time.
///
/// The query solves one problem: only the role that restores a recipe may see the deleted
/// recipes (ADR-0043 (d)). It is `ListRecipes` with `show_deleted` and has its own permission.
#[derive(Clone)]
pub struct ListDeletedRecipes {
    ctx: AppContext,
}

impl UseCase for ListDeletedRecipes {
    const OPERATION: Operation = Operation::ListDeletedRecipes;
}

impl ListDeletedRecipes {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return one page of the recipes, also the deleted ones.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission `RecipeRestore`.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code of the filter.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: RecipeListInput,
    ) -> Result<RecipeList, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        read_list(&self.ctx, input, true).await
    }
}

/// The parsed request of `GetRecipe`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GetRecipeInput {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The page of the version list. The first page is page 0.
    pub page: u32,
}

/// The result of `GetRecipe`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeView {
    /// The recipe with the headers of all its versions. It can be deleted.
    pub recipe: Recipe,
    /// The machine of the recipe, live or archived.
    pub machine: Machine,
    /// One page of the versions, the newest number first.
    pub versions: VersionList,
}

/// Shows one recipe with the machine and one page of its versions.
///
/// The query solves one problem: the recipe page needs the recipe, its machine, and a page of
/// versions from one read. A deleted recipe is visible only to the role that restores it.
#[derive(Clone)]
pub struct GetRecipe {
    ctx: AppContext,
}

impl UseCase for GetRecipe {
    const OPERATION: Operation = Operation::GetRecipe;
}

impl GetRecipe {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the recipe, its machine, and one page of its versions.
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
        input: GetRecipeInput,
    ) -> Result<RecipeView, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let recipe = read_recipe(tx.as_mut(), &input.reference).await?;
        require_visible(user, &recipe)?;
        let machine = read_machine(tx.plant(), &recipe).await?;
        let size = self.ctx.settings.pages.versions;
        let versions = read_version_list(tx.as_mut(), recipe.id(), (input.page, size)).await?;
        Ok(RecipeView {
            recipe,
            machine,
            versions,
        })
    }
}

/// The parsed request of `GetRecipeForm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecipeFormInput {
    /// The command that the form serves. The query checks its permission (ADR-0046 (b)).
    pub purpose: RecipeFormPurpose,
}

/// The data of the page `New recipe`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeForm {
    /// The live machines that the form offers, in the order of the code number.
    pub machines: Vec<MachineSummary>,
}

/// Gives the data of the form that creates a recipe, and checks the rename of a recipe.
///
/// The query solves one problem: the form must offer exactly the live machines, and only a role
/// that may run the command of the purpose may open it. `CreateRecipe` needs `RecipeCreate`, and
/// `RenameRecipe` needs `RecipeEdit` (ADR-0046 (c)). A line holds few machines, so the query
/// reads them all.
#[derive(Clone)]
pub struct GetRecipeForm {
    ctx: AppContext,
}

impl UseCase for GetRecipeForm {
    const OPERATION: Operation = Operation::GetRecipeForm;
}

impl GetRecipeForm {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the live machines of the form.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) with the command of the purpose when the role lacks its
    /// permission.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: RecipeFormInput,
    ) -> Result<RecipeForm, AppError> {
        authorize(
            input.purpose.operation(),
            user,
            meta,
            self.ctx.security.as_ref(),
        )
        .await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let machines = tx
            .plant()
            .list_machines(&MachineFilter::default(), 0, u32::MAX)
            .await?;
        Ok(RecipeForm { machines })
    }
}
