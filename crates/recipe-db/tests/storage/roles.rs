//! Tests of the delta requirement "Roles from the database init script".

use sqlx::Row as _;

use crate::support::{Role, TestDatabase, connect, connect_error, server};

/// SQLSTATE of "insufficient privilege". The server checks it after a successful login.
const INSUFFICIENT_PRIVILEGE: &str = "42501";
/// SQLSTATE of "invalid password".
const INVALID_PASSWORD: &str = "28P01";
const DATABASES: [&str; 3] = ["recipe", "postgres", "template1"];

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
async fn recipe_app_reads_statistics_but_cannot_switch_to_the_role() {
    let mut conn = connect(server(), Role::Superuser, "postgres").await;

    let row = sqlx::query(
        "SELECT inherit_option, set_option FROM pg_auth_members \
         WHERE roleid = 'pg_read_all_stats'::regrole AND member = 'recipe_app'::regrole",
    )
    .fetch_one(&mut conn)
    .await
    .unwrap();

    assert!(row.get::<bool, _>("inherit_option"));
    assert!(!row.get::<bool, _>("set_option"));
}
