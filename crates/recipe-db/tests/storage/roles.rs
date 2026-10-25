//! Tests of the delta requirement "Roles from the database init script".

use std::process::{Command, Output};

use sqlx::Row as _;

use crate::support::{Role, TestDatabase, connect, connect_error, server};

/// SQLSTATE of "insufficient privilege". The server checks it after a successful login.
const INSUFFICIENT_PRIVILEGE: &str = "42501";
/// SQLSTATE of "invalid password".
const INVALID_PASSWORD: &str = "28P01";
const DATABASES: [&str; 3] = ["recipe", "postgres", "template1"];
/// A text that only the statement of another session holds.
const STATEMENT_MARKER: &str = "statement-text-of-another-session";
/// The lines of `support/02-test-hosts.sh`. The harness connects from the Docker host.
const HARNESS_HBA_LINES: &[&str] = &[
    "host all all all scram-sha-256",
    "host replication recipe_replicator all scram-sha-256",
];
/// The lines of `01-roles.sh` in the order of `data-model.md` 5.1, with the subnet of the harness.
const PRODUCTION_HBA_LINES: &[&str] = &[
    "local all postgres peer",
    "host recipe recipe_migrator 172.28.0.0/255.255.0.0 scram-sha-256",
    "host recipe recipe_app 172.28.0.0/255.255.0.0 scram-sha-256",
    "host recipe recipe_backup 172.28.0.0/255.255.0.0 scram-sha-256",
    "host replication recipe_replicator 172.28.0.0/255.255.0.0 scram-sha-256",
    "host recipe pg_isready 127.0.0.1/255.255.255.255 scram-sha-256",
    "host all all all reject",
];

#[tokio::test]
async fn the_five_roles_have_the_right_attributes() {
    let mut conn = connect(server(), Role::Superuser, "postgres").await;

    let rows = sqlx::query(
        "SELECT rolname, rolcanlogin, rolreplication, rolsuper, rolcreatedb, rolcreaterole \
         FROM pg_roles \
         WHERE rolname IN ('recipe_migrator', 'recipe_app', 'recipe_backup', \
                           'recipe_replicator', 'audit_owner') \
         ORDER BY rolname COLLATE \"C\"",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();

    let found: Vec<(String, bool, bool, bool, bool, bool)> = rows
        .iter()
        .map(|row| {
            (
                row.get("rolname"),
                row.get("rolcanlogin"),
                row.get("rolreplication"),
                row.get("rolsuper"),
                row.get("rolcreatedb"),
                row.get("rolcreaterole"),
            )
        })
        .collect();
    let expected = vec![
        ("audit_owner".to_owned(), false, false, false, false, false),
        ("recipe_app".to_owned(), true, false, false, false, false),
        ("recipe_backup".to_owned(), true, false, false, false, false),
        (
            "recipe_migrator".to_owned(),
            true,
            false,
            false,
            false,
            false,
        ),
        (
            "recipe_replicator".to_owned(),
            true,
            true,
            false,
            false,
            false,
        ),
    ];
    assert_eq!(found, expected);
}

#[tokio::test]
async fn the_database_recipe_belongs_to_the_migrator() {
    let mut conn = connect(server(), Role::Superuser, "postgres").await;

    let owner: String = sqlx::query_scalar(
        "SELECT pg_get_userbyid(datdba) FROM pg_database WHERE datname = 'recipe'",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();

    assert_eq!(owner, "recipe_migrator");
}

#[tokio::test]
async fn public_holds_no_connect_and_no_temporary_on_the_three_databases() {
    let mut conn = connect(server(), Role::Superuser, "postgres").await;

    for name in DATABASES {
        let has_acl: bool =
            sqlx::query_scalar("SELECT datacl IS NOT NULL FROM pg_database WHERE datname = $1")
                .bind(name)
                .fetch_one(&mut conn)
                .await
                .unwrap();
        let public_privileges: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_database d, aclexplode(d.datacl) a \
             WHERE d.datname = $1 AND a.grantee = 0 \
               AND a.privilege_type IN ('CONNECT', 'TEMPORARY')",
        )
        .bind(name)
        .fetch_one(&mut conn)
        .await
        .unwrap();
        assert!(has_acl, "{name} still has the default privileges");
        assert_eq!(public_privileges, 0, "PUBLIC holds a privilege on {name}");
    }
}

#[tokio::test]
async fn the_connect_grants_on_recipe_go_to_three_login_roles_only() {
    let mut conn = connect(server(), Role::Superuser, "postgres").await;

    for (role, expected) in [
        (Role::Migrator, true),
        (Role::App, true),
        (Role::Backup, true),
        (Role::Replicator, false),
    ] {
        let allowed: bool =
            sqlx::query_scalar("SELECT has_database_privilege($1, 'recipe', 'CONNECT')")
                .bind(role.name())
                .fetch_one(&mut conn)
                .await
                .unwrap();

        assert_eq!(allowed, expected, "CONNECT of {} on recipe", role.name());
    }
}

#[tokio::test]
async fn the_passwords_of_the_secret_files_open_sessions() {
    // The secret files end with a newline. The password must not include it.
    let database = TestDatabase::create().await;

    for role in [Role::Migrator, Role::App, Role::Backup] {
        let mut conn = database.connect(role).await;

        let user: String = sqlx::query_scalar("SELECT current_user::text")
            .fetch_one(&mut conn)
            .await
            .unwrap();

        assert_eq!(user, role.name());
    }
}

#[tokio::test]
async fn recipe_replicator_and_recipe_app_cannot_open_a_session_on_postgres() {
    for role in [Role::Replicator, Role::App] {
        let code = connect_error(server(), role, "postgres").await;

        assert_eq!(code, INSUFFICIENT_PRIVILEGE, "{} on postgres", role.name());
    }
}

#[tokio::test]
async fn recipe_replicator_cannot_open_a_session_on_recipe() {
    let code = connect_error(server(), Role::Replicator, "recipe").await;

    assert_eq!(code, INSUFFICIENT_PRIVILEGE);
}

#[tokio::test]
async fn recipe_replicator_holds_no_privilege_on_any_table() {
    // The test database is a copy of `recipe`. No test connects to `recipe` itself.
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Superuser).await;

    let grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_class c, aclexplode(c.relacl) a \
         WHERE a.grantee = 'recipe_replicator'::regrole",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();

    assert_eq!(grants, 0);
}

