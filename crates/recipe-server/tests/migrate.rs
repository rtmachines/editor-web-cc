#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Runs the built binary `recipe-server migrate` against a PostgreSQL container (task 11.2).
//!
//! The container and the runs of the binary come from `tests/support/`. `recipe-server` must not
//! depend on `sqlx`, so the test reads `_sqlx_migrations` with the `psql` of the container.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

mod support;

use support::cli::{
    CHILD_LIMIT, OUTPUT_LIMIT, lines_with, read_capped, run_bounded, run_migrate,
    run_migrate_with_env, text_of,
};
use support::database::{
    MIGRATOR_PASSWORD, Postgres, host_port, query, start_container, wait_until_ready,
};
use support::temp::{TempDir, random_secret};

const LABEL_VALUE: &str = "recipe-server";
const MIN_SECRET_LENGTH: usize = 32;
/// `0001` to `0006` (`data-model.md` section 8).
const EXPECTED_MIGRATIONS: i64 = 8;
/// A port that no server listens on. The case with a missing secret file uses it.
const CLOSED_PORT: u16 = 1;

/// The number of rows of `_sqlx_migrations` in the database `recipe`.
async fn applied_migrations(container: &Postgres) -> i64 {
    query(container, "SELECT count(*) FROM _sqlx_migrations")
        .await
        .parse()
        .unwrap()
}

/// Write the TOML file of a run and return its path.
fn write_config(dir: &TempDir, name: &str, port: u16, secret: &Path) -> PathBuf {
    let text = format!(
        "[db]\nhost = '127.0.0.1'\nport = {port}\nname = 'recipe'\nmigrator_user = 'recipe_migrator'\n\n\
         [secrets]\nmigrator_password_file = '{}'\n",
        secret.display()
    );
    dir.write(name, &text)
}

#[test]
fn random_secrets_differ_and_are_long() {
    let first = random_secret();
    let second = random_secret();

    assert_ne!(first, second);
    assert!(first.len() >= MIN_SECRET_LENGTH, "{first}");
    assert_ne!(first, "test-superuser-password");
}

#[test]
fn temporary_directories_have_random_names_and_are_new() {
    let first = TempDir::new();
    let second = TempDir::new();

    assert_ne!(*first, *second);
    assert!(std::fs::create_dir(&*first).is_err());
}

#[cfg(unix)]
#[test]
fn temporary_directories_and_files_are_private_to_the_owner() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new();
    let file = dir.write(
        "secret", "value
",
    );

    let directory_mode = std::fs::metadata(&*dir).unwrap().permissions().mode();
    let file_mode = std::fs::metadata(&file).unwrap().permissions().mode();
    assert_eq!(directory_mode & 0o777, 0o700);
    assert_eq!(file_mode & 0o777, 0o600);
}

#[test]
fn read_capped_keeps_the_first_bytes_and_drops_the_rest() {
    let kept = read_capped(Cursor::new(b"0123456789".to_vec()), 4);

    assert_eq!(kept, b"0123");
}

#[test]
fn read_capped_keeps_a_short_input_whole() {
    let kept = read_capped(Cursor::new(b"abc".to_vec()), 4);

    assert_eq!(kept, b"abc");
}

/// A command that prints a short text and exits.
fn printing_command() -> Command {
    #[cfg(unix)]
    {
        let mut command = Command::new("echo");
        command.arg("out");
        command
    }
    #[cfg(not(unix))]
    {
        Command::new("hostname")
    }
}

/// A command that runs for about 30 seconds.
fn sleeping_command() -> Command {
    #[cfg(unix)]
    {
        let mut command = Command::new("sleep");
        command.arg("30");
        command
    }
    #[cfg(not(unix))]
    {
        let mut command = Command::new("ping");
        command.args(["-n", "30", "127.0.0.1"]);
        command
    }
}

