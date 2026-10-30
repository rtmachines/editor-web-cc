# ADR-0103: Volume switch, restore target, and WAL line after a switch

Status: Accepted
Date: 2026-10-29
Deciders: project owner

## Context

A restore into production, the rollback with the `_premigrate` dump, and the major upgrade of PostgreSQL each need a new `db` volume. The documents leave three steps open:

- No document gives the mechanism that makes a new volume the volume of `db`.
- The runbook restores into an empty cluster with no project role. On an empty volume, `db` runs `01-roles.sh`, and that script creates the roles and the database `recipe`.
- No document handles the WAL archive after a switch. A new cluster has a new system identifier and starts its segment names at timeline 1 again.

The documents also disagree in six places:

1. Order. `reliability.md` section 7.10 step 8 fills the new volume before the switch. The runbook section "Major upgrade" restores the `_now` set after the switch, in the running `db`.
2. New-volume rule. The runbook cites ADR-0020 for "the restore goes into a new `db` volume". ADR-0020 states this rule for the major upgrade only. No normative document states it for a restore of the same major version.
3. Role source. `.claude/rules/sql.md` says that roles come only from `01-roles.sh`. `reliability.md` section 7.7 and `restore-test` create the roles from the globals file of the set.
4. Client rules and default databases. Neither the globals file nor the dump holds the `pg_hba.conf` of ADR-0069 or the `REVOKE` statements of ADR-0032. A cluster without `01-roles.sh` keeps the catch-all line `host all all all scram-sha-256` of the image.
5. Old base backups. ADR-0077 (f) expects that old base backups stay in `/wal/base` next to a new cluster. ADR-0099 (c) says that moved segments keep the older base backups recoverable. The running stack cannot use the moved files, because `restore_command` reads `/wal` only.
6. Count of `.env` values. ADR-0062 (i) and `reliability.md` section 7.1 fix the count at eight. A variable for the volume name is a ninth value.

Facts that each option must handle:

- `db` mounts `pgdata` read-write. `app` and `backup` mount it read-only for `statvfs` and for `backup-health`.
- After a reboot, Docker starts each container with `restart: always` again, also a stopped one. A switch therefore deletes the old containers. It does not only stop them.
- `docker compose down -v` deletes a volume that Compose created. That includes `pgdata` of 0.1.0.
- `pg_receivewal` runs without a slot. It continues after the newest complete segment in `/wal`. Old names of another system can stop it in a restart loop, or they can mix two systems in one directory without an error.

## Decision drivers

- Data safety comes before the time of the procedure. The old cluster stays unchanged until the switch.
- A wrong volume name fails closed. It never starts an empty cluster, and an upgrade never continues on a volume other than the volume of the running `db`.
- The production restore uses the code path that the weekly restore test proves.
- A restored cluster never serves a connection under the client rules of the image.
- Each write of the new cluster reaches the WAL archive.
- Each rule has one source. An accepted ADR changes only by its Status line and an "Amended by" note.

## Considered options

Volume switch:

- Option A: an external volume. A variable of `deploy/.env` names it. A missing name stops `up`, and `down -v` cannot delete it. Cons: a ninth `.env` value and one more step in the host preparation.
- Option B: one subdirectory of one volume per cluster. Cons: the cluster of 0.1.0 lies at the root of the volume. A new subdirectory lies inside its live data directory.
- Option C: an override file mounts a new volume. Cons: one command without the override starts `db` on the old volume, and the operator gets no message.
- Option D: a fixed name and a copy back. Cons: three copies of the cluster at the peak and one delete of the live name in each restore.

Restore target:

- Option E: `db` runs `01-roles.sh` on the new volume. The operator drops the empty database and restores into `db`. Cons: a `DROP DATABASE` in production, and no gate of `restore-test` runs.
- Option F: a one-shot pgtools service fills the new volume offline, with the gates of `restore-test`. Cons: the client rules need one shared source. A rollback to 0.1.0 has no such service.
- Option G: `db` starts without the init mount, and the operator restores into `db`. Cons: until a manual fix, the image line admits the superuser over TCP.

