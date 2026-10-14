//! The queries `ListUsers` and `GetUser` (design D9).

use recipe_domain::identity::{Login, TypedLogin};

use crate::auth::{Operation, RequestMeta, authorize};
use crate::context::UseCase;
use crate::error::{AppError, Missing, StorageError};
use crate::identity::{Authenticated, IdentityContext};
use crate::ports::{Isolation, LoginThrottleState, UserSummary};

/// The parsed request of `ListUsers`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListUsersInput {
    /// The text that a login must contain, or `None` for every user. `_` is a plain character.
    pub q: Option<String>,
    /// The page number. The first page is page 0.
    pub page: u32,
}

/// Lists the users, one page at a time.
///
/// The query solves one problem: an Administrator must find a user among the logins. The list
/// is ordered by login in byte order. A page holds `page.users` rows.
#[derive(Clone)]
pub struct ListUsers {
    ctx: IdentityContext,
}

impl UseCase for ListUsers {
    const OPERATION: Operation = Operation::ListUsers;
}

impl ListUsers {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Return one page of users. The rows hold no password hash.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        input: ListUsersInput,
    ) -> Result<Vec<UserSummary>, AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadOnly).await?;
        let users = tx
            .users()
            .list(input.q.as_deref(), input.page, self.ctx.settings.page_users)
            .await?;
        Ok(users)
    }
}

/// A user together with the throttle state of the login.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserDetail {
    /// The user without the password hash.
    pub user: UserSummary,
    /// The failed attempts of the login in the throttle window, over every address.
    pub throttle: LoginThrottleState,
}

/// Shows one user and the throttle state of the login.
///
/// The query solves one problem: an Administrator must see whether a user is locked out before
/// the unlock. The state holds the failures in the window and the time of the last failure.
#[derive(Clone)]
pub struct GetUser {
    ctx: IdentityContext,
}

impl UseCase for GetUser {
    const OPERATION: Operation = Operation::GetUser;
}

impl GetUser {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Return the user with `login` and the throttle state.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission. The check comes
    /// before the lookup, so an unknown login gives the same answer.
    /// Returns `NotFound` (`E-WEB-005`) when the login is unknown.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
        login: &Login,
    ) -> Result<UserDetail, AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadOnly).await?;
        let user = tx
            .users()
            .summary_by_login(login)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::User(login.clone()),
            })?;
        let typed = TypedLogin::parse(login.as_str())
            .map_err(|_reason| StorageError::invariant("a login is not a typed login"))?;
        let throttle = tx
            .attempts()
            .throttle_state(&typed, self.ctx.settings.throttle.window)
            .await?;
        Ok(UserDetail { user, throttle })
    }
}
