# ADR-0084: Code of a generated column in the restore of `pre-data`

Status: Accepted
Date: 2026-10-24
Deciders: project owner

## Context

ADR-0067 (k) says that the restore of the section `pre-data` runs no code of ADR-0067 (h). Its first risk says that step 4 stops a changed definition "before that code runs". The review of `deploy/pgtools/bin/restore-test` found that this premise is false for a generated column:

1. Step 4 runs `pg_restore --section=pre-data --create` on the dump of the production database. The role is the superuser of the scratch cluster.
2. `CREATE TABLE` with a generated column runs the planner on the column expression. The planner executes an `IMMUTABLE` function call with constant arguments (`heap.c` and `clauses.c` of `REL_16_STABLE`).
3. The comparison of the function bodies (`compare_code`) runs after that statement. A changed body of a known function can therefore run before the comparison.
4. A `CHECK` constraint does not start the planner at `CREATE TABLE`. The fault applies to the generated column only.

The conditions of an attack:

- The attacker holds the migrator credential. ADR-0067 names this attacker.
- The attacker changes the body of a known `IMMUTABLE` function. The attacker also adds or changes a generated column that calls that function with constant arguments.
- The attacker changes the production database directly with the credential, not through a reviewed migration.
- The release dump is safe. Its two generated columns, `unit.parent_path` and `unit.depth`, call only built-in functions of `pg_catalog`.

The effect of an attack:

- The code runs in the scratch server inside the service `backup`. It runs as the superuser of the scratch cluster and as uid 999.
- A superuser can write files as uid 999. Two examples are `lo_from_bytea` with `lo_export`, and `COPY ... TO PROGRAM` in a `VOLATILE` function that an `IMMUTABLE` function calls.
- uid 999 can write to `/backups`. The code can therefore change or delete the sets on `/backups`.
- The code can write `pg_proc.prosrc` in the scratch catalog before `compare_code` reads it. The comparison of step 4 then finds no change, and the test can pass with no `ERROR` line.
- The code can also keep the restore of `pre-data` from ending. Then the comparison never runs.
- `backup` joins only the network `backend`, which is `internal: true`. The scratch server listens on `127.0.0.1` only.
- `backup` does not mount the volume `walarchive`. The code cannot write that volume.
- A site procedure copies the latest dump to the external backup storage (`reliability.md` section 7.7). ADR-0066 and ADR-0070 do not describe that procedure. The off-site copy therefore stays safe only while the procedure does not copy a set that the code rewrote.

## Decision drivers

- The project gives data safety priority over performance. A dump with unknown code never restores without an alarm (R31, R38).
- The attack needs a stolen migrator credential. With that credential, the attacker can already change the production database.
- The volume `walarchive` stays the last defense of ADR-0067.
- The documents state the real order of the restore and of the comparison, and the real limits of the detection.

## Considered options

### Option A: An offline check of the function bodies before `initdb`

- Pros: the planner of `pre-data` runs only the function bodies of the release.
- Cons: a new release list, a new round of task 8.3, and new storage tests. The text of a dump entry must stay the same between the harness and production.

### Option B: Accept the risk and record it

- Pros: no script and no test of P4.1 changes. The documents name the residual risk.
- Cons: a changed body of a known function can run as the scratch superuser before the comparison of step 4. That code can write files as uid 999 and can hide the change from the comparison.

## Decision

The project uses **Option B** (user decision of 2026-10-24).

Residual risk:

- (a) The restore of `pre-data` can run the code of a generated column before step 4 compares the function bodies. A changed body of a known `IMMUTABLE` function then runs as the superuser of the scratch cluster.
  - The code can write the files that uid 999 can write, for example the sets on `/backups`.
  - The code can write `pg_proc.prosrc` in the scratch catalog before the comparison. It can also keep the restore from ending. The detection by step 4 is therefore not guaranteed.
  - When the code does neither, the comparison finds the changed lines. The test then stops with an `ERROR` line before the data load.
- (b) The project accepts this risk. These conditions keep the risk small. They do not limit what the code can do after it starts:
  - The attack needs the migrator credential and a new or changed generated column.
  - The release dump has no generated column that calls a function outside `pg_catalog`. This condition keeps the release clean. It does not stop the attacker, because the attacker changes the production database directly.
  - The scratch cluster holds no production password. Its superuser password is random (ADR-0067 (c)).
  - `backup` joins only the internal network `backend` and has no route outside Compose (ADR-0070 (a)).
  - `backup` does not mount the volume `walarchive`, so the code cannot change it.
  - Nothing protects the sets on `/backups` from the code. The off-site copy stays safe only while the site procedure does not copy a set that the code rewrote.

