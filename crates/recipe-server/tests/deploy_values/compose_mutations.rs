//! Proof that each group of checks on `deploy/compose.yaml` and `deploy/.env.example` can fail.
//!
//! Each case changes one copy of a text and asserts a fault. No file of `deploy/` changes.

use crate::inputs::{Inputs, Mutation, assert_mutations_fail, check_all};

const HEALTHCHECK: [Mutation; 6] = [
    (
        "compose.yaml",
        "interval: 10s",
        "interval: 20s",
        "healthcheck.interval",
    ),
    (
        "compose.yaml",
        "retries: 3",
        "retries: 4",
        "healthcheck.retries",
    ),
    (
        "compose.yaml",
        "      timeout: 5s",
        "      timeout: 6s",
        "healthcheck.timeout",
    ),
    (
        "compose.yaml",
        "start_period: 30s",
        "start_period: 20s",
        "healthcheck.start_period",
    ),
    (
        "compose.yaml",
        "start_interval: 1s",
        "start_interval: 2s",
        "healthcheck.start_interval",
    ),
    // A missing key is a fault, not a pass.
    (
        "compose.yaml",
        "retries: 3",
        "tries: 3",
        "healthcheck.retries",
    ),
];

const HEALTH_SERVICES: [&str; 5] = ["proxy", "app", "db", "backup", "walarchive"];
const ALL_SERVICES: [&str; 8] = [
    "proxy",
    "backup-premigrate",
    "migrate",
    "app",
    "db",
    "backup",
    "walarchive",
    "pitr-check",
];

/// One mutation changes the same text in several services. Each of them must report its own fault,
/// so a service that falls out of a check cannot hide behind the others.
fn assert_each_service(name: &str, from: &str, to: &str, services: &[&str], suffix: &str) {
    let report = check_all(&Inputs::load().with_replaced(name, from, to));
    for service in services {
        let part = format!("{service}.{suffix}");
        assert!(
            report.has_fault_for(&part),
            "no fault for {part:?} after {from:?} -> {to:?}\nfaults:\n{}",
            report.message()
        );
    }
}

#[test]
fn a_changed_or_missing_healthcheck_value_is_a_fault_in_each_service() {
    for (name, from, to, part) in HEALTHCHECK {
        assert_each_service(name, from, to, &HEALTH_SERVICES, part);
    }
}

#[test]
fn a_changed_limit_is_a_fault_in_each_service() {
    let small = [
        "proxy",
        "backup-premigrate",
        "migrate",
        "backup",
        "walarchive",
        "pitr-check",
    ];
    let cases: [(&str, &str, &str, &[&str]); 6] = [
        ("cpus: 0.5", "cpus: 0.6", "cpus", &small),
        (
            "mem_limit: 256m",
            "mem_limit: 512m",
            "mem_limit",
            &["migrate", "backup-premigrate"],
        ),
        (
            "mem_limit: 128m",
            "mem_limit: 64m",
            "mem_limit",
            &["proxy", "walarchive"],
        ),
        (
            "mem_limit: 512m",
            "mem_limit: 640m",
            "mem_limit",
            &["app", "backup", "pitr-check"],
        ),
        (
            "memswap_limit: 512m",
            "swap_limit: 512m",
            "memswap_limit",
            &["app", "backup", "pitr-check"],
        ),
        (
            "memswap_limit: 256m",
            "swap_limit: 256m",
            "memswap_limit",
            &["migrate", "backup-premigrate"],
        ),
    ];
    for (from, to, suffix, services) in cases {
        assert_each_service("compose.yaml", from, to, services, suffix);
    }
}

#[test]
fn a_changed_security_option_or_logging_anchor_is_a_fault_in_each_service() {
    let logged = &ALL_SERVICES[..7];
    let cases: [(&str, &str, &str, &[&str]); 5] = [
        ("- ALL", "- NET_RAW", "cap_drop", &ALL_SERVICES),
        (
            "no-new-privileges:true",
            "no-new-privileges:false",
            "security_opt",
            &ALL_SERVICES,
        ),
        (
            "read_only: true",
            "read_only: false",
            "read_only",
            &["app", "proxy"],
        ),
        ("logging: *logging", "logging: *other", "logging", logged),
        (
            "tmpfs:",
            "ramfs:",
            "tmpfs",
            &["backup", "backup-premigrate", "walarchive"],
        ),
    ];
    for (from, to, suffix, services) in cases {
        assert_each_service("compose.yaml", from, to, services, suffix);
    }
}

