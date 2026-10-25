//! Tests of the settings type of the job `alerts` (`reliability.md` section 7.11).

use std::time::Duration;

use crate::jobs::alerts::AlertSettings;

#[test]
fn the_defaults_equal_the_rows_of_the_settings_table() {
    let settings = AlertSettings::default();
    assert_eq!(settings.interval, Duration::from_secs(60));
    assert_eq!(settings.panic_count, 3);
    assert_eq!(settings.panic_window, Duration::from_secs(10 * 60));
    assert_eq!(settings.not_ready_for, Duration::from_secs(60));
    assert_eq!(settings.failed_login_count, 50);
    assert_eq!(settings.failed_login_window, Duration::from_secs(15 * 60));
    assert_eq!(settings.security_window, Duration::from_secs(15 * 60));
    assert_eq!(
        settings.backup_max_age_alert,
        Duration::from_secs(26 * 60 * 60)
    );
    assert_eq!(
        settings.audit_max_check_age,
        Duration::from_secs(26 * 60 * 60)
    );
    assert_eq!(settings.wal_max_lag_bytes, 16 * 1024 * 1024);
    assert_eq!(settings.disk_warn_percent, 15);
}
