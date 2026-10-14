// reliability.md section 2.3: test code may use `unwrap`, `expect`, and indexing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

//! Architecture rules of `recipe-web` (design D17, ADR-0013).
//!
//! Only `recipe-server` may call `bootstrap_administrator`. The function has no `Operation`,
//! so a route in `recipe-web` that called it would create an Administrator without a permission check.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_NAME: &str = "bootstrap_administrator";

/// Return every Rust file below `dir`.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(rust_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
    found
}

/// Return every Rust file below `dir` that contains `needle`.
fn files_naming(dir: &Path, needle: &str) -> Vec<PathBuf> {
    rust_files(dir)
        .into_iter()
        .filter(|path| fs::read_to_string(path).unwrap().contains(needle))
        .collect()
}

/// Return true when `source` holds `word` and the character before it is not part of a name.
fn uses_word(source: &str, word: &str) -> bool {
    source.match_indices(word).any(|(at, _)| {
        source
            .split_at(at)
            .0
            .chars()
            .next_back()
            .is_none_or(|before| !(before.is_alphanumeric() || before == '_'))
    })
}

/// Return every Rust file below `dir` that uses one of `words` as a word of its own.
fn files_using_words(dir: &Path, words: &[&str]) -> Vec<PathBuf> {
    rust_files(dir)
        .into_iter()
        .filter(|path| {
            let source = fs::read_to_string(path).unwrap();
            words.iter().any(|word| uses_word(&source, word))
        })
        .collect()
}

#[test]
fn architecture_sources_do_not_name_the_bootstrap_function() {
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let found = files_naming(&sources, FORBIDDEN_NAME);
    assert!(found.is_empty(), "{FORBIDDEN_NAME} appears in {found:?}");
}

/// The words that switch off the escape of askama (`design-system.md` section 12). They are built
/// with `concat!`, so this file never holds one of them in a single literal.
const ESCAPE_BYPASS_WORDS: [&str; 5] = [
    concat!("escape", " ="),
    concat!("ext", " ="),
    concat!("Sa", "fe("),
    concat!("Html", "Safe"),
    concat!("Maybe", "Safe"),
];

#[test]
fn architecture_sources_do_not_switch_off_the_template_escape() {
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let found = files_using_words(&sources, &ESCAPE_BYPASS_WORDS);
    assert!(found.is_empty(), "escape bypass in {found:?}");
}

#[test]
fn architecture_word_scan_finds_each_planted_word_and_ignores_longer_names() {
    let root = std::env::temp_dir().join(format!("recipe-web-words-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for (index, word) in ESCAPE_BYPASS_WORDS.iter().enumerate() {
        fs::write(
            root.join(format!("planted{index}.rs")),
            format!("#[template(path = \"a.html\", {word} \"none\")]\nstruct A;"),
        )
        .unwrap();
    }
    fs::write(
        root.join("clean.rs"),
        "let next = 1; let text = 2; let context = 3; fn is_safe() {} let escaped = 4;",
    )
    .unwrap();
    let found = files_using_words(&root, &ESCAPE_BYPASS_WORDS);
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(found.len(), ESCAPE_BYPASS_WORDS.len(), "{found:?}");
    assert!(found.iter().all(|path| !path.ends_with("clean.rs")));
}

#[test]
fn architecture_scan_finds_a_planted_name() {
    let root = std::env::temp_dir().join(format!("recipe-web-scan-{}", std::process::id()));
    let nested = root.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join("planted.rs"),
        format!("fn x() {{ {FORBIDDEN_NAME}(); }}"),
    )
    .unwrap();
    fs::write(root.join("clean.rs"), "fn y() {}").unwrap();
    let found = files_naming(&root, FORBIDDEN_NAME);
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].ends_with("planted.rs"));
}
