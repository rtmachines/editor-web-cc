//! Proof that each group of checks on the Caddyfiles, the crontab files, and the pgtools scripts can fail.
//!
//! Each case changes one copy of a text and asserts a fault. No file of `deploy/` changes.

use crate::inputs::{Inputs, Mutation, assert_mutations_fail, check_all};

const CADDYFILE: [Mutation; 10] = [
    // The header name and the directive must be whole words.
    (
        "Caddyfile",
        "?Strict-Transport-Security",
        "?X-Strict-Transport-Security",
        "Strict-Transport-Security",
    ),
    (
        "Caddyfile",
        "includeSubDomains",
        "xincludeSubDomains",
        "Strict-Transport-Security",
    ),
    (
        "Caddyfile",
        "keepalive 5s",
        "keepalive 9s",
        "proxy.upstream_keepalive",
    ),
    (
        "Caddyfile",
        "keepalive 5s",
        "keepalive 11s",
        "proxy.upstream_keepalive",
    ),
    (
        "Caddyfile",
        "keepalive 5s",
        "tcp_keepalive 5s",
        "proxy.upstream_keepalive",
    ),
    (
        "Caddyfile",
        "max-age=31536000",
        "max-age=100",
        "http.hsts_max_age",
    ),
    (
        "Caddyfile",
        "includeSubDomains",
        "preload",
        "Strict-Transport-Security",
    ),
    (
        "Caddyfile",
        "reverse_proxy app:8080",
        "reverse_proxy app:8081",
        "http.public_port",
    ),
    (
        "Caddyfile.dev",
        "keepalive 5s",
        "keepalive 9s",
        "Caddyfile.dev",
    ),
    (
        "Caddyfile.dev",
        "reverse_proxy app:8080",
        "reverse_proxy app:8081",
        "Caddyfile.dev",
    ),
];

#[test]
fn a_changed_keepalive_hsts_or_upstream_port_line_is_a_fault() {
    assert_mutations_fail(&CADDYFILE);
}

#[test]
fn a_keepalive_that_equals_the_table_but_reaches_the_body_read_time_is_a_fault() {
    // Rule: proxy.upstream_keepalive is below http.body_read_timeout (reliability.md 7.1, ADR-0065 (i)).
    let table_row = "| `proxy.upstream_keepalive` | 5 s.";
    let base = Inputs::load();
    let changed = base
        .with_replaced(
            "reliability.md",
            table_row,
            "| `proxy.upstream_keepalive` | 10 s.",
        )
        .with_replaced("Caddyfile", "keepalive 5s", "keepalive 10s");
    let report = check_all(&changed);
    assert!(
        report.has_fault_for("proxy.upstream_keepalive"),
        "{}",
        report.message()
    );
    assert!(
        report.message().contains("http.body_read_timeout"),
        "{}",
        report.message()
    );
}

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

const SCRIPT_CONSTANTS: [Mutation; 13] = [
    // Bash rejects a unit in an arithmetic value, although 720h equals 30 days.
    (
        "backup-once",
        "RETENTION_DAYS=30",
        "RETENTION_DAYS=720h",
        "backup.retention",
    ),
    // Bash arithmetic reads 030 as octal 24.
    (
        "backup-once",
        "RETENTION_DAYS=30",
        "RETENTION_DAYS=030",
        "backup.retention",
    ),
    // A later assignment wins in the shell, so it is the value under test.
    (
        "restore-test",
        "STOP_TIMEOUT_SECONDS=60",
        "STOP_TIMEOUT_SECONDS=60\nSTOP_TIMEOUT_SECONDS=1",
        "scratch.stop_timeout",
    ),
    (
        "pitr-check",
        "SCRATCH_PORT=5433",
        "SCRATCH_PORT=5433\nSCRATCH_PORT=5434",
        "scratch.port",
    ),
    (
        "backup-health",
        "MAX_AGE_SECONDS=93600",
        "MAX_AGE_SECONDS=86400",
        "backup.max_age_alert",
    ),
    (
        "backup-once",
        "RETENTION_DAYS=30",
        "RETENTION_DAYS=14",
        "backup.retention",
    ),
    (
        "basebackup-once",
        "RETENTION_COUNT=4",
        "RETENTION_COUNT=2",
        "basebackup.retention",
    ),
    (
        "restore-test",
        "SCRATCH_PORT=5433",
        "SCRATCH_PORT=5434",
        "scratch.port",
    ),
    (
        "restore-test",
        "START_TIMEOUT_SECONDS=60",
        "START_TIMEOUT_SECONDS=90",
        "scratch.restore_start_timeout",
    ),
    (
        "pitr-check",
        "START_TIMEOUT_SECONDS=600",
        "START_TIMEOUT_SECONDS=300",
        "scratch.pitr_start_timeout",
    ),
    (
        "restore-test",
        "DETAIL_LENGTH=160",
        "DETAIL_LENGTH=80",
        "restore_test.detail_length",
    ),
    (
        "pitr-check",
        "LOG_TAIL_LINES=20",
        "LOG_TAIL_LINES=10",
        "pitr_check.log_tail",
    ),
    (
        "pitr-check",
        "READY_POLL_SECONDS=1",
        "READY_POLL_SECONDS=2",
        "pitr_check.ready_poll",
    ),
];

