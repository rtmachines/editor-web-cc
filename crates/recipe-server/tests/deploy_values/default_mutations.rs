//! Proof that the defaults of the pgtools scripts, of the initdb script, and of the Dockerfile can fail a check.
//!
//! Each case changes one copy of a text and asserts a fault that names the key of `reliability.md` 7.11.

use crate::inputs::{Mutation, assert_mutations_fail};

const CONNECTION_DEFAULTS: [Mutation; 7] = [
    // The default of `PGPORT` in two scripts, and the host and the database name.
    (
        "backup-once",
        "${PGPORT:-5432}",
        "${PGPORT:-5433}",
        "db.port",
    ),
    (
        "walarchive-entry",
        "${PGPORT:-5432}",
        "${PGPORT:-5433}",
        "db.port",
    ),
    (
        "basebackup-once",
        "${PGHOST:-db}",
        "${PGHOST:-database}",
        "db.host",
    ),
    (
        "backup-once",
        "${PGDATABASE:-recipe}",
        "${PGDATABASE:-recipes}",
        "db.name",
    ),
    (
        "restore-test",
        "${PGDATABASE:-recipe}",
        "${PGDATABASE:-recipes}",
        "db.name",
    ),
    // A changed row of the table is a fault in each script that repeats it.
    (
        "reliability.md",
        "`db` / 5432 / `recipe`",
        "`db` / 5433 / `recipe`",
        "backup-once",
    ),
    (
        "walarchive-entry",
        "export PGHOST=${PGHOST:-db}",
        "export PGHOST=${PGHOST:-db}\nexport PGHOST=${PGHOST:-other}",
        "db.host",
    ),
];

#[test]
fn a_changed_connection_default_of_a_script_is_a_fault_that_names_the_key() {
    assert_mutations_fail(&CONNECTION_DEFAULTS);
}

const SECRET_DEFAULTS: [Mutation; 4] = [
    (
        "backup-once",
        "${BACKUP_SECRET_FILE:-/run/secrets/backup_password}",
        "${BACKUP_SECRET_FILE:-/run/secrets/backup_pass}",
        "secrets.backup_password_file",
    ),
    (
        "walarchive-entry",
        "${WALARCHIVE_SECRET_FILE:-/run/secrets/replicator_password}",
        "${WALARCHIVE_SECRET_FILE:-/run/secrets/replicator_pass}",
        "secrets.replicator_password_file",
    ),
    (
        "basebackup-once",
        "${WALARCHIVE_SECRET_FILE:-/run/secrets/replicator_password}",
        "${WALARCHIVE_SECRET_FILE:-/run/secrets/backup_password}",
        "secrets.replicator_password_file",
    ),
    (
        "01-roles.sh",
        "${SECRET_DIR:-/run/secrets}",
        "${SECRET_DIR:-/run/secret}",
        "SECRET_DIR",
    ),
];

#[test]
fn a_changed_secret_default_of_a_script_is_a_fault_that_names_the_key() {
    assert_mutations_fail(&SECRET_DEFAULTS);
}

const EXPOSED_PORTS: [Mutation; 3] = [
    (
        "Dockerfile",
        "EXPOSE 8080 9000",
        "EXPOSE 8080 9001",
        "http.internal_port",
    ),
    (
        "Dockerfile",
        "EXPOSE 8080 9000",
        "EXPOSE 8081 9000",
        "http.public_port",
    ),
    (
        "Dockerfile",
        "EXPOSE 8080 9000",
        "EXPOSE 8080",
        "http.internal_port",
    ),
];

#[test]
fn a_changed_exposed_port_of_the_dockerfile_is_a_fault_that_names_the_key() {
    assert_mutations_fail(&EXPOSED_PORTS);
}
