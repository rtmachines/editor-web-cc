//! Tests of `assert_clean_html` with literal inputs (task 7.1, design D10).

use super::{assert_clean_html, assert_no_user_text_in_hx_attributes};

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
fn clean_html_accepts_text_that_only_looks_like_a_handler() {
    assert_clean_html("<input value=\"/onbegin=alert(1)\">");
    assert_clean_html("<input value='&#39;onfocus=alert(1)&#39;'>");
    assert_clean_html("<input value=\"a onfocus=b\">");
    assert_clean_html("<p>a onerror=b /onbegin=c</p>");
    assert_clean_html("<a href=\"/x\">one=1</a>");
    assert_clean_html("<script src=\"/static/htmx.min.js\"></script>");
}
