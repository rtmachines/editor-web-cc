# ADR-0106: Recovery of production from the WAL archive

Status: Accepted
Date: 2026-10-30
Deciders: project owner

## Context

`reliability.md` section 7.7 and ADR-0020 give a recovery point objective of minutes and a recovery time objective of 1 hour (R31). ADR-0068 keeps a WAL archive and weekly base backups on a second disk (A35). The CI script `pitr-check` proves a recovery into a scratch cluster (ADR-0077).

No document gives the steps to recover production from the WAL archive. ADR-0103 (x) leaves this recovery out of scope. The runbook lists it under "Known gaps", and the only workaround is a restore of a dump. A restore of a dump loses up to 24 hours of changes and audit entries.

Facts that each option must handle:

- A recovery from a base backup keeps the system identifier of the cluster. At the end of the recovery, PostgreSQL starts a new timeline with the next free number.
- The guard of ADR-0103 (s) compares only the system identifier. After a recovery, the old segments and the new cluster have the same identifier.
- `pg_receivewal` uses no replication slot (ADR-0068 (b)). With no segment in `/wal`, it starts at the flush position and the timeline of the server. On a timeline above 1, it also writes the history file of that timeline.
- The move of ADR-0103 (s) step 4 selects segment files and `.partial` files. It leaves the timeline history files `<8 hex>.history` in `/wal`.
- With `recovery_target_timeline = latest`, a recovery reads each history file in the archive. A history file of another WAL line can lead the recovery onto that line.
- A gap in the archive ends a recovery at the gap. PostgreSQL then promotes the cluster and reports no error.
- A disk failure can destroy `pgdata` and leave the WAL archive disk intact. Then `pg_controldata` of the old volume can fail, and no clean shutdown exists.
- The audit chain is a hash chain (`data-model.md` 4.9). A recovery to a consistent point keeps a whole prefix of the chain.

## Decision drivers

- Recovery point objective: minutes. Recovery time objective: 1 hour (ADR-0020, R31).
- Data safety comes before the time of the procedure. A recovery that ends early never becomes production without a stop.
- The recovery uses the switch of ADR-0103 and the tag rule of ADR-0105 (a). The old volume stays unchanged.
- Two WAL lines never mix in `/wal`.
- CI proves the recovery point and the recovery time with the production tool.
- Each rule has one source. An accepted ADR changes only by its Status line and an "Amended by" note.

## Considered options

Recovery target:

- Option A: the end of the archive only, with the `.partial` file. Cons: a logical fault, for example a wrong manual statement, comes back with the replay. The only way back is then a dump, with a loss of up to 24 hours.
- Option B: the end of the archive by default, and an optional target time. Cons: a second mode, more tests, and the risk of a wrong time.
- Option C: option B and also a target LSN or a target transaction ID. Cons: the operator cannot read an LSN or a transaction ID in the UI. The gain over option B is small.

Tool:

- Option D: a new one-shot service `pitr-volume` fills a new volume from a base backup and `/wal`. Cons: one more service and one more test suite.
- Option E: a mode of `restore-volume`. Cons: one script then has two sources, two sets of gates, and two mounts.
- Option F: a production mode of `pitr-check`. Cons: `pitr-check` writes onto the WAL archive disk, and a full disk stops `pg_receivewal`. It also needs a marker from CI.

WAL line after the switch:

- Option G: the new WAL line of ADR-0103 (r), (s), with the history files in the move. Cons: a later recovery to a time before this recovery needs a separate procedure on the moved files.
- Option H: keep the old segments, because the identifier stays the same. Cons: two timelines and an old `.partial` file stay in `/wal`. The start point of `pg_receivewal` and the cutoff of `pg_archivecleanup` then work on mixed timelines. The recovery point can get lost without an alert.

Proof:

- Option I: a new CI job with its own script, and a recovery drill on the site. Cons: CI runs longer, and the operator gets one more procedure.
- Option J: more phases in the job `restore-switch`. Cons: one job of more than 40 minutes. A fault of one path hides the other path.
- Option K: a new CI job without a measurement of the recovery time. Cons: the recovery time objective stays without proof.

## Decision

The project uses **Option B**, **Option D**, **Option G**, and **Option I**. These are decisions of the user of 2026-10-30.

Recovery target:

- (a) The default target is the end of the archive. The `restore_command` of ADR-0068 (l) replays the `.partial` file.
  - The operator can give one target time with `--target-time <time>`. The value has the grammar `^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$` and is UTC.
  - With a target time, the script sets `recovery_target_time`, `recovery_target_inclusive = off`, and `recovery_target_action = promote`.
  - PostgreSQL 16 stops with `FATAL` when the archive ends before the target. It also stops when the target lies before the consistent point of the base backup. The script then writes one `ERROR` line.
- (b) The operator selects the target time. The source is the audit log of the old cluster when `db` still runs, else the off-site audit copy.
  - The operator reads `occurred_at` of the first unwanted audit entry. The target is that time, cut to the full second.
  - A commit in the same second before that entry is then also lost. The user accepts this limit.
  - A manual statement as a superuser writes no audit entry. Before each such step of the runbook, the operator writes the start time in UTC into the change log of the site.
  - For a fault without an audit entry, that start time, cut to the full second, is the target. A fault without such a record needs an estimate of the operator.
- (c) Without a target time, the script selects the newest base backup by the grammar of ADR-0077 (d).
  - `basebackup-once` writes the file `backup_end_time` into the temp directory after `pg_verifybackup` passes and before the rename. This item amends ADR-0068 (g).
  - The file holds one line: the UTC time after `pg_verifybackup`, in the grammar of (a), rounded up to the next full second. The end of the base backup lies before that time.
  - With a target time, the script selects the newest base backup whose `backup_end_time` lies at or before the target. A target inside a run therefore never selects that run.
  - A base backup without the file is older than this ADR. With a target time, its line `START TIME` of `backup_label` counts, and only with the zone `UTC`.
  - A target inside the run of such an older base backup stops the recovery with `FATAL` (item (a)). The operator never selects a later target, because that target can replay the fault.
  - `pg_verifybackup` of a copy ignores the file `backup_end_time`, because the manifest does not list it. `pitr-volume` and `pitr-check` then delete the file from the copy.
- (d) The last line of the script holds the values that the operator compares before the switch:
  - the name of the base backup, the system identifier, and the new timeline
  - `chain_seq`, `occurred_at`, and `action` of the newest audit entry
  - the value of `heartbeat.last_at` in `app_setting`
  - A value that does not fit the expectation ends the attempt. The operator deletes the new volume and does not switch.

Service and script:

- (e) A new one-shot service `pitr-volume` fills a new volume from the WAL archive.
  - It has the profile `restore`, as `restore-volume` has. It is not one of the seven services, and `docker compose up` does not start it.
  - It has `network_mode: none`, the mount `walarchive:/wal:ro`, the tmpfs `/run/pgpass`, and no secret.
  - It gets `COMPOSE_SUBNET` for the checks of ADR-0103 (m). It gets `container.backup_memory` and `container.small_cpus`.
  - This item amends ADR-0067 (b) and ADR-0070 (j). Their lists now name the temporary container of `pitr-volume`.
  - The operator mounts the target: `docker compose run --rm --no-deps -T -v <new volume>:/var/lib/postgresql/data pitr-volume [--target-time <time>]`.
  - The image is the `recipe-pgtools` image of the tag of the schema at the recovery point (ADR-0083 (a), ADR-0105 (a)). A target time before the `migrate` of an upgrade needs the previous tag. The change log of the upgrade gives the time of `migrate`.
