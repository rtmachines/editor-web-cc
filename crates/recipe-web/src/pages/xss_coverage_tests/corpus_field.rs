//! The corpus check: does the code of a corpus test send a value in a request field?

use super::compare::is_word;

/// The calls that read a text. A name in their argument is a probe and no request field.
const PROBE_CALLS: [&str; 4] = ["contains(", "starts_with(", "ends_with(", "find("];

/// Return the index just after the `)` that closes the `(` at `open`, or the text length.
///
/// A parenthesis inside a string literal does not count.
fn after_closing_paren(text: &str, open: usize) -> usize {
    let (mut depth, mut in_string, mut escaped) = (0_usize, false, false);
    for (at, c) in text.char_indices().skip_while(|(at, _)| *at < open) {
        match (in_string, c) {
            (true, _) if escaped => escaped = false,
            (true, '\\') => escaped = true,
            (true, '"') => in_string = false,
            (true, _) => {}
            (false, '"') => in_string = true,
            (false, '(') => depth = depth.saturating_add(1),
            (false, ')') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return at.saturating_add(1);
                }
            }
            (false, _) => {}
        }
    }
    text.len()
}

/// Return the text without the arguments of each call or macro whose name starts with `assert`.
///
/// A name in a message or in an argument of an assertion does not submit a field.
fn without_assertions(code: &str) -> String {
    let mut kept = String::new();
    let mut rest = code;
    while let Some(at) = rest.find("assert") {
        let start = rest
            .get(..at)
            .unwrap_or_default()
            .rfind(|c: char| !is_word(c))
            .map_or(0, |end| end.saturating_add(1));
        let tail = rest.get(start..).unwrap_or_default();
        let word_len = tail.find(|c: char| !is_word(c)).unwrap_or(tail.len());
        let word = tail.get(..word_len).unwrap_or_default();
        let after = tail.get(word_len..).unwrap_or_default().trim_start();
        let call = after.strip_prefix('!').unwrap_or(after).trim_start();
        if (word.starts_with("assert") || word.starts_with("debug_assert")) && call.starts_with('(')
        {
            kept.push_str(rest.get(..start).unwrap_or_default());
            let open = rest.len().saturating_sub(call.len());
            let close = after_closing_paren(rest, open);
            rest = rest.get(close..).unwrap_or_default();
        } else {
            let resume = start.saturating_add(word_len.max(1));
            kept.push_str(rest.get(..resume).unwrap_or(rest));
            rest = rest.get(resume..).unwrap_or_default();
        }
    }
    kept.push_str(rest);
    kept
}

/// Return the text after each place where `key` stands as a whole word.
fn tails_of<'a>(code: &'a str, key: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    code.match_indices(key).filter_map(move |(at, _)| {
        let before = code.get(..at).and_then(|head| head.chars().next_back());
        if before.is_some_and(is_word) {
            return None;
        }
        code.get(at.saturating_add(key.len())..)
    })
}

/// Return true when the quoted name `"field"` is no argument of a probe call such as `contains`.
fn is_quoted_outside_probe(code: &str, field: &str) -> bool {
    let quoted = format!("\"{field}\"");
    code.match_indices(&quoted).any(|(at, _)| {
        let head = code.get(..at).unwrap_or_default();
        let call = head.trim_end().trim_end_matches('&').trim_end();
        !PROBE_CALLS.iter().any(|probe| call.ends_with(probe))
    })
}

/// Return true when the code has a form key `field=` with a value that is not a literal.
///
/// A placeholder `{` or a closed literal `"` after the `=` means that the value comes from a
/// variable. A literal such as `field=wrong` is a constant.
fn has_form_key_with_value(code: &str, field: &str) -> bool {
    tails_of(code, &format!("{field}=")).any(|tail| tail.starts_with(['{', '"']))
}

/// Return true when the code has a struct key `field: value` with a value that is not a literal.
fn has_struct_key_with_value(code: &str, field: &str) -> bool {
    tails_of(code, &format!("{field}:")).any(|tail| {
        let value = tail.trim_start();
        let gap = tail.len().saturating_sub(value.len());
        gap > 0 && !value.starts_with(|c: char| c == '"' || c.is_ascii_digit())
    })
}

/// Return true when the code of a corpus test sends a value in the request field `field`.
///
/// The field counts in one of three forms. The first is a quoted name (`("note", ..)`, a list of
/// names) outside a probe call such as `contains("note")`. The second is a form key `note={..}`
/// with a value that is no literal. The third is a struct key `note: ..` with a value that is no
/// literal. A name in an assertion, in a comment, or inside a longer word does not count.
///
/// The check does not trace a value. It does not prove that the corpus variable is the value. A
/// quoted name in a list or in a pair counts even when its partner is a constant. A helper
/// parameter counts at every call. A form key with a placeholder counts for a constant variable.
/// A struct key counts for every struct, also for a struct that is no request. The check solves
/// the fault of a constant next to the corpus value, such as `password=wrong`.
pub(super) fn submits_field(code: &str, field: &str) -> bool {
    let code = without_assertions(code);
    is_quoted_outside_probe(&code, field)
        || has_form_key_with_value(&code, field)
        || has_struct_key_with_value(&code, field)
}
