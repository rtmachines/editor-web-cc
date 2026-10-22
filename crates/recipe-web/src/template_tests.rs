//! Rules that every file below `templates/` must obey (design D8 and D10, `design-system.md` 12).
//!
//! Three rules hold: a template has no literal word, the text attributes hold no letter outside an
//! expression, and no output skips the escape of askama. Each rule is a small function with unit
//! tests on literal inputs, so the rule can fail. One test per rule then applies it to every file.

use std::fs;
use std::path::{Path, PathBuf};

use crate::test_support::source_walk::files_below;

/// The kind of an askama construct.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Comment,
    Tag,
    Expr,
}

/// A piece of a template: markup, or the body of one askama construct.
enum Piece<'a> {
    Markup(&'a str),
    Askama(Kind, &'a str),
}

/// The openers and closers of the askama constructs.
const OPENERS: [(&str, &str, Kind); 3] = [
    ("{#", "#}", Kind::Comment),
    ("{%", "%}", Kind::Tag),
    ("{{", "}}", Kind::Expr),
];

/// The attributes that carry text for a person.
const TEXT_ATTRIBUTES: [&str; 4] = ["aria-label", "title", "placeholder", "alt"];

/// Split a template into markup and askama constructs. An unclosed construct stays in the markup.
fn pieces(text: &str) -> Vec<Piece<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    loop {
        let found = OPENERS
            .iter()
            .filter_map(|(open, close, kind)| rest.find(open).map(|at| (at, *open, *close, *kind)))
            .min_by_key(|(at, ..)| *at);
        let Some((at, open, close, kind)) = found else {
            break;
        };
        let (before, from_open) = rest.split_at(at);
        let Some((body, tail)) = from_open
            .strip_prefix(open)
            .and_then(|after_open| after_open.split_once(close))
        else {
            break;
        };
        out.push(Piece::Markup(before));
        out.push(Piece::Askama(kind, body));
        rest = tail;
    }
    out.push(Piece::Markup(rest));
    out
}

/// Remove every askama comment, tag, and expression.
fn strip_askama(text: &str) -> String {
    pieces(text)
        .into_iter()
        .filter_map(|piece| match piece {
            Piece::Markup(markup) => Some(markup),
            Piece::Askama(..) => None,
        })
        .collect()
}

/// Split markup into the text outside the tags and the inside of each tag.
///
/// A quote inside a tag hides a `>`. An unclosed tag stays in the text, so its letters count.
fn split_markup(markup: &str) -> (String, Vec<String>) {
    let mut outside = String::new();
    let mut tags = Vec::new();
    let mut current: Option<String> = None;
    let mut quote: Option<char> = None;
    for c in markup.chars() {
        let Some(tag) = current.as_mut() else {
            if c == '<' {
                current = Some(String::new());
            } else {
                outside.push(c);
            }
            continue;
        };
        if let Some(open_quote) = quote {
            tag.push(c);
            if c == open_quote {
                quote = None;
            }
        } else if c == '>' {
            tags.extend(current.take());
        } else {
            if c == '"' || c == '\'' {
                quote = Some(c);
            }
            tag.push(c);
        }
    }
    if let Some(unclosed) = current {
        outside.push('<');
        outside.push_str(&unclosed);
    }
    (outside, tags)
}

/// Remove every character entity such as `&amp;` or `&#8212;`.
fn strip_entities(text: &str) -> String {
    let mut parts = text.split('&');
    let mut out = parts.next().unwrap_or_default().to_owned();
    for part in parts {
        match part.split_once(';') {
            Some((name, tail))
                if !name.is_empty()
                    && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '#') =>
            {
                out.push_str(tail);
            }
            _ => {
                out.push('&');
                out.push_str(part);
            }
        }
    }
    out
}

/// Return the alphabetic characters that remain after the removal of all markup (rule "no literal word").
fn letters_left(template: &str) -> String {
    let (outside, _tags) = split_markup(&strip_askama(template));
    strip_entities(&outside)
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect()
}

/// Return the values of the attribute `name` in the inside of one tag.
fn attribute_values<'a>(tag: &'a str, name: &str) -> Vec<&'a str> {
    let marker = format!("{name}=");
    let mut parts = tag.split(marker.as_str());
    let mut previous = parts.next().unwrap_or_default();
    let mut values = Vec::new();
    for part in parts {
        if previous.ends_with(char::is_whitespace) {
            let value = match part.chars().next().filter(|c| *c == '"' || *c == '\'') {
                Some(quote) => part
                    .strip_prefix(quote)
                    .and_then(|inner| inner.split(quote).next()),
                None => part.split_whitespace().next(),
            };
            values.extend(value);
        }
        previous = part;
    }
    values
}

