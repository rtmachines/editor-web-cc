# Runbook of the recipe editor

This guide gives the operator the steps for the Compose stack of the recipe editor.
Run each `docker compose` command in the directory `deploy/` of the release.
Replace each value in angle brackets.
The rules behind the steps are in [reliability.md](../architecture/reliability.md) and in the ADRs of the section "Related".

**Contents**

- [Host preparation](#host-preparation)
  - [Check the host](#check-the-host)
  - [Create the environment file](#create-the-environment-file)
  - [Prepare the second disk](#prepare-the-second-disk)
  - [Create the secret files](#create-the-secret-files)
  - [Check the host firewall](#check-the-host-firewall)
- [First start, first administrator, and image transfer](#first-start-first-administrator-and-image-transfer)
  - [Load the images](#load-the-images)
  - [Start the stack](#start-the-stack)
  - [Check the client rules of the database](#check-the-client-rules-of-the-database)
  - [Create the first administrator](#create-the-first-administrator)
- [Check client_ip of the first login](#check-client_ip-of-the-first-login)
- [Destructive commands](#destructive-commands)
- [Stop, start, and restart](#stop-start-and-restart)
  - [Stop the whole stack](#stop-the-whole-stack)
  - [Start the whole stack again](#start-the-whole-stack-again)
  - [Restart one service](#restart-one-service)
  - [Stop and start, or down and up](#stop-and-start-or-down-and-up)
  - [After a host reboot](#after-a-host-reboot)
- [Backup now, restore test, and restore into production](#backup-now-restore-test-and-restore-into-production)
  - [Run a backup now](#run-a-backup-now)
  - [Run the restore test](#run-the-restore-test)
  - [Statement outside the role grammar](#statement-outside-the-role-grammar)
  - [Copy the backups off site](#copy-the-backups-off-site)
  - [Delete orphan temp files](#delete-orphan-temp-files)
  - [Change log of the site](#change-log-of-the-site)
  - [Restore into production](#restore-into-production)
- [Check the stored data against the platform ranges](#check-the-stored-data-against-the-platform-ranges)
- [Check the stored type rows against the identifier rules](#check-the-stored-type-rows-against-the-identifier-rules)
- [Check the stored data against the S7 layout rules](#check-the-stored-data-against-the-s7-layout-rules)
- [Check the live S7 machines for a PLC connection record](#check-the-live-s7-machines-for-a-plc-connection-record)
- [Check the TwinCAT version before you add a TwinCat3 machine](#check-the-twincat-version-before-you-add-a-twincat3-machine)
- [Upgrade and rollback](#upgrade-and-rollback)
  - [Upgrade](#upgrade)
  - [Rollback](#rollback)
  - [Switch back](#switch-back)
  - [Keep the images for a rollback](#keep-the-images-for-a-rollback)
  - [Concurrent index on an audit object](#concurrent-index-on-an-audit-object)
- [Major upgrade](#major-upgrade)
- [Key rotation](#key-rotation)
- [Certificate rotation](#certificate-rotation)
- [Change a database password](#change-a-database-password)
  - [Check the log of db for a password](#check-the-log-of-db-for-a-password)
- [Change a setting](#change-a-setting)
- [Alert webhook](#alert-webhook)
  - [Rules of the URL](#rules-of-the-url)
  - [Change the webhook](#change-the-webhook)
  - [Disable the webhook](#disable-the-webhook)
  - [Find a failed call](#find-a-failed-call)
- [Subnet change](#subnet-change)
- [A gap in the WAL stream](#a-gap-in-the-wal-stream)
- [A plant device in front of Caddy](#a-plant-device-in-front-of-caddy)
- [An older volume](#an-older-volume)
- [Read the logs](#read-the-logs)
  - [Read the lines of a service](#read-the-lines-of-a-service)
  - [Filter the lines of app](#filter-the-lines-of-app)
  - [Find the lines of a user report](#find-the-lines-of-a-user-report)
  - [Read the lines of a job](#read-the-lines-of-a-job)
  - [Keep the logs longer](#keep-the-logs-longer)
- [Alarms, read-only mode, and restart loop](#alarms-read-only-mode-and-restart-loop)
  - [Alarms](#alarms)
  - [Low disk space](#low-disk-space)
  - [Read-only mode](#read-only-mode)
  - [Restart loop](#restart-loop)
- [Service unhealthy](#service-unhealthy)
  - [Read why a service is unhealthy](#read-why-a-service-is-unhealthy)
  - [Messages of backup-health](#messages-of-backup-health)
  - [Messages of backup-once](#messages-of-backup-once)
  - [Messages of walarchive](#messages-of-walarchive)
- [Known gaps](#known-gaps)
- [Related](#related)

## Host preparation

### Check the host

1. Check that the host has at least 8 GiB of memory and 4 vCPU (ADR-0070 (m)).
2. Run `docker version` and `docker compose version`. Engine 25 and Compose 2.24 are the lowest versions.
3. Check that the host clock runs with NTP (`reliability.md` 7.9).
4. Run `printenv | grep -o '^PG[^=]*'`. The command prints nothing when the host is clean.

The command prints names and no values. The server refuses to start with a variable `PG*` (ADR-0069 (b)).

### Create the environment file

1. Copy `.env.example` to `.env`.
2. Set the nine values of `.env`. Each directory is an absolute path.
3. Set `RECIPE_BACKUP_DIR` and `RECIPE_WALARCHIVE_DIR` to directories on the second disk. Do not use the mount point for `RECIPE_WALARCHIVE_DIR`.
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

Without the mount point, the parent directory of `RECIPE_WALARCHIVE_DIR` is on the second disk too.
The gap procedure moves files there as a rename (ADR-0099 (c)).

The host commands use the shell values only. Compose reads `.env` itself.
A shell variable with the same name wins over `.env`, so repeat steps 6 and 7 after each edit of `.env`.
Compose passes only the entries of `environment:` to a container. A `PG*` variable of the host shell does not reach it.

The ninth value, `RECIPE_PGDATA_VOLUME`, is a volume name and not a directory. The next section sets it.

### Create the first volume

Use this section for a new installation. An existing installation keeps the name of its running volume (ADR-0103 (d)).
Compose never creates the volume `pgdata`. The value `RECIPE_PGDATA_VOLUME` of `.env` names it (ADR-0103 (c), (d)).

1. Print a new name. The name has the form `recipe-editor_pgdata_<YYYYMMDD_HHMMSS>` in UTC (ADR-0103 (e)):

```bash
date -u +recipe-editor_pgdata_%Y%m%d_%H%M%S
```

2. Write the name as the value of `RECIPE_PGDATA_VOLUME` in `.env`.
3. Load the values into the shell again. Do not export them:

```bash
. ./.env
```

4. Create the volume:

```bash
docker volume create "$RECIPE_PGDATA_VOLUME"
```

5. Check that the command prints the name of step 2:

```bash
docker volume inspect --format '{{ .Name }}' "$RECIPE_PGDATA_VOLUME"
```

Repeat step 3 in each new shell. The shell gets the name from `.env` only.

### Prepare the second disk

1. Create an ext4 or xfs filesystem on the second disk. The publish of a set needs hard links (ADR-0074 (e)).
2. Mount the disk. Do not use a network share (ADR-0068).
3. Create both directories and both marker files:

```bash
install -d -m 0755 -o 999 -g 999 "$RECIPE_BACKUP_DIR"
install -d -m 0711 -o 999 -g 999 "$RECIPE_WALARCHIVE_DIR"
install -m 0600 -o 999 -g 999 /dev/null "$RECIPE_BACKUP_DIR/.backup-marker"
install -m 0600 -o 999 -g 999 /dev/null "$RECIPE_WALARCHIVE_DIR/.walarchive-marker"
```

4. List the disk of the Docker root directory, where the volume `pgdata` will lie, and the disk of each directory:

```bash
df --output=source,target "$(docker info --format '{{.DockerRootDir}}')" "$RECIPE_BACKUP_DIR" "$RECIPE_WALARCHIVE_DIR"
```

5. Check that the column `Filesystem` of the first data line differs from the two other data lines.

If one data line equals the first, the healthcheck of `backup` fails (`reliability.md` 7.7). The section "Service unhealthy" gives the next step.

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

2. Create the nine random values with the command below.

```bash
( umask 077 && openssl rand -hex 32 > "$RECIPE_SECRETS_DIR/<name>" )
```

Run the command once for each name: `postgres_password`, `migrator_password`, `app_password`, `backup_password`, `replicator_password`, `login_hmac_key`, `csrf_signing_key`, `plc_credential_key`, `backup_manifest_key`.
The file `plc_credential_key` holds one line of 64 lower-case hex characters. The key has no rotation (ADR-0154).
The file `backup_manifest_key` holds one line of 64 lower-case hex characters. A key rotation adds a second line (ADR-0174 (n), (q)).

3. Create the file `alerts_webhook_url`. Write the URL of the webhook into it.
4. Leave `alerts_webhook_url` empty when the plant has no webhook:

```bash
install -m 0600 /dev/null "$RECIPE_SECRETS_DIR/alerts_webhook_url"
```

5. Copy the certificate and the key of the plant PKI to `tls_cert` and `tls_key`.
6. Read the owner and the mode of each file in the table below.

| File | Reader | Owner and mode |
|---|---|---|
| `postgres_password`, `backup_password`, `replicator_password`, `backup_manifest_key` | uid 999 | `999`, `0400` |
| `login_hmac_key`, `csrf_signing_key`, `plc_credential_key`, `alerts_webhook_url` | uid 65532 | `65532`, `0400` |
| `tls_cert`, `tls_key` | uid 0 | `0`, `0400` |
| `app_password`, `migrator_password` | uid 999 and uid 65532 | `0`, `0400`, then an ACL |

7. Set the owner and the mode with one `chown` and one `chmod` for each row:

```bash
chown 999 "$RECIPE_SECRETS_DIR"/{postgres_password,backup_password,replicator_password,backup_manifest_key}
chmod 0400 "$RECIPE_SECRETS_DIR"/{postgres_password,backup_password,replicator_password,backup_manifest_key}
chown 65532 "$RECIPE_SECRETS_DIR"/{login_hmac_key,csrf_signing_key,plc_credential_key,alerts_webhook_url}
chmod 0400 "$RECIPE_SECRETS_DIR"/{login_hmac_key,csrf_signing_key,plc_credential_key,alerts_webhook_url}
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

11. List the disk of the key file and the disk of the backup directory:

```bash
df --output=source "$RECIPE_SECRETS_DIR/backup_manifest_key" "$RECIPE_BACKUP_DIR"
```

12. Check that the column `Filesystem` of the two data lines differs. If the lines are equal, move `RECIPE_SECRETS_DIR` to another disk (ADR-0174 (p)).
13. Copy the file `backup_manifest_key` to one offline medium. Keep the medium apart from the backup disk and the off-site copy (ADR-0174 (r)).

ADR-0062 (l) to (o) gives the rules of this section.
The section "Alert webhook" gives the rules of the URL of step 3.

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
3. Run `docker compose ps -a migrate backup-premigrate`. Check that both show `Exited (0)`.
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

## Destructive commands

- Never run `docker compose down -v`.
- The volume that `RECIPE_PGDATA_VOLUME` names is never a target of `docker volume rm`. Delete an old `pgdata` volume only when the three conditions of ADR-0103 (t) hold. First, a `restore-test` passed on a set of the new cluster. Second, the flush check and the WAL check passed on the new WAL line. Third, the switch is older than `volume_switch.keep_min_age` (`reliability.md` 7.11). No script and no retention deletes the volume.
- Each delete step of this page names what it deletes.
- Each delete step of a set, a file, or a directory of the backup disks follows a passed restore test.
- The section "Subnet change" runs `docker compose down` without `-v`. It deletes no volume.
- Never run `docker image prune -a` or `docker system prune -a`. They delete every image that no container uses. This includes the images that a rollback needs (section "Keep the images for a rollback").
- Never run `docker volume prune --all`. It deletes every named volume that no container uses. After `docker compose down`, this includes `pgdata`.
- Never run `docker system prune`, with or without options. It can delete stopped containers and networks of the stack.
- The one allowed prune command is `docker image prune` without `-a`. It deletes dangling images only.
- To delete the images of an old release, first check that you keep the image files and the release notes of that release. Then run `docker image rm recipe-server:<old tag> recipe-pgtools:<old tag>`. Docker refuses to delete an image that a container uses.

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
3. Run `docker compose up -d --no-deps app`. After a rollback, use "Start app after a rollback".
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
2. Run `docker compose up -d --no-deps <service>`. For `app` after a rollback, use "Start app after a rollback".
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
After a failed rollback restore after a failed upgrade, `app` stays stopped and has no container. Steps 1 and 4 cannot pass in that state.
Use the section [Failed rollback restore after a failed upgrade](#failed-rollback-restore-after-a-failed-upgrade), and do not start `app` for these steps.

From step 2 of [Restore into production](#restore-into-production) until the procedure starts `app` and `backup` again, the two services can have no container.
The same holds from step 1 of [Recover from the WAL archive](#recover-from-the-wal-archive).
From step 1 of [Major upgrade](#major-upgrade) until the procedure starts `app` again, `app` can have no container.
In [Rollback](#rollback), step 1 of the restore of the `_premigrate` set deletes `backup`.
From that step until the procedure starts `app` and `backup` again, `backup` can have no container.
From the pass of command 1 in that step, `app` can have no container too.

While `app` has no container, `proxy` can stay unhealthy, because its healthcheck calls `http://app:9000/healthz`.
Skip step 4 in each window. Step 4 runs `docker compose exec app`.

In each window, step 1 waits only for `db` and `walarchive`, and only for a service that `docker compose ps -a` lists.
Step 4 of the recovery deletes the container of `walarchive`.
The switch deletes the containers of `db` and `walarchive`, and later steps create them again.
Until step 8 of the recovery deletes the container of `db`, skip this wait when `db` did not run at step 1 of the recovery.
Such a `db` cannot become `healthy`, and the healthcheck of `walarchive` needs a running `db`.

After step 1 of this section, continue the procedure at the step where it stopped.
Two cases need an action before you continue:

- If the reboot stopped step 4 of the restore or step 6 of the recovery, the new volume is not complete. Delete it with `docker volume rm <new volume>`. Then start again at step 3 of the restore or at step 5 of the recovery.
- If the reboot stopped step 7 of the restore or step 10 of the recovery, the shell lost the variable `target`. Set `target` to the directory that sub-step 1 made. If sub-step 1 made no directory, run sub-step 1 again.

In the major upgrade, step 8 runs restore steps 3 and 4, and step 10 runs restore step 7.
In the rollback, step 1 runs restore step 3, step 3 runs restore step 4, and step 6 runs restore step 7.

1. Run `docker compose ps -a`. Wait until `db`, `app`, `proxy`, `backup`, and `walarchive` show `healthy`.
2. If a service restarts, run `docker compose logs <service>`. For `app`, use the section "Alarms, read-only mode, and restart loop".
3. Check that the host mounts the second disk. A missing host directory stops the start of `backup` and `walarchive`.
4. Run `docker compose exec app /usr/local/bin/recipe-server healthcheck --path /readyz`. Check that the exit code is 0.
5. Run `iptables -S DOCKER-USER`. Check that the `ESTABLISHED,RELATED` rule and the three `DROP` rules are present.
6. If a rule is missing, run the section "Check the host firewall" again.
7. Save the rules with the method of the host.

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

At each restore drill, also run the section "Compare the offline copy" (ADR-0174 (r)).

### Statement outside the role grammar

The restore test stops at a statement of the globals file outside the role grammar (ADR-0095).
A role setting outside the grammar comes from a change by hand.
The grammar accepts a role setting for `log_error_verbosity` and `log_parameter_max_length` only.

1. Read the line number in the `ERROR` line of the output.
2. Open the globals file `globals_<YYYYMMDD_HHMM><suffix>.sql` of the set that the test checked.
3. Read the line with that line number.
4. Write the start time into the change log ([rule](#change-log-of-the-site)). Run the command below in the directory `deploy/`. Use the role and the setting name of that line:

```bash
docker compose exec -u postgres db psql -d recipe -c "ALTER ROLE <role> RESET <setting>"
```

5. If the site needs the setting, extend the grammar in a change (ADR-0095 (e)).
6. Run `docker compose exec backup backup-now`.
7. Run `docker compose exec backup restore-test`.
8. Check that the exit code is 0 and the output holds the line `restore test passed`.

### Copy the backups off site

The off-site copy protects the sets and the WAL archive when the host or the second disk is lost.
No document gives a copy tool or a schedule. The site chooses both with its own procedure (`reliability.md` 7.7, ADR-0020).
This section names the files to copy, the host directories, and the check of a copy.

| What | Host path | Files to copy |
|---|---|---|
| Sets | `$RECIPE_BACKUP_DIR` | For each set: `recipe_<stamp>.dump`, `globals_<stamp>.sql`, `recipe_<stamp>.counts`, `recipe_<stamp>.dump.sha256`, and `recipe_<stamp>.dump.sha256.hmac` |
| Base backups | `$RECIPE_WALARCHIVE_DIR/base` | Each directory `base_<YYYYMMDD_HHMMSS>` |
| WAL segments | `$RECIPE_WALARCHIVE_DIR` | The segment files and the `.partial` file at the top of the directory |

`<stamp>` is the part of the dump name between `recipe_` and `.dump`, as in the section "Restore into production".
Do not copy the names that start with `.tmp-`, the marker files, `.lock`, and the directory `.pitr-scratch`.
A run in progress or a killed run leaves them (`reliability.md` 7.7).
The files belong to uid 999 and have the mode `0600`. The top directory of `RECIPE_WALARCHIVE_DIR` has the mode `0711`.
The result files `restore-test.result` and `base-backup.result` are the exceptions. They have the mode `0644` (ADR-0151 (g)).
The copy therefore runs as `root` or as uid 999.

The `backup` service writes one set each day at `backup.time`. `walarchive` writes WAL all the time.
A restore from a copy may lose every change after the copy.
A copy changes nothing on the host. Never move or delete a set, a base backup, or a WAL segment of the host (ADR-0099, section "Low disk space").

1. Run the section "Run the restore test". Continue only after it passes.
2. Read the name of the set in the line `restore test passed`.
3. Copy the three data files of that set. Copy its manifest `recipe_<stamp>.dump.sha256` last.
4. Copy the base backups and the WAL segments of the table.
5. Check the copy of the set. Run the command in the directory of the copy:

```bash
sha256sum -c --strict -- recipe_<stamp>.dump.sha256
```

6. Check that the command prints `OK` for each of the three data files and ends with exit code 0.
7. Protect the copies with the same access control as the database.

A set without a manifest is not complete, so the last copy of the manifest keeps a failed copy from looking complete (`reliability.md` 7.7).

A passed restore test does not show that the dump code left the set unchanged. A dump may hold code that rewrote the sets of the disk (ADR-0084).
The site procedure therefore keeps the existing protected copies. It does not replace them with a new copy.

The dump holds password hashes and the audit log. The globals file holds no password.

No document gives a check of a copy of the base backups or of the WAL segments. `basebackup-once` checked each base backup before it gave the final name (`reliability.md` 7.7).
No document gives the steps that restore a set from a copy to the host.

Copy these host files too, with the same protection:

- The file `deploy/.env` and the files of `RECIPE_SECRETS_DIR`. Step 6 of "Restore into production" reads the secret files.
- The files `Caddyfile` and `compose.yaml`, if you edited them. A release replaces both files (section "A plant device in front of Caddy").
- The image files and the release notes of the running release and of the previous release. A release moves the images as files (ADR-0064 (j), (k)).

Copy the secret files again after the sections "Key rotation", "Certificate rotation", and "Change a database password".

The file `plc_credential_key` is part of the secret backup.
The database holds the PLC passwords only in encrypted form (ADR-0154).
Without the key file, a restore cannot read the stored PLC passwords.
The key has no rotation (ADR-0154). A new key makes every stored PLC password unreadable.
An operator must then type each PLC password again.

The audit copy of step 8 of "Restore into production" is the off-site copy of a set.
That set has a later dump time than the set that you restore.
Its dump holds the audit entries up to its dump time (`reliability.md` 7.7).
You can run the section "Run a backup now" before step 1, so the set holds the newest entries.
No document gives the steps that read an entry from a copy.

### Delete orphan temp files

A killed run leaves files with the name `.tmp-<final name>` on the backup disk.
It also leaves directories `.tmp-base_*` in `base/` on the WAL archive disk (ADR-0074 (b), ADR-0077 (h)).
This step deletes only these files and directories.

1. Choose a time away from `backup.time`, `basebackup.time`, and `restore_test.time` of `reliability.md` 7.11.
2. Run the section "Run the restore test". Continue only after it passes.
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

### Change log of the site

The change log of the site is the written record of the operator.
The sections "Restore into production" and "Upgrade" name the entries that it keeps.

A manual statement as a superuser writes no audit entry (ADR-0106 (b)). The statement runs through `docker compose exec -u postgres db`.
A recovery to a target time before such a statement needs the start time of the statement.

Before each manual statement as a superuser that can change data, roles, or settings, write the start time in UTC into the change log.
Read the time with this command:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

- Write the time before you run the statement, and not after it.
- A step that runs another step by number follows the same rule.
- A statement that only reads, such as `SELECT`, needs no entry.
- The section "Recover from the WAL archive" uses this time, cut to the full second, as the target time.

### Restore into production

The restore writes into a new volume and never into the volume of the running `db` (ADR-0103 (a)).
The command `restore-volume` fills the new volume while `db` runs on the old volume.
Then the switch makes the new volume the volume of `db` (ADR-0103 (b)). The old volume stays unchanged.

Use these placeholders:

- `<stamp>` is the part of the dump file name between `recipe_` and `.dump`, for example `<YYYYMMDD_HHMM>_now`.
- `<new volume>` is the name that step 3 prints. `<old volume>` is the value of `RECIPE_PGDATA_VOLUME` in `.env` before step 5.
- `<identifier>` is the field `system_identifier` that step 4 prints. `<old identifier>` is the number that step 5 writes into the change log.
- `<old timeline>` is the number after `Latest checkpoint's TimeLineID` that step 5 writes into the change log (ADR-0106 (l)).
- `<tag>` is the tag of the image of the old cluster. Normally it is the value of `RECIPE_VERSION` in `.env`.

Each start of `db` or `app` on a volume uses the `compose.yaml` and the tag that fit that volume (ADR-0105 (a)).
The change log of the site keeps the name, the tag, the identifier, and the timeline of each volume that a switch leaves.
It also keeps one entry for each upgrade. That entry holds the previous tag, the new tag, the start time of `migrate`, the identifier, and the timeline (step 10 of "Upgrade").

Warning: a restore of a dump loses every change and audit entry after the dump time. The loss is up to 24 hours.
Delete no volume and no file of the WAL archive during the procedure. ADR-0103 (t) gives the delete rule.
A recovery from the WAL archive gives a recovery point of minutes (`reliability.md` 7.7, ADR-0068 (l), R31).
The section [Recover from the WAL archive](#recover-from-the-wal-archive) gives the steps for that recovery.

On a new host, run the section "Restore the key file on a new host" before step 1.

If the set is not the newest set, steps 1 to 8 of `restore-volume` are the test of that set (ADR-0103 (b)).
The login check of step 8 of `restore-test` then has no equal before the switch.

1. Run the restore test as in "Run the restore test". Continue only after it passes. The test checks the newest complete set.
   Read the name of the set in the line `restore test passed`. Restore that set.
2. Delete the containers of `app` and `backup`. No write after this point reaches the restored cluster, and no retention deletes the set.
   A restart of Docker or of the host starts a stopped container with `restart: always` again, on the old volume.
   Optional: first copy the logs of the two containers with `docker compose logs --no-color app backup > <file>`, because `rm` deletes them.
   Then delete the two containers:

```bash
docker compose rm -s -f app backup
```

3. Create the new volume. The command prints its name, which has the form `recipe-editor_pgdata_<YYYYMMDD_HHMMSS>` in UTC (ADR-0103 (e)):

```bash
docker volume create "recipe-editor_pgdata_$(date -u +%Y%m%d_%H%M%S)"
```

4. Fill the new volume. The container has no network. It mounts one secret, the file `backup_manifest_key` (ADR-0174 (y)). `db` keeps running on the old volume:

```bash
docker compose run --rm --no-deps -T -v "<new volume>:/var/lib/postgresql/data" restore-volume <stamp>
```

   The last output line holds `restore volume completed`, the name of the set, and the field `system_identifier`. Note that field.
   If the command exits with a code other than 0, read its `ERROR` line. The new volume holds no production data and `.env` does not name it.
   Delete the new volume with `docker volume rm <new volume>`, fix the cause, and start again at step 3 (ADR-0103 (i)).
   To end the attempt, set `RECIPE_VERSION` in `.env` to `<tag>`. Do this before you start a container.
   For a plain rollback, `<tag>` is the new release tag. Run [Leave the file compose.previous.yaml](#leave-the-file-composepreviousyaml) with `<write tag>` as `<tag>`.
   Then start `app` with `docker compose up -d --no-deps app`. Create `backup` again with `docker compose up -d --no-deps --force-recreate backup`.
   For a failed upgrade, `.env` keeps the previous tag and `COMPOSE_FILE`. Start no `app` and no container of the new image. See [Failed rollback restore after a failed upgrade](#failed-rollback-restore-after-a-failed-upgrade).
   In every other case, start `app` with `docker compose up -d --no-deps app`. Then start `backup` with `docker compose up -d --no-deps backup`.
   In a plain rollback, the old volume fits `<tag>` only.
   After a failed upgrade, the old volume holds the schema of the failed `migrate`. That schema can match no image (ADR-0083).
   In a major upgrade, run these commands in the directory of `<old tag>` (section "Major upgrade", ADR-0105 (c), (e)).

5. Run the switch (ADR-0103 (f)). Never run `docker compose down` or `docker compose start` here:
   1. Stop the five containers and delete them. The commands delete containers only:

```bash
docker compose stop proxy app backup walarchive db
docker compose rm -f proxy app backup walarchive db
```

   2. Read the state of the old volume without a network. The line `Database cluster state` must show `shut down`:

```bash
docker run --rm --network none --security-opt no-new-privileges:true -e LC_ALL=C -v "<old volume>:/var/lib/postgresql/data:ro" --entrypoint pg_controldata "recipe-pgtools:<tag>" /var/lib/postgresql/data
```

   3. If the state differs, keep the old name in `.env` and run `docker compose up -d --no-deps db`. Then run sub-step 1 and sub-step 2 again.
      In a major upgrade, run that `up` command in the directory of `<old tag>`. Its `compose.yaml` has the old major version (ADR-0105 (f)).
      After that `up` command, change back into the directory of the new release. Then run sub-step 1 and sub-step 2 again there.
   4. Write the old name, `<tag>`, and the lines `Database system identifier` and `Latest checkpoint's TimeLineID` of that output into the change log of the site (ADR-0105 (b), ADR-0106 (l)).
   5. Write `<new volume>` as the value of `RECIPE_PGDATA_VOLUME` in `.env`. Load the values into the shell again. Do not export them:

```bash
. ./.env
```

   6. Print the name of `pgdata`. It must equal `<new volume>`:

```bash
docker compose config | awk '/^volumes:/{v=1;next} /^[^ ]/{v=0} v&&/^  pgdata:/{p=1;next} v&&/^  [^ ]/{p=0} p&&/^    name:/{print $2}'
```

   7. Run the command of sub-step 2 again with `<new volume>`. The state must be `shut down`. The identifier must equal `<identifier>`.
      Else stop and delete no volume. Run the [Switch back](#switch-back) of ADR-0103 (g) at once.
   8. Start `db`. Run `docker compose ps db` until `db` shows `healthy`. Then check that `db` mounts the new volume:

```bash
docker compose up -d --no-deps db
docker inspect --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Name}}{{end}}{{end}}' "$(docker ps -q --filter label=com.docker.compose.project=recipe-editor --filter label=com.docker.compose.service=db)"
```

6. Write the start time into the change log ([rule](#change-log-of-the-site)). Set the password of each login role from its secret file (ADR-0033). Never type a password on a command line:

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

Until this step, no role has a known password, so no password login succeeds (ADR-0103 (k)).
Psql reads each secret file inside `db`, so no password reaches a command line.
The setting `log_min_duration_statement = -1` stops the slow-statement line from holding a password (`reliability.md` 3.4).
Count the lines of the log of `db` that name a role next to `PASSWORD`. The count prints no log line:

```bash
docker compose logs db | grep -c -E 'ROLE (postgres|recipe_[a-z]+) PASSWORD'
```

The count must be 0. A count above 0 means a password may be in the log.

7. Start the new WAL line before `app` (ADR-0103 (r), (s)). Run the sub-steps in one shell, because they share the variable `target`:
   1. Create the target directory. The command fails on an existing name:

```bash
target="${RECIPE_WALARCHIVE_DIR%/}.moved-$(date -u +%Y%m%d_%H%M%S)"
mkdir -m 0700 -- "$target"
```

   2. Check that both paths show the same number, as in step 4 of "A gap in the WAL stream". If they differ, stop before any move:

```bash
stat -c %d "$RECIPE_WALARCHIVE_DIR" "$target"
```

   3. Write the lines `Database system identifier` and `Latest checkpoint's TimeLineID` of the old cluster into the file `old-system-identifier` (ADR-0106 (k)):

```bash
printf '%s\n%s\n' 'Database system identifier: <old identifier>' "Latest checkpoint's TimeLineID: <old timeline>" > "$target/old-system-identifier"
```

      If the change log entry of the old cluster holds no timeline, or holds the word `unreadable`, write the first line only (ADR-0106 (l)):

```bash
printf '%s\n' 'Database system identifier: <old identifier>' > "$target/old-system-identifier"
```

   4. Move the segment files, the `.partial` file, each timeline history file, `.system-identifier`, and `base-backup.result`:

```bash
find "$RECIPE_WALARCHIVE_DIR" -maxdepth 1 -type f -regextype posix-extended \( -regex '.*/[0-9A-F]{24}(\.partial)?' -o -regex '.*/[0-9A-F]{8}\.history' -o -name '.system-identifier' -o -name 'base-backup.result' \) -exec mv -t "$target" -- {} +
```

      ADR-0106 (k) and ADR-0151 (q) name these files. A history file has 8 uppercase hexadecimal digits and the suffix `.history`. The command moves no other name. The file `base-backup.result` describes the base backups of the old cluster, so it moves too.

   5. Move each base backup of the old cluster into the directory `base` of the target:

```bash
mkdir -m 0700 -- "$target/base"
find "$RECIPE_WALARCHIVE_DIR/base" -mindepth 1 -maxdepth 1 -type d -regextype posix-extended -regex '.*/base_[0-9]{8}_[0-9]{6}' -exec mv -t "$target/base" -- {} +
```

   6. Check that no segment file, no base backup, no file `.system-identifier`, and no file `base-backup.result` remain:

```bash
ls -A "$RECIPE_WALARCHIVE_DIR" "$RECIPE_WALARCHIVE_DIR/base"
```

   7. Run `docker compose up -d --no-deps walarchive`. At its start, `walarchive` writes the file `.system-identifier` of the new cluster.
   8. Run steps 9 to 13 of "A gap in the WAL stream". Take no base backup before the stream check passes.

   The marker file, `.lock`, `.pitr-scratch`, and the directory `base` stay in the archive directory.
   Do not delete the target directory. ADR-0103 (t) gives the delete rule.

8. Truncate the tables `session` and `login_attempt`. Then re-apply the user changes:
   1. Write the start time into the change log ([rule](#change-log-of-the-site)). Then truncate the tables:

```bash
docker compose exec -u postgres db psql -d recipe -c "TRUNCATE session, login_attempt"
```

   2. Re-apply each user deactivation and password reset recorded after the dump time. Use the off-site audit copy, if one exists.

9. Start `app` after a record of the restored audit time:
   1. Record the value of `audit.last_check_at` with the `app_setting` command of the first place below step 10.
   2. If the command prints no row, record "missing".
   3. Record the highest `id` of `security_event` with the `security_event` command below step 10.
   4. Run `docker compose up -d --no-deps app`. After a rollback, use "Start app after a rollback".
   5. Run `docker compose up -d --no-deps proxy backup`.
   6. Check with the `docker inspect` command of step 5 that `app` and `backup` mount the new volume. Use the service names `app` and `backup`.
   7. Wait until the alert `replication` is inactive (step 14 of "A gap in the WAL stream").
10. Check the audit chain. Wait for the next completed run of the job `audit_chain_check`. Run no command for it.

The section "Copy the backups off site" describes the off-site audit copy of step 8.

The next completed run is the applicable run that completes first (ADR-0100, `data-model.md` 4.9).
The catch-up run starts after `audit.catchup_delay`. It applies when the restored `audit.last_check_at` is missing or older than `audit.catchup_age` at the start of `app`.
The nightly run at `audit.check_time` stays active in all cases. A newer completed nightly check suppresses the catch-up run.
`reliability.md` 7.11 gives the values of these settings.

The wait is the delay to the start of the run plus the time of the check (ADR-0100).
No setting limits the time of the whole check. `audit.check_statement_timeout` limits the head read and each batch. `audit.max_check_age` only raises an alert. The wait therefore has no fixed upper limit.
Read the result in three places.

The first place is `audit.last_check_at` in `app_setting`. The value is a time in UTC.
The restore brings back the value of the dump. That value can lie in the future, for example after a correction of the clock (`reliability.md` 3.7, clock trust).
Step 9 records this value with the command below before the start of `app`.

```bash
docker compose exec -u postgres db psql -d recipe -c "SELECT value FROM app_setting WHERE key = 'audit.last_check_at'"
```

After step 9, read the value again. A value that differs from the recorded value shows a completed check.
A value equal to the recorded value is not a new check.

An active `audit_check_age` is unresolved evidence about the age or the clock trust of the check (ADR-0093 (a)). It does not prove that no check completed. Investigate it in section "Alarms, read-only mode, and restart loop" before you accept the result.

The second place is the event log of `app`. An `ERROR` line of the job `audit_chain_check` with the `error_code` `E-AUD-002` shows a break.
The command below uses the form of the subsection "Read the lines of a job" in the section "Read the logs". It adds the pattern of the code:

```bash
docker compose logs --no-log-prefix app | grep -F '"job":"audit_chain_check"' | grep -F '"error_code":"E-AUD-002"'
```

The third place is the alert condition `audit_chain_broken` (section "Alarms"). It becomes active after a break.

The durable proof of a chain without a break is the table `security_event` (ADR-0100).
A completed check as in the first place and no new row of the kind `audit_chain_broken` show a chain without a break.
Step 9 records the highest `id` of the table with the command below, before the start of `app`:

```bash
docker compose exec -u postgres db psql -d recipe -c "SELECT coalesce(max(id), 0) FROM security_event"
```

After the check completes, count the new rows of the kind. Replace `<id>` with the recorded value. The count must be 0:

```bash
docker compose exec -u postgres db psql -d recipe -c "SELECT count(*) FROM security_event WHERE kind = 'audit_chain_broken' AND id > <id>"
```

The dump brings back the old rows, so the count uses `id` and not `occurred_at`. A restored row can carry a future time.
The job writes the row of a break before it records `audit.last_check_at`.

Docker can drop log lines, and a failed snapshot of the job `alerts` evaluates no condition (`reliability.md` 3.5 and 3.7).
The `E-AUD-002` line and the alert are diagnostic signals. A missing line or an inactive alert alone does not show a chain without a break.
A check without a break writes no log line, so a missing line alone does not show a completed check.

A run that returns another error code is not a completed check, and `audit.last_check_at` keeps its value.
Read that code in the `ERROR` line. Then follow its row in `reliability.md` 1.2.

The alert `audit_check_age` becomes active when `audit.last_check_at` is older than `audit.max_check_age`.

### Recover from the WAL archive

This recovery fills a new volume from a base backup and the WAL archive (ADR-0106).
It gives a recovery point of minutes (R31). It also works after the loss of the `pgdata` disk, if the WAL archive disk is intact.
Use it also to undo a logical fault, for example a wrong manual statement, up to a chosen time.
The recovery uses the switch of "Restore into production". The old volume stays unchanged.

Warning: the recovery loses each write after the recovery point.
Delete no volume and no file of the WAL archive during the procedure (ADR-0103 (t)).
The downtime includes the copy, the replay, the new WAL line, and its first base backup.
The section [Recovery drill](#recovery-drill) measures the recovery time on the data of the site.

Use these placeholders in addition to the placeholders of "Restore into production":

- `<new volume>` is the name that step 5 prints.
- `<time>` is the target time. It has the form `YYYY-MM-DDTHH:MM:SSZ` in UTC.
- `<seq>`, `<hash>`, and `<lsn>` are the values that steps 2 and 3 print.
- `<recovery tag>` is the tag of the schema at the recovery point. Normally it equals `<tag>`.
- `<timeline>` is the field `timeline` of the last output line of step 6.

A target time before the `migrate` of an upgrade needs the previous tag as `<recovery tag>`. The change log of the upgrade gives the time of `migrate`.
If `<recovery tag>` differs from `<tag>`, run steps 6 to 12 in the directory `deploy/` of that release, with its `.env` and images.
Each start of `db` or `app` on the new volume uses that tag (ADR-0105 (a)).

Choose the target before step 1:

- Without `--target-time`, the recovery ends at the end of the archive. It replays the `.partial` file.
- With `--target-time <time>`, the recovery ends before the first commit at or after that time. Use it for a logical fault.
- Read the time in the audit log of the old cluster, if `db` runs. Else read it in the off-site audit copy.
- Take `occurred_at` of the first unwanted audit entry. Cut it to the full second.
- A commit in the same second before that entry is also lost.
- A manual statement as a superuser writes no audit entry. Use its start time from the [change log of the site](#change-log-of-the-site), cut to the full second.
- Without such a record, the target is an estimate of the operator. The values of step 7 then matter most.

1. Delete the containers of `app` and `backup`. After this point, `app` writes no audit entry.
   A restart of Docker or of the host starts a stopped container with `restart: always` again, on the old volume.
   Optional: first copy the logs of the two containers with `docker compose logs --no-color app backup > <file>`, because `rm` deletes them.
   Then delete the two containers:

```bash
docker compose rm -s -f app backup
```

2. If `db` runs, read the newest audit entry. Note `chain_seq` and the hash:

```bash
docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT chain_seq, encode(entry_hash, 'hex') FROM public.audit_log ORDER BY chain_seq DESC LIMIT 1"
```

   Only a recovery to the end of the archive uses `<seq>` and `<hash>`. With a target time, step 6 does not pass them, because the newest entry can lie after the target.

3. If `db` runs, read the WAL position. Note it as `<lsn>`:

```bash
docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT pg_current_wal_lsn()"
```

   Run the flush check of step 12 of "A gap in the WAL stream" with `<lsn>` in place of `<position>`. Run it until it returns `t`.
   Only a recovery to the end of the archive uses `<lsn>` as `PITR_EXPECTED_LSN`. With a target time, step 6 does not pass it.

4. Delete the container of `walarchive`. The `.partial` file then stays stable.
   A restart of Docker or of the host starts a stopped container with `restart: always` again. A running `walarchive` can change the `.partial` file.
   Optional: first copy the log of the container with `docker compose logs --no-color walarchive > <file>`, because `rm` deletes it.
   Then delete the container:

```bash
docker compose rm -s -f walarchive
```

5. Create the new volume. Run step 3 of "Restore into production". The command prints the name of the volume.
6. Fill the new volume. The container has no network and no secret. `db` keeps running on the old volume, if it runs:

```bash
docker compose run --rm --no-deps -T -e PITR_EXPECTED_LSN=<lsn> -e PITR_MARKER_SEQ=<seq> -e PITR_MARKER_HASH=<hash> -v "<new volume>:/var/lib/postgresql/data" pitr-volume
```

   - With a target time, omit the three options `-e` and add `--target-time <time>` after `pitr-volume`.
   - If `db` does not run, omit the three options `-e`. The script then checks the end of the archive without them.
   - The last output line is `pitr volume completed`. Note its fields `base_backup`, `system_identifier`, `timeline`, and `drill`.
   - If the command exits with a code other than 0, read its `ERROR` line. The new volume holds no production data.
   - The line `the recovery ended before the end of the WAL archive` shows a gap in the archive. Do not switch to this volume.
   - A base backup from before this release has no file `backup_end_time`. A target inside its run stops the recovery with `FATAL`. Never select a later target, because it can replay the fault.
   - The line `cannot read heartbeat.last_at of the recovered cluster` shows that the recovered cluster has no row `heartbeat.last_at`.
     No migration creates the row. The job `heartbeat` writes it first, one `heartbeat.interval` after the start of `app`.
     A target time before that first write, on a new site, can give this line. Select a later target time or recover to the end of the archive.
   - After a failed run, end the attempt as in step 7.

7. Compare the values of the last line with your expectation. A value that does not fit ends the attempt:
   1. The field `base_backup` names the base backup that you expect.
   2. The field `system_identifier` equals the identifier of the old cluster.
   3. The fields `chain_seq`, `occurred_at`, and `action` show the newest audit entry that you expect to keep.
   4. The field `heartbeat_last_at` shows a time that fits the recovery point.
   5. The field `drill` is `false`.

   To end the attempt, delete the new volume.
   If `<recovery tag>` differs from `<tag>`, change into the directory `deploy/` of `<tag>`.
   Set `RECIPE_VERSION` in `.env` to `<tag>`.
   If step 4 ran, create `walarchive` again with `docker compose up -d --no-deps walarchive`.
   Then end the attempt as in step 4 of "Restore into production". Do not switch.
8. Run the switch. Run step 5 of "Restore into production" with `<new volume>`, `<identifier>`, and `<timeline>`. Make these changes (ADR-0106 (j)):
   1. In sub-step 2, use the rule of step 2 of [Switch back](#switch-back). The old volume needs no state. Never start `db` on it.
   2. Skip sub-step 3.
   3. In sub-step 4, write the identifier and the timeline that `pg_controldata` reads into the change log. If it reads none, write the word `unreadable`.
   4. In sub-step 7, the timeline of the new volume must also equal `<timeline>`.
   5. Refuse a drill volume. A volume that you filled with `--drill` never becomes production. Before sub-step 1, check the file `pitr-drill` at the root of the new volume:

```bash
docker run --rm --network none --security-opt no-new-privileges:true -v "<new volume>:/var/lib/postgresql/data:ro" --entrypoint find "recipe-pgtools:<recovery tag>" /var/lib/postgresql/data -maxdepth 1 -name pitr-drill
```

      When the command prints a path, the file exists. This volume is a drill volume. Do not switch. End the attempt as in step 7.
      Do the same when the field `drill` of step 7 is `true`.

   If sub-step 7 fails, run [Switch back](#switch-back) at once. If the old volume was unreadable, no switch back exists. The new volume is then the only cluster.
9. Set the role passwords. Run step 6 of "Restore into production". The cluster already holds the passwords of production, and the secret files stay the one source.
10. Start the new WAL line. Run step 7 of "Restore into production". In a recovery, `<old identifier>` equals `<identifier>`.
11. Truncate `session` and `login_attempt`. Run sub-step 1 of step 8 of "Restore into production".
    1. Re-apply each user change recorded after the recovery point. Use the audit log of the old cluster, or the off-site audit copy.
    2. Run step 9 of "Restore into production". It records the audit values, starts `app`, then `proxy` and `backup`, and waits for the alert `replication`.
12. Wait for the audit chain check. Run step 10 of "Restore into production". Then prove the first set of the new cluster:

```bash
docker compose exec backup backup-now
docker compose exec backup restore-test
```

The truncate of step 11 matters. A session that the old cluster revoked after the recovery point comes back with the recovery.

### Recovery drill

The drill measures the recovery time on the data of the site (ADR-0106 (o)).
The recovery time objective is 1 hour (R31, `reliability.md` 7.7).
The drill runs steps 5 and 6 of "Recover from the WAL archive" on a drill volume while the stack runs.
It never switches the drill volume into production. The file `pitr-drill` and the switch keep that volume out of production (ADR-0106 (j)).
The script checks the free space first, so the drill never puts `app` into read-only mode.

Run the drill after each upgrade of `reliability.md` 7.10. The last step of "Upgrade" links this section.

Use these placeholders:

- `<drill volume>` is the name that step 4 prints.
- `<time>` is the target time of step 3. It has the form `YYYY-MM-DDTHH:MM:SSZ` in UTC.
- `<fill time>` is the time that step 5 prints. `<run time>` is the time of the last run of `basebackup-once` from step 6.

1. Check that no process `basebackup-once` runs. The drill runs outside that run. If one runs, wait until it ends:

```bash
docker compose top walarchive
```

2. Read the end of the newest base backup. The target time must lie at or after it:

```bash
newest=$(ls -1 "$RECIPE_WALARCHIVE_DIR/base" | grep -E '^base_[0-9]{8}_[0-9]{6}$' | sort | tail -n 1)
cat "$RECIPE_WALARCHIVE_DIR/base/$newest/backup_end_time"
```

   A base backup from before this release has no file `backup_end_time`. Its line `START TIME` in `backup_label` counts (ADR-0106 (c)).
3. Print the target time. The command uses 10 minutes. That is two times `heartbeat.interval` of 5 min (`reliability.md` 7.11):

```bash
date -u -d '10 minutes ago' +%Y-%m-%dT%H:%M:%SZ
```

   If the result lies before the time of step 2, wait and run this step again.
   The job `heartbeat` then gives a commit after the target.
4. Create the drill volume. Run step 3 of "Restore into production". The command prints the name of the volume.
5. Fill the drill volume. `app`, `backup`, and `walarchive` keep running. The container has no network and no secret:

```bash
time docker compose run --rm --no-deps -T -v "<drill volume>:/var/lib/postgresql/data" pitr-volume --target-time <time> --drill
```

   - Omit the three options `-e`, as step 6 of "Recover from the WAL archive" says for a target time.
   - The last output line is `pitr volume completed`. Check that its field `drill` is `true`.
   - The line `real` of `time` is the fill time. Note it as `<fill time>`.
   - If the command exits with a code other than 0, read its `ERROR` line. Delete the drill volume as in step 8. The drill then gives no recovery time.
   - The line `another run holds the lock of the WAL archive` shows that `basebackup-once` runs. Delete the drill volume. Start again at step 1.
6. Read the time of the last run of `basebackup-once`. The weekly run starts at `basebackup.time`:

```bash
docker compose logs --timestamps walarchive | grep -F -e 'base backup started' -e 'base backup completed'
```

   Take the last pair of lines. Subtract the time of the line `base backup started` from the time of the line `base backup completed`.
   Note the result as `<run time>`.
   If the command prints no such pair, use the newest base backup directory. Subtract the stamp of its name from `backup_end_time`:

```bash
newest=$(ls -1 "$RECIPE_WALARCHIVE_DIR/base" | grep -E '^base_[0-9]{8}_[0-9]{6}$' | sort | tail -n 1)
stamp=${newest#base_}
end=$(cat "$RECIPE_WALARCHIVE_DIR/base/$newest/backup_end_time")
echo $(( $(date -u -d "$end" +%s) - $(date -u -d "${stamp:0:4}-${stamp:4:2}-${stamp:6:2}T${stamp:9:2}:${stamp:11:2}:${stamp:13:2}Z" +%s) ))
```

   - The command prints `<run time>` in seconds.
   - The result is at most 2 seconds too long. The file holds the next full second after the end, and the stamp cuts the start to the full second.
   - A base backup from before this release has no file `backup_end_time`. Then the fallback gives no result.
7. Add `<fill time>` and `<run time>`. Write both times and their sum into the change log of the site.
   - The sum must stay below the recovery time objective of 1 hour.
   - A larger sum is a finding of the drill.
8. Delete the drill volume. Never switch it:

```bash
docker volume rm <drill volume>
```

9. If the drill held the lock at `basebackup.time`, the weekly run refused to start. Its log holds the line `another base backup run holds the lock`.
   Then run `docker compose exec walarchive basebackup-once` at once.

## Check the stored data against the platform ranges

The query lists the stored rows that the platform ranges reject. It writes nothing.
The output has the columns `machine_code`, `reference_number`, `version_number`, `version_status`, `tag`, `part`, and `stored_text`.
The column `part` is `value`, `limit_min`, `limit_max`, `s7_address`, or `wide_text`.
A row can be a false hit when its type tree holds a `ULINT`, `LWORD`, `LTIME`, `REAL`, or `LREAL` leaf.
The false hit can be a `value` row, a `limit_min` row, or a `limit_max` row of that leaf.

A row of `wide_text` follows ADR-0180. It is a parameter on `S7_1200` or `S7_1500` with a `WCHAR` or `WSTRING` value that the rule refuses.
The column `stored_text` holds the text leaf.
A `WCHAR` value refuses a character above `U+D7FF`. A `WSTRING` value refuses a character from `U+D000` up.
The query compares each text leaf with the limit of its own type, also in a struct with both types.
A `wide_text` row is a true hit. Do not accept it as a false hit.
An `Approved` version with a `wide_text` row becomes not loadable after the upgrade.

1. Run the query from the directory `deploy/` of the new release:

```bash
docker compose exec -T -u postgres db psql -d recipe -v ON_ERROR_STOP=1 < db/check-platform-ranges.sql
```

2. The owner reads each output row.
3. Accept each row that needs no change.
4. Fix a draft row with `SetParameterValue`, `ReplaceLimits`, or `UpdateParameterDefinition`.
5. Fix an approved row by creating a new version.
6. Fix each `wide_text` row before the upgrade. Replace the refused character in a draft, or create a new version for an approved row.
7. Write the result into the [change log of the site](#change-log-of-the-site).

## Check the stored type rows against the identifier rules

The query lists the stored type rows that the identifier rules of ADR-0143 reject. It writes nothing.
The output has the columns `machine_code`, `type_name`, `revision`, `part`, and `stored_text`.
A case pair gives two rows, one for each spelling.
The column `part` is `struct_name_case`, `member_name_case`, `array_subscript`, `reserved_struct_name`, or `s7_reserved_struct_name`.
The parts `struct_name_case`, `member_name_case`, and `array_subscript` must give 0 rows before an upgrade.
Migration `0011` stops the upgrade otherwise.
The part `reserved_struct_name` is for information. A struct with a reserved name gets no new revision.
The owner signs off each row of `reserved_struct_name`.

A row of `s7_reserved_struct_name` follows ADR-0186. It is a struct on a machine with `S7_300`, `S7_1200`, or `S7_1500`.
The name of the struct is a Siemens reserved name or matches the family `PD_TEL<n>`, in any case.
The part is for information. The rule refuses only new writes. The struct keeps its rows and gets no new revision.
A listed `Approved` version stays loadable after the upgrade.
The owner signs off each row of `s7_reserved_struct_name`.

1. Run the query from the directory `deploy/` of the new release:

```bash
docker compose exec -T -u postgres db psql -d recipe -v ON_ERROR_STOP=1 < db/check-identifier-rules.sql
```

2. The owner reads each output row.
3. Stop the upgrade when a row of `struct_name_case`, `member_name_case`, or `array_subscript` remains.
4. Accept or sign off each row of `reserved_struct_name` and `s7_reserved_struct_name`.
5. Write the result into the [change log of the site](#change-log-of-the-site).

## Check the stored data against the S7 layout rules

The query lists the stored rows that the S7 layout rules of ADR-0147, ADR-0148, ADR-0177, ADR-0178, ADR-0179, and ADR-0187 affect. It writes nothing.
The output has the columns `machine_code`, `reference_number`, `version_number`, `tag`, `part`, and `type_text`.
The column `part` is `nested_array`, `layout_changed`, `array_bound`, `bool_array_3d`, or `nesting`.
A type row has no reference, no version, and no tag.

The parts have three kinds of result for an upgrade.
The first kind refuses only new writes. A listed `Approved` version stays loadable after the upgrade:

- A row of `nested_array` is an array of an array on `S7_300`, `S7_1200`, or `S7_1500`, or a parameter of such a type.
  ADR-0177 adds `S7_300` to the rule of ADR-0148.
- A row of `nesting` is a struct row or a parameter type with more than 8 S7 levels on `S7_300`, `S7_1200`, or `S7_1500` (ADR-0179).
  A struct counts 1 level. An array of a struct counts 3 levels.

The second kind changes `loadable`. A listed `Approved` version becomes not loadable after the upgrade:

- A row of `array_bound` is a parameter with an S7 address and an array bound outside `-32768` to `32767` (ADR-0178).
- A row of `bool_array_3d` is a parameter with an S7 address and a `BOOL` array with three dimensions (ADR-0187).

The third kind can change `loadable`. The export check of ADR-0119 (b) runs the end rule and the overlap rule on the new layout (ADR-0147 (k1), ADR-0149):

- A row of `layout_changed` is a parameter with an S7 address whose layout changes with the upgrade (ADR-0147).
  A fault of one of these rules can make a listed `Approved` version not loadable.

A version is not loadable when any part shows a loadability fault for one of its parameters, for example `bool_array_3d`.
The owner signs off each row of all five parts.
The owner fixes each row of the second kind before the upgrade. A draft row needs a change. An approved row needs a new version.
The owner records each listed row in the upgrade ticket.

1. Run the query from the directory `deploy/` of the new release:

```bash
docker compose exec -T -u postgres db psql -d recipe -v ON_ERROR_STOP=1 < db/check-s7-layout-rules.sql
```

2. The owner reads each output row.
3. Sign off each row of `nested_array`, `layout_changed`, `array_bound`, `bool_array_3d`, and `nesting`.
4. Write the result into the [change log of the site](#change-log-of-the-site).

## Check the live S7 machines for a PLC connection record

Migration `0013` moves the data block number to the machine (ADR-0156).
Migration `0014` copies the number from the PLC connection record of the machine.
A live machine on `S7_300`, `S7_1200`, or `S7_1500` without that record has no number to copy.
Migration `0013` stops on such a machine before it adds the column and changes nothing.
The query lists these machines. It writes nothing.
The output has one column, `code`, ordered by `code`.
An archived machine and a machine on `CodesysIec` or `TwinCat3` are not in the output.

1. Run the query. The previous release can still run.

```bash
docker compose exec -T -u postgres db psql -d recipe -v ON_ERROR_STOP=1 <<'SQL'
SELECT machine.code
FROM machine
WHERE machine.archived_at IS NULL
  AND machine.plc_platform IN ('S7_300', 'S7_1200', 'S7_1500')
  AND NOT EXISTS (
      SELECT 1 FROM plc_connection WHERE plc_connection.machine_id = machine.id
  )
ORDER BY machine.code;
SQL
```

2. Save the PLC connection of each listed machine with the previous release.
3. Run the query again. It must return 0 rows.
4. Write the result into the [change log of the site](#change-log-of-the-site).
5. Continue with the upgrade when the query returns 0 rows.

If you skip this check, `migrate` stops with this message. The list names the same machines:

```text
migration 0013 stopped: live S7 machines without a PLC connection record: <codes>. Save the PLC connection of each listed machine with the previous release (docs/ops/runbook.md).
```

## Check the TwinCAT version before you add a TwinCat3 machine

The editor does not read the TwinCAT version of a controller.
The operator checks the version before a plant adds a TwinCat3 machine.
The types `LDATE`, `LTOD`, and `LDT` need TwinCAT 3.1.4026.0 or later.
Assumption A46 requires this version for each TwinCat3 machine (ADR-0142).

1. Read the TwinCAT version of the controller in the TwinCAT engineering environment.
2. Compare the version with 3.1.4026.0.
3. If the version is lower, the machine does not meet A46. Update the controller before you add the machine.

## Upgrade and rollback

The steps follow `reliability.md` 7.10 (ADR-0082, ADR-0083).
A rollback needs the images of the previous release. See the section "Keep the images for a rollback".

### Upgrade

Before step 1, do the text block "Manifest key file before step 1" and run the "Probe of the previous image". Both are after step 24.

1. Read the release notes. Note whether the migration follows the expand pattern. Note the line "manual superuser step".
   Note the new release tag. Only command 1 of the `_premigrate` restore in the section "Rollback" uses it, as `<new tag>`.
2. Do the three parts below in this order: the installation, the name check, and the segment check.
   Avoid the times `backup.time`, `basebackup.time`, and `restore_test.time` of `reliability.md` 7.11 for the whole upgrade.
   1. Run the section "Check the stored data against the platform ranges" from the directory `deploy/` of the new release.
   Run it after the segment check and before step 3.
   Continue only when the query returns 0 rows or the owner accepted each row.
   2. Run the section "Check the stored type rows against the identifier rules" from the directory `deploy/` of the new release.
   Run it after the platform-ranges check and before step 3.
   Continue only when the parts `struct_name_case`, `member_name_case`, and `array_subscript` give 0 rows.
   3. Run the section "Check the stored data against the S7 layout rules" from the directory `deploy/` of the new release.
   Run it after the identifier check and before step 3.
   Continue only when the owner signed off each row.
   4. Run the section "Check the live S7 machines for a PLC connection record".
   Run it after the S7 layout check and before step 3.
   Save the PLC connection of each listed machine with the previous release.
   Continue only when the query returns 0 rows.
3. Run the restore test of the previous image in a new container. `.env` still holds the previous tag. Continue only after the test passes.
   Run only one of the two commands.

   If the probe of the previous image printed `check`, run this command:

```bash
docker compose run --rm --no-deps backup restore-test
```

   If the probe printed `no-check`, run this command:

```bash
docker compose -f compose.yaml -f compose.previous.yaml run --rm --no-deps backup restore-test
```

4. Load the new images as in the section "Load the images". Do not stop the running stack.
5. Set `RECIPE_VERSION` in `.env` to the new release tag.
   Run [Leave the file compose.previous.yaml](#leave-the-file-composepreviousyaml) with `<write tag>` as the new release tag.
   The copied `.env` can hold `COMPOSE_FILE` from an earlier rollback. That section deletes it.
   This step does not create `backup`. Command 1 of the four commands gives `backup` the new image and the key.
6. Run steps 6 and 7 of the section "Create the environment file". The image tags show the new tag.
   Then do the steps of "Key file of step 6".
7. If the plant has a device, use steps 1 to 3 of "A plant device in front of Caddy" again.
8. Run `docker compose up -d backup-premigrate`. It writes the rollback set `_premigrate`.
   - In the first release with the mount of ADR-0103 (l), this command replaces `db` once on the same volume.
   - `app` loses its connections for the start time of `db` (ADR-0105 (k)).
9. Wait for exit code 0 of `backup-premigrate` with `docker compose ps -a backup-premigrate`.
10. Write the change log entry of the upgrade, then run `migrate`. A recovery to a target time reads this entry (ADR-0106 (e)).
   1. Read the identifier and the timeline of the cluster from `db`. Write both lines into the entry:

```bash
docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT 'Database system identifier: ' || system_identifier FROM pg_control_system() UNION ALL SELECT 'Latest checkpoint''s TimeLineID: ' || timeline_id FROM pg_control_checkpoint()"
```

   2. Write the previous tag and the new tag into the entry. The previous tag is the value of `RECIPE_VERSION` before step 5.
   3. Read the time and write it into the entry as the start time of `migrate`. Do this before sub-step 4:

```bash
date -u +%Y-%m-%dT%H:%M:%SZ
```

   4. Run `docker compose up -d --no-deps migrate`. Without `--no-deps`, Compose runs `backup-premigrate` a second time.
11. Wait for exit code 0 of `migrate` with `docker compose ps -a migrate`. Between the commit of `0013` and the row lock of `0014`, a previous-release command that updates a live S7 machine fails with `E-DB-003`.
12. If `backup-premigrate` or `migrate` fails, stop. Do the parts below in this order.
   1. Set the previous tag in `.env`. Check it (step 6).
   2. If the probe of the previous image printed `no-check`, add the line `COMPOSE_FILE=compose.yaml:compose.previous.yaml` to `.env`. Delete each older line `COMPOSE_FILE`.
   3. For the same probe result, run `docker compose up -d --no-deps --force-recreate backup`. The running `backup` then runs the previous image without the key and without the weekly `restore-test`.
   4. For a `_premigrate` restore after this failure, use step 1 of the section "Rollback". `<new tag>` there is the tag that step 1 of this list noted.
13. Start the guard step. Write the start time into the change log ([rule](#change-log-of-the-site)). Run `docker compose exec -u postgres db psql -d recipe`.
14. In a second terminal, print the guard statement with the command below. Paste the output into `psql`.

```bash
sed -n '/^CREATE FUNCTION audit_guard\.audit_objects_dropped()/,/^\$\$;$/p' db/initdb/01-roles.sh \
  | sed '1s/^CREATE FUNCTION/CREATE OR REPLACE FUNCTION/'
```

The command prints the statement of the new `01-roles.sh` from `CREATE OR REPLACE FUNCTION` to the line `$$;`.
15. Compute the hash of the function in the same session:

```sql
SET search_path = pg_catalog;
SELECT encode(sha256(convert_to(pg_get_functiondef('audit_guard.audit_objects_dropped()'::regprocedure), 'UTF8')), 'hex');
```

16. Leave `psql` with `\q`. Read field 4 of the expected line:

```bash
grep -F 'FUNCTION|audit_guard audit_objects_dropped()|' pgtools/restore-code.expected | cut -d'|' -f4
```

17. Compare the two hashes. Continue only when they are equal. Otherwise run steps 13 to 17 again.
18. Run `docker compose up -d --no-deps app`. Docker replaces the container of `app` only.
19. Check `/readyz` with `docker compose exec app /usr/local/bin/recipe-server healthcheck --path /readyz`. The command ends with exit code 0.
20. If you used step 7, run `docker compose up -d --force-recreate --no-deps proxy`.
21. Run the check of the section "Check client_ip of the first login".
22. Log in, open one recipe, and export one version. Check its hash as `api.md` section 6 describes.
23. Run the four commands of the list below, while the new `app` serves.
24. Run the [Recovery drill](#recovery-drill) after the four commands pass (ADR-0106 (o)).

Manifest key file before step 1 (ADR-0174):
The new Compose file mounts the secret `backup_manifest_key` in `backup`, `backup-premigrate`, and `restore-volume`. A release before ADR-0174 has no such file.
Run steps 2, 7, 9, and 11 to 13 of the section "Create the secret files" for `backup_manifest_key` only. The steps 2 and 13 name the value and the offline copy.
Never replace an existing file. A new key makes a restore refuse each set of the old key (ADR-0174 (x)).

Probe of the previous image (ADR-0174 (aa)):
An image has the tag check when it holds `/etc/pgtools/manifest-tag.pl`. Run the probe once, before step 3. Note the result.
The command prints `check` or `no-check`. It uses no key and no mount:

```bash
docker run --rm --network none --entrypoint sh recipe-pgtools:<previous tag> -c 'if test -f /etc/pgtools/manifest-tag.pl; then printf "check\n"; else printf "no-check\n"; fi'
```

When the result is `no-check`, the previous image never gets the key.
Each Compose command that runs `backup`, `backup-premigrate`, or `restore-volume` with that image adds the file `compose.previous.yaml`.
After a failed upgrade and in a rollback, `COMPOSE_FILE` in `.env` names `compose.yaml` and `compose.previous.yaml` while `.env` holds that tag.
The file `compose.previous.yaml` also turns off the weekly `restore-test` of the previous `backup` until the next upgrade.
When the result is `check`, no step adds the file, and the previous image gets the key.

Key file of step 6 (ADR-0154):
The new `app` refuses to start when the file `plc_credential_key` is missing or has a wrong form.
A release before this change has no such file. Run the steps before step 8, so `docker compose up` finds the file.
Never replace an existing file. A new key makes every stored PLC password unreadable.
The first start of the new `app` finds no row of `security.plc_credential_key_file` in `app_setting`. It writes one entry `SettingChanged` with the old value `null` and the path as the new value (ADR-0155).

1. Create the file when it does not exist:

```bash
[ -e "$RECIPE_SECRETS_DIR/plc_credential_key" ] || ( umask 077 && openssl rand -hex 32 > "$RECIPE_SECRETS_DIR/plc_credential_key" )
```

2. Set the owner and the mode of the file:

```bash
chown 65532 "$RECIPE_SECRETS_DIR/plc_credential_key"
chmod 0400 "$RECIPE_SECRETS_DIR/plc_credential_key"
```

3. Read the size of the file. Check that the output is `64` or `65`. The value `65` includes the final line feed:

```bash
stat -c '%s' "$RECIPE_SECRETS_DIR/plc_credential_key"
```

4. Count the lines that hold 64 lower-case hex characters. Check that the output is `1`:

```bash
grep -c -E '^[0-9a-f]{64}$' "$RECIPE_SECRETS_DIR/plc_credential_key"
```

5. Read the owner and the mode. Check that the output is `400 65532`:

```bash
stat -c '%a %u' "$RECIPE_SECRETS_DIR/plc_credential_key"
```

6. Stop the upgrade when a check fails. Do not run `docker compose up` before the file passes.

Installation of step 2 (ADR-0105 (e), (i)):
Each release has its own directory `deploy/`. Keep the directory of the running release unchanged. A major upgrade needs it.
The `compose.yaml` of 0.1.0 ignores `RECIPE_PGDATA_VOLUME`. A name check on that file proves nothing, so install the new files first.

1. Unpack the directory `deploy/` of the new release next to the directory of the running release.
2. Copy `.env` of the running release into the new directory.
3. Change into the new directory. Run every later command of this section there.

Name check of step 2 (ADR-0105 (j)):
The release needs the value `RECIPE_PGDATA_VOLUME` in `.env`. Without it, every Compose command stops.
A wrong name of an existing empty volume gives `db` a new empty cluster. The name check stops the upgrade before that.

1. Read the volume of the running `db`. The command does not use Compose:

```bash
docker inspect --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Name}}{{end}}{{end}}' "$(docker ps -q --filter label=com.docker.compose.project=recipe-editor --filter label=com.docker.compose.service=db)"
```

2. Write the printed name as the value of `RECIPE_PGDATA_VOLUME` in `.env`. The name is normally `recipe-editor_pgdata`.
3. Load the values into the shell again. Do not export them:

```bash
. ./.env
```

4. Print the fields `name` and `external` of `pgdata` in the new stack:

```bash
docker compose config | awk '/^volumes:/{v=1;next} /^[^ ]/{v=0} v&&/^  pgdata:/{p=1;next} v&&/^  [^ ]/{p=0} p&&/^    (name|external):/{print $1, $2}'
```

5. Read the output of step 4. Stop the upgrade and report the output when one of these cases occurs:
   1. The output holds no line `external: true`.
   2. The value of `name:` differs from `$RECIPE_PGDATA_VOLUME`.
   3. The value of `name:` differs from the name of step 1.
6. After a stop, run no Compose command that starts or replaces a container.

Case 1 shows the files of 0.1.0 or another `compose.yaml` without an external `pgdata`. Run the installation again.

Segment check of step 2 (ADR-0104, ADR-0105 (h), (n)):
The new `walarchive` refuses to start when `/wal` holds a segment and the file `/wal/.system-identifier` is missing.
An installation of 0.1.0 is in this state.

Run the check once in step 2, after the name check.
The check must run before step 8 replaces `db`. In step 2, `.env` still holds the previous tag.
The check runs in a new container of the previous image.

Never use `docker compose exec` into the old `walarchive` container.
The old `walarchive` of 0.1.0 can restart in a loop after a restart of `db` or of the host. The check does not need that container.
The check runs only when the file is missing.
Run all steps in one shell, because step 2 uses the variable `step`.

1. Store the step in a shell variable:

```bash
step=$(cat <<'STEP'
set -euo pipefail
umask 077
export LC_ALL=C
dir=${WAL_DIR:-/wal}
file=$dir/.system-identifier
IFS= read -r id
[[ $id =~ ^[0-9]+$ ]] || { printf 'ERROR: the system identifier of db is not a number.\n' >&2; exit 1; }
if [[ -e $file || -L $file ]]; then
  printf 'The segment check does not run. The file .system-identifier exists.\n'
  exit 0
fi
# A complete segment has 24 upper-case hex characters and no suffix. LC_ALL=C sorts the names by age.
segment=$(ls -1 -- "$dir" | { grep -E '^[0-9A-F]{24}$' || true; } | sort | tail -n 1)
# Without a complete segment, the step reads the newest .partial file (ADR-0105 (h)).
if [[ -z $segment ]]; then
  segment=$(ls -1 -- "$dir" | { grep -E '^[0-9A-F]{24}\.partial$' || true; } | sort | tail -n 1)
fi
[[ -n $segment ]] || { printf 'ERROR: the WAL archive holds no complete segment and no .partial file.\n' >&2; exit 1; }
# The long header of the first page holds xlp_info at offset 2 and xlp_sysid at offset 24 (little-endian).
read -r info < <(od -An -t u2 --endian=little -j 2 -N 2 -- "$dir/$segment")
read -r sysid < <(od -An -t u8 --endian=little -j 24 -N 8 -- "$dir/$segment")
[[ $info =~ ^[0-9]+$ && $sysid =~ ^[0-9]+$ ]] \
  || { printf 'ERROR: cannot read the header of the WAL segment %s.\n' "$segment" >&2; exit 1; }
(( info & 2 )) || { printf 'ERROR: the first page of the WAL segment %s has no long header.\n' "$segment" >&2; exit 1; }
[[ $sysid == "$id" ]] \
  || { printf 'ERROR: the header of the WAL segment %s holds another system identifier.\n' "$segment" >&2; exit 1; }
printf 'The segment check passed. The segment %s holds the system identifier of db.\n' "$segment"
temp=$(mktemp "$dir/.system-identifier.tmp.XXXXXX")
cannot_publish() { rm -f -- "$temp"; printf 'ERROR: cannot publish the file .system-identifier.\n' >&2; exit 1; }
printf '%s\n' "$id" >"$temp"
sync -- "$temp" || cannot_publish
ln -T -- "$temp" "$file" || cannot_publish
rm -f -- "$temp"
sync -- "$dir" || { printf 'ERROR: the file .system-identifier exists, but the sync of the directory failed.\n' >&2; exit 1; }
STEP
)
```

2. Read the system identifier in `db` as `postgres`. Pipe it into a new container of the previous image:

```bash
docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT system_identifier FROM pg_control_system()" \
  | docker compose run --rm --no-deps -T --entrypoint sh walarchive -c 'exec bash -c "$1"' sh "$step"
```

3. Read the output of step 2. It has one of these lines:
   - `The segment check passed.` The step wrote the file.
   - `The segment check does not run.` The file exists, so the step wrote nothing.
   - An `ERROR` line. Stop the upgrade here. The step wrote no file, except after the error of the directory sync.
4. Check that the file holds the identifier of `db`. Check that both commands print the same number:

```bash
cat "$RECIPE_WALARCHIVE_DIR/.system-identifier"
docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT system_identifier FROM pg_control_system()"
```

The step compares the identifier of `db` with `xlp_sysid` of the newest complete segment in `/wal`.
`xlp_sysid` is 8 bytes at offset 24 of the first page, little-endian.
The step therefore checks the value and does not trust the volume name.
Without a complete segment, the step reads the first page of the newest `.partial` file (ADR-0105 (h)).
That page has the long page header, because `pg_receivewal` writes each segment from its start.

If the step finds no complete segment and no `.partial` file, run `SELECT pg_switch_wal()` in `db` as `postgres`.
Wait for a complete segment in `RECIPE_WALARCHIVE_DIR`. Then run step 2 again.
A new complete segment arrives only while the old `walarchive` streams.

The restore test of step 3 checks the newest complete set of the old schema with the old image (ADR-0083).
It runs in a new container, because a `backup` container of 0.1.0 can restart in a loop (ADR-0105 (g)).
The rollback set of step 8 holds the old guard function. The old image therefore accepts it after a rollback restore (ADR-0097 (h1)).

Only the superuser can replace the guard function (ADR-0069 (p)). The release notes name steps 13 to 17 as a "manual superuser step".
Run no `backup-now` before step 17 passes.
The guard step is the one place that replaces the guard function. The section "An older volume" uses this place too.

Never run two `app` containers at the same time (ADR-0096).
The old `app` has no guard lock of the last Administrator, so the two versions do not exclude each other.
Step 18 replaces the one container of `app`.

The old `app` of step 18 keeps running on the old schema only when the migration follows the expand pattern.
A release with two or more migration files can fail after the first file. Then no image matches the schema.
Choose one of two actions (ADR-0083). The first is a forward fix. The second is a restore of the `_premigrate` dump with the second list of the section "Rollback".

Recovery after a failure between two steps (ADR-0097 (h2)):

- Step 8 or 9 fails: nothing changed. Set the previous tag in `.env` and stop. If the probe printed `no-check`, also do parts 2 and 3 of step 12.
- Step 11 fails: follow step 12 and ADR-0083. The guard function stays old. For a `_premigrate` restore, use step 1 of the section "Rollback".
- A step from 13 to 17 fails: the schema is new and the guard function is old. Run steps 13 to 17 again. Run no `backup-now` until step 17 passes.
- A step of the segment check fails: nothing changed, and the step wrote no file. Read the `ERROR` line. Stop the upgrade, except for the case without a complete segment and without a `.partial` file. The error `the file .system-identifier exists, but the sync of the directory failed` is the exception. The file exists, so a second run prints `The segment check does not run.` Check the disk of the WAL archive before the next step.
- A command of the list of step 23 fails: keep the new `app` and every set. Read the `ERROR` line (ADR-0083 (f)).

The four commands give the new image of `recipe-pgtools` to `backup` and `walarchive`:

1. Run `docker compose up -d --no-deps backup walarchive`. This command gives `backup` the new image and the key.
2. Run `docker compose exec backup backup-now`.
3. Run `docker compose exec backup restore-test`. Check that the test passes and names the `_now` set.
   - If the test names a `_premigrate` set of the same UTC minute, rerun commands 2 and 3 later.
4. Run `docker compose exec walarchive basebackup-once`.

Command 1 can stop a job that runs at `backup.time` or `basebackup.time`. Step 2 avoids these times.

In the first upgrade with ADR-0151, the alerts `restore_test` and `base_backup` can be active from step 18 until step 23 ends (ADR-0151 (n)). The release notes name them.

If the restore test of command 3 fails, a rollback does not start by itself (ADR-0083 (f)).
Keep the new `app`. Keep every set. Read the `ERROR` line in the output of `restore-test`.

After the first upgrade to a release with ADR-0123, run the check of the section [Check the log of db for a password](#check-the-log-of-db-for-a-password) once.
A data volume created before ADR-0123 can hold a role password in the log of db.

### Rollback

Check first that the images of the previous tag exist (section "Keep the images for a rollback").

1. Run the command below to start the previous `app` on the newer schema:

```bash
RECIPE_VERSION=<previous tag> docker compose up -d --no-deps app
```

2. Keep the new tag in `.env`. The schema stays newer.
3. Run the four commands of the upgrade with the tag of `.env`.

Warning: `.env` holds the new tag, so a later `docker compose up -d` gives `app` the new image again (ADR-0083).

After a rollback with the expand pattern, the previous release cannot create an S7 machine.
Migration `0013` adds a check that refuses its `INSERT` with `E-DB-003`. No wrong row appears (ADR-0156).
The previous release can still read and save the PLC connection of an existing machine.
A trigger of migration `0014` writes each data block number that the previous release saves into the machine (ADR-0157).
The machine number therefore stays equal to the number of the PLC connection.

If the migration did not follow the expand pattern, use the `_premigrate` dump.
Warning: this restore loses every change and audit entry after the `_premigrate` dump.
The lost period runs from the `_premigrate` dump to the stop of `app`.
No test checks the `_premigrate` set. The test before the upgrade checks a set of the same schema (ADR-0083).

The restore goes into a new `db` volume and never into the volume of the running `db` (ADR-0103 (a), (q)).
It runs `restore-volume` with the previous tag, which is the tag of the schema of the set.
A rollback restore to the schema of 0.1.0 has no procedure, because the image of 0.1.0 has no `restore-volume` (ADR-0103 (q)).

Use these placeholders in addition to the placeholders of [Restore into production](#restore-into-production):

- `<tag>` is the tag that the old volume runs. It is the value of `RECIPE_VERSION` in `.env` now.
  In the plain rollback, `<tag>` is the new tag. After a failed upgrade, `<tag>` is the previous tag.
- `<new tag>` is the new release tag that step 1 of the section "Upgrade" notes. Only command 1 of step 1 uses it. In the plain rollback, it equals `<tag>`.
- `<stamp>` is the name of the `_premigrate` set, for example `<YYYYMMDD_HHMM>_premigrate`.

1. Delete the container of `backup`. Run the check of the set below, which is command 1. Only after it passes, delete `app`.
   Then create the new volume. Run step 3 of "Restore into production".
   Run no step 2 of that list, because the commands below already delete `backup` and `app`.
   Skip step 1 of that list. Steps 1 to 8 of `restore-volume` test the `_premigrate` set (ADR-0103 (b)).
   For a failed upgrade, see [Failed rollback restore after a failed upgrade](#failed-rollback-restore-after-a-failed-upgrade).
   Optional: first copy the log of the container of `backup` with the next block, because `rm` deletes it.

```bash
docker compose logs --no-color backup > <file>
```

   The block of command 1 first deletes the container of `backup`. Then it runs command 1.

```bash
docker compose rm -s -f backup
env -u COMPOSE_FILE RECIPE_VERSION=<new tag> docker compose -f compose.yaml run --rm restore-volume --check-only <stamp>
```

   Run the next block only after command 1 exits with the code 0.

```bash
docker compose rm -s -f app
```

   Command 1 checks the tag of the set with the new image. This step applies to both states of `.env`.
   In a plain rollback, `.env` holds the new tag. After a failed upgrade, `.env` holds the previous tag and `COMPOSE_FILE`.
   The option `-f` and `env -u COMPOSE_FILE` ignore each `COMPOSE_FILE`. The variable `RECIPE_VERSION` selects the new image in both states.
   Without these parts, the second state runs the check with the previous image. That image stops with `usage: restore-volume <stamp>`. This stop looks like a failed check.
   The order of the parts matters. The new image with the file `compose.previous.yaml` has no key, so its check stops. The previous image has no mode `--check-only`.
   Delete the container of `backup` before command 1. A weekly `restore-test` of a running `backup` can hold the lock of `restore-volume`.
   Command 1 or the fill of step 3 then stops with a fault that the set does not cause.
   `backup` has `restart: always`, so a reboot or a restart of the container engine starts a stopped container again.
   Delete `app` only after command 1 passes. A failed check then leaves the running `app` unchanged.
   The reason of the delete is the later start of `app`. `app` has `restart: always`, so a stopped container starts again after a reboot or a restart of the container engine.
   Its writes then make the lost period of the rollback longer, and no message shows them. After a failed upgrade, the previous `app` also starts on the schema of the failed `migrate`.
   The delete applies to both states of `.env`. In a plain rollback, the later start of `app` creates the container again.
   A failed command 1 stops the rollback. In a plain rollback, `.env` still holds the new tag.
   Then run [Leave the file compose.previous.yaml](#leave-the-file-composepreviousyaml) with `<write tag>` as the new release tag.
   After that, start `app` and `backup`. Create `backup` again as step 4 of "Restore into production" says.
   After a failed upgrade, a failed command 1 keeps the previous tag and `COMPOSE_FILE` in `.env`. No step then starts a container of the new image.
   Start only `backup` again with the first block of the section linked above. `app` keeps running.

2. Set `RECIPE_VERSION` in `.env` to the previous tag. Do this only after command 1 of step 1 passes.
   If the probe of the previous image printed `no-check`, add the line `COMPOSE_FILE=compose.yaml:compose.previous.yaml` to `.env`. Delete each older line `COMPOSE_FILE`.
   Run the probe again when you did not note its result. See the section "Upgrade".
   Load the values into the shell again with `. ./.env`.
   `walarchive` and `backup` then start with the previous image (ADR-0103 (q)).
3. Fill the new volume. Run step 4 of "Restore into production" with `<stamp>` of the `_premigrate` set.
   If step 4 fails, end the attempt as step 4 says. `.env` then holds `<tag>` again, because the old volume holds the newer schema (ADR-0105 (c)).
   For a plain rollback, run [Leave the file compose.previous.yaml](#leave-the-file-composepreviousyaml) with `<write tag>` as `<tag>`, the new release tag.
   Run it before you start `app` and `backup`.
   For a failed upgrade, `.env` keeps the previous tag and `COMPOSE_FILE`. Start only `backup`. See [Failed rollback restore after a failed upgrade](#failed-rollback-restore-after-a-failed-upgrade).
4. Run the switch. Run step 5 of "Restore into production". If its sub-step 7 fails, run the [Switch back](#switch-back) at once.
5. Set the role passwords. Run step 6 of "Restore into production".
6. Start the new WAL line. Run step 7 of "Restore into production". `walarchive` starts with the previous tag, before `app` (ADR-0103 (q), (s)).
7. Truncate `session` and `login_attempt`, and re-apply the user changes. Run step 8 of "Restore into production".
8. Record the values of sub-steps 1 to 3 of step 9 of "Restore into production". Then start `app`, and then `proxy` and `backup`:

```bash
docker compose up -d --no-deps app
docker compose up -d --no-deps proxy backup
```

9. Run sub-steps 6 and 7 of step 9 of "Restore into production". Then run step 10, the check of the audit chain.

`.env` holds the previous tag, so the first command starts the previous image of `app`. Do not use "Start app after a rollback" here.
A later upgrade sets a new tag in `.env`.

Warning: never use the `compose.yaml` of 0.1.0 after a switch.
That file starts `db` on the old cluster and serves old data without an error.
After the delete of the old volume, that file makes an empty cluster.
The `walarchive` of 0.1.0 has no guard and streams another system into `/wal`.

#### Start app after a rollback

After a rollback with the expand pattern, `.env` keeps the new tag. A later command can then start `app` with the new image (ADR-0083).
Every command that starts or recreates `app` therefore passes the previous tag in the same way as step 1 of the rollback.
Put `RECIPE_VERSION=<previous tag>` before `docker compose`. Keep the other options of the command, for example `--force-recreate`.
This holds until the section "Upgrade" sets a new tag.

#### Leave the file compose.previous.yaml

Use this section when an edit of `.env` writes a tag whose image has the tag check (ADR-0174 (aj)).
The function writes `RECIPE_VERSION=<write tag>` and deletes each line `COMPOSE_FILE` from `.env`. Then it runs `unset COMPOSE_FILE`.
A `COMPOSE_FILE` in the shell takes precedence over `.env`, so the function clears it too.
Each place below names its value of `<write tag>`:

1. Step 5 of "Upgrade" and step 7 of "Major upgrade". `<write tag>` is the new release tag.
   Step 6 of "Load the images" also writes that tag, but a first installation has no `COMPOSE_FILE`, so it needs no exit.
   These places do not create `backup`. The four commands of "Upgrade" give `backup` the new image and the key.
2. Step 3 of "Rollback" and step 4 of "Restore into production", in the plain rollback only. `<write tag>` is the new release tag, which equals `<tag>` there.
   Only these places then create `backup` again. The command is in step 4 of "Restore into production".
3. Step 3 of "Switch back". Use it only when the probe of ADR-0174 (aa) finds the tag check in the image of `<tag>`. Else `COMPOSE_FILE` stays.
   `<write tag>` is the `<tag>` of that section, the tag of the old volume. It always writes that tag.
   After the plain rollback, that tag is the new release tag. Only after a failed upgrade is it the previous tag.
   Then this place must not write the new release tag, because the new `backup` would start on the schema of the failed `migrate`.
   This place does not create `backup`. Step 1 of "Switch back" deletes the containers, and step 8 creates `backup` with the key.

After the exit, the configuration must show `backup_manifest_key` on `backup`, `backup-premigrate`, and `restore-volume`. Print it with `docker compose --profile restore config`.

The other edits that write a previous tag need no exit. Step 12 of "Upgrade", step 2 of "Rollback", and the fault list of "Upgrade" write the previous tag.
They set `COMPOSE_FILE` when the probe finds no tag check.

The function runs `perl` on the Docker host. Run `perl -v` once before the upgrade. Install Perl from the distribution if the command fails.
In `deploy/`, paste the block, then run `leave_previous_overlay . <write tag>`.

```bash
leave_previous_overlay() {
  local directory=$1 write_tag=$2 temp=''
  if ! {
    temp=$(mktemp "$directory/.env.XXXXXX") &&
    perl -e 'my $tag = shift @ARGV;
      open my $env, "<", shift @ARGV or exit 1;
      $_ = "";
      while (1) {
        my $count = read $env, my $chunk, 65536;
        defined $count or exit 1;
        last if $count == 0;
        $_ .= $chunk;
      }
      close $env or exit 1;
      if (length $_) {
        s/^(?:COMPOSE_FILE|RECIPE_VERSION)=[^\n]*(?:\n|\z)//mg;
        print "RECIPE_VERSION=$tag\n$_" or exit 1;
      }
      close(STDOUT) or exit 1;' "$write_tag" "$directory/.env" >"$temp" &&
    chown --reference="$directory/.env" -- "$temp" &&
    chmod --reference="$directory/.env" -- "$temp" &&
    mv -f -- "$temp" "$directory/.env"
  }
  then
    rm -f -- "$temp" || printf 'ERROR: cannot delete the environment temp file.\n' >&2
    printf 'ERROR: cannot leave the previous Compose overlay.\n' >&2
    return 1
  fi
  unset COMPOSE_FILE
}
```

After an interrupt, delete a left file `.env.??????` in `deploy/`. `.env` stays unchanged, because `mv` is atomic.
The function copies the owner of `.env` with `chown --reference`. An operator without root cannot do this when another uid owns `.env`.
An example is `.env` that belongs to root after an edit with `sudo`.
Then the function keeps `.env` and returns a code other than 0.
Give `.env` back to the operator, for example with `sudo chown`. Then run the function again.

#### Failed rollback restore after a failed upgrade

Use this section after a failed upgrade, when the `_premigrate` restore of the section "Rollback" fails.
`.env` then holds the previous tag and `COMPOSE_FILE`. No step starts a container of the new image.
Only `backup` starts again. `app` stays stopped until the owner decides the next action.

After a failed check, `app` keeps running, and the first block applies.
After a failed full restore, `app` stays stopped, and the second block applies.
After a switch back, `app` stays stopped, and the first block applies. Step 1 of "Switch back" already deleted the container of `app`.

After a failed check, or at step 8 of "Switch back", start only `backup`:

```bash
docker compose up -d --no-deps backup
```

After a failed `restore-volume <stamp>`, delete the container of `app` before the start of `backup`:

```bash
docker compose rm -s -f app
docker compose up -d --no-deps backup
```

Step 1 of "Rollback" already deleted the container of `app`. The `rm` line of the second block repeats that delete and changes nothing then.
The line keeps this state free of a link to an earlier step.
The reason of the delete is the start after a reboot. A stopped `app` with `restart: always` starts again after a reboot (section "After a host reboot").

`app` stays stopped until a forward correction of ADR-0083, a new restore, or a restore with `pitr-volume` (ADR-0106).
The operator starts `app` earlier only after the operator checks that the schema of the old volume works with the previous image.
An example is a migration of the expand pattern. The owner chose this rule for three reasons (ADR-0174 (aj)):

1. The old volume holds the schema of the failed `migrate`. That schema can match no image (ADR-0083).
2. Each write after a restart of `app` makes the lost period longer.
3. The project gives data safety priority over availability.

Start `app` with the section "Stop, start, and restart" after that confirmation.
After that start, wait until the alert `replication` is inactive (step 14 of "A gap in the WAL stream").

While `app` stays stopped, no job `alerts` runs. Run two checks once a day, after 04:00 UTC.
The time is `backup.time` plus `backup.max_age_alert` minus 24 h (`reliability.md` §7.11). The daily run then has the margin of the alert `backup_age`.

1. Run the stream check of step 9 of "A gap in the WAL stream". It passes on one row with the state `streaming` and a `flush_lsn`.
2. Run the dump check below. It passes when it prints a name and exits with the code 0.

For the daily dump check, list the complete daily sets of the current UTC day:

```bash
docker compose exec -T -e LC_ALL=C backup sh -c 'ls /backups/recipe_$(date -u +%Y%m%d)_[0-9][0-9][0-9][0-9].dump.sha256'
```

The option `-e LC_ALL=C` sets the locale of the match (ADR-0074 (c)).
The glob `[0-9][0-9][0-9][0-9]` matches only the four digits of the time of a daily set.
A `_premigrate` set, a `_now` set, or a foreign name such as `recipe_<day>_abcd.dump.sha256` does not count.
A foreign name would hide a missing set of the day and give a false pass.
The check reads the stamp and not the age. `backup-health` takes the age from the modification time of the manifest, so an age check hides a failed dump.
The stream check does not compare the lag with `wal.max_lag_bytes`. The wait for the alert `replication` after the start of `app` does.

### Switch back

The switch back returns `db` to the old volume (ADR-0103 (g)). It uses the steps of the switch with the old name.
Use it when sub-step 7 of step 5 of "Restore into production" fails. Use it also to return to the old data.

Warning: the switch back loses each write after the switch. The old volume does not change.
Delete no volume. After a major upgrade, run the switch back in the directory `deploy/` of the old release, with its images (ADR-0105 (e)).

Use these placeholders in addition to the placeholders of "Restore into production":

- `<old volume>` and `<old identifier>` are the name and the identifier that the first switch wrote into the change log.
- `<old timeline>` is the number after `Latest checkpoint's TimeLineID` in the same entry. An entry from before ADR-0106 holds no timeline (ADR-0106 (l)).
- `<left volume>` is the volume that `db` mounts before step 1. It is the value of `RECIPE_PGDATA_VOLUME` in the `.env` of the release that runs now. `<left identifier>` is its `system_identifier`.
- `<left timeline>` is the number after `Latest checkpoint's TimeLineID` in the output of step 2. If the output holds none, it is the word `unreadable`.
- `<tag>` is the tag of the image of the cluster in `<old volume>`. The change log holds it next to `<old volume>` (ADR-0105 (b)). Use the `recipe-pgtools` image of that tag.
- `<left tag>` is the tag of the cluster in `<left volume>`. It is the value of `RECIPE_VERSION` in `.env` before step 3. After a major upgrade, it is `<new tag>`.

1. Stop the five containers and delete them. Run sub-step 1 of step 5 of "Restore into production". Never run `docker compose down`.
2. Read the identifier of the volume that you leave. The state can differ from `shut down`. Never start `db` on this volume:

```bash
docker run --rm --network none --security-opt no-new-privileges:true -e LC_ALL=C -v "<left volume>:/var/lib/postgresql/data:ro" --entrypoint pg_controldata "recipe-pgtools:<left tag>" /var/lib/postgresql/data
```

   Write `<left volume>` and `<left tag>` into the change log. Write the lines `Database system identifier` and `Latest checkpoint's TimeLineID` too, if the output holds them.
   Their values are `<left identifier>` and `<left timeline>`.
   If the output holds no timeline, write the word `unreadable` in place of the timeline (ADR-0106 (j)).
3. Write `<old volume>` as `RECIPE_PGDATA_VOLUME` and `<tag>` as `RECIPE_VERSION` in `.env` (ADR-0105 (d)).
   Run the block "Probe of the previous image (ADR-0174 (aa))" with the image `recipe-pgtools:<tag>` and use its result, `check` or `no-check`.
   If the probe of ADR-0174 (aa) finds the tag check in the image of `<tag>`, run [Leave the file compose.previous.yaml](#leave-the-file-composepreviousyaml) with `<write tag>` as `<tag>`.
   Else keep `COMPOSE_FILE`. It must name `compose.yaml` and `compose.previous.yaml` while `.env` holds a tag without the tag check.
   This tag belongs to the old volume. After a plain rollback, it is the new release tag.
   Only after a failed upgrade is it the previous tag. Each previous image in a later release has the tag check.
   Step 8 with the new release tag would start the new `backup` on the schema of the failed `migrate`.
   ADR-0174 (aj) forbids that start.
   Then run sub-steps 5 and 6 of step 5 of "Restore into production".
   The name that sub-step 6 prints must equal `<old volume>`.
4. Run the command of sub-step 2 of step 5 of "Restore into production" with `<old volume>`.
   The state must be `shut down`. The identifier must equal `<old identifier>`.
   The timeline must equal `<old timeline>`. If the entry holds no timeline, compare the identifier only (ADR-0106 (l)).
5. If the check of step 4 fails, stop. Start no `db` and delete no volume. Keep both volumes and the entry of the change log.
   The site stays down until you find the cause.
6. Start `db`. Run sub-step 8 of step 5 of "Restore into production" with `<old volume>`.
7. Start `walarchive` before `app`. Choose one of two cases:
   1. If sub-step 4 of step 7 of "Restore into production" did not run, `/wal` still holds the old cluster line. Run this command, then step 9 of "A gap in the WAL stream":

```bash
docker compose up -d --no-deps walarchive
```

   2. If that sub-step ran, start the new WAL line again. Run step 7 of "Restore into production".
      Use `<left identifier>` in place of `<old identifier>`, and `<left timeline>` in place of `<old timeline>`.
      If the change log holds no timeline for `<left volume>`, or holds the word `unreadable`, write the first line only in sub-step 3 (ADR-0106 (l)).
8. Start `app`, and then `proxy` and `backup` (ADR-0103 (g)):
   After a failed upgrade, start only `backup`. `app` and `proxy` stay stopped.
   See [Failed rollback restore after a failed upgrade](#failed-rollback-restore-after-a-failed-upgrade).

```bash
docker compose up -d --no-deps app
docker compose up -d --no-deps proxy backup
```

9. Run sub-step 6 of step 9 of "Restore into production". Then wait until the alert `replication` is inactive.
   After a failed upgrade, step 1 deleted the container of `app`, and the job `alerts` runs in `app`.
   Sub-step 6 then inspects the service `backup` only.
   Do not wait for the alert. Run the stream check of step 9 of "A gap in the WAL stream" instead.
   That check is a query in `db` and needs no `app`. It passes on one row with the state `streaming` and a `flush_lsn`.
   `app` can stay stopped for a long time, and `pitr-volume` needs the WAL archive. A wait at the later start of `app` leaves the stream unchecked until then.
   The later start of `app` waits for the alert `replication`, as the section linked in step 8 says.

### Keep the images for a rollback

A plant host can have no registry. A release moves the images as files of `docker save` (ADR-0064 (j)).
A rollback starts the previous image of `app` (`reliability.md` 7.10 step 6).
After a restore of the `_premigrate` set, the four commands of the upgrade also start the previous image of `recipe-pgtools` (ADR-0083 (d)).
Without the image, Compose tries to pull it, and the rollback stops.
Keep these images on the host:

- `recipe-server` and `recipe-pgtools` with the tag of the running release
- `recipe-server` and `recipe-pgtools` with the previous tag

No document says how long a release stays rollback-ready. Delete the images of a release only when you decide that you will not roll back to it.
Keep the image files and the release notes of that release (section "Copy the backups off site").
You can then load the images again with the section "Load the images".
Docker stores the images in its root directory. That is the disk of `pgdata` (section "Low disk space").

Check the images after each upgrade:

1. Run `docker image ls recipe-server`. Run the same command for `recipe-pgtools`.
2. Check that each list shows the tag of the running release and the previous tag.
3. Run `docker image inspect --format '{{.Id}}' recipe-server:<previous tag>`. Run the same command for `recipe-pgtools:<previous tag>`.
4. Compare each image ID with the release notes of the previous release.
5. If an image is missing, load it with steps 1 to 5 of the section "Load the images".
6. After such a load, do not run steps 6 and 7 of that section.

To free disk space, run the command below. It deletes only dangling images. A dangling image has no tag:

```bash
docker image prune
```

Docker asks for confirmation. A `docker load` of a tag that already exists can leave the old image without a tag.
The command keeps every image that has a tag. It therefore keeps the images of the previous release.
To delete the images of an old release, read the section "Destructive commands" first.

### Concurrent index on an audit object

This procedure is not part of the normal upgrade. Run it only when a release needs a concurrent index on an audit object.
A migration cannot create this index. `CREATE INDEX CONCURRENTLY` must be the only statement of its file (`data-model.md` 5.1 and 8).

The role `recipe_migrator` runs five statements in one `psql` session (`reliability.md` 7.10 step 7).
The client rules let this role connect from the Compose subnet only. No document gives a command that opens a session as this role.

1. Open one `psql` session as `recipe_migrator` on the database `recipe`.
2. Run the five statements in this order:

```sql
GRANT CREATE ON SCHEMA public TO audit_owner;
SET ROLE audit_owner;
CREATE INDEX CONCURRENTLY <index_name> ON <audit_table> (<column>);
RESET ROLE;
REVOKE CREATE ON SCHEMA public FROM audit_owner;
```

3. Run all five statements, also when the index statement fails. The role `audit_owner` must not keep `CREATE` on `public`.
4. Leave `psql` with `\q`.

The next migration records the same index with `CREATE INDEX IF NOT EXISTS` (`data-model.md` 5.1). That migration needs an entry in the list `EXEMPT` of `deploy/ci/migration_rewrite.py` with the ADR that adds the index (ADR-0136).

## Major upgrade

A major upgrade of PostgreSQL is a dump and a restore into a new `db` volume.
The new volume uses the same `initdb` arguments as `compose.yaml`. The project does not use `pg_upgrade` (ADR-0020).

1. Delete the container of `app`. The next dump then holds every change.
   A restart of Docker or of the host starts a stopped container with `restart: always` again, on the old volume.
   Optional: first copy the log of the container with `docker compose logs --no-color app > <file>`, because `rm` deletes it.
   Then delete the container:

```bash
docker compose rm -s -f app
```

2. Run `docker compose exec backup backup-now`. The new `_now` set holds the globals file and the dump (`reliability.md` 7.7).
3. Run `docker compose exec backup restore-test`. Continue only after it passes.
4. Check that the line `restore test passed` names the `_now` set of step 2.

The test comes before the switch (`reliability.md` 7.10 step 8).
The command `restore-volume` of the new release then fills a new volume with the `_now` set.
The switch makes that volume the volume of `db` (ADR-0103 (b), (p)).
The release that changes the major version holds no migration file (ADR-0103 (p)). Do not run `backup-premigrate` or `migrate`.

Use the placeholders of [Restore into production](#restore-into-production) and these two:

- `<old tag>` is the value of `RECIPE_VERSION` in `.env` before step 7. It is the tag of the old cluster.
- `<new tag>` is the tag of the new release. It is the tag of the cluster in the new volume.

If a step before step 8 fails, change into the directory of `<old tag>`. Then run `docker compose up -d --no-deps app`, so that `app` serves again on the old volume.

5. Write `<old tag>` into the change log of the site. Step 5 of "Restore into production" adds the lines `Database system identifier` and `Latest checkpoint's TimeLineID` of the old cluster to that entry. Keep the directory `deploy/` of `<old tag>` and the images of `<old tag>` (ADR-0105 (e)).
   The restart of sub-step 3 of step 5 of "Restore into production" and the switch back run in that directory.
6. Do the installation and the name check of step 2 of "Upgrade" (ADR-0105 (i), (j)). A refusal stops the procedure.
7. Load the new images as in "Load the images". Do not stop `db`. Step 6 of that section writes `<new tag>` into `.env`.
   Then run [Leave the file compose.previous.yaml](#leave-the-file-composepreviousyaml) with `<write tag>` as `<new tag>`, the new release tag.
8. Fill the new volume and run the switch. Run steps 2 to 5 of "Restore into production".
   Skip step 1 of that list, because steps 3 and 4 above are the test. Use the `_now` set of step 2 as `<stamp>`.
   - Step 4 runs `restore-volume` with the `recipe-pgtools` image of `<new tag>`. That image builds on the `db` image of the new major version (ADR-0103 (p)).
   - In sub-steps 2 and 3 of step 5, `<tag>` is `<old tag>`. In sub-step 7, `<tag>` is `<new tag>`.
   - Run the `up` command of sub-step 3 of step 5 in the directory of `<old tag>` (ADR-0105 (f)).
     After that `up` command, change back into the directory of the new release. Then run sub-step 1 and sub-step 2 again there.
   - If step 4 fails, end the attempt in the directory of `<old tag>` (ADR-0105 (c)).
   - Sub-step 8 of step 5 changes the image and the volume of `db` in one replacement (ADR-0103 (p)).
   - If sub-step 7 fails, run the [Switch back](#switch-back) at once.
9. Set the role passwords. Run step 6 of "Restore into production".
10. Start the new WAL line. Run step 7 of "Restore into production". `walarchive` starts with `<new tag>`, before `app` (ADR-0103 (s)).
    Keep the images of `<old tag>` while the moved files in the target directory exist (ADR-0103 (p)).
11. Truncate `session` and `login_attempt`, and re-apply the user changes. Run step 8 of "Restore into production".
12. Record the values of sub-steps 1 to 3 of step 9 of "Restore into production". Then run its sub-steps 4 to 7.
    `.env` holds `<new tag>`, so `app` starts with the new image. Do not use "Start app after a rollback".
13. Run step 10 of "Restore into production", the check of the audit chain.
14. Check `/readyz`, log in, and export one version. Run steps 19 and 22 of "Upgrade".
15. Run `backup-now` and then `restore-test` on the new cluster. Wait for a UTC minute later than the `_now` set of step 2. The test must pass:

```bash
docker compose exec backup backup-now
docker compose exec backup restore-test
```

The rollback of a major upgrade is the [Switch back](#switch-back) (ADR-0103 (p)).
A dump of the new major version does not restore into the old one. The switch back loses each write after the switch.
The old volume stays unchanged.

1. Change into the directory `deploy/` of `<old tag>` of step 5. Its `.env` holds the old volume and `<old tag>` (ADR-0105 (e)).
2. Run the [Switch back](#switch-back) in that directory. In that section, `<tag>` is `<old tag>`, `<left tag>` is `<new tag>`, and `<left volume>` is the new volume.
   Take the name of the new volume from the `.env` of the directory `deploy/` of the new release.

## Key rotation

The files `login_hmac_key` and `csrf_signing_key` hold one key per line, the newest key first (ADR-0035).
The file `plc_credential_key` is not part of this section. It holds one key and has no rotation (ADR-0154).
The file `backup_manifest_key` has its own subsection, "Rotate the manifest key".

1. Run the command below once for each key file. Use `login_hmac_key` and `csrf_signing_key` for `<name>`.

```bash
( umask 077 && { openssl rand -hex 32; cat "$RECIPE_SECRETS_DIR/<name>"; } > "$RECIPE_SECRETS_DIR/<name>.new" )
```

The command writes the new key as the first line and keeps the old line. It prints no key.

2. Count the keys and the lines of each new file. Both numbers must be 2:

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

4. Run `docker compose up -d --force-recreate --no-deps app`. After a rollback, use "Start app after a rollback".
5. Run `docker compose ps app` until `app` shows `healthy`.
6. Open the login page from a workstation. Log in with your account. Check that the login works.
7. Wait for the value of `security.key_rotation_overlap` (`reliability.md` 7.11). Count the time from step 4.
8. Delete the old line from the key file. This step deletes one key and no data.
9. Run the two commands of step 2 on `"$RECIPE_SECRETS_DIR/<name>"`. Check that both numbers are 1.
10. Run `stat -c '%a %u' "$RECIPE_SECRETS_DIR/<name>"`. Check that the output is `400 65532`.
11. Run `ls -A "$RECIPE_SECRETS_DIR"`. Check that the list holds no editor copy of the key file.
12. Run `docker compose up -d --force-recreate --no-deps app` again. After a rollback, use "Start app after a rollback".
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

### Rotate the manifest key

The file `backup_manifest_key` signs the manifest of each set (ADR-0174).
The file holds at most 2 lines. Each line is one key of 64 lower-case hex characters, and the first line is the newest key (ADR-0174 (n)).
`backup-once` signs with the first line. A restore accepts a tag of either line (ADR-0174 (o)).
This rotation is separate from `security.key_rotation_overlap`. A key line stays while a kept set, a `_premigrate` set, or an off-site copy needs it (ADR-0174 (q)).
The key id of a set is the second field of its tag file `recipe_<stamp>.dump.sha256.hmac`.

No host account and no other container uses uid 999. The host file belongs to uid 999 and has the mode `0400`.
A process with uid 999 or root, or a container with uid 999, is an online writer. An online writer is a known limit of the tag (ADR-0174 (b)).
Use these placeholders:

- `<tag>` is the value of `RECIPE_VERSION` in `.env`. The image `recipe-pgtools:<tag>` must hold `/etc/pgtools/manifest-tag.pl`.
- `<copy>` is the path of the file `backup_manifest_key` on the offline medium.

The command below prints the key id of each line of a key file, in file order. It prints no key:

```bash
docker run --rm --network none --security-opt no-new-privileges:true -v "$RECIPE_SECRETS_DIR/backup_manifest_key:/run/secrets/backup_manifest_key:ro" --entrypoint perl "recipe-pgtools:<tag>" /etc/pgtools/manifest-tag.pl key-ids /run/secrets/backup_manifest_key
```

#### Routine rotation

Use the routine rotation on the schedule of the site.

1. Write the new key as the first line and keep the old key as the second line. The command prints no key:

```bash
( umask 077 && { openssl rand -hex 32; cat "$RECIPE_SECRETS_DIR/backup_manifest_key"; } > "$RECIPE_SECRETS_DIR/backup_manifest_key.new" )
```

2. Run the two commands of step 2 of "Key rotation" on the new file. Check that both numbers are 2.
3. Set the owner and the mode of the new file. Then move it over the old file:

```bash
chown 999 "$RECIPE_SECRETS_DIR/backup_manifest_key.new"
chmod 0400 "$RECIPE_SECRETS_DIR/backup_manifest_key.new"
mv "$RECIPE_SECRETS_DIR/backup_manifest_key.new" "$RECIPE_SECRETS_DIR/backup_manifest_key"
```

4. Run `docker compose up -d --force-recreate --no-deps backup`. Run `docker compose ps backup` until `backup` shows `healthy`.
5. Run the section "Run a backup now". Check that the new set has the new key id as the second field of its tag file.
6. Copy the new key file to the offline medium. Run the section "Compare the offline copy".
7. Copy the secret files again, as the section "Copy the backups off site" says.
8. Delete the old line only when no kept set, no `_premigrate` set, and no off-site copy has its key id. List the key ids of the sets of the host:

```bash
cut -d' ' -f2 "$RECIPE_BACKUP_DIR"/recipe_*.dump.sha256.hmac | sort | uniq -c
```

9. Run the same command in the directory of each off-site copy. Delete the old line only when no line shows the old key id.
10. Delete the old line from the key file. This step deletes one key and no data.
11. Run the two commands of step 2 of "Key rotation" on the file. Check that both numbers are 1.
12. Run `stat -c '%a %u' "$RECIPE_SECRETS_DIR/backup_manifest_key"`. Check that the output is `400 999`.
13. Run `ls -A "$RECIPE_SECRETS_DIR"`. Check that the list holds no editor copy of the key file.
14. Run step 4 again. Copy the new key file to the offline medium and run the section "Compare the offline copy".

A set of the old key that stays on the host or off site needs the old line. Delete the line later, when no such set remains.

#### Compromise rotation

Use the compromise rotation after each incident of the class of ADR-0084.
The SQL of a set of such an incident can read the key (ADR-0174 (q), known limit (ah)).
The new file holds only the new key. A restore then refuses each set of the old key.

1. Write a file with only the new key. The command prints no key:

```bash
( umask 077 && openssl rand -hex 32 > "$RECIPE_SECRETS_DIR/backup_manifest_key.new" )
```

2. Run the two commands of step 2 of "Key rotation" on the new file. Check that both numbers are 1.
3. Run the commands of step 3 of the routine rotation.
4. Run step 4 and step 5 of the routine rotation.
5. Copy the new key file to the offline medium. Run the section "Compare the offline copy".
6. Copy the secret files again, as the section "Copy the backups off site" says.

The sets of the old key stay on the disk but no restore accepts them. Plan a new set for each use of the old sets.

#### Compare the offline copy

Keep one offline copy of the key file. Keep it apart from the backup disk and the off-site copy (ADR-0174 (r)).
Run these steps at each copy and at each restore drill, as in "Run the restore test".

1. Compare the copy with the host file. The command prints nothing when both files are equal:

```bash
cmp -- <copy> "$RECIPE_SECRETS_DIR/backup_manifest_key"
```

2. Print the key ids of the host file with the command at the start of this section.
3. Print the key id of the newest tag file: `cut -d' ' -f2 "$RECIPE_BACKUP_DIR/recipe_<stamp>.dump.sha256.hmac"`.
4. Check that the key id of step 3 equals a key id of step 2.
5. If a step shows a difference, write the host file again to the medium. Then run the steps again.

The copy keeps each key that a kept set or an off-site copy needs.

#### Restore the key file on a new host

Do these steps before the first restore on a new host. A restore refuses each set without a valid tag (ADR-0174 (x)).

1. Create the secret directory as in step 1 of "Create the secret files".
2. Copy the key file from the offline medium. The command writes the mode `0600` first:

```bash
( umask 077 && cat <copy> > "$RECIPE_SECRETS_DIR/backup_manifest_key" )
```

3. Set the owner and the mode, as in step 7 of "Create the secret files":

```bash
chown 999 "$RECIPE_SECRETS_DIR/backup_manifest_key"
chmod 0400 "$RECIPE_SECRETS_DIR/backup_manifest_key"
```

4. Run `stat -c '%a %u' "$RECIPE_SECRETS_DIR/backup_manifest_key"`. Check that the output is `400 999`.
5. Run steps 11 and 12 of "Create the secret files". Check that the key file and the backup disk have two different filesystems (ADR-0174 (p)).
6. Print the key ids of the file with the command at the start of this section.
7. Print the key ids of the sets that you restore, with the command of step 8 of the routine rotation. Check that each of them is in the list of step 6.

The Compose file mounts the file as `/run/secrets/backup_manifest_key` in the services `backup`, `backup-premigrate`, and `restore-volume`.
No other service mounts it. Compose reads the host file when it starts the container (ADR-0174 (m)).

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

1. Do steps 1 and 2 of the section "Stop the whole stack". No `backup` or `walarchive` run must be active.
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
8. Write the start time into the change log ([rule](#change-log-of-the-site)). Set the new password. Psql reads the file inside `db`, so no password reaches a command line:

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

10. Restart the service of the last column of the table:
    1. Run `docker compose up -d --force-recreate --no-deps <service>`.
    2. For `app` after a rollback, use "Start app after a rollback" instead.
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

### Check the log of db for a password

The command prints the role names and no password:

```bash
docker compose logs --no-log-prefix db | grep -B1 -E "PASSWORD +E?'" | grep -o -E 'ROLE (postgres|recipe_[a-z]+)' | sort -u
```

`-B1` covers the statement of `recipe_replicator`, which spans two lines. psql writes `E'...'` for a value with a backslash, so the pattern also finds that form. An empty output means that the log holds no password. The operator rotates each named role with the steps of that section. The operator also checks the copy of the log collector, when the site forwards the log.

## Change a setting

The server `app` reads 18 configuration keys from its environment and derives `alerts.webhook_url` from the webhook file (ADR-0062 (a), ADR-0063 (c), ADR-0154). The table `app_setting` records these 19 keys (ADR-0155). For a key file, the table records the path and never the key.
The file `compose.yaml` holds the environment entries of `app`. An entry has the form `RECIPE_<SECTION>__<NAME>`, with a double underscore between the parts.
For example, the key `approval.separation_of_duties` is the entry `RECIPE_APPROVAL__SEPARATION_OF_DUTIES`.

The file `.env` holds only the nine values of the Compose interpolation. `app` does not read it (ADR-0062 (i), (j)).
The only value of `.env` in the environment of `app` is `PROXY_ADDRESS`, inside the entry `RECIPE_HTTP__TRUSTED_PROXIES`.
A secret never goes into an environment entry. A key with the suffix `_file` names a secret file (ADR-0062 (g), (h)).

These keys have a procedure:

| Goal | Key | Where to change it |
|---|---|---|
| Turn separation of duties on or off | `approval.separation_of_duties`, the value `true` or `false` | `compose.yaml`, with the steps below |
| Change the session timeouts | `session.idle_timeout` and `session.absolute_timeout`, whole seconds | `compose.yaml`, with the steps below |
| Change the languages of the translations | `i18n.translation_langs`, a comma list of language tags | `compose.yaml`, with the steps below |
| Add a trusted proxy | `http.trusted_proxies`, a comma list of networks | Section "A plant device in front of Caddy" |
| Change or disable the webhook | The file named by `alerts.webhook_url_file` | Sections "Change the webhook" and "Disable the webhook" |
| Change the login or CSRF key | The files named by `security.login_hmac_key_file` and `security.csrf_signing_key_file` | Section "Key rotation" |
| Keep the PLC credential key | The file named by `security.plc_credential_key_file` | Sections "Create the secret files" and "Upgrade". The key has no rotation (ADR-0154) |
| Change the password of the role `recipe_app` | The file named by `secrets.app_password_file` | Section "Change a database password" |

No document describes a change of the other keys. These are `db.host`, `db.port`, `db.name`, `db.app_user`, `http.public_port`, and `http.internal_port`.
The keys that name a file path and the key `alerts.webhook_ca_file` are also in this group. The section "Known gaps" names the last key.
Other files of the stack use these values. `Caddyfile` sends requests to `app:8080`, and the health check of `proxy` uses `app:9000`.

The value `false` for `session.secure` stops the start in a release build (ADR-0062 (b)). Keep the value `true`.

The server reads the environment at each start only. To change a key of the first three rows, do these steps:

1. Open `compose.yaml`. In the service `app`, find the entry of the key.
2. Write the new value in double quotes, as the other entries do.
3. Run `docker compose config | grep -F '<entry name>:'`. Check that the output shows the new value.
4. Run `docker compose up -d --force-recreate --no-deps app`. Without `--no-deps`, Compose runs `backup-premigrate` and `migrate` again (ADR-0082). After a rollback, use "Start app after a rollback".
5. Run `docker compose ps app` until `app` shows `healthy`.
6. Open `/audit` as an Administrator. Filter the action `SettingChanged`.
7. Open the newest entry of the changed key.
8. Check that `after` shows the new value and `before` shows the old value.

While `app` restarts, Caddy answers `502` (ADR-0065 (p)).
The server writes one entry `SettingChanged` for each key whose value differs from the stored value. All entries of one start share one `request_id`. The audit log keeps the entries without a time limit (`reliability.md` 7.12).

The server reads only the names of the table in ADR-0062 (a). It does not read a misspelled name, so the default of the key stays.
If no entry shows, the value may equal the stored value, or the name in `compose.yaml` may be wrong. Repeat step 3.

A bad value stops the start. `app` writes one `ERROR` line and exits with code 1 (`reliability.md` 7.12, ADR-0061 (b)).
The line names the key, or it holds the text `the configuration is not valid` for a value of the wrong type. A value of `0` for a timeout is also bad.

The restart policy `always` then restarts `app`. Use the section "Restart loop" to read the line.
Write the old value again. Then run step 4.

A new release replaces `compose.yaml`, and the edit is lost (section "Known gaps"). After each upgrade, write the edit again.
Do it before the step of the section "Upgrade" that replaces `app`. Otherwise the next start records the value of the new file with an entry `SettingChanged`.

The other values of `reliability.md` 7.11 are code defaults and not configuration keys (ADR-0062 (a)). Examples are `disk.*`, `backup.retention`, and `basebackup.retention`.
A change of a code default needs a new release. See the sections "Read-only mode" and "Low disk space".

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
5. Run `docker compose up -d --force-recreate --no-deps app`. After a rollback, use "Start app after a rollback".
6. Run `docker compose ps app` until `app` shows `healthy`.
7. Compute the fingerprint of the file. The command prints a hash and does not print the URL:

```bash
tr -d '\n' < "$RECIPE_SECRETS_DIR/alerts_webhook_url" | sha256sum
```

8. Open `/audit` as an Administrator. Filter the action `SettingChanged`.
9. Open the newest entry of the key `alerts.webhook_url`.
10. Compare the value of `after` with `sha256:` and the hash of step 7.

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
6. After a rollback, read the warning in the section "Rollback" first.
7. Start the stack with the steps of the section "Start the whole stack again". Do not run `docker compose up -d`.
8. Run the check of the section "Check the client rules of the database".
9. Run the section "Check the host firewall" again.

If `db` does not start, put the old file back in the volume of `db`. Then change the lines and start again:

```bash
docker compose run --rm --no-deps -u postgres --entrypoint sh db -c 'cp -p -- "$PGDATA/pg_hba.conf.old" "$PGDATA/pg_hba.conf"'
```

## A gap in the WAL stream

An outage of `walarchive` longer than `pg.wal_keep_size` leaves a gap (ADR-0068 (e), ADR-0099).
The alert `replication` is active during the outage.

The procedure moves the WAL segment files and the `.partial` file from the top of `RECIPE_WALARCHIVE_DIR` (ADR-0068 (l)).
The marker file, `.lock`, the directory `base`, the directory `.pitr-scratch`, the file `.system-identifier`, and the file `base-backup.result` stay.
The cluster does not change in a gap, so its system identifier stays the same (ADR-0105 (o)).

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

7. Run `ls -A "$RECIPE_WALARCHIVE_DIR"`. Check that no segment file remains. The marker file, `.lock`, `base`, `.pitr-scratch`, `.system-identifier`, and `base-backup.result` stay.
8. Run `docker compose up -d --no-deps walarchive`. The stream starts on a directory without segments.
   - The guard finds the file `.system-identifier` and no segment. It compares the identifier with the identifier of `db` and passes.
9. Check the stream. Take no base backup before it passes:

```bash
docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT state, flush_lsn FROM pg_stat_replication WHERE usename = 'recipe_replicator' AND application_name = 'walarchive'"
```

The check passes when the result has exactly one row, the state `streaming`, and a `flush_lsn`.

10. Take the replacement base backup. The output holds the line `base backup completed`. Note its field `base`:

```bash
docker compose exec walarchive basebackup-once
```

If the output says `another base backup run holds the lock`, run this step again after the weekly run ends.

11. Switch the WAL segment. Note the returned position:

```bash
docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT pg_switch_wal()"
```

The server then closes the segment that holds the end of the base backup.

12. Run the flush check until it returns `t`. Run the query again by hand until then:

```bash
docker compose exec -T -u postgres db psql -d recipe -At -c "SELECT EXISTS (SELECT 1 FROM pg_stat_replication WHERE application_name = 'walarchive' AND usename = 'recipe_replicator' AND flush_lsn >= '<position>'::pg_lsn)"
```

The value `t` shows that the archive holds the end segment of the base backup as a complete file.

13. Check the WAL of the base backup in the archive. Replace `<base>` with the field `base` of step 10:

```bash
docker compose exec walarchive pg_verifybackup --quiet --skip-checksums --ignore=backup_end_time --wal-directory=/wal /wal/base/<base>
```

The check passes on the exit code 0.

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

4. Run `docker compose up -d --force-recreate --no-deps proxy app`. After a rollback, use "Start app after a rollback".
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
2. Write the start time into the change log ([rule](#change-log-of-the-site)). Run the `GRANT` of the script and the `REVOKE`:

```sql
GRANT pg_read_all_stats TO audit_owner WITH INHERIT TRUE;
REVOKE pg_read_all_stats FROM recipe_app;
```

3. Run `\drg recipe_app`. Check that the result shows no `pg_read_all_stats`.
4. Do not replace the guard function now. Steps 13 to 17 of "Upgrade" replace it after `migrate` (ADR-0097 (h3)).
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

The runbook deletes no `pgdata` volume. The section [Destructive commands](#destructive-commands) gives the delete rule (ADR-0103 (t)).

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

An ordinary validation error of a user, with the HTTP status 4xx, is an `INFO` line. A search for `ERROR` does not find it.
Some 4xx errors have the level `WARN` or `ERROR`. `reliability.md` 3.3 gives the level policy.

The names of the alert conditions are in the section "Alarms". The names of the jobs are in `reliability.md` 2.4.

### Find the lines of a user report

The error fragment of the UI shows the error code and the `request_id` (`api.md` 5.2).

1. Read the code and the `request_id` in the error fragment.
2. Run `docker compose logs --no-log-prefix app | grep -F '"request_id":"<request_id>"'`.
3. Read the field `error_code` of the lines.

The line of the backstop panic layer holds no `request_id`. Search for the code `E-WEB-010` in that case (`reliability.md` 3.2).
The `request_id` is also present in the audit log and in the security events (`reliability.md` 3.6).

### Read the lines of a job

The supervisor writes a line for a failed run and for a panic of a job only. The supervisor writes no line for a run that ends well (ADR-0100, `crates/recipe-server/src/supervisor.rs`).
So a missing line does not show that no fault occurred. It also does not show that a run happened.

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
| `restore_test` | Run `docker compose exec backup cat /backups/restore-test.result`. Find the case under the table. | Run `docker compose exec backup restore-test`. Use the section "Run the restore test". |
| `base_backup` | Run `docker compose exec walarchive cat /wal/base-backup.result`. Find the case under the table. | Run `docker compose exec walarchive basebackup-once`. |
| `replication` | Run `docker compose ps walarchive` and `docker compose logs walarchive`. For a gap, use the section "A gap in the WAL stream". | None beyond the first action. |
| `low_disk` | Read the field `volume` of the line: `pgdata`, `backups`, or `walarchive`. | Use the section "Low disk space". For `pgdata`, use the section "Read-only mode" too. |
| `failed_logins` | Open `/security-events`. Filter the kind `login_failed`. Read `login` and `client_ip`. | If a user cannot log in, an Administrator opens `/users/<login>` and uses the unlock form (`LoginUnlock`, `api.md` 2.8). If `client_ip` equals `PROXY_ADDRESS`, use the section "Check client_ip of the first login". |
| `security_events` | Open `/security-events`. Filter the kind `csrf_rejected`. Read `client_ip`. Then filter the kind `forbidden`. Read `client_ip`. | If `client_ip` equals `PROXY_ADDRESS`, use the section "Check client_ip of the first login". No document gives another action. |
| `audit_chain_broken` | Read the line `E-AUD-002` in `docker compose logs app`. It names the `chain_seq`. Open the row in `/security-events`. | The condition stays active while the newest row is younger than `audit.max_check_age`. Each nightly check adds a row while the chain stays broken (`reliability.md` 3.7). No document gives a fix. |
| `audit_check_age` | Run `docker compose ps app`. Read the lines with the field `job` of `audit_chain_check` in `docker compose logs app`. | A run that returns an error code other than `E-AUD-002` is not a completed check. Follow the row of that code in `reliability.md` 1.2 (ADR-0100 (e)). A stored time that is later than the time of `db` by more than `alerts.interval` also keeps the alert active (`reliability.md` 3.7). |
| `export_hash_mismatch` | Read the line `E-EXP-002` in `docker compose logs app`. It names the recipe and the version. | The server refuses the export of that version (`reliability.md` 1.2). No document gives a fix. |

The alerts `restore_test` and `base_backup` are active in four cases (ADR-0151 (i)). The result file holds the keys `result` and `finished_at`.
For `base_backup`, replace `backup` with `walarchive` in the commands of this list. Only the first case always writes an `ERROR` line.

1. The key `result` is `failed`. Run `docker compose logs backup` and read the last `ERROR` line.
2. The key `result` is `passed`, and `finished_at` is older than 8 days. No later run wrote a result, so the log can show no `ERROR` line.
3. The command `cat` reports that the file does not exist. A killed run writes no file. The log shows no `ERROR` line.
4. The file exists, but `app` cannot read or parse it. The job `alerts` writes one `WARN` line in `docker compose logs app` (ADR-0151 (j1)).

To find the `WARN` line of the case 4, run `docker compose logs --no-log-prefix --since 1h app | grep -F 'The result file of the'`. The line names the check and ends with `cannot be read.` or `is malformed.`.
The directory `/wal` needs the mode `0711`, and the file needs the mode `0644` (ADR-0151 (k)). The command `cat` can succeed when `app` cannot read the file.
Run `docker compose exec walarchive ls -ld /wal /wal/base-backup.result` to read both modes.

### Low disk space

The condition `low_disk` is active when a volume has less free space than `disk.warn_percent` (`reliability.md` 3.7).
The webhook body names no volume, so read the field `volume` of the `WARN` lines (ADR-0075 (d), (e)).
The three volumes lie on these disks:

| Volume | Disk |
|---|---|
| `pgdata` | The disk of the Docker root directory. The value `RECIPE_PGDATA_VOLUME` of `.env` names the volume |
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

1. If the condition `backup_age` is active too, use its row first.
2. Run the section "Delete orphan temp files". It deletes only the temp names of killed runs.
3. If the volume stays low, extend the filesystem of the second disk with the site procedure.

The retention runs only after a complete new set (`reliability.md` 7.7).
No document gives the site procedure of step 3.

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
`Login`, `Logout`, every query, every export, and `ExportPlcConnection` continue (A24 in `overview.md` 9).
Every other command returns `503` with the code `E-WEB-006`.

To end the mode, do these steps:

1. Run `docker compose logs --no-log-prefix --since 1h app | grep -F '"volume":"pgdata"'`.
2. If a line says `The free disk space cannot be read.`, use the case "The mode stays on" below.
3. Load the values of `.env` with `. ./.env`. Do not export them.
4. Read the mount point: `docker volume inspect --format '{{ .Mountpoint }}' "$RECIPE_PGDATA_VOLUME"`.
5. Run `df -h <mount point>`.
6. Increase the free space of that disk. Delete only files outside the volumes `pgdata`, `backups`, and `walarchive`.
7. Delete no set, no base backup, and no WAL segment by hand. Delete nothing inside `pgdata`.
8. Wait for `health.disk_check_interval`. Load a page again. The banner goes away when the free space is above `disk.resume_percent`.

No document names a file that is safe to delete outside the three volumes.
If the disk has no such file, extend its filesystem with the site procedure.

#### The mode stays on

If the free space looks fine and the banner stays, find the cause in this list:

- The free space is between `disk.readonly_percent` and `disk.resume_percent`. Increase the free space until it is above `disk.resume_percent`.
- The measurement fails each run. `app` reads `pgdata` through a read-only mount. Run the command below. The output must show the line `/var/lib/postgresql/data false`. The value `false` shows a read-only mount.

```bash
docker inspect --format '{{ range .Mounts }}{{ .Destination }} {{ .RW }}{{ println }}{{ end }}' "$(docker compose ps -q app)"
```

- If the mount line is present, run `docker compose ps` and read the health of `app` and `db`. No document gives a further fix.

The mode ends after the first measurement that succeeds and shows more than `disk.resume_percent` free.

### Restart loop

A restart loop of `app` ends the job `alerts`, so no alert comes (`reliability.md` 3.7).
Use the host monitoring for this case.

1. Run `docker compose ps`. Look for a service that restarts or shows `unhealthy`.
2. For a service other than `app`, use the section "Service unhealthy".
3. Run `docker compose logs app`. Read the `ERROR` line of the failed start.
4. If the line names a variable `PG*`, list the names in the Compose output:

```bash
docker compose config | grep -o '^ *PG[A-Za-z_]*:'
```

5. Delete that variable from `compose.yaml`, or from `.env` when `compose.yaml` reads its value there.
6. Run `docker compose up -d --no-deps app`. After a rollback, use "Start app after a rollback".

Without `--no-deps` in step 6, Compose runs `backup-premigrate` and `migrate` again (section "Start the whole stack again").

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

3. Run the check by hand with the command of the next table.
4. Read the exit code with `echo $?`. Exit code 0 is a pass.
5. Run `docker compose logs --tail 50 <service>`. Read the `ERROR` lines (section "Read the logs").
6. Find the row of the service in the next table.

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
The rows about the manifest key file come from `backup-once`, from `backup-entry` at the start of `backup`, and from `restore-test` and `restore-volume`.
The rows about the tag file come from `restore-test` and `restore-volume` (ADR-0174 (w)).
`restore-volume` prints its line on the console. Only the message `the manifest key file has a bad line` has the field `line`. It names the line of the key file.

| Message | Cause | Action |
|---|---|---|
| `another backup run holds the lock` | Another run writes in `RECIPE_BACKUP_DIR`. It is the nightly run, `backup-now`, or `backup-premigrate`. A run that does not end holds the lock too. | Run `docker compose top backup`. If a process `backup-once` runs, wait for its end. Then run the command again. |
| `a file with a name of the new set exists` | The name of a set holds the UTC minute. A second run in the same minute finds the names. A temp file of a killed run can hold a name. | Wait one minute. Run the command again. |
| `the marker file of the backup disk is missing` | The file `.backup-marker` is not in `RECIPE_BACKUP_DIR`. | Use the same action as in the table "Messages of backup-health". |
| `cannot read the password file of recipe_backup` or a message that starts with `the password file of recipe_backup` | The file `backup_password` is unreadable, is empty, or has a wrong form. | Check the owner and the mode as in the section "Create the secret files". The file holds one line and no carriage return. |
| `cannot publish a file of the set without replacing a file` | The filesystem of the second disk may lack hard links (ADR-0074 (e)). | Check the filesystem as in step 1 of the section "Prepare the second disk". |
| `pg_dump failed`, `pg_dumpall failed`, or `the snapshot session ended before it answered` | The tool or the database stopped the run. | Read the lines above the message. Run `docker compose ps db`. No document gives a further action. |
| `the manifest key file is missing` | The container has no file `/run/secrets/backup_manifest_key`. The Compose file may lack the secret, or the host file is missing. | Run the steps of the section "Create the secret files" for `backup_manifest_key`. After a rollback, read the section "Leave the file compose.previous.yaml". |
| `cannot read the manifest key file` | The file exists, but the user of the container cannot read it. It may be a link or have a wrong owner or mode. | Check the owner and the mode as in the section "Create the secret files". Do not print the file. |
| `the manifest key file exceeds the size limit` | The file is larger than the limit of the tag program. It may hold more than the 2 key lines. | Write the file again as the section "Rotate the manifest key" says. Do not print the file. |
| `the manifest key file has a bad line` | The field `line` names a line that is not 64 lower-case hex characters. | Compare the file with the rules of the section "Rotate the manifest key". Do not print the file. |
| `the manifest key file holds a carriage return` or `the manifest key file holds a NUL byte` | An editor changed the file. | Write the file again as the section "Create the secret files" says. Do not print the file. |
| `the manifest key file has too many lines` | The file holds more than 2 lines (ADR-0174 (n)). | Delete the oldest line as the section "Rotate the manifest key" says. |
| `the manifest key file lies on the backup disk` | `RECIPE_SECRETS_DIR` is on the filesystem of `RECIPE_BACKUP_DIR` (ADR-0174 (p)). | Run steps 11 and 12 of the section "Create the secret files". |
| `cannot check the filesystem of the manifest key file` | The command `stat` failed for the key file or for `RECIPE_BACKUP_DIR` (ADR-0174 (p)). | Check that the host mounts `RECIPE_BACKUP_DIR`. Check the key file as in the section "Create the secret files". |
| `the manifest tag program failed` | The tag program gave a fault that has no own message. The field `line` is absent. | Read the lines above the message. Check the key file as in the section "Create the secret files". Do not print the file. |
| `the newest set has no tag file` | The newest set has no file `recipe_<stamp>.dump.sha256.hmac`. A previous image wrote the set, or a writer deleted the file. The field `set` names the set. | Do not restore the set. Run `backup-now` with the image that has the tag check. Then run the restore test again (ADR-0174 (x)). |
| `the tag file of the set has a bad line` | The tag file of the set has a wrong form. The field `set` names the set. | Do not edit the file. Run `backup-now` again, then run the restore test again. |
| `cannot read the tag file of the set` | The tag file exists, but the user of the container cannot read it. | Do not edit the file. Run `backup-now` again, then run the restore test again. |
| `the tag file of the set exceeds the size limit` | The tag file is larger than the limit. A writer may have changed it. The field `set` names the set. | Do not restore the set. Run `backup-now` again, then run the restore test again. |
| `the tag of the set comes from an unknown key` | The tag names a key that no line of `backup_manifest_key` holds. The key may be lost or rotated out (ADR-0174 (q)). | Put the old key line back from the offline copy. See the subsection "Compare the offline copy". |
| `the tag of the set does not match the manifest` | Someone changed the manifest or the tag file after the run wrote them. The set may be crafted. | Do not restore the set. Use an older set whose tag matches, or use the section "Compromise rotation". |
| `cannot read the manifest` | The manifest of the set exists, but the tag program cannot read it. | Do not restore the set. Use an older set whose tag matches. |
| `the manifest exceeds the size limit` | The manifest is larger than the limit. A writer may have changed it. The field `set` names the set. | Do not restore the set. Use an older set whose tag matches, or use the section "Compromise rotation". |
| `cannot load the manifest tag library` | The image `recipe-pgtools` lacks the file `manifest-tag.sh`, or the file is unreadable. | Compare the image ID with the release notes as in step 5 of the section "Load the images". |
| `cannot write the manifest tag file` | `backup-once` could not create the temp tag file in `RECIPE_BACKUP_DIR`. The disk may be full or read-only, or a file of that name exists. | Use the section "Low disk space". Check the filesystem as in step 1 of the section "Prepare the second disk". |
| `cannot copy the manifest` | `restore-test` or `restore-volume` could not copy the manifest into its private directory. | Check the free space of the private directory and the state of the backup disk. Run the command again. |
| `cannot copy the manifest tag file` | `restore-test` or `restore-volume` could not copy the tag file into its private directory. | Check the free space of the private directory and the state of the backup disk. Run the command again. |
| `cannot set the manifest copy permissions` | `chmod` failed on the private copy of the manifest. | Check the private directory as in the section "Create the secret files". Run the command again. |
| `cannot set the manifest tag copy permissions` | `chmod` failed on the private copy of the tag file. | Check the private directory as in the section "Create the secret files". Run the command again. |
| `cannot read the manifest size limit` | The restore script found no valid size limit in the file `manifest-tag.pl` of the image. | Compare the image ID with the release notes as in step 5 of the section "Load the images". |
| `cannot record the manifest size fault` | The restore script could not write the fault file in its private directory. | Check the free space of the private directory. Run the command again. |
| `the checked manifest argument is missing` | A restore script called a check without the path of the checked manifest. This is a fault of the image. | Compare the image ID with the release notes as in step 5 of the section "Load the images". |
| `cannot read the manifest of the newest set` | The restore script could not read the private copy of the manifest. | Check the free space of the private directory. Run the command again. |
| `the counts copy does not match the manifest` | The hash of the private counts copy differs from the hash in the manifest. The set may be crafted. | Do not restore the set. Use an older set whose tag matches, or use the section "Compromise rotation". |
| `cannot copy the counts file` | `restore-test` or `restore-volume` could not copy the counts file into its private directory. | Check the free space of the private directory and the state of the backup disk. Run the command again. |
| `cannot set the counts copy permissions` | `chmod` failed on the private copy of the counts file. | Check the private directory as in the section "Create the secret files". Run the command again. |
| `cannot hash the counts copy` | The command `sha256sum` failed on the private copy of the counts file. | Check the free space of the private directory. Run the command again. |
| `the newest set lacks a regular file` | A file of the newest set is missing or is not a regular file. The field `set` names the set. | Do not restore the set. Run `backup-now`, then run the restore test again. |

A run of `backup-once` has no time limit (ADR-0074 (d)). If `db` does not answer, the run holds the lock.
Then each later run and `backup-premigrate` stops with `another backup run holds the lock`. The held run deletes nothing.
After `backup.max_age_alert`, `backup-health` fails and the alert `backup_age` is active.

Find the run with `docker compose top backup`. No document gives the time after which a run counts as hung.
No document gives the command that ends a hung run.

### Messages of walarchive

The container ends when `pg_receivewal` or `supercronic` exits (ADR-0068 (c)). Docker then starts it again.
The `ERROR` lines are in `docker compose logs walarchive`.
`basebackup-once` runs the same guard of the file `.system-identifier`. It writes its messages to its output.

| Message | Cause | Action |
|---|---|---|
| `no process pg_receivewal runs` | `pg_receivewal` stopped, or the container just restarted. | Read `docker compose logs walarchive`. Use the next rows. |
| `a child process exited` | The field `child` names `pg_receivewal` or `supercronic`. The field `exit_code` gives its code. | Read the lines above it, for the text of `pg_receivewal`. If `df` shows a full disk, use the section "Low disk space". If the alert `replication` stays active, use the section "A gap in the WAL stream". |
| `the marker file of the WAL archive disk is missing` | The file `.walarchive-marker` is not in `RECIPE_WALARCHIVE_DIR`. | Run `install -m 0600 -o 999 -g 999 /dev/null "$RECIPE_WALARCHIVE_DIR/.walarchive-marker"`. |
| `cannot read the password file of recipe_replicator` or a message that starts with `the password file of recipe_replicator` | The file `replicator_password` is unreadable, is empty, or has a wrong form. | Check the owner and the mode as in the section "Create the secret files". The file holds one line and no carriage return. |
| `segments are in the WAL archive and the file .system-identifier is missing` | `RECIPE_WALARCHIVE_DIR` holds a segment file or a `.partial` file and no file `.system-identifier`. An installation of 0.1.0 is in this state until the segment check runs. The container refuses to start. | Run the segment check of step 2 of the section "Upgrade". When `.env` holds the new tag, put `RECIPE_VERSION=<previous tag>` before `docker compose run`. Do not create the file by hand. |
| `the file .system-identifier holds the identifier of another system` | The file names another cluster than the running `db`. The segments may come from that cluster. The fields `file_identifier` and `server_identifier` show both numbers. | Do not edit, move, or delete the file. Run the name check of step 2 of the section "Upgrade". If the names agree, no document gives a further action. |
| `the file .system-identifier is malformed` | The file is empty, holds a carriage return, holds a character other than a digit, or holds more than one line. | Run `cat -A "$RECIPE_WALARCHIVE_DIR/.system-identifier"`. Do not edit the file by hand. No document gives a further action. |
| `cannot read the file .system-identifier` | The file is not a regular file, is a link, or user 999 cannot read it. | Run `ls -ln "$RECIPE_WALARCHIVE_DIR/.system-identifier"`. Do not replace the file. No document gives a further action. |
| `cannot read the system identifier of the server` | The replication connection to `db` failed, or `IDENTIFY_SYSTEM` gave no number. The field `detail` gives the first line of the error of libpq (ADR-0105 (s)). | Run `docker compose ps db`. Read the lines above the message. Check the password file as in the row of `recipe_replicator`. |
| `cannot write the file .system-identifier` | `walarchive` could not create the temp file or the hard link in `RECIPE_WALARCHIVE_DIR`. The disk may be full, read-only, or without hard links (ADR-0074 (e)). | Use the section "Low disk space". Check the filesystem as in step 1 of the section "Prepare the second disk". |
| `the file .system-identifier exists, but the sync of the directory failed` | `walarchive` linked the file in `RECIPE_WALARCHIVE_DIR`, but `sync` of the directory failed. The file exists, and the disk may be failing. | Use the section "Low disk space". Check the filesystem as in step 1 of the section "Prepare the second disk". Run `walarchive` again. It reads the file and compares it. |
| `mktemp: failed to create directory via template '/run/pgpass/run.XXXXXX': Permission denied` | The image `recipe-pgtools` of 0.1.0 has no mount point `/run/pgpass`. After a restart of the container, uid 999 cannot write into the tmpfs. The same fault can stop `backup` of 0.1.0 (ADR-0104). | Step 23 of the section "Upgrade" replaces the container with the image of the fix. The restore test of step 3 and the segment check do not need the old containers. |
| `cannot load wal-identity.sh` | The image `recipe-pgtools` lacks the file of the guard. | Compare the image ID with the release notes as in step 5 of the section "Load the images". |

## A lost one-time password

A `503 E-WEB-009` after `POST /users` or after a password reset can lose the one-time password (ADR-0172 (a)).
Use this procedure to get a new one-time password. It needs no database command.

1. Open `/users` as an administrator. Find the login of the user in the list.
2. If the user is not in the list, create the user again at `/users/new`. Then stop.
3. If the user is in the list, open the user page.
4. Open the section "Reset the password".
5. Select "Reset password for <login>". The new one-time password appears once.
6. Give the new one-time password to the user at once.

## Known gaps

The documents do not yet close the gaps below. Each bullet gives the workaround of this page, if one exists.

- The networks `edge` and `egress` have no fixed subnet. Docker can give them new subnets after `docker compose down` and `docker compose up` (section [Stop and start, or down and up](#stop-and-start-or-down-and-up)). Read the subnets again and write the firewall rules again (section "Check the host firewall").
- A restore of a set with the schema of 0.1.0 has no procedure, also the rollback restore of a `_premigrate` set. The image of 0.1.0 has no `restore-volume` (ADR-0103 (h), (q)). The `backup-now` of the upgrade makes the first set that `restore-volume` can restore. No workaround.
- The code of a dump runs as the superuser of the cluster that `restore-volume` fills, and that cluster becomes production. The checks of ADR-0103 (m) and (o) cannot find each change inside the catalog. The user accepts this residual risk (ADR-0103 (o)). A later ADR may restore `pre-data` as a role that is not a superuser (ADR-0084). No workaround.
- No document gives the transfer of the external images `caddy` and `postgres`, or the digest check after `docker load`. The section "Load the images" names only `recipe-server` and `recipe-pgtools`. No workaround.
- The trusted-proxy entries in `Caddyfile` and `compose.yaml` do not survive a release. A new release replaces both files. After each upgrade, write the entries again (sections "Upgrade" and "A plant device in front of Caddy").
- The soak test of `reliability.md` section 6 is not implemented. The repository holds no soak script and no nightly CI job for it. The 2 h local run with an RSS chart did not run, so no file `docs/ops/soak-<date>.md` exists. The limits of that test stay unproven. They are `soak.memory_ceiling` of `reliability.md` 7.11, zero restarts, and the p99 latency bound of section 6. No workaround.
- The section "Check the host firewall" gives no rule that limits the network `egress` to the webhook host (ADR-0063 (p)). No workaround.
- The file `compose.yaml` sets no `RECIPE_ALERTS__WEBHOOK_CA_FILE` and mounts no CA file. The key `alerts.webhook_ca_file` therefore stays empty, and the webhook client trusts the webpki roots only (ADR-0063 (j)). No workaround.
- No procedure recovers access when the only administrator cannot log in. The command `create-admin` does nothing when an active administrator exists. Only a user with `UserManage` resets a password (`api.md` section 3). No workaround.
- A throttled login is a separate case. It ends by time, because there is no permanent lockout (`domain-model.md` 2.3, `login.delay_formula`, `login.ip_budget`). Wait, then log in again.
- The login throttle counts failures per login and address and per address, but not per login over all addresses. A client with many addresses therefore gets many guesses against one login. The alert `failed_logins` reports such a flood, but it does not stop it ([ADR-0150](../adr/0150-no-per-login-limit-over-all-addresses.md)). Follow the row `failed_logins` of the section [Alarms](#alarms).
- A wrong current password in `ChangeOwnPassword` writes no security event. The alert `failed_logins` therefore does not count it. [ADR-0150](../adr/0150-no-per-login-limit-over-all-addresses.md) records this as a residual risk.
- Only an image that checks the tag restores a set with a tag file (ADR-0174 (z)). Case 2 of ADR-0174 (aj) is the one exception, and the section "Rollback" gives its check.
- An online writer defeats the tag of the manifest at three points (ADR-0174 (ag)). It can change a temp file before the hash, write into the dump file after the check, or change the scratch cluster of `restore-test`. No workaround.
- The key lies in the container where the SQL of a set runs as a superuser (ADR-0174 (ah)). A holder of the key or of a shell as uid 999 in `backup` can sign a crafted set. After such an incident, use the section "Compromise rotation".
- The tag does not cover the modification time of a file (ADR-0174 (ai)). A writer can delete newer sets and change the times, so the alert `backup_age` gives no proof of a replay. Only the tag check of a restore proves a set.
- The previous image of the first release with the tag ignores the tag file of each set of its schema (ADR-0174 (aj)). The sections "Upgrade" and "Rollback" give the safeguards.
  - The retention of that image deletes the four files of an old set and keeps its tag file. Such a tag file has no manifest, so no reader takes it for a set. It is harmless, and the operator can delete it.
  - The limit ends with the second release that holds the tag check, because its previous image has the check.
- The server of `pitr-check` and `pitr-volume` reads the configuration files and the catalog of a base backup (ADR-0174 (ak)). An online writer of `/wal` has the power of the online writer above. An offline writer of `/wal` can change a base backup. The tag adds no check for it. No workaround.
- No document gives a procedure for the cases below. Each item links the section where the case appears. That section gives the part of the procedure that exists.
  - Alarms: an action for `panics`, `audit_chain_broken`, `export_hash_mismatch`, and `security_events` ([Alarms](#alarms))
  - Alarms: a fix when `not_ready` comes from a migration with another checksum ([Alarms](#alarms))
  - Restore: an action when the run of `audit_chain_check` in step 10 fails or does not complete ([Restore into production](#restore-into-production))
  - Disks: an extension of the filesystem of the second disk ([Low disk space](#low-disk-space))
  - Disks: free space on the disk of `pgdata` ([Read-only mode](#read-only-mode))
  - Disks: a measurement of `pgdata` that keeps failing ([The mode stays on](#the-mode-stays-on))
  - Alert webhook: a test call ([Change the webhook](#change-the-webhook))
  - Service unhealthy: the time after which a run of `backup-once` counts as hung, and the command that ends it ([Messages of backup-once](#messages-of-backup-once))
  - Service unhealthy: `proxy` unhealthy while `app` is healthy, and `db` unhealthy for a cause other than a change of `pg_hba.conf` ([Read why a service is unhealthy](#read-why-a-service-is-unhealthy))
  - Rotation: a test of a new `recipe_migrator` password before the next run of `migrate` ([Change a database password](#change-a-database-password))
  - Off-site copy: a copy tool and a schedule ([Copy the backups off site](#copy-the-backups-off-site))
  - Off-site copy: a check of a copy of the base backups and of the WAL segments ([Copy the backups off site](#copy-the-backups-off-site))
  - Off-site copy: the restore of a set from a copy to the host ([Copy the backups off site](#copy-the-backups-off-site))
  - Off-site copy: the read of an audit entry from a copy ([Copy the backups off site](#copy-the-backups-off-site))
  - Off-site copy: the exclusion of a set that dump code rewrote from the copy (ADR-0084, [Copy the backups off site](#copy-the-backups-off-site))
  - Images: the time that a release stays rollback-ready ([Keep the images for a rollback](#keep-the-images-for-a-rollback))
  - Index: a `psql` session as `recipe_migrator` for a concurrent index ([Concurrent index on an audit object](#concurrent-index-on-an-audit-object))
  - Settings: a change of the keys `db.host`, `db.port`, `db.name`, `db.app_user`, `http.public_port`, and `http.internal_port` ([Change a setting](#change-a-setting))

A later change will close these gaps with its own ADR.

## Related

- [reliability.md](../architecture/reliability.md): sections 1.2, 2.4, 3.1 to 3.7, 6, 7.1, 7.2, 7.3, 7.7, 7.8, 7.9, 7.10, 7.11, and 7.12
- [api.md](../architecture/api.md): sections 2.8, 2.9, 3, 5.2, and 6
- [data-model.md](../architecture/data-model.md): sections 4.9, 5.1, and 8
- [domain-model.md](../architecture/domain-model.md): section 2.3
- [overview.md](../architecture/overview.md): section 9
- [ADR-0020](../adr/0020-deployment-migrations-backup.md): Deployment, migrations, backup and failure policy
- [ADR-0031](../adr/0031-migrator-configuration-and-secret-files.md): Configuration of the migrator and the secret files of the roles
- [ADR-0033](../adr/0033-globals-dump-of-the-backup.md): Globals dump of the backup
- [ADR-0035](../adr/0035-identity-sessions-clarifications.md): Identity, sessions, and the guard of security events
- [ADR-0059](../adr/0059-health-probe-readiness-and-health-router.md): Health probe subcommand, readiness rule, and health router layers
- [ADR-0061](../adr/0061-start-order-of-serve.md): Start order of serve
- [ADR-0062](../adr/0062-configuration-keys-and-secret-files.md): Configuration keys of serve, their delivery, and the secret files
- [ADR-0063](../adr/0063-webhook-of-the-alerts.md): Webhook of the alerts
- [ADR-0064](../adr/0064-images-and-their-distribution.md): Images and their distribution
- [ADR-0065](../adr/0065-proxy-tls-and-default-trusted-proxy.md): TLS of the proxy and the default trusted proxy
- [ADR-0066](../adr/0066-backup-disks-file-sets-and-schedule.md): Backup disks, file sets, and schedule
- [ADR-0067](../adr/0067-restore-test-in-the-backup-service.md): Restore test in the backup service
- [ADR-0068](../adr/0068-wal-archive-and-base-backups.md): WAL archive and base backups
- [ADR-0069](../adr/0069-hardening-of-the-database-access.md): Hardening of the database access
- [ADR-0070](../adr/0070-container-limits-networks-healthchecks-and-stack-test.md): Container limits, networks, healthchecks, and the stack test
- [ADR-0071](../adr/0071-read-only-banner.md): Read-only banner
- [ADR-0073](../adr/0073-read-faults-and-the-audit-chain-alert.md): Read faults of the jobs and the audit chain alert
- [ADR-0074](../adr/0074-names-on-the-backup-disks.md): Names of the files on the backup disks
- [ADR-0075](../adr/0075-names-of-the-alert-conditions.md): Names of the alert conditions
- [ADR-0077](../adr/0077-recovery-check-wal-cutoff-and-scratch-constants.md): Marker entry of the recovery check, WAL cutoff, and scratch constants
- [ADR-0082](../adr/0082-upgrade-and-rollback-without-dependencies.md): Upgrade and rollback without the dependencies of app
- [ADR-0083](../adr/0083-pgtools-image-in-the-upgrade-and-the-rollback.md): Image of `backup` and `walarchive` in the upgrade and the rollback
- [ADR-0084](../adr/0084-generated-column-code-in-the-pre-data-restore.md): Code of a generated column in the restore of `pre-data`
- [ADR-0087](../adr/0087-drain-of-the-internal-listener.md): Drain of the internal listener
- [ADR-0088](../adr/0088-run-interface-of-the-jobs-and-cut-count.md): Run interface of the jobs and the cut count of the jobs
- [ADR-0089](../adr/0089-job-errors-first-disk-run-and-stop-line.md): Errors of the jobs, the first run of `disk_check`, and the stop line
- [ADR-0095](../adr/0095-role-grammar-of-the-globals-file.md): Role grammar of the globals file in the restore test
- [ADR-0096](../adr/0096-guard-lock-of-the-last-administrator.md): Guard lock of the last Administrator
- [ADR-0097](../adr/0097-schema-free-names-in-the-drop-guard.md): Names without a schema in the drop guard of the audit objects
- [ADR-0099](../adr/0099-stream-before-the-replacement-base-backup.md): Stream before the replacement base backup after a WAL gap
- [ADR-0100](../adr/0100-wait-for-the-audit-chain-job-after-a-restore.md): Wait for the scheduled audit chain check after a restore
- [ADR-0102](../adr/0102-end-the-restore-after-the-audit-chain-result.md): End the restore after the result of the audit chain check
- [ADR-0103](../adr/0103-volume-switch-restore-target-and-wal-line.md): Volume switch, restore target, and WAL line after a switch
- [ADR-0106](../adr/0106-recovery-of-production-from-the-wal-archive.md): Recovery of production from the WAL archive
