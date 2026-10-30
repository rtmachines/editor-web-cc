//! Checks of the secret names and the roles that `deploy/` repeats from `reliability.md` 7.11.
//!
//! The rows `secrets.*_password_file` give the container path of a password file. The last part of
//! the path is the name of the Docker secret. The row `db.migrator_user` gives the name of a role.

use crate::fault::Report;
use crate::inputs::{CLUSTER_ACCESS, INITDB};
use crate::lines::Compose;
use crate::table::Table;

const COMPOSE: &str = "deploy/compose.yaml";
const SECRET_MOUNT: &str = "/run/secrets/";
/// The services with a `secrets:` list and the key of 7.11 for each secret in it (`reliability.md` 7.1, "Secrets").
const SECRET_LISTS: [(&str, &str); 10] = [
    ("backup-premigrate", "secrets.backup_password_file"),
    ("migrate", "secrets.migrator_password_file"),
    ("app", "secrets.app_password_file"),
    ("db", "secrets.postgres_password_file"),
    ("db", "secrets.migrator_password_file"),
    ("db", "secrets.app_password_file"),
    ("db", "secrets.backup_password_file"),
    ("db", "secrets.replicator_password_file"),
    ("backup", "secrets.backup_password_file"),
    ("walarchive", "secrets.replicator_password_file"),
];
/// The keys of the four login passwords that the initdb script reads (`reliability.md` 7.11).
pub const INITDB_SECRETS: [&str; 4] = [
    "secrets.migrator_password_file",
    "secrets.app_password_file",
    "secrets.backup_password_file",
    "secrets.replicator_password_file",
];
const MIGRATOR_USER: &str = "db.migrator_user";

/// The name of the Docker secret of a key: the part of its path after `/run/secrets/`.
fn secret_name(table: &Table, key: &str) -> Option<String> {
    let path = table.literal(key)?;
    path.strip_prefix(SECRET_MOUNT).map(str::to_owned)
}

/// Each `secrets:` list of a service holds the secret that its key of 7.11 names.
pub fn check_secret_lists(compose: &Compose, table: &Table) -> Report {
    let mut report = Report::default();
    for (service_name, key) in SECRET_LISTS {
        report.cover(key);
        let label = format!("{service_name}.secrets ({key})");
        let Some(name) = secret_name(table, key) else {
            report.fail(
                COMPOSE,
                &label,
                "a path below /run/secrets/ in 7.11",
                "none",
            );
            continue;
        };
        let Some(service) = compose.service(service_name) else {
            report.fail(COMPOSE, &label, "the service exists", "missing");
            continue;
        };
        let listed = service.list("secrets");
        if !listed.contains(&name) {
            report.fail(COMPOSE, &label, name, format!("{listed:?}"));
        }
    }
    report
}

/// The user of a `pg_hba.conf` line that `cluster-access.sh` writes, for example `printf 'host recipe user ...`.
fn hba_user(line: &str) -> Option<&str> {
    let mut words = line.split_whitespace();
    words.find(|word| *word == "'host")?;
    words.nth(1)
}

/// The name of the role that `CREATE ROLE <name> ...` makes.
fn created_role(line: &str) -> Option<&str> {
    let mut words = line.split_whitespace();
    (words.next()? == "CREATE" && words.next()? == "ROLE")
        .then(|| words.next())
        .flatten()
}

/// The initdb script reads the four password files and creates the migrator role of 7.11.
/// The shared client rules in `cluster-access.sh` hold the `pg_hba.conf` line of that role (ADR-0103 (l)).
pub fn check_initdb(text: &str, cluster_access: &str, table: &Table) -> Report {
    let mut report = Report::default();
    for key in INITDB_SECRETS {
        report.cover(key);
        let Some(name) = secret_name(table, key) else {
            report.fail(INITDB, key, "a path below /run/secrets/ in 7.11", "none");
            continue;
        };
        let want = format!("\"$SECRET_DIR/{name}\"");
        if !text.contains(&want) {
            report.fail(INITDB, key, want, "missing");
        }
    }
    report.cover(MIGRATOR_USER);
    let Some(user) = table.literal(MIGRATOR_USER) else {
        report.fail(INITDB, MIGRATOR_USER, "a role name in 7.11", "none");
        return report;
    };
    if !text.lines().any(|line| created_role(line) == Some(&user)) {
        let label = format!("{MIGRATOR_USER} CREATE ROLE");
        report.fail(INITDB, &label, user.clone(), "missing");
    }
    if !cluster_access
        .lines()
        .any(|line| hba_user(line) == Some(&user))
    {
        let label = format!("{MIGRATOR_USER} pg_hba.conf line");
        report.fail(CLUSTER_ACCESS, &label, user, "missing");
    }
    report
}
