//! Tests of `GET /security-events` (task 11.4, ADR-0051 (h), (i), specs "Security event page").

use axum::http::StatusCode;
use recipe_domain::audit::SecurityEventKind;

use crate::pages::audit::test_world::{AuditWorld, audit_world};
use crate::pages::test_helpers::{assert_clean_html, assert_error, assert_htmx_error};
use crate::test_support::plant::{HTMX, main_of};
use crate::test_support::{PEER, get_with, post_pre_session};

/// The number of rows in the body of the table.
fn body_rows(main: &str) -> usize {
    main.matches("<tr><td><span class=\"data\">").count()
}

/// Return the markup after the start of the table body. The select of the kinds is not part of it.
fn table_of(main: &str) -> &str {
    main.split_once("<tbody>").map_or("", |(_head, rest)| rest)
}

/// Return the markup of the filter form of the page.
fn filter_form(main: &str) -> &str {
    let (_before, rest) = main
        .split_once("<form method=\"get\" action=\"/security-events\"")
        .unwrap();
    rest.split("</form>").next().unwrap()
}

/// Fail one login of `login` with the User-Agent header `agent`.
async fn fail_login(world: &AuditWorld, login: &str, agent: &str) {
    let fields = format!("login={login}&password=wrong-password-1");
    let reply = post_pre_session(&world.fixture, "/login", &fields, &[("user-agent", agent)]).await;
    assert!(reply.status.as_u16() < 500, "{}", reply.body);
}

/// Read `/security-events?query` as the Administrator. Return the markup of `main`.
async fn page(world: &AuditWorld, query: &str) -> String {
    let path = format!("/security-events?{query}");
    let reply = world.get_en(&world.data.admin, &path).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
    main_of(&reply.body).to_owned()
}

/// Seed one `forbidden` event of the Operator `user3` and a failed login of `ghost`.
///
/// The read model groups the failed logins of one client address and minute into one row.
async fn seed_events(world: &AuditWorld) {
    let denied = world.get_en(&world.operator, "/audit").await;
    assert_eq!(denied.status, StatusCode::FORBIDDEN, "{}", denied.body);
    fail_login(world, "ghost", "<img src=x onerror=alert(1)>").await;
}

#[tokio::test]
async fn security_events_give_403_and_one_forbidden_event_to_an_engineer_and_an_operator() {
    let world = audit_world().await;
    for session in [&world.data.anowak, &world.operator] {
        let before = world.fixture.sink.events().len();
        let reply = world.get_en(session, "/security-events").await;
        assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", "denied");
        let events = world.fixture.sink.events();
        assert_eq!(events.len(), before.checked_add(1).unwrap());
        assert_eq!(
            events.last().map(|event| event.kind()),
            Some(SecurityEventKind::Forbidden)
        );
    }
}

#[tokio::test]
async fn security_events_page_of_an_administrator_has_a_boosted_get_filter_form_and_a_table() {
    let world = audit_world().await;
    seed_events(&world).await;
    let reply = world.get_en(&world.data.admin, "/security-events").await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("<h1>Security events</h1>"), "{main}");
    let form = filter_form(main);
    assert!(!form.contains("csrf_token"), "{form}");
    assert!(!form.contains("name=\"before\""), "{form}");
    let (_before, rest) = main
        .split_once("<form method=\"get\" action=\"/security-events\"")
        .unwrap();
    let tag = rest.split('>').next().unwrap();
    assert!(tag.contains("hx-boost=\"true\""), "{tag}");
    assert!(tag.contains("hx-target=\"#content\""), "{tag}");
    assert!(tag.contains("hx-push-url=\"true\""), "{tag}");
    assert_eq!(tag.matches(" hx-").count(), 3, "{tag}");
    assert_eq!(main.matches("<caption").count(), 1, "{main}");
    assert_eq!(body_rows(main), 2, "{main}");
    assert!(main.contains("No older entries."), "{main}");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn security_events_rows_show_the_login_or_the_unknown_login_and_never_the_hmac() {
    let world = audit_world().await;
    seed_events(&world).await;
    let reply = world.get_en(&world.data.admin, "/security-events").await;
    let main = table_of(main_of(&reply.body));
    for word in [
        "Access refused",
        "Failed login",
        "unknown login",
        "user3",
        "192.0.2.10",
    ] {
        assert!(main.contains(word), "{word}: {main}");
    }
    assert!(!main.to_ascii_lowercase().contains("hmac"), "{main}");
}

