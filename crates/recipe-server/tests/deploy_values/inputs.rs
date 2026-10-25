//! The text of the files under test and the run of every group of checks on it.
//!
//! `check_all` takes the text as an argument, so a test can change a copy of one file.

use std::path::{Path, PathBuf};

use crate::compose::{
    check_db_command, check_environment, check_grace_periods, check_healthchecks, check_limits,
    check_ports,
};
use crate::defaults::{check_dockerfile_ports, check_script_defaults, check_secret_dir};
use crate::fault::Report;
use crate::files::{
    check_caddyfile, check_crontab, check_env_example, check_scratch_settings,
    check_script_assignments, parse_env,
};
use crate::lines::Compose;
use crate::overrides::{BUILD, DEV, check_override};
use crate::rules::{check_required_keys, check_sum_rules};
use crate::secrets::{check_initdb, check_secret_lists};
use crate::table::{HEADING_POSTGRES, HEADING_SETTINGS, Table};
use crate::topology::{
    check_backend_network, check_logging, check_secrets, check_security, check_trusted_proxies,
};
use crate::units::check_units;

pub const RELIABILITY: &str = "docs/architecture/reliability.md";
pub const COMPOSE: &str = "deploy/compose.yaml";
pub const COMPOSE_DEV: &str = "deploy/compose.dev.yaml";
pub const COMPOSE_BUILD: &str = "deploy/compose.build.yaml";
pub const ENV_EXAMPLE: &str = "deploy/.env.example";
pub const INITDB: &str = "deploy/db/initdb/01-roles.sh";
pub const DOCKERFILE: &str = "deploy/Dockerfile";
const CADDYFILES: [&str; 2] = ["deploy/Caddyfile", "deploy/Caddyfile.dev"];
/// The crontab files, and the key of `reliability.md` 7.11 and the command for each line.
const CRONTABS: [(&str, &[(&str, &str)]); 2] = [
    (
        "deploy/pgtools/crontab.backup",
        &[
            ("backup.time", "backup-once"),
            ("restore_test.time", "restore-test"),
        ],
    ),
    (
        "deploy/pgtools/crontab.walarchive",
        &[("basebackup.time", "basebackup-once")],
    ),
];
/// Every script of `deploy/pgtools/bin/`. A script holds a constant or a default of 7.11, or none.
const SCRIPTS: [&str; 9] = [
    "backup-entry",
    "backup-health",
    "backup-now",
    "backup-once",
    "basebackup-once",
    "pitr-check",
    "restore-test",
    "walarchive-entry",
    "walarchive-health",
];
const SCRIPT_DIRECTORY: &str = "deploy/pgtools/bin/";

/// A file under test: the path from the repository root and its text.
#[derive(Clone)]
pub struct Source {
    pub path: String,
    pub text: String,
}

