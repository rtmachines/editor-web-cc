## ADDED Requirements

### Requirement: Service of the recovery from the WAL archive
The one-shot service `pitr-volume` SHALL fill a new, empty volume from a base backup and the WAL archive. It SHALL have the profile `restore`, no network, no secret, and a read-only mount of `/wal`. It SHALL hold the lock `/wal/.lock` until it exits. A failed step SHALL write one `ERROR` line, end with a code other than 0, and change no file outside the new volume (R31, A35).

#### Scenario: Volume that is not empty
- **GIVEN** a target volume that holds a file
- **WHEN** the operator runs `pitr-volume`
- **THEN** the script writes one `ERROR` line before any PostgreSQL tool runs
- **AND** the volume does not change

#### Scenario: Lock held by another run
- **GIVEN** a run of `basebackup-once` that holds `/wal/.lock`
- **WHEN** the operator runs `pitr-volume`
- **THEN** the script writes one `ERROR` line and copies nothing

#### Scenario: Base backup of another system
- **GIVEN** a base backup whose system identifier differs from `/wal/.system-identifier`
- **WHEN** `pitr-volume` checks the copy
- **THEN** the script writes one `ERROR` line and starts no server

#### Scenario: Wrong user
- **GIVEN** a container that runs the script as a uid other than 999, or as uid 999 with a name other than `postgres`
- **WHEN** the operator runs `pitr-volume`
- **THEN** the script writes one `ERROR` line before any PostgreSQL tool runs

### Requirement: Target of a recovery from the WAL archive
Without a target time, `pitr-volume` SHALL recover to the end of the WAL archive, including the `.partial` file. With `--target-time`, it SHALL accept only a UTC time of the grammar of ADR-0106 (a). It SHALL stop the recovery before the first commit at or after that time. It SHALL select the newest base backup whose end time lies at or before the target. A base backup without an end time SHALL count with its start time in UTC (R31, A35).

#### Scenario: Recovery to the end of the archive
- **GIVEN** a base backup, then a commit of an audit entry, and the WAL archive that holds the commit only in the `.partial` file
- **WHEN** `pitr-volume` runs without a target time
- **THEN** the new volume holds the audit entry

#### Scenario: Recovery to a target time
- **GIVEN** a commit of an audit entry A, then a target time, then a commit of an audit entry B
- **WHEN** `pitr-volume` runs with that target time
- **THEN** the new volume holds the entry A and does not hold the entry B

#### Scenario: Target after the end of the archive
- **GIVEN** a target time after the newest commit in the WAL archive
- **WHEN** `pitr-volume` runs with that target time
- **THEN** the script writes one `ERROR` line and ends with a code other than 0

#### Scenario: Bad target time
- **GIVEN** a target time without the suffix `Z`, or with a date that is not a date
- **WHEN** the operator runs `pitr-volume` with it
- **THEN** the script writes one `ERROR` line before any PostgreSQL tool runs

#### Scenario: Target inside the run of a base backup
- **GIVEN** a base backup A, a newer base backup B, and a target time after the start of B and before its end time
- **WHEN** `pitr-volume` runs with that target time
- **THEN** the script selects the base backup A

#### Scenario: Fault without an audit entry
- **GIVEN** a manual statement as a superuser, and its start time in the change log of the site
- **WHEN** `pitr-volume` runs with that start time as the target time
- **THEN** the new volume does not hold the change of that statement

### Requirement: End time of a base backup
`basebackup-once` SHALL write the file `backup_end_time` into the temp directory of each base backup after `pg_verifybackup` passes and before the rename. The file SHALL hold one UTC time at or after the end of the base backup. A check of a copy with `pg_verifybackup` SHALL ignore that file, and the copy SHALL then lose it (R31, A35).

#### Scenario: Published base backup
- **GIVEN** a run of `basebackup-once` whose `pg_verifybackup` passes
- **WHEN** the run renames the temp directory
- **THEN** the base backup holds the file `backup_end_time` with one time of the grammar of ADR-0106 (a)

