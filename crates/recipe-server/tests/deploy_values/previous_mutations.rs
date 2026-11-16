//! Proof that the check of `compose.previous.yaml` and `crontab.previous` can fail (ADR-0174 (aa)).
//!
//! The cases change a good copy that this file holds. So they run before the two files exist.
//! The good copy is also the shape that the two files of `deploy/` must have.

use crate::fault::Report;
use crate::inputs::{COMPOSE_PREVIOUS, CRONTAB_PREVIOUS};
use crate::overrides::{PREVIOUS, check_override};
use crate::previous::check_previous;

const GOOD_COMPOSE: &str = "\
name: recipe-editor

services:
  backup:
    secrets: !override
      - backup_password
    environment:
      BACKUP_CRONTAB: /etc/pgtools/crontab.previous
    volumes:
      - ./crontab.previous:/etc/pgtools/crontab.previous:ro
  backup-premigrate:
    secrets: !override
      - backup_password
  restore-volume:
    secrets: !reset []
";

const GOOD_CRONTAB: &str = "\
CRON_TZ=UTC
0 2 * * * backup-once
";

/// The lines of `deploy/pgtools/crontab.backup` that the check reads.
const BACKUP_CRONTAB: &str = "\
CRON_TZ=UTC
0 2 * * * backup-once
0 5 * * 0 restore-test
";

