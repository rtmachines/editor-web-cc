# Design

## Context

See `proposal.md`, section "Why". ADR-0103 holds every decision of the restore into a new volume, the switch, the switch back, and the WAL line. The architecture follows it in `reliability.md` 7.1, 7.7, 7.8, 7.10, 7.11 and in `data-model.md` 5.1. This design gives only the file layout, the order of the work, and the choices that ADR-0103 leaves to the implementation.

Current state of the code (branch `docs/restore-volume-switch`, commit `cdf925c`):

- `deploy/compose.yaml` declares `pgdata: {}`. Compose creates it as `recipe-editor_pgdata`.
- `deploy/db/initdb/01-roles.sh` writes `pg_hba.conf` with `printf` lines and runs the `REVOKE` statements inline.
- `deploy/pgtools/bin/restore-test` holds every restore step in one file. `deploy/pgtools/lib/` holds only `globals-grammar.sh`.
- The build of `recipe-pgtools` is in `deploy/compose.build.yaml` (ADR-0064 (i)) and in two `docker/build-push-action` steps of `ci.yml`. Both use the one context `deploy/pgtools`. That context has no `.dockerignore`.
- `deploy_values` reads the `pg_hba.conf` lines of `01-roles.sh` (`secrets.rs`). It lists the services and the scripts in `topology.rs`, `compose.rs`, and `inputs.rs`.
- The storage harness (`crates/recipe-db/tests/storage/support/container.rs`) and the server harness (`crates/recipe-server/tests/support/database.rs`) copy `01-roles.sh` into the container.
- The CI jobs `compose` and `upgrade` delete their volumes with `down -v`. That command does not delete an external volume.

## Goals / Non-Goals

**Goals:**

- Keep `cargo test --workspace` and the pgtools test suites green after each task.
- Cut code out of `restore-test` without a change of its behavior. The existing suite `restore-test-test` proves this.
- Keep one copy of each rule: the client rules, the `REVOKE` statements, the gates of the restore, and the guard of the system identifier.

**Non-Goals:**

- No change of production Rust code, of a migration, or of `grants.expected`.
- No new ADR. ADR-0103 is a draft of this pull request until the merge. ADR-0103, the Status line of ADR-0099, and `reliability.md` 7.7, 7.8, and 7.10 already hold the decisions of this design.
- No task changes `docs/architecture/` or an ADR.
- No automation of the switch. The switch stays a runbook procedure. Only CI scripts run its steps.

## Decisions

### D1. Shared file of the client rules

- Path: `deploy/db/cluster-access.sh`. It defines three functions and runs no command at source time.
- `check_compose_subnet <subnet>` holds the subnet grammar of `01-roles.sh`. `01-roles.sh` calls it before its first change, as today. `restore-volume` calls it in step 1.
- `write_hba_conf <subnet> <file>` writes the seven lines of ADR-0069 (r) to (t) to a temp file and renames it.
- `print_default_db_revokes` prints the two `REVOKE` statements of ADR-0032 for `postgres` and `template1`. The two statements for `recipe` stay in `01-roles.sh`, because `restore-volume` gets the access list of `recipe` from the dump.
- `db` mounts the file read-only at `/etc/recipe-db/cluster-access.sh`. This path is outside `/docker-entrypoint-initdb.d`, so the entrypoint never runs it. `01-roles.sh` sources this fixed path. No variable changes the path, so ADR-0069 (u) stays.
- The new mount changes the service configuration of `db`. In this release, step 3 command 1 of `reliability.md` 7.10 therefore replaces `db` once on the same volume (ADR-0103 (z)). The CI job `upgrade` checks the volume name and the system identifier of the new container (task 1.5).
- The `recipe-pgtools` image copies the file to `/etc/pgtools/cluster-access.sh`.
- Alternative: two copies and an equality test. ADR-0103 (l) rejects a second copy.

### D2. Build contexts

