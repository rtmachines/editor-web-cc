# Proposal

## Why

The repo review of 2026-11-08 found two MEDIUM faults. `docs/plan/2026-11-08-fix-plan.md` section 4 (WP1) holds them.

- D7-1: a failed weekly `restore-test` or `basebackup-once` writes one `ERROR` line to the container log only. No condition of `reliability.md` §3.7 reads the result. At an incident, the newest set can fail to restore.
- D2-1: the scan of the backup directory in the job `alerts` has no time limit. A stalled mount stops the job.

The owner chose a new alert condition (decision Q-G (1)), because data safety comes first.

The change satisfies R30, R31, and R37. It uses Assumptions A16 and A35.

## What Changes

- The new ADR-0151 amends ADR-0060 (a), (y), ADR-0066 (m), ADR-0067 (step 9), ADR-0068 (g), ADR-0083 (b), and ADR-0103 (s).
- `restore-test` writes `/backups/restore-test.result`. `basebackup-once` writes `/wal/base-backup.result`. Each file holds the result `passed` or `failed` and the end time.
- A run that a `TERM`, `INT`, or `HUP` signal ends writes the result `failed`. `basebackup-once` gets the signal traps of `restore-test`.
- The job `alerts` gets the conditions `restore_test` and `base_backup` after `backup_age`.
- A condition is active on a result `failed`, an old result, a missing result after the grace, or a read fault.
- `reliability.md` §7.11 gets three rows: `restore_test.max_age_alert`, `basebackup.max_age_alert`, and `check_result.max_bytes`.
- The top directory of `walarchive` gets the mode `0711` (ADR-0151 (k), option K1 of the owner).
- The switch of a WAL line also moves `/wal/base-backup.result` into the target directory (ADR-0151 (q)).
- One free function of `recipe-server` gives the time limit of ADR-0133 to each blocking read of the backup volumes.
- Step 5 of the upgrade procedure runs `basebackup-once` once.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: the result files of the two weekly checks, the two new alerts, and the time limit of the reads of the backup volumes.

## Impact

- Code: `recipe-app` (`AlertCondition`, a new port and its memory adapter, `AlertSettings`, the job `alerts`).
- Code: `recipe-server` (`backups.rs`, `disk.rs`, a new shared module, a new result reader, the wiring).
- Scripts: `restore-test`, `basebackup-once`, `walarchive-entry`, a new file of `deploy/pgtools/lib/`, and their tests.
- Image and CI: `deploy/pgtools/Dockerfile`, `deploy/pgtools/.dockerignore`, `deploy/ci/prepare.sh`, `modes.sh`, `switch-steps.sh`, and `test_restore_switch.py`.
- Documents: ADR-0151, `docs/adr/README.md`, six Status lines, `reliability.md` §1.1, §3.7, §7.7, §7.10, §7.11, §8, `domain-model.md` §5, `overview.md` §5, the runbook, and `CHANGELOG.md`.
- No migration, no new error code, no new key of `serve`, and no change of the export.

## Out of scope

- An alert for `pitr-check`, `restore-volume`, or `pitr-volume`. They run on demand.
- A check of the off-site copy (ADR-0020).
- A new webhook body. The body stays `{"alert":"<name>"}`.
- A new setting for the time limit. The reads use `health.disk_check_interval` (ADR-0133).
