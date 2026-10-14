//! Test triggers on any table, created by the superuser of the harness (design D14).
//!
//! `recipe_migrator` holds no privilege on the audit objects, and a test trigger must not depend
//! on the privileges of the adapter. The superuser creates each probe. The table, the timing,
//! the events, and the body come from the test, not from input.

use sqlx::PgConnection;

use super::{Role, TestDatabase};

/// Create a row trigger on `table` as the superuser.
///
/// `timing` is `BEFORE` or `AFTER`. `events` is a list such as `INSERT OR UPDATE`. `body` is the
/// PL/pgSQL text between `BEGIN` and the final `RETURN NEW`. A body that raises an exception
/// without a code of `reliability.md` section 1.2 fails the statement with SQLSTATE `P0001`.
pub async fn install(database: &TestDatabase, table: &str, timing: &str, events: &str, body: &str) {
    let mut admin = database.connect(Role::Superuser).await;
    install_on(&mut admin, table, timing, events, body).await;
}

/// Create a row trigger on `table` through an open session of the superuser.
pub async fn install_on(
    admin: &mut PgConnection,
    table: &str,
    timing: &str,
    events: &str,
    body: &str,
) {
    let statements = format!(
        "CREATE FUNCTION test_probe_{table}() RETURNS trigger LANGUAGE plpgsql AS $probe$ \
         BEGIN {body} RETURN NEW; END $probe$; \
         CREATE TRIGGER test_probe_{table} {timing} {events} ON {table} \
         FOR EACH ROW EXECUTE FUNCTION test_probe_{table}();"
    );
    sqlx::raw_sql(&statements).execute(admin).await.unwrap();
}
