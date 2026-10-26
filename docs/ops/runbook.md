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
3. Set `RECIPE_BACKUP_DIR` and `RECIPE_WALARCHIVE_DIR` to directories on the second disk.
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
3. Check that both directories lie outside the filesystem of the volume `pgdata`.
4. Create both directories and both marker files:

```bash
install -d -m 0755 -o 999 -g 999 "$RECIPE_BACKUP_DIR"
install -d -m 0700 -o 999 -g 999 "$RECIPE_WALARCHIVE_DIR"
install -m 0600 -o 999 -g 999 /dev/null "$RECIPE_BACKUP_DIR/.backup-marker"
install -m 0600 -o 999 -g 999 /dev/null "$RECIPE_WALARCHIVE_DIR/.walarchive-marker"
```

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

3. Create the file `alerts_webhook_url`. Write the URL of the webhook into it. ADR-0063 (e) gives the rules of the URL.
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

The command does nothing when an active administrator exists.
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

The restore goes into a new `db` volume (ADR-0020).
The old volume `pgdata` stays on the host.
The step that makes a new volume the volume of `db` is open. No document gives its command.
The steps below start with an empty new cluster as `db`. It holds no database `recipe` and no project role.
`<stamp>` is the part of the dump file name between `recipe_` and `.dump`, for example `<YYYYMMDD_HHMM>_now`.

Warning: a restore of a dump loses every change and audit entry after the dump time. The loss is up to 24 hours.
Keep `pgdata` and the WAL archive disk unchanged.
A recovery from the WAL archive gives a recovery point of minutes (`reliability.md` 7.7, ADR-0068 (l), R31).
This page gives no steps for that recovery.

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
19. Check `/readyz` with `docker compose exec app /usr/local/bin/recipe-server healthcheck --path /readyz`.
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
4. If the test names a `_premigrate` set of the same UTC minute, rerun commands 2 and 3 later.

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

If the migration did not follow the expand pattern, use the `_premigrate` dump.
Warning: this restore loses every change and audit entry after the `_premigrate` dump.
The lost period runs from the `_premigrate` dump to the stop of `app`.
No test checks the `_premigrate` set. The test before the upgrade checks a set of the same schema (ADR-0083).
The restore goes into a new `db` volume. The switch step is open, as the section "Restore into production" says.

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

A major upgrade of PostgreSQL is a dump and a restore into a new `db` volume.
The new volume uses the same `initdb` arguments as `compose.yaml`. The project does not use `pg_upgrade` (ADR-0020).

1. Run `docker compose stop app`. The next dump then holds every change.
2. Run `docker compose exec backup backup-now`. The new `_now` set holds the globals file and the dump (`reliability.md` 7.7).
3. Run `docker compose exec backup restore-test`. Continue only after it passes.
4. Check that the line `restore test passed` names the `_now` set of step 2.

The test comes before the switch (`reliability.md` 7.10 step 8).
The switch makes the new volume the volume of `db`. No document gives the command of this switch (ADR-0020), so the switch step is open.
After the switch, restore the `_now` set with steps 4 to 10 of the section "Restore into production". Use the name of the `_now` set for `<stamp>`.
Keep the old volume `pgdata` on the host.

## Key rotation

The files `login_hmac_key` and `csrf_signing_key` hold one key per line, the newest key first (ADR-0035).

1. Run the command below once for each key file. Use `login_hmac_key` and `csrf_signing_key` for `<name>`.

```bash
( umask 077 && { openssl rand -hex 32; cat "$RECIPE_SECRETS_DIR/<name>"; } > "$RECIPE_SECRETS_DIR/<name>.new" )
```

The command writes the new key as the first line and keeps the old line. It prints no key.

2. Set the owner and the mode of each new file. Then move it over the old file:

```bash
chown 65532 "$RECIPE_SECRETS_DIR/<name>.new"
chmod 0400 "$RECIPE_SECRETS_DIR/<name>.new"
mv "$RECIPE_SECRETS_DIR/<name>.new" "$RECIPE_SECRETS_DIR/<name>"
```

