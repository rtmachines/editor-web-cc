//! Checks of the security options, the networks, the logging anchor, and the secrets of `deploy/compose.yaml`.
//!
//! The rules come from `reliability.md` 7.1 ("Networks and capabilities", "Secrets"), 7.8, and design D9.

use std::collections::BTreeMap;

use crate::canon::{Bare, Canon};
use crate::fault::Report;
use crate::lines::{Compose, Service, field, field_at, section, unquote};
use crate::table::Table;

const FILE: &str = "deploy/compose.yaml";
/// The log driver of the logging anchor (`reliability.md` 3.5, ADR-0070 (s)).
const LOG_DRIVER: &str = "json-file";

/// The seven services of `reliability.md` 7.1 and the CI check `pitr-check` (design D9).
const SERVICES: [&str; 8] = [
    "app",
    "backup",
    "backup-premigrate",
    "db",
    "migrate",
    "pitr-check",
    "proxy",
    "walarchive",
];
/// The services that mount a tmpfs on `/run/pgpass` (`reliability.md` 7.1, "Networks and capabilities").
const PGPASS_SERVICES: [&str; 3] = ["backup", "backup-premigrate", "walarchive"];
/// The services with `read_only: true` (`reliability.md` 7.1).
const READ_ONLY_SERVICES: [&str; 2] = ["app", "proxy"];
/// The Docker secrets of `reliability.md` 7.1 and design D9, one host file for each.
const SECRETS: [&str; 10] = [
    "postgres_password",
    "migrator_password",
    "app_password",
    "backup_password",
    "replicator_password",
    "login_hmac_key",
    "csrf_signing_key",
    "alerts_webhook_url",
    "tls_cert",
    "tls_key",
];
const SECRET_DIR_VARIABLE: &str = "${RECIPE_SECRETS_DIR:?}";
const PROXY_VARIABLE: &str = "${PROXY_ADDRESS:?}";
/// The mount path of the secrets inside a container (`reliability.md` 7.11, keys `*_file`).
const SECRET_MOUNT: &str = "/run/secrets/";

fn expect_scalar(report: &mut Report, service: &Service, field_name: &str, want: &str) {
    let found = service.scalar(field_name);
    if found != Some(want) {
        let label = format!("{}.{field_name}", service.name);
        report.fail(FILE, &label, want, found.unwrap_or("missing"));
    }
}

fn check_service_options(report: &mut Report, service: &Service) {
    let name = service.name;
    let drop_all = ["ALL".to_owned()];
    let no_new = ["no-new-privileges:true".to_owned()];
    let add: Vec<String> = if name == "proxy" {
        vec!["NET_BIND_SERVICE".to_owned()]
    } else {
        Vec::new()
    };
    report.compare_list(
        FILE,
        &format!("{name}.cap_drop"),
        &drop_all,
        Some(service.list("cap_drop")),
    );
    report.compare_list(
        FILE,
        &format!("{name}.security_opt"),
        &no_new,
        Some(service.list("security_opt")),
    );
    report.compare_list(
        FILE,
        &format!("{name}.cap_add"),
        &add,
        Some(service.list("cap_add")),
    );
    if name != "proxy" && !service.list("ports").is_empty() {
        report.fail(
            FILE,
            &format!("{name}.ports"),
            "no published port",
            "a ports list",
        );
    }
}

/// The reader cannot follow a YAML merge key, so a `<<:` line inside a service is a fault.
fn check_merge_keys(report: &mut Report, service: &Service) {
    for line in service.merge_lines() {
        report.fail(
            FILE,
            &format!("{}.<<", service.name),
            "no merge key",
            line.trim(),
        );
    }
}

fn check_special_options(report: &mut Report, compose: &Compose) {
    for name in READ_ONLY_SERVICES {
        if let Some(service) = compose.service(name) {
            expect_scalar(report, service, "read_only", "true");
        }
    }
    if let Some(db) = compose.service("db") {
        expect_scalar(report, db, "user", "999:999");
    }
    let pgpass = ["/run/pgpass".to_owned()];
    for name in PGPASS_SERVICES {
        if let Some(service) = compose.service(name) {
            report.compare_list(
                FILE,
                &format!("{name}.tmpfs"),
                &pgpass,
                Some(service.list("tmpfs")),
            );
        }
    }
    if let Some(proxy) = compose.service("proxy")
        && !proxy
            .list("volumes")
            .iter()
            .any(|item| item == "caddy:/data")
    {
        report.fail(FILE, "proxy.volumes", "caddy:/data", "missing");
    }
}

fn check_service_networks(report: &mut Report, compose: &Compose) {
    for service in &compose.services {
        let want: &[&str] = match service.name {
            "proxy" => &["backend", "edge"],
            "app" => &["backend", "egress"],
            "pitr-check" => &[],
            _ => &["backend"],
        };
        let want: Vec<String> = want.iter().map(|name| (*name).to_owned()).collect();
        report.compare_list(
            FILE,
            &format!("{}.networks", service.name),
            &want,
            Some(service.networks()),
        );
        check_isolation(report, service);
    }
}

