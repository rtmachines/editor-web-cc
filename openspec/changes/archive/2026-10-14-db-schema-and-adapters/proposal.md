# Proposal: db-schema-and-adapters

## Why

The use cases of `recipe-app` run only on the in-memory adapters. With the PostgreSQL schema, the roles, and the `sqlx` adapters, a command commits its change and its audit entry in one database transaction.

The change satisfies R04, R11, R12, R14, R17, R18, R20, R21, R29, R30, R31, R34, R35, and R36 of `overview.md` section 7.

It uses these Assumptions of `overview.md` section 9: A1, A3, A7, A15, A26, A31, A32, A38.

## What Changes

- `deploy/db/initdb/01-roles.sh`: the five roles, the database `recipe`, and the replication line of `pg_hba.conf` (`data-model.md` section 5.1).
- `migrations/`: `0001_schema.sql`, `0002_triggers.sql`, `0003_grants.sql`, and `0004_seed_plc_types.sql` (`data-model.md` sections 3, 4, 5.2, 7, and 8).
- `recipe-db`: the `sqlx` adapters of the eight ports of `recipe-app`, the map from `sqlx::Error` to `StorageError`, the connection pool, and the migrator.
- `recipe-db` tests: constraints, triggers, grants, the atomicity of each command of `data-model.md` section 6, and the races of `reliability.md` section 6. One contract suite runs on both adapters.
- `recipe-server`: the subcommand `migrate` with a minimal configuration file.
- Docs: ADR-0030 for the guard of a version status write. ADR-0031 for the migrator configuration and the secret files. ADR-0032 closes the default databases to `PUBLIC`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: add the init script roles, the grants of `recipe_app`, the trigger codes, the idle transaction limit, and the migrator configuration.
- `audit`: add the owner of the audit objects and concurrent audit inserts.
- `plant-model`: add the stable identifiers of the seed types.
- `recipe-management`: add the guard of a version status write.

## Impact

- Code: `crates/recipe-db`, `crates/recipe-server`, `migrations/`, and `deploy/db/initdb/`.
- Dependencies: design D16. Tests use `testcontainers`.
- `crates/recipe-app/src/memory` can change when the contract suite finds a difference in a row image.
- Docs: three ADRs, `reliability.md` section 7.11, and `data-model.md` sections 5.1 and 6.

## Out of scope

- `recipe-server create-admin`, the ports and adapters of users, sessions, and login attempts, and the tests of the Identity and Access rows of `data-model.md` section 6 (P2.5).
- The retry of `SetUserRole` and `DeactivateUser` on `40001` (P2.5).
- `MaintenanceStore`, the job `audit_chain_check`, and the startup settings comparison (P4.1).
- The subcommand `serve`, the role check of `app` at startup, and the readiness check (later steps of the plan).
- `deploy/compose.yaml` (P4.1).
