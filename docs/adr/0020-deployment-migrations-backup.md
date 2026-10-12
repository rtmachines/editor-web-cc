# ADR-0020: Deployment, migrations, backup and failure policy

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

ADR-0003 chose Docker Compose on one host and said that the application runs migrations at start. The design review found three defects (one high finding each of the PostgreSQL and security reviews, PostgreSQL M7, architecture F16, traceability B71 to B73). Migrations at start contradicted `reliability.md` section 5. Migration `0001` could not create roles. The SQL seed could not read environment variables.

ADR-0003 also recorded the reading of a broken draft sentence. This ADR restates it (Assumption A1). The draft says: "Use PostgreSQL and backend service in a Docker container as the database". It is read as: PostgreSQL is the database, and the database and the backend run in Docker containers.

The same review found more defects:

- The only backup was a daily dump on the same disk as `pgdata` (architecture F10).
- The dump contained session tokens and password hashes in clear (a high finding of the security review).
- The low-disk check used `pg_stat_file`, which cannot report free space (architecture F17, PostgreSQL M9).
- `panic = "abort"` turned one bad stored value into a crash loop for all users (architecture F8).
- The data directory had checksums off and a libc collation (PostgreSQL M8).
- The health endpoints were defined twice (security M9, traceability B69).

The owner accepted WAL archiving to a second disk and the panic policy "unwind with a panic layer".

## Decision drivers

- 24/7 operation on one on-premises host (R30, A1).
- Data safety over performance (R31).
- The runtime container must not hold schema-owner credentials.
- One normative description of the services.

## Considered options

### Option A: Migrations in the application at start

- Pros: one container.
- Cons: the app holds the migrator secret, a failed migration becomes a restart loop, and the roles cannot be created this way.

### Option B: One-shot `migrate` service, roles from the database init script, Rust bootstrap of the first user

- Pros: least privilege at runtime, a failed migration stops the start without a loop, the seed can hash a password.
- Cons: one more service and a start order.

### Option C: Manual migrations by an operator

- Pros: full control.
- Cons: not reproducible, and the runbook grows.

## Decision

The project uses **Option B**.

Services (`deploy/compose.yaml`):

| Service | Role | Restart |
|---|---|---|
| `proxy` | Caddy. TLS termination. Mandatory. Re-encrypts to upstream when the TLS termination point of the plant is elsewhere. HSTS. Health paths return 404 on the public site | always |
| `backup-premigrate` | One-shot `pg_dump` before `migrate` | no |
| `migrate` | `recipe-server migrate`. Holds the migrator secret. Runs after `db` is healthy and `backup-premigrate` exited 0 | no |
| `app` | `recipe-server serve`. Starts after `migrate` exited 0. `expose` only, no `ports` | always |
| `db` | `postgres:16.x-bookworm`, pinned. `POSTGRES_INITDB_ARGS="--data-checksums --lc-collate=C --lc-ctype=C.UTF-8"`. Port 5432 not published | always |
| `backup` | Daily `pg_dump --format=custom --exclude-table-data=session --exclude-table-data=login_attempt` at 02:00 UTC plus `pg_dumpall --globals-only`. Retention 30 days. `backup-now` one-shot command | always |
| `walarchive` | `pg_receivewal` to a bind mount on a second disk. Weekly `pg_basebackup`. Both connect as `recipe_replicator` | always |

Roles and secrets:

- `deploy/db/initdb/01-roles.sh` runs as superuser on the first start. It creates `recipe_migrator`, `recipe_app`, `recipe_backup`, `recipe_replicator` (`LOGIN REPLICATION`, password from the secret `replicator_password`), `audit_owner` (NOLOGIN) and the database. It adds the `pg_hba.conf` line `host replication recipe_replicator <compose subnet> scram-sha-256`. Migrations never create roles.
- `recipe_migrator` is a member of `audit_owner` with `INHERIT FALSE`, so that `0003_grants.sql` can transfer the audit objects to `audit_owner` (`data-model.md` 5.1). `0003` runs every grant on these objects first, then the ownership transfer. It uses no `SET ROLE` and needs no sequence grant. After the transfer the migrator holds no privilege on the audit objects. CI applies `0001` to `0004` as `recipe_migrator` on a fresh cluster and then runs the grants test.
- Secrets are Docker secrets (`*_FILE`) only. No env file for passwords. The superuser password exists only in `db`.
- Two more secrets hold keys of 32 bytes: `login_hmac_key` for `security_event.login_hmac` and `csrf_signing_key` for the pre-session CSRF cookie. Rotation: the operator adds a new key, both keys are accepted for `security.key_rotation_overlap`, and then the operator deletes the old key (`reliability.md` 7.1).
- The backup uses `recipe_backup` with `pg_read_all_data`. The dump has a SHA-256 manifest. A restore verifies the manifest first.
- `recipe-server create-admin` seeds the first Administrator from stdin or a secret file.

Migrations:

- Folder `migrations/` at the repository root. Files `NNNN_description.sql`. `0001_schema`, `0002_triggers`, `0003_grants`, `0004_seed_plc_types`.
- Forward-only. `lock_timeout = 2s` with retry. A file that needs `CREATE INDEX CONCURRENTLY` starts with `-- no-transaction`. CI rejects table-rewriting DDL on every table (`reliability.md` 5).
- `ALTER DEFAULT PRIVILEGES` gives `recipe_app` SELECT on future tables.

Failure policy:

- Panics unwind. `CatchPanicLayer` returns 500 with the `request_id` and logs at ERROR. Background jobs are supervised and restarted. The draft words "must not crash" are read as "contain and restart" (Assumption A41).
- A watchdog thread probes the server every 10 seconds. After 3 failures in 30 seconds it exits the process, and Docker restarts the container.
- `overflow-checks = true` stays. An overflow is a caught panic, not a wrong value.

Health and disk:

- `/healthz` and `/readyz` are served on an internal listener on port 9000. The body is `ok` or `not ready`. Readiness is cached for 5 seconds and does not include the disk check.
- Free disk space is read with `statvfs` on a read-only mount of `pgdata` inside `app`. Below 5 % free, the application enters read-only mode. `Login`, `Logout`, every query, and every export continue. Every other command returns 503 `E-WEB-006`. The only audit actions in read-only mode are `UserLoggedIn`, `UserLoggedOut`, and `VersionExported`. Above 10 % the mode ends.
- The `backup` healthcheck fails when the backup path is on the same filesystem as `pgdata`.

Alerts:

- A supervised background job `alerts` inside `app` checks the conditions of `reliability.md` 3.7. It writes to the event log and to an optional webhook.
- The `backup` container runs no alert cron, because its image has no `recipe-server` binary.

Major upgrade:

- A major PostgreSQL upgrade is a dump and restore into a new `db` volume with the same `initdb` arguments. The project does not use `pg_upgrade`.

## Consequences

### Positive

- Recovery point is minutes, not a day.
- The runtime role cannot alter the schema or the audit log.
- One bad value makes one request fail, not the whole plant.

### Negative

- Seven services instead of four. The runbook describes the start order.
- The second disk for the WAL archive is a hardware requirement (A35).

### Risks and mitigations

- Risk: the second disk is missing. Mitigation: the `backup` healthcheck fails when the backup path is on the same filesystem as `pgdata`. The runbook records RPO 24 h as the fallback.
- Risk: PostgreSQL 16 support ends in November 2028. Mitigation: the runbook has a dump-and-restore procedure into a new volume, and the backup includes the globals.

## Related

- Supersedes ADR-0003.
- ADR-0014 (create-admin), ADR-0022 (audit_owner).
- `reliability.md` sections 2, 5, 7.
- `data-model.md` sections 5 and 8.
- Review findings: security M6, M9 and two high findings. Architecture F8, F10, F16, F17. PostgreSQL M7, M8, M9 and one high finding. Traceability Pass A, item 20. Traceability B69 to B76, C12, C13, C20.
