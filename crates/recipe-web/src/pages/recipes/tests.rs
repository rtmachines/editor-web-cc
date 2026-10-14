//! Tests of `GET /recipes` (task 6.2, design D9, spec "Recipe list page").

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use crate::i18n::Lang;
use crate::pages::test_helpers::assert_clean_html;
use crate::test_support::{Fixture, Reply, TestSession, get_page, text};

/// Send `GET /recipes` in the language `code`, with the session when there is one.
async fn get_recipes(fixture: &Fixture, session: Option<&TestSession>, code: &str) -> Reply {
    get_page(fixture, session, code, "/recipes").await
}

#[tokio::test]
async fn recipes_page_of_an_operator_shows_the_heading_and_the_empty_state_in_english() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);
    let reply = get_recipes(&fixture, Some(&session), "en").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("<h1>Recipes</h1>"), "{}", reply.body);
    assert_eq!(reply.body.matches("<h1").count(), 1);
    assert!(
        reply
            .body
            .contains("<p class=\"empty\">No recipe to show.</p>"),
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .contains("<title>Recipes · Recipe editor</title>")
    );
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn recipes_page_shows_the_heading_and_the_empty_state_in_polish() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);
    let reply = get_recipes(&fixture, Some(&session), "pl").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let expected = format!("<h1>{}</h1>", text(Lang::Pl, "recipes-title"));
    assert!(reply.body.contains(&expected), "{}", reply.body);
    assert!(reply.body.contains("Brak receptur do pokazania."));
    assert!(reply.body.contains("<html lang=\"pl\">"));
}

#[tokio::test]
async fn recipes_page_answers_every_role() {
    let fixture = Fixture::new();
    for (n, role) in [
        (1, Role::Administrator),
        (2, Role::ProcessEngineer),
        (3, Role::Operator),
    ] {
        let session = fixture.seed_session(n, role, false);
        let reply = get_recipes(&fixture, Some(&session), "en").await;
        assert_eq!(reply.status, StatusCode::OK, "{role:?}: {}", reply.body);
    }
}

#[tokio::test]
async fn recipes_page_without_a_session_redirects_to_the_login() {
    let fixture = Fixture::new();
    let reply = get_recipes(&fixture, None, "en").await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/login");
}

#[tokio::test]
async fn recipes_page_during_a_password_change_redirects_to_the_password_page() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(3, Role::Operator, true);
    let reply = get_recipes(&fixture, Some(&session), "en").await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{}", reply.body);
    assert_eq!(reply.header("location"), "/me/password");
}

#[tokio::test]
async fn recipes_page_works_in_read_only_mode() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::ProcessEngineer, false);
    fixture.state.read_only().set(true);
    let reply = get_recipes(&fixture, Some(&session), "en").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("<h1>Recipes</h1>"));
}

#[tokio::test]
async fn recipes_page_navigation_of_an_operator_marks_the_recipe_list() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(3, Role::Operator, false);
    let reply = get_recipes(&fixture, Some(&session), "en").await;
    let nav = reply.body.split_once("<nav").unwrap().1;
    let nav = nav.split("</nav>").next().unwrap();
    assert!(
        nav.contains("<a class=\"nav-link\" href=\"/recipes\" aria-current=\"page\">Recipes</a>"),
        "{nav}"
    );
    assert!(nav.contains("<a class=\"nav-link\" href=\"/me\">My account</a>"));
    assert!(!nav.contains("/users"), "{nav}");
    assert_eq!(nav.matches("aria-current").count(), 1);
}

#[tokio::test]
async fn recipes_page_holds_the_language_switch_that_returns_to_the_list() {
    let fixture = Fixture::new();
    let session = fixture.seed_session(3, Role::Operator, false);
    let reply = get_recipes(&fixture, Some(&session), "en").await;
    assert!(
        reply
            .body
            .contains("<input type=\"hidden\" name=\"next\" value=\"/recipes\">"),
        "{}",
        reply.body
    );
}