/// Return true when a tag is an `input` of the type `submit` or `button`.
///
/// The attribute `value` of such a tag is the label of the control.
fn is_label_input(tag: &str) -> bool {
    let name = tag.split_whitespace().next().unwrap_or_default();
    name.eq_ignore_ascii_case("input")
        && attribute_values(tag, "type")
            .iter()
            .any(|kind| kind.eq_ignore_ascii_case("submit") || kind.eq_ignore_ascii_case("button"))
}

/// Return true when a tag is a `button`. Its `value` is data and it may hold a key such as `pl`.
fn is_button(tag: &str) -> bool {
    tag.split_whitespace()
        .next()
        .is_some_and(|name| name.eq_ignore_ascii_case("button"))
}

/// Return the values of the attribute `value` that read as a label of a control.
///
/// The value of an `input` of the type `submit` or `button` holds no letter. The value of a
/// `button` is empty or one key of `[a-z0-9_-]+`. A label has a capital or a space.
fn label_values(tag: &str) -> Vec<&str> {
    let values = attribute_values(tag, "value");
    if is_label_input(tag) {
        values
            .into_iter()
            .filter(|value| value.chars().any(char::is_alphabetic))
            .collect()
    } else if is_button(tag) {
        values
            .into_iter()
            .filter(|value| !value.is_empty() && !is_key_literal(value))
            .collect()
    } else {
        Vec::new()
    }
}

/// Return the text attributes that hold a letter outside an expression (rule "text attributes").
///
/// The attribute `value` counts on a button and on an input of the type `submit` or `button`.
fn text_attributes_with_letters(template: &str) -> Vec<String> {
    let (_outside, tags) = split_markup(&strip_askama(template));
    let mut found = Vec::new();
    for tag in &tags {
        for name in TEXT_ATTRIBUTES {
            for value in attribute_values(tag, name) {
                if value.chars().any(char::is_alphabetic) {
                    found.push(format!("{name}=\"{value}\""));
                }
            }
        }
        found.extend(
            label_values(tag)
                .into_iter()
                .map(|value| format!("value=\"{value}\"")),
        );
    }
    found
}

/// Return the text between each pair of equal quotes in the body of one askama construct.
fn string_literals(body: &str) -> Vec<&str> {
    let mut literals = Vec::new();
    let mut open: Option<(char, usize)> = None;
    let mut escaped = false;
    for (index, c) in body.char_indices() {
        let Some((quote, start)) = open else {
            if c == '"' || c == '\'' {
                open = Some((c, index.saturating_add(c.len_utf8())));
            }
            continue;
        };
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == quote {
            literals.extend(body.get(start..index));
            open = None;
        }
    }
    literals
}

/// Return true when a literal is a key, a path, or a kind: pieces of `[a-z0-9_.-]+` joined by `/`.
fn is_key_literal(literal: &str) -> bool {
    literal.split('/').all(|piece| {
        !piece.is_empty()
            && piece
                .bytes()
                .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-'))
    })
}

/// Return the string literals inside `{{ … }}` and `{% … %}` that are not a key (rule "no label
/// in a construct"). A label passed to a macro is such a literal.
fn bad_literals(template: &str) -> Vec<String> {
    pieces(template)
        .into_iter()
        .flat_map(|piece| match piece {
            Piece::Askama(Kind::Comment, _) | Piece::Markup(_) => Vec::new(),
            Piece::Askama(_, body) => string_literals(body),
        })
        .filter(|literal| !is_key_literal(literal))
        .map(str::to_owned)
        .collect()
}

/// Return true when a text is one string literal.
fn is_string_literal(text: &str) -> bool {
    let text = text.trim();
    let Some(quote) = text.chars().next().filter(|c| *c == '"' || *c == '\'') else {
        return false;
    };
    text.len() > 1
        && text.ends_with(quote)
        && text
            .strip_prefix(quote)
            .and_then(|inner| inner.strip_suffix(quote))
            .is_some_and(|inner| !inner.contains(quote))
}

/// Return the leading identifier of a text.
fn leading_name(text: &str) -> &str {
    let text = text.trim_start();
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(text.len());
    text.split_at(end).0
}

/// Return true when a filter call named `e` or `escape` has an argument.
fn is_escape_with_argument(name: &str, after_name: &str) -> bool {
    (name == "e" || name == "escape")
        && after_name
            .trim_start()
            .strip_prefix('(')
            .is_some_and(|inside| !inside.trim_start().starts_with(')'))
}

