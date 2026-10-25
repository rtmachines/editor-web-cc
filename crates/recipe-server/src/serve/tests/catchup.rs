//! The one-shot catch-up of `audit_chain_check` (ADR-0060, step 9 of ADR-0061).
//!
//! The start reads `audit.last_check_at` once. The catch-up run reads it again, but a skipped run
//! reads it too. So the tests count the reads of the chain head, which only a real check makes.

use recipe_app::memory::AuditChainMethod;
use recipe_app::ports::TimeStateKey;

use super::{advance, quiet_plan, running_with};
use crate::serve::test_support::stub::{Event, StubHost};

const ONE_HOUR_BEFORE: &str = "2026-10-24T11:00:00.000000Z";
const TWENTY_FIVE_HOURS_BEFORE: &str = "2026-10-23T11:00:00.000000Z";
const CATCHUP_DELAY_SECONDS: u64 = 10 * 60;

fn host_with_last_check(text: Option<&str>) -> StubHost {
    let host = StubHost::new();
    if let Some(text) = text {
        host.operations
            .seed_state_text(TimeStateKey::AuditLastCheckAt, text)
            .unwrap();
    }
    host
}

/// Return the number of reads of the chain head. Only a real check reads it.
fn chain_heads(host: &StubHost) -> u64 {
    host.audit_chain.calls(AuditChainMethod::Head)
}

#[test]
fn a_missing_last_check_gets_a_catchup_after_the_delay() {
    let host = host_with_last_check(None);
    let server = running_with(&host, quiet_plan());
    assert_eq!(host.log.count(&Event::StateRead), 1);
    assert_eq!(chain_heads(&host), 0, "at the start");
    advance(&server, CATCHUP_DELAY_SECONDS - 60);
    assert_eq!(chain_heads(&host), 0, "before the delay");
    advance(&server, 120);
    assert!(chain_heads(&host) >= 1, "after the delay");
    let _report = server.close();
}

#[test]
fn a_last_check_of_25_hours_gets_a_catchup() {
    let host = host_with_last_check(Some(TWENTY_FIVE_HOURS_BEFORE));
    let server = running_with(&host, quiet_plan());
    assert_eq!(chain_heads(&host), 0, "before the delay");
    advance(&server, CATCHUP_DELAY_SECONDS + 60);
    assert!(chain_heads(&host) >= 1, "after the delay");
    let _report = server.close();
}

#[test]
fn a_last_check_of_one_hour_gets_no_catchup() {
    let host = host_with_last_check(Some(ONE_HOUR_BEFORE));
    let server = running_with(&host, quiet_plan());
    advance(&server, CATCHUP_DELAY_SECONDS + 60);
    assert_eq!(host.log.count(&Event::StateRead), 1);
    assert_eq!(chain_heads(&host), 0, "after the delay");
    let _report = server.close();
}

#[test]
fn a_check_that_completes_before_the_catchup_suppresses_it() {
    let host = host_with_last_check(Some(TWENTY_FIVE_HOURS_BEFORE));
    let server = running_with(&host, quiet_plan());
    host.operations
        .seed_state_text(TimeStateKey::AuditLastCheckAt, ONE_HOUR_BEFORE)
        .unwrap();
    advance(&server, CATCHUP_DELAY_SECONDS + 60);
    assert_eq!(
        host.log.count(&Event::StateRead),
        2,
        "the skip reads the key"
    );
    assert_eq!(chain_heads(&host), 0, "after the delay");
    let _report = server.close();
}

#[test]
fn the_catchup_delay_comes_from_the_plan() {
    let host = host_with_last_check(None);
    let mut plan = quiet_plan();
    plan.settings.audit_catchup_delay = std::time::Duration::from_secs(5);
    let server = running_with(&host, plan);
    assert_eq!(chain_heads(&host), 0, "before the delay");
    advance(&server, 6);
    assert!(chain_heads(&host) >= 1, "after the delay");
    let _report = server.close();
}

#[test]
fn the_catchup_age_comes_from_the_plan() {
    let host = host_with_last_check(Some(ONE_HOUR_BEFORE));
    let mut plan = quiet_plan();
    plan.settings.audit_catchup_age = std::time::Duration::from_secs(30 * 60);
    let server = running_with(&host, plan);
    assert_eq!(chain_heads(&host), 0, "before the delay");
    advance(&server, CATCHUP_DELAY_SECONDS + 60);
    assert!(chain_heads(&host) >= 1, "after the delay");
    let _report = server.close();
}

#[test]
fn the_nightly_run_starts_at_the_time_of_the_plan() {
    let host = host_with_last_check(Some(ONE_HOUR_BEFORE));
    let mut plan = quiet_plan();
    plan.settings.audit_check_hour = 12;
    plan.settings.audit_check_minute = 1;
    let server = running_with(&host, plan);
    assert_eq!(host.audit_chain.calls(AuditChainMethod::Head), 0);
    advance(&server, 70);
    assert!(host.audit_chain.calls(AuditChainMethod::Head) >= 1);
    let _report = server.close();
}
