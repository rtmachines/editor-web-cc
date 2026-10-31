# operations Specification

## Purpose
The operations capability describes how the recipe editor runs 24/7 on one Docker host. It covers the services, backups, health checks, timeouts, read-only mode, supervised jobs, and the event log.

## Requirements

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
Users SHALL reach the application over HTTPS through the `proxy` service. The `proxy` service SHALL handle TLS and redirect HTTP to HTTPS. In production, it SHALL read its certificate and key from secret files. It SHALL return `404` for `/healthz*` and `/readyz*` (A18).

#### Scenario: Health path through the proxy
- **GIVEN** the service `proxy` in front of the service `app`
- **WHEN** a client sends `GET /healthz` to the public address
- **THEN** `proxy` returns `404`

#### Scenario: Readiness path through the proxy
- **GIVEN** the service `proxy` in front of the service `app`
- **WHEN** a client sends `GET /readyz?x=1` to the public address
- **THEN** `proxy` returns `404`

#### Scenario: Plain HTTP request
- **GIVEN** the service `proxy` with a site certificate
- **WHEN** a client sends `GET /login` over plain HTTP
- **THEN** `proxy` answers with a permanent redirect to the same path over HTTPS
- **AND** the answer holds CSP, `nosniff`, and `Referrer-Policy`, but no HSTS

#### Scenario: HTMX request while the application restarts
- **GIVEN** the service `proxy` in front of the service `app`, and `app` does not answer
- **WHEN** a client sends a request with `HX-Request: true` to the public address
- **THEN** `proxy` returns `502` with `HX-Retarget: #error-panel` and `HX-Push-Url: false`
- **AND** the answer holds HSTS, CSP, `nosniff`, and `Referrer-Policy`, and no `Server` header

#### Scenario: Variant of a health path
- **GIVEN** the service `proxy` in front of the service `app`
- **WHEN** a client sends `GET /HEALTHZ`, `GET /healthz/`, or `GET /readyz%2F` to the public address
- **THEN** `proxy` returns `404` for each request

### Requirement: Pinned database image
The `db` service SHALL use a pinned PostgreSQL 16 image. The `initdb` run SHALL enable data checksums. It SHALL set the collation `C` for `lc-collate` and `C.UTF-8` for `lc-ctype` (A15).

#### Scenario: New database volume
- **GIVEN** an empty volume that the operator created with `docker volume create`
- **AND** the value `RECIPE_PGDATA_VOLUME` of `deploy/.env` names that volume
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
`recipe-server create-admin` SHALL read the login from `--login` and the password from stdin or from the file of `--password-file`. It SHALL connect as the role of `db.app_user`. It SHALL do nothing when an active Administrator exists. It SHALL check the password policy, set `must_change_password`, and write `UserCreated` with an empty actor (R21, R34, A23).

#### Scenario: First run of create-admin
- **GIVEN** a database with no active Administrator
- **WHEN** the operator runs `recipe-server create-admin` with the login `admin` and a password on stdin
- **THEN** the subcommand creates the Administrator `admin` with `must_change_password` set to true
- **AND** the audit log holds the action `UserCreated` with an empty actor

#### Scenario: Second run of create-admin
- **GIVEN** an active Administrator
- **WHEN** the operator runs `recipe-server create-admin` with a new login
- **THEN** the subcommand does nothing
- **AND** the subcommand writes one `INFO` line and exits with code 0

#### Scenario: Password outside the policy
- **GIVEN** a database with no active Administrator
- **WHEN** the operator runs `recipe-server create-admin` with the login `admin` and a password of 11 characters on stdin
- **THEN** the subcommand creates no user
- **AND** the subcommand writes one `ERROR` line with `E-IDN-006` and exits with code 1

#### Scenario: Password from a terminal
- **GIVEN** a database with no active Administrator
- **WHEN** the operator runs `recipe-server create-admin` with the login `admin` and stdin connected to a terminal
- **THEN** the subcommand reads no password, creates no user, and exits with code 1

#### Scenario: Password file with two lines
- **GIVEN** a password file with two lines
- **WHEN** the operator runs `recipe-server create-admin` with the login `admin` and `--password-file` set to that file
- **THEN** the subcommand creates no user and exits with code 1
- **AND** the output holds no line of the file

### Requirement: Durability settings of PostgreSQL
The `db` service SHALL set `fsync`, `synchronous_commit`, and `full_page_writes` to `on`. The `db` service SHALL set `wal_level` to `replica` and `archive_mode` to `off`, because `walarchive` streams WAL with `pg_receivewal` (R31, A35).

#### Scenario: Commit durability
- **GIVEN** the service `db` with the settings of this requirement
- **WHEN** a command commits a change
- **THEN** PostgreSQL writes the commit to disk before the client gets the result

### Requirement: Nightly dump
The service `backup` SHALL write a dump at 02:00 UTC every day to the `backups` volume. The dump SHALL exclude the data of `session` and `login_attempt`. The same run SHALL write the roles with `pg_dumpall --globals-only` and the row count of each table except these two. One SHA-256 manifest SHALL cover the files of the set. After a successful dump, the service SHALL delete the sets older than 30 days (R31, A16).

#### Scenario: Nightly dump run
- **GIVEN** the service `backup` and the clock at 02:00 UTC
- **WHEN** the dump run ends without an error
- **THEN** `backup` has written `recipe_<YYYYMMDD_HHMM>.dump`, the globals file, the counts file, and the manifest `recipe_<YYYYMMDD_HHMM>.dump.sha256` last
- **AND** `backup` deletes the sets older than 30 days only after the new set is complete

#### Scenario: Failed dump run
- **GIVEN** the service `backup` with sets older than 30 days and a database that does not answer
- **WHEN** the dump run fails
- **THEN** `backup` writes an `ERROR` event
- **AND** every old set stays

#### Scenario: Newest sets stay
- **GIVEN** a newest daily set and a newest `_premigrate` set, both older than 30 days
- **WHEN** a dump run deletes the old sets
- **THEN** both newest sets stay

#### Scenario: Name of the new set exists
- **GIVEN** a file in the `backups` volume with a name of the new set
- **WHEN** the dump run starts
- **THEN** `backup` writes an `ERROR` event
- **AND** no file in the `backups` volume changes

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
The service `backup` SHALL run the restore test every Sunday at 05:00 UTC, on demand before each release, and on demand in steps 5 and 6 of the upgrade procedure. The test SHALL check the manifest and restore the newest set into a private scratch cluster and SHALL write one result line to the event log (R31, A16).

#### Scenario: Restore test before a release
- **GIVEN** a complete set in the `backups` volume
- **WHEN** the operator runs the restore test
- **THEN** the test restores the set into a scratch cluster that is not the production cluster
- **AND** the test writes the result to the event log
- **AND** a failed restore test writes an `ERROR` event

#### Scenario: Restore test after an upgrade
- **GIVEN** an upgrade whose migration adds a function to `restore-code.expected`
- **AND** the operator gives `backup` the image of the new release
- **WHEN** the operator runs `backup-now` and then the restore test
- **THEN** the test selects the `_now` set and passes

