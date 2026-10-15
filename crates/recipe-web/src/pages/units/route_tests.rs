//! The matrix rows and the sweep pages of the unit pages (design D12).
//!
//! `routes::tests` and `pages::sweep_tests` collect both lists. The rows of each task are added in
//! the order of the tasks.

use axum::http::Method;

use crate::routes::matrix::{Endpoint, Want, endpoint};

/// Return the matrix rows of the unit routes.
///
/// A row holds an endpoint and the answer for each of the six principals: no session, the
/// Administrator, the ProcessEngineer, the Operator, the user who must change the password, and the
/// deactivated user (`api.md` section 3.3).
pub(crate) fn route_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Forbidden, Login, Page, Password, Redirect};
    let manage = [Login, Page, Page, Forbidden, Password, Login];
    let view = [Login, Page, Page, Page, Password, Login];
    let added = Redirect("/machines/M7");
    let renamed = Redirect("/machines/M7/units/U1.2");
    let archived = Redirect("/machines/M7?archived=U1.2");
    vec![
        (
            endpoint(Method::GET, "/machines/{code}/units/new", ""),
            manage,
        ),
        (
            endpoint(
                Method::POST,
                "/machines/{code}/units",
                "parent_path={path}&segment=5&name=Added",
            ),
            [Login, added, added, Forbidden, Password, Login],
        ),
        (
            endpoint(Method::GET, "/machines/{code}/units/{path}", ""),
            view,
        ),
        (
            endpoint(
                Method::POST,
                "/machines/{code}/units/{path}/rename",
                "name=Renamed",
            ),
            [Login, renamed, renamed, Forbidden, Password, Login],
        ),
        (
            endpoint(Method::POST, "/machines/{code}/units/{path}/archive", ""),
            [Login, archived, archived, Forbidden, Password, Login],
        ),
    ]
}

/// Return the pattern and the request path of each full `GET` page of the unit pages.
///
/// The sweep requests each page in both languages.
pub(crate) fn sweep_pages() -> Vec<(&'static str, String)> {
    vec![
        (
            "/machines/{code}/units/new",
            "/machines/M7/units/new".to_owned(),
        ),
        (
            "/machines/{code}/units/{path}",
            "/machines/M7/units/U1.2".to_owned(),
        ),
    ]
}