#[test]
fn the_address_of_proxy_belongs_to_the_network_backend() {
    let moved = Inputs::load()
        .with_replaced(
            "compose.yaml",
            "        ipv4_address: ${PROXY_ADDRESS:?}",
            "        aliases: [proxy]",
        )
        .with_replaced(
            "compose.yaml",
            "      edge: {}",
            "      edge:\n        ipv4_address: ${PROXY_ADDRESS:?}",
        );
    let report = check_all(&moved);
    assert!(
        report.has_fault_for("proxy.networks.backend.ipv4_address"),
        "{}",
        report.message()
    );
}

const GRACE_AND_LIMITS: [Mutation; 10] = [
    (
        "compose.yaml",
        "stop_grace_period: 40s",
        "stop_grace_period: 45s",
        "app.stop_grace_period",
    ),
    (
        "compose.yaml",
        "stop_grace_period: 60s",
        "stop_grace_period: 30s",
        "db.stop_grace_period",
    ),
    (
        "compose.yaml",
        "shm_size: 256m",
        "shm_size: 128m",
        "db.shm_size",
    ),
    (
        "compose.yaml",
        "shm_size: 256m",
        "size: 256m",
        "db.shm_size",
    ),
    (
        "compose.yaml",
        "mem_limit: 2g",
        "mem_limit: 3g",
        "db.mem_limit",
    ),
    (
        "compose.yaml",
        "memswap_limit: 2g",
        "swap_limit: 2g",
        "db.memswap_limit",
    ),
    (
        "compose.yaml",
        "mem_limit: 128m",
        "mem_limit: 64m",
        "proxy.mem_limit",
    ),
    (
        "compose.yaml",
        "cpus: 0.5",
        "cpus: 0.6",
        "container.small_cpus",
    ),
    ("compose.yaml", "cpus: 1", "cpus: 2", "app.cpus"),
    (
        "compose.yaml",
        "mem_limit: 256m",
        "mem_limit: 512m",
        "migrate.mem_limit",
    ),
];

#[test]
fn a_changed_grace_period_or_resource_limit_is_a_fault() {
    assert_mutations_fail(&GRACE_AND_LIMITS);
}

const DB_COMMAND: [Mutation; 11] = [
    (
        "compose.yaml",
        "max_connections=50",
        "max_connections=60",
        "db.max_connections",
    ),
    (
        "compose.yaml",
        "shared_buffers=512MB",
        "shared_buffers=1GB",
        "pg.shared_buffers",
    ),
    (
        "compose.yaml",
        "wal_keep_size=1GB",
        "wal_keep_size=2GB",
        "pg.wal_keep_size",
    ),
    (
        "compose.yaml",
        "wal_keep_size=1GB",
        "wal_keep=1GB",
        "pg.wal_keep_size",
    ),
    (
        "compose.yaml",
        "checkpoint_timeout=5min",
        "checkpoint_timeout=10min",
        "pg.checkpoint_timeout",
    ),
    (
        "compose.yaml",
        "max_wal_senders=3",
        "max_wal_senders=5",
        "pg.max_wal_senders",
    ),
    (
        "compose.yaml",
        "log_min_duration_statement=1000",
        "log_min_duration_statement=500",
        "db.slow_query",
    ),
    ("compose.yaml", "fsync=on", "fsync=off", "command fsync"),
    (
        "compose.yaml",
        "timezone=UTC",
        "timezone=Europe/Warsaw",
        "command timezone",
    ),
    (
        "compose.yaml",
        "wal_level=replica",
        "wal_level=minimal",
        "command wal_level",
    ),
    (
        "compose.yaml",
        "default_transaction_isolation=read committed",
        "default_transaction_isolation=serializable",
        "default_transaction_isolation",
    ),
];

#[test]
fn a_changed_postgres_setting_is_a_fault() {
    assert_mutations_fail(&DB_COMMAND);
}

