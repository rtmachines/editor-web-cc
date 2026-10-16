//! The matrix rows and the sweep pages of the parameter pages (design D12).
//!
//! `routes::tests` and `pages::sweep_tests` collect both lists. The agents of wave 5 add the rows
//! of the parameter routes here. The placeholder `{key}` names the parameter `LINE` of `R-100`.

use axum::http::Method;

use crate::routes::matrix::{Endpoint, Want, endpoint};

/// The body of a valid `AddParameter` on the draft 3 of `R-100`. The draft has the `row_version` 1.
const ADD_BODY: &str = "row_version=1&unit_path=U1&tag=MATRIX_P&name_en=Matrix&description_en=Matrix+row&type=INT&value_type=INT&s7_address=DB9.DBW0&ordinal=0&value=1";
/// The body of `SetParameterValue` for `LINE` (`ST_Line@1`): four zones with the value 0 in each leaf.
const VALUE_BODY: &str = "row_version=1&value.zones%5B1%5D.speed=0&value.zones%5B1%5D.temp_max=0&value.zones%5B2%5D.speed=0&value.zones%5B2%5D.temp_max=0&value.zones%5B3%5D.speed=0&value.zones%5B3%5D.temp_max=0&value.zones%5B4%5D.speed=0&value.zones%5B4%5D.temp_max=0";
/// The body of a valid save of the edit form of `LINE` without a change.
const DEFINITION_BODY: &str = "row_version=1&unit_path=U1&tag=LINE&name_en=line&description_en=The+zones+of+the+press+line&type=ST_Line%401&value_type=ST_Line%401&s7_address=DB1.DBB0&ordinal=0&value.zones%5B1%5D.speed=0&value.zones%5B1%5D.temp_max=0&value.zones%5B2%5D.speed=0&value.zones%5B2%5D.temp_max=0&value.zones%5B3%5D.speed=0&value.zones%5B3%5D.temp_max=0&value.zones%5B4%5D.speed=0&value.zones%5B4%5D.temp_max=0";
/// The path of the page of `LINE`. A save of the edit form without HTMX goes to it.
const LINE_PAGE: &str = "/recipes/R-100/versions/3/parameters/73570007-0000-0000-0000-000000000001";

/// Return the matrix rows of the parameter routes.
///
/// A row holds an endpoint and the answer for each of the six principals: no session, the
/// Administrator, the ProcessEngineer, the Operator, the user who must change the password, and the
/// deactivated user (`api.md` section 3.3).
pub(crate) fn route_rows() -> Vec<(Endpoint, [Want; 6])> {
    let mut rows = new_rows();
    rows.extend(parameter_rows());
    rows.extend(table_and_translation_rows());
    rows
}

/// The answers of a route that every role reads.
const VIEW: [Want; 6] = [
    Want::Login,
    Want::Page,
    Want::Page,
    Want::Page,
    Want::Password,
    Want::Login,
];
/// The answers of a route that needs `ParameterEdit`.
const EDIT: [Want; 6] = [
    Want::Login,
    Want::Page,
    Want::Page,
    Want::Forbidden,
    Want::Password,
    Want::Login,
];

/// Return the rows of the form of a new parameter and of `AddParameter`.
fn new_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Forbidden, Login, Password, Redirect};
    let added = Redirect("/recipes/R-100/versions/3");
    vec![
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/new",
                "",
            ),
            EDIT,
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters",
                ADD_BODY,
            ),
            [Login, added, added, Forbidden, Password, Login],
        ),
    ]
}

/// Return the rows of the page, the edit form, the value, and the limits of `LINE`.
fn parameter_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Forbidden, Login, Password, Redirect};
    let (view, edit) = (VIEW, EDIT);
    let saved = Redirect(LINE_PAGE);
    vec![
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/{key}",
                "",
            ),
            view,
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/definition",
                DEFINITION_BODY,
            ),
            [Login, saved, saved, Forbidden, Password, Login],
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/value",
                VALUE_BODY,
            ),
            edit,
        ),
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
                "",
            ),
            view,
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
                "row_version=1",
            ),
            edit,
        ),
    ]
}

/// Return the rows of the table, the translations, and the removal of `LINE`.
///
/// The translation `pl` of `LINE` exists, so its removal succeeds. A command without HTMX goes to
/// the parameter page or, after the removal, to the version page.
fn table_and_translation_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Forbidden, Login, Password, Redirect};
    let saved = Redirect(LINE_PAGE);
    let removed = Redirect("/recipes/R-100/versions/3");
    let translation = "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}";
    vec![
        (
            endpoint(Method::GET, "/recipes/{ref}/versions/{n}/parameters", ""),
            VIEW,
        ),
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/{key}/translations",
                "",
            ),
            VIEW,
        ),
        (
            endpoint(
                Method::POST,
                translation,
                "row_version=1&name=Linia&description=",
            ),
            [Login, saved, saved, Forbidden, Password, Login],
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}/delete",
                "row_version=1",
            ),
            [Login, saved, saved, Forbidden, Password, Login],
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/delete",
                "row_version=1",
            ),
            [Login, removed, removed, Forbidden, Password, Login],
        ),
    ]
}

/// Return the pattern and the request path of each full `GET` page of the parameter pages.
///
/// The sweep requests each page in both languages. Without `HX-Request`, the table answers the
/// version page, and the limit rows and the translation rows answer the parameter page.
pub(crate) fn sweep_pages() -> Vec<(&'static str, String)> {
    vec![
        (
            "/recipes/{ref}/versions/{n}/parameters",
            "/recipes/R-100/versions/3/parameters".to_owned(),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/{key}/translations",
            format!("{LINE_PAGE}/translations"),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/new",
            "/recipes/R-100/versions/3/parameters/new".to_owned(),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/{key}",
            LINE_PAGE.to_owned(),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
            format!("{LINE_PAGE}/limits"),
        ),
    ]
}