/// `pitr-check` has `network_mode: none` and mounts no secret. No other service has a `network_mode`.
/// Design D10 and the comment above `pitr-check` in `compose.yaml`.
fn check_isolation(report: &mut Report, service: &Service) {
    let mode = service.scalar("network_mode");
    if service.name != "pitr-check" {
        if let Some(found) = mode {
            report.fail(
                FILE,
                &format!("{}.network_mode", service.name),
                "no network_mode",
                found,
            );
        }
        return;
    }
    expect_scalar(report, service, "network_mode", "none");
    let secrets = service.list("secrets");
    if !secrets.is_empty() {
        report.fail(
            FILE,
            "pitr-check.secrets",
            "no secret",
            format!("{secrets:?}"),
        );
    }
}

/// Every service drops all capabilities and sets `no-new-privileges`. Only `proxy` adds `NET_BIND_SERVICE`.
/// Only `proxy` publishes a port. `reliability.md` 7.1, "Networks and capabilities".
pub fn check_security(compose: &Compose) -> Report {
    let mut report = Report::default();
    let found: Vec<String> = {
        let mut names: Vec<String> = compose.services.iter().map(|s| s.name.to_owned()).collect();
        names.sort();
        names
    };
    let want: Vec<String> = SERVICES.map(String::from).to_vec();
    report.compare_list(FILE, "services", &want, Some(found));
    for service in &compose.services {
        check_service_options(&mut report, service);
        check_merge_keys(&mut report, service);
    }
    check_special_options(&mut report, compose);
    check_service_networks(&mut report, compose);
    report
}

/// The logging anchor uses `log.max_size`, `log.max_file`, `log.mode`, and `log.max_buffer_size`.
/// Each of the seven services uses it. `reliability.md` 7.11 rows `log.*`, design D9 last rule.
pub fn check_logging(compose: &Compose, table: &Table) -> Report {
    let mut report = Report::default();
    let anchor = section(&compose.all, 0, "x-logging: &logging");
    let options = section(&anchor, 2, "options:");
    let driver = field_at(&anchor, 2, "driver");
    if driver != Some(LOG_DRIVER) {
        report.fail(
            FILE,
            "x-logging driver",
            LOG_DRIVER,
            driver.unwrap_or("missing"),
        );
    }
    let rows = [
        ("max-size", "log.max_size"),
        ("max-file", "log.max_file"),
        ("mode", "log.mode"),
        ("max-buffer-size", "log.max_buffer_size"),
    ];
    for (name, key) in rows {
        report.cover(key);
        let found = field_at(&options, 4, name);
        report.compare(
            FILE,
            &format!("x-logging options {name} ({key})"),
            table.expected(key),
            found,
            Bare::Plain,
        );
    }
    for service in &compose.services {
        if service.name != "pitr-check" && service.scalar("logging") != Some("*logging") {
            report.fail(
                FILE,
                &format!("{}.logging", service.name),
                "*logging",
                "missing",
            );
        }
    }
    report
}

/// The network `backend` is internal and has the subnet and the `ip_range` of the variables.
/// `proxy` has the address `PROXY_ADDRESS` on it. `reliability.md` 7.1, design D9 first rule.
pub fn check_backend_network(compose: &Compose, env: &BTreeMap<&str, &str>) -> Report {
    let mut report = Report::default();
    let networks = section(&compose.all, 0, "networks:");
    let backend = section(&networks, 2, "backend:");
    for (name, want) in [
        ("internal", "true"),
        ("subnet", "${COMPOSE_SUBNET:?}"),
        ("ip_range", "${COMPOSE_IP_RANGE:?}"),
    ] {
        let found = field(&backend, name);
        if found != Some(want) {
            report.fail(
                FILE,
                &format!("networks.backend.{name}"),
                want,
                found.unwrap_or("missing"),
            );
        }
    }
    if let Some(proxy) = compose.service("proxy") {
        let networks = proxy.section("networks");
        let found = field(&section(&networks, 6, "backend:"), "ipv4_address");
        if found != Some(PROXY_VARIABLE) {
            report.fail(
                FILE,
                "proxy.networks.backend.ipv4_address",
                PROXY_VARIABLE,
                found.unwrap_or("missing"),
            );
        }
    }
    report.absorb(check_address_plan(env));
    report
}

