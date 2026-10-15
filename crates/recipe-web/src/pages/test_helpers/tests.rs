//! Tests of `assert_clean_html` with literal inputs (task 7.1, design D10).

use axum::http::StatusCode;

use super::{
    askama_escape, assert_clean_html, assert_error, assert_no_user_text_in_hx_attributes, encode,
    hx_attributes, payloads,
};
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
#[should_panic(expected = "wrong code")]
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
