//! Rules that join several values of `reliability.md` 7.11.
//!
//! The sum rules read the table and not a file of `deploy/`. The files of `deploy/` equal the
//! table by the other checks, so a rule that holds for the table holds for the files.

use std::collections::BTreeSet;

use crate::fault::Report;
use crate::table::Table;

const DOCUMENT: &str = "docs/architecture/reliability.md 7.11";

/// The four stop steps that add up (`reliability.md` 7.3, "Shutdown", steps 3 to 6).
const STOP_STEPS: [&str; 4] = [
    "shutdown.drain",
    "shutdown.pool_close",
    "shutdown.internal_drain",
    "shutdown.runtime_stop",
];
/// Keys that are not a value of a Compose file, so no file of `deploy/` has to show them.
/// `log.session_ref_length` is the length of a reference in a log line of the application.
const NOT_IN_COMPOSE: [&str; 1] = ["log.session_ref_length"];
/// The prefixes of the keys that `compose.yaml` shows, and the one single key that it shows too.
const COMPOSE_PREFIXES: [&str; 3] = ["pg.", "container.", "log."];
const COMPOSE_SINGLE_KEYS: [&str; 1] = ["db.max_connections"];

/// The time of a key in milliseconds, or a fault in the report.
fn millis_of(report: &mut Report, table: &Table, key: &str) -> Option<u128> {
    report.cover(key);
    let found = table.millis(key);
    if found.is_none() {
        report.fail(DOCUMENT, key, "a time", table.raw(key).unwrap_or("no row"));
    }
    found
}

/// The sum rule of the stop.
///
/// Source: `reliability.md` 7.3 line 584 and design D4 line 219:
/// `shutdown.drain + shutdown.pool_close + shutdown.internal_drain + shutdown.runtime_stop < app.stop_grace_period`.
pub fn stop_sum_holds(table: &Table) -> Report {
    let mut report = Report::default();
    let grace = millis_of(&mut report, table, "app.stop_grace_period");
    let mut sum: Option<u128> = Some(0);
    for key in STOP_STEPS {
        let step = millis_of(&mut report, table, key);
        sum = sum
            .zip(step)
            .and_then(|(total, part)| total.checked_add(part));
    }
    if let (Some(sum), Some(grace)) = (sum, grace)
        && sum >= grace
    {
        let rule = "sum of shutdown.drain, pool_close, internal_drain, runtime_stop";
        report.fail(
            DOCUMENT,
            rule,
            format!("below {grace} ms (app.stop_grace_period)"),
            format!("{sum} ms"),
        );
    }
    report
}

/// The rule of the request limit.
///
/// Source: `reliability.md` 7.3 line 585 and design D4 line 219: `http.request_timeout <= shutdown.drain`.
pub fn request_timeout_fits_drain(table: &Table) -> Report {
    let mut report = Report::default();
    let request = millis_of(&mut report, table, "http.request_timeout");
    let drain = millis_of(&mut report, table, "shutdown.drain");
    if let (Some(request), Some(drain)) = (request, drain)
        && request > drain
    {
        report.fail(
            DOCUMENT,
            "http.request_timeout",
            format!("at most {drain} ms (shutdown.drain)"),
            format!("{request} ms"),
        );
    }
    report
}

/// Both sum rules.
pub fn check_sum_rules(table: &Table) -> Report {
    let mut report = stop_sum_holds(table);
    report.absorb(request_timeout_fits_drain(table));
    report
}

/// Each `pg.*`, `container.*`, and `log.*` key of the table, and `db.max_connections`, is read from
/// `compose.yaml`. Source: design D14 row T23 and `reliability.md` 7.6, 7.8, 3.5.
pub fn check_required_keys(table: &Table, read: &BTreeSet<String>) -> Report {
    let mut report = Report::default();
    for key in table.keys() {
        let wanted = COMPOSE_PREFIXES
            .iter()
            .any(|prefix| key.starts_with(prefix))
            || COMPOSE_SINGLE_KEYS.contains(&key.as_str());
        if wanted && !NOT_IN_COMPOSE.contains(&key.as_str()) && !read.contains(key) {
            report.fail(
                "deploy/compose.yaml",
                key,
                "a value in the file",
                "no check reads the key",
            );
        }
    }
    report
}