/// `PROXY_ADDRESS` lies inside `COMPOSE_SUBNET` and outside `COMPOSE_IP_RANGE`. The range lies inside the subnet.
pub fn check_address_plan(env: &BTreeMap<&str, &str>) -> Report {
    let mut report = Report::default();
    let subnet = env.get("COMPOSE_SUBNET").and_then(|text| parse_cidr(text));
    let range = env
        .get("COMPOSE_IP_RANGE")
        .and_then(|text| parse_cidr(text));
    let proxy = env.get("PROXY_ADDRESS").and_then(|text| parse_addr(text));
    let (Some(subnet), Some(range), Some(proxy)) = (subnet, range, proxy) else {
        report.fail(
            "deploy/.env.example",
            "COMPOSE_SUBNET, COMPOSE_IP_RANGE, PROXY_ADDRESS",
            "an IPv4 CIDR, an IPv4 CIDR, and an IPv4 address",
            "a value that does not parse",
        );
        return report;
    };
    let range_inside = range.1 >= subnet.1 && cidr_contains(subnet, range.0);
    if !range_inside {
        report.fail(
            "deploy/.env.example",
            "COMPOSE_IP_RANGE",
            "inside COMPOSE_SUBNET",
            "outside",
        );
    }
    if !cidr_contains(subnet, proxy) {
        report.fail(
            "deploy/.env.example",
            "PROXY_ADDRESS",
            "inside COMPOSE_SUBNET",
            "outside",
        );
    }
    if cidr_contains(range, proxy) {
        report.fail(
            "deploy/.env.example",
            "PROXY_ADDRESS",
            "outside COMPOSE_IP_RANGE",
            "inside",
        );
    }
    report
}

/// One variable feeds `http.trusted_proxies` of `app` and the address of `proxy`.
/// The result is the default of 7.11 (`172.28.0.2/32`).
pub fn check_trusted_proxies(
    compose: &Compose,
    env: &BTreeMap<&str, &str>,
    table: &Table,
) -> Report {
    let mut report = Report::default();
    report.cover("http.trusted_proxies");
    let raw = compose
        .service("app")
        .and_then(|app| app.env("RECIPE_HTTP__TRUSTED_PROXIES"));
    let label = "app.environment.RECIPE_HTTP__TRUSTED_PROXIES (http.trusted_proxies)";
    let Some(raw) = raw.filter(|text| text.starts_with(PROXY_VARIABLE)) else {
        report.fail(
            FILE,
            label,
            format!("{PROXY_VARIABLE}/32"),
            raw.unwrap_or("missing"),
        );
        return report;
    };
    let address = env.get("PROXY_ADDRESS").copied().unwrap_or_default();
    let found = raw.replace(PROXY_VARIABLE, address);
    let expected = table
        .literal("http.trusted_proxies")
        .map(|text| (Canon::Text(text), table.shown("http.trusted_proxies")));
    report.compare(FILE, label, expected, Some(&found), Bare::Plain);
    report
}

/// Each secret has the host file `${RECIPE_SECRETS_DIR:?}/<name>`. `app` and `migrate` list each secret
/// that one of their `*_FILE` variables names. `reliability.md` 7.1, "Secrets".
pub fn check_secrets(compose: &Compose) -> Report {
    let mut report = Report::default();
    let secrets = section(&compose.all, 0, "secrets:");
    for name in SECRETS {
        let block = section(&secrets, 2, &format!("{name}:"));
        let want = format!("{SECRET_DIR_VARIABLE}/{name}");
        let found = field(&block, "file");
        if found != Some(want.as_str()) {
            report.fail(
                FILE,
                &format!("secrets.{name}.file"),
                want,
                found.unwrap_or("missing"),
            );
        }
    }
    for service_name in ["app", "migrate"] {
        let Some(service) = compose.service(service_name) else {
            continue;
        };
        let listed = service.list("secrets");
        for line in service.section("environment") {
            let value = line.split_once(':').map_or("", |(_, rest)| unquote(rest));
            if let Some(name) = value.strip_prefix(SECRET_MOUNT)
                && !listed.iter().any(|item| item == name)
            {
                report.fail(FILE, &format!("{service_name}.secrets"), name, "not listed");
            }
        }
    }
    report
}

/// An IPv4 address as a number.
fn parse_addr(text: &str) -> Option<u32> {
    if text.split('.').count() != 4 {
        return None;
    }
    let mut value: u32 = 0;
    for part in text.split('.') {
        let octet: u8 = part.parse().ok()?;
        value = value.checked_mul(256)?.checked_add(u32::from(octet))?;
    }
    Some(value)
}

/// An IPv4 CIDR as the address and the prefix length.
fn parse_cidr(text: &str) -> Option<(u32, u32)> {
    let (address, prefix) = text.split_once('/')?;
    let prefix: u32 = prefix.parse().ok().filter(|length| *length <= 32)?;
    Some((parse_addr(address)?, prefix))
}

/// True when the address lies inside the network. A prefix over 32 is no network, so it contains nothing.
fn cidr_contains(net: (u32, u32), address: u32) -> bool {
    let Some(shift) = 32u32.checked_sub(net.1) else {
        return false;
    };
    let mask = u32::MAX.checked_shl(shift).unwrap_or(0);
    (address & mask) == (net.0 & mask)
}
