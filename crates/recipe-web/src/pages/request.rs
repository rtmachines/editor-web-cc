//! The data of one request that every page handler needs.

use std::net::IpAddr;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use recipe_app::auth::{Operation, RequestMeta};
use recipe_app::error::AppError;
use recipe_app::identity::Authenticated;
use recipe_domain::audit::{RequestId, UserAgent};
use recipe_domain::identity::{SessionId, SessionToken};

use crate::client::ClientAddr;
use crate::cookies::{SESSION_COOKIE, request_cookie};
use crate::error::WebError;
use crate::i18n::Lang;
use crate::layout::ReadOnlyFlag;
use crate::pipeline::{HX_REQUEST, request_id_in};

/// The header that HTMX sets to the `id` of the element that receives the answer.
const HX_TARGET: &str = "hx-target";

/// What a page handler reads from the layers before it.
///
/// The type solves one problem: every handler needs the request id, the client address, the
/// session, and the language. One extractor reads them from the request extensions, so no handler
/// repeats the code and no handler guesses an address. A request without a client address is a
/// defect of the pipeline and gives `E-WEB-010`.
pub(crate) struct PageRequest {
    request_id: RequestId,
    client: IpAddr,
    user_agent: Option<UserAgent>,
    lang: Lang,
    htmx: bool,
    hx_target: Option<String>,
    session: Option<Authenticated>,
    cookie_session: Option<SessionId>,
    get_target: Option<String>,
    read_only: ReadOnlyFlag,
}

impl PageRequest {
    /// Return the read-only mode that the gate stored for this request.
    ///
    /// A request that never passed the gate gets the mode off. Every page route passes it.
    pub(crate) const fn read_only(&self) -> ReadOnlyFlag {
        self.read_only
    }

    /// Return the value of the header `HX-Target`, or `None` when the request has none.
    ///
    /// HTMX sets the header to the `id` of the target element, without the `#`. The layout reads
    /// it to choose between the full page and the content answer (design D6).
    pub(crate) fn hx_target(&self) -> Option<&str> {
        self.hx_target.as_deref()
    }

    /// Return the path and the query of a `GET` request, or `None` for another method.
    ///
    /// The language switch of the layout sends this text as the field `next`. A page that
    /// answers another method has no switch, so a user cannot leave it by accident (design D5).
    pub(crate) fn path_and_query(&self) -> Option<&str> {
        self.get_target.as_deref()
    }

    /// Return the id of the request.
    pub(crate) const fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// Return the canonical address of the client.
    pub(crate) const fn client(&self) -> IpAddr {
        self.client
    }

    /// Return the UI language of the request.
    pub(crate) const fn lang(&self) -> Lang {
        self.lang
    }

    /// Return true when the request has the header `HX-Request: true`.
    pub(crate) const fn is_htmx(&self) -> bool {
        self.htmx
    }

    /// Return the id of the session in the request cookie, valid or not.
    ///
    /// `Login` deletes this session (`api.md` section 7.3).
    pub(crate) const fn cookie_session(&self) -> Option<&SessionId> {
        self.cookie_session.as_ref()
    }

    /// Return the session of the request, or `E-WEB-004` when the request has none.
    ///
    /// # Errors
    ///
    /// Returns `AppError::Unauthenticated` without a session. A route that needs a session never
    /// gets here without one, because the session layer stops it first.
    pub(crate) fn auth(&self) -> Result<&Authenticated, WebError> {
        self.session
            .as_ref()
            .ok_or(WebError::App(AppError::Unauthenticated))
    }

    /// Tell whether the user of the request may run `operation` in the current mode.
    ///
    /// The method solves one problem: a page view must not offer a control that the gate would
    /// answer with `503 E-WEB-006`. It returns true when the role of the session has the permission
    /// of the operation and the mode permits the operation. The mode permits an operation when the
    /// mode is off or `Operation::runs_in_read_only` is true. The method reads the `ReadOnlyFlag`
    /// of the request, so the gate and the page use one value of the mode (ADR-0113 (a)).
    ///
    /// # Errors
    ///
    /// Returns the error of [`PageRequest::auth`] unchanged, `E-WEB-004`, without a session.
    pub(crate) fn may_run(&self, operation: Operation) -> Result<bool, WebError> {
        let role = self.auth()?.user().role();
        let mode_permits = !self.read_only.is_on() || operation.runs_in_read_only();
        Ok(mode_permits && operation.requirement().allows(role))
    }

    /// Return the CSRF token of the session as hex text, or an empty text without a session.
    pub(crate) fn csrf_token(&self) -> String {
        self.session
            .as_ref()
            .map(|session| session.csrf_token().to_hex())
            .unwrap_or_default()
    }

