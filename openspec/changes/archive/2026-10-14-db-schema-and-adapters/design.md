# Design: db-schema-and-adapters

## Context

See `proposal.md`, section Why. `recipe-db` is an empty crate. `migrations/` is empty, and `deploy/db/initdb/` does not exist. `recipe-app` holds the ports, the use cases, and the in-memory adapters of P2.3. The archived design of `app-use-cases-and-ports` (decisions D1 to D21) and ADR-0029 are binding.

The rules come from these sources. This document does not repeat them.

| Topic | Source |
|---|---|
| DDL, triggers, grants, seed, migration files | `data-model.md` sections 1, 3, 4, 5, 7, 8 |
| Statements and lock sequence of each command | `data-model.md` section 6, ADR-0015 |
| Advisory locks of derived types | `data-model.md` section 1.4 |
| Row images, state rule, aggregates | `data-model.md` section 10, ADR-0029 |
| Map from SQLSTATE to `StorageError` | `reliability.md` section 1.1 |
| Error codes and user messages | `reliability.md` section 1.2 |
| Migrations, migrator lock, retries | `reliability.md` section 5, ADR-0020 |
| Test levels | `reliability.md` section 6 |
| Timeouts, pool, settings | `reliability.md` sections 7.4, 7.5, 7.11 |
| Roles and secrets | `data-model.md` section 5.1, `reliability.md` section 7.1, ADR-0020 |
| Port contracts | `crates/recipe-app/src/ports`, archived design D5, D9, D11 |

## Goals / Non-Goals

**Goals:**

- Each port method runs the statements of its row in `data-model.md` section 6 and takes only the lock in its name.
- The migration files hold the SQL of `data-model.md` without a change. A difference is a defect of the file, not of the document, unless an ADR records it.
- One contract suite runs every port method on `MemoryStore` and on the `sqlx` adapter, with the same assertions.
- No `sqlx` type leaves `recipe-db`.

**Non-Goals:**

- No adapter for a port that does not exist in `recipe-app` today (decision D1).
- No `serve` wiring, no configuration key that `migrate` does not read.

## Decisions

### D1. Scope (answers of the user, 2026-10-13)

- The migrations hold the full DDL of `data-model.md` section 3, also `app_user`, `session`, `login_attempt`, and `security_event`. Their constraints, triggers, and grants get tests now.
- `create-admin`, `UserRepository`, `SessionRepository`, `LoginAttemptStore`, and the tests of the Identity and Access rows of section 6 go to P2.5.
- `SecurityEventSink` exists today, so this change implements it.
- Commands use only `Isolation::ReadCommitted` and `Isolation::ReadOnly`. `40001` maps to `Deadlock`. P2.5 adds the one retry.

### D2. Public surface of `recipe-db`

- `Database` wraps the pool. It gives `Arc<dyn UnitOfWork>` and `Arc<dyn SecurityEventSink>`.
- `Connection` holds host, port, database, user, and `Password`. `Password` has a hand-written `Debug` that prints `[redacted]`.
- `PoolSettings` and `MigrateSettings` hold the values of `reliability.md` section 7.11. Their `Default` equals that table, and a comment names each key.
- Every public function returns `StorageError`. The crate has no public error type (`reliability.md` section 1.1).
- `migrate_from_directory` serves only the storage tests. It sits behind the feature `test-support`, which only the dev-dependency of `recipe-db` enables (review fix 13.5).
- Queries use the runtime API `sqlx::query` with binds. The macros `query!` need a database at compile time, so the crate does not use them. The feature `macros` stays for `sqlx::migrate!`.

### D3. Transaction

- `PgTransaction` owns `sqlx::Transaction<'static, Postgres>`. One struct implements the five sub-ports, and each accessor returns `self`. Each port has its own module with its `impl`, so no file passes 800 lines.
- `begin` sends `BEGIN ISOLATION LEVEL READ COMMITTED`, `... READ ONLY`, or `... SERIALIZABLE`. Drop without `finish` rolls back. `finish` commits.
- `append_audit` runs one `INSERT INTO audit_log ... SELECT ... FROM unnest(...) WITH ORDINALITY ORDER BY ordinality`. The rows reach trigger 4.9 in the order of the list. The statement omits `chain_seq`, `prev_hash`, and `entry_hash`, so the trigger sets them.
- The audit insert is the last statement before the commit. After the head lock of 4.9 only the foreign-key check of the actor takes `FOR KEY SHARE` on `app_user`. The rule `FOR NO KEY UPDATE` of ADR-0015 makes that lock safe.

