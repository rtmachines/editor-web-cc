## ADDED Requirements

### Requirement: Load test in CI
CI SHALL run the Load level of `reliability.md` §6 against the Compose stack on `workflow_dispatch` and on a push to `main`. A login flood SHALL send 200 concurrent `POST /login` requests with random logins. The result SHALL hold exactly 200 answers with finite times, and no request SHALL wait more than 5 seconds. At least one request SHALL get `503 E-WEB-009` from the Argon2 semaphore. A failed bound SHALL fail the job (R30, R31).

#### Scenario: Trigger of the load test
- **GIVEN** the workflow `ci.yml` and a pull request
- **WHEN** CI runs for the pull request
- **THEN** the job `load` does not start
- **AND** the job `load` starts on a push to `main` and on `workflow_dispatch`

#### Scenario: Login flood within its bounds
- **GIVEN** the Compose stack of the job `load` with the Argon2 semaphore of 4 permits
- **AND** a new container `app` with 1 CPU and no failed login in `login_attempt`
- **WHEN** the job sends 200 concurrent `POST /login` requests with random logins through Caddy
- **THEN** the result holds 200 answers, and no request waits more than 5 seconds
- **AND** at least one request gets `503 E-WEB-009`

#### Scenario: Slow request in the flood
- **GIVEN** a flood result in which one request waits 6 seconds
- **WHEN** the job checks the result
- **THEN** the job fails and names the bound of 5 seconds

#### Scenario: Flood without a semaphore answer
- **GIVEN** a flood result without a `503` answer
- **WHEN** the job checks the result
- **THEN** the job fails and names the missing `503`

#### Scenario: Incomplete flood result
- **GIVEN** a flood result with 199 answers
- **WHEN** the job checks the result
- **THEN** the job fails and names the count of 200 answers

### Requirement: Cause of each 503 answer of the flood
The count of `503` answers of the flood SHALL equal the count of `E-WEB-009` log lines with the cause "password hash permit". The job SHALL read only the log lines of `app` in the time window of the flood. A different count SHALL fail the job (R30).

#### Scenario: Each 503 answer comes from the semaphore
- **GIVEN** a flood result with three `503` answers
- **AND** three log lines of `E-WEB-009` with the cause "password hash permit" in the time window of the flood
- **WHEN** the job checks the result
- **THEN** the check of the cause passes

#### Scenario: A 503 answer without the semaphore cause
- **GIVEN** a flood result with three `503` answers and two log lines of `E-WEB-009` with the cause "password hash permit"
- **WHEN** the job checks the result
- **THEN** the job fails and names both counts

### Requirement: Memory of the container app under load
The job of the load test SHALL read the memory of the container `app` after the timings and after the flood. The container SHALL stay under its memory limit of 512 MiB, with no OOM kill and no restart. A failed bound or a missing memory value SHALL fail the job (R30, R31).

#### Scenario: Memory limit exceeded
- **GIVEN** a load run in which the kernel kills a process of `app` for its memory limit
- **WHEN** the job checks the memory of `app`
- **THEN** the job fails and names the OOM kill

#### Scenario: Restart of the container
- **GIVEN** a load run in which the container `app` has a restart count of 1
- **WHEN** the job checks the memory of `app`
- **THEN** the job fails and names the restart

#### Scenario: Peak at the memory limit
- **GIVEN** a memory reading with `memory.peak` equal to 512 MiB
- **WHEN** the job checks the memory of `app`
- **THEN** the job fails and names the limit of 512 MiB

#### Scenario: Missing memory value
- **GIVEN** a memory reading in which `memory.peak` is `unavailable`
- **WHEN** the job checks the memory of `app`
- **THEN** the job fails and names the missing value

### Requirement: Tests of the CI scripts on a pull request
Each pull request SHALL run the unit tests and the structure checks of `deploy/ci/` in the job `ci-scripts`. A failed test or check SHALL fail the job (R31).