#[test]
fn run_bounded_returns_the_output_of_a_child_that_exits() {
    let output = run_bounded(printing_command(), Duration::from_secs(20), 1024);

    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

#[test]
fn run_bounded_truncates_the_output_at_the_limit() {
    let output = run_bounded(printing_command(), Duration::from_secs(20), 2);

    assert_eq!(output.stdout.len(), 2);
}

#[test]
#[should_panic(expected = "did not exit within")]
fn run_bounded_kills_a_child_that_runs_too_long() {
    run_bounded(sleeping_command(), Duration::from_millis(300), 1024);
}

/// The published port must listen on the loopback address only.
fn assert_port_on_loopback(container: &Postgres) {
    let mut command = Command::new("docker");
    command.args(["port", container.id(), "5432/tcp"]);
    let output = run_bounded(command, CHILD_LIMIT, OUTPUT_LIMIT);
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(output.status.success(), "docker port failed: {text}");
    assert!(!text.trim().is_empty(), "docker port lists no binding");
    for line in text.lines() {
        assert!(line.starts_with("127.0.0.1:"), "not loopback only: {line}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn migrate_applies_every_migration_once_and_fails_with_one_error_line() {
    let container = start_container(LABEL_VALUE).await;
    assert_port_on_loopback(&container);
    wait_until_ready(&container).await;
    let host_port = host_port(&container).await;
    let dir = TempDir::new();
    let valid_secret = dir.write("valid_secret", &format!("{MIGRATOR_PASSWORD}\n"));
    let valid = write_config(&dir, "valid.toml", host_port, &valid_secret);

    // A valid configuration applies the eight migrations.
    let first = run_migrate(valid.clone()).await;
    assert_eq!(first.status.code(), Some(0), "{}", text_of(&first));
    assert_eq!(applied_migrations(&container).await, EXPECTED_MIGRATIONS);
    assert_eq!(lines_with(&first, "ERROR"), 0);

    // A second run changes nothing.
    let second = run_migrate(valid).await;
    assert_eq!(second.status.code(), Some(0), "{}", text_of(&second));
    assert_eq!(applied_migrations(&container).await, EXPECTED_MIGRATIONS);

    // An environment value overrides the file: the file names a closed port, the value the real one.
    // Other `RECIPE_` values are not keys. The program ignores them and does not stop.
    let closed = write_config(&dir, "closed.toml", CLOSED_PORT, &valid_secret);
    let environment = vec![
        ("RECIPE_DB__PORT", host_port.to_string()),
        ("RECIPE_UNRELATED", "1".to_owned()),
        ("RECIPE_DB__UNKNOWN", "2".to_owned()),
    ];
    let overridden = run_migrate_with_env(closed, environment).await;
    assert_eq!(
        overridden.status.code(),
        Some(0),
        "{}",
        text_of(&overridden)
    );

    // A missing secret file fails before any connection. The port is closed, so a connection
    // attempt would give another message.
    let absent = dir.join("absent_secret");
    let missing = write_config(&dir, "missing.toml", CLOSED_PORT, &absent);
    let output = run_migrate(missing).await;
    let text = text_of(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert_eq!(lines_with(&output, "ERROR"), 1, "{text}");
    assert!(text.contains("secrets.migrator_password_file"), "{text}");
    assert!(!text.contains(MIGRATOR_PASSWORD), "{text}");

    // A wrong password fails with one ERROR line and does not echo the password.
    let wrong_secret = dir.write("wrong_secret", "wrong-password-text\n");
    let wrong = write_config(&dir, "wrong.toml", host_port, &wrong_secret);
    let output = run_migrate(wrong).await;
    let text = text_of(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert_eq!(lines_with(&output, "ERROR"), 1, "{text}");
    assert!(!text.contains("wrong-password-text"), "{text}");
    assert!(!text.contains(MIGRATOR_PASSWORD), "{text}");
    assert_eq!(applied_migrations(&container).await, EXPECTED_MIGRATIONS);
}
