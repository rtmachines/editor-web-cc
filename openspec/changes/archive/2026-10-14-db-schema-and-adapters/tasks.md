# Tasks: db-schema-and-adapters

Each code task starts with a failing test. Run it, see it fail, and keep the output. Then write the code. A task that only adds tests for finished code reports "first run passed". After each code task, run the clippy, test, and fmt commands of `openspec/config.yaml`, section `operations.apply.guidance`. A `[mechanical]` task goes to a Haiku subagent. Sonnet does every other code task. `target/p24-brief.md` is the common brief of the subagents.

The ste-lint command is the last command of the section "Local checks" of `CONTRIBUTING.md`. The storage tests need Docker (design D12). `cargo test -p recipe-db --test storage <filter>` runs one area of them.

A trigger test asserts the SQLSTATE `23514` and the code at the start of the message. A constraint test asserts the SQLSTATE and the constraint name. A fixture uses the platform of its machine and obeys every trigger of `data-model.md` section 4.

## 1. Documents

- [x] 1.1 (docs/adr) Write ADR-0030 for design D7. Change the rows `DiscardDraft`, `RetireVersion`, and `ApproveVersion` of `data-model.md` section 6: the lock gives the row version of the guard, and 0 rows give `E-DB-008`. Add the index row and the decision map entry "Concurrency". Check with the ste-lint command.
- [x] 1.2 (docs/adr) Write ADR-0031 for design D10 and D15. Add the rows `db.host`, `db.port`, `db.name`, `db.migrator_user`, and three `secrets.*_password_file` rows to `reliability.md` section 7.11. Add the index row and the decision map entry. Check with the ste-lint command.
- [x] 1.3 (docs/adr) Write ADR-0032 for design D17: `01-roles.sh` revokes `CONNECT` and `TEMPORARY` of `PUBLIC` on `postgres` and `template1`. Add the two statements to the SQL block of `data-model.md` section 5.1. Add the index row and the decision map entry. Check with the ste-lint command.
- [x] 1.4 (CHANGELOG.md) Add the three ADRs, the migrations, the roles script, the adapters, and the subcommand `migrate` under `[Unreleased]`. Check with the ste-lint command.

## 2. Crate, roles, and migrator

- [x] 2.1 (recipe-db) Add the dependencies of D16, `Connection`, `Password`, `PoolSettings`, and `MigrateSettings` (D2). First write failing tests. `format!("{:?}", password)` shows `[redacted]` and never the secret. Each `Default` value equals its key in `reliability.md` section 7.11. Check with `cargo test -p recipe-db settings`.
- [x] 2.2 (recipe-db) Write the container part of the test harness of D12 in `tests/storage/`, without the migration. First write the failing role test from the delta requirement "Roles from the database init script". It fails because `01-roles.sh` does not exist. Check with `cargo test -p recipe-db --test storage roles`.
- [x] 2.3 (deploy) Write `deploy/db/initdb/01-roles.sh` from `data-model.md` section 5.1, ADR-0031, ADR-0032, and design D17. Check with `bash -n` on the script and with `cargo test -p recipe-db --test storage roles`.
- [x] 2.4 (recipe-db) Add `recipe_db::migrate` (D10) and the template part of the harness (D12). First write failing tests from the baseline requirement "Migrator lock and retries". A held table lock gives `55P03`, and a second attempt on a new connection succeeds after the pause. A lock that stays held gives `E-DB-001` after the last retry. A test database has the owner `recipe_migrator`. Check with `cargo test -p recipe-db --test storage migrate`.

## 3. Schema

- [x] 3.1 (recipe-db) Write `0001_schema.sql` (D11). First write a failing document test. It reads the SQL blocks of `data-model.md` sections 3.0 to 3.17 and compares them with the file. The compare drops comments and makes each run of white space one space. A second test lists every table, constraint, and index name of section 3 in `pg_catalog`. Check with `cargo test -p recipe-db --test storage schema`.

## 4. Triggers