/// Every file under test.
#[derive(Clone)]
pub struct Inputs {
    files: Vec<Source>,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Read a file as bytes. A CRLF line ending is read as LF by the line functions.
fn read_text(path: &str) -> String {
    let full = repository_root().join(path);
    let bytes = std::fs::read(&full)
        .map_err(|error| format!("cannot read {}: {error}", full.display()))
        .unwrap();
    String::from_utf8(bytes)
        .map_err(|error| format!("{} is not UTF-8: {error}", full.display()))
        .unwrap()
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

impl Inputs {
    /// Read the files from the working tree.
    pub fn load() -> Inputs {
        let mut paths: Vec<String> = vec![
            RELIABILITY.to_owned(),
            COMPOSE.to_owned(),
            COMPOSE_DEV.to_owned(),
            COMPOSE_BUILD.to_owned(),
            ENV_EXAMPLE.to_owned(),
            INITDB.to_owned(),
            DOCKERFILE.to_owned(),
        ];
        paths.extend(CADDYFILES.map(String::from));
        paths.extend(CRONTABS.iter().map(|(path, _)| (*path).to_owned()));
        paths.extend(SCRIPTS.map(|name| format!("{SCRIPT_DIRECTORY}{name}")));
        let files = paths
            .into_iter()
            .map(|path| Source {
                text: read_text(&path),
                path,
            })
            .collect();
        Inputs { files }
    }

    /// The text of a file under test.
    pub fn text(&self, path: &str) -> &str {
        let found = self.files.iter().find(|source| source.path == path);
        &found.expect("the file is under test").text
    }

    /// A copy in which every `from` of the file with this name is `to`.
    /// The call stops when the file is unknown or does not hold `from`, so a mutation is never a no-op.
    pub fn with_replaced(&self, name: &str, from: &str, to: &str) -> Inputs {
        let mut files = self.files.clone();
        let target = files
            .iter_mut()
            .find(|source| file_name(&source.path) == name)
            .expect("the name of a file under test");
        assert!(target.text.contains(from), "{name} does not hold {from:?}");
        target.text = target.text.replace(from, to);
        Inputs { files }
    }

    fn with_paths<'a>(&'a self, paths: &'a [&str]) -> impl Iterator<Item = &'a Source> {
        self.files
            .iter()
            .filter(|source| paths.contains(&source.path.as_str()))
    }
}

fn check_scripts(inputs: &Inputs, settings: &Table) -> Report {
    let mut report = Report::default();
    for name in SCRIPTS {
        let text = inputs.text(&format!("{SCRIPT_DIRECTORY}{name}"));
        report.absorb(check_script_assignments(name, text, settings));
        report.absorb(check_script_defaults(name, text, settings));
        if name == "restore-test" || name == "pitr-check" {
            report.absorb(check_scratch_settings(name, text, settings));
        }
    }
    report
}

fn check_crontabs(inputs: &Inputs, settings: &Table) -> Report {
    let mut report = Report::default();
    for (path, entries) in CRONTABS {
        report.absorb(check_crontab(path, inputs.text(path), entries, settings));
    }
    report
}

fn check_compose(inputs: &Inputs, settings: &Table, postgres: &Table) -> Report {
    let compose = Compose::parse(inputs.text(COMPOSE));
    let env = parse_env(inputs.text(ENV_EXAMPLE));
    let mut report = Report::default();
    report.absorb(check_healthchecks(&compose, settings));
    report.absorb(check_grace_periods(&compose, settings));
    report.absorb(check_limits(&compose, settings));
    report.absorb(check_db_command(&compose, settings, postgres));
    report.absorb(check_environment(&compose, settings));
    report.absorb(check_secret_lists(&compose, settings));
    report.absorb(check_ports(&compose, settings));
    report.absorb(check_security(&compose));
    report.absorb(check_logging(&compose, settings));
    report.absorb(check_backend_network(&compose, &env));
    report.absorb(check_trusted_proxies(&compose, &env, settings));
    report.absorb(check_secrets(&compose));
    report
}

/// One mutation: the name of the file, the text to replace, the new text, and a part of the key
/// (or of the file) that the fault must name.
pub type Mutation = (&'static str, &'static str, &'static str, &'static str);

/// Prove that every mutation makes `check_all` report a fault that names its part.
pub fn assert_mutations_fail(cases: &[Mutation]) {
    let base = Inputs::load();
    for (name, from, to, part) in cases {
        let report = check_all(&base.with_replaced(name, from, to));
        assert!(
            report.has_fault_for(part),
            "no fault for {part:?} after {name}: {from:?} -> {to:?}\nfaults:\n{}",
            report.message()
        );
    }
}

/// Run every group of checks on the inputs.
pub fn check_all(inputs: &Inputs) -> Report {
    let reliability = inputs.text(RELIABILITY);
    let settings = Table::parse(reliability, HEADING_SETTINGS);
    let postgres = Table::parse(reliability, HEADING_POSTGRES);
    let mut report = Report::default();
    if settings.len() == 0 || postgres.len() == 0 {
        report.fail(
            RELIABILITY,
            "tables 7.11 and 7.6",
            "rows in both tables",
            "no rows",
        );
    }
    report.absorb(check_units(&settings));
    report.absorb(check_compose(inputs, &settings, &postgres));
    report.absorb(check_override(COMPOSE_DEV, inputs.text(COMPOSE_DEV), &DEV));
    report.absorb(check_override(
        COMPOSE_BUILD,
        inputs.text(COMPOSE_BUILD),
        &BUILD,
    ));
    report.absorb(check_env_example(inputs.text(ENV_EXAMPLE)));
    report.absorb(check_initdb(inputs.text(INITDB), &settings));
    report.absorb(check_secret_dir(inputs.text(INITDB), &settings));
    report.absorb(check_dockerfile_ports(inputs.text(DOCKERFILE), &settings));
    for source in inputs.with_paths(&CADDYFILES) {
        report.absorb(check_caddyfile(&source.path, &source.text, &settings));
    }
    report.absorb(check_crontabs(inputs, &settings));
    report.absorb(check_scripts(inputs, &settings));
    report.absorb(check_sum_rules(&settings));
    let required = check_required_keys(&settings, &report.covered);
    report.absorb(required);
    report
}
