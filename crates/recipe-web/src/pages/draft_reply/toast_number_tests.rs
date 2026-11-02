//! Tests of the change number in the success toast of a draft command (task 4.5, design D4).
//!
//! Each toast of a command after which the draft stays live names the new `row_version`. The number
//! equals `rowVersion` in `HX-Trigger` of the same answer (ADR-0108 (f), R39, I12).

use axum::http::{HeaderMap, StatusCode};

use crate::pages::parameters::test_kit::{
    ADD, INT_FIELDS, SPEED_FIELDS, parameter_path, post_htmx,
};
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::Reply;
use crate::test_support::plant::HTMX_CONTENT;
use crate::test_support::recipes::{HOLD_KEY, LINE_KEY, SPEED_KEY};

/// The words that open the change number in the English toast.
const CHANGE_NUMBER: &str = " Change number: ";
/// The start of the value of `HX-Trigger`.
const TRIGGER_START: &str = "{\"rowVersion\": ";

/// Read the number of `HX-Trigger: {"rowVersion": n}` of `reply`.
fn trigger_number(reply: &Reply) -> i64 {
    let header = reply.header("hx-trigger");
    let number = header
        .strip_prefix(TRIGGER_START)
        .and_then(|rest| rest.strip_suffix('}'));
    assert!(number.is_some(), "hx-trigger: {header}");
    number.unwrap().parse().unwrap()
}

/// Return the content of the element with the first `id="toast"` of `body`.
///
/// The content runs from the end of the start tag to the matching end tag. Nested elements of
/// the same tag name are counted. The first `id="toast"` is the region of the layout or the
/// out-of-band element. The result is `None` when the element has no start tag or no end tag.
fn toast_content(body: &str) -> Option<&str> {
    let (head, after_id) = body.split_once("id=\"toast\"")?;
    let (_, tag) = head.rsplit_once('<')?;
    let name: String = tag
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    let (_, content) = after_id.split_once('>')?;
    let open = format!("<{name}");
    let close = format!("</{name}>");
    let mut depth = 1_usize;
    for (at, _) in content.match_indices('<') {
        let tail = content.get(at..)?;
        if tail.starts_with(&close) {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return content.get(..at);
            }
        } else if tail
            .strip_prefix(&open)
            .is_some_and(|rest| rest.starts_with([' ', '>', '/', '\t', '\r', '\n']))
        {
            depth = depth.checked_add(1)?;
        }
    }
    None
}

/// Assert that the toast of `reply` holds `sentence` and the change number `expected`, and that
/// `HX-Trigger` of the same answer holds the same number.
///
/// The text is read only from the content of the first element with `id="toast"`.
pub(crate) fn assert_toast_number(reply: &Reply, sentence: &str, expected: i64) {
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    let region = toast_content(&reply.body);
    assert!(region.is_some(), "toast element: {}", reply.body);
    let region = region.unwrap();
    let split = region.split_once(sentence);
    assert!(split.is_some(), "toast sentence {sentence}: {}", reply.body);
    let rest = split.unwrap().1.strip_prefix(CHANGE_NUMBER);
    assert!(
        rest.is_some(),
        "change number after {sentence}: {}",
        reply.body
    );
    let digits: String = rest
        .unwrap()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let in_toast: i64 = digits.parse().unwrap();
    let ending = format!("{CHANGE_NUMBER}{digits}.");
    assert!(region.contains(&ending), "{}", reply.body);
    assert_eq!(in_toast, trigger_number(reply), "{}", reply.body);
    assert_eq!(in_toast, expected, "{}", reply.body);
}

#[tokio::test]
async fn toast_of_a_saved_value_names_the_change_number() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/value");
    let reply = post_htmx(&world, &path, &[("row_version", "1"), ("value", "1500")]).await;
    assert_toast_number(&reply, "The value of SPEED is saved.", 2);
}

#[tokio::test]
async fn toast_of_saved_limits_names_the_change_number() {
    let world = recipe_world();
    let pairs = [
        ("row_version", "1"),
        ("limits[0].leaf_path", ""),
        ("limits[0].min", "T#1s"),
        ("limits[0].max", "T#2m"),
    ];
    let reply = post_htmx(&world, &parameter_path(HOLD_KEY, "/limits"), &pairs).await;
    assert_toast_number(&reply, "The limits of HOLD are saved.", 2);
}

#[tokio::test]
async fn toast_of_an_added_parameter_names_the_change_number() {
    let world = recipe_world();
    let reply = post_htmx(&world, ADD, &INT_FIELDS).await;
    assert_toast_number(&reply, "Parameter NEW_P is added.", 2);
}

#[tokio::test]
async fn toast_of_a_saved_parameter_names_the_change_number() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/definition");
    let reply = post_htmx(&world, &path, &SPEED_FIELDS).await;
    assert_toast_number(&reply, "Parameter SPEED is saved.", 2);
}

#[tokio::test]
async fn toast_of_a_deleted_parameter_names_the_change_number() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/delete");
    let reply = post_htmx(&world, &path, &[("row_version", "1")]).await;
    assert_toast_number(&reply, "Parameter SPEED is deleted.", 2);
}

#[tokio::test]
async fn toast_of_a_deleted_parameter_on_the_content_target_names_the_change_number() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/delete");
    let reply = world
        .post_with(&world.engineer, &path, "row_version=1", &HTMX_CONTENT)
        .await;
    assert_toast_number(&reply, "Parameter SPEED is deleted.", 2);
}

#[tokio::test]
async fn toast_of_a_saved_translation_names_the_change_number() {
    let world = recipe_world();
    let pairs = [
        ("row_version", "1"),
        ("name", "Prędkość"),
        ("description", "Prędkość prasy"),
    ];
    let path = parameter_path(SPEED_KEY, "/translations/pl");
    let reply = post_htmx(&world, &path, &pairs).await;
    assert_toast_number(&reply, "The translation pl is saved.", 2);
}

#[tokio::test]
async fn toast_of_a_deleted_translation_names_the_change_number() {
    let world = recipe_world();
    let path = parameter_path(LINE_KEY, "/translations/pl/delete");
    let reply = post_htmx(&world, &path, &[("row_version", "1")]).await;
    assert_toast_number(&reply, "The translation pl is deleted.", 2);
}

/// Build a `200 OK` reply with `HX-Trigger` for row version 2 and the given body.
fn hand_built_reply(body: &str) -> Reply {
    let mut headers = HeaderMap::new();
    headers.insert("hx-trigger", "{\"rowVersion\": 2}".parse().unwrap());
    Reply {
        status: StatusCode::OK,
        headers,
        body: body.to_owned(),
    }
}

#[test]
#[should_panic(expected = "toast sentence")]
fn helper_rejects_a_sentence_outside_an_empty_toast() {
    let reply = hand_built_reply(
        "<div id=\"toast\"></div><p>The value of SPEED is saved. Change number: 2.</p>",
    );
    assert_toast_number(&reply, "The value of SPEED is saved.", 2);
}

#[test]
fn helper_reads_a_sentence_inside_nested_elements_of_the_toast() {
    let reply = hand_built_reply(concat!(
        "<div id=\"toast\" role=\"status\"><div class=\"toast\"><div>",
        "The value of SPEED is saved. Change number: 2.</div></div></div>",
        "<div>Later text</div>",
    ));
    assert_toast_number(&reply, "The value of SPEED is saved.", 2);
}
