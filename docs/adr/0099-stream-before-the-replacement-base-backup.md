# ADR-0099: Stream before the replacement base backup after a WAL gap

Status: Accepted. Amended by ADR-0103 (item (c): the delete rule after a switch, moved files are a recovery source only for a separate procedure)
Date: 2026-10-26
Deciders: project owner

## Context

ADR-0068 (e) gives the procedure after a gap in the WAL stream. An outage of `walarchive` longer than `pg.wal_keep_size` leaves the gap. `reliability.md` section 7.7 and the runbook section "A gap in the WAL stream" use the same order:

1. Stop `walarchive`.
2. Move the old segments aside.
3. Take a replacement base backup with `basebackup-once`.
4. Start the stream again.

The review of step P4.1 found a silent loss of the recovery point in this order:

- `pg_receivewal` uses no replication slot (ADR-0068 (b)). With no segment in its directory, it starts at the segment of the current flush position of the server.
- The base backup with `--wal-method=fetch` holds WAL up to its end segment N. The server then writes segment N+1.
- When the server passes into segment N+2 before step 4, segment N+1 exists nowhere. The stream starts at N+2 or later.
- A recovery from the replacement base backup then stops at the end of segment N.
- The alert `replication` stays inactive, because the stream runs. The loss stays silent until the next weekly base backup.

The documents also leave two points open:

- No document names the target directory of the moved segments. The runbook uses the placeholder `<moved directory>`.
- The runbook selects segment files with a glob of 24 characters. No document says where this name form comes from.

## Decision drivers

- Recovery point objective: minutes (ADR-0020, R31). Data safety comes before the time of the procedure.
- A gap in the archive never stays silent.
- The procedure uses the existing scripts and tools. It adds no setting and no number to `reliability.md` section 7.11.
- A move of a segment is a rename on one filesystem, never a copy.
- No base backup and no WAL segment disappears by hand before the retention of ADR-0068 (i), (j) would delete it.

## Considered options

### Option A: Start the stream first, then take the base backup, then check its WAL in the archive

- Pros: the stream holds every segment from its start on. The base backup starts after the stream, so its WAL is in the archive. The existing tool `pg_verifybackup` proves this.
- Cons: three more operator steps: the stream check, a segment switch, and the WAL check.

### Option B: A temporary replication slot during the procedure

- Pros: the server keeps every segment until the stream receives it.
- Cons: ADR-0068 (b) uses no slot. A slot that stays after a failed procedure can fill the disk of `pgdata`.

### Option C: Keep the order and take a second base backup after the stream starts

- Pros: the second base backup has no gap after it.
- Cons: two base backups need twice the time and the space. The first base backup has no use.

## Decision

The project uses **Option A**. This decision amends ADR-0068 (e).

Order of the procedure:

- (a) The runbook section "A gap in the WAL stream" uses these steps:
  1. Run `docker compose exec backup restore-test`. Continue only after it passes.
  2. Run `docker compose stop walarchive`.
  3. Create the target directory of (c) with `mkdir`. The command fails on an existing name.
  4. Run `stat -c %d` on `RECIPE_WALARCHIVE_DIR` and on the target directory. A different number stops the procedure before any move.
  5. List the segment files and the `.partial` file at the top of `RECIPE_WALARCHIVE_DIR`. Use the name form of (d).
  6. Move the files of the list into the target directory with `mv -t`.
  7. Run `ls -A "$RECIPE_WALARCHIVE_DIR"`. No segment file stays. The marker file, `.lock`, `base`, and `.pitr-scratch` stay.
  8. Run `docker compose up -d --no-deps walarchive`. The stream starts on a directory without segments.
  9. Run the stream check of (a1). Take no base backup before it passes.
  10. Run `docker compose exec walarchive basebackup-once`. The output holds the line `base backup completed`. Note its field `base`.
  11. Run the segment switch of (a2). Note the returned position.
  12. Run the flush check of (a2) until it returns `t`.
  13. Run the WAL check of (a3). It passes on the exit code 0.
  14. Wait until the alert `replication` is inactive.
- (a1) The stream check reads `pg_stat_replication` through the socket login of ADR-0077 (b):

  ```bash
  docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT state, flush_lsn FROM pg_stat_replication WHERE usename = 'recipe_replicator' AND application_name = 'walarchive'"
  ```

  - The check passes when the result has exactly one row, the state `streaming`, and a `flush_lsn`.
  - This is the row of the alert `replication` (ADR-0068 (p)). The row of a running base backup has another `application_name`.
