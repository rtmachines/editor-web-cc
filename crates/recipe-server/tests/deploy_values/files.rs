//! Checks of the values in `deploy/.env.example`, the Caddyfiles, the crontab files, and the pgtools scripts.

use std::collections::BTreeMap;

use crate::canon::{Bare, Canon, canon};
use crate::fault::Report;
use crate::lines::{code_lines, unquote, without_comment};
use crate::table::Table;

const ENV_FILE: &str = "deploy/.env.example";

/// The eight values of the Compose interpolation (`reliability.md` 7.1, "Secrets", and design D9).
/// The file holds no password, key, or token.
const ENV_KEYS: [&str; 8] = [
    "COMPOSE_IP_RANGE",
    "COMPOSE_SUBNET",
    "PROXY_ADDRESS",
    "RECIPE_BACKUP_DIR",
    "RECIPE_SECRETS_DIR",
    "RECIPE_VERSION",
    "RECIPE_WALARCHIVE_DIR",
    "SITE_ADDRESS",
];
/// The addresses of design D9 (the row `deploy/.env.example`) and of `reliability.md` 7.11 (`http.trusted_proxies`).
const ENV_ADDRESSES: [(&str, &str); 3] = [
    ("COMPOSE_SUBNET", "172.28.0.0/16"),
    ("COMPOSE_IP_RANGE", "172.28.1.0/24"),
    ("PROXY_ADDRESS", "172.28.0.2"),
];
/// A script assignment `NAME=value`: the file, the variable, the key of 7.11, and the unit of a bare number.
type Assignment = (&'static str, &'static str, &'static str, Bare);

/// The constants of the pgtools scripts. Each one has a comment with its key in the script.
pub const SCRIPT_ASSIGNMENTS: [Assignment; 11] = [
    (
        "backup-health",
        "MAX_AGE_SECONDS",
        "backup.max_age_alert",
        Bare::Seconds,
    ),
    (
        "backup-once",
        "RETENTION_DAYS",
        "backup.retention",
        Bare::Days,
    ),
    (
        "basebackup-once",
        "RETENTION_COUNT",
        "basebackup.retention",
        Bare::Plain,
    ),
    ("restore-test", "SCRATCH_PORT", "scratch.port", Bare::Plain),
    (
        "restore-test",
        "START_TIMEOUT_SECONDS",
        "scratch.restore_start_timeout",
        Bare::Seconds,
    ),
    (
        "restore-test",
        "STOP_TIMEOUT_SECONDS",
        "scratch.stop_timeout",
        Bare::Seconds,
    ),
    (
        "restore-test",
        "DETAIL_LENGTH",
        "restore_test.detail_length",
        Bare::Plain,
    ),
    ("pitr-check", "SCRATCH_PORT", "scratch.port", Bare::Plain),
    (
        "pitr-check",
        "START_TIMEOUT_SECONDS",
        "scratch.pitr_start_timeout",
        Bare::Seconds,
    ),
    (
        "pitr-check",
        "STOP_TIMEOUT_SECONDS",
        "scratch.stop_timeout",
        Bare::Seconds,
    ),
    (
        "pitr-check",
        "LOG_TAIL_LINES",
        "pitr_check.log_tail",
        Bare::Plain,
    ),
];
/// The settings of the scratch clusters that both scripts set (`reliability.md` 7.11 rows `scratch.*`).
const SCRATCH_COMMON: [(&str, &str); 5] = [
    ("shared_buffers", "scratch.shared_buffers"),
    ("maintenance_work_mem", "scratch.maintenance_work_mem"),
    ("max_connections", "scratch.max_connections"),
    (
        "max_parallel_workers_per_gather",
        "scratch.max_parallel_workers_per_gather",
    ),
    (
        "max_parallel_maintenance_workers",
        "scratch.max_parallel_maintenance_workers",
    ),
];
/// The settings of the cluster of `restore-test` only (ADR-0067 (f)).
const SCRATCH_RESTORE: [(&str, &str); 3] = [
    ("fsync", "scratch.restore_fsync"),
    ("wal_level", "scratch.restore_wal_level"),
    ("max_wal_senders", "scratch.restore_max_wal_senders"),
];
/// The setting of the cluster of `pitr-check` only (ADR-0068 (o)).
const SCRATCH_PITR: [(&str, &str); 1] = [("hot_standby", "scratch.pitr_hot_standby")];

/// The `NAME=value` lines of an env file.
pub fn parse_env(text: &str) -> BTreeMap<&str, &str> {
    code_lines(text)
        .into_iter()
        .filter_map(|line| line.trim().split_once('='))
        .collect()
}

/// The file holds exactly the eight values, and the three addresses equal design D9.
pub fn check_env_example(text: &str) -> Report {
    let mut report = Report::default();
    let env = parse_env(text);
    let found: Vec<String> = env.keys().map(ToString::to_string).collect();
    let want: Vec<String> = ENV_KEYS.map(String::from).to_vec();
    report.compare_list(ENV_FILE, "the names of the values", &want, Some(found));
    for (name, value) in ENV_ADDRESSES {
        let found = env.get(name).copied();
        if found != Some(value) {
            report.fail(ENV_FILE, name, value, found.unwrap_or("missing"));
        }
    }
    report
}

/// The upstream keep-alive, the HSTS age, and the port of `reverse_proxy` of a Caddyfile
/// (`reliability.md` 7.11 rows `proxy.upstream_keepalive`, `http.hsts_max_age`, `http.public_port`).
pub fn check_caddyfile(file: &str, text: &str, table: &Table) -> Report {
    let mut report = Report::default();
    let lines = code_lines(text);
    let keepalive = lines
        .iter()
        .find_map(|line| line.trim().strip_prefix("keepalive "));
    report.cover("proxy.upstream_keepalive");
    report.compare(
        file,
        "keepalive (proxy.upstream_keepalive)",
        table.expected("proxy.upstream_keepalive"),
        keepalive,
        Bare::Plain,
    );
    if let Some(fault) = keepalive_below_body_read(keepalive, table) {
        report.fail(
            file,
            "keepalive (proxy.upstream_keepalive)",
            fault,
            keepalive.unwrap_or("missing"),
        );
    }
    check_hsts(&mut report, file, &lines, table);
    let target = lines
        .iter()
        .find_map(|line| line.trim().strip_prefix("reverse_proxy app:"));
    let port = target.and_then(|rest| rest.split_whitespace().next());
    report.cover("http.public_port");
    report.compare(
        file,
        "reverse_proxy port (http.public_port)",
        table.expected("http.public_port"),
        port,
        Bare::Plain,
    );
    report
}

/// The directives of the line `header ?Strict-Transport-Security "<directives>"`. The header name
/// must be the whole name, with or without the mark `?` of Caddy.
fn hsts_directives(lines: &[&str]) -> Vec<String> {
    let value = lines.iter().find_map(|line| {
        let rest = line.trim().strip_prefix("header ")?;
        let (name, value) = rest.split_once(' ')?;
        matches!(
            name,
            "?Strict-Transport-Security" | "Strict-Transport-Security"
        )
        .then_some(value)
    });
    value
        .map(unquote)
        .unwrap_or_default()
        .split(';')
        .map(|directive| directive.trim().to_owned())
        .collect()
}

/// The `max-age` of the HSTS header equals `http.hsts_max_age`, and the header has `includeSubDomains`.
fn check_hsts(report: &mut Report, file: &str, lines: &[&str], table: &Table) {
    let directives = hsts_directives(lines);
    let age = directives
        .iter()
        .find_map(|directive| directive.strip_prefix("max-age="));
    report.cover("http.hsts_max_age");
    report.compare(
        file,
        "Strict-Transport-Security max-age (http.hsts_max_age)",
        table.expected("http.hsts_max_age"),
        age,
        Bare::Seconds,
    );
    if !directives
        .iter()
        .any(|directive| directive == "includeSubDomains")
    {
        report.fail(
            file,
            "Strict-Transport-Security",
            "includeSubDomains",
            "missing",
        );
    }
}

/// The rule `proxy.upstream_keepalive` is below `http.body_read_timeout` (`reliability.md` 7.1 and 7.11).
/// Returns the text of the fault, or `None` when the rule holds or the keep-alive is missing.
fn keepalive_below_body_read(keepalive: Option<&str>, table: &Table) -> Option<String> {
    let Some(Canon::Millis(keep)) = keepalive.and_then(|text| canon(text, Bare::Plain)) else {
        return None;
    };
    let body = table.millis("http.body_read_timeout")?;
    (keep >= body).then(|| format!("below {body} ms (http.body_read_timeout)"))
}

/// The cron prefix `minute hour * * weekday` and the zone word of a time such as `Sunday 03:30 UTC`.
fn cron_schedule(raw: &str) -> Option<(String, String)> {
    let cleaned = raw.replace('`', "");
    let mut words = cleaned.split_whitespace();
    let first = words.next()?;
    let (weekday, time) = if first == "Sunday" {
        ("0", words.next()?)
    } else {
        ("*", first)
    };
    let (hour, minute) = time.split_once(':')?;
    let (hour, minute): (u32, u32) = (hour.parse().ok()?, minute.parse().ok()?);
    let zone = words
        .next()?
        .trim_matches(|c: char| !(c.is_alphanumeric() || c == '/' || c == '_'));
    // A zone word has letters, `/`, and `_` only, so `ADR-0067` after a time is no zone.
    let is_zone = !zone.is_empty()
        && zone
            .chars()
            .all(|c| c.is_alphabetic() || c == '/' || c == '_');
    is_zone.then(|| (format!("{minute} {hour} * * {weekday}"), zone.to_owned()))
}

/// A crontab file runs each command at the time of its key in 7.11, in the zone of the document.
/// The file holds those jobs and no other job, each once. `reliability.md` 7.11, ADR-0066.
pub fn check_crontab(file: &str, text: &str, entries: &[(&str, &str)], table: &Table) -> Report {
    let mut report = Report::default();
    let lines: Vec<&str> = code_lines(text).into_iter().map(str::trim).collect();
    let mut wanted: Vec<String> = Vec::new();
    let mut zones: Vec<String> = Vec::new();
    for (key, command) in entries {
        report.cover(key);
        let Some((prefix, zone)) = table.raw(key).and_then(cron_schedule) else {
            report.fail(file, key, "a time and a zone in 7.11", "no time or no zone");
            continue;
        };
        let want = format!("{prefix} {command}");
        if !lines.contains(&want.as_str()) {
            let found = lines
                .iter()
                .find(|line| line.ends_with(command))
                .copied()
                .unwrap_or("no line");
            report.fail(file, &format!("{key} ({command})"), want.as_str(), found);
        }
        wanted.push(want);
        let zone_line = format!("CRON_TZ={zone}");
        if !zones.contains(&zone_line) {
            zones.push(zone_line);
        }
    }
    check_cron_zones(&mut report, file, &lines, &zones);
    check_cron_extra_jobs(&mut report, file, &lines, wanted);
    report
}

/// The file sets the zone once per expected zone. A later `CRON_TZ=` would change the zone below it.
fn check_cron_zones(report: &mut Report, file: &str, lines: &[&str], want: &[String]) {
    let found: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| line.starts_with("CRON_TZ="))
        .collect();
    if found != want {
        report.fail(file, "CRON_TZ", format!("{want:?}"), format!("{found:?}"));
    }
}

