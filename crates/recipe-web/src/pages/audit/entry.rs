//! `GET /audit/{id}`: one audit entry with its states (`api.md` section 2.9, ADR-0051 (c), (f)).
//!
//! The page shows the stored record of the hash chain. `before` and `after` show as the text of
//! `format!("{:#}", value)` in a `pre` element, and askama escapes that text. A role with only
//! `AuditViewRecipes` gets no trace fields, because the gaps of `chain_seq` would show the entries
//! outside its scope. The page never shows `entry_hash` or `prev_hash`, because the view holds none.

use askama::Template;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::audit::{AuditEntryView, EntryTrace, GetAuditEntry};
use recipe_app::ports::AuditEntityType;

use super::filter::{AUDIT_PATH, cursor_url};
use super::rows::{AuditRowView, entity_word, label_text, state_text};
use crate::error::WebError;
use crate::layout::{Layout, NavTarget};
use crate::pages::path_values::path_audit_entry;
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The trace fields of an entry, ready for the page.
struct TraceView {
    chain_seq: String,
    client_ip: Option<String>,
    session_ref: Option<String>,
}

impl From<&EntryTrace> for TraceView {
    fn from(trace: &EntryTrace) -> Self {
        Self {
            chain_seq: trace.chain_seq.to_string(),
            client_ip: trace.client_ip.map(|address| address.to_string()),
            session_ref: trace
                .session_ref
                .as_ref()
                .map(|session| session.as_str().to_owned()),
        }
    }
}

/// The page `audit-entry.html`.
#[derive(Template)]
#[template(path = "audit-entry.html")]
struct EntryPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    time: String,
    actor: String,
    action: String,
    entity_type: String,
    entity: String,
    aggregate: String,
    request_id: String,
    trace: Option<TraceView>,
    object_href: String,
    aggregate_href: String,
    list_href: &'static str,
    before: Option<String>,
    after: Option<String>,
}

impl<'a> EntryPage<'a> {
    /// Build the page from the view of the use case.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(t: Texts<'a>, request: &PageRequest, view: &AuditEntryView) -> Result<Self, WebError> {
        let row = &view.row;
        let summary = AuditRowView::new(t, row, false);
        let layout = Layout::for_session(
            t,
            request,
            t.with_arg("audit-entry-title", "id", row.id),
            Some(NavTarget::Audit),
        )?;
        let entity_id = row.entity_id.as_uuid().to_string();
        let aggregate_id = row.aggregate_id.as_uuid().to_string();
        Ok(Self {
            t,
            time: summary.time,
            actor: summary.actor,
            action: summary.action,
            entity_type: entity_word(t, row.entity_type),
            entity: label_text(&row.label, &row.entity_id.short()),
            aggregate: format!(
                "{} {}",
                entity_word(t, AuditEntityType::from(row.aggregate_type)),
                row.aggregate_id.short()
            ),
            request_id: view.request_id.to_string(),
            trace: view.trace.as_ref().map(TraceView::from),
            object_href: cursor_url(
                AUDIT_PATH,
                &[
                    ("entity_type", row.entity_type.as_sql()),
                    ("entity_id", &entity_id),
                ],
                None,
            ),
            aggregate_href: cursor_url(AUDIT_PATH, &[("aggregate", &aggregate_id)], None),
            list_href: AUDIT_PATH,
            before: state_text(view.before.as_ref()),
            after: state_text(view.after.as_ref()),
            layout,
        })
    }
}

/// `GET /audit/{id}`: show one audit entry of the scope of the role.
pub(super) async fn entry(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let id = path_audit_entry(path)?;
    let view = GetAuditEntry::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), id)
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let page = EntryPage::new(t, &request, &view)?;
    Ok(render(&page, request.request_id())?.into_response())
}