- [x] 4.1 (recipe-db) Add sections 4.1, 4.2, and 4.3 to `0002_triggers.sql` and to its document test. First write failing tests. Each of `E-RCP-030`, `E-RCP-031`, `E-RCP-032`, `E-PLT-031`, and `E-PLT-032` has a positive and a negative test. A delete of a parameter cascades to its limit and translation rows in a live draft. Check with `cargo test -p recipe-db --test storage triggers::version_mutable`.
- [x] 4.2 (recipe-db) Add sections 4.4 and 4.7. First write failing tests. An update adds 1 to `row_version`, and `a_bump` fires before `b_transition`. As `recipe_migrator`, each forbidden transition and frozen column of `reliability.md` section 6 gives `E-RCP-033`. Task 5.2 repeats the transitions as `recipe_app`, because the grants come with `0003`. The allowed transitions pass. Each of the five `set_updated_at` triggers of section 3.0 sets `updated_at`. Check with `cargo test -p recipe-db --test storage triggers::transition`.
- [x] 4.3 (recipe-db) Add sections 4.5 and 4.8. First write failing tests as `recipe_migrator`. Each of `E-PLT-030`, `E-PLT-037`, `E-PLT-038`, and `E-RCP-034` has a positive and a negative test. A change without a reference passes. Check with `cargo test -p recipe-db --test storage triggers::frozen`.
- [x] 4.4 (recipe-db) Add section 4.6. First write failing tests. An update and a delete of `plc_type` and `plc_type_member` give `E-PLT-033`. A late member gives `E-PLT-034`. A member of a primitive gives `E-PLT-035`. A struct without a member gives `E-PLT-036` at commit. Check with `cargo test -p recipe-db --test storage triggers::immutable_types`.
- [x] 4.5 (recipe-db) Add the first SQL block of section 4.9. The document test skips the second block (D11). First write failing tests from the baseline requirement "Hash chain of audit entries". Each insert gets the next `chain_seq` and the `prev_hash` of the head. The batch statement of section 4.9 finds no break. A changed `after_state` gives a break at its `chain_seq`. A rollback leaves no gap. Check with `cargo test -p recipe-db --test storage triggers::hash_chain`.
- [x] 4.6 (recipe-db) Add sections 4.10 and 4.12. First write failing tests. Parameter 4 001 of a version gives `E-RCP-035`. Limit row 32 769 gives `E-RCP-036`. `security_event_purge()` deletes only rows older than 400 days, at most 5 000 per call, and returns the count. Check with `cargo test -p recipe-db --test storage triggers::budget`.

## 5. Grants

- [x] 5.1 (recipe-db) [mechanical] Write `crates/recipe-db/tests/grants.expected` from `data-model.md` section 5.2. A table grant is the line `table|PRIVILEGE`. A column grant is the line `table.column|PRIVILEGE`. The file also holds `SELECT` on `_sqlx_migrations` and `audit_chain_head`. First write a failing test of the file: sorted lines, no duplicate, and every table name in section 3. Check with `cargo test -p recipe-db --test storage grants::file`.
- [x] 5.2 (recipe-db) Write `0003_grants.sql` from section 5.2 in its order, and the grant test of section 5.3. First write failing tests from the delta requirements "Least privilege of the application role" and "Owner of the audit objects". The table grants come from `role_table_grants` with `grantee = 'recipe_app'` and `table_schema = 'public'`. The column grants come from `aclexplode` on `pg_attribute.attacl`. Add the static test of D11 for the migration files. As `recipe_app`, each forbidden transition of `reliability.md` section 6 gives `E-RCP-033`, and each frozen column without a grant gives `42501`. Check with `cargo test -p recipe-db --test storage grants`.
- [x] 5.3 (recipe-db) Add the test migration with the five steps of `data-model.md` section 5.1. First write the failing test. After the five steps, the checks of task 5.2 pass again, and `audit_owner` has no `CREATE` on `public`. Check with `cargo test -p recipe-db --test storage grants::five_steps`.

## 6. Seed

- [x] 6.1 (recipe-db) [mechanical] Write `0004_seed_plc_types.sql` from `data-model.md` section 7 with the identifiers of D11. The 80-character rows of `STRING` and `WSTRING` take `1e` and `1f`. First write failing tests from the delta requirement "Stable identifiers of the seed types". Each row equals its row of the matrix: platforms, size, leaf class, and string length. A second run of the migrator changes no row. Check with `cargo test -p recipe-db --test storage seed`.

## 7. Constraint tests

These tasks only add tests. They run as the table owner `recipe_migrator`, or with `SET ROLE audit_owner` for an audit table.

