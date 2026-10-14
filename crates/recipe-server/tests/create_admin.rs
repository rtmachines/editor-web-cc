#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Runs the built binary `recipe-server create-admin` against a PostgreSQL container (task 7.2).
//!
//! The harness is the one of `tests/migrate.rs`: `postgres:16.15-bookworm` with
//! `deploy/db/initdb/01-roles.sh` and the four secret files. The duplication is accepted: no
//! shared test crate exists. The test applies the migrations with the subcommand `migrate`, runs
//! `create-admin` as `recipe_app`, and reads the result with the `psql` of the container.

use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher, RandomState};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use testcontainers::bollard::models::{HostConfig, PortBinding};
use testcontainers::core::{ExecCommand, IntoContainerPort};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, CopyTargetOptions, GenericImage, ImageExt};

const IMAGE: &str = "postgres";
const IMAGE_TAG: &str = "16.15-bookworm";
const POSTGRES_PORT: u16 = 5432;
const LABEL_KEY: &str = "org.recipe-editor.server-tests";
const LABEL_VALUE: &str = "recipe-server-create-admin";
const COMPOSE_SUBNET: &str = "172.28.0.0/16";
const INITDB_ARGS: &str = "--data-checksums --lc-collate=C --lc-ctype=C.UTF-8";
const INIT_SCRIPT_SOURCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/db/initdb/01-roles.sh"
);
const INIT_SCRIPT_TARGET: &str = "/docker-entrypoint-initdb.d/01-roles.sh";
const EXECUTABLE_MODE: u32 = 0o755;
const SECRET_MODE: u32 = 0o644;
const SECRET_WORDS: usize = 4;
const MIGRATOR_PASSWORD: &str = "test-migrator-password";
const APP_PASSWORD: &str = "test-app-password";
/// The roles with a secret file, as `(file name, password)`.
const SECRETS: [(&str, &str); 4] = [
    ("migrator_password", MIGRATOR_PASSWORD),
    ("app_password", APP_PASSWORD),
    ("backup_password", "test-backup-password"),
    ("replicator_password", "test-replicator-password"),
];
const READY_ATTEMPTS: u32 = 240;
const READY_PAUSE: Duration = Duration::from_millis(500);
const ERROR_LEVEL: &str = "\"level\":\"ERROR\"";
const INFO_LEVEL: &str = "\"level\":\"INFO\"";
const ARGON2_MARKER: &str = "$argon2id$";
/// A password that meets the policy of Assumption A23.
const GOOD_PASSWORD: &str = "Zq7-plant-recipe-ok";
/// A password of 11 characters.
const SHORT_PASSWORD: &str = "Zq7-plant-o";
const SECOND_LINE: &str = "secondlinemarker";

/// A random hexadecimal secret of 64 characters.
fn random_secret() -> String {
    (0..SECRET_WORDS)
        .map(|_| format!("{:016x}", RandomState::new().build_hasher().finish()))
        .collect()
}

/// A temporary directory that the drop removes.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "recipe-server-create-admin-{}-{}",
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

/// Publish the PostgreSQL port on 127.0.0.1 only, on a free host port.
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
        "the create-admin test needs Docker: cannot start the PostgreSQL container. Start Docker and run the test again",
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

/// Run one SQL query as the superuser in the database `recipe` and return its text.
async fn query(container: &ContainerAsync<GenericImage>, sql: &str) -> String {
    let (code, text) = exec(
        container,
        &["psql", "-U", "postgres", "-d", "recipe", "-tAc", sql],
    )
    .await;
    assert_eq!(code, 0, "psql failed: {text}");
    text
}

/// The TOML file of a run. It names the migrator and the app secret files.
fn write_config(dir: &TempDir, port: u16, migrator: &Path, app: &Path) -> PathBuf {
    let text = format!(
        "[db]\nhost = '127.0.0.1'\nport = {port}\nname = 'recipe'\n\n\
         [secrets]\nmigrator_password_file = '{}'\napp_password_file = '{}'\n",
        migrator.display(),
        app.display()
    );
    dir.write("recipe.toml", &text)
}

fn text_of(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn lines_with(output: &Output, level: &str) -> usize {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.contains(level))
        .count()
}

/// Run `recipe-server migrate --config <path>`.
async fn run_migrate(config: PathBuf) -> Output {
    tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_recipe-server"))
            .arg("migrate")
            .arg("--config")
            .arg(config)
            .output()
            .unwrap()
    })
    .await
    .unwrap()
}

