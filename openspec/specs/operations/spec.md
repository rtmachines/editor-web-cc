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
The init script of the `db` service SHALL create the roles `recipe_migrator`, `recipe_app`, `recipe_backup`, `recipe_replicator`, and `audit_owner` and the database `recipe` once, on an empty volume. It SHALL read each password from a secret file. It SHALL revoke `CONNECT` and `TEMPORARY` of `PUBLIC` on `recipe`, `postgres`, and `template1`. `recipe_app` SHALL NOT hold `pg_read_all_stats`. A migration SHALL NOT create a role (R04, R34, A1).

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
Each service SHALL have a hard memory limit without swap and a CPU limit. This includes the service `pitr-check` in the profile `pitr`. The values SHALL equal the keys `container.*` of `reliability.md` section 7.11 (R30, R04).

#### Scenario: Memory limit of the application
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator reads the limits of the service `app`
- **THEN** the memory limit and the memory with swap are both 512 MiB

#### Scenario: CPU limit of the recovery check
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator reads the limits of the service `pitr-check`
- **THEN** the CPU limit is 0.5 and the memory limit and the memory with swap are both 512 MiB

### Requirement: Stack test in CI
CI SHALL build the images and start the stack from an empty state on `workflow_dispatch` and on a push to `main` (ADR-0094). `/readyz` SHALL return `ok` within 60 seconds. The job SHALL then run the backup, the restore test, the point-in-time recovery, and a stop of `app` with exit code 0 (R04, R30, R31).

#### Scenario: Trigger of the stack test
- **GIVEN** the workflow `ci.yml` and a push to a branch other than `main`
- **WHEN** the push arrives
- **THEN** the job `compose` does not start, and a developer starts it with `workflow_dispatch`

#### Scenario: Start from zero
- **GIVEN** a CI runner with no volume of the project
- **WHEN** the job starts the stack
- **THEN** `/readyz` of `app` returns `ok` within 60 seconds
