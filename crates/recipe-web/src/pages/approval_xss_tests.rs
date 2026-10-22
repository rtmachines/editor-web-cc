//! The XSS corpus through the text fields of the new pages of P3.4 (task 13.1, design D10).
//!
//! A filter value and the value of the unknown key `q` get every payload of
//! `tests/web/xss-corpus.txt`, also the NUL payload. The `detail` and the `user_agent` of a
//! security event get every payload too, because the test writes the event through the sink. A
//! text that a command stores gets the payloads without a control character. `Text` and the name
//! of a recipe refuse such a character (`reliability.md` section 4.3). The tests
//! `recipe_xss_text_fields_accept_a_valid_text_and_refuse_a_control_character` and
//! `recipe_xss_sent_name_of_a_refused_rename_is_echoed_escaped` of `recipe_xss_tests.rs` cover
//! that refusal.
//!
//! A value that a page shows again is escaped text. A value that a page refuses gives
//! `422 E-WEB-008`. No answer holds the raw payload, no `hx-*` attribute holds it, and each `<svg`
//! of an answer is the fixed tag of its icon. The fields are `change_note`, the filter values,
//! `actor`, `before`, `after`, `detail`, and `user_agent`.
//!
//! No new page has a field `q`, so the `q` test sends the key as an unknown key. Two tests of
//! `xss_tests.rs` are near it. `xss_the_query_pages_are_every_get_page_of_the_table_without_a_query_extractor`
//! probes each GET page with the key `x` and the value `1`. `xss_corpus_in_the_query_of_the_pages_is_escaped`
//! sends the corpus to the pages that answer `200`, `/audit/{id}` among them. The `q` test adds the
//! corpus as the value of an unknown key on the five pages that answer `422`.

use std::net::{IpAddr, Ipv4Addr};

use axum::http::StatusCode;
use recipe_app::identity::new_request_id;
use recipe_app::ports::SecurityEventSink as _;
use recipe_domain::audit::{SecurityEvent, UserAgent};

use super::audit::test_world::{AuditWorld, audit_world};
use super::test_helpers::{askama_escape, encode, part_of, payloads};
use super::xss_kit::{assert_safe, has_control, plain_payloads, post_as_engineer};
use super::xss_tests::with_query;
use crate::test_support::plant::{ENGLISH, HTMX, TARGET_CONTENT, main_of};
use crate::test_support::{Reply, post_fields};

/// The pages about a new version: the approval preview, the diff, and the history. The draft is
/// version 2 of `R-600`, based on version 1. These pages do not show the change note today.
/// The guard test sends a note to them, so a later change that shows the note must escape it.
const NOTE_PAGES: [&str; 3] = [
    "/recipes/R-600/versions/2/approve",
    "/recipes/R-600/versions/2/diff/1",
    "/recipes/R-600/history",
];
/// The pages with a filter or a query, and the fields whose values no payload can satisfy.
const REFUSING_FIELDS: [(&str, &[&str]); 5] = [
    (
        "/audit",
        &[
            "aggregate",
            "entity_type",
            "entity_id",
            "actor",
            "action",
            "from",
            "to",
            "before",
        ],
    ),
    (
        "/security-events",
        &["kind", "client_ip", "from", "to", "before"],
    ),
    ("/recipes/R-500/history", &["before"]),
    ("/recipes/R-500/versions/2/approve", &["kind", "page", "rv"]),
    ("/recipes/R-500/versions/2/diff/1", &["kind", "page"]),
];
/// The label of a recipe in a row of the audit list and of the history (ADR-0051 (e)).
const RECIPE_LABEL: &str = "R-500";
/// The client address of a seeded security event.
const EVENT_ADDRESS: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 77));
/// The most characters that the filter `login` of the security events accepts.
const LOGIN_MAX_CHARS: usize = 64;

/// Return `text` as the inside of a JSON string, as the audit pages show a stored text.
///
/// The payloads of `plain_payloads` hold no control character, so only the quote and the
/// backslash need an escape.
fn json_escaped(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Return the text of a JSON member `"key": "value"` as the page shows it, in HTML.
fn shown_member(key: &str, value: &str) -> String {
    format!(
        "&#34;{key}&#34;: &#34;{}&#34;",
        askama_escape(&json_escaped(value))
    )
}

/// Assert a refusal: `422`, the code `E-WEB-008`, and a safe body.
fn assert_refused(reply: &Reply, payload: &str, what: &str) {
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{what}: {}",
        reply.body
    );
    assert!(reply.body.contains("E-WEB-008"), "{what}: {}", reply.body);
    assert_safe(reply, payload, what);
}

/// Send `GET path` as the Administrator, in English.
async fn get_admin(world: &AuditWorld, path: &str) -> Reply {
    world.get_en(&world.data.admin, path).await
}