WAL line:

- Start the new WAL line before `app` starts. Cons: longer downtime.
- Start it after `app` starts. Cons: a write before the first base backup of the new line is only in `pgdata`.

## Decision

The project uses **Option A** for the switch and **Option F** for the restore target. The new WAL line starts before `app`.

New volume and order (conflicts 1 and 2):

- (a) Every restore into production writes into a new, empty volume. This rule covers a restore of the same major version, the rollback with the `_premigrate` dump, and the major upgrade.
  - No procedure restores into the volume of a running `db`. No procedure writes into the old volume.
  - This item amends ADR-0020, section "Major upgrade" and its last risk. The rule now holds for every restore, not only for the major upgrade.
- (b) Each restore into production uses this order:
  1. `restore-test` passes. When the set to restore is not the newest set, steps 1 to 8 of `restore-volume` are the test of that set. The login check of `restore-test` step 8 then has no equal before the switch.
  2. The operator stops `app` and `backup` with `docker compose stop app backup`. No write after this point reaches the restored cluster. No retention deletes the set during the run.
  3. The operator creates the new volume.
  4. `restore-volume` of (h) fills the new volume. `db` runs on the old volume during the whole run.
  5. The switch of (f) makes the new volume the volume of `db`.
  6. The operator sets the role passwords in `db` from the secret files (ADR-0033).
  7. The new WAL line of (r) starts.
  8. Before `app` starts, the operator truncates `session` and `login_attempt` and re-applies the user changes after the dump time (`reliability.md` section 7.7). The operator records the values of ADR-0100 (c). Then the operator runs `docker compose up -d --no-deps app`, then `docker compose up -d --no-deps proxy backup`. The wait of ADR-0102 ends the restore.
  - `reliability.md` section 7.10 step 8 states the right order. The runbook order "restore after the switch" is wrong and goes away.

Volume switch (conflict 6):

- (c) `compose.yaml` declares the volume `pgdata` with `external: true` and `name: ${RECIPE_PGDATA_VOLUME:?}`.
  - One key moves the mounts of `db`, `app`, and `backup` together.
  - Compose never creates and never deletes this volume. `docker compose down -v` cannot delete it.
  - A name without a volume stops `docker compose up` with an error. It does not start an empty cluster.
- (d) `RECIPE_PGDATA_VOLUME` is the ninth value of `deploy/.env`. It is mandatory and has no default. This item amends ADR-0062 (i).
  - An existing installation sets the name that `docker inspect` of the running `db` container shows for `/var/lib/postgresql/data`, normally `recipe-editor_pgdata`.
  - The operator sets the name in step 2 of `reliability.md` section 7.10, before the first Compose command of the upgrade. `${RECIPE_PGDATA_VOLUME:?}` stops every Compose command without the value.
  - Directly after the edit, the operator reads the name of `pgdata` in `docker compose config`. The operator compares it with the name that `docker inspect` of the running `db` container shows for `/var/lib/postgresql/data`. A difference stops the upgrade.
  - The check also covers a name of an existing empty volume. Without the check, a replacement of `db` on that volume makes a new cluster.
  - A new installation creates its first volume in the host preparation with `docker volume create`. The name has the form of (e).
