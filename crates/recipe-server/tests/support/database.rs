//! The PostgreSQL container of the tests that run the built binary.
//!
//! The container is `postgres:16.15-bookworm` with `deploy/db/initdb/01-roles.sh` and the four
//! secret files, as the storage tests of `recipe-db` start it. No shared test crate exists, so the
//! tests of `recipe-server` include this file. The tests cannot use `sqlx`, so they read the
//! database with the `psql` of the container.

use std::collections::HashMap;
use std::process::Command;
use std::sync::Once;
use std::time::Duration;

use testcontainers::bollard::models::{HostConfig, PortBinding};
use testcontainers::core::{ExecCommand, IntoContainerPort};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, CopyTargetOptions, GenericImage, ImageExt};

use super::cli::{CHILD_LIMIT, OUTPUT_LIMIT, run_bounded};
use super::temp::random_secret;

const IMAGE: &str = "postgres";
const IMAGE_TAG: &str = "16.15-bookworm";
const POSTGRES_PORT: u16 = 5432;
const LABEL_KEY: &str = "org.recipe-editor.server-tests";
const COMPOSE_SUBNET: &str = "172.28.0.0/16";
const INITDB_ARGS: &str = "--data-checksums --lc-collate=C --lc-ctype=C.UTF-8";
const INIT_SCRIPT_SOURCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/db/initdb/01-roles.sh"
);
const INIT_SCRIPT_TARGET: &str = "/docker-entrypoint-initdb.d/01-roles.sh";
/// The test-only init script of the storage harness of `recipe-db` (ADR-0069 (u)). The test
/// connects from the Docker host, and `01-roles.sh` admits only the Compose subnet.
const TEST_HOSTS_SOURCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../recipe-db/tests/storage/support/02-test-hosts.sh"
);
const TEST_HOSTS_TARGET: &str = "/docker-entrypoint-initdb.d/02-test-hosts.sh";
const EXECUTABLE_MODE: u32 = 0o755;
/// The container secret files stay world-readable. The container user id differs from the host
/// user id, so a mode of `0o600` would lock the PostgreSQL user out. The files hold test values.
const SECRET_MODE: u32 = 0o644;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
const READY_ATTEMPTS: u32 = 240;
const READY_PAUSE: Duration = Duration::from_millis(500);
/// The password of the migrator role.
pub const MIGRATOR_PASSWORD: &str = "test-migrator-password";
/// The password of the application role.
pub const APP_PASSWORD: &str = "test-app-password";
/// The roles with a secret file, as `(file name, password)`.
const SECRETS: [(&str, &str); 4] = [
    ("migrator_password", MIGRATOR_PASSWORD),
    ("app_password", APP_PASSWORD),
    ("backup_password", "test-backup-password"),
    ("replicator_password", "test-replicator-password"),
];

/// The container type that `start_container` returns.
pub type Postgres = ContainerAsync<GenericImage>;

/// The containers of earlier runs are removed once for each process.
///
/// Several tests of one process start containers at the same time. A second removal would stop
/// the container of a test that already runs.
///
/// The `Once` works for one process per test binary. A runner that starts several processes of
/// one binary, such as `cargo nextest`, can remove the containers of a concurrent run.
static OLD_CONTAINERS: Once = Once::new();

/// Remove the containers that earlier runs with the label value `label_value` left behind.
///
/// The removal is best effort: a failed `docker` command leaves the containers. A `docker` command
/// that does not exit within `CHILD_LIMIT` fails the test, as in every other bounded run.
fn remove_old_containers(label_value: &str) {
    let filter = format!("label={LABEL_KEY}={label_value}");
    let mut list = Command::new("docker");
    list.args(["ps", "-aq", "--filter", &filter]);
    let listing = run_bounded(list, CHILD_LIMIT, OUTPUT_LIMIT);
    let identifiers: Vec<String> = String::from_utf8_lossy(&listing.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    if identifiers.is_empty() {
        return;
    }
    let mut remove = Command::new("docker");
    remove
        .args(["rm", "--force", "--volumes"])
        .args(&identifiers);
    let _ = run_bounded(remove, CHILD_LIMIT, OUTPUT_LIMIT);
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
///
/// `label_value` marks the container, so the next run of the same test file can remove a
/// container that a killed run left behind. The removal runs once per process. See
/// `OLD_CONTAINERS` for the limit with several processes of one test binary.
pub async fn start_container(label_value: &'static str) -> Postgres {
    OLD_CONTAINERS.call_once(|| remove_old_containers(label_value));
    // A checkout with CRLF line endings must not break the shell script.
    let script = std::fs::read_to_string(INIT_SCRIPT_SOURCE)
        .expect("deploy/db/initdb/01-roles.sh is readable")
        .replace("\r\n", "\n");
    let test_hosts = std::fs::read_to_string(TEST_HOSTS_SOURCE)
        .expect("the test-only init script 02-test-hosts.sh is readable")
        .replace("\r\n", "\n");
    let mut request = GenericImage::new(IMAGE, IMAGE_TAG)
        .with_exposed_port(POSTGRES_PORT.tcp())
        .with_label(LABEL_KEY, label_value)
        .with_env_var("POSTGRES_PASSWORD", random_secret())
        .with_env_var("POSTGRES_INITDB_ARGS", INITDB_ARGS)
        .with_env_var("COMPOSE_SUBNET", COMPOSE_SUBNET)
        .with_startup_timeout(STARTUP_TIMEOUT)
        .with_host_config_modifier(bind_to_loopback)
        .with_copy_to(
            CopyTargetOptions::new(INIT_SCRIPT_TARGET).with_mode(EXECUTABLE_MODE),
            script.into_bytes(),
        )
        .with_copy_to(
            CopyTargetOptions::new(TEST_HOSTS_TARGET).with_mode(EXECUTABLE_MODE),
            test_hosts.into_bytes(),
        );
    for (name, password) in SECRETS {
        request = request.with_copy_to(
            CopyTargetOptions::new(format!("/run/secrets/{name}")).with_mode(SECRET_MODE),
            format!("{password}\n").into_bytes(),
        );
    }
    request.start().await.expect(
        "these tests need Docker: cannot start the PostgreSQL container. Start Docker and run the test again",
    )
}

/// Return the host port that Docker published for the PostgreSQL port.
pub async fn host_port(container: &Postgres) -> u16 {
    container
        .get_host_port_ipv4(POSTGRES_PORT.tcp())
        .await
        .unwrap()
}

/// Run a command in the container and return its exit code and its standard output.
async fn exec(container: &Postgres, command: &[&str]) -> (i64, String) {
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
pub async fn wait_until_ready(container: &Postgres) {
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
pub async fn query(container: &Postgres, sql: &str) -> String {
    let (code, text) = exec(
        container,
        &[
            "gosu", "postgres", "psql", "-U", "postgres", "-d", "recipe", "-tAc", sql,
        ],
    )
    .await;
    assert_eq!(code, 0, "psql failed: {text}");
    text
}
