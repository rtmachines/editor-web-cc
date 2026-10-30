//! Checks of the values of `deploy/compose.yaml` against `reliability.md` sections 7.11 and 7.6.
//!
//! Each function reads the text of the file as an argument and returns a report of faults.

use crate::canon::{Bare, Canon};
use crate::db_command::command_settings;
use crate::fault::Report;
use crate::lines::{Compose, Service, flow_list};
use crate::table::Table;

const FILE: &str = "deploy/compose.yaml";

/// The services with a healthcheck (`reliability.md` 7.1, last column of the table).
const HEALTH_SERVICES: [&str; 5] = ["proxy", "app", "db", "backup", "walarchive"];
/// The Compose key of a healthcheck and its key in 7.11 (`reliability.md` 7.1, line "Healthchecks:").
const HEALTH_FIELDS: [(&str, &str); 5] = [
    ("interval", "health.check_interval"),
    ("retries", "health.check_failures"),
    ("timeout", "health.check_timeout"),
    ("start_period", "health.start_period"),
    ("start_interval", "health.start_interval"),
];
/// The memory key and the CPU key of 7.11 for each service (`reliability.md` 7.8, "Resource limits").
const LIMITS: [(&str, &str, &str); 9] = [
    ("proxy", "container.small_memory", "container.small_cpus"),
    (
        "backup-premigrate",
        "container.oneshot_memory",
        "container.small_cpus",
    ),
    (
        "migrate",
        "container.oneshot_memory",
        "container.small_cpus",
    ),
    ("app", "container.app_memory", "container.app_cpus"),
    ("db", "container.db_memory", "container.db_cpus"),
    ("backup", "container.backup_memory", "container.small_cpus"),
    (
        "walarchive",
        "container.small_memory",
        "container.small_cpus",
    ),
    (
        "pitr-check",
        "container.backup_memory",
        "container.small_cpus",
    ),
    (
        "restore-volume",
        "container.backup_memory",
        "container.small_cpus",
    ),
];
/// The Compose key and the key of 7.11 for the grace periods and the shared memory.
/// The rows are `shutdown.drain / app.stop_grace_period` and `db.stop_grace_period / db.shm_size`.
const GRACE: [(&str, &str, &str); 3] = [
    ("app", "stop_grace_period", "app.stop_grace_period"),
    ("db", "stop_grace_period", "db.stop_grace_period"),
    ("db", "shm_size", "db.shm_size"),
];
/// The variables of `environment`: the service, the variable, and its key in 7.11 (design D7).
/// `migrate` holds its secret path. `db` holds the path of the superuser password (ADR-0069).
const ENV_ROWS: &[(&str, &str, &str, Bare)] = &[
    ("app", "RECIPE_DB__HOST", "db.host", Bare::Plain),
    ("app", "RECIPE_DB__PORT", "db.port", Bare::Plain),
    ("app", "RECIPE_DB__NAME", "db.name", Bare::Plain),
    ("app", "RECIPE_DB__APP_USER", "db.app_user", Bare::Plain),
    (
        "app",
        "RECIPE_SECRETS__APP_PASSWORD_FILE",
        "secrets.app_password_file",
        Bare::Plain,
    ),
    (
        "migrate",
        "RECIPE_SECRETS__MIGRATOR_PASSWORD_FILE",
        "secrets.migrator_password_file",
        Bare::Plain,
    ),
    (
        "db",
        "POSTGRES_PASSWORD_FILE",
        "secrets.postgres_password_file",
        Bare::Plain,
    ),
    (
        "app",
        "RECIPE_SECURITY__LOGIN_HMAC_KEY_FILE",
        "security.login_hmac_key_file",
        Bare::Plain,
    ),
    (
        "app",
        "RECIPE_SECURITY__CSRF_SIGNING_KEY_FILE",
        "security.csrf_signing_key_file",
        Bare::Plain,
    ),
    (
        "app",
        "RECIPE_HTTP__PUBLIC_PORT",
        "http.public_port",
        Bare::Plain,
    ),
    (
        "app",
        "RECIPE_HTTP__INTERNAL_PORT",
        "http.internal_port",
        Bare::Plain,
    ),
    (
        "app",
        "RECIPE_APPROVAL__SEPARATION_OF_DUTIES",
        "approval.separation_of_duties",
        Bare::Plain,
    ),
    (
        "app",
        "RECIPE_ALERTS__WEBHOOK_URL_FILE",
        "alerts.webhook_url_file",
        Bare::Plain,
    ),
    (
        "app",
        "RECIPE_SESSION__SECURE",
        "session.secure",
        Bare::Plain,
    ),
    (
        "app",
        "RECIPE_SESSION__IDLE_TIMEOUT",
        "session.idle_timeout",
        Bare::Seconds,
    ),
    (
        "app",
        "RECIPE_SESSION__ABSOLUTE_TIMEOUT",
        "session.absolute_timeout",
        Bare::Seconds,
    ),
    (
        "app",
        "RECIPE_I18N__TRANSLATION_LANGS",
        "i18n.translation_langs",
        Bare::Plain,
    ),
];
/// Settings of the `command:` list of `db` that 7.6 gives a fixed value.
const POSTGRES_FIXED: [&str; 8] = [
    "fsync",
    "synchronous_commit",
    "full_page_writes",
    "wal_level",
    "archive_mode",
    "log_lock_waits",
    "default_transaction_isolation",
    "timezone",
];
/// Settings of the `command:` list of `db` that take a value of 7.11 (`reliability.md` 7.6).
const POSTGRES_SIZED: [(&str, &str, Bare); 5] = [
    ("max_wal_senders", "pg.max_wal_senders", Bare::Plain),
    ("wal_keep_size", "pg.wal_keep_size", Bare::Plain),
    ("checkpoint_timeout", "pg.checkpoint_timeout", Bare::Plain),
    ("max_connections", "db.max_connections", Bare::Plain),
    ("log_min_duration_statement", "db.slow_query", Bare::Millis),
];

