## ADDED Requirements

### Requirement: Tag of the backup manifest
Each run of `backup-once` SHALL write the tag file `recipe_<stamp>.dump.sha256.hmac` for its set. The tag SHALL be HMAC-SHA-256 with the first key of the secret `backup_manifest_key` over the message of ADR-0174 (f). The run SHALL compute each hash of the manifest from a file of the run before its publish. The run SHALL publish the tag file before the manifest. A run that cannot write the tag SHALL publish no manifest (R31, A16).

#### Scenario: Nightly run with a tag
- **GIVEN** the service `backup` with a valid key file and the clock at 02:00 UTC
- **WHEN** the dump run ends without an error
- **THEN** the set holds the tag file `recipe_<stamp>.dump.sha256.hmac`
- **AND** the run publishes the tag file before the manifest
- **AND** the tag covers the label `recipe-backup-manifest-v1`, one line feed, and the bytes of the manifest

#### Scenario: Manifest from the files of the run
- **GIVEN** a dump run that hashed its temp dump
- **WHEN** a test changes a byte of the published dump before the run writes the manifest
- **THEN** the manifest holds the hash of the bytes that `pg_dump` wrote

#### Scenario: Missing key file
- **GIVEN** the service `backup` without the file `/run/secrets/backup_manifest_key`
- **WHEN** a dump run starts
- **THEN** the run writes one `ERROR` line before `pg_dump` runs
- **AND** no manifest of a new set exists
- **AND** every old set stays

#### Scenario: Key file on the backup disk
- **GIVEN** a key file on the same filesystem as `/backups`
- **WHEN** a dump run starts
- **THEN** the run writes one `ERROR` line and publishes no file

#### Scenario: Tag file is no set
- **GIVEN** a tag file `recipe_20261115_0200.dump.sha256.hmac` without its manifest
- **WHEN** a reader of the backup sets lists the complete sets
- **THEN** the reader does not count the tag file as a set

### Requirement: Key of the backup manifest
The key SHALL be the Docker secret `backup_manifest_key` on the host, outside the backup disk. Only `backup`, `backup-premigrate`, and `restore-volume` of `compose.yaml` SHALL mount it. Each line of the key file SHALL hold one key as 64 lower-case hex characters. The file SHALL hold at most `backup_manifest.max_keys` lines. A check SHALL accept a tag of each key line. A script SHALL stop with one `ERROR` line on a bad key file (R34, R38).

#### Scenario: Key file with a carriage return
- **GIVEN** a key file whose key line ends with a carriage return
- **WHEN** `backup-once` or `restore-test` checks the key file
- **THEN** the script writes one `ERROR` line and exits with a code other than 0

#### Scenario: Key line of the wrong length
- **GIVEN** a key file whose line holds 63 or 65 hex characters
- **WHEN** a script checks the key file
- **THEN** the script writes one `ERROR` line that holds no character of the line

#### Scenario: Empty key file, empty line, or NUL byte
- **GIVEN** a key file that is empty, ends with two line feeds, or holds a NUL byte
- **WHEN** a script checks the key file
- **THEN** the script writes one `ERROR` line and exits with a code other than 0

#### Scenario: Three key lines
- **GIVEN** a key file with three valid key lines
- **WHEN** a script checks the key file
- **THEN** the script writes one `ERROR` line and exits with a code other than 0

#### Scenario: Rotation with two key lines
- **GIVEN** a set that the old key signed and a key file with the new key first and the old key second
- **WHEN** a dump run ends and the operator runs `restore-volume` for the older set
- **THEN** the new set has the key id of the new key
- **AND** the tag check of the older set passes

#### Scenario: Start of the backup service without a valid key
- **GIVEN** the service `backup` with a missing key file
- **WHEN** the container starts
- **THEN** `backup-entry` writes one `ERROR` line and supercronic does not start

#### Scenario: Mounts of the key
- **GIVEN** the Compose file of the recipe editor
- **WHEN** a reader lists the services that mount `backup_manifest_key`
- **THEN** the list is `backup`, `backup-premigrate`, and `restore-volume`

### Requirement: Protection of the manifest key
An `ERROR` line about the key file SHALL hold no byte of the file. No script SHALL put the key into an argument list, an environment value, or a shell variable. An image without the tag check SHALL NOT get the key (R34, R38).

