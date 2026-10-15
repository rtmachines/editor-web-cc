//! Tests of the redirect helpers of `render.rs` (task 6.1, design D19).

use axum::http::header::{LOCATION, SET_COOKIE};
use axum::http::{HeaderValue, StatusCode};
use recipe_domain::audit::RequestId;
use recipe_domain::error_codes::ErrorCode;
use uuid::Uuid;

use askama::Template;

use super::{page_url, see_other_path};
use crate::error::WebError;
use crate::i18n::{Bundles, Lang};
use crate::pages::Texts;

/// The id of the request that the tests pass to `see_other_path`.
fn request_id() -> RequestId {
    RequestId::from(Uuid::from_u128(42))
}

#[test]
fn render_see_other_path_builds_a_303_with_the_location_and_no_cookie() {
    let response = see_other_path("/machines/M7", request_id(), []).unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get(LOCATION).unwrap(), "/machines/M7");
    assert!(response.headers().get(SET_COOKIE).is_none());
}

#[test]
fn render_see_other_path_keeps_the_path_with_its_query() {
    for location in [
        "/machines?archived=M7",
        "/machines/M7?archived=U1.2",
        "/machines/M7/types/ST_Zone?rev=2",
        "/",
    ] {
        let response = see_other_path(location, request_id(), []).unwrap();
        assert_eq!(response.headers().get(LOCATION).unwrap(), location);
    }
}

#[test]
fn render_see_other_path_appends_every_cookie_in_order() {
    let cookies = [
        HeaderValue::from_static("a=1; Path=/"),
        HeaderValue::from_static("b=2; Path=/"),
    ];
    let response = see_other_path("/me", request_id(), cookies).unwrap();
    let found: Vec<&HeaderValue> = response.headers().get_all(SET_COOKIE).iter().collect();
    assert_eq!(found, ["a=1; Path=/", "b=2; Path=/"]);
}

#[test]
fn render_see_other_path_refuses_a_location_that_is_not_a_header_value() {
    for location in ["/a\nb", "/a\r\nSet-Cookie: x=y", "/a\u{7f}b", "/a\u{0}b"] {
        let error = see_other_path(location, request_id(), []).unwrap_err();
        assert!(matches!(error, WebError::Internal { .. }), "{location:?}");
        assert_eq!(error.code(), ErrorCode::WEB_010);
    }
}

#[test]
fn render_see_other_path_refuses_a_location_with_a_character_outside_ascii() {
    for location in ["/machines?q=\u{141}", "/\u{141}", "/a\u{e9}b"] {
        let error = see_other_path(location, request_id(), []).unwrap_err();
        assert!(matches!(error, WebError::Internal { .. }), "{location:?}");
    }
}

#[test]
fn render_see_other_path_refuses_a_location_that_leaves_the_site() {
    for location in [
        "",
        "machines",
        "//evil.example",
        "/\\evil.example",
        "/a\\b",
        "http://evil.example/",
        "https:/evil.example",
    ] {
        let error = see_other_path(location, request_id(), []).unwrap_err();
        assert!(matches!(error, WebError::Internal { .. }), "{location:?}");
    }
}

// --- Texts with one argument --------------------------------------------------------------------

#[test]
fn render_with_arg_inserts_the_argument_in_both_languages() {
    let bundles = Bundles::load().unwrap();
    let en = Texts::new(&bundles, Lang::En);
    let pl = Texts::new(&bundles, Lang::Pl);
    assert_eq!(
        en.with_arg("tree-add-sub-unit-to", "path", "U1.2"),
        "Add a sub-unit to U1.2"
    );
    assert_eq!(
        pl.with_arg("tree-add-sub-unit-to", "path", "U1.2"),
        "Dodaj podjednostkę do U1.2"
    );
    assert_eq!(
        en.with_arg("udt-member-name", "number", 3),
        "Name of member 3"
    );
    assert_eq!(pl.with_arg("pager-current", "page", 12), "Strona 12");
}

#[test]
fn render_with_arg_shows_the_key_for_an_unknown_key_or_a_wrong_argument_name() {
    let bundles = Bundles::load().unwrap();
    let en = Texts::new(&bundles, Lang::En);
    assert_eq!(en.with_arg("no-such-key", "path", "U1"), "no-such-key");
    assert_eq!(
        en.with_arg("tree-add-sub-unit-to", "wrong", "U1"),
        "tree-add-sub-unit-to"
    );
}

// --- The address of a page of a list ------------------------------------------------------------

/// The search text of the spec "Search text in the pager links": every kind of reserved character.
const SEARCH: &str = "a&b #+%\u{141}";
/// The percent-encoded form of `SEARCH`.
const ENCODED: &str = "a%26b%20%23%2B%25%C5%81";

#[derive(Template)]
#[template(path = "test/pager.html")]
struct PagerPage<'a> {
    t: Texts<'a>,
    previous: String,
    next: String,
    current: u32,
}

#[test]
fn render_page_url_encodes_every_query_value() {
    let url = page_url("/machines", &[("q", SEARCH)], 2);
    assert_eq!(url, format!("/machines?q={ENCODED}&page=2"));
}

#[test]
fn render_page_url_keeps_the_order_of_the_pairs_and_ends_with_the_page() {
    let url = page_url("/machines", &[("q", "x"), ("show_archived", "1")], 3);
    assert_eq!(url, "/machines?q=x&show_archived=1&page=3");
    let usage = page_url("/machines/M7/types/ST_Zone/usage", &[("rev", "2")], 1);
    assert_eq!(usage, "/machines/M7/types/ST_Zone/usage?rev=2&page=1");
}

#[test]
fn render_page_url_without_a_pair_holds_the_page_only() {
    assert_eq!(page_url("/machines", &[], 4), "/machines?page=4");
}

#[test]
fn render_page_url_encodes_the_key_too_and_keeps_only_the_unreserved_characters() {
    assert_eq!(page_url("/p", &[("a b", "c")], 1), "/p?a%20b=c&page=1");
    assert_eq!(
        page_url("/p", &[("k", "AZaz09-._~")], 1),
        "/p?k=AZaz09-._~&page=1"
    );
    let reserved = page_url("/p", &[("k", "/=?\"'<>\n\u{e9}")], 1);
    assert_eq!(reserved, "/p?k=%2F%3D%3F%22%27%3C%3E%0A%C3%A9&page=1");
}

#[test]
fn render_pager_link_for_a_search_text_holds_the_encoded_text() {
    let bundles = Bundles::load().unwrap();
    let html = PagerPage {
        t: Texts::new(&bundles, Lang::En),
        previous: String::new(),
        next: page_url("/machines", &[("q", SEARCH)], 2),
        current: 1,
    }
    .render()
    .unwrap();
    let link =
        format!("<a class=\"target-link\" href=\"/machines?q={ENCODED}&#38;page=2\">Next page</a>");
    assert!(html.contains(&link), "{html}");
    for raw in ["a&b", " #", "Ł"] {
        assert!(!html.contains(raw), "raw text {raw:?} in {html}");
    }
}

#[test]
fn render_see_other_path_error_carries_the_id_of_the_request() {
    for location in ["//evil.example", "/a\nb", "/\u{141}"] {
        let error = see_other_path(location, request_id(), []).unwrap_err();
        assert!(
            matches!(error, WebError::Internal { request_id: found } if found == request_id()),
            "{location:?}"
        );
    }
}