- (a2) The segment switch and the flush check use the same socket login:
  - The switch is `SELECT pg_switch_wal()`. The server then closes the segment that holds the end of the base backup.
  - The flush check is the query of CI step 7 (ADR-0077 (c)) with the returned position:

  ```sql
  SELECT EXISTS (SELECT 1 FROM pg_stat_replication WHERE application_name = 'walarchive' AND usename = 'recipe_replicator' AND flush_lsn >= '<position>'::pg_lsn)
  ```

  - The value `t` shows that the archive holds the end segment of the base backup as a complete file. `pg_waldump` does not read a `.partial` file.
  - The step has no time limit and no poll interval. The operator runs the query again by hand.
- (a3) The WAL check is the existing option `--wal-directory` of `pg_verifybackup`:

  ```bash
  docker compose exec walarchive pg_verifybackup --quiet --skip-checksums --wal-directory=/wal /wal/base/<base>
  ```

  - `pg_verifybackup` reads the WAL range of the `backup_manifest` from `/wal` with `pg_waldump`. A missing or unreadable segment of that range fails the check.
  - `--skip-checksums` is safe here. `basebackup-once` checked the checksums of the same files before the rename (ADR-0068 (g)).
  - The image `recipe-pgtools` comes from `postgres:16` and holds `pg_waldump`. The check needs no new script and no new subcommand. A new script needs its own ADR.

Why the order closes the gap:

- (b) The new order closes the gap of the old order:
  - `pg_receivewal` starts in step 8 at the start of segment S. S holds the flush position of the server at that time (PostgreSQL 16 documentation of `pg_receivewal`, starting point rule 3).
  - Step 9 proves that the stream runs before the base backup starts.
  - The base backup starts in step 10 with a new checkpoint. Its start location lies after the flush position of step 8, so its start segment is S or later.
  - While the stream runs, `pg_receivewal --synchronous` writes each segment in order. The archive therefore holds every segment from S on.
  - A segment that the server starts during the base backup reaches the archive like every other segment. The stream does not wait for the end of the base backup.
  - ADR-0068 (h) still applies. `fetch` uses one sender, and the WAL of the base backup stays in the server through `pg.wal_keep_size`.
  - The weekly run of `basebackup-once` already runs next to a running stream. Step 10 is the same situation.
- (b1) The stream can fail to start, for example without the marker file, without the secret, or without `db`:
  - Step 9 then finds no row or another state. The operator takes no base backup.
  - The operator reads `docker compose logs walarchive`, corrects the cause, and runs step 9 again.
  - The old segments stay in the target directory. The procedure loses no segment while it waits.
  - The order stays valid after any wait. The stream always starts at the current position, and the base backup always starts after it.
- (b2) The stream can stop after step 9:
  - `restart: always` starts `walarchive` again (ADR-0068 (c)). `pg_receivewal` then continues after the newest complete segment in its directory.
  - A segment that the server no longer holds stops `pg_receivewal` again. The alert `replication` becomes active.
  - A missing segment inside the range of the base backup fails the WAL check of step 13.
- (b3) A failed WAL check means that the archive misses WAL of the replacement base backup. The operator runs the procedure again from step 2 with a new target directory.

Target directory of the moved segments:

- (c) The target directory is a sibling of the archive directory on the host:

  ```bash
  target="${RECIPE_WALARCHIVE_DIR%/}.moved-$(date -u +%Y%m%d_%H%M%S)"
  mkdir -m 0700 -- "$target"
  ```

  - The name comes from the setting `RECIPE_WALARCHIVE_DIR` of the environment file and the literal suffix `.moved-<YYYYMMDD_HHMMSS>` of the runbook. The time is UTC, in the form of the names `base_<YYYYMMDD_HHMMSS>` (ADR-0077 (d)).
  - The directory lies outside the directory that `pg_receivewal` writes to. No container mounts it, so no script reads it or deletes in it.
  - The runbook section "Create the environment file" sets `RECIPE_WALARCHIVE_DIR` to a directory on the second disk. That directory must not be the mount point of the second disk. Its parent directory is then on the second disk too.
  - Step 4 proves that the two paths are on one filesystem. `mv` then renames each file and copies no data.
  - The operator deletes the target directory, not a script. The delete comes after the retention of `basebackup-once` deletes the last base backup older than the replacement base backup.
  - The operator finds this state when `/wal/base` holds no name of the grammar `^base_[0-9]{8}_[0-9]{6}$` lower than `<base>` in byte order.
  - Until then, the moved segments keep the older base backups recoverable up to the start of the gap. The space use equals the normal retention of ADR-0068 (j).

Names of the segment files:

- (d) The names of the WAL segment files follow the PostgreSQL naming of section 30.6 "WAL Internals" of the PostgreSQL 16 documentation.
  - Section 30.6 gives the first name `000000010000000000000001`. The macro `XLogFileName` of `src/include/access/xlog_internal.h` writes the timeline, the log number, and the segment number as 8 upper-case hex characters each.
  - `pg_receivewal` adds the suffix `.partial` to the segment that it writes. Every other name at the top of `RECIPE_WALARCHIVE_DIR` stays in place.

Changes of other documents:

- (e) The follow-up edits of this ADR are:
  - `docs/adr/0068-wal-archive-and-base-backups.md`: the Status line gets "Amended by ADR-0099 (order of the gap procedure, target directory of the moved segments)". No other line changes.
  - `docs/adr/README.md`: a new index row for ADR-0099 and the same note in the Status cell of row 0068.
  - `docs/adr/README.md` decision map, row "Deployment, migrations, backup, panic policy, health": the text "Also 0068 (WAL archive)" names the place.
  - After the item of 0077 in that place, add "Also 0099 (stream before the replacement base backup, target of the moved segments)".
  - `docs/architecture/reliability.md` section 7.7, the bullet of the gap: the new order of (a) in one or two sentences, with the citation "ADR-0068 (e), ADR-0099".
  - `docs/ops/runbook.md` section "A gap in the WAL stream": the 14 steps of (a) and the commands of (a1) to (a3).
  - The same section also gets the target directory of (c), the sentence of (d), and the delete rule of (c). The placeholder `<moved directory>` goes away.
  - `docs/ops/runbook.md` section "Create the environment file", step 3: the sentence of (c) about the mount point.
  - No number of `reliability.md` section 7.11 changes. No spec of `openspec/specs/` changes, because the requirements "Continuous WAL archive" and "Point-in-time recovery from the WAL archive" stay true.
  - ADR-0083 (e) cites ADR-0068 (e) for an outage shorter than `pg.wal_keep_size`. That text stays true and does not change.
  - No script changes, so no test of `deploy/pgtools/tests` changes. No automatic test runs the procedure. `ste-lint.py` of CI checks the text of the changed files. A manual run of the procedure on a test stack can follow when the user permits Docker.

## Consequences

### Positive

- After the procedure, the archive holds every segment from the start of the replacement base backup on.
- Step 13 proves this with an existing tool. A gap of the procedure no longer stays silent.
- The target directory has one name form, one disk, and one delete rule.
- No setting and no number of section 7.11 changes.

### Negative

- The procedure has 14 steps instead of 11.
- The archive holds a few segments before the start of the replacement base backup. The retention deletes them after the older base backups.
- The target directory keeps its space until the retention deletes the last older base backup.

### Risks and mitigations

- Risk: the weekly run of `basebackup-once` holds `/wal/.lock` during step 10. Mitigation: the run of step 10 writes "another base backup run holds the lock". The operator runs step 10 again after the weekly run ends.
- Risk: `RECIPE_WALARCHIVE_DIR` is the mount point of the second disk on an older host. Step 4 then stops the procedure. Mitigation: the runbook section "Create the environment file" states the rule of (c). A change of the directory of the volume is outside the scope of this ADR.
- Risk: the operator deletes the target directory too early. Then the older base backups lose their WAL. Mitigation: the delete rule of (c) names a check of the names in `/wal/base`.
- Risk: the flush check of step 12 never returns `t`. Mitigation: the alert `replication` becomes active, and step 13 fails. The operator follows (b2) and (b3).

## Related

- Amends ADR-0068 (e) (order of the gap procedure, target directory of the moved segments).
- ADR-0020 (recovery point), ADR-0066 (script rules), ADR-0068 (b), (c), (g), (h), (j), (p), ADR-0074 (marker file), ADR-0083 (e).
- ADR-0077 (b) (socket login), (c) (flush check), (d) (disk layout).
- `docs/architecture/reliability.md` section 7.6 (`pg.wal_keep_size`, senders) and section 7.7 (WAL archive).
- `docs/ops/runbook.md` sections "A gap in the WAL stream" and "Create the environment file".
- PostgreSQL 16 documentation: `pg_receivewal` (starting point), `pg_verifybackup` (options `--wal-directory` and `--skip-checksums`), section 9.27.3 (`pg_switch_wal`), section 30.6 (WAL Internals).
- Source `src/include/access/xlog_internal.h` of `REL_16_STABLE` (macro `XLogFileName`).
- Baseline requirements: `operations` "Continuous WAL archive" and "Point-in-time recovery from the WAL archive". The change adds no delta.
- `overview.md` section 7: R31. Section 9: A35.
