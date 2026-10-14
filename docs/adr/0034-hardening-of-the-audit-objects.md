# ADR-0034: Hardening of the audit objects

Status: Accepted
Date: 2026-10-14
Deciders: project owner

## Context

The security review of step P2.4 found two paths around the protection of the audit objects of `data-model.md` sections 4.9 and 5.1:

- `recipe_migrator` owns the database `recipe`, so it is a member of `pg_database_owner`, which owns the schema `public`. A schema owner can drop every object in the schema, also the audit objects of `audit_owner`. For example, `DROP SCHEMA public CASCADE` drops `audit_log` without `SET ROLE`. A static check of the migration files cannot see every form of such a statement.
- The function `audit_canonical` runs inside the `SECURITY DEFINER` function `audit_hash_chain`. It multiplies a `numeric` by an `integer` with the unqualified operator `*`. `pg_catalog` has no exact match for these types. An exact match in `public` would therefore win, and `recipe_migrator` can create one there. That code would then run as `audit_owner` on every audit insert.

The migration files have not shipped. They can still change in place (`data-model.md` section 8).

## Decision drivers

- The hash chain of ADR-0022 holds only when no role except `audit_owner` can change the audit objects.
- The residual risk of the migrator credential stays as narrow as `data-model.md` section 5.1 describes.
- A fix is cheap before the first release and expensive after it.

## Considered options

### Option A: An event trigger of the superuser and a qualified operator

- Pros: the database itself stops a drop of an audit object by any role except `audit_owner`. No migration text can bypass it. The operator fix closes the code path inside the definer function.
- Cons: the init script creates one more schema, function, and event trigger.

### Option B: Move the audit objects into a schema of `audit_owner`

- Pros: the schema owner `pg_database_owner` loses its power over the audit objects.
- Cons: every name of sections 3, 4, and 5 changes, and the search path of every function changes.

### Option C: Only a stronger static check of the migration files

- Pros: no change of the database.
- Cons: a check of text cannot see dynamic SQL in a `DO` block or every comment form.

## Decision

The project uses **Option A**, and keeps the static check of the migration files as a second line.

- `audit_canonical` in section 4.9 uses `OPERATOR(pg_catalog.*)` and the literal `1000000::pg_catalog.numeric`. `0002_triggers.sql` holds the same text.
- `01-roles.sh` connects to `recipe` after it creates the database. As the superuser, it creates the schema `audit_guard`, the function `audit_guard.audit_objects_dropped()`, and the event trigger `audit_objects_dropped` on `sql_drop`. The SQL block of `data-model.md` section 5.1 holds these statements.
- The function rejects the drop of `audit_log`, `audit_chain_head`, `security_event`, `audit_canonical`, `audit_hash_chain`, and `security_event_purge` when `current_user` is not `audit_owner`. It raises SQLSTATE `42501`. A failed drop rolls back the whole statement.
- Only the superuser owns `audit_guard`. No other role has a privilege on it. Only the superuser can change or disable an event trigger.
- A database created from the template `recipe` inherits the schema and the event trigger.

## Consequences

### Positive

- A migration cannot drop an audit object without `SET ROLE audit_owner`, also through `DROP SCHEMA ... CASCADE` or a `DO` block.
- No object in `public` can change the hash of an audit entry.

### Negative

- The init script holds more SQL.
- A superuser who must drop an audit object first disables the event trigger.

### Risks and mitigations

- Risk: a later migration adds an audit object and forgets the guard list. Mitigation: the grant test of `data-model.md` section 5.3 compares the guard list with the objects that `audit_owner` owns.

## Related

- `data-model.md` sections 4.9, 5.1, 5.3, 8
- ADR-0022 (hash chain), ADR-0032 (default databases)
