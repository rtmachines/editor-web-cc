//! Helpers that the tests of the pages share.

use std::iter::Peekable;
use std::str::Chars;

use axum::http::StatusCode;
use recipe_app::auth::RequestMeta;
use recipe_app::blocking::{AppSettings, Blocking, PageSizes};
use recipe_app::identity::new_request_id;
use recipe_app::recipe::{ApproveVersion, ApproveVersionInput, CreateVersion, CreateVersionInput};
use recipe_domain::recipe::ReferenceNumber;
use recipe_domain::text::LongText;

use crate::test_support::recipes::approval::ApprovalData;
use crate::test_support::{Fixture, Reply};

mod tests;

/// The value of `HX-Retarget` of an HTMX error answer.
const ERROR_PANEL: &str = "#error-panel";
/// The value of `HX-Reswap` of an HTMX error answer (ADR-0036).
const ERROR_RESWAP: &str = "innerHTML show:#error-panel:top";
/// The empty toast that every HTMX error answer swaps out of band (ADR-0049 (k)).
pub(crate) const TOAST_CLEAR: &str = "<div id=\"toast\" hx-swap-oob=\"innerHTML\"></div>";
/// The payloads of the corpus file, one per line. A line that starts with `#` is a comment.
const CORPUS: &str = include_str!("../../tests/web/xss-corpus.txt");
/// A payload with a NUL character. A text file cannot hold it, so the tests add it.
const NUL_PAYLOAD: &str = "\u{0}<script>alert(1)</script>";

/// The `Content-Security-Policy` of every response (`api.md` section 9).
pub(crate) const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

/// Return the payloads of the corpus file and the NUL payload.
pub(crate) fn payloads() -> Vec<String> {
    let mut payloads: Vec<String> = CORPUS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect();
    payloads.push(NUL_PAYLOAD.to_owned());
    payloads
}

/// Percent-encode every byte that is not an unreserved character, so a payload fits a form or a path.
pub(crate) fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// Escape a text like askama does in HTML (`&`, `<`, `>`, `"`, and `'`).
pub(crate) fn askama_escape(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '&' => "&#38;".to_owned(),
            '<' => "&#60;".to_owned(),
            '>' => "&#62;".to_owned(),
            '"' => "&#34;".to_owned(),
            '\'' => "&#39;".to_owned(),
            other => other.to_string(),
        })
        .collect()
}

/// Assert this status with the code in the body.
pub(crate) fn assert_error(reply: &Reply, status: StatusCode, code: &str, what: &str) {
    assert_eq!(reply.status, status, "{what}: {}", reply.body);
    assert!(reply.body.contains(code), "{what}: {}", reply.body);
}

/// Assert the HTMX headers of an error answer (`api.md` section 5.3, ADR-0036, ADR-0037 (z)).
///
/// An HTMX request gets `HX-Retarget`, `HX-Reswap`, and `HX-Push-Url: false`. A plain request gets
/// none of them. `what` names the case in each failure message.
fn assert_error_headers(reply: &Reply, htmx: bool, what: &str) {
    let expected = if htmx {
        (ERROR_PANEL, ERROR_RESWAP, "false")
    } else {
        ("", "", "")
    };
    let (retarget, reswap, push) = (
        reply.header("hx-retarget"),
        reply.header("hx-reswap"),
        reply.header("hx-push-url"),
    );
    let actual = (retarget.as_str(), reswap.as_str(), push.as_str());
    assert_eq!(actual, expected, "{what}: {:?}", reply.headers);
}

/// Assert an HTMX error answer: the status, the code in the body, and the three headers.
///
/// Do not use it for `E-WEB-004`. That answer sets `HX-Redirect: /login` and none of the three
/// headers (`api.md` section 5.1).
pub(crate) fn assert_htmx_error(reply: &Reply, status: StatusCode, code: &str) {
    assert_error(reply, status, code, code);
    assert_error_headers(reply, true, code);
}

/// Assert the headers and the empty toast of a refusal, and that it sends no `HX-Redirect`.
///
/// An HTMX request gets the three headers and one empty toast. A plain request gets neither.
pub(crate) fn assert_refusal_headers(reply: &Reply, htmx: bool, what: &str) {
    assert_error_headers(reply, htmx, what);
    assert_eq!(reply.header("hx-redirect"), "", "{what}");
    let toasts = reply.body.matches(TOAST_CLEAR).count();
    assert_eq!(toasts, usize::from(htmx), "{what}: {}", reply.body);
}

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

