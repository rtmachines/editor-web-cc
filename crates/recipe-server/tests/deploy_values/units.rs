//! The unit of a value in the table 7.11 of `reliability.md`, read by the type of its key.
//!
//! Each key has one type. Each type accepts a closed set of unit words. A word after the number that
//! the set does not hold is a fault, so a changed unit never disappears. A short list of rows holds
//! one explanation word after the number. Each of those rows names its word in `EXPLAINED`.

use crate::canon::{Canon, apply_unit, parse_milli, strip_marks, trim_mark};
use crate::fault::Report;
use crate::table::Table;

const DOCUMENT: &str = "docs/architecture/reliability.md 7.11";
const SHOWN_CHARS: usize = 80;

/// The type of the value of a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A time with the unit `ms`, `s`, `min`, `h`, `day`, or `days`.
    Duration,
    /// A size in the binary spelling of Docker Compose: `KiB`, `MiB`, `GiB`.
    SizeIec,
    /// A size in the spelling of PostgreSQL: `kB`, `MB`, `GB`.
    SizePg,
    /// A size in the short spelling of the Docker logging driver: `50m`, joined to the number.
    SizeDocker,
    /// A number of CPUs. It has no unit word.
    Cpu,
    /// A share in percent, with `%` or with no unit.
    Percent,
    /// A count, a port, or a limit. It has no unit word.
    Count,
    /// A text that starts with a digit, for example a time of day. The reader takes its first word.
    Label,
}

const DURATION_KEYS: &[&str] = &[
    "pitr_volume.start_timeout",
    "pitr_volume.stop_timeout",
    "session.idle_timeout",
    "session.absolute_timeout",
    "session.cleanup_interval",
    "login.attempt_retention",
    "login.attempt_cleanup_interval",
    "security.key_rotation_overlap",
    "i18n.lang_cookie_max_age",
    "http.request_timeout",
    "http.body_read_timeout",
    "http.accept_error_pause",
    "proxy.upstream_keepalive",
    "http.retry_after",
    "http.hsts_max_age",
    "static.cache_max_age",
    "db.statement_timeout",
    "db.lock_timeout",
    "db.idle_in_transaction_timeout",
    "db.pool_acquire_timeout",
    "db.pool_idle_timeout",
    "db.pool_max_lifetime",
    "db.pool_wait_warn",
    "db.stop_grace_period",
    "db.slow_query",
    "timeout.export",
    "timeout.validation",
    "migrate.lock_timeout",
    "migrate.retry_pause",
    "migrate.connect_timeout",
    "argon2.acquire_timeout",
    "health.readiness_cache",
    "health.db_check_timeout",
    "health.check_interval",
    "health.check_timeout",
    "health.start_period",
    "health.start_interval",
    "health.disk_check_interval",
    "disk.warn_log_interval",
    "watchdog.probe_interval",
    "watchdog.window",
    "watchdog.probe_timeout",
    "job.restart_delay",
    "audit.check_statement_timeout",
    "audit.max_check_age",
    "audit.catchup_delay",
    "audit.catchup_age",
    "heartbeat.interval",
    "alerts.interval",
    "alerts.webhook_timeout",
    "alerts.panic_window",
    "alerts.not_ready_for",
    "alerts.failed_login_window",
    "alerts.security_window",
    "backup.retention",
    "backup.max_age_alert",
    "security_event.retention",
    "security_event.purge_interval",
    "scratch.stop_timeout",
    "pitr_check.ready_poll",
    "scratch.restore_start_timeout",
    "scratch.pitr_start_timeout",
    "restore_volume.start_timeout",
    "restore_volume.stop_timeout",
    "pitr_volume.start_timeout",
    "pitr_volume.stop_timeout",
    "volume_switch.keep_min_age",
    "shutdown.drain",
    "app.stop_grace_period",
    "shutdown.pool_close",
    "shutdown.runtime_stop",
    "shutdown.internal_drain",
    "clock.skew_warn",
    "clock.check_interval",
    "pg.checkpoint_timeout",
];

const SIZE_IEC_KEYS: &[&str] = &[
    "argon2.m_cost",
    "http.body_limit.form",
    "http.body_limit.value",
    "http.body_limit.translation",
    "budget.value_bytes",
    "db.shm_size",
    "audit.check_batch_bytes",
    "wal.max_lag_bytes",
    "soak.memory_ceiling",
    "container.app_memory",
    "container.db_memory",
    "container.small_memory",
    "container.backup_memory",
    "container.oneshot_memory",
];

const SIZE_PG_KEYS: &[&str] = &[
    "scratch.shared_buffers",
    "scratch.maintenance_work_mem",
    "pg.wal_keep_size",
];

const SIZE_DOCKER_KEYS: &[&str] = &["log.max_size", "log.max_buffer_size"];

const CPU_KEYS: &[&str] = &[
    "container.app_cpus",
    "container.db_cpus",
    "container.small_cpus",
];

const PERCENT_KEYS: &[&str] = &[
    "pg.shared_buffers",
    "disk.warn_percent",
    "disk.readonly_percent",
    "disk.resume_percent",
];

const LABEL_KEYS: &[&str] = &["login.delay_formula", "audit.check_time", "backup.time"];