- (f) The script `deploy/pgtools/bin/pitr-volume` runs these steps:
  1. It refuses a target that is not empty and an argument outside the grammar of (a). It refuses a run as a user other than uid 999 with the name `postgres` (ADR-0105 (r)).
     - The arguments are none, `--target-time <time>`, and `--drill`. `--drill` requires `--target-time`.
  2. It checks the marker file of `/wal`. It opens `/wal/.lock` read-only and takes `flock -n`. A run that does not get the lock writes one `ERROR` line. The lock stays to the exit, so `basebackup-once` cannot delete a file.
  3. It reads `/wal/.system-identifier` with the checks of `wal-identity.sh`. A missing or malformed file stops the run.
  4. It selects the base backup of (c). It checks the free space of the target against the size of the base backup and of the segment files in `/wal`.
     - After the copy and the replay, the free space of the target disk must stay at or above `disk.warn_percent`. A drill on the `pgdata` disk then never puts `app` into read-only mode.
  5. It copies the base backup into the target and runs `pg_verifybackup` on the copy, with the checksums. The check ignores `backup_end_time` of (c), and the script then deletes that file from the copy.
  6. The system identifier of the copy in `pg_controldata` must equal `/wal/.system-identifier`.
  7. It keeps the SHA-256 values of `postgresql.conf`, `postgresql.auto.conf`, `pg_hba.conf`, and `pg_ident.conf` of the copy in shell variables (ADR-0105 (q)).
  8. It writes `recovery.signal` and starts a server with `listen_addresses=''` on a private socket in `/run/pgpass`. It passes each recovery setting with `pg_ctl -o` and writes none into a configuration file.
     - The settings are the `restore_command` of ADR-0068 (l), the target of (a), `hot_standby = off` (ADR-0068 (o)), `archive_mode = off`, and the `scratch.*` sizes of `reliability.md` 7.11.
     - The start waits up to `pitr_volume.start_timeout`. The connection check polls every `pitr_check.ready_poll`. A failed start prints `pitr_check.log_tail` lines of the server log.
  9. After the promotion, it runs the checks of (g). A failed check ends the run with an `ERROR` line.
  10. It reads the values of (d).
  11. It stops the server with `--mode=fast` within `pitr_volume.stop_timeout`. `pg_controldata` must show `shut down`. With `--drill`, it writes the empty file `pitr-drill` at the top of the target. Then it runs `sync`.
  12. It writes one JSON line `pitr volume completed` with the values of (d) and the field `drill`.
  - A failed step writes one `ERROR` line and ends the script with a code other than 0. The operator then deletes the new volume. Nothing else changed.
- (g) The checks after the promotion are a gate. Each failed check writes its own `ERROR` line:
  1. `pg_is_in_recovery()` gives `false`, and the timeline is above the timeline of the base backup.
  2. Without a target time, the end of the recovery lies in the newest segment of `/wal` on the timeline of the base backup. The history file of the new timeline gives the end. Else the recovery stopped at a gap, and the run stops with `the recovery ended before the end of the WAL archive`.
     - When the operator gives `PITR_EXPECTED_LSN`, the end must also lie at or after that position. The same `ERROR` line applies. The value comes from step 3 of (i).
  3. The audit chain check of `data-model.md` 4.9 over the whole table gives no break, and the head matches the last row.
  4. When the operator gives `PITR_MARKER_SEQ` and `PITR_MARKER_HASH`, exactly one row of `audit_log` has these values. The grammars of ADR-0077 (b) apply.
  5. The four configuration files of step 7 keep their SHA-256 values. No file `recovery.signal` or `standby.signal` stays.
  6. The values of ADR-0103 (m) equal the file `restore-roles.expected` of the image.
- (h) `pitr-check` and `pitr-volume` share one library of their common steps in `deploy/pgtools/lib/`. `pitr-check` keeps its gate `PITR_CI=1`, its scratch directory, and its checks (ADR-0077 (d), (e)).
  - `pitr-check` handles `backup_end_time` as step 5 of (f) says. This item amends ADR-0077 (d).

Order of the recovery:

- (i) Each recovery from the WAL archive uses this order:
  1. The operator runs `docker compose stop app backup`. After this point, `app` writes no audit entry.
  2. When `db` runs, the operator reads `chain_seq` and `entry_hash` of the newest audit entry through the socket login of ADR-0077 (b). In a recovery to the end, these values are the expected marker of (g) step 4.
  3. When `db` runs, the operator reads `pg_current_wal_lsn()`. The operator runs the flush check of ADR-0099 (a2) with that position until it returns `t`. In a recovery to the end, that position is `PITR_EXPECTED_LSN` of (g) step 2.
  4. The operator runs `docker compose stop walarchive`. The `.partial` file then stays stable.
  5. The operator creates the new volume (ADR-0103 (e)).
  6. `pitr-volume` fills the new volume. `db` keeps running on the old volume when it runs.
  7. The operator compares the values of (d). A value that does not fit ends the attempt as ADR-0105 (c) says.
  8. The switch of ADR-0103 (f) runs with the changes of (j).
  9. The operator sets the role passwords from the secret files and counts the log lines, as ADR-0103 (b) step 6. The cluster already holds the passwords of production. The step keeps the secret files as the one source.
  10. The new WAL line of (k) starts.
  11. The operator truncates `session` and `login_attempt`. The operator re-applies each user change after the recovery point. The operator records the values of ADR-0100 (c). Then the operator starts `app`, then `proxy` and `backup`.
  12. The wait of ADR-0102 ends the recovery. Then `backup-now` and `restore-test` prove the first set of the new cluster (ADR-0103 (w)).
  - The truncate of step 11 matters. A session that the old cluster revoked after the recovery point comes back with the recovery.
