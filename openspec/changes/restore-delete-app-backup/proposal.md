# Proposal

## Why

Step 2 of "Restore into production" and step 1 of "Recover from the WAL archive" stop `app` and `backup` with `docker compose stop app backup`. Both services have `restart: always`, so a reboot or a restart of Docker starts them again inside the window.

`app` then writes to the old cluster, and the lost period grows without a message. `backup` runs `backup-once` with its retention, and its `restore-test` can hold the lock of `restore-volume`.

The change satisfies R31 (data safety) and R34 (safe long-term operation). It uses A35 (WAL archive on a second disk).

## What Changes

- New ADR-0175 records the delete. It amends ADR-0103 (b) step 2, ADR-0106 (i) step 1, and ADR-0106 (p) step 5.
- Both procedures delete the two containers with `docker compose rm -s -f app backup`.
- Each later start of `app` or `backup` in both procedures uses `docker compose up -d --no-deps`, also on a failed attempt.
- Before the delete, the runbook gives an optional copy of the logs of `app` and `backup`, because `rm` deletes them.
- The CI scripts `deploy/ci/restore-switch.sh` and `deploy/ci/pitr-recovery.sh` run the same command. A test compares each script with the runbook.
- `reliability.md` §7.7, `docs/ops/runbook.md`, and `CHANGELOG.md` get the new command. The runbook section "After a host reboot" names the window without containers.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: a restore into production and a recovery from the WAL archive delete the containers of `app` and `backup`.

## Out of scope

- Step 1 of "Rollback" (ADR-0174 (aj)) keeps `docker compose stop backup`. The owner decision of this change keeps that accepted rollback order unchanged.
- Step 1 of "Major upgrade" (`docker compose stop app`) and its failure path with `docker compose start app`.
- Step 4 of "Recover from the WAL archive" (`docker compose stop walarchive`).
- A change of the restart policy in `compose.yaml` (ADR-0175, option B).
- A new error code or a new setting of `reliability.md` §7.11.

## Impact

- `docs/adr/0175-delete-app-and-backup-before-a-restore.md`, the Status lines of ADR-0103 and ADR-0106, `docs/adr/README.md`.
- `deploy/ci/restore-switch.sh`, `deploy/ci/pitr-recovery.sh`, `deploy/ci/test_restore_switch.py`, `deploy/ci/test_pitr_recovery.py`, a comment in `.github/workflows/ci.yml`.
- `docs/architecture/reliability.md` §7.7, `docs/ops/runbook.md`, `CHANGELOG.md`.
- `openspec/specs/operations/spec.md` through the delta of this change.
