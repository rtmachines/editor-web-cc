# Proposal

## Why

A writer of the backup disk can replace a set and compute a new SHA-256 manifest. `restore-test` and `restore-volume` then run the SQL of that set as a superuser (ADR-0095 (h)). On 2026-11-15 the project owner chose option D of ADR-0095: an HMAC tag of the manifest with a key outside the backup disk. The tag protects against an offline writer of the disk (ADR-0174 (a)).

The change satisfies R31 (data safety), R34 (safe long-term operation), and R38 (check all external input). It uses A16 (daily dump and weekly restore test).

## What Changes

- New ADR-0174 records the threat model, the tag, the key, the tool, the write order, the check order, and the known limits. It amends ADR-0066, ADR-0067, ADR-0074, ADR-0095 (h), and ADR-0103.
- `backup-once` hashes each file of the set before its publish. It writes the tag file `recipe_<stamp>.dump.sha256.hmac` before it publishes the manifest.
- A new Docker secret `backup_manifest_key` holds at most two keys. `backup`, `backup-premigrate`, and `restore-volume` mount it. The key file must lie on a filesystem other than the backup disk.
- `restore-test` and `restore-volume` check the tag before any PostgreSQL tool reads the set. A missing or bad tag stops the run. No option skips the check.
- `backup-entry` checks the key file at each start.
- A Perl program of the base image computes and checks the tag, so the key enters no argument list and no shell variable.
- The new file `deploy/compose.previous.yaml` keeps the key away from a previous image without the tag check. It also turns off the weekly `restore-test` of that image.
- `restore-volume --check-only <stamp>` checks a set and stops. A rollback across the first release with the tag runs it with the new image first (ADR-0174 (aj)).
- `reliability.md` §7.1, §7.7, §7.10, §7.11 and `docs/ops/runbook.md` get the key, the tag file, the rotations, the offline copy, and the operator steps.
- **BREAKING** for the operator: each installation must create the secret file `backup_manifest_key` before the upgrade.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: the nightly dump writes a tag. The restore test and the restore into a new volume check it. The restore service mounts one secret.

## Out of scope

- A tag for the base backups and the WAL archive (ADR-0174 (d), (ak)).
- A defense against an online writer of the backup disk (ADR-0174 (ag)).
- A tag check of an off-site copy. The operator compares only the key ids of the offline key copy (ADR-0174 (r)).
- A replay alarm through `backup_age` (ADR-0174 (ai)).
- The comparison of `pg_restore --file` output, the second candidate of ADR-0095 (h).
- A change of ADR-0084: the attacker with the migrator credential stays a residual risk.
- A new error code. The scripts of `recipe-pgtools` write `ERROR` lines only.
- A new package in the image `recipe-pgtools`.

## Impact

- `deploy/pgtools/bin/backup-once`, `backup-entry`, `restore-test`, `restore-volume`, `deploy/pgtools/lib/restore-steps.sh`, new `lib/manifest-tag.sh` and `lib/manifest-tag.pl`, `deploy/pgtools/Dockerfile`, `deploy/pgtools/.dockerignore`.
- `deploy/compose.yaml` (secret and three mounts), new `deploy/compose.previous.yaml` and `deploy/crontab.previous`, `deploy/ci/check.sh`, `deploy/ci/prepare.sh`, `deploy/ci/recovery.sh`, `deploy/ci/restore-switch.sh`, `deploy/ci/upgrade.sh`, `deploy/ci/upgrade.compose.yaml`, `deploy/ci/upgrade_checks.py`, `deploy/ci/test_upgrade_checks.py`, the suites of `deploy/pgtools/tests/`.
- `crates/recipe-server/tests/deploy_values/` (secret lists, script defaults, override rules) and `crates/recipe-server/src/backups/tests.rs`.
- `docs/architecture/reliability.md`, `docs/ops/runbook.md`, `CHANGELOG.md`.
