//! Helpers that the tests of the pages share.

use std::iter::Peekable;
use std::str::Chars;

use crate::test_support::Reply;

mod tests;

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

/// The characters of a page, one at a time, with one look ahead.
type Cursor<'a> = Peekable<Chars<'a>>;

/// Skip the characters of `cursor` while `keep` is true.
fn skip_while(cursor: &mut Cursor<'_>, keep: impl Fn(char) -> bool) {
    while cursor.next_if(|c| keep(*c)).is_some() {}
}

/// Read the characters of `cursor` while `keep` is true.
fn take_while(cursor: &mut Cursor<'_>, keep: impl Fn(char) -> bool) -> String {
    let mut text = String::new();
    while let Some(c) = cursor.next_if(|c| keep(*c)) {
        text.push(c);
    }
    text
}

/// Read the value of an attribute: a quoted text or a text up to a space or `>`.
fn take_value(cursor: &mut Cursor<'_>) -> String {
    match cursor.next_if(|c| matches!(c, '"' | '\'')) {
        Some(quote) => {
            let value = take_while(cursor, |c| c != quote);
            cursor.next();
            value
        }
        None => take_while(cursor, |c| !c.is_ascii_whitespace() && c != '>'),
    }
}

/// A start tag: its lower case name and its attributes with a value, in source order.
struct StartTag {
    name: String,
    attributes: Vec<(String, String)>,
}

/// Read the start tag that `cursor` is in. The cursor stands behind the `<`.
///
/// The tokenizer splits on ASCII white space only, like a browser. Whatever stands between two
/// attributes (spaces, a slash, or nothing after a closing quote) ends the previous one. The text
/// inside a quoted value is never read as an attribute. Names are lower case.
fn read_start_tag(cursor: &mut Cursor<'_>) -> StartTag {
    let name = take_while(cursor, |c| {
        !c.is_ascii_whitespace() && !matches!(c, '/' | '>')
    });
    let mut attributes = Vec::new();
    loop {
        skip_while(cursor, |c| c.is_ascii_whitespace() || c == '/');
        if cursor.peek().is_none_or(|c| *c == '>') {
            return StartTag {
                name: name.to_ascii_lowercase(),
                attributes,
            };
        }
        let attribute = take_while(cursor, |c| {
            !c.is_ascii_whitespace() && !matches!(c, '/' | '>' | '=')
        });
        skip_while(cursor, |c| c.is_ascii_whitespace());
        if cursor.next_if_eq(&'=').is_some() {
            skip_while(cursor, |c| c.is_ascii_whitespace());
            let value = take_value(cursor);
            attributes.push((attribute.to_ascii_lowercase(), value));
        }
    }
}

/// Return all start tags of `html`.
fn start_tags(html: &str) -> Vec<StartTag> {
    let mut tags = Vec::new();
    let mut cursor = html.chars().peekable();
    while let Some(c) = cursor.next() {
        if c == '<' && cursor.peek().is_some_and(char::is_ascii_alphabetic) {
            tags.push(read_start_tag(&mut cursor));
        }
    }
    tags
}

/// Return true when a tag has a `src` attribute with a value.
fn has_src(tag: &StartTag) -> bool {
    tag.attributes
        .iter()
        .any(|(name, value)| name == "src" && !value.is_empty())
}

/// Assert that a page has no inline script, no style element, no inline style, and no inline
/// event handler.
///
/// A handler is an attribute `on…=` in a start tag. It counts after a space, a slash, and a
/// quote (design D10). The text of a quoted value is not an attribute.
pub(crate) fn assert_clean_html(body: &str) {
    let tags = start_tags(body);
    for tag in tags.iter().filter(|tag| tag.name == "script") {
        assert!(has_src(tag), "inline script: {:?}", tag.attributes);
    }
    assert!(
        !body.to_ascii_lowercase().contains("<style"),
        "style element"
    );
    assert!(!body.contains(" style="), "inline style");
    assert!(!body.contains("|safe"));
    for (name, _value) in tags.iter().flat_map(|tag| &tag.attributes) {
        assert!(name != "style", "inline style");
        assert!(
            name.len() <= 2 || !name.starts_with("on"),
            "event handler {name}"
        );
    }
}

/// Assert that no attribute whose name starts with `hx-` or `data-hx-` holds `needle` (design D13).
///
/// The tokenizer reads the full value of each attribute, also a value with spaces.
pub(crate) fn assert_no_user_text_in_hx_attributes(body: &str, needle: &str) {
    for tag in start_tags(body) {
        for (name, value) in &tag.attributes {
            if name.starts_with("hx-") || name.starts_with("data-hx-") {
                assert!(
                    !value.contains(needle),
                    "user text in an hx attribute: {name}={value}"
                );
            }
        }
    }
}
