# ADR-0105: Tag of a data volume and the checks of the upgrade

Status: Accepted. Amended by ADR-0106 (item (b): the change log also holds the timeline of each volume)
Date: 2026-10-30
Deciders: project owner

## Context

A code review of the change `restore-volume-switch` found faults in ADR-0103, ADR-0104, the runbook, and the scripts. The user decided the first four groups below on 2026-10-30.

Start of `db` and `app` on a volume:

- After a failed `restore-volume` in a rollback restore, the runbook starts `app` and `backup` with the previous tag. The old volume holds the newer schema.
- A switch back after a rollback restore also starts `app` with the previous tag on the old volume.
- ADR-0083 forbids both states. `/readyz` still reports ready, because the readiness does not compare the schema with the image.
- In a major upgrade, step 2 of ADR-0103 (f) can start `db` again on the old volume. That start uses the new `compose.yaml`. The image of the new major version refuses the old data directory, and `db` restarts in a loop.
- ADR-0103 (f), (g), (p), and (q) do not give the files and the tag of such a start.

Restore test and identifier step:

- `reliability.md` 7.10 step 3 runs `docker compose exec backup restore-test`. A `backup` container of 0.1.0 can restart in a loop after a restart, because of the fault of ADR-0104.
- The identifier step of ADR-0104 stops when `/wal` holds no complete segment. When the old `walarchive` restarts in a loop, `pg_switch_wal()` brings no complete segment into `/wal`. The step then cannot pass.

Name check:

- The `compose.yaml` of 0.1.0 declares `pgdata` without `external` and ignores `RECIPE_PGDATA_VOLUME`. A name check on the files of 0.1.0 passes and proves nothing.
- `reliability.md` 7.10 gives no step that installs the new files of `deploy/` before the name check.

Edited text of ADR-0103:

- Commits after the acceptance of ADR-0103 changed its body. An accepted ADR changes only by its Status line and an "Amended by" note.
- The edited text held rules that other documents use. One sentence of it contradicts ADR-0104, because it pipes the identifier into the old `walarchive` container.
- This ADR restores the body of ADR-0103 and holds those rules.

The review also found faults in `restore-volume`, `wal-identity.sh`, and `cluster-access.sh`. Items (q) to (u) give the fixes. They are the defaults of the change, not decisions of the user.

## Decision drivers

- A start of `db` or `app` never meets a volume that its files or its image do not fit.
- An upgrade never continues on a volume other than the volume of the running `db`.
- A step of the upgrade does not depend on the state of a container of 0.1.0.
- The identifier step keeps its check of the identifier. It never trusts the volume name.
- Each rule has one source. An accepted ADR changes only by its Status line and an "Amended by" note.

## Considered options

Start on a volume:

- Option A: the change log of the site names the tag of each volume. Each start of `db` or `app` uses the files and the tag of that volume. Cons: the operator keeps the directory `deploy/` of the old release after a major upgrade.
- Option B: `app` refuses a schema that its image does not fit. Cons: a change of production code, and it does not cover `db` of another major version.
- Option C: keep the procedures and add warnings. Cons: the faults stay.

Restore test of the upgrade:

- Option D: `docker compose exec backup restore-test`. Cons: the test depends on the state of the old `backup` container.
- Option E: `docker compose run --rm --no-deps backup restore-test` with the previous tag. Cons: the test needs the previous tag for one command.

Name check:

- Option F: compare only the names. Cons: the files of 0.1.0 pass the check without effect.
- Option G: also refuse a `pgdata` that is not external. Cons: none found.

## Decision

The project uses **Option A**, **Option E**, and **Option G**. These are decisions of the user of 2026-10-30.

Tag of a data volume:

