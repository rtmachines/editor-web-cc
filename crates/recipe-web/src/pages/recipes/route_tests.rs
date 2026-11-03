//! The matrix rows and the sweep pages of the recipe pages (design D12).
//!
//! `routes::tests` and `pages::sweep_tests` collect both lists. The row `GET /recipes` moved here
//! from the rows of step P3.1. Wave 4 adds the rows of the form, the recipe page, and the rename.

use axum::http::Method;

use crate::routes::matrix::{Endpoint, Want, endpoint};

/// Return the matrix rows of the recipe routes.
///
/// A row holds an endpoint and the answer for each of the six principals: no session, the
/// Administrator, the ProcessEngineer, the Operator, the user who must change the password, and the
/// deactivated user (`api.md` section 3.3).
pub(crate) fn route_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Conflict, Forbidden, Login, Page, Password, Redirect};
    let view = [Login, Page, Page, Page, Password, Login];
    let create = [Login, Page, Page, Forbidden, Password, Login];
    let created = Redirect("/recipes/R-500/versions/1");
    let renamed = Redirect("/recipes/R-100");
    // `R-100` is live and has an approved version, so the delete is refused and the restore too.
    let delete = [
        Login,
        Conflict("E-RCP-010"),
        Conflict("E-RCP-010"),
        Forbidden,
        Password,
        Login,
    ];
    let restore = [
        Login,
        Conflict("E-RCP-025"),
        Forbidden,
        Forbidden,
        Password,
        Login,
    ];
    vec![
        (endpoint(Method::GET, "/recipes", ""), view),
        (endpoint(Method::GET, "/recipes/new", ""), create),
        (
            endpoint(
                Method::POST,
                "/recipes",
                "machine_code=M7&reference_number=R-500&name=Created&change_note=First",
            ),
            [Login, created, created, Forbidden, Password, Login],
        ),
        (endpoint(Method::GET, "/recipes/{ref}", ""), view),
        (
            endpoint(Method::POST, "/recipes/{ref}/rename", "name=Renamed"),
            [Login, renamed, renamed, Forbidden, Password, Login],
        ),
        (endpoint(Method::POST, "/recipes/{ref}/delete", ""), delete),
        (
            endpoint(Method::POST, "/recipes/{ref}/restore", ""),
            restore,
        ),
    ]
}

/// Return the pattern and the request path of each full `GET` page of the recipe pages.
///
/// The sweep requests each page in both languages.
pub(crate) fn sweep_pages() -> Vec<(&'static str, String)> {
    vec![
        ("/recipes", "/recipes".to_owned()),
        ("/recipes/new", "/recipes/new".to_owned()),
        ("/recipes/{ref}", "/recipes/R-100".to_owned()),
    ]
}

// --- Read-only mode (ADR-0113 (d)) --------------------------------------------------------------

/// Spec "Read-only pages", ADR-0113 (d): in read-only mode `GET /recipes/new` answers `200` with
/// the heading and the notice, and no form that sends `POST /recipes`.
#[cfg(test)]
#[tokio::test]
async fn recipes_new_form_in_read_only_mode_shows_the_notice_in_place_of_the_form() {
    use axum::http::StatusCode;

    use super::test_world::recipe_world;
    use crate::i18n::Lang;
    use crate::test_support::plant::{cookie_of, main_of};
    use crate::test_support::text;

    let world = recipe_world();
    world.fixture.state.read_only().set(true);
    for lang in [Lang::En, Lang::Pl] {
        let notice = text(lang, "readonly-form-unavailable");
        let heading = text(lang, "recipe-new-title");
        let reply = world
            .get(&world.engineer, "/recipes/new", &[cookie_of(lang)])
            .await;
        let body = main_of(&reply.body);
        assert_eq!(reply.status, StatusCode::OK, "{lang}: {body}");
        assert!(body.contains(&notice), "{lang}: {body}");
        assert!(
            body.contains(&format!("<h1>{heading}</h1>")),
            "{lang}: {body}"
        );
        for forbidden in [
            "id=\"recipe-new-form\"",
            "id=\"recipe-create-submit\"",
            "action=\"/recipes\"",
            "<input",
        ] {
            assert!(!body.contains(forbidden), "{lang} {forbidden}: {body}");
        }
    }
}

/// Regression, ADR-0113 (d): the permission check runs before the notice, so an Operator gets
/// `403 E-WEB-002` in read-only mode.
#[cfg(test)]
#[tokio::test]
async fn recipes_new_form_in_read_only_mode_still_refuses_an_operator() {
    use axum::http::StatusCode;

    use super::test_world::recipe_world;

    let world = recipe_world();
    world.fixture.state.read_only().set(true);
    let reply = world.get_en(&world.operator, "/recipes/new").await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"), "{}", reply.body);
}