fn missing(report: &mut Report, name: &str) {
    report.fail(
        FILE,
        &format!("service {name}"),
        "the service exists",
        "missing",
    );
}

/// The five healthchecks use the five keys `health.*` (`reliability.md` 7.1, "Healthchecks:").
pub fn check_healthchecks(compose: &Compose, table: &Table) -> Report {
    let mut report = Report::default();
    for name in HEALTH_SERVICES {
        let Some(service) = compose.service(name) else {
            missing(&mut report, name);
            continue;
        };
        for (field, key) in HEALTH_FIELDS {
            report.cover(key);
            let label = format!("{name}.healthcheck.{field} ({key})");
            report.compare(
                FILE,
                &label,
                table.expected(key),
                service.health(field),
                Bare::Plain,
            );
        }
    }
    report
}

/// The grace periods and the shared memory of `app` and `db`.
pub fn check_grace_periods(compose: &Compose, table: &Table) -> Report {
    let mut report = Report::default();
    for (name, field, key) in GRACE {
        report.cover(key);
        let Some(service) = compose.service(name) else {
            missing(&mut report, name);
            continue;
        };
        let label = format!("{name}.{field} ({key})");
        report.compare(
            FILE,
            &label,
            table.expected(key),
            service.scalar(field),
            Bare::Plain,
        );
    }
    report
}

/// Each service has `mem_limit` equal to `memswap_limit` and `cpus` from the `container.*` keys.
pub fn check_limits(compose: &Compose, table: &Table) -> Report {
    let mut report = Report::default();
    for (name, memory, cpus) in LIMITS {
        report.cover(memory);
        report.cover(cpus);
        let Some(service) = compose.service(name) else {
            missing(&mut report, name);
            continue;
        };
        for field in ["mem_limit", "memswap_limit"] {
            let label = format!("{name}.{field} ({memory})");
            report.compare(
                FILE,
                &label,
                table.expected(memory),
                service.scalar(field),
                Bare::Plain,
            );
        }
        let label = format!("{name}.cpus ({cpus})");
        report.compare(
            FILE,
            &label,
            table.expected(cpus),
            service.scalar("cpus"),
            Bare::Plain,
        );
    }
    report
}

/// The expected `shared_buffers`: `pg.shared_buffers` of the memory `container.db_memory`.
fn expected_shared_buffers(table: &Table) -> Option<(Canon, String)> {
    let (Canon::Percent(percent), Canon::Bytes(memory)) = (
        table.canon("pg.shared_buffers")?,
        table.canon("container.db_memory")?,
    ) else {
        return None;
    };
    let bytes = memory.checked_mul(percent)?.checked_div(100_000)?;
    let shown = format!(
        "{} of {} (7.11 `pg.shared_buffers`)",
        table.shown("pg.shared_buffers"),
        table.shown("container.db_memory")
    );
    Some((Canon::Bytes(bytes), shown))
}

/// The `command:` list of `db` holds the settings of `reliability.md` 7.6 and the keys `pg.*`.
pub fn check_db_command(compose: &Compose, settings: &Table, postgres: &Table) -> Report {
    let mut report = Report::default();
    let Some(service) = compose.service("db") else {
        missing(&mut report, "db");
        return report;
    };
    let found = command_settings(FILE, &service.items("command"), &mut report);
    for name in POSTGRES_FIXED {
        let want = postgres.literal(name);
        let value = found.get(name).copied();
        if want.as_deref() != value {
            let label = format!("db.command {name} (7.6)");
            let shown = want.unwrap_or_else(|| "a row in 7.6".to_owned());
            report.fail(FILE, &label, shown, value.unwrap_or("missing"));
        }
    }
    for (name, key, bare) in POSTGRES_SIZED {
        report.cover(key);
        let label = format!("db.command {name} ({key})");
        report.compare(
            FILE,
            &label,
            settings.expected(key),
            found.get(name).copied(),
            bare,
        );
    }
    report.cover("pg.shared_buffers");
    report.cover("container.db_memory");
    let label = "db.command shared_buffers (pg.shared_buffers)";
    report.compare(
        FILE,
        label,
        expected_shared_buffers(settings),
        found.get("shared_buffers").copied(),
        Bare::Plain,
    );
    report
}

