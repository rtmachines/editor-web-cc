#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Registry test of the error codes (`reliability.md` section 1.2, change `review-code-fixes`, design D1).
//!
//! Each module of `tests/error_registry/` gives one sample for each variant of one error type. The test
//! compares the `code()` of the samples with the rows of section 1.2. It reads the document only.
//! A fifth fact checks that each variant name in a `name_of` function has a sample.
//!
//! To add an error type to the scope, add a `mod` line below and one entry to `SCOPE`.

// The modules live in `tests/error_registry/`. A module of a test root file looks beside the root file,
// so each `mod` line names its path.
#[path = "error_registry/app.rs"]
mod app;
#[path = "error_registry/audit.rs"]
mod audit;
#[path = "error_registry/check_result.rs"]
mod check_result;
#[path = "error_registry/export.rs"]
mod export;
#[path = "error_registry/identity.rs"]
mod identity;
#[path = "error_registry/plant.rs"]
mod plant;
#[path = "error_registry/recipe.rs"]
mod recipe;
#[path = "error_registry/rows.rs"]
mod rows;
#[path = "error_registry/sample.rs"]
mod sample;
#[path = "error_registry/storage.rs"]
mod storage;
#[path = "error_registry/web.rs"]
mod web;

use std::collections::BTreeSet;

use recipe_domain::error_codes::REGISTRY;
use rows::{Row, rows_of};
use sample::Sample;

const RELIABILITY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/architecture/reliability.md"
);

/// The function of a module that returns its samples.
type SampleFn = fn() -> Vec<Sample>;

/// The types that an owner cell names and that the scope leaves out (design D1, fact 4).
///
/// `SnapshotItem::UnknownUnit` returns `E-PLT-020`, like `PlantError::UnitNotFound`.
const EXCLUDED: &[&str] = &["SnapshotItem"];

/// One error type in scope: the name in the owner cells of section 1.2, the function of its samples,
/// and the source text of its module. The fifth fact reads the source text.
type ScopeEntry = (&'static str, SampleFn, &'static str);

/// The error types in scope.
const SCOPE: &[ScopeEntry] = &[
    (
        "AuditError",
        audit::samples,
        include_str!("error_registry/audit.rs"),
    ),
    (
        "StorageError",
        storage::samples,
        include_str!("error_registry/storage.rs"),
    ),
    (
        "ExportError",
        export::samples,
        include_str!("error_registry/export.rs"),
    ),
    (
        "IdentityError",
        identity::samples,
        include_str!("error_registry/identity.rs"),
    ),
    (
        "PlantError",
        plant::samples,
        include_str!("error_registry/plant.rs"),
    ),
    (
        "RecipeError",
        recipe::samples,
        include_str!("error_registry/recipe.rs"),
    ),
    (
        "AppError",
        app::samples,
        include_str!("error_registry/app.rs"),
    ),
    (
        "CheckResultFault",
        check_result::samples,
        include_str!("error_registry/check_result.rs"),
    ),
    (
        "WebError",
        web::samples,
        include_str!("error_registry/web.rs"),
    ),
];

fn documented_rows() -> Vec<Row> {
    let document = std::fs::read_to_string(RELIABILITY).unwrap();
    let rows = rows_of(&document);
    assert!(rows.len() >= 100, "{}", rows.len());
    rows
}

fn all_samples() -> Vec<Sample> {
    SCOPE
        .iter()
        .flat_map(|(_type_name, samples, _source)| samples())
        .collect()
}

/// Return the name of the type in a name `Type::Variant`.
fn type_of(variant: &str) -> &str {
    variant
        .split_once("::")
        .map_or(variant, |(type_name, _variant)| type_name)
}

fn is_in_scope(variant: &str) -> bool {
    SCOPE
        .iter()
        .any(|(type_name, _samples, _source)| *type_name == type_of(variant))
}

#[test]
fn error_registry_every_sample_code_is_in_the_registry() {
    for sample in all_samples() {
        assert!(
            REGISTRY.contains(&sample.code),
            "{} returns {}, which is not in REGISTRY",
            sample.name,
            sample.code
        );
    }
}

#[test]
fn error_registry_every_sample_code_belongs_to_a_row_that_names_its_variant() {
    let rows = documented_rows();
    for sample in all_samples() {
        let names_it = rows.iter().any(|row| {
            row.code == sample.code.as_str() && row.variants.iter().any(|name| name == sample.name)
        });
        assert!(
            names_it,
            "{} returns {}, but no row of reliability.md section 1.2 has this code and names this variant",
            sample.name, sample.code
        );
    }
}

#[test]
fn error_registry_every_pair_of_row_and_variant_has_a_sample() {
    let samples: BTreeSet<(&str, &str)> = all_samples()
        .iter()
        .map(|sample| (sample.code.as_str(), sample.name))
        .collect();
    for row in documented_rows() {
        for name in row.variants.iter().filter(|name| is_in_scope(name)) {
            assert!(
                samples.contains(&(row.code.as_str(), name.as_str())),
                "the row {} names {name}, but no sample of that variant returns {}",
                row.code,
                row.code
            );
        }
    }
}

#[test]
fn error_registry_scope_holds_every_type_of_an_owner_cell() {
    let rows = documented_rows();
    let types: BTreeSet<&str> = rows
        .iter()
        .flat_map(|row| row.variants.iter())
        .map(|name| type_of(name))
        .collect();
    for type_name in types {
        assert!(
            is_in_scope(type_name) || EXCLUDED.contains(&type_name),
            "an owner cell of reliability.md section 1.2 names {type_name}, which is neither in SCOPE nor in EXCLUDED"
        );
    }
}

/// Return the names `Type::Variant` that the `name_of` function in a module source returns.
///
/// The function ends at the first line that holds only a closing brace. A delegate arm has no name.
fn named_variants(type_name: &str, source: &str) -> Vec<String> {
    let body = source
        .split_once("fn name_of")
        .and_then(|(_before, rest)| rest.split_once("\n}\n"))
        .map(|(body, _after)| body)
        .unwrap();
    let prefix = format!("{type_name}::");
    body.split('"')
        .skip(1)
        .step_by(2)
        .filter(|literal| literal.starts_with(&prefix))
        .map(str::to_owned)
        .collect()
}

#[test]
fn error_registry_has_a_sample_of_each_named_variant() {
    for (type_name, samples, source) in SCOPE {
        let named = named_variants(type_name, source);
        assert!(!named.is_empty(), "name_of of {type_name} names no variant");
        let sampled: BTreeSet<&str> = samples().iter().map(|sample| sample.name).collect();
        for name in &named {
            assert!(
                sampled.contains(name.as_str()),
                "name_of of {type_name} returns {name}, but samples() has no sample of it"
            );
        }
    }
}
