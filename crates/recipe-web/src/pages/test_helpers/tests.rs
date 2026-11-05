//! Tests of the page helpers: `assert_clean_html` (task 7.1, design D10) and `assert_common_headers` (design D4).

use axum::http::{HeaderMap, StatusCode};
use uuid::Uuid;

use super::{
    SECURITY_HEADERS, askama_escape, assert_clean_html, assert_common_headers, assert_error,
    assert_no_user_text_in_hx_attributes, encode, hx_attributes, payloads,
};
use crate::WebSettings;
use crate::test_support::Reply;

#[test]
#[should_panic(expected = "event handler")]
fn clean_html_finds_a_handler_after_a_space() {
    assert_clean_html("<a href=\"/x\" onclick=alert(1)>x</a>");
}

#[test]
#[should_panic(expected = "event handler")]
fn clean_html_finds_a_handler_after_a_slash() {
    assert_clean_html("<svg/onbegin=alert(1)>");
}

#[test]
#[should_panic(expected = "event handler")]
fn clean_html_finds_a_handler_after_a_double_quote() {
    assert_clean_html("<a href=\"x\"onclick=alert(1)>x</a>");
}

#[test]
#[should_panic(expected = "event handler")]
fn clean_html_finds_a_handler_after_a_single_quote() {
    assert_clean_html("<a title='x'onclick=alert(1)>x</a>");
}

#[test]
#[should_panic(expected = "event handler")]
fn clean_html_finds_an_upper_case_handler() {
    assert_clean_html("<body\tONLOAD=alert(1)>");
}

#[test]
#[should_panic(expected = "style element")]
fn clean_html_rejects_a_style_element() {
    assert_clean_html("<style>a{color:red}</style>");
}

#[test]
#[should_panic(expected = "style element")]
fn clean_html_rejects_an_upper_case_style_element_with_an_attribute() {
    assert_clean_html("<STYLE media=\"all\">a{}</STYLE>");
}

#[test]
#[should_panic(expected = "inline style")]
fn clean_html_finds_a_style_attribute_after_a_quote() {
    assert_clean_html("<a href=\"x\"style=\"color:red\">x</a>");
}

#[test]
#[should_panic(expected = "inline style")]
fn clean_html_finds_an_upper_case_style_attribute_after_a_tab() {
    assert_clean_html("<p\tSTYLE=\"color:red\">x</p>");
}

#[test]
#[should_panic(expected = "inline script")]
fn clean_html_rejects_an_inline_script() {
    assert_clean_html("<script>alert(1)</script>");
}

#[test]
#[should_panic(expected = "inline script")]
fn clean_html_rejects_an_upper_case_inline_script() {
    assert_clean_html("<SCRIPT>alert(1)</SCRIPT>");
}

#[test]
#[should_panic(expected = "inline script")]
fn clean_html_rejects_a_script_with_an_empty_src() {
    assert_clean_html("<script src=\"\"></script>");
}

#[test]
#[should_panic(expected = "inline script")]
fn clean_html_rejects_a_script_whose_src_is_inside_another_value() {
    assert_clean_html("<script title=' src=\"/x.js\"'>alert(1)</script>");
}

#[test]
#[should_panic(expected = "inline script")]
fn clean_html_rejects_a_script_whose_src_name_only_ends_like_src() {
    assert_clean_html("<script data-src=\"/x.js\">alert(1)</script>");
}

#[test]
fn clean_html_accepts_scripts_with_a_real_src_in_any_case() {
    assert_clean_html("<SCRIPT SRC=\"/static/a.js\" defer></SCRIPT>");
    assert_clean_html("<script defer src='/static/a.js'></script>");
    assert_clean_html("<script src=/static/a.js></script>");
}

#[test]
fn clean_html_does_not_split_a_tag_on_a_non_ascii_space() {
    assert_clean_html("<a title=\"x\" \u{a0}onclick=1>x</a>");
}

#[test]
#[should_panic(expected = "user text in an hx attribute")]
fn hx_attributes_find_a_needle_after_a_space_inside_the_value() {
    assert_no_user_text_in_hx_attributes("<div hx-vals='{\"a\": \"NEEDLE\"}'>x</div>", "NEEDLE");
}

#[test]
#[should_panic(expected = "user text in an hx attribute")]
fn hx_attributes_find_a_needle_in_a_data_hx_attribute() {
    assert_no_user_text_in_hx_attributes("<a data-hx-get=\"/x/NEEDLE\">x</a>", "NEEDLE");
}

#[test]
#[should_panic(expected = "user text in an hx attribute")]
fn hx_attributes_find_a_needle_in_an_upper_case_name_after_a_quote() {
    assert_no_user_text_in_hx_attributes("<a id=\"a\"HX-POST='/x/NEEDLE'>x</a>", "NEEDLE");
}