/// The table is the source: a changed row makes the unchanged file a fault.
const TABLE_ROWS: [Mutation; 8] = [
    (
        "reliability.md",
        "| `pg.max_wal_senders` | 3 |",
        "| `pg.max_wal_senders` | 4 |",
        "pg.max_wal_senders",
    ),
    (
        "reliability.md",
        "| `pg.checkpoint_timeout` | 5 min |",
        "| `pg.checkpoint_timeout` | 10 min |",
        "pg.checkpoint_timeout",
    ),
    (
        "reliability.md",
        "| `fsync` | `on` |",
        "| `fsync` | `off` |",
        "command fsync",
    ),
    (
        "reliability.md",
        "| `container.db_memory` / `container.db_cpus` | 2 GiB / 2 |",
        "| `container.db_memory` / `container.db_cpus` | 3 GiB / 2 |",
        "container.db_memory",
    ),
    (
        "reliability.md",
        "| `log.max_size` / `log.max_file` | 50m / 20 |",
        "| `log.max_size` / `log.max_file` | 40m / 20 |",
        "log.max_size",
    ),
    (
        "reliability.md",
        "| `http.public_port` / `http.internal_port` | 8080 / 9000.",
        "| `http.public_port` / `http.internal_port` | 8080 / 9001.",
        "http.internal_port",
    ),
    (
        "reliability.md",
        "| `health.check_interval`",
        "| `health.check_period`",
        "health.check_interval",
    ),
    (
        "reliability.md",
        "| `pg.shared_buffers` | 25 % of",
        "| `pg.shared_buffers` | 50 % of",
        "pg.shared_buffers",
    ),
];

#[test]
fn a_changed_row_of_the_table_is_a_fault_in_the_file() {
    assert_mutations_fail(&TABLE_ROWS);
}

const APP_ENV_AND_PORTS: [Mutation; 10] = [
    // A word or a mark after the value is a change of the value.
    (
        "compose.yaml",
        "RECIPE_DB__NAME: recipe",
        "RECIPE_DB__NAME: \"recipe archive\"",
        "db.name",
    ),
    (
        "compose.yaml",
        "RECIPE_DB__NAME: recipe",
        "RECIPE_DB__NAME: recipe.",
        "db.name",
    ),
    (
        "compose.yaml",
        "RECIPE_HTTP__PUBLIC_PORT: \"8080\"",
        "RECIPE_HTTP__PUBLIC_PORT: \"8081\"",
        "http.public_port",
    ),
    (
        "compose.yaml",
        "RECIPE_SESSION__IDLE_TIMEOUT: \"1800\"",
        "RECIPE_SESSION__IDLE_TIMEOUT: \"900\"",
        "session.idle_timeout",
    ),
    (
        "compose.yaml",
        "RECIPE_DB__NAME: recipe",
        "RECIPE_DB__NAME: other",
        "db.name",
    ),
    (
        "compose.yaml",
        "http://app:9000/healthz",
        "http://app:9001/healthz",
        "proxy.healthcheck URL",
    ),
    ("compose.yaml", "- \"9000\"", "- \"9001\"", "app.expose"),
    (
        "compose.yaml",
        "\"/readyz\"",
        "\"/healthz\"",
        "app.healthcheck.test",
    ),
    (
        "compose.yaml",
        "\"-U\", \"pg_isready\"",
        "\"-U\", \"recipe_app\"",
        "pg_isready line",
    ),
    (
        "compose.yaml",
        "\"-p\", \"5432\"",
        "\"-p\", \"5433\"",
        "pg_isready line",
    ),
];

#[test]
fn a_changed_application_value_port_or_probe_line_is_a_fault() {
    assert_mutations_fail(&APP_ENV_AND_PORTS);
}

const SECURITY: [Mutation; 10] = [
    // A list in flow style hides no item.
    (
        "compose.yaml",
        "    init: true",
        "    cap_add: [SYS_ADMIN]",
        "backup.cap_add",
    ),
    (
        "compose.yaml",
        "    init: true",
        "    ports: [\"8080:8080\"]",
        "backup.ports",
    ),
    (
        "compose.yaml",
        "- NET_BIND_SERVICE",
        "- NET_ADMIN",
        "proxy.cap_add",
    ),
    (
        "compose.yaml",
        "    init: true",
        "    cap_add:\n      - SYS_ADMIN",
        "backup.cap_add",
    ),
    ("compose.yaml", "- ALL", "- NET_RAW", "cap_drop"),
    (
        "compose.yaml",
        "no-new-privileges:true",
        "no-new-privileges:false",
        "security_opt",
    ),
    (
        "compose.yaml",
        "user: \"999:999\"",
        "user: \"0:0\"",
        "db.user",
    ),
    (
        "compose.yaml",
        "read_only: true",
        "read_only: false",
        "read_only",
    ),
    ("compose.yaml", "    expose:", "    ports:", "app.ports"),
    (
        "compose.yaml",
        "      - login_hmac_key",
        "      - other_key",
        "app.secrets",
    ),
];

