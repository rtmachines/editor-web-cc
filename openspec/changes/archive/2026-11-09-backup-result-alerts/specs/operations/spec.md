## ADDED Requirements

### Requirement: Result file of a weekly backup check
Each run of `restore-test` and `basebackup-once` that holds its lock SHALL write its result file at its end. The file SHALL hold the result `passed` or `failed` and the end time in UTC. It SHALL hold no path and no secret. A reader SHALL never see a partial file. A run without the marker file or without its lock SHALL write no result file (R31, A16, A35, ADR-0151).

#### Scenario: Restore test that passes
- **GIVEN** a complete set in the `backups` volume and a scratch cluster that restores it
- **WHEN** `restore-test` ends with the exit code 0
- **THEN** `/backups/restore-test.result` holds the result `passed` and the end time of the run

#### Scenario: Restore test that fails
- **GIVEN** a complete set whose row counts differ from the restored tables
- **WHEN** `restore-test` ends with an exit code other than 0
- **THEN** `/backups/restore-test.result` holds the result `failed` and the end time of the run

#### Scenario: Base backup with a failed check
- **GIVEN** a new base backup that `pg_verifybackup` rejects
- **WHEN** `basebackup-once` ends
- **THEN** `/wal/base-backup.result` holds the result `failed` and the end time of the run
- **AND** the run deletes no base backup and no WAL segment

#### Scenario: Run without its lock
- **GIVEN** a restore test that holds the lock of the restore test
- **WHEN** a second `restore-test` starts
- **THEN** the second run writes no result file

### Requirement: Result of a run that a signal ends
A run of `restore-test` or `basebackup-once` that holds its lock and gets a `TERM`, `INT`, or `HUP` signal SHALL write the result `failed`. It SHALL never write the result `passed` (R31, ADR-0151).

#### Scenario: Base backup that a signal ends
- **GIVEN** a `basebackup-once` run that holds the lock of the WAL archive
- **WHEN** the run gets a `TERM` signal
- **THEN** `/wal/base-backup.result` holds the result `failed`

### Requirement: Read right in the WAL archive directory
The top directory of `walarchive` SHALL have the mode `0711`. `app` SHALL open the result file by its name and SHALL NOT list the names in that directory (R31, A35, ADR-0151).

#### Scenario: Mode of the WAL archive directory
- **GIVEN** a WAL archive disk with the marker file and a top directory with the mode `0700`
- **WHEN** `walarchive-entry` starts
- **THEN** the top directory has the mode `0711`

### Requirement: Alerts of the restore test and the base backup
The job `alerts` SHALL test the conditions `restore_test` and `base_backup` after `backup_age`. A condition SHALL be active on the result `failed` and on a result older than its limit of `reliability.md` §7.11. A missing file SHALL make it active after the start grace. A passed result inside the limit SHALL keep it inactive (R31, R37, A16, A35, ADR-0151).

#### Scenario: Passed result inside the limit
- **GIVEN** `/backups/restore-test.result` with the result `passed` and an end time 2 days ago
- **WHEN** the job `alerts` runs
- **THEN** the event log holds no alert `restore_test`

#### Scenario: Failed result
- **GIVEN** `/wal/base-backup.result` with the result `failed` and an end time 1 hour ago
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line with the field `alert` `base_backup`
- **AND** the webhook gets the body `{"alert":"base_backup"}`

#### Scenario: Result older than the limit
- **GIVEN** `/backups/restore-test.result` with the result `passed` and an end time 9 days ago
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line with the field `alert` `restore_test`

#### Scenario: Missing result after the start grace
- **GIVEN** an installation whose first start of `app` was 30 days ago and no file `/wal/base-backup.result`
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line with the field `alert` `base_backup`

#### Scenario: Missing result on a new installation
- **GIVEN** a new installation whose first start of `app` was 1 day ago and no file `/backups/restore-test.result`
- **WHEN** the job `alerts` runs
- **THEN** the event log holds no alert `restore_test`

### Requirement: Read fault of a result file
A result file that the job `alerts` cannot read or parse SHALL make its condition active. This rule SHALL also apply during the start grace. The job SHALL write one `WARN` line of the fault (R31, R37, ADR-0073, ADR-0151).

#### Scenario: Malformed result file
- **GIVEN** `/backups/restore-test.result` with the result `ok`
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line of the fault
- **AND** the event log holds one `WARN` line with the field `alert` `restore_test`

#### Scenario: Read fault during the start grace
- **GIVEN** a new installation whose first start of `app` was 1 day ago
- **AND** a file `/wal/base-backup.result` that `app` cannot read
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line of the fault
- **AND** the event log holds one `WARN` line with the field `alert` `base_backup`

### Requirement: Comparison time of a result
The job `alerts` SHALL compare a result time with `database_now` plus the time of the run up to the end of the read. A result time later than that comparison time by more than `alerts.interval` SHALL make the condition active (R31, ADR-0093, ADR-0151).

#### Scenario: Result written during the reads of the run
- **GIVEN** reads of the run that take 50 seconds before the read of `/backups/restore-test.result`
- **AND** a result `passed` with an end time 100 seconds after the database time of the snapshot
- **WHEN** the job `alerts` runs
- **THEN** the event log holds no alert `restore_test`

#### Scenario: Result time ahead of the clock
- **GIVEN** a result `passed` with an end time ahead of the comparison time by more than `alerts.interval`
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line with the field `alert` `restore_test`

### Requirement: Time limit of the reads of the backup volumes
One scan of the backup directory and one read of a result file SHALL end after `health.disk_check_interval` at most. A read that reaches this limit SHALL count as a read fault. While the call of a path still runs, a new read of that path SHALL count as a read fault at once. It SHALL start no second call (R30, R31, ADR-0133, ADR-0151).

#### Scenario: Stalled backup directory
- **GIVEN** a scan of the backup directory that does not return
- **WHEN** the job `alerts` runs
- **THEN** the run ends within `health.disk_check_interval` plus the time of the other conditions
- **AND** the event log holds one `WARN` line with the field `alert` `backup_age`

#### Scenario: Backup directory still stalled at the next run
- **GIVEN** a scan of the backup directory that still runs after the limit of the previous run
- **WHEN** the job `alerts` runs again
- **THEN** the run starts no second scan of the backup directory
- **AND** the event log holds one `WARN` line with the field `alert` `backup_age`
