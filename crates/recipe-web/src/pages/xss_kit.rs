//! Helpers that the XSS test files share.
//!
//! The files `plant_xss_tests.rs`, `recipe_xss_tests.rs`, `approval_xss_tests.rs`, and
//! `approval_text_xss_tests.rs` call this module. A change of the safety check or of the payload
//! filter then reaches every test of these four files. The file `xss_tests.rs` keeps its own check.

use super::audit::test_world::AuditWorld;
use super::test_helpers::{
    askama_escape, assert_clean_html, assert_no_user_text_in_hx_attributes, payloads,
};
use crate::components_tests::assert_fixed_svg_tags;
use crate::test_support::{Reply, post_fields};

/// The most `<script>` tags that an answer may hold. They are the three files of `base.html`.
const MAX_SCRIPT_TAGS: usize = 3;
/// The markup that a payload brings into a page. A page holds none of it.
///
/// An `<svg` tag is not in the list. A page may hold the fixed icon tags of `design-system.md`
/// section 8, so `assert_fixed_svg_tags` checks each `<svg` instead. The text `onerror=` is not in
/// the list either. Escaped text may hold it, and `assert_clean_html` finds a real handler.
const FORBIDDEN_MARKUP: [&str; 1] = ["<img"];

/// Return true when a text field refuses the payload, because it holds a control character.
pub(super) fn has_control(payload: &str) -> bool {
    payload.chars().any(char::is_control)
}

/// Return true when escaping changes the payload, so the raw payload must not appear in an answer.
pub(super) fn escapes(payload: &str) -> bool {
    askama_escape(payload) != payload
}

/// Return the payloads that a text field accepts: the ones with no control character.
pub(super) fn plain_payloads() -> Vec<String> {
    payloads()
        .into_iter()
        .filter(|payload| !has_control(payload))
        .collect()
}

/// Assert that an answer holds no markup of `payload`.
///
/// The page keeps its own scripts with `src`. No inline script, no image, and no event handler may
/// appear. No `hx-*` attribute may hold the payload, and each `<svg` is a fixed icon tag. The raw
/// payload may appear only when escaping leaves it unchanged.
pub(super) fn assert_safe(reply: &Reply, payload: &str, what: &str) {
    let body = &reply.body;
    assert_clean_html(body);
    for marker in FORBIDDEN_MARKUP {
        assert!(!body.contains(marker), "{what}: {marker} in the answer");
    }
    assert_fixed_svg_tags(body);
    assert_no_user_text_in_hx_attributes(body, payload);
    let scripts = body.matches("<script").count();
    assert!(scripts <= MAX_SCRIPT_TAGS, "{what}: {scripts} scripts");
    if escapes(payload) {
        assert!(!body.contains(payload), "{what}: raw payload in {body}");
    }
}

/// Assert a refused request: a client error status, the error code, and a safe body.
pub(super) fn assert_refused(reply: &Reply, code: &str, payload: &str, what: &str) {
    assert!(
        reply.status.is_client_error(),
        "{what}: {} {}",
        reply.status,
        reply.body
    );
    assert!(
        reply.body.contains(code),
        "{what}: no {code} in {}",
        reply.body
    );
    assert_safe(reply, payload, what);
}

/// Assert that an HTMX answer sends its error into `#error-panel` and keeps the URL.
pub(super) fn assert_error_panel(reply: &Reply, what: &str) {
    assert_eq!(reply.header("hx-retarget"), "#error-panel", "{what}");
    assert_eq!(reply.header("hx-push-url"), "false", "{what}");
}

/// Send the form `body` to `path` as the engineer `anowak`. The answer has a status below 400.
pub(super) async fn post_as_engineer(world: &AuditWorld, path: &str, body: &str) -> Reply {
    let router = world.fixture.router();
    let reply = post_fields(&router, path, Some(&world.data.anowak), &[], body).await;
    assert!(reply.status.as_u16() < 400, "{path}: {}", reply.body);
    reply
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, StatusCode};

    use super::{assert_safe, escapes, has_control, plain_payloads};
    use crate::pages::test_helpers::payloads;
    use crate::test_support::Reply;

    /// Build a reply with the status `200` and the body `body`.
    fn reply_of(body: &str) -> Reply {
        Reply {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: body.to_owned(),
        }
    }

    #[test]
    #[should_panic(expected = "inline script")]
    fn assert_safe_refuses_an_answer_with_a_raw_script_payload() {
        let payload = "<script>alert(1)</script>";
        assert_safe(
            &reply_of(&format!("<p>{payload}</p>")),
            payload,
            "raw script",
        );
    }

    #[test]
    #[should_panic(expected = "raw payload in")]
    fn assert_safe_refuses_an_answer_that_holds_a_raw_payload_that_needs_an_escape() {
        let payload = "<b>x</b>";
        assert_safe(
            &reply_of(&format!("<p>{payload}</p>")),
            payload,
            "needs an escape",
        );
    }

    #[test]
    #[should_panic(expected = "unknown svg tag")]
    fn assert_safe_refuses_an_answer_with_an_unknown_svg_tag() {
        let body = "<p><svg viewBox=\"0 0 1 1\"></svg></p>";
        assert_safe(&reply_of(body), "x", "unknown svg");
    }

    #[test]
    #[should_panic(expected = "user text in an hx attribute")]
    fn assert_safe_refuses_an_answer_with_the_payload_in_an_hx_attribute() {
        let body = "<a hx-get=\"/x/needle\">x</a>";
        assert_safe(&reply_of(body), "needle", "hx attribute");
    }

    #[test]
    #[should_panic(expected = "<img in the answer")]
    fn assert_safe_refuses_an_answer_with_an_image_tag() {
        assert_safe(&reply_of("<p><img src=\"x\"></p>"), "x", "image");
    }

    #[test]
    #[should_panic(expected = "4 scripts")]
    fn assert_safe_refuses_an_answer_with_more_than_three_script_tags() {
        let body = "<script src=\"a.js\"></script>".repeat(4);
        assert_safe(&reply_of(&body), "x", "four scripts");
    }

    #[test]
    fn assert_safe_accepts_an_answer_that_shows_the_payload_escaped() {
        let payload = "<script>alert(1)</script>";
        let shown = crate::pages::test_helpers::askama_escape(payload);
        let body = format!("<p>{shown}</p><script src=\"a.js\"></script>");
        assert_safe(&reply_of(&body), payload, "escaped");
    }

    #[test]
    fn has_control_and_escapes_tell_the_payloads_apart() {
        assert!(has_control("a\u{0}b"));
        assert!(!has_control("a b"));
        assert!(escapes("<b>"));
        assert!(!escapes("plain"));
    }

    #[test]
    fn plain_payloads_keep_every_payload_without_a_control_character() {
        let plain = plain_payloads();
        let expected = payloads()
            .into_iter()
            .filter(|payload| !payload.chars().any(char::is_control))
            .count();
        assert_eq!(plain.len(), expected);
        assert!(plain.len() < payloads().len(), "the NUL payload must go");
        assert!(plain.iter().all(|payload| !has_control(payload)));
    }
}
