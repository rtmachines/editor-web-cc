//! Tests of the web-ui requirements for the technical account `ExportClient` (task 6.9, ADR-0109).

use axum::http::StatusCode;
use recipe_domain::identity::Role;

use crate::test_support::{Fixture, Reply, TestSession, get_with, post_fields};

const HTMX: (&str, &str) = ("hx-request", "true");
const ENGLISH: (&str, &str) = ("cookie", "recipe_lang=en");

const PERSON_NOTICE_PL: &str = "Użytkownik musi zmienić je przy pierwszym logowaniu.";
const PERSON_NOTICE_EN: &str = "The user must change it at the first login.";
const TECHNICAL_NOTICE_PL: &str = "To hasło jest stałym poświadczeniem klienta.";
const TECHNICAL_NOTICE_EN: &str = "This password is the permanent credential of the client.";

/// A fixture with the Administrator `user1`, the Operator `user2`, and the `ExportClient` `user3`.
fn world() -> (Fixture, TestSession) {
    let fixture = Fixture::new();
    let admin = fixture.seed_session(1, Role::Administrator, false);
    fixture.seed_account(2, Role::Operator, "correct horse battery", true);
    fixture.seed_account(3, Role::ExportClient, "correct horse battery", false);
    (fixture, admin)
}

async fn get_en(fixture: &Fixture, session: &TestSession, path: &str) -> Reply {
    get_with(
        &fixture.router(),
        path,
        &[("cookie", &session.cookie), ENGLISH],
    )
    .await
}

/// Post a form as the Administrator. `headers` adds to the session headers of the helper.
async fn post_as(
    fixture: &Fixture,
    session: &TestSession,
    path: &str,
    body: &str,
    headers: &[(&str, &str)],
) -> Reply {
    post_fields(&fixture.router(), path, Some(session), headers, body).await
}

/// Return the values of the `<option>` elements of the `<select>` with the id `select_id`.
fn option_values(body: &str, select_id: &str) -> Vec<String> {
    let marker = format!("<select id=\"{select_id}\"");
    let (_before, rest) = body.split_once(&marker).unwrap();
    let (select, _after) = rest.split_once("</select>").unwrap();
    select
        .split("<option value=\"")
        .skip(1)
        .map(|part| part.split('"').next().unwrap().to_owned())
        .collect()
}

// --- Role ExportClient in the user form ---------------------------------------------------------

#[tokio::test]
async fn pages_users_new_form_offers_export_client_after_the_person_roles() {
    let (fixture, admin) = world();
    let reply = get_en(&fixture, &admin, "/users/new").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(
        option_values(&reply.body, "role"),
        [
            "administrator",
            "process_engineer",
            "operator",
            "export_client"
        ]
    );
    for label in [
        ">Administrator</option>",
        ">Process engineer</option>",
        ">Operator</option>",
        ">Export client</option>",
    ] {
        assert!(reply.body.contains(label), "{label}: {}", reply.body);
    }
}

#[tokio::test]
async fn pages_users_new_form_labels_export_client_in_polish() {
    let (fixture, admin) = world();
    let reply = get_with(
        &fixture.router(),
        "/users/new",
        &[("cookie", &admin.cookie)],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains(">Klient eksportu</option>"),
        "{}",
        reply.body
    );
}

// --- Password notice of a technical account -----------------------------------------------------

#[tokio::test]
async fn pages_users_create_of_a_technical_account_shows_the_technical_notice_only() {
    let (fixture, admin) = world();
    let reply = post_as(
        &fixture,
        &admin,
        "/users",
        "login=plc-m12&role=export_client",
        &[],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("id=\"one-time-password\""));
    assert!(reply.body.contains(TECHNICAL_NOTICE_PL), "{}", reply.body);
    assert!(!reply.body.contains(PERSON_NOTICE_PL), "{}", reply.body);
    assert!(!reply.body.contains("pierwszym logowaniu"));
}

