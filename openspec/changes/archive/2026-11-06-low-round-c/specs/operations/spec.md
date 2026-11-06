## ADDED Requirements

### Requirement: Time limit of the disk measurement
One measurement of the free space of a volume SHALL end after `health.disk_check_interval` at most. A measurement that reaches this limit SHALL count as an unreadable volume. While the call of a volume still runs, a new measurement of that volume SHALL count as unreadable at once. It SHALL start no second call (R31, A24).

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

#### Scenario: Data volume answers again
- **GIVEN** the application in read-only mode after a stalled `statvfs` call on `pgdata`, and that call is over
- **WHEN** the job `disk_check` runs and the free space on `pgdata` is above `disk.resume_percent`
- **THEN** the application leaves read-only mode
