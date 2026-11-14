## MODIFIED Requirements

### Requirement: Roles from the database init script
The `db` init script SHALL create the roles `recipe_migrator`, `recipe_app`, `recipe_backup`, `recipe_replicator`, and `audit_owner`, and the database `recipe`, once, on an empty volume. It SHALL read each password from a secret file. It SHALL stop before its first change in each of four cases. The cases are a password file that is empty, holds a carriage return, holds a NUL byte, or holds more than one line. It SHALL revoke `CONNECT` and `TEMPORARY` of `PUBLIC` on `recipe`, `postgres`, and `template1`. `recipe_app` SHALL NOT hold `pg_read_all_stats`. Only this script and `restore-volume` SHALL create a role in a production cluster (R04, R34, R38, A1).

#### Scenario: First start on an empty volume
- **GIVEN** an empty volume of the service `db` and the four password secret files
- **WHEN** the service `db` starts for the first time
- **THEN** the five roles and the database `recipe` exist
- **AND** `PUBLIC` holds no `CONNECT` and no `TEMPORARY` privilege on `recipe`

#### Scenario: Password file with a NUL byte
- **GIVEN** an empty volume of the service `db` and a password file of `recipe_app` that holds a NUL byte
- **WHEN** the init script runs
- **THEN** the script writes one line that names the file and exits with code 1
- **AND** the script creates no role and does not change `pg_hba.conf`

#### Scenario: Replication role
- **GIVEN** the database `recipe` after the first start
- **WHEN** `recipe_replicator` connects to `recipe` as a normal session
- **THEN** the database rejects the connection
- **AND** `recipe_replicator` holds no privilege on any table

#### Scenario: Normal session on the maintenance database
- **GIVEN** the database `postgres` after the first start
- **WHEN** `recipe_app` or `recipe_replicator` connects to `postgres` as a normal session
- **THEN** the database rejects the connection

#### Scenario: Statistics of other sessions
- **GIVEN** a session of `recipe_app`
- **WHEN** the session reads the column `query` of `pg_stat_activity` for a session of another role
- **THEN** the column holds no statement text

#### Scenario: Cluster of the restore into a new volume
- **GIVEN** a volume that `restore-volume` filled from a backup set
- **WHEN** the service `db` starts on that volume for the first time
- **THEN** the init script does not run
- **AND** the roles and the database `recipe` are those of the set

## ADDED Requirements

### Requirement: Check with the release toolchain on a pull request
Each pull request SHALL check the package `recipe-server` with `cargo check --release` in the job `release-toolchain`. The job SHALL use the toolchain version that `deploy/Dockerfile` sets in `RUSTUP_TOOLCHAIN` (ADR-0064 (d)). An error of the check SHALL fail the job (R31, R34).

#### Scenario: Release toolchain check on a pull request
- **GIVEN** the workflow `ci.yml` and a pull request
- **WHEN** CI runs for the pull request
- **THEN** the job `release-toolchain` checks `recipe-server` with the version of `RUSTUP_TOOLCHAIN` in `deploy/Dockerfile`

#### Scenario: Code that needs a newer toolchain
- **GIVEN** a pull request with code that passes `cargo check` with the current stable toolchain and fails it with the version of `deploy/Dockerfile`
- **WHEN** the job `release-toolchain` runs
- **THEN** the job fails

#### Scenario: No pinned version in the image file
- **GIVEN** a `deploy/Dockerfile` without exactly one version in `RUSTUP_TOOLCHAIN`
- **WHEN** the job `release-toolchain` runs
- **THEN** the job fails before the check
