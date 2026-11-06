# Proposal: migration-rewrite-forms

## Why

The CI check `deploy/ci/migration_rewrite.py` rejects four forms of `data-model.md` §8. Other forms that rewrite a table, scan it, or hold a strong lock on it still pass. ADR-0127 names this gap as a known risk. The closure review of 2026-11-05 keeps it as the residual risk of the finding D4-2.

A passing form can lock a large table, such as `audit_log`, during an upgrade. Then the `lock_timeout` retries of the migrator fail, and the upgrade stops.

The change satisfies R30 (stable 24/7 operation), R31 (data safety over performance), and R34 (forward-only migrations for long-term operation).

## What Changes

- The check also rejects these statements on a table or a domain that the same file did not create. `design.md` D1 gives them 14 form names:
  - A stored generated column. Each primary key and exclusion constraint.
  - A `CHECK` or foreign key without `NOT VALID`, and a unique constraint without `USING INDEX`. Inline constraints of `ADD COLUMN` count too.
  - An index without `CONCURRENTLY`.
  - `VACUUM FULL`, `CLUSTER`, `LOCK TABLE`, and `SET TABLESPACE`, `LOGGED`, `UNLOGGED`, or `ACCESS METHOD`.
  - `ALTER DOMAIN` with a constraint without `NOT VALID`, with `SET NOT NULL`, or with `VALIDATE CONSTRAINT`. The last form blocks writes in PostgreSQL 16.
- A form inside a function body or an `EXECUTE` string counts.
- `EXEMPT` gets the index of `0006_operations.sql` (ADR-0069). A migration that records a runbook index needs its own entry.
- New ADR-0136 records the forms. `data-model.md` §5.1 and §8, `reliability.md` §5 and §7.10, and `docs/ops/runbook.md` follow it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: ADDED requirement "CI rejects a migration that locks or rewrites an existing table". The current requirements stay unchanged.

## Impact

- `deploy/ci/migration_rewrite.py` and its tests. The CI job `migration-rewrite` stays unchanged.
- The documents of What Changes, the Status line of ADR-0127, `docs/adr/README.md`, and `CHANGELOG.md`.

## Out of scope

- No change of a shipped migration and no new migration.
- No check of `REINDEX`, `ALTER INDEX ... SET TABLESPACE`, `ALTER TABLE ALL IN TABLESPACE`, `ATTACH PARTITION`, or a materialized view.
- No check that a `CONCURRENTLY` file starts with `-- no-transaction`. The migration run of CI already fails on such a file.
- No change of Rust code.