- [x] 7.1 (recipe-db) [mechanical] Test each `CHECK`, `UNIQUE`, foreign key, and partial index of `machine` and `unit` (`data-model.md` sections 3.1 and 3.2). An archived machine frees its code. Check with `cargo test -p recipe-db --test storage constraints::plant`.
- [x] 7.2 (recipe-db) [mechanical] Test each constraint and unique index of `plc_type` and `plc_type_member` (sections 3.3 and 3.4), also `dims_valid` with `NULL` bounds. Check with `cargo test -p recipe-db --test storage constraints::types`.
- [x] 7.3 (recipe-db) [mechanical] Test each constraint of `app_user`, `session`, `login_attempt`, and `security_event` (sections 3.5 to 3.8), also the aggregate index per minute. Check with `cargo test -p recipe-db --test storage constraints::identity`.
- [x] 7.4 (recipe-db) [mechanical] Test each constraint and partial index of `recipe`, `recipe_version`, and `draft_editor` (sections 3.9 to 3.11). Check with `cargo test -p recipe-db --test storage constraints::recipe`.
- [x] 7.5 (recipe-db) [mechanical] Test each constraint of `parameter`, `parameter_limit`, and `parameter_translation` (sections 3.12 to 3.14). Check with `cargo test -p recipe-db --test storage constraints::parameter`.
- [x] 7.6 (recipe-db) [mechanical] Test each constraint of `audit_log`, `audit_chain_head`, and `app_setting` (sections 3.15 to 3.17). Check with `cargo test -p recipe-db --test storage constraints::audit`.

## 8. Errors and transactions

- [x] 8.1 (recipe-db) Add the private error map of D4. First write failing unit tests with a test type that implements `DatabaseError`. Each SQLSTATE and each error kind of `reliability.md` section 1.1 gives its variant. A trigger code outside the trigger ranges gives `E-DB-003`. Check with `cargo test -p recipe-db error`.
- [x] 8.2 (recipe-db) [mechanical] Add the table of the trigger messages of D4. First write a failing test that reads `reliability.md` section 1.2. Each message equals its row without the sentence "Reference: {request_id}.". Check with `cargo test -p recipe-db trigger_messages`.
- [x] 8.3 (recipe-db) Add `Database::connect` with the settings of D9. First write failing tests. A pooled connection shows the three timeouts of `reliability.md` section 7.4 and `TimeZone` `UTC`. An idle transaction that passes its limit ends, and its write is gone. A connect to a closed port gives `E-DB-005`. Check with `cargo test -p recipe-db --test storage pool`.
- [x] 8.4 (recipe-db) Test the error map on the database, after section 9. The tests reach each failure through the port adapters, because the map is private and no `sqlx` type leaves the crate (D2). First write failing tests from the delta requirement "Trigger code of a storage error" and the baseline requirements "Statement timeout" and "Lock and pool timeouts". A deadlock of two connections gives `E-DB-001`. Check with `cargo test -p recipe-db --test storage errors`.
- [x] 8.5 (recipe-db) Add `PgUnitOfWork` and `PgTransaction` with `begin`, `finish`, and `append_audit` (D3). First write failing tests. A commit keeps the rows and the entries in list order, also for 100 entries in one statement. A drop keeps nothing. A write in `Isolation::ReadOnly` fails. A failed audit insert gives `E-AUD-001` and keeps nothing. Check with `cargo test -p recipe-db --test storage unit_of_work`.
- [x] 8.6 (recipe-db) Add `PgSecurityEventSink` (D8). First write failing tests from the baseline requirements "Security event transaction" and "Aggregated security events". An event survives the rollback of the command. Three `login_failed` events of one address in one minute give one row with the count 3. Check with `cargo test -p recipe-db --test storage security_events`.

## 9. Port adapters and contract suite

Each task adds contract cases that run on `MemoryStore` and on `Database` (D13). A case checks the result, the row image keys and JSON types, the content of `limits`, `translations`, and `members`, and the state after the write. A case uses only the seed types of `MemoryStore` (D13). A difference of `MemoryStore` is fixed in `crates/recipe-app/src/memory` after the failing case. A version read runs three set queries (D6).

