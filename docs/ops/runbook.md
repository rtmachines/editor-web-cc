# Runbook of the recipe editor

This guide gives the operator the steps for the Compose stack of the recipe editor.
Run each `docker compose` command in the directory `deploy/` of the release.
Replace each value in angle brackets.
The rules behind the steps are in [reliability.md](../architecture/reliability.md) and in the ADRs of the section "Related".

## Host preparation

### Check the host

1. Check that the host has at least 8 GiB of memory and 4 vCPU (ADR-0070 (m)).
2. Run `docker version` and `docker compose version`. Engine 25 and Compose 2.24 are the lowest versions.
3. Check that the host clock runs with NTP (`reliability.md` 7.9).
4. Run `printenv | grep -o '^PG[^=]*'`. The command prints nothing when the host is clean.

The command prints names and no values. The server refuses to start with a variable `PG*` (ADR-0069 (b)).

### Create the environment file

1. Copy `.env.example` to `.env`.
2. Set the eight values of `.env`. Each directory is an absolute path.
3. Set `RECIPE_BACKUP_DIR` and `RECIPE_WALARCHIVE_DIR` to directories on the second disk. `RECIPE_WALARCHIVE_DIR` must not be the mount point of the second disk. Then its parent directory is on the second disk too, and the gap procedure moves files there as a rename (ADR-0099 (c)).
4. Set `RECIPE_SECRETS_DIR` to a directory outside the repository.
5. Load the values into the shell. Do not export them:

```bash
. ./.env
```

6. Read the values that Compose uses:

```bash
docker compose config | grep -E 'image: recipe-|subnet:|ip_range:|ipv4_address:|device:|file:|SITE_ADDRESS:'
```

7. Compare each result with `.env`. If one differs, a shell variable wins (ADR-0083 (c)). Run `unset <name>`.
8. Check that no `PG*` name reaches a container:

```bash
docker compose config | grep -o '^ *PG[A-Za-z_]*:'
```

9. Check that the command prints nothing.

The host commands use the shell values only. Compose reads `.env` itself.
A shell variable with the same name wins over `.env`, so repeat steps 6 and 7 after each edit of `.env`.
Compose passes only the entries of `environment:` to a container. A `PG*` variable of the host shell does not reach it.

### Prepare the second disk

1. Create an ext4 or xfs filesystem on the second disk. The publish of a set needs hard links (ADR-0074 (e)).
2. Mount the disk. Do not use a network share (ADR-0068).
3. Create both directories and both marker files:

```bash
install -d -m 0755 -o 999 -g 999 "$RECIPE_BACKUP_DIR"
install -d -m 0700 -o 999 -g 999 "$RECIPE_WALARCHIVE_DIR"
install -m 0600 -o 999 -g 999 /dev/null "$RECIPE_BACKUP_DIR/.backup-marker"
install -m 0600 -o 999 -g 999 /dev/null "$RECIPE_WALARCHIVE_DIR/.walarchive-marker"
```

4. List the disk of the Docker root directory, where the volume `pgdata` will lie, and the disk of each directory:

```bash
df --output=source,target "$(docker info --format '{{.DockerRootDir}}')" "$RECIPE_BACKUP_DIR" "$RECIPE_WALARCHIVE_DIR"
```

5. Check that the column `Filesystem` of the first line differs from the two other lines.

If one line equals the first, the healthcheck of `backup` fails (`reliability.md` 7.7). The section "Service unhealthy" gives the next step.

A network share may not keep the `fsync` of the WAL stream.
The marker files have the names of ADR-0074 (a).
Each script of `backup` and `walarchive` writes nothing without its marker file.
Uid 999 is the user of `recipe-pgtools`.

Without a second disk, the healthcheck of `backup` fails and the recovery point is 24 hours ([A35](../architecture/overview.md), [ADR-0020](../adr/0020-deployment-migrations-backup.md)).

### Create the secret files

The directory `RECIPE_SECRETS_DIR` belongs to `root` and has the mode `0700`. Each file holds one line.

1. Create the directory:

```bash
install -d -m 0700 -o root -g root "$RECIPE_SECRETS_DIR"
```

2. Create the seven random values with the command below.

```bash
( umask 077 && openssl rand -hex 32 > "$RECIPE_SECRETS_DIR/<name>" )
```

Run the command once for each name: `postgres_password`, `migrator_password`, `app_password`, `backup_password`, `replicator_password`, `login_hmac_key`, `csrf_signing_key`.

3. Create the file `alerts_webhook_url`. Write the URL of the webhook into it. The section "Alert webhook" gives the rules of the URL.
4. Leave `alerts_webhook_url` empty when the plant has no webhook:

```bash
install -m 0600 /dev/null "$RECIPE_SECRETS_DIR/alerts_webhook_url"
```

5. Copy the certificate and the key of the plant PKI to `tls_cert` and `tls_key`.
6. Read the owner and the mode of each file in the table below.

| File | Reader | Owner and mode |
|---|---|---|
| `postgres_password`, `backup_password`, `replicator_password` | uid 999 | `999`, `0400` |
| `login_hmac_key`, `csrf_signing_key`, `alerts_webhook_url` | uid 65532 | `65532`, `0400` |
| `tls_cert`, `tls_key` | uid 0 | `0`, `0400` |
| `app_password`, `migrator_password` | uid 999 and uid 65532 | `0`, `0400`, then an ACL |

7. Set the owner and the mode with one `chown` and one `chmod` for each row:

```bash
chown 999 "$RECIPE_SECRETS_DIR"/{postgres_password,backup_password,replicator_password}
chmod 0400 "$RECIPE_SECRETS_DIR"/{postgres_password,backup_password,replicator_password}
chown 65532 "$RECIPE_SECRETS_DIR"/{login_hmac_key,csrf_signing_key,alerts_webhook_url}
chmod 0400 "$RECIPE_SECRETS_DIR"/{login_hmac_key,csrf_signing_key,alerts_webhook_url}
chown 0 "$RECIPE_SECRETS_DIR"/{tls_cert,tls_key}
chmod 0400 "$RECIPE_SECRETS_DIR"/{tls_cert,tls_key}
chown 0 "$RECIPE_SECRETS_DIR"/{app_password,migrator_password}
chmod 0400 "$RECIPE_SECRETS_DIR"/{app_password,migrator_password}
```

8. Give the two files with two readers an ACL for both uids:

```bash
setfacl -m u:999:r,u:65532:r "$RECIPE_SECRETS_DIR/app_password"
setfacl -m u:999:r,u:65532:r "$RECIPE_SECRETS_DIR/migrator_password"
```

9. Check each file with `stat -c '%a %u' <file>`. The two files with an ACL show the mode `440`.
10. Check the ACL of both files with `getfacl`. The owning group has no right.

ADR-0062 (l) to (o) gives the rules of this section.

### Check the host firewall

Run this section after the first start. Docker creates the networks `edge` and `egress` at that start.
The rules follow ADR-0070 (e) and the Docker documentation of the chain `DOCKER-USER`.

1. Read the Engine version with `docker version`. From Engine 28, stop here (ADR-0070 (e)).
2. Read the subnet of each Compose network:

```bash
for n in edge egress backend; do
  docker network inspect --format '{{.Name}} {{range .IPAM.Config}}{{.Subnet}} {{end}}' "recipe-editor_$n"
done
```

3. List the chain with `iptables -S DOCKER-USER`.
4. Add the rules below. Replace each subnet with its value from step 2:

```bash
iptables -I DOCKER-USER -d <edge subnet> ! -s <edge subnet> -m conntrack ! --ctstate DNAT -j DROP
iptables -I DOCKER-USER -d <egress subnet> ! -s <egress subnet> -m conntrack ! --ctstate DNAT -j DROP
iptables -I DOCKER-USER -d <backend subnet> ! -s <backend subnet> -m conntrack ! --ctstate DNAT -j DROP
iptables -I DOCKER-USER -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT
```

5. List the chain again. Check that the `ESTABLISHED,RELATED` rule comes first, then the three `DROP` rules.
6. Run `docker compose ps`. Check that each service with a healthcheck shows `healthy`.
7. Open the login page from a workstation. Check that it loads.
8. Save the rules with the method of the host. A reboot deletes a rule that no tool saves.
9. Run `docker compose config --quiet`. The command prints nothing when the files are valid.

Each `DROP` rule drops routed traffic from outside to a Compose subnet (ADR-0070 (e)).
The Docker chain sees a packet after the address translation. The `conntrack` match `! --ctstate DNAT` therefore keeps the published ports of `proxy` open.
The `-s` match keeps the traffic inside one network. The `ESTABLISHED,RELATED` rule keeps the replies to connections that start inside Compose.
ADR-0063 (p) asks for a rule that limits `egress` to the webhook host. No document gives that rule, so this page gives none.

Run this section again after a subnet change, and after `docker compose down` and `docker compose up`. Docker can give `edge` and `egress` new subnets.
To delete a rule of an old subnet, run its command again with `-D` in place of `-I`.

## First start, first administrator, and image transfer

### Load the images

1. Run `sha256sum <image file>` for each image file of the release.
2. Compare each result with the SHA-256 in the release notes (ADR-0064 (k)).
3. Run `docker load -i <image file>` for each file.
4. Run `docker image inspect --format '{{.Id}}' recipe-server:<version>`. Run the same command for `recipe-pgtools:<version>`.
5. Compare each image ID with the release notes.
6. Set `RECIPE_VERSION` in `.env` to the release tag.
7. Run steps 6 and 7 of the section "Create the environment file". The image tags show the release tag.

### Start the stack

1. Run `docker compose up -d`. Compose starts the services in the order of `reliability.md` 7.1.
2. Run `docker compose ps` until `db`, `app`, `proxy`, `backup`, and `walarchive` show `healthy`.
3. Check that `migrate` and `backup-premigrate` show exit code 0.
4. Run the section "Check the host firewall".

