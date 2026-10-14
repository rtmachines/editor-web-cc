//! Tests of the pages of the user administration (task 6.8, `api.md` section 2.8).

use axum::http::StatusCode;
use recipe_domain::audit::SecurityEventKind;
use recipe_domain::identity::Role;

use crate::pages::test_helpers::{
    assert_clean_html, assert_no_user_text_in_hx_attributes, set_cookie_of,
};
use crate::test_support::{
    Fixture, Reply, TestSession, capture_logs, get_with, post_fields, post_pre_session,
};

const PASSWORD: &str = "correct horse battery";
const WRONG_FIELD: &str = "wrong+password+here";
const HTMX: (&str, &str) = ("hx-request", "true");
const ENGLISH: (&str, &str) = ("cookie", "recipe_lang=en");
const ONE_TIME_ALPHABET: &str = "23456789ABCDEFGHJKLMNPQRSTUVWXYZ";

// --- Helpers ------------------------------------------------------------------------------------

/// A fixture with the Administrator `user1` and the Operator `user2` with a password.
fn world() -> (Fixture, TestSession) {
    let fixture = Fixture::new();
    let admin = fixture.seed_session(1, Role::Administrator, false);
    fixture.seed_account(2, Role::Operator, PASSWORD, false);
    (fixture, admin)
}

async fn get_as(fixture: &Fixture, session: &TestSession, path: &str) -> Reply {
    get_with(
        &fixture.router(),
        path,
        &[("cookie", &session.cookie), ENGLISH],
    )
    .await
}

async fn post_as(
    fixture: &Fixture,
    session: &TestSession,
    path: &str,
    body: &str,
    htmx: bool,
) -> Reply {
    let headers: &[(&str, &str)] = if htmx { &[HTMX, ENGLISH] } else { &[ENGLISH] };
    post_fields(&fixture.router(), path, Some(session), headers, body).await
}

/// Return the status of `GET /me` with the cookie. `200` means that the session lives.
async fn me_status(fixture: &Fixture, cookie: &str) -> StatusCode {
    get_with(&fixture.router(), "/me", &[("cookie", cookie)])
        .await
        .status
}

/// Return the one-time password that a page shows.
fn one_time_password(body: &str) -> String {
    let (_before, rest) = body.split_once("id=\"one-time-password\">").unwrap();
    rest.split('<').next().unwrap().to_owned()
}

fn assert_one_time_password_shape(password: &str) {
    assert_eq!(password.len(), 20, "{password}");
    assert!(password.chars().all(|c| ONE_TIME_ALPHABET.contains(c)));
}

async fn login(fixture: &Fixture, login: &str, password_field: &str) -> Reply {
    let fields = format!("login={login}&password={password_field}");
    post_pre_session(fixture, "/login", &fields, &[]).await
}

// --- GET /users ---------------------------------------------------------------------------------

#[tokio::test]
async fn pages_users_list_shows_the_users_with_a_search_form() {
    let (fixture, admin) = world();
    let reply = get_as(&fixture, &admin, "/users").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("<title>Users</title>"),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("href=\"/users/user1\""));
    assert!(reply.body.contains("href=\"/users/user2\""));
    assert!(reply.body.contains("href=\"/users/new\""));
    assert!(reply.body.contains("name=\"q\""));
    let expected = format!("hx-headers='{{\"X-CSRF-Token\":\"{}\"}}'", admin.csrf);
    assert!(reply.body.contains(&expected));
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn pages_users_list_filters_by_q_and_folds_the_case() {
    let (fixture, admin) = world();
    let reply = get_as(&fixture, &admin, "/users?q=user2").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("href=\"/users/user2\""));
    assert!(!reply.body.contains("href=\"/users/user1\""));
    let upper = get_as(&fixture, &admin, "/users?q=USER2").await;
    assert!(
        upper.body.contains("href=\"/users/user2\""),
        "{}",
        upper.body
    );
    let empty = get_as(&fixture, &admin, "/users?q=").await;
    assert!(empty.body.contains("href=\"/users/user1\""));
}

#[tokio::test]
async fn pages_users_list_maps_the_one_based_page_to_the_page_of_the_use_case() {
    let (fixture, admin) = world();
    let first = get_as(&fixture, &admin, "/users?page=1").await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    assert!(first.body.contains("href=\"/users/user1\""));
    let second = get_as(&fixture, &admin, "/users?page=2").await;
    assert_eq!(second.status, StatusCode::OK, "{}", second.body);
    assert!(!second.body.contains("href=\"/users/user1\""));
    assert!(second.body.contains("No user found."), "{}", second.body);
    assert!(
        second.body.contains("href=\"/users?page=1\""),
        "{}",
        second.body
    );
}