#[test]
fn a_changed_constant_of_a_pgtools_script_is_a_fault() {
    assert_mutations_fail(&SCRIPT_CONSTANTS);
}

const SCRATCH_SETTINGS: [Mutation; 8] = [
    // A word in a comment is no setting.
    (
        "restore-test",
        "max_connections=10",
        "max_connections=20 # max_connections=10",
        "scratch.max_connections",
    ),
    (
        "pitr-check",
        "'shared_buffers=64MB'",
        "'shared_buffers=64MB' 'shared_buffers=128MB'",
        "scratch.shared_buffers",
    ),
    (
        "restore-test",
        "fsync=off",
        "fsync=on",
        "scratch.restore_fsync",
    ),
    (
        "restore-test",
        "shared_buffers=64MB",
        "shared_buffers=128MB",
        "scratch.shared_buffers",
    ),
    (
        "restore-test",
        "wal_level=minimal",
        "wal_level=replica",
        "scratch.restore_wal_level",
    ),
    (
        "pitr-check",
        "'hot_standby=off'",
        "'hot_standby=on'",
        "scratch.pitr_hot_standby",
    ),
    (
        "pitr-check",
        "'max_connections=10'",
        "'max_connections=20'",
        "scratch.max_connections",
    ),
    (
        "pitr-check",
        "'hot_standby=off'",
        "'hot_standby=off' 'fsync=off'",
        "fsync",
    ),
];

#[test]
fn a_changed_setting_of_a_scratch_cluster_is_a_fault() {
    assert_mutations_fail(&SCRATCH_SETTINGS);
}

const ONE_ASSIGNMENT: [Mutation; 9] = [
    // The shell runs every assignment, so the file holds exactly one assignment of a name.
    (
        "backup-once",
        "RETENTION_DAYS=30",
        "RETENTION_DAYS=30\nRETENTION_DAYS=14",
        "backup.retention",
    ),
    (
        "backup-once",
        "RETENTION_DAYS=30",
        "RETENTION_DAYS=14\nRETENTION_DAYS=30",
        "backup.retention",
    ),
    // A prefix word does not hide an assignment.
    (
        "backup-once",
        "RETENTION_DAYS=30",
        "RETENTION_DAYS=30\nreadonly RETENTION_DAYS=14",
        "backup.retention",
    ),
    (
        "backup-once",
        "RETENTION_DAYS=30",
        "RETENTION_DAYS=30\nexport RETENTION_DAYS=14",
        "backup.retention",
    ),
    (
        "backup-once",
        "RETENTION_DAYS=30",
        "RETENTION_DAYS=30\ndeclare -r RETENTION_DAYS=14",
        "backup.retention",
    ),
    (
        "backup-once",
        "RETENTION_DAYS=30",
        "RETENTION_DAYS=30\nlocal RETENTION_DAYS=14",
        "backup.retention",
    ),
    (
        "backup-once",
        "RETENTION_DAYS=30",
        "RETENTION_DAYS_OLD=30",
        "backup.retention",
    ),
    // A later word `name=value` in another command line hides a changed setting.
    (
        "restore-test",
        "max_wal_senders=0",
        "max_wal_senders=0\necho max_connections=10",
        "scratch.max_connections",
    ),
    (
        "pitr-check",
        "'max_connections=10'",
        "'max_connections=20'\necho max_connections=10",
        "scratch.max_connections",
    ),
];

#[test]
fn a_second_or_a_missing_assignment_of_a_script_name_is_a_fault() {
    assert_mutations_fail(&ONE_ASSIGNMENT);
}

#[test]
fn a_missing_scratch_setting_is_a_fault() {
    let missing: [Mutation; 1] = [(
        "restore-test",
        "max_connections=10",
        "max_connection=10",
        "scratch.max_connections",
    )];
    assert_mutations_fail(&missing);
}

#[test]
fn a_prefix_word_on_the_only_assignment_is_no_fault() {
    for prefix in ["export ", "readonly ", "declare -r ", "local "] {
        let changed = Inputs::load().with_replaced(
            "backup-once",
            "RETENTION_DAYS=30",
            &format!("{prefix}RETENTION_DAYS=30"),
        );
        let report = check_all(&changed);
        assert!(
            !report.has_fault_for("backup.retention"),
            "{prefix:?}: {}",
            report.message()
        );
    }
}
