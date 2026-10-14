//! The session layer (design D10, D11 step 7, `api.md` section 7.1).

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use recipe_app::auth::Requirement;
use recipe_app::error::AppError;
use recipe_app::identity::{Authenticated, authenticate};
use recipe_domain::identity::SessionToken;
use tracing::Span;

use crate::cookies::{SESSION_COOKIE, request_cookie};
use crate::error::WebError;
use crate::routes::Access;
use crate::state::WebState;

/// What a route needs from the session cookie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionNeed {
    /// The route ignores the cookie. It sees no session.
    Ignore,
    /// The route uses a valid session and works without one.
    Optional,
    /// The route needs a valid session. Else the answer is `E-WEB-004`.
    Required,
}

impl Access {
    /// Return what the route needs from the session cookie.
    fn session_need(self) -> SessionNeed {
        match self {
            Self::Static => SessionNeed::Ignore,
            Self::Lang => SessionNeed::Optional,
            Self::Operation(operation) => match operation.requirement() {
                Requirement::NoSession => SessionNeed::Ignore,
                Requirement::Permission(_)
                | Requirement::AnyOf(..)
                | Requirement::Authenticated => SessionNeed::Required,
            },
        }
    }
}

/// Add the span fields of a session to the span of the request.
fn record_session(session: &Authenticated) {
    let span = Span::current();
    span.record(
        "session_ref",
        tracing::field::display(session.session_ref()),
    );
    span.record("user_id", tracing::field::display(session.user().user()));
    span.record("role", tracing::field::display(session.user().role()));
}

/// Turn the session cookie into `Option<Authenticated>` and store it as a request extension.
///
/// A missing or invalid cookie gives `None`. A route that needs a session answers `E-WEB-004`
/// for `None`. A route with the requirement `NoSession` ignores the cookie. A request that
/// matched no route goes on, because the fallback serves no data.
pub(crate) async fn session_layer(
    State(state): State<WebState>,
    mut request: Request,
    next: Next,
) -> Result<Response, WebError> {
    let Some(access) = request.extensions().get::<Access>().copied() else {
        return Ok(next.run(request).await);
    };
    let need = access.session_need();
    let session = if need == SessionNeed::Ignore {
        None
    } else {
        let token = request_cookie(request.headers(), SESSION_COOKIE)
            .and_then(|text| SessionToken::parse(&text).ok());
        match token {
            Some(token) => authenticate(state.identity(), &token).await?,
            None => None,
        }
    };
    if need == SessionNeed::Required && session.is_none() {
        return Err(WebError::App(AppError::Unauthenticated));
    }
    if let Some(session) = &session {
        record_session(session);
    }
    request.extensions_mut().insert(session);
    Ok(next.run(request).await)
}
