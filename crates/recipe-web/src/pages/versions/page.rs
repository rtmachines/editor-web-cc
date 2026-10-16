//! `GET /recipes/{ref}/versions/{n}`: the version page (`api.md` section 2.5, design D7,
//! ADR-0043 (e), (i), and (j)).
//!
//! The page reads `GetVersion` first and `ListParameters` second. A save from the page sends the
//! `row_version` of the first read. A change between the two reads then gives a conflict. It never
//! hides a change from the user (ADR-0008, ADR-0042).

use std::collections::BTreeSet;

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{Html, IntoResponse, Response};
use recipe_app::auth::Operation;
use recipe_app::recipe::{
    GetVersion, GetVersionInput, ListParameters, ListParametersInput, ParameterList, VersionView,
};
use recipe_domain::plant::UnitPath;
use recipe_domain::recipe::{ParameterKey, ReferenceNumber, VersionState};
use serde::Deserialize;

use super::actions::{VersionActions, version_actions};
use super::outdated::{OutdatedList, version_list};
use super::parameter_table::ParameterTable;
use super::path_version;
use crate::error::WebError;
use crate::layout::{Layout, NavTarget};
use crate::pages::machines::platform_text;
use crate::pages::path_values::{Page, page as read_page, query_values, unit_path};
use crate::pages::recipes::state_badge;
use crate::pages::render::{instant, render};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The query string of the version page. A key other than `unit` and `page` gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VersionQuery {
    unit: Option<String>,
    page: Option<String>,
}

impl VersionQuery {
    /// Read the unit and the page of the parameter table.
    ///
    /// # Errors
    ///
    /// Returns the code of `UnitPath` for a bad unit, and `422 E-WEB-008` for a bad page.
    fn check(self) -> Result<(Option<UnitPath>, Page), WebError> {
        let unit = self.unit.as_deref().map(unit_path).transpose()?;
        Ok((unit, read_page(self.page.as_deref())?))
    }
}

/// The page `version.html`.
///
/// The type solves one problem: one template shows a live draft with its link Add a parameter and
/// every other version without it. `add_href` is `Some` only for a live draft of a live recipe and
/// a role that holds the permission of `AddParameter`.
#[derive(Template)]
#[template(path = "version.html")]
struct VersionPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    kind: &'static str,
    word: String,
    deleted: bool,
    note: String,
    created_by: String,
    created_at: String,
    approved_by: Option<String>,
    approved_at: Option<String>,
    updated_at: String,
    platform: String,
    machine: String,
    machine_href: String,
    recipe_href: String,
    recipe_text: String,
    outdated: OutdatedList,
    table: ParameterTable,
    add_href: Option<String>,
    actions: VersionActions,
}

/// Return the address of the form of a new parameter, or `None` when the page shows no link.
fn add_href(
    request: &PageRequest,
    view: &VersionView,
    base: &str,
    list: &ParameterList,
) -> Result<Option<String>, WebError> {
    let role = request.auth()?.user().role();
    let editable = view.detail.state == VersionState::LiveDraft
        && !view.recipe.is_deleted()
        && Operation::AddParameter.requirement().allows(role);
    let unit = list
        .unit
        .as_ref()
        .map(|path| format!("?unit={path}"))
        .unwrap_or_default();
    Ok(editable.then(|| format!("{base}/parameters/new{unit}")))
}

/// Return `layout` with the success sentence `toast` in `#toast`, or `layout` as it is for `None`.
fn with_toast(layout: Layout<'_>, toast: Option<String>) -> Layout<'_> {
    match toast {
        Some(text) => layout.with_toast(text),
        None => layout,
    }
}

impl<'a> VersionPage<'a> {
    /// Build the page from the version and one page of its parameters.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(
        t: Texts<'a>,
        request: &PageRequest,
        (view, list): (&VersionView, &ParameterList),
        (page, toast): (Page, Option<String>),
        outdated: OutdatedList,
    ) -> Result<Self, WebError> {
        let detail = &view.detail;
        let reference = view.recipe.reference();
        let base = format!("/recipes/{reference}/versions/{}", detail.number);
        let mut args = fluent::FluentArgs::new();
        args.set("number", detail.number);
        args.set("reference", reference.to_string());
        let layout = Layout::for_session(
            t,
            request,
            t.with("version-title", &args),
            Some(NavTarget::Recipes),
        )?;
        let layout = with_toast(layout, toast);
        let keys: BTreeSet<ParameterKey> = view.outdated.iter().map(|item| item.key).collect();
        let (kind, key) = state_badge(detail.state);
        let machine = &view.machine;
        Ok(Self {
            t,
            kind,
            word: t.get(key),
            deleted: view.recipe.is_deleted(),
            note: detail.change_note.to_string(),
            created_by: detail.created_by.to_string(),
            created_at: instant(&detail.created_at),
            approved_by: detail.approved_by.as_ref().map(ToString::to_string),
            approved_at: detail.approved_at.as_ref().map(instant),
            updated_at: instant(&detail.updated_at),
            platform: platform_text(t, machine.platform()),
            machine: machine.code().to_string(),
            machine_href: if machine.is_live() {
                format!("/machines/{}", machine.code())
            } else {
                format!("/machines/by-id/{}", machine.id())
            },
            recipe_href: format!("/recipes/{reference}"),
            recipe_text: t.with_arg("recipe-title", "reference", reference),
            outdated,
            table: ParameterTable::new(t, &base, list, &keys, page),
            add_href: add_href(request, view, &base, list)?,
            actions: version_actions(t, request.auth()?.user().role(), view),
            layout,
        })
    }
}

/// `GET /recipes/{ref}/versions/{n}`: show the header of the version and its parameter table.
pub(crate) async fn version(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    query: Result<Query<VersionQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let target = path_version(path)?;
    let (unit, page) = query_values(query)?.check()?;
    let answer = version_answer(&state, &request, target, (unit, page), None).await?;
    Ok(answer.html.into_response())
}

/// The rendered version page and the `row_version` of the version that it shows.
pub(crate) struct VersionAnswer {
    /// The page, or the content answer of an HTMX request.
    pub(crate) html: Html<String>,
    /// The `row_version` of the first read, for the header `HX-Trigger` of a command answer.
    pub(crate) row_version: i64,
}

/// Render the version page with an optional success sentence in `#toast`.
///
/// A command that ends on the version page calls this function with its toast, for example the
/// approval (ADR-0049 (e)). The `h1` then names the toast in `aria-describedby`.
///
/// # Errors
///
/// Returns the errors of `GetVersion` and `ListParameters`, and `E-WEB-004` without a session.
pub(crate) async fn version_answer(
    state: &WebState,
    request: &PageRequest,
    (reference, number): (ReferenceNumber, u32),
    (unit, page): (Option<UnitPath>, Page),
    toast: Option<String>,
) -> Result<VersionAnswer, WebError> {
    let user = request.auth()?.user();
    let ctx = state.identity().app().clone();
    let target = GetVersionInput {
        reference: reference.clone(),
        number,
        rebind_bound: state.settings().form_rebind_leaves,
    };
    let view = GetVersion::new(ctx.clone())
        .execute(user, &request.meta(), target)
        .await?;
    let input = ListParametersInput {
        reference,
        number,
        unit,
        page: page.index,
    };
    let list = ListParameters::new(ctx)
        .execute(user, &request.meta(), input)
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let outdated = version_list(request, &view, t)?;
    let version_page = VersionPage::new(t, request, (&view, &list), (page, toast), outdated)?;
    Ok(VersionAnswer {
        html: render(&version_page, request.request_id())?,
        row_version: view.detail.row_version,
    })
}