#### Scenario: Copy of a base backup
- **GIVEN** a base backup with the file `backup_end_time`
- **WHEN** `pitr-check` or `pitr-volume` checks its copy
- **THEN** `pg_verifybackup` passes
- **AND** the copy does not hold the file `backup_end_time`

### Requirement: Checks of a recovered cluster
After the promotion, `pitr-volume` SHALL check the recovered cluster before the clean stop. A failed check SHALL end the run. The checks SHALL cover the end of the replay, the timeline, the audit chain, and an optional marker entry. The checks SHALL also cover the configuration files and the roles of ADR-0103 (m). The last line SHALL hold the values of ADR-0106 (d) (R31, R29, A35).

#### Scenario: Gap in the archive
- **GIVEN** a WAL archive with a missing segment between the base backup and the newest segment
- **WHEN** `pitr-volume` runs without a target time
- **THEN** the script writes the `ERROR` line `the recovery ended before the end of the WAL archive`

#### Scenario: End before the expected position
- **GIVEN** a value `PITR_EXPECTED_LSN` after the end of the recovery
- **WHEN** `pitr-volume` runs without a target time
- **THEN** the script writes the `ERROR` line `the recovery ended before the end of the WAL archive`

#### Scenario: Break of the audit chain
- **GIVEN** a recovered cluster whose audit chain check finds a break
- **WHEN** `pitr-volume` runs its checks
- **THEN** the script writes one `ERROR` line and ends with a code other than 0

#### Scenario: Missing marker entry
- **GIVEN** values of `PITR_MARKER_SEQ` and `PITR_MARKER_HASH` that no row of the recovered `audit_log` holds
- **WHEN** `pitr-volume` runs its checks
- **THEN** the script writes one `ERROR` line and ends with a code other than 0

#### Scenario: Recovery setting in a configuration file
- **GIVEN** a recovered cluster whose `postgresql.auto.conf` differs from the copy of the base backup
- **WHEN** `pitr-volume` runs its checks
- **THEN** the script writes one `ERROR` line and ends with a code other than 0

#### Scenario: Completed recovery
- **GIVEN** a recovered cluster that passes every check
- **WHEN** `pitr-volume` stops the server
- **THEN** `pg_controldata` shows `shut down`
- **AND** the last line `pitr volume completed` holds the system identifier, the new timeline, the newest audit entry, and `heartbeat.last_at`

### Requirement: Order of a recovery from the WAL archive
A recovery of production from the WAL archive SHALL stop `app` and `backup` first. When `db` runs, the operator SHALL then read the marker entry, and the flush check SHALL pass before `walarchive` stops. `pitr-volume` SHALL fill a new volume, and the switch SHALL follow. The new WAL line SHALL start before `app`. The operator SHALL truncate `session` and `login_attempt` before `app` starts (R31, R34, A35).

#### Scenario: Recovery after the loss of the pgdata disk
- **GIVEN** a `db` that does not start, and an intact WAL archive disk
- **WHEN** the operator runs the recovery without a target time
- **THEN** `db` runs on the new volume with every commit that the archive received
- **AND** `walarchive` streams the new WAL line before `app` starts

#### Scenario: Revoked session after the recovery point
- **GIVEN** a session that the old cluster deleted after the recovery point
- **WHEN** the operator truncates `session` before the start of `app`
- **THEN** the session token of that session does not authenticate

### Requirement: Recovery drill
The runbook SHALL give a recovery drill. The drill SHALL run `pitr-volume` with a target time and `--drill` on a drill volume while the stack runs. The drill SHALL change no volume of production and no file of `/wal`. The free space of the target disk SHALL stay at or above `disk.warn_percent`. After each upgrade, the operator SHALL record the time of the fill and of the last base backup (R31, R34).