/// Return the part of `main` from the first `start` to the next `end`, or to the end.
///
/// The function fails when `start` is not in `main`.
pub(crate) fn part_of<'a>(main: &'a str, start: &str, end: &str) -> &'a str {
    assert!(main.contains(start), "{start} is not in {main}");
    let rest = &main[main.find(start).unwrap()..];
    let to = rest[start.len()..]
        .find(end)
        .map_or(rest.len(), |at| at.checked_add(start.len()).unwrap());
    &rest[..to]
}

/// Return the section `kind` of the diff in `main`.
pub(crate) fn section<'a>(main: &'a str, kind: &str) -> &'a str {
    part_of(
        main,
        &format!("<section class=\"diff-section\" id=\"diff-{kind}\""),
        "</section>",
    )
}

/// Return the position of `needle` in `text`, or fail with `text`.
pub(crate) fn position(text: &str, needle: &str) -> usize {
    assert!(text.contains(needle), "{needle} is not in {text}");
    text.find(needle).unwrap()
}

/// Assert that `first` and `second` stand in `text` and that `first` comes before `second`.
///
/// The check fails when a view swaps the old and the new side of a row.
pub(crate) fn assert_before(text: &str, first: &str, second: &str) {
    let (at_first, at_second) = (position(text, first), position(text, second));
    assert!(
        at_first < at_second,
        "{first} must come before {second}: {text}"
    );
}

/// A fixture with the approval data, `diff` parameters on each page of a diff, and `blocking`.
///
/// The fixture lets a test cut a diff into many pages and make the diff stage fail.
pub(crate) async fn small_pages(diff: u32, blocking: Blocking) -> (Fixture, ApprovalData) {
    let defaults = AppSettings::default();
    let settings = AppSettings {
        blocking,
        pages: PageSizes {
            diff,
            ..defaults.pages
        },
        ..defaults
    };
    let fixture = Fixture::with_app_settings(settings);
    let data = fixture.seed_approval_recipes().await;
    (fixture, data)
}

/// Return the data of a seed request with a new request identifier.
fn seed_meta() -> RequestMeta {
    RequestMeta::new(new_request_id().unwrap(), None, None, None)
}