- [x] 9.1 (recipe-db) Write the contract harness of D13 and its first case for `insert_machine` and `lock_machine_shared`. First run the failing case: the `sqlx` adapter does not implement `PlantRepository`. Check with `cargo test -p recipe-db --test storage contract::machine`.
- [x] 9.2 (recipe-db) Implement the machine methods of `PlantRepository`: the three reads, the two machine locks, `insert_machine`, `rename_machine`, and `archive_machine`. First write the failing contract cases. A live duplicate code gives `machine_code_live_unique`. Check with `cargo test -p recipe-db --test storage contract::machine`.
- [x] 9.3 (recipe-db) Implement the unit methods of `PlantRepository`. First write the failing contract cases. A missing parent gives `unit_parent_exists`. `lock_unit_tree_for_update` orders by `id`. `archive_units` keeps the order and the count of its input. Check with `cargo test -p recipe-db --test storage contract::unit`.
- [x] 9.4 (recipe-db) Implement `UnitLookup` and `TypeCatalog`. First write the failing contract cases. `pg_locks` shows the class-2 keys of `lock_derived_types`. A second `STRING(20)` on one machine gives the first id and no image. A struct image holds its members. Check with `cargo test -p recipe-db --test storage contract::types`.
- [x] 9.5 (recipe-db) Implement the reads and locks of `RecipeRepository`. First write the failing contract cases. `version_parameters` orders by tag in byte order with `COLLATE "C"` and holds limits and translations. `lock_version_shared` gives `approved_by` and `approved_at`. Check with `cargo test -p recipe-db --test storage contract::recipe_read`.
- [x] 9.6 (recipe-db) Implement the writes of `RecipeRepository` with the guard of D7. First write the failing contract cases and the cases of the delta requirement "Guard of a version status write". An illegal transition gives `E-RCP-033`. A version copy keeps the keys. Check with `cargo test -p recipe-db --test storage contract::recipe_write`.
- [x] 9.7 (recipe-db) Implement `lock_draft`, `read_draft`, `touch_draft`, `insert_parameter`, and `update_definition` of `DraftRepository`. First write the failing contract cases. A touch adds 1 to `row_version` and one `draft_editor` row. `update_definition` keeps the value, the limits, and the translations. Check with `cargo test -p recipe-db --test storage contract::draft`.
- [x] 9.8 (recipe-db) Implement `set_value`, `replace_limits`, `upsert_translation`, `delete_translation`, and `delete_parameter`. First write the failing contract cases. `set_value` keeps the limit and translation rows. A deleted parameter with limits and translations gives an image with both arrays. Check with `cargo test -p recipe-db --test storage contract::draft`.

## 10. Use cases on the database

These tasks add tests of the use cases of `recipe-app` on `Database` (D14). The first task adds the shared fixture.

- [x] 10.1 (recipe-db) Add the fixture of D14 and the midway failure tests of the seven Plant Model commands and `EnsureDerivedType` (`data-model.md` section 6). The superuser of the harness creates each test trigger. First run the failing test of the fixture. Each failure leaves no row of the command and an unchanged chain head. Check with `cargo test -p recipe-db --test storage atomic::plant`.
- [x] 10.2 (recipe-db) Add the midway failure tests of `CreateRecipe`, `RenameRecipe`, `DeleteRecipe`, `RestoreRecipe`, `CreateVersion`, `DiscardDraft`, `ApproveVersion`, `RetireVersion`, and `ExportVersion`. Check with `cargo test -p recipe-db --test storage atomic::recipe`.
- [x] 10.3 (recipe-db) Add the midway failure tests of the eight draft commands. `RemoveParameter` on a parameter with limits and translations succeeds. Check with `cargo test -p recipe-db --test storage atomic::draft`.
- [x] 10.4 (recipe-db) Add the approval races of `reliability.md` section 6. Two approvals with one row version: one writes, and the other gets `AlreadyApproved` without an entry. An older row version gets `E-WEB-001`. A raw parameter insert as `recipe_app` during an approval waits and then gets `E-RCP-031`. `AddParameter` during an approval gets `E-RCP-002` from the domain. `DeleteRecipe` during `ApproveVersion` fails. Check with `cargo test -p recipe-db --test storage race::approve`.
- [x] 10.5 (recipe-db) Add the lock order races. `ArchiveUnit` against `AddParameter` with a new derived type gives no `40P01` and then `E-PLT-009`. `ArchiveMachine` against `CreateVersion` gives the same result. Two edits with one row version: one succeeds. Two concurrent `RenameMachine` give consecutive `chain_seq` values. Check with `cargo test -p recipe-db --test storage race::order`.

## 11. Subcommand `migrate`

- [x] 11.1 (recipe-server) Add the argument parser and the configuration of D15 with the keys of ADR-0031. First write failing tests. `migrate --config <path>` reads the TOML file. An environment value overrides a file value. A missing secret file gives an error before a connection. An unknown subcommand gives an error. Check with `cargo test -p recipe-server config`.
- [x] 11.2 (recipe-server) Connect `main` to `recipe_db::migrate`. First write failing tests from the delta requirement "Configuration of the migrator". The test runs the binary against a container of D12. It finds exit code 0 and four applied migrations. A missing secret file gives one `ERROR` line and exit code 1. Check with `cargo test -p recipe-server --test migrate`.

