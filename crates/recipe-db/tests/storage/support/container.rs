//! The PostgreSQL container of the storage tests (design D12).
//!
//! One container serves the whole test binary. A dedicated thread owns the container and its
//! runtime. Each `#[tokio::test]` has its own runtime, so no test runtime can own the container.
//! The tests get only plain connection data: a host, a port, and the test passwords.

use std::collections::HashMap;
use std::process::Command;
use std::sync::mpsc::{Sender, channel};
use std::sync::{LazyLock, OnceLock};
use std::time::Duration;

use recipe_db::settings::{Connection, MigrateSettings, Password};
use sqlx::ConnectOptions as _;
use testcontainers::bollard::models::PortBinding;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, CopyTargetOptions, GenericImage, ImageExt};
use uuid::Uuid;

use super::session::options;

const IMAGE: &str = "postgres";
const IMAGE_TAG: &str = "16.15-bookworm";
const POSTGRES_PORT: u16 = 5432;
/// The database that `01-roles.sh` creates. The harness migrates it once and uses it as template.
pub const TEMPLATE_DATABASE: &str = "recipe";
/// The label that marks the containers of this test binary. The next run removes them.
const LABEL_KEY: &str = "org.recipe-editor.storage-tests";
const LABEL_VALUE: &str = "recipe-db";
/// The subnet that `01-roles.sh` writes into the lines of `pg_hba.conf`.
const COMPOSE_SUBNET: &str = "172.28.0.0/16";
const INITDB_ARGS: &str = "--data-checksums --lc-collate=C --lc-ctype=C.UTF-8";
/// The address that the published port binds to. No other host can reach the container.
const LOOPBACK: &str = "127.0.0.1";
/// The host port "0" asks Docker for a free port.
const FREE_HOST_PORT: &str = "0";
const MAX_CONNECTIONS: &str = "max_connections=200";
const INIT_SCRIPT_SOURCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/db/initdb/01-roles.sh"
);
const INIT_SCRIPT_TARGET: &str = "/docker-entrypoint-initdb.d/01-roles.sh";
/// The test-only init script with the host lines of the harness (ADR-0069 (u)). The tests of
/// `recipe-server` mount the same file.
const TEST_HOSTS_SOURCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/storage/support/02-test-hosts.sh"
);
/// The name sorts after `01-roles.sh`, so the image runs it second.
const TEST_HOSTS_TARGET: &str = "/docker-entrypoint-initdb.d/02-test-hosts.sh";
/// The shared file of the client rules (ADR-0103 (l)). The path is outside the init directory,
/// so the image never runs it. The init script sources it from the same path.
const CLUSTER_ACCESS_SOURCE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/db/cluster-access.sh"
);
const CLUSTER_ACCESS_TARGET: &str = "/etc/recipe-db/cluster-access.sh";
const EXECUTABLE_MODE: u32 = 0o755;
const SECRET_MODE: u32 = 0o644;
/// The mode of a file that bash sources and never runs.
const SOURCED_MODE: u32 = 0o644;
/// The start returns at once. `wait_for_login` waits for the final start of the server.
/// The init scripts run on the Unix socket only, so a TCP login proves that they are done.
const START_GRACE_MILLIS: u64 = 100;
const START_TIMEOUT: Duration = Duration::from_secs(120);
const LOGIN_ATTEMPTS: u32 = 120;
const LOGIN_PAUSE: Duration = Duration::from_millis(500);
const LOG_TAIL_BYTES: usize = 4000;

/// The superuser password of this run: two random UUIDs, 64 hexadecimal characters.
static SUPERUSER_PASSWORD: LazyLock<String> =
    LazyLock::new(|| format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()));

/// A login role of the test container, with its test password.
#[derive(Debug, Clone, Copy)]
pub enum Role {
    /// The PostgreSQL superuser. Only the harness uses it.
    Superuser,
    /// The owner of the schema.
    Migrator,
    /// The runtime role.
    App,
    /// The role of `pg_dump`.
    Backup,
    /// The role of the replication connection.
    Replicator,
}

impl Role {
    /// The roles whose password the init script reads from a secret file.
    pub const WITH_SECRET_FILE: [Role; 4] =
        [Role::Migrator, Role::App, Role::Backup, Role::Replicator];