### D4. Map from `sqlx::Error` to `StorageError`

- One private function follows the order of `reliability.md` section 1.1.
- A trigger message starts with a code and a space. `ErrorCode::find` reads that code. Only the trigger ranges `E-PLT-030..049` and `E-RCP-030..049` count.
- `StorageError::Constraint` carries the user message of the code from `reliability.md` section 1.2, not the database text. A table in `recipe-db` holds the 16 trigger messages and the message of `E-DB-003`. A test compares the table with section 1.2.
- `Duplicate` and `MissingReference` carry `DatabaseError::constraint()`. The names equal the constants of `recipe_app::error`.
- `PoolTimedOut` gives `PoolTimeout`. `Io`, `Tls`, `PoolClosed`, and `WorkerCrashed` give `Unavailable`. The original error stays the `source` of `Other`.
- Unit tests build `sqlx::Error::Database` from a test type that implements `DatabaseError`. Integration tests produce each SQLSTATE in a real database.

### D5. Row images

- Every image is `to_jsonb(<alias>)` of the row (ADR-0029). A lock statement returns the image before the change. A write of one row without child rows returns the image after it with `RETURNING`.
- The image of a parameter adds `limits` (ordered by `leaf_path`) and `translations` (ordered by `lang`). The image of a struct adds `members` (ordered by `ordinal`).
- A write with child rows (`insert_parameter`, `insert_struct`, `insert_version`) reads its image after the writes in a separate statement. A CTE that writes rows does not see the rows of its sibling statements.
- `delete_parameter` and `delete_translation` build the image in a CTE of the `DELETE`. The CTE reads the rows before the delete, also the child rows of the cascade.
- `MemoryStore` and the `sqlx` adapter give the same keys and the same JSON type for each key. The contract suite checks both. A difference is fixed in `recipe-app/src/memory`, after a failing contract test.

### D6. Statements and locks

- Each method runs the statements of its row in `data-model.md` section 6. A statement that locks several rows orders them by `id`.
- A read of a version (`lock_draft`, `read_draft`, `version_parameters`) runs three set queries: parameters, limits, and translations of the version. Rust joins them. No query runs once for each parameter.
- An order of tags, leaf paths, or languages uses `COLLATE "C"`, so the byte order does not depend on the cluster collation.
- A method takes no lock that its name does not name. The use case owns the order of ADR-0015.
- `lock_derived_types` runs `SELECT pg_advisory_xact_lock(2, hashtext($1::text || ':' || $2))` for each key, in the given order.
- `ensure_derived` runs `INSERT ... ON CONFLICT DO NOTHING RETURNING`. When no row returns, it selects the existing row by the key of its unique index and gives `created: None`.
- `u32` and `u16` fields convert with `try_from` (`data-model.md` section 1.2). `NUMERIC` uses the `sqlx` feature `rust_decimal`. `INET` binds as text with a cast, so no network crate enters the graph.

### D7. Guard of a version status write (ADR-0030)

- `lock_version`, `lock_versions`, and `lock_draft` record the `row_version` that they read in the transaction.
- `discard_version`, `retire_version`, and `approve_version` run `UPDATE ... WHERE id = $v AND row_version = $locked`. A version without a recorded lock, or a statement with 0 rows, gives `E-DB-008` and writes nothing.
- The status condition stays out of the `WHERE` (decision of the user after the review, 2026-10-13). Trigger 4.7 then rejects an illegal transition with `E-RCP-033`, as the port contract says.
- The use case compares the expected row version before the write (ADR-0008, ADR-0028). ADR-0030 replaces the condition `row_version = $expected` and the result "0 rows" in three rows of `data-model.md` section 6.

### D8. Security events

- `PgSecurityEventSink::record` runs one short transaction on the pool.
- The kinds `login_failed`, `login_throttled`, and `csrf_rejected` use `INSERT ... ON CONFLICT (client_ip, kind, (date_trunc('minute', occurred_at AT TIME ZONE 'UTC'))) WHERE ... DO UPDATE SET count = security_event.count + 1` (`data-model.md` section 3.8). Other kinds insert one row.