/// Return the problems of the body of one expression or tag.
fn body_problems(body: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let body = body.trim_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '+' | '~'));
    if let Some(block) = body
        .strip_prefix("filter")
        .filter(|rest| rest.starts_with(char::is_whitespace))
    {
        let name = leading_name(block);
        if block
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|word| word == "safe")
        {
            problems.push("filter block with safe".to_owned());
        }
        if is_escape_with_argument(
            name,
            block.trim_start().strip_prefix(name).unwrap_or_default(),
        ) {
            problems.push("escape filter block with an argument".to_owned());
        }
        return problems;
    }
    let mut prefix = String::new();
    let mut after_operator = false;
    for (index, part) in body.split('|').enumerate() {
        if index > 0 && !after_operator {
            let name = leading_name(part);
            let after_name = part.trim_start().strip_prefix(name).unwrap_or_default();
            if name == "safe" && !is_string_literal(&prefix) {
                problems.push(format!("safe filter on `{}`", prefix.trim()));
            }
            if is_escape_with_argument(name, after_name) {
                problems.push(format!("filter `{name}` with an argument"));
            }
        }
        after_operator = part.is_empty();
        if index > 0 {
            prefix.push('|');
        }
        prefix.push_str(part);
    }
    problems
}

/// Return the unescaped outputs of a template (rule "unescaped output").
fn unescaped_output(template: &str) -> Vec<String> {
    pieces(template)
        .into_iter()
        .flat_map(|piece| match piece {
            Piece::Askama(Kind::Comment, _) | Piece::Markup(_) => Vec::new(),
            Piece::Askama(_, body) => body_problems(body),
        })
        .collect()
}

/// Return the path and the text of every template.
fn templates() -> Vec<(PathBuf, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");
    files_below(&root)
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).unwrap();
            (path, text)
        })
        .collect()
}

// --- Rule "no literal word" -------------------------------------------------------------------

#[test]
fn template_words_a_literal_word_in_markup_is_found() {
    assert_eq!(letters_left("<p>Hello</p>"), "Hello");
    assert_eq!(letters_left("<p>{{ t.x }} and</p>"), "and");
    assert_eq!(letters_left("{% if a %}word{% endif %}"), "word");
}

#[test]
fn template_words_a_word_in_an_unclosed_tag_is_found() {
    assert_eq!(letters_left("<p class=\"a\" word"), "pclassaword");
}

#[test]
fn template_words_markup_comments_entities_and_expressions_leave_no_letter() {
    let template = "<!doctype html>\n<a href=\"{{ x }}\" class='a>b'>&nbsp;{{ y }}&#8212;</a>\n\
                    {# note #}{% if a < b %}{{ c }}{% endif %}";
    assert_eq!(letters_left(template), "");
}

#[test]
fn template_words_an_ampersand_without_an_entity_stays() {
    assert_eq!(letters_left("a &b c"), "abc");
}

#[test]
fn template_words_no_template_holds_a_literal_word() {
    let all = templates();
    assert!(all.len() >= 14, "found {} templates", all.len());
    for (path, text) in all {
        assert_eq!(letters_left(&text), "", "{}", path.display());
    }
}

// --- Rule "text attributes" -------------------------------------------------------------------

#[test]
fn template_attributes_a_letter_in_a_text_attribute_is_found() {
    for attribute in TEXT_ATTRIBUTES {
        let template = format!("<input {attribute}=\"Name\">");
        assert_eq!(
            text_attributes_with_letters(&template),
            [format!("{attribute}=\"Name\"")]
        );
    }
    assert_eq!(text_attributes_with_letters("<img alt='x'>"), ["alt=\"x\""]);
    assert_eq!(
        text_attributes_with_letters("<a title=word>"),
        ["title=\"word\""]
    );
}

#[test]
fn template_attributes_an_expression_or_another_attribute_passes() {
    assert!(
        text_attributes_with_letters("<input aria-label=\"{{ t.name }}\" title=\"\">").is_empty()
    );
    assert!(text_attributes_with_letters("<title>{{ x }}</title>").is_empty());
    assert!(text_attributes_with_letters("<p data-title=\"x\" class=\"title\">").is_empty());
    assert!(
        text_attributes_with_letters("<a placeholder=\"{% if a %}{{ b }}{% endif %}\">").is_empty()
    );
}

#[test]
fn template_attributes_no_template_has_a_literal_text_attribute() {
    for (path, text) in templates() {
        assert_eq!(
            text_attributes_with_letters(&text),
            Vec::<String>::new(),
            "{}",
            path.display()
        );
    }
}

#[test]
fn template_attributes_the_value_of_a_button_or_a_submit_input_is_found() {
    assert_eq!(
        text_attributes_with_letters("<button value=\"Save\">"),
        ["value=\"Save\""]
    );
    assert_eq!(
        text_attributes_with_letters("<input type=\"submit\" value=\"Go\">"),
        ["value=\"Go\""]
    );
    assert_eq!(
        text_attributes_with_letters("<INPUT value='Go' type='button'>"),
        ["value=\"Go\""]
    );
}