#[tokio::test]
async fn the_replicator_password_is_accepted_before_the_privilege_check() {
    // A wrong password would give 28P01 before the privilege check gives 42501.
    let code = connect_error(server(), Role::Replicator, "recipe").await;

    assert_ne!(code, INVALID_PASSWORD);
}

#[tokio::test]
async fn the_membership_of_the_migrator_in_audit_owner_does_not_inherit() {
    let mut conn = connect(server(), Role::Superuser, "postgres").await;

    let inherit: bool = sqlx::query_scalar(
        "SELECT inherit_option FROM pg_auth_members \
         WHERE roleid = 'audit_owner'::regrole AND member = 'recipe_migrator'::regrole",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();

    assert!(!inherit);
}

#[tokio::test]
async fn only_audit_owner_holds_pg_read_all_stats() {
    // ADR-0069 (f): the function replication_state() of audit_owner reads the statistics.
    let mut conn = connect(server(), Role::Superuser, "postgres").await;

    // The predefined role pg_monitor is a member of pg_read_all_stats in every cluster.
    let members: Vec<(String, bool)> = sqlx::query_as(
        "SELECT pg_get_userbyid(member)::text, inherit_option FROM pg_auth_members \
         WHERE roleid = 'pg_read_all_stats'::regrole \
           AND member <> 'pg_monitor'::regrole ORDER BY 1",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();
    assert_eq!(members, vec![("audit_owner".to_owned(), true)]);

    for role in Role::WITH_SECRET_FILE {
        let held: bool = sqlx::query_scalar("SELECT pg_has_role($1, 'pg_read_all_stats', 'USAGE')")
            .bind(role.name())
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert!(!held, "{} holds pg_read_all_stats", role.name());
    }
}

#[tokio::test]
async fn recipe_app_reads_no_statement_text_of_another_session() {
    let database = TestDatabase::create().await;
    let mut other = database.connect(Role::Migrator).await;
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut other)
        .await
        .unwrap();
    sqlx::query(&format!("SELECT '{STATEMENT_MARKER}'"))
        .execute(&mut other)
        .await
        .unwrap();
    let mut app = database.connect(Role::App).await;

    let text: Option<String> =
        sqlx::query_scalar("SELECT query FROM pg_stat_activity WHERE pid = $1")
            .bind(pid)
            .fetch_one(&mut app)
            .await
            .unwrap();

    let text = text.unwrap_or_default();
    assert!(!text.contains(STATEMENT_MARKER), "{text}");
    let mut admin = database.connect(Role::Superuser).await;
    let seen: String = sqlx::query_scalar("SELECT query FROM pg_stat_activity WHERE pid = $1")
        .bind(pid)
        .fetch_one(&mut admin)
        .await
        .unwrap();
    assert!(seen.contains(STATEMENT_MARKER), "{seen}");
}

#[tokio::test]
async fn pg_hba_conf_holds_the_harness_lines_and_then_the_lines_of_the_init_script() {
    // data-model.md 5.1: the harness lines of 02-test-hosts.sh come first (ADR-0069 (u)).
    let mut conn = connect(server(), Role::Superuser, "postgres").await;

    let rules: Vec<String> = sqlx::query_scalar(
        "SELECT concat_ws(' ', type, array_to_string(database, ','), \
             array_to_string(user_name, ','), \
             CASE WHEN netmask IS NULL THEN address ELSE address || '/' || netmask END, \
             auth_method) \
         FROM pg_hba_file_rules ORDER BY rule_number",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap();
    let faults: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pg_hba_file_rules WHERE error IS NOT NULL")
            .fetch_one(&mut conn)
            .await
            .unwrap();

    let expected: Vec<&str> = [HARNESS_HBA_LINES, PRODUCTION_HBA_LINES].concat();
    assert_eq!(faults, 0);
    assert_eq!(rules, expected);
}

/// Run `psql -U postgres` through the socket of the harness container, as the given system user.
fn socket_psql(system_user: Option<&str>) -> Output {
    let mut command = Command::new("docker");
    command.arg("exec");
    if let Some(user) = system_user {
        command.args(["-u", user]);
    }
    command
        .arg(&server().container_id)
        .args(["psql", "-U", "postgres", "-d", "postgres", "--no-psqlrc"])
        .args(["-tAc", "SELECT 1"])
        .output()
        .expect("docker exec runs")
}

#[test]
fn the_superuser_logs_in_through_the_socket_as_the_system_user_postgres() {
    // Spec scenario "Superuser through the socket": the line `local all postgres peer`.
    let output = socket_psql(Some("postgres"));

    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "1");
}

#[test]
fn the_superuser_cannot_log_in_through_the_socket_as_another_system_user() {
    // Spec scenario "Superuser through the socket from another system user": `docker exec`
    // without `-u` runs as root, and `peer` compares the system user with the role name.
    let output = socket_psql(None);

    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Peer authentication failed"), "{stderr}");
}