/// Send `GET path` with the query `key=payload` as the Administrator.
async fn get_with_value(world: &AuditWorld, path: &str, key: &str, payload: &str) -> Reply {
    let full = with_query(path, key, &encode(payload));
    get_admin(world, &full).await
}

/// Return the ids of the audit entries of `action`, newest first, read as the Administrator.
async fn entry_ids(world: &AuditWorld, action: &str) -> Vec<u64> {
    let list = get_admin(world, &format!("/audit?action={action}")).await;
    main_of(&list.body)
        .split("href=\"/audit/")
        .skip(1)
        .filter_map(|rest| rest.split('"').next()?.parse().ok())
        .collect()
}

/// Return the content of the block `<pre class="json">` number `index` (0 is the state before).
fn json_block(main: &str, index: usize) -> &str {
    let part = main
        .split("<pre class=\"json\">")
        .nth(index.checked_add(1).unwrap());
    part.and_then(|rest| rest.split("</pre>").next()).unwrap()
}

// --- change_note -----------------------------------------------------------------------------

/// Create version 2 of `R-600` as `anowak`, based on version 1, with the change note `payload`.
async fn create_noted_version(world: &AuditWorld, payload: &str) {
    let body = format!("based_on=1&change_note={}", encode(payload));
    let reply = post_as_engineer(world, "/recipes/R-600/versions", &body).await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{payload:?}");
}

/// Guard: a page about a new version never holds a raw change note.
///
/// No page of `NOTE_PAGES` shows the note today. The test passes without an echo. It fails when a
/// later change shows the note unescaped.
#[tokio::test]
async fn approval_xss_guard_the_pages_of_the_new_version_hold_no_raw_change_note() {
    for payload in plain_payloads() {
        let world = audit_world().await;
        create_noted_version(&world, &payload).await;
        for path in NOTE_PAGES {
            let reply = get_admin(&world, path).await;
            let what = format!("{path} {payload:?}");
            assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
            assert_safe(&reply, &payload, &what);
        }
    }
}

#[tokio::test]
async fn approval_xss_change_note_is_escaped_in_the_state_of_the_audit_entry() {
    for payload in plain_payloads() {
        let world = audit_world().await;
        create_noted_version(&world, &payload).await;
        let ids = entry_ids(&world, "VersionCreated").await;
        let newest = ids.first().unwrap();
        let reply = get_admin(&world, &format!("/audit/{newest}")).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_safe(&reply, &payload, "VersionCreated entry");
        let main = main_of(&reply.body);
        assert!(
            main.contains(&shown_member("change_note", &payload)),
            "{payload:?}: {main}"
        );
    }
}

#[tokio::test]
async fn approval_xss_change_note_is_escaped_in_the_answer_of_the_approval() {
    for payload in plain_payloads() {
        let world = audit_world().await;
        create_noted_version(&world, &payload).await;
        let rv = world.row_version_of("R-600", 2).await;
        let headers = [ENGLISH, HTMX, TARGET_CONTENT];
        let path = "/recipes/R-600/versions/2/approve";
        let body = format!("row_version={rv}");
        let router = world.fixture.router();
        let reply = post_fields(&router, path, Some(&world.data.admin), &headers, &body).await;
        assert_eq!(reply.status, StatusCode::OK, "{payload:?}: {}", reply.body);
        assert_safe(&reply, &payload, "POST approve");
        assert!(
            reply.body.contains(&askama_escape(&payload)),
            "{payload:?}: {}",
            reply.body
        );
    }
}

// --- filter values, actor, and q ------------------------------------------------------------

#[tokio::test]
async fn approval_xss_filter_values_that_no_payload_can_satisfy_give_e_web_008() {
    let world = audit_world().await;
    for (path, fields) in REFUSING_FIELDS {
        for field in fields {
            for payload in payloads() {
                let reply = get_with_value(&world, path, field, &payload).await;
                assert_refused(&reply, &payload, &format!("{path}?{field}={payload:?}"));
            }
        }
    }
}

#[tokio::test]
async fn approval_xss_the_key_q_is_refused_on_each_new_page() {
    let world = audit_world().await;
    for (path, _fields) in REFUSING_FIELDS {
        for payload in payloads() {
            let reply = get_with_value(&world, path, "q", &payload).await;
            assert_refused(&reply, &payload, &format!("{path}?q={payload:?}"));
        }
    }
}

