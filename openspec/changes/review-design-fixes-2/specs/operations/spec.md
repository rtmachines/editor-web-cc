## MODIFIED Requirements

### Requirement: Checks of a recovered cluster
After the promotion, `pitr-volume` SHALL check the recovered cluster before the clean stop. A failed check SHALL end the run. The checks SHALL cover the end of the replay, the timeline, the audit chain, and an optional marker entry. Without a target time, the end of the replay SHALL be the end of the archived records. After the record at the end of the replay, the newest segment SHALL hold only zero bytes. A clean end SHALL pass without a `WARN` line. A torn last record with only zero bytes after it SHALL pass with one `WARN` line. That line SHALL say that the newest record may be lost. The checks SHALL also cover the configuration files and the roles of ADR-0103 (m). The last line SHALL hold the values of ADR-0106 (d) (R31, R29, A35, ADR-0217).

#### Scenario: Gap in the archive
- **GIVEN** a WAL archive with a missing segment between the base backup and the newest segment
- **WHEN** `pitr-volume` runs without a target time
- **THEN** the script writes the `ERROR` line `the recovery ended before the end of the WAL archive`

#### Scenario: Damaged record in the newest segment
- **GIVEN** a WAL archive whose newest segment holds a damaged record before the newest commit
- **WHEN** `pitr-volume` runs without a target time and without `db`
- **THEN** the script writes the `ERROR` line `the recovery ended before the end of the WAL archive`
- **AND** the script ends with a code other than 0 and writes no line `pitr volume completed`

#### Scenario: Torn last record after a crash
- **GIVEN** a WAL archive whose newest segment ends with a torn record and only zero bytes after it
- **WHEN** `pitr-volume` runs without a target time and without `db`
- **THEN** the check of the end passes and writes one `WARN` line that says that the newest record may be lost
- **AND** the recovered cluster holds every commit before the torn record

#### Scenario: Clean end of the archive
- **GIVEN** a WAL archive whose newest segment holds only zero bytes after its last complete record
- **WHEN** `pitr-volume` runs without a target time and without `db`
- **THEN** the check of the end passes and writes no `WARN` line

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

## ADDED Requirements

### Requirement: Failed stop of a scratch server
`restore-test` and `pitr-check` SHALL delete the directory of a scratch cluster only after a successful stop of its server. A failed stop SHALL keep the directory, write one `ERROR` line, and end the run with a code other than 0. A failed stop of a stale scratch server SHALL end the run before `initdb` (R30, R31, R35, ADR-0218).

#### Scenario: Failed stop at the end of a restore test
- **GIVEN** a restore test whose scratch server does not stop
- **WHEN** the trap of `restore-test` runs
- **THEN** the directory of the scratch cluster stays
- **AND** the script writes one `ERROR` line and ends with a code other than 0

#### Scenario: Failed stop of a stale scratch server
- **GIVEN** a stale scratch directory of an earlier restore test whose server does not stop
- **WHEN** `restore-test` starts
- **THEN** the stale directory stays and the script runs no `initdb`
- **AND** the result of the run is `failed`

#### Scenario: Failed stop after a passed WAL check
- **GIVEN** a `pitr-check` run that writes `pitr check passed` and whose scratch server does not stop
- **WHEN** the trap of `pitr-check` runs
- **THEN** the directory of the scratch cluster stays
- **AND** the script ends with a code other than 0

### Requirement: Manifest time ahead of the clock
For each complete set that it reads, `backup-health` SHALL treat a manifest time later than its clock by more than `alerts.interval` as untrusted. An untrusted time SHALL fail the healthcheck. A time ahead of the clock by `alerts.interval` or less SHALL count as age 0 (R31, R35, ADR-0093, ADR-0219).

#### Scenario: Manifest time far ahead of the clock
- **GIVEN** the newest manifest with a modification time 61 seconds after the clock of `backup`
- **WHEN** `backup-health` runs
- **THEN** the healthcheck fails

#### Scenario: Manifest time a little ahead of the clock
- **GIVEN** the newest manifest with a modification time 60 seconds after the clock of `backup`, and every other check passes
- **WHEN** `backup-health` runs
- **THEN** the healthcheck passes

#### Scenario: Older set ahead of the clock
- **GIVEN** an older complete set with a manifest time 61 seconds after the clock of `backup`, and a newest set that passes every check
- **WHEN** `backup-health` runs
- **THEN** the healthcheck fails
