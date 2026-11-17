## ADDED Requirements

### Requirement: Container of app in a major upgrade
Step 1 of a major upgrade SHALL delete the container of `app` with `docker compose rm -s -f app` before the dump of the `_now` set. After the delete, each start of `app` in the major upgrade SHALL use `docker compose up -d --no-deps app`. This rule SHALL also apply when a step before the switch fails (R31, R34).

#### Scenario: Restart of Docker before the switch of a major upgrade
- **GIVEN** a major upgrade after the delete of the container of `app`
- **WHEN** the Docker daemon starts again before the switch
- **THEN** Docker starts no container of `app`
- **AND** the old cluster gets no write of `app` after the dump of the `_now` set

#### Scenario: Failed step before the switch of a major upgrade
- **GIVEN** a major upgrade after the delete of the container of `app`
- **WHEN** a step before the switch fails
- **THEN** the operator creates `app` again with `docker compose up -d --no-deps app` in the directory of the old release
- **AND** `app` serves again on the old volume

#### Scenario: Host reboot inside the window of a major upgrade
- **GIVEN** a major upgrade after the delete of the container of `app` and before the start of `app`
- **WHEN** the operator runs the runbook section "After a host reboot"
- **THEN** the section does not wait for `app` and `proxy`
- **AND** the operator continues the major upgrade at the step where it stopped

### Requirement: Container of walarchive in a recovery
Step 4 of a recovery from the WAL archive SHALL delete the container of `walarchive` with `docker compose rm -s -f walarchive`. After the delete, each start of `walarchive` in the recovery SHALL use `docker compose up -d --no-deps walarchive`. To end a failed attempt, the operator SHALL first return to the directory `deploy/` of `<tag>` with `RECIPE_VERSION` set to `<tag>`. Only then SHALL the operator create `walarchive` again (R31, R34, A35).

#### Scenario: Restart of Docker after the delete of walarchive
- **GIVEN** a recovery after `docker compose rm -s -f walarchive`
- **WHEN** the Docker daemon starts again before step 10 or the end of a failed attempt creates `walarchive` again
- **THEN** Docker starts no container of `walarchive`
- **AND** the `.partial` file in `/wal` stays unchanged

#### Scenario: Failed attempt after the delete of walarchive
- **GIVEN** a recovery after `docker compose rm -s -f walarchive`
- **WHEN** a value of the last line of `pitr-volume` does not fit and the operator ends the attempt
- **THEN** the operator creates `walarchive` again with `docker compose up -d --no-deps walarchive`

#### Scenario: Failed attempt with a recovery tag that differs from the tag
- **GIVEN** a recovery that runs step 6 in the directory of a `<recovery tag>` that differs from `<tag>`
- **WHEN** the operator ends the attempt after the delete of `walarchive`
- **THEN** the operator changes into the directory `deploy/` of `<tag>` and sets `RECIPE_VERSION` to `<tag>` before `docker compose up -d --no-deps walarchive`
- **AND** the new container of `walarchive` runs the `recipe-pgtools` image of `<tag>`

#### Scenario: Refused drill volume in the switch of a recovery
- **GIVEN** a recovery after `docker compose rm -s -f walarchive` and a new volume with the file `pitr-drill`
- **WHEN** step 8 refuses the volume before sub-step 1 of the switch
- **THEN** the operator does not switch and ends the attempt as in step 7
- **AND** the operator creates `walarchive` again in the directory `deploy/` of `<tag>`

## MODIFIED Requirements

### Requirement: Order of a recovery from the WAL archive
A recovery of production from the WAL archive SHALL first delete the containers of `app` and `backup`. If `db` runs, the operator SHALL read the marker entry, and the flush check SHALL pass before the delete of `walarchive`. `pitr-volume` SHALL fill a new volume before the switch. The new WAL line SHALL start before `app`. The operator SHALL truncate `session` and `login_attempt` before `app` starts. Later starts of `app` and `backup` SHALL use `docker compose up -d --no-deps` (R31, R34, A35).

#### Scenario: Recovery after the loss of the pgdata disk
- **GIVEN** a `db` that does not start, and an intact WAL archive disk
- **WHEN** the operator runs the recovery without a target time
- **THEN** `db` runs on the new volume with every commit that the archive received
- **AND** `walarchive` streams the new WAL line before `app` starts

#### Scenario: Revoked session after the recovery point
- **GIVEN** a session that the old cluster deleted after the recovery point
- **WHEN** the operator truncates `session` before the start of `app`
- **THEN** the session token of that session does not authenticate

#### Scenario: Restart of Docker during a recovery
- **GIVEN** a recovery after `docker compose rm -s -f app backup`
- **WHEN** the Docker daemon starts again before the start of `app`
- **THEN** Docker starts no container of `app` and no container of `backup`
- **AND** no audit entry of `app` reaches the old cluster

### Requirement: Containers of app and backup in a restore into production
After `restore-test` passes and before the operator creates the new volume, a restore into production SHALL delete the containers of `app` and `backup` with `docker compose rm -s -f app backup`. After the delete, each start of `app` or `backup` in the restore SHALL use `docker compose up -d --no-deps`. The rollback after an upgrade SHALL keep its own first step (R31, R34).

#### Scenario: Restart of Docker during a restore
- **GIVEN** a restore into production after the delete of the containers of `app` and `backup`
- **WHEN** the Docker daemon starts again before the switch
- **THEN** Docker starts no container of `app` and no container of `backup`
- **AND** the old cluster gets no write of `app`, and no `backup-once` and no `restore-test` runs

#### Scenario: Failed fill of the new volume
- **GIVEN** a restore into production after the delete of the containers of `app` and `backup`
- **WHEN** `restore-volume` fails and the operator ends the attempt
- **THEN** the operator creates `app` and `backup` again with `docker compose up -d --no-deps`

#### Scenario: Rollback restore
- **GIVEN** a rollback with the `_premigrate` set
- **WHEN** the operator starts the restore
- **THEN** the operator deletes the container of `backup` with `docker compose rm -s -f backup` before the check of the set
- **AND** the operator deletes `app` only after that check passes

#### Scenario: Restart of Docker during a rollback restore
- **GIVEN** a rollback after the delete of the container of `backup`
- **WHEN** the Docker daemon starts again before the switch
- **THEN** Docker starts no container of `backup`
- **AND** no `backup-once` and no `restore-test` runs on the old volume

### Requirement: Check of the rollback set with the new image
In the rollback of ADR-0174 (aj), the operator SHALL first delete the container of `backup` with `docker compose rm -s -f backup`. Then the operator SHALL run `restore-volume --check-only <stamp>` with the new image on a set of the previous schema. Only after a pass SHALL the operator delete the container of `app` with `docker compose rm -s -f app` and run `restore-volume <stamp>` with the previous image (R31, R34).

#### Scenario: Pass of the check of the rollback set
- **GIVEN** a rollback with the `_premigrate` set and no container of `backup`
- **WHEN** `restore-volume --check-only <stamp>` of the new image exits with the code 0
- **THEN** the operator runs `docker compose rm -s -f app` before `restore-volume <stamp>` of the previous image
- **AND** a later restart of Docker starts no `app` on the old volume
