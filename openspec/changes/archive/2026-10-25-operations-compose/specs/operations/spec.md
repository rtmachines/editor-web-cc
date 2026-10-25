# Spec Delta

## MODIFIED Requirements

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

### Requirement: Restore test
The service `backup` SHALL run the restore test every Sunday at 05:00 UTC and on demand before each release. It SHALL also run the test on demand in steps 5 and 6 of the upgrade procedure. The test SHALL check the manifest and restore the newest set into a private scratch cluster. Before the data load, it SHALL compare the names of the functions, triggers, and event triggers of the dump with the list of the release. Before the data load, it SHALL also compare the code of each function, `CHECK` constraint, and generated column with that list. After the restore of `post-data`, it SHALL compare the code of each trigger and event trigger with that list. Before the data load, it SHALL stop on each procedure, aggregate, materialized view, and operator of the dump. The trigger code SHALL include the enabled state. It SHALL compare the row counts with the counts file of the set. It SHALL run the audit chain check and log in with each login role. It SHALL write one result line to the event log (R31, A16).

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

### Requirement: Alert conditions
The job `alerts` SHALL check the alert conditions of `reliability.md` section 3.7 every 60 seconds. For each active condition, the job SHALL write one `WARN` line with the field `alert`. The condition `low_disk` SHALL write one line for each low or unreadable volume, with the field `volume`. A read fault of the backup directory or of a disk mount SHALL make its condition active. The audit chain condition SHALL stay active while the newest `audit_chain_broken` row is younger than 26 hours (R37, ADR-0073, ADR-0075).

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

## ADDED Requirements

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