#[test]
fn a_changed_capability_security_option_or_secret_list_is_a_fault() {
    assert_mutations_fail(&SECURITY);
}

const LOGGING_AND_SECRETS: [Mutation; 7] = [
    (
        "compose.yaml",
        "max-size: 50m",
        "max-size: 10m",
        "log.max_size",
    ),
    (
        "compose.yaml",
        "max-file: \"20\"",
        "max-file: \"5\"",
        "log.max_file",
    ),
    (
        "compose.yaml",
        "mode: non-blocking",
        "mode: blocking",
        "log.mode",
    ),
    (
        "compose.yaml",
        "max-buffer-size: 4m",
        "max-buffer: 4m",
        "log.max_buffer_size",
    ),
    (
        "compose.yaml",
        "logging: *logging",
        "logging: *other",
        "app.logging",
    ),
    (
        "compose.yaml",
        "file: ${RECIPE_SECRETS_DIR:?}/tls_key",
        "file: /tmp/tls_key",
        "secrets.tls_key.file",
    ),
    (
        "compose.yaml",
        "file: ${RECIPE_SECRETS_DIR:?}/csrf_signing_key",
        "file: ${RECIPE_SECRETS_DIR:?}/csrf_key",
        "secrets.csrf_signing_key.file",
    ),
];

#[test]
fn a_changed_logging_option_or_secret_file_is_a_fault() {
    assert_mutations_fail(&LOGGING_AND_SECRETS);
}

const NETWORK_AND_PROXIES: [Mutation; 9] = [
    (
        "compose.yaml",
        "internal: true",
        "internal: false",
        "networks.backend.internal",
    ),
    (
        "compose.yaml",
        "ip_range: ${COMPOSE_IP_RANGE:?}",
        "ip_range: ${COMPOSE_SUBNET:?}",
        "networks.backend.ip_range",
    ),
    (
        "compose.yaml",
        "ipv4_address: ${PROXY_ADDRESS:?}",
        "ipv4_address: 172.28.0.9",
        "ipv4_address",
    ),
    (
        "compose.yaml",
        "      edge: {}",
        "      other: {}",
        "proxy.networks",
    ),
    (
        "compose.yaml",
        "${PROXY_ADDRESS:?}/32",
        "${PROXY_ADDRESS:?}/24",
        "http.trusted_proxies",
    ),
    (
        "compose.yaml",
        "${PROXY_ADDRESS:?}/32",
        "172.28.0.2/32",
        "http.trusted_proxies",
    ),
    (
        ".env.example",
        "PROXY_ADDRESS=172.28.0.2",
        "PROXY_ADDRESS=172.28.0.9",
        "http.trusted_proxies",
    ),
    (
        ".env.example",
        "COMPOSE_IP_RANGE=172.28.1.0/24",
        "COMPOSE_IP_RANGE=172.28.0.0/24",
        "PROXY_ADDRESS",
    ),
    (
        ".env.example",
        "COMPOSE_SUBNET=172.28.0.0/16",
        "COMPOSE_SUBNET=10.0.0.0/16",
        "COMPOSE_IP_RANGE",
    ),
];

#[test]
fn a_changed_network_address_or_trusted_proxy_line_is_a_fault() {
    assert_mutations_fail(&NETWORK_AND_PROXIES);
}

const ENV_EXAMPLE: [Mutation; 2] = [
    (
        ".env.example",
        "SITE_ADDRESS=",
        "SITE_ADDR=",
        "the names of the values",
    ),
    (
        ".env.example",
        "RECIPE_VERSION=",
        "VERSION=",
        "the names of the values",
    ),
];

#[test]
fn a_missing_or_extra_value_of_the_env_example_is_a_fault() {
    assert_mutations_fail(&ENV_EXAMPLE);
}
