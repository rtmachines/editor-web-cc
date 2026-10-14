//! The fixture of the use case tests on the database (design D14).
//!
//! `App` holds a test database, an `AppContext` over the `sqlx` adapters, an administrator who
//! exists in `app_user`, and the data of one request. It also holds a session of the superuser,
//! which creates test triggers and reads the audit tables. A seeded plant comes from the
//! fixtures of `recipe_migrator`, so a test fails in the command and not in the seed.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use recipe_app::auth::{CurrentUser, RequestMeta};
use recipe_app::blocking::AppSettings;
use recipe_app::context::AppContext;
use recipe_app::memory::MemoryClock;
use recipe_db::Database;
use recipe_db::settings::PoolSettings;
use recipe_domain::audit::{RequestId, SessionRef};
use recipe_domain::identity::{Login, Role as UserRole, UserId};
use recipe_domain::plant::MachineCode;
use sqlx::PgConnection;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{Role, TestDatabase, fixtures, probe};

/// The login of the user of the fixture.
const LOGIN: &str = "admin";
/// The session reference of the request.
const SESSION_REF: &str = "0123456789abcdef";
/// The client address of the request.
const CLIENT_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7));

/// A machine that the fixture inserted.
#[derive(Debug, Clone, Copy)]
pub struct Seeded {
    /// The live code of the machine.
    pub code: MachineCode,
    /// The identifier of the machine.
    pub id: Uuid,
}

/// The head of the audit chain and the number of audit rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainHead {
    /// `last_seq` of `audit_chain_head`.
    pub last_seq: i64,
    /// `last_hash` of `audit_chain_head`.
    pub last_hash: Option<Vec<u8>>,
    /// The number of rows in `audit_log`.
    pub rows: i64,
}

/// A test database with a context of the use cases.
pub struct App {
    /// The administrator of the requests.
    pub user: CurrentUser,
    /// The data of the request.
    pub meta: RequestMeta,
    context: AppContext,
    migrator: PgConnection,
    admin: PgConnection,
    /// The handle of the test database. The sessions above belong to it.
    _database: TestDatabase,
}

impl App {
    /// Create the database, the user, the pool as `recipe_app`, and the context.
    pub async fn create() -> Self {
        Self::create_with(&PoolSettings::default()).await
    }

    /// Like `create`, with the settings of the pool. A race test uses a long `lock_timeout`.
    pub async fn create_with(pool: &PoolSettings) -> Self {
        let database = TestDatabase::create().await;
        let mut migrator = database.connect(Role::Migrator).await;
        let user_id = fixtures::user(&mut migrator).await;
        let connected = Database::connect(&database.target(Role::App), pool)
            .await
            .unwrap();
        let context = AppContext::new(
            connected.unit_of_work(),
            connected.security_events(),
            Arc::new(MemoryClock::new(OffsetDateTime::now_utc())),
            AppSettings::default(),
        );
        let admin = database.connect(Role::Superuser).await;
        Self {
            _database: database,
            user: CurrentUser::new(
                UserId::from(user_id),
                Login::parse(LOGIN).unwrap(),
                UserRole::Administrator,
            ),
            meta: RequestMeta::new(
                RequestId::from(Uuid::new_v4()),
                Some(CLIENT_IP),
                None,
                SessionRef::parse(SESSION_REF).ok(),
            ),
            context,
            migrator,
            admin,
        }
    }

    /// Open another plain session of a role in the test database.
    pub async fn session(&self, role: Role) -> PgConnection {
        self._database.connect(role).await
    }

    /// A clone of the context. The clone shares the adapters.
    pub fn context(&self) -> AppContext {
        self.context.clone()
    }

    /// The session of `recipe_migrator`, which inserts fixture rows.
    pub fn migrator(&mut self) -> &mut PgConnection {
        &mut self.migrator
    }

    /// The session of the superuser, which reads every table.
    pub fn admin(&mut self) -> &mut PgConnection {
        &mut self.admin
    }

    /// Insert a live machine on `platform` with the units of `paths`, parents first.
    pub async fn seed_machine(&mut self, platform: &str, paths: &[&str]) -> Seeded {
        let id = fixtures::machine(&mut self.migrator, platform).await;
        for path in paths {
            fixtures::unit(&mut self.migrator, id, path).await;
        }
        let code: String = sqlx::query_scalar("SELECT code FROM machine WHERE id = $1")
            .bind(id)
            .fetch_one(&mut self.migrator)
            .await
            .unwrap();
        Seeded {
            code: MachineCode::parse(&code).unwrap(),
            id,
        }
    }

    /// Create a row trigger as the superuser of the harness.
    pub async fn install_probe(&mut self, table: &str, timing: &str, events: &str, body: &str) {
        probe::install_on(&mut self.admin, table, timing, events, body).await;
    }

    /// Read the head of the chain and the number of audit rows.
    pub async fn head(&mut self) -> ChainHead {
        let (last_seq, last_hash): (i64, Option<Vec<u8>>) =
            sqlx::query_as("SELECT last_seq, last_hash FROM audit_chain_head WHERE singleton")
                .fetch_one(&mut self.admin)
                .await
                .unwrap();
        let rows = sqlx::query_scalar("SELECT count(*) FROM audit_log")
            .fetch_one(&mut self.admin)
            .await
            .unwrap();
        ChainHead {
            last_seq,
            last_hash,
            rows,
        }
    }

    /// Count the rows of a query with one identifier bind.
    pub async fn count(&mut self, query: &str, id: Uuid) -> i64 {
        sqlx::query_scalar(query)
            .bind(id)
            .fetch_one(&mut self.admin)
            .await
            .unwrap()
    }

    /// Count the machines with a code, live or archived.
    pub async fn count_machines_with_code(&mut self, code: &str) -> i64 {
        sqlx::query_scalar("SELECT count(*) FROM machine WHERE code = $1")
            .bind(code)
            .fetch_one(&mut self.admin)
            .await
            .unwrap()
    }

    /// The name of a machine.
    pub async fn machine_name(&mut self, id: Uuid) -> String {
        sqlx::query_scalar("SELECT name FROM machine WHERE id = $1")
            .bind(id)
            .fetch_one(&mut self.admin)
            .await
            .unwrap()
    }

    /// The name of the unit with a path.
    pub async fn unit_name(&mut self, machine: Uuid, path: &str) -> String {
        sqlx::query_scalar("SELECT name FROM unit WHERE machine_id = $1 AND path = $2")
            .bind(machine)
            .bind(path)
            .fetch_one(&mut self.admin)
            .await
            .unwrap()
    }

    /// Assert that the chain head moved by one step for each action and that the new audit rows
    /// have these actions in order.
    pub async fn assert_chain_moved(&mut self, before: &ChainHead, actions: &[&str]) {
        let expected = i64::try_from(actions.len()).unwrap();
        let after = self.head().await;
        assert_eq!(after.last_seq.checked_sub(before.last_seq), Some(expected));
        assert_eq!(after.rows.checked_sub(before.rows), Some(expected));
        let stored: Vec<String> = sqlx::query_scalar(
            "SELECT action FROM audit_log WHERE chain_seq > $1 ORDER BY chain_seq",
        )
        .bind(before.last_seq)
        .fetch_all(&mut self.admin)
        .await
        .unwrap();
        assert_eq!(stored, actions);
        assert_ne!(after.last_hash, before.last_hash);
    }

    /// Assert that the head and the number of audit rows did not change.
    pub async fn assert_chain_unchanged(&mut self, before: &ChainHead) {
        assert_eq!(&self.head().await, before);
    }
}
