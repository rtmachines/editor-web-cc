//! The refusals of the start (ADR-0061 (b)): one message, no later step, and a full unwind.
//!
//! The start returns the error. The function `main` writes the one `ERROR` line and exits with
//! code 1. Each test checks the message, the steps that ran, and what the unwind closed.

use recipe_app::error::{AppError, StorageError};
use recipe_app::memory::{Fault, OperationsMethod};
use recipe_app::ports::RoleCheck;
use recipe_db::settings::PoolSettings;

use super::expect_refusal;
use crate::serve::host::ListenerKind;
use crate::serve::start::start_run_error;
use crate::serve::test_support::stub::{Event, Fail, Pool, StubHost};
use crate::serve::test_support::{INTERNAL_PORT, PUBLIC_PORT};
use crate::supervisor::{JobName, StartRunFault};

const WEBHOOK: Event = Event::Webhook {
    has_url: false,
    has_ca: false,
};

fn failing(step: Fail) -> StubHost {
    let mut host = StubHost::new();
    host.fail = Some(step);
    host
}

/// The events that every start produces before the main pool opens.
fn before_pools() -> Vec<Event> {
    vec![
        WEBHOOK,
        Event::WatchdogStart(INTERNAL_PORT),
        Event::Runtime(2),
        Event::Signals,
    ]
}

/// The close of a start with both pools open.
fn unwind_with_both_pools() -> Vec<Event> {
    vec![
        Event::Disarm,
        Event::PoolClosed(Pool::Main),
        Event::PoolClosed(Pool::Health),
        Event::WatchdogStop,
    ]
}

fn pools_open() -> Vec<Event> {
    vec![
        Event::OpenMain(PoolSettings::default().size),
        Event::OpenHealth(2),
    ]
}

#[test]
fn a_bad_ca_file_refuses_at_step_2_and_names_the_key_only() {
    let host = failing(Fail::Webhook);
    let message = expect_refusal(&host);
    assert!(message.contains("alerts.webhook_ca_file"), "{message}");
    assert_eq!(host.log.events(), vec![WEBHOOK]);
}

#[test]
fn a_watchdog_that_cannot_start_refuses_before_the_runtime() {
    let host = failing(Fail::Watchdog);
    let message = expect_refusal(&host);
    assert!(
        message.contains("the watchdog thread cannot start"),
        "{message}"
    );
    assert_eq!(
        host.log.events(),
        vec![WEBHOOK, Event::WatchdogStart(INTERNAL_PORT)]
    );
}

#[test]
fn a_runtime_that_cannot_start_stops_the_watchdog_and_does_not_disarm_it() {
    let host = failing(Fail::Runtime);
    let message = expect_refusal(&host);
    assert!(message.contains("the runtime cannot start"), "{message}");
    assert_eq!(
        host.log.events(),
        vec![
            WEBHOOK,
            Event::WatchdogStart(INTERNAL_PORT),
            Event::Runtime(2),
            Event::WatchdogStop
        ]
    );
}

#[test]
fn a_signal_handler_that_cannot_register_refuses_before_the_pools() {
    let host = failing(Fail::Signals);
    let message = expect_refusal(&host);
    assert!(
        message.contains("the signal handlers cannot register"),
        "{message}"
    );
    let mut expected = before_pools();
    expected.push(Event::WatchdogStop);
    assert_eq!(host.log.events(), expected);
}

#[test]
fn a_main_pool_that_cannot_open_unwinds_the_watchdog() {
    let host = failing(Fail::MainPool);
    let message = expect_refusal(&host);
    assert!(message.contains("the main pool cannot open"), "{message}");
    let mut expected = before_pools();
    expected.extend([
        Event::OpenMain(PoolSettings::default().size),
        Event::Disarm,
        Event::WatchdogStop,
    ]);
    assert_eq!(host.log.events(), expected);
}

#[test]
fn a_health_pool_that_cannot_open_closes_the_main_pool() {
    let host = failing(Fail::HealthPool);
    let message = expect_refusal(&host);
    assert!(message.contains("the health pool cannot open"), "{message}");
    let mut expected = before_pools();
    expected.extend(pools_open());
    expected.extend([
        Event::Disarm,
        Event::PoolClosed(Pool::Main),
        Event::WatchdogStop,
    ]);
    assert_eq!(host.log.events(), expected);
}

#[test]
fn a_role_that_is_not_allowed_refuses_before_the_hasher_and_the_disk_check() {
    let host = StubHost::new();
    host.operations.set_role_check(RoleCheck {
        current_user: "postgres".to_owned(),
        is_superuser: true,
        owns_relations: false,
    });
    let message = expect_refusal(&host);
    assert!(
        message.contains("the role of the connection is not allowed"),
        "{message}"
    );
    assert!(!host.log.contains(&Event::Hasher));
    assert!(!host.log.contains(&Event::DiskRead));
    let events = host.log.events();
    assert_eq!(
        events.get(events.len() - 4..).unwrap(),
        unwind_with_both_pools()
    );
}