- (e) A new volume gets the name `recipe-editor_pgdata_<YYYYMMDD_HHMMSS>`. The time is UTC, in the form of the target directory of ADR-0099 (c).
- (f) The switch has these steps:
  1. Run `docker compose stop` and then `docker compose rm -f` for `proxy`, `app`, `backup`, `walarchive`, and `db`. These commands delete containers only. Never use `docker compose down` here.
  2. Run `pg_controldata` on the old volume in a `recipe-pgtools` container with `--network none` and a read-only mount. Use the image of the tag of the old cluster. The state must be `shut down`. Else keep the old name, run `docker compose up -d --no-deps db`, and run step 1 again. Write the old name and its line `Database system identifier` into the change log of the site. Then set `RECIPE_PGDATA_VOLUME` to the new name.
  3. Run `docker compose config`. The volume `pgdata` shows the new name.
  4. Run `pg_controldata` on the volume that step 3 shows in a `recipe-pgtools` container with `--network none` and a read-only mount. The state must be `shut down`. The system identifier must equal the field `system_identifier` of the line of (i) step 10. Else stop, and delete no volume. A switch runs the switch back of (g) at once, and a failed switch back stops as (g) says. The volume then holds no cluster of this run of `restore-volume`.
  5. Run `docker compose up -d --no-deps db`. Then `docker inspect` of the `db` container shows the new volume.
  6. After the start of `app` and `backup`, `docker inspect` of each container shows the new volume.
  - The procedure uses `rm` and `up -d --no-deps` only. It never uses `start`, because `start` reuses the mount of an old container.
- (g) A switch back uses the steps of (f) with the old name. In step 2 of (f), a switch back writes the name that it leaves into the change log. It also writes the identifier when `pg_controldata` reads one. A switch back needs no state of that volume. It skips the else branch of step 2 and never starts `db` on that volume. In step 4 of (f), the system identifier must equal the identifier that step 2 of the first switch wrote.
  - When step 4 of (s) moved files, the new WAL line of (r) starts again. Before that step, `/wal` still holds the line of the old cluster, and (r) does not apply.
  - After step 5 of (f), a switch back starts `walarchive` before `app`. Before step 4 of (s), it runs `docker compose up -d --no-deps walarchive` and the stream check of ADR-0099 (a1). After that step, it runs steps 2 to 7 of (s). Then it runs `docker compose up -d --no-deps app`, then `docker compose up -d --no-deps proxy backup`.
  - The switch does not change the old volume. The switch back loses each write after the switch.
  - When the check of step 4 of (f) fails in a switch back, the operator stops. The operator starts no `db` and deletes no volume. The operator keeps both volumes and the entry of the change log. The site stays down until the operator finds the cause. This is the one case where the procedure ends with `db` stopped.
  - After a major upgrade, the switch back also needs the old `compose.yaml` and the old images. The digest of the `db` image is not a variable. Use the `recipe-pgtools` image of the tag of the old cluster in step 4 of (f).

Restore target (conflicts 3 and 4):

- (h) A new one-shot service `restore-volume` fills a new volume offline.
  - It has the profile `restore`, as `pitr-check` has the profile `pitr`. It is not one of the seven services, and `docker compose up` does not start it.
  - It has `network_mode: none`, the mount `backups:/backups:ro`, the tmpfs `/run/pgpass`, and no secret.
  - It gets the value `COMPOSE_SUBNET` for the client rules of (l). It gets `container.backup_memory` (ADR-0067 (b)) and `container.small_cpus` (ADR-0070 (j)).
  - This item amends ADR-0067 (b) and ADR-0070 (j). Their lists now name the temporary container of `restore-volume`.
  - The operator mounts the target: `docker compose run --rm -v <new volume>:/var/lib/postgresql/data restore-volume <stamp>`.
  - The image is the `recipe-pgtools` image of the tag of the schema of the set (ADR-0083 (a)).