- `compose.build.yaml` gives the pgtools build `additional_contexts: {db: ./db}`. The `Dockerfile` uses `COPY --from=db`.
- Both `docker/build-push-action` steps of `recipe-pgtools` in `ci.yml` get `build-contexts: db=deploy/db`.
- `deploy/db/.dockerignore` and `deploy/pgtools/.dockerignore` are allow lists (ADR-0064 (f), ADR-0103 (l)). `deploy/pgtools` has no `.dockerignore` today, so this change adds both files (task 2.3).
- `deploy/ci/upgrade.sh` `build_previous` always passes `--build-context db=$UPGRADE_PREVIOUS_DIR/deploy/db`. A previous release without the named context ignores it. A previous release with this change needs it.
- `.gitattributes` gives `text eol=lf` to each file that the image copies and to both `.dockerignore` files. A `deploy_values` check reads the `COPY` lines of the `Dockerfile` and refuses a file without such a rule (task 3.3).

### D3. Shared restore library

- Path: `deploy/pgtools/lib/restore-steps.sh`, copied to `/etc/pgtools/restore-steps.sh`. `restore-test` and `restore-volume` source it.
- The library holds the functions of ADR-0067 steps 1, 2, and 4 to 7. These are the set selection by stamp, the manifest check, the globals check, and the name check.
- It also holds the section restore, the code comparison, the count comparison, and the chain check. It also holds `log`, `fail`, and `tool_detail`.
- The library holds no value of `reliability.md` 7.11. Each script keeps its own constants and settings, so the checks of `deploy_values` keep their place. `restore-test` keeps `fsync=off`, `wal_level=minimal`, `max_wal_senders=0`, and `scratch.port`.
- The random passwords of the login roles (`set_passwords`) and the login check stay in `restore-test` only (ADR-0103 (k)).
- Refactor order: the first task moves the functions and changes no case of `restore-test-test`. The suite must pass before and after.

### D4. Script `restore-volume`

- The script follows the ten steps of ADR-0103 (i). Each step is one function. The stub suite `deploy/pgtools/tests/restore-volume-test` tests each stop point, like `restore-test-test`.
- Step 1: the target `/var/lib/postgresql/data` is empty when `find -mindepth 1 -maxdepth 1` prints nothing. The stamp grammar is the stamp part of the manifest grammar of `reliability.md` 7.7.
- Step 2: `exec 9<"/backups/.scratch/.lock"` and `flock 9`. The script never creates the lock file.
- Step 4: `initdb` uses the arguments of `reliability.md` 7.1, `--auth-local=peer`, `--auth-host=scram-sha-256`, and a random `--pwfile` in the work directory of the tmpfs `/run/pgpass`.
- Step 4 runs `initdb` with `env -u LC_ALL` and the value `LANG` of the image. The other pgtools scripts export `LC_ALL=C`. Without this rule, `initdb` writes `lc_messages`, `lc_monetary`, `lc_numeric`, and `lc_time` as `C`. The entrypoint of `db` writes them from `LANG=en_US.utf8`.
- Step 5: `pg_ctl start -w -t <restore_volume.start_timeout>` with `-o` for `listen_addresses=''`, the private socket directory, and the five `scratch.*` sizes. The script passes no `fsync` option.
- Step 5 writes the server log to `/run/pgpass/<work>/server.log`. A log inside the target is a new top-level name, and step 8 refuses it.
- The tmpfs `/run/pgpass` has no size option, as in `backup`. Its pages count toward `container.backup_memory`, so that limit bounds the log and the work files.
- Steps 7 and 8 source `/etc/pgtools/cluster-access.sh` and run the query `restore-roles.sql` of D5.
- Step 9: `pg_ctl stop --mode=fast -w -t <restore_volume.stop_timeout>`, then `pg_controldata`, then `sync`.
- A `trap` stops the temporary server on each exit. It deletes no file of the target. ADR-0103 (i) gives the delete of the volume to the operator.

### D5. Expected file of the roles