#[tokio::test]
async fn security_events_detail_shows_as_formatted_json_in_a_pre_element() {
    let world = audit_world().await;
    seed_events(&world).await;
    let reply = world
        .get_en(&world.data.admin, "/security-events?kind=forbidden")
        .await;
    let main = main_of(&reply.body);
    assert!(
        main.contains(
            "<pre class=\"json\">{\n  &#34;operation&#34;: &#34;ListAuditEntries&#34;\n}</pre>"
        ),
        "{main}"
    );
    assert!(!main.contains("<pre tabindex"), "{main}");
    assert!(!main.contains("<pre class=\"json\" tabindex"), "{main}");
}

#[tokio::test]
async fn security_events_escape_the_user_agent() {
    let world = audit_world().await;
    seed_events(&world).await;
    let reply = world.get_en(&world.data.admin, "/security-events").await;
    let main = main_of(&reply.body);
    assert!(
        main.contains("&#60;img src=x onerror=alert(1)&#62;"),
        "{main}"
    );
    assert!(!main.contains("<img src=x"), "{main}");
}

#[tokio::test]
async fn security_events_filter_by_kind_login_and_client_address() {
    let world = audit_world().await;
    seed_events(&world).await;
    let forbidden = page(&world, "kind=forbidden").await;
    assert_eq!(body_rows(&forbidden), 1, "{forbidden}");
    assert!(forbidden.contains("Access refused"), "{forbidden}");
    let ghost = page(&world, "login=ghost").await;
    assert_eq!(body_rows(&ghost), 1, "{ghost}");
    assert!(ghost.contains("unknown login"), "{ghost}");
    assert!(!table_of(&ghost).contains("Access refused"), "{ghost}");
    let known = page(&world, "login=USER3").await;
    assert_eq!(body_rows(&known), 1, "{known}");
    assert!(!known.contains("unknown login"), "{known}");
    let nobody = page(&world, "login=nobody").await;
    assert_eq!(body_rows(&nobody), 0, "{nobody}");
    assert!(nobody.contains("No event matches the filters."), "{nobody}");
    let same = page(&world, &format!("client_ip={}", PEER.ip())).await;
    assert_eq!(body_rows(&same), 2, "{same}");
    let other = page(&world, "client_ip=10.9.9.9").await;
    assert_eq!(body_rows(&other), 0, "{other}");
}

#[tokio::test]
async fn security_events_date_filters_use_whole_days_in_utc() {
    let world = audit_world().await;
    seed_events(&world).await;
    let all = body_rows(main_of(
        &world
            .get_en(&world.data.admin, "/security-events")
            .await
            .body,
    ));
    // The clock of the tests starts on 1970-01-12.
    for (query, want) in [
        ("from=1970-01-12&to=1970-01-12", all),
        ("to=9999-12-31", all),
        ("to=1970-01-11", 0),
        ("from=1970-01-13", 0),
    ] {
        let path = format!("/security-events?{query}");
        let reply = world.get_en(&world.data.admin, &path).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
        assert_eq!(body_rows(main_of(&reply.body)), want, "{path}");
    }
}

