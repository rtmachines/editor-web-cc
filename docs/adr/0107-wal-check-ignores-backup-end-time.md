# ADR-0107: The WAL check ignores the file backup_end_time

Status: Accepted
Date: 2026-10-31
Deciders: project owner

## Context

ADR-0106 (c) makes `basebackup-once` write the file `backup_end_time` into each base backup directory under `/wal/base/<name>/`. The `backup_manifest` of the base backup does not list this file.

The WAL check of ADR-0099 (a3) runs `pg_verifybackup` on that directory. `pg_verifybackup` fails on a file that the manifest does not list. The WAL check therefore fails on every new base backup.

The check of a copy in ADR-0106 (c), (f) already ignores `backup_end_time`. The WAL check needs the same rule.

## Decision

This is the decision of the user of 2026-10-31. It amends ADR-0099 (a3).

- (a) The WAL check of ADR-0099 (a3) adds the option `--ignore=backup_end_time`:

  ```bash
  docker compose exec walarchive pg_verifybackup --quiet --skip-checksums --ignore=backup_end_time --wal-directory=/wal /wal/base/<base>
  ```

  - Every run of `pg_verifybackup` on a published base backup directory under `/wal/base/` ignores `backup_end_time`.
  - The file stays in the base backup directory, as ADR-0106 (c) states. No other rule of ADR-0099 or ADR-0106 changes.

## Consequences

- The WAL check passes on a base backup with `backup_end_time` when its WAL range is complete in `/wal`.
- The check of a missing or unreadable segment does not change. `--ignore` skips only the one file name.
- A base backup from before ADR-0106 has no `backup_end_time`. The option then has no effect, and the check stays the same.

## Related

- Amends ADR-0099 (a3) (the WAL check ignores `backup_end_time`).
- ADR-0106 (c) (the file `backup_end_time`), ADR-0106 (f) (the check of a copy).
- `docs/ops/runbook.md` section "A gap in the WAL stream".
- PostgreSQL 16 documentation: `pg_verifybackup` (options `--ignore` and `--wal-directory`).
