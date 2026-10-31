//! Checks of the values in `deploy/.env.example`, the Caddyfiles, the crontab files, and the pgtools scripts.

use std::collections::{BTreeMap, BTreeSet};

use crate::canon::{Bare, Canon, canon};
use crate::fault::Report;
use crate::lines::{code_lines, unquote, without_comment};
use crate::table::Table;

const ENV_FILE: &str = "deploy/.env.example";

/// The nine values of the Compose interpolation (`reliability.md` 7.1, "Secrets", ADR-0103 (d), and design D9).
/// The file holds no password, key, or token.
const ENV_KEYS: [&str; 9] = [
    "COMPOSE_IP_RANGE",
    "COMPOSE_SUBNET",
    "PROXY_ADDRESS",
    "RECIPE_BACKUP_DIR",
    "RECIPE_PGDATA_VOLUME",
    "RECIPE_SECRETS_DIR",
    "RECIPE_VERSION",
    "RECIPE_WALARCHIVE_DIR",
    "SITE_ADDRESS",
];
const VOLUME_KEY: &str = "RECIPE_PGDATA_VOLUME";
const VOLUME_PREFIX: &str = "recipe-editor_pgdata_";
/// The addresses of design D9 (the row `deploy/.env.example`) and of `reliability.md` 7.11 (`http.trusted_proxies`).
const ENV_ADDRESSES: [(&str, &str); 3] = [
    ("COMPOSE_SUBNET", "172.28.0.0/16"),
    ("COMPOSE_IP_RANGE", "172.28.1.0/24"),
    ("PROXY_ADDRESS", "172.28.0.2"),
];
/// A script assignment `NAME=value`: the file, the variable, the key of 7.11, and the unit of a bare number.
type Assignment = (&'static str, &'static str, &'static str, Bare);

/// The constants of the pgtools scripts. Each one has a comment with its key in the script.
/// The file is a name in `deploy/pgtools/bin/`, or a name that ends in `.sh` in `deploy/pgtools/lib/`.
pub const SCRIPT_ASSIGNMENTS: [Assignment; 24] = [
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
    (
        "restore-volume",
        "DETAIL_LENGTH",
        "restore_test.detail_length",
        Bare::Plain,
    ),
    (
        "wal-identity.sh",
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
        "restore-volume",
        "START_TIMEOUT_SECONDS",
        "restore_volume.start_timeout",
        Bare::Seconds,
    ),
    (
        "restore-volume",
        "STOP_TIMEOUT_SECONDS",
        "restore_volume.stop_timeout",
        Bare::Seconds,
    ),
    (
        "pitr-check",
        "LOG_TAIL_LINES",
        "pitr_check.log_tail",
        Bare::Plain,
    ),
    (
        "pitr-check",
        "READY_POLL_SECONDS",
        "pitr_check.ready_poll",
        Bare::Seconds,
    ),
    (
        "pitr-volume",
        "START_TIMEOUT_SECONDS",
        "pitr_volume.start_timeout",
        Bare::Seconds,
    ),
    (
        "pitr-volume",
        "STOP_TIMEOUT_SECONDS",
        "pitr_volume.stop_timeout",
        Bare::Seconds,
    ),
    (
        "pitr-volume",
        "READY_POLL_SECONDS",
        "pitr_check.ready_poll",
        Bare::Seconds,
    ),
    (
        "pitr-volume",
        "LOG_TAIL_LINES",
        "pitr_check.log_tail",
        Bare::Plain,
    ),
    (
        "pitr-volume",
        "DISK_WARN_PERCENT",
        "disk.warn_percent",
        Bare::Percent,
    ),
    ("pitr-volume", "SCRATCH_PORT", "scratch.port", Bare::Plain),
    (
        "pitr-volume",
        "CLEAN_LENGTH",
        "restore_test.detail_length",
        Bare::Plain,
    ),
    (
        "pitr-volume",
        "DETAIL_LENGTH",
        "restore_test.detail_length",
        Bare::Plain,
    ),
];
/// The settings of the scratch clusters that the three scripts set (`reliability.md` 7.11 rows `scratch.*`).
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
/// The setting of the cluster of `pitr-check` only (ADR-0068 (o)). Its start options live in `recovery-steps.sh`.
const SCRATCH_PITR: [(&str, &str); 1] = [("hot_standby", "scratch.pitr_hot_standby")];

/// The `NAME=value` lines of an env file.
pub fn parse_env(text: &str) -> BTreeMap<&str, &str> {
    code_lines(text)
        .into_iter()
        .filter_map(|line| line.trim().split_once('='))
        .collect()
}

/// The file holds exactly the nine values, its header counts them, and the three addresses equal design D9.
/// The volume name has the form of ADR-0103 (e).
pub fn check_env_example(text: &str) -> Report {
    let mut report = Report::default();
    let env = parse_env(text);
    check_env_header(&mut report, text);
    check_volume_name(&mut report, &env);
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

/// The header line `# The file holds <count> values and no password, key, or token.` names the count of `ENV_KEYS`.
fn check_env_header(report: &mut Report, text: &str) {
    let header = text
        .lines()
        .find(|line| line.starts_with("# The file holds "))
        .unwrap_or("missing");
    if !header.starts_with("# The file holds nine values ") {
        report.fail(
            ENV_FILE,
            "header count",
            "# The file holds nine values",
            header,
        );
    }
}

/// `RECIPE_PGDATA_VOLUME` is `recipe-editor_pgdata_<YYYYMMDD_HHMMSS>` (ADR-0103 (e)).
fn check_volume_name(report: &mut Report, env: &BTreeMap<&str, &str>) {
    let found = env.get(VOLUME_KEY).copied().unwrap_or("missing");
    let stamp = found.strip_prefix(VOLUME_PREFIX).unwrap_or("");
    let is_stamp = stamp.split_once('_').is_some_and(|(date, time)| {
        date.len() == 8
            && time.len() == 6
            && date.chars().chain(time.chars()).all(|c| c.is_ascii_digit())
    });
    if !is_stamp {
        report.fail(
            ENV_FILE,
            VOLUME_KEY,
            "recipe-editor_pgdata_<YYYYMMDD_HHMMSS>",
            found,
        );
    }
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

/// A script or a library sets the constants of `SCRIPT_ASSIGNMENTS` for its name to the values of 7.11.
pub fn check_script_assignments(script: &str, text: &str, table: &Table) -> Report {
    let mut report = Report::default();
    let lines = code_lines(text);
    for (file, name, key, bare) in SCRIPT_ASSIGNMENTS {
        if file != script {
            continue;
        }
        report.cover(key);
        let label = format!("{name} ({key})");
        let directory = if script.ends_with(".sh") {
            "lib"
        } else {
            "bin"
        };
        let path = format!("deploy/pgtools/{directory}/{script}");
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

/// A script or a library sets the settings of its scratch cluster to the values of 7.11.
/// `recovery-steps.sh` holds the settings of `pitr-check` (ADR-0106 (h)).
/// Only `restore-test` sets a `fsync` option (ADR-0067 (f), ADR-0103 (j)).
pub fn check_scratch_settings(script: &str, text: &str, table: &Table) -> Report {
    let mut report = Report::default();
    let lines = code_lines(text);
    let directory = if script.ends_with(".sh") {
        "lib"
    } else {
        "bin"
    };
    let file = format!("deploy/pgtools/{directory}/{script}");
    let own: &[(&str, &str)] = match script {
        "restore-test" => &SCRATCH_RESTORE,
        "recovery-steps.sh" => &SCRATCH_PITR,
        _ => &[],
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
    if script != "restore-test" {
        report.absorb(check_no_fsync(&file, text));
    }
    report
}

/// A script or a library that starts no cluster of `restore-test` sets no `fsync` option.
pub fn check_no_fsync(file: &str, text: &str) -> Report {
    let mut report = Report::default();
    if !setting_words(&code_lines(text), "fsync").is_empty() {
        report.fail(file, "fsync", "no fsync setting", "a fsync setting");
    }
    report
}

/// The `Dockerfile` of the image `recipe-pgtools` and the allow lists of its two build contexts.
pub const PGTOOLS_DOCKERFILE: &str = "deploy/pgtools/Dockerfile";
pub const DB_CONTEXT_LIST: &str = "deploy/db/.dockerignore";
pub const PGTOOLS_CONTEXT_LIST: &str = "deploy/pgtools/.dockerignore";
/// The name of the second build context. It is `deploy/db` (`compose.build.yaml`, design D2).
const DB_CONTEXT: &str = "db";

/// Remove a leading `./` and a trailing `/`. A directory source `bin/` and the entry `!bin` then match.
fn context_name(path: &str) -> &str {
    path.trim_start_matches("./").trim_end_matches('/')
}

/// The names of the build stages: `FROM image AS name`. A `COPY --from=name` of a stage reads no context.
fn stage_names(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let (first, second_last) = (words.first()?, words.iter().rev().nth(1)?);
            (first.eq_ignore_ascii_case("FROM") && second_last.eq_ignore_ascii_case("AS"))
                .then(|| words.last().map(|name| (*name).to_owned()))
                .flatten()
        })
        .collect()
}

/// The sources of the `COPY` lines of the Dockerfile for each context: `(db, pgtools)`.
/// A source with `--from=db` belongs to the db context. A source without `--from` belongs to the
/// pgtools context. A wildcard or an unknown `--from` is a fault, because no list can name it.
fn copy_sources(report: &mut Report, dockerfile: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let joined = dockerfile.replace("\\\r\n", " ").replace("\\\n", " ");
    let lines = code_lines(&joined);
    let stages = stage_names(&lines);
    let (mut db, mut pgtools) = (BTreeSet::new(), BTreeSet::new());
    for line in lines {
        let mut words = line.split_whitespace();
        if !words
            .next()
            .is_some_and(|word| word.eq_ignore_ascii_case("COPY"))
        {
            continue;
        }
        let (flags, mut paths): (Vec<&str>, Vec<&str>) =
            words.partition(|word| word.starts_with("--"));
        paths.pop();
        let from = flags.iter().find_map(|flag| flag.strip_prefix("--from="));
        let target = match from {
            None => &mut pgtools,
            Some(DB_CONTEXT) => &mut db,
            Some(name) if stages.iter().any(|stage| stage == name) => continue,
            Some(name) => {
                report.fail(
                    PGTOOLS_DOCKERFILE,
                    line,
                    "--from of the context db or of a stage",
                    name,
                );
                continue;
            }
        };
        for path in paths {
            if path.contains(['*', '?', '[']) {
                report.fail(
                    PGTOOLS_DOCKERFILE,
                    path,
                    "a source without a wildcard",
                    line,
                );
            }
            target.insert(context_name(path).to_owned());
        }
    }
    (db, pgtools)
}

/// The names that an allow list admits. The first line of code is `*`. Every other line is `!name`.
fn admitted_names(report: &mut Report, file: &str, text: &str) -> Vec<String> {
    let lines = code_lines(text);
    if lines.first().map(|line| line.trim()) != Some("*") {
        report.fail(
            file,
            "first line",
            "*",
            lines.first().copied().unwrap_or("no line"),
        );
    }
    let mut names = Vec::new();
    for line in lines.iter().skip(1) {
        match line.trim().strip_prefix('!') {
            Some(name) => names.push(context_name(name).to_owned()),
            None => report.fail(file, line.trim(), "a line !name", "another line"),
        }
    }
    names.sort();
    names
}

/// Each allow list admits exactly the sources of the `COPY` lines for its context (ADR-0064 (f), ADR-0103 (l)).
/// A `COPY` source is a path of the Dockerfile. A directory source `bin/` is admitted as `!bin`.
/// A file in a directory is admitted by its own path, so `!lib` would admit too much.
/// The list of the other context never admits the same source. So `deploy/pgtools/tests/` stays out.
pub fn check_context_lists(dockerfile: &str, db_list: &str, pgtools_list: &str) -> Report {
    let mut report = Report::default();
    let (db, pgtools) = copy_sources(&mut report, dockerfile);
    for (file, list, sources) in [
        (DB_CONTEXT_LIST, db_list, db),
        (PGTOOLS_CONTEXT_LIST, pgtools_list, pgtools),
    ] {
        let admitted = admitted_names(&mut report, file, list);
        let want: Vec<String> = sources.into_iter().collect();
        report.compare_list(file, "the names that !name admits", &want, Some(admitted));
    }
    report
}

/// The directory of the library files in the repository, and the directory that holds them in the image.
const LIBRARY_PREFIX: &str = "deploy/pgtools/lib/";
const LIBRARY_DESTINATION: &str = "/etc/pgtools/";

/// The pairs `(source, destination)` of the `COPY` lines that read the pgtools context.
/// A `COPY` with `--from` reads another context or a stage, so it is not in the answer.
fn pgtools_copies(dockerfile: &str) -> Vec<(String, String)> {
    let joined = dockerfile.replace("\\\r\n", " ").replace("\\\n", " ");
    let mut copies = Vec::new();
    for line in code_lines(&joined) {
        let mut words = line.split_whitespace();
        if !words
            .next()
            .is_some_and(|word| word.eq_ignore_ascii_case("COPY"))
        {
            continue;
        }
        let (flags, mut paths): (Vec<&str>, Vec<&str>) =
            words.partition(|word| word.starts_with("--"));
        if flags.iter().any(|flag| flag.starts_with("--from=")) {
            continue;
        }
        let Some(destination) = paths.pop() else {
            continue;
        };
        copies.extend(
            paths
                .into_iter()
                .map(|path| (context_name(path).to_owned(), destination.to_owned())),
        );
    }
    copies
}

/// Each file of `deploy/pgtools/lib/` reaches the image `recipe-pgtools`. The Dockerfile copies it to
/// `/etc/pgtools/` with a `COPY` line, and the allow list of the pgtools context admits its own path.
/// A script of `bin/` sources the file from there, so a missing copy breaks the script in the image.
pub fn check_library_copies(dockerfile: &str, pgtools_list: &str, listing: &[String]) -> Report {
    let mut report = Report::default();
    let copies = pgtools_copies(dockerfile);
    let admitted = admitted_names(&mut Report::default(), PGTOOLS_CONTEXT_LIST, pgtools_list);
    for path in listing
        .iter()
        .filter(|path| path.starts_with(LIBRARY_PREFIX))
    {
        let source = path.strip_prefix("deploy/pgtools/").unwrap_or(path);
        match copies.iter().find(|(name, _)| name == source) {
            None => report.fail(
                PGTOOLS_DOCKERFILE,
                source,
                "a COPY line from the pgtools context",
                "no COPY line",
            ),
            Some((_, destination)) if destination != LIBRARY_DESTINATION => report.fail(
                PGTOOLS_DOCKERFILE,
                source,
                format!("the destination {LIBRARY_DESTINATION}"),
                destination.as_str(),
            ),
            Some(_) => {}
        }
        if !admitted.iter().any(|name| name == source) {
            report.fail(
                PGTOOLS_CONTEXT_LIST,
                source,
                "an entry !name with its own path",
                "no entry",
            );
        }
    }
    report
}

/// The path of the attribute file, as the faults of `check_line_endings` show it.
const ATTRIBUTES_FILE: &str = ".gitattributes";

/// True when the glob `pattern` matches the whole `text`. `*` matches any run of characters but `/`.
/// `?` matches one character but `/`. Every other character matches itself.
fn glob_matches(pattern: &[char], text: &[char]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some(('*', rest)) => {
            for skip in 0..=text.len() {
                if glob_matches(rest, text.get(skip..).unwrap_or_default()) {
                    return true;
                }
                if text.get(skip) == Some(&'/') {
                    return false;
                }
            }
            false
        }
        Some((c, rest)) => text.split_first().is_some_and(|(first, tail)| {
            ((*c == '?' && *first != '/') || c == first) && glob_matches(rest, tail)
        }),
    }
}

/// True when some line of `.gitattributes` sets `text` and `eol=lf` for the path.
/// Supported subset: a comment, an empty line, and `pattern attribute...`.
/// A pattern is a path from the repository root, or a name without `/` that matches at any depth.
/// A pattern may use `*` and `?`. It never uses `**`, a trailing `/`, `!`, or a character class.
/// An attribute is `text` or `eol=lf` as a plain word. The check does not model `-text` or the
/// override order of Git, so a later line `-text` for the same path is outside its scope.
fn has_lf_rule(attributes: &str, path: &str) -> bool {
    let chars = |text: &str| text.chars().collect::<Vec<char>>();
    let name = path.rsplit('/').next().unwrap_or(path);
    code_lines(attributes).iter().any(|line| {
        let mut words = line.split_whitespace();
        let Some(pattern) = words.next() else {
            return false;
        };
        let pattern = pattern.trim_start_matches('/');
        let rest: Vec<&str> = words.collect();
        let target = if pattern.contains('/') { path } else { name };
        glob_matches(&chars(pattern), &chars(target))
            && rest.contains(&"text")
            && rest.contains(&"eol=lf")
    })
}

/// The files of the repository that a `COPY` source names. A file is its own path.
/// A directory source is each file under it. An empty answer means that the source does not exist.
fn files_of_source(listing: &[String], path: &str) -> Vec<String> {
    let prefix = format!("{path}/");
    listing
        .iter()
        .filter(|file| file.as_str() == path || file.starts_with(&prefix))
        .cloned()
        .collect()
}

/// Each file that a `COPY` line of the Dockerfile names has a `text eol=lf` rule in `.gitattributes`.
/// The two allow lists have one too. A CRLF line ending of a Windows checkout then never reaches the
/// build context (ADR-0064 (g), design D2). `listing` holds the repository paths under `deploy/db`
/// and `deploy/pgtools`. A source that the listing lacks is a fault. A `COPY` from a stage names no file.
/// The pattern matching follows `has_lf_rule`.
pub fn check_line_endings(dockerfile: &str, attributes: &str, listing: &[String]) -> Report {
    let mut report = Report::default();
    let (db, pgtools) = copy_sources(&mut report, dockerfile);
    let mut wanted: BTreeSet<String> = [DB_CONTEXT_LIST, PGTOOLS_CONTEXT_LIST]
        .map(String::from)
        .into();
    for (directory, sources) in [("deploy/db", db), ("deploy/pgtools", pgtools)] {
        for source in sources {
            let path = format!("{directory}/{source}");
            let files = files_of_source(listing, &path);
            if files.is_empty() {
                report.fail(
                    &path,
                    &source,
                    "a file or a directory of the repository",
                    "none",
                );
            }
            wanted.extend(files);
        }
    }
    for path in wanted {
        if !has_lf_rule(attributes, &path) {
            report.fail(
                ATTRIBUTES_FILE,
                &path,
                "a rule with text and eol=lf",
                "no rule",
            );
        }
    }
    report
}
