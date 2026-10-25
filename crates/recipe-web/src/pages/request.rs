//! The data of one request that every page handler needs.

use std::net::IpAddr;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use recipe_app::auth::RequestMeta;
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
