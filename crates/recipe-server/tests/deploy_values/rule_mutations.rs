//! Proof that the sum rules and the presence rule can fail, and unit tests of the readers.

use std::collections::BTreeMap;

use crate::canon::{Bare, Canon, canon, canon_whole};
use crate::inputs::{Inputs, RELIABILITY};
use crate::lines::{unquote, without_comment};
use crate::rules::{check_required_keys, request_timeout_fits_drain, stop_sum_holds};
use crate::table::{HEADING_SETTINGS, Table};
use crate::topology::check_address_plan;

const DRAIN_ROW: &str = "| `shutdown.drain` / `app.stop_grace_period` | 30 s / 40 s |";
const POOL_ROW: &str = "| `shutdown.pool_close` / `shutdown.runtime_stop` | 5 s / 3 s.";
const INTERNAL_ROW: &str = "| `shutdown.internal_drain` | 0 s.";
const REQUEST_ROW: &str = "| `http.request_timeout` | 30 s |";

/// The table 7.11 of the document after one change.
fn settings_after(from: &str, to: &str) -> Table {
    let inputs = Inputs::load().with_replaced("reliability.md", from, to);
    Table::parse(inputs.text(RELIABILITY), HEADING_SETTINGS)
}

fn settings() -> Table {
    Table::parse(Inputs::load().text(RELIABILITY), HEADING_SETTINGS)
}

#[test]
fn the_stop_sum_holds_for_the_table_and_fails_at_the_limit() {
    // reliability.md 7.3 line 584: the sum 38 s is below 40 s.
    assert!(stop_sum_holds(&settings()).faults.is_empty());
    // The sum 38 s equals 38 s, so the strict rule fails.
    let at_limit = stop_sum_holds(&settings_after(
        DRAIN_ROW,
        "| `shutdown.drain` / `app.stop_grace_period` | 30 s / 38 s |",
    ));
    assert!(
        at_limit.has_fault_for("sum of shutdown"),
        "{}",
        at_limit.message()
    );
    let below = stop_sum_holds(&settings_after(
        DRAIN_ROW,
        "| `shutdown.drain` / `app.stop_grace_period` | 30 s / 39 s |",
    ));
    assert!(below.faults.is_empty(), "{}", below.message());
}

#[test]
fn each_step_of_the_stop_sum_counts() {
    let pool = stop_sum_holds(&settings_after(
        POOL_ROW,
        "| `shutdown.pool_close` / `shutdown.runtime_stop` | 8 s / 3 s.",
    ));
    assert!(pool.has_fault_for("sum of shutdown"), "{}", pool.message());
    let runtime = stop_sum_holds(&settings_after(
        POOL_ROW,
        "| `shutdown.pool_close` / `shutdown.runtime_stop` | 5 s / 6 s.",
    ));
    assert!(
        runtime.has_fault_for("sum of shutdown"),
        "{}",
        runtime.message()
    );
    let internal = stop_sum_holds(&settings_after(
        INTERNAL_ROW,
        "| `shutdown.internal_drain` | 5 s.",
    ));
    assert!(
        internal.has_fault_for("sum of shutdown"),
        "{}",
        internal.message()
    );
    let drain = stop_sum_holds(&settings_after(
        DRAIN_ROW,
        "| `shutdown.drain` / `app.stop_grace_period` | 36 s / 40 s |",
    ));
    assert!(
        drain.has_fault_for("sum of shutdown"),
        "{}",
        drain.message()
    );
}

#[test]
fn a_missing_step_of_the_stop_sum_is_a_fault() {
    let missing = settings_after(
        "`shutdown.runtime_stop` | 5 s / 3 s.",
        "`shutdown.runtime_halt` | 5 s / 3 s.",
    );
    let report = stop_sum_holds(&missing);
    assert!(
        report.has_fault_for("shutdown.runtime_stop"),
        "{}",
        report.message()
    );
}

#[test]
fn the_request_timeout_fits_the_drain_and_fails_above_it() {
    // reliability.md 7.3 line 585: 30 s is at most 30 s.
    assert!(request_timeout_fits_drain(&settings()).faults.is_empty());
    let above = request_timeout_fits_drain(&settings_after(
        REQUEST_ROW,
        "| `http.request_timeout` | 31 s |",
    ));
    assert!(
        above.has_fault_for("http.request_timeout"),
        "{}",
        above.message()
    );
    let below = request_timeout_fits_drain(&settings_after(
        REQUEST_ROW,
        "| `http.request_timeout` | 29 s |",
    ));
    assert!(below.faults.is_empty(), "{}", below.message());
}

