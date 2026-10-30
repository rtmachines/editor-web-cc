# ADR-0083: Image of `backup` and `walarchive` in the upgrade and the rollback

Status: Accepted. Amended by a note (VX11-4: reconciles the normative wording). Amended by ADR-0103 (item (d): `walarchive` starts before `app` after a rollback restore)
Date: 2026-10-24
Deciders: project owner

## Context

`reliability.md` section 7.10 gives the upgrade procedure. ADR-0082 sets `docker compose up -d --no-deps app` in steps 4 and 6. The review of section 7.10 found these faults:

1. No step of section 7.10 gives the new `recipe-pgtools` image to the long-running services `backup` and `walarchive`. Steps 3 and 4 replace only `backup-premigrate`, `migrate`, and `app`.
2. `restore-test` compares the newest set with the `restore-code.expected` of the running image (ADR-0067 (j)). That file holds the definitions of the schema of one release.
3. With the old image, the first daily dump after the upgrade holds the new schema. Each later weekly test then fails when the migration adds or changes a line of `restore-code.expected`.
4. `restore-test` selects the newest set by name, also a `_premigrate` set. With the new image and the `_premigrate` set of the old schema, the test fails at the first comparison of the code.
5. A fix of a pgtools script in a release does not reach `backup` and `walarchive`.
6. After a rollback with the expand pattern, the schema stays newer (ADR-0059 (i)). The previous image holds the previous `restore-code.expected`.

Each fault fails closed. The review found no false pass and no data loss. The user chose a new ADR before the runbook of task 1.5 and the CI job of task 8.7.

## Decision drivers

- Data safety over performance. A weekly restore test proves the newest set (R31, A16).
- A test that fails on each good set gives no proof. A real fault then looks like the known false failure (R35).
- 24/7 operation with Docker Compose on one host (R30, A1).
- Section 7.10 stays a short list of commands for the runbook and the CI job (R34).
- No change of a script, no new setting, and no new value of `deploy/.env`.

## Considered options

### Option A: The image of `backup` and `walarchive` follows the schema

- Pros: the `restore-code.expected` of the running image always equals the schema of `db`. A fix of the release reaches both services. No script changes.
- Cons: steps 5 and 6 get three more commands. After a rollback with the expand pattern, `app` and `backup` run the images of two releases.

### Option B: Steps 5 and 6 give `backup` and `walarchive` the image of `app`

- Pros: one tag for each service after each step.
- Cons: after a rollback with the expand pattern, the previous `restore-code.expected` does not hold the newer definitions. Each restore test then fails closed until the next upgrade.

### Option C: `restore-test` skips each `_premigrate` set

- Pros: the test never selects a set of the schema before the migration.
- Cons: the script changes. `backup` keeps the old image, so the daily sets of the new schema still fail. A fix of the release still does not reach the services.

### Option D: A separate tag variable for `recipe-pgtools`

- Pros: `compose.yaml` holds the tag of the schema for the pgtools services. A plain `docker compose up -d` keeps it.
- Cons: `deploy/.env` gets a ninth value. `compose.yaml`, `deploy/.env.example`, and the CI job change. The tags of `migrate` and `app` still differ after a rollback, so the rule of (c) stays necessary.

## Decision

The project uses **Option A**.

Rule:

- (a) `backup` and `walarchive` run the `recipe-pgtools` image of the release whose migrations the schema of `db` holds. This ADR calls its tag the tag of the schema.
  - The storage test of ADR-0067 (j) proves that the `restore-code.expected` of a release equals the schema after its migrations.
  - After a rollback with the expand pattern, the image of `app` differs from the tag of the schema (ADR-0059 (i), ADR-0082 (b)).
  - A restore test proves a set only with the image of the tag of the schema. This item amends ADR-0067 (j).

Upgrade:

- (b) Step 5 of section 7.10 ends with three commands. They run after the checks of step 5 pass, while the new `app` serves.
  1. `docker compose up -d --no-deps backup walarchive`. Compose replaces both containers, and each one gets the new image.
  2. `docker compose exec backup backup-now`. The new `_now` set holds the new schema and is the newest set.
  3. `docker compose exec backup restore-test`. The operator checks that the test passes and names the `_now` set.
  - `restore-test` sorts the names with `LC_ALL=C`. In the same UTC minute, the `_premigrate` set sorts after the `_now` set, and the test selects it.
  - The operator then runs commands 2 and 3 again in a later UTC minute.
  - This item amends ADR-0020 (the upgrade procedure) and ADR-0067 (a) (the times of the restore test).

Rollback:

- (c) Step 6 gives the previous tag only to `app`: `RECIPE_VERSION=<previous tag> docker compose up -d --no-deps app`.
  - Compose takes a variable of the shell environment before a value of `deploy/.env` (Compose documentation of the interpolation).
  - `deploy/.env` keeps the new tag, because the schema stays newer. A later `docker compose up -d` therefore runs no previous `migrate` against the newer schema.
  - Then step 6 runs the three commands of (b) with the tag of `deploy/.env`. When step 5 ran command 1, Compose finds no change and keeps both containers.
  - This item amends ADR-0082 (b) (the previous tag reaches only `app`).