- (i) The script `deploy/pgtools/bin/restore-volume <stamp>` runs these steps:
  1. It refuses a target that is not empty and a stamp outside the grammar of the set names. A live data directory is never empty, so the script cannot write over a cluster.
  2. It opens the existing lock `/backups/.scratch/.lock` read-only and takes `flock` on it. A missing lock file stops the run. It does not run the cleanup of stale runs of `restore-test`.
  3. It runs the gates of steps 1 and 2 of `restore-test`. The gates are the manifest, the role grammar of ADR-0095, the name list, and the refused kinds. It writes nothing to `/backups`.
  4. It runs `initdb` into the target with the arguments of `reliability.md` section 7.1, as the user `postgres` with uid 999. It keeps copies of the three configuration files of (o) and the list of names of (o) in `/run/pgpass`.
  5. It starts a temporary server with `listen_addresses=''` on a private socket. It passes each temporary setting with `pg_ctl -o` and writes none into a configuration file.
  6. It restores the checked globals file. Then it restores the dump with the code comparisons, the counts, and the audit chain check of `restore-test`.
  7. It runs the two `REVOKE` statements of the function of (m). It writes `pg_hba.conf` with the function of (l).
  8. It runs the checks of (m) and (o). A difference ends the run with an `ERROR` line.
  9. It stops the server with `--mode=fast`. `pg_controldata` must show `shut down`. Then it runs `sync`.
  10. It writes one JSON line `restore volume completed` with the name of the set and the field `system_identifier` of `pg_controldata`.
  - A failed step writes one `ERROR` line and ends the script with a code other than 0. The operator then deletes the new volume. Nothing else changed.
- (j) `restore-test` and `restore-volume` share one library of their common steps in `deploy/pgtools/lib/`.
  - `fsync` stays `on` in `restore-volume`. Only `restore-test` turns it off (ADR-0067 (f)).
  - The temporary server uses the sizes `scratch.shared_buffers`, `scratch.maintenance_work_mem`, `scratch.max_connections`, and the two `scratch.max_parallel_*` rows (ADR-0067 (e)).
  - Its `pg_ctl` waits use the new rows `restore_volume.start_timeout` and `restore_volume.stop_timeout`. It uses no `scratch.*` timeout and no TCP port.
  - This item amends ADR-0067: the steps of the restore test also serve the production restore.
- (k) Role source of a production cluster:
  - A production cluster gets its roles from one of two sources. One is `01-roles.sh` on an empty volume of `db`.
  - The other is the checked globals file of a backup set in `restore-volume`. No other path creates a role.
  - A migration never creates a role. This rule of `data-model.md` section 5.1 stays.
  - A restored cluster holds the roles of the set, not the roles of the current `01-roles.sh`. This is the same rule as in `restore-test`.
  - The globals file holds no password, and `restore-volume` holds no secret. `initdb` gives `postgres` a random password, as in `restore-test` (ADR-0067 (c)).
  - After the switch, the operator sets the password of each login role and of `postgres` in `db` from the secret files (ADR-0033).
  - Until then, no role has a known password, so no password login succeeds.
  - This item amends ADR-0095: its check also runs in `restore-volume` directly before psql runs the globals file.
- (l) One shared file in `deploy/db/` holds the seven lines of `pg_hba.conf` of ADR-0069 (r) to (t). It holds a function that writes the file for a value of `COMPOSE_SUBNET`.
  - `compose.yaml` mounts the file read-only into `db`, outside `/docker-entrypoint-initdb.d`. `01-roles.sh` sources it.
  - The `Dockerfile` of `recipe-pgtools` copies the file into the image. `restore-volume` sources it.
  - The build gets `deploy/db` as a second named context (`additional_contexts` of Compose). The context `deploy/pgtools` stays. ADR-0064 (f) applies to both contexts.
  - No second copy of the lines exists. The production scripts hold no test switch, so ADR-0069 (u) stays.
  - This item amends ADR-0069 (r): the lines come from the shared file. It also amends ADR-0069 (v): a cluster of `restore-volume` needs no manual step of an older volume.
