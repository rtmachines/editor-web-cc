//! The query `GetRecipeHistory` (spec "Recipe history query", ADR-0051 (g)).

use recipe_domain::recipe::{Recipe, ReferenceNumber};

use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::AppError;
use crate::ports::{AuditEntryId, AuditRow, CursorPage, Isolation};
use crate::recipe::{read_recipe, require_visible};

/// The parsed request of `GetRecipeHistory`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeHistoryInput {
    /// The reference number of the recipe.
    pub reference: ReferenceNumber,
    /// The cursor: the history keeps the entries with a smaller id. `None` starts at the newest entry.
    pub before: Option<AuditEntryId>,
}

/// The result of `GetRecipeHistory`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeHistory {
    /// The recipe of the history. It can be deleted.
    pub recipe: Recipe,
    /// One page of the entries with the aggregate id of the recipe, newest first, without states.
    pub page: CursorPage<AuditRow>,
}

/// Shows the timeline of one recipe: the entries of the recipe, its versions, and its parameters.
///
/// The query solves one problem: every role must see what changed in a recipe without the full
/// audit log (A20, R24). A deleted recipe is visible only to the role that restores it.
#[derive(Clone)]
pub struct GetRecipeHistory {
    ctx: AppContext,
}

impl UseCase for GetRecipeHistory {
    const OPERATION: Operation = Operation::GetRecipeHistory;
}

impl GetRecipeHistory {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the recipe and one page of its entries.
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
        input: RecipeHistoryInput,
    ) -> Result<RecipeHistory, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let recipe = read_recipe(tx.as_mut(), &input.reference).await?;
        require_visible(user, &recipe)?;
        let size = self.ctx.settings.pages.history;
        let page = tx
            .audit_reads()
            .history(recipe.id(), input.before, size)
            .await?;
        Ok(RecipeHistory { recipe, page })
    }
}
