use sha2::{Digest, Sha256};

use super::*;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn static_files_embedded_htmx_matches_the_pinned_hash() {
    let digest = Sha256::digest(HTMX_BYTES);
    assert_eq!(hex(digest.as_slice()), HTMX_SHA256);
}

#[test]
fn static_files_pin_names_an_htmx_2_release() {
    assert!(HTMX_VERSION.starts_with("2."), "{HTMX_VERSION}");
    assert_eq!(HTMX_SHA256.len(), 64);
    assert!(
        HTMX_SHA256
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
}

#[test]
fn static_files_embedded_file_is_the_upstream_script() {
    assert!(HTMX_BYTES.starts_with(b"var htmx="));
}

/// The file names and the SHA-256 of the 8 font files, checked against the npm tarballs and the
/// jsDelivr listing of `@fontsource/atkinson-hyperlegible-{next,mono}@5.3.0`.
const PINNED_FONTS: [(&str, &str); 8] = [
    (
        "atkinson-hyperlegible-next-latin-400-normal.woff2",
        "5cb240a4d99ab7e6fc3a4d4e6d11d89789fdffe9c86a8384000d662c53283f6f",
    ),
    (
        "atkinson-hyperlegible-next-latin-ext-400-normal.woff2",
        "0f9e442184b58992113a9b26c0b85c380410a60e5f8f3ddc03c54fe80f81bbe8",
    ),
    (
        "atkinson-hyperlegible-next-latin-700-normal.woff2",
        "0dbe07ed7679407db9d90f747d5ca3bd1b2bb7ccbcbe41c8246cfc6d2c7bc3bc",
    ),
    (
        "atkinson-hyperlegible-next-latin-ext-700-normal.woff2",
        "acd5d0771650ceddafa2beb172dc7ada45581f7641db03a70f4a13677ce799d1",
    ),
    (
        "atkinson-hyperlegible-mono-latin-400-normal.woff2",
        "87b5b94c18025729fc6ed4e8c25221ade3746dc87213943fa4de49f1f0f26787",
    ),
    (
        "atkinson-hyperlegible-mono-latin-ext-400-normal.woff2",
        "1960608ff6f1c6c450adb7c6ce4e3d05a7e05dde61449b9f88ae2d14de78e3a1",
    ),
    (
        "atkinson-hyperlegible-mono-latin-700-normal.woff2",
        "f30319aa2ab224c5a19d177617e2141b002fcb95c1afe233214f9d342e47687d",
    ),
    (
        "atkinson-hyperlegible-mono-latin-ext-700-normal.woff2",
        "46ba8678220358e306d918613782dd0f41194be0fd935da9c02d6326abd62fa8",
    ),
];

#[test]
fn static_files_font_table_lists_the_eight_pinned_files() {
    let listed: Vec<&str> = FONTS.iter().map(|f| f.path).collect();
    let pinned: Vec<String> = PINNED_FONTS
        .iter()
        .map(|(name, _)| format!("/static/fonts/{name}"))
        .collect();
    assert_eq!(listed, pinned);
}

#[test]
fn static_files_each_embedded_font_matches_its_constant() {
    for font in &FONTS {
        let expected = PINNED_FONTS
            .iter()
            .find(|(name, _)| format!("/static/fonts/{name}") == font.path)
            .map(|(_, sha256)| *sha256);
        let digest = Sha256::digest(font.bytes);
        assert_eq!(
            Some(hex(digest.as_slice()).as_str()),
            expected,
            "{}",
            font.path
        );
    }
}

// --- The two scripts (design D7, `design-system.md` section 10.8) --------------------------------

/// The words that no script may hold.
const FORBIDDEN_SCRIPT_WORDS: [&str; 12] = [
    "eval",
    "Function(",
    "innerHTML",
    "outerHTML",
    "insertAdjacentHTML",
    "document.write",
    "fetch(",
    "XMLHttpRequest",
    "sendBeacon",
    "WebSocket",
    "EventSource",
    "import(",
];

/// Return the problems of one script: its length, its IIFE head, and its forbidden words.
fn script_problems(source: &str, max_lines: usize) -> Vec<String> {
    let mut problems = Vec::new();
    if source.lines().count() > max_lines {
        problems.push(format!("more than {max_lines} lines"));
    }
    let mut lines = source.lines();
    if lines.next() != Some("(function () {") {
        problems.push("first line is not `(function () {`".to_owned());
    }
    if lines.next().map(str::trim) != Some("\"use strict\";") {
        problems.push("second line is not `\"use strict\";`".to_owned());
    }
    for word in FORBIDDEN_SCRIPT_WORDS {
        if source.contains(word) {
            problems.push(format!("holds `{word}`"));
        }
    }
    if !selector_calls_are_literals(source) {
        problems.push("a querySelector argument is not one string literal".to_owned());
    }
    problems
}

/// Return true when each `querySelector` call takes exactly one string literal.
fn selector_calls_are_literals(source: &str) -> bool {
    source.split("querySelector").skip(1).all(|after| {
        let call = after.strip_prefix("All").unwrap_or(after);
        let Some(argument) = call.strip_prefix('(') else {
            return false;
        };
        let mut chars = argument.chars();
        let Some(quote) = chars.next().filter(|c| *c == '"' || *c == '\'') else {
            return false;
        };
        let rest: String = chars.collect();
        match rest.split_once(quote) {
            Some((literal, tail)) => !literal.contains('\\') && tail.trim_start().starts_with(')'),
            None => false,
        }
    })
}

#[test]
fn script_row_version_is_a_short_strict_iife_without_forbidden_words() {
    let source = std::str::from_utf8(ROW_VERSION_BYTES).unwrap();
    assert_eq!(script_problems(source, 20), Vec::<String>::new());
}

#[test]
fn script_tree_is_a_short_strict_iife_without_forbidden_words() {
    let source = std::str::from_utf8(TREE_BYTES).unwrap();
    assert_eq!(script_problems(source, 50), Vec::<String>::new());
}

#[test]
fn script_problems_find_a_long_script() {
    let source = format!(
        "(function () {{\n\"use strict\";\n{}}})();\n",
        "var a;\n".repeat(30)
    );
    assert_eq!(script_problems(&source, 20), ["more than 20 lines"]);
}

#[test]
fn script_problems_find_a_missing_strict_head() {
    let plain = "(function () {\nvar a;\n})();\n";
    assert_eq!(
        script_problems(plain, 20),
        ["second line is not `\"use strict\";`"]
    );
    let arrow = "(() => {\n\"use strict\";\n})();\n";
    assert_eq!(
        script_problems(arrow, 20),
        ["first line is not `(function () {`"]
    );
}

#[test]
fn script_problems_find_each_forbidden_word() {
    for word in FORBIDDEN_SCRIPT_WORDS {
        let source = format!("(function () {{\n\"use strict\";\nx {word};\n}})();\n");
        assert_eq!(script_problems(&source, 20), [format!("holds `{word}`")]);
    }
}

#[test]
fn script_selector_calls_accept_one_string_literal() {
    assert!(selector_calls_are_literals("no call here"));
    assert!(selector_calls_are_literals(
        "document.querySelectorAll('input[name=\"row_version\"]');"
    ));
    assert!(selector_calls_are_literals("a.querySelector(\".tree\" );"));
}

#[test]
fn script_selector_calls_reject_a_built_argument() {
    assert!(!selector_calls_are_literals("a.querySelector(name);"));
    assert!(!selector_calls_are_literals(
        "a.querySelector(\"x\" + name);"
    ));
    assert!(!selector_calls_are_literals("a.querySelectorAll(`x${y}`);"));
    assert!(!selector_calls_are_literals("a.querySelector(\"x"));
    assert!(!selector_calls_are_literals("a.querySelector;"));
}

#[test]
fn static_files_table_lists_the_four_files_with_their_media_types() {
    let listed: Vec<(&str, &str)> = STATIC_FILES
        .iter()
        .map(|file| (file.path, file.media_type))
        .collect();
    assert_eq!(
        listed,
        [
            ("/static/htmx.min.js", "application/javascript"),
            ("/static/app.css", "text/css; charset=utf-8"),
            ("/static/row-version.js", "application/javascript"),
            ("/static/tree.js", "application/javascript"),
        ]
    );
    for file in &STATIC_FILES {
        assert!(!file.bytes.is_empty(), "{}", file.path);
    }
}

#[test]
fn static_files_each_embedded_font_is_woff2() {
    for font in &FONTS {
        assert!(font.bytes.starts_with(b"wOF2"), "{}", font.path);
    }
}