- (m) The shared file of (l) also holds a function that prints the two `REVOKE` statements of ADR-0032. `01-roles.sh` and `restore-volume` run its output.
  - No second copy of the statements exists. This item amends ADR-0032, which names `01-roles.sh` as the only place of the statements.
  - A check of `restore-volume` compares these values with the file `restore-roles.expected` of the image. A difference ends the run. A storage test compares that file with a cluster of `01-roles.sh` and all migrations.
    - `pg_hba.conf`
    - the access lists of `postgres`, `template1`, and `recipe`
    - the attributes of each role in `pg_authid`
    - each row of `pg_auth_members`
    - each row of `pg_db_role_setting`
    - the grants of `grants.expected`
  - `restore-roles.expected` holds the lines of `pg_hba.conf` with a placeholder for the subnet. `restore-volume` puts its value `COMPOSE_SUBNET` of (h) into the placeholder before the comparison. The storage test proves the file with a test subnet. `COMPOSE_SUBNET` is already a value of `deploy/.env` (ADR-0062 (i)), so the count of (d) stays at nine.
  - `restore-roles.expected` knows one state of the roles. Each change of a role attribute or a membership in `01-roles.sh` therefore ships with a forward step for existing clusters. A new backup set then holds the new values.
  - Until the first backup set after such an upgrade, a restore uses the `recipe-pgtools` image of the previous release, as in a rollback (q). After such a restore, the operator runs the upgrade to the current release again. That upgrade runs the forward step again, so each forward step must be idempotent. This is a decision of the user of 2026-10-29.
  - A storage test has no cluster of `restore-volume`. The CI job `restore-switch` of the operations change of (z) therefore compares `postgresql.conf` and `postgresql.auto.conf` of a cluster of `restore-volume` with a cluster of `01-roles.sh`. `db` gets `listen_addresses = '*'` only from the sample file of the image.
- (n) After the switch, the entrypoint of `db` finds `PG_VERSION` and skips the init. `01-roles.sh` does not run on a cluster of `restore-volume`.
- (o) The code of the dump runs as the superuser of the target cluster during the restore (ADR-0084, ADR-0095 (h)). That cluster then becomes production. ADR-0084 accepts this risk only for a scratch cluster that the run deletes.
  - In step 8 of (i), `restore-volume` compares `postgresql.conf`, `postgresql.auto.conf`, and `pg_ident.conf` with the copies that step 4 kept in `/run/pgpass`.
  - It also compares the list of names at the top of the data directory with the list after `initdb`. The files `postmaster.pid` and `postmaster.opts` of the running server do not count.
  - A difference ends the run with an `ERROR` line. The checks of (m) also run in step 8.
  - A change inside the catalog that these comparisons cannot find stays a residual risk. The user accepts this risk in this ADR. This is a decision of the user of 2026-10-29.
  - A later ADR may restore `pre-data` as a role that is not a superuser (ADR-0084 (e)). That ADR can close the residual risk.

Major upgrade and rollback:

- (p) A major upgrade runs `restore-volume` with the `recipe-pgtools` image of the new release. That image builds on the `db` image of the new major version.
  - The command `docker compose up -d --no-deps db` of (f) changes the image and the volume of `db` in one replacement.
  - The release that changes the major version of `db` holds no new migration. The schema of the set then equals the schema of that release (ADR-0083 (a)).
  - The release notes of that release state that it holds no migration file. A CI step compares the list of `migrations/` with the previous tag and fails on a new file.
  - The CI step ships with the operations change of (z).
  - The storage test of that release runs on the new major version and proves its `restore-code.expected` (ADR-0067 (j), ADR-0101).
  - A dump of the new major version does not restore into the old one. The rollback after the switch is therefore the switch back of (g), and it loses each write after the switch.
  - The old base backups need the binaries of the old major version. Keep the old `recipe-pgtools` image while the moved files of (s) exist.
  - A CI proof of the major upgrade with an image of a newer major version is out of scope. A later change adds it.