/// Every code line except the zone lines is an expected job. Each expected job matches one line.
fn check_cron_extra_jobs(report: &mut Report, file: &str, lines: &[&str], wanted: Vec<String>) {
    let mut jobs: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| !line.starts_with("CRON_TZ="))
        .collect();
    for want in &wanted {
        if let Some(at) = jobs.iter().position(|line| *line == want.as_str()) {
            jobs.remove(at);
        }
    }
    for extra in jobs {
        report.fail(file, "unexpected cron job", "no other job", extra);
    }
}

/// The words that may stand before an assignment in Bash and still set the variable.
const ASSIGNMENT_PREFIXES: [&str; 4] = ["export", "readonly", "declare", "local"];

/// The words of a line that Bash reads as assignments. A line starts with an assignment, or with one
/// of `ASSIGNMENT_PREFIXES`, then option words such as `-r`, then the assignments.
fn assignment_words(line: &str) -> Vec<&str> {
    let words: Vec<&str> = without_comment(line).split_whitespace().collect();
    if !words
        .first()
        .is_some_and(|first| ASSIGNMENT_PREFIXES.contains(first))
    {
        return words.into_iter().take(1).collect();
    }
    words
        .into_iter()
        .skip(1)
        .filter(|word| !word.starts_with('-'))
        .collect()
}

