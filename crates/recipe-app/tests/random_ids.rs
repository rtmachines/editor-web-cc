#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Allowed in test code by reliability.md section 2.3.

//! Guard of design D2: no production file of `recipe-app` names `Uuid::new_v4`.
//!
//! `Uuid::new_v4` panics when the operating system gives no random bytes. Production code calls
//! `random_uuid()`, which returns `AppError::TaskFailed` instead.

use std::fs;
use std::path::{Path, PathBuf};

/// The path that production code must not name, as a call or as a function value.
const FORBIDDEN_PATH: &str = "Uuid::new_v4";

/// Tell whether a directory entry holds test code or an in-memory adapter.
fn is_excluded(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if path.is_dir() {
        return name == "tests" || name == "memory";
    }
    name == "tests.rs" || name.ends_with("_tests.rs")
}

/// Collect the production `.rs` files under `dir`.
fn production_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if is_excluded(&path) {
            continue;
        }
        if path.is_dir() {
            production_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// Return the lines of `source` that name `Uuid::new_v4`. A line that starts with `//` does not count.
fn forbidden_lines(source: &str) -> Vec<(usize, String)> {
    source
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .filter(|(_, line)| line.contains(FORBIDDEN_PATH))
        .map(|(index, line)| (index.saturating_add(1), line.trim().to_owned()))
        .collect()
}

#[test]
fn no_production_file_of_recipe_app_calls_uuid_new_v4() {
    let source_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    production_files(&source_dir, &mut files);
    assert!(!files.is_empty(), "the scan found no source file");
    let mut calls = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file).unwrap();
        for (line, text) in forbidden_lines(&source) {
            calls.push(format!("{}:{line}: {text}", file.display()));
        }
    }
    assert!(
        calls.is_empty(),
        "production code calls Uuid::new_v4 at {} sites: {calls:#?}",
        calls.len()
    );
}

#[test]
fn random_ids_scan_skips_a_line_that_starts_with_a_comment() {
    let source =
        "/// Doc about `Uuid::new_v4(`.\n    // Uuid::new_v4()\nlet id = Uuid::new_v4();\n";
    assert_eq!(
        forbidden_lines(source),
        [(3, "let id = Uuid::new_v4();".to_owned())]
    );
}

#[test]
fn random_ids_scan_flags_uuid_new_v4_passed_as_a_function_value() {
    let source = "let ids = std::iter::repeat_with(Uuid::new_v4);\nlet id = value.unwrap_or_else(Uuid::new_v4);\n";
    assert_eq!(
        forbidden_lines(source),
        [
            (
                1,
                "let ids = std::iter::repeat_with(Uuid::new_v4);".to_owned()
            ),
            (2, "let id = value.unwrap_or_else(Uuid::new_v4);".to_owned()),
        ]
    );
}