#[tokio::test]
async fn security_events_malformed_values_give_422_with_every_field_label() {
    let world = audit_world().await;
    let path = "/security-events?kind=other&client_ip=999&from=2026-13-01&to=2026-02-30&before=abc&login=%01";
    let cookie = format!("{}; recipe_lang=en", world.data.admin.cookie);
    let reply = get_with(
        &world.fixture.router(),
        path,
        &[("cookie", cookie.as_str()), HTMX],
    )
    .await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    assert_eq!(reply.header("hx-push-url"), "false");
    for label in [
        "Kind",
        "Login",
        "Client address",
        "From",
        "To",
        "Position in the list",
    ] {
        assert!(reply.body.contains(label), "{label}: {}", reply.body);
    }
}

#[tokio::test]
async fn security_events_an_htmx_422_links_each_visible_filter_field_to_its_input() {
    let world = audit_world().await;
    let path = "/security-events?kind=other&client_ip=999&from=bad&to=bad&before=abc&login=%01";
    let cookie = format!("{}; recipe_lang=en", world.data.admin.cookie);
    let router = world.fixture.router();
    let reply = get_with(&router, path, &[("cookie", cookie.as_str()), HTMX]).await;
    assert_htmx_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008");
    for field in ["kind", "login", "client_ip", "from", "to"] {
        assert!(
            reply.body.contains(&format!("href=\"#{field}\"")),
            "{field}: {}",
            reply.body
        );
    }
    assert!(!reply.body.contains("href=\"#before\""), "{}", reply.body);
}

#[tokio::test]
async fn security_events_each_visible_filter_input_has_the_id_of_its_field_name() {
    let world = audit_world().await;
    let main = page(&world, "").await;
    let form = filter_form(&main);
    for field in ["kind", "login", "client_ip", "from", "to"] {
        assert!(form.contains(&format!("id=\"{field}\"")), "{field}: {form}");
        assert!(
            form.contains(&format!("name=\"{field}\"")),
            "{field}: {form}"
        );
    }
}

#[tokio::test]
async fn security_events_other_bad_queries_give_422() {
    let world = audit_world().await;
    for path in [
        "/security-events?x=1",
        "/security-events?from=2026-10-02&to=2026-10-01",
        "/security-events?client_ip=1.2.3",
        "/security-events?before=0",
    ] {
        let reply = world.get_en(&world.data.admin, path).await;
        assert_error(&reply, StatusCode::UNPROCESSABLE_ENTITY, "E-WEB-008", path);
    }
}

#[tokio::test]
async fn security_events_empty_fields_mean_no_filter() {
    let world = audit_world().await;
    seed_events(&world).await;
    let all = body_rows(main_of(
        &world
            .get_en(&world.data.admin, "/security-events")
            .await
            .body,
    ));
    let path = "/security-events?kind=&login=&client_ip=&from=&to=&before=";
    let reply = world.get_en(&world.data.admin, path).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert_eq!(body_rows(main_of(&reply.body)), all);
}

#[tokio::test]
async fn security_events_cursor_pager_ends_at_the_oldest_event() {
    let world = audit_world().await;
    seed_events(&world).await;
    let reply = world
        .get_en(&world.data.admin, "/security-events?before=1")
        .await;
    let main = main_of(&reply.body);
    assert_eq!(body_rows(main), 0, "{main}");
    assert!(
        main.contains("href=\"/security-events\">Newest entries</a>"),
        "{main}"
    );
    assert!(main.contains("No older entries."), "{main}");
}

#[tokio::test]
async fn security_events_words_come_from_the_polish_bundle() {
    let world = audit_world().await;
    seed_events(&world).await;
    let reply = world.get(&world.data.admin, "pl", "/security-events").await;
    let page = main_of(&reply.body);
    assert!(page.contains("<h1>Zdarzenia bezpieczeństwa</h1>"), "{page}");
    let main = table_of(page);
    assert!(main.contains("Odmowa dostępu"), "{main}");
    assert!(main.contains("nieznany login"), "{main}");
    assert!(main.contains("Nieudane logowanie"), "{main}");
    assert!(!page.contains("Access refused"), "{page}");
    assert!(!page.contains("unknown login"), "{page}");
}
