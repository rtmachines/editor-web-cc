//! Tests of the layout and the navigation links (task 5.1, design D5).

use axum::extract::FromRequestParts;
use axum::http::{Method, Request};
use recipe_app::auth::Operation;
use recipe_domain::identity::Role;

use super::{Layout, NavTarget, nav_links};
use crate::client::ClientAddr;
use crate::i18n::{Bundles, Lang};
use crate::pages::PageRequest;
use crate::test_support::PEER;

/// Return the paths of the links in order.
fn paths(links: &[super::NavLink]) -> Vec<&'static str> {
    links.iter().map(|link| link.path).collect()
}

/// Build the request data of a request with the method and the URI.
async fn page_request(method: Method, uri: &str) -> PageRequest {
    let request = Request::builder().method(method).uri(uri).body(()).unwrap();
    let (mut parts, ()) = request.into_parts();
    parts.extensions.insert(ClientAddr::of(PEER.ip()));
    PageRequest::from_request_parts(&mut parts, &())
        .await
        .map_err(|_rejection| "rejected")
        .unwrap()
}

#[test]
fn nav_target_names_its_path_text_key_and_query() {
    let rows = [
        (
            NavTarget::Recipes,
            "/recipes",
            "nav-recipes",
            Operation::ListRecipes,
        ),
        (
            NavTarget::Users,
            "/users",
            "nav-users",
            Operation::ListUsers,
        ),
        (NavTarget::Me, "/me", "nav-me", Operation::GetOwnProfile),
    ];
    for (target, path, key, operation) in rows {
        assert_eq!(target.path(), path);
        assert_eq!(target.text_key(), key);
        assert_eq!(target.operation(), operation);
    }
}

#[test]
fn nav_links_of_an_administrator_show_all_three_pages() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    let links = nav_links(texts, Role::Administrator, false, None);
    assert_eq!(paths(&links), ["/recipes", "/users", "/me"]);
    assert!(links.iter().all(|link| !link.current));
    assert_eq!(
        links.first().map(|link| link.text.as_str()),
        Some("Recipes")
    );
}

#[test]
fn nav_links_of_a_process_engineer_and_an_operator_hide_the_user_list() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::Pl);
    for role in [Role::ProcessEngineer, Role::Operator] {
        let links = nav_links(texts, role, false, None);
        assert_eq!(paths(&links), ["/recipes", "/me"], "{role}");
    }
}

#[test]
fn nav_links_follow_the_requirement_of_the_query() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    for role in Role::ALL {
        let shown = paths(&nav_links(texts, role, false, None));
        for target in [NavTarget::Recipes, NavTarget::Users, NavTarget::Me] {
            let allowed = target.operation().requirement().allows(role);
            assert_eq!(shown.contains(&target.path()), allowed, "{role} {target:?}");
        }
    }
}

#[test]
fn nav_links_are_empty_while_the_password_must_change() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    for role in Role::ALL {
        assert!(nav_links(texts, role, true, Some(NavTarget::Me)).is_empty());
    }
}

#[test]
fn nav_links_mark_only_the_current_target() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    let links = nav_links(texts, Role::Administrator, false, Some(NavTarget::Users));
    let current: Vec<&str> = links
        .iter()
        .filter(|link| link.current)
        .map(|link| link.path)
        .collect();
    assert_eq!(current, ["/users"]);
}

#[test]
fn nav_links_of_a_target_the_role_lacks_show_no_current_link() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    let links = nav_links(texts, Role::Operator, false, Some(NavTarget::Users));
    assert!(links.iter().all(|link| !link.current));
}

#[tokio::test]
async fn path_and_query_of_a_get_request_holds_path_and_query() {
    let with_query = page_request(Method::GET, "/users?q=ab").await;
    assert_eq!(with_query.path_and_query(), Some("/users?q=ab"));
    let without = page_request(Method::GET, "/me").await;
    assert_eq!(without.path_and_query(), Some("/me"));
}

#[tokio::test]
async fn path_and_query_of_another_method_is_none() {
    for method in [Method::POST, Method::PUT, Method::DELETE] {
        let request = page_request(method, "/users/anowak/reset-password").await;
        assert_eq!(request.path_and_query(), None);
    }
}

#[test]
fn layout_for_login_has_the_nonce_and_next_login_and_no_session_data() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    let layout = Layout::for_login(texts, Lang::En, "Sign in".to_owned(), "abc");
    assert_eq!(layout.pre_nonce.as_deref(), Some("abc"));
    assert_eq!(layout.next.as_deref(), Some("/login"));
    assert!(layout.nav.is_empty());
    assert!(layout.csrf_token.is_empty());
    assert_eq!(layout.user_login, None);
    assert_eq!(layout.toast, None);
}

#[test]
fn layout_bare_has_no_navigation_switch_token_or_user() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::Pl);
    let layout = Layout::bare(texts, Lang::Pl, "E-WEB-005".to_owned());
    assert!(layout.nav.is_empty());
    assert_eq!(layout.next, None);
    assert_eq!(layout.pre_nonce, None);
    assert_eq!(layout.user_login, None);
    assert!(layout.csrf_token.is_empty());
}

#[test]
fn layout_with_toast_sets_the_toast_text() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    let layout = Layout::bare(texts, Lang::En, "t".to_owned()).with_toast("Done.".to_owned());
    assert_eq!(layout.toast.as_deref(), Some("Done."));
}
