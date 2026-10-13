# operations

## Purpose

The operations capability describes how the recipe editor runs 24/7 on one Docker host. It covers the services, backups, health checks, timeouts, read-only mode, supervised jobs, and the event log.

## ADDED Requirements

### Requirement: Seven Compose services
The system SHALL run seven services on one Docker host: `proxy`, `backup-premigrate`, `migrate`, `app`, `db`, `backup`, and `walarchive`. The one-shot services `backup-premigrate` and `migrate` SHALL have `restart: no` (R04, A1).

#### Scenario: Start order of the services
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator runs `docker compose up -d`
- **THEN** Docker starts `db` first and waits until `db` is healthy
- **AND** Docker runs `backup-premigrate` until it finishes, before `migrate` starts
- **AND** Docker starts `app` only after `migrate` finishes with exit code 0
- **AND** Docker starts `proxy` last

### Requirement: Healthcheck and restart policy
Every long-running service SHALL have a healthcheck and the restart policy `always`. The `app` service SHALL run the healthcheck `GET http://localhost:9000/readyz` every 10 seconds. Docker SHALL mark `app` unhealthy after 3 failed checks (R04, A1).

#### Scenario: Failed readiness checks
- **GIVEN** the service `app` with a readiness endpoint that returns `not ready`
- **WHEN** Docker runs the healthcheck of `app` and the check fails 3 times
- **THEN** Docker marks `app` unhealthy

### Requirement: HTTPS access through the proxy
Users SHALL reach the application over HTTPS through the `proxy` service. The `proxy` service SHALL handle TLS and redirect HTTP to HTTPS. It SHALL return `404` for `/healthz*` and `/readyz*` (A18).

#### Scenario: Health path through the proxy
- **GIVEN** the service `proxy` in front of the service `app`
- **WHEN** a client sends `GET /healthz` to the public address
- **THEN** `proxy` returns `404`

### Requirement: Pinned database image
The `db` service SHALL use a pinned PostgreSQL 16 image. The `initdb` run SHALL enable data checksums. It SHALL set the collation `C` for `lc-collate` and `C.UTF-8` for `lc-ctype` (A15).

#### Scenario: New database volume
- **GIVEN** an empty volume of the service `db`
- **WHEN** the operator runs `docker compose up -d db`
- **THEN** the initdb run creates the cluster with data checksums enabled
- **AND** the initdb run sets the collation `C` for `lc-collate`

### Requirement: Forward-only migrations
Migrations SHALL be forward-only, and a changed checksum SHALL stop the migrator with an error. Only the service `migrate` SHALL hold the migrator secret. The service `app` SHALL NOT connect as `recipe_migrator` (R34).

#### Scenario: Changed migration file
- **GIVEN** the applied migration `0001_schema.sql` with a stored checksum
- **WHEN** the service `migrate` starts with a changed `0001_schema.sql`
- **THEN** the migrator stops with an error

### Requirement: Dump before a migration
The service `backup-premigrate` SHALL write a `pg_dump` with the suffix `_premigrate` before `migrate` applies a migration (R34).

#### Scenario: Dump before a release
- **GIVEN** a release with a new migration file
- **WHEN** the operator runs `docker compose up -d`
- **THEN** `backup-premigrate` writes the dump with the suffix `_premigrate`
- **AND** `migrate` runs after the dump finishes

### Requirement: Migrator lock and retries
The migrator SHALL take an advisory lock, so two migrators cannot run at the same time. It SHALL set `lock_timeout` to 2 seconds. It SHALL retry a migration that failed on SQLSTATE `55P03` up to 10 times, with a pause of 5 seconds (R31, R34).

#### Scenario: Lock timeout during a migration
- **GIVEN** a migration that waits longer than 2 seconds for a lock
- **WHEN** the service `migrate` runs the migration
- **THEN** the migrator runs the migration again after 5 seconds
- **AND** the migrator stops with an error after 10 failed retries

### Requirement: Application role check at startup
At startup, the service `app` SHALL check that `current_user` is `recipe_app`, is not a superuser, and owns no table. If a check fails, `app` SHALL refuse to serve (R30, R34).

#### Scenario: Application role is the migrator
- **GIVEN** the service `app` configured with the role `recipe_migrator`
- **WHEN** the service `app` starts
- **THEN** the role check fails
- **AND** `app` does not serve requests

### Requirement: First administrator
The subcommand `recipe-server create-admin` SHALL read the login from an argument and the password from stdin or from a secret file. It SHALL do nothing when an active Administrator exists. It SHALL set `must_change_password` on the new Administrator and SHALL write the audit entry `UserCreated` with an empty actor (R21, R34).

