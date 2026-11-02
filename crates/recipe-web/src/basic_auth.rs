//! The Basic layer and the extractor `BasicUser` (design D3, ADR-0109 (l), (mb), (p)).
//!
//! A Basic route has an account but no session. The layer solves one problem: it checks the
//! `Authorization` header once, before the handler, so a handler that takes `BasicUser` always
//! sees an accepted `ExportClient` account. The layer never reads the session cookie and never
//! logs the header, the login, or the password.

use axum::extract::{FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{Extensions, HeaderMap};
use axum::middleware::Next;
use axum::response::Response;
use recipe_app::auth::{CurrentUser, RequestMeta};
use recipe_app::identity::{BasicPrincipal, authenticate_basic};
use recipe_domain::audit::UserAgent;

use crate::basic_header::parse_basic;
use crate::client::ClientAddr;
use crate::error::WebError;
use crate::pipeline::request_id_in;
use crate::state::WebState;

/// The account of a Basic request and the data that its audit entry copies.
///
/// The type solves one problem: the handler of a Basic route needs the account and the request
/// data, but it has no session. `meta` therefore holds no session reference (ADR-0109 (mb)).
/// A request that did not pass `basic_layer` is a defect of the router and gives `E-WEB-010`.
#[derive(Clone, Debug)]
pub(crate) struct BasicUser {
    principal: BasicPrincipal,
    meta: RequestMeta,
}

impl BasicUser {
    /// Return the account that the Basic check accepted.
    pub(crate) const fn user(&self) -> &CurrentUser {
        &self.principal.user
    }

    /// Return the request data for an audit entry or a security event.
    pub(crate) const fn meta(&self) -> &RequestMeta {
        &self.meta
    }
}

/// Build the request data of a Basic request. It holds no session reference.
///
/// A request without a client address is a defect of the pipeline and gives `E-WEB-010`.
fn basic_meta(extensions: &Extensions, headers: &HeaderMap) -> Result<RequestMeta, WebError> {
    let request_id = request_id_in(extensions);
    let Some(client) = extensions.get::<ClientAddr>().copied() else {
        return Err(WebError::Internal { request_id });
    };
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(UserAgent::from);
    Ok(RequestMeta::new(
        request_id,
        Some(client.ip()),
        user_agent,
        None,
    ))
}

/// Check the `Authorization: Basic` header and store the `BasicPrincipal` as a request extension.
///
/// The layer wraps the handler of each row with `Access::Basic`, inside the navigation guard
/// (ADR-0109 (p)). A bad header gives `E-WEB-012`. The use case gives the other faults.
pub(crate) async fn basic_layer(
    State(state): State<WebState>,
    mut request: Request,
    next: Next,
) -> Result<Response, WebError> {
    let meta = basic_meta(request.extensions(), request.headers())?;
    let credentials = parse_basic(request.headers())?;
    let principal = authenticate_basic(state.identity(), credentials, &meta).await?;
    request.extensions_mut().insert(principal);
    Ok(next.run(request).await)
}

impl<S: Send + Sync> FromRequestParts<S> for BasicUser {
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let Some(principal) = parts.extensions.get::<BasicPrincipal>().cloned() else {
            return Err(WebError::Internal {
                request_id: request_id_in(&parts.extensions),
            });
        };
        let meta = basic_meta(&parts.extensions, &parts.headers)?;
        Ok(Self { principal, meta })
    }
}

#[cfg(test)]
mod challenge_tests;
#[cfg(test)]
mod order_tests;
#[cfg(test)]
mod tests;
