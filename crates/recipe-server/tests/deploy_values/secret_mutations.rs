//! Proof that the secret paths, the secret lists, and the roles of `deploy/` can fail a check.
//!
//! Each case changes one copy of a text and asserts a fault that names the key of `reliability.md` 7.11.

use crate::inputs::{Mutation, assert_mutations_fail};

const SECRET_AND_ROLE: [Mutation; 15] = [
    // The environment variable of `migrate` and of `db` that holds a secret path.
    (
        "compose.yaml",
        "RECIPE_SECRETS__MIGRATOR_PASSWORD_FILE: /run/secrets/migrator_password",
        "RECIPE_SECRETS__MIGRATOR_PASSWORD_FILE: /run/secrets/migrator_pass",
        "secrets.migrator_password_file",
    ),
    (
        "compose.yaml",
        "POSTGRES_PASSWORD_FILE: /run/secrets/postgres_password",
        "POSTGRES_PASSWORD_FILE: /run/secrets/pg_password",
        "secrets.postgres_password_file",
    ),
    // The `secrets:` list of each service names the file of its key.
    (
        "compose.yaml",
        "- backup_password",
        "- backup_pass",
        "secrets.backup_password_file",
    ),
    (
        "compose.yaml",
        "- replicator_password",
        "- replicator_pass",
        "secrets.replicator_password_file",
    ),
    (
        "compose.yaml",
        "- postgres_password",
        "- postgres_pass",
        "secrets.postgres_password_file",
    ),
    // The initdb script reads the file of each login password.
    (
        "01-roles.sh",
        "\"$SECRET_DIR/migrator_password\"",
        "\"$SECRET_DIR/migrator_pass\"",
        "secrets.migrator_password_file",
    ),
    (
        "01-roles.sh",
        "\"$SECRET_DIR/app_password\"",
        "\"$SECRET_DIR/app_pass\"",
        "secrets.app_password_file",
    ),
    (
        "01-roles.sh",
        "\"$SECRET_DIR/backup_password\"",
        "\"$SECRET_DIR/backup_pass\"",
        "secrets.backup_password_file",
    ),
    (
        "01-roles.sh",
        "\"$SECRET_DIR/replicator_password\"",
        "\"$SECRET_DIR/replicator_pass\"",
        "secrets.replicator_password_file",
    ),
    // The role of the migrator in the initdb script.
    (
        "01-roles.sh",
        "CREATE ROLE recipe_migrator ",
        "CREATE ROLE recipe_migrate  ",
        "db.migrator_user",
    ),
    (
        "cluster-access.sh",
        "host    recipe       recipe_migrator ",
        "host    recipe       recipe_migrate  ",
        "db.migrator_user",
    ),
    // The shared client rules hold the `pg_hba.conf` line of the migrator (ADR-0103 (z)).
    (
        "cluster-access.sh",
        "      printf 'host    recipe       recipe_migrator    %-16s scram-sha-256\\n' \"$subnet\"\n",
        "",
        "db.migrator_user",
    ),
    // A changed row of the table is a fault in each file that repeats it.
    (
        "reliability.md",
        "`recipe_migrator`. The role of",
        "`recipe_migrate`. The role of",
        "db.migrator_user",
    ),
    (
        "reliability.md",
        "`/run/secrets/migrator_password`. Password of",
        "`/run/secrets/migrator_pw`. Password of",
        "secrets.migrator_password_file",
    ),
    (
        "reliability.md",
        "`/run/secrets/postgres_password`. Password of",
        "`/run/secrets/postgres_pw`. Password of",
        "secrets.postgres_password_file",
    ),
];

#[test]
fn a_changed_secret_path_secret_list_or_role_is_a_fault_that_names_the_key() {
    assert_mutations_fail(&SECRET_AND_ROLE);
}