#### Scenario: Drill during operation
- **GIVEN** a running stack and a target time two `heartbeat.interval` before the start of the drill
- **WHEN** the operator runs the drill
- **THEN** `pitr-volume` passes and prints the time of the fill
- **AND** `db` and `walarchive` keep running on their volumes

#### Scenario: Drill without enough free space
- **GIVEN** a `pgdata` disk whose free space after the copy is below `disk.warn_percent`
- **WHEN** the operator runs the drill
- **THEN** the script writes one `ERROR` line and copies nothing

#### Scenario: Switch to a drill volume
- **GIVEN** a drill volume with the file `pitr-drill`
- **WHEN** the operator runs the switch to that volume
- **THEN** the operator starts no `db` on that volume and deletes no volume

### Requirement: Recovery test in CI
CI SHALL run the job `pitr-recovery` where the job `compose` runs. The job SHALL kill `db` with `SIGKILL` after the archive flushed a marker commit. It SHALL recover with `pitr-volume`, switch, and start `app`. A login with the password of the marker commit SHALL pass. The time from the kill to the start of `app` SHALL stay at or below 1 hour. The job SHALL also prove the refusals of a gap, a target after the archive, and a drill volume (R31, A35).

#### Scenario: Recovery point after a crash
- **GIVEN** a commit of `ChangeOwnPassword` in the `.partial` file only, and a `db` killed with `SIGKILL`
- **WHEN** the job recovers into a new volume and starts `app`
- **THEN** a login with the new password passes

#### Scenario: Target time in CI
- **GIVEN** the marker 1, a target time, and the marker 2
- **WHEN** the job runs the drill with that target time
- **THEN** the newest audit entry of the drill volume is the marker 1

#### Scenario: Recovery time above the objective
- **GIVEN** a run whose time from the kill to the start of `app` is above 1 hour
- **WHEN** the job ends
- **THEN** the job fails and names the time of each step

#### Scenario: Unreadable old volume in CI
- **GIVEN** an old volume without `global/pg_control` after the kill
- **WHEN** the job runs the switch
- **THEN** the change log file holds the word `unreadable` for the old volume
- **AND** `db` starts on the new volume

#### Scenario: New WAL line after the recovery
- **GIVEN** the cluster of the new volume and its new WAL line
- **WHEN** the job runs `basebackup-once`, a new marker commit, and `pitr-check`
- **THEN** `pitr-check` passes

## MODIFIED Requirements

### Requirement: Container limits
Each service SHALL have a hard memory limit without swap and a CPU limit. This includes the service `pitr-check` in the profile `pitr` and the services `restore-volume` and `pitr-volume` in the profile `restore`. The values SHALL equal the keys `container.*` of `reliability.md` section 7.11 (R30, R04).

#### Scenario: Memory limit of the application
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator reads the limits of the service `app`
- **THEN** the memory limit and the memory with swap are both 512 MiB

#### Scenario: CPU limit of the recovery check
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator reads the limits of the service `pitr-check`
- **THEN** the CPU limit is 0.5 and the memory limit and the memory with swap are both 512 MiB

#### Scenario: Limits of the restore into a new volume
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator reads the limits of the service `restore-volume`
- **THEN** the CPU limit is 0.5 and the memory limit and the memory with swap are both 512 MiB

#### Scenario: Limits of the recovery into a new volume
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator reads the limits of the service `pitr-volume`
- **THEN** the CPU limit is 0.5 and the memory limit and the memory with swap are both 512 MiB

### Requirement: Volume switch
The switch SHALL delete the containers of `proxy`, `app`, `backup`, `walarchive`, and `db`. It SHALL NOT use `docker compose down` or `docker compose start`. Before `db` starts on the new volume, `pg_controldata` SHALL show the state `shut down` and the system identifier of the last line of `restore-volume` or `pitr-volume`. After a recovery from the WAL archive, the timeline SHALL also equal that line, and the switch SHALL need no state of the old volume. The switch SHALL refuse a volume with the file `pitr-drill`. After the start of `app` and `backup`, `docker inspect` of each container SHALL show the new volume (R31, R34).

