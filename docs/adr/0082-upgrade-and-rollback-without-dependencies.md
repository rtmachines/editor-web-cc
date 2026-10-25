# ADR-0082: Upgrade and rollback without the dependencies of app

Status: Accepted. Amended by ADR-0083 (the previous tag reaches only `app` in step 6)
Date: 2026-10-24
Deciders: project owner

## Context

`reliability.md` section 7.10 gives the upgrade procedure. Step 3 runs `docker compose up -d backup-premigrate migrate` and waits for exit code 0 of `migrate`. Steps 4 and 6 run `docker compose up -d app`. The review of `deploy/compose.yaml` found these faults in steps 4 and 6:

1. In `compose.yaml`, `app` depends on `db` (healthy) and `migrate` (completed successfully). `migrate` depends on `db` (healthy) and `backup-premigrate` (completed successfully).
2. The reference of `docker compose up` says: "Unless they are already running, this command also starts any linked services." A one-shot service that exited does not run. Step 4 therefore starts `backup-premigrate` and `migrate` again.
3. In step 6, the previous image tag also changes the images of `migrate` and `backup-premigrate`. Compose then recreates and starts both services.
4. The previous migrator finds applied migrations that its binary does not hold. `Migrator::run` in `crates/recipe-db/src/migrator.rs` runs without `ignore_missing`, so it fails. Then `app` does not start, and the rollback stops.
5. A second `_premigrate` dump in the same UTC minute finds a file with a name of the new set. `backup-once` then writes one `ERROR` line and stops (ADR-0066 (h) step 1). Then `migrate` and `app` do not start.

## Decision drivers

- A rollback after an expand migration starts the previous `app` on the newer schema (R30, R31, ADR-0059 (i)).
- One upgrade writes one `_premigrate` set before `migrate` (ADR-0020, ADR-0066).
- No migrator runs against a schema with migrations that its binary does not hold (R34).
- The runbook and the CI job use the commands of section 7.10.

## Considered options

### Option A: `--no-deps` in steps 4 and 6

- Pros: Compose replaces only the container of `app`. Step 3 stays the only step that runs `backup-premigrate` and `migrate`. The previous `migrate` never runs against the newer schema.
- Cons: Compose does not check the `depends_on` conditions of `app` in steps 4 and 6. The operator must complete step 3 first.

### Option B: The migrator ignores migrations that its binary does not hold

- Pros: steps 4 and 6 keep the short command.
- Cons: a previous migrator then accepts a schema with unknown migrations. Step 4 still writes a second `_premigrate` dump, and that dump fails in the same UTC minute. The code of `recipe-db` changes.

## Decision

The project uses **Option A**.

Upgrade:

- (a) Step 4 of section 7.10 is `docker compose up -d --no-deps app`. Step 3 already runs `backup-premigrate` and `migrate` and waits for exit code 0 of `migrate`.
  - Compose then replaces only the container of `app`. It starts no one-shot service again.
  - This item amends ADR-0020 (the start of `app` after `migrate`). In steps 4 and 6, step 3 and not Compose holds the start order.

Rollback:

- (b) Step 6 is `docker compose up -d --no-deps app` with the previous image tag.
  - Without `--no-deps`, Compose runs the previous `migrate` against the newer schema. The migrator fails, and `app` does not start.
  - With `--no-deps`, the previous `app` starts on the newer schema. A database with a newer migration keeps it ready (ADR-0059 (i)).
  - After a restore of the `_premigrate` dump, the operator starts the previous image with the same command.

Runbook and CI:

- (c) The runbook of task 1.5 writes steps 4 and 6 with the commands of (a) and (b). The CI job of task 8.7 uses the same command for each step of section 7.10 that it runs.
  - A first start from zero keeps `docker compose up -d`. Compose then runs the whole start order of section 7.1.

## Consequences

### Positive

- A rollback after an expand migration starts the previous `app`.
- One upgrade writes exactly one `_premigrate` set.
- No previous migrator runs against a newer schema.

### Negative

- Steps 4 and 6 depend on the order of the steps. Compose does not stop a step 4 without step 3.

### Risks and mitigations

- Risk: the operator runs step 4 before step 3. Then the new `app` starts on the old schema. Mitigation: the new `app` stays `not ready` (ADR-0059 (h)), and step 5 checks `/readyz`.
- Risk: the operator uses the command without `--no-deps`. Mitigation: the runbook gives the full command of each step.

## Related

- Amends ADR-0020 (the start of `app` after `migrate` in steps 4 and 6 of the upgrade procedure).
- ADR-0059 (h), (i) (readiness on a newer schema), ADR-0064 (j) (step 2 of the upgrade), ADR-0066 (h) (the `_premigrate` set), ADR-0070 (w) (CI job `compose`).
- `docs/architecture/reliability.md` sections 5 (start order) and 7.10 (steps 3, 4, and 6).
- `crates/recipe-db/src/migrator.rs` (`Migrator::run`). `deploy/pgtools/bin/backup-once` (step 1). `deploy/compose.yaml` (`depends_on` of `app` and `migrate`).
- Docker Compose reference of `docker compose up` (option `--no-deps`).
- `overview.md` section 7: R30, R31, R34. Section 9: A1.
