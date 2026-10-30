//! Proof that each group of checks on the Caddyfiles, the crontab files, and the pgtools scripts can fail.
//!
//! Each case changes one copy of a text and asserts a fault. No file of `deploy/` changes.

use crate::files::{
    DB_CONTEXT_LIST, PGTOOLS_CONTEXT_LIST, PGTOOLS_DOCKERFILE, check_context_lists,
    check_line_endings,
};
use crate::inputs::{GITATTRIBUTES_FILE, Inputs, Mutation, assert_mutations_fail, check_all};

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

/// One change of an allow list or of the Dockerfile: the file, the text to replace, the new text,
/// and the file that the fault must name.
type ContextCase = (&'static str, &'static str, &'static str, &'static str);

const CONTEXT_LISTS: [ContextCase; 9] = [
    // The directory source `bin/` needs its entry.
    (PGTOOLS_CONTEXT_LIST, "!bin\n", "", PGTOOLS_CONTEXT_LIST),
    // A suite of tests/ is mounted, not copied. The list never admits it.
    (
        PGTOOLS_CONTEXT_LIST,
        "!bin\n",
        "!bin\n!tests\n",
        PGTOOLS_CONTEXT_LIST,
    ),
    // A file in lib/ needs its own path. A whole directory admits too much.
    (
        PGTOOLS_CONTEXT_LIST,
        "!lib/restore-steps.sh\n",
        "!lib\n",
        PGTOOLS_CONTEXT_LIST,
    ),
    // One list admits each source.
    (
        PGTOOLS_CONTEXT_LIST,
        "!bin\n",
        "!bin\n!cluster-access.sh\n",
        PGTOOLS_CONTEXT_LIST,
    ),
    (
        DB_CONTEXT_LIST,
        "!cluster-access.sh\n",
        "!cluster-access.sh\n!bin\n",
        DB_CONTEXT_LIST,
    ),
    // The first line of code excludes everything.
    (DB_CONTEXT_LIST, "*\n", "!x\n", DB_CONTEXT_LIST),
    // A new COPY source without an entry, and an entry without a source.
    (
        PGTOOLS_DOCKERFILE,
        "COPY --chmod=0644 --from=db cluster-access.sh /etc/pgtools/cluster-access.sh",
        "COPY --chmod=0644 --from=db cluster-access.sh other.sh /etc/pgtools/",
        DB_CONTEXT_LIST,
    ),
    (
        PGTOOLS_DOCKERFILE,
        "crontab.backup crontab.walarchive",
        "crontab.backup",
        PGTOOLS_CONTEXT_LIST,
    ),
    // A COPY from the db context moves its source from one list to the other.
    (
        PGTOOLS_DOCKERFILE,
        "--from=db cluster-access.sh",
        "cluster-access.sh",
        DB_CONTEXT_LIST,
    ),
];

#[test]
fn a_context_list_that_differs_from_the_copy_sources_is_a_fault() {
    let base = Inputs::load();
    let files = [PGTOOLS_DOCKERFILE, DB_CONTEXT_LIST, PGTOOLS_CONTEXT_LIST];
    let texts = files.map(|path| base.text(path).to_owned());
    for (file, from, to, named) in CONTEXT_LISTS {
        let mut changed = texts.clone();
        let index = files.iter().position(|path| *path == file).unwrap();
        assert!(
            changed[index].contains(from),
            "{file} does not hold {from:?}"
        );
        changed[index] = changed[index].replace(from, to);
        let report = check_context_lists(&changed[0], &changed[1], &changed[2]);
        assert!(
            report.has_fault_for(named),
            "no fault for {named:?} after {file}: {from:?} -> {to:?}\nfaults:\n{}",
            report.message()
        );
    }
}

#[test]
fn a_wildcard_source_or_an_unknown_from_is_a_fault() {
    let base = Inputs::load();
    let dockerfile = base.text(PGTOOLS_DOCKERFILE);
    let (db, pgtools) = (base.text(DB_CONTEXT_LIST), base.text(PGTOOLS_CONTEXT_LIST));
    for (from, to) in [
        ("crontab.backup crontab.walarchive", "crontab.* "),
        ("--from=db", "--from=other"),
    ] {
        let changed = dockerfile.replace(from, to);
        assert_ne!(changed, dockerfile, "the Dockerfile does not hold {from:?}");
        let report = check_context_lists(&changed, db, pgtools);
        assert!(
            report.has_fault_for(PGTOOLS_DOCKERFILE),
            "{from:?}: {}",
            report.message()
        );
    }
}

#[test]
fn a_copy_from_a_build_stage_needs_no_entry() {
    let base = Inputs::load();
    let dockerfile = format!(
        "FROM busybox AS tools\n{}\nCOPY --from=tools /bin/tool /usr/local/bin/tool\n",
        base.text(PGTOOLS_DOCKERFILE)
    );
    let report = check_context_lists(
        &dockerfile,
        base.text(DB_CONTEXT_LIST),
        base.text(PGTOOLS_CONTEXT_LIST),
    );
    assert!(report.faults.is_empty(), "{}", report.message());
}

const LINE_ENDINGS: [Mutation; 8] = [
    // A file that a COPY line names needs its own rule.
    (
        GITATTRIBUTES_FILE,
        "deploy/pgtools/restore-code.expected text eol=lf\n",
        "",
        "restore-code.expected",
    ),
    (
        GITATTRIBUTES_FILE,
        "deploy/pgtools/restore-roles.expected text eol=lf\n",
        "",
        "restore-roles.expected",
    ),
    // The rule needs both attributes.
    (
        GITATTRIBUTES_FILE,
        "deploy/pgtools/restore-code.expected text eol=lf",
        "deploy/pgtools/restore-code.expected eol=lf",
        "restore-code.expected",
    ),
    (
        GITATTRIBUTES_FILE,
        "deploy/pgtools/restore-code.expected text eol=lf",
        "deploy/pgtools/restore-code.expected text eol=crlf",
        "restore-code.expected",
    ),
    // A pattern for the files of a directory source covers each file of it.
    (
        GITATTRIBUTES_FILE,
        "deploy/pgtools/bin/* text eol=lf\n",
        "",
        "deploy/pgtools/bin/restore-test",
    ),
    // A basename pattern covers a file of a directory and a file of the other context.
    (
        GITATTRIBUTES_FILE,
        "*.sh text eol=lf\n",
        "",
        "cluster-access.sh",
    ),
    (
        GITATTRIBUTES_FILE,
        "crontab.* text eol=lf\n",
        "",
        "crontab.walarchive",
    ),
    // The two allow lists need a rule.
    (
        GITATTRIBUTES_FILE,
        "deploy/db/.dockerignore text eol=lf\n",
        "",
        "deploy/db/.dockerignore",
    ),
];

#[test]
fn a_copy_source_without_a_line_ending_rule_is_a_fault() {
    assert_mutations_fail(&LINE_ENDINGS);
}

#[test]
fn a_copy_source_that_does_not_exist_is_a_fault() {
    let base = Inputs::load();
    let changed = base
        .text(PGTOOLS_DOCKERFILE)
        .replace("restore-code.sql", "missing-file.sql");
    let report = check_line_endings(&changed, base.text(GITATTRIBUTES_FILE), base.listing());
    assert!(
        report.has_fault_for("missing-file.sql"),
        "{}",
        report.message()
    );
}

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
