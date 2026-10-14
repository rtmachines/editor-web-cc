#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Runs the built binary `recipe-server migrate` against a PostgreSQL container (task 11.2).
//!
//! The test starts `postgres:16.15-bookworm` with `deploy/db/initdb/01-roles.sh` and the four
//! secret files, as the storage tests of `recipe-db` do. The duplication is accepted: no shared
//! test crate exists. `recipe-server` must not depend on `sqlx`, so the test reads
//! `_sqlx_migrations` with the `psql` of the container.

use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher, RandomState};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

use testcontainers::bollard::models::{HostConfig, PortBinding};
use testcontainers::core::{ExecCommand, IntoContainerPort};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, CopyTargetOptions, GenericImage, ImageExt};

const IMAGE: &str = "postgres";
const IMAGE_TAG: &str = "16.15-bookworm";
const POSTGRES_PORT: u16 = 5432;
const LABEL_KEY: &str = "org.recipe-editor.server-tests";
const LABEL_VALUE: &str = "recipe-server";
const COMPOSE_SUBNET: &str = "172.28.0.0/16";
const INITDB_ARGS: &str = "--data-checksums --lc-collate=C --lc-ctype=C.UTF-8";
const INIT_SCRIPT_SOURCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/db/initdb/01-roles.sh"
);
const INIT_SCRIPT_TARGET: &str = "/docker-entrypoint-initdb.d/01-roles.sh";
const EXECUTABLE_MODE: u32 = 0o755;
const SECRET_MODE: u32 = 0o644;
const MIN_SECRET_LENGTH: usize = 32;
const SECRET_WORDS: usize = 4;
const MIGRATOR_PASSWORD: &str = "test-migrator-password";
/// The roles with a secret file, as `(file name, password)`.
const SECRETS: [(&str, &str); 4] = [
    ("migrator_password", MIGRATOR_PASSWORD),
    ("app_password", "test-app-password"),
    ("backup_password", "test-backup-password"),
    ("replicator_password", "test-replicator-password"),
];
const READY_ATTEMPTS: u32 = 240;
const READY_PAUSE: Duration = Duration::from_millis(500);
const EXPECTED_MIGRATIONS: i64 = 4;
const ERROR_LEVEL: &str = "\"level\":\"ERROR\"";
/// A port that no server listens on. The case with a missing secret file uses it.
const CLOSED_PORT: u16 = 1;

/// A random hexadecimal secret of 64 characters.
///
/// `RandomState` takes its keys from the operating system, so no new crate is needed.
fn random_secret() -> String {
    (0..SECRET_WORDS)
        .map(|_| format!("{:016x}", RandomState::new().build_hasher().finish()))
        .collect()
}

/// A temporary directory that the drop removes.
struct TempDir(PathBuf);

