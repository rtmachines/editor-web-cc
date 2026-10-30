# Proposal

## Why

Release 0.1.0 has no working restore into production, rollback restore, or major upgrade. This change builds the decisions of ADR-0103. It satisfies R04, R30, R31, and R34.

The change uses A1, A15, and A35.

## What Changes

- **BREAKING** `pgdata` is an external volume with `name: ${RECIPE_PGDATA_VOLUME:?}`. `deploy/.env` holds nine values (ADR-0103 (c), (d)).
- One shared file in `deploy/db/` holds the lines of `pg_hba.conf` and the two `REVOKE` statements. `01-roles.sh` and the `recipe-pgtools` image use it (ADR-0103 (l), (m)).
- A one-shot service `restore-volume` with the profile `restore` fills a new volume offline. It applies the gates of `restore-test` and the checks of ADR-0103 (m) and (o).
- `restore-test` and `restore-volume` share one library in `deploy/pgtools/lib/` (ADR-0103 (j)).
- The file `restore-roles.expected` and a storage test prove the roles, the grants, and the client rules of a restored cluster.
- **BREAKING** `walarchive-entry` and `basebackup-once` check `/wal/.system-identifier`. The release notes and the runbook give the one step for an existing installation (ADR-0103 (s)).
- The upgrade sets `RECIPE_PGDATA_VOLUME` and compares it with the mount of the running `db` before the first Compose command. The one step of the identifier file compares the identifier with the newest segment (ADR-0105 (j), (n)).
- The first upgrade to this release replaces `db` once on the same volume, because `db` gets a new mount (ADR-0105 (k)).
- A CI job `restore-switch` runs the restore, the switch, the WAL line, and the switch back.
- A CI step fails a release that changes the major version of `db` and adds a migration file (ADR-0103 (p)).
- The runbook gets the full procedures of the restore, the rollback restore, and the major upgrade.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: the data volume, the roles and client rules of a restored cluster, and the restore into a new volume. It also changes the switch, the switch back, the WAL line, its guard, the delete rule, the container limits, and the CI jobs.

## Impact

- Deploy: `compose.yaml`, `compose.build.yaml`, `.env.example`, `db/`, `pgtools/`, and `ci/`.
- CI: `.github/workflows/ci.yml`.
- Crates: tests only, in `recipe-db` and `recipe-server`. No production Rust code changes.
- Documents: `docs/ops/runbook.md`, `CHANGELOG.md`, `README.md`, and the dev-stack line of `CLAUDE.md`. No task changes an ADR or a file of `docs/architecture/`. ADR-0103 and `reliability.md` 7.7, 7.8, and 7.10 already hold the decisions of this change.
- No new error code and no new dependency. The three rows of `reliability.md` 7.11 for `restore_volume.*` and `volume_switch.keep_min_age` are already in `reliability.md`.

## Out of scope

- A recovery of production from the WAL archive (ADR-0103 (x)).
- A rollback restore to 0.1.0 (ADR-0103 (q)).
- A CI proof of the major upgrade with an image of a newer major version (ADR-0103 (p)).
- A restore of `pre-data` as a role that is not a superuser (ADR-0103 (o)).
- A script that deletes the old volume or the moved WAL files (ADR-0103 (t)).
