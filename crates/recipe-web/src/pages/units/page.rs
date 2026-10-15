//! `GET /machines/{code}/units/{path}`: the unit page (`api.md` section 2.2, design D6).
//!
//! The page shows the machine, the path, the name, the parent, and the live children. A role with
//! `UnitManage` also gets the rename form and the archive form of a live unit. An archived unit
//! shows the large badge "Archived" and no form.

use askama::Template;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::auth::Operation;
use recipe_app::plant::{GetUnit, UnitRef, UnitView};
use recipe_domain::identity::Role;
use recipe_domain::plant::{MachineCode, Unit};

use crate::error::WebError;
use crate::layout::{Layout, NavTarget};
use crate::pages::path_values::path_code_unit;
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::state::WebState;

/// A link to the page of another unit.
struct UnitLink {
    path: String,
    name: String,
    href: String,
}

impl UnitLink {
    /// Build the link to `unit` of the machine `code`.
    fn of(code: MachineCode, unit: &Unit) -> Self {
        Self {
            path: unit.path().to_string(),
            name: unit.name().to_string(),
            href: format!("/machines/{code}/units/{}", unit.path()),
        }
    }
}

/// The page `unit.html`.
///
/// The type solves one problem: one template shows a live unit with its forms and an archived unit
/// without them. The flags `can_rename` and `can_archive` are true only for a live unit and a role
/// that holds the permission of the command (`Operation::requirement`).
#[derive(Template)]
#[template(path = "unit.html")]
struct UnitPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    code: String,
    path: String,
    name: String,
    archived: bool,
    machine_href: String,
    parent: Option<UnitLink>,
    children: Vec<UnitLink>,
    can_rename: bool,
    can_archive: bool,
    rename_action: String,
    archive_action: String,
    warning: String,
    archive_button: String,
}

impl<'a> UnitPage<'a> {
    /// Build the page from the result of `GetUnit`.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    fn new(t: Texts<'a>, request: &PageRequest, view: &UnitView) -> Result<Self, WebError> {
        let code = view.machine.code();
        let path = view.unit.path();
        let live = view.unit.is_live();
        let role: Role = request.auth()?.user().role();
        let allows = |operation: Operation| live && operation.requirement().allows(role);
        let title = t.with_arg("unit-title", "path", path);
        let layout = Layout::for_session(t, request, title, Some(NavTarget::Machines))?;
        let count = t.with_arg("unit-archive-count", "count", view.descendants);
        Ok(Self {
            layout,
            t,
            code: code.to_string(),
            path: path.to_string(),
            name: view.unit.name().to_string(),
            archived: !live,
            machine_href: format!("/machines/{code}"),
            parent: view.parent.as_ref().map(|unit| UnitLink::of(code, unit)),
            children: view
                .children
                .iter()
                .map(|unit| UnitLink::of(code, unit))
                .collect(),
            can_rename: allows(Operation::RenameUnit),
            can_archive: allows(Operation::ArchiveUnit),
            rename_action: format!("/machines/{code}/units/{path}/rename"),
            archive_action: format!("/machines/{code}/units/{path}/archive"),
            warning: format!("{} {count}", t.get("unit-archive-warning")),
            archive_button: t.with_arg("unit-archive-submit", "path", path),
        })
    }
}

/// `GET /machines/{code}/units/{path}`: show a live or an archived unit of a live machine.
pub(super) async fn show(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let (code, unit) = path_code_unit(path)?;
    let view = GetUnit::new(state.identity().app().clone())
        .execute(
            request.auth()?.user(),
            &request.meta(),
            UnitRef { code, path: unit },
        )
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let page = UnitPage::new(t, &request, &view)?;
    Ok(render(&page, request.request_id())?.into_response())
}