/// A change of one text: the text to replace, the new text, and a part that the fault must name.
type Case = (&'static str, &'static str, &'static str);

fn faults(compose: &str, crontab: &str) -> Report {
    let mut report = check_override(COMPOSE_PREVIOUS, compose, &PREVIOUS);
    report.absorb(check_previous(compose, crontab, BACKUP_CRONTAB));
    report
}

fn assert_changes_fail(base: &str, cases: &[Case], run: impl Fn(&str) -> Report) {
    for (from, to, part) in cases {
        assert!(base.contains(from), "the good copy does not hold {from:?}");
        let report = run(&base.replacen(from, to, 1));
        assert!(
            report.has_fault_for(part),
            "no fault for {part:?} after {from:?} -> {to:?}\nfaults:\n{}",
            report.message()
        );
    }
}

#[test]
fn the_good_copies_have_no_fault() {
    let report = faults(GOOD_COMPOSE, GOOD_CRONTAB);
    assert!(report.faults.is_empty(), "faults:\n{}", report.message());
}

#[test]
fn the_crontab_may_omit_the_zone_line_and_the_compose_file_may_name_the_key_in_a_comment() {
    let compose = GOOD_COMPOSE.replacen(
        "services:",
        "# It never names backup_manifest_key in a key.\nservices:",
        1,
    );
    let crontab = "# The nightly dump only.\n0 2 * * * backup-once\n";
    let report = faults(&compose, crontab);
    assert!(report.faults.is_empty(), "faults:\n{}", report.message());
}

#[test]
fn the_manifest_key_in_the_compose_file_is_a_fault() {
    let cases: [Case; 4] = [
        (
            "      - backup_password\n    environment:",
            "      - backup_password\n      - backup_manifest_key\n    environment:",
            "backup_manifest_key",
        ),
        (
            "  backup-premigrate:\n    secrets: !override\n      - backup_password",
            "  backup-premigrate:\n    secrets: !override\n      - backup_manifest_key",
            "backup_manifest_key",
        ),
        (
            "    secrets: !reset []",
            "    secrets:\n      - backup_manifest_key",
            "backup_manifest_key",
        ),
        (
            "      BACKUP_CRONTAB: /etc/pgtools/crontab.previous",
            "      BACKUP_CRONTAB: /etc/pgtools/crontab.previous\n      BACKUP_MANIFEST_KEY_FILE: /run/secrets/backup_manifest_key",
            "backup_manifest_key",
        ),
    ];
    assert_changes_fail(GOOD_COMPOSE, &cases, |text| faults(text, GOOD_CRONTAB));
}

#[test]
fn a_changed_secret_list_is_a_fault() {
    let cases: [Case; 4] = [
        (
            "  backup:\n    secrets: !override",
            "  backup:\n    secrets:",
            "backup.secrets",
        ),
        (
            "  backup-premigrate:\n    secrets: !override\n      - backup_password",
            "  backup-premigrate:\n    secrets: !override\n      - backup_password\n      - recipe_app_password",
            "backup-premigrate.secrets",
        ),
        (
            "    secrets: !reset []",
            "    secrets: !override\n      - backup_password",
            "restore-volume.secrets",
        ),
        (
            "  backup-premigrate:\n    secrets: !override\n      - backup_password",
            "  backup-premigrate:\n    secrets: !reset []",
            "backup-premigrate.secrets",
        ),
    ];
    assert_changes_fail(GOOD_COMPOSE, &cases, |text| faults(text, GOOD_CRONTAB));
}

#[test]
fn an_extra_key_or_service_in_the_compose_file_is_a_fault() {
    let cases: [Case; 6] = [
        (
            "  backup:\n",
            "  backup:\n    mem_limit: 64m\n",
            "backup.mem_limit",
        ),
        (
            "  restore-volume:\n",
            "  restore-volume:\n    network_mode: host\n",
            "restore-volume.network_mode",
        ),
        (
            "  backup-premigrate:\n",
            "  backup-premigrate:\n    volumes:\n      - ./crontab.previous:/etc/pgtools/crontab.previous:ro\n",
            "backup-premigrate.volumes",
        ),
        (
            "  restore-volume:\n",
            "  restore-volume:\n    environment:\n      BACKUP_CRONTAB: /etc/pgtools/crontab.previous\n",
            "restore-volume.environment",
        ),
        (
            "    secrets: !reset []\n",
            "    secrets: !reset []\n  app:\n    secrets: !reset []\n",
            "app",
        ),
        (
            "services:",
            "networks:\n  backend:\n    internal: false\n\nservices:",
            "networks",
        ),
    ];
    assert_changes_fail(GOOD_COMPOSE, &cases, |text| faults(text, GOOD_CRONTAB));
}

#[test]
fn a_changed_environment_of_backup_is_a_fault() {
    let cases: [Case; 4] = [
        (
            "      BACKUP_CRONTAB: /etc/pgtools/crontab.previous\n",
            "      BACKUP_CRONTAB: /etc/pgtools/crontab.previous\n      BACKUP_DIR: /elsewhere\n",
            "backup.environment",
        ),
        (
            "      BACKUP_CRONTAB: /etc/pgtools/crontab.previous\n",
            "      BACKUP_CRONTAB: /etc/pgtools/crontab.backup\n",
            "backup.environment",
        ),
        (
            "      BACKUP_CRONTAB: /etc/pgtools/crontab.previous\n",
            "      BACKUP_DIR: /elsewhere\n",
            "backup.environment",
        ),
        (
            "    environment:\n      BACKUP_CRONTAB: /etc/pgtools/crontab.previous\n",
            "",
            "backup.environment",
        ),
    ];
    assert_changes_fail(GOOD_COMPOSE, &cases, |text| faults(text, GOOD_CRONTAB));
}

#[test]
fn a_changed_mount_of_backup_is_a_fault() {
    let mount = "      - ./crontab.previous:/etc/pgtools/crontab.previous:ro\n";
    let cases: [Case; 5] = [
        (
            "/etc/pgtools/crontab.previous:ro\n",
            "/etc/pgtools/crontab.previous\n",
            "backup.volumes",
        ),
        (
            "/etc/pgtools/crontab.previous:ro\n",
            "/etc/pgtools/crontab.previous:rw\n",
            "backup.volumes",
        ),
        (
            "./crontab.previous:/etc",
            "./crontab.backup:/etc",
            "backup.volumes",
        ),
        (
            mount,
            "      - ./crontab.previous:/etc/pgtools/crontab.previous:ro\n      - backups:/backups\n",
            "backup.volumes",
        ),
        (
            "    volumes:\n",
            "    volumes: !override\n",
            "backup.volumes",
        ),
    ];
    assert_changes_fail(GOOD_COMPOSE, &cases, |text| faults(text, GOOD_CRONTAB));
}

#[test]
fn a_missing_service_block_is_a_fault() {
    let cases: [Case; 2] = [
        (
            "  backup-premigrate:\n    secrets: !override\n      - backup_password\n",
            "",
            "backup-premigrate",
        ),
        (
            "  restore-volume:\n    secrets: !reset []\n",
            "",
            "restore-volume",
        ),
    ];
    assert_changes_fail(GOOD_COMPOSE, &cases, |text| faults(text, GOOD_CRONTAB));
}

#[test]
fn a_changed_line_in_the_previous_crontab_is_a_fault() {
    let cases: [Case; 7] = [
        (
            "0 2 * * * backup-once\n",
            "0 2 * * * backup-once\n0 5 * * 0 restore-test\n",
            "restore-test",
        ),
        (
            "0 2 * * * backup-once\n",
            "0 2 * * * backup-once\n0 6 * * 1 restore-test\n",
            "restore-test",
        ),
        (
            "0 2 * * * backup-once\n",
            "0 3 * * * backup-once\n",
            "backup-once",
        ),
        (
            "0 2 * * * backup-once\n",
            "0 2 * * * backup-once\n0 2 * * * backup-once\n",
            "backup-once",
        ),
        ("0 2 * * * backup-once\n", "", "backup-once"),
        (
            "0 2 * * * backup-once\n",
            "0 2 * * * backup-once\n0 4 * * * backup-now\n",
            "unexpected line",
        ),
        (
            "CRON_TZ=UTC\n",
            "CRON_TZ=Europe/Warsaw\n",
            "unexpected line",
        ),
    ];
    assert_changes_fail(GOOD_CRONTAB, &cases, |text| faults(GOOD_COMPOSE, text));
}

#[test]
fn the_fault_of_the_previous_crontab_names_its_file() {
    let report = faults(
        GOOD_COMPOSE,
        "0 2 * * * backup-once\n0 5 * * 0 restore-test\n",
    );
    assert!(
        report.has_fault_for(CRONTAB_PREVIOUS),
        "faults:\n{}",
        report.message()
    );
}
