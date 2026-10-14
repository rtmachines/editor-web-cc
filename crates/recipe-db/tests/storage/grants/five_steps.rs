//! Task 5.3: a migration with the five steps of `data-model.md` section 5.1.
//!
//! The migration adds an index to an audit table as `audit_owner`. Afterwards the state checks of
//! section 5.3 pass again, and `audit_owner` holds no `CREATE` on `public`.

use sqlx::Connection as _;

use super::audit_guard::audit_violations;
use super::checks::assert_least_privilege_state;
use crate::support::{Role, TestDatabase};

const INDEX: &str = "audit_log_session_ref_idx";
/// The five steps in one file. The index is step 3.
const FIVE_STEPS: &str = "\
GRANT CREATE ON SCHEMA public TO audit_owner;
SET ROLE audit_owner;
CREATE INDEX IF NOT EXISTS audit_log_session_ref_idx ON audit_log (session_ref);
RESET ROLE;
REVOKE CREATE ON SCHEMA public FROM audit_owner;
";

#[test]
fn the_guard_accepts_the_five_steps() {
    assert_eq!(audit_violations(FIVE_STEPS), Vec::<String>::new());
}

#[tokio::test]
async fn the_state_checks_pass_again_after_the_five_steps() {
    let database = TestDatabase::create().await;
    let mut migrator = database.connect(Role::Migrator).await;

    // The simple protocol runs the five statements as one implicit transaction.
    sqlx::raw_sql(FIVE_STEPS)
        .execute(&mut migrator)
        .await
        .unwrap();
    migrator.ping().await.unwrap();

    assert_least_privilege_state(&database).await;
    let mut admin = database.connect(Role::Superuser).await;
    let owner: String = sqlx::query_scalar(
        "SELECT pg_get_userbyid(relowner)::text FROM pg_class \
         WHERE relname = $1 AND relkind = 'i' AND relnamespace = 'public'::regnamespace",
    )
    .bind(INDEX)
    .fetch_one(&mut admin)
    .await
    .unwrap();
    assert_eq!(owner, "audit_owner");
    let owner_session: String = sqlx::query_scalar("SELECT current_user::text")
        .fetch_one(&mut migrator)
        .await
        .unwrap();
    assert_eq!(owner_session, "recipe_migrator");
}