/// The values of every assignment `NAME=value` of a name in the code lines. The shell runs all of
/// them, so a check must see them all and not only the last one.
fn assignment_values<'a>(lines: &[&'a str], name: &str) -> Vec<&'a str> {
    lines
        .iter()
        .copied()
        .flat_map(assignment_words)
        .filter_map(|word| word.strip_prefix(name)?.strip_prefix('='))
        .map(unquote)
        .collect()
}

/// A fault for a name that the file assigns zero times or more than once.
fn fail_not_once(report: &mut Report, file: &str, label: &str, count: usize) {
    report.fail(
        file,
        label,
        "exactly one assignment",
        format!("{count} assignments"),
    );
}

/// A constant of a script is a decimal integer without a leading zero and without a unit.
/// Bash arithmetic reads `030` as octal and rejects `720h`. The document gives the unit.
fn is_plain_integer(value: &str) -> bool {
    let digits = !value.is_empty() && value.chars().all(|c| c.is_ascii_digit());
    digits && (value.len() == 1 || !value.starts_with('0'))
}

/// Every word `name=value` of a script, without quotes, on any code line. A word in an `echo`
/// line counts too, so no later word can hide the real setting.
fn setting_words(lines: &[&str], name: &str) -> Vec<String> {
    lines
        .iter()
        .flat_map(|line| without_comment(line).split_whitespace())
        .filter_map(|word| {
            let word = word.trim_matches(['\'', '"']);
            word.strip_prefix(name)?
                .strip_prefix('=')
                .map(str::to_owned)
        })
        .collect()
}

