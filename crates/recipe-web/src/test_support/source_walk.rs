//! The walk of the source files and the test-path rule, shared by the guard tests.
//!
//! Several guard tests read the files of the crate. Each test needs the same two items: a list
//! of the files below a folder, and a rule that tells test code from production code. One copy of
//! each item keeps the guards equal.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Returns true when the path has a part with a test name.
///
/// Pass a path relative to the crate or to a folder of it. Then a folder above the crate does
/// not count.
///
/// A part has a test name when its stem is `test` or `tests`, ends with `_tests`, or starts
/// with `test_`. The suffix `.rs` is not part of the stem.
pub(crate) fn is_test_path(relative: &Path) -> bool {
    relative.components().any(|part| {
        let name = part.as_os_str().to_string_lossy();
        let stem = name.strip_suffix(".rs").unwrap_or(&name);
        stem == "test" || stem == "tests" || stem.ends_with("_tests") || stem.starts_with("test_")
    })
}

/// Lists every file below a folder, also in the sub-folders. The order is not stable.
///
/// # Errors
///
/// Returns the I/O error of the first folder or entry that the walk cannot read.
pub(crate) fn list_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            found.extend(list_files(&path)?);
        } else {
            found.push(path);
        }
    }
    Ok(found)
}

/// Lists every file below a folder in sorted order. When the walk cannot read a folder, the test
/// fails.
pub(crate) fn files_below(dir: &Path) -> Vec<PathBuf> {
    let mut files = list_files(dir).unwrap();
    files.sort();
    files
}

#[test]
fn files_below_lists_nested_files_in_sorted_order() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("templates");

    let files = files_below(&root);

    let mut sorted = files.clone();
    sorted.sort();
    assert_eq!(files, sorted);
    assert!(files.iter().any(|path| path.ends_with("base.html")));
    assert!(
        files
            .iter()
            .any(|path| path.ends_with("test/danger-action.html"))
    );
    assert!(files.iter().all(|path| path.is_file()));
}

#[test]
fn list_files_returns_the_error_of_a_missing_folder() {
    let missing = Path::new(env!("CARGO_MANIFEST_DIR")).join("no-such-folder");

    let error = list_files(&missing).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}
