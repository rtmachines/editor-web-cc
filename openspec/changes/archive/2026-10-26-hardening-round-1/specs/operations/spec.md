## ADDED Requirements

### Requirement: Role grammar of the globals file
The restore test SHALL check each line of the globals file before any PostgreSQL tool runs. A line with a control character other than a tab SHALL stop the test, also between the restrict pair and inside a comment. Between the pair, a line SHALL be a comment, a blank line, or one statement of the role grammar. Any other line SHALL stop the test with an `ERROR` event that names the line number (R31, R38, A16).

#### Scenario: Globals file with a program call
- **GIVEN** a set whose globals file holds the line `COPY (SELECT 1) TO PROGRAM 'id';` between `\restrict` and `\unrestrict`
- **AND** the manifest of the set matches its files
- **WHEN** the restore test runs
- **THEN** the test stops before any PostgreSQL tool runs
- **AND** the test writes an `ERROR` event that names the role grammar and the line number

#### Scenario: Carriage return in a comment
- **GIVEN** a set whose globals file holds, between the pair, one line with the comment `-- note`, a carriage return byte, and `COPY (SELECT 1) TO PROGRAM 'id';`
- **WHEN** the restore test runs
- **THEN** the test stops before any PostgreSQL tool runs
- **AND** the test writes an `ERROR` event that names the control character

#### Scenario: Role setting outside the list
- **GIVEN** a set whose globals file holds the line `ALTER ROLE recipe_app SET session_preload_libraries TO 'x';`
- **WHEN** the restore test runs
- **THEN** the test stops before any PostgreSQL tool runs and writes an `ERROR` event

#### Scenario: Two statements on one line
- **GIVEN** a set whose globals file holds the line `CREATE ROLE a; COPY (SELECT 1) TO PROGRAM 'id';`
- **WHEN** the restore test runs
- **THEN** the test stops before any PostgreSQL tool runs and writes an `ERROR` event

#### Scenario: Globals file of the release
- **GIVEN** a set that `backup-once` wrote from a database after `01-roles.sh` and every migration
- **WHEN** the restore test runs
- **THEN** the globals file passes the check
- **AND** the test restores the roles and logs in with each login role

### Requirement: Guard function of an older volume
The upgrade of an older volume SHALL replace the audit guard function after `migrate` exits with code 0 and before the first new backup. The SHA-256 of the new function SHALL equal its line in the `restore-code.expected` of the new image. The rollback set `_premigrate` SHALL hold the old guard function (R31, A16).

#### Scenario: Upgrade of an older volume
- **GIVEN** a volume of the previous release and its newest complete set
- **WHEN** the operator runs the upgrade of the runbook with the manual superuser step
- **THEN** the set `_premigrate` holds the old guard function
- **AND** the restore test of the new image passes on the new `_now` set

#### Scenario: Migration failure before the guard step
- **GIVEN** a volume of the previous release
- **WHEN** `migrate` fails during the upgrade
- **THEN** the guard function stays old
- **AND** the restore test of the previous image passes on the newest set
