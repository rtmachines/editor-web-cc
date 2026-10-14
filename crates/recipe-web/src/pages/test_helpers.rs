//! Helpers that the tests of the pages share.

use crate::test_support::Reply;

/// Return the `Set-Cookie` text of the cookie `name`, or `None`.
pub(crate) fn set_cookie_of(reply: &Reply, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    reply
        .set_cookies()
        .into_iter()
        .find(|text| text.starts_with(&prefix))
}

/// Return the value of the hidden field `name` of a page.
pub(crate) fn field_value(body: &str, name: &str) -> String {
    let marker = format!("name=\"{name}\" value=\"");
    let (_before, rest) = body.split_once(&marker).unwrap();
    rest.split('"').next().unwrap().to_owned()
}

/// Assert that a page has no inline script, no inline style, and no inline event handler.
pub(crate) fn assert_clean_html(body: &str) {
    for part in body.split("<script").skip(1) {
        let tag = part.split('>').next().unwrap();
        assert!(tag.contains(" src=\""), "inline script: {tag}");
    }
    assert!(!body.contains(" style="), "inline style");
    assert!(!body.contains("|safe"));
    let lower = body.to_ascii_lowercase();
    for (at, _) in lower.match_indices(" on") {
        let rest = lower.get(at..).and_then(|text| text.strip_prefix(" on"));
        let name: String = rest
            .unwrap()
            .chars()
            .take_while(char::is_ascii_alphabetic)
            .collect();
        let after = rest.unwrap().get(name.len()..).unwrap_or_default();
        assert!(
            name.is_empty() || !after.starts_with('='),
            "event handler on{name}"
        );
    }
}

/// Assert that no attribute whose name starts with `hx-` holds `needle` (design D13).
pub(crate) fn assert_no_user_text_in_hx_attributes(body: &str, needle: &str) {
    for part in body.split(" hx-").skip(1) {
        let attribute = part.split('>').next().unwrap();
        let value_end = attribute.find(' ').unwrap_or(attribute.len());
        let head = attribute.get(..value_end).unwrap_or_default();
        assert!(
            !head.contains(needle),
            "user text in an hx attribute: hx-{head}"
        );
    }
}