- (q) The rollback with the `_premigrate` dump runs `restore-volume` with the previous tag, the tag of the schema of the set.
  - `deploy/.env` holds the previous tag before step 4 of (b). `walarchive` and `backup` therefore start with the previous image.
  - `walarchive` starts in step 7 of (b), before `app`. This item amends the order of ADR-0083 (d), where `walarchive` gets the previous image after `app`.
  - The image of 0.1.0 has no `restore-volume`. The image of a newer release has the wrong `restore-code.expected` for a set of 0.1.0. A rollback restore to 0.1.0 therefore has no procedure. The project accepts this limit.

WAL line after a switch (conflict 5):

- (r) Every switch starts a new WAL line before `app` starts. This rule covers the restore, the rollback restore, the major upgrade, and a switch back after step 4 of (s) (g).
  - The first base backup of the new line exists before the first write of `app`.
- (s) The operator runs these steps after step 6 of (b) and before the start of `app`:
  1. The switch of (f) already stopped `walarchive` and deleted its container.
  2. Create the target directory and check one filesystem, as steps 3 and 4 of ADR-0099 (a). The target directory follows ADR-0099 (c).
  3. Copy the line `Database system identifier` of the cluster that the switch leaves from the change log (step 2 of (f)) into the file `old-system-identifier` in the target directory.
  4. Move the segment files, the `.partial` file, and `/wal/.system-identifier`, as steps 5 to 7 of ADR-0099 (a).
  5. Also move each directory of `/wal/base` that matches `^base_[0-9]{8}_[0-9]{6}$` into the directory `base` of the target directory.
  6. Run step 8 of ADR-0099 (a). The command starts `walarchive` with the image of the tag in `deploy/.env`.
  7. Run steps 9 to 13 of ADR-0099 (a). They are the stream check, the base backup, the segment switch, the flush check, and the WAL check.
  - After step 6, `walarchive-entry` and `basebackup-once` read `/wal/.system-identifier`. When the file is missing and no segment exists, they write the identifier of `IDENTIFY_SYSTEM` into it. When the file is missing and a segment exists, they refuse to start. They also refuse to start when the file holds another identifier.
  - The scripts get this guard in the operations change of (z).
  - An existing installation has segments and no file. The operator writes the file once, before `walarchive` first starts with the image of the first release with the guard.
  - For this step, the operator reads the system identifier in `db` and pipes it into the old `walarchive` container. That container writes `/wal/.system-identifier`. The release notes of that release name this step.
  - The step runs only when `/wal/.system-identifier` is missing.
  - Before the write, the container compares the value with `xlp_sysid` of the newest complete segment in `/wal`. That value is 8 bytes at offset 24 of the first page, the long page header, little-endian.
  - A difference stops the step, and the step writes no file. The step therefore checks the identifier and does not trust the volume name of (d).
  - "A gap in the WAL stream" of ADR-0099 (a) keeps `/wal/.system-identifier` in `/wal`. The cluster does not change in a gap, so its identifier stays the same. Only step 4 of (s) moves the file.
  - This item amends step 7 of ADR-0099 (a). The file `/wal/.system-identifier` also stays in `/wal` in that step.
  - The marker file, `/wal/.lock`, `/wal/.pitr-scratch`, and the empty directory `/wal/base` stay. A directory `.tmp-base_*` of a killed run stays under ADR-0077 (h).
  - Step 14 of ADR-0099 (a) runs after the start of `app`, because the job `alerts` runs in `app`.
- (t) The operator deletes the old `pgdata` volume and the target directory of (s) only when three conditions hold:
  1. A `restore-test` passed on a set of the new cluster.
  2. The flush check and the WAL check of ADR-0099 (a) passed on the new WAL line.
  3. The switch is older than `volume_switch.keep_min_age`.
  - No script and no retention deletes them. This item amends ADR-0099 (c) for the target directory of a switch. The delete rule of ADR-0099 (c) stays for the target directory of a gap.
  - The conditions need no `pitr-check`. `pitr-check` keeps its gate `PITR_CI=1` (ADR-0077). CI proves the recovery procedure.