#### Scenario: First run of create-admin
- **GIVEN** a database with no active Administrator
- **WHEN** the operator runs `recipe-server create-admin` with the login `admin` and a password on stdin
- **THEN** the subcommand creates the Administrator `admin` with `must_change_password` set to true
- **AND** the audit log holds the action `UserCreated` with an empty actor

#### Scenario: Second run of create-admin
- **GIVEN** an active Administrator
- **WHEN** the operator runs `recipe-server create-admin` with a new login
- **THEN** the subcommand does nothing

### Requirement: Durability settings of PostgreSQL
The `db` service SHALL set `fsync`, `synchronous_commit`, and `full_page_writes` to `on`. The `db` service SHALL set `wal_level` to `replica` and `archive_mode` to `off`, because `walarchive` streams WAL with `pg_receivewal` (R31, A35).

#### Scenario: Commit durability
- **GIVEN** the service `db` with the settings of this requirement
- **WHEN** a command commits a change
- **THEN** PostgreSQL writes the commit to disk before the client gets the result

### Requirement: Nightly dump
The service `backup` SHALL write a dump at 02:00 UTC every day to the `backups` volume. The dump SHALL exclude the data of `session` and `login_attempt`. The same run SHALL write the roles with `pg_dumpall --globals-only`. Each dump SHALL have a SHA-256 manifest. The service SHALL keep 30 daily dumps (R31, A16).

#### Scenario: Nightly dump run
- **GIVEN** the service `backup` with a retention of 30 daily dumps
- **WHEN** the clock reaches 02:00 UTC
- **THEN** `backup` writes the dump `recipe_<YYYYMMDD_HHMM>.dump` and its `.sha256` manifest
- **AND** `backup` deletes older dumps only after a successful new dump

### Requirement: Continuous WAL archive
The service `walarchive` SHALL stream WAL to a bind mount on a second disk with `pg_receivewal`. It SHALL write a weekly base backup to the same disk. With the second disk, the recovery point objective SHALL be minutes, and the recovery time objective SHALL be 1 hour (R31, A35).

#### Scenario: Weekly base backup
- **GIVEN** the service `walarchive` with the second disk mounted
- **WHEN** the weekly base backup runs
- **THEN** `walarchive` writes the base backup to the second disk

### Requirement: Recovery point without the second disk
Without the second disk, the recovery point objective SHALL be 24 hours (A35).

#### Scenario: Host without the second disk
- **GIVEN** a host without the second disk
- **WHEN** the operator plans a recovery
- **THEN** the most recent recoverable state is up to 24 hours old

### Requirement: Restore test
The restore script SHALL run each week and before each release. The script SHALL restore the latest dump into a scratch database. It SHALL compare the row count of each table with the source (R31, A16).

#### Scenario: Restore test before a release
- **GIVEN** a dump in the `backups` volume
- **WHEN** the operator runs the restore script
- **THEN** the script restores the dump into a scratch database
- **AND** the script writes the result to the event log
- **AND** a failed restore test writes an `ERROR` event

### Requirement: Health endpoints on the internal port
The service `app` SHALL serve `/healthz` and `/readyz` on the internal port 9000 only. Both endpoints SHALL return the plain text `ok` or `not ready` and no version, host name, or check detail (R30).

#### Scenario: Health endpoint on the public port
- **GIVEN** the public listener of the service `app` on port 8080
- **WHEN** a client sends `GET /readyz` to port 8080
- **THEN** the service `app` serves `/readyz` only on port 9000

### Requirement: Readiness check
The endpoint `/readyz` SHALL return `not ready` when a pooled connection cannot run `SELECT 1` within 2 seconds. It SHALL also return `not ready` when the schema version does not match the binary. The endpoint SHALL cache its result for 5 seconds (R30).

#### Scenario: Database does not answer
- **GIVEN** the service `db` that does not answer
- **WHEN** a client sends `GET /readyz` on port 9000
- **THEN** the endpoint returns `not ready`

### Requirement: Graceful shutdown
On `SIGTERM`, the service `app` SHALL stop accepting new connections and SHALL let requests in flight finish. After 30 seconds, `app` SHALL close the pool and exit with code 0. The Compose setting `stop_grace_period` of `app` SHALL be 40 seconds (R30).

#### Scenario: Shutdown with a request in flight
- **GIVEN** the service `app` with one request in flight
- **WHEN** Docker sends `SIGTERM` to `app`
- **THEN** `app` stops accepting new connections
- **AND** `/readyz` returns `not ready`
- **AND** `app` exits with code 0 after the request finishes or after 30 seconds

### Requirement: Request timeout
The service `app` SHALL stop an HTTP request after 30 seconds and return `E-WEB-009` (R30).