#### Scenario: Matching system identifier
- **GIVEN** a new volume that `restore-volume` filled, and `RECIPE_PGDATA_VOLUME` that names it
- **WHEN** the check of `pg_controldata` passes and the operator runs `docker compose up -d --no-deps db`
- **THEN** `docker inspect` of `db` shows the new volume
- **AND** the old volume does not change

#### Scenario: Mounts of app and backup after the switch
- **GIVEN** `db` on the new volume after the switch
- **WHEN** the operator starts `app` and `backup` with `docker compose up -d --no-deps`
- **THEN** `docker inspect` of `app` and of `backup` shows the new volume

#### Scenario: Wrong volume name
- **GIVEN** a value `RECIPE_PGDATA_VOLUME` that names a volume of another system identifier
- **WHEN** the operator runs the check of `pg_controldata`
- **THEN** the operator starts no `db` on that volume and deletes no volume
- **AND** the operator runs the switch back

#### Scenario: Old cluster not shut down
- **GIVEN** a switch whose first step deleted a `db` container that did not stop cleanly
- **WHEN** `pg_controldata` of the old volume shows a state other than `shut down`
- **THEN** the operator keeps the old volume name and runs `docker compose up -d --no-deps db`
- **AND** the operator runs the first step of the switch again before the name changes

#### Scenario: Switch after a recovery from the WAL archive
- **GIVEN** a new volume that `pitr-volume` filled, and an old volume that `pg_controldata` cannot read
- **WHEN** the operator runs the switch
- **THEN** the change log holds the word `unreadable` for the old volume
- **AND** `db` starts on the new volume after the identifier and the timeline match

#### Scenario: Old timeline on the new volume name
- **GIVEN** a value `RECIPE_PGDATA_VOLUME` that names the old volume of a recovery, with the same system identifier and the old timeline
- **WHEN** the operator runs the check of `pg_controldata`
- **THEN** the operator starts no `db` on that volume and deletes no volume

#### Scenario: Drill volume on the new volume name
- **GIVEN** a value `RECIPE_PGDATA_VOLUME` that names a volume with the file `pitr-drill`
- **WHEN** the operator runs the check of the switch
- **THEN** the operator starts no `db` on that volume and deletes no volume

### Requirement: Switch back
A switch back SHALL use the steps of the switch with the old volume name. The first switch wrote two values into the change log of the site. The system identifier and the timeline of the old volume SHALL equal these values. An entry without a timeline SHALL need the system identifier only. When this check fails, the operator SHALL start no `db` and delete no volume (R31, R34).

#### Scenario: Switch back to the old volume
- **GIVEN** a switch to a new volume and the change log entry of the old volume
- **WHEN** the operator runs the switch back
- **THEN** `db` runs on the old volume with the data of the time before the switch
- **AND** `walarchive` starts before `app`

#### Scenario: Failed check of a switch back
- **GIVEN** an old volume whose system identifier differs from the change log entry
- **WHEN** the operator runs the switch back
- **THEN** no container of `db` starts
- **AND** both volumes stay on the host

#### Scenario: Switch back after a recovery from the WAL archive
- **GIVEN** a recovery from the WAL archive, and a change log entry of the old volume with its timeline
- **WHEN** the operator runs the switch back to a volume whose timeline differs from that entry
- **THEN** no container of `db` starts
- **AND** both volumes stay on the host

#### Scenario: Change log entry without a timeline
- **GIVEN** a change log entry of the old volume from before ADR-0106, with the system identifier and no timeline
- **WHEN** the operator runs the switch back and the identifier matches
- **THEN** `db` starts on the old volume

### Requirement: WAL line after a switch
Every switch SHALL start a new WAL line before `app` starts. The operator SHALL move the old segments, the `.partial` file, the timeline history files, the base backups, and `/wal/.system-identifier` into a target directory. The first base backup of the new line SHALL pass its checks before the first write of `app` (R31, A35).