- (a) Each start of `db` or `app` on a data volume uses the `compose.yaml` and the tag of the release that fits that volume. The tag is the value `RECIPE_VERSION`.
  - `db` takes its image from `compose.yaml` as a digest. A `compose.yaml` fits a volume when its `db` image has the PostgreSQL major version of that volume.
  - `app` takes its image from the tag. A tag fits a volume when ADR-0083 permits its `app` image on the schema of that volume.
  - That tag is the tag of the schema (ADR-0083 (a)). After a rollback with the expand pattern, the previous tag also fits (ADR-0083 (c)).
  - Inside one major version, the installed `compose.yaml` serves each tag of that major version. Only `RECIPE_VERSION` changes, as in ADR-0083 (c).
  - The upgrade of `reliability.md` 7.10 keeps this rule. Its replacement of `db` keeps the major version, and the new `app` starts only after `migrate`.
  - The readiness of `app` does not compare the schema with its image. The operator therefore applies the rule before each start.
- (b) The change log of the site holds the tag of each volume that a switch leaves.
  - In step 2 of ADR-0103 (f), the operator writes the tag next to the old name and the line `Database system identifier`.
  - A switch back writes the same three values for the volume that it leaves (ADR-0103 (g)).
  - A new volume fits the tag of the image of `restore-volume`. That tag is the tag of the schema of the set (ADR-0103 (h)).
  - This item amends ADR-0103 (f) step 2 and (g).
- (c) A run can fail before step 2 of ADR-0103 (f) sets the new name. Then `db` stays on the old volume. Before `app` and `backup` start again, the operator sets the tag of the old volume in `deploy/.env`.
  - In the rollback restore of `reliability.md` 7.10 step 6, that tag is the new tag. The old volume holds the newer schema.
  - This item amends ADR-0103 (q). `deploy/.env` holds the previous tag only while the rollback restore continues.
- (d) A switch back reads the tag of the volume that it returns to from the change log. The operator sets that tag in `deploy/.env` before step 5 of ADR-0103 (f) starts `db`.
  - `walarchive`, `app`, and `backup` then start with that tag. After a major upgrade, the switch back runs in the directory of (e).
  - This item amends ADR-0103 (g).
- (e) The files of a release:
  - The operator installs the files of each new release into a new directory `deploy/`. The operator copies `.env` of the running release into it and keeps the old directory unchanged.
  - In a major upgrade, the operator keeps the directory and the images of the old release until the delete of ADR-0103 (t).
  - Each start of `db` on the old volume of a major upgrade runs in that directory. Its `compose.yaml` has the old major version. Its `.env` holds the old name and the old tag.
  - This item amends ADR-0103 (p) and (g). The "old `compose.yaml`" of ADR-0103 (g) is this directory.
- (f) In a major upgrade, step 2 of ADR-0103 (f) can find a state other than `shut down`.
  - Then the operator runs `docker compose up -d --no-deps db` in the directory of (e). `db` starts with the image of the old major version on the old volume.
  - Then the operator runs step 1 of ADR-0103 (f) again in the directory of the new release.
  - This item amends ADR-0103 (f) step 2.

Restore test and identifier step:

- (g) In step 3 of `reliability.md` 7.10, `restore-test` runs in a new container of the previous image: `docker compose run --rm --no-deps backup restore-test`.
  - `RECIPE_VERSION` holds the previous tag for this command.
  - A new container gets a new tmpfs `/run/pgpass`. The fault of the image of 0.1.0 (ADR-0104) therefore does not stop the test.
  - `--no-deps` keeps `db` unchanged. The test does not depend on the state of the old `backup` container.
  - This item amends ADR-0067 (a) for step 3 of 7.10. The other runs of `restore-test` keep `docker compose exec backup restore-test`.
- (h) The identifier step of ADR-0104 reads a `.partial` file when `/wal` holds no complete segment.
  - The step finds the newest name of the grammar `^[0-9A-F]{24}\.partial$`, with `LC_ALL=C`. It reads `xlp_sysid` from the first page of that file.
  - `pg_receivewal` writes each segment from its start. The first page of a `.partial` file therefore has the long page header.
  - The step checks the flag of the long header, as for a complete segment. A first page without the flag stops the step with one `ERROR` line, and the step writes no file.
  - A complete segment always comes first. The step reads a `.partial` file only when no complete segment exists.
  - Without a complete segment and without a `.partial` file, the step stops. Then ADR-0104 (d) gives the next action.
  - This item amends ADR-0104 (c) step 2 and (d).

Installation and name check:

- (i) In `reliability.md` 7.10, a new step installs the files of the new release before the name check. The files are `compose.yaml` and the rest of `deploy/`, in the directory of (e).
- (j) The name check refuses the upgrade in three cases:
  1. `docker compose config` does not show `external: true` for the volume `pgdata`.
  2. The name of `pgdata` in `docker compose config` differs from `RECIPE_PGDATA_VOLUME` in `deploy/.env`.
  3. That name differs from the volume that `docker inspect` of the running `db` container shows for `/var/lib/postgresql/data`.
  - Case 1 finds the files of 0.1.0. Their `compose.yaml` ignores `RECIPE_PGDATA_VOLUME`, and Compose gives its own volume the name `recipe-editor_pgdata`.
  - The operator sets `RECIPE_PGDATA_VOLUME` before the first Compose command of the upgrade. `${RECIPE_PGDATA_VOLUME:?}` stops each Compose command without the value.
  - The check also covers a name of an existing empty volume. Without the check, a replacement of `db` on that volume makes a new cluster.
  - After a refusal, the operator runs no Compose command that starts or replaces a container.
  - The CI job `upgrade` runs the check on a `compose.yaml` without `external: true` and expects the refusal of case 1.
  - This item amends ADR-0103 (d).

Rules out of the edited text of ADR-0103:

- (k) The mount of ADR-0103 (l) changes the service configuration of `db`. In the first release with that mount, step 3 command 1 of `reliability.md` 7.10 replaces `db` once on the same volume.
  - `app` loses its connections for the start time of `db`. This is a decision of the user of 2026-10-30.
  - ADR-0104 cites this rule as ADR-0103 (z). This item amends ADR-0103 (z).
- (l) The CI job `upgrade` no longer requires the same `db` container.
  - It checks that the new `db` container mounts the same volume.
  - It also checks that `pg_control_system()` returns the same `system_identifier` as before the upgrade.
  - This item amends ADR-0103 (z).
- (m) A storage test has no cluster of `restore-volume`. The CI job `restore-switch` therefore compares `postgresql.conf` and `postgresql.auto.conf` of a cluster of `restore-volume` with a cluster of `01-roles.sh`.
  - `db` gets `listen_addresses = '*'` only from the sample file of the image.
  - The operations change of ADR-0103 (z) adds the job `restore-switch`.
  - This item amends the last bullet of ADR-0103 (m) and ADR-0103 (z).
- (n) An existing installation has segments and no file `/wal/.system-identifier`. The operator writes the file once, before `walarchive` first starts with an image that has the guard of ADR-0103 (s).
  - ADR-0104 gives the position and the form of this step. The step runs only when the file is missing.
  - The step compares the identifier of `db` with `xlp_sysid` of the long page header of the first page. `xlp_sysid` is 8 bytes at offset 24, little-endian.
  - A difference stops the step, and the step writes no file. The step therefore checks the identifier and does not trust the volume name of (j).
  - The release notes of that release name this step. This item amends ADR-0103 (s).
- (o) "A gap in the WAL stream" of ADR-0099 (a) keeps `/wal/.system-identifier` in `/wal`.
  - The cluster does not change in a gap, so its identifier stays the same. Only step 4 of ADR-0103 (s) moves the file.
  - This item amends step 7 of ADR-0099 (a). The file also stays in `/wal` in that step.
- (p) The runbook text of the identifier step and the value `WAL_IDENTITY_STEP` of `deploy/ci/upgrade.sh` hold the same script. A test compares the two texts.

Fixes of the scripts:

- (q) `restore-volume` keeps the baselines of the checks of ADR-0103 (o) in shell variables.
  - In step 4 of ADR-0103 (i), it keeps the SHA-256 values of the three configuration files and the list of top-level names.
  - It writes no baseline into `/run/pgpass`. The temporary server runs as uid 999, and uid 999 can write there.
  - Step 8 of ADR-0103 (i) compares with the values in the shell variables.
  - Residual risk: the checks do not stop arbitrary code that runs as uid 999 in the temporary server. The script also runs as uid 999.
  - A later ADR that restores `pre-data` as a role that is not a superuser can close this risk (ADR-0084 (e)).
  - This item amends ADR-0103 (i) step 4 and (o).