#### Scenario: CI script tests on a pull request
- **GIVEN** the workflow `ci.yml` and a pull request
- **WHEN** CI runs for the pull request
- **THEN** the job `ci-scripts` runs each unit test `deploy/ci/test_*.py` and the structure checks of `deploy/ci/check.sh`

#### Scenario: Failed unit test of a CI script
- **GIVEN** a pull request in which one test `deploy/ci/test_*.py` fails
- **WHEN** the job `ci-scripts` runs
- **THEN** the job fails

### Requirement: Request times at the size budget
The job of the load test SHALL seed a draft at the size budget on the Compose stack through the HTTP routes. The draft SHALL have 4 000 parameters, 256 000 leaves, and 32 000 limit rows before the first timing. `ApproveVersion`, `CreateVersion`, and the version page SHALL each finish in less than 10 seconds. A slower request or a request that is not successful SHALL fail the job (R30, A26).

#### Scenario: Version at the size budget
- **GIVEN** a draft with 4 000 parameters, 256 000 leaves, and 32 000 limit rows that the administrator edited
- **WHEN** a second user runs `ApproveVersion` on the draft
- **AND** the job runs `CreateVersion` from the `Approved` version
- **AND** the job opens the version page of the `Approved` version
- **THEN** each of the three requests finishes in less than 10 seconds

#### Scenario: Seed below the planned counts
- **GIVEN** a seeded draft with 31 999 limit rows
- **WHEN** the job checks the counts of the draft
- **THEN** the job fails before the first timing and names the count of limit rows

#### Scenario: Slow approval
- **GIVEN** a timing result in which `ApproveVersion` takes 10 seconds
- **WHEN** the job checks the result
- **THEN** the job fails and names `ApproveVersion` and the bound of 10 seconds

### Requirement: End-to-end smoke on the Compose stack
The job `compose` SHALL run a browser smoke suite through Caddy after its stack checks and before the stop of `app`. The suite SHALL create a machine, a recipe, and a parameter, and set limits. A second user who did not edit the draft SHALL approve it. The suite SHALL then export the version, check its content hash, and check the audit page (R04, R20, R27, A34).

#### Scenario: Flow of the smoke suite
- **GIVEN** the Compose stack of the job `compose` after its stack checks
- **WHEN** the smoke suite runs the flow through Caddy
- **THEN** the second user approves the draft, and the version gets the status `Approved`
- **AND** the export returns `200` with `loadable` set to `true`
- **AND** the SHA-256 of the RFC 8785 form of the hashed body equals `content_sha256`
- **AND** the audit page shows the entry of the approval

#### Scenario: Approval by the editor of the draft
- **GIVEN** a draft that the first user edited in the smoke suite
- **WHEN** the first user sends the approval of the draft
- **THEN** the approval fails with `E-RCP-015`
- **AND** the draft keeps the status `Draft`

#### Scenario: Failed smoke step
- **GIVEN** a smoke run in which one step fails
- **WHEN** the job `compose` reads the result
- **THEN** the job fails, and the trace of the failed test is a CI artifact

### Requirement: Passwords of the smoke suite
Each password that the smoke suite sets, other than the one-time password that the page shows, SHALL come from an environment variable. No step that types a password SHALL write a trace. No password SHALL be in the repository or in a CI artifact (R20, A34).

#### Scenario: Password missing from the environment
- **GIVEN** the smoke suite and no value in one of its password variables
- **WHEN** the suite starts
- **THEN** the suite stops before the first request and names the variable

#### Scenario: Steps that type a password
- **GIVEN** a smoke run in which a step after the setup project fails
- **WHEN** the job `compose` uploads the traces
- **THEN** no trace holds a step of the setup project `smoke-login`

#### Scenario: Password in a CI artifact
- **GIVEN** a trace or a report file of the smoke run that holds the value of a password
- **WHEN** the job scans the files before the upload
- **THEN** the job fails and uploads no file of the smoke run