- (u) After a switch, `/wal/base` holds no base backup of the old cluster. The case of ADR-0077 (f) "the old base backups can name higher segments" no longer occurs.
  - The rule of ADR-0077 (f) for `pg_archivecleanup` stays. This item amends the consequence that ADR-0077 (f) states.
- (v) `restore_command` reads `/wal` only (`reliability.md` section 7.7). The moved files are therefore no recovery source for the running stack.
  - A recovery of the old cluster needs a procedure that reads the target directory in a separate container. Never move the files back into `/wal` while `walarchive` writes there.
  - This item amends the sentence of ADR-0099 (c) "the moved segments keep the older base backups recoverable". That sentence holds only for such a separate procedure.
- (w) The backup sets of the old cluster stay on the backup disk. `backup.retention` deletes them.
  - A set of the same major version restores into the new cluster.
  - After the switch, `backup-now` and `restore-test` prove the first set of the new cluster.

Recovery from the WAL archive:

- (x) A recovery of production from the WAL archive is out of scope. This ADR does not block it.
  - A later ADR can add a one-shot service that fills a new volume from `/wal`. That ADR reuses (a), (c) to (g), and (r) to (t).

Settings:

- (y) `reliability.md` section 7.11 gets three rows:
  - `restore_volume.start_timeout` 60 s. It equals `scratch.restore_start_timeout`, because the server also starts on a new cluster without a recovery.
  - `restore_volume.stop_timeout` 600 s. The fast stop with `fsync` on writes the shutdown checkpoint of the restored data. The value equals `scratch.pitr_start_timeout`, the longest `pg_ctl` wait of section 7.11. The wait stays inside the recovery time objective of 1 h (`reliability.md` 7.7).
  - `volume_switch.keep_min_age` 30 days. It equals `backup.retention`, so the sets of the old cluster expire with it. It is above 4 weekly base backups of `basebackup.retention`, 28 days.

Changes of other documents:

- (z) The follow-up edits of this ADR are:
  - The Status lines of ADR-0020, ADR-0032, ADR-0062, ADR-0067, ADR-0069, ADR-0070, ADR-0077, ADR-0083, ADR-0095, and ADR-0099 get "Amended by ADR-0103" with the scope. No other line of these ADRs changes.
  - `docs/adr/README.md`: a new index row, the same notes in the Status cells, and the decision map.
  - `reliability.md` sections 7.1, 7.7, 7.8, 7.10, and 7.11. `data-model.md` section 5.1. `.claude/rules/sql.md`, the line of the role source.
  - `docs/ops/runbook.md`: a first line in the sections "Restore into production", "Rollback", and "Major upgrade", the citation of the new-volume rule, and "Known gaps".
  - A later OpenSpec change of the capability `operations` changes `compose.yaml`, `deploy/.env.example`, the pgtools scripts, the tests, the spec, and the runbook procedures.
  - That change also adds the CI step of (p) and the CI job `restore-switch` of (m).
  - In that change, the harnesses of ADR-0101 that mount `01-roles.sh` also mount the shared file of (l). They are `crates/recipe-db/tests/storage/support/container.rs` and `crates/recipe-server/tests/support/database.rs`.
  - In that change, the `deploy_values` checks that read the `printf` lines of `01-roles.sh` read the shared file (`crates/recipe-server/tests/deploy_values/secrets.rs`).
  - The mount of (l) changes the service configuration of `db`. In the first release with that mount, step 3 command 1 of `reliability.md` section 7.10 therefore replaces `db` once on the same volume. `app` loses its connections for the start time of `db`. This is a decision of the user of 2026-10-30.
  - In that change, the CI job `upgrade` no longer requires the same `db` container. It checks that the new `db` container mounts the same volume and that `pg_control_system()` returns the same `system_identifier` as before the upgrade.

