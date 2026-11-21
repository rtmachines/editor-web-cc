//! Source test of design D4: the permission check is the first statement of every recipe query.
//!
//! The test reads the source of each query file. The body of each `execute` function must start
//! with `authorize(Self::OPERATION`. A form query starts with `authorize(input.purpose.operation()`,
//! the check of the command of its purpose (ADR-0046 (b)). This rule keeps a query from reading
//! a port before it checks the permission (ADR-0013).

use std::ffi::OsStr;
use std::path::Path;

/// The query files and their text. A new query file needs a row here.
/// The test `every_file_with_an_execute_function_is_listed` fails without it.
/// The test files (`*_tests.rs`) are not query files.
const SOURCES: [(&str, &str); 6] = [
    ("approval_preview.rs", include_str!("approval_preview.rs")),
    ("diff.rs", include_str!("diff.rs")),
    ("parameter_form.rs", include_str!("parameter_form.rs")),
    ("parameters.rs", include_str!("parameters.rs")),
    ("recipes.rs", include_str!("recipes.rs")),
    ("versions.rs", include_str!("versions.rs")),
];

/// The text that starts the signature of the function that a query exposes.
const EXECUTE: &str = "pub async fn execute";

/// The text that the first statement of each `execute` body must start with.
const FIRST_STATEMENT: &str = "authorize(Self::OPERATION";

/// The first statement of a form query: the check of the command of its purpose (ADR-0046 (b)).
const PURPOSE_STATEMENT: &str = "authorize(input.purpose.operation()";

/// Return true when `body` starts with one of the two forms of the permission check.
/// The test ignores white space, because rustfmt can put each argument on its own line.
fn starts_with_the_check(body: &str) -> bool {
    let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    compact.starts_with(FIRST_STATEMENT) || compact.starts_with(PURPOSE_STATEMENT)
}

/// Return the text of each `execute` body in `source`, from its first statement to the end of the file.
fn bodies(source: &str) -> Vec<&str> {
    source
        .split(EXECUTE)
        .skip(1)
        .filter_map(|chunk| chunk.split_once('{'))
        .map(|(_signature, body)| body.trim_start())
        .collect()
}

#[test]
fn the_body_of_each_execute_function_starts_with_the_permission_check() {
    for (name, source) in SOURCES {
        let found = bodies(source);
        assert!(!found.is_empty(), "{name}: no execute function");
        for body in found {
            assert!(
                starts_with_the_check(body),
                "{name}: the first statement is {:?}",
                body.lines().next()
            );
        }
    }
}

#[test]
fn a_body_that_starts_with_another_statement_is_found() {
    let late = "pub async fn execute(&self) -> Result<(), AppError> {\n\
                let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;\n\
                authorize(Self::OPERATION, user, meta, sink).await?;\n}";
    let early = "pub async fn execute(&self) -> Result<(), AppError> {\n\
                 authorize(Self::OPERATION, user, meta, sink).await?;\n}";

    assert!(!bodies(late).iter().all(|b| b.starts_with(FIRST_STATEMENT)));
    assert!(bodies(early).iter().all(|b| b.starts_with(FIRST_STATEMENT)));
}

#[test]
fn a_body_that_checks_the_permission_of_its_purpose_is_accepted() {
    let purpose = "pub async fn execute(&self) -> Result<(), AppError> {\n\
                   authorize(input.purpose.operation(), user, meta, sink).await?;\n}";
    let late = "pub async fn execute(&self) -> Result<(), AppError> {\n\
                let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;\n\
                authorize(input.purpose.operation(), user, meta, sink).await?;\n}";
    let other = "pub async fn execute(&self) -> Result<(), AppError> {\n\
                 authorize(Operation::GetRecipe, user, meta, sink).await?;\n}";

    assert!(bodies(purpose).iter().all(|b| starts_with_the_check(b)));
    assert!(!bodies(late).iter().all(|b| starts_with_the_check(b)));
    assert!(!bodies(other).iter().all(|b| starts_with_the_check(b)));
}

#[test]
fn every_file_with_an_execute_function_is_listed() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/recipe/queries");
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.extension() != Some(OsStr::new("rs")) {
            continue;
        }
        let name = path.file_name().and_then(OsStr::to_str).unwrap();
        if name.ends_with("_tests.rs") {
            continue;
        }
        let has_execute = std::fs::read_to_string(&path).unwrap().contains(EXECUTE);
        let is_listed = SOURCES.iter().any(|(listed, _)| *listed == name);
        assert!(
            is_listed || !has_execute,
            "{name} has an execute function and is not in SOURCES"
        );
    }
}

/// The text of the resolve of the platform types. It runs after the transaction ends (ADR-0042
/// (f)).
const RESOLVE: &str = "PlatformTypes::collect(";

/// The text that ends the transaction of a query.
const END_OF_TRANSACTION: &str = "drop(tx)";

/// Return `body` with each line cut at its first `//`, so that a comment never matches.
fn without_comments(body: &str) -> String {
    let lines: Vec<&str> = body
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .collect();
    lines.join("\n")
}

/// Return true when `drop(tx)` comes before the first resolve in the code of `body`.
fn drops_the_transaction_before_the_resolve(body: &str) -> bool {
    let code = without_comments(body);
    match (code.find(END_OF_TRANSACTION), code.find(RESOLVE)) {
        (Some(drop_at), Some(resolve_at)) => drop_at < resolve_at,
        _ => false,
    }
}

#[test]
fn each_execute_of_the_parameter_queries_drops_the_transaction_before_the_resolve() {
    let source = include_str!("parameters.rs");
    let resolving: Vec<&str> = bodies(source)
        .into_iter()
        .filter(|body| body.contains(RESOLVE))
        .collect();

    assert_eq!(
        resolving.len(),
        3,
        "parameters.rs: three execute bodies resolve"
    );
    for body in resolving {
        assert!(
            drops_the_transaction_before_the_resolve(body),
            "parameters.rs: the resolve runs before the end of the transaction: {:?}",
            body.lines().next()
        );
    }
}

#[test]
fn a_body_that_resolves_before_it_drops_the_transaction_is_found() {
    let late = "let t = PlatformTypes::collect(a);\ndrop(tx);";
    let early = "drop(tx);\nlet t = PlatformTypes::collect(a);";

    assert!(!drops_the_transaction_before_the_resolve(late));
    assert!(drops_the_transaction_before_the_resolve(early));
}

#[test]
fn a_body_with_the_drop_only_in_a_comment_is_found() {
    let commented = "// drop(tx);\nlet t = PlatformTypes::collect(a);";

    assert!(!drops_the_transaction_before_the_resolve(commented));
}
