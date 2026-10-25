//! Proof that the crontab check compares the whole job list and the zone, and that the logging check reads the driver.
//!
//! An extra job, a changed zone word in a document row, and a changed driver must each give a fault.
//! No file of `deploy/` changes.

use crate::inputs::{Mutation, assert_mutations_fail};

const SCHEDULE: [Mutation; 6] = [
    // A job that no key of 7.11 names must not run beside the expected jobs.
    (
        "crontab.backup",
        "0 5 * * 0 restore-test",
        "0 5 * * 0 restore-test\n0 3 * * * backup-once",
        "unexpected cron job",
    ),
    // A second copy of an expected job runs it twice.
    (
        "crontab.walarchive",
        "30 3 * * 0 basebackup-once",
        "30 3 * * 0 basebackup-once\n30 3 * * 0 basebackup-once",
        "unexpected cron job",
    ),
    // The zone word of the document row decides the zone of the crontab file.
    (
        "reliability.md",
        "02:00 UTC / 30 days",
        "02:00 Europe/Warsaw / 30 days",
        "CRON_TZ",
    ),
    (
        "reliability.md",
        "Sunday 05:00 UTC",
        "Sunday 05:00 Europe/Warsaw",
        "CRON_TZ",
    ),
    (
        "reliability.md",
        "Sunday 03:30 UTC / 4",
        "Sunday 03:30 Europe/Warsaw / 4",
        "CRON_TZ",
    ),
    // A time without a zone word gives no zone to compare.
    (
        "reliability.md",
        "Sunday 05:00 UTC",
        "Sunday 05:00",
        "restore_test.time",
    ),
];

#[test]
fn an_extra_cron_job_or_a_changed_zone_word_is_a_fault() {
    assert_mutations_fail(&SCHEDULE);
}

const DRIVER: [Mutation; 2] = [
    (
        "compose.yaml",
        "driver: json-file",
        "driver: local",
        "x-logging driver",
    ),
    (
        "compose.yaml",
        "driver: json-file",
        "driver: syslog",
        "x-logging driver",
    ),
];

#[test]
fn a_changed_logging_driver_is_a_fault() {
    assert_mutations_fail(&DRIVER);
}
