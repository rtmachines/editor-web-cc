//! The matrix rows and the sweep pages of the type pages (design D12).
//!
//! `routes::tests` and `pages::sweep_tests` collect both lists. Each row holds the answer of the six
//! principals of `api.md` section 3.3: no session, Administrator, ProcessEngineer, Operator, a user
//! with `must_change_password`, and a user who was deactivated after the session started.

use axum::http::Method;

use super::test_kit::get_as;
use crate::routes::matrix::{Endpoint, PRINCIPALS, Want, answer, assert_want, endpoint};
use crate::test_support::plant::world;

/// Return the matrix rows of the type routes.
///
/// A row holds an endpoint and the answer for each of the six principals.
pub(crate) fn route_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::*;
    let view = [Login, Page, Page, Page, Password, Login];
    let manage = [Login, Page, Page, Forbidden, Password, Login];
    let defined = [
        Login,
        Redirect("/machines/M7/types/ST_Press"),
        Redirect("/machines/M7/types/ST_Press"),
        Forbidden,
        Password,
        Login,
    ];
    // The world of the matrix seeds `ST_Zone@3` (design D12), so a new revision is the fourth.
    let revised = [
        Login,
        Redirect("/machines/M7/types/ST_Zone?rev=4"),
        Redirect("/machines/M7/types/ST_Zone?rev=4"),
        Forbidden,
        Password,
        Login,
    ];
    vec![
        (endpoint(Method::GET, "/machines/{code}/types", ""), view),
        (
            endpoint(Method::GET, "/machines/{code}/types/new", ""),
            manage,
        ),
        (
            endpoint(Method::POST, "/machines/{code}/types", NEW_BODY),
            defined,
        ),
        (
            endpoint(
                Method::POST,
                "/machines/{code}/types/{name}/revisions",
                REVISION_BODY,
            ),
            revised,
        ),
        (
            endpoint(Method::GET, "/machines/{code}/types/{name}", ""),
            view,
        ),
        (
            endpoint(Method::GET, "/machines/{code}/types/{name}/usage", ""),
            view,
        ),
    ]
}

/// The body that defines the new struct `ST_Press` with one member.
const NEW_BODY: &str = "name=ST_Press&members[0].name=speed&members[0].type=INT";
/// The body that defines the next revision of `ST_Zone` with one member.
const REVISION_BODY: &str = "members[0].name=speed&members[0].type=INT";

/// Return the pattern and the request path of each full `GET` page of the type pages.
///
/// The sweep requests each page in both languages.
pub(crate) fn sweep_pages() -> Vec<(&'static str, String)> {
    vec![
        ("/machines/{code}/types", "/machines/M7/types".to_owned()),
        (
            "/machines/{code}/types/new",
            "/machines/M7/types/new".to_owned(),
        ),
        (
            "/machines/{code}/types/{name}",
            "/machines/M7/types/ST_Line".to_owned(),
        ),
        (
            "/machines/{code}/types/{name}/usage",
            "/machines/M7/types/ST_Zone/usage?rev=2".to_owned(),
        ),
    ]
}

#[tokio::test]
async fn types_matrix_each_row_of_the_type_area_answers_each_principal() {
    for (endpoint, wants) in route_rows() {
        for (principal, want) in PRINCIPALS.into_iter().zip(wants) {
            let reply = answer(&endpoint, principal).await;
            let what = format!("{} {} as {principal:?}", endpoint.method, endpoint.path);
            assert_want(&reply, want, &what);
        }
    }
}

#[test]
fn types_matrix_rows_and_sweep_pages_name_routes_of_the_table() {
    let table: Vec<(String, &str)> = crate::routes::table()
        .iter()
        .map(|row| (row.method.to_string(), row.path))
        .collect();
    for (endpoint, _wants) in route_rows() {
        let key = (endpoint.method.to_string(), endpoint.path);
        assert!(table.contains(&key), "{key:?} is not in the table");
    }
    for (pattern, _path) in sweep_pages() {
        assert!(table.contains(&("GET".to_owned(), pattern)), "{pattern}");
    }
    let area: Vec<(String, &str)> = super::rows()
        .iter()
        .map(|row| (row.method.to_string(), row.path))
        .collect();
    for key in &area {
        let in_matrix = route_rows()
            .iter()
            .any(|(endpoint, _wants)| (endpoint.method.to_string(), endpoint.path) == *key);
        assert!(in_matrix, "{key:?} has no matrix row");
    }
}

#[tokio::test]
async fn types_pages_mark_the_machines_link_of_the_top_bar_as_current() {
    let world = world();
    let link = "<a class=\"nav-link\" href=\"/machines\" aria-current=\"page\">Machines</a>";
    for (_pattern, path) in sweep_pages() {
        let reply = get_as(&world, &world.engineer, &path, &[]).await;
        let top = reply.body.split("</header>").next().unwrap();
        assert!(top.contains(link), "{path}: {top}");
        assert_eq!(top.matches("aria-current=\"page\"").count(), 1, "{path}");
    }
}