- `deploy/pgtools/restore-roles.sql` is one query. It prints one line per value of ADR-0103 (m) in a fixed order. The line kinds are `ACL`, `GRANT`, `HBA`, `MEMBER`, `ROLE`, and `SETTING`, in this order.
- Inside a kind, the query sorts the lines with `COLLATE "C"`. `HBA` lines keep the order of `pg_hba.conf`, because the first rule that matches wins.
- `HBA` lines come from `pg_read_file('pg_hba.conf')` without comments and blank lines. The subnet becomes the placeholder `@COMPOSE_SUBNET@` in the expected file.
- The query normalizes each `HBA` line field by field. It splits the line on whitespace and joins the fields with one space. `01-roles.sh` pads the subnet to 16 columns, so a raw comparison fails for each subnet that is not 13 characters long.
- `ROLE` lines hold only these columns of `pg_authid`: `rolname`, `rolsuper`, `rolinherit`, `rolcreaterole`, `rolcreatedb`, `rolcanlogin`, `rolreplication`, `rolbypassrls`, `rolconnlimit`, and `rolvaliduntil`. The query never reads `rolpassword`.
- `ACL` lines hold one line per item of `aclexplode(datacl)`, with the names of the grantor and the grantee. The item order of `datacl` then does not count.
- `MEMBER` lines hold the names of the role, the member, and the grantor, and the three options of `pg_auth_members`.
- No line holds an OID or a password hash. The two clusters have other OIDs, and a password hash must never reach the event log (`reliability.md` 1.1, 7.7).
- `GRANT` lines use the format of `crates/recipe-db/tests/grants.expected`. The storage test also compares them with that file, so the two files cannot drift.
- The storage test `crates/recipe-db/tests/storage/restore_roles.rs` runs the query on the harness cluster. It drops the two known lines of `support/02-test-hosts.sh` before the comparison. The harness gets no separate hba path.
- `restore-volume` and the storage test use the same query file, as `restore-code.sql` does (ADR-0067 (j)).

### D6. Compose and the environment file

- `volumes.pgdata` becomes `external: true` and `name: ${RECIPE_PGDATA_VOLUME:?}`.
- `deploy/.env.example` adds `RECIPE_PGDATA_VOLUME=recipe-editor_pgdata_20261030_120000`. A comment gives the value `recipe-editor_pgdata` for an existing installation (ADR-0103 (d)).
- The service `restore-volume` copies the shape of `pitr-check`. It adds `backups:/backups:ro`, `tmpfs: /run/pgpass`, and `COMPOSE_SUBNET: ${COMPOSE_SUBNET:?}`. It mounts no `pgdata`. The operator gives the target with `-v`.
- `deploy_values` treats `restore-volume` like `pitr-check`: no network, no secret, the limit row of `container.backup_memory` and `container.small_cpus`. Its script joins `SCRIPTS`. A new check reads its `scratch.*` sizes and its two `restore_volume.*` waits, and refuses an `fsync` option.

### D7. Guard of the system identifier

- One file `deploy/pgtools/lib/wal-identity.sh` holds the guard. `walarchive-entry` runs it before `pg_receivewal`. `basebackup-once` runs it before `pg_basebackup`.
- The guard reads the server identifier with `IDENTIFY_SYSTEM` over the replication connection of the script.
- A segment exists when one name at the top of `/wal` matches the segment grammar or the `.partial` form of ADR-0099 (d).
- The guard writes a new file under a temp name and publishes it with `ln -T` without `-f` (ADR-0074 (b)). When the link fails, it reads the file again and compares. This handles two first starts at the same time.
- The file holds one line: the decimal identifier.
- The guard refuses a malformed file with its own message. A malformed file is empty, holds a CR, holds a character other than an ASCII digit, or holds more than one line. The runbook section "Messages of walarchive" names that message.
- A gap of the WAL stream moves the segments and keeps the file. The guard then finds the file and no segment, compares the identifier, and passes.

### D8. Step for an existing installation

The first start of the new `walarchive` image finds segments and no file, so it refuses to start. ADR-0103 (s) and `reliability.md` 7.10 step 5 therefore give one step before command 1 of that step. The step runs only when `/wal/.system-identifier` is missing. The release notes and the runbook section "Upgrade" use this form:

1. Read `SELECT system_identifier FROM pg_control_system()` in `db` as `postgres`.
2. Pipe the value into the old `walarchive` container.
3. That container finds the newest complete segment in `/wal` by the segment grammar, with `LC_ALL=C`. It reads `xlp_sysid` from the long page header of the first page: 8 bytes at offset 24, little-endian.
4. A difference between the two values stops the step with one `ERROR` line. The step then writes no file.
5. Else the container writes `/wal/.system-identifier` with `umask 077`, a temp name, and `ln -T`.

The step checks the value and does not trust the volume name. A wrong volume name at the upgrade gives `db` the identifier of another cluster, and step 4 then stops (ADR-0103 (d), (s)).

When the old `walarchive` container is already gone, the step runs in `docker compose run --rm --no-deps --entrypoint sh walarchive` with the previous tag in `deploy/.env`. Steps 3 to 5 stay the same.

The CI job `upgrade` runs the same step when the file is missing. A previous release with the guard already has the file, so the job then skips the step.

`reliability.md` 7.10 step 2 sets `RECIPE_PGDATA_VOLUME` before the first Compose command of the upgrade. Then it compares the name of `pgdata` in `docker compose config` with the volume that `docker inspect` of the running `db` shows for `/var/lib/postgresql/data`. A difference stops the upgrade (ADR-0103 (d)).

### D9. CI job `restore-switch`

- A new script `deploy/ci/restore-switch.sh` has the subcommands `prepare`, `restore`, `switch`, `access`, `wal-line`, `check`, `refuse`, `switch-back`, `keep`, and `cleanup`. The job in `ci.yml` copies the trigger, the environment, and the build steps of the job `compose` (ADR-0094).
- The job follows ADR-0103 (b), (f), (g), and (s) in these steps:
  1. `prepare` starts the stack, logs in, and writes data with `http_probe`. Then it runs `backup-now` and `restore-test`.
  2. `restore` creates a new volume and runs `restore-volume` while `db` runs. `switch` runs the switch of ADR-0103 (f).
  3. `access` runs the checks of the client rules and the login checks before the passwords. Then it sets the passwords. `wal-line` starts the new WAL line of ADR-0103 (s).
  4. `check` starts `app`. It checks `/readyz`, a login, an export hash, and the counts of the set on the new line. It checks the chain query of `restore-test` and `pitr-check` on the new line.
  5. `refuse` runs the negative cases: `restore-volume` on the live volume, and `up` with a missing external volume.
  6. `switch-back` returns to the old volume. It checks the old data and the WAL line again. `keep` runs `docker compose down -v` and checks that both data volumes stay.
- A CI proof of the major upgrade with an image of a newer major version is out of scope (ADR-0103 (p)).
- The job compares `postgresql.conf` and `postgresql.auto.conf` of the old cluster of `01-roles.sh` and the new cluster of `restore-volume`. It runs the comparison after the clean stop of each cluster (ADR-0103 (m)). It compares the whole files. A filter of lines, for example of the `lc_*` lines, is not permitted. A storage test has no cluster of `restore-volume`, so no storage test compares these files.
- Every external volume of a CI run has the label `recipe.ci=<project name>`. `deploy/ci/cleanup.sh`, `upgrade.sh cleanup`, and `restore-switch.sh cleanup` delete only volumes with that label.

### D10. CI step of the major upgrade

- `deploy/ci/migration_freeze.py` reads the major version of the `postgres:<major>.` image line in `deploy/compose.yaml` at `HEAD` and at the previous tag. The previous tag is `git describe --tags --abbrev=0 --match 'v*' HEAD^`.
- When the major versions differ, the step lists `git diff --name-only --diff-filter=A <tag> HEAD -- migrations/` and fails on each name. A unit test `deploy/ci/test_migration_freeze.py` covers both branches. `deploy/ci/check.sh` runs it.
- The step runs in a job without Docker on each pull request and each push, with `fetch-depth: 0`. The image line of `db` is the one source of the major version. No field of the release notes takes part.

### D11. Runbook

