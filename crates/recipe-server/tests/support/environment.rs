//! The host variables that a child process of a test must not inherit.
//!
//! A test binary includes this file by `#[path]` when it starts a child process and does not
//! include the module `support`. The file needs nothing but the standard library.

use std::process::Command;

/// The prefixes of the host variables that the child must not inherit.
const FOREIGN_PREFIXES: [&str; 2] = ["PG", "RECIPE_"];

/// Compare the start of `name` with `prefix` and ignore the case of ASCII letters.
///
/// Windows keeps a name such as `pgHost` in the environment of a child process. A name that is
/// shorter than the prefix, or that cuts a character in two, does not match.
pub fn starts_with_ignore_case(name: &str, prefix: &str) -> bool {
    name.get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

/// Remove every host variable with the prefix `PG` or `RECIPE_` from the environment of `command`.
///
/// The host environment must not change a test. Each test sets only its own values. The function
/// leaves `RUST_LOG` alone, so each caller decides about it.
pub fn remove_foreign_variables(command: &mut Command) -> &mut Command {
    for (name, _value) in std::env::vars_os() {
        let name_text = name.to_string_lossy();
        if FOREIGN_PREFIXES
            .iter()
            .any(|prefix| starts_with_ignore_case(&name_text, prefix))
        {
            command.env_remove(&name);
        }
    }
    command
}
