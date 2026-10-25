//! Tests of the layout and the navigation links (task 5.1, design D5).

use askama::Template;
use axum::Router;
use axum::extract::connect_info::MockConnectInfo;
use axum::extract::{FromRequestParts, State};
use axum::http::{Method, Request, StatusCode};
use axum::response::Html;
use axum::routing::get;
use recipe_app::auth::Operation;
use recipe_domain::identity::Role;

use super::{Layout, NavTarget, ReadOnlyFlag, Shape, nav_links};
use crate::client::ClientAddr;
use crate::error::WebError;
use crate::i18n::{Bundles, Lang};
use crate::pages::{PageRequest, Texts};
use crate::routes::{Access, RouteEntry, assemble, table};
use crate::state::WebState;
use crate::test_support::{Fixture, PEER, Reply, get_with, text};

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
            NavTarget::Machines,
            "/machines",
            "nav-machines",
            Operation::ListMachines,
        ),
        (
            NavTarget::Users,
            "/users",
            "nav-users",
            Operation::ListUsers,
        ),
        (
            NavTarget::Audit,
            "/audit",
            "nav-audit",
            Operation::ListAuditEntries,
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
fn nav_links_of_an_administrator_show_all_five_pages_in_order() {
    // ADR-0051 (j): the fifth link "Audit" stands after "Users".
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    let links = nav_links(texts, Role::Administrator, false, None);
    assert_eq!(
        paths(&links),
        ["/recipes", "/machines", "/users", "/audit", "/me"]
    );
    assert!(links.iter().all(|link| !link.current));
    assert_eq!(
        links.first().map(|link| link.text.as_str()),
        Some("Recipes")
    );
    assert_eq!(
        links.get(1).map(|link| link.text.as_str()),
        Some("Machines")
    );
}

#[test]
fn nav_links_of_a_process_engineer_and_an_operator_hide_the_user_list() {
    // ADR-0051 (j): the ProcessEngineer sees "Audit" (`AuditViewRecipes`), the Operator does not.
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::Pl);
    let expected = [
        (
            Role::ProcessEngineer,
            vec!["/recipes", "/machines", "/audit", "/me"],
        ),
        (Role::Operator, vec!["/recipes", "/machines", "/me"]),
    ];
    for (role, paths_of_role) in expected {
        let links = nav_links(texts, role, false, None);
        assert_eq!(paths(&links), paths_of_role, "{role}");
    }
}

#[test]
fn nav_links_audit_link_has_its_text_in_both_languages_and_follows_the_row_of_the_query() {
    let bundles = Bundles::load().unwrap();
    for (lang, word) in [(Lang::En, "Audit"), (Lang::Pl, "Audyt")] {
        let texts = crate::pages::Texts::new(&bundles, lang);
        for role in Role::ALL {
            let audit = nav_links(texts, role, false, Some(NavTarget::Audit))
                .into_iter()
                .find(|link| link.path == "/audit");
            let allowed = Operation::ListAuditEntries.requirement().allows(role);
            assert_eq!(audit.is_some(), allowed, "{role} {lang}");
            if let Some(link) = audit {
                assert_eq!(link.text, word);
                assert!(link.current);
            }
        }
    }
}

#[test]
fn nav_links_audit_link_is_current_only_on_the_audit_target() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    let links = nav_links(texts, Role::Administrator, false, Some(NavTarget::Machines));
    let audit = links.iter().find(|link| link.path == "/audit").unwrap();
    assert!(!audit.current);
}

#[test]
fn nav_links_audit_link_is_hidden_while_the_password_must_change() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    assert!(nav_links(texts, Role::Administrator, true, Some(NavTarget::Audit)).is_empty());
}

