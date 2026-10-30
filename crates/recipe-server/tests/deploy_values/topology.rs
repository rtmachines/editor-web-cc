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

/// The seven services of `reliability.md` 7.1, the CI check `pitr-check` (design D9), and the
/// restore service `restore-volume` (ADR-0103 (h)).
const SERVICES: [&str; 9] = [
    "app",
    "backup",
    "backup-premigrate",
    "db",
    "migrate",
    "pitr-check",
    "proxy",
    "restore-volume",
    "walarchive",
];
/// The services that mount a tmpfs on `/run/pgpass` (`reliability.md` 7.1, "Networks and capabilities").
const PGPASS_SERVICES: [&str; 4] = [
    "backup",
    "backup-premigrate",
    "restore-volume",
    "walarchive",
];
/// The one-shot services outside the seven. Each has `network_mode: none`, no secret, and no network.
const ISOLATED_SERVICES: [&str; 2] = ["pitr-check", "restore-volume"];
/// The service that fills a new volume (ADR-0103 (h), design D6) and its mounts and variable.
const RESTORE_SERVICE: &str = "restore-volume";
const RESTORE_PROFILE: &str = "restore";
/// The backup disk is read-only. The operator mounts the target volume with `-v` at run time.
const RESTORE_BACKUPS_MOUNT: &str = "backups:/backups:ro";
const RESTORE_SUBNET_VARIABLE: &str = "${COMPOSE_SUBNET:?}";
/// The stamp of the set is the argument of the run, so the entrypoint is the script itself.
const RESTORE_ENTRYPOINT: &str = "restore-volume";
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
/// The bind mount of the shared client rules into `db` (ADR-0103 (l), design D1).
const CLUSTER_ACCESS_MOUNT: &str = "./db/cluster-access.sh:/etc/recipe-db/cluster-access.sh:ro";
/// The file name of those rules. No other service mounts it as source or as target.
const CLUSTER_ACCESS_FILE: &str = "cluster-access.sh";

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
    check_cluster_access_mount(report, compose);
    check_restore_service(report, compose);
    if let Some(proxy) = compose.service("proxy")
        && !proxy
            .list("volumes")
            .iter()
            .any(|item| item == "caddy:/data")
    {
        report.fail(FILE, "proxy.volumes", "caddy:/data", "missing");
    }
}

/// `db` mounts the shared client rules read-only. No other service mounts that file.
fn check_cluster_access_mount(report: &mut Report, compose: &Compose) {
    for service in &compose.services {
        let volumes = service.list("volumes");
        if service.name == "db" {
            if !volumes.iter().any(|item| item == CLUSTER_ACCESS_MOUNT) {
                report.fail(FILE, "db.volumes", CLUSTER_ACCESS_MOUNT, "missing");
            }
        } else if let Some(item) = volumes
            .iter()
            .find(|item| item.contains(CLUSTER_ACCESS_FILE))
        {
            report.fail(
                FILE,
                &format!("{}.volumes", service.name),
                "no mount of cluster-access.sh",
                item,
            );
        }
    }
}

fn check_service_networks(report: &mut Report, compose: &Compose) {
    for service in &compose.services {
        let want: &[&str] = match service.name {
            "proxy" => &["backend", "edge"],
            "app" => &["backend", "egress"],
            "pitr-check" | RESTORE_SERVICE => &[],
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

/// `pitr-check` and `restore-volume` have `network_mode: none` and mount no secret.
/// No other service has a `network_mode`. Design D10 and the comments in `compose.yaml`.
fn check_isolation(report: &mut Report, service: &Service) {
    let mode = service.scalar("network_mode");
    if !ISOLATED_SERVICES.contains(&service.name) {
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
            &format!("{}.secrets", service.name),
            "no secret",
            format!("{secrets:?}"),
        );
    }
}

/// `restore-volume` has the profile `restore`, so `docker compose up` never starts it.
/// It mounts the backup disk read-only and nothing else: no `pgdata`, no `cluster-access.sh`.
/// The image holds the copy of the client rules. It gets `COMPOSE_SUBNET` for those rules.
/// The entrypoint is the script, so the stamp of `docker compose run` is its argument.
/// ADR-0103 (h), (l), design D4 and D6.
fn check_restore_service(report: &mut Report, compose: &Compose) {
    let Some(service) = compose.service(RESTORE_SERVICE) else {
        return;
    };
    let name = service.name;
    let profile = [RESTORE_PROFILE.to_owned()];
    report.compare_list(
        FILE,
        &format!("{name}.profiles"),
        &profile,
        Some(service.list("profiles")),
    );
    let mounts = [RESTORE_BACKUPS_MOUNT.to_owned()];
    report.compare_list(
        FILE,
        &format!("{name}.volumes"),
        &mounts,
        Some(service.list("volumes")),
    );
    let entrypoint = [RESTORE_ENTRYPOINT.to_owned()];
    report.compare_list(
        FILE,
        &format!("{name}.entrypoint"),
        &entrypoint,
        Some(service.list("entrypoint")),
    );
    if service.scalar("command").is_some() {
        report.fail(
            FILE,
            &format!("{name}.command"),
            "no command",
            "a command key",
        );
    }
    let subnet = service.env("COMPOSE_SUBNET");
    if subnet != Some(RESTORE_SUBNET_VARIABLE) {
        report.fail(
            FILE,
            &format!("{name}.environment.COMPOSE_SUBNET"),
            RESTORE_SUBNET_VARIABLE,
            subnet.unwrap_or("missing"),
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
    report.absorb(check_pgdata_volume(compose));
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
        if !ISOLATED_SERVICES.contains(&service.name)
            && service.scalar("logging") != Some("*logging")
        {
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

/// The volume `pgdata` is external and Compose takes its name from `RECIPE_PGDATA_VOLUME`.
/// It has no other key, so Compose never creates or deletes it (ADR-0103 (c), design D6).
fn check_pgdata_volume(compose: &Compose) -> Report {
    let mut report = Report::default();
    let volumes = section(&compose.all, 0, "volumes:");
    let pgdata = section(&volumes, 2, "pgdata:");
    for (name, want) in [("external", "true"), ("name", "${RECIPE_PGDATA_VOLUME:?}")] {
        let found = field_at(&pgdata, 4, name);
        if found != Some(want) {
            report.fail(
                FILE,
                &format!("volumes.pgdata.{name}"),
                want,
                found.unwrap_or("missing"),
            );
        }
    }
    if pgdata.len() != 2 {
        report.fail(
            FILE,
            "volumes.pgdata keys",
            "external and name only",
            pgdata.join(" ").trim().to_owned(),
        );
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