- (d) After a restore of the `_premigrate` dump, the schema is the previous schema. The operator sets the previous tag in `deploy/.env`. Then the operator runs `docker compose up -d --no-deps app` and the three commands of (b).
  - Command 1 then gives the previous image to `backup` and `walarchive`.
  - The `_now` set of command 2 is newer than each set of the new schema. Command 3 therefore compares a set of the previous schema with the previous `restore-code.expected`.

`walarchive`:

- (e) Command 1 replaces `walarchive` in the same step as `backup`. `walarchive` runs no check of the schema, but a fix of the release can change its scripts. One tag for both services keeps one rule.
  - The replacement stops the stream for the time of a container start. `pg_receivewal` uses no slot, and `db` keeps WAL through `pg.wal_keep_size` (ADR-0068 (b), section 7.6).
  - An outage shorter than `pg.wal_keep_size` leaves no gap in the archive (ADR-0068 (e)).
  - The job `alerts` can raise the condition `replication` during the replacement (section 3.7).

Time and disk:

- (f) The three commands run while the new `app` serves. They add no time to the downtime of step 4.
  - Section 7.11 gives no limit for a whole restore test. Its run time depends on the size of the database, as for the weekly run.
  - The scratch cluster needs the free space of one restored database on the backup disk, as for the weekly run (ADR-0067).
  - The `_now` set adds one set. The retention of ADR-0066 (k) deletes it after `backup.retention`.
  - The test uses `container.backup_memory` and the `scratch.*` settings of section 7.11, as for the weekly run.
  - The upgrade therefore holds the restore test. A failed test does not start a rollback by itself. The runbook names the next action.

Runbook and CI:

- (g) The runbook of task 1.5 writes steps 5 and 6 with the commands of (b) to (d). The CI job of task 8.7 uses the same commands for each step of section 7.10 that it runs.
  - No script changes. `deploy/pgtools/bin/backup-now` exists and runs `backup-once _now` (ADR-0066 (i)).

## Consequences

### Positive

- After an upgrade or a rollback, the weekly restore test compares each set with the `restore-code.expected` of its schema.
- A pgtools fix of a release reaches `backup` and `walarchive` in the upgrade.
- Each upgrade and each rollback ends with a tested set of the current schema.

### Negative

- Steps 5 and 6 get three commands each. The restore test makes the upgrade longer, but not the downtime.
- After a rollback with the expand pattern, `app` runs the previous image, and `deploy/.env` holds the new tag. A later `docker compose up -d` gives `app` the new image again.

### Risks and mitigations

- Risk: command 1 stops a job of the schedule that runs, for example `backup-once` at `backup.time`. Mitigation: the runbook names `backup.time`, `basebackup.time`, and `restore_test.time`, and the operator avoids them.
  - A stopped `backup-once` leaves its temp names (section 7.7). The next run of the job writes a new set.
- Risk: a release with two or more migration files can fail after the first file in step 3. Then no image matches the schema, and each restore test fails closed. Mitigation: the runbook gives a forward fix or a restore of the `_premigrate` dump.
- Risk: no restore test checks the `_premigrate` set itself, because the test always selects the newest set. Mitigation: the restore test before the release (ADR-0067 (a)) checks a set of the same schema with the same image.
- Risk: in a rollback with the expand pattern, the operator sets the previous tag in `deploy/.env`. A later replacement of `backup` then gives it the previous image, and the weekly test fails closed. Mitigation: the runbook gives the full command of step 6.

## Related

- Amends ADR-0020 (the image of `backup` and `walarchive` in the upgrade procedure).
- Amends ADR-0067 (a) (the restore test in steps 5 and 6) and (j) (the image of the tag of the schema).
- Amends ADR-0082 (b) (the previous tag reaches only `app`, and the commands of step 6 after `app`).
- ADR-0059 (i) (readiness on a newer schema), ADR-0066 (i), (k) (`backup-now`, retention).
- ADR-0068 (b), (e) (stream without a slot), ADR-0074 (c) (grammar of the set names).
- ADR-0076 and ADR-0078 (the lines of `restore-code.expected`).
- `docs/architecture/reliability.md` sections 7.7 (the restore test), 7.10 (steps 5 and 6), and 7.11 (`scratch.*`, `container.backup_memory`, `pg.wal_keep_size`).
- `deploy/pgtools/bin/restore-test` (`select_set`, `check_names`, `compare_code`) and `deploy/pgtools/bin/backup-now`. `deploy/compose.yaml` (the `image:` of `backup` and `walarchive`).
- Docker Compose documentation of the interpolation (the shell environment comes before `.env`).
- Baseline requirement: `operations` "Restore test".
- `overview.md` section 7: R30, R31, R34, R35. Section 9: A1, A16, A35.