#[test]
fn template_attributes_the_value_of_other_tags_and_expressions_passes() {
    assert!(text_attributes_with_letters("<input type=\"text\" value=\"x\">").is_empty());
    assert!(text_attributes_with_letters("<input name=\"a\" value=\"x\">").is_empty());
    assert!(text_attributes_with_letters("<option value=\"x\">").is_empty());
    assert!(text_attributes_with_letters("<button value=\"{{ a }}\">").is_empty());
    assert!(text_attributes_with_letters("<input type=submit value=\"\">").is_empty());
    assert!(text_attributes_with_letters("<button name=\"lang\" value=\"pl\">").is_empty());
    assert_eq!(
        text_attributes_with_letters("<button value=\"save now\">"),
        ["value=\"save now\""]
    );
}

// --- Rule "string literals in constructs" -----------------------------------------------------

#[test]
fn template_literals_a_label_in_an_expression_or_a_tag_is_found() {
    assert_eq!(bad_literals("{{ \"Delete\" }}"), ["Delete"]);
    assert_eq!(
        bad_literals("{{ buttons::primary('Save now') }}"),
        ["Save now"]
    );
    assert_eq!(bad_literals("{% if a == \"Yes\" %}x{% endif %}"), ["Yes"]);
    assert_eq!(
        bad_literals("{{ f(\"ok\", \"Two Words\", \"\") }}"),
        ["Two Words", ""]
    );
    assert_eq!(bad_literals("{{ \"a/B.html\" }}"), ["a/B.html"]);
    assert_eq!(bad_literals("{{ \"a//b\" }}"), ["a//b"]);
}

#[test]
fn template_literals_a_key_a_path_piece_or_a_kind_passes() {
    assert!(bad_literals("{{ t.get(\"user-title\") }}").is_empty());
    assert!(bad_literals("{% import \"components/toast.html\" as toasts -%}").is_empty());
    assert!(bad_literals("{% if kind == 'draft' %}{{ x::y(\"a_b.c-1\") }}{% endif %}").is_empty());
    assert!(bad_literals("{# \"Word\" #}<p>\"Word\"</p>").is_empty());
}

#[test]
fn template_literals_no_template_holds_a_label_in_a_construct() {
    for (path, text) in templates() {
        assert_eq!(
            bad_literals(&text),
            Vec::<String>::new(),
            "{}",
            path.display()
        );
    }
}

// --- Rule "unescaped output" ------------------------------------------------------------------

#[test]
fn template_escape_a_filter_block_with_safe_is_found() {
    assert_eq!(
        unescaped_output("{% filter safe %}x{% endfilter %}"),
        ["filter block with safe"]
    );
    assert_eq!(
        unescaped_output("{%- filter upper|safe -%}"),
        ["filter block with safe"]
    );
    assert!(unescaped_output("{% filter upper %}x{% endfilter %}").is_empty());
}

#[test]
fn template_escape_safe_on_a_variable_is_found() {
    assert_eq!(unescaped_output("{{ x|safe }}"), ["safe filter on `x`"]);
    assert_eq!(unescaped_output("{{ x | safe }}"), ["safe filter on `x`"]);
    assert_eq!(
        unescaped_output("{{ \"a\"|upper|safe }}"),
        ["safe filter on `\"a\"|upper`"]
    );
    assert_eq!(
        unescaped_output("{% if a %}{{ \"a\" ~ b|safe }}{% endif %}"),
        ["safe filter on `\"a\" ~ b`"]
    );
}

#[test]
fn template_escape_safe_on_a_string_literal_passes() {
    assert!(unescaped_output("{{ \"<b>\"|safe }}").is_empty());
    assert!(unescaped_output("{{ '<b>' | safe }}").is_empty());
}

#[test]
fn template_escape_an_escape_filter_with_an_argument_is_found() {
    assert_eq!(
        unescaped_output("{{ x|e(\"none\") }}"),
        ["filter `e` with an argument"]
    );
    assert_eq!(
        unescaped_output("{{ x | escape (\"html\") }}"),
        ["filter `escape` with an argument"]
    );
    assert_eq!(
        unescaped_output("{% filter e(\"none\") %}"),
        ["escape filter block with an argument"]
    );
}

#[test]
fn template_escape_the_plain_escape_filter_and_other_words_pass() {
    assert!(unescaped_output("{{ x|e }}").is_empty());
    assert!(unescaped_output("{{ x|escape }}").is_empty());
    assert!(unescaped_output("{{ x|e() }}").is_empty());
    assert!(unescaped_output("{% if a || b %}{{ safe }}{% endif %}").is_empty());
    assert!(unescaped_output("{# x|safe #}").is_empty());
    assert!(unescaped_output("<p>x|safe</p>").is_empty());
}

#[test]
fn template_escape_no_template_skips_the_escape() {
    for (path, text) in templates() {
        assert_eq!(
            unescaped_output(&text),
            Vec::<String>::new(),
            "{}",
            path.display()
        );
    }
}
