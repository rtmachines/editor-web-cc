# ADR-0077: Marker entry of the recovery check, WAL cutoff, and scratch constants

Status: Accepted. Amended by a note on (g). The row `pitr_check.ready_poll` is the connection check of the scratch cluster, not the archive poll that goes away (`reliability.md` 7.11). Amended by ADR-0103 (item (f): after a switch, the old base backups move with the old WAL line)
Date: 2026-10-24
Deciders: project owner

## Context

Step P4.1 writes the scripts `basebackup-once` and `pitr-check` of ADR-0068 and the script `restore-test` of ADR-0067. The review of these scripts found five points that the documents leave open or get wrong:

1. ADR-0068 (m) lets `pitr-check` commit an audit entry. The script writes a raw `audit_log` row with an unknown action and no actor. No command writes this row. `data-model.md` section 10 permits no such action, and an unknown action breaks each audit list page. `audit_log` is append-only, so the row stays forever.
2. ADR-0068 (m) lets `pitr-check` take its own base backup. The requirement "Point-in-time recovery from the WAL archive" asks for a recovery from the newest base backup of the service `walarchive`. No CI step restores a base backup that `basebackup-once` keeps.
3. ADR-0068 (j) gives `pg_archivecleanup` the start segment of the oldest kept base backup by name. The order of the names and the order of the WAL can differ, for example after a restore into a new cluster. `pg_archivecleanup` then deletes WAL that a kept base backup needs.
4. Seven numeric constants of `pitr-check` and four of `restore-test` have no row in `reliability.md` section 7.11.
5. A directory `.tmp-base_*` of a killed run of `basebackup-once` stays. No document names it, and no runbook step deletes it.

## Decision drivers

- The project gives data safety priority over performance. No delete takes WAL that a kept base backup needs (R31).
- Every audit entry comes from a command through `UnitOfWork::commit` with a real actor (R29).
- A failed step writes one `ERROR` line and stops (R35).
- A name on a disk and a value from the environment are external input. Each one has one anchored grammar (R38).
- CI proves the recovery from a base backup that the plant keeps (R31, A35).
- Every numeric setting of a script of `recipe-pgtools` comes from `reliability.md` section 7.11.

## Considered options

### Option A: A command of the application writes the marker, and `pitr-check` recovers from a copy of the newest kept base backup

- Pros: the marker is a normal audit entry with an actor and a known action. `pitr-check` needs no secret and no connection to `db`. CI proves the base backup that the plant keeps.
- Cons: the CI job gets more steps. The copy needs free space on the WAL archive disk.

### Option B: A new audit action for `pitr-check` with no actor

- Pros: the script stays one step of CI.
- Cons: a new action, a new case of the actor rule, and a change of `AuditAction`. The script needs the password of `recipe_app`. The production image holds a script that writes into `audit_log`.

### Option C: CI writes the marker, and `pitr-check` keeps its own base backup

- Pros: `pitr-check` needs no copy.
- Cons: `pitr-check` still needs the password of `recipe_replicator`. No CI step proves a base backup that `basebackup-once` keeps.

## Decision

The project uses **Option A**.

Marker entry (user decision):

- (a) CI step 7 writes the marker entry with the command `ChangeOwnPassword` at `POST /me/password` over HTTP. The command uses the session of the administrator of CI step 5. This item amends ADR-0068 (m).
  - `create-admin` sets `must_change_password` (`domain-model.md` section 2.3). That session can open only `GET /me/password`, `POST /me/password`, `POST /logout`, and `POST /lang` (`api.md` 7.4).
  - The command commits one audit entry `OwnPasswordChanged` through `UnitOfWork::commit` (`domain-model.md` section 5, `data-model.md` section 6, row `ChangeOwnPassword`).
  - The request carries the CSRF token of the session and the `Origin` header of `api.md` 7.3. The new password comes from `openssl rand -hex 32`, as in CI step 2.
  - A request without `HX-Request` gets `303` to `/recipes` with a new session cookie. Every other answer stops the CI step.

Identity of the marker:

- (b) After the command, CI reads the newest row of `audit_log` with `docker compose exec -T -u postgres db psql -d recipe`.
  - The socket line `local all postgres peer` of `data-model.md` 5.1 and ADR-0069 (r) admits this login. Only the system user `postgres` in `db` passes.
  - CI reads `chain_seq` and `entry_hash` as lower-case hex of the row with the highest `chain_seq`. CI requires the action `OwnPasswordChanged` of that row.
  - CI gives the values to `pitr-check` in the environment values `PITR_MARKER_SEQ` and `PITR_MARKER_HASH`.
  - `pitr-check` accepts `PITR_MARKER_SEQ` only by `^[1-9][0-9]{0,18}$` and `PITR_MARKER_HASH` only by `^[0-9a-f]{64}$`. A bad value stops the script before it writes a file.

Wait for the archive:

- (c) Through the same socket login, CI reads `pg_current_wal_lsn()` after the commit. Then CI reads the row of `pg_stat_replication` with `application_name = 'walarchive'` until its `flush_lsn` is at or after that position.
  - The source `receivelog.c` of `REL_16_STABLE` reports the flush position without a replication slot only with `--synchronous`. ADR-0068 (a) sets this option.
  - The documentation of `pg_receivewal` 16 says that `--synchronous` sends a status packet back to the server immediately after each flush.
  - CI forces no segment switch, so the marker stays in the partial last segment. CI checks that `walarchive` holds the segment of `pg_walfile_name(pg_current_wal_lsn() - 1)` only as a `.partial` file.
  - The recovery of (e) therefore proves the fallback to `.partial` of ADR-0068 (l).
  - The wait reads once each second and stops after 60 s with a failed CI step. These values are bounds of the CI job, as the poll of 60 s of CI step 4. No image and no service of the plant uses them. Section 7.11 therefore gets no row for them.

Recovery from the newest base backup (user decision):

- (d) CI runs `basebackup-once`, then the marker command, then `pitr-check`. This item amends ADR-0068 (m).
  - `pitr-check` opens no connection to `db`, writes nothing to `db`, and mounts no secret. Its temporary container joins no network.
  - `pitr-check` keeps the gate `PITR_CI=1`, because its copy uses free space on the WAL archive disk. A full disk stops `pg_receivewal`.
  - `pitr-check` takes `flock -n` on `/wal/.lock` before it selects a base backup, and it holds the lock until it exits. `basebackup-once` renames into `/wal/base` and deletes in `/wal` only under this lock. The copy and the recovery therefore see one fixed set of files.
  - A run that does not get the lock writes one `ERROR` line and copies nothing.
  - `pitr-check` selects the newest base backup by the grammar below. It copies that directory into `/wal/.pitr-scratch/run.XXXXXX` and runs `pg_verifybackup` on the copy.
  - The scratch cluster starts from the copy with the `restore_command` of ADR-0068 (l). `pitr-check` never changes `/wal/base` or a WAL file.
  - The copy needs free space on the WAL archive disk for the newest base backup. The scratch cluster also writes WAL during the recovery.
- No document gave the layout of the WAL archive disk. This ADR records it:
  - The WAL segments of `pg_receivewal` lie at the top of `/wal`.
  - A base backup is the directory `/wal/base/base_<YYYYMMDD_HHMMSS>` in UTC. A run of `basebackup-once` writes it first under the temp name `/wal/base/.tmp-base_<YYYYMMDD_HHMMSS>`.
  - A reader finds a base backup only by the anchored grammar `^base_[0-9]{8}_[0-9]{6}$` of the whole name. A shell script sets `LC_ALL=C` for the match. A link or a file with this name is not a base backup.
  - The newest base backup is the last name of the grammar in byte order.
  - The marker file `/wal/.walarchive-marker`, the lock file `/wal/.lock`, and the scratch directory `/wal/.pitr-scratch` always stay.

Checks of the recovery:

- (e) The checks of ADR-0068 (n) run after the end of the recovery. They use the private socket of the scratch cluster.
  - The recovery is complete: `pg_is_in_recovery()` gives `false`.
  - Exactly one row of `audit_log` has the `chain_seq` of `PITR_MARKER_SEQ` and the `entry_hash` of `PITR_MARKER_HASH`.
  - The audit chain check of `data-model.md` section 4.9 over the whole table gives no break, and the head matches the last row.

WAL cutoff:

- (f) `basebackup-once` gives `pg_archivecleanup` the lowest start segment of all kept base backups. This item amends ADR-0068 (j).
  - The start segment of a base backup is the file name in the line `START WAL LOCATION` of its `backup_label`.
  - The comparison uses the 16 hex characters after the 8 characters of the timeline. `pg_archivecleanup.c` of `REL_16_STABLE` compares `walfile + 8` with the cutoff `+ 8` by `strcmp`.
  - A kept base backup without a readable start segment stops the run before any delete.
  - After a restore into a new cluster, the old base backups can name higher segments than the new one. The lowest start segment then keeps every segment of the new cluster.

Constants of the scratch clusters (user decision):

- (g) These constants get rows in `reliability.md` section 7.11:
  - `scratch.port` 5433 and `scratch.stop_timeout` 60 s, for the scratch servers of `restore-test` and `pitr-check`.
  - `scratch.restore_start_timeout` 60 s, for the `pg_ctl start` of `restore-test`.
  - `scratch.pitr_start_timeout` 600 s, for the `pg_ctl start` of `pitr-check` with its recovery. It stays inside the recovery time objective of 1 hour.
  - `restore_test.detail_length` 160 characters of a tool message in an `ERROR` line of `restore-test`.
  - `pitr_check.log_tail` 20 lines of the server log after a failed start of the scratch cluster of `pitr-check`.
- These values of `pitr-check` go away in the new design:
  - The wait of 60 s and the poll of 1 s for the archive. CI waits for the flush of (c) instead.
  - The 8 bytes of WAL at the end of the commit record. The flush position of (c) replaces the read of the WAL file.
  - The position `pg_current_wal_lsn() - 1` of the script, for the same reason.

Runbook:

- (h) A directory `.tmp-base_*` of a killed run stays in `/wal/base`. The operator deletes it when no `basebackup-once` runs.

## Consequences

### Positive

- The audit log holds no row without a command and no unknown action.
- CI proves a recovery from a base backup that `basebackup-once` keeps, with the fallback to `.partial`.
- No delete of `pg_archivecleanup` takes WAL of a kept base backup.
- Each numeric constant of the two scripts has one row in section 7.11.

### Negative

- The CI job gets three more reads of `db` and one more request to `app`.
- The copy of a base backup needs free space on the WAL archive disk.
- A base backup of an old cluster can keep old segments longer, because the cutoff uses the lowest start segment.

### Risks and mitigations

- Risk: the session of CI step 5 expires before step 7. Mitigation: the command gets `E-WEB-004`, and the CI step stops with the code in its log.
- Risk: the stack of CI is in the read-only mode of `reliability.md` 7.8. Mitigation: the command gets `503 E-WEB-006`, and the CI step stops with the code in its log.
- Risk: `pitr-check` cannot prove that the marker lies after the start of the base backup. Mitigation: the order of CI step 7 puts the marker after `basebackup-once`.
- Risk: a killed run of `pitr-check` leaves a directory `run.*` in `.pitr-scratch`. Mitigation: `pitr-check` runs only in CI, and the CI disk images are temporary.

## Related

- Amends ADR-0068 (j) (the WAL cutoff) and (m) (the marker entry and the base backup of `pitr-check`).
- ADR-0031 (secret files), ADR-0035 (`ChangeOwnPassword`), ADR-0066 (script rules), ADR-0067 (scratch cluster), ADR-0069 (r) (socket line), ADR-0070 (CI job), ADR-0074 (names on the backup disks).
- `docs/architecture/reliability.md` section 7.7 (layout, cutoff, recovery check, runbook step) and section 7.11 (the new rows of (g)).
- `data-model.md` sections 4.9, 5.1, and 6. `api.md` 7.3 and 7.4.
- PostgreSQL 16 documentation of `pg_receivewal` (option `--synchronous`). Source `src/bin/pg_basebackup/receivelog.c` and `src/bin/pg_archivecleanup/pg_archivecleanup.c` of `REL_16_STABLE`.
- Baseline requirement: `operations` "Continuous WAL archive". The delta keeps `operations` "Point-in-time recovery from the WAL archive".
- `overview.md` section 7: R29, R31, R35, R38. Section 9: A35.