/// A script sets the constants of `SCRIPT_ASSIGNMENTS` for its name to the values of 7.11.
pub fn check_script_assignments(script: &str, text: &str, table: &Table) -> Report {
    let mut report = Report::default();
    let lines = code_lines(text);
    for (file, name, key, bare) in SCRIPT_ASSIGNMENTS {
        if file != script {
            continue;
        }
        report.cover(key);
        let label = format!("{name} ({key})");
        let path = format!("deploy/pgtools/bin/{script}");
        let values = assignment_values(&lines, name);
        let [found] = values.as_slice() else {
            fail_not_once(&mut report, &path, &label, values.len());
            continue;
        };
        report.compare(&path, &label, table.expected(key), Some(found), bare);
        if !is_plain_integer(found) {
            report.fail(
                &path,
                &label,
                "a decimal integer without a unit or a leading zero",
                *found,
            );
        }
    }
    report
}

/// A script sets the settings of its scratch cluster to the values of 7.11.
/// `pitr-check` sets no `fsync` option (ADR-0067 (f)).
pub fn check_scratch_settings(script: &str, text: &str, table: &Table) -> Report {
    let mut report = Report::default();
    let lines = code_lines(text);
    let file = format!("deploy/pgtools/bin/{script}");
    let own: &[(&str, &str)] = if script == "restore-test" {
        &SCRATCH_RESTORE
    } else {
        &SCRATCH_PITR
    };
    for (name, key) in SCRATCH_COMMON.iter().chain(own) {
        report.cover(key);
        let label = format!("{name} ({key})");
        let words = setting_words(&lines, name);
        let [found] = words.as_slice() else {
            fail_not_once(&mut report, &file, &label, words.len());
            continue;
        };
        report.compare(
            &file,
            &label,
            table.expected(key),
            Some(found.as_str()),
            Bare::Plain,
        );
    }
    if script == "pitr-check" && !setting_words(&lines, "fsync").is_empty() {
        report.fail(&file, "fsync", "no fsync setting", "a fsync setting");
    }
    report
}
