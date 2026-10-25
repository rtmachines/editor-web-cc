//! The order of the ten steps and the numbers that each step reads (ADR-0061).

use recipe_app::memory::DiskFailure;
use recipe_db::settings::PoolSettings;
use recipe_domain::audit::AuditAction;

use super::expect_running;
use crate::serve::host::ListenerKind;
use crate::serve::test_support::stub::{Event, StubHost};
use crate::serve::test_support::{self, INTERNAL_PORT, PUBLIC_PORT};

const SETTING_ROWS: usize = 18;

fn setting_changes(host: &StubHost) -> usize {
    host.store
        .audit_entries()
        .iter()
        .filter(|entry| entry.action() == AuditAction::SettingChanged)
        .count()
}

#[test]
fn a_healthy_start_runs_the_steps_in_the_order_of_adr_0061() {
    let host = StubHost::new();
    let server = expect_running(&host);
    let events = host.log.events();
    let expected = vec![
        Event::Webhook {
            has_url: false,
            has_ca: false,
        },
        Event::WatchdogStart(INTERNAL_PORT),
        Event::Runtime(2),
        Event::Signals,
        Event::OpenMain(PoolSettings::default().size),
        Event::OpenHealth(2),
        Event::RoleCheck,
        Event::Hasher,
        Event::DiskRead,
        Event::UnitOfWork,
        Event::Bind(ListenerKind::Internal, INTERNAL_PORT),
        Event::Arm,
        Event::StateRead,
        Event::ClockRead,
        Event::Bind(ListenerKind::Public, PUBLIC_PORT),
    ];
    assert_eq!(events, expected);
    let _report = server.close();
}

#[test]
fn the_hasher_is_built_off_the_thread_of_the_runtime() {
    let host = StubHost::new();
    let server = expect_running(&host);
    let built_on = host.hasher_thread();
    assert!(built_on.is_some(), "the start never built the hasher");
    // The runtime of the stub is a current-thread runtime, so `block_on` runs on this thread.
    assert_ne!(built_on, Some(std::thread::current().id()));
    let _report = server.close();
}

#[test]
fn the_watchdog_arms_after_the_internal_listener_opens_and_never_before() {
    let host = StubHost::new();
    let server = expect_running(&host);
    let bind = host
        .log
        .index_of(&Event::Bind(ListenerKind::Internal, INTERNAL_PORT));
    let arm = host.log.index_of(&Event::Arm);
    let start = host.log.index_of(&Event::WatchdogStart(INTERNAL_PORT));
    assert!(start < bind && bind < arm, "{:?}", host.log.events());
    assert_eq!(host.log.count(&Event::Disarm), 0);
    let _report = server.close();
}

#[test]
fn the_public_listener_opens_after_the_loops_have_started() {
    let host = StubHost::new();
    let server = expect_running(&host);
    let loops = host.log.index_of(&Event::ClockRead);
    let public = host
        .log
        .index_of(&Event::Bind(ListenerKind::Public, PUBLIC_PORT));
    assert!(loops < public, "{:?}", host.log.events());
    let _report = server.close();
}

#[test]
fn the_first_disk_check_runs_before_the_comparison_of_the_settings() {
    let host = StubHost::new();
    let server = expect_running(&host);
    let disk = host.log.index_of(&Event::DiskRead);
    let compare = host.log.index_of(&Event::UnitOfWork);
    assert!(disk < compare, "{:?}", host.log.events());
    assert_eq!(host.log.count(&Event::DiskRead), 1);
    let _report = server.close();
}

#[test]
fn the_comparison_of_the_settings_writes_one_row_per_setting() {
    let host = StubHost::new();
    let server = expect_running(&host);
    assert_eq!(setting_changes(&host), SETTING_ROWS);
    assert!(!server.read_only().is_on());
    let _report = server.close();
}

#[test]
fn a_low_disk_turns_the_read_only_mode_on_and_the_start_goes_on() {
    let host = StubHost::new();
    host.set_pgdata_free(1);
    let server = expect_running(&host);
    assert!(server.read_only().is_on());
    assert_eq!(setting_changes(&host), SETTING_ROWS);
    assert!(
        host.log
            .contains(&Event::Bind(ListenerKind::Public, PUBLIC_PORT))
    );
    let _report = server.close();
}

#[test]
fn an_unreadable_disk_turns_the_read_only_mode_on_and_the_start_goes_on() {
    let host = StubHost::new();
    host.disk.fail_on_call(1, DiskFailure::Unreadable);
    let server = expect_running(&host);
    assert!(server.read_only().is_on());
    assert_eq!(host.log.count(&Event::DiskRead), 1);
    assert_eq!(setting_changes(&host), SETTING_ROWS);
    assert!(
        host.log
            .contains(&Event::Bind(ListenerKind::Public, PUBLIC_PORT))
    );
    let _report = server.close();
}

#[test]
fn the_runtime_has_the_workers_of_the_plan() {
    let host = StubHost::new();
    let mut plan = test_support::plan();
    plan.settings.runtime_workers = 3;
    let server = super::running_with(&host, plan);
    assert!(host.log.contains(&Event::Runtime(3)));
    let _report = server.close();
}

#[test]
fn the_pools_open_with_the_sizes_of_the_plan() {
    let host = StubHost::new();
    let mut plan = test_support::plan();
    plan.settings.main_pool.size = 7;
    plan.settings.health_pool_size = 3;
    let server = super::running_with(&host, plan);
    assert!(host.log.contains(&Event::OpenMain(7)));
    assert!(host.log.contains(&Event::OpenHealth(3)));
    let _report = server.close();
}

#[test]
fn the_webhook_gets_the_url_and_the_ca_path_of_the_plan() {
    let host = StubHost::new();
    let mut plan = test_support::plan();
    plan.webhook_url =
        crate::config::webhook::parse_webhook_text("https://hooks.example/alert").unwrap();
    plan.configured.webhook_ca_file = Some("/run/secrets/ca".to_owned());
    let server = super::running_with(&host, plan);
    assert_eq!(
        host.log.events().first(),
        Some(&Event::Webhook {
            has_url: true,
            has_ca: true,
        })
    );
    let _report = server.close();
}
