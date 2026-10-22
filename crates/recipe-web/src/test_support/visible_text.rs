//! The visible text of an HTML answer, shared by the tests that read the words of a page.
//!
//! A test that checks a sentence must not match the markup around it. One copy of the reader
//! keeps every such test on the same rule.

/// Returns the text that a user reads: the markup is gone.
///
/// The reader drops every character from `<` to the next `>`. It also drops a `>` outside a tag.
/// It does not decode entities.
pub(crate) fn visible_text(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _other if !in_tag => text.push(c),
            _skipped => {}
        }
    }
    text
}

#[test]
fn visible_text_drops_the_tags_and_keeps_the_words() {
    let html = "<p class=\"a\">One <b>two</b></p><ul><li>three</li></ul>";

    assert_eq!(visible_text(html), "One twothree");
}

#[test]
fn visible_text_of_plain_text_is_the_text() {
    assert_eq!(visible_text("no markup"), "no markup");
}
