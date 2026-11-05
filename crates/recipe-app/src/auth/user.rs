//! The current user and the data of one request (design D6).

use std::net::IpAddr;

use recipe_domain::audit::{RequestId, SessionRef, UserAgent};
use recipe_domain::identity::{Login, Role, UserId};

/// The user who runs a use case.
///
/// The type solves one problem: a use case must not read the role from a cached session.
/// The session never caches the role (ADR-0014). `recipe-web` builds this value per request
/// from the session and the `app_user` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentUser {
    user: UserId,
    login: Login,
    role: Role,
}

impl CurrentUser {
    /// Build the current user from the three values of the `app_user` row.
    #[must_use]
    pub const fn new(user: UserId, login: Login, role: Role) -> Self {
        Self { user, login, role }
    }

    /// Return the identifier of the user.
    #[must_use]
    pub const fn user(&self) -> UserId {
        self.user
    }

    /// Return the login of the user.
    #[must_use]
    pub const fn login(&self) -> &Login {
        &self.login
    }

    /// Return the role of the user.
    #[must_use]
    pub const fn role(&self) -> Role {
        self.role
    }
}

/// The data of one HTTP request that audit entries and security events copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestMeta {
    request_id: RequestId,
    client_ip: Option<IpAddr>,
    user_agent: Option<UserAgent>,
    session_ref: Option<SessionRef>,
}

impl RequestMeta {
    /// Build the request data. A request without a session has no session reference.
    #[must_use]
    pub const fn new(
        request_id: RequestId,
        client_ip: Option<IpAddr>,
        user_agent: Option<UserAgent>,
        session_ref: Option<SessionRef>,
    ) -> Self {
        Self {
            request_id,
            client_ip,
            user_agent,
            session_ref,
        }
    }

    /// Return the identifier of the request.
    #[must_use]
    pub const fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// Return the address of the client, when the request has one.
    #[must_use]
    pub const fn client_ip(&self) -> Option<IpAddr> {
        self.client_ip
    }

    /// Return the User-Agent header, when the request has one.
    #[must_use]
    pub const fn user_agent(&self) -> Option<&UserAgent> {
        self.user_agent.as_ref()
    }

    /// Return the session reference, when the request has a session.
    #[must_use]
    pub const fn session_ref(&self) -> Option<&SessionRef> {
        self.session_ref.as_ref()
    }
}

#[cfg(test)]
mod tests;
