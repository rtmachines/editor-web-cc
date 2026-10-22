//! Tests of `GET /audit/{id}` (task 11.4, ADR-0051 (c), (f), specs "Audit entry page").

use axum::http::StatusCode;
use recipe_domain::audit::SecurityEventKind;

use super::test_world::{AuditWorld, audit_world};
use crate::pages::recipes::test_world::recipe_world;
use crate::pages::test_helpers::{assert_clean_html, assert_error};
use crate::test_support::plant::{HTMX, main_of};
use crate::test_support::post_fields;
use crate::test_support::recipes::HOLD_KEY;

/// The name of the rename. Its markup must show escaped inside the state.
const MARKUP_NAME: &str = "name=%3Cscript%3Ealert(1)%3C%2Fscript%3E";
/// The form of a new machine. The main unit holds markup.
const MACHINE_FORM: &str = "code=M95&name=Line&plc_platform=S7_300&main_unit_name=Infeed";

/// Return the number in the first link `href="/audit/N"` of `html`.
fn first_entry_id(html: &str) -> u64 {
    html.split("href=\"/audit/")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .and_then(|text| text.parse().ok())
        .unwrap()
}

/// Send the form `body` to `path` as the engineer `anowak`. The answer is no client error.
async fn post_as_engineer(world: &AuditWorld, path: &str, body: &str) {
    let reply = post_fields(
        &world.fixture.router(),
        path,
        Some(&world.data.anowak),
        &[],
        body,
    )
    .await;
    assert!(reply.status.as_u16() < 400, "{path}: {}", reply.body);
}

/// Return the id of the newest entry of `action`, read as the Administrator.
async fn newest_entry_of(world: &AuditWorld, action: &str) -> u64 {
    let list = world
        .get_en(&world.data.admin, &format!("/audit?action={action}"))
        .await;
    first_entry_id(main_of(&list.body))
}

/// Rename `R-500` to the markup name and return the id of the entry.
async fn renamed_entry(world: &AuditWorld) -> u64 {
    post_as_engineer(world, "/recipes/R-500/rename", MARKUP_NAME).await;
    newest_entry_of(world, "RecipeRenamed").await
}

#[tokio::test]
async fn audit_entry_shows_the_fields_the_trace_and_the_escaped_states_to_an_administrator() {
    let world = audit_world().await;
    let id = renamed_entry(&world).await;
    let reply = world
        .get_en(&world.data.admin, &format!("/audit/{id}"))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(
        main.contains(&format!("<h1>Audit entry {id}</h1>")),
        "{main}"
    );
    for word in [
        "Recipe renamed",
        "anowak",
        "R-500",
        "Request id",
        "Position in the chain",
        "Client address",
    ] {
        assert!(main.contains(word), "{word}: {main}");
    }
    assert_eq!(main.matches("<pre class=\"json\">").count(), 2, "{main}");
    assert!(main.contains("&#60;script&#62;"), "{main}");
    assert!(
        main.contains("&#34;name&#34;: &#34;&#60;script&#62;alert(1)&#60;/script&#62;&#34;"),
        "{main}"
    );
    assert!(!main.contains("<script>"), "{main}");
    assert!(!main.contains("tabindex"), "{main}");
    assert_clean_html(&reply.body);
}

#[tokio::test]
async fn audit_entry_puts_an_h2_before_each_state_and_shows_no_hash() {
    let world = audit_world().await;
    let id = renamed_entry(&world).await;
    let reply = world
        .get_en(&world.data.admin, &format!("/audit/{id}"))
        .await;
    let main = main_of(&reply.body);
    assert_eq!(main.matches("<h2").count(), 2, "{main}");
    for title in ["State before", "State after"] {
        let (_head, rest) = main.split_once(title).unwrap();
        let (_heading, after) = rest.split_once("</h2>").unwrap();
        assert!(after.trim_start().starts_with("<pre"), "{title}: {main}");
    }
    for secret in ["entry_hash", "prev_hash", "password_hash", "login_hmac"] {
        assert!(!main.contains(secret), "{secret}: {main}");
    }
}

#[tokio::test]
async fn audit_entry_of_an_engineer_has_no_chain_position_client_address_or_session() {
    let world = audit_world().await;
    let id = renamed_entry(&world).await;
    let reply = world
        .get_en(&world.data.anowak, &format!("/audit/{id}"))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("Recipe renamed"), "{main}");
    assert!(main.contains("Request id"), "{main}");
    for word in [
        "Position in the chain",
        "Client address",
        "Session reference",
    ] {
        assert!(!main.contains(word), "{word}: {main}");
    }
}

#[tokio::test]
async fn audit_entry_without_a_before_state_says_so() {
    let world = audit_world().await;
    let id = newest_entry_of(&world, "RecipeCreated").await;
    let reply = world
        .get_en(&world.data.admin, &format!("/audit/{id}"))
        .await;
    let main = main_of(&reply.body);
    assert_eq!(main.matches("<pre class=\"json\">").count(), 1, "{main}");
    assert_eq!(main.matches("No state.").count(), 1, "{main}");
}

