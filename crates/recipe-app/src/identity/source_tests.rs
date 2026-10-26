//! Source test of ADR-0013: the permission check is the first statement of every identity command.
//!
//! The test reads the source of each command file. The body of each `execute` function must start
//! with `authorize(Self::OPERATION`. The command `Login` has no session, so it starts with
//! `require_no_session(Self::OPERATION`. This rule keeps a command from reading the user or a port
//! before it checks the permission (ADR-0013, ADR-0035 (b)).

use std::ffi::OsStr;
use std::path::Path;

/// The text that the first statement of a command with a session must start with.
const AUTHORIZE: &str = "authorize(Self::OPERATION";

/// The text that the first statement of the command without a session must start with.
const NO_SESSION: &str = "require_no_session(Self::OPERATION";

/// The command files, their text, and the text that starts each `execute` body.
/// A new command file needs a row here.
/// The test `every_file_with_an_execute_function_is_listed` fails without it.
/// The files `authenticate.rs` and `bootstrap.rs` have no `execute` function.
/// The test files and the directory `queries/` are not command files.
const SOURCES: [(&str, &str, &str); 9] = [
    (
        "activate_user.rs",
        include_str!("activate_user.rs"),
        AUTHORIZE,
    ),
    (
        "change_own_password.rs",
        include_str!("change_own_password.rs"),
        AUTHORIZE,
    ),
    ("create_user.rs", include_str!("create_user.rs"), AUTHORIZE),
    (
        "deactivate_user.rs",
        include_str!("deactivate_user.rs"),
        AUTHORIZE,
    ),
    ("login.rs", include_str!("login.rs"), NO_SESSION),
    ("logout.rs", include_str!("logout.rs"), AUTHORIZE),
    (
        "reset_password.rs",
        include_str!("reset_password.rs"),
        AUTHORIZE,
    ),
    (
        "set_user_role.rs",
        include_str!("set_user_role.rs"),
        AUTHORIZE,
    ),
    (
        "unlock_login.rs",
        include_str!("unlock_login.rs"),
        AUTHORIZE,
    ),
];

/// The text that starts the signature of the function that a command exposes.
const EXECUTE: &str = "pub async fn execute";

/// Return true when `body` starts with `first_statement`.
/// The test ignores white space, because rustfmt can put each argument on its own line.
fn starts_with(body: &str, first_statement: &str) -> bool {
    let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    compact.starts_with(first_statement)
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
    let mut offenders = Vec::new();
    for (name, source, first_statement) in SOURCES {
        let found = bodies(source);
        assert!(!found.is_empty(), "{name}: no execute function");
        for body in found {
            if !starts_with(body, first_statement) {
                offenders.push(format!(
                    "{name}: the first statement is {:?}",
                    body.lines().next()
                ));
            }
        }
    }
    assert!(offenders.is_empty(), "{offenders:#?}");
}

#[test]
fn a_body_that_starts_with_another_statement_is_found() {
    let late = "pub async fn execute(&self) -> Result<(), AppError> {\n\
                let user = auth.user();\n\
                authorize(Self::OPERATION, user, meta, sink).await?;\n}";
    let early = "pub async fn execute(&self) -> Result<(), AppError> {\n\
                 authorize(\n    Self::OPERATION,\n    auth.user(),\n).await?;\n}";

    assert!(!bodies(late).iter().all(|b| starts_with(b, AUTHORIZE)));
    assert!(bodies(early).iter().all(|b| starts_with(b, AUTHORIZE)));
}

#[test]
fn every_file_with_an_execute_function_is_listed() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/identity");
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
        let is_listed = SOURCES.iter().any(|(listed, _, _)| *listed == name);
        assert!(
            is_listed || !has_execute,
            "{name} has an execute function and is not in SOURCES"
        );
    }
}
