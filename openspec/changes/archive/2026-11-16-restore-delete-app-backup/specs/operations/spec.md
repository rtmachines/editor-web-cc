## ADDED Requirements

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
- **THEN** the operator stops `backup` before the check of the set
- **AND** the operator deletes `app` only after that check passes

## MODIFIED Requirements

### Requirement: Order of a recovery from the WAL archive
A recovery of production from the WAL archive SHALL first delete the containers of `app` and `backup`. When `db` runs, the operator SHALL then read the marker entry, and the flush check SHALL pass before `walarchive` stops. `pitr-volume` SHALL fill a new volume before the switch. The new WAL line SHALL start before `app`. The operator SHALL truncate `session` and `login_attempt` before `app` starts. Later starts of `app` and `backup` SHALL use `docker compose up -d --no-deps` (R31, R34, A35).

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