#### Scenario: Manifest mismatch
- **GIVEN** a set whose dump file differs from its manifest
- **WHEN** the restore test runs
- **THEN** the test restores nothing and writes an `ERROR` event

#### Scenario: Changed function body
- **GIVEN** a set whose dump holds a known function with a changed body
- **AND** no generated column of the dump calls that function
- **WHEN** the restore test runs
- **THEN** the test stops before the data load and writes an `ERROR` event
- **AND** the changed function does not run

#### Scenario: Unknown trigger in the dump
- **GIVEN** a set whose dump holds a trigger that `restore-code.expected` does not hold
- **WHEN** the restore test runs
- **THEN** the test stops before the data load and writes an `ERROR` event

#### Scenario: Changed trigger definition
- **GIVEN** a set whose dump holds a known trigger with a changed definition
- **WHEN** the restore test runs
- **THEN** the test stops before the row counts and writes an `ERROR` event

#### Scenario: Refused kind
- **GIVEN** a set whose dump holds an entry of the kind `PROCEDURE`
- **WHEN** the restore test runs
- **THEN** the test stops before the data load and writes an `ERROR` event
- **AND** the test restores nothing

#### Scenario: Aggregate, materialized view, or operator in the dump
- **GIVEN** a set whose dump holds an aggregate, a materialized view, or an operator
- **WHEN** the restore test runs
- **THEN** the test stops before the data load

#### Scenario: Changed check constraint or generated column
- **GIVEN** a set whose dump holds a `CHECK` constraint or a generated column with code that differs from `restore-code.expected`
- **WHEN** the restore test runs
- **THEN** the test stops before the data load

#### Scenario: Unknown event trigger in the dump
- **GIVEN** a set whose dump holds an event trigger that `restore-code.expected` does not hold
- **WHEN** the restore test runs
- **THEN** the test stops before the data load and writes an `ERROR` event

#### Scenario: Changed trigger after the post-data restore
- **GIVEN** a set whose dump holds a trigger with code or enabled state that differs from `restore-code.expected`
- **WHEN** the restore of `post-data` completes
- **THEN** the restore test fails

#### Scenario: Row counts, audit chain, and login roles
- **GIVEN** a complete set with a counts file
- **WHEN** the restore test runs
- **THEN** the test compares the row counts with the counts file
- **AND** the test runs the audit chain check
- **AND** the test logs in with each login role

### Requirement: Health endpoints on the internal port
The service `app` SHALL serve `/healthz` and `/readyz` on the internal port 9000 only. Both endpoints SHALL return the plain text `ok` with the status 200, or the plain text `not ready` with the status 503. They SHALL return no version, host name, or check detail (R30).

#### Scenario: Health endpoint on the public port
- **GIVEN** the public listener of the service `app` on port 8080
- **WHEN** a client sends `GET /readyz` to port 8080
- **THEN** the response has the status 404

#### Scenario: Status of a readiness failure
- **GIVEN** the service `app` whose database does not answer
- **WHEN** a client sends `GET /readyz` on port 9000
- **THEN** the response has the status 503 and the body `not ready`

### Requirement: Readiness check
The endpoint `/readyz` SHALL return `not ready` when a pooled connection cannot run `SELECT 1` within 2 seconds. It SHALL also return `not ready` when a migration of the binary is missing in the database, has another checksum, or when a failed migration row exists. A database with a newer migration SHALL stay ready. The endpoint SHALL cache its result for 5 seconds (R30, R34).

#### Scenario: Database does not answer
- **GIVEN** the service `db` that does not answer
- **WHEN** a client sends `GET /readyz` on port 9000
- **THEN** the endpoint returns `not ready`

#### Scenario: Previous image on a newer schema
- **GIVEN** a database with the migrations `0001` to `0007` and an `app` binary that holds `0001` to `0006`
- **WHEN** a client sends `GET /readyz` on port 9000
- **THEN** the endpoint returns `ok`

#### Scenario: Changed checksum of an applied migration
- **GIVEN** a database whose row of `0002` has a checksum that differs from the binary
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
After 30 seconds, the service `app` SHALL stop an HTTP request and return `E-WEB-009`. This limit SHALL apply to every path except the export paths under `/export/v1/`, which keep their own limits (R30, ADR-0072).

#### Scenario: Slow request
- **GIVEN** an HTTP request to `app` on a path outside `/export/v1/`
- **WHEN** the request runs for more than 30 seconds
- **THEN** the response has the code `E-WEB-009`

#### Scenario: Export outlasts the request timeout
- **GIVEN** a request to a path under `/export/v1/` whose handler commits its `VersionExported` entry after 30 seconds
- **WHEN** the handler returns its answer
- **THEN** the client gets the answer of the handler
- **AND** the response does not have the code `E-WEB-009`

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
A panic in an HTTP request on the public listener SHALL give `E-WEB-010` with the `request_id` of the request. The open transaction SHALL roll back, and the process SHALL keep serving requests (R30, A41). The health router has no `CatchPanicLayer`. A panic in a health request SHALL end only its connection. The connection task SHALL contain that panic (`reliability.md` §2.4 rule 6).

#### Scenario: Panic in a handler
- **GIVEN** a request on the public listener whose handler panics
- **WHEN** the panic occurs
- **THEN** the response has the code `E-WEB-010`
- **AND** the body holds the same `request_id` as the header `X-Request-Id`
- **AND** `app` serves the next request

#### Scenario: Panic on an export route
- **GIVEN** a request to a path under `/export/v1/` whose handler panics
- **WHEN** the panic occurs
- **THEN** the response is a JSON object with the code `E-WEB-010`, a message, and the `request_id`

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
The application SHALL enter read-only mode when the free space on `pgdata` falls below 5 %. It SHALL also enter read-only mode when the job `disk_check` cannot measure the free space on `pgdata` on a system with `statvfs`. This rule SHALL also hold for the first run at startup (R31, A24, ADR-0073).

#### Scenario: Low disk space
- **GIVEN** the free space on `pgdata` at 4 %
- **WHEN** the job `disk_check` measures the free space
- **THEN** the application enters read-only mode

#### Scenario: Unreadable data volume
- **GIVEN** the running service `app` with read-only mode off, and a `statvfs` call on `pgdata` that fails
- **WHEN** the job `disk_check` runs
- **THEN** the application enters read-only mode
- **AND** the event log holds one `WARN` line of the fault

#### Scenario: Unreadable data volume at startup
- **GIVEN** a `statvfs` call on `pgdata` that fails
- **WHEN** the service `app` starts and the first request is `POST /recipes`
- **THEN** the server opens its listeners
- **AND** the response has the code `E-WEB-006`

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
The application SHALL write one JSON object per line to stdout. Each HTTP response SHALL carry the `request_id` in the header `X-Request-Id`. This includes the responses of the internal health router, which SHALL log the finish line of a request at the level `DEBUG` (R37).

#### Scenario: HTTP request logged
- **GIVEN** an HTTP request to the application
- **WHEN** the request finishes
- **THEN** the event log holds one JSON line with the field `request_id`
- **AND** the response holds the header `X-Request-Id` with the same value

