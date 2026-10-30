## ADDED Requirements

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
`01-roles.sh` and `restore-volume` SHALL write `pg_hba.conf` with one function of one shared file in `deploy/db/`. Both SHALL run the two `REVOKE` statements of the default databases from a second function of that file. A cluster of `restore-volume` SHALL have the client rules of a cluster of `01-roles.sh` (R34, A1).

#### Scenario: Superuser over TCP on a restored cluster
- **GIVEN** the service `db` on a volume that `restore-volume` filled
- **WHEN** a client connects as `postgres` over TCP from the subnet of `backend`
- **THEN** the database rejects the connection

#### Scenario: Same lines in both clusters
- **GIVEN** a cluster of `01-roles.sh` and a cluster of `restore-volume` with the same value `COMPOSE_SUBNET`
- **WHEN** the operator compares their files `pg_hba.conf`
- **THEN** the two files hold the same lines in the same order

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
Every restore into production SHALL write into a new, empty volume while `db` runs on the old volume. `restore-volume` SHALL refuse a target that is not empty and a stamp outside the grammar of the set names. It SHALL run the gates of steps 1 and 2 of `restore-test` before `initdb` runs. It SHALL write nothing to `/backups` (R31, R34).

#### Scenario: Target is the live volume
- **GIVEN** the running service `db` on its volume
- **WHEN** the operator runs `restore-volume` with that volume as the target
- **THEN** the script writes one `ERROR` line and exits with a code other than 0
- **AND** the cluster on that volume does not change

#### Scenario: Stamp outside the grammar
- **GIVEN** an empty new volume
- **WHEN** the operator runs `restore-volume` with the stamp `20261030_1200_old`
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
Before the clean shutdown, `restore-volume` SHALL compare six kinds of values with `restore-roles.expected`. They are the client rules, the access lists of three databases, the role attributes, the role memberships, the role settings, and the grants. It SHALL compare the configuration files and the top-level names of the data directory with the state after `initdb`. A difference SHALL end the run (R31, R34).

#### Scenario: Changed role attribute
- **GIVEN** a globals file that passes the role grammar and gives `recipe_app` the attribute `CREATEROLE`
- **WHEN** `restore-volume` runs for that set
- **THEN** the script writes one `ERROR` line and exits with a code other than 0

#### Scenario: Changed configuration file
- **GIVEN** a dump whose code writes a line into `postgresql.auto.conf` during the restore
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
The switch SHALL delete the containers of `proxy`, `app`, `backup`, `walarchive`, and `db`. It SHALL NOT use `docker compose down` or `docker compose start`. Before `db` starts on the new volume, `pg_controldata` SHALL show the state `shut down` and the system identifier of the line `restore volume completed`. After the start of `app` and `backup`, `docker inspect` of each container SHALL show the new volume (R31, R34).

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

### Requirement: Switch back
A switch back SHALL use the steps of the switch with the old volume name. The system identifier of the old volume SHALL equal the identifier that the first switch wrote into the change log of the site. When this check fails, the operator SHALL start no `db` and delete no volume (R31, R34).

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

### Requirement: WAL line after a switch
Every switch SHALL start a new WAL line before `app` starts. The operator SHALL move the old segments, the `.partial` file, the base backups, and `/wal/.system-identifier` into a target directory. The first base backup of the new line SHALL pass its checks before the first write of `app` (R31, A35).

#### Scenario: WAL line after a restore
- **GIVEN** a switch to a volume that `restore-volume` filled
- **WHEN** the operator starts `walarchive` after the move of the old files
- **THEN** `/wal` holds only segments of the new cluster
- **AND** the flush check and the WAL check pass before `app` starts

#### Scenario: Old base backups after a switch
- **GIVEN** a WAL line after a switch
- **WHEN** `basebackup-once` keeps the newest base backups and runs `pg_archivecleanup`
- **THEN** `/wal/base` holds no base backup of the old cluster