#[tokio::test]
async fn pages_users_list_rejects_a_bad_q_or_page_with_422() {
    let (fixture, admin) = world();
    let long_q = "a".repeat(65);
    let bad = [
        "/users?page=0".to_owned(),
        "/users?page=abc".to_owned(),
        "/users?page=-1".to_owned(),
        "/users?page=99999999999".to_owned(),
        "/users?q=%3Cscript%3Ealert(1)%3C%2Fscript%3E".to_owned(),
        "/users?q=a%20b".to_owned(),
        "/users?q=%C5%82".to_owned(),
        format!("/users?q={long_q}"),
        "/users?other=1".to_owned(),
    ];
    for path in bad {
        let reply = get_as(&fixture, &admin, &path).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{path}");
        assert!(reply.body.contains("E-WEB-008"), "{path}: {}", reply.body);
        assert!(!reply.body.contains("<script>alert(1)"), "{path}");
    }
}

#[tokio::test]
async fn pages_users_list_is_forbidden_for_an_operator_before_any_lookup() {
    let (fixture, _admin) = world();
    let operator = fixture.session_of(2);
    for path in ["/users", "/users/new", "/users/user1"] {
        let reply = get_as(&fixture, &operator, path).await;
        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-002"), "{path}");
    }
    let kinds: Vec<_> = fixture.sink.events().iter().map(|e| e.kind()).collect();
    assert_eq!(kinds, [SecurityEventKind::Forbidden; 3]);
}

#[tokio::test]
async fn pages_users_list_without_a_session_goes_to_login() {
    let fixture = Fixture::new();
    let reply = get_with(&fixture.router(), "/users", &[]).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(reply.header("location"), "/login");
}

// --- GET /users/new and POST /users -------------------------------------------------------------

#[tokio::test]
async fn pages_users_new_form_has_login_role_and_the_token() {
    let (fixture, admin) = world();
    let reply = get_as(&fixture, &admin, "/users/new").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("name=\"login\""));
    assert!(reply.body.contains("name=\"role\""));
    for role in ["administrator", "process_engineer", "operator"] {
        assert!(reply.body.contains(&format!("value=\"{role}\"")), "{role}");
    }
    let hidden = format!("name=\"csrf_token\" value=\"{}\"", admin.csrf);
    assert!(reply.body.contains(&hidden));
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn pages_users_create_shows_the_one_time_password_once() {
    let (fixture, admin) = world();
    let (logs, _guard) = capture_logs();
    let reply = post_as(
        &fixture,
        &admin,
        "/users",
        "login=newuser&role=operator",
        false,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("cache-control"), "no-store");
    let password = one_time_password(&reply.body);
    assert_one_time_password_shape(&password);
    assert!(reply.body.contains("newuser"));
    assert_clean_html(&reply.body);
    // The password is not in the log, the audit entries, or a later page.
    assert!(!logs.text().contains(&password));
    let audit = format!("{:?}", fixture.store.audit_entries());
    assert!(!audit.contains(&password));
    let detail = get_as(&fixture, &admin, "/users/newuser").await;
    assert_eq!(detail.status, StatusCode::OK, "{}", detail.body);
    assert!(!detail.body.contains(&password));
    let list = get_as(&fixture, &admin, "/users").await;
    assert!(!list.body.contains(&password));
    // The user logs in with it and must change it.
    let first = login(&fixture, "newuser", &password).await;
    assert_eq!(first.status, StatusCode::SEE_OTHER, "{}", first.body);
    assert_eq!(first.header("location"), "/me/password");
}

