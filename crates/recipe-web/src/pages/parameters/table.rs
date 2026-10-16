//! `GET .../parameters`: the parameter table of a version (`api.md` section 2.6, design D7,
//! ADR-0043 (j)).
//!
//! An HTMX request gets the fragment `parameter-table.html`. Any other request gets the version
//! page, which holds the same table. The removal of a parameter answers the same fragment.

use std::collections::BTreeSet;

use askama::Template;
use axum::extract::rejection::{PathRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::Uri;
use axum::response::{Html, IntoResponse, Response};
use recipe_app::recipe::{GetVersion, GetVersionInput, ListParameters, ListParametersInput};
use recipe_domain::plant::UnitPath;
use recipe_domain::recipe::ParameterKey;
use serde::Deserialize;

use super::reply::{Ctx, Target};
use crate::error::WebError;
use crate::pages::path_values::{Page, page as read_page, query_values, unit_path};
use crate::pages::render::render;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;
use crate::pages::versions::page::version;
use crate::pages::versions::parameter_table::ParameterTable;
use crate::pages::versions::path_version;
use crate::state::WebState;

/// The query string of `GET .../parameters`. A key other than `unit` and `page` gives
/// `422 E-WEB-008`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TableQuery {
    unit: Option<String>,
    page: Option<String>,
}

/// The fragment `parameter-table.html` alone.
#[derive(Template)]
#[template(path = "parameter-table.html")]
struct TableFragment<'a> {
    t: Texts<'a>,
    table: ParameterTable,
}

/// Render the parameter table of the version of `ctx`: one page of one unit or of every unit.
///
/// The table reads `GetVersion` for the outdated parameters and then `ListParameters`, as the
/// version page does.
///
/// # Errors
///
/// Returns the error of the queries, or `E-WEB-010` when the template cannot render.
pub(crate) async fn table_body(
    ctx: Ctx<'_>,
    unit: Option<UnitPath>,
    page: Page,
) -> Result<Html<String>, WebError> {
    let target = ctx.target;
    let user = ctx.request.auth()?.user();
    let app = ctx.state.identity().app().clone();
    let input = GetVersionInput {
        reference: target.reference.clone(),
        number: target.number,
        rebind_bound: 0,
    };
    let view = GetVersion::new(app.clone())
        .execute(user, &ctx.request.meta(), input)
        .await?;
    let input = ListParametersInput {
        reference: target.reference.clone(),
        number: target.number,
        unit,
        page: page.index,
    };
    let list = ListParameters::new(app)
        .execute(user, &ctx.request.meta(), input)
        .await?;
    let outdated: BTreeSet<ParameterKey> = view.outdated.iter().map(|item| item.key).collect();
    let t = ctx.texts();
    let table = ParameterTable::new(t, &target.version_href(), &list, &outdated, page);
    render(&TableFragment { t, table }, ctx.request.request_id())
}

/// `GET .../parameters`: show the parameter table, or the version page without HTMX (design D7).
pub(crate) async fn table(
    State(state): State<WebState>,
    path: Result<Path<(String, String)>, PathRejection>,
    uri: Uri,
    query: Result<Query<TableQuery>, QueryRejection>,
    request: PageRequest,
) -> Result<Response, WebError> {
    if !request.is_htmx() {
        return version(State(state), path, Query::try_from_uri(&uri), request).await;
    }
    let (reference, number) = path_version(path)?;
    let query = query_values(query)?;
    let unit = query.unit.as_deref().map(unit_path).transpose()?;
    let page = read_page(query.page.as_deref())?;
    let target = Target {
        reference,
        number,
        key: None,
    };
    let ctx = Ctx {
        state: &state,
        request: &request,
        target: &target,
    };
    Ok(table_body(ctx, unit, page).await?.into_response())
}