#[test]
fn a_role_check_that_fails_to_read_refuses_with_its_error_code() {
    let host = StubHost::new();
    host.operations
        .fail_on_call(OperationsMethod::RoleCheck, 1, Fault::Unavailable);
    let message = expect_refusal(&host);
    assert!(
        message.contains("the role of the connection cannot be read"),
        "{message}"
    );
    assert!(message.contains("E-DB-005"), "{message}");
    assert!(!host.log.contains(&Event::DiskRead));
}

#[test]
fn a_hasher_that_cannot_start_refuses_before_the_disk_check() {
    let host = failing(Fail::Hasher);
    let message = expect_refusal(&host);
    assert!(
        message.contains("the password hasher cannot start"),
        "{message}"
    );
    assert!(!host.log.contains(&Event::DiskRead));
    let events = host.log.events();
    assert_eq!(
        events.get(events.len() - 4..).unwrap(),
        unwind_with_both_pools()
    );
}

#[test]
fn a_failed_comparison_of_the_settings_refuses_before_any_listener() {
    let host = StubHost::new();
    host.store.fail_append_audit(Fault::Unavailable);
    let message = expect_refusal(&host);
    assert!(
        message.contains("the comparison of the settings failed"),
        "{message}"
    );
    assert!(message.contains("E-AUD-001"), "{message}");
    assert!(host.log.contains(&Event::DiskRead));
    assert!(
        !host
            .log
            .contains(&Event::Bind(ListenerKind::Internal, INTERNAL_PORT))
    );
    assert!(!host.log.contains(&Event::Arm));
}

#[test]
fn an_internal_port_in_use_refuses_and_the_watchdog_never_arms() {
    let host = failing(Fail::BindInternal);
    let message = expect_refusal(&host);
    assert!(
        message.contains("the internal listener cannot bind its port"),
        "{message}"
    );
    assert!(!host.log.contains(&Event::Arm));
    let events = host.log.events();
    assert_eq!(
        events.get(events.len() - 4..).unwrap(),
        unwind_with_both_pools()
    );
}

#[test]
fn a_public_port_in_use_refuses_and_unwinds_after_the_arm() {
    let host = failing(Fail::BindPublic);
    let message = expect_refusal(&host);
    assert!(
        message.contains("the public listener cannot bind its port"),
        "{message}"
    );
    let arm = host.log.index_of(&Event::Arm);
    let bind = host
        .log
        .index_of(&Event::Bind(ListenerKind::Public, PUBLIC_PORT));
    assert!(arm < bind);
    let events = host.log.events();
    assert_eq!(
        events.get(events.len() - 4..).unwrap(),
        unwind_with_both_pools()
    );
}

#[test]
fn a_failed_read_of_the_last_audit_check_refuses_before_the_public_listener() {
    let host = StubHost::new();
    host.operations
        .fail_on_call(OperationsMethod::State, 1, Fault::Unavailable);
    let message = expect_refusal(&host);
    assert!(
        message.contains("the last audit check cannot be read"),
        "{message}"
    );
    assert!(message.contains("E-DB-005"), "{message}");
    assert!(
        !host
            .log
            .contains(&Event::Bind(ListenerKind::Public, PUBLIC_PORT))
    );
}

#[test]
fn a_panic_of_the_first_disk_check_refuses_the_start_and_unwinds_everything() {
    let mut host = StubHost::new();
    host.disk_panics = true;
    let message = expect_refusal(&host);
    assert!(
        message.contains("the first run of the job disk_check panicked"),
        "{message}"
    );
    let mut expected = before_pools();
    expected.extend(pools_open());
    expected.extend([Event::RoleCheck, Event::Hasher, Event::DiskRead]);
    expected.extend(unwind_with_both_pools());
    // The list has no `UnitOfWork` event and no `Bind` event, so no later step ran.
    assert_eq!(host.log.events(), expected);
}

#[test]
fn a_refused_first_run_is_an_error_with_the_code_and_the_job() {
    let fault = StartRunFault::Failed(AppError::Storage(StorageError::Unavailable));
    let message = format!("{:#}", start_run_error(JobName::DiskCheck, fault));
    assert!(
        message.contains("the first run of the job disk_check failed"),
        "{message}"
    );
    assert!(message.contains("E-DB-005"), "{message}");
}

#[test]
fn a_panic_of_the_first_run_is_an_error_that_names_the_job() {
    let message = format!(
        "{:#}",
        start_run_error(JobName::DiskCheck, StartRunFault::Panicked)
    );
    assert!(
        message.contains("the first run of the job disk_check panicked"),
        "{message}"
    );
}

#[test]
fn a_cancelled_first_run_is_an_error_that_names_the_job() {
    let message = format!(
        "{:#}",
        start_run_error(JobName::DiskCheck, StartRunFault::Cancelled)
    );
    assert!(
        message.contains("the runtime cancelled the first run of the job disk_check"),
        "{message}"
    );
}
