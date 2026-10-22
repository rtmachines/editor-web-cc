//! The query `ListSecurityEvents` (spec "Security event list query", ADR-0051 (h)).

use std::net::IpAddr;

use recipe_domain::audit::{LoginHmac, SecurityEventKind};
use recipe_domain::identity::{KeyRing, Login, TypedLogin};

use crate::auth::{Operation, RequestMeta, authorize};
use crate::context::UseCase;
use crate::error::AppError;
use crate::identity::{Authenticated, IdentityContext};
use crate::ports::{
    CursorPage, Isolation, LoginMatch, SecurityEventFilter, SecurityEventId, SecurityEventRow,
    TimeRange,
};

/// The parsed request of `ListSecurityEvents`. Each `None` keeps every row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecurityEventListInput {
    /// The kind of the rows.
    pub kind: Option<SecurityEventKind>,
    /// The typed login of the rows: the login of an account or a login without an account.
    pub login: Option<TypedLogin>,
    /// The client address of the rows.
    pub client_ip: Option<IpAddr>,
    /// The range of the times of the rows.
    pub range: TimeRange,
    /// The cursor: the list keeps the rows with a smaller id. `None` starts at the newest row.
    pub before: Option<SecurityEventId>,
}

/// Return the subject of the filter `login` under the current key of `keys`.
///
/// A typed login outside the grammar of `Login` matches only by its HMAC.
fn login_match(keys: &KeyRing, typed: &TypedLogin) -> LoginMatch {
    LoginMatch {
        login: Login::parse(typed.as_str()).ok(),
        hmac: LoginHmac::compute(keys, typed),
    }
}

/// Lists the security events, newest first, one cursor page at a time.
///
/// The query solves one problem: an Administrator must find the failed logins of a login that
/// has no account. The event stores only the HMAC of that login, so the filter compares the HMAC
/// under the current key. An event under an older key does not match (ADR-0051 (h)).
#[derive(Clone)]
pub struct ListSecurityEvents {
    ctx: IdentityContext,
}

impl UseCase for ListSecurityEvents {
    const OPERATION: Operation = Operation::ListSecurityEvents;
}

impl ListSecurityEvents {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: IdentityContext) -> Self {
        Self { ctx }
    }

    /// Return one page of the rows that match the filters. A row holds no `login_hmac`.
    ///
    /// The filter `client_ip` is canonical, so an IPv4-mapped address matches its IPv4 address.
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
        input: SecurityEventListInput,
    ) -> Result<CursorPage<SecurityEventRow>, AppError> {
        authorize(
            Self::OPERATION,
            auth.user(),
            meta,
            self.ctx.app.security.as_ref(),
        )
        .await?;
        let filter = SecurityEventFilter {
            kind: input.kind,
            login: input
                .login
                .map(|typed| login_match(&self.ctx.login_keys, &typed)),
            client_ip: input.client_ip.map(|ip| ip.to_canonical()),
            range: input.range,
        };
        let mut tx = self.ctx.app.uow.begin(Isolation::ReadOnly).await?;
        let size = self.ctx.app.settings.pages.security_events;
        let page = tx
            .audit_reads()
            .security_events(&filter, input.before, size)
            .await?;
        Ok(page)
    }
}
