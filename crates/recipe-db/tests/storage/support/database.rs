//! One test database for each test (design D12).
//!
//! `CREATE DATABASE ... TEMPLATE recipe` copies the migrated schema in a fraction of a second.
//! The copy has no database privilege, so the harness repeats the `REVOKE` and `GRANT`
//! statements of `01-roles.sh` for the new database.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use recipe_db::settings::Connection;
use sqlx::{ConnectOptions as _, PgConnection};

use super::container::{Role, TEMPLATE_DATABASE, server};
use super::session::{connect, options};

/// The key of the advisory lock that serializes `CREATE DATABASE` from one template.
const CREATE_LOCK_KEY: i64 = 7_240_001;

thread_local! {
    /// The databases that a fixture hands to the test thread.
    static KEPT: RefCell<Vec<TestDatabase>> = const { RefCell::new(Vec::new()) };
}

static NEXT_NUMBER: AtomicU32 = AtomicU32::new(0);

/// SQLSTATE "object in use". A session that the server rejects can still count as a user of
/// the template for a moment.
const OBJECT_IN_USE: &str = "55006";
const CREATE_ATTEMPTS: u32 = 50;
const CREATE_PAUSE: Duration = Duration::from_millis(100);

/// Run `CREATE DATABASE` again while a short-lived session blocks the template.
async fn create_with_retry(admin: &mut PgConnection, statement: &str) {
    for attempt in 1..=CREATE_ATTEMPTS {
        let result = sqlx::query(statement).execute(&mut *admin).await;
        let in_use = matches!(
            &result,
            Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some(OBJECT_IN_USE)
        );
        if !in_use || attempt == CREATE_ATTEMPTS {
            result.expect("the database is created");
            return;
        }
        tokio::time::sleep(CREATE_PAUSE).await;
    }
}

/// A database that only one test uses.
#[derive(Debug)]
pub struct TestDatabase {
    name: String,
}

impl TestDatabase {
    /// Create a database from the template, owned by `recipe_migrator`.
    pub async fn create() -> Self {
        let number = NEXT_NUMBER.fetch_add(1, Ordering::Relaxed);
        let name = format!("test_{number}");
        let mut admin = options(server(), Role::Superuser, "postgres")
            .connect()
            .await
            .expect("the superuser connects");
        // Two statements from one template at the same time can fail on the checkpoint.
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(CREATE_LOCK_KEY)
            .execute(&mut admin)
            .await
            .expect("the lock is free");
        // The name comes from a counter, not from input.
        let statements = [
            format!(
                "CREATE DATABASE \"{name}\" TEMPLATE {TEMPLATE_DATABASE} OWNER recipe_migrator"
            ),
            format!("REVOKE TEMPORARY ON DATABASE \"{name}\" FROM PUBLIC"),
            format!("REVOKE CONNECT ON DATABASE \"{name}\" FROM PUBLIC"),
            format!(
                "GRANT CONNECT ON DATABASE \"{name}\" \
                 TO recipe_migrator, recipe_app, recipe_backup"
            ),
        ];
        create_with_retry(&mut admin, &statements[0]).await;
        for statement in statements.iter().skip(1) {
            sqlx::query(statement)
                .execute(&mut admin)
                .await
                .expect("the statement runs");
        }
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(CREATE_LOCK_KEY)
            .execute(&mut admin)
            .await
            .expect("the lock is released");
        Self { name }
    }

    /// Open a session of a role and keep the database until the test thread ends.
    ///
    /// A helper that returns only a connection uses this call. The connection needs the database.
    pub async fn into_session(self, role: Role) -> PgConnection {
        let session = self.connect(role).await;
        self.keep_until_thread_ends();
        session
    }

    /// Hand the database to the test thread. The thread drops it when the test ends.
    ///
    /// A fixture uses this call when it returns sessions and not the handle.
    pub fn keep_until_thread_ends(self) {
        // The thread-local is gone only while the thread ends. Then the closure drops the database.
        let _ = KEPT.try_with(|kept| kept.borrow_mut().push(self));
    }

    /// The name of the database.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Open a normal session of a role in this database.
    pub async fn connect(&self, role: Role) -> PgConnection {
        connect(server(), role, &self.name).await
    }

    /// The connection data of a role for this database.
    pub fn target(&self, role: Role) -> Connection {
        server().target(role, &self.name)
    }
}

impl Drop for TestDatabase {
    /// Drop the database, so that one run does not fill the disk of the container.
    fn drop(&mut self) {
        // A `Drop` cannot await, and `block_on` panics inside the runtime of the test.
        // A new thread has its own runtime, so the drop stays synchronous.
        let statement = format!("DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)", self.name);
        let worker = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime.block_on(async {
                let mut admin = options(server(), Role::Superuser, "postgres")
                    .connect()
                    .await
                    .map_err(|error| error.to_string())?;
                sqlx::query(&statement)
                    .execute(&mut admin)
                    .await
                    .map_err(|error| error.to_string())
            })
        });
        // A failed drop must not panic here, because a panic while unwinding aborts the process.
        // The lint `print_stderr` denies output in test helpers, so the error is ignored.
        // The next run removes the container with all its databases.
        let _ = worker.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dropping_a_test_database_removes_it_from_the_server() {
        let database = TestDatabase::create().await;
        let name = database.name().to_owned();
        // An open session must not block the drop.
        let _session = database.connect(Role::App).await;

        drop(database);

        let mut admin = connect(server(), Role::Superuser, "postgres").await;
        let found: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_database WHERE datname = $1")
            .bind(&name)
            .fetch_one(&mut admin)
            .await
            .expect("the count is read");
        assert_eq!(found, 0, "the database {name} still exists");
    }
}