    /// The name of the role in the database.
    pub const fn name(self) -> &'static str {
        match self {
            Role::Superuser => "postgres",
            Role::Migrator => "recipe_migrator",
            Role::App => "recipe_app",
            Role::Backup => "recipe_backup",
            Role::Replicator => "recipe_replicator",
        }
    }

    /// The test password of the role. The secret file holds it with a trailing newline.
    ///
    /// The superuser password is random for each run (review S2). The other roles keep a fixed
    /// test password, because only the database in the container checks them.
    pub fn password(self) -> &'static str {
        match self {
            Role::Superuser => SUPERUSER_PASSWORD.as_str(),
            Role::Migrator => "test-migrator-password",
            Role::App => "test-app-password",
            Role::Backup => "test-backup-password",
            Role::Replicator => "test-replicator-password",
        }
    }

    /// The path of the secret file in the container (`reliability.md` section 7.11).
    fn secret_path(self) -> String {
        let prefix = self.name().strip_prefix("recipe_").unwrap_or(self.name());
        format!("/run/secrets/{prefix}_password")
    }
}

/// The address of the test container as plain data.
#[derive(Debug, Clone)]
pub struct Server {
    /// The host name of the Docker host.
    pub host: String,
    /// The host port that maps to the port 5432 of the container.
    pub port: u16,
    /// The identifier of the container, for `docker exec` in the script tests.
    pub container_id: String,
}

impl Server {
    /// The connection data of a role for one database, in the form that `recipe-db` takes.
    pub fn target(&self, role: Role, database: &str) -> Connection {
        Connection {
            host: self.host.clone(),
            port: self.port,
            database: database.to_owned(),
            user: role.name().to_owned(),
            password: Password::from(role.password().to_owned()),
        }
    }
}

type Outcome = Result<Server, String>;

static SERVER: OnceLock<Outcome> = OnceLock::new();

/// Return the container of this test binary. The first call starts it.
///
/// The call fails the test with a clear message when Docker is not available. It never skips.
pub fn server() -> &'static Server {
    let outcome = SERVER.get_or_init(start_container_thread);
    assert!(
        outcome.is_ok(),
        "{}",
        outcome.as_ref().err().map_or("", String::as_str)
    );
    outcome.as_ref().expect("checked above")
}

fn start_container_thread() -> Outcome {
    let (sender, receiver) = channel();
    std::thread::Builder::new()
        .name("storage-container".to_owned())
        .spawn(move || run_container_thread(&sender))
        .map_err(|error| format!("Cannot start the container thread: {error}"))?;
    receiver
        .recv()
        .map_err(|_| "The container thread ended without a result.".to_owned())?
}

/// Own the runtime and the container until the process ends.
///
/// Nothing drops a container that a static holds. The label lets the next run remove it.
fn run_container_thread(sender: &Sender<Outcome>) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            // The receiver is gone only when the process ends.
            let _ = sender.send(Err(format!("Cannot build the runtime: {error}")));
            return;
        }
    };
    match runtime.block_on(boot()) {
        Ok((server, container)) => {
            let _ = sender.send(Ok(server));
            let _keep_alive = (runtime, container);
            loop {
                std::thread::park();
            }
        }
        Err(message) => {
            let _ = sender.send(Err(message));
        }
    }
}

async fn boot() -> Result<(Server, ContainerAsync<GenericImage>), String> {
    remove_old_containers();
    let request = container_request()?;
    let container = request.start().await.map_err(|error| {
        format!(
            "The storage tests need Docker. Cannot start {IMAGE}:{IMAGE_TAG}: {error}. \
             Start Docker and run the tests again."
        )
    })?;
    let host = container
        .get_host()
        .await
        .map_err(|error| format!("Cannot read the host of the container: {error}"))?
        .to_string();
    let port = container
        .get_host_port_ipv4(POSTGRES_PORT.tcp())
        .await
        .map_err(|error| format!("Cannot read the port of the container: {error}"))?;
    // The port is mapped on IPv4. On Windows, "localhost" tries IPv6 first and each connect is slow.
    let host = if host == "localhost" {
        "127.0.0.1".to_owned()
    } else {
        host
    };
    let server = Server {
        host,
        port,
        container_id: container.id().to_owned(),
    };
    if let Err(message) = wait_for_login(&server, &container).await {
        let log = container.stderr_to_vec().await.unwrap_or_default();
        return Err(format!("{message}\n{}", tail(&log)));
    }
    prepare_template(&server).await?;
    Ok((server, container))
}

/// Migrate the database `recipe` once and close every connection to it.
///
/// `CREATE DATABASE ... TEMPLATE` fails while a session uses the template. No test connects
/// to `recipe` after this step.
async fn prepare_template(server: &Server) -> Result<(), String> {
    let target = server.target(Role::Migrator, TEMPLATE_DATABASE);
    recipe_db::migrate(&target, &MigrateSettings::default())
        .await
        .map_err(|error| format!("The migration into {TEMPLATE_DATABASE} failed: {error}"))?;
    let mut admin = options(server, Role::Superuser, "postgres")
        .connect()
        .await
        .map_err(|error| format!("The superuser cannot connect: {error}"))?;
    sqlx::query(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity \
         WHERE datname = $1 AND pid <> pg_backend_pid()",
    )
    .bind(TEMPLATE_DATABASE)
    .execute(&mut admin)
    .await
    .map_err(|error| format!("Cannot close the sessions of {TEMPLATE_DATABASE}: {error}"))?;
    let open: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname = $1")
        .bind(TEMPLATE_DATABASE)
        .fetch_one(&mut admin)
        .await
        .map_err(|error| format!("Cannot count the sessions of {TEMPLATE_DATABASE}: {error}"))?;
    if open == 0 {
        Ok(())
    } else {
        Err(format!("{open} sessions of {TEMPLATE_DATABASE} stay open."))
    }
}