3. Run `docker compose up -d --force-recreate --no-deps app`.
4. Wait for `security.key_rotation_overlap` of `reliability.md` 7.11.
5. Delete the old line from the key file. This step deletes one key and no data.
6. Run `docker compose up -d --force-recreate --no-deps app` again.

The server accepts both keys during the overlap.

## Certificate rotation

1. Copy the new certificate and the new key to `tls_cert` and `tls_key`.
2. Check the owner `0` and the mode `0400` of both files.
3. Run `docker compose up -d --force-recreate --no-deps proxy`.

A rotation recreates the container `proxy`, so the service stops for a short time (ADR-0065 (c)).

## Subnet change

A change of the subnet edits `COMPOSE_SUBNET`, `COMPOSE_IP_RANGE`, and `PROXY_ADDRESS` together.
`PROXY_ADDRESS` lies inside `COMPOSE_SUBNET` and outside `COMPOSE_IP_RANGE`.
The host lines of `pg_hba.conf` name the subnet, so they change too.

1. Run the restore test. Continue only after it passes.
2. Set the three new values in `.env`.
3. Run steps 6 and 7 of the section "Create the environment file". The new values show.
4. Write the lines of `pg_hba.conf` with the new subnet. Use steps 5 to 8 of the section "An older volume".
5. Run `docker compose down`. The command deletes the containers and the networks. It deletes no volume.
6. Run `docker compose up -d`.
7. Run the check of the section "Check the client rules of the database".
8. Run the section "Check the host firewall" again.

If `db` does not start, put the old file back in the volume of `db`. Then change the lines and start again:

```bash
docker compose run --rm --no-deps -u postgres --entrypoint sh db -c 'cp -p -- "$PGDATA/pg_hba.conf.old" "$PGDATA/pg_hba.conf"'
```

## A gap in the WAL stream

An outage of `walarchive` longer than `pg.wal_keep_size` leaves a gap (ADR-0068 (e)).
The alert `replication` is active during the outage.

The step moves the WAL segment files and the `.partial` file from the top of `RECIPE_WALARCHIVE_DIR` (ADR-0068 (l)).
The marker file, `.lock`, the directory `base`, and the directory `.pitr-scratch` stay.
The documents do not name the target directory. Use a directory on the WAL archive disk. Below, `<moved directory>` is its path.

1. Run `docker compose exec backup restore-test`. Continue only after it passes.
2. Run `docker compose stop walarchive`.
3. Create the target directory:

```bash
install -d -m 0700 -o 999 -g 999 <moved directory>
```

4. Check that both paths show the same number:

```bash
stat -c %d "$RECIPE_WALARCHIVE_DIR" <moved directory>
```

5. List the files that step 6 moves:

```bash
find "$RECIPE_WALARCHIVE_DIR" -maxdepth 1 -type f \( -name '[0-9A-F]???????????????????????' -o -name '[0-9A-F]???????????????????????.partial' \)
```

6. Move the files of the list:

```bash
find "$RECIPE_WALARCHIVE_DIR" -maxdepth 1 -type f \( -name '[0-9A-F]???????????????????????' -o -name '[0-9A-F]???????????????????????.partial' \) -exec mv -t <moved directory> -- {} +
```

7. Run `ls -A "$RECIPE_WALARCHIVE_DIR"`. Check that no segment file remains and that the marker file and `base` stay.
8. Take a new base backup:

```bash
docker compose run --rm --no-deps walarchive basebackup-once
```

9. Check that the output holds the line `base backup completed`.
10. Run `docker compose up -d --no-deps walarchive`.
11. Wait until the alert `replication` is inactive.

Keep the moved files until the next restore test and the next `basebackup-once` pass. This section deletes nothing.

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

## Alarms, read-only mode, and restart loop

### Alarms

The job `alerts` writes one `WARN` line for each active condition. The field `alert` holds the name. The condition `low_disk` writes one line for each low or unreadable volume. That line has the field `volume`.
Read the lines with `docker compose logs app`.
`reliability.md` 3.7 gives the conditions and the webhook.

