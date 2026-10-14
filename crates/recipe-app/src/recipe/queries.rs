//! The recipe queries (design D9 of the change `web-ui-foundation`, ADR-0036 l).

use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::AppError;

/// The recipe list that `ListRecipes` returns.
///
/// Step P3.1 has no read port for recipes, so the list has no rows. Step P3.3 adds the port, the
/// filters, the paging, and the rows to this type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecipeList;

/// Gives the recipe list.
///
/// The query solves one problem: the page `/recipes` must check the permission `RecipeView`
/// before it shows anything, as every query does (ADR-0013). Every role may run it, also in
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

    /// Return the recipe list.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `AppError::Storage` when the sink cannot write the `forbidden` event.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
    ) -> Result<RecipeList, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        Ok(RecipeList)
    }
}

#[cfg(test)]
mod tests;