#[test]
#[should_panic(expected = "user text in an hx attribute")]
fn hx_attributes_find_a_needle_in_a_value_without_quotes() {
    assert_no_user_text_in_hx_attributes("<a hx-get=/x/NEEDLE>x</a>", "NEEDLE");
}

#[test]
fn hx_attributes_accept_a_needle_outside_hx_attributes() {
    assert_no_user_text_in_hx_attributes("<p>a hx-get=NEEDLE</p>", "NEEDLE");
    assert_no_user_text_in_hx_attributes("<a title=\"NEEDLE\" hx-get=\"/x\">x</a>", "NEEDLE");
    assert_no_user_text_in_hx_attributes("<a data-x=\"NEEDLE\" data-hx=\"y\">x</a>", "NEEDLE");
}

#[test]
fn hx_attributes_reader_finds_attributes_and_ignores_text() {
    let html = "<p>an hx-boost=\"evil\" text</p><body hx-history=\"false\" data-hx-swap='x y'>\
                <a hx-target=#a href=\"/x\">";
    let found: Vec<(String, String, String)> = hx_attributes(html)
        .into_iter()
        .map(|a| (a.name, a.value, a.element))
        .collect();
    let expected = [
        ("hx-history", "false", "body"),
        ("hx-swap", "x y", "body"),
        ("hx-target", "#a", "a"),
    ];
    assert_eq!(found.len(), expected.len(), "{found:?}");
    for (item, want) in found.iter().zip(expected) {
        assert_eq!((item.0.as_str(), item.1.as_str(), item.2.as_str()), want);
    }
}

#[test]
fn payloads_hold_the_corpus_lines_and_end_with_the_nul_payload() {
    let all = payloads();
    assert!(all.len() > 10, "{all:?}");
    assert!(all.iter().all(|payload| !payload.starts_with('#')));
    assert!(all.iter().any(|payload| payload.contains("<script>")));
    assert!(
        all.last()
            .is_some_and(|payload| payload.starts_with('\u{0}'))
    );
}

#[test]
fn encode_keeps_unreserved_bytes_and_escapes_the_others() {
    assert_eq!(encode("aZ09-._~"), "aZ09-._~");
    assert_eq!(encode("a b&c=\u{e9}"), "a%20b%26c%3D%C3%A9");
}

#[test]
fn askama_escape_replaces_the_five_html_characters() {
    assert_eq!(
        askama_escape("<a href=\"x\">&'</a>"),
        "&#60;a href=&#34;x&#34;&#62;&#38;&#39;&#60;/a&#62;"
    );
    assert_eq!(askama_escape("plain text"), "plain text");
}

#[test]
fn assert_error_accepts_the_status_with_the_code_in_the_body() {
    let reply = Reply {
        status: StatusCode::CONFLICT,
        headers: axum::http::HeaderMap::new(),
        body: "<p>E-PLT-011</p>".to_owned(),
    };
    assert_error(&reply, StatusCode::CONFLICT, "E-PLT-011", "conflict");
}

#[test]
#[should_panic(expected = "wrong code: the code E-PLT-011 is missing")]
fn assert_error_rejects_another_code() {
    let reply = Reply {
        status: StatusCode::CONFLICT,
        headers: axum::http::HeaderMap::new(),
        body: "<p>E-PLT-012</p>".to_owned(),
    };
    assert_error(&reply, StatusCode::CONFLICT, "E-PLT-011", "wrong code");
}

#[test]
fn clean_html_accepts_text_that_only_looks_like_a_handler() {
    assert_clean_html("<input value=\"/onbegin=alert(1)\">");
    assert_clean_html("<input value='&#39;onfocus=alert(1)&#39;'>");
    assert_clean_html("<input value=\"a onfocus=b\">");
    assert_clean_html("<p>a onerror=b /onbegin=c</p>");
    assert_clean_html("<a href=\"/x\">one=1</a>");
    assert_clean_html("<script src=\"/static/htmx.min.js\"></script>");
}

/// A path of a page. Its answer must have `Cache-Control: no-store`.
const PAGE_PATH: &str = "/recipes";
/// A path of a static file. Its answer must have the cache rule of `static.cache_max_age`.
const STATIC_PATH: &str = "/static/app.css";
/// A well formed UUID of version 4. The check of the request id accepts version 7 only.
const VERSION_4_ID: &str = "11111111-1111-4111-8111-111111111111";

/// Build a reply that has every header of `SECURITY_HEADERS` and a version 7 request id.
///
/// The cache rule is the one that `api.md` section 9 gives for `path`.
fn complete_reply(path: &str) -> Reply {
    let settings = WebSettings::default();
    let cache = if path.starts_with("/static/") {
        format!(
            "public, max-age={}",
            settings.static_cache_max_age.as_secs()
        )
    } else {
        "no-store".to_owned()
    };
    let hsts = format!(
        "max-age={}; includeSubDomains",
        settings.hsts_max_age.as_secs()
    );
    let mut headers = HeaderMap::new();
    headers.insert("x-request-id", Uuid::now_v7().to_string().parse().unwrap());
    for (name, value) in SECURITY_HEADERS {
        let text = match name {
            "cache-control" => cache.as_str(),
            "strict-transport-security" => hsts.as_str(),
            _ => value.unwrap(),
        };
        headers.insert(name, text.parse().unwrap());
    }
    Reply {
        status: StatusCode::OK,
        headers,
        body: String::new(),
    }
}