- The sections "Restore into production", "Rollback", and "Major upgrade" get the steps of ADR-0103 (b), (f), (g), (q), and (s), with commands. Each first line that says "incomplete" goes away.
- "Host preparation" creates the first volume. "Create the environment file" names nine values. "Change a setting" names nine values. "A gap in the WAL stream" keeps `/wal/.system-identifier` in place (ADR-0103 (s)). "Low disk space" and "Read-only mode" read the volume name from `.env` and hold no literal `recipe-editor_pgdata`.
- "Upgrade" gets the name step and its check in step 2. It also gets the one replacement of `db` in step 3 command 1, and D8 before step 5 command 1.
- "Rollback" gets the rule of the migration plan, step 4: never use the 0.1.0 `compose.yaml` after a switch.
- `README.md` and the dev-stack line of `CLAUDE.md` add `docker volume create "$RECIPE_PGDATA_VOLUME"` before the first `up`.
- The runbook follows ADR-0103 (t) for the delete of an old `pgdata` volume. "Destructive commands" replaces the line "Never run `docker volume rm` on `pgdata`" with the delete rule of ADR-0103 (t). "An older volume" replaces the line "The runbook never deletes `pgdata`" with a link to that rule.
- "Known gaps" loses the lines of the volume switch and of the eight values. The lines of 0.1.0 sets, the residual risk, and the WAL recovery stay.

### D12. Agents

No task runs on Codex now. Every task except a `[mechanical]` task runs on a Sonnet agent with effort high, one task per agent. A `[mechanical]` task runs on Haiku. The STE checks run on Sonnet. Opus reviews only.

## Risks / Trade-offs

- [The refactor of `restore-test` changes a gate by mistake] → The first library task changes no test case. `restore-test-test` must pass unchanged before the next task starts.
- [The stub suites cannot prove `initdb`, `pg_ctl`, `pg_restore`, and `pg_controldata`] → The CI job `restore-switch` runs the real tools on a real stack.
- [The harness and production differ in `pg_hba.conf`] → The storage test drops only the two known lines of `02-test-hosts.sh` and compares the rest.
- [The guard refuses the old segments, and the job `upgrade` fails] → The task of D8 lands in the same push as the guard.
- [The first upgrade of a site misses the step of D8] → `walarchive` refuses to start and writes an `ERROR` line. The alert `replication` becomes active. The runbook row of `walarchive` names the step. No data is lost, because `db` keeps `pg.wal_keep_size`.
- [A wrong `RECIPE_PGDATA_VOLUME` on an existing installation] → Compose stops on a missing volume. For an existing volume, `reliability.md` 7.10 step 2 compares the name with the mount of the running `db`. This check runs before any Compose command that starts or replaces a container. A difference stops the upgrade. The segment check of D8 is a second stop. The CI job `upgrade` runs both checks and one mutation with a wrong existing volume (task 6.4).
- [The new mount of `db` replaces `db` at the first upgrade] → The replacement keeps the volume and the cluster. `app` loses its connections for the start time of `db`. The release notes and the runbook section "Upgrade" state this. The CI job `upgrade` checks the volume name and the system identifier of the new container (task 1.5).

## Migration Plan

1. The release note names four manual steps of an existing installation. Set `RECIPE_PGDATA_VOLUME` and check it in step 2 of `reliability.md` 7.10. Run the step of D8 before the new `walarchive` starts. Keep `docker compose down -v` forbidden. Expect one replacement of `db` in step 3 command 1.
2. The release holds no migration file. The schema of `db` does not change.
3. A rollback of this release to 0.1.0 is safe only while `RECIPE_PGDATA_VOLUME` is `recipe-editor_pgdata` and no switch ran. The 0.1.0 `compose.yaml` mounts the Compose volume `recipe-editor_pgdata`. The file `/wal/.system-identifier` stays, and the old scripts ignore it.
4. After a switch, never use the 0.1.0 `compose.yaml`. It starts `db` on the old cluster and serves old data without an error. After the delete of the old volume, it makes an empty cluster. The 0.1.0 `walarchive` has no guard and streams another system into `/wal`. The release notes and the runbook section "Rollback" state this rule.