fn container_request() -> Result<testcontainers::ContainerRequest<GenericImage>, String> {
    let script = read_script(INIT_SCRIPT_SOURCE)?;
    let test_hosts = read_script(TEST_HOSTS_SOURCE)?;
    let cluster_access = read_script(CLUSTER_ACCESS_SOURCE)?;
    let mut request = GenericImage::new(IMAGE, IMAGE_TAG)
        .with_exposed_port(POSTGRES_PORT.tcp())
        .with_wait_for(WaitFor::millis(START_GRACE_MILLIS))
        .with_label(LABEL_KEY, LABEL_VALUE)
        .with_env_var("POSTGRES_PASSWORD", Role::Superuser.password())
        .with_env_var("POSTGRES_INITDB_ARGS", INITDB_ARGS)
        .with_env_var("COMPOSE_SUBNET", COMPOSE_SUBNET)
        .with_cmd(["postgres", "-c", MAX_CONNECTIONS])
        .with_startup_timeout(START_TIMEOUT)
        .with_host_config_modifier(bind_to_loopback)
        .with_copy_to(
            CopyTargetOptions::new(INIT_SCRIPT_TARGET).with_mode(EXECUTABLE_MODE),
            script.into_bytes(),
        )
        .with_copy_to(
            CopyTargetOptions::new(TEST_HOSTS_TARGET).with_mode(EXECUTABLE_MODE),
            test_hosts.into_bytes(),
        )
        .with_copy_to(
            CopyTargetOptions::new(CLUSTER_ACCESS_TARGET).with_mode(SOURCED_MODE),
            cluster_access.into_bytes(),
        );
    for role in Role::WITH_SECRET_FILE {
        let content = format!("{}\n", role.password());
        request = request.with_copy_to(
            CopyTargetOptions::new(role.secret_path()).with_mode(SECRET_MODE),
            content.into_bytes(),
        );
    }
    Ok(request)
}

/// Publish the PostgreSQL port on the loopback address only, on a free host port (review S2).
///
/// `testcontainers` 0.28 binds a published port to every address. Its `HostConfig` modifier
/// runs after the defaults, so it replaces the port bindings with one that names `127.0.0.1`.
fn bind_to_loopback(host_config: &mut testcontainers::bollard::models::HostConfig) {
    let binding = PortBinding {
        host_ip: Some(LOOPBACK.to_owned()),
        host_port: Some(FREE_HOST_PORT.to_owned()),
    };
    host_config.port_bindings = Some(HashMap::from([(
        format!("{POSTGRES_PORT}/tcp"),
        Some(vec![binding]),
    )]));
}

/// Read an init script. A checkout with CRLF line endings must not break the shell script.
fn read_script(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map(|text| text.replace("\r\n", "\n"))
        .map_err(|error| format!("Cannot read the init script {path}: {error}"))
}

/// Wait until the server accepts a TCP login of the superuser.
///
/// A failed init script stops the container, so the loop ends at once in that case.
async fn wait_for_login(
    server: &Server,
    container: &ContainerAsync<GenericImage>,
) -> Result<(), String> {
    let mut last_error = String::new();
    for _ in 0..LOGIN_ATTEMPTS {
        match options(server, Role::Superuser, "postgres").connect().await {
            Ok(_) => return Ok(()),
            Err(error) => last_error = error.to_string(),
        }
        if !container.is_running().await.unwrap_or(false) {
            return Err(format!(
                "The container stopped during the start. Last error: {last_error}"
            ));
        }
        tokio::time::sleep(LOGIN_PAUSE).await;
    }
    Err(format!(
        "The container does not accept a login. Last error: {last_error}"
    ))
}

/// Remove the containers that earlier runs of this binary left behind.
///
/// A failure here is not an error. The start of the new container reports a missing Docker.
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

/// Return the last bytes of a log as text.
fn tail(log: &[u8]) -> String {
    let start = log.len().saturating_sub(LOG_TAIL_BYTES);
    String::from_utf8_lossy(log.get(start..).unwrap_or_default()).into_owned()
}
