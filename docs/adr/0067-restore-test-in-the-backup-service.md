# ADR-0067: Restore test in the backup service

Status: Accepted. Amended by ADR-0076 (name list and trigger lines of the restore test). Amended by ADR-0083 (the restore test in the upgrade and the rollback, the image of the schema). Amended by ADR-0084 (item (k) and the first risk: the code of a generated column in the restore of `pre-data`). Amended by ADR-0095 (f) (the first risk: step 3 runs the globals file before the check of step 4). Amended by ADR-0103 (the steps of the restore test also serve `restore-volume`, item (b): `container.backup_memory` for its container). Amended by ADR-0105 (item (a): in step 3 of `reliability.md` 7.10, `restore-test` runs in a new container of the previous image)
Date: 2026-10-23
Deciders: project owner

## Context

`reliability.md` section 7.7 requires a restore test each week and before each release. The script restores the latest dump into a scratch database, compares the row counts, and runs the audit chain check. ADR-0033 adds a login with each role. Step P4.1 writes the script. Three rules of the documents cannot all hold for one design:

- The dump holds the event trigger `audit_objects_dropped` and the schema `audit_guard`. A restore of these objects needs a superuser.
- The superuser password exists only in `db` and in the initdb script (section 7.1).
- The event log is the stdout of a container (section 3.6). The output of `docker compose exec` does not reach the log of the container.
- `backup` has 128 MiB (`container.small_memory`). A scratch cluster and `pg_restore` need more.

The plan review found more points:

- `initdb` uses `trust` by default. A login with a wrong password then passes, and the login step proves nothing.
- A restore runs the code of the dump as the restoring role. An attacker with the migrator credential can plant a function that runs when the restore checks a constraint. A changed body of a known function has the same effect, and a check of the names does not find it.
- A scratch cluster with the default settings can exceed the memory limit of its container.

## Decision drivers

- A weekly test proves that a backup set restores, before an operator needs it (R31, ADR-0020).
- The restore test reads no production password except the password of `recipe_backup`.
- The result reaches the event log of `reliability.md` section 3.6.
- The restore test cannot change the production cluster or fill the disk of `pgdata` (A24).
- Every number comes from `reliability.md` section 7.11.

## Considered options

### Option A: A private scratch cluster inside the service `backup`

- Pros: the test needs no production superuser. Its output goes to the container log. Its data stays on the backup disk.
- Cons: `backup` needs 512 MiB. The test sets random passwords and does not use the production passwords.

### Option B: A scratch database in the cluster of `db` through `docker compose exec db`

- Pros: the superuser of `db` restores every object. `backup` keeps 128 MiB.
- Cons: the output of `exec` is not in the container log. The scratch data lands on the disk of `pgdata` and can start read-only mode (A24). An error of the script can reach the production cluster.

### Option C: An eighth service for the restore test

- Pros: the test has its own limits.
- Cons: it breaks the rule of seven services of ADR-0020 and `reliability.md` section 7.1.

## Decision

The project uses **Option A**.

Place and schedule:

- (a) The service `backup` runs `restore-test` every Sunday at 05:00 UTC through supercronic (ADR-0066). The new row `restore_test.time` holds the time. Before each release, the operator runs `docker compose exec backup restore-test`.
- (b) The new row `container.backup_memory` gives 512 MiB to `backup` and to the temporary container of `pitr-check` (ADR-0068). This is a change of the limit of `backup` from 128 MiB by a decision of the user. ADR-0070 changes the row `container.small_memory`.

Scratch cluster:

- (c) The script runs `initdb` as uid 999 in a scratch directory on the backup disk. It uses the `initdb` arguments of `reliability.md` section 7.1. It also uses `--auth-local=peer --auth-host=scram-sha-256` with a random superuser password file.
- (d) The scratch server listens on `127.0.0.1` only. Its socket is in a private directory with mode `0700`.
- (e) The scratch server uses these settings of section 7.11: `scratch.shared_buffers` 64 MB, `scratch.maintenance_work_mem` 64 MB, `scratch.max_connections` 10, `scratch.max_parallel_workers_per_gather` 0, and `scratch.max_parallel_maintenance_workers` 0.
- (f) Only the cluster of `restore-test` also uses `scratch.restore_fsync` `off`, `scratch.restore_wal_level` `minimal`, and `scratch.restore_max_wal_senders` 0. The cluster is disposable, so the restore runs faster on 0.5 CPU.
- (g) A `trap` deletes the cluster on each exit of the script.

Code of the dump (user decision):