/// The rows whose value cell holds one explanation word right after the number.
/// The word counts only for its own key. Any other word after the number is a fault.
const EXPLAINED: &[(&str, &str)] = &[
    ("container.small_cpus", "each"),
    ("login.delay_after_failures", "failures"),
    ("login.ip_budget", "failures"),
    ("login_attempt.purge_batch", "rows"),
    ("password.max", "characters"),
    ("http.user_agent_max", "characters"),
    ("log.session_ref_length", "hex"),
    ("setting.value_max", "characters"),
    ("page.parameters", "rows"),
    ("page.users", "rows"),
    ("page.machines", "rows"),
    ("page.type_usage", "rows"),
    ("page.recipes", "rows"),
    ("page.versions", "rows"),
    ("page.diff", "parameters"),
    ("diff.leaf_rows", "rows"),
    ("diff.permits", "diffs"),
    ("export.permits", "exports"),
    ("page.audit", "rows"),
    ("page.history", "rows"),
    ("page.security_events", "rows"),
    ("form.array_chunk", "elements"),
    ("form.rebind_leaves", "new"),
    ("audit.check_batch", "rows"),
    ("runtime.worker_threads", "Tokio"),
    ("basebackup.retention", "base"),
    ("security_event.purge_batch", "rows"),
    ("restore_test.detail_length", "characters"),
    ("pitr_check.log_tail", "lines"),
];

/// The type of a key. A key that no list names is a count.
pub fn kind_of(key: &str) -> Kind {
    let lists: [(&[&str], Kind); 7] = [
        (DURATION_KEYS, Kind::Duration),
        (SIZE_IEC_KEYS, Kind::SizeIec),
        (SIZE_PG_KEYS, Kind::SizePg),
        (SIZE_DOCKER_KEYS, Kind::SizeDocker),
        (CPU_KEYS, Kind::Cpu),
        (PERCENT_KEYS, Kind::Percent),
        (LABEL_KEYS, Kind::Label),
    ];
    lists
        .iter()
        .find(|(keys, _)| keys.contains(&key))
        .map_or(Kind::Count, |(_, kind)| *kind)
}

/// The unit words that the type accepts.
fn unit_words(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Duration => &["ms", "s", "min", "h", "day", "days"],
        Kind::SizeIec => &["KiB", "MiB", "GiB"],
        Kind::SizePg => &["kB", "MB", "GB"],
        Kind::SizeDocker => &["k", "m", "g"],
        Kind::Percent => &["%"],
        Kind::Cpu | Kind::Count | Kind::Label => &[],
    }
}

fn is_group(word: &str) -> bool {
    word.len() == 3 && word.bytes().all(|byte| byte.is_ascii_digit())
}

/// The word after the number: a unit word, no unit, or the first unknown word.
fn next_unit<'a>(key: &str, kind: Kind, next: Option<&'a str>) -> Result<Option<&'a str>, String> {
    let Some(word) = next.map(trim_mark) else {
        return Ok(None);
    };
    let is_unit = kind != Kind::SizeDocker && unit_words(kind).contains(&word);
    if is_unit {
        return Ok(Some(word));
    }
    let is_note = word.starts_with('(') || word == "/" || EXPLAINED.contains(&(key, word));
    if is_note {
        Ok(None)
    } else {
        Err(word.to_owned())
    }
}

/// The value of a number and its unit. A time and a size need a unit.
fn resolve(kind: Kind, number: &str, unit: Option<&str>) -> Result<Canon, String> {
    let milli = parse_milli(number).ok_or_else(|| number.to_owned())?;
    match unit {
        Some(word) if unit_words(kind).contains(&word) => {
            apply_unit(milli, word).ok_or_else(|| word.to_owned())
        }
        Some(word) => Err(word.to_owned()),
        None if matches!(kind, Kind::Cpu | Kind::Count) => Ok(Canon::Number(milli)),
        None if kind == Kind::Percent => Ok(Canon::Percent(milli)),
        None => Err("(no unit word)".to_owned()),
    }
}

/// Read the leading value of the value cell of a key by the type of the key.
///
/// A text that does not start with a number is `Canon::Text` of its first word. The error is the
/// first word that the type does not know.
pub fn read_typed(key: &str, kind: Kind, text: &str) -> Result<Canon, String> {
    let cleaned = strip_marks(text);
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let Some(&first) = words.first() else {
        return Err("(empty)".to_owned());
    };
    let head = trim_mark(first);
    let split = head
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(head.len());
    let (digits, suffix) = head.split_at(split);
    if kind == Kind::Label || parse_milli(digits).is_none() {
        return Ok(Canon::Text(head.to_owned()));
    }
    if !suffix.is_empty() {
        return resolve(kind, digits, Some(suffix)).map_err(|_| head.to_owned());
    }
    let ended = head.len() != first.len();
    let groups: Vec<&str> = if ended || digits.contains('.') {
        Vec::new()
    } else {
        words
            .iter()
            .skip(1)
            .copied()
            .take_while(|w| is_group(w))
            .collect()
    };
    let number = format!("{digits}{}", groups.concat());
    let next = words.iter().skip(1).copied().nth(groups.len());
    let unit = if ended {
        None
    } else {
        next_unit(key, kind, next)?
    };
    resolve(kind, &number, unit)
}

/// One fault for each row of the table whose value cell holds a word that the type of its key does
/// not know. The fault names the key, the row text, and the word.
pub fn check_units(table: &Table) -> Report {
    let mut report = Report::default();
    for key in table.keys() {
        let kind = kind_of(key);
        let raw = table.raw(key).unwrap_or_default();
        if let Err(word) = read_typed(key, kind, raw) {
            let row: String = raw.chars().take(SHOWN_CHARS).collect();
            report.fail(
                DOCUMENT,
                key,
                format!("only the unit words of the type {kind:?}"),
                format!("unknown word {word:?} in the row text {row:?}"),
            );
        }
    }
    report
}
