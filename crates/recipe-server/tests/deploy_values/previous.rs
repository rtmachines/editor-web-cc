//! The check of `deploy/compose.previous.yaml` and `deploy/crontab.previous` (ADR-0174 (aa)).
//!
//! An image without the tag check never gets the key. The key check of the generic rule for override
//! files (`overrides.rs`) names the keys. This check names the values below them.

use crate::fault::Report;
use crate::inputs::{COMPOSE_PREVIOUS, CRONTAB_PREVIOUS};
use crate::lines::{Compose, Service, code_lines, indent_of, list_items};

const KEY_SECRET: &str = "backup_manifest_key";
const PASSWORD_SECRET: &str = "backup_password";
const KEEP_PASSWORD: [&str; 2] = ["backup", "backup-premigrate"];
const NO_SECRET: &str = "restore-volume";
const OVERRIDE_TAG: &str = "!override";
const RESET_TAG: &str = "!reset []";
const CRONTAB_VARIABLE: &str = "BACKUP_CRONTAB";
/// Where the previous image reads the mounted crontab.
const CRONTAB_TARGET: &str = "/etc/pgtools/crontab.previous";
/// The one mount of `backup`: the file of `deploy/` beside the Compose file, read-only.
const CRONTAB_MOUNT: &str = "./crontab.previous:/etc/pgtools/crontab.previous:ro";
const BACKUP_JOB: &str = "backup-once";
const WEEKLY_JOB: &str = "restore-test";
const ZONE_PREFIX: &str = "CRON_TZ=";
const KEY_INDENT: usize = 4;

/// The items of a block list below a key of a service. The key may carry a tag such as `!override`.
fn items_below<'a>(service: &Service<'a>, key: &str) -> Vec<&'a str> {
    let header = format!("{key}:");
    let below: Vec<&'a str> = service
        .lines
        .iter()
        .copied()
        .skip_while(|line| !(indent_of(line) == KEY_INDENT && line.trim().starts_with(&header)))
        .skip(1)
        .take_while(|line| indent_of(line) > KEY_INDENT)
        .collect();
    list_items(&below)
}

/// Check both files against the rule of ADR-0174 (aa). `backup_crontab` is `deploy/pgtools/crontab.backup`.
pub fn check_previous(compose_text: &str, crontab_text: &str, backup_crontab: &str) -> Report {
    let mut report = Report::default();
    check_no_key(&mut report, compose_text);
    let compose = Compose::parse(compose_text);
    for name in KEEP_PASSWORD {
        check_kept_password(&mut report, &compose, name);
    }
    check_no_secret(&mut report, &compose);
    check_backup_extras(&mut report, &compose);
    check_crontab_previous(&mut report, crontab_text, backup_crontab);
    report
}

/// No code line of the Compose file names the key secret. A comment may name it.
fn check_no_key(report: &mut Report, text: &str) {
    for line in code_lines(text) {
        if line.contains(KEY_SECRET) {
            report.fail(
                COMPOSE_PREVIOUS,
                KEY_SECRET,
                "no name of the manifest key",
                line.trim(),
            );
        }
    }
}

/// `backup` and `backup-premigrate` keep the password only: `secrets: !override` and one item.
fn check_kept_password(report: &mut Report, compose: &Compose, name: &str) {
    let Some(service) = compose.service(name) else {
        report.fail(COMPOSE_PREVIOUS, name, "a block of this service", "missing");
        return;
    };
    let key = format!("{name}.secrets");
    let tag = service.scalar("secrets").unwrap_or("missing");
    if tag != OVERRIDE_TAG {
        report.fail(
            COMPOSE_PREVIOUS,
            &key,
            format!("secrets: {OVERRIDE_TAG}"),
            tag,
        );
    }
    let items = items_below(service, "secrets");
    if items != [PASSWORD_SECRET] {
        report.fail(
            COMPOSE_PREVIOUS,
            &key,
            format!("[{PASSWORD_SECRET:?}]"),
            format!("{items:?}"),
        );
    }
}

/// `restore-volume` mounts no secret: `secrets: !reset []` and no item.
fn check_no_secret(report: &mut Report, compose: &Compose) {
    let Some(service) = compose.service(NO_SECRET) else {
        report.fail(
            COMPOSE_PREVIOUS,
            NO_SECRET,
            "a block of this service",
            "missing",
        );
        return;
    };
    let key = format!("{NO_SECRET}.secrets");
    let tag = service.scalar("secrets").unwrap_or("missing");
    if tag != RESET_TAG || !items_below(service, "secrets").is_empty() {
        report.fail(COMPOSE_PREVIOUS, &key, format!("secrets: {RESET_TAG}"), tag);
    }
}

/// `backup` sets only `BACKUP_CRONTAB` and adds only the read-only mount of `crontab.previous`.
fn check_backup_extras(report: &mut Report, compose: &Compose) {
    let Some(service) = compose.service("backup") else {
        return;
    };
    let variables: Vec<&str> = service
        .section("environment")
        .into_iter()
        .map(str::trim)
        .collect();
    let wanted = format!("{CRONTAB_VARIABLE}: {CRONTAB_TARGET}");
    if variables != [wanted.as_str()] {
        report.fail(
            COMPOSE_PREVIOUS,
            "backup.environment",
            format!("only {wanted}"),
            format!("{variables:?}"),
        );
    }
    let mounts = items_below(service, "volumes");
    let tag = service.scalar("volumes").unwrap_or("missing");
    if mounts != [CRONTAB_MOUNT] || !tag.is_empty() {
        report.fail(
            COMPOSE_PREVIOUS,
            "backup.volumes",
            format!("one plain list with only {CRONTAB_MOUNT}"),
            format!("{mounts:?} after {tag:?}"),
        );
    }
}

/// The line of a job in the crontab of the project, or `None`.
fn job_line<'a>(lines: &[&'a str], job: &str) -> Option<&'a str> {
    lines
        .iter()
        .copied()
        .find(|line| line.split_whitespace().last() == Some(job))
}

/// `crontab.previous` holds the zone lines and the line of `backup-once` of `crontab.backup`.
/// It holds no line of `restore-test` and no other line.
fn check_crontab_previous(report: &mut Report, text: &str, backup_crontab: &str) {
    let source: Vec<&str> = code_lines(backup_crontab)
        .into_iter()
        .map(str::trim)
        .collect();
    let found: Vec<&str> = code_lines(text).into_iter().map(str::trim).collect();
    let Some(wanted) = job_line(&source, BACKUP_JOB) else {
        report.fail(
            CRONTAB_PREVIOUS,
            BACKUP_JOB,
            "a line in crontab.backup",
            "no line",
        );
        return;
    };
    let count = found.iter().filter(|line| **line == wanted).count();
    if count != 1 {
        let shown = job_line(&found, BACKUP_JOB).unwrap_or("no line");
        report.fail(CRONTAB_PREVIOUS, BACKUP_JOB, wanted, shown);
    }
    for line in found {
        let is_zone = line.starts_with(ZONE_PREFIX) && source.contains(&line);
        if line.split_whitespace().last() == Some(WEEKLY_JOB) {
            report.fail(CRONTAB_PREVIOUS, WEEKLY_JOB, "no line of this job", line);
        } else if line != wanted && !is_zone {
            report.fail(CRONTAB_PREVIOUS, "unexpected line", "no other line", line);
        }
    }
}
