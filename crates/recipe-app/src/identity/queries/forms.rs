//! The form and profile queries (design D9). They read no table.

use recipe_domain::identity::{Login, Role};

use crate::auth::{Operation, RequestMeta, authorize};
use crate::context::UseCase;
use crate::error::AppError;
use crate::identity::no_session::require_no_session;
use crate::identity::{Authenticated, IdentityContext};

/// The data of the page `New user`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserForm {
    /// The roles that the form offers.
    pub roles: Vec<Role>,
}

/// Gives the data of the form that creates a user.
///
/// The query solves one problem: the form must offer exactly the roles of the domain, and only an
/// Administrator may open it.
#[derive(Clone)]
pub struct GetUserForm {
    ctx: IdentityContext,
}

impl UseCase for GetUserForm {
    const OPERATION: Operation = Operation::GetUserForm;
}

impl GetUserForm {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Return the roles of the form.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks the permission.
    /// Returns `AppError::Storage` when the sink cannot write the `forbidden` event.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
    ) -> Result<UserForm, AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        Ok(UserForm {
            roles: Role::ALL.to_vec(),
        })
    }
}

/// The profile of the user of the session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnProfile {
    /// The login of the user.
    pub login: Login,
    /// The role of the user in this request.
    pub role: Role,
    /// Whether the user must change the password before any other request.
    pub must_change_password: bool,
}

/// Gives the profile of the current user.
///
/// The query solves one problem: every page needs the login, the role, and the password flag of
/// the user. The session check of this request already read the values from the `app_user` row.
#[derive(Clone)]
pub struct GetOwnProfile {
    ctx: IdentityContext,
}

impl UseCase for GetOwnProfile {
    const OPERATION: Operation = Operation::GetOwnProfile;
}

impl GetOwnProfile {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Return the profile of the user of `auth`.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the requirement of the operation fails.
    /// Returns `AppError::Storage` when the sink cannot write the `forbidden` event.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
    ) -> Result<OwnProfile, AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        Ok(OwnProfile {
            login: auth.user().login().clone(),
            role: auth.user().role(),
            must_change_password: auth.must_change_password(),
        })
    }
}

/// The data of the page `Change password`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnPasswordForm {
    /// Whether the page is the only page that the user may open.
    pub must_change_password: bool,
}

/// Gives the data of the form that changes the own password.
///
/// The query solves one problem: the page must tell a user with a one-time password that no other
/// page is open yet. Every authenticated role may open it.
#[derive(Clone)]
pub struct GetOwnPasswordForm {
    ctx: IdentityContext,
}

impl UseCase for GetOwnPasswordForm {
    const OPERATION: Operation = Operation::GetOwnPasswordForm;
}

impl GetOwnPasswordForm {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Return the data of the form.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the requirement of the operation fails.
    /// Returns `AppError::Storage` when the sink cannot write the `forbidden` event.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        auth: &Authenticated,
        meta: &RequestMeta,
    ) -> Result<OwnPasswordForm, AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        Ok(OwnPasswordForm {
            must_change_password: auth.must_change_password(),
        })
    }
}

/// The data of the login page. The form has no preset value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoginForm;

/// Gives the data of the login form.
///
/// The query solves one problem: the login page opens without a session, so no user exists for
/// `authorize`. The query checks the requirement `NoSession` instead.
#[derive(Clone, Copy, Debug, Default)]
pub struct GetLoginForm;

impl UseCase for GetLoginForm {
    const OPERATION: Operation = Operation::GetLoginForm;
}

impl GetLoginForm {
    /// Build the query.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Return the empty login form.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the catalog row of the operation has another
    /// requirement than `NoSession`.
    pub fn execute(self) -> Result<LoginForm, AppError> {
        require_no_session(Self::OPERATION)?;
        Ok(LoginForm)
    }
}
