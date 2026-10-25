//! XSS sweeps next to the banner (task 6.4, `api.md` section 9, corpus of task 6.10).
//!
//! The recipe list reflects the search text `q` in the `value` of its search input. That input is
//! the one sink that a GET request reaches on a page with the banner. The login page reflects no
//! typed field: `login.html` renders the login input blank, and the password never comes back.
//! So the login sweep proves that nothing of the payload comes back, and the list sweeps prove the
//! escaping of the one reflected value.

use axum::http::StatusCode;

use super::{EN_BANNER, LANGS, PL_BANNER, assert_banner, content, failed_login, page, world};
use crate::pages::test_helpers::{
    askama_escape, assert_clean_html, assert_no_user_text_in_hx_attributes, encode, payloads,
};

/// Return whether the search text accepts `payload`. `Text::parse` refuses a control character.
fn is_valid_search(payload: &str) -> bool {
    !payload.chars().any(char::is_control)
}

/// Return the path of the recipe list with the search text `payload`.
fn search_path(payload: &str) -> String {
    format!("/recipes?q={}", encode(payload))
}

/// Assert that `body` reflects `payload` only as escaped text in the `value` of the search input.
fn assert_reflected_escaped(body: &str, payload: &str, what: &str) {
    let value = format!(" value=\"{}\"", askama_escape(payload));
    assert_eq!(body.matches(&value).count(), 1, "{what}: {body}");
    if askama_escape(payload) != payload {
        assert!(!body.contains(payload), "{what}: raw payload in {body}");
    }
    assert_no_user_text_in_hx_attributes(body, payload);
    assert_clean_html(body);
}

#[tokio::test]
async fn banner_xss_sweep_a_hostile_login_is_not_reflected_next_to_the_banner() {
    let (fixture, _session) = world(true);

    for payload in payloads() {
        let reply = failed_login(&fixture, "en", &payload).await;
        let what = format!("payload {payload:?}");

        assert!(reply.status.is_client_error(), "{what}: {}", reply.status);
        assert_banner(&reply.body, EN_BANNER, &what);
        assert_no_user_text_in_hx_attributes(&reply.body, &payload);
        assert!(!reply.body.contains(&payload), "{what}: {}", reply.body);
        let escaped = askama_escape(&payload);
        assert!(!reply.body.contains(&escaped), "{what}: {}", reply.body);
    }
}

#[tokio::test]
async fn banner_xss_sweep_a_hostile_search_is_escaped_in_the_page_and_the_content_answer() {
    let (fixture, session) = world(true);
    let valid: Vec<String> = payloads()
        .into_iter()
        .filter(|payload| is_valid_search(payload))
        .collect();
    assert!(valid.len() > 10, "the corpus is too short: {}", valid.len());

    for payload in &valid {
        let path = search_path(payload);
        for (code, message) in LANGS {
            let full = page(&fixture, Some(&session), code, &path).await;
            let part = content(&fixture, &session, code, &path).await;

            for (reply, shape) in [(full, "page"), (part, "content")] {
                let what = format!("{payload:?} {code} {shape}");
                assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
                assert_banner(&reply.body, message, &what);
                assert_reflected_escaped(&reply.body, payload, &what);
            }
        }
    }
}

#[tokio::test]
async fn banner_xss_sweep_a_search_with_a_control_character_is_refused_as_a_field_error() {
    let (fixture, session) = world(true);
    let invalid: Vec<String> = payloads()
        .into_iter()
        .filter(|payload| !is_valid_search(payload))
        .collect();
    assert!(!invalid.is_empty(), "the corpus has no invalid search text");

    for payload in &invalid {
        let reply = page(&fixture, Some(&session), "pl", &search_path(payload)).await;
        let what = format!("{payload:?}");

        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{what}");
        assert!(reply.body.contains("E-WEB-008"), "{what}: {}", reply.body);
        assert_banner(&reply.body, PL_BANNER, &what);
        assert!(!reply.body.contains(payload), "{what}: {}", reply.body);
        assert_no_user_text_in_hx_attributes(&reply.body, payload);
    }
}