#### Scenario: Error line without key bytes
- **GIVEN** a key file whose line holds 63 hex characters
- **WHEN** `backup-once` checks the key file
- **THEN** stdout and stderr hold no character of that line

#### Scenario: Previous image without the tag check
- **GIVEN** an upgrade from an image without `/etc/pgtools/manifest-tag.pl`
- **WHEN** the previous image runs `restore-test` or `restore-volume`
- **THEN** its container has no file `/run/secrets/backup_manifest_key`

### Requirement: Tag check before a restore
`restore-test` and `restore-volume` SHALL check the tag of the selected set before any PostgreSQL tool reads a file of the set. A missing tag file, a malformed tag line, an unknown key id, or a different tag SHALL stop the run. No option SHALL skip the check. `restore-test` SHALL NOT select an older set after a bad tag (R31, R38, A16).

#### Scenario: Set with a manifest computed without the key
- **GIVEN** a newest set whose dump and manifest a writer changed without the key
- **WHEN** the restore test runs
- **THEN** the test writes one `ERROR` line before any PostgreSQL tool runs
- **AND** the result file holds `failed`

#### Scenario: Missing tag file
- **GIVEN** a newest set without its tag file
- **WHEN** the restore test runs
- **THEN** the test stops before any PostgreSQL tool runs
- **AND** the test does not select an older set

#### Scenario: Tag of another key
- **GIVEN** a newest set whose tag file comes from a key that the key file does not hold
- **WHEN** the restore test runs
- **THEN** the test stops before any PostgreSQL tool runs

#### Scenario: Restore into a new volume with a bad tag
- **GIVEN** a set whose tag does not match its manifest and an empty new volume
- **WHEN** the operator runs `restore-volume` for that set
- **THEN** the script stops before `initdb` runs
- **AND** no file of `/backups` changes

#### Scenario: Valid tag
- **GIVEN** a complete set that `backup-once` wrote with the current key
- **WHEN** the restore test runs
- **THEN** the tag check passes and the test continues with the manifest check
- **AND** the real `pg_restore` restores the set

### Requirement: Reads of the set before the tag check
Before the tag check passes, `restore-test` and `restore-volume` SHALL read only the manifest, the tag file, and the key file. The check SHALL read a private copy of the manifest and of the tag file. Each later check of the manifest SHALL read the same private copy (R31, R38).

#### Scenario: Bad tag and unreadable set files
- **GIVEN** a set with a bad tag whose dump, globals file, and counts file are directories
- **WHEN** the restore test runs
- **THEN** the test stops with the `ERROR` line of the tag check
- **AND** no `ERROR` line names a read fault of the dump, the globals file, or the counts file

#### Scenario: Manifest change after the tag check
- **GIVEN** a set with a valid tag
- **WHEN** a writer changes the manifest on the disk after the tag check
- **THEN** the manifest check reads the private copy and not the changed file

### Requirement: Check of a set without a restore
`restore-volume --check-only <stamp>` SHALL run the checks of the set up to the globals file and then stop. It SHALL run no PostgreSQL tool, need no target volume, and write no file outside its work directory. A fault SHALL stop it with the same `ERROR` line as a full restore and an exit code other than 0 (R31, R38).

#### Scenario: Check of a valid set
- **GIVEN** a `_premigrate` set with a valid tag and no target volume
- **WHEN** the operator runs `restore-volume --check-only <stamp>` with the new image
- **THEN** the command writes one `INFO` line and exits with the code 0
- **AND** no `initdb` runs

#### Scenario: Check of a set with a bad tag
- **GIVEN** a `_premigrate` set whose dump a writer changed without the key
- **WHEN** the operator runs `restore-volume --check-only <stamp>`
- **THEN** the command writes one `ERROR` line and exits with a code other than 0
- **AND** the rollback stops before the previous image runs

### Requirement: Rolled-back backup service
While `deploy/.env` holds the tag of a previous image without the tag check, `COMPOSE_FILE` SHALL name `compose.yaml` and `compose.previous.yaml`. With these files, `backup` SHALL run the nightly dump and no `restore-test` (R31, R34).

#### Scenario: Schedule after a rollback
- **GIVEN** a rollback to a previous image without `/etc/pgtools/manifest-tag.pl`
- **WHEN** the service `backup` of that image starts
- **THEN** its crontab holds the line of `backup-once` and no line of `restore-test`
- **AND** its container has no file `/run/secrets/backup_manifest_key`