/// Run `recipe-server create-admin` and pipe `stdin_text` into it.
async fn run_create_admin(
    config: PathBuf,
    login: &'static str,
    password_file: Option<PathBuf>,
    stdin_text: String,
) -> Output {
    tokio::task::spawn_blocking(move || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_recipe-server"));
        command
            .arg("create-admin")
            .args(["--login", login])
            .arg("--config")
            .arg(config);
        if let Some(file) = password_file {
            command.arg("--password-file").arg(file);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        // The child may exit before it reads stdin, for example with a password file.
        let _ = stdin.write_all(stdin_text.as_bytes());
        drop(stdin);
        child.wait_with_output().unwrap()
    })
    .await
    .unwrap()
}

/// No run may print a password, a hash, or a line of a rejected file.
fn assert_no_secret(output: &Output) {
    let text = text_of(output);
    for needle in [
        GOOD_PASSWORD,
        SHORT_PASSWORD,
        APP_PASSWORD,
        MIGRATOR_PASSWORD,
        ARGON2_MARKER,
        SECOND_LINE,
    ] {
        assert!(!text.contains(needle), "output holds {needle}: {text}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_admin_creates_one_administrator_and_refuses_bad_passwords() {
    let container = start_container().await;
    wait_until_ready(&container).await;
    let host_port = container
        .get_host_port_ipv4(POSTGRES_PORT.tcp())
        .await
        .unwrap();
    let dir = TempDir::new();
    let migrator = dir.write("migrator_secret", &format!("{MIGRATOR_PASSWORD}\n"));
    let app = dir.write("app_secret", &format!("{APP_PASSWORD}\n"));
    let config = write_config(&dir, host_port, &migrator, &app);
    let migrated = run_migrate(config.clone()).await;
    assert_eq!(migrated.status.code(), Some(0), "{}", text_of(&migrated));

    // A password of 11 characters gives E-IDN-006, exit 1, and no user.
    let short = run_create_admin(
        config.clone(),
        "boss.one",
        None,
        format!("{SHORT_PASSWORD}\n"),
    )
    .await;
    let text = text_of(&short);
    assert_eq!(short.status.code(), Some(1), "{text}");
    assert_eq!(lines_with(&short, ERROR_LEVEL), 1, "{text}");
    assert!(text.contains("E-IDN-006"), "{text}");
    assert_no_secret(&short);
    assert_eq!(
        query(&container, "SELECT count(*) FROM app_user").await,
        "0"
    );

    // A password file with two lines gives exit 1, and the output holds no line of the file.
    let two_lines = dir.write("two_lines", &format!("{GOOD_PASSWORD}\n{SECOND_LINE}\n"));
    let rejected =
        run_create_admin(config.clone(), "boss.one", Some(two_lines), String::new()).await;
    let text = text_of(&rejected);
    assert_eq!(rejected.status.code(), Some(1), "{text}");
    assert_eq!(lines_with(&rejected, ERROR_LEVEL), 1, "{text}");
    assert_no_secret(&rejected);
    assert_eq!(
        query(&container, "SELECT count(*) FROM app_user").await,
        "0"
    );

    // A first run with a valid password on stdin creates the Administrator.
    let first = run_create_admin(
        config.clone(),
        "boss.one",
        None,
        format!("{GOOD_PASSWORD}\n"),
    )
    .await;
    let text = text_of(&first);
    assert_eq!(first.status.code(), Some(0), "{text}");
    assert_eq!(lines_with(&first, INFO_LEVEL), 1, "{text}");
    assert_eq!(lines_with(&first, ERROR_LEVEL), 0, "{text}");
    assert!(text.contains("boss.one"), "{text}");
    assert_no_secret(&first);
    assert_eq!(
        query(
            &container,
            "SELECT login || ',' || role || ',' || must_change_password FROM app_user"
        )
        .await,
        "boss.one,administrator,true"
    );
    assert_eq!(
        query(
            &container,
            "SELECT count(*) || ',' || count(actor_user_id) FROM audit_log WHERE action = 'UserCreated'"
        )
        .await,
        "1,0"
    );

    // A second run with another login writes one INFO line, exits 0, and creates nothing.
    let second = run_create_admin(config, "boss.two", None, format!("{GOOD_PASSWORD}\n")).await;
    let text = text_of(&second);
    assert_eq!(second.status.code(), Some(0), "{text}");
    assert_eq!(lines_with(&second, INFO_LEVEL), 1, "{text}");
    assert_eq!(lines_with(&second, ERROR_LEVEL), 0, "{text}");
    assert_no_secret(&second);
    assert_eq!(
        query(&container, "SELECT count(*) FROM app_user").await,
        "1"
    );
    assert_eq!(
        query(
            &container,
            "SELECT count(*) FROM audit_log WHERE action = 'UserCreated'"
        )
        .await,
        "1"
    );
}
