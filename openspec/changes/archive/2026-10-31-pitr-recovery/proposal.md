# Proposal

## Why

No procedure recovers production from the WAL archive. The only workaround is a restore of a dump, which loses up to 24 hours of changes and audit entries. This change builds ADR-0106 and satisfies R31 and R34.

The change uses A1, A15, and A35.

## What Changes

- A one-shot service `pitr-volume` fills a new volume from a base backup and `/wal`. It mounts `/wal` read-only and no secret (ADR-0106 (e), (f)).
- The default target is the end of the archive with the `.partial` file. An optional `--target-time` undoes a logical fault (ADR-0106 (a), (b)).
- `basebackup-once` writes `backup_end_time`. A target time never selects a base backup that ended after it (ADR-0106 (c)).
- A gate checks the end of the replay, the audit chain, a marker entry, the configuration files, and the roles (ADR-0106 (g)).
- The switch refuses a drill volume with the file `pitr-drill` (ADR-0106 (j), (o)).
- `pitr-check` and `pitr-volume` share one library (ADR-0106 (h)).
- The switch of a recovery compares the timeline too. The change log holds the timeline (ADR-0106 (j), (l)).
- The move of a new WAL line also takes the history files (ADR-0106 (k)).
- **BREAKING** The guard refuses a segment of another timeline in `/wal` (ADR-0106 (m)).
- New rows `pitr_volume.start_timeout` and `pitr_volume.stop_timeout` (ADR-0106 (n)).
- A CI job `pitr-recovery` proves the recovery point, the target time, and the recovery time (ADR-0106 (p)).
- The runbook gets the recovery and a drill. "Known gaps" loses two bullets (ADR-0106 (o)).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: the recovery from the WAL archive, the switch, the switch back, the WAL line, the guard, the container limits, and a CI job.

## Impact

- Deploy: `compose.yaml`, `pgtools/`, and `ci/`. CI: `.github/workflows/ci.yml`.
- Crates: tests in `recipe-server` only. No production Rust code changes.
- Documents: ADR-0106, six Status lines, `docs/adr/README.md`, `reliability.md` 7.7, 7.8, and 7.11, the runbook, and `CHANGELOG.md`.
- No new error code, Assumption, or dependency.

## Out of scope

- A target LSN or a target transaction ID (ADR-0106, option C).
- A recovery from the moved files of an earlier WAL line (ADR-0103 (v)).
- A recovery after the loss of the WAL archive disk. The restore of a dump stays (ADR-0103).
- A script that deletes the old volume, the drill volume, or moved WAL files (ADR-0103 (t)).
