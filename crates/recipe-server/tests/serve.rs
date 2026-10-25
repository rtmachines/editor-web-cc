#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Runs the built binary `recipe-server serve` as a child process (design D14, row T22).
//!
//! The database is a PostgreSQL container from `tests/support/`. The helper sets the two port keys
//! to ports that it finds free just before the start, so no test uses a fixed port. Every wait has
//! a limit, and the helper kills a child that hangs.

use std::path::{Path, PathBuf};
use std::time::Duration;

#[path = "support/net.rs"]
mod net;
#[path = "support/process.rs"]
mod process;
mod support;

use net::{Answer, free_ports, http_get, poll_until, port_is_open};
use process::{ServerProcess, has_line};
use support::cli::{lines_with, run_migrate, text_of};
use support::database::{
    MIGRATOR_PASSWORD, Postgres, host_port, query, start_container, wait_until_ready,
};
use support::temp::{TempDir, random_secret};

const LABEL_VALUE: &str = "recipe-server-serve";
/// The time that a start may take, from the spawn to the answer of `/readyz`.
const START_LIMIT: Duration = Duration::from_secs(90);
/// The time that a log line may take to arrive after the event that causes it.
const LOG_LIMIT: Duration = Duration::from_secs(10);
/// The time that a process may take to exit.
const EXIT_LIMIT: Duration = Duration::from_secs(60);
/// The time that the server may take to stop after `SIGTERM`.
/// It equals `app.stop_grace_period` in `reliability.md` 7.11. Docker sends `SIGKILL` after it.
#[cfg(unix)]
const STOP_LIMIT: Duration = Duration::from_secs(40);
/// A port that no server listens on. The cases that fail before the database use it.
const CLOSED_PORT: u16 = 1;
/// The number of keys that `CompareSettings` records (ADR-0062 (a)).
const SETTING_COUNT: u32 = 18;
/// The count after a third start changed one setting.
const SETTING_COUNT_AFTER_CHANGE: u32 = 19;
/// A log line of the level `ERROR`.
const ERROR_MARKER: &str = "\"level\":\"ERROR\"";
const SETTING_CHANGED: &str = "SELECT count(*) FROM audit_log WHERE action = 'SettingChanged'";
const AUDIT_ENTRIES: &str = "SELECT count(*) FROM audit_log";
const STARTED: &str = "the server started";
#[cfg(unix)]
const STOPPED: &str = "the server stopped";
const COMMAND_FAILED: &str = "the command failed";
const KEY_MARKER: &str = "keymarker-not-a-hex-key-0123456789";

/// The files of one run. The ports come with the started server.
struct Setup {
    dir: TempDir,
    migrator_secret: PathBuf,
    app_secret: PathBuf,
    login_key: PathBuf,
    csrf_key: PathBuf,
}

impl Setup {
    fn new() -> Self {
        let dir = TempDir::new();
        let migrator_secret = dir.write("migrator_secret", &format!("{MIGRATOR_PASSWORD}\n"));
        let app_secret = dir.write("app_secret", "test-app-password\n");
        let login_key = dir.write("login_key", &format!("{}\n", random_secret()));
        let csrf_key = dir.write("csrf_key", &format!("{}\n", random_secret()));
        Self {
            dir,
            migrator_secret,
            app_secret,
            login_key,
            csrf_key,
        }
    }

    /// The environment of a start against the database on `database_port`.
    /// It holds no port key. The start adds both.
    fn environment(&self, database_port: u16) -> Vec<(&'static str, String)> {
        let text = |path: &Path| path.to_str().unwrap().to_owned();
        vec![
            ("RECIPE_DB__HOST", "127.0.0.1".to_owned()),
            ("RECIPE_DB__PORT", database_port.to_string()),
            ("RECIPE_SECRETS__APP_PASSWORD_FILE", text(&self.app_secret)),
            (
                "RECIPE_SECURITY__LOGIN_HMAC_KEY_FILE",
                text(&self.login_key),
            ),
            (
                "RECIPE_SECURITY__CSRF_SIGNING_KEY_FILE",
                text(&self.csrf_key),
            ),
        ]
    }