#[tokio::test]
async fn audit_entry_links_to_the_object_and_to_the_whole_aggregate() {
    let world = audit_world().await;
    let recipe = world.data.r500.as_uuid();
    let list = world
        .get_en(
            &world.data.admin,
            &format!("/audit?aggregate={recipe}&action=VersionApproved"),
        )
        .await;
    let id = first_entry_id(main_of(&list.body));
    let reply = world
        .get_en(&world.data.anowak, &format!("/audit/{id}"))
        .await;
    let main = main_of(&reply.body);
    let version = world.data.r500_v1.as_uuid();
    assert!(
        main.contains(&format!(
            "href=\"/audit?entity_type=recipe_version&#38;entity_id={version}\">Entries of this object</a>"
        )),
        "{main}"
    );
    assert!(
        main.contains(&format!(
            "href=\"/audit?aggregate={recipe}\">Entries of the whole aggregate</a>"
        )),
        "{main}"
    );
}

#[tokio::test]
async fn audit_entry_outside_the_recipe_scope_looks_like_an_absent_entry_to_an_engineer() {
    let world = audit_world().await;
    post_as_engineer(&world, "/machines", MACHINE_FORM).await;
    let outside = newest_entry_of(&world, "MachineCreated").await;
    let admin = world
        .get_en(&world.data.admin, &format!("/audit/{outside}"))
        .await;
    assert_eq!(admin.status, StatusCode::OK, "{}", admin.body);
    let hidden = world
        .get_en(&world.data.anowak, &format!("/audit/{outside}"))
        .await;
    assert_error(&hidden, StatusCode::NOT_FOUND, "E-WEB-005", "outside");
    let absent_id = 999_999;
    let absent = world
        .get_en(&world.data.anowak, &format!("/audit/{absent_id}"))
        .await;
    assert_error(&absent, StatusCode::NOT_FOUND, "E-WEB-005", "absent");
    // The two answers differ only in the id and the request id.
    for (reply, id) in [(&hidden, outside), (&absent, absent_id)] {
        assert!(
            reply.body.contains(&format!("Audit entry {id}")),
            "{}",
            reply.body
        );
    }
}

#[tokio::test]
async fn audit_entry_malformed_ids_give_404_without_the_typed_text() {
    let world = audit_world().await;
    // The typed text has a letter that no hexadecimal request id holds, so the check is stable.
    for path in [
        "/audit/zqx",
        "/audit/0",
        "/audit/-1",
        "/audit/01",
        "/audit/1.5",
        "/audit/99999999999999999999999",
    ] {
        let reply = world.get_en(&world.data.admin, path).await;
        assert_error(&reply, StatusCode::NOT_FOUND, "E-WEB-005", path);
        assert!(!reply.body.contains("zqx"), "{path}: {}", reply.body);
    }
}

#[tokio::test]
async fn audit_entry_gives_403_and_one_forbidden_event_to_an_operator() {
    let world = audit_world().await;
    let before = world.fixture.sink.events().len();
    let reply = world.get_en(&world.operator, "/audit/1").await;
    assert_error(&reply, StatusCode::FORBIDDEN, "E-WEB-002", "operator");
    let events = world.fixture.sink.events();
    assert_eq!(events.len(), before.checked_add(1).unwrap());
    assert_eq!(
        events.last().map(|event| event.kind()),
        Some(SecurityEventKind::Forbidden)
    );
}

#[tokio::test]
async fn audit_entry_shows_a_time_leaf_of_a_state_as_its_stored_integer_count() {
    let world = recipe_world();
    let path = format!("/recipes/R-100/versions/3/parameters/{HOLD_KEY}/value");
    let body = "row_version=1&value=T%235s";
    let saved = world.post_with(&world.engineer, &path, body, &[HTMX]).await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    let list = world
        .get_en(&world.admin, "/audit?action=ParameterValueSet")
        .await;
    let id = first_entry_id(main_of(&list.body));
    let reply = world.get_en(&world.admin, &format!("/audit/{id}")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let main = main_of(&reply.body);
    assert!(main.contains("&#34;&#34;: 90000\n"), "{main}");
    assert!(main.contains("&#34;&#34;: 5000\n"), "{main}");
    assert!(!main.contains("T#"), "{main}");
}

#[tokio::test]
async fn audit_entry_words_come_from_the_polish_bundle() {
    let world = audit_world().await;
    let id = newest_entry_of(&world, "VersionApproved").await;
    let reply = world
        .get(&world.data.admin, "pl", &format!("/audit/{id}"))
        .await;
    let main = main_of(&reply.body);
    assert!(main.contains("Zatwierdzono wersję"), "{main}");
    assert!(!main.contains("Version approved"), "{main}");
}