### D9. Pool and connection settings

- `after_connect` sets `statement_timeout`, `lock_timeout`, and `idle_in_transaction_session_timeout` (`reliability.md` section 7.4). It also sets `TimeZone = 'UTC'`, so the timestamps of a row image never depend on the server configuration.
- The pool sets `max_connections`, `acquire_timeout`, `idle_timeout`, `max_lifetime`, and `test_before_acquire = true` (section 7.5).

### D10. Migrator (ADR-0031)

- Each attempt of `recipe_db::migrate` opens one dedicated `PgConnection` as `recipe_migrator`, without a pool. It sets `lock_timeout` at session level. Then it runs `sqlx::migrate!("../../migrations")`, which takes its own advisory lock.
- sqlx 0.8.6 does not release that advisory lock after a failed migration. A failed attempt therefore closes its connection, and the next attempt opens a new one.
- The `55P03` of a failed attempt sits inside `MigrateError`. On `55P03` the migrator waits `migrate.retry_pause` and runs again, at most `migrate.retries` times. It skips the migrations that it applied before.

### D11. Migration files

- `0001_schema.sql` holds sections 3.0 to 3.17 in this order, also the row of `audit_chain_head`. `0002_triggers.sql` holds sections 4.1 to 4.10 and 4.12. `0003_grants.sql` holds section 5.2 in its order. `0004_seed_plc_types.sql` holds section 7.
- The document test of `0002` skips the second SQL block of section 4.9. That block holds the statements of the nightly check, not a migration.
- Seed identifiers are `5eed0000-0000-4000-8000-0000000000NN`. `NN` is the row number of the matrix in hexadecimal. The 254-character rows of `STRING` and `WSTRING` take their matrix numbers. The 80-character rows take `1e` and `1f`.
- A static test reads every migration file. It rejects `DROP`, `ALTER`, or `TRUNCATE` of an audit object outside the five steps of `data-model.md` section 5.1. It also rejects `DO`, `DROP SCHEMA`, `DROP ... CASCADE`, and a `SET ROLE` to another role. It reads comments and literals.
- `recipe_migrator` owns the schema `public` through `pg_database_owner`. The event trigger `audit_objects_dropped` of ADR-0034 therefore stops a drop of an audit object in the database. The static test is the second line.

### D12. Test harness

- One test binary `tests/storage/main.rs` with one module per area. One container per binary saves the start of a cluster for each test.
- The container is `postgres:16.15-bookworm` with the `POSTGRES_INITDB_ARGS` of `reliability.md` section 7.1. The harness copies `deploy/db/initdb/01-roles.sh` and four secret files into the container and sets `COMPOSE_SUBNET`. CI therefore runs the real script (`data-model.md` section 5.3).
- The harness runs `recipe_db::migrate` once into `recipe` and then closes every connection to `recipe`. No test connects to `recipe`. Each test gets `CREATE DATABASE ... TEMPLATE recipe OWNER recipe_migrator` and the `REVOKE` and `GRANT` statements of `01-roles.sh` for that database.
- Fixtures connect as `recipe_migrator`. The adapter connects as `recipe_app`. A test trigger on an audit object comes from the superuser of the container, which only the harness holds. Without Docker, the harness fails with a clear message and skips nothing.
- P4.1 pins the same image tag in `deploy/compose.yaml`.

### D13. Contract suite

- `tests/storage/contract/` defines `Harness` with two implementations: `MemoryStore` and `Database`. A macro writes one test for each implementation.
- The suite seeds through the ports, so it needs no access to the internals of either adapter. The `sqlx` harness inserts one `app_user` row as `recipe_migrator`, because P2.5 owns the user port.
- The memory harness calls `MemoryStore::seed_primitive_types`. A case uses only the primitives of that seed: `BOOL`, `INT`, `DINT`, `REAL`, `LREAL`, `TIME`, and the two rows each of `STRING` and `WSTRING`.
- A case compares the content of `limits`, `translations`, and `members`, not only their keys, so an empty child array fails.
- `recipe-db` uses `recipe-app` with the feature `memory` as a dev-dependency. `recipe-server` never enables the feature.

### D14. Use case tests on the database