### Check the client rules of the database

The init script writes the file `pg_hba.conf` on the first start only (ADR-0069 (r) to (t)).
The expected lines are below. `<subnet>` is the value of `COMPOSE_SUBNET`.

```text
local   all          postgres                            peer
host    recipe       recipe_migrator    <subnet>         scram-sha-256
host    recipe       recipe_app         <subnet>         scram-sha-256
host    recipe       recipe_backup      <subnet>         scram-sha-256
host    replication  recipe_replicator  <subnet>         scram-sha-256
host    recipe       pg_isready         127.0.0.1/32     scram-sha-256
host    all          all                all              reject
```

1. Read the rules of the running server:

```bash
docker compose exec -u postgres db psql -d recipe -c "SELECT line_number, type, database, user_name, address, auth_method, error FROM pg_hba_file_rules ORDER BY line_number"
```

2. Compare the result with the expected lines. The server shows exactly seven rows.
3. Check that the column `error` is empty in each row.
4. Look for the catch-all line of the image. It has `{all}` as database, `{all}` as user, and the method `scram-sha-256`.
5. If the row is present, apply the section "An older volume" now. Do not delete `pgdata`.

### Create the first administrator

The command does nothing when an active administrator exists. If the only administrator cannot log in, see [Known gaps](#known-gaps).
The new administrator must change the password at the first login.

1. Run the command below. It reads the password without an echo and passes it on stdin.

```bash
read -r -s -p 'Password: ' admin_password
printf '%s\n' "$admin_password" | docker compose run --rm --no-deps -T app create-admin --login <login>
unset admin_password
```

2. Open `https://<SITE_ADDRESS>/login` and log in as the new administrator.
3. Change the password at `/me/password`.
4. Run the check of the section "Check client_ip of the first login".

## Stop, start, and restart

The services `proxy`, `app`, `db`, `backup`, and `walarchive` have the restart policy `always`.
The services `backup-premigrate` and `migrate` are one-shot services with the policy `no`. They run once at a start and exit with code 0 (`reliability.md` 7.1).
Compose does not run a one-shot service again unless a command starts it.

### Stop the whole stack

1. Choose a time away from `backup.time`, `basebackup.time`, and `restore_test.time` of `reliability.md` 7.11.
2. Run `docker compose top backup walarchive`. Check that no process `backup-once`, `basebackup-once`, or `restore-test` runs.
3. Run `docker compose stop proxy`. No new request then reaches `app`.
4. Run `docker compose stop app`. Docker waits at most `app.stop_grace_period` for the exit.
5. Run `docker compose stop`. Compose stops `backup` and `walarchive` first and `db` last. Docker waits at most `db.stop_grace_period` for `db`.
6. Run `docker compose ps -a`. Check that `app` shows `Exited (0)`.
7. Run `docker compose logs --tail 20 app`. Check that one `INFO` line says `the server stopped`.

The order of steps 3 to 5 is the reverse of the start order in `reliability.md` 7.1.
`db` stops last, so `app` closes its database pools while `db` still runs (`reliability.md` 7.3).
The exit code of `app` is 0, also after a cut drain.
After a cut, one `WARN` line gives the counts `cut_public`, `cut_internal`, and `cut_jobs`. A cut loses no data (ADR-0087).
If the exit code is not 0, Docker may have sent `SIGKILL`. Read the last lines of `docker compose logs app`.
A stop during a run of `backup` or `walarchive` can leave temp files. The section "Delete orphan temp files" deletes them.

### Start the whole stack again

Use these steps after a stop of the same release. After a change of `RECIPE_VERSION`, use the section "Upgrade".
Do not run `docker compose up -d` here. Compose then runs `backup-premigrate` and `migrate` again.
The run of `backup-premigrate` writes a new `_premigrate` set of the current schema.
That new set is then the newest one. The section "Rollback" restores the newest `_premigrate` set.
The option `--no-deps` skips the `depends_on` checks (ADR-0082), so the steps wait for `healthy` by hand.

1. Run `docker compose up -d --no-deps db`.
2. Run `docker compose ps db` until `db` shows `healthy`.
3. Run `docker compose up -d --no-deps app`.
4. Run `docker compose ps app` until `app` shows `healthy`.
5. Run `docker compose up -d --no-deps proxy backup walarchive`.
6. Run `docker compose ps -a`. Check that `db`, `app`, `proxy`, `backup`, and `walarchive` show `healthy`.
7. Run `docker compose exec app /usr/local/bin/recipe-server healthcheck --path /readyz`. Check that the exit code is 0.

A service shows `starting` until its first healthcheck passes.
`migrate` and `backup-premigrate` keep the exit code 0 of their last run. After `docker compose down`, they do not exist.
If `app` does not become `healthy`, use the section "Alarms, read-only mode, and restart loop".

### Restart one service

Replace `<service>` with `proxy`, `app`, `db`, `backup`, or `walarchive`.

1. Run `docker compose stop <service>`.
2. Run `docker compose up -d --no-deps <service>`.
3. Run `docker compose ps <service>` until the service shows `healthy`.

Without `--no-deps`, a start of `app` or `proxy` runs `backup-premigrate` and `migrate` again (ADR-0082).
Do not restart `backup-premigrate` or `migrate` by hand. The section "Upgrade" runs them.
Start `db` before `app`. The option `--no-deps` does not wait for `db`.
While `app` restarts, Caddy answers `502` (ADR-0065 (p)). While `db` is down, `/readyz` answers `not ready` (`reliability.md` 7.2).
After a restart of `db`, check the health of `app` with the last step of the section "Start the whole stack again".
A long stop of `db` or `walarchive` can leave a gap in the WAL stream. See the section "A gap in the WAL stream".
Before you stop `backup` or `walarchive`, do steps 1 and 2 of the section "Stop the whole stack".

### Stop and start, or down and up

- `docker compose stop` keeps the containers and the networks. The next start reuses them.
- `docker compose down` deletes the containers and the networks. It deletes no volume.
- The next `docker compose up` creates the containers again and can give `edge` and `egress` new subnets.
- The network `backend` keeps `COMPOSE_SUBNET`.

After `docker compose down` and `docker compose up`, run the section "Check the host firewall" again. The `DROP` rules name the subnets.
A deliberate change of `COMPOSE_SUBNET` follows the section "Subnet change".
After `docker compose down`, use the steps of "Start the whole stack again". They create the containers.
Never run `docker compose down -v` (section "Destructive commands").

### After a host reboot

When the Docker service starts, Docker starts the containers with the policy `always`.
Docker does not apply `depends_on` then. `app` can fail its start while `db` is not ready, and Docker starts it again (ADR-0061 (b)).
The one-shot services do not run, because their policy is `no`.
A service that you stopped with `docker compose stop` can start again after a reboot.

1. Run `docker compose ps -a`. Wait until `db`, `app`, `proxy`, `backup`, and `walarchive` show `healthy`.
2. If a service restarts, run `docker compose logs <service>`. For `app`, use the section "Alarms, read-only mode, and restart loop".
3. Check that the host mounts the second disk. A missing host directory stops the start of `backup` and `walarchive`.
4. Run `docker compose exec app /usr/local/bin/recipe-server healthcheck --path /readyz`. Check that the exit code is 0.
5. Run `iptables -S DOCKER-USER`. Check that the `ESTABLISHED,RELATED` rule and the three `DROP` rules are present.
6. If a rule is missing, run the section "Check the host firewall" again. Save the rules with the method of the host.

A host reboot ends the containers, so the WAL stream stops until `walarchive` runs again.
If the alert `replication` stays active, use the section "A gap in the WAL stream".

## Backup now, restore test, and restore into production

### Run a backup now

1. Run `docker compose exec backup backup-now`.
2. Check that the output holds the line `backup completed` with the name of the new `_now` set.

### Run the restore test

1. Run `docker compose exec backup restore-test`.
2. Check that the exit code is 0 and the output holds the line `restore test passed`.
3. Check that the line names the set that you expect (ADR-0067).

### Statement outside the role grammar

The restore test stops at a statement of the globals file outside the role grammar (ADR-0095).
A role setting outside the grammar comes from a change by hand.

1. Read the line number in the `ERROR` line of the output.
2. Open the globals file `globals_<YYYYMMDD_HHMM><suffix>.sql` of the set that the test checked.
3. Read the line with that line number.
4. Delete that setting from the role in the database.
5. If the setting is needed, extend the grammar in a change (ADR-0095 (e)).
6. Run `docker compose exec backup backup-now`.
7. Run `docker compose exec backup restore-test`.
8. Check that the exit code is 0 and the output holds the line `restore test passed`.

### Copy the backups off site

1. Copy the latest dump and the WAL archive to the external backup storage with the site procedure (ADR-0020).
2. Protect the copies with the same access control as the database.

### Delete orphan temp files

A killed run leaves files with the name `.tmp-<final name>` on the backup disk.
It also leaves directories `.tmp-base_*` in `base/` on the WAL archive disk (ADR-0074 (b), ADR-0077 (h)).
This step deletes only these files and directories.

1. Choose a time away from `backup.time`, `basebackup.time`, and `restore_test.time` of `reliability.md` 7.11.
2. Run the restore test of the previous section. Continue only after it passes.
3. Run `docker compose top backup walarchive`. Check that no process `backup-once`, `basebackup-once`, or `restore-test` runs.
4. Check that `backup-premigrate` does not run with `docker compose ps -a backup-premigrate`.
5. List the files that the next step deletes:

```bash
find "$RECIPE_BACKUP_DIR" -maxdepth 1 -type f -name '.tmp-*'
find "$RECIPE_WALARCHIVE_DIR/base" -maxdepth 1 -type d -name '.tmp-base_*'
```

6. Delete the files and the directories of the list:

```bash
find "$RECIPE_BACKUP_DIR" -maxdepth 1 -type f -name '.tmp-*' -delete
find "$RECIPE_WALARCHIVE_DIR/base" -maxdepth 1 -type d -name '.tmp-base_*' -exec rm -r -- {} +
```

The marker files, the lock files, and every complete set stay.
A live run writes under a temp name (`reliability.md` 7.7, ADR-0077 (h)).
The check of step 3 shows one instant, so the time of step 1 keeps a scheduled run away from steps 5 and 6.

### Restore into production

Warning: this procedure is not complete for 0.1.0. See [Known gaps](#known-gaps).

The restore goes into a new `db` volume (ADR-0020).
The old volume `pgdata` stays on the host.
The step that makes a new volume the volume of `db` is open. No document gives its command. See "Known gaps".
The steps below start with an empty new cluster as `db`. It holds no database `recipe` and no project role.
`<stamp>` is the part of the dump file name between `recipe_` and `.dump`, for example `<YYYYMMDD_HHMM>_now`.

Warning: a restore of a dump loses every change and audit entry after the dump time. The loss is up to 24 hours.
Keep `pgdata` and the WAL archive disk unchanged.
A recovery from the WAL archive gives a recovery point of minutes (`reliability.md` 7.7, ADR-0068 (l), R31).
This page gives no steps for that recovery. See "Known gaps".

1. Run the restore test. Continue only after it passes. The test checks the newest complete set.
2. Run `docker compose stop app`.
3. Read the name of the set in the line `restore test passed`. Restore that set.
4. Restore the globals file. The file gives one known error, the existing role `postgres`:

```bash
docker compose exec -T -u postgres db psql --no-psqlrc --no-password --dbname postgres < "$RECIPE_BACKUP_DIR/globals_<stamp>.sql"
```

5. Restore the dump. Any `ERROR` line stops the restore:

```bash
docker compose exec -T -u postgres db pg_restore --exit-on-error --no-password --create --dbname postgres < "$RECIPE_BACKUP_DIR/recipe_<stamp>.dump"
```

6. Set the password of each login role from its secret file (ADR-0033). Never type a password on a command line:

```bash
docker compose exec -T -u postgres db psql -v ON_ERROR_STOP=1 --no-password --no-psqlrc --dbname postgres <<'SQL'
\set postgres_password `cat /run/secrets/postgres_password`
\set migrator_password `cat /run/secrets/migrator_password`
\set app_password `cat /run/secrets/app_password`
\set backup_password `cat /run/secrets/backup_password`
\set replicator_password `cat /run/secrets/replicator_password`
SET log_statement = 'none';
SET log_min_error_statement = 'panic';
SET log_min_duration_statement = -1;
ALTER ROLE postgres PASSWORD :'postgres_password';
ALTER ROLE recipe_migrator PASSWORD :'migrator_password';
ALTER ROLE recipe_app PASSWORD :'app_password';
ALTER ROLE recipe_backup PASSWORD :'backup_password';
ALTER ROLE recipe_replicator PASSWORD :'replicator_password';
SQL
```

Psql reads each secret file inside `db`, so no password reaches a command line.
The setting `log_min_duration_statement = -1` stops the slow-statement line from holding a password (`reliability.md` 3.4).
Count the lines of the log of `db` that name a role next to `PASSWORD`. The count prints no log line:

```bash
docker compose logs db | grep -c -E 'ROLE (postgres|recipe_[a-z]+) PASSWORD'
```

The count must be 0. A count above 0 means a password may be in the log.

7. Truncate the tables `session` and `login_attempt`:

```bash
docker compose exec -u postgres db psql -d recipe -c "TRUNCATE session, login_attempt"
```

8. Re-apply each user deactivation and password reset recorded after the dump time. Use the off-site audit copy, if one exists.
9. Run `docker compose up -d --no-deps app`.
10. Check the audit chain: wait for the next run of the job `audit_chain_check`. The alert `audit_chain_broken` stays inactive.

No command runs the chain check on demand. The job runs at `audit.check_time` of `reliability.md` 7.11.
After a start of `app`, it runs once more after `audit.catchup_delay`. It does so only when `audit.last_check_at` is missing or older than `audit.catchup_age`.
The value `audit.last_check_at` shows the time of the last completed check (`data-model.md` 4.9). Read it:

```bash
docker compose exec -u postgres db psql -d recipe -c "SELECT value FROM app_setting WHERE key = 'audit.last_check_at'"
```

The alert `audit_check_age` is active when this value is older than `audit.max_check_age`.

## Upgrade and rollback

The steps follow `reliability.md` 7.10 (ADR-0082, ADR-0083).

### Upgrade

1. Read the release notes. Note whether the migration follows the expand pattern. Note the line "manual superuser step".
2. Avoid the times `backup.time`, `basebackup.time`, and `restore_test.time` of `reliability.md` 7.11 for the whole upgrade.
3. Run `docker compose exec backup restore-test` with the old image. Continue only after it passes.
4. Load the new images as in the section "Load the images". Do not stop the running stack.
5. Set `RECIPE_VERSION` in `.env` to the new release tag.
6. Run steps 6 and 7 of the section "Create the environment file". The image tags show the new tag.
7. If the plant has a device, use steps 1 to 3 of "A plant device in front of Caddy" again.
8. Run `docker compose up -d backup-premigrate`. It writes the rollback set `_premigrate`.
9. Wait for exit code 0 of `backup-premigrate` with `docker compose ps -a backup-premigrate`.
10. Run `docker compose up -d --no-deps migrate`. Without `--no-deps`, Compose runs `backup-premigrate` a second time.
11. Wait for exit code 0 of `migrate` with `docker compose ps -a migrate`.
12. If `backup-premigrate` or `migrate` fails, stop. Set the previous tag in `.env`. Check it (step 6). The old `app` keeps running.
13. Start the guard step. Run `docker compose exec -u postgres db psql -d recipe`.
14. Run the statement `CREATE FUNCTION audit_guard.audit_objects_dropped()` of the new `deploy/db/initdb/01-roles.sh` as `CREATE OR REPLACE FUNCTION`.
15. Compute the hash of the function in the same session:

```sql
SET search_path = pg_catalog;
SELECT encode(sha256(convert_to(pg_get_functiondef('audit_guard.audit_objects_dropped()'::regprocedure), 'UTF8')), 'hex');
```

16. Leave `psql` with `\q`. Read field 4 of the expected line:

```bash
grep -F 'FUNCTION|audit_guard audit_objects_dropped()|' deploy/pgtools/restore-code.expected | cut -d'|' -f4
```

17. Compare the two hashes. Continue only when they are equal. Otherwise run steps 13 to 17 again.
18. Run `docker compose up -d --no-deps app`. Docker replaces the container of `app` only.
19. Check `/readyz` with `docker compose exec app /usr/local/bin/recipe-server healthcheck --path /readyz`. The command ends with exit code 0.
20. If you used step 7, run `docker compose up -d --force-recreate --no-deps proxy`.
21. Run the check of the section "Check client_ip of the first login".
22. Log in, open one recipe, and export one version. Check its hash as `api.md` section 6 describes.
23. Run the three commands of the next list while the new `app` serves.

The restore test of step 3 checks the newest complete set of the old schema with the old image (ADR-0083).
The rollback set of step 8 holds the old guard function. The old image therefore accepts it after a rollback restore (ADR-0097 (h1)).
Only the superuser can replace the guard function (ADR-0069 (p)). The release notes name steps 13 to 17 as a "manual superuser step".
Run no `backup-now` before step 17 passes.
The guard step is the one place that replaces the guard function. The section "An older volume" uses this place too.

Never run two `app` containers at the same time (ADR-0096).
The old `app` has no guard lock of the last Administrator, so the two versions do not exclude each other.
Step 18 replaces the one container of `app`.

The old `app` of step 12 keeps running on the old schema only when the migration follows the expand pattern.
A release with two or more migration files can fail after the first file. Then no image matches the schema.
Choose one of two actions (ADR-0083). The first is a forward fix. The second is a restore of the `_premigrate` dump with the second list of the section "Rollback".

Recovery after a failure between two steps (ADR-0097 (h2)):

- Step 8 or 9 fails: nothing changed. Set the previous tag in `.env` and stop.
- Step 11 fails: follow step 12 and ADR-0083. The guard function stays old.
- A step from 13 to 17 fails: the schema is new and the guard function is old. Run steps 13 to 17 again. Run no `backup-now` until step 17 passes.
- A step of the next list fails: keep the new `app` and every set. Read the `ERROR` line (ADR-0083 (f)).

The three commands give the new image of `recipe-pgtools` to `backup` and `walarchive`:

1. Run `docker compose up -d --no-deps backup walarchive`.
2. Run `docker compose exec backup backup-now`.
3. Run `docker compose exec backup restore-test`. Check that the test passes and names the `_now` set.
   - If the test names a `_premigrate` set of the same UTC minute, rerun commands 2 and 3 later.

Command 1 can stop a job that runs at `backup.time` or `basebackup.time`. Step 2 avoids these times.
If the restore test of command 3 fails, a rollback does not start by itself (ADR-0083 (f)).
Keep the new `app`. Keep every set. Read the `ERROR` line in the output of `restore-test`.

### Rollback

1. Run the command below to start the previous `app` on the newer schema:

```bash
RECIPE_VERSION=<previous tag> docker compose up -d --no-deps app
```

2. Keep the new tag in `.env`. The schema stays newer.
3. Run the three commands of the upgrade with the tag of `.env`.

Warning: `.env` holds the new tag, so a later `docker compose up -d` gives `app` the new image again (ADR-0083).

If the migration did not follow the expand pattern, use the `_premigrate` dump.
Warning: this restore loses every change and audit entry after the `_premigrate` dump.
The lost period runs from the `_premigrate` dump to the stop of `app`.
No test checks the `_premigrate` set. The test before the upgrade checks a set of the same schema (ADR-0083).
The restore goes into a new `db` volume. The switch step is open, as the section "Restore into production" says. See "Known gaps".

1. Run `docker compose stop app`.
2. Use steps 4 to 8 of "Restore into production" for the newest `_premigrate` set. Use its name as `<stamp>`.
3. Set the previous tag in `.env`.
4. Run steps 6 and 7 of the section "Create the environment file". The image tags show the previous tag.
5. Run `docker compose up -d --no-deps app`.
6. Run step 10 of the section "Restore into production".
7. Run the three commands of the upgrade with the tag of `.env`.

### Concurrent index on an audit object

`reliability.md` 7.10 step 7 and `data-model.md` 5.1 give this procedure.
It is not part of the normal upgrade.

## Major upgrade

Warning: this procedure is not complete for 0.1.0. See [Known gaps](#known-gaps).

A major upgrade of PostgreSQL is a dump and a restore into a new `db` volume.
The new volume uses the same `initdb` arguments as `compose.yaml`. The project does not use `pg_upgrade` (ADR-0020).

1. Run `docker compose stop app`. The next dump then holds every change.
2. Run `docker compose exec backup backup-now`. The new `_now` set holds the globals file and the dump (`reliability.md` 7.7).
3. Run `docker compose exec backup restore-test`. Continue only after it passes.
4. Check that the line `restore test passed` names the `_now` set of step 2.

The test comes before the switch (`reliability.md` 7.10 step 8).
The switch makes the new volume the volume of `db`. No document gives the command of this switch (ADR-0020), so the switch step is open. See "Known gaps".
After the switch, restore the `_now` set with steps 4 to 10 of the section "Restore into production". Use the name of the `_now` set for `<stamp>`.
Keep the old volume `pgdata` on the host.

## Key rotation

The files `login_hmac_key` and `csrf_signing_key` hold one key per line, the newest key first (ADR-0035).

1. Run the command below once for each key file. Use `login_hmac_key` and `csrf_signing_key` for `<name>`.

```bash
( umask 077 && { openssl rand -hex 32; cat "$RECIPE_SECRETS_DIR/<name>"; } > "$RECIPE_SECRETS_DIR/<name>.new" )
```

The command writes the new key as the first line and keeps the old line. It prints no key.

2. Count the keys and the lines of each new file. The two numbers must be equal, and each must be 2:

```bash
grep -c -E '^[0-9a-f]{64}$' "$RECIPE_SECRETS_DIR/<name>.new"
grep -c '' "$RECIPE_SECRETS_DIR/<name>.new"
```

3. Set the owner and the mode of each new file. Then move it over the old file:

```bash
chown 65532 "$RECIPE_SECRETS_DIR/<name>.new"
chmod 0400 "$RECIPE_SECRETS_DIR/<name>.new"
mv "$RECIPE_SECRETS_DIR/<name>.new" "$RECIPE_SECRETS_DIR/<name>"
```

4. Run `docker compose up -d --force-recreate --no-deps app`.
5. Run `docker compose ps app` until `app` shows `healthy`.
6. Open the login page from a workstation. Log in with your account. Check that the login works.
7. Wait for the value of `security.key_rotation_overlap` (`reliability.md` 7.11). Count the time from step 4.
8. Delete the old line from the key file. This step deletes one key and no data.
9. Run the two commands of step 2 on `"$RECIPE_SECRETS_DIR/<name>"`. Check that the two numbers are equal and that each is 1.
10. Run `stat -c '%a %u' "$RECIPE_SECRETS_DIR/<name>"`. Check that the output is `400 65532`.
11. Run `ls -A "$RECIPE_SECRETS_DIR"`. Check that the list holds no editor copy of the key file.
12. Run `docker compose up -d --force-recreate --no-deps app` again.
13. Run `docker compose ps app` until `app` shows `healthy`.
14. Log in again from a workstation. Check that the login works.

The server accepts every key of the file.
The server stops the start of `app` when a line of a key file is not 64 lower-case hex characters (ADR-0035 (d)).
Then use the section "Restart loop" to read the `ERROR` line. The line names the key `security.login_hmac_key_file` or `security.csrf_signing_key_file`.
Steps 1 to 3 need no check of the owner and the mode, because they set both before the move.
An editor can change the owner or the mode in step 8 and can leave a copy in the directory.

The key `login_hmac_key` has one more effect. The filter `login` of `/security-events` signs the typed login with the first key (`api.md` 2.9).
After step 4, the filter finds an event of an unknown login only when the server wrote the event under the new key.
An event of a login that belongs to a user stays visible, because the filter also matches the user.
The key `csrf_signing_key` signs the pre-session cookie of the login page. After step 12, a login page that was open before step 4 may fail.
The user then gets `403 E-WEB-007`. The user opens the login page again to get a new cookie.

## Certificate rotation

1. Keep a copy of the old pair:

```bash
cp -p "$RECIPE_SECRETS_DIR/tls_cert" "$RECIPE_SECRETS_DIR/tls_cert.old"
cp -p "$RECIPE_SECRETS_DIR/tls_key" "$RECIPE_SECRETS_DIR/tls_key.old"
```

2. Check that `openssl` reads the new certificate. The command prints the subject and the end date:

```bash
openssl x509 -in <new certificate> -noout -subject -enddate
```

3. Check that the new key matches the new certificate. The command prints nothing when both public keys are equal:

```bash
diff <(openssl x509 -in <new certificate> -noout -pubkey) <(openssl pkey -in <new key> -pubout)
```

4. Copy the new certificate and the new key to `tls_cert` and `tls_key`.
5. Check the owner `0` and the mode `0400` of both files.
6. Run `docker compose up -d --force-recreate --no-deps proxy`.
7. Open the login page from a workstation. Check that the browser accepts the certificate.
8. Delete the two copies with `rm -- "$RECIPE_SECRETS_DIR"/tls_cert.old "$RECIPE_SECRETS_DIR"/tls_key.old`. This step deletes the old pair only.

A rotation recreates the container `proxy`, so the service stops for a short time (ADR-0065 (c)).
If TLS fails, copy the two `.old` files over `tls_cert` and `tls_key`. Then repeat step 6.
A bad pair stops TLS, so do not skip steps 2 and 3.
A change of the key alone needs a new certificate, because the certificate holds the public key. Use the same steps with the new pair.

## Change a database password

Change a password when it may be exposed.
`db` reads the password files at its init only (`reliability.md` 7.1, `01-roles.sh`). A new file therefore changes no password in the database.
The step `ALTER ROLE` sets the new password from the new file, as step 6 of "Restore into production" does (ADR-0033).
A running container keeps the file that it started with (ADR-0065 (c)). `db` and the service that uses the password must start again.

| Role | File | Services that read the file | Start again after the change |
|---|---|---|---|
| `postgres` | `postgres_password` | `db` only | No service |
| `recipe_migrator` | `migrator_password` | `migrate` | No service. `migrate` reads the file at its next run |
| `recipe_app` | `app_password` | `app` | `app` |
| `recipe_backup` | `backup_password` | `backup` and `backup-premigrate` | `backup` |
| `recipe_replicator` | `replicator_password` | `walarchive` | `walarchive` |

Replace `<name>` with a file of the table. Replace `<role>` with the role of the same row.

1. Do steps 1 and 2 of the section "Stop the whole stack". No run of `backup` or `walarchive` must be active.
2. Create the new file. The command prints no password:

```bash
( umask 077 && openssl rand -hex 32 > "$RECIPE_SECRETS_DIR/<name>.new" )
```

3. Set the owner and the mode of the new file as the table of "Create the secret files" gives.
4. For `app_password` and `migrator_password`, give the new file the ACL of step 8 of "Create the secret files".
5. Move the new file over the old file:

```bash
mv "$RECIPE_SECRETS_DIR/<name>.new" "$RECIPE_SECRETS_DIR/<name>"
```

6. Run `docker compose up -d --force-recreate --no-deps db`.
7. Run `docker compose ps db` until `db` shows `healthy`.
8. Set the new password. Psql reads the file inside `db`, so no password reaches a command line:

```bash
docker compose exec -T -u postgres db psql -v ON_ERROR_STOP=1 --no-password --no-psqlrc --dbname postgres <<'SQL'
\set new_password `cat /run/secrets/<name>`
SET log_statement = 'none';
SET log_min_error_statement = 'panic';
SET log_min_duration_statement = -1;
ALTER ROLE <role> PASSWORD :'new_password';
SQL
```

9. Count the lines of the log of `db` that name a role next to `PASSWORD`. The count must be 0:

```bash
docker compose logs db | grep -c -E 'ROLE (postgres|recipe_[a-z]+) PASSWORD'
```

10. Run `docker compose up -d --force-recreate --no-deps <service>` for the service of the last column of the table.
11. Run `docker compose ps -a`. Check that `db`, `app`, `proxy`, `backup`, and `walarchive` show `healthy`.
12. For `recipe_app`, run `docker compose exec app /usr/local/bin/recipe-server healthcheck --path /readyz`. Check that the exit code is 0.
13. For `recipe_backup`, run `docker compose exec backup backup-now`. Check that the output holds the line `backup completed`.
14. For `recipe_replicator`, run `docker compose logs --tail 20 walarchive`. Check that the output holds no `ERROR` line.

Do steps 8 to 10 without a pause. Between them, a new connection of the service can fail with the old password.
If step 8 fails, the role keeps its old password. Run step 8 again and do not start step 10 first.
During the restart of `db` in step 6, `app`, `backup`, and `walarchive` cannot reach the database.
Read the notes of the section "Restart one service". A long stop of `db` can leave a gap in the WAL stream. See the section "A gap in the WAL stream".
For `recipe_migrator`, no service uses the new password before the next run of `migrate` in the section "Upgrade". No document gives an earlier test.
For `postgres`, no service of the stack uses the password after the init of `db`.

## Alert webhook

The job `alerts` sends one HTTPS `POST` for each active alert condition to the webhook host (`reliability.md` 3.7, ADR-0063).
The body is `{"alert":"<name>"}` with the header `Content-Type: application/json`. It holds no other field (ADR-0063 (l)).
The value `<name>` is a name from the table in the section "Alarms" (ADR-0075 (c)).
The secret file `alerts_webhook_url` holds the URL. The file `compose.yaml` sets `RECIPE_ALERTS__WEBHOOK_URL_FILE` to its path in `app`.
The section "Create the secret files" creates the file. The server reads the file at each start only.

### Rules of the URL

The server checks the file at each start. A broken file stops the start (ADR-0063 (a), (b)).

- The file holds one line. The server accepts one final line feed.
- The file holds no carriage return.
- The URL is absolute and has a host.
- The scheme of the URL is `https`.
- The URL holds a token of at least 128 random bits (ADR-0063 (e)). No code checks this rule.
- An empty file means no webhook. The start then writes one `INFO` line.

The client follows no redirect. If the webhook host redirects, write the final URL into the file (`reliability.md` 3.7).
With the settings of `compose.yaml`, the client trusts the webpki roots only. A certificate of a plant authority may therefore fail. See the section "Known gaps".
One run sends all calls at the same time. The value of `alerts.webhook_timeout` limits them together (ADR-0063 (m)).

### Change the webhook

The URL is a secret. Do not put it in a shell command, because the shell history keeps it.

1. Create an empty new file:

```bash
install -m 0600 /dev/null "$RECIPE_SECRETS_DIR/alerts_webhook_url.new"
```

2. Open the new file in an editor. Write the new URL as one line.
3. Set the owner and the mode of the new file. Then move it over the old file:

```bash
chown 65532 "$RECIPE_SECRETS_DIR/alerts_webhook_url.new"
chmod 0400 "$RECIPE_SECRETS_DIR/alerts_webhook_url.new"
mv "$RECIPE_SECRETS_DIR/alerts_webhook_url.new" "$RECIPE_SECRETS_DIR/alerts_webhook_url"
```

4. Run `ls -A "$RECIPE_SECRETS_DIR"`. Check that the list holds no editor copy of the URL.
5. Run `docker compose up -d --force-recreate --no-deps app`.
6. Run `docker compose ps app` until `app` shows `healthy`.
7. Compute the fingerprint of the file. The command prints a hash and does not print the URL:

```bash
tr -d '\n' < "$RECIPE_SECRETS_DIR/alerts_webhook_url" | sha256sum
```

8. Open `/audit` as an Administrator. Filter the action `SettingChanged`.
9. Open the newest entry of the key `alerts.webhook_url`. Compare the value of `after` with `sha256:` and the hash of step 7.

While `app` restarts, Caddy answers `502` (ADR-0065 (p)).
The server writes the entry `SettingChanged` at the start, and only when the fingerprint differs (`reliability.md` 7.12).
The entry shows that the URL changed. It does not show the host (ADR-0063 (c)).
No command sends a test call. A call goes out only for an active condition. The entry therefore checks the file and does not check the connection.
If `app` does not become `healthy`, read its `ERROR` line. The line names the key `alerts.webhook_url_file` and one of these texts:

- `holds a carriage return`
- `holds more than one line`
- `does not use the scheme https`
- `holds a text that is not an absolute URL`
- `cannot be read` and an error kind

Repeat the steps from step 1 with a new URL.

### Disable the webhook

1. Do steps 1 to 6 of the section "Change the webhook". In step 2, leave the new file empty.
2. Run `docker compose logs --no-log-prefix app | grep -F 'alerts have no webhook'`. Check that the output names the key `alerts.webhook_url_file`.

The entry `SettingChanged` for the key `alerts.webhook_url` then shows an empty value in `after`.

### Find a failed call

A failed call writes one `WARN` line with the field `condition`. The field holds the name of the condition. The line holds no part of the URL (ADR-0063 (g), (n)).
Read the lines of the last day:

```bash
docker compose logs --no-log-prefix --since 24h app | grep -F '"condition":"'
```

| Text of the line | Possible cause | Action |
|---|---|---|
| `The connection to the webhook host failed.` | The webhook host is down, or the name or the route from the network `egress` fails. | Check the host name in the URL. Check the route from `egress` with the owner of the plant network. |
| `The TLS handshake with the webhook host failed.` | The certificate of the webhook host may come from a plant authority that the webpki roots do not hold. | See the section "Known gaps". |
| `The webhook call took too long.` | The webhook host did not answer within `alerts.webhook_timeout`. | Check the load of the webhook host. |
| `The webhook host answered with the status <code>.` | The status is outside `2xx`. | For `301`, `302`, or `303`, write the final URL. For other codes, check the URL and the token with the owner of the webhook host. |

The job does not retry a failed call. The next run sends the call again while the condition stays active. The value of `alerts.interval` sets the time between runs.
A short condition may therefore end before the webhook host gets the call. The `alert` lines in the log still show it.
The job writes no line for a call that works. A missing line with `condition` shows no fault. It does not show that the webhook host got the call.

## Subnet change

A change of the subnet edits `COMPOSE_SUBNET`, `COMPOSE_IP_RANGE`, and `PROXY_ADDRESS` together.
`PROXY_ADDRESS` lies inside `COMPOSE_SUBNET` and outside `COMPOSE_IP_RANGE`.
The host lines of `pg_hba.conf` name the subnet, so they change too.

1. Run the restore test. Continue only after it passes.
2. Set the three new values in `.env`.
3. Run steps 6 and 7 of the section "Create the environment file". The new values show.
4. Write the lines of `pg_hba.conf` with the new subnet. Use steps 5 to 8 of the section "An older volume".
5. Run `docker compose down`. The command deletes the containers and the networks. It deletes no volume.
6. Run `docker compose up -d`. After a rollback, read the warning in the section "Rollback" first.
7. Run the check of the section "Check the client rules of the database".
8. Run the section "Check the host firewall" again.

If `db` does not start, put the old file back in the volume of `db`. Then change the lines and start again:

```bash
docker compose run --rm --no-deps -u postgres --entrypoint sh db -c 'cp -p -- "$PGDATA/pg_hba.conf.old" "$PGDATA/pg_hba.conf"'
```

## A gap in the WAL stream

An outage of `walarchive` longer than `pg.wal_keep_size` leaves a gap (ADR-0068 (e), ADR-0099).
The alert `replication` is active during the outage.

The procedure moves the WAL segment files and the `.partial` file from the top of `RECIPE_WALARCHIVE_DIR` (ADR-0068 (l)).
The marker file, `.lock`, the directory `base`, and the directory `.pitr-scratch` stay.
The names of the segment files have 24 upper-case hex characters (PostgreSQL 16 documentation, section 30.6).
The first 8 characters give the timeline, the next 8 the log number, and the last 8 the segment number.
`pg_receivewal` adds the suffix `.partial` to the segment that it writes. Every other name at the top of `RECIPE_WALARCHIVE_DIR` stays in place.
Run all steps in one shell, because steps 3 to 6 use the variable `target`.

1. Run `docker compose exec backup restore-test`. Continue only after it passes.
2. Run `docker compose stop walarchive`.
3. Create the target directory. The command fails on an existing name:

```bash
target="${RECIPE_WALARCHIVE_DIR%/}.moved-$(date -u +%Y%m%d_%H%M%S)"
mkdir -m 0700 -- "$target"
```

4. Check that both paths show the same number. If the numbers differ, stop the procedure before any move:

```bash
stat -c %d "$RECIPE_WALARCHIVE_DIR" "$target"
```

5. List the files that step 6 moves:

```bash
find "$RECIPE_WALARCHIVE_DIR" -maxdepth 1 -type f \( -name '[0-9A-F]???????????????????????' -o -name '[0-9A-F]???????????????????????.partial' \)
```

6. Move the files of the list:

```bash
find "$RECIPE_WALARCHIVE_DIR" -maxdepth 1 -type f \( -name '[0-9A-F]???????????????????????' -o -name '[0-9A-F]???????????????????????.partial' \) -exec mv -t "$target" -- {} +
```

7. Run `ls -A "$RECIPE_WALARCHIVE_DIR"`. Check that no segment file remains. The marker file, `.lock`, `base`, and `.pitr-scratch` stay.
8. Run `docker compose up -d --no-deps walarchive`. The stream starts on a directory without segments.
9. Check the stream. The check passes when the result has exactly one row, the state `streaming`, and a `flush_lsn`. Take no base backup before it passes:

```bash
docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT state, flush_lsn FROM pg_stat_replication WHERE usename = 'recipe_replicator' AND application_name = 'walarchive'"
```

10. Take the replacement base backup. The output holds the line `base backup completed`. Note its field `base`:

```bash
docker compose exec walarchive basebackup-once
```

11. Switch the WAL segment. Note the returned position. The server then closes the segment that holds the end of the base backup:

```bash
docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT pg_switch_wal()"
```

12. Run the flush check until it returns `t`. The value `t` shows that the archive holds the end segment of the base backup as a complete file. Run the query again by hand until then:

```bash
docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT EXISTS (SELECT 1 FROM pg_stat_replication WHERE application_name = 'walarchive' AND usename = 'recipe_replicator' AND flush_lsn >= '<position>'::pg_lsn)"
```

13. Check the WAL of the base backup in the archive. The check passes on the exit code 0. Replace `<base>` with the field `base` of step 10:

```bash
docker compose exec walarchive pg_verifybackup --quiet --skip-checksums --wal-directory=/wal /wal/base/<base>
```

14. Wait until the alert `replication` is inactive.

If step 9 finds no row or another state, take no base backup.
Read `docker compose logs walarchive` and fix the cause, for example a missing marker file or secret.
Then run step 9 again. The old segments stay in the target directory, so the wait loses no segment.
If step 13 fails, the archive misses WAL of the base backup. Run the procedure again from step 2 with a new target directory.

This section deletes nothing. Delete the target directory by hand, not with a script.
Delete it only when `/wal/base` holds no name of the form `^base_[0-9]{8}_[0-9]{6}$` that is lower than `<base>` in byte order.
Until then, the moved segments keep the older base backups recoverable up to the start of the gap.
List the names with `docker compose exec walarchive ls /wal/base`.

## A plant device in front of Caddy

Add each device as one `/32` entry (ADR-0065 (m)).
The CIDR goes into two places, and the two lists use different separators.
`Caddyfile` uses a list with spaces. `compose.yaml` uses a list with commas and no space.

1. Open `Caddyfile`. In the block `servers`, delete the comment mark of the line `trusted_proxies static`.
2. Write the CIDR of each device on that line. Separate the CIDRs with spaces:

```text
trusted_proxies static 192.0.2.10/32 192.0.2.11/32
```

3. Open `compose.yaml`. In `RECIPE_HTTP__TRUSTED_PROXIES` of `app`, keep the entry of `proxy`. Add each CIDR after a comma:

```yaml
RECIPE_HTTP__TRUSTED_PROXIES: ${PROXY_ADDRESS:?}/32,192.0.2.10/32,192.0.2.11/32
```

4. Run `docker compose up -d --force-recreate --no-deps proxy app`.
5. Run the check of the section "Check client_ip of the first login".

A new release replaces `Caddyfile` and `compose.yaml`. The section "Upgrade" writes the entries again.

## An older volume

`01-roles.sh` runs on an empty volume only.
For an older volume, the superuser applies the changes of ADR-0069 (v) by hand.
Do this before the upgrade that adds the migration `0006`.
The script `deploy/db/initdb/01-roles.sh` holds the text of the `GRANT`.
The guard step of the section "Upgrade" replaces the guard function after `migrate`.
The `REVOKE` cancels the grant of the previous script (ADR-0069 (f)).

1. Run `docker compose exec -u postgres db psql -d recipe`.
2. Run the `GRANT` of the script and the `REVOKE`:

```sql
GRANT pg_read_all_stats TO audit_owner WITH INHERIT TRUE;
REVOKE pg_read_all_stats FROM recipe_app;
```

3. Run `\drg recipe_app`. Check that the result shows no `pg_read_all_stats`.
4. Do not replace the guard function now. Steps 13 to 17 of the section "Upgrade" replace it after `migrate` (ADR-0097 (h3)).
5. Copy the old file:

```bash
docker compose exec -u postgres db sh -c 'cp -p -- "$PGDATA/pg_hba.conf" "$PGDATA/pg_hba.conf.old"'
```

6. Write the new `pg_hba.conf`. Use the expected lines of the section "Check the client rules of the database":

```bash
docker compose exec -T -u postgres db sh -c 'umask 077 && cat > "$PGDATA/pg_hba.conf.new" && mv -- "$PGDATA/pg_hba.conf.new" "$PGDATA/pg_hba.conf"' < <file with the seven lines>
```

7. Run the query of step 1 of the section "Check the client rules of the database" before the reload.
8. Continue only when the column `error` is empty in every row. Otherwise put the old file back and stop.
9. Reload the configuration:

```bash
docker compose exec -u postgres db psql -d recipe -c "SELECT pg_reload_conf()"
```

10. Run the check of the section "Check the client rules of the database". The catch-all line is gone.

To put the old file back, run the command below. After a reload, run step 9 again:

```bash
docker compose exec -u postgres db sh -c 'cp -p -- "$PGDATA/pg_hba.conf.old" "$PGDATA/pg_hba.conf"'
```

The runbook never deletes `pgdata`.

## Read the logs

The server `app` writes its event log to stdout, as one JSON object per line (`reliability.md` 3.1).
Docker keeps a rolling window of lines for each container. The keys `log.max_size` and `log.max_file` of `reliability.md` 7.11 set its size (`reliability.md` 3.5).
Docker discards the oldest lines first. A full log buffer can also drop lines.
The image of `app` holds no shell, so run each filter on the host with `grep`.
The filters of this section read the lines of `app`.

### Read the lines of a service

- Run `docker compose logs --tail 100 app` to read the last 100 lines.
- Run `docker compose logs --since 1h --no-log-prefix app` to read the last hour without the service prefix. Replace `1h` with another time span.
- Run `docker compose logs --follow app` to read new lines as they come. Press Ctrl+C to stop.
- Replace `app` with `proxy`, `db`, `backup`, or `walarchive` to read another service.

A failed start also writes the text `Error: ` and the error chain to stderr after its JSON `ERROR` line. That text is not JSON (`reliability.md` 3.1).

### Filter the lines of app

Each line has the fields `timestamp`, `level`, `fields`, and `target`. The fields `span` and `spans` occur only inside a span (`crates/recipe-server/src/logging.rs`).
The values of an event are in `fields`. The values `request_id` and `job` of a span are in `span` (`reliability.md` 3.2).
Use `grep -F` with the pattern `"<field>":"<value>"` and no space:

| Goal | Pattern |
|---|---|
| Read the errors | `"level":"ERROR"` |
| Read the warnings | `"level":"WARN"` |
| Read one error code | `"error_code":"E-AUD-002"` |
| Read one request | `"request_id":"<request_id>"` |
| Read one job | `"job":"audit_chain_check"` |
| Read one alert condition | `"alert":"low_disk"` |
| Read one volume of `low_disk` | `"volume":"pgdata"` |

Run the filter on the output of `docker compose logs`. This example reads the errors of the last day:

```bash
docker compose logs --no-log-prefix --since 24h app | grep -F '"level":"ERROR"'
```

To combine two patterns, add a second `grep -F` after the first one.
A user error with the HTTP status 4xx is an `INFO` line, so a search for `ERROR` does not find it (`reliability.md` 3.3).
The names of the alert conditions are in the section "Alarms". The names of the jobs are in `reliability.md` 2.4.

### Find the lines of a user report

The error fragment of the UI shows the error code and the `request_id` (`api.md` 5.2).

1. Read the code and the `request_id` in the error fragment.
2. Run `docker compose logs --no-log-prefix app | grep -F '"request_id":"<request_id>"'`.
3. Read the field `error_code` of the lines.

The line of the backstop panic layer holds no `request_id`. Search for the code `E-WEB-010` in that case (`reliability.md` 3.2).
The `request_id` is also present in the audit log and in the security events (`reliability.md` 3.6).

### Read the lines of a job

The supervisor writes a line for a failed run and for a panic of a job only. A run that ends well writes no line (ADR-0100 (c), `crates/recipe-server/src/supervisor.rs`).
So a missing line shows no fault, and it does not show that a run happened.
A failed run writes an `ERROR` line with the field `error_code`. The job `alerts` writes a `WARN` line instead (ADR-0089 (d)).
A panic writes an `ERROR` line with the value `panic` in the field `outcome` of the span (ADR-0088 (a)).

```bash
docker compose logs --no-log-prefix app | grep -F '"job":"audit_chain_check"'
docker compose logs --no-log-prefix app | grep -F '"outcome":"panic"'
```

To see whether the audit chain check completed, read `audit.last_check_at` as the section "Restore into production" shows.

### Keep the logs longer

The rolling window of Docker keeps about 1 GB for each container. The application writes no log file.
A site that needs a longer window forwards stdout to its own collector (`reliability.md` 3.5). No document gives the setup of that collector.

## Alarms, read-only mode, and restart loop

### Alarms

The job `alerts` writes one `WARN` line for each active condition. The field `alert` holds the name. The condition `low_disk` writes one line for each low or unreadable volume. That line has the field `volume`.
Read the lines with the commands of the section "Read the logs".
`reliability.md` 3.7 gives the conditions and the webhook.

| Name | First action | Then |
|---|---|---|
| `panics` | Read the panic lines in `docker compose logs app`. Note the field `request_id` or `job`. | No document gives a further action. A panic is a bug, and the server keeps serving (`reliability.md` 2.4). |
| `not_ready` | Run `docker compose ps`. Read the health of `db` and `app`. Read `docker compose logs db app`. | Run `docker compose exec app /usr/local/bin/recipe-server healthcheck --path /healthz`. Run it again with `--path /readyz`. If `app` restarts, use the section "Restart loop". If only `/readyz` fails and `db` is healthy, a migration of the binary may be missing from `_sqlx_migrations` or may have another checksum (`reliability.md` 7.2). No document gives a fix. |
| `backup_age` | Run `docker compose logs backup`. Read the last `ERROR` line. Then run `docker compose exec backup backup-now`. | None beyond the first action. |
| `replication` | Run `docker compose ps walarchive` and `docker compose logs walarchive`. For a gap, use the section "A gap in the WAL stream". | None beyond the first action. |
| `low_disk` | Read the field `volume` of the line: `pgdata`, `backups`, or `walarchive`. | Use the section "Low disk space". For `pgdata`, use the section "Read-only mode" too. |
| `failed_logins` | Open `/security-events`. Filter the kind `login_failed`. Read `login` and `client_ip`. | If a user cannot log in, an Administrator opens `/users/<login>` and uses the unlock form (`LoginUnlock`, `api.md` 2.8). If `client_ip` equals `PROXY_ADDRESS`, use the section "Check client_ip of the first login". |
| `security_events` | Open `/security-events`. Filter the kind `csrf_rejected`. Read `client_ip`. Then filter the kind `forbidden`. Read `client_ip`. | If `client_ip` equals `PROXY_ADDRESS`, use the section "Check client_ip of the first login". No document gives another action. |
| `audit_chain_broken` | Read the line `E-AUD-002` in `docker compose logs app`. It names the `chain_seq`. Open the row in `/security-events`. | The condition stays active while the newest row is younger than `audit.max_check_age`. Each nightly check adds a row while the chain stays broken (`reliability.md` 3.7). No document gives a fix. |
| `audit_check_age` | Run `docker compose ps app`. Read the lines with the field `job` of `audit_chain_check` in `docker compose logs app`. | A run that returns an error code is not a completed check. Follow the row of that code in `reliability.md` 1.2 (ADR-0100 (e)). A stored time that is later than the time of `db` by more than `alerts.interval` also keeps the alert active (`reliability.md` 3.7). |
| `export_hash_mismatch` | Read the line `E-EXP-002` in `docker compose logs app`. It names the recipe and the version. | The server refuses the export of that version (`reliability.md` 1.2). No document gives a fix. |

### Low disk space

The condition `low_disk` is active when a volume has less free space than `disk.warn_percent` (`reliability.md` 3.7).
The webhook body names no volume, so read the field `volume` of the `WARN` lines (ADR-0075 (d), (e)).
The three volumes lie on these disks:

| Volume | Disk |
|---|---|
| `pgdata` | The disk of the Docker root directory |
| `backups` | The second disk, in the directory `RECIPE_BACKUP_DIR` |
| `walarchive` | The second disk, in the directory `RECIPE_WALARCHIVE_DIR` |

1. Read the volume of the condition:

```bash
docker compose logs --no-log-prefix --since 1h app | grep -F '"alert":"low_disk"'
```

2. List the use of the three disks:

```bash
df -h "$(docker info --format '{{.DockerRootDir}}')" "$RECIPE_BACKUP_DIR" "$RECIPE_WALARCHIVE_DIR"
```

3. Read the column `Use%` of the disk of that volume. The free space is about 100 % minus `Use%`.

For `pgdata`, use the section "Read-only mode".

For `backups` or `walarchive`, do these steps:

1. If the condition `backup_age` is active too, use its row first. The retention runs only after a complete new set (`reliability.md` 7.7).
2. Run the section "Delete orphan temp files". It deletes only the temp names of killed runs.
3. If the volume stays low, extend the filesystem of the second disk with the site procedure. No document gives that procedure.

Delete no set, no base backup, and no WAL segment by hand.
The scripts delete them by `backup.retention` and `basebackup.retention` (`reliability.md` 7.7, ADR-0099).
These two values are constants of the scripts `backup-once` and `basebackup-once`. A change needs a new release.
A full disk of `walarchive` stops `pg_receivewal` (ADR-0077).
Read the section "Destructive commands" before each delete.

### Read-only mode

The server turns read-only mode on and off by itself. The operator has no switch, no setting, and no command for it.
The job `disk_check` measures the free space of `pgdata` every `health.disk_check_interval` (`reliability.md` 7.8).

- The mode turns on when the free space is below `disk.readonly_percent`.
- The mode also turns on when the measurement fails. The free space can be fine in this case.
- The mode turns off, without a restart, when a measurement shows more than `disk.resume_percent` free.
- Between the two thresholds, the mode does not change.

The values of `disk.*` are code defaults and not configuration keys (`reliability.md` 7.11). A change needs a new release.

Each page shows the read-only banner (ADR-0071). The server writes no log line when the mode changes, so the banner shows the mode.
Every query, `Login`, `Logout`, and `ExportVersion` still work (A24 in `overview.md` 9).
Every other command returns `503` with the code `E-WEB-006`.

To end the mode, do these steps:

1. Run `docker compose logs --no-log-prefix --since 1h app | grep -F '"volume":"pgdata"'`.
2. If a line says `The free disk space cannot be read.`, use the case "The mode stays on" below.
3. Read the mount point: `docker volume inspect --format '{{ .Mountpoint }}' recipe-editor_pgdata`.
4. Run `df -h <mount point>`.
5. Increase the free space of that disk. Delete only files outside the volumes `pgdata`, `backups`, and `walarchive`.
6. Delete no set, no base backup, and no WAL segment by hand. Delete nothing inside `pgdata`.
7. Wait for `health.disk_check_interval`. Load a page again. The banner goes away when the free space is above `disk.resume_percent`.

No document names a file that is safe to delete outside the three volumes.
If the disk has no such file, extend its filesystem with the site procedure.

#### The mode stays on

If the free space looks fine and the banner stays, find the cause in this list:

- The free space is between `disk.readonly_percent` and `disk.resume_percent`. Increase the free space until it is above `disk.resume_percent`.
- The measurement fails each run. `app` reads `pgdata` through a read-only mount. Run `docker compose config | grep 'pgdata:/var/lib/postgresql/data:ro'`. The output must show the line.
- If the mount line is present, run `docker compose ps` and read the health of `app` and `db`. No document gives a further fix.

The mode ends after the first measurement that succeeds and shows more than `disk.resume_percent` free.

### Restart loop

A restart loop of `app` ends the job `alerts`, so no alert comes (`reliability.md` 3.7).
Use the host monitoring for this case.

1. Run `docker compose ps`. Look for a service that restarts or shows `unhealthy`. For a service other than `app`, use the section "Service unhealthy".
2. Run `docker compose logs app`. Read the `ERROR` line of the failed start.
3. If the line names a variable `PG*`, list the names in the Compose output:

```bash
docker compose config | grep -o '^ *PG[A-Za-z_]*:'
```

4. Delete that variable from `compose.yaml`, or from `.env` when `compose.yaml` reads its value there.
5. Run `docker compose up -d`.

A variable of the host shell does not reach a container, so `unset` has no effect on the server.

## Service unhealthy

Docker marks a service `unhealthy` after `health.check_failures` failed checks in a row (`reliability.md` 7.1, ADR-0070 (n)).
Docker runs a check every `health.check_interval`. A service shows `starting` until its first check passes.
This section covers `proxy`, `app`, `db`, `backup`, and `walarchive`. The one-shot services have no healthcheck.

### Read why a service is unhealthy

1. Run `docker compose ps -a`. Look for `unhealthy`, `Restarting`, or `Exited`.
2. Read the last results of the healthcheck of one service:

```bash
docker inspect --format '{{range .State.Health.Log}}{{.ExitCode}} {{.Output}}{{end}}' "$(docker compose ps -q <service>)"
```

3. Run the check by hand with the command of the next table. Read the exit code with `echo $?`. Exit code 0 is a pass.
4. Run `docker compose logs --tail 50 <service>`. Read the `ERROR` lines (section "Read the logs").
5. Find the row of the service in the next table.

| Service | The check passes when | Run the check by hand | Next |
|---|---|---|---|
| `proxy` | `wget` inside `proxy` gets `http://app:9000/healthz`. So `app` answers on its internal listener. | `docker compose exec proxy wget -q -O /dev/null http://app:9000/healthz` | The check does not test TLS or the certificate. If `app` is not `healthy`, fix `app` first. If `app` is `healthy`, no document gives a cause or a fix. |
| `app` | `recipe-server healthcheck --path /readyz` exits with code 0. The health pool runs `SELECT 1`, and every migration of the binary is in `_sqlx_migrations` (`reliability.md` 7.2). | `docker compose exec app /usr/local/bin/recipe-server healthcheck --path /readyz` | Use the row `not_ready` of the section "Alarms" and the section "Restart loop". If the `ERROR` line names `alerts.webhook_url_file`, use the section "Change the webhook". |
| `db` | `pg_isready -h 127.0.0.1 -p 5432 -U pg_isready -d recipe` exits with code 0. `db` is healthy only after its init (ADR-0070 (q)). | `docker compose exec db pg_isready -h 127.0.0.1 -p 5432 -U pg_isready -d recipe` | Read `docker compose logs db`. `app`, `backup`, and `walarchive` depend on `db` (`reliability.md` 7.1). After a change of `pg_hba.conf`, see the end of the section "Subnet change". For any other cause, no document gives a fix. |
| `backup` | `backup-health` exits with code 0. The backup directory is not on the filesystem of `pgdata`. The marker file exists. A complete set is younger than `backup.max_age_alert`. | `docker compose exec backup backup-health`. The check writes nothing and deletes nothing. | Use the table "Messages of backup-health". |
| `walarchive` | `walarchive-health` exits with code 0. A process `pg_receivewal` runs. The check does not test the lag. The job `alerts` tests it against `wal.max_lag_bytes`. | `docker compose exec walarchive walarchive-health`. The check writes nothing and deletes nothing. | Use the table "Messages of walarchive". |
| `backup-premigrate` and `migrate` | They have no healthcheck. Exit code 0 is a pass (`reliability.md` 7.1). | `docker compose ps -a`, then `docker compose logs backup-premigrate migrate` | The service `app` starts only after `migrate` ends with exit code 0. See step 12 of "Upgrade" and step 3 of "Start the stack". For `backup-premigrate`, use the table "Messages of backup-once". |

After you fix the cause, restart the service with the section "Restart one service".

### Messages of backup-health

The script writes one `ERROR` line with a field `message`.

| Message | Cause | Action |
|---|---|---|
| `the backup directory is missing` | The container has no directory `/backups`. | Run steps 6 and 7 of the section "Create the environment file". Check the value of `RECIPE_BACKUP_DIR`. |
| `the directory of pgdata is missing` or `the file PG_VERSION is missing, so pgdata is not mounted` | `backup` has no read-only mount of the volume `pgdata`. | Compare your `compose.yaml` with the file of the release. No document gives a further action. |
| `the backup directory is on the filesystem of pgdata` | `RECIPE_BACKUP_DIR` is on the disk of the Docker root directory. | Run steps 4 and 5 of the section "Prepare the second disk". |
| `the marker file of the backup disk is missing` | The file `.backup-marker` is not in `RECIPE_BACKUP_DIR`. | Run `install -m 0600 -o 999 -g 999 /dev/null "$RECIPE_BACKUP_DIR/.backup-marker"`. |
| `no complete set is younger than the maximum age` | No run made a set within `backup.max_age_alert`. A run failed, did not end, or did not start. | Use the row `backup_age` of the section "Alarms". Then use the table "Messages of backup-once". |

The script `backup-entry` writes the message about the marker file too. Then the container `backup` stops and Docker starts it again.

### Messages of backup-once

The nightly run, `backup-now`, and `backup-premigrate` run the same script. `backup-now` prints the line on the terminal.
Read the other runs with `docker compose logs backup` and `docker compose logs backup-premigrate`.

| Message | Cause | Action |
|---|---|---|
| `another backup run holds the lock` | Another run writes in `RECIPE_BACKUP_DIR`. It is the nightly run, `backup-now`, or `backup-premigrate`. A run that does not end holds the lock too. | Run `docker compose top backup`. If a process `backup-once` runs, wait for its end. Then run the command again. |
| `a file with a name of the new set exists` | The name of a set holds the UTC minute. A second run in the same minute finds the names. A temp file of a killed run can hold a name. | Wait one minute. Run the command again. |
| `the marker file of the backup disk is missing` | The file `.backup-marker` is not in `RECIPE_BACKUP_DIR`. | Use the same action as in the table "Messages of backup-health". |
| `cannot read the password file of recipe_backup` or a message that starts with `the password file of recipe_backup` | The file `backup_password` is unreadable, is empty, or has a wrong form. | Check the owner and the mode as in the section "Create the secret files". The file holds one line and no carriage return. |
| `cannot publish a file of the set without replacing a file` | The filesystem of the second disk may lack hard links (ADR-0074 (e)). | Check the filesystem as in step 1 of the section "Prepare the second disk". |
| `pg_dump failed`, `pg_dumpall failed`, or `the snapshot session ended before it answered` | The tool or the database stopped the run. | Read the lines above the message. Run `docker compose ps db`. No document gives a further action. |

A run of `backup-once` has no time limit (ADR-0074 (d)). If `db` does not answer, the run holds the lock.
Then each later run and `backup-premigrate` stops with `another backup run holds the lock`. The held run deletes nothing.
After `backup.max_age_alert`, `backup-health` fails and the alert `backup_age` is active.
Find the run with `docker compose top backup`. No document gives the time after which a run counts as hung.
No document gives the command that ends a hung run.

### Messages of walarchive

The container ends when `pg_receivewal` or `supercronic` exits (ADR-0068 (c)). Docker then starts it again.
The `ERROR` lines are in `docker compose logs walarchive`.

| Message | Cause | Action |
|---|---|---|
| `no process pg_receivewal runs` | `pg_receivewal` stopped, or the container just restarted. | Read `docker compose logs walarchive`. Use the next rows. |
| `a child process exited` | The field `child` names `pg_receivewal` or `supercronic`. The field `exit_code` gives its code. | Read the lines above it, for the text of `pg_receivewal`. If `df` shows a full disk, use the section "Low disk space". If the alert `replication` stays active, use the section "A gap in the WAL stream". |
| `the marker file of the WAL archive disk is missing` | The file `.walarchive-marker` is not in `RECIPE_WALARCHIVE_DIR`. | Run `install -m 0600 -o 999 -g 999 /dev/null "$RECIPE_WALARCHIVE_DIR/.walarchive-marker"`. |
| `cannot read the password file of recipe_replicator` or a message that starts with `the password file of recipe_replicator` | The file `replicator_password` is unreadable, is empty, or has a wrong form. | Check the owner and the mode as in the section "Create the secret files". The file holds one line and no carriage return. |

## Destructive commands

- Never run `docker compose down -v`.
- Never run `docker volume rm` on `pgdata`.
- Each delete step of this page names what it deletes.
- Each delete step of a set, a file, or a directory of the backup disks follows a passed restore test.
- The section "Subnet change" runs `docker compose down` without `-v`. It deletes no volume.

## Check client_ip of the first login

A wrong `PROXY_ADDRESS` gives each client the address of `proxy` (ADR-0065).

1. Open the login page from a workstation. Log in with a wrong password.
2. Open `/security-events` as an administrator.
3. Filter the kind `login_failed` and the login that you used.
4. Compare `client_ip` with the address of the workstation.
5. If `client_ip` equals `PROXY_ADDRESS`, compare `PROXY_ADDRESS` in `.env` with the output of `docker compose config`.
6. For a plant device in front of Caddy, use the section "A plant device in front of Caddy".

The file `compose.yaml` holds `${PROXY_ADDRESS:?}` and not an address.
Read the address in the output of `docker compose config` at the service `proxy` (`ipv4_address`) and at the entry `RECIPE_HTTP__TRUSTED_PROXIES` of the service `app`.

## Related

- [reliability.md](../architecture/reliability.md): sections 3.1, 3.2, 3.5, 3.7, 7.1, 7.2, 7.3, 7.7, 7.8, 7.10, and 7.11
- [api.md](../architecture/api.md): section 2.9
- [ADR-0020](../adr/0020-deployment-migrations-backup.md), [ADR-0031](../adr/0031-migrator-configuration-and-secret-files.md), [ADR-0033](../adr/0033-globals-dump-of-the-backup.md), [ADR-0035](../adr/0035-identity-sessions-clarifications.md), [ADR-0062](../adr/0062-configuration-keys-and-secret-files.md), [ADR-0063](../adr/0063-webhook-of-the-alerts.md)
- [ADR-0064](../adr/0064-images-and-their-distribution.md), [ADR-0065](../adr/0065-proxy-tls-and-default-trusted-proxy.md), [ADR-0068](../adr/0068-wal-archive-and-base-backups.md), [ADR-0069](../adr/0069-hardening-of-the-database-access.md)
- [ADR-0070](../adr/0070-container-limits-networks-healthchecks-and-stack-test.md), [ADR-0074](../adr/0074-names-on-the-backup-disks.md), [ADR-0075](../adr/0075-names-of-the-alert-conditions.md), [ADR-0077](../adr/0077-recovery-check-wal-cutoff-and-scratch-constants.md)
- [ADR-0082](../adr/0082-upgrade-and-rollback-without-dependencies.md), [ADR-0083](../adr/0083-pgtools-image-in-the-upgrade-and-the-rollback.md)
- [ADR-0059](../adr/0059-health-probe-readiness-and-health-router.md), [ADR-0061](../adr/0061-start-order-of-serve.md), [ADR-0066](../adr/0066-backup-disks-file-sets-and-schedule.md), [ADR-0087](../adr/0087-drain-of-the-internal-listener.md)
- [ADR-0071](../adr/0071-read-only-banner.md), [ADR-0073](../adr/0073-read-faults-and-the-audit-chain-alert.md), [ADR-0088](../adr/0088-run-interface-of-the-jobs-and-cut-count.md), [ADR-0089](../adr/0089-job-errors-first-disk-run-and-stop-line.md), [ADR-0100](../adr/0100-wait-for-the-audit-chain-job-after-a-restore.md)

## Known gaps

The documents do not yet close the gaps below. Each bullet gives the workaround of this page, if one exists.

- The networks `edge` and `egress` have no fixed subnet. Docker can give them new subnets after `docker compose down` and `docker compose up`. Read the subnets again and write the firewall rules again (section "Check the host firewall").
- No document gives the mechanism that makes a new volume the volume of `db`. The restore, the rollback, and the major upgrade therefore stop before the volume switch. No workaround.
- No document gives the steps to recover production from the WAL archive. The recovery point of minutes (ADR-0020, `reliability.md` 7.7) has no procedure yet. The workaround is a restore of a dump, with a loss of up to 24 hours.
- No document gives the transfer of the external images `caddy` and `postgres`, or the digest check after `docker load`. The section "Load the images" names only `recipe-server` and `recipe-pgtools`. No workaround.
- The trusted-proxy entries in `Caddyfile` and `compose.yaml` do not survive a release. A new release replaces both files. After each upgrade, write the entries again (sections "Upgrade" and "A plant device in front of Caddy").
- The rule that a restore goes into a new volume is only in a non-normative design note (D13). No ADR holds it. No workaround.
- No command runs the audit chain check on demand. The job `audit_chain_check` runs by schedule at `audit.check_time`, and once after `audit.catchup_delay` at a start of `app` when `audit.last_check_at` is older than `audit.catchup_age`. After a restore, wait for the next run.
- The soak test of `reliability.md` section 6 is not implemented. The repository holds no soak script and no nightly CI job for it. The 2 h local run with an RSS chart did not run, so no file `docs/ops/soak-<date>.md` exists. The limits of that test stay unproven. They are `soak.memory_ceiling` of `reliability.md` 7.11, zero restarts, and the p99 latency bound of section 6. No workaround.
- The section "Check the host firewall" gives no rule that limits the network `egress` to the webhook host (ADR-0063 (p)). No workaround.
- The file `compose.yaml` sets no `RECIPE_ALERTS__WEBHOOK_CA_FILE` and mounts no CA file. The key `alerts.webhook_ca_file` therefore stays empty, and the webhook client trusts the webpki roots only (ADR-0063 (j)). No workaround.
- No test measures the recovery time objective of `reliability.md` 7.7 (ADR-0020).
- The procedure "Restore into production" assumes an empty cluster with no project role and no database `recipe`. On an empty volume, the service `db` runs `db/initdb/01-roles.sh` and creates them (`deploy/compose.yaml`, `deploy/db/initdb/01-roles.sh`). The procedure is not complete for 0.1.0. No workaround.
- The restore into a new volume and the major upgrade do not handle the WAL archive (`walarchive`). After a restore into a new cluster, the old base backups can name higher segments than the new cluster (ADR-0077). The section "A gap in the WAL stream" covers an outage only. No workaround.
- No procedure recovers access when the only administrator cannot log in. The command `create-admin` does nothing when an active administrator exists. Only a user with `UserManage` resets a password (`api.md` section 3). No workaround.
- A throttled login is a separate case. It ends by time, because there is no permanent lockout (`domain-model.md` 2.3, `login.delay_formula`, `login.ip_budget`). Wait, then log in again.

A later change will close these gaps with its own ADR.
