# ADR-0097: Names without a schema in the drop guard of the audit objects

Status: Proposed
Date: 2026-10-25
Deciders: project owner

## Context

ADR-0034 adds the event trigger `audit_objects_dropped`. Its function compares the `object_identity` of each dropped object with eight names in the schema `public`, for example `public.audit_log`.

The security audit of step P4.2 found a path around this comparison:

1. `recipe_migrator` owns the database `recipe`. Through `pg_database_owner` it also owns the schema `public`.
2. The owner of a schema can rename it. After `ALTER SCHEMA public RENAME TO moved`, the identity of the table is `moved.audit_log`.
3. `DROP SCHEMA moved CASCADE` then drops every audit object. The guard finds no name of its list and lets the drop pass.

The audit did not run this attack on PostgreSQL. A probe task runs it as a negative test before this ADR becomes Accepted.

`01-roles.sh` creates the guard before the migrations create the audit objects. The guard therefore cannot know the object ids.

## Decision drivers

- No role except `audit_owner` drops an audit object (R20, I13).
- The fix changes no migration that shipped (`data-model.md` section 8).
- The residual risk of the migrator credential stays as narrow as `data-model.md` section 5.1 says.

## Considered options

### Option A: Compare the object type and the name without the schema

- Pros: a rename of any schema keeps the protection. One function body changes. No migration and no grant changes.
- Cons: an object with a guarded name in another schema is also guarded.

### Option B: Compare the object ids

- Pros: the comparison does not depend on a name.
- Cons: the guard exists before the objects. A list of ids needs a table and a step of the superuser after each migration that adds an audit object.

### Option C: Take the schema `public` away from `pg_database_owner`

- Pros: `recipe_migrator` can no longer rename or drop the schema.
- Cons: `0003_grants.sql` and the five steps of `data-model.md` section 5.1 run `GRANT` and `REVOKE` on `public` as its owner. A changed owner breaks these files.

### Option D: A second event trigger that refuses a rename of `public`

- Pros: the search path of every function stays valid.
- Cons: one more function and one more event trigger. `restore-code.expected` and the name list of step 2 of the restore test get two more lines.

## Decision

The project uses **Option A**.

- (a) `audit_guard.audit_objects_dropped()` reads the object type and the last element of `address_names` of each dropped object. It ignores `schema_name` and the argument types.
- (b) The function refuses the drop when the type is `table` or `function`, the name is in the list, and `current_user` is not `audit_owner`.
- (c) The list holds eight names: `audit_log`, `audit_chain_head`, `security_event`, `audit_canonical`, `audit_hash_chain`, `security_event_purge`, `security_event_guard`, and `replication_state`.
- (d) The error stays SQLSTATE `42501` with the message "audit object % can be dropped only by audit_owner".
- (e) The grant test of `data-model.md` section 5.3 compares the names of the list with the names of the tables and functions that `audit_owner` owns.
- (f) A negative test runs first, as a probe before this ADR becomes Accepted. `recipe_migrator` renames `public` and drops the renamed schema with `CASCADE` in one transaction. The test expects `42501` and an intact `audit_log`.
  - The analysis says that the test fails before the change. The rename needs the owner of the schema and `CREATE` on the database, and the migrator gets both through `pg_database_owner`. `sql_drop` then reports `moved.audit_log`.
  - When the test passes before the change, the finding is false. The implementer then stops and reports to the project owner.
- (g) `01-roles.sh` and the SQL block of `data-model.md` section 5.1 hold the new function. `restore-code.expected` gets the new SHA-256 of the function. A document test keeps the SQL block of section 5.1 equal to the function of `01-roles.sh`.
- (h) `01-roles.sh` runs only on an empty volume. For an older volume, the superuser replaces the guard function during the upgrade (ADR-0069 (p)). The upgrade runs in this order:
  1. The restore test of the old image checks the newest set.
  2. `backup-premigrate` writes the rollback set `_premigrate`. The operator waits for exit code 0.
  3. `migrate` runs with `--no-deps` and applies the migrations. The operator waits for exit code 0.
  4. The superuser runs the `CREATE OR REPLACE FUNCTION` of the new `01-roles.sh`.
  5. A query computes the SHA-256 of `pg_get_functiondef` of the function. It must equal field 4 of the line `FUNCTION|audit_guard audit_objects_dropped()` of the new `restore-code.expected`.
  6. The new `app` starts. Then the new image of `recipe-pgtools` replaces the old one in `backup` and `walarchive`.
  7. `backup-now` writes a new set, and the restore test of the new image checks it.