#### Scenario: Slow request
- **GIVEN** an HTTP request to `app`
- **WHEN** the request runs for more than 30 seconds
- **THEN** the response has the code `E-WEB-009`

### Requirement: Statement timeout
A SQL statement that runs longer than 10 seconds SHALL stop and return `E-DB-002` (R30).

#### Scenario: Long statement
- **GIVEN** a SQL statement of a command
- **WHEN** the statement runs for more than 10 seconds
- **THEN** the command returns the code `E-DB-002`

### Requirement: Lock and pool timeouts
A lock wait longer than 2 seconds SHALL return `E-DB-001`. A wait for a pooled connection longer than 5 seconds SHALL return `E-DB-004` (R30).

#### Scenario: Lock wait
- **GIVEN** a command that waits for a row lock held by another transaction
- **WHEN** the wait lasts more than 2 seconds
- **THEN** the command returns the code `E-DB-001`

#### Scenario: Pool wait
- **GIVEN** all pooled connections of `app` in use
- **WHEN** a command waits for a connection for more than 5 seconds
- **THEN** the command returns the code `E-DB-004`

### Requirement: Panic in a request
A panic in an HTTP request SHALL give `E-WEB-010`. The open transaction SHALL roll back, and the process SHALL keep serving requests (R30, A41).

#### Scenario: Panic in a handler
- **GIVEN** a request whose handler panics
- **WHEN** the panic occurs
- **THEN** the response has the code `E-WEB-010`
- **AND** `app` serves the next request

### Requirement: Restart of a panicked job
A panic in a supervised job SHALL restart the job after 10 seconds (R30, A41).

#### Scenario: Panic in the job alerts
- **GIVEN** the job `alerts` that panics in one iteration
- **WHEN** 10 seconds pass after the panic
- **THEN** the job `alerts` runs again

### Requirement: Panic in a blocking task
A panic in a `spawn_blocking` task SHALL give `E-WEB-011` (R30, A41).

#### Scenario: Panic in password hashing
- **GIVEN** a `Login` request whose Argon2id hashing panics
- **WHEN** the `JoinError` reaches the use case
- **THEN** the response has the code `E-WEB-011`

### Requirement: Watchdog
The watchdog thread SHALL probe the internal health listener every 10 seconds. After 3 failed probes in 30 seconds, the watchdog SHALL stop the process with exit code 70. Docker SHALL restart the container (A41).

#### Scenario: Stalled runtime
- **GIVEN** the service `app` with a stalled runtime
- **WHEN** the watchdog gets 3 failed probes within 30 seconds
- **THEN** the watchdog writes one `ERROR` line
- **AND** the watchdog stops the process with exit code 70
- **AND** Docker restarts the container

### Requirement: Read-only mode start
The application SHALL enter read-only mode when the free space on `pgdata` falls below 5 % (A24).

#### Scenario: Low disk space
- **GIVEN** the free space on `pgdata` at 4 %
- **WHEN** the job `disk_check` measures the free space
- **THEN** the application enters read-only mode

### Requirement: Read-only mode end
The application SHALL leave read-only mode without a restart when the free space on `pgdata` rises above 10 % (A24).

#### Scenario: Free space recovers
- **GIVEN** the application in read-only mode and the free space at 11 %
- **WHEN** the job `disk_check` measures the free space
- **THEN** the application leaves read-only mode without a restart

### Requirement: Disk space warning
The application SHALL write a `WARN` event every 10 minutes when the free space on `pgdata` falls below 15 % (A24).

#### Scenario: Warning threshold
- **GIVEN** the free space on `pgdata` at 14 %
- **WHEN** 10 minutes pass
- **THEN** the event log holds a `WARN` event about the free space

### Requirement: No write in read-only mode
In read-only mode, a command that gets `E-WEB-006` SHALL write nothing (A24).

#### Scenario: Write command in read-only mode
- **GIVEN** the application in read-only mode
- **WHEN** a ProcessEngineer runs `CreateRecipe` with the reference number `R-100`
- **THEN** the system creates no recipe `R-100`
- **AND** the system writes no audit entry

### Requirement: Supervised jobs
The application SHALL run the jobs `cleanup`, `disk_check`, `audit_chain_check`, `heartbeat`, `clock_check`, and `alerts` inside the service `app` (R30).

#### Scenario: Job list
- **GIVEN** the stopped service `app`
- **WHEN** the service `app` starts
- **THEN** the six supervised jobs run inside `app`

### Requirement: Job intervals
The job `disk_check` SHALL run every 30 seconds. The job `heartbeat` SHALL write `heartbeat.last_at` in `app_setting` every 5 minutes, with no audit entry (R30).