/// Return `reply` without the header `name`.
fn without(mut reply: Reply, name: &str) -> Reply {
    reply.headers.remove(name);
    reply
}

/// Return `reply` with the header `name` set to `value`.
fn with(mut reply: Reply, name: &'static str, value: &str) -> Reply {
    reply.headers.insert(name, value.parse().unwrap());
    reply
}

#[test]
fn common_headers_accept_a_complete_reply_to_a_page_path() {
    assert_common_headers(&complete_reply(PAGE_PATH), PAGE_PATH, "page");
}

#[test]
fn common_headers_accept_the_static_cache_rule_on_a_static_path() {
    assert_common_headers(&complete_reply(STATIC_PATH), STATIC_PATH, "static");
}

#[test]
fn security_headers_name_the_five_headers_of_api_section_9() {
    let names: Vec<&str> = SECURITY_HEADERS.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        names,
        [
            "content-security-policy",
            "x-content-type-options",
            "referrer-policy",
            "strict-transport-security",
            "cache-control",
        ]
    );
}

#[test]
#[should_panic(expected = "the request id is missing")]
fn common_headers_reject_a_reply_without_a_request_id() {
    let reply = without(complete_reply(PAGE_PATH), "x-request-id");
    assert_common_headers(&reply, PAGE_PATH, "no id");
}

#[test]
#[should_panic(expected = "the request id is not a UUID")]
fn common_headers_reject_a_request_id_that_is_not_a_uuid() {
    let reply = with(complete_reply(PAGE_PATH), "x-request-id", "not-a-uuid");
    assert_common_headers(&reply, PAGE_PATH, "bad id");
}

#[test]
#[should_panic(expected = "the request id is not version 7")]
fn common_headers_reject_a_version_4_request_id() {
    let id = VERSION_4_ID.to_owned();
    let reply = with(complete_reply(PAGE_PATH), "x-request-id", &id);
    assert_common_headers(&reply, PAGE_PATH, "v4 id");
}

/// Define a test that removes the header `$name` and expects the panic of the presence check.
///
/// The macro takes the whole `expected` text as a literal, because `should_panic` accepts no macro.
macro_rules! missing_header_test {
    ($test:ident, $name:literal, $expected:literal) => {
        #[test]
        #[should_panic(expected = $expected)]
        fn $test() {
            let reply = without(complete_reply(PAGE_PATH), $name);
            assert_common_headers(&reply, PAGE_PATH, "missing");
        }
    };
}

missing_header_test!(
    common_headers_reject_a_reply_without_the_csp,
    "content-security-policy",
    "the header content-security-policy is missing"
);
missing_header_test!(
    common_headers_reject_a_reply_without_nosniff,
    "x-content-type-options",
    "the header x-content-type-options is missing"
);
missing_header_test!(
    common_headers_reject_a_reply_without_the_referrer_policy,
    "referrer-policy",
    "the header referrer-policy is missing"
);
missing_header_test!(
    common_headers_reject_a_reply_without_hsts,
    "strict-transport-security",
    "the header strict-transport-security is missing"
);
missing_header_test!(
    common_headers_reject_a_reply_without_cache_control,
    "cache-control",
    "the header cache-control is missing"
);

#[test]
#[should_panic(expected = "the value of content-security-policy is wrong")]
fn common_headers_reject_a_changed_csp() {
    let reply = with(
        complete_reply(PAGE_PATH),
        "content-security-policy",
        "default-src *",
    );
    assert_common_headers(&reply, PAGE_PATH, "csp");
}

#[test]
#[should_panic(expected = "the value of strict-transport-security is wrong")]
fn common_headers_reject_a_changed_hsts_value() {
    let reply = with(
        complete_reply(PAGE_PATH),
        "strict-transport-security",
        "max-age=1",
    );
    assert_common_headers(&reply, PAGE_PATH, "hsts");
}

#[test]
#[should_panic(expected = "the value of cache-control is wrong")]
fn common_headers_reject_a_public_cache_rule_on_a_page_path() {
    let reply = with(
        complete_reply(PAGE_PATH),
        "cache-control",
        "public, max-age=86400",
    );
    assert_common_headers(&reply, PAGE_PATH, "page cache");
}

#[test]
#[should_panic(expected = "the value of cache-control is wrong")]
fn common_headers_reject_no_store_on_a_static_path() {
    let reply = with(complete_reply(STATIC_PATH), "cache-control", "no-store");
    assert_common_headers(&reply, STATIC_PATH, "static cache");
}
