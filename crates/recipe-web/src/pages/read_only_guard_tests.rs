//! Source guard for the read-only mode (ADR-0113 (a), design D5).
//!
//! The guard solves one problem: a page that asks the permission matrix directly shows a form in
//! read-only mode. Every page must ask `PageRequest::may_run`, so the guard lists each source
//! file under `src/pages/` that still holds a bare `requirement().allows(` chain.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The chain that a page must not write outside `request.rs`, with all whitespace removed.
const FORBIDDEN_CHAIN: &str = "requirement().allows(";

/// Return the number of bare permission chains in `source`, ignoring all whitespace.
fn count_bare_chains(source: &str) -> usize {
    let squeezed: String = source.chars().filter(|c| !c.is_whitespace()).collect();
    squeezed.matches(FORBIDDEN_CHAIN).count()
}

/// Return true when `name` is the file name of a test file.
fn is_test_file_name(name: &str) -> bool {
    name.ends_with("_tests.rs") || name == "tests.rs" || name.starts_with("test_")
}

/// Collect every Rust source under `dir` that the guard must read.
fn collect_sources(dir: &Path, found: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_sources(&path, found)?;
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let is_rust = path.extension().is_some_and(|extension| extension == "rs");
        let is_request =
            name == "request.rs" && path.parent().is_some_and(|p| p.ends_with("pages"));
        if is_rust && !is_request && !is_test_file_name(name) {
            found.push(path);
        }
    }
    Ok(())
}

#[test]
fn search_finds_a_chain_on_one_line() {
    let source = "let ok = Operation::AddUnit.requirement().allows(role);";
    assert_eq!(count_bare_chains(source), 1);
}

#[test]
fn search_finds_a_chain_split_over_two_lines() {
    let source = "Operation::ListSecurityEvents\n    .requirement()\n    .allows(role)\n";
    assert_eq!(count_bare_chains(source), 1);
}

#[test]
fn search_ignores_the_may_run_call() {
    assert_eq!(count_bare_chains("request.may_run(Operation::AddUnit)?"), 0);
}

#[test]
fn read_only_guard_pages_use_may_run_only() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("pages");
    let mut sources = Vec::new();
    collect_sources(&root, &mut sources).unwrap();
    sources.sort();
    let mut offenders = Vec::new();
    for path in &sources {
        let text = fs::read_to_string(path).unwrap();
        let count = count_bare_chains(&text);
        if count > 0 {
            let shown = path.strip_prefix(&root).unwrap_or(path);
            offenders.push(format!("{} ({count})", shown.display()));
        }
    }
    assert!(
        offenders.is_empty(),
        "bare `requirement().allows(` outside pages/request.rs, use PageRequest::may_run:\n{}",
        offenders.join("\n")
    );
}
