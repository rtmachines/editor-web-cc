//! Script checks: the constants, scratch settings, and single assignments of the pgtools scripts.

use crate::inputs::{Inputs, Mutation, assert_mutations_fail, check_all};

const SCRIPT_CONSTANTS: [Mutation; 15] = [
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
        "restore-volume",
        "DETAIL_LENGTH=160",
        "DETAIL_LENGTH=161",
        "restore-volume",
    ),
    // The library `wal-identity.sh` holds its own copy of the constant.
    (
        "wal-identity.sh",
        "DETAIL_LENGTH=160",
        "DETAIL_LENGTH=161",
        "wal-identity.sh",
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

const SCRATCH_SETTINGS: [Mutation; 9] = [
    // A word in a comment is no setting.
    (
        "restore-test",
        "max_connections=10",
        "max_connections=20 # max_connections=10",
        "scratch.max_connections",
    ),
    (
        "recovery-steps.sh",
        "shared_buffers=64MB",
        "shared_buffers=64MB -c shared_buffers=128MB",
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
        "recovery-steps.sh",
        "hot_standby=off",
        "hot_standby=on",
        "scratch.pitr_hot_standby",
    ),
    (
        "recovery-steps.sh",
        "max_connections=10",
        "max_connections=20",
        "scratch.max_connections",
    ),
    (
        "recovery-steps.sh",
        "hot_standby=off",
        "hot_standby=off -c fsync=off",
        "fsync",
    ),
    // The start options of `pitr-check` come from the library, so a copy in the script is a fault.
    (
        "pitr-check",
        "SCRATCH_PORT=5433",
        "SCRATCH_PORT=5433\nstart_options=\"-c fsync=off\"",
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
        "recovery-steps.sh",
        "max_connections=10",
        "max_connections=20\necho max_connections=10",
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