#### Scenario: Health request
- **GIVEN** the service `app` with the log filter `info`
- **WHEN** Docker sends `GET /healthz` to port 9000
- **THEN** the response holds the header `X-Request-Id` and the security headers of `api.md` section 9
- **AND** the event log holds no line for the request

### Requirement: Log level by HTTP status
The application SHALL log an input error from a user, HTTP 4xx, at level `INFO`. It SHALL log an HTTP 5xx response at level `ERROR`. The finish line of the internal health router SHALL have the level `DEBUG` for every status (R35).

#### Scenario: Input error
- **GIVEN** a form with a value outside its process limit
- **WHEN** the request returns the status `422`
- **THEN** the event log holds an `INFO` line for the request

#### Scenario: Readiness failure
- **GIVEN** the service `app` whose database does not answer
- **WHEN** Docker sends `GET /readyz` to port 9000 and gets the status 503
- **THEN** the event log holds no `ERROR` line for the request

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
The job `alerts` SHALL check the alert conditions of `reliability.md` section 3.7 every 60 seconds. For each active condition, the job SHALL write one `WARN` line with the field `alert` (R37, ADR-0073, ADR-0075).

#### Scenario: Panic count above the limit
- **GIVEN** 4 panic events within 10 minutes
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line with the field `alert`

#### Scenario: Audit chain break of the last night
- **GIVEN** a `security_event` row of the kind `audit_chain_broken` that is 20 hours old
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line with the field `alert` for the audit chain check

#### Scenario: Old audit chain break
- **GIVEN** a newest `security_event` row of the kind `audit_chain_broken` that is 30 hours old
- **WHEN** the job `alerts` runs
- **THEN** the event log holds no alert for the audit chain check

#### Scenario: Unreadable backup directory
- **GIVEN** a backup directory that the job cannot read
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line of the fault
- **AND** the event log holds one `WARN` line with the field `alert` for the dump age

#### Scenario: Unreadable disk mount
- **GIVEN** a `statvfs` call on the mount of `walarchive` that fails
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line of the fault
- **AND** the event log holds one `WARN` line with the field `alert` `low_disk` and the field `volume` `walarchive`

#### Scenario: Two low volumes
- **GIVEN** free disk below `disk.warn_percent` on `backups` and on `walarchive`, and a webhook URL
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line with the field `alert` `low_disk` for each of the two volumes
- **AND** the job sends one `POST` with the body `{"alert":"low_disk"}`

#### Scenario: Audit chain break younger than 26 hours
- **GIVEN** a newest `security_event` row of the kind `audit_chain_broken` that is 25 hours old, which is younger than 26 hours
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line with the field `alert` for the audit chain check

### Requirement: Startup settings comparison
At startup, the server SHALL compare each of its configuration keys with the stored value in `app_setting` in one transaction. For each changed value, the server SHALL write one `SettingChanged` audit entry with `actor = None`. The comparison SHALL also run in read-only mode (A24, A34).

#### Scenario: Changed separation of duties
- **GIVEN** the stored value `true` of `approval.separation_of_duties`
- **WHEN** the operator runs `docker compose up -d app` with `approval.separation_of_duties` set to `false`
- **THEN** the audit log holds one `SettingChanged` entry with the old value `true` and the new value `false`

#### Scenario: Comparison in read-only mode
- **GIVEN** free space on `pgdata` at 4 % and a changed value of `session.idle_timeout`
- **WHEN** the service `app` starts
- **THEN** the audit log holds one `SettingChanged` entry for `session.idle_timeout`
- **AND** every command except `Login`, `Logout`, and `ExportVersion` returns `E-WEB-006`

### Requirement: Failed startup settings comparison
If the startup settings comparison fails, the server SHALL write one `ERROR` line and SHALL NOT start (A34).

#### Scenario: Comparison transaction fails
- **GIVEN** a startup settings comparison whose transaction fails
- **WHEN** the service `app` starts
- **THEN** the server writes one `ERROR` line
- **AND** the server does not open its listeners

### Requirement: Storage error of a known constraint
The database can reject a duplicate key or a missing reference. Then the system SHALL return the domain error that the constraint name gives for the command. `reliability.md` section 1.1 lists these constraint names (R35, R36).

#### Scenario: Duplicate reference number found by the database
- **GIVEN** two requests that run `CreateRecipe` with the reference number `R-100` at the same time
- **WHEN** the database rejects the second insert with the constraint `recipe_reference_unique`
- **THEN** the system rejects the second command with `E-RCP-001`

#### Scenario: Missing parent unit found by the database
- **GIVEN** a request that runs `AddUnit` on `M12` with the parent path `U1`
- **WHEN** the database rejects the insert with the constraint `unit_parent_exists`
- **THEN** the system rejects the command with `E-PLT-003`

### Requirement: Storage error of an unknown constraint
When the database rejects a duplicate key of a constraint that the command does not know, the system SHALL return `E-DB-006`. When it rejects a missing reference of a constraint that the command does not know, the system SHALL return `E-DB-007` (R35, R36).

#### Scenario: Duplicate key of an unknown constraint
- **GIVEN** a command whose insert breaks a unique constraint that the command does not map
- **WHEN** the database rejects the insert
- **THEN** the system rejects the command with `E-DB-006`
- **AND** the transaction of the command writes nothing

#### Scenario: Missing reference of an unknown constraint
- **GIVEN** a command whose insert breaks a foreign key that the command does not map
- **WHEN** the database rejects the insert
- **THEN** the system rejects the command with `E-DB-007`
- **AND** the transaction of the command writes nothing

### Requirement: Roles from the database init script
The `db` init script SHALL create the roles `recipe_migrator`, `recipe_app`, `recipe_backup`, `recipe_replicator`, and `audit_owner`, and the database `recipe`, once, on an empty volume. It SHALL read each password from a secret file. It SHALL revoke `CONNECT` and `TEMPORARY` of `PUBLIC` on `recipe`, `postgres`, and `template1`. `recipe_app` SHALL NOT hold `pg_read_all_stats`. Only this script and `restore-volume` SHALL create a role in a production cluster (R04, R34, A1).

#### Scenario: First start on an empty volume
- **GIVEN** an empty volume of the service `db` and the four password secret files
- **WHEN** the service `db` starts for the first time
- **THEN** the five roles and the database `recipe` exist
- **AND** `PUBLIC` holds no `CONNECT` and no `TEMPORARY` privilege on `recipe`

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

### Requirement: Least privilege of the application role
The role `recipe_app` SHALL hold exactly the grants of `data-model.md` sections 5.2, 5.4, and 5.5. It SHALL NOT be a superuser, own a table, or switch to another role. An `UPDATE` of a column outside its grant SHALL fail with SQLSTATE `42501` before a trigger runs (R20, R29, R34).

#### Scenario: Update of a frozen version column
- **GIVEN** the approved version 2 of the recipe `R-100`
- **WHEN** `recipe_app` runs an `UPDATE` of `number`, `recipe_id`, `based_on_version_id`, or `row_version` of version 2
- **THEN** the database rejects the statement with SQLSTATE `42501`
- **AND** version 2 does not change

