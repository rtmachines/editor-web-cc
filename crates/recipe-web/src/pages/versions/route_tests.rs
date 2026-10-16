//! The matrix rows and the sweep pages of the version pages (design D12).
//!
//! `routes::tests` and `pages::sweep_tests` collect both lists. Wave 4 adds the rows of the version
//! list, the version form, and the version page. Wave 5 adds the row of the rebind (task 12.2).

use axum::http::Method;

use crate::routes::matrix::{Endpoint, Want, endpoint};

/// Return the matrix rows of the version routes.
///
/// A row holds an endpoint and the answer for each of the six principals: no session, the
/// Administrator, the ProcessEngineer, the Operator, the user who must change the password, and the
/// deactivated user (`api.md` section 3.3).
pub(crate) fn route_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Conflict, Forbidden, Login, Page, Password, Redirect};
    let view = [Login, Page, Page, Page, Password, Login];
    let rebound = Redirect("/recipes/R-100/versions/3");
    let create = [Login, Page, Page, Forbidden, Password, Login];
    // The recipe R-100 has the live draft 3, so a new version gives E-RCP-011 (`form_tests` covers
    // the success on a recipe without a draft).
    let blocked = Conflict("E-RCP-011");
    vec![
        (endpoint(Method::GET, "/recipes/{ref}/versions", ""), view),
        (
            endpoint(Method::GET, "/recipes/{ref}/versions/new", ""),
            create,
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions",
                "based_on=2&change_note=Next",
            ),
            [Login, blocked, blocked, Forbidden, Password, Login],
        ),
        (
            endpoint(Method::GET, "/recipes/{ref}/versions/{n}", ""),
            view,
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/rebind",
                REBIND_BODY,
            ),
            [Login, rebound, rebound, Forbidden, Password, Login],
        ),
    ]
}

/// A rebind of `ZONES` from `ST_Zone@2` to `ST_Zone@3` with the value of each new leaf.
///
/// The identifiers are the fixed ones of `seed_plant` and `seed_recipes`. A command without
/// `HX-Request` answers `303` to the version page.
const REBIND_BODY: &str = "row_version=1&from_type_id=73570003-0000-0000-0000-000000000002&to_type_id=73570013-0000-0000-0000-000000000001&new_member_values[73570007-0000-0000-0000-000000000017][1].temp_min=1&new_member_values[73570007-0000-0000-0000-000000000017][2].temp_min=2&new_member_values[73570007-0000-0000-0000-000000000017][3].temp_min=3&new_member_values[73570007-0000-0000-0000-000000000017][4].temp_min=4";

/// Return the pattern and the request path of each full `GET` page of the version pages.
///
/// The sweep requests each page in both languages. Without `HX-Request`, the version list answers
/// the recipe page.
pub(crate) fn sweep_pages() -> Vec<(&'static str, String)> {
    vec![
        (
            "/recipes/{ref}/versions",
            "/recipes/R-100/versions".to_owned(),
        ),
        (
            "/recipes/{ref}/versions/new",
            "/recipes/R-100/versions/new".to_owned(),
        ),
        (
            "/recipes/{ref}/versions/{n}",
            "/recipes/R-100/versions/3".to_owned(),
        ),
    ]
}
