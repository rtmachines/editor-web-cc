//! `GET /machines/{code}` and `GET /machines/by-id/{uuid}`: the machine page
//! (`api.md` section 2.2, design D6 and D7, ADR-0037 d and x).
//!
//! The code finds the live machine. The identifier finds a live or an archived machine, because a
//! new machine can take the code of an archived one (A32). The page of an archived machine is
//! read-only: it has no form and no link to a route below `/machines/{code}`.

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::Operation;
use recipe_app::plant::{GetMachine, GetMachineInput, MachineRef, MachineView};
use serde::Deserialize;

use super::tree::{Scope, UnitTree};
use super::{platform_text, query_values};
use crate::error::WebError;
use crate::layout::{Layout, NavTarget};
use crate::pages::path_values::{path_code, path_machine_id, unit_path};
use crate::pages::render::{instant, render};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// The query string of `GET /machines/{code}`. A key other than `archived` gives `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodeQuery {
    archived: Option<String>,
}

/// The page `machine.html`.
///
/// The type solves one problem: one template shows a live machine with its forms and an archived
/// machine without them. The flags `can_rename`, `can_archive`, and `can_add` are true only for a
/// live machine and a role that holds the permission of the command (`Operation::requirement`).
#[derive(Template)]
#[template(path = "machine.html")]
struct MachinePage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    code: String,
    name: String,
    platform: String,
    archived_at: Option<String>,
    tree: UnitTree,
    live: bool,
    can_rename: bool,
    can_archive: bool,
    can_add: bool,
    types_href: String,
    add_href: String,
    rename_action: String,
    archive_action: String,
}

impl<'a> MachinePage<'a> {
    /// Build the page from the machine and the confirmed archive message.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(t: Texts<'a>, request: &PageRequest, view: &MachineView) -> Result<Self, WebError> {
        let role = request.auth()?.user().role();
        let machine = &view.machine;
        let code = machine.code();
        let live = machine.is_live();
        let title = t.with_arg("machine-title", "code", code);
        let mut layout = Layout::for_session(t, request, title, Some(NavTarget::Machines))?;
        if let Some(path) = &view.archived {
            layout = layout.with_toast(t.with_arg("unit-archived-toast", "path", path));
        }
        let allows = |operation: Operation| live && operation.requirement().allows(role);
        // The tree of an archived machine shows every unit and has no link (ADR-0037 d).
        let scope = if live { Scope::Live } else { Scope::All };
        Ok(Self {
            layout,
            t,
            code: code.to_string(),
            name: machine.name().to_string(),
            platform: platform_text(t, machine.platform()),
            archived_at: machine.archived_at().as_ref().map(instant),
            tree: UnitTree::build(machine, scope, role, None, live),
            live,
            can_rename: allows(Operation::RenameMachine),
            can_archive: allows(Operation::ArchiveMachine),
            can_add: allows(Operation::AddUnit),
            types_href: format!("/machines/{code}/types"),
            add_href: format!("/machines/{code}/units/new"),
            rename_action: format!("/machines/{code}/rename"),
            archive_action: format!("/machines/{code}/archive"),
        })
    }
}

/// Run `GetMachine` for `machine` and render the page.
async fn show(
    state: &WebState,
    request: &PageRequest,
    input: GetMachineInput,
) -> Result<Response, WebError> {
    let view = GetMachine::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), input)
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let page = MachinePage::new(t, request, &view)?;
    Ok(render(&page, request.request_id())?.into_response())
}

/// `GET /machines/{code}`: show the live machine with this code.
///
/// The optional `archived` is the path of a unit that the last archive archived. The page shows
/// the success message only when the machine holds that unit archived.
pub(crate) async fn by_code(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    query: Result<Query<CodeQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    let query = query_values(query)?;
    let archived = query.archived.as_deref().map(unit_path).transpose()?;
    let input = GetMachineInput {
        machine: MachineRef::Code(code),
        archived,
    };
    show(&state, &request, input).await
}

/// `GET /machines/by-id/{uuid}`: show a live or an archived machine by its identifier.
pub(crate) async fn by_id(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let id = path_machine_id(path)?;
    let input = GetMachineInput {
        machine: MachineRef::Id(id),
        archived: None,
    };
    show(&state, &request, input).await
}
