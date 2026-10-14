//! One test database for each test (design D12).
//!
//! `CREATE DATABASE ... TEMPLATE recipe` copies the migrated schema in a fraction of a second.
//! The copy has no database privilege, so the harness repeats the `REVOKE` and `GRANT`
//! statements of `01-roles.sh` for the new database.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use recipe_db::settings::Connection;
use sqlx::{ConnectOptions as _, PgConnection};

use super::container::{Role, TEMPLATE_DATABASE, server};
use super::session::{connect, options};

/// The key of the advisory lock that serializes `CREATE DATABASE` from one template.
const CREATE_LOCK_KEY: i64 = 7_240_001;

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
