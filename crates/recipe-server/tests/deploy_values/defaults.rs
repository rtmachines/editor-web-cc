//! Checks of the defaults that `deploy/` repeats from `reliability.md` 7.11.
//!
//! A shell default has the form `${NAME:-value}`. The checks read every such form of a variable in a
//! script, so a default that nobody sets in Compose still equals the table. The Dockerfile `EXPOSE`
//! line holds the two listener ports.

use crate::canon::Bare;
use crate::compose::port_text;
use crate::fault::Report;
use crate::inputs::{DOCKERFILE, INITDB};
use crate::lines::{code_lines, without_comment};
use crate::secrets::INITDB_SECRETS;
use crate::table::Table;

const SCRIPT_DIRECTORY: &str = "deploy/pgtools/bin/";
/// A default of a script: the script, the variable, and the key of 7.11.
/// The rows `db.user` and `db.backup_user` do not exist, so the `PGUSER` defaults are not here.
const SCRIPT_DEFAULTS: [(&str, &str, &str); 11] = [
    ("backup-once", "PGHOST", "db.host"),
    ("backup-once", "PGPORT", "db.port"),
    ("backup-once", "PGDATABASE", "db.name"),
    (
        "backup-once",
        "BACKUP_SECRET_FILE",
        "secrets.backup_password_file",
    ),
    ("basebackup-once", "PGHOST", "db.host"),
    ("basebackup-once", "PGPORT", "db.port"),
    (
        "basebackup-once",
        "WALARCHIVE_SECRET_FILE",
        "secrets.replicator_password_file",
    ),
    ("restore-test", "PGDATABASE", "db.name"),
    ("walarchive-entry", "PGHOST", "db.host"),
    ("walarchive-entry", "PGPORT", "db.port"),
    (
        "walarchive-entry",
        "WALARCHIVE_SECRET_FILE",
        "secrets.replicator_password_file",
    ),
];

/// The value of each form `${name:-value}` on the code lines, in file order.
fn default_values<'a>(lines: &[&'a str], name: &str) -> Vec<&'a str> {
    let opener = format!("${{{name}:-");
    lines
        .iter()
        .copied()
        .flat_map(|line| without_comment(line).split(opener.as_str()).skip(1))
        .filter_map(|rest| rest.split_once('}').map(|(value, _)| value))
        .collect()
}

/// Every default of a script in `SCRIPT_DEFAULTS` equals its key in 7.11. A missing default is a fault.
pub fn check_script_defaults(script: &str, text: &str, table: &Table) -> Report {
    let mut report = Report::default();
    let lines = code_lines(text);
    let path = format!("{SCRIPT_DIRECTORY}{script}");
    for (file, name, key) in SCRIPT_DEFAULTS {
        if file != script {
            continue;
        }
        report.cover(key);
        let label = format!("default of {name} ({key})");
        let found = default_values(&lines, name);
        if found.is_empty() {
            report.compare(&path, &label, table.expected(key), None, Bare::Plain);
        }
        for value in found {
            report.compare(&path, &label, table.expected(key), Some(value), Bare::Plain);
        }
    }
    report
}

/// The directory of a secret path of 7.11, for example `/run/secrets` for `/run/secrets/app_password`.
fn secret_directory(table: &Table, key: &str) -> Option<String> {
    let path = table.literal(key)?;
    path.rsplit_once('/')
        .map(|(directory, _)| directory.to_owned())
}

/// The default of `SECRET_DIR` in the initdb script is the directory of the four password paths of 7.11.
pub fn check_secret_dir(text: &str, table: &Table) -> Report {
    let mut report = Report::default();
    let found = default_values(&code_lines(text), "SECRET_DIR");
    for key in INITDB_SECRETS {
        report.cover(key);
        let label = format!("default of SECRET_DIR ({key})");
        let Some(directory) = secret_directory(table, key) else {
            report.fail(INITDB, &label, "a path in 7.11", "none");
            continue;
        };
        if found.is_empty() {
            report.fail(INITDB, &label, directory.clone(), "missing");
        }
        for value in &found {
            if *value != directory {
                report.fail(INITDB, &label, directory.clone(), *value);
            }
        }
    }
    report
}

/// The `EXPOSE` lines of the Dockerfile name the public port and the internal port of 7.11, in this order.
pub fn check_dockerfile_ports(text: &str, table: &Table) -> Report {
    let mut report = Report::default();
    report.cover("http.public_port");
    report.cover("http.internal_port");
    let want = [
        port_text(table, "http.public_port").unwrap_or_default(),
        port_text(table, "http.internal_port").unwrap_or_default(),
    ];
    let found: Vec<String> = code_lines(text)
        .into_iter()
        .filter_map(|line| line.trim().strip_prefix("EXPOSE "))
        .flat_map(str::split_whitespace)
        .map(|port| port.split('/').next().unwrap_or(port).to_owned())
        .collect();
    report.compare_list(
        DOCKERFILE,
        "EXPOSE (http.public_port, http.internal_port)",
        &want,
        Some(found),
    );
    report
}