#### Scenario: Delete of a machine row
- **GIVEN** the live machine `M12`
- **WHEN** `recipe_app` runs a `DELETE` of the row of `M12`
- **THEN** the database rejects the statement with SQLSTATE `42501`
- **AND** the row of `M12` stays

#### Scenario: Role switch
- **GIVEN** a session of `recipe_app`
- **WHEN** the session runs `SET ROLE audit_owner`
- **THEN** the database rejects the statement

#### Scenario: Replication state as the application role
- **GIVEN** a session of `recipe_app` and the service `walarchive` that streams WAL
- **WHEN** the session runs `SELECT * FROM replication_state()`
- **THEN** the result has one row with `connected` set to true

### Requirement: Trigger code of a storage error
When a database trigger rejects a write, the system SHALL return the code at the start of the trigger message. The error SHALL carry the user message of that code. When a `CHECK` constraint without a trigger code rejects a write, the system SHALL return `E-DB-003`. A deadlock SHALL return `E-DB-001` (R35, R36).

#### Scenario: Parameter write in an approved version
- **GIVEN** the approved version 2 of the recipe `R-100`
- **WHEN** a write of a parameter row of version 2 reaches the database
- **THEN** the system rejects the write with `E-RCP-031`
- **AND** the transaction of the write changes nothing

#### Scenario: Check constraint without a trigger code
- **GIVEN** the live machine `M12`
- **WHEN** a write of the machine name with 201 characters reaches the database
- **THEN** the system rejects the write with `E-DB-003`

### Requirement: Idle transaction limit
The database SHALL end a session of `app` that stays idle inside an open transaction for more than 15 seconds. The open transaction SHALL roll back (R30).

#### Scenario: Idle transaction
- **GIVEN** a connection of `app` with an open transaction
- **WHEN** the connection sends no statement for more than 15 seconds
- **THEN** the database ends the session
- **AND** the transaction writes nothing

### Requirement: Configuration of the migrator
The subcommand `recipe-server migrate` SHALL read the connection values from its configuration and the password of `recipe_migrator` only from the file that `secrets.migrator_password_file` names. When it cannot read a value, it SHALL write one `ERROR` line and exit with code 1 before it connects (R34).

#### Scenario: Missing secret file
- **GIVEN** a configuration with `secrets.migrator_password_file` set to a path without a file
- **WHEN** the operator runs `recipe-server migrate`
- **THEN** the subcommand writes one `ERROR` line
- **AND** the subcommand exits with code 1 and applies no migration

#### Scenario: Migration of an empty database
- **GIVEN** the database `recipe` after the first start of `db` and a valid configuration
- **WHEN** the operator runs `recipe-server migrate`
- **THEN** the migrator applies every migration of the binary, from `0001_schema.sql` to `0006`, as `recipe_migrator`
- **AND** the subcommand exits with code 0

### Requirement: No row values in the log of a storage error
The log line of a storage error SHALL hold the error code and the messages of the error chain. It SHALL NOT hold the row values of a database detail. The database server SHALL log the errors of the application role without their detail (R35, R37).

#### Scenario: Not-null failure on a user row
- **GIVEN** a write of an `app_user` row that fails on a `NOT NULL` column
- **WHEN** the error reaches the HTTP mapping
- **THEN** the event log holds the code of the error
- **AND** the event log and the server log of `db` hold no `$argon2id$`

### Requirement: Health probe subcommand
The subcommand `recipe-server healthcheck` SHALL send `GET` to the internal listener and exit with code 0 only for the status 200. It SHALL read no secret, open no database connection, and end within 3 seconds. The Compose healthcheck of `app` SHALL use it (R30, A41).

#### Scenario: Ready application
- **GIVEN** the service `app` whose `/readyz` returns `ok`
- **WHEN** Docker runs `recipe-server healthcheck` in the container
- **THEN** the subcommand exits with code 0

#### Scenario: Listener does not answer
- **GIVEN** a process that accepts the connection on port 9000 and sends no answer
- **WHEN** the operator runs `recipe-server healthcheck`
- **THEN** the subcommand exits with code 1 after at most 3 seconds

### Requirement: Startup order of the server
At startup, `serve` SHALL check its configuration and both key files. Before it opens a listener, `serve` SHALL also measure the free space on `pgdata` and compare the settings. It SHALL open the internal listener before the public listener and before the watchdog counts a failed probe. When a step fails, `main` SHALL write one `ERROR` line and exit with code 1. This count excludes the line of the panic hook (R30, A24, A41, ADR-0061 (j)).

#### Scenario: Key file with a carriage return
- **GIVEN** the key file of `csrf_signing_key` with a carriage return
- **WHEN** the service `app` starts
- **THEN** the server writes one `ERROR` line that names the key and not its content
- **AND** the server opens no listener and exits with code 1

#### Scenario: Low disk at startup
- **GIVEN** free space on `pgdata` at 4 %
- **WHEN** the service `app` starts and the first request is `POST /recipes`
- **THEN** the response has the code `E-WEB-006`

### Requirement: Header and body read timeout
The service `app` SHALL close a connection that does not send its request headers within 10 seconds. It SHALL stop a request whose body does not arrive within 10 seconds and return `E-WEB-009` (R30).

#### Scenario: Slow request headers
- **GIVEN** a client that opens a connection to port 8080 and sends one header line
- **WHEN** 10 seconds pass without the end of the headers
- **THEN** the service `app` closes the connection

#### Scenario: Slow request body
- **GIVEN** a `POST /login` with a declared body of 100 bytes and 10 bytes sent
- **WHEN** 10 seconds pass without the rest of the body
- **THEN** the response has the code `E-WEB-009`

### Requirement: Health during the drain
During the drain after `SIGTERM`, the internal listener SHALL keep serving. `/healthz` SHALL return `ok`, and `/readyz` SHALL return `not ready`. The watchdog SHALL stop at the signal, so the process exits with code 0 (R30, A41). After both pools close, the internal listener SHALL stop and close its open connections at once (R30, A41).

#### Scenario: Drain with a slow request
- **GIVEN** the service `app` with one request in flight that runs for 20 seconds
- **WHEN** Docker sends `SIGTERM` to `app`
- **THEN** `/healthz` on port 9000 returns `ok` during the drain
- **AND** the client of the request gets its answer
- **AND** `app` exits with code 0

#### Scenario: Internal listener after the pool close
- **GIVEN** the service `app` after `SIGTERM` with an open connection on port 9000
- **WHEN** both pools close
- **THEN** the internal listener closes that connection at once
- **AND** `app` exits with code 0 within 40 seconds

### Requirement: Database connection without environment values
The subcommands `serve`, `migrate`, and `create-admin` SHALL refuse to start when a variable whose name starts with `PG` is set. Each SHALL write one `ERROR` line that names the variable and not its value, and exit with code 1. The server SHALL build its connections only from its configuration and secret files, without the password file of libpq. The TLS mode SHALL be `disable` inside the Compose network (R34, R38).