| Name | First action |
|---|---|
| `panics` | Read the panic lines in `docker compose logs app`. Note the field `request_id` or `job`. |
| `not_ready` | Run `docker compose ps`. Read the health of `db` and `app`. Read `docker compose logs db app`. |
| `backup_age` | Run `docker compose logs backup`. Read the last `ERROR` line. Then run `docker compose exec backup backup-now`. |
| `replication` | Run `docker compose ps walarchive` and `docker compose logs walarchive`. For a gap, use the section "A gap in the WAL stream". |
| `low_disk` | Read the field `volume` of the line: `pgdata`, `backups`, or `walarchive`. Free space on that disk. Delete only files outside the three volumes. Delete no set, no base backup, and no WAL segment by hand. For `pgdata`, use the section "Read-only mode". |
| `failed_logins` | Open `/security-events`. Filter the kind `login_failed`. Read `login` and `client_ip`. |
| `security_events` | Open `/security-events`. Filter the kinds `csrf_rejected` and `forbidden`. Read `client_ip`. |
| `audit_chain_broken` | Read the line `E-AUD-002` in `docker compose logs app`. It names the `chain_seq`. Open the row in `/security-events`. |
| `audit_check_age` | Run `docker compose ps app`. Read the lines with the field `job` of `audit_chain_check` in `docker compose logs app`. |
| `export_hash_mismatch` | Read the line `E-EXP-002` in `docker compose logs app`. It names the recipe and the version. |

### Read-only mode

The application enters read-only mode when the free space of `pgdata` is low or unknown (`reliability.md` 7.8).
The UI shows the read-only banner, and a change returns `E-WEB-006`.

1. Read the mount point: `docker volume inspect --format '{{ .Mountpoint }}' recipe-editor_pgdata`.
2. Run `df -h <mount point>`.
3. Free space on the disk of the volume. Delete only files outside the volumes `pgdata`, `backups`, and `walarchive`.
4. Delete no set, no base backup, and no WAL segment by hand. Delete nothing inside `pgdata`.
5. Wait for the next measurement. The mode ends without a restart (`reliability.md` 7.8).

### Restart loop

A restart loop of `app` ends the job `alerts`, so no alert comes (`reliability.md` 3.7).
Use the host monitoring for this case.

1. Run `docker compose ps`. Look for a service that restarts or shows `unhealthy`.
2. Run `docker compose logs app`. Read the `ERROR` line of the failed start.
3. If the line names a variable `PG*`, list the names in the Compose output:

```bash
docker compose config | grep -o '^ *PG[A-Za-z_]*:'
```

4. Delete that variable from `compose.yaml`, or from `.env` when `compose.yaml` reads its value there.
5. Run `docker compose up -d`.

A variable of the host shell does not reach a container, so `unset` has no effect on the server.

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
5. If `client_ip` equals `PROXY_ADDRESS`, check the value of `PROXY_ADDRESS` in `.env` and in `compose.yaml`.
6. For a plant device in front of Caddy, use the section "A plant device in front of Caddy".

## Related

- [reliability.md](../architecture/reliability.md): sections 3.7, 7.1, 7.7, 7.8, 7.10, and 7.11
- [ADR-0020](../adr/0020-deployment-migrations-backup.md), [ADR-0033](../adr/0033-globals-dump-of-the-backup.md), [ADR-0062](../adr/0062-configuration-keys-and-secret-files.md), [ADR-0063](../adr/0063-webhook-of-the-alerts.md)
- [ADR-0064](../adr/0064-images-and-their-distribution.md), [ADR-0065](../adr/0065-proxy-tls-and-default-trusted-proxy.md), [ADR-0068](../adr/0068-wal-archive-and-base-backups.md), [ADR-0069](../adr/0069-hardening-of-the-database-access.md)
- [ADR-0070](../adr/0070-container-limits-networks-healthchecks-and-stack-test.md), [ADR-0074](../adr/0074-names-on-the-backup-disks.md), [ADR-0075](../adr/0075-names-of-the-alert-conditions.md), [ADR-0077](../adr/0077-recovery-check-wal-cutoff-and-scratch-constants.md)
- [ADR-0082](../adr/0082-upgrade-and-rollback-without-dependencies.md), [ADR-0083](../adr/0083-pgtools-image-in-the-upgrade-and-the-rollback.md)