- (h) The data load and the chain check run code of the dump as the superuser of the scratch cluster. This code is the body of each function, each `CHECK` constraint of a table or a domain, and each generated column.
- (i) The file `deploy/pgtools/restore-code.expected` holds one line for each of these objects outside `pg_catalog` and `information_schema`. A line holds the kind, the name, the owner, and the SHA-256 of the definition. The query `deploy/pgtools/restore-code.sql` builds the lines.
- (j) A storage test of `recipe-db` runs the same query on the harness database after `01-roles.sh` and every migration. A difference from the file fails CI, as for `grants.expected`. The pgtools image copies the query and the file.
- (k) The script restores the section `pre-data` first. This section runs no code of (h), because the tables are empty and no trigger exists yet. Then the script builds the lines of (i) in the scratch cluster. A `CHECK` constraint that `pg_dump` moves to `post-data` gives a missing line, and the test stops.
- (l) A missing, extra, or changed line stops the test before the data load.

Steps:

1. The script selects the newest complete set of ADR-0066 and runs `sha256sum -c --strict` on its manifest. A mismatch stops the test before the restore.
2. It runs `pg_restore -l` on the dump. It compares the entries `FUNCTION`, `TRIGGER`, and `EVENT TRIGGER` with a list that the release builds from `migrations/` and `deploy/db/initdb/01-roles.sh`. An unknown entry stops the test.
3. It restores the globals file. Then it sets a random password for each login role.
4. It runs `pg_restore --section=pre-data` into the scratch cluster. It compares the lines of (i) with `restore-code.expected`. A difference stops the test (l).
5. It runs `pg_restore --section=data` and then `pg_restore --section=post-data`.
6. It compares the row count of each table of the counts file with the restored table (ADR-0066 (g)).
7. It runs the audit chain check of `data-model.md` section 4.9 in SQL.
8. Each login role logs in over `127.0.0.1` with its random password. A login with a wrong password must fail.
9. The script writes one JSON line with the result. A failed step writes an `ERROR` line and ends the script with a code other than 0.

## Consequences

### Positive

- Each week proves that the newest set restores, that its counts match, and that its hash chain holds.
- The test proves the password step of ADR-0033 with a real check of the password.
- `backup` reads no production password except its own.
- The production cluster and the disk of `pgdata` stay untouched.

### Negative

- `backup` gets 512 MiB instead of 128 MiB. The sum of the limits on the host grows by 384 MiB.
- The scratch cluster needs free space on the backup disk for one restored database.
- The login step uses random passwords. It does not prove that the production secret files hold the right passwords.

### Risks and mitigations

- Risk: a dump runs code as the restoring superuser. Mitigation: step 2 stops the test on an unknown function or trigger. Step 4 stops it on a changed definition of (h), before that code runs. The `walarchive` volume and the off-site copy stay the last defense.
- Risk: a new migration changes a definition of (h) and the file of (i) stays old. Mitigation: the storage test of (j) fails CI before the release.
- Risk: `backup` runs as uid 999, the owner of the files of `pgdata`. The read-only mount of `pgdata` lets it read each raw file, also the SCRAM verifiers in `pg_authid`. Mitigation: none in this step. This ADR names the risk as a residual risk, because `backup` already reads every table through `pg_read_all_data`.
- Risk: a large plant database exceeds 512 MiB. Mitigation: the `scratch.*` settings bound the memory. The CI job measures the peak.

## Related

- ADR-0020 (seven services), ADR-0033 (globals dump and the login per role), ADR-0066 (file sets and schedule), ADR-0068 (`pitr-check`), ADR-0070 (limits).
- `reliability.md` section 7.7 (the restore test in a private cluster, the counts file, the login per role).
- `reliability.md` section 7.11: the new rows `restore_test.time` Sunday 05:00 UTC and `container.backup_memory` 512 MiB. The new rows `scratch.shared_buffers` 64 MB, `scratch.maintenance_work_mem` 64 MB, `scratch.max_connections` 10.
- `reliability.md` section 7.11: the new rows `scratch.max_parallel_workers_per_gather` 0, `scratch.max_parallel_maintenance_workers` 0, `scratch.restore_fsync` `off`, `scratch.restore_wal_level` `minimal`, `scratch.restore_max_wal_senders` 0.
- `overview.md` section 3 (the container `backup` runs the restore test).
- `data-model.md` sections 4.9 and 5.1. `crates/recipe-db/tests/grants.expected` (the model of the file of (i)).
- Baseline requirement: `operations` "Restore test".
- `overview.md` section 7: R31. Section 9: A16, A24.
