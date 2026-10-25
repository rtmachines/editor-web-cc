//! The one `AlertCounters` of the process reaches the job `alerts` (task 7.8).
//!
//! The panic hook and the use cases write to the counters. The job `alerts` reads them. If the
//! start made a new counter for the job, the alert `panics` would never fire.

use recipe_app::operations::AlertCounters;
use recipe_app::ports::AlertCondition;

use super::advance;
use crate::serve::test_support::{self, stub::StubHost};
use crate::serve::{Host as _, wiring};

const PANICS_ABOVE_THE_LIMIT: u64 = 4;
const ALERT_INTERVAL_SECONDS: u64 = 61;

#[test]
fn the_panics_of_the_shared_counters_reach_the_alerts_job() {
    let host = StubHost::new();
    let counters = AlertCounters::default();
    for _ in 0..PANICS_ABOVE_THE_LIMIT {
        counters.record_panic();
    }
    let server = super::running_with_counters(&host, test_support::plan(), counters.clone());
    assert_eq!(host.webhook.sent(AlertCondition::Panics), 0);
    advance(&server, ALERT_INTERVAL_SECONDS);
    assert!(host.webhook.sent(AlertCondition::Panics) >= 1);
    let _report = server.close();
}

#[test]
fn the_shared_counters_reach_the_context_of_the_use_cases() {
    let host = StubHost::new();
    let plan = test_support::plan();
    let runtime = host.build_runtime(1).unwrap();
    let main = runtime
        .block_on(host.open_main_pool(&plan.settings.main_pool))
        .unwrap();
    let counters = AlertCounters::default();

    let context = wiring::app_context(&host, &plan, &main, &counters);
    context.counters().record_export_hash_mismatch();

    assert_eq!(counters.export_hash_mismatches(), 1);
    assert_eq!(counters.panics(), 0);
}

#[test]
fn a_fresh_counter_raises_no_panics_alert() {
    let host = StubHost::new();
    let server = super::expect_running(&host);
    advance(&server, ALERT_INTERVAL_SECONDS);
    assert_eq!(host.webhook.sent(AlertCondition::Panics), 0);
    let _report = server.close();
}