#[tokio::test]
async fn pages_users_create_with_htmx_gives_the_same_page() {
    let (fixture, admin) = world();
    let reply = post_as(
        &fixture,
        &admin,
        "/users",
        "login=newuser&role=operator",
        true,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_one_time_password_shape(&one_time_password(&reply.body));
}

#[tokio::test]
async fn pages_users_create_with_a_known_login_gives_409_and_no_password() {
    let (fixture, admin) = world();
    let reply = post_as(
        &fixture,
        &admin,
        "/users",
        "login=user2&role=operator",
        false,
    )
    .await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert!(reply.body.contains("E-IDN-001"));
    assert!(!reply.body.contains("one-time-password"));
}

#[tokio::test]
async fn pages_users_create_rejects_bad_fields_with_422_and_never_echoes_them() {
    let (fixture, admin) = world();
    let cases = [
        "login=ab&role=operator",
        "login=%3Cscript%3Ealert(1)%3C%2Fscript%3E&role=operator",
        "login=newuser&role=root",
        "login=newuser",
        "role=operator",
        "login=newuser&role=operator&extra=1",
    ];
    for body in cases {
        let reply = post_as(&fixture, &admin, "/users", body, false).await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert!(reply.body.contains("E-WEB-008"), "{body}: {}", reply.body);
        assert!(!reply.body.contains("<script>alert(1)"), "{body}");
        assert!(!reply.body.contains("one-time-password"), "{body}");
    }
}

#[tokio::test]
async fn pages_users_create_is_forbidden_for_an_operator() {
    let (fixture, _admin) = world();
    let operator = fixture.session_of(2);
    let reply = post_as(
        &fixture,
        &operator,
        "/users",
        "login=newuser&role=operator",
        false,
    )
    .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-002"));
    assert!(!reply.body.contains("one-time-password"));
}

// --- GET /users/{login} -------------------------------------------------------------------------

#[tokio::test]
async fn pages_users_detail_shows_the_user_the_throttle_state_and_five_forms() {
    let (fixture, admin) = world();
    for _attempt in 0..2 {
        let failed = login(&fixture, "user2", WRONG_FIELD).await;
        assert_eq!(failed.status, StatusCode::UNAUTHORIZED);
    }
    let reply = get_as(&fixture, &admin, "/users/user2").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("<title>User</title>"), "{}", reply.body);
    assert!(
        reply.body.contains("<dd id=\"failures\">2</dd>"),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("Operator"));
    assert!(reply.body.contains("Active"));
    for action in ["role", "deactivate", "activate", "reset-password", "unlock"] {
        let form = format!("action=\"/users/user2/{action}\"");
        assert!(reply.body.contains(&form), "{action}");
    }
    assert!(reply.body.contains("hx-sync=\"closest form:drop\""));
    assert_no_user_text_in_hx_attributes(&reply.body, "user2");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn pages_users_detail_of_an_unknown_login_depends_on_the_role() {
    let (fixture, admin) = world();
    let missing = get_as(&fixture, &admin, "/users/ghost").await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND, "{}", missing.body);
    assert!(missing.body.contains("E-WEB-005"));
    let operator = fixture.session_of(2);
    let denied = get_as(&fixture, &operator, "/users/ghost").await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN, "{}", denied.body);
    assert!(denied.body.contains("E-WEB-002"));
}

#[tokio::test]
async fn pages_users_a_login_that_is_not_a_login_gives_404_to_every_caller_without_a_use_case() {
    let (fixture, admin) = world();
    let operator = fixture.session_of(2);
    let bad = [
        "a",
        "ab",
        "%3Cscript%3Ealert(1)%3C%2Fscript%3E",
        "has%20space",
        "a%2Fb%2Fc",
        "%C5%82%C5%82%C5%82",
    ];
    for value in bad {
        for session in [&admin, &operator] {
            let detail = get_as(&fixture, session, &format!("/users/{value}")).await;
            assert_eq!(
                detail.status,
                StatusCode::NOT_FOUND,
                "{value}: {}",
                detail.body
            );
            assert!(detail.body.contains("E-WEB-005"), "{value}");
            assert!(!detail.body.contains("<script>alert(1)"), "{value}");
            for action in ["role", "deactivate", "activate", "reset-password", "unlock"] {
                let path = format!("/users/{value}/{action}");
                let body = if action == "role" {
                    "role=operator"
                } else {
                    ""
                };
                let reply = post_as(&fixture, session, &path, body, true).await;
                assert_eq!(
                    reply.status,
                    StatusCode::NOT_FOUND,
                    "{path}: {}",
                    reply.body
                );
                assert!(reply.body.contains("E-WEB-005"), "{path}");
            }
        }
    }
    assert!(fixture.sink.events().is_empty(), "no use case ran");
}

