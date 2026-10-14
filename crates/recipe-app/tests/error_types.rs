#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Allowed in test code by reliability.md section 2.3.

//! Guard of `reliability.md` section 1.1: only the listed types derive `thiserror::Error`.
//!
//! The test scans every source file of every crate. A public type with an error derive that
//! section 1.1 does not list has no `code()` and no row in section 1.2, so the test fails.

use std::fs;
use std::path::{Path, PathBuf};

/// The public error types of `reliability.md` section 1.1, with the two carriers of ADR-0027.
const ERROR_TYPES: [&str; 10] = [
    "PlantError",
    "RecipeError",
    "IdentityError",
    "ExportError",
    "RecipeErrors",
    "DraftError",
    "AppError",
    "AuditError",
    "StorageError",
    "WebError",
];

/// Collect the `.rs` files under `dir`.
fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// Tell whether the text of a `derive` attribute names the error derive.
fn derives_error(attribute: &str) -> bool {
    attribute
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
        .any(|token| token == "thiserror::Error" || token == "Error")
}

/// Return the name of a `pub enum` or `pub struct` line.
fn public_type_name(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("pub enum ")
        .or_else(|| line.strip_prefix("pub struct "))?;
    Some(
        rest.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect(),
    )
}

/// Return the public types of `source` that derive the error trait.
fn error_derives(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut attribute = String::new();
    let mut in_derive = false;
    let mut pending = false;
    for line in source.lines().map(str::trim) {
        if in_derive || line.starts_with("#[derive(") {
            attribute.push_str(line);
            in_derive = !line.contains(")]");
            if !in_derive {
                pending = pending || derives_error(&attribute);
                attribute.clear();
            }
        } else if line.starts_with("///") || line.starts_with("#[") || line.is_empty() {
            continue;
        } else {
            if pending {
                names.extend(public_type_name(line));
            }
            pending = false;
        }
    }
    names
}

#[test]
fn error_types_only_the_types_of_section_1_1_derive_thiserror() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for entry in fs::read_dir(&crates).unwrap() {
        let source_dir = entry.unwrap().path().join("src");
        if source_dir.is_dir() {
            rust_files(&source_dir, &mut files);
        }
    }
    assert!(!files.is_empty(), "the scan found no source file");
    let mut unlisted = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file).unwrap();
        for name in error_derives(&source) {
            if !ERROR_TYPES.contains(&name.as_str()) {
                unlisted.push(format!("{name} in {}", file.display()));
            }
        }
    }
    assert!(
        unlisted.is_empty(),
        "public types with an error derive outside reliability.md section 1.1: {unlisted:?}"
    );
}

#[test]
fn error_types_scan_finds_a_derive_over_several_lines() {
    let source = "/// Doc.\n#[derive(\n    Debug,\n    thiserror::Error,\n)]\npub enum Sample {}\n";
    assert_eq!(error_derives(source), ["Sample"]);
}

#[test]
fn error_types_scan_finds_the_imported_derive_and_skips_other_derives() {
    let source =
        "#[derive(Debug, Error)]\npub struct Imported;\n#[derive(Debug)]\npub struct Plain;\n";
    assert_eq!(error_derives(source), ["Imported"]);
}

#[test]
fn error_types_scan_skips_a_private_type() {
    let source = "#[derive(Debug, thiserror::Error)]\nenum Hidden {}\n";
    assert!(error_derives(source).is_empty());
}
