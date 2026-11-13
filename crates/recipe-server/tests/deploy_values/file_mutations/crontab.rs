//! Crontab checks: the zone and the schedule lines of the backup crontab files.

use crate::inputs::{Mutation, assert_mutations_fail};

const CRONTAB: [Mutation; 6] = [
    // A later assignment would change the zone of the lines below it.
    (
        "crontab.backup",
        "CRON_TZ=UTC",
        "CRON_TZ=UTC\nCRON_TZ=Europe/Warsaw",
        "CRON_TZ",
    ),
    (
        "crontab.backup",
        "0 2 * * * backup-once",
        "0 3 * * * backup-once",
        "backup.time",
    ),
    (
        "crontab.backup",
        "0 5 * * 0 restore-test",
        "0 6 * * 0 restore-test",
        "restore_test.time",
    ),
    (
        "crontab.backup",
        "CRON_TZ=UTC",
        "CRON_TZ=Europe/Warsaw",
        "CRON_TZ",
    ),
    (
        "crontab.walarchive",
        "30 3 * * 0 basebackup-once",
        "30 4 * * 0 basebackup-once",
        "basebackup.time",
    ),
    (
        "crontab.walarchive",
        "30 3 * * 0 basebackup-once",
        "30 3 * * 1 basebackup-once",
        "basebackup.time",
    ),
];

#[test]
fn a_changed_schedule_or_time_zone_is_a_fault() {
    assert_mutations_fail(&CRONTAB);
}