impl TempDir {
    /// Make a directory with a random name. `create_dir` fails when the path exists (review R13).
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "recipe-server-migrate-{}-{}",
            std::process::id(),
            random_secret()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // A leftover directory in the temporary folder does no harm.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Remove the containers that earlier runs of this test left behind.
fn remove_old_containers() {
    let filter = format!("label={LABEL_KEY}={LABEL_VALUE}");
    let Ok(listing) = Command::new("docker")
        .args(["ps", "-aq", "--filter", &filter])
        .output()
    else {
        return;
    };
    let identifiers: Vec<String> = String::from_utf8_lossy(&listing.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    if identifiers.is_empty() {
        return;
    }
    let _ = Command::new("docker")
        .args(["rm", "--force", "--volumes"])
        .args(&identifiers)
        .output();
}

/// Publish the PostgreSQL port on 127.0.0.1 only, on a free host port (review S2).
fn bind_to_loopback(host_config: &mut HostConfig) {
    let binding = PortBinding {
        host_ip: Some("127.0.0.1".to_owned()),
        host_port: Some("0".to_owned()),
    };
    host_config.port_bindings = Some(HashMap::from([(
        format!("{POSTGRES_PORT}/tcp"),
        Some(vec![binding]),
    )]));
}

/// Start the container. The call fails with a clear message when Docker is not available.
async fn start_container() -> ContainerAsync<GenericImage> {
    remove_old_containers();
    // A checkout with CRLF line endings must not break the shell script.
    let script = std::fs::read_to_string(INIT_SCRIPT_SOURCE)
        .expect("deploy/db/initdb/01-roles.sh is readable")
        .replace("\r\n", "\n");
    let mut request = GenericImage::new(IMAGE, IMAGE_TAG)
        .with_exposed_port(POSTGRES_PORT.tcp())
        .with_label(LABEL_KEY, LABEL_VALUE)
        .with_env_var("POSTGRES_PASSWORD", random_secret())
        .with_env_var("POSTGRES_INITDB_ARGS", INITDB_ARGS)
        .with_env_var("COMPOSE_SUBNET", COMPOSE_SUBNET)
        .with_startup_timeout(Duration::from_secs(120))
        .with_host_config_modifier(bind_to_loopback)
        .with_copy_to(
            CopyTargetOptions::new(INIT_SCRIPT_TARGET).with_mode(EXECUTABLE_MODE),
            script.into_bytes(),
        );
    for (name, password) in SECRETS {
        request = request.with_copy_to(
            CopyTargetOptions::new(format!("/run/secrets/{name}")).with_mode(SECRET_MODE),
            format!("{password}\n").into_bytes(),
        );
    }
    request.start().await.expect(
        "the migrate test needs Docker: cannot start the PostgreSQL container. Start Docker and run the test again",
    )
}

/// Run a command in the container and return its exit code and its standard output.
async fn exec(container: &ContainerAsync<GenericImage>, command: &[&str]) -> (i64, String) {
    let mut result = container
        .exec(ExecCommand::new(command.iter().copied()))
        .await
        .unwrap();
    let output = result.stdout_to_vec().await.unwrap();
    let code = result.exit_code().await.unwrap().unwrap_or(-1);
    (code, String::from_utf8_lossy(&output).trim().to_owned())
}

/// Wait until the final server of the container accepts connections on TCP.
///
/// The init scripts run on the Unix socket only, so a TCP answer proves that they are done.
async fn wait_until_ready(container: &ContainerAsync<GenericImage>) {
    let mut ready = false;
    for _ in 0..READY_ATTEMPTS {
        let (code, _) = exec(container, &["pg_isready", "-h", "127.0.0.1", "-q"]).await;
        if code == 0 {
            ready = true;
            break;
        }
        tokio::time::sleep(READY_PAUSE).await;
    }
    assert!(ready, "the container does not accept connections");
}

/// The number of rows of `_sqlx_migrations` in the database `recipe`.
async fn applied_migrations(container: &ContainerAsync<GenericImage>) -> i64 {
    let (code, text) = exec(
        container,
        &[
            "psql",
            "-U",
            "postgres",
            "-d",
            "recipe",
            "-tAc",
            "SELECT count(*) FROM _sqlx_migrations",
        ],
    )
    .await;
    assert_eq!(code, 0, "psql failed: {text}");
    text.parse().unwrap()
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

/// Run `recipe-server migrate --config <path>` and return its output.
async fn run_migrate(config: PathBuf) -> Output {
    run_migrate_with_env(config, Vec::new()).await
}

/// Run the same command with extra environment values for the child process only.
async fn run_migrate_with_env(config: PathBuf, envs: Vec<(&'static str, String)>) -> Output {
    tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_recipe-server"))
            .arg("migrate")
            .arg("--config")
            .arg(config)
            .envs(envs)
            .output()
            .unwrap()
    })
    .await
    .unwrap()
}

fn text_of(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn error_lines(output: &Output) -> usize {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.contains(ERROR_LEVEL))
        .count()
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

    assert_ne!(first.0, second.0);
    assert!(std::fs::create_dir(&first.0).is_err());
}

/// The published port must listen on the loopback address only (review S2).
fn assert_port_on_loopback(container: &ContainerAsync<GenericImage>) {
    let output = Command::new("docker")
        .args(["port", container.id(), "5432/tcp"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(output.status.success(), "docker port failed: {text}");
    assert!(!text.trim().is_empty(), "docker port lists no binding");
    for line in text.lines() {
        assert!(line.starts_with("127.0.0.1:"), "not loopback only: {line}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn migrate_applies_four_migrations_once_and_fails_with_one_error_line() {
    let container = start_container().await;
    assert_port_on_loopback(&container);
    wait_until_ready(&container).await;
    let host_port = container
        .get_host_port_ipv4(POSTGRES_PORT.tcp())
        .await
        .unwrap();
    let dir = TempDir::new();
    let valid_secret = dir.write("valid_secret", &format!("{MIGRATOR_PASSWORD}\n"));
    let valid = write_config(&dir, "valid.toml", host_port, &valid_secret);

    // A valid configuration applies the four migrations.
    let first = run_migrate(valid.clone()).await;
    assert_eq!(first.status.code(), Some(0), "{}", text_of(&first));
    assert_eq!(applied_migrations(&container).await, EXPECTED_MIGRATIONS);
    assert_eq!(error_lines(&first), 0);

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
    let absent = dir.0.join("absent_secret");
    let missing = write_config(&dir, "missing.toml", CLOSED_PORT, &absent);
    let output = run_migrate(missing).await;
    let text = text_of(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert_eq!(error_lines(&output), 1, "{text}");
    assert!(text.contains("secrets.migrator_password_file"), "{text}");
    assert!(!text.contains(MIGRATOR_PASSWORD), "{text}");

    // A wrong password fails with one ERROR line and does not echo the password.
    let wrong_secret = dir.write("wrong_secret", "wrong-password-text\n");
    let wrong = write_config(&dir, "wrong.toml", host_port, &wrong_secret);
    let output = run_migrate(wrong).await;
    let text = text_of(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert_eq!(error_lines(&output), 1, "{text}");
    assert!(!text.contains("wrong-password-text"), "{text}");
    assert!(!text.contains(MIGRATOR_PASSWORD), "{text}");
    assert_eq!(applied_migrations(&container).await, EXPECTED_MIGRATIONS);
}