Amendment of ADR-0067:

- (c) This item amends ADR-0067 (k) and the first risk of ADR-0067.
  - The restore of `pre-data` runs no `CHECK` constraint and no trigger. It can run the code of a generated column, because the planner folds a constant call to an `IMMUTABLE` function.
  - The comparison of step 4 stops the data load on a changed definition of ADR-0067 (h). It does not stop the code of a generated column in `pre-data`. That code can also hide its change from the comparison (item (a)).
  - The words "before that code runs" of the first risk of ADR-0067 apply only to the code that the data load runs.

Rule for migrations:

- (d) A migration that adds or changes a generated column or a column default calls only built-in functions of `pg_catalog` in that expression.
  - The review of each migration checks this rule. `AGENTS.md` and `.claude/rules/sql.md` hold the review item.
  - A migration that needs another function in such an expression needs a new ADR.
  - The rule also covers the `DEFAULT` expression that ADR-0078 names as a residual risk.
  - The rule keeps the release clean. It does not stop an attacker who changes the production database directly with the migrator credential.

Follow-up:

- (e) P4.2 may close the gap in one of two ways. Each way needs a new ADR that amends this ADR. This change does not reopen Option A.
  - Option A: an offline check of the function bodies before `initdb`.
  - A restore of `pre-data` by a role that is not a superuser. That role cannot call `lo_export`, cannot run `COPY ... TO PROGRAM`, and cannot write `pg_proc`.
  - P4.2 may also add a storage test of rule (d). After all migrations, no row of `pg_depend` has `classid = 'pg_attrdef'::regclass` and `refclassid = 'pg_proc'::regclass`. The test keeps the release clean. It does not stop the attacker of (a).
  - P4.2 may also define how the site procedure keeps a set that the code rewrote out of the off-site copy.

Documents:

- (f) Step 4 of `reliability.md` section 7.7 names the residual risk of (a) and the limit of the detection.

## Consequences

### Positive

- The documents state the real order of the restore and of the comparison, and the real limits of the detection.
- The rule of (d) keeps the release dump free of project code in the restore of `pre-data`.
- P4.1 needs no new round of task 8.3.

### Negative

- An attacker with the migrator credential can run code as the scratch superuser in `backup` before the comparison of step 4.
- That code can write the files that uid 999 can write, for example the sets on `/backups`.
- That code can read the files that uid 999 can read, for example the secret `backup_password` and the files of `pgdata`. ADR-0067 names this read access of `backup` as a residual risk.
- That code can hide its change from the comparison of step 4 or keep the restore from ending. The weekly run can then end with no `ERROR` line, or it can give no result.

### Risks and mitigations

- Risk: the code damages the sets on `/backups` and hides the change from step 4. Mitigation: the volume `walarchive` stays the last defense (ADR-0067), because `backup` does not mount it. The off-site copy is a defense only while it holds no set that the code rewrote. No item of this ADR guarantees that the weekly run shows a fault.
- Risk: a migration adds a generated column that calls a project function. Mitigation: the review item of (d). The storage test of ADR-0067 (j) also shows the new line of `restore-code.expected`.
- Risk: a partition key expression may use the same planner step. Mitigation: the release has no partitioned table. A migration that adds one needs a new ADR.

## Related

- Amends ADR-0067 (k) and the first risk of ADR-0067.
- Amends ADR-0078 by item (d): a mitigation of the residual risk of a `DEFAULT` expression.
- ADR-0066 (the sets on the backup disk), ADR-0070 (the network `backend`), ADR-0076 (c) (the kinds of `pre-data`), ADR-0078 (residual risks).
- `docs/architecture/reliability.md` section 7.7 (step 4 of the restore test, the site procedure of the off-site copy).
- `deploy/pgtools/bin/restore-test` (`restore_section`, `compare_code`).
- `migrations/0001_schema.sql` (`unit.parent_path`, `unit.depth`, `dims_valid`).
- Source `src/backend/catalog/heap.c` (`cookDefault`) and `src/backend/optimizer/util/clauses.c` (`evaluate_function`) of `REL_16_STABLE`.
- PostgreSQL 16 documentation, section "Function Volatility Categories" (a `STABLE` or `IMMUTABLE` function can call a `VOLATILE` function).
- Baseline requirement: `operations` "Restore test".
- `overview.md` section 7: R31, R38. Section 9: A16.
