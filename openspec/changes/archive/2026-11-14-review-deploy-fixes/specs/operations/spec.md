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

### Requirement: Password files of the database init script
The `db` init script SHALL stop before its first change in each of four cases. The cases are a password file that is empty or holds a carriage return, a NUL byte, or more than one line (R04, R38, A1).

#### Scenario: Password file with a NUL byte
- **GIVEN** an empty volume of the service `db` and a password file of `recipe_app` that holds a NUL byte
- **WHEN** the init script runs
- **THEN** the script writes one line that names the file and exits with code 1
- **AND** the script creates no role and does not change `pg_hba.conf`
