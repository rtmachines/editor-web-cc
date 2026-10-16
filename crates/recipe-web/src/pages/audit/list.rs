//! `GET /audit`: the audit list with its filter form (`api.md` section 2.9, ADR-0051 (c) to (e)).
//!
//! The filter form sends `GET` with no token and no cursor, so a change of a filter starts at the
//! newest entry. The link "Entries of the whole aggregate" and the link "Entries of this object" set
//! `aggregate` and `entity_id`. The form keeps them as hidden fields. A malformed value gives
//! `422 E-WEB-008` for every bad field at once.

use askama::Template;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use fluent::FluentArgs;
use recipe_app::audit::{AggregateLabel, AuditList, AuditListInput, ListAuditEntries};
use recipe_app::auth::Operation;
use recipe_app::ports::{
    AggregateId, AuditEntityType, AuditEntryId, AuditFilter, EntityFilter, EntityId,
};
use recipe_domain::audit::AuditAction;
use recipe_domain::identity::{Login, Permission};
use serde::Deserialize;

use super::choices::{ActionGroup, action_groups, entity_choices};
use super::filter::{
    AUDIT_PATH, BAD_VALUE, cursor_url, not_listed, optional, present, time_range, uuid, value_of,
};
use super::rows::{AuditRowView, entity_word, label_text};
use crate::error::{FieldReason, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::path_values::{cursor, query_values};
use crate::pages::recipes::Choice;
use crate::pages::recipes::fields::FieldLines;
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The raw query string. A key that is not listed here gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuditQuery {
    aggregate: Option<String>,
    entity_type: Option<String>,
    entity_id: Option<String>,
    actor: Option<String>,
    action: Option<String>,
    from: Option<String>,
    to: Option<String>,
    before: Option<String>,
}

/// The checked query: the input of the use case and the filters that the links keep.
struct Checked {
    input: AuditListInput,
    /// The filters with a value, in the order of the form. The texts are valid.
    pairs: Vec<(&'static str, String)>,
}

impl AuditQuery {
    /// Read the entity filter. `entity_id` needs `entity_type` (ADR-0051 (d)).
    fn entity(&self, lines: &mut FieldLines) -> Result<Option<EntityFilter>, WebError> {
        let entity_type = lines.take_or(
            optional(self.entity_type.as_deref(), |text| {
                AuditEntityType::from_sql(text).ok_or(not_listed("entity_type"))
            }),
            None,
        )?;
        let entity_id = lines.take_or(
            optional(self.entity_id.as_deref(), |text| {
                uuid("entity_id", text).map(EntityId::from)
            }),
            None,
        )?;
        if entity_id.is_some()
            && entity_type.is_none()
            && present(self.entity_type.as_deref()).is_none()
        {
            lines.push("entity_id", BAD_VALUE);
        }
        Ok(entity_type.map(|entity_type| EntityFilter {
            entity_type,
            entity_id,
        }))
    }

    /// Return the filters with a value, as the links keep them.
    fn pairs(&self) -> Vec<(&'static str, String)> {
        [
            ("aggregate", &self.aggregate),
            ("entity_type", &self.entity_type),
            ("entity_id", &self.entity_id),
            ("actor", &self.actor),
            ("action", &self.action),
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
        let aggregate = lines.take_or(
            optional(self.aggregate.as_deref(), |text| {
                uuid("aggregate", text).map(AggregateId::from)
            }),
            None,
        )?;
        let entity = self.entity(&mut lines)?;
        let actor = lines.take_or(
            optional(self.actor.as_deref(), |text| {
                Login::parse(text).map_err(|_reason| WebError::BadRequest {
                    field: "actor",
                    reason: FieldReason::LoginGrammar,
                })
            }),
            None,
        )?;
        let action = lines.take_or(
            optional(self.action.as_deref(), |text| {
                AuditAction::ALL
                    .into_iter()
                    .find(|action| action.as_str() == text)
                    .ok_or(not_listed("action"))
            }),
            None,
        )?;
        let range = time_range(&mut lines, self.from.as_deref(), self.to.as_deref());
        let before = lines.take_or(cursor(self.before.as_deref()), None)?;
        lines.check()?;
        Ok(Checked {
            pairs: self.pairs(),
            input: AuditListInput {
                filter: AuditFilter {
                    aggregate,
                    entity,
                    actor,
                    action,
                    range,
                },
                before: before.map(AuditEntryId::from),
            },
        })
    }
}

/// A line above the table that names an active filter and links to the page without it.
struct FilterNotice {
    text: String,
    clear_href: String,
}

/// The hidden fields that keep the entity filter (ADR-0051 (d)).
struct EntityHidden {
    entity_type: String,
    entity_id: String,
}

/// The page `audit.html`.
#[derive(Template)]
#[template(path = "audit.html")]
struct AuditPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    security_href: Option<String>,
    entity_types: Vec<Choice>,
    entity_type: String,
    actions: Vec<ActionGroup>,
    action: String,
    actor: String,
    from: String,
    to: String,
    aggregate_hidden: Option<String>,
    entity_hidden: Option<EntityHidden>,
    aggregate_notice: Option<FilterNotice>,
    object_notice: Option<FilterNotice>,
    caption: String,
    empty: String,
    rows: Vec<AuditRowView>,
    pager_label: String,
    newest: String,
    older: String,
}

/// Return the address of the list with the filters of `pairs` except the keys `without`.
fn url_without(pairs: &[(&'static str, String)], without: &[&str]) -> String {
    let kept: Vec<(&str, &str)> = pairs
        .iter()
        .filter(|(key, _value)| !without.contains(key))
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    cursor_url(AUDIT_PATH, &kept, None)
}

/// Build the text "Aggregate: type label" of the filter `aggregate` (ADR-0051 (d), design D8).
///
/// With a label, the text holds the word of the aggregate type and the label. An empty page gives
/// no label, so the text uses the key `audit-filter-aggregate-id` and holds the short id only.
fn aggregate_text(t: Texts<'_>, id: &AggregateId, label: Option<&AggregateLabel>) -> String {
    let short = id.short();
    let mut args = FluentArgs::new();
    match label {
        Some(found) => {
            args.set(
                "type",
                entity_word(t, AuditEntityType::from(found.aggregate_type)),
            );
            args.set("label", label_text(&found.label, &short));
            t.with("audit-filter-aggregate", &args)
        }
        None => {
            args.set("label", short);
            t.with("audit-filter-aggregate-id", &args)
        }
    }
}

/// Build the text "Object: type id" of the entity filter.
fn object_text(t: Texts<'_>, entity: &EntityFilter) -> String {
    let mut args = FluentArgs::new();
    args.set("type", entity_word(t, entity.entity_type));
    args.set(
        "id",
        entity.entity_id.map(|id| id.short()).unwrap_or_default(),
    );
    t.with("audit-filter-object", &args)
}

/// Return the entity filter when it names one object. Only then the form keeps hidden fields.
fn object_filter(filter: &AuditFilter) -> Option<&EntityFilter> {
    filter
        .entity
        .as_ref()
        .filter(|entity| entity.entity_id.is_some())
}

/// Build the line that names the filter `aggregate` and links to the page without it.
fn aggregate_notice(
    t: Texts<'_>,
    filter: &AuditFilter,
    pairs: &[(&'static str, String)],
    list: &AuditList,
) -> Option<FilterNotice> {
    filter.aggregate.map(|id| FilterNotice {
        text: aggregate_text(t, &id, list.aggregate.as_ref()),
        clear_href: url_without(pairs, &["aggregate"]),
    })
}

/// Build the line that names the object filter and links to the page without it.
fn object_notice(
    t: Texts<'_>,
    entity: &EntityFilter,
    pairs: &[(&'static str, String)],
) -> FilterNotice {
    FilterNotice {
        text: object_text(t, entity),
        clear_href: url_without(pairs, &["entity_type", "entity_id"]),
    }
}

/// Return the address of the next older page, or an empty text on the last page.
fn older_href(list: &AuditList, pairs: &[(&str, &str)]) -> String {
    list.page
        .rows
        .last()
        .filter(|_row| list.page.has_older)
        .map(|row| cursor_url(AUDIT_PATH, pairs, Some(row.id.get())))
        .unwrap_or_default()
}

impl<'a> AuditPage<'a> {
    /// Build the page from the checked query and the result of the use case.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(
        t: Texts<'a>,
        request: &PageRequest,
        checked: &Checked,
        list: &AuditList,
    ) -> Result<Self, WebError> {
        let role = request.auth()?.user().role();
        let all = role.has(Permission::AuditView);
        let layout = Layout::for_session(t, request, t.get("audit-title"), Some(NavTarget::Audit))?;
        let filter = &checked.input.filter;
        let pairs = &checked.pairs;
        let borrowed: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let hidden_entity = object_filter(filter);
        Ok(Self {
            t,
            security_href: Operation::ListSecurityEvents
                .requirement()
                .allows(role)
                .then(|| "/security-events".to_owned()),
            entity_types: entity_choices(t, all),
            entity_type: value_of(pairs, "entity_type"),
            actions: action_groups(t, all),
            action: value_of(pairs, "action"),
            actor: value_of(pairs, "actor"),
            from: value_of(pairs, "from"),
            to: value_of(pairs, "to"),
            aggregate_hidden: filter.aggregate.map(|id| id.as_uuid().to_string()),
            entity_hidden: hidden_entity.map(|entity| EntityHidden {
                entity_type: entity.entity_type.as_sql().to_owned(),
                entity_id: value_of(pairs, "entity_id"),
            }),
            aggregate_notice: aggregate_notice(t, filter, pairs, list),
            object_notice: hidden_entity.map(|entity| object_notice(t, entity, pairs)),
            caption: t.get("audit-caption"),
            empty: t.get("audit-empty"),
            rows: list
                .page
                .rows
                .iter()
                .map(|row| AuditRowView::new(t, row, true))
                .collect(),
            pager_label: t.get("pager-label"),
            newest: cursor_url(AUDIT_PATH, &borrowed, None),
            older: older_href(list, &borrowed),
            layout,
        })
    }
}

/// `GET /audit`: list the audit entries of the scope of the role, newest first.
pub(super) async fn list(
    State(state): State<WebState>,
    request: PageRequest,
    query: Result<Query<AuditQuery>, QueryRejection>,
) -> Result<Response, WebError> {
    let checked = query_values(query)?.check()?;
    let list = ListAuditEntries::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            checked.input.clone(),
        )
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let page = AuditPage::new(t, &request, &checked, &list)?;
    Ok(render(&page, request.request_id())?.into_response())
}
