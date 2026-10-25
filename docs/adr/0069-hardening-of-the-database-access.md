# ADR-0069: Hardening of the database access

Status: Accepted
Date: 2026-10-23
Deciders: project owner

## Context

Step P4.1 runs `serve` on the Compose database for the first time. The reviews of steps P2.4 and P2.5 deferred these points to this step, and the plan review of P4.1 found more:

- `recipe-db` builds `PgConnectOptions::new()`. sqlx 0.8.6 then reads twelve `PG*` environment values, the pgpass file of libpq, and the TLS mode `prefer`.
- No setter of sqlx 0.8.6 clears the value of `PGOPTIONS`. `std::env::remove_var` is `unsafe` in edition 2024, and every crate has `#![forbid(unsafe_code)]`.
- `recipe_app` holds `pg_read_all_stats` for the replication lag of `reliability.md` section 3.7. The role then also reads the statement text of every session in `pg_stat_activity`.
- The purge of `security_event` and the two alert sums of section 3.7 filter on `occurred_at`. No index serves them.
- `recipe-server migrate` has no connect timeout.
- The image of `db` appends the line `host all all all scram-sha-256` to `pg_hba.conf`. Every role, also the superuser, can then log in over TCP from each container and from the Docker host.
- The superuser password has a secret file but no key of `reliability.md` section 7.11. ADR-0031 says: "Each password has one secret file and one key".

## Decision drivers

- Least privilege of every role (R34).
- No value outside the configuration and the secret files changes a connection (R38).
- The audit objects stay under the guard of ADR-0034.
- No `unsafe` code (`reliability.md` section 2).
- A migration rewrites no table and scans no table under an exclusive lock (`reliability.md` section 5).

## Considered options

### Option A: Refuse `PG*` values, a definer function for the replication state, and an own `pg_hba.conf`

- Pros: no environment value reaches a connection. `recipe_app` reads one row of the replication state and no statement text. Only the four login roles connect over TCP, and only from the subnet of `backend`.
- Cons: a host shell with a `PG*` value stops the start. A migration adds one more audit object.

### Option B: Keep the current access and record the risks

- Pros: no change of the database or of the start.
- Cons: `PGOPTIONS` can change each session of `recipe_app`. `recipe_app` keeps the statement text of every session. The superuser keeps a TCP login.

### Option C: TLS from each client to `db`

- Pros: TLS protects the traffic inside the Compose network.
- Cons: `db` needs a certificate and a rotation. The `PG*` values and `pg_read_all_stats` stay open. No document designs the certificate.

## Decision

The project uses **Option A**.

Connections:

- (a) `recipe-db` builds its connect options without the pgpass file of libpq. Code sets every field. The TLS mode is `ssl_mode = disable` inside the Compose network. A later TLS to `db` is a new ADR.
- (b) `serve`, `migrate`, and `create-admin` refuse to start when a variable whose name starts with `PG` is set. The check runs directly after the logger starts, before every other step (ADR-0061 step 2).
- (c) The refusal writes one `ERROR` line with the name of the variable and not its value. The exit code is 1. The subcommand `healthcheck` opens no database connection and has no such check.
- (d) The new row `migrate.connect_timeout` holds 10 s. `migrate` writes one `ERROR` line and exits with code 1 when it cannot connect within that time.
- (e) The new row `secrets.postgres_password_file` holds `/run/secrets/postgres_password`. Only the image of `db` reads it.

Replication state:

- (f) `01-roles.sh` grants `pg_read_all_stats` to `audit_owner` with `INHERIT TRUE`. The script no longer grants it to `recipe_app`.
- (g) The migration `0006_operations.sql` creates the function `public.replication_state()` as `audit_owner`. It is `SECURITY DEFINER` with `search_path = pg_catalog, public, pg_temp` and qualifies every name (`.claude/rules/sql.md`).
- (h) The function reads `pg_stat_replication` only for `usename = 'recipe_replicator'` and `application_name = 'walarchive'`. It returns one row `connected boolean, lag_bytes numeric`.
- (i) `0006` revokes `EXECUTE` on the function from `PUBLIC` and grants it to `recipe_app`. A storage test checks with `has_function_privilege` that `PUBLIC`, `recipe_backup`, and `recipe_migrator` cannot run it.
- (j) The job `alerts` reads the replication condition of section 3.7 through this function (ADR-0068 (p)).

Index and migration:

- (k) `0006` creates `security_event_occurred_at_idx ON security_event (occurred_at)`. The index serves `security_event_purge()` and the two alert sums of section 3.7.
- (l) A plain `CREATE INDEX` is permitted, because `security_event` is small at this release. A storage test with 50 000 rows checks with `EXPLAIN` that the three statements use the index.
- (m) `0006` runs the five steps of `data-model.md` section 5.1 in one file. Step 3 creates the index and the function. A migration therefore also runs `SET ROLE audit_owner` to create an object that `audit_owner` owns.
- (n) `data-model.md` gets the new section 5.5 with the DDL and the grant of `0006`, as section 5.4 holds `0005`. Section 3.8 gets prose only, so the document test of `0001` stays green.

Guard and grant test:

- (o) The guard list of `audit_guard.audit_objects_dropped()` in `01-roles.sh` gets `public.replication_state()`. `audit_owner` then owns 8 objects.
- (p) Only the superuser can replace the guard function. For an older volume, the runbook replaces it as the superuser.
- (q) `grants.expected` gets routine rows, for example `replication_state()|EXECUTE` and the row of `security_event_purge()`. The grant test of `data-model.md` section 5.3 also reads `information_schema.role_routine_grants`.

Client rules:

- (r) `01-roles.sh` writes a new `pg_hba.conf` in place of the catch-all line of the image. The socket line admits `postgres` only, with the method `peer`.
  - Only a process of the system user `postgres` in the container can then log in through the socket. A process of `root` cannot.
  - A runbook step runs `psql` in `db` as the system user `postgres`, for example `docker compose exec -u postgres db psql`.
- (s) The `host` lines admit `recipe_migrator`, `recipe_app`, and `recipe_backup` to `recipe`, and `recipe_replicator` to `replication`. Each line admits only `${COMPOSE_SUBNET}` with `scram-sha-256`.
  - One more line `host recipe pg_isready 127.0.0.1/32 scram-sha-256` serves the healthcheck of `db` (ADR-0070 (q)). It stands before the reject line of (t).
  - No role `pg_isready` exists, and `01-roles.sh` never creates one. No login with this name therefore passes.
  - The probe ends at the password request of the server, so `db` writes no `FATAL` line.
- (t) A final line rejects every other connection.
- (u) The production script holds no test switch. The storage harness mounts a test-only init script that puts the host lines of the harness at the top of `pg_hba.conf`.
- (v) `01-roles.sh` runs only on an empty volume. For an older volume, the runbook gives the two grant statements, the new guard function, and the new `pg_hba.conf` as the superuser. ADR-0070 gives the host firewall check.

## Consequences

### Positive

- No `PG*` value and no pgpass file can change a connection of the server.
- `recipe_app` reads one row of the replication state and no statement text of another session.
- The purge and the alert sums read the index and not the whole table.
- The superuser has no TCP login. A container outside `backend` cannot log in.

### Negative

- A host shell with a `PG*` value stops the start of each of the three subcommands. The runbook names the check.
- The traffic from `app` to `db` stays plain TCP inside the network `backend`. ADR-0070 limits that network.
- An older volume needs manual steps of the superuser.

### Risks and mitigations

- Risk: the migrator credential can run `SET ROLE audit_owner` and then read `pg_read_all_stats`. Mitigation: this is the residual risk of `data-model.md` section 5.1. The credential exists only in the one-shot `migrate` container.
- Risk: a later audit object misses the guard list. Mitigation: the grant test compares the guard list with the objects of `audit_owner` (ADR-0034).

## Related

- Amends ADR-0020 (`recipe_app` reads the replication lag through `replication_state()`, and `01-roles.sh` writes the whole `pg_hba.conf`).
- Amends ADR-0034 (the guard list gets `public.replication_state()`).
- ADR-0031 (secret files and keys), ADR-0035 (guard of `security_event`), ADR-0068 (replication state), ADR-0070 (networks and the host firewall).
- `data-model.md` sections 3.8 (prose of the index) and 5.1 (the purposes of `SET ROLE audit_owner`, `pg_read_all_stats`, `pg_hba.conf`).
- `data-model.md` sections 5.3 (routine grants), the new 5.5 (DDL and grant of `0006`), and 8 (the file `0006_operations.sql`).
- `reliability.md` sections 3.7 and 7.7 (the source of the replication condition), 5 (the client rules and the refusal of `PG*` values).
- `reliability.md` section 7.11: the new rows `migrate.connect_timeout` 10 s and `secrets.postgres_password_file` `/run/secrets/postgres_password`.
- Baseline requirements: `operations` "Roles from the database init script", "Least privilege of the application role", and "Configuration of the migrator".
- The delta adds `operations` "Database connection without environment values", "Client rules of the database", and "Connect timeout of the migrator".
- `overview.md` section 7: R20, R29, R34, R38. Section 9: A1.
