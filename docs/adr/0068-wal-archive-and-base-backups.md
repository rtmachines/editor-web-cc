# ADR-0068: WAL archive and base backups

Status: Accepted. Amended by ADR-0077 (WAL cutoff, marker entry and base backup of `pitr-check`). Amended by ADR-0081 (CPU limit of the temporary container of `pitr-check`). Amended by ADR-0099 (order of the gap procedure, target directory of the moved segments). Amended by ADR-0106 (item (g): the file `backup_end_time` of each base backup)
Date: 2026-10-23
Deciders: project owner

## Context

`reliability.md` section 7.7 gives the service `walarchive`: `pg_receivewal --synchronous` to a second disk, a weekly `pg_basebackup`, and a recovery point of minutes (ADR-0020). Step P4.1 writes the scripts. The documents leave these points open:

- A replication slot keeps WAL until the archive receives it. `max_slot_wal_keep_size` has no limit by default, so a long outage of `walarchive` can fill the disk of `pgdata`.
- `pg_basebackup -X stream` uses two replication connections. With `pg_receivewal`, it uses all `pg.max_wal_senders` (3), and no sender stays spare (section 7.6).
- In PostgreSQL 16, `pg_verifybackup` reads only the plain format.
- No document gives the day and the time of the base backup, or a retention for base backups and WAL segments.
- Recovery does not read the file `.partial` of `pg_receivewal`. Without it, the newest minutes of WAL are lost, against the recovery point of ADR-0020.
- No test runs a recovery from the archive.

## Decision drivers

- Recovery point objective: minutes. Recovery time objective: 1 hour (ADR-0020, R31).
- An outage of `walarchive` never stops a commit in `db` and never fills the disk of `pgdata`.
- A delete runs only after a base backup passes its check.
- One sender stays spare for a future replica (section 7.6).
- CI proves the recovery.

## Considered options

### Option A: No slot, `--wal-method=fetch`, plain format, and a fallback to `.partial`

- Pros: an outage cannot fill the disk of `pgdata`. The base backup uses one sender. `pg_verifybackup` checks each base backup. The recovery replays the last partial segment.
- Cons: an outage longer than `pg.wal_keep_size` leaves a gap in the archive.

### Option B: A replication slot with a bounded `max_slot_wal_keep_size`

- Pros: no gap after an outage below the bound.
- Cons: a new setting and a new risk for the disk of `pgdata`. Section 7.6 describes the design without a slot.

### Option C: `-X stream` and the tar format

- Pros: the base backup holds its own WAL.
- Cons: no sender stays spare during the backup. `pg_verifybackup` of PostgreSQL 16 cannot read the tar format.

## Decision

The project uses **Option A**.

Stream:

- (a) `walarchive-entry` checks the marker of ADR-0066 and writes a pgpass file from `/run/secrets/replicator_password`. Then it starts `pg_receivewal --synchronous` with the connection value `application_name=walarchive` and starts supercronic.
- (b) `pg_receivewal` uses no replication slot. `db` sets no `synchronous_standby_names`, so a commit never waits for the archive.
- (c) The entry script runs `wait -n`. The first exit of a child ends the container, and `restart: always` starts it again.
- (d) The script `walarchive-health` checks that the `pg_receivewal` process runs (`reliability.md` section 7.1).
- (e) An outage longer than `pg.wal_keep_size` (1 GB) leaves a gap. The runbook then moves the old segments aside, takes a new base backup, and starts the stream again.

Base backups:

- (f) `basebackup-once` runs every Sunday at 03:30 UTC. The new row `basebackup.time` holds the time.
- (g) The script runs `pg_basebackup --wal-method=fetch -Fp --checkpoint=fast --manifest-checksums=SHA256`. Then `pg_verifybackup` checks the new base backup.
- (h) The method `fetch` uses one sender. `db` keeps the WAL of the backup through `pg.wal_keep_size`.
- (i) After a passed check, the script keeps the newest 4 base backups. The new row `basebackup.retention` holds the number.
- (j) Then `pg_archivecleanup` deletes the WAL segments before the oldest kept base backup.
- (k) A failed `pg_verifybackup` deletes no base backup and no WAL segment. Each delete follows the rules of ADR-0066 (l) and (m).

Recovery:

- (l) A recovery uses `restore_command = 'cp /wal/%f %p || cp /wal/%f.partial %p'`. The fallback replays the transactions of the partial last segment.
- (m) The CI script `pitr-check` takes a base backup, commits an audit entry, and waits until the archive holds it. Then it recovers a scratch cluster from the base backup and the archive.
- (n) `pitr-check` checks that the recovery is complete, that the entry exists, and that the audit chain check of `data-model.md` section 4.9 passes.
- (o) `pitr-check` runs in a temporary container of the image `recipe-pgtools` with `container.backup_memory` (ADR-0067). Its cluster uses the `scratch.*` settings of ADR-0067 (e).
  - The cluster also sets `hot_standby` from the row `scratch.pitr_hot_standby` (`off`). With `hot_standby = on`, PostgreSQL stops the recovery, because `scratch.max_connections` is lower than `db.max_connections`.
  - The checks of (n) run after the end of the recovery, so the cluster needs no connection during the recovery.

Replication state:

- (p) The base backup adds a sender row of `recipe_replicator`. The job `alerts` therefore reads only the row of `recipe_replicator` with `application_name = 'walarchive'`, through the function of ADR-0069.

## Consequences

### Positive

- The recovery point stays minutes, also for the newest partial segment.
- An outage of `walarchive` stops no commit and cannot fill the disk of `pgdata`.
- Each kept base backup has a passed check.
- CI proves a recovery from the archive on every push.

### Negative

- An outage longer than `pg.wal_keep_size` leaves a gap until the next base backup.
- Four base backups need space on the second disk for four copies of the database.

### Risks and mitigations

- Risk: a defect in the retention deletes the WAL of a kept base backup. Mitigation: the test script of `deploy/pgtools/tests/` checks that a failed `pg_verifybackup` deletes nothing.
- Risk: a network share as the second disk (A35) does not keep the `fsync` of `--synchronous`. Mitigation: the runbook states the risk.

## Related

- ADR-0020 (recovery point and recovery time), ADR-0066 (disks and script rules), ADR-0067 (scratch settings and memory), ADR-0069 (`replication_state()`), ADR-0070 (CI job).
- `reliability.md` section 7.6 (`max_wal_senders` keeps its spare sender), 7.7 (stream, base backup, retention, recovery).
- `reliability.md` section 7.11: the new rows `basebackup.time` Sunday 03:30 UTC, `basebackup.retention` 4 base backups, and `scratch.pitr_hot_standby` `off`.
- Baseline requirement: `operations` "Continuous WAL archive". The delta adds `operations` "Point-in-time recovery from the WAL archive".
- `overview.md` section 7: R31. Section 9: A35.