## 12. Verification

- [x] 12.1 (workspace) Run every command of the section "Local checks" of `CONTRIBUTING.md` and `openspec validate db-schema-and-adapters --strict`. `recipe-db` and `recipe-server` reach 80 % line coverage. Check with the output of each command.

## 13. Fixes of the code review

The four reviews of step 4 found no CRITICAL and no HIGH finding. The user accepted these fixes on 2026-10-14.

- [x] 13.1 (docs/adr) Write ADR-0033 (globals dump of the backup) and ADR-0034 (hardening of the audit objects). Change ADR-0030, ADR-0031, ADR-0032, `data-model.md` sections 4.9, 5.1, 5.3, `reliability.md` section 7.7, the status of ADR-0020, the ADR index, and `CHANGELOG.md`. Check with the ste-lint command.
- [x] 13.2 (recipe-db) Apply ADR-0034 and ADR-0031 to `0002_triggers.sql` and `01-roles.sh`: the qualified operator, the event trigger, the log settings, and the secret rules. Add `*.sql text eol=lf` to `.gitattributes`. First write failing tests. A drop of an audit object by `recipe_migrator` fails. The guard list equals the objects of `audit_owner`. A bad secret file stops the script. No migration file holds a carriage return. Check with `cargo test -p recipe-db --test storage`.
- [x] 13.3 (recipe-db) Make the static check of design D11 read comments and literals. It also rejects `DO`, `DROP SCHEMA`, `DROP ... CASCADE`, and a `SET ROLE` to another role than `audit_owner`. First write the failing cases of each bypass. Check with `cargo test -p recipe-db --test storage grants::audit_guard`.
- [x] 13.4 (recipe-db) Map the SQLSTATEs of a lost connection to `Unavailable`. Merge the three row errors into one enum. Match the aggregated event kinds by the enum. Key the guard map by `RecipeVersionId`. Delete the unused `tracing`. Read `unit_usage` from the live versions. Insert struct members with one statement. First write the failing tests. Check with `cargo test -p recipe-db`.
- [x] 13.5 (recipe-db) Move `migrate_from_directory` behind the feature `test-support`, which only the dev-dependency of `recipe-db` enables. Check with `cargo test -p recipe-db --test storage migrate` and `cargo deny --locked check`.
- [x] 13.6 (recipe-db) Harden the test harness. Use a random superuser password for each run, the port on 127.0.0.1 only, and random names of temporary directories. Add the missing foreign-key and primary-key tests, and derive the inline names in the coverage meta-test. Add the hash-chain test with forged chain values as `recipe_app`. Check with `cargo test -p recipe-db --test storage`.
- [x] 13.7 (recipe-db) Seed the contract suite through the ports (design D13). Keep only the insert of `app_user`. Check with `cargo test -p recipe-db --test storage contract`.
- [x] 13.8 (recipe-server) Use the secret rules of ADR-0031 and reject an unknown configuration key. First write the failing tests. Check with `cargo test -p recipe-server`.

## Workflow follow-up

- Deferred to P2.5: keep the SQLSTATE of `40001` apart from `40P01` for the retry (review C6). Protect `security_event.count` and `occurred_at` against a change by `recipe_app` (review S6). Test the aggregated kinds through `PgSecurityEventSink` (review R6, Q4).
- Deferred to P4.1: a connect timeout of the migrator with a new row of section 7.11 (review C7). Narrow `pg_read_all_stats` of `recipe_app` (review S7). Set the TLS mode and ignore the `PG*` environment values (review S8). Add an index on `security_event.occurred_at` (review Q9). The backup scripts of ADR-0033.
- Accepted: `main` returns `anyhow::Result<()>`, so the Rust runtime also prints the error to stderr (review R10, C4).
- At the commit, set the executable bit of `deploy/db/initdb/01-roles.sh` (review C11).

- Run the ste-lint command on each base spec merged with its delta before the archive.
- Archive with the `openspec` CLI. A hook runs `cargo test --workspace` first.
- P2.5: the domain has only `SecurityEvent::forbidden` now. When P2.5 adds the login events, it adds sink tests of `login_failed`, `login_throttled`, and `csrf_rejected` through `PgSecurityEventSink`. P4.1 adds the test of `audit_chain_broken`.