#[test]
fn nav_links_follow_the_requirement_of_the_query() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    for role in Role::ALL {
        let shown = paths(&nav_links(texts, role, false, None));
        for target in [
            NavTarget::Recipes,
            NavTarget::Machines,
            NavTarget::Users,
            NavTarget::Audit,
            NavTarget::Me,
        ] {
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
fn nav_links_mark_the_machines_target_as_the_only_current_link() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    let links = nav_links(texts, Role::Operator, false, Some(NavTarget::Machines));
    let current: Vec<&str> = links
        .iter()
        .filter(|link| link.current)
        .map(|link| link.path)
        .collect();
    assert_eq!(current, ["/machines"]);
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
async fn page_request_hx_target_is_the_header_value_or_none() {
    let request = Request::builder()
        .uri("/me")
        .header("hx-target", "content")
        .body(())
        .unwrap();
    let (mut parts, ()) = request.into_parts();
    parts.extensions.insert(ClientAddr::of(PEER.ip()));
    let with = PageRequest::from_request_parts(&mut parts, &())
        .await
        .map_err(|_rejection| "rejected")
        .unwrap();
    assert_eq!(with.hx_target(), Some("content"));
    let without = page_request(Method::GET, "/me").await;
    assert_eq!(without.hx_target(), None);
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
    let layout = Layout::for_login(
        texts,
        Lang::En,
        "Sign in".to_owned(),
        "abc",
        ReadOnlyFlag::default(),
    );
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
    let layout = Layout::bare(
        texts,
        Lang::Pl,
        "E-WEB-005".to_owned(),
        ReadOnlyFlag::default(),
    );
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
    let layout = Layout::bare(texts, Lang::En, "t".to_owned(), ReadOnlyFlag::default())
        .with_toast("Done.".to_owned());
    assert_eq!(layout.toast.as_deref(), Some("Done."));
}

// --- Spec "Content answer of a page" (design D6, ADR-0037 w) ------------------------------------

/// The path and the query of the test page. The field `next` of the switch holds both.
const CONTENT_URI: &str = "/test/content?x=1";

/// The page that the tests of the answer shape ask for: the layout and one heading.
#[derive(Template)]
#[template(path = "test/content-page.html")]
struct ContentPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
}

/// The handler of the test page. It builds the layout the way a real page does.
async fn content_page(
    State(state): State<WebState>,
    request: PageRequest,
) -> Result<Html<String>, WebError> {
    let texts = Texts::new(state.bundles(), request.lang());
    let layout = Layout::for_session(texts, &request, texts.get("me-title"), Some(NavTarget::Me))?;
    Ok(Html(ContentPage { layout, t: texts }.render().unwrap()))
}

/// Send `GET /test/content?x=1` as an Operator in English with the extra headers.
async fn ask_content_page(headers: &[(&str, &str)]) -> Reply {
    let fixture = Fixture::new();
    let session = fixture.seed_session(2, Role::Operator, false);
    let mut rows = table();
    rows.push(RouteEntry {
        method: Method::GET,
        path: "/test/content",
        access: Access::Operation(Operation::GetOwnProfile),
        handler: get(content_page),
    });
    let router: Router =
        assemble(fixture.state.clone(), rows, Router::new()).layer(MockConnectInfo(PEER));
    let cookie = format!("{}; recipe_lang=en", session.cookie);
    let mut all = vec![("cookie", cookie.as_str())];
    all.extend_from_slice(headers);
    get_with(&router, CONTENT_URI, &all).await
}

#[test]
fn layout_of_the_login_page_and_the_error_page_has_the_page_shape() {
    let bundles = Bundles::load().unwrap();
    let texts = crate::pages::Texts::new(&bundles, Lang::En);
    let login = Layout::for_login(
        texts,
        Lang::En,
        "Sign in".to_owned(),
        "abc",
        ReadOnlyFlag::default(),
    );
    let bare = Layout::bare(
        texts,
        Lang::En,
        "E-WEB-005".to_owned(),
        ReadOnlyFlag::default(),
    );
    for layout in [login, bare] {
        assert_eq!(layout.shape, Shape::Page);
        assert!(!layout.is_content());
    }
}

/// The request headers of an HTMX request that targets `main`.
const HTMX_CONTENT: [(&str, &str); 2] = [("hx-request", "true"), ("hx-target", "content")];

#[tokio::test]
async fn layout_content_answer_holds_title_regions_and_heading() {
    let reply = ask_content_page(&HTMX_CONTENT).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let title = format!(
        "<title>{} · {}</title>",
        text(Lang::En, "me-title"),
        text(Lang::En, "app-name")
    );
    assert!(reply.body.contains(&title), "{}", reply.body);
    assert!(reply.body.contains("<div id=\"error-panel\"></div>"));
    assert!(
        reply
            .body
            .contains("<div id=\"toast\" role=\"status\" aria-live=\"polite\"></div>")
    );
    let heading = format!(
        "<h1 tabindex=\"-1\" autofocus>{}</h1>",
        text(Lang::En, "me-title")
    );
    assert!(reply.body.contains(&heading), "{}", reply.body);
}

#[tokio::test]
async fn layout_content_answer_holds_no_top_bar_and_no_skip_link() {
    let reply = ask_content_page(&HTMX_CONTENT).await;
    for frame in [
        "<!DOCTYPE",
        "<html",
        "<head",
        "<body",
        "<main",
        "class=\"topbar\"",
        "class=\"skip-link\"",
        "class=\"lang-switch\"",
        "<form",
        "<script",
    ] {
        assert!(!reply.body.contains(frame), "found {frame}: {}", reply.body);
    }
}

#[tokio::test]
async fn layout_content_answer_updates_the_language_field_out_of_band() {
    let reply = ask_content_page(&HTMX_CONTENT).await;
    let field = "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/test/content?x=1\" hx-swap-oob=\"true\">";
    assert!(reply.body.contains(field), "{}", reply.body);
    assert_eq!(reply.body.matches("name=\"next\"").count(), 1);
}

#[tokio::test]
async fn layout_full_page_keeps_its_frame_and_gives_the_field_next_an_id() {
    let reply = ask_content_page(&[]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    for frame in [
        "<!DOCTYPE html>",
        "class=\"skip-link\"",
        "class=\"topbar\"",
        "<main id=\"content\">",
        "</main>",
    ] {
        assert!(reply.body.contains(frame), "lost {frame}: {}", reply.body);
    }
    let field =
        "<input type=\"hidden\" id=\"lang-next\" name=\"next\" value=\"/test/content?x=1\">";
    assert!(reply.body.contains(field), "{}", reply.body);
    assert!(!reply.body.contains("hx-swap-oob"));
    assert!(!reply.body.contains("autofocus"));
    assert!(!reply.body.contains("tabindex"));
}

#[tokio::test]
async fn layout_htmx_request_with_another_target_gets_the_full_page() {
    for target in ["unit-form", "unit-tree", "machine-name", "body", "#content"] {
        let reply = ask_content_page(&[("hx-request", "true"), ("hx-target", target)]).await;
        assert!(reply.body.contains("class=\"topbar\""), "{target}");
        assert!(!reply.body.contains("hx-swap-oob"), "{target}");
        assert!(!reply.body.contains("autofocus"), "{target}");
    }
}

#[tokio::test]
async fn layout_htmx_request_without_a_target_gets_the_full_page() {
    let reply = ask_content_page(&[("hx-request", "true")]).await;
    assert!(reply.body.contains("class=\"topbar\""), "{}", reply.body);
    assert!(!reply.body.contains("hx-swap-oob"));
}

#[tokio::test]
async fn layout_target_content_without_hx_request_gets_the_full_page() {
    for headers in [
        vec![("hx-target", "content")],
        vec![("hx-request", "false"), ("hx-target", "content")],
    ] {
        let reply = ask_content_page(&headers).await;
        assert!(reply.body.contains("class=\"topbar\""), "{headers:?}");
        assert!(!reply.body.contains("hx-swap-oob"), "{headers:?}");
        assert!(!reply.body.contains("autofocus"), "{headers:?}");
    }
}