### Requirement: System identifier guard of the WAL archive
`walarchive-entry` and `basebackup-once` SHALL read `/wal/.system-identifier` before they write to `/wal`. When the file is missing and no segment exists, they SHALL write the identifier of the server into it. When the file is missing next to a segment, they SHALL refuse to start. They SHALL also refuse a file with another identifier and a malformed file. In a gap of the WAL stream, the file SHALL stay in `/wal` (R31, A35).

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

#### Scenario: Malformed identifier file
- **GIVEN** a file `/wal/.system-identifier` that is empty, holds a CR, or holds a character other than a digit
- **WHEN** `walarchive-entry` or `basebackup-once` starts
- **THEN** the script writes one `ERROR` line with the message of a malformed file
- **AND** the script writes no file to `/wal`

### Requirement: Identifier file of an existing installation
An existing installation SHALL write `/wal/.system-identifier` once, before `walarchive` first starts with an image that has the guard. The step SHALL run only when the file is missing. It SHALL compare the identifier of `db` with the system identifier in the page header of the newest complete segment. A difference SHALL stop the step, and the step SHALL write no file (R31, A35).

#### Scenario: Existing installation before the guard
- **GIVEN** a directory `/wal` with segments of `db` and no file `.system-identifier`
- **WHEN** the operator pipes the system identifier of `db` into the old `walarchive` container before the upgrade
- **THEN** `/wal/.system-identifier` holds the identifier of `db`
- **AND** `walarchive-entry` of the new image starts `pg_receivewal`

#### Scenario: Segments of another system before the guard
- **GIVEN** a directory `/wal` whose newest complete segment holds a system identifier other than that of `db`, and no file `.system-identifier`
- **WHEN** the operator pipes the system identifier of `db` into the old `walarchive` container
- **THEN** the step writes one `ERROR` line and stops
- **AND** no file `/wal/.system-identifier` exists

#### Scenario: Identifier file before the upgrade
- **GIVEN** a directory `/wal` that already holds the file `.system-identifier`
- **WHEN** the upgrade reaches the step of the identifier file
- **THEN** the step does not run, and the file does not change

### Requirement: Data volume at the upgrade
Before the first Compose command of an upgrade, `deploy/.env` SHALL hold `RECIPE_PGDATA_VOLUME`. The operator SHALL compare the name of `pgdata` in `docker compose config` with the volume that `docker inspect` of the running `db` shows for `/var/lib/postgresql/data`. A difference SHALL stop the upgrade. When the upgrade replaces `db`, the new container SHALL mount the same volume and SHALL have the same system identifier (R31, R34).

#### Scenario: Wrong existing volume at the upgrade
- **GIVEN** a running `db` on its volume and a value `RECIPE_PGDATA_VOLUME` that names another existing volume
- **WHEN** the operator runs the name check of the upgrade
- **THEN** the upgrade stops before any Compose command that starts or replaces a container
- **AND** `db` keeps its container and its volume

#### Scenario: Replacement of the database at the first upgrade
- **GIVEN** a stack of release 0.1.0 and a passed name check
- **WHEN** the upgrade runs `docker compose up -d backup-premigrate`, and Compose replaces `db`
- **THEN** the new `db` container mounts the same volume
- **AND** `pg_control_system()` returns the same `system_identifier` as before the upgrade

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

## MODIFIED Requirements

### Requirement: Pinned database image
The `db` service SHALL use a pinned PostgreSQL 16 image. The `initdb` run SHALL enable data checksums. It SHALL set the collation `C` for `lc-collate` and `C.UTF-8` for `lc-ctype` (A15).

#### Scenario: New database volume
- **GIVEN** an empty volume that the operator created with `docker volume create`
- **AND** the value `RECIPE_PGDATA_VOLUME` of `deploy/.env` names that volume
- **WHEN** the operator runs `docker compose up -d db`
- **THEN** the initdb run creates the cluster with data checksums enabled
- **AND** the initdb run sets the collation `C` for `lc-collate`

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

### Requirement: Container limits
Each service SHALL have a hard memory limit without swap and a CPU limit. This includes the service `pitr-check` in the profile `pitr` and the service `restore-volume` in the profile `restore`. The values SHALL equal the keys `container.*` of `reliability.md` section 7.11 (R30, R04).

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