#[test]
fn a_key_that_no_check_reads_is_a_fault() {
    let table = settings();
    let report = check_required_keys(&table, &std::collections::BTreeSet::new());
    for key in [
        "pg.shared_buffers",
        "container.app_memory",
        "log.max_size",
        "db.max_connections",
    ] {
        assert!(report.has_fault_for(key), "no fault for {key}");
    }
    // The length of a session reference is no value of the Compose file.
    assert!(!report.has_fault_for("log.session_ref_length"));
    assert!(!report.has_fault_for("db.pool_size"));
}

#[test]
fn the_address_plan_needs_the_proxy_in_the_subnet_and_outside_the_range() {
    let plan = |subnet, range, proxy| {
        let env = BTreeMap::from([
            ("COMPOSE_SUBNET", subnet),
            ("COMPOSE_IP_RANGE", range),
            ("PROXY_ADDRESS", proxy),
        ]);
        check_address_plan(&env)
    };
    assert!(
        plan("172.28.0.0/16", "172.28.1.0/24", "172.28.0.2")
            .faults
            .is_empty()
    );
    assert!(plan("172.28.0.0/16", "172.28.0.0/24", "172.28.0.2").has_fault_for("PROXY_ADDRESS"));
    assert!(plan("172.28.0.0/16", "172.29.1.0/24", "172.28.0.2").has_fault_for("COMPOSE_IP_RANGE"));
    assert!(plan("172.28.0.0/16", "172.28.1.0/24", "172.29.0.2").has_fault_for("PROXY_ADDRESS"));
    assert!(
        plan("172.28.0.0/16", "172.28.1.0/24", "not-an-address").has_fault_for("COMPOSE_SUBNET")
    );
}

#[test]
fn the_canonical_form_equates_the_spellings_of_one_value() {
    let same = |left: &str, right: &str, bare: Bare| canon(left, bare) == canon(right, Bare::Plain);
    assert!(same("30s", "30 s", Bare::Plain));
    assert!(same("30000", "30 s", Bare::Millis));
    assert!(same("1000", "1 s", Bare::Millis));
    assert!(same("1800", "30 min", Bare::Seconds));
    assert!(same("30", "30 days", Bare::Days));
    assert!(same("2g", "2 GiB", Bare::Plain));
    assert!(same("512MB", "512 MiB", Bare::Plain));
    assert!(same("1GB", "1 GB", Bare::Plain));
    assert_eq!(canon("25 %", Bare::Plain), Some(Canon::Percent(25_000)));
    assert_eq!(canon("0.5", Bare::Plain), Some(Canon::Number(500)));
    assert_eq!(
        canon("`off`", Bare::Plain),
        Some(Canon::Text("off".to_owned()))
    );
}

#[test]
fn the_whole_value_of_a_deployed_file_counts() {
    let plain = |text: &str| canon_whole(unquote(text), Bare::Plain);
    assert_eq!(plain("5433 # scratch.port"), canon("5433", Bare::Plain));
    assert_eq!(plain("\"recipe\""), canon("`recipe`.", Bare::Plain));
    assert_ne!(plain("\"rec'ipe\""), canon("recipe", Bare::Plain));
    assert_ne!(plain("\"recipe #archive\""), canon("recipe", Bare::Plain));
    assert_ne!(plain("\"recipe\" extra"), canon("recipe", Bare::Plain));
    assert_ne!(plain("recipe archive"), canon("recipe", Bare::Plain));
    assert_ne!(plain("recipe."), canon("recipe", Bare::Plain));
    assert_ne!(plain("8080."), canon("8080", Bare::Plain));
    assert_ne!(plain("5 s extra"), canon("5 s", Bare::Plain));
    assert_ne!(plain("5s min"), canon("5s", Bare::Plain));
    assert_eq!(plain("5 s"), canon("5s", Bare::Plain));
}

#[test]
fn a_shell_comment_starts_after_a_space_outside_quotes() {
    assert_eq!(without_comment("A=1 # B=2"), "A=1 ");
    assert_eq!(without_comment("A='x # y' B=2"), "A='x # y' B=2");
    assert_eq!(without_comment("A=\"x # y\" # z"), "A=\"x # y\" ");
    assert_eq!(without_comment("echo ${#A} $#"), "echo ${#A} $#");
    assert_eq!(without_comment("# whole line"), "");
}

#[test]
fn the_canonical_form_keeps_different_values_apart() {
    assert_ne!(canon("30s", Bare::Plain), canon("31s", Bare::Plain));
    assert_ne!(canon("1GB", Bare::Plain), canon("2GB", Bare::Plain));
    assert_ne!(canon("5", Bare::Plain), canon("5", Bare::Seconds));
    assert_ne!(canon("5min", Bare::Plain), canon("5 s", Bare::Plain));
    assert_ne!(canon("5x", Bare::Plain), canon("5", Bare::Plain));
}
