//! Statements of `recipe_app` that its grants reject or that a trigger rejects (`data-model.md`
//! section 5.2, `reliability.md` section 6).
//!
//! A fixture runs as the table owner `recipe_migrator`. The statement under test runs as
//! `recipe_app` in a second session of the same database.

use sqlx::PgConnection;
use sqlx::postgres::PgQueryResult;
use uuid::Uuid;

use crate::support::fixtures::{self, Draft};
use crate::support::{Role, TestDatabase, assert_privilege_denied, assert_trigger};

const TRANSITION: &str = "E-RCP-033";

/// Two sessions on one database: the fixture owner and the application role.
struct Sessions {
    database: TestDatabase,
    owner: PgConnection,
    app: PgConnection,
    draft: Draft,
}

async fn sessions() -> Sessions {
    let database = TestDatabase::create().await;
    let mut owner = database.connect(Role::Migrator).await;
    let draft = Draft::create(&mut owner).await;
    let app = database.connect(Role::App).await;
    Sessions {
        database,
        owner,
        app,
        draft,
    }
}

async fn approved() -> Sessions {
    let mut sessions = sessions().await;
    fixtures::approve(
        &mut sessions.owner,
        sessions.draft.version,
        sessions.draft.user,
    )
    .await;
    sessions
}

async fn retired() -> Sessions {
    let mut sessions = approved().await;
    fixtures::retire(&mut sessions.owner, sessions.draft.version).await;
    sessions
}

/// Run a statement of the test with the version as `$1`.
async fn run(
    app: &mut PgConnection,
    statement: &'static str,
    version: Uuid,
) -> Result<PgQueryResult, sqlx::Error> {
    sqlx::query(statement).bind(version).execute(app).await
}

async fn status_and_row_version(owner: &mut PgConnection, version: Uuid) -> (String, i64) {
    sqlx::query_as("SELECT status, row_version FROM recipe_version WHERE id = $1")
        .bind(version)
        .fetch_one(owner)
        .await
        .unwrap()
}

// ---- columns and tables without a grant (42501) ----

#[tokio::test]
async fn app_cannot_update_a_frozen_column_of_an_approved_version() {
    let mut s = approved().await;
    let other = fixtures::recipe(&mut s.owner, s.draft.machine, "TwinCat3", s.draft.user).await;
    let before = status_and_row_version(&mut s.owner, s.draft.version).await;

    let statements: [&'static str; 3] = [
        "UPDATE recipe_version SET number = 2 WHERE id = $1",
        "UPDATE recipe_version SET based_on_version_id = $1 WHERE id = $1",
        "UPDATE recipe_version SET row_version = 99 WHERE id = $1",
    ];
    for statement in statements {
        assert_privilege_denied(run(&mut s.app, statement, s.draft.version).await);
    }
    let recipe_change = sqlx::query("UPDATE recipe_version SET recipe_id = $2 WHERE id = $1")
        .bind(s.draft.version)
        .bind(other)
        .execute(&mut s.app)
        .await;
    assert_privilege_denied(recipe_change);

    assert_eq!(
        status_and_row_version(&mut s.owner, s.draft.version).await,
        before
    );
}

#[tokio::test]
async fn app_cannot_delete_a_machine() {
    let mut s = sessions().await;

    let result = sqlx::query("DELETE FROM machine WHERE id = $1")
        .bind(s.draft.machine)
        .execute(&mut s.app)
        .await;

    assert_privilege_denied(result);
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM machine WHERE id = $1")
        .bind(s.draft.machine)
        .fetch_one(&mut s.owner)
        .await
        .unwrap();
    assert_eq!(rows, 1);
}

#[tokio::test]
async fn app_cannot_update_the_audit_chain_head() {
    let mut s = sessions().await;

    let result = sqlx::query("UPDATE audit_chain_head SET last_seq = 5")
        .execute(&mut s.app)
        .await;

    assert_privilege_denied(result);
    // `recipe_migrator` holds no privilege on the table after 0003.
    let mut admin = s.database.connect(Role::Superuser).await;
    let last_seq: i64 = sqlx::query_scalar("SELECT last_seq FROM audit_chain_head")
        .fetch_one(&mut admin)
        .await
        .unwrap();
    assert_eq!(last_seq, 0);
}

#[tokio::test]
async fn app_cannot_update_or_delete_an_audit_log_row() {
    let mut s = sessions().await;

    let update = sqlx::query("UPDATE audit_log SET action = 'Renamed'")
        .execute(&mut s.app)
        .await;
    let delete = sqlx::query("DELETE FROM audit_log")
        .execute(&mut s.app)
        .await;

    assert_privilege_denied(update);
    assert_privilege_denied(delete);
}

// ---- forbidden transitions that the grant allows to write (E-RCP-033) ----

#[tokio::test]
async fn app_an_approved_version_cannot_become_a_draft() {
    let mut s = approved().await;

    let result = run(
        &mut s.app,
        "UPDATE recipe_version SET status = 'draft', approved_by = NULL, approved_at = NULL, \
             content_sha256 = NULL WHERE id = $1",
        s.draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
}

#[tokio::test]
async fn app_a_retired_version_cannot_become_approved() {
    let mut s = retired().await;

    let result = run(
        &mut s.app,
        "UPDATE recipe_version SET status = 'approved', retired_at = NULL WHERE id = $1",
        s.draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
}

#[tokio::test]
async fn app_a_retired_version_cannot_become_a_draft() {
    let mut s = retired().await;

    let result = run(
        &mut s.app,
        "UPDATE recipe_version SET status = 'draft', approved_by = NULL, approved_at = NULL, \
             retired_at = NULL, content_sha256 = NULL WHERE id = $1",
        s.draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
}

#[tokio::test]
async fn app_a_discarded_draft_cannot_be_restored() {
    let mut s = sessions().await;
    run(
        &mut s.app,
        "UPDATE recipe_version SET deleted_at = now() WHERE id = $1",
        s.draft.version,
    )
    .await
    .unwrap();

    let result = run(
        &mut s.app,
        "UPDATE recipe_version SET deleted_at = NULL WHERE id = $1",
        s.draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
}

#[tokio::test]
async fn app_the_content_hash_of_an_approved_version_is_frozen() {
    let mut s = approved().await;

    let result = run(
        &mut s.app,
        "UPDATE recipe_version SET content_sha256 = repeat('b', 64) WHERE id = $1",
        s.draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
}

#[tokio::test]
async fn app_an_approved_version_cannot_be_discarded() {
    let mut s = approved().await;

    let result = run(
        &mut s.app,
        "UPDATE recipe_version SET deleted_at = now() WHERE id = $1",
        s.draft.version,
    )
    .await;

    assert_trigger(result, TRANSITION);
}