#### Scenario: WAL line after a restore
- **GIVEN** a switch to a volume that `restore-volume` filled
- **WHEN** the operator starts `walarchive` after the move of the old files
- **THEN** `/wal` holds only segments of the new cluster
- **AND** the flush check and the WAL check pass before `app` starts

#### Scenario: Old base backups after a switch
- **GIVEN** a WAL line after a switch
- **WHEN** `basebackup-once` keeps the newest base backups and runs `pg_archivecleanup`
- **THEN** `/wal/base` holds no base backup of the old cluster

#### Scenario: WAL line after a recovery from the WAL archive
- **GIVEN** a switch to a volume that `pitr-volume` filled, and a history file of an earlier timeline in `/wal`
- **WHEN** the operator moves the old files and starts `walarchive`
- **THEN** `/wal` holds no segment and no history file of the old WAL line
- **AND** `/wal` holds the history file of the new timeline after the stream starts

### Requirement: System identifier guard of the WAL archive
`walarchive-entry` and `basebackup-once` SHALL read `/wal/.system-identifier` before they write to `/wal`. When the file is missing and no segment exists, they SHALL write the identifier of the server into it. When the file is missing next to a segment, they SHALL refuse to start. They SHALL also refuse a file with another identifier, a malformed file, and a segment whose timeline differs from the timeline of the server. In a gap of the WAL stream, the file and the history files SHALL stay in `/wal` (R31, A35).

#### Scenario: First start of the WAL archive
- **GIVEN** a directory `/wal` with no segment and no file `.system-identifier`
- **WHEN** `walarchive-entry` starts
- **THEN** the file `/wal/.system-identifier` holds the system identifier of `db`
- **AND** `pg_receivewal` starts

#### Scenario: Segments without the file
- **GIVEN** a directory `/wal` with a segment and no file `.system-identifier`
- **WHEN** `walarchive-entry` or `basebackup-once` starts
- **THEN** the script writes one `ERROR` line and writes no file to `/wal`

#### Scenario: Another system
- **GIVEN** a file `/wal/.system-identifier` that holds an identifier other than that of `db`
- **WHEN** `walarchive-entry` or `basebackup-once` starts
- **THEN** the script writes one `ERROR` line and writes no file to `/wal`

#### Scenario: Same system
- **GIVEN** a file `/wal/.system-identifier` that holds the identifier of `db`
- **WHEN** `walarchive-entry` starts
- **THEN** `pg_receivewal` starts and continues after the newest complete segment

#### Scenario: Gap of the WAL stream
- **GIVEN** a gap of the WAL stream, and the operator moved the segments and the `.partial` file out of `/wal`
- **WHEN** `walarchive-entry` starts on the same cluster
- **THEN** `/wal/.system-identifier` still holds the identifier of `db`
- **AND** `pg_receivewal` starts

#### Scenario: Failed read of the server identifier
- **GIVEN** a replication connection to `db` that fails with an error of libpq
- **WHEN** `walarchive-entry` starts
- **THEN** the script writes one `ERROR` line with the field `detail`, which holds the cleaned first line of that error
- **AND** the script writes no file to `/wal`

#### Scenario: Malformed identifier file
- **GIVEN** a file `/wal/.system-identifier` that is empty, holds a CR, or holds a character other than a digit
- **WHEN** `walarchive-entry` or `basebackup-once` starts
- **THEN** the script writes one `ERROR` line with the message of a malformed file
- **AND** the script writes no file to `/wal`

#### Scenario: Segment of another timeline
- **GIVEN** a file `/wal/.system-identifier` that holds the identifier of `db`, and a segment of timeline 1 while `db` runs on timeline 2
- **WHEN** `walarchive-entry` or `basebackup-once` starts
- **THEN** the script writes the `ERROR` line `segments of another timeline are in the WAL archive`
- **AND** the script writes no file to `/wal`
