// reliability.md section 2.3: test code may use `unwrap`, `expect`, and indexing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

//! Architecture rules of `recipe-web` (design D17, ADR-0013).
//!
//! Only `recipe-server` may call `bootstrap_administrator`. The function has no `Operation`,
//! so a route in `recipe-web` that called it would create an Administrator without a permission check.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_NAME: &str = "bootstrap_administrator";

/// Return every Rust file below `dir` that contains `needle`.
fn files_naming(dir: &Path, needle: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(files_naming(&path, needle));
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && fs::read_to_string(&path).unwrap().contains(needle)
        {
            found.push(path);
        }
    }
    found
}

#[test]
fn architecture_sources_do_not_name_the_bootstrap_function() {
    let sources = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let found = files_naming(&sources, FORBIDDEN_NAME);
    assert!(found.is_empty(), "{FORBIDDEN_NAME} appears in {found:?}");
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