#[tokio::test]
async fn pages_users_create_of_a_person_keeps_the_first_login_notice() {
    let (fixture, admin) = world();
    let reply = post_as(
        &fixture,
        &admin,
        "/users",
        "login=newuser&role=operator",
        &[ENGLISH],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains(PERSON_NOTICE_EN), "{}", reply.body);
    assert!(!reply.body.contains(TECHNICAL_NOTICE_EN), "{}", reply.body);
}

#[tokio::test]
async fn pages_users_reset_of_a_technical_account_shows_the_technical_notice_in_the_fragment() {
    let (fixture, admin) = world();
    let reply = post_as(
        &fixture,
        &admin,
        "/users/user3/reset-password",
        "",
        &[HTMX, ENGLISH],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("id=\"one-time-password\""));
    assert!(reply.body.contains(TECHNICAL_NOTICE_EN), "{}", reply.body);
    assert!(!reply.body.contains(PERSON_NOTICE_EN), "{}", reply.body);
}

#[tokio::test]
async fn pages_users_reset_of_a_person_keeps_the_first_login_notice() {
    let (fixture, admin) = world();
    let reply = post_as(
        &fixture,
        &admin,
        "/users/user2/reset-password",
        "",
        &[HTMX, ENGLISH],
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains(PERSON_NOTICE_EN), "{}", reply.body);
    assert!(!reply.body.contains(TECHNICAL_NOTICE_EN), "{}", reply.body);
}

#[tokio::test]
async fn pages_users_reset_of_a_technical_account_refreshes_facts_without_a_forced_change() {
    let (fixture, admin) = world();
    // The flag of a technical account is false by rule. The answer takes it from the role.
    fixture.seed_account(4, Role::ExportClient, "correct horse battery", true);
    for login in ["user3", "user4"] {
        let path = format!("/users/{login}/reset-password");
        let reply = post_as(&fixture, &admin, &path, "", &[HTMX, ENGLISH]).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(reply.body.contains("id=\"one-time-password\""));
        let (_before, after) = reply.body.split_once("id=\"user-facts\"").unwrap();
        let facts = after.split("</dl>").next().unwrap();
        assert!(facts.contains(login), "{login}: {facts}");
        assert!(!facts.contains("Password change"), "{login}: {facts}");
        assert!(!reply.body.contains("id=\"user-actions\""), "{login}");
    }
}

// --- Role form of a technical account -----------------------------------------------------------

#[tokio::test]
async fn pages_users_detail_of_a_person_offers_the_three_person_roles_only() {
    let (fixture, admin) = world();
    let reply = get_en(&fixture, &admin, "/users/user2").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(reply.body.contains("action=\"/users/user2/role\""));
    assert_eq!(
        option_values(&reply.body, "role"),
        ["administrator", "process_engineer", "operator"]
    );
    assert!(!reply.body.contains("export_client"), "{}", reply.body);
}

#[tokio::test]
async fn pages_users_detail_of_a_technical_account_shows_no_role_form() {
    let (fixture, admin) = world();
    let reply = get_en(&fixture, &admin, "/users/user3").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(!reply.body.contains("/users/user3/role"), "{}", reply.body);
    assert!(!reply.body.contains("<select id=\"role\""));
    assert!(!reply.body.contains("Set role"));
    // The other commands stay: a technical account has a rotating password.
    assert!(reply.body.contains("/users/user3/reset-password"));
}

#[tokio::test]
async fn pages_users_one_time_password_label_ends_with_a_colon_for_a_technical_account() {
    let (fixture, admin) = world();
    let created = post_as(
        &fixture,
        &admin,
        "/users",
        "login=plc-m12&role=export_client",
        &[],
    )
    .await;
    assert_eq!(created.status, StatusCode::OK, "{}", created.body);
    assert!(
        created
            .body
            .contains("<p>Hasło: <code class=\"data\" id=\"one-time-password\">"),
        "{}",
        created.body
    );
    let reset = post_as(
        &fixture,
        &admin,
        "/users/user3/reset-password",
        "",
        &[HTMX, ENGLISH],
    )
    .await;
    assert_eq!(reset.status, StatusCode::OK, "{}", reset.body);
    assert!(
        reset
            .body
            .contains("<p>Password: <code class=\"data\" id=\"one-time-password\">"),
        "{}",
        reset.body
    );
}
