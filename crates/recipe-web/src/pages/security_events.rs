//! `GET /security-events`: the security events (`api.md` section 2.9, ADR-0051 (h), (i)).
//!
//! The page lists the events newest first with a cursor pager and a boosted `GET` filter form. A
//! malformed filter value gives `422 E-WEB-008` for every bad field at once. A row shows the login
//! of the account or the words "unknown login". The page never shows `login_hmac`, because the
//! row of the use case holds none. Askama escapes each text, also the user agent and the detail.

use askama::Template;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use recipe_app::audit::{ListSecurityEvents, SecurityEventListInput};
use recipe_app::auth::Operation;
use recipe_app::ports::{CursorPage, SecurityEventId, SecurityEventRow};
use recipe_domain::audit::SecurityEventKind;
use recipe_domain::identity::{TypedLogin, TypedLoginReason};
use serde::Deserialize;

use super::audit::filter::{address, not_listed, optional, present, time_range, value_of};
use super::audit::rows::state_text;
use super::path_values::{cursor, query_values};
use super::recipes::Choice;
use super::recipes::fields::FieldLines;
use super::render::{cursor_url, render};
use super::request::PageRequest;
use super::texts::Texts;
use crate::error::{FieldReason, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::render::instant;
use crate::routes::{RouteEntry, operation_row};
use crate::state::WebState;

/// The path of the page. The pager and the filter form use it.
const EVENTS_PATH: &str = "/security-events";

/// Return the route row of the security event page.
pub(crate) fn rows() -> Vec<RouteEntry> {
    vec![operation_row(
        Method::GET,
        EVENTS_PATH,
        Operation::ListSecurityEvents,
        get(list),
    )]
}

/// The raw query string. A key that is not listed here gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventsQuery {
    kind: Option<String>,
    login: Option<String>,
    client_ip: Option<String>,
    from: Option<String>,
    to: Option<String>,
    before: Option<String>,
}

/// The checked query: the input of the use case and the filters that the links keep.
struct Checked {
    input: SecurityEventListInput,
    /// The filters with a value, in the order of the form. The texts are valid.
    pairs: Vec<(&'static str, String)>,
}

/// Read the typed login of the field `login`. A bad value names the reason of its grammar.
fn typed_login(text: &str) -> Result<TypedLogin, WebError> {
    TypedLogin::parse(text).map_err(|reason| WebError::BadRequest {
        field: "login",
        reason: match reason {
            TypedLoginReason::Length => FieldReason::LoginLength,
            TypedLoginReason::Control => FieldReason::LoginControl,
        },
    })
}

impl EventsQuery {
    /// Return the filters with a value, as the links keep them.
    fn pairs(&self) -> Vec<(&'static str, String)> {
        [
            ("kind", &self.kind),
            ("login", &self.login),
            ("client_ip", &self.client_ip),
            ("from", &self.from),
            ("to", &self.to),
        ]
        .into_iter()
        .filter_map(|(key, value)| present(value.as_deref()).map(|text| (key, text.to_owned())))
        .collect()
    }