#### Scenario: Failed upgrade
- **GIVEN** a failed command 1 or 2 of the upgrade, and a previous image without the tag check
- **WHEN** the operator sets the previous tag and `COMPOSE_FILE` in `deploy/.env` and runs `docker compose up -d --no-deps --force-recreate backup`
- **THEN** the new container of `backup` runs no `restore-test`
- **AND** it has no file `/run/secrets/backup_manifest_key`

### Requirement: Exit from the rolled-back backup service
Each edit that writes a tag with the tag check into `deploy/.env` SHALL delete `COMPOSE_FILE` from it and SHALL run `unset COMPOSE_FILE`. After the rollback step that sets the new tag again, the operator SHALL create the container of `backup` again. Step 2 of the next upgrade SHALL NOT create it again. Command 1 of step 5 SHALL then create it with the new image and the key (R31, R34).

#### Scenario: Exit from the rolled-back state
- **GIVEN** `deploy/.env` with the previous tag and `COMPOSE_FILE`
- **WHEN** the operator sets a tag with the tag check, deletes `COMPOSE_FILE`, runs `unset COMPOSE_FILE`, and creates the container of `backup` again
- **THEN** the new container has the file `/run/secrets/backup_manifest_key`
- **AND** `backup-entry` starts supercronic

#### Scenario: Exit in the next upgrade
- **GIVEN** a copied `deploy/.env` with the previous tag and `COMPOSE_FILE` in step 2 of the next upgrade
- **WHEN** the operator sets the new tag, deletes `COMPOSE_FILE`, and runs `unset COMPOSE_FILE`
- **THEN** `docker compose config` shows `backup_manifest_key` on the three services
- **AND** the running container of `backup` stays unchanged until command 1 of step 5

### Requirement: Exit in the switch back
Step 3 of the switch back SHALL run the exit from the rolled-back backup service. It SHALL do so only when the tag of the old volume has the tag check. The exit SHALL always write the tag of the old volume. After the rollback of §7.10 step 6, that tag is the new tag. Only after a failed upgrade is it the previous tag. The switch back SHALL NOT create `backup` again at that step, because its last step creates `backup` (R31, R34).

#### Scenario: Exit in the switch back
- **GIVEN** `deploy/.env` with the previous tag and `COMPOSE_FILE` after command 2 of the rollback, and an old volume of the new tag
- **WHEN** step 3 of the switch back writes the new tag, deletes `COMPOSE_FILE`, and runs `unset COMPOSE_FILE`
- **THEN** the last step of the switch back creates `backup` with the file `/run/secrets/backup_manifest_key`
- **AND** no step of the switch back runs `--force-recreate`

### Requirement: Failed rollback restore after a failed upgrade
The rollback step that sets the new tag again SHALL apply only to the rollback of §7.10 step 6. After a failed upgrade, a failed check or a failed restore of the `_premigrate` set SHALL keep the previous tag and `COMPOSE_FILE` in `deploy/.env`. The operator SHALL then start only `backup` again (R31, R34).

#### Scenario: Failed check after a failed upgrade
- **GIVEN** `deploy/.env` with the previous tag and `COMPOSE_FILE` after a failed command 1 or 2 of the upgrade
- **WHEN** `restore-volume --check-only <stamp>` of the `_premigrate` restore exits with a code other than 0
- **THEN** `deploy/.env` keeps the previous tag and `COMPOSE_FILE`
- **AND** the operator starts only `backup` again with the previous image
- **AND** the running `app` of the previous image stays unchanged

#### Scenario: Failed full restore after a failed upgrade
- **GIVEN** `deploy/.env` with the previous tag and `COMPOSE_FILE` after a failed command 1 or 2 of the upgrade, and a pass of the check
- **WHEN** `restore-volume <stamp>` of the `_premigrate` set exits with a code other than 0
- **THEN** `deploy/.env` keeps the previous tag and `COMPOSE_FILE`
- **AND** the operator deletes the container of `app` with `docker compose rm -s -f app`
- **AND** the operator starts only `backup` again with the previous image
- **AND** `app` stays stopped

### Requirement: Stopped app after a failed rollback restore
After a failed upgrade and a failed `_premigrate` restore, the operator SHALL delete the container of `app` with `docker compose rm -s -f app`. `app` SHALL stay stopped until a forward correction, a new restore, or `pitr-volume`. The operator SHALL start it earlier only after a check that the old schema works with the previous image. The switch back after a failed upgrade SHALL follow the same rule. Its check after the start SHALL inspect only `backup` and the WAL stream in `db` (R31, R34).