#### Scenario: Interval of the heartbeat job
- **GIVEN** the job `heartbeat` in the running state
- **WHEN** 5 minutes pass
- **THEN** the job writes `heartbeat.last_at` in `app_setting`
- **AND** the job writes no audit entry

### Requirement: Cleanup job
The job `cleanup` SHALL delete expired sessions and login attempts older than 24 hours. The job SHALL delete login attempts in batches of 5 000 rows. Each batch SHALL run in its own transaction, and the job SHALL write no audit entry (R30).

#### Scenario: Old login attempts
- **GIVEN** 12 000 login attempts older than 24 hours
- **WHEN** the job `cleanup` runs
- **THEN** the job deletes the login attempts in batches of 5 000 rows
- **AND** the job writes no audit entry

### Requirement: Event log format
The application SHALL write one JSON object per line to stdout. Each HTTP response SHALL carry the `request_id` in the header `X-Request-Id` (R37).

#### Scenario: HTTP request logged
- **GIVEN** an HTTP request to the application
- **WHEN** the request finishes
- **THEN** the event log holds one JSON line with the field `request_id`
- **AND** the response holds the header `X-Request-Id` with the same value

### Requirement: Log level by HTTP status
The application SHALL log an input error from a user, HTTP 4xx, at level `INFO`. It SHALL log an HTTP 5xx response at level `ERROR` (R35).

#### Scenario: Input error
- **GIVEN** a form with a value outside its process limit
- **WHEN** the request returns the status `422`
- **THEN** the event log holds an `INFO` line for the request

### Requirement: One log line per error
The application SHALL log each error once, in the HTTP mapping of `recipe-web` or at the loop boundary of a job. A job SHALL add the field `job` to the log line of its error (R35).

#### Scenario: Storage error in a request
- **GIVEN** a request whose use case returns a storage error
- **WHEN** the error reaches the HTTP mapping
- **THEN** the event log holds one `ERROR` line for the error
- **AND** no lower layer writes a log line for the error

### Requirement: Redaction in the event log
The event log SHALL NOT hold passwords, password hashes, session tokens, cookies, CSRF values, or secrets (R37).

#### Scenario: Login request logged
- **GIVEN** a `POST /login` request with a password
- **WHEN** the application logs the request
- **THEN** the log line holds no password and no password hash

### Requirement: Secrets as files
The services SHALL get every password and key as a Docker secret file. The application SHALL read a secret only through a `*_FILE` variable. The key files of `login_hmac_key` and `csrf_signing_key` SHALL hold the newest key first. The server SHALL sign with the first key and SHALL accept every key in the file. After a rotation, the operator SHALL keep the old key for 24 hours (R04, R34).

#### Scenario: Key rotation
- **GIVEN** the key file of `csrf_signing_key` with a new key on the first line and the old key on the second line
- **WHEN** a browser sends a pre-session CSRF cookie that the old key signed
- **THEN** the server accepts the cookie
- **AND** the server signs each new cookie with the new key

### Requirement: Clock skew warning
The job `clock_check` SHALL compare the clock of `app` with the clock of `db` at startup and every hour. The job SHALL write a `WARN` event when the difference is above 5 seconds (R37).

#### Scenario: Skew of 7 seconds
- **GIVEN** a clock difference of 7 seconds between `app` and `db`
- **WHEN** the job `clock_check` runs
- **THEN** the event log holds a `WARN` event about the clock skew

### Requirement: Alert conditions
The job `alerts` SHALL check the alert conditions of `reliability.md` section 3.7 every 60 seconds. For each active condition, the job SHALL write one `WARN` line with the field `alert` (R37).

#### Scenario: Panic count above the limit
- **GIVEN** 4 panic events within 10 minutes
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line with the field `alert`

### Requirement: Startup settings comparison
At startup, the server SHALL compare each configured setting with the stored value in `app_setting` in one transaction. For each changed value, the server SHALL write one `SettingChanged` audit entry with `actor = None` (A34).

#### Scenario: Changed separation of duties
- **GIVEN** the stored value `true` of `approval.separation_of_duties`
- **WHEN** the operator runs `docker compose up -d app` with `approval.separation_of_duties` set to `false`
- **THEN** the audit log holds one `SettingChanged` entry with the old value `true` and the new value `false`

### Requirement: Failed startup settings comparison
If the startup settings comparison fails, the server SHALL write one `ERROR` line and SHALL NOT start (A34).

#### Scenario: Comparison transaction fails
- **GIVEN** a startup settings comparison whose transaction fails
- **WHEN** the service `app` starts
- **THEN** the server writes one `ERROR` line
- **AND** the server does not open its listeners