#### Scenario: Environment value of the password
- **GIVEN** the environment value `PGPASSWORD` with a wrong password and a valid password secret file
- **WHEN** the service `app` starts
- **THEN** the server writes one `ERROR` line that names `PGPASSWORD` and not its value
- **AND** the server opens no database connection and exits with code 1

#### Scenario: Environment value of the options
- **GIVEN** the environment value `PGOPTIONS` and a valid configuration
- **WHEN** the operator runs `recipe-server migrate`
- **THEN** the subcommand writes one `ERROR` line that names `PGOPTIONS`
- **AND** the subcommand exits with code 1 and applies no migration

### Requirement: Network isolation of the services
The Compose file SHALL connect only `proxy` to the network `edge`, which holds the published ports. It SHALL connect only `app` to the network `egress` for outbound calls. It SHALL connect the seven services to the network `backend` with `internal: true`. Each service SHALL drop every Linux capability and run with `no-new-privileges`. Only `proxy` SHALL get `NET_BIND_SERVICE` back (R04, R34).

#### Scenario: Network of the database
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator reads the networks of the service `db`
- **THEN** `db` is only on the network `backend`
- **AND** the network `backend` has `internal: true`

#### Scenario: Outbound network of the application
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator reads the members of the network `egress`
- **THEN** `app` is the only member
- **AND** no service publishes a port on `egress`

#### Scenario: Capabilities of the application
- **GIVEN** the running service `app`
- **WHEN** the operator reads the capabilities of the container
- **THEN** the container holds no Linux capability

### Requirement: Client rules of the database
The init script of the `db` service SHALL replace the catch-all line of `pg_hba.conf` of the image. The roles `recipe_migrator`, `recipe_app`, `recipe_backup`, and `recipe_replicator` SHALL connect only from the subnet of `backend`. The role `postgres` SHALL connect only through the local socket, and only from a process of the system user `postgres`. The healthcheck SHALL use the role name `pg_isready`, which no script creates. The server SHALL reject every other connection (R34, A1).

#### Scenario: Superuser over TCP
- **GIVEN** the database after the first start
- **WHEN** a client connects as `postgres` over TCP from the subnet of `backend`
- **THEN** the database rejects the connection

#### Scenario: Superuser through the socket
- **GIVEN** the database after the first start
- **WHEN** a process of the system user `postgres` in the container connects as `postgres` through the socket
- **THEN** the database accepts the connection without a password

#### Scenario: Superuser through the socket from another system user
- **GIVEN** the database after the first start
- **WHEN** a process of the system user `root` in the container connects as `postgres` through the socket
- **THEN** the database rejects the connection

#### Scenario: Healthcheck of the database
- **GIVEN** the running service `db`
- **WHEN** Docker runs the healthcheck of `db`
- **THEN** the healthcheck passes
- **AND** the log of `db` holds no `FATAL` line for the probe

### Requirement: Connect timeout of the migrator
The subcommand `recipe-server migrate` SHALL stop with one `ERROR` line and exit with code 1 when it cannot connect within 10 seconds (R30, R34).

#### Scenario: Database does not accept connections
- **GIVEN** a database host that accepts no connection
- **WHEN** the operator runs `recipe-server migrate`
- **THEN** the subcommand exits with code 1 after about 10 seconds

### Requirement: Image of the application
The image of `recipe-server` SHALL run as a user that is not root. It SHALL hold no shell and no package manager. The Compose file SHALL name each image of the project with a version tag and each external image with a pinned digest (R04, R34).

#### Scenario: User of the application container
- **GIVEN** the running service `app`
- **WHEN** the operator reads the user of the container
- **THEN** the user id is 65532

### Requirement: Disks of the backup and the WAL archive
The volumes `backups` and `walarchive` SHALL bind to directories on the second disk. The scripts of `backup` and `walarchive` SHALL refuse to start when the marker file of the disk is missing. The healthcheck of `backup` SHALL fail when the backup path is on the filesystem of `pgdata` (R31, A35).

#### Scenario: Backup path on the data filesystem
- **GIVEN** a `backups` volume on the same filesystem as `pgdata`
- **WHEN** Docker runs the healthcheck of `backup`
- **THEN** the healthcheck fails

#### Scenario: Second disk not mounted
- **GIVEN** a backup directory without the marker file
- **WHEN** the service `backup` starts
- **THEN** the service writes an `ERROR` line and writes no dump

### Requirement: Names of the backup sets
A reader of the backup sets SHALL count a set only by a manifest name that matches the grammar of `reliability.md` section 7.7. A temp name `.tmp-<name>` SHALL NOT count as a set. The retention SHALL NOT delete a name outside that grammar (R31, A35, ADR-0074).

#### Scenario: Temp files of a killed run
- **GIVEN** a backup directory on the second disk with the marker file
- **AND** the four files of one set, each only under its temp name `.tmp-<name>`
- **WHEN** Docker runs the healthcheck of `backup`
- **THEN** the healthcheck fails, because the directory holds no complete set

#### Scenario: Name outside the grammar of a set
- **GIVEN** a file `recipe_20260101_0200_copy.dump.sha256` older than 30 days in the backup directory
- **WHEN** the retention runs after a successful new set
- **THEN** the file stays in the backup directory

### Requirement: Dump on demand
The command `backup-now` in the running service `backup` SHALL write one complete set with the suffix `_now` and its manifest. Two runs SHALL NOT write at the same time (R31, A16).

#### Scenario: Dump before a maintenance
- **GIVEN** the running service `backup`
- **WHEN** the operator runs `docker compose exec backup backup-now`
- **THEN** a set with the suffix `_now` and its manifest exist in `backups`

### Requirement: Point-in-time recovery from the WAL archive
The service `walarchive` SHALL keep a verified base backup and the WAL segments after it. A recovery from the newest base backup and the WAL archive SHALL restore every transaction that the archive received. This includes the transactions of the partial last segment (R31, A35).

#### Scenario: Recovery after a committed change
- **GIVEN** a base backup, then a committed audit entry, and the WAL archive that received it
- **WHEN** the operator recovers a scratch cluster from the base backup and the WAL archive
- **THEN** the scratch cluster holds the audit entry
- **AND** the audit chain check passes on the scratch cluster

### Requirement: Alert of an export hash mismatch
The job `alerts` SHALL raise one alert when one or more exports returned `E-EXP-002` since its last run (R30, R37).

#### Scenario: Hash mismatch during an export
- **GIVEN** one export that returned `E-EXP-002` after the last run of the job `alerts`
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line with the field `alert` for the hash mismatch

### Requirement: Webhook of the alerts
When a webhook URL is set, the job `alerts` SHALL send one HTTPS `POST` with the condition name for each active condition of a run. It SHALL read the URL only from its secret file. A failed call SHALL write one `WARN` line without the URL and SHALL NOT stop the job. An empty secret file SHALL mean no webhook. A missing file at a set path, or a URL that does not use `https`, SHALL stop the start (R30, R37, ADR-0075).

