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
use recipe_app::plant::{GetMachine, GetMachineInput, MachineConnection, MachineRef, MachineView};
use recipe_domain::plant::PlcConnection;
use serde::Deserialize;

use super::commands::RenameForm;
use super::connection::mode_key;
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

/// The record of the PLC connection as the page shows it: the labelled values of the platform shape.
///
/// The type solves one problem: the page shows the record and the password state, and a template
/// cannot show a password. The type holds no password and no sealed text, only the label and the
/// value of each row (R31, A47).
struct ConnectionRows {
    rows: Vec<(String, String)>,
}

impl ConnectionRows {
    /// Build the rows of `stored`. The password row of an IEC record shows the state only.
    fn new(t: Texts<'_>, stored: &MachineConnection) -> Self {
        let row = |key: &str, value: String| (t.get(key), value);
        let rows = match &stored.connection {
            PlcConnection::S7 {
                host,
                rack,
                slot,
                db_number,
            } => vec![
                row("field-plc-host", host.to_string()),
                row("field-plc-rack", rack.value().to_string()),
                row("field-plc-slot", slot.value().to_string()),
                row("field-plc-db-number", db_number.value().to_string()),
            ],
            PlcConnection::Iec {
                host,
                port,
                address,
                mode,
                login,
            } => {
                let state = if stored.password_set {
                    "machine-connection-password-stored"
                } else {
                    "machine-connection-password-none"
                };
                let mut rows = vec![
                    row("field-plc-host", host.to_string()),
                    row("field-plc-port", port.value().to_string()),
                    row("field-plc-address", address.to_string()),
                    row("field-plc-comm-mode", t.get(mode_key(*mode))),
                ];
                if let Some(login) = login {
                    rows.push(row("field-plc-login", login.to_string()));
                }
                rows.push(row("field-plc-password", t.get(state)));
                rows
            }
        };
        Self { rows }
    }
}

/// The page `machine.html`.
///
/// The type solves one problem: one template shows a live machine with its forms and an archived
/// machine without them. The flags `can_archive`, `can_add`, `can_edit_connection`, and `form.shown` are true only for a
/// live machine and only when `PageRequest::may_run` permits the command (ADR-0113).
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
    /// The stored connection, or `None` when the machine has no record.
    connection: Option<ConnectionRows>,
    live: bool,
    can_archive: bool,
    can_add: bool,
    /// True for a live machine when `PageRequest::may_run` permits `SetPlcConnection`.
    can_edit_connection: bool,
    types_href: String,
    connection_href: String,
    recipes_href: String,
    add_href: String,
    archive_action: String,
    /// The rename form. It is shown only for a live machine and when
    /// `PageRequest::may_run` permits the rename (ADR-0113).
    form: RenameForm,
}

impl<'a> MachinePage<'a> {
    /// Build the page from the machine and the confirmed archive message.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(t: Texts<'a>, request: &PageRequest, view: &MachineView) -> Result<Self, WebError> {
        let machine = &view.machine;
        let code = machine.code();
        let live = machine.is_live();
        let title = t.with_arg("machine-title", "code", code);
        let mut layout = Layout::for_session(t, request, title, Some(NavTarget::Machines))?;
        if let Some(path) = &view.archived {
            layout = layout.with_toast(t.with_arg("unit-archived-toast", "path", path));
        }
        let allows =
            |operation: Operation| Ok::<bool, WebError>(live && request.may_run(operation)?);
        // The tree of an archived machine shows every unit and has no link (ADR-0037 d).
        let scope = if live { Scope::Live } else { Scope::All };
        let form = RenameForm::new(code, machine.name().to_string(), request.csrf_token())
            .shown(allows(Operation::RenameMachine)?);
        Ok(Self {
            layout,
            t,
            code: code.to_string(),
            name: machine.name().to_string(),
            platform: platform_text(t, machine.platform()),
            archived_at: machine.archived_at().as_ref().map(instant),
            tree: UnitTree::build(
                machine,
                scope,
                request.may_run(Operation::AddUnit)?,
                None,
                live,
            ),
            connection: view
                .connection
                .as_ref()
                .map(|stored| ConnectionRows::new(t, stored)),
            live,
            can_archive: allows(Operation::ArchiveMachine)?,
            can_add: allows(Operation::AddUnit)?,
            can_edit_connection: allows(Operation::SetPlcConnection)?,
            types_href: format!("/machines/{code}/types"),
            connection_href: format!("/machines/{code}/connection/edit"),
            recipes_href: format!("/recipes?machine={code}"),
            add_href: format!("/machines/{code}/units/new"),
            archive_action: format!("/machines/{code}/archive"),
            form,
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
