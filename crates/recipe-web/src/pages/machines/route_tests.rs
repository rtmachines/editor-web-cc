//! The matrix rows and the sweep pages of the machine pages (design D12).
//!
//! `routes::tests` and `pages::sweep_tests` collect both lists. The rows of task 7.1 and later
//! tasks are added in the order of the tasks.

use axum::http::Method;

use crate::routes::matrix::{Endpoint, Want, endpoint};
use crate::test_support::plant::M12_ID;

/// Return the matrix rows of the machine routes.
///
/// A row holds an endpoint and the answer for each of the six principals: no session, the
/// Administrator, the ProcessEngineer, the Operator, the user who must change the password, and the
/// deactivated user (`api.md` section 3.3).
pub(crate) fn route_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Conflict, Forbidden, Login, Page, Password, Redirect};
    // The machine M7 has a live draft, so its archive gives E-PLT-011 (a test of the module
    // `commands_tests` covers the success on a machine without a draft).
    let blocked = Conflict("E-PLT-011");
    let renamed = Redirect("/machines/M7");
    let view = [Login, Page, Page, Page, Password, Login];
    let manage = [Login, Page, Page, Forbidden, Password, Login];
    let created = Redirect("/machines/M99");
    vec![
        (endpoint(Method::GET, "/machines", ""), view),
        (endpoint(Method::GET, "/machines/new", ""), manage),
        (
            endpoint(
                Method::POST,
                "/machines",
                "code=M99&name=Created&plc_platform=S7_300&main_unit_name=Infeed",
            ),
            [Login, created, created, Forbidden, Password, Login],
        ),
        (endpoint(Method::GET, "/machines/{code}", ""), view),
        (endpoint(Method::GET, "/machines/by-id/{uuid}", ""), view),
        (endpoint(Method::GET, "/machines/{code}/units", ""), view),
        (
            endpoint(Method::POST, "/machines/{code}/rename", "name=Renamed"),
            [Login, renamed, renamed, Forbidden, Password, Login],
        ),
        (
            endpoint(Method::POST, "/machines/{code}/archive", ""),
            [Login, blocked, blocked, Forbidden, Password, Login],
        ),
    ]
}

/// Return the pattern and the request path of each full `GET` page of the machine pages.
///
/// The sweep requests each page in both languages.
pub(crate) fn sweep_pages() -> Vec<(&'static str, String)> {
    vec![
        ("/machines", "/machines".to_owned()),
        ("/machines/new", "/machines/new".to_owned()),
        ("/machines/{code}", "/machines/M7".to_owned()),
        (
            "/machines/by-id/{uuid}",
            format!("/machines/by-id/{M12_ID}"),
        ),
        ("/machines/{code}/units", "/machines/M7/units".to_owned()),
    ]
}