    /// Check every value. Collect each bad field into one answer (ADR-0041 (a)).
    ///
    /// # Errors
    ///
    /// Returns `422 E-WEB-008` with one line for each bad field.
    fn check(self) -> Result<Checked, WebError> {
        let mut lines = FieldLines::default();
        let kind = lines.take_or(
            optional(self.kind.as_deref(), |text| {
                SecurityEventKind::ALL
                    .into_iter()
                    .find(|kind| kind.as_sql() == text)
                    .ok_or(not_listed("kind"))
            }),
            None,
        )?;
        let login = lines.take_or(optional(self.login.as_deref(), typed_login), None)?;
        let client_ip = lines.take_or(optional(self.client_ip.as_deref(), address), None)?;
        let range = time_range(&mut lines, self.from.as_deref(), self.to.as_deref());
        let before = lines.take_or(cursor(self.before.as_deref()), None)?;
        lines.check()?;
        Ok(Checked {
            pairs: self.pairs(),
            input: SecurityEventListInput {
                kind,
                login,
                client_ip,
                range,
                before: before.map(SecurityEventId::from),
            },
        })
    }
}

/// One row of the table of security events, in the UI language.
struct EventRowView {
    time: String,
    kind: String,
    count: String,
    client_ip: String,
    user_agent: String,
    request_id: String,
    login: String,
    detail: Option<String>,
}

impl EventRowView {
    /// Build the view of `row`.
    fn new(t: Texts<'_>, row: &SecurityEventRow) -> Self {
        Self {
            time: instant(&row.occurred_at),
            kind: t.get(&format!(
                "security-kind-{}",
                row.kind.as_sql().replace('_', "-")
            )),
            count: row.count.to_string(),
            client_ip: row
                .client_ip
                .map(|address| address.to_string())
                .unwrap_or_default(),
            user_agent: row
                .user_agent
                .as_ref()
                .map(|agent| agent.as_str().to_owned())
                .unwrap_or_default(),
            request_id: row.request_id.to_string(),
            login: row.login.as_ref().map_or_else(
                || t.get("security-login-unknown"),
                std::string::ToString::to_string,
            ),
            detail: state_text(row.detail.as_ref()),
        }
    }
}

/// The page `security-events.html`.
#[derive(Template)]
#[template(path = "security-events.html")]
struct EventsPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    kinds: Vec<Choice>,
    kind: String,
    login: String,
    client_ip: String,
    from: String,
    to: String,
    caption: String,
    empty: String,
    rows: Vec<EventRowView>,
    pager_label: String,
    newest: String,
    older: String,
}

/// Return the choices of the kind select: every kind, in the order of `data-model.md` 3.8.
fn kind_choices(t: Texts<'_>) -> Vec<Choice> {
    std::iter::once(Choice::new("", t.get("security-filter-any-kind")))
        .chain(SecurityEventKind::ALL.into_iter().map(|kind| {
            let key = format!("security-kind-{}", kind.as_sql().replace('_', "-"));
            Choice::new(kind.as_sql(), t.get(&key))
        }))
        .collect()
}

impl<'a> EventsPage<'a> {
    /// Build the page from the checked query and the page of events.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(
        t: Texts<'a>,
        request: &PageRequest,
        pairs: &[(&'static str, String)],
        page: &CursorPage<SecurityEventRow>,
    ) -> Result<Self, WebError> {
        let layout =
            Layout::for_session(t, request, t.get("security-title"), Some(NavTarget::Audit))?;
        let borrowed: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let older = page
            .rows
            .last()
            .filter(|_row| page.has_older)
            .map(|row| cursor_url(EVENTS_PATH, &borrowed, Some(row.id.get())))
            .unwrap_or_default();
        Ok(Self {
            t,
            kinds: kind_choices(t),
            kind: value_of(pairs, "kind"),
            login: value_of(pairs, "login"),
            client_ip: value_of(pairs, "client_ip"),
            from: value_of(pairs, "from"),
            to: value_of(pairs, "to"),
            caption: t.get("security-caption"),
            empty: t.get("security-empty"),
            rows: page
                .rows
                .iter()
                .map(|row| EventRowView::new(t, row))
                .collect(),
            pager_label: t.get("pager-label"),
            newest: cursor_url(EVENTS_PATH, &borrowed, None),
            older,
            layout,
        })
    }
}

/// `GET /security-events`: list the security events, newest first.
async fn list(
    State(state): State<WebState>,
    request: PageRequest,
    query: Result<Query<EventsQuery>, QueryRejection>,
) -> Result<Response, WebError> {
    let Checked { input, pairs } = query_values(query)?.check()?;
    let page = ListSecurityEvents::new(state.identity().clone())
        .execute(request.auth()?, &request.meta(), input)
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let view = EventsPage::new(t, &request, &pairs, &page)?;
    Ok(render(&view, request.request_id())?.into_response())
}

#[cfg(test)]
mod tests;
