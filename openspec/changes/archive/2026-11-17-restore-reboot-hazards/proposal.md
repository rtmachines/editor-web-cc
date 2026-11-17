# Proposal

## Why

ADR-0175 (i), (j), and (k) keep three stop steps. `app`, `backup`, and `walarchive` have `restart: always`, so a reboot or a restart of Docker starts a stopped container again:

- Step 1 of "Major upgrade" stops `app`. `app` then writes to the old cluster after the `_now` dump, and the switch loses these writes.
- Step 4 of "Recover from the WAL archive" stops `walarchive`. It then writes into `/wal` again, so the `.partial` file changes.
- In "Rollback", step 1 of the restore of the `_premigrate` set stops `backup`. `backup` then runs its crontab on the old volume until the switch.

The change satisfies R31 and R34, and uses A35.

## What Changes

- New ADR-0176 records the decision. It amends ADR-0175 (h), (i), (j), (k), ADR-0106 (i) step 4 and (p) step 5, and ADR-0174 (aj).
- Step 1 of "Major upgrade" deletes the container with `docker compose rm -s -f app`. A failed step before step 8 starts `app` with `docker compose up -d --no-deps app`.
- Step 4 of the recovery deletes the container with `docker compose rm -s -f walarchive`. Each later start uses `docker compose up -d --no-deps walarchive`. A failed attempt first returns to the release of `<tag>`.
- `deploy/ci/pitr-recovery.sh` runs the same delete, and a test compares it with the runbook.
- "After a host reboot" names the windows of the major upgrade and of the rollback. It drops the case "Run step 4 of the recovery again".
- Hazard (i), option A (owner decision of 2026-11-16): step 1 of the restore of the `_premigrate` set deletes `backup` with `docker compose rm -s -f backup` before the check of the set. `deploy/ci/upgrade.sh` follows.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: the major upgrade, the recovery, and the rollback delete the containers of `app`, `walarchive`, and `backup`.

## Out of scope

- Hardening of the `deploy/ci` tests (review findings D2, D3, T-01 to T-05 of 2026-11-13). A separate change does it.
- A new CI job for the major upgrade.
- Step 2 of "A gap in the WAL stream" (ADR-0099 (a)).
- A change of the restart policy in `compose.yaml`.

## Impact

- ADR-0176, the Status lines of ADR-0174, ADR-0175, ADR-0106, and `docs/adr/README.md`.
- `docs/ops/runbook.md`, `reliability.md` §7.7 and §7.10, `CHANGELOG.md`, `.github/workflows/ci.yml`.
- `deploy/ci/pitr-recovery.sh`, `deploy/ci/upgrade.sh`, and their tests in `deploy/ci/`.
- `openspec/specs/operations/spec.md` through the delta.