#[tokio::test]
async fn approval_xss_login_filter_of_the_security_events_is_echoed_escaped_or_refused() {
    let world = audit_world().await;
    let (mut echoed, mut refused) = (0_usize, 0_usize);
    for payload in payloads() {
        let reply = get_with_value(&world, "/security-events", "login", &payload).await;
        let what = format!("login {payload:?}");
        if has_control(&payload) || payload.chars().count() > LOGIN_MAX_CHARS {
            refused = refused.checked_add(1).unwrap();
            assert_refused(&reply, &payload, &what);
            continue;
        }
        echoed = echoed.checked_add(1).unwrap();
        assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
        assert_safe(&reply, &payload, &what);
        let value = format!("value=\"{}\"", askama_escape(&payload));
        assert!(reply.body.contains(&value), "{what}: {}", reply.body);
    }
    assert!(
        echoed > 0 && refused > 0,
        "{echoed} echoed, {refused} refused"
    );
}

// --- before and after -----------------------------------------------------------------------

#[tokio::test]
async fn approval_xss_before_and_after_of_an_audit_entry_show_escaped_json() {
    for payload in plain_payloads() {
        let world = audit_world().await;
        for name in [payload.as_str(), "Other name"] {
            let reply = post_as_engineer(
                &world,
                "/recipes/R-500/rename",
                &format!("name={}", encode(name)),
            )
            .await;
            assert!(reply.status.as_u16() < 400, "{name:?}");
        }
        let ids = entry_ids(&world, "RecipeRenamed").await;
        assert_eq!(ids.len(), 2, "{ids:?}");
        let expected = shown_member("name", &payload);
        // The newest entry renamed the recipe away from the payload. The older one renamed to it.
        for (id, state) in ids.iter().zip([0_usize, 1]) {
            let reply = get_admin(&world, &format!("/audit/{id}")).await;
            let what = format!("entry {id} {payload:?}");
            assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
            assert_safe(&reply, &payload, &what);
            let block = json_block(main_of(&reply.body), state);
            assert!(block.contains(&expected), "{what}: {block}");
        }
    }
}

/// ADR-0051 (e): a row of the audit list and of the history labels a recipe by its reference.
///
/// The recipe name is a stored text, but no row shows it. The test renames the recipe to each
/// payload. Then the table body of each page must hold the label `<span class="data">R-500</span>`.
/// The whole `main` must hold neither the raw payload nor its escaped form. A later change that puts
/// the name into a row fails the test, also with an escaped name. Such a change needs a new ADR and
/// a change of this test.
#[tokio::test]
async fn approval_xss_the_rows_of_the_audit_list_and_the_history_label_a_recipe_by_its_reference() {
    for payload in plain_payloads() {
        let world = audit_world().await;
        let body = format!("name={}", encode(&payload));
        post_as_engineer(&world, "/recipes/R-500/rename", &body).await;
        for path in [
            "/audit",
            "/audit?action=RecipeRenamed",
            "/recipes/R-500/history",
        ] {
            let reply = get_admin(&world, path).await;
            let what = format!("{path} {payload:?}");
            assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
            assert_safe(&reply, &payload, &what);
            let main = main_of(&reply.body);
            let rows = part_of(main, "<tbody>", "</tbody>");
            let label = format!("<span class=\"data\">{RECIPE_LABEL}</span>");
            assert!(rows.contains(&label), "{what}: {rows}");
            assert!(!main.contains(&payload), "{what}: raw name in {main}");
            assert!(
                !main.contains(&askama_escape(&payload)),
                "{what}: escaped name in {main}"
            );
        }
    }
}

// --- detail and user_agent ------------------------------------------------------------------

/// Record a `Forbidden` event whose operation and user agent are `payload`.
///
/// A real request cannot make this event. The operation names come from code. A header limits a
/// user agent to visible characters. So the test writes the event through the sink. The page must
/// escape what the store holds.
async fn record_hostile_event(world: &AuditWorld, payload: &str) {
    let event = SecurityEvent::forbidden(
        world.data.admin_id,
        payload,
        new_request_id().unwrap(),
        Some(EVENT_ADDRESS),
        Some(UserAgent::from(payload)),
    );
    world.fixture.sink.record(event).await.unwrap();
}

#[tokio::test]
async fn approval_xss_detail_and_user_agent_of_a_security_event_are_escaped() {
    for payload in payloads() {
        let world = audit_world().await;
        record_hostile_event(&world, &payload).await;
        let reply = get_admin(&world, "/security-events?kind=forbidden").await;
        let what = format!("event {payload:?}");
        assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
        assert_safe(&reply, &payload, &what);
        if has_control(&payload) {
            continue;
        }
        let main = main_of(&reply.body);
        let agent = format!("<td class=\"data\">{}</td>", askama_escape(&payload));
        assert!(main.contains(&agent), "{what}: {main}");
        assert!(
            main.contains(&shown_member("operation", &payload)),
            "{what}: {main}"
        );
    }
}