## Consequences

### Positive

- A wrong volume name fails closed. `down -v` cannot delete `pgdata`.
- The old cluster stays live and unchanged until the switch. No `DROP DATABASE` is necessary.
- The production restore uses the gates of the weekly restore test.
- A restored cluster never serves a connection under the client rules of the image.
- Each write of the new cluster reaches the WAL archive. Two systems never mix in `/wal`.
- One switch serves the restore, the rollback, the major upgrade, and a later recovery from the WAL archive.

### Negative

- The host preparation creates the volume, and `deploy/.env` holds nine values.
- The downtime of a restore includes the whole run of `restore-volume`, the new WAL line, and its first base backup.
- The old volume and the moved WAL files use disk space for at least `volume_switch.keep_min_age`.
- A rollback restore to 0.1.0 has no procedure.
- The release that changes the major version of `db` holds no migration file (p).
- An existing installation writes `/wal/.system-identifier` once, before the guard of (s) first runs.
- The first upgrade to the release with the mount of (l) replaces `db` once (z).

### Risks and mitigations

- Risk: a typo in `-v <new volume>` or a missing `-v` option fills a stray volume or an anonymous volume. Mitigation: step 4 of (f) compares the system identifier before `db` starts.
- Risk: the operator keeps an old `app` or `backup` container, and a later `start` reuses the old mount. Mitigation: the switch uses `rm` and `up -d --no-deps` only, and step 6 of (f) checks the mounts.
- Risk: the operator sets a wrong existing volume, for example the old one. Mitigation: step 4 of (f), the change log of the site, and the `docker inspect` checks of (f).
- Risk: the operator sets a wrong existing volume at an upgrade. Mitigation: the comparison of (d) stops the upgrade, and the segment check of (s) writes no identifier file of another system.
- Risk: a reboot during the switch. Mitigation: the deleted containers do not come back. The new `db` starts on the new volume, or no `db` container exists. Both states are safe.
- Risk: the operator deletes the old volume too early. Mitigation: the three conditions of (t).
- Risk: a check of (f) fails after step 1, and a clean-up deletes the old volume. Mitigation: step 4 of (f) deletes no volume and runs the switch back of (g) at once.
- Risk: the code of the dump changes the future production cluster. Mitigation: the checks of (m) and (o). The user accepts the residual risk of a catalog change that the checks cannot find (o).

## Related

- Amends ADR-0020 (every restore goes into a new volume), ADR-0032 (a function of the shared file prints the `REVOKE` statements), ADR-0062 (i) (nine values).
- Amends ADR-0067 (b) and the steps of the restore test (shared steps with `restore-volume`), ADR-0069 (r), (v) (shared file of the client rules).
- Amends ADR-0070 (j) (the temporary container of `restore-volume` in `container.small_cpus`), ADR-0077 (f) (old base backups move with the line).
- Amends ADR-0083 (d) (`walarchive` and `backup` start before `app` after a rollback restore), ADR-0095 (the check also in `restore-volume`).
- Amends ADR-0099 (a) (the identifier file stays in a gap) and ADR-0099 (c) (delete rule after a switch).
- ADR-0033 (passwords from the secret files), ADR-0064 (f) (build contexts), ADR-0068 (WAL archive), ADR-0084 (e) (residual risk of `pre-data`).
- ADR-0100 and ADR-0102 (end of a restore), ADR-0101 (storage tests).
- `docs/architecture/reliability.md` sections 7.1, 7.7, 7.8, 7.10, 7.11. `docs/architecture/data-model.md` section 5.1.
- `docs/ops/runbook.md` sections "Restore into production", "Rollback", "Major upgrade", and "Known gaps".
- Baseline requirements: `operations` "Pinned database image", "Roles from the database init script", "Restore test", and "Continuous WAL archive". The later operations change adds the delta.
- `overview.md` section 7: R31, R34. Section 9: A35.