/// Approve the draft version 2 of `R-500` and start the draft version 3 of `R-500` on it.
///
/// Version 3 has the same parameters as version 2, because nobody edits it. So a diff of version 3
/// against version 2 and the approval preview of version 3 have no change. Call it once for each
/// fixture, after `seed_approval_recipes`.
pub(crate) async fn seed_unedited_version_3(fixture: &Fixture, data: &ApprovalData) {
    let ctx = fixture.state.identity().app().clone();
    let reference = ReferenceNumber::parse("R-500").unwrap();
    let approve = ApproveVersionInput {
        reference: reference.clone(),
        number: 2,
        expected_row_version: data.r500_v2_row_version,
        allow_outdated: false,
    };
    ApproveVersion::new(ctx.clone())
        .execute(&data.admin_user(), &seed_meta(), approve)
        .await
        .unwrap();
    let create = CreateVersionInput {
        reference,
        based_on: Some(2),
        change_note: LongText::parse("Third version").unwrap(),
    };
    let created = CreateVersion::new(ctx)
        .execute(&data.anowak_user(), &seed_meta(), create)
        .await
        .unwrap();
    assert_eq!(created.number, 3);
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

/// An `hx-*` attribute of a start tag.
pub(crate) struct HxAttribute {
    /// The name without a `data-` prefix.
    pub(crate) name: String,
    /// The full value of the attribute.
    pub(crate) value: String,
    /// The lower case name of the element that holds the attribute.
    pub(crate) element: String,
}

/// Return the `hx-*` and `data-hx-*` attributes of every start tag of `html` that have a value.
///
/// The reader uses the tokenizer of `assert_clean_html`. An attribute starts inside a tag only, so
/// the text of a paragraph is no attribute. A quoted value may hold spaces.
pub(crate) fn hx_attributes(html: &str) -> Vec<HxAttribute> {
    let mut found = Vec::new();
    for tag in start_tags(html) {
        for (name, value) in tag.attributes {
            let bare = name.strip_prefix("data-").unwrap_or(&name);
            if bare.starts_with("hx-") {
                found.push(HxAttribute {
                    name: bare.to_owned(),
                    value,
                    element: tag.name.clone(),
                });
            }
        }
    }
    found
}

/// Return the name and the value of every `hx-*` attribute that has a value in `html`.
pub(crate) fn hx_pairs(html: &str) -> Vec<(String, String)> {
    hx_attributes(html)
        .into_iter()
        .map(|attribute| (attribute.name, attribute.value))
        .collect()
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

/// A tag of a page: its lower case name, whether it closes, and its attributes in source order.
///
/// The type solves one problem: `StartTag` drops an attribute with no value, such as `required`,
/// and it has no end tags. A sweep that checks a missing attribute needs both.
pub(crate) struct Tag {
    /// The lower case name of the element.
    pub(crate) name: String,
    /// True for an end tag such as `</p>`.
    pub(crate) closing: bool,
    /// The attributes in source order. An attribute with no value has `None`.
    pub(crate) attributes: Vec<(String, Option<String>)>,
}

impl Tag {
    /// Return the value of the attribute `name`. An attribute with no value gives an empty text.
    pub(crate) fn attribute(&self, name: &str) -> Option<&str> {
        let found = self.attributes.iter().find(|(known, _value)| known == name);
        found.map(|(_name, value)| value.as_deref().unwrap_or_default())
    }
}

/// Return true for a character that separates attributes.
fn is_separator(c: &char) -> bool {
    c.is_ascii_whitespace() || *c == '/'
}

/// Read the tag that `cursor` is in. The cursor stands behind the `<`.
///
/// A quoted value may hold `<` and `>`. An unquoted value keeps a leading `/`. Only white space
/// separates a name from its `=` and the `=` from its value.
fn read_tag(cursor: &mut Cursor<'_>) -> Tag {
    let closing = cursor.next_if_eq(&'/').is_some();
    let name = take_while(cursor, |c| !is_separator(&c) && c != '>');
    let mut attributes = Vec::new();
    loop {
        skip_while(cursor, |c| is_separator(&c));
        if cursor.peek().is_none_or(|c| *c == '>') {
            break;
        }
        let attribute = take_while(cursor, |c| !is_separator(&c) && !"=>".contains(c));
        if attribute.is_empty() {
            cursor.next();
            continue;
        }
        skip_while(cursor, |c| c.is_ascii_whitespace());
        let value = cursor.next_if_eq(&'=').map(|_equals| {
            skip_while(cursor, |c| c.is_ascii_whitespace());
            take_value(cursor)
        });
        attributes.push((attribute.to_ascii_lowercase(), value));
    }
    Tag {
        name: name.to_ascii_lowercase(),
        closing,
        attributes,
    }
}

/// Skip a comment, a doctype, or another declaration. The cursor stands on the `!`.
///
/// A comment ends at `-->`. The dash count starts at two, so `<!-->` ends at once, like in a browser.
fn skip_declaration(cursor: &mut Cursor<'_>) {
    cursor.next();
    let is_comment = cursor.next_if_eq(&'-').is_some() && cursor.next_if_eq(&'-').is_some();
    if !is_comment {
        skip_while(cursor, |c| c != '>');
        cursor.next();
        return;
    }
    let mut dashes = 2_u8;
    for c in cursor.by_ref() {
        match c {
            '-' => dashes = dashes.saturating_add(1),
            '>' if dashes >= 2 => return,
            _other => dashes = 0,
        }
    }
}

/// Return all start tags and end tags of `html` in source order.
///
/// A page escapes every `<` of a text, so each `<` that a letter or `/` follows opens a tag. The
/// reader skips the markup inside a comment.
pub(crate) fn tags(html: &str) -> Vec<Tag> {
    let mut found = Vec::new();
    let mut cursor = html.chars().peekable();
    while let Some(c) = cursor.next() {
        let next = cursor.peek().copied().filter(|_| c == '<');
        match next {
            Some('!') => skip_declaration(&mut cursor),
            Some(n) if n.is_ascii_alphabetic() || n == '/' => found.push(read_tag(&mut cursor)),
            _other => {}
        }
    }
    found
}