#### Scenario: Switch back after a failed upgrade
- **GIVEN** `deploy/.env` with the previous tag and `COMPOSE_FILE` after a failed command 1 or 2 of the upgrade
- **AND** a failed switch of the `_premigrate` restore
- **WHEN** the switch back returns `db` to the old volume
- **THEN** the last step of the switch back starts only `backup` with the previous image
- **AND** `app` and `proxy` stay stopped
- **AND** the check after the start inspects only `backup` and checks the WAL stream with a query in `db`

#### Scenario: Reboot after a failed full restore
- **GIVEN** a failed upgrade and a failed `restore-volume <stamp>` of the `_premigrate` set
- **WHEN** the operator runs `docker compose rm -s -f app` and the host then restarts Docker
- **THEN** no container of `app` exists, so Docker starts no `app` on the schema of the failed `migrate`

### Requirement: Checks while app stays stopped
While `app` stays stopped after a failed rollback restore after a failed upgrade, no job `alerts` runs. The operator SHALL then check the WAL stream in `db` and a complete daily set once a day. The check SHALL accept only `recipe_<day>_[0-9][0-9][0-9][0-9].dump.sha256` of the current UTC day. The operator SHALL run the checks after `backup.time` plus `backup.max_age_alert` minus 24 h. At the later start of `app`, the operator SHALL wait until the alert `replication` is inactive (R31, R34).

#### Scenario: Checks while app stays stopped
- **GIVEN** `app` stopped after a failed rollback restore after a failed upgrade
- **WHEN** the time of the day is after `backup.time` plus `backup.max_age_alert` minus 24 h
- **THEN** the operator checks that `pg_stat_replication` in `db` shows one row with the state `streaming` and a `flush_lsn`
- **AND** the operator checks that a complete daily set has a stamp with the current UTC day

#### Scenario: Foreign name in the dump check
- **GIVEN** `app` stopped after a failed rollback restore after a failed upgrade
- **AND** `/backups` holds only the file `recipe_<day>_abcd.dump.sha256` for the current UTC day `<day>`
- **WHEN** the operator runs the dump check, which sets `LC_ALL=C` for the match
- **THEN** the dump check gives a fault

#### Scenario: Later start of app
- **GIVEN** `app` stopped after a failed rollback restore after a failed upgrade, and the confirmation that the schema works with the previous image
- **WHEN** the operator starts `app`
- **THEN** the operator waits until the alert `replication` is inactive

### Requirement: Check of the rollback set with the new image
In the rollback of ADR-0174 (aj), the operator SHALL run `restore-volume --check-only <stamp>` with the new image on a set of the previous schema. Only after a pass SHALL the operator delete the container of `app` with `docker compose rm -s -f app` and run `restore-volume <stamp>` with the previous image (R31, R34).

#### Scenario: Pass of the check of the rollback set
- **GIVEN** a rollback with the `_premigrate` set and a stopped `backup`
- **WHEN** `restore-volume --check-only <stamp>` of the new image exits with the code 0
- **THEN** the operator runs `docker compose rm -s -f app` before `restore-volume <stamp>` of the previous image
- **AND** a later restart of Docker starts no `app` on the old volume

## MODIFIED Requirements

### Requirement: Service of the restore into a new volume
The one-shot service `restore-volume` SHALL have the profile `restore`, `network_mode: none`, the mount `backups:/backups:ro`, a tmpfs on `/run/pgpass`, and the one secret `backup_manifest_key`. With `compose.previous.yaml`, it SHALL mount no secret and no password. `docker compose up` SHALL NOT start it. The operator SHALL run a restore with the `recipe-pgtools` image of the schema of the set. The requirement "Check of the rollback set with the new image" gives the one exception (R31, R34).

#### Scenario: Start of the stack
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator runs `docker compose up -d`
- **THEN** Docker starts no container of `restore-volume`

#### Scenario: Isolation of the restore service
- **GIVEN** the Compose file of the recipe editor
- **WHEN** the operator reads the service `restore-volume`
- **THEN** the service joins no network and mounts only the secret `backup_manifest_key`
- **AND** it mounts `backups` read-only