    /// The same environment, with the values of `overrides` in place of the values of the same names.
    fn environment_with(
        &self,
        database_port: u16,
        overrides: &[(&'static str, String)],
    ) -> Vec<(&'static str, String)> {
        let mut environment = self.environment(database_port);
        environment.retain(|(name, _)| overrides.iter().all(|(other, _)| other != name));
        environment.extend(overrides.iter().cloned());
        environment
    }
}

/// Start the server with `environment`. The helper picks two new free ports for each start.
fn start_server(environment: &[(&str, String)]) -> ServerProcess {
    ServerProcess::spawn_on_free_ports(environment, START_LIMIT)
}

/// Start the server again with the ports of `previous`. The ports are settings (ADR-0062 (a)),
/// so a start with other ports would record two changes.
fn restart_server(previous: &ServerProcess, environment: &[(&str, String)]) -> ServerProcess {
    ServerProcess::spawn_on_ports(
        environment,
        previous.public_port(),
        previous.internal_port(),
    )
}

/// A PostgreSQL container with all migrations applied.
struct Database {
    container: Postgres,
    port: u16,
}

impl Database {
    async fn start(setup: &Setup) -> Self {
        let container = start_container(LABEL_VALUE).await;
        wait_until_ready(&container).await;
        let port = host_port(&container).await;
        let config = setup.dir.write(
            "migrate.toml",
            &format!(
                "[db]\nhost = '127.0.0.1'\nport = {port}\nname = 'recipe'\n\n\
                 [secrets]\nmigrator_password_file = '{}'\n",
                setup.migrator_secret.display()
            ),
        );
        let migrated = run_migrate(config).await;
        assert_eq!(migrated.status.code(), Some(0), "{}", text_of(&migrated));
        Self { container, port }
    }

    /// The number that a query with `SELECT count(*)` returns.
    async fn count(&self, sql: &str) -> u32 {
        query(&self.container, sql).await.parse().unwrap()
    }
}

/// Call `probe` until it returns a value or the process exits. Fail the test with the log otherwise.
fn wait_for<T>(server: &mut ServerProcess, what: &str, mut probe: impl FnMut() -> Option<T>) -> T {
    let mut found = None;
    let _ = poll_until(START_LIMIT, || {
        found = probe();
        (found.is_some() || server.exited()).then_some(())
    });
    assert!(found.is_some(), "{what} did not happen:\n{}", server.log());
    found.unwrap()
}

/// Wait until `/readyz` answers 200 on the internal port. A start that passed step 10 does this.
fn wait_until_ready_answer(server: &mut ServerProcess) -> Answer {
    let internal_port = server.internal_port();
    wait_for(server, "the answer 200 of /readyz", || {
        let answer = http_get(internal_port, "/readyz")?;
        (answer.status == 200).then_some(answer)
    })
}

#[test]
fn free_ports_are_two_different_ports_that_nothing_listens_on() {
    let (public_port, internal_port) = free_ports();

    assert_ne!(public_port, internal_port);
    for port in [public_port, internal_port] {
        assert!(!port_is_open(port), "{port}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn healthz_answers_200_ok_on_the_internal_listener_after_the_start() {
    let setup = Setup::new();
    let database = Database::start(&setup).await;
    let mut server = start_server(&setup.environment(database.port));
    let internal_port = server.internal_port();

    let answer = wait_for(&mut server, "the answer of /healthz", || {
        http_get(internal_port, "/healthz")
    });

    assert_eq!(answer.status, 200, "{}", server.log());
    assert!(answer.body.contains("ok"), "{}", answer.body);
    assert!(!server.log().contains(ERROR_MARKER), "{}", server.log());
}

/// `/readyz` answers 503 `not ready` until the public listener is bound, and 200 after that
/// (ADR-0091 (e)). The window of the 503 spans the steps 9 and 10 of the start, a few
/// milliseconds. A test cannot observe it without a race, so it checks the other side: every
/// answer before the 200 is a 503, and the public port accepts a connection at the 200.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readyz_answers_200_only_when_the_public_listener_accepts_connections() {
    let setup = Setup::new();
    let database = Database::start(&setup).await;
    let mut server = start_server(&setup.environment(database.port));
    let (public_port, internal_port) = (server.public_port(), server.internal_port());

    let answer = wait_for(&mut server, "the answer 200 of /readyz", || {
        let answer = http_get(internal_port, "/readyz")?;
        if answer.status != 200 {
            assert_eq!(answer.status, 503, "{}", answer.body);
            assert!(answer.body.contains("not ready"), "{}", answer.body);
            return None;
        }
        Some(answer)
    });

    assert!(answer.body.contains("ok"), "{}", answer.body);
    assert!(port_is_open(public_port), "{}", server.log());
    // The server opens the gate before it writes the line, and the reader thread adds a delay.
    let started = poll_until(LOG_LIMIT, || {
        has_line(server.stdout_lines(), "INFO", STARTED).then_some(())
    });
    assert!(
        started.is_some(),
        "the line {STARTED:?} did not appear:\n{}",
        server.log()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_start_with_a_wrong_role_exits_with_code_1_and_one_error_line_and_opens_no_listener() {
    let setup = Setup::new();
    let database = Database::start(&setup).await;
    // The migrator owns the tables, so the role check of step 5 refuses it.
    let environment = setup.environment_with(
        database.port,
        &[
            ("RECIPE_DB__APP_USER", "recipe_migrator".to_owned()),
            (
                "RECIPE_SECRETS__APP_PASSWORD_FILE",
                setup.migrator_secret.to_str().unwrap().to_owned(),
            ),
        ],
    );
    let mut server = start_server(&environment);

    let output = server.wait_exit(EXIT_LIMIT);

    let text = text_of(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert_eq!(lines_with(&output, "ERROR"), 1, "{text}");
    assert!(text.contains(COMMAND_FAILED), "{text}");
    assert!(
        text.contains("the role of the connection is not allowed"),
        "{text}"
    );
    assert!(!text.contains(MIGRATOR_PASSWORD), "{text}");
    assert!(!has_line(server.stdout_lines(), "INFO", STARTED), "{text}");
    assert!(!port_is_open(server.internal_port()));
    assert!(!port_is_open(server.public_port()));
    // Step 7 writes the first audit entries. A refused role never reaches it.
    assert_eq!(database.count(AUDIT_ENTRIES).await, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_first_start_writes_the_setting_entries_and_an_equal_start_writes_none() {
    let setup = Setup::new();
    let database = Database::start(&setup).await;
    assert_eq!(database.count(SETTING_CHANGED).await, 0);

    // The first start records all 18 keys.
    let mut first = start_server(&setup.environment(database.port));
    wait_until_ready_answer(&mut first);
    assert_eq!(database.count(SETTING_CHANGED).await, SETTING_COUNT);
    assert_eq!(database.count(AUDIT_ENTRIES).await, SETTING_COUNT);
    first.kill();

    // A second start with the same settings changes nothing.
    let mut second = restart_server(&first, &setup.environment(database.port));
    wait_until_ready_answer(&mut second);
    assert_eq!(database.count(SETTING_CHANGED).await, SETTING_COUNT);
    assert_eq!(database.count(AUDIT_ENTRIES).await, SETTING_COUNT);
    second.kill();

    // A start with one changed setting writes one entry. The test thus sees a write when it happens.
    let changed = setup.environment_with(
        database.port,
        &[("RECIPE_APPROVAL__SEPARATION_OF_DUTIES", "false".to_owned())],
    );
    let mut third = restart_server(&first, &changed);
    wait_until_ready_answer(&mut third);
    assert_eq!(
        database.count(SETTING_CHANGED).await,
        SETTING_COUNT_AFTER_CHANGE
    );
}

#[test]
fn a_key_file_fault_ends_the_start_with_code_1_and_one_error_line_that_holds_no_key() {
    let setup = Setup::new();
    let missing = setup.dir.join("absent_key");
    let unreadable = setup.dir.write("bad_key", &format!("{KEY_MARKER}\n"));
    let cases = [
        (
            "RECIPE_SECURITY__LOGIN_HMAC_KEY_FILE",
            missing,
            "security.login_hmac_key_file",
        ),
        (
            "RECIPE_SECURITY__CSRF_SIGNING_KEY_FILE",
            unreadable,
            "security.csrf_signing_key_file",
        ),
    ];

    for (variable, file, key) in cases {
        let environment = setup.environment_with(
            CLOSED_PORT,
            &[(variable, file.to_str().unwrap().to_owned())],
        );
        let mut server = start_server(&environment);

        let output = server.wait_exit(EXIT_LIMIT);

        let text = text_of(&output);
        assert_eq!(output.status.code(), Some(1), "{key}: {text}");
        assert_eq!(lines_with(&output, "ERROR"), 1, "{key}: {text}");
        assert!(text.contains(COMMAND_FAILED), "{key}: {text}");
        assert!(text.contains(key), "{key}: {text}");
        assert!(!text.contains(KEY_MARKER), "{key}: {text}");
        assert!(!port_is_open(server.internal_port()), "{key}");
        assert!(!port_is_open(server.public_port()), "{key}");
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sigterm_stops_the_server_with_code_0_and_closes_both_listeners() {
    let setup = Setup::new();
    let database = Database::start(&setup).await;
    let mut server = start_server(&setup.environment(database.port));
    wait_until_ready_answer(&mut server);
    let (public_port, internal_port) = (server.public_port(), server.internal_port());
    assert!(port_is_open(internal_port));
    assert!(port_is_open(public_port));

    send_sigterm(&server);
    let output = server.wait_exit(STOP_LIMIT);

    let text = text_of(&output);
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert_eq!(lines_with(&output, "ERROR"), 0, "{text}");
    assert!(has_line(server.stdout_lines(), "INFO", STOPPED), "{text}");
    assert!(!port_is_open(internal_port));
    assert!(!port_is_open(public_port));
}

/// Send `SIGTERM` to the child with the `kill` program, so the test needs no new crate.
#[cfg(unix)]
fn send_sigterm(server: &ServerProcess) {
    let status = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(server.id().to_string())
        .status()
        .unwrap();
    assert!(status.success(), "kill -TERM failed");
}
