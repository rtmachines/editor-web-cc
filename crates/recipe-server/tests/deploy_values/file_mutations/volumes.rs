//! Volume checks: the constants, waits, sizes, and fsync options of the restore and pitr volume scripts.

use crate::inputs::{Mutation, assert_mutations_fail};

const RESTORE_VOLUME_WAITS: [Mutation; 4] = [
    (
        "restore-volume",
        "START_TIMEOUT_SECONDS=60",
        "START_TIMEOUT_SECONDS=90",
        "restore_volume.start_timeout",
    ),
    (
        "restore-volume",
        "STOP_TIMEOUT_SECONDS=600",
        "STOP_TIMEOUT_SECONDS=300",
        "restore_volume.stop_timeout",
    ),
    // A later assignment wins in the shell, so a second assignment is a fault.
    (
        "restore-volume",
        "STOP_TIMEOUT_SECONDS=600",
        "STOP_TIMEOUT_SECONDS=600\nSTOP_TIMEOUT_SECONDS=1",
        "restore_volume.stop_timeout",
    ),
    // The wait of the scratch clusters is another row of 7.11.
    (
        "restore-volume",
        "START_TIMEOUT_SECONDS=60",
        "START_TIMEOUT_SECONDS=600",
        "restore_volume.start_timeout",
    ),
];

#[test]
fn a_changed_wait_of_restore_volume_is_a_fault() {
    assert_mutations_fail(&RESTORE_VOLUME_WAITS);
}

const PITR_VOLUME_CONSTANTS: [Mutation; 16] = [
    (
        "pitr-volume",
        "SCRATCH_PORT=5433",
        "SCRATCH_PORT=5434",
        "scratch.port",
    ),
    // A later assignment wins in the shell, so a second assignment is a fault.
    (
        "pitr-volume",
        "SCRATCH_PORT=5433",
        "SCRATCH_PORT=5433\nSCRATCH_PORT=5434",
        "scratch.port",
    ),
    (
        "pitr-volume",
        "CLEAN_LENGTH=160",
        "CLEAN_LENGTH=80",
        "restore_test.detail_length",
    ),
    (
        "pitr-volume",
        "CLEAN_LENGTH=160",
        "CLEAN_LENGTH=0160",
        "restore_test.detail_length",
    ),
    (
        "pitr-volume",
        "CLEAN_LENGTH=160",
        "CLEAN_LENGTH=160\nCLEAN_LENGTH=161",
        "restore_test.detail_length",
    ),
    (
        "pitr-volume",
        "DETAIL_LENGTH=160",
        "DETAIL_LENGTH=80",
        "restore_test.detail_length",
    ),
    (
        "pitr-volume",
        "DETAIL_LENGTH=160",
        "DETAIL_LENGTH=0160",
        "restore_test.detail_length",
    ),
    (
        "pitr-volume",
        "DETAIL_LENGTH=160",
        "DETAIL_LENGTH=160\nDETAIL_LENGTH=161",
        "restore_test.detail_length",
    ),
    (
        "pitr-volume",
        "START_TIMEOUT_SECONDS=1800",
        "START_TIMEOUT_SECONDS=900",
        "pitr_volume.start_timeout",
    ),
    (
        "pitr-volume",
        "STOP_TIMEOUT_SECONDS=600",
        "STOP_TIMEOUT_SECONDS=300",
        "pitr_volume.stop_timeout",
    ),
    // A later assignment wins in the shell, so a second assignment is a fault.
    (
        "pitr-volume",
        "STOP_TIMEOUT_SECONDS=600",
        "STOP_TIMEOUT_SECONDS=600
STOP_TIMEOUT_SECONDS=1",
        "pitr_volume.stop_timeout",
    ),
    // The wait of the scratch server of `pitr-check` is another row of 7.11.
    (
        "pitr-volume",
        "START_TIMEOUT_SECONDS=1800",
        "START_TIMEOUT_SECONDS=600",
        "pitr_volume.start_timeout",
    ),
    (
        "pitr-volume",
        "READY_POLL_SECONDS=1",
        "READY_POLL_SECONDS=2",
        "pitr_check.ready_poll",
    ),
    (
        "pitr-volume",
        "LOG_TAIL_LINES=20",
        "LOG_TAIL_LINES=10",
        "pitr_check.log_tail",
    ),
    (
        "pitr-volume",
        "DISK_WARN_PERCENT=15",
        "DISK_WARN_PERCENT=10",
        "disk.warn_percent",
    ),
    (
        "pitr-volume",
        "DISK_WARN_PERCENT=15",
        "DISK_WARN_PERCENT=015",
        "disk.warn_percent",
    ),
];

#[test]
fn a_changed_constant_of_pitr_volume_is_a_fault() {
    assert_mutations_fail(&PITR_VOLUME_CONSTANTS);
}

const RESTORE_VOLUME_SIZES: [Mutation; 6] = [
    (
        "restore-volume",
        "shared_buffers=64MB",
        "shared_buffers=128MB",
        "scratch.shared_buffers",
    ),
    (
        "restore-volume",
        "maintenance_work_mem=64MB",
        "maintenance_work_mem=128MB",
        "scratch.maintenance_work_mem",
    ),
    (
        "restore-volume",
        "max_connections=10",
        "max_connections=20",
        "scratch.max_connections",
    ),
    (
        "restore-volume",
        "max_parallel_workers_per_gather=0",
        "max_parallel_workers_per_gather=2",
        "scratch.max_parallel_workers_per_gather",
    ),
    (
        "restore-volume",
        "max_parallel_maintenance_workers=0",
        "max_parallel_maintenance_workers=2",
        "scratch.max_parallel_maintenance_workers",
    ),
    // A word in a comment is no setting, and a second word is a fault.
    (
        "restore-volume",
        "max_connections=10",
        "max_connections=10 max_connections=20",
        "scratch.max_connections",
    ),
];

#[test]
fn a_changed_scratch_size_of_restore_volume_is_a_fault() {
    assert_mutations_fail(&RESTORE_VOLUME_SIZES);
}

const RESTORE_VOLUME_FSYNC: [Mutation; 3] = [
    // The temporary server keeps the durability settings of initdb (ADR-0103 (j)).
    (
        "restore-volume",
        "max_connections=10",
        "max_connections=10 fsync=off",
        "fsync",
    ),
    (
        "restore-volume",
        "max_connections=10",
        "max_connections=10 fsync=on",
        "fsync",
    ),
    (
        "restore-volume",
        "max_connections=10",
        "max_connections=10 'fsync=off'",
        "fsync",
    ),
];

#[test]
fn an_fsync_option_of_restore_volume_is_a_fault() {
    assert_mutations_fail(&RESTORE_VOLUME_FSYNC);
}
