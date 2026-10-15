//! The type list: `GET /machines/{code}/types` (`api.md` section 2.3, design D6).

use askama::Template;
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use recipe_app::plant::{ListTypes, TypeList, TypesInput};

use super::{can_define, current_nav, revision_href};
use crate::error::WebError;
use crate::layout::Layout;
use crate::pages::path_values::path_code;
use crate::pages::render::render;
use crate::pages::{PageRequest, Texts};
use crate::state::WebState;

/// One row of the table: a revision of a struct.
struct TypeRow {
    /// The address of the type page of this revision.
    href: String,
    /// The name of the struct.
    name: String,
    /// The revision, from 1.
    revision: u32,
    /// The number of members.
    members: usize,
    /// True for the highest revision of the name.
    latest: bool,
}

/// The page `types.html`.
#[derive(Template)]
#[template(path = "types.html")]
struct TypesPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    title: String,
    rows: Vec<TypeRow>,
    new_href: Option<String>,
    machine_href: String,
    machine_text: String,
}

/// Turn the result of `ListTypes` into the rows of the table.
fn table_rows(list: &TypeList) -> Vec<TypeRow> {
    let code = list.machine.code();
    list.rows
        .iter()
        .map(|row| TypeRow {
            href: revision_href(code, &row.name, row.revision),
            name: row.name.to_string(),
            revision: row.revision,
            members: row.members,
            latest: row.latest,
        })
        .collect()
}

/// `GET /machines/{code}/types`: list the struct revisions of a live machine.
pub(super) async fn list(
    State(state): State<WebState>,
    path: Result<Path<String>, PathRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    let code = path_code(path)?;
    let types = ListTypes::new(state.identity().app().clone())
        .execute(request.auth()?.user(), &request.meta(), TypesInput { code })
        .await?;
    let t = Texts::new(state.bundles(), request.lang());
    let title = t.with_arg("types-title", "code", code);
    let layout = Layout::for_session(t, &request, title.clone(), current_nav())?;
    let machine_text = t.with_arg("machine-title", "code", code);
    let page = TypesPage {
        layout,
        t,
        title,
        rows: table_rows(&types),
        new_href: can_define(&request)?.then(|| format!("/machines/{code}/types/new")),
        machine_href: format!("/machines/{code}"),
        machine_text,
    };
    Ok(render(&page, request.request_id())?.into_response())
}
