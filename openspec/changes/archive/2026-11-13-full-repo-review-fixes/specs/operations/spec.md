## MODIFIED Requirements

### Requirement: Time limit of the disk measurement
One measurement of the free space of a volume SHALL end after `health.disk_check_interval` at most. A measurement that reaches this limit SHALL count as an unreadable volume. While a call of a volume that reached the limit still runs, a new measurement of that volume SHALL count as unreadable at once. It SHALL start no second call. A volume SHALL hold at most one stalled call for each job that measures it (R31, A24, ADR-0161).

#### Scenario: Stalled data volume
- **GIVEN** the running service `app` with read-only mode off, and a `statvfs` call on `pgdata` that does not return
- **WHEN** the job `disk_check` runs
- **THEN** the run ends within `health.disk_check_interval`
- **AND** the application enters read-only mode

#### Scenario: Data volume still stalled at the next run
- **GIVEN** a `statvfs` call on `pgdata` that still runs after the limit of the previous run
- **WHEN** the job `disk_check` runs again
- **THEN** the run starts no second `statvfs` call on `pgdata`
- **AND** the application stays in read-only mode

#### Scenario: Two jobs measure a stalled data volume
- **GIVEN** a data volume `pgdata` whose `statvfs` calls do not return
- **WHEN** the jobs `disk_check` and `alerts` both start a measurement of `pgdata` before the limit of the other
- **THEN** each job holds one stalled `statvfs` call on `pgdata`
- **AND** the next run of each job starts no new `statvfs` call on `pgdata`

#### Scenario: Data volume answers again
- **GIVEN** the application in read-only mode after a stalled `statvfs` call on `pgdata`, and that call is over
- **WHEN** the job `disk_check` runs and the free space on `pgdata` is above `disk.resume_percent`
- **THEN** the application leaves read-only mode

### Requirement: Time limit of the reads of the backup volumes
One scan of the backup directory and one read of a result file SHALL each end after `health.disk_check_interval` at most. A read that reaches this limit SHALL count as a read fault. While a call of a path that reached the limit still runs, a new read of it SHALL count as a read fault at once. It SHALL start no second call. A path SHALL hold at most one stalled call for each job that reads it. A free-space measurement of the path is a separate call (R30, R31, ADR-0133, ADR-0151, ADR-0161).

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