#### Scenario: Webhook does not answer
- **GIVEN** a webhook URL whose host does not answer and an active alert condition
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line for the failed call without the URL
- **AND** the job runs again at its next interval

#### Scenario: Name of the condition in the body
- **GIVEN** an active condition "Free disk below `disk.warn_percent`" and a webhook URL
- **WHEN** the job `alerts` runs
- **THEN** the body of the `POST` is `{"alert":"low_disk"}`

#### Scenario: Missing webhook file
- **GIVEN** a set path of `alerts.webhook_url_file` and no file at that path
- **WHEN** the operator starts `recipe-server serve`
- **THEN** the process writes one `ERROR` line that names the key and not a URL
- **AND** the process exits with code 1

### Requirement: Alerts on a new installation
On a new installation, a missing dump SHALL raise no alert during 26 hours after the first start of `app`. A missing completed audit chain check SHALL raise no alert in the same time. An unreadable backup directory SHALL raise the alert of the dump age also in this time (R30, R37, ADR-0073).

#### Scenario: First hour after the installation
- **GIVEN** a new installation whose first start of `app` was 1 hour ago and no dump
- **WHEN** the job `alerts` runs
- **THEN** the event log holds no alert for the dump age

#### Scenario: Unreadable backup directory on a new installation
- **GIVEN** a new installation whose first start of `app` was 1 hour ago and a backup directory that the job cannot read
- **WHEN** the job `alerts` runs
- **THEN** the event log holds one `WARN` line with the field `alert` for the dump age

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

### Requirement: Stack test in CI
CI SHALL build the images and start the stack from an empty state on `workflow_dispatch` and on a push to `main` (ADR-0094). `/readyz` SHALL return `ok` within 60 seconds. The job SHALL then run the backup, the restore test, the point-in-time recovery, and a stop of `app` with exit code 0 (R04, R30, R31).

#### Scenario: Trigger of the stack test
- **GIVEN** the workflow `ci.yml` and a push to a branch other than `main`
- **WHEN** the push arrives
- **THEN** the job `compose` does not start, and a developer starts it with `workflow_dispatch`

#### Scenario: Start from zero
- **GIVEN** a CI runner with no volume of the project
- **WHEN** the job creates an empty external data volume and starts the stack
- **THEN** `/readyz` of `app` returns `ok` within 60 seconds

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

### Requirement: External data volume
`compose.yaml` SHALL declare the volume `pgdata` as an external volume. The mandatory value `RECIPE_PGDATA_VOLUME` of `deploy/.env` SHALL name it and SHALL have no default. Compose SHALL NOT create or delete this volume. `deploy/.env` SHALL hold nine values (R31, R34, A1).

#### Scenario: Missing name of the data volume
- **GIVEN** a `deploy/.env` without the value `RECIPE_PGDATA_VOLUME`
- **WHEN** the operator runs `docker compose up -d`
- **THEN** Compose stops with an error
- **AND** no container of `db` starts

#### Scenario: Name without a volume
- **GIVEN** a value `RECIPE_PGDATA_VOLUME` that names no Docker volume
- **WHEN** the operator runs `docker compose up -d db`
- **THEN** Compose stops with an error
- **AND** no empty cluster starts

#### Scenario: Down with volumes
- **GIVEN** a running stack on the volume that `RECIPE_PGDATA_VOLUME` names
- **WHEN** a user runs `docker compose down -v`
- **THEN** the volume and its cluster stay on the host

#### Scenario: Values of the example file
- **GIVEN** the file `deploy/.env.example`
- **WHEN** the operator reads its values
- **THEN** the file holds nine values, and `RECIPE_PGDATA_VOLUME` is one of them

### Requirement: Client rules from one shared file
`01-roles.sh` and `restore-volume` SHALL write `pg_hba.conf` with one function of one shared file in `deploy/db/`. Both SHALL run the two `REVOKE` statements of the default databases from a second function of that file. A cluster of `restore-volume` SHALL have the client rules of a cluster of `01-roles.sh`. A failed write SHALL NOT publish a part of `pg_hba.conf` (R34, A1).

#### Scenario: Superuser over TCP on a restored cluster
- **GIVEN** the service `db` on a volume that `restore-volume` filled
- **WHEN** a client connects as `postgres` over TCP from the subnet of `backend`
- **THEN** the database rejects the connection

#### Scenario: Same lines in both clusters
- **GIVEN** a cluster of `01-roles.sh` and a cluster of `restore-volume` with the same value `COMPOSE_SUBNET`
- **WHEN** the operator compares their files `pg_hba.conf`
- **THEN** the two files hold the same lines in the same order

#### Scenario: Failed write of the client rules
- **GIVEN** a data directory with a file `pg_hba.conf` and a temp name that the function cannot write
- **WHEN** `write_hba_conf` runs
- **THEN** the function fails
- **AND** the file `pg_hba.conf` does not change

### Requirement: Service of the restore into a new volume
The one-shot service `restore-volume` SHALL have the profile `restore`, `network_mode: none`, the mount `backups:/backups:ro`, a tmpfs on `/run/pgpass`, and no secret. `docker compose up` SHALL NOT start it. The operator SHALL run it with the `recipe-pgtools` image of the schema of the set (R31, R34).

#### Scenario: Start of the stack
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator runs `docker compose up -d`
- **THEN** Docker starts no container of `restore-volume`

#### Scenario: Isolation of the restore service
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator reads the service `restore-volume`
- **THEN** the service joins no network and mounts no secret
- **AND** it mounts `backups` read-only

### Requirement: Restore into a new volume
Every restore into production SHALL write into a new, empty volume while `db` runs on the old volume. `restore-volume` SHALL refuse a target that is not empty and a stamp outside the grammar of the set names. It SHALL refuse a user other than uid 999 with the name `postgres`. It SHALL run the gates of steps 1 and 2 of `restore-test` before `initdb` runs. It SHALL write nothing to `/backups` (R31, R34).

#### Scenario: Target is the live volume
- **GIVEN** the running service `db` on its volume
- **WHEN** the operator runs `restore-volume` with that volume as the target
- **THEN** the script writes one `ERROR` line and exits with a code other than 0
- **AND** the cluster on that volume does not change

#### Scenario: Stamp outside the grammar
- **GIVEN** an empty new volume
- **WHEN** the operator runs `restore-volume` with the stamp `20261030_1200_old`
- **THEN** the script writes one `ERROR` line before any PostgreSQL tool runs

#### Scenario: Wrong user of the restore
- **GIVEN** an empty new volume
- **WHEN** `restore-volume` runs as a uid other than 999, or as uid 999 with a name other than `postgres`
- **THEN** the script writes one `ERROR` line before any PostgreSQL tool runs

#### Scenario: Manifest mismatch before the restore
- **GIVEN** a set whose dump file differs from its manifest
- **WHEN** the operator runs `restore-volume` for that set
- **THEN** the script stops before `initdb` runs
- **AND** no file of `/backups` changes

#### Scenario: Completed restore
- **GIVEN** a complete set that passed `restore-test` and an empty new volume
- **WHEN** `restore-volume` runs for that set
- **THEN** the last line is `restore volume completed` with the name of the set and the field `system_identifier`
- **AND** `pg_controldata` of the volume shows the state `shut down`

