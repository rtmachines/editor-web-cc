# SQL rules

Source: `docs/architecture/data-model.md` §1, §4 to §6, §8 and `reliability.md` §5, §6. Load the skill `postgresql-table-design` before you write DDL.

## Conventions (data-model §1)

- Identifiers are `snake_case`. No quoted identifiers.
- An entity table has `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`. A log table has `BIGINT GENERATED ALWAYS AS IDENTITY`.
- Time columns are `TIMESTAMPTZ`. All rows of one transaction share the value of `now()`.
- A string column is `TEXT` with a `CHECK` on its length or pattern. No `VARCHAR(n)`.
- An enumerated column is `TEXT` with `CHECK (... IN (...))`. No `CREATE TYPE ... AS ENUM`.
- Every foreign key column has an index, unless a primary key or a unique constraint starts with that column.
- Soft delete uses a nullable `deleted_at` or `archived_at` column (ADR-0021).
- Every function sets `search_path = pg_catalog, public, pg_temp`. A `SECURITY DEFINER` function also schema-qualifies every name.
- §1.1 lists the tables that are exceptions. §1.2 maps Rust names to SQL names. §1.3 gives the text limits.
- A trigger error message starts with its code from `reliability.md` §1.2. §4.11 maps each trigger code to its function.

## Migrations (reliability §5, data-model §8)

- A migration is `migrations/NNNN_description.sql`, applied by `sqlx::migrate!` as `recipe_migrator`.
- Migrations are forward-only. There is no down file. A rollback is a new forward migration or a restore.
- Never edit a migration that shipped. `sqlx` stops on a changed checksum.
- No migration rewrites a table or scans it under an exclusive lock. CI rejects such a migration.
- `CREATE INDEX CONCURRENTLY` goes in a file that starts with `-- no-transaction` and holds that one statement.
- A breaking change uses expand and contract across three releases.
- Each seed row keeps one UUID forever and uses `ON CONFLICT DO NOTHING`.

## Roles and grants (data-model §5)

- Roles come only from `deploy/db/initdb/01-roles.sh`. A migration never creates a role.
- `recipe_app` gets column-level `UPDATE` grants. Identifier columns, structure columns, and `row_version` get none.
- `recipe_app` deletes only rows that are not business history. Soft delete covers the rest.
- A row lock (`FOR UPDATE`, `FOR NO KEY UPDATE`, `FOR SHARE`, `FOR KEY SHARE`) needs an `UPDATE` grant on a column of the table.
- `recipe_app` cannot row-lock a table without that grant. `plc_type` therefore uses the class-2 advisory lock (ADR-0015, §1.4).
- `audit_log` and `security_event` are append-only for `recipe_app`. §5.2 names the two exceptions.
- Only a migration file runs `SET ROLE audit_owner`, and the same file runs `RESET ROLE`.
- A new grant goes into `crates/recipe-db/tests/grants.expected` in the same change.

## Locks and transactions (data-model §1.4, §6)

- One transaction per command, except `Login` (T1 and T2). Lock order: `machine` → `recipe` → `recipe_version` → `unit` → children.
- A command takes only the locks that it needs, always in this order.
- A draft command locks `recipe_version` `FOR UPDATE`, requires a live draft, and compares `row_version`.
- Then the draft command runs the touch statement and writes `draft_editor`. Separation of duties (I16) reads `draft_editor`.
- The audit insert runs in the transaction of the command. §10 gives the state rule and the `aggregate_type` of each entity.
- A statement that locks several rows of one table orders them by `id`. `app_user` rows get `FOR NO KEY UPDATE`.
- Advisory locks use `pg_advisory_xact_lock(class, key)`. Class 1 is `Login` T1. Class 2 is the class of derived types. No code takes class 3.
- A command sorts its class-2 keys in the order struct name, `STRING`, `WSTRING`, `ARRAY`. It takes them before its first `plc_type` insert.
- Only the `sqlx` migrator uses the one-key form `pg_advisory_lock(bigint)`.
- Isolation is `READ COMMITTED` with no retry. `SetUserRole` and `DeactivateUser` run `SERIALIZABLE` and retry once on `40001`.
- A query reads in `READ COMMITTED READ ONLY`. A draft query of ADR-0042 and the preview and diff queries of ADR-0050 read in `REPEATABLE READ READ ONLY`. A query never retries.

## Tests (reliability §6)

- Every trigger has a positive test and a negative test. The negative test asserts the SQLSTATE and the trigger code.
- Every `CHECK`, `UNIQUE`, foreign key, and partial index has a test that violates it.
- Every operation of `data-model.md` §6 has a test that fails midway and finds no partial write.
- The grant test compares `role_table_grants` and `role_column_grants` with `grants.expected`.