- (j) The switch of a recovery has two changes:
  - Step 2 of ADR-0103 (f) uses the rule of the switch back (ADR-0103 (g)). It needs no state of the old volume and never starts `db` on that volume. It writes the identifier and the timeline when `pg_controldata` reads them, else the word `unreadable`.
  - Step 4 of ADR-0103 (f) compares the system identifier and the timeline of the new volume with the line of (f) step 12.
  - Step 4 also refuses a volume that holds the file `pitr-drill` or a line whose field `drill` is `true`. A drill volume never becomes production.
  - These changes amend ADR-0103 (f).
  - After a failed read of the old volume, no switch back exists. The new volume is then the only cluster.
- (k) A recovery starts the new WAL line of ADR-0103 (r), (s).
  - Step 4 of ADR-0103 (s) also moves each file of the grammar `^[0-9A-F]{8}\.history$` at the top of `/wal`. This rule holds for every switch. It amends ADR-0103 (s) step 4.
  - In a gap of the WAL stream (ADR-0099 (a)), the history files stay in `/wal`, because the WAL line does not change.
  - The file `old-system-identifier` also holds the line `Latest checkpoint's TimeLineID` of the old cluster.
  - The delete rule of ADR-0103 (t) applies to the moved files and to the old volume.

Identity of a volume:

- (l) The change log of the site also holds the timeline of each volume that a switch leaves. A switch back compares the system identifier and the timeline. This item amends ADR-0105 (b) and ADR-0103 (g).
  - An entry from before this ADR holds no timeline. A switch back to that volume compares the system identifier only.
- (m) `walarchive-entry` and `basebackup-once` also refuse to start when a segment at the top of `/wal` has a timeline other than the timeline of `IDENTIFY_SYSTEM`. The `ERROR` line is `segments of another timeline are in the WAL archive`.
  - After a recovery, the identifier stays the same, so the check of ADR-0103 (s) cannot find old segments. This check finds them.
  - `db` is a primary server, so the stream never follows a timeline switch. Every segment of one WAL line has one timeline.
  - This item amends ADR-0103 (s).

Recovery time:

- (n) `reliability.md` section 7.11 gets two rows:
  - `pitr_volume.start_timeout` 1800 s. The `pg_ctl start` of `pitr-volume` waits for the end of the recovery. One week of WAL lies between two base backups.
  - `pitr_volume.stop_timeout` 600 s. It equals `restore_volume.stop_timeout`.
  - The rows `pitr_check.ready_poll` and `pitr_check.log_tail` also serve `pitr-volume`.
  - The two waits use 2400 s. The rest of the hour covers the copy, the switch, the new WAL line, and the start of `app`.
- (o) The runbook gets a recovery drill. The operator runs steps 5 and 6 of (i) on a drill volume while the stack runs, with `--drill`.
  - The drill uses a target time two `heartbeat.interval` before its start, after the end of the newest base backup. The job `heartbeat` then gives a commit after the target.
  - The drill runs outside the run of `basebackup-once`. `pitr-volume` holds the lock of `/wal`. When `basebackup-once` refused a run because of the lock, the operator runs it again at once.
  - The operator writes the time of the fill and the time of the last run of `basebackup-once` into the change log. The sum must stay below the recovery time objective of 1 hour. A larger sum is a finding of the drill.
  - The operator deletes the drill volume. The file `pitr-drill` and step 4 of (j) keep it out of production.
  - The operator runs the drill after each upgrade of `reliability.md` 7.10.

Proof in CI:

- (p) A new CI job `pitr-recovery` runs the script `deploy/ci/pitr-recovery.sh`. It runs where the job `compose` runs (ADR-0094). Its steps are:
  1. It starts the stack, creates the administrator, and changes the password once. Then it runs `basebackup-once`.
  2. It commits the marker 1 with `ChangeOwnPassword` (ADR-0077 (a)). It records a target time after that commit. It commits the marker 2 with a new password.
  3. It waits for the flush of the marker 2 (ADR-0077 (c)). It runs no `pg_switch_wal()`, so the marker 2 stays in the `.partial` file.
  4. It runs the drill of (o) with that target time. The newest audit entry is the marker 1. A switch of the drill volume fails. It deletes the drill volume.
     - A second drill uses a target time after the newest commit and expects the `ERROR` line of (a).
     - A third run uses a copy of `/wal` without one segment after the base backup. It expects the `ERROR` line of (g) step 2.
  5. It stops `app` and `backup` and reads `pg_current_wal_lsn()`. The segment of that position exists in `/wal` only as a `.partial` file.
     - Then it kills `db` with `SIGKILL` and stops `walarchive`. It deletes `global/pg_control` of the old volume, so step 2 of (j) writes `unreadable`.
  6. It runs `pitr-volume` to the end with the marker 2 and `PITR_EXPECTED_LSN`. Then it runs the switch with the changes of (j), the passwords, and the new WAL line. Then it starts `app`.
  7. A login with the password of the marker 2 passes. This login proves the recovery point.
  8. `backup-now` and `restore-test` pass on the new cluster. `basebackup-once`, a new marker, and `pitr-check` pass on the new WAL line.
  9. The guard of (m) refuses a segment of the old timeline in a copy of `/wal`.
  - The job writes the time of each step. It fails when the time from step 5 to the start of `app` is above the recovery time objective of 1 hour (`reliability.md` 7.7).

Changes of other documents:

- (q) The follow-up edits of this ADR are:
  - The Status lines of ADR-0067, ADR-0068, ADR-0070, ADR-0077, ADR-0103, and ADR-0105 get "Amended by ADR-0106" with the scope. No other line of these ADRs changes.
  - `docs/adr/README.md`: a new index row, the same notes in the Status cells, and the decision map.
  - `reliability.md` sections 7.7, 7.8, and 7.11.
  - A later OpenSpec change of the capability `operations` changes `compose.yaml`, the pgtools scripts, the tests, the CI workflow, the spec, the runbook, and `CHANGELOG.md`.
  - The runbook loses two bullets of "Known gaps": the recovery from the WAL archive and the measurement of the recovery time.

## Consequences

### Positive

- Production recovers with a loss of minutes, also after the loss of the `pgdata` disk.
- A logical fault can be undone to a time with a loss of minutes.
- A recovery that stops at a gap never becomes production without an `ERROR` line.
- Two WAL lines never mix in `/wal`, also when they share one system identifier.
- CI proves the recovery point with a login, and the drill measures the recovery time on the data of the site.

### Negative

- The downtime includes the copy, the replay, the new WAL line, and its first base backup.
- The new volume needs free space on the `pgdata` disk for the base backup and the replayed WAL.
- A recovery to a time before an earlier recovery needs a procedure on the moved files. No document gives it.
- After a failed read of the old volume, no switch back exists.
- A base backup from before this ADR has no `backup_end_time`. A target inside its run stops with `FATAL` until the retention of `basebackup.retention` deletes it.

### Risks and mitigations

- Risk: the operator selects a wrong target time. Mitigation: the values of (d), and the old volume stays unchanged.
- Risk: a manual fault has no record of its time. Mitigation: the change log rule of (b). Else the operator estimates the time and compares the values of (d).
- Risk: the archive holds a gap after the newest base backup. Mitigation: check 2 of (g) stops the run without a target time. With a target time after the gap, PostgreSQL stops with `FATAL`.
- Risk: the operator forgets the move of (k). Mitigation: the guard of (m) refuses to start `walarchive`.
- Risk: the image tag does not fit the schema at the recovery point. Mitigation: check 6 of (g) and the tag rule of (e).
- Risk: one week of WAL does not replay within `pitr_volume.start_timeout`. Mitigation: the drill of (o) measures the time on the data of the site.

## Related

- Amends ADR-0067 (b) and ADR-0070 (j) (the temporary container of `pitr-volume`), ADR-0068 (g) and ADR-0077 (d) (the file `backup_end_time`).
- Amends ADR-0103 (f), (g), (s), (x), and ADR-0105 (b).
- ADR-0020 and ADR-0068 (recovery point and recovery time), ADR-0077 (marker, layout, scratch constants), ADR-0083 (a) (tag of the schema).
- ADR-0094 (trigger of the job `compose`), ADR-0099 (gap procedure, flush check), ADR-0100 and ADR-0102 (end of a restore), ADR-0104.
- `docs/architecture/reliability.md` sections 7.7, 7.8, and 7.11. `data-model.md` sections 4.9 and 5.1.
- `docs/ops/runbook.md` sections "Restore into production", "Switch back", and "Known gaps".
- Baseline requirements of `operations`: "Point-in-time recovery from the WAL archive", "Volume switch", and "Switch back".
- Also "WAL line after a switch" and "System identifier guard of the WAL archive".
- `overview.md` section 7: R31, R34. Section 9: A35.