#### Scenario: No known password after the restore
- **GIVEN** the service `db` on a volume that `restore-volume` filled
- **WHEN** a login role connects with a password before the operator sets the passwords
- **THEN** the database rejects the login

### Requirement: Checks of a restored cluster
Before the clean shutdown, `restore-volume` SHALL compare six kinds of values with `restore-roles.expected`. They are the client rules, the access lists of three databases, the role attributes, the role memberships, the role settings, and the grants. It SHALL compare the configuration files and the top-level names of the data directory with the state after `initdb`. It SHALL keep that state in shell variables and SHALL NOT write it into `/run/pgpass`. A difference SHALL end the run (R31, R34).

#### Scenario: Changed role attribute
- **GIVEN** a globals file that passes the role grammar and gives `recipe_app` the attribute `CREATEROLE`
- **WHEN** `restore-volume` runs for that set
- **THEN** the script writes one `ERROR` line and exits with a code other than 0

#### Scenario: Changed configuration file
- **GIVEN** a dump whose code writes a line into `postgresql.auto.conf` during the restore
- **WHEN** `restore-volume` compares the configuration files
- **THEN** the script writes one `ERROR` line and exits with a code other than 0

#### Scenario: Changed copy in the tmpfs
- **GIVEN** a dump whose code changes `postgresql.auto.conf` and writes files into `/run/pgpass` during the restore
- **WHEN** `restore-volume` compares the configuration files
- **THEN** the script writes one `ERROR` line and exits with a code other than 0

#### Scenario: Extra name in the data directory
- **GIVEN** a restore that leaves a new file at the top of the data directory
- **WHEN** `restore-volume` compares the top-level names
- **THEN** the script writes one `ERROR` line and exits with a code other than 0

#### Scenario: Proof of the expected file
- **GIVEN** a cluster of `01-roles.sh` and every migration
- **WHEN** the storage test reads the values of `restore-roles.expected` from that cluster
- **THEN** the values equal the file after the test subnet replaces the placeholder

#### Scenario: Subnet of another length
- **GIVEN** the value `COMPOSE_SUBNET` `192.168.100.0/24` or `10.0.0.0/8`
- **WHEN** `restore-volume` compares the client rules of its cluster with `restore-roles.expected`
- **THEN** the comparison of the `HBA` lines passes

#### Scenario: No secret in the compared values
- **GIVEN** the file `restore-roles.expected` and the output of `restore-roles.sql`
- **WHEN** the storage test reads each line
- **THEN** no line holds a password hash and no line holds an OID

### Requirement: Volume switch
The switch SHALL delete the containers of `proxy`, `app`, `backup`, `walarchive`, and `db`. It SHALL NOT use `docker compose down` or `docker compose start`. Before `db` starts on the new volume, `pg_controldata` SHALL show the state `shut down` and the system identifier of the last line of `restore-volume` or `pitr-volume`. After the start of `app` and `backup`, `docker inspect` of each container SHALL show the new volume (R31, R34).

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

### Requirement: Volume switch after a recovery from the WAL archive
The switch SHALL compare the new volume with the last line of `pitr-volume`. After a recovery from the WAL archive, the timeline SHALL also equal that line, and the switch SHALL need no state of the old volume. The switch SHALL refuse a volume with the file `pitr-drill` (R31, R34).

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
`walarchive-entry` and `basebackup-once` SHALL read `/wal/.system-identifier` before they write to `/wal`. When the file is missing and no segment exists, they SHALL write the identifier of the server into it. When the file is missing next to a segment, they SHALL refuse to start. They SHALL also refuse a file with another identifier and a malformed file. In a gap of the WAL stream, the file and the history files SHALL stay in `/wal` (R31, A35).

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

### Requirement: Timeline guard of the WAL archive
`walarchive-entry` and `basebackup-once` SHALL refuse to start when a segment in `/wal` has a timeline other than the timeline of the server (R31, A35).

#### Scenario: Segment of another timeline
- **GIVEN** a file `/wal/.system-identifier` that holds the identifier of `db`, and a segment of timeline 1 while `db` runs on timeline 2
- **WHEN** `walarchive-entry` or `basebackup-once` starts
- **THEN** the script writes the `ERROR` line `segments of another timeline are in the WAL archive`
- **AND** the script writes no file to `/wal`

### Requirement: Identifier file of an existing installation
An existing installation SHALL write `/wal/.system-identifier` once, before `walarchive` first starts with an image that has the guard. The step SHALL run only when the file is missing. It SHALL compare the identifier of `db` with the system identifier in the page header of the newest complete segment. Without a complete segment, it SHALL read the page header of the newest `.partial` file. A difference SHALL stop the step, and the step SHALL write no file (R31, A35).

#### Scenario: Existing installation before the guard
- **GIVEN** a directory `/wal` with segments of `db` and no file `.system-identifier`
- **WHEN** the operator pipes the system identifier of `db` into a new container of the previous image, before the upgrade replaces `db`
- **THEN** `/wal/.system-identifier` holds the identifier of `db`
- **AND** `walarchive-entry` of the new image starts `pg_receivewal`

#### Scenario: Segments of another system before the guard
- **GIVEN** a directory `/wal` whose newest complete segment holds a system identifier other than that of `db`, and no file `.system-identifier`
- **WHEN** the operator pipes the system identifier of `db` into a new container of the previous image
- **THEN** the step writes one `ERROR` line and stops
- **AND** no file `/wal/.system-identifier` exists

#### Scenario: Only a partial file of db
- **GIVEN** a directory `/wal` with a `.partial` file of `db`, no complete segment, and no file `.system-identifier`
- **WHEN** the operator pipes the system identifier of `db` into a new container of the previous image
- **THEN** `/wal/.system-identifier` holds the identifier of `db`

#### Scenario: Partial file without a long header
- **GIVEN** a directory `/wal` with no complete segment and a `.partial` file whose first page has no long page header
- **WHEN** the operator pipes the system identifier of `db` into a new container of the previous image
- **THEN** the step writes one `ERROR` line and stops
- **AND** no file `/wal/.system-identifier` exists

#### Scenario: No segment and no partial file
- **GIVEN** a directory `/wal` with no complete segment, no `.partial` file, and no file `.system-identifier`
- **WHEN** the operator pipes the system identifier of `db` into a new container of the previous image
- **THEN** the step writes one `ERROR` line and stops
- **AND** no file `/wal/.system-identifier` exists

#### Scenario: Identifier file before the upgrade
- **GIVEN** a directory `/wal` that already holds the file `.system-identifier`
- **WHEN** the upgrade reaches the step of the identifier file
- **THEN** the step does not run, and the file does not change

### Requirement: Position and form of the identifier step
The step of the identifier file SHALL run before the upgrade replaces `db`. It SHALL run in a new container of the previous image with `docker compose run`. It SHALL NOT use `docker compose exec` into the old `walarchive` container (R31, A35).

