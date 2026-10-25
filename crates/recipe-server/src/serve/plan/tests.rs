//! Tests of the numbers and the overlay of the plan.

use std::path::Path;
use std::time::Duration;

use recipe_domain::recipe::LangCode;
use recipe_web::CidrList;

use super::{Plan, ServeSettings, ShutdownSettings};
use crate::serve::host::ListenerKind;
use crate::serve::test_support::{configured, key_ring};

const fn seconds(count: u64) -> Duration {
    Duration::from_secs(count)
}

#[test]
fn the_default_numbers_equal_the_table_of_reliability_7_11() {
    let settings = ServeSettings::default();
    assert_eq!(settings.runtime_workers, 2);
    assert_eq!(settings.job_restart_delay, seconds(10));
    assert_eq!(
        (settings.audit_check_hour, settings.audit_check_minute),
        (3, 0)
    );
    assert_eq!(settings.audit_catchup_delay, seconds(10 * 60));
    assert_eq!(settings.audit_catchup_age, seconds(24 * 60 * 60));
    assert_eq!(settings.heartbeat_interval, seconds(5 * 60));
    assert_eq!(settings.clock_check_interval, seconds(60 * 60));
    assert_eq!(settings.disk_check_interval, seconds(30));
    assert_eq!(settings.accept_error_pause, seconds(1));
    assert_eq!(settings.health_pool_size, 2);
}

#[test]
fn the_default_stop_times_equal_the_table_of_reliability_7_11() {
    let shutdown = ShutdownSettings::default();
    assert_eq!(shutdown.drain, seconds(30));
    assert_eq!(shutdown.pool_close, seconds(5));
    assert_eq!(shutdown.internal_drain, Duration::ZERO);
    assert_eq!(shutdown.runtime_stop, seconds(3));
}

#[test]
fn the_volumes_are_the_mount_points_of_the_compose_file() {
    let settings = ServeSettings::default();
    assert_eq!(settings.pgdata_path, Path::new("/var/lib/postgresql/data"));
    assert_eq!(settings.backups_path, Path::new("/backups"));
    assert_eq!(settings.walarchive_path, Path::new("/wal"));
}

#[test]
fn the_internal_listener_drains_for_zero_seconds_and_the_public_one_for_30() {
    let settings = ServeSettings::default();
    let internal = settings.listener(ListenerKind::Internal);
    let public = settings.listener(ListenerKind::Public);
    assert_eq!(internal.drain_deadline, Duration::ZERO);
    assert_eq!(public.drain_deadline, seconds(30));
}

#[test]
fn both_listeners_share_the_header_timeout_and_the_accept_pause() {
    let mut settings = ServeSettings::default();
    settings.web.body_read_timeout = seconds(11);
    settings.accept_error_pause = seconds(2);
    for kind in [ListenerKind::Internal, ListenerKind::Public] {
        let listener = settings.listener(kind);
        assert_eq!(listener.header_read_timeout, seconds(11));
        assert_eq!(listener.accept_error_pause, seconds(2));
    }
}

#[test]
fn the_health_pool_has_its_own_size_and_the_other_numbers_of_the_main_pool() {
    let mut settings = ServeSettings::default();
    settings.main_pool.size = 9;
    settings.health_pool_size = 3;
    let pool = settings.health_pool();
    assert_eq!(pool.size, 3);
    assert_eq!(settings.main_pool.size, 9);
}

#[test]
fn the_plan_takes_five_numbers_from_the_checked_configuration() {
    let mut values = configured();
    values.idle_timeout = seconds(101);
    values.absolute_timeout = seconds(202);
    values.separation_of_duties = false;
    let langs = vec![
        LangCode::parse("pl").unwrap(),
        LangCode::parse("de").unwrap(),
    ];
    let plan = Plan::new(
        values,
        key_ring(),
        key_ring(),
        None,
        CidrList::default(),
        langs.clone(),
    );
    let settings = plan.settings;
    assert_eq!(settings.identity.session_idle_timeout, seconds(101));
    assert_eq!(settings.identity.session_absolute_timeout, seconds(202));
    assert!(!settings.app.approval_policy.separation_of_duties);
    assert_eq!(settings.web.translation_langs, langs);
}

#[test]
fn the_plan_keeps_the_other_numbers_at_their_default() {
    let plan = Plan::new(
        configured(),
        key_ring(),
        key_ring(),
        None,
        CidrList::default(),
        Vec::new(),
    );
    assert_eq!(plan.settings.runtime_workers, 2);
    assert_eq!(plan.settings.heartbeat_interval, seconds(300));
    assert!(plan.webhook_url.is_none());
}