- (r) In step 1 of ADR-0103 (i), `restore-volume` checks that it runs as uid 999 and that the name of that user is `postgres`. Else it writes one `ERROR` line before any PostgreSQL tool runs. This item amends ADR-0103 (i) step 1.
- (s) When `IDENTIFY_SYSTEM` fails, the guard of `wal-identity.sh` keeps the error of libpq.
  - The line `cannot read the system identifier of the server` gets the field `detail`. The field holds the first line of that error.
  - The guard cleans the field as `tool_detail` of `restore-steps.sh` does. The password comes from the pgpass file, so the error holds no password.
- (t) The guard of `wal-identity.sh` and the identifier step of ADR-0104 run `sync` on the temp file before `ln -T`. After `ln -T`, they run `sync` on the directory `/wal`.
  - A crash then cannot leave the final name on a file without its line.
  - This item amends ADR-0104 (c) step 4.
- (u) `write_hba_conf` of `deploy/db/cluster-access.sh` renames its temp file only after the write succeeds. A failed write never publishes a part of `pg_hba.conf`.

Changes of other documents:

- (v) The follow-up edits of this ADR are:
  - The body of ADR-0103 returns to its accepted text of commit `cdf925c`. Its Status line names ADR-0104 and this ADR.
  - The Status lines of ADR-0067, ADR-0099, and ADR-0104 get "Amended by ADR-0105" with the scope. ADR-0099 loses the note of ADR-0103 on its item (a).
  - `docs/adr/README.md` gets a new index row, the same notes in the Status cells, and the decision map.
  - `reliability.md` sections 7.7, 7.8, and 7.10.
  - `docs/ops/runbook.md` sections "Restore into production", "Upgrade", "Rollback", "Switch back", "Major upgrade", "A gap in the WAL stream", and "Messages of walarchive".
  - The OpenSpec change `restore-volume-switch`: `proposal.md`, `design.md`, the spec, and `tasks.md`.
  - `CHANGELOG.md`, section `[Unreleased]`.

## Consequences

### Positive

- No procedure starts `app` on a schema that its image does not fit. No procedure starts `db` with an image of another major version.
- The restore test and the identifier step of the upgrade do not depend on a container of 0.1.0.
- The name check finds the files of 0.1.0.
- ADR-0103 again holds its accepted text. Each moved rule has one source.

### Negative

- The operator keeps one directory `deploy/` per release. After a major upgrade, the old directory stays until the delete of ADR-0103 (t).
- The change log holds one more value per switch.
- The restore test of step 3 of 7.10 needs the previous tag for one command.

### Risks and mitigations

- Risk: the operator sets a tag that does not fit. Mitigation: the change log of (b) names the tag of each volume, and the runbook names the tag in each step that starts `app`.
- Risk: the operator sets a wrong existing volume at an upgrade. Mitigation: the check of (j) stops the upgrade, and the step of (n) writes no identifier file of another system.
- Risk: the first page of a `.partial` file holds no header yet. Mitigation: the flag check of (h) stops the step, and the step writes no file.
- Risk: code of the dump runs as uid 999 in the temporary server of `restore-volume`. Mitigation: the baselines of (q) stay out of its reach. The residual risk of (q) stays.

## Related

- Amends ADR-0103 (d), (f) step 2, (g), (i) steps 1 and 4, (m), (o), (p), (q), (s), and (z).
- Amends ADR-0104 (c) steps 2 and 4 and (d). Amends ADR-0099 (a) step 7 and ADR-0067 (a) (the restore test of step 3 of 7.10).
- ADR-0083 (a), (c) (tag of the schema), ADR-0084 (e) (residual risk of `pre-data`), ADR-0074 (b) (publish with `ln -T`).
- `docs/architecture/reliability.md` sections 7.7, 7.8, and 7.10.
- `docs/ops/runbook.md` sections "Restore into production", "Upgrade", "Rollback", "Switch back", "Major upgrade", and "Messages of walarchive".
- `overview.md` section 7: R31, R34. Section 9: A35.