#### Scenario: Step before the replacement of db
- **GIVEN** an installation of 0.1.0 with segments in `/wal` and no file `.system-identifier`
- **WHEN** the upgrade reaches `docker compose up -d backup-premigrate`, which replaces `db`
- **THEN** `/wal/.system-identifier` already holds the identifier of `db`

#### Scenario: Old walarchive in a restart loop
- **GIVEN** a directory `/wal` with segments of `db`, no file `.system-identifier`, and an old `walarchive` container that restarts in a loop
- **WHEN** the operator pipes the system identifier of `db` into a new container of the previous image
- **THEN** `/wal/.system-identifier` holds the identifier of `db`

### Requirement: Data volume at the upgrade
Before the name check, the operator SHALL install the new release in a new directory `deploy/`. `deploy/.env` SHALL hold `RECIPE_PGDATA_VOLUME` before the first Compose command. The name check SHALL stop the upgrade when `pgdata` is not `external: true`, or when its name differs from `RECIPE_PGDATA_VOLUME` or from the volume of the running `db`. A new `db` container SHALL mount the same volume and SHALL have the same system identifier (R31, R34).

#### Scenario: Wrong existing volume at the upgrade
- **GIVEN** a running `db` on its volume and a value `RECIPE_PGDATA_VOLUME` that names another existing volume
- **WHEN** the operator runs the name check of the upgrade
- **THEN** the upgrade stops before any Compose command that starts or replaces a container
- **AND** `db` keeps its container and its volume

#### Scenario: Files of the previous release at the name check
- **GIVEN** a running `db` of release 0.1.0 and the `compose.yaml` of 0.1.0, which declares `pgdata` without `external: true`
- **WHEN** the operator runs the name check of the upgrade
- **THEN** the upgrade stops before any Compose command that starts or replaces a container

#### Scenario: Name other than the value of the environment file
- **GIVEN** a `compose.yaml` whose external volume `pgdata` has a name other than `RECIPE_PGDATA_VOLUME`
- **WHEN** the operator runs the name check of the upgrade
- **THEN** the upgrade stops before any Compose command that starts or replaces a container

#### Scenario: Replacement of the database at the first upgrade
- **GIVEN** a stack of release 0.1.0 and a passed name check
- **WHEN** the upgrade runs `docker compose up -d backup-premigrate`, and Compose replaces `db`
- **THEN** the new `db` container mounts the same volume
- **AND** `pg_control_system()` returns the same `system_identifier` as before the upgrade

### Requirement: Restore test before the upgrade
Before the upgrade replaces `db`, `restore-test` SHALL run in a new container of the previous image with `docker compose run --rm --no-deps backup`. It SHALL NOT use `docker compose exec` into the old `backup` container (R31).

#### Scenario: Old backup in a restart loop
- **GIVEN** an old `backup` container of 0.1.0 that restarts in a loop, and a complete set of the old schema
- **WHEN** the operator runs the restore test of the upgrade with the previous tag
- **THEN** the test runs in a new container and names the newest complete set
- **AND** `db` keeps its container

### Requirement: Tag of a data volume
Each start of `db` or `app` on a data volume SHALL use the `compose.yaml` and the tag that fit that volume. The `db` image of that `compose.yaml` SHALL have the PostgreSQL major version of the volume. ADR-0083 SHALL permit the `app` image of that tag on the schema of the volume. The change log of the site SHALL hold the tag of each volume that a switch leaves (R31, R34).

#### Scenario: Failed restore in a rollback
- **GIVEN** a rollback restore of a `_premigrate` set, a `deploy/.env` with the previous tag, and an old volume with the newer schema
- **WHEN** `restore-volume` fails before the switch sets the new name
- **THEN** the operator sets the new tag in `deploy/.env` again
- **AND** `app` and `backup` start with the new tag on the old volume

#### Scenario: Switch back after a rollback restore
- **GIVEN** a switch of a rollback restore and a change log that holds the new tag next to the old volume
- **WHEN** the operator runs the switch back
- **THEN** `deploy/.env` holds the new tag before `db` starts
- **AND** `walarchive`, `app`, and `backup` start with the new tag on the old volume

#### Scenario: Restart of the old cluster in a major upgrade
- **GIVEN** a major upgrade, and an old volume whose state is not `shut down` after the first step of the switch
- **WHEN** the operator starts `db` again on the old volume
- **THEN** the command runs in the directory `deploy/` of the old release
- **AND** `db` starts with the image of the old major version

### Requirement: Delete rule of the old volume
No script and no retention SHALL delete the old `pgdata` volume or the target directory of a switch. The operator SHALL delete them only after three conditions. A `restore-test` passed on a set of the new cluster. The flush check and the WAL check passed on the new line. The switch is older than `volume_switch.keep_min_age` (R31).

#### Scenario: Retention after a switch
- **GIVEN** a switch of 3 days ago and the target directory of that switch
- **WHEN** the retention of `backup-once` and of `basebackup-once` runs
- **THEN** the old volume and every file of the target directory stay

### Requirement: Major upgrade release without migration
CI SHALL compare the major version of the `db` image with the previous release tag. When the major version changes, CI SHALL fail on each file of `migrations/` that the previous tag does not hold (R34).

#### Scenario: New migration in a major upgrade release
- **GIVEN** a branch that changes the major version of the `db` image and adds a migration file
- **WHEN** CI runs
- **THEN** the step of the migration list fails and names the new file

#### Scenario: New migration with the same major version
- **GIVEN** a branch that adds a migration file and keeps the major version of the `db` image
- **WHEN** CI runs
- **THEN** the step of the migration list passes

### Requirement: Restore and switch test in CI
CI SHALL run the job `restore-switch` on `workflow_dispatch` and on a push to `main` (ADR-0094). The job SHALL restore a set into a new volume, switch, start a new WAL line, start `app`, and check the restored data. It SHALL compare `postgresql.conf` and `postgresql.auto.conf` of the cluster of `restore-volume` with the cluster of `01-roles.sh`. Then it SHALL switch back to the old volume (R04, R31).

#### Scenario: Restore and switch on a real stack
- **GIVEN** a stack with data, a passed `backup-now`, and a passed `restore-test`
- **WHEN** the job runs `restore-volume`, the switch, and the new WAL line, and starts `app`
- **THEN** `/readyz` returns `ok`, a login succeeds, and the hash of an export equals the hash before the restore
- **AND** the row counts equal the counts file of the set, and the audit chain check passes

#### Scenario: Refused targets
- **GIVEN** the running stack of the job
- **WHEN** the job runs `restore-volume` on the live volume, and `docker compose up` with a missing external volume
- **THEN** both commands fail, and the live cluster does not change

#### Scenario: Switch back in CI
- **GIVEN** the stack after the switch
- **WHEN** the job runs the switch back
- **THEN** `db` serves the data of the time before the switch

#### Scenario: Configuration files of both clusters
- **GIVEN** the old cluster of `01-roles.sh` and the new cluster of `restore-volume`, each after a clean stop
- **WHEN** the job compares their files `postgresql.conf` and `postgresql.auto.conf`
- **THEN** the files of both clusters are equal

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
