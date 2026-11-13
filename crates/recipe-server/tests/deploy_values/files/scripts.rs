use crate::canon::Bare;
use crate::fault::Report;
use crate::lines::{code_lines, unquote, without_comment};
use crate::table::Table;

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