- (h1) Reasons for this order:
  - The rollback set of step 2 holds the old guard and the old functions. The old image therefore accepts it after a rollback restore (ADR-0083 (d)).
  - A failure in step 3 leaves the old guard. The weekly restore test of the old image then still passes.
  - The new set of step 7 holds the new guard, so the new image accepts it.
  - Two commands for steps 2 and 3 let the operator see the rollback set before the schema changes.
- (h2) Recovery after a failure between two steps:
  - Step 2 fails: nothing changed. The operator sets the previous tag and stops.
  - Step 3 fails: the operator follows the failed-migration step of the runbook and ADR-0083. The guard stays old.
  - Step 4 or 5 fails: the schema is new, and the guard is old. The operator runs step 4 again. The operator runs no `backup-now` until step 5 passes.
  - Step 7 fails: the operator keeps the new `app` and every set and reads the `ERROR` line (ADR-0083 (f)).
- (h3) The release notes name step 4 as a "manual superuser step". The section "An older volume" of the runbook uses the same place in the upgrade for the guard step of `0006`.
- (h4) Step 3 of `reliability.md` section 7.10 splits into the two commands of steps 2 and 3. `migrate` needs `--no-deps`. Without it, Compose runs `backup-premigrate` a second time, and that dump fails in the same UTC minute (ADR-0082).
- (i) This item amends ADR-0034: the comparison of the names. The other items of ADR-0034 stay.

## Consequences

### Positive

- A rename of the schema `public` no longer opens a path to drop an audit object.
- A move of an audit object to another schema keeps the protection.

### Negative

- A rename of `public` stays possible for `recipe_migrator`. This path is part of the residual risk of the migrator credential (`data-model.md` section 5.1):
  - The migrator owns the database, so it can create a new schema `public` with a look-alike `audit_log` that has no trigger.
  - The application inserts into the unqualified `audit_log`, so new entries go to the unprotected table. The real history stays in the renamed schema.
  - The nightly check then reads the look-alike table and head.
  - The path adds no authority beyond `SET ROLE audit_owner`, which the migrator already has.
- The superuser of an older volume runs one more step during the upgrade.

### Risks and mitigations

- Risk: a later audit object has a name outside the list. Mitigation: the grant test compares the list with the objects that `audit_owner` owns.
- Risk: `address_names` of a later PostgreSQL release has another form. Mitigation: the negative tests and the test of `DROP SCHEMA public CASCADE` fail when the guard finds no name.
- Risk: the guard of an older volume and `restore-code.expected` differ. Then the restore test stops. Mitigation: the order and the hash check of (h).
- Risk: the upgrade order of (h) fails on a real volume. Mitigation: a CI job upgrades a volume of the previous release and restores its rollback set.

## Related

- Amends ADR-0034 (the comparison of the names).
- ADR-0035 and ADR-0069 (names in the guard list), ADR-0067 (restore code list).
- `data-model.md` sections 5.1 and 5.3. `reliability.md` section 7.10. `docs/ops/runbook.md` (upgrade, older volume).
- ADR-0082 and ADR-0083 (upgrade and rollback).
- `deploy/db/initdb/01-roles.sh`, `crates/recipe-db/tests/storage/grants/audit_hardening.rs`, `deploy/pgtools/restore-code.expected`.
- PostgreSQL 16 documentation, function `pg_event_trigger_dropped_objects`.
- Baseline requirement: `audit` "Owner of the audit objects".
- `overview.md` section 7: R20. `domain-model.md` section 3: I13.
