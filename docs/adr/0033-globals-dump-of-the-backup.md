# ADR-0033: Globals dump of the backup

Status: Accepted
Date: 2026-10-14
Deciders: project owner

## Context

The code review of step P2.4 found two defects in the globals dump of `reliability.md` section 7.7:

- The service `backup` connects as `recipe_backup` and runs `pg_dumpall --globals-only`. Without the option `-l`, `pg_dumpall` connects to the database `postgres` and then to `template1`. ADR-0032 removes `CONNECT` of `PUBLIC` on both, so the daily globals dump fails.
- `recipe_backup` is not a superuser. It cannot read `pg_authid`, which holds the password hashes. A globals dump with role passwords therefore fails even without ADR-0032.

## Decision drivers

- The daily backup runs without the superuser (ADR-0020).
- ADR-0032 stays: a login role reaches no default database.
- Each password has one source: its Docker secret (ADR-0031).

## Considered options

### Option A: Dump the globals from the database `recipe`, without passwords

- Pros: no new grant. `recipe_backup` already has `CONNECT` on `recipe`. The secrets stay the only source of the passwords.
- Cons: a restore must set the role passwords from the secret files.

### Option B: Grant `CONNECT` on `postgres` to `recipe_backup`

- Pros: the default `pg_dumpall` command works for the connection.
- Cons: it reverses a part of ADR-0032, and the password hashes still need the superuser.

## Decision

The project uses **Option A**.

- The service `backup` runs `pg_dumpall --globals-only --no-role-passwords -l recipe` as `recipe_backup`.
- A restore re-creates the roles from this file. Then it sets the password of each login role from its secret file, as `01-roles.sh` does.
- `reliability.md` section 7.7 and the residual-risk paragraph of `data-model.md` section 5.1 hold this rule.
- Step P4.1 writes the backup scripts and the restore runbook with this command and adds a test.

## Consequences

### Positive

- The daily globals dump works with ADR-0032 in force.
- A backup file holds no password hash.

### Negative

- A restore has one more step: it sets the role passwords from the secret files.

### Risks and mitigations

- Risk: an operator restores the globals and forgets the passwords. Mitigation: the restore runbook of P4.1 runs the password step, and the restore test of `reliability.md` section 7.7 logs in with each role.

## Related

- `reliability.md` sections 7.1 and 7.7
- `data-model.md` section 5.1
- ADR-0020 (deployment and backup), ADR-0031 (secret files), ADR-0032 (default databases)