    /// Build the request data that audit entries and security events copy.
    pub(crate) fn meta(&self) -> RequestMeta {
        RequestMeta::new(
            self.request_id,
            Some(self.client),
            self.user_agent.clone(),
            self.session
                .as_ref()
                .map(|session| session.session_ref().clone()),
        )
    }
}

impl<S: Send + Sync> FromRequestParts<S> for PageRequest {
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let request_id = request_id_in(&parts.extensions);
        let Some(client) = parts.extensions.get::<ClientAddr>().copied() else {
            return Err(WebError::Internal { request_id });
        };
        let user_agent = parts
            .headers
            .get(axum::http::header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(UserAgent::from);
        let htmx = parts
            .headers
            .get(HX_REQUEST)
            .is_some_and(|value| value == "true");
        let hx_target = parts
            .headers
            .get(HX_TARGET)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let cookie_session = request_cookie(&parts.headers, SESSION_COOKIE)
            .and_then(|text| SessionToken::parse(&text).ok())
            .map(|token| SessionId::of(&token));
        let get_target = (parts.method == axum::http::Method::GET)
            .then(|| parts.uri.path_and_query().map(ToString::to_string))
            .flatten();
        let read_only = parts
            .extensions
            .get::<ReadOnlyFlag>()
            .copied()
            .unwrap_or_default();
        Ok(Self {
            read_only,
            get_target,
            request_id,
            client: client.ip(),
            user_agent,
            lang: Lang::from_headers(&parts.headers),
            htmx,
            hx_target,
            session: parts
                .extensions
                .get::<Option<Authenticated>>()
                .cloned()
                .flatten(),
            cookie_session,
        })
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::extract::Path;
    use axum::extract::connect_info::MockConnectInfo;
    use axum::http::{Method, StatusCode};
    use axum::response::{IntoResponse as _, Response};
    use axum::routing::get;
    use recipe_app::auth::Operation;
    use recipe_domain::identity::Role;

    use super::PageRequest;
    use crate::routes::{Access, RouteEntry, assemble};
    use crate::test_support::{Fixture, PEER, get_with};

    /// Answer `true` or `false` for the operation that the path names, or the code of the error.
    async fn probe(request: PageRequest, Path(name): Path<String>) -> Response {
        let operation = match name.as_str() {
            "update-definition" => Operation::UpdateParameterDefinition,
            "export" => Operation::ExportApprovedVersion,
            "logout" => Operation::Logout,
            _ => Operation::GetVersion,
        };
        match request.may_run(operation) {
            Ok(allowed) => allowed.to_string().into_response(),
            Err(error) => error.code().to_string().into_response(),
        }
    }

    /// Build a router with one probe route. `Access::Lang` ignores the mode and the role.
    fn router(fixture: &Fixture) -> Router {
        let rows = vec![RouteEntry {
            method: Method::GET,
            path: "/probe/{name}",
            access: Access::Lang,
            handler: get(probe),
        }];
        assemble(fixture.state.clone(), rows, Router::new()).layer(MockConnectInfo(PEER))
    }

    /// Ask `may_run` for `name` as a user with `role` while the read-only mode is `read_only`.
    async fn answer(role: Role, read_only: bool, name: &str) -> String {
        let fixture = Fixture::new();
        let session = fixture.seed_session(1, role, false);
        fixture.state.read_only().set(read_only);
        let reply = get_with(
            &router(&fixture),
            &format!("/probe/{name}"),
            &[("cookie", session.cookie.as_str())],
        )
        .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        reply.body
    }

    #[tokio::test]
    async fn may_run_lets_a_process_engineer_edit_a_definition_in_normal_mode() {
        let body = answer(Role::ProcessEngineer, false, "update-definition").await;
        assert_eq!(body, "true");
    }

    #[tokio::test]
    async fn may_run_stops_a_process_engineer_from_editing_a_definition_in_read_only_mode() {
        let body = answer(Role::ProcessEngineer, true, "update-definition").await;
        assert_eq!(body, "false");
    }

    #[tokio::test]
    async fn may_run_refuses_an_operator_to_edit_a_definition_in_both_modes() {
        let normal = answer(Role::Operator, false, "update-definition").await;
        let read_only = answer(Role::Operator, true, "update-definition").await;
        assert_eq!((normal.as_str(), read_only.as_str()), ("false", "false"));
    }

    #[tokio::test]
    async fn may_run_lets_an_operator_export_log_out_and_read_in_read_only_mode() {
        for name in ["export", "logout", "get-version"] {
            assert_eq!(answer(Role::Operator, true, name).await, "true", "{name}");
        }
    }

    #[tokio::test]
    async fn may_run_without_a_session_gives_e_web_004() {
        let fixture = Fixture::new();
        let reply = get_with(&router(&fixture), "/probe/get-version", &[]).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_eq!(reply.body, "E-WEB-004");
    }
}