- `AppContext::new` takes `Database::unit_of_work`, `Database::security_events`, and `MemoryClock`.
- A midway failure comes from a test trigger in the test database. The harness creates it as the superuser, because `recipe_migrator` holds no privilege on the audit objects. The trigger raises on a chosen statement. The test then finds no row of the command and an unchanged chain head.
- A race test runs two use cases on two connections. The second one starts after `pg_locks` shows the first lock, so the order is deterministic.

### D15. Subcommand `migrate` (ADR-0031)

- `recipe-server` parses `migrate [--config <path>]` by hand. Three subcommands need no parser crate.
- `figment` reads the TOML file and the environment with the prefix `RECIPE_`. The keys are those of ADR-0031.
- A missing value or secret file gives one `ERROR` event, and `main` returns the error (`reliability.md` section 2.3).

### D16. Dependencies

- `recipe-db`: `recipe-app`, `recipe-domain`, `sqlx` (features `uuid`, `time`, `json`, `rust_decimal`), `tokio`, `async-trait`, `serde_json`, `uuid`, `time`, `rust_decimal`, `tracing`. Dev: `testcontainers`, `tokio` (`macros`), `recipe-app` (`memory`).
- `recipe-server`: `figment` (features `toml`, `env`), `tokio`, `tracing`, `tracing-subscriber`.

### D17. Init script (ADR-0031, ADR-0032)

- `01-roles.sh` runs with `set -euo pipefail` and calls `psql` with `ON_ERROR_STOP=1`.
- `psql` reads each password from its secret file with `\set` and a backtick command. A password never appears in the argument list of a process.
- The script checks that `COMPOSE_SUBNET` is an IPv4 CIDR before it writes the `pg_hba.conf` line.
- ADR-0032: the script revokes `CONNECT` and `TEMPORARY` of `PUBLIC` on `postgres` and `template1`. A normal session of `recipe_app` or `recipe_replicator` then reaches no database except the grants of `data-model.md` section 5.1.
- ADR-0031: the session sets `log_statement = 'none'` and `log_min_error_statement = 'panic'` before the role statements. A secret file with no line, a carriage return, or a second line stops the script.
- ADR-0034: the script connects to `recipe` and creates the schema `audit_guard` and the event trigger `audit_objects_dropped`.

### Signatures

```rust
pub struct Database { pool: PgPool }
impl Database {
    pub async fn connect(target: &Connection, pool: &PoolSettings) -> Result<Self, StorageError>;
    pub fn unit_of_work(&self) -> Arc<dyn UnitOfWork>;
    pub fn security_events(&self) -> Arc<dyn SecurityEventSink>;
}
pub async fn migrate(target: &Connection, settings: &MigrateSettings) -> Result<(), StorageError>;
fn map_error(error: sqlx::Error) -> StorageError;
```

```sql
-- D7: status write of DiscardDraft. $2 is the row_version that lock_version read.
UPDATE recipe_version SET deleted_at = now() WHERE id = $1 AND row_version = $2
RETURNING to_jsonb(recipe_version.*);
```

## Risks / Trade-offs

- [`to_jsonb` and the in-memory images differ in a JSON type] → The contract suite compares the keys and the JSON types. The fix goes into `memory`.
- [One container for many parallel tests] → Each test has its own database and small pools. The harness sets `max_connections` of the container to 200.
- [The guard of D7 can never fail under `FOR UPDATE`] → It costs one bind. It turns a future defect of a lock path into `E-DB-008` instead of a silent write.
- [Docker on the Windows host] → `testcontainers` uses the Docker Engine of the host. CI on Linux runs the same tests.
- [`CREATE DATABASE ... TEMPLATE` copies no database privilege] → The harness repeats the `REVOKE` and `GRANT` of `01-roles.sh` for each test database.
- [`recipe_migrator` owns `public` and can drop an audit object without `SET ROLE`] → `data-model.md` section 5.1 already accepts the migrator credential as a residual risk. The static test of D11 rejects such a statement in a migration file, and the nightly check detects a broken chain.

## Migration Plan

The first deployment applies `0001` to `0004` to an empty database. Nothing ships yet, so a defect in a migration file is fixed in the file before the first release. After the first release, `data-model.md` section 8 applies.

## Open Questions

None.
