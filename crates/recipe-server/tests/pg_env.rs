#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Runs the built binary `recipe-server` as a child process to check the start checks of
//! ADR-0069: the refusal of a `PG*` value and the connect timeout of `migrate`.
//!
//! The tests need no database. A child process gets its environment through `Command::env`,
//! because `std::env::set_var` is unsafe (edition 2024).

use std::hash::{BuildHasher, Hasher, RandomState};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

mod bounded_run;
use bounded_run::run_bounded;

const ERROR_LEVEL: &str = "\"level\":\"ERROR\"";
/// A value that would change every session of the role. It must never appear in the output.
const OPTIONS_VALUE: &str = "-c search_path=attack";
const OPTIONS_NAME: &str = "PGOPTIONS";
/// The connect timeout of `migrate` is `migrate.connect_timeout` of `reliability.md` section 7.11.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// The time that the child process may need above `CONNECT_TIMEOUT` to start and to exit.
const CONNECT_SLACK: Duration = Duration::from_secs(5);
/// The longest run of a child process. A hung child is killed after it.
const CHILD_LIMIT: Duration = Duration::from_secs(40);
const HEX_WORDS: usize = 2;

/// Compare the start of `name` with `prefix` and ignore the case of ASCII letters.
///
/// Windows keeps a name such as `pgHost` in the environment of a child process. A name that is
/// shorter than the prefix, or that cuts a character in two, does not match.
fn starts_with_ignore_case(name: &str, prefix: &str) -> bool {
    name.get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

#[test]
fn the_prefix_compare_ignores_case_and_short_names() {
    assert!(starts_with_ignore_case("PGHOST", "PG"));
    assert!(starts_with_ignore_case("pgHost", "PG"));
    assert!(starts_with_ignore_case("Recipe_Db_Host", "RECIPE_"));
    assert!(!starts_with_ignore_case("P", "PG"));
    assert!(!starts_with_ignore_case("RECIPE", "RECIPE_"));
    assert!(!starts_with_ignore_case("PATH", "PG"));
    assert!(!starts_with_ignore_case("\u{e9}\u{e9}", "PG"));
    assert!(!starts_with_ignore_case("P\u{e9}", "PG"));
}

fn binary(arguments: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_recipe-server"));
    command.args(arguments).env_remove("RUST_LOG");
    // The host environment must not change the test. Each test sets only its own values.
    for (name, _value) in std::env::vars_os() {
        let name_text = name.to_string_lossy();
        if starts_with_ignore_case(&name_text, "PG")
            || starts_with_ignore_case(&name_text, "RECIPE_")
        {
            command.env_remove(&name);
        }
    }
    command
}

fn text_of(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn error_lines(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.contains(ERROR_LEVEL))
        .map(str::to_owned)
        .collect()
}

/// Run the subcommand with `PGOPTIONS` set and check the refusal (ADR-0069 (b), (c)).
fn assert_refuses_pg_options(arguments: &[&str]) {
    let mut command = binary(arguments);
    command.env(OPTIONS_NAME, OPTIONS_VALUE);

    let output = run_bounded(command, CHILD_LIMIT);

    let text = text_of(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    let lines = error_lines(&output);
    assert_eq!(lines.len(), 1, "{text}");
    assert!(lines[0].contains(OPTIONS_NAME), "{text}");
    assert!(!text.contains("search_path"), "{text}");
    assert!(!text.contains("attack"), "{text}");
}

#[test]
fn serve_refuses_a_set_pg_variable_and_names_it_without_its_value() {
    assert_refuses_pg_options(&["serve"]);
}

#[test]
fn migrate_refuses_a_set_pg_variable_before_it_reads_the_configuration() {
    assert_refuses_pg_options(&["migrate"]);
}

#[test]
fn create_admin_refuses_a_set_pg_variable_before_it_reads_the_password() {
    assert_refuses_pg_options(&["create-admin", "--login", "admin"]);
}

/// A temporary directory that the drop removes.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let random: String = (0..HEX_WORDS)
            .map(|_| format!("{:016x}", RandomState::new().build_hasher().finish()))
            .collect();
        let path = std::env::temp_dir().join(format!(
            "recipe-server-pg-env-{}-{random}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // A leftover directory in the temporary folder does no harm.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A listener that never accepts: the handshake of TCP completes in the backlog, and no
/// PostgreSQL answer ever comes. Only the connect timeout ends the wait of the migrator.
#[test]
fn migrate_exits_with_one_error_line_when_the_database_does_not_answer_in_time() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let dir = TempDir::new();
    let secret = dir.0.join("migrator_password");
    std::fs::write(&secret, "not-a-real-secret\n").unwrap();
    let config = dir.0.join("config.toml");
    std::fs::write(
        &config,
        format!(
            "[db]\nhost = '127.0.0.1'\nport = {port}\nname = 'recipe'\n\n[secrets]\nmigrator_password_file = '{}'\n",
            secret.display()
        ),
    )
    .unwrap();
    let mut command = binary(&["migrate", "--config"]);
    command.arg(&config);
    let started = Instant::now();

    let output = run_bounded(command, CHILD_LIMIT);

    let elapsed = started.elapsed();
    let text = text_of(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    let lines = error_lines(&output);
    assert_eq!(lines.len(), 1, "{text}");
    assert!(lines[0].contains("E-DB-005"), "{text}");
    assert!(lines[0].contains("does not answer"), "{text}");
    assert!(!text.contains("not-a-real-secret"), "{text}");
    assert!(elapsed >= CONNECT_TIMEOUT, "{elapsed:?}");
    assert!(
        elapsed < CONNECT_TIMEOUT.checked_add(CONNECT_SLACK).unwrap(),
        "{elapsed:?}"
    );
    drop(listener);
}