#[tokio::test]
async fn pages_users_a_login_that_is_not_utf_8_gives_404_e_web_005_on_all_six_routes() {
    let (fixture, admin) = world();
    for value in ["%FF", "ab%FFcd", "%C3"] {
        let detail = get_as(&fixture, &admin, &format!("/users/{value}")).await;
        assert_eq!(detail.status, StatusCode::NOT_FOUND, "{value}");
        assert!(
            detail.body.contains("E-WEB-005"),
            "{value}: {}",
            detail.body
        );
        assert!(detail.body.contains("<html"), "{value}: a full page");
        for action in ["role", "deactivate", "activate", "reset-password", "unlock"] {
            let path = format!("/users/{value}/{action}");
            let body = if action == "role" {
                "role=operator"
            } else {
                ""
            };
            let plain = post_as(&fixture, &admin, &path, body, false).await;
            assert_eq!(
                plain.status,
                StatusCode::NOT_FOUND,
                "{path}: {}",
                plain.body
            );
            assert!(plain.body.contains("E-WEB-005"), "{path}: {}", plain.body);
            let htmx = post_as(&fixture, &admin, &path, body, true).await;
            assert_eq!(htmx.status, StatusCode::NOT_FOUND, "{path}: {}", htmx.body);
            assert!(htmx.body.contains("E-WEB-005"), "{path}: {}", htmx.body);
            assert_eq!(htmx.header("hx-retarget"), "#error-panel", "{path}");
            assert_eq!(htmx.header("hx-reswap"), "innerHTML", "{path}");
        }
    }
    assert!(fixture.sink.events().is_empty(), "no use case ran");
}

// --- The five POST routes -----------------------------------------------------------------------

#[tokio::test]
async fn pages_users_role_change_ends_the_sessions_of_the_target() {
    let (fixture, admin) = world();
    let target = fixture.session_of(2);
    assert_eq!(me_status(&fixture, &target.cookie).await, StatusCode::OK);
    let reply = post_as(
        &fixture,
        &admin,
        "/users/user2/role",
        "role=process_engineer",
        true,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("<html"), "a fragment");
    assert!(reply.body.contains("user2"), "{}", reply.body);
    assert_eq!(
        me_status(&fixture, &target.cookie).await,
        StatusCode::SEE_OTHER
    );
    let detail = get_as(&fixture, &admin, "/users/user2").await;
    assert!(detail.body.contains("Process engineer"), "{}", detail.body);
}