/// The variables of `environment` of `app`, `migrate`, and `db` equal their keys in 7.11.
pub fn check_environment(compose: &Compose, table: &Table) -> Report {
    let mut report = Report::default();
    for (service_name, name, key, bare) in ENV_ROWS {
        report.cover(key);
        let Some(service) = compose.service(service_name) else {
            missing(&mut report, service_name);
            continue;
        };
        let label = format!("{service_name}.environment.{name} ({key})");
        let found = service.env(name);
        check_raw_digits(&mut report, &label, key, found);
        report.compare(FILE, &label, table.expected(key), found, *bare);
    }
    report
}

/// The keys that the application reads as a bare integer of the environment.
/// The two ports go through `raw_port_texts` in `src/config/serve.rs` and `parse_port` in
/// `src/port.rs`, which accept ASCII digits only. The two session timeouts and `db.port` are
/// `u64` and `u16` fields that the number reader of figment fills. A text with a unit, such as
/// `30min`, is a string for that reader, and the application refuses it. The canonical comparison
/// accepts such a text, so the digits rule runs before it.
const RAW_INTEGER_KEYS: [&str; 5] = [
    "http.public_port",
    "http.internal_port",
    "session.idle_timeout",
    "session.absolute_timeout",
    "db.port",
];

/// A bare integer key of the environment holds ASCII digits only: no sign, no dot, no space, no
/// exponent, no unit. A text such as `9000.0` equals 9000 as a number, but the application refuses
/// it at startup. A text such as `30min` equals 1800 seconds for the canonical comparison.
fn check_raw_digits(report: &mut Report, label: &str, key: &str, found: Option<&str>) {
    let Some(text) = found else {
        return;
    };
    let is_plain = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    if RAW_INTEGER_KEYS.contains(&key) && !is_plain {
        let rule = "ASCII digits only (a bare integer in src/config/serve.rs)";
        report.fail(FILE, label, rule, text);
    }
}

/// The port of a key of 7.11 as text, for example `9000`.
pub fn port_text(table: &Table, key: &str) -> Option<String> {
    match table.canon(key)? {
        Canon::Number(milli) => Some(milli.checked_div(1000)?.to_string()),
        _ => None,
    }
}

/// The ports of the two listeners, the `expose` list, and the two probe lines.
pub fn check_ports(compose: &Compose, table: &Table) -> Report {
    let mut report = Report::default();
    let (Some(app), Some(proxy), Some(db)) = (
        compose.service("app"),
        compose.service("proxy"),
        compose.service("db"),
    ) else {
        missing(&mut report, "app, proxy, or db");
        return report;
    };
    let public = port_text(table, "http.public_port").unwrap_or_default();
    let internal = port_text(table, "http.internal_port").unwrap_or_default();
    report.cover("http.public_port");
    report.cover("http.internal_port");
    let want_expose = [public, internal.clone()];
    report.compare_list(
        FILE,
        "app.expose (http.public_port, http.internal_port)",
        &want_expose,
        Some(app.list("expose")),
    );
    check_probe_lines(&mut report, app, proxy, &internal);
    check_pg_isready(&mut report, db, table);
    report
}

/// The probe of `proxy` (last item of its `test`) and the probe of `app` (`reliability.md` 7.1).
fn check_probe_lines(report: &mut Report, app: &Service, proxy: &Service, internal: &str) {
    let url = format!("http://app:{internal}/healthz");
    let probe = proxy
        .health("test")
        .and_then(flow_list)
        .and_then(|list| list.last().cloned());
    let want = [url];
    report.compare_list(
        FILE,
        "proxy.healthcheck URL (http.internal_port)",
        &want,
        probe.map(|last| vec![last]),
    );
    let want_app: Vec<String> = [
        "CMD",
        "/usr/local/bin/recipe-server",
        "healthcheck",
        "--path",
        "/readyz",
    ]
    .map(String::from)
    .to_vec();
    report.compare_list(
        FILE,
        "app.healthcheck.test (reliability.md 7.1)",
        &want_app,
        app.health("test").and_then(flow_list),
    );
}

/// The `pg_isready` line of `db` (`reliability.md` 7.1 row `db`, design D9 line "The healthcheck of `db`").
fn check_pg_isready(report: &mut Report, db: &Service, table: &Table) {
    report.cover("db.port");
    report.cover("db.name");
    let port = port_text(table, "db.port").unwrap_or_default();
    let name = match table.canon("db.name") {
        Some(Canon::Text(text)) => text,
        _ => String::new(),
    };
    let want: Vec<String> = [
        "CMD",
        "pg_isready",
        "-h",
        "127.0.0.1",
        "-p",
        &port,
        "-U",
        "pg_isready",
        "-d",
        &name,
    ]
    .map(String::from)
    .to_vec();
    report.compare_list(
        FILE,
        "db.healthcheck.test pg_isready line",
        &want,
        db.health("test").and_then(flow_list),
    );
}