#[tokio::test]
async fn pages_users_role_change_without_htmx_gives_a_full_page() {
    let (fixture, admin) = world();
    let reply = post_as(
        &fixture,
        &admin,
        "/users/user2/role",
        "role=administrator",
        false,
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("<html"));
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn pages_users_role_change_rejects_a_bad_role_and_the_last_administrator() {
    let (fixture, admin) = world();
    let bad = post_as(&fixture, &admin, "/users/user2/role", "role=root", true).await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", bad.body);
    assert!(bad.body.contains("E-WEB-008"));
    let missing = post_as(&fixture, &admin, "/users/user2/role", "", true).await;
    assert_eq!(missing.status, StatusCode::UNPROCESSABLE_ENTITY);
    let last = post_as(&fixture, &admin, "/users/user1/role", "role=operator", true).await;
    assert_eq!(last.status, StatusCode::CONFLICT, "{}", last.body);
    assert!(last.body.contains("E-IDN-002"));
    let unknown = post_as(&fixture, &admin, "/users/ghost/role", "role=operator", true).await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn pages_users_deactivate_ends_the_sessions_and_activate_restores_the_login() {
    let (fixture, admin) = world();
    let target = fixture.session_of(2);
    let off = post_as(&fixture, &admin, "/users/user2/deactivate", "", true).await;
    assert_eq!(off.status, StatusCode::OK, "{}", off.body);
    assert_eq!(
        me_status(&fixture, &target.cookie).await,
        StatusCode::SEE_OTHER
    );
    let refused = login(&fixture, "user2", "correct+horse+battery").await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
    let again = post_as(&fixture, &admin, "/users/user2/deactivate", "", true).await;
    assert_eq!(again.status, StatusCode::CONFLICT, "{}", again.body);
    assert!(again.body.contains("E-IDN-010"));
    let on = post_as(&fixture, &admin, "/users/user2/activate", "", true).await;
    assert_eq!(on.status, StatusCode::OK, "{}", on.body);
    let twice = post_as(&fixture, &admin, "/users/user2/activate", "", true).await;
    assert_eq!(twice.status, StatusCode::CONFLICT, "{}", twice.body);
    assert!(twice.body.contains("E-IDN-009"));
    let back = login(&fixture, "user2", "correct+horse+battery").await;
    assert_eq!(back.status, StatusCode::SEE_OTHER, "{}", back.body);
}

#[tokio::test]
async fn pages_users_deactivate_of_the_last_administrator_gives_409() {
    let (fixture, admin) = world();
    let reply = post_as(&fixture, &admin, "/users/user1/deactivate", "", true).await;
    assert_eq!(reply.status, StatusCode::CONFLICT, "{}", reply.body);
    assert!(reply.body.contains("E-IDN-002"));
}

#[tokio::test]
async fn pages_users_reset_password_shows_a_new_password_once_and_ends_the_sessions() {
    let (fixture, admin) = world();
    let target = fixture.session_of(2);
    let (logs, _guard) = capture_logs();
    let reply = post_as(&fixture, &admin, "/users/user2/reset-password", "", true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(reply.header("cache-control"), "no-store");
    let password = one_time_password(&reply.body);
    assert_one_time_password_shape(&password);
    assert_eq!(
        me_status(&fixture, &target.cookie).await,
        StatusCode::SEE_OTHER
    );
    assert!(!logs.text().contains(&password));
    assert!(!format!("{:?}", fixture.store.audit_entries()).contains(&password));
    let detail = get_as(&fixture, &admin, "/users/user2").await;
    assert!(!detail.body.contains(&password));
    let old = login(&fixture, "user2", "correct+horse+battery").await;
    assert_eq!(
        old.status,
        StatusCode::UNAUTHORIZED,
        "the old password ended"
    );
    let fresh = login(&fixture, "user2", &password).await;
    assert_eq!(fresh.status, StatusCode::SEE_OTHER, "{}", fresh.body);
    assert_eq!(fresh.header("location"), "/me/password");
    assert!(set_cookie_of(&fresh, "__Host-recipe_session").is_some());
}

#[tokio::test]
async fn pages_users_unlock_clears_the_login_delay_with_the_client_address() {
    let (fixture, admin) = world();
    for _attempt in 0..6 {
        let _reply = login(&fixture, "user2", WRONG_FIELD).await;
    }
    let blocked = login(&fixture, "user2", "correct+horse+battery").await;
    assert_eq!(
        blocked.status,
        StatusCode::TOO_MANY_REQUESTS,
        "{}",
        blocked.body
    );
    let reply = post_as(&fixture, &admin, "/users/user2/unlock", "", true).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let kinds: Vec<_> = fixture.sink.events().iter().map(|e| e.kind()).collect();
    assert!(kinds.contains(&SecurityEventKind::Unlock), "{kinds:?}");
    let free = login(&fixture, "user2", "correct+horse+battery").await;
    assert_eq!(free.status, StatusCode::SEE_OTHER, "{}", free.body);
}

#[tokio::test]
async fn pages_users_commands_are_forbidden_for_an_operator() {
    let (fixture, _admin) = world();
    fixture.seed_account(3, Role::Operator, PASSWORD, false);
    let operator = fixture.session_of(3);
    for (action, body) in [
        ("role", "role=operator"),
        ("deactivate", ""),
        ("activate", ""),
        ("reset-password", ""),
        ("unlock", ""),
    ] {
        let path = format!("/users/user2/{action}");
        let reply = post_as(&fixture, &operator, &path, body, true).await;
        assert_eq!(
            reply.status,
            StatusCode::FORBIDDEN,
            "{path}: {}",
            reply.body
        );
        assert!(reply.body.contains("E-WEB-002"), "{path}");
    }
    assert_eq!(
        me_status(&fixture, &fixture.session_of(2).cookie).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn pages_users_a_command_without_the_csrf_token_changes_nothing() {
    let (fixture, admin) = world();
    let request = crate::test_support::request(axum::http::Method::POST, "/users/user2/deactivate")
        .header("cookie", admin.cookie.as_str())
        .header("sec-fetch-site", "same-origin")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(axum::body::Body::empty())
        .unwrap();
    let reply = crate::test_support::call(&fixture.router(), request).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-007"));
    let login = login(&fixture, "user2", "correct+horse+battery").await;
    assert_eq!(login.status, StatusCode::SEE_OTHER, "user2 is still active");
}

#[tokio::test]
async fn pages_users_pages_follow_the_language_cookie() {
    let (fixture, admin) = world();
    let polish = get_with(&fixture.router(), "/users", &[("cookie", &admin.cookie)]).await;
    assert!(
        polish.body.contains("<title>Użytkownicy</title>"),
        "{}",
        polish.body
    );
    assert!(polish.body.contains("<html lang=\"pl\">"));
}
