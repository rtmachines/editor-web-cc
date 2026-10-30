# ADR-0032: Default databases closed to PUBLIC

Status: Accepted. Amended by ADR-0103 (a function of a shared file prints the two statements for `01-roles.sh` and `restore-volume`)
Date: 2026-10-13
Deciders: project owner

## Context

The PostgreSQL review of step P2.4 ran the role statements of `data-model.md` section 5.1 on PostgreSQL 16.15:

- A new cluster gives `PUBLIC` the privileges `CONNECT` and `TEMPORARY` on the databases `postgres` and `template1`.
- The image adds the line `host all all all scram-sha-256` to `pg_hba.conf`.
- `recipe_app` and `recipe_replicator` can therefore open a normal session on `postgres` and create temporary tables there.
- `data-model.md` section 5.1 says that `recipe_replicator` has no `CONNECT` grant. That statement is true only for the database `recipe`.

## Decision drivers

- Least privilege for every login role (R34, ADR-0020).
- `recipe_replicator` needs only replication connections.
- No service of `reliability.md` section 7.1 connects to `postgres` or `template1` as a role other than the superuser. The globals dump of the service `backup` connects to `recipe` (ADR-0033).

## Considered options

### Option A: Revoke the privileges of PUBLIC on both default databases

- Pros: two statements in the init script. A login role reaches only the grants of section 5.1.
- Cons: a tool that connects to `postgres` as a login role fails. No service of section 7.1 does that.

### Option B: Accept the default privileges

- Pros: no change.
- Cons: a stolen password of `recipe_app` or `recipe_replicator` gives a session with `TEMPORARY` outside the database `recipe`.

## Decision

The project uses **Option A**.

`01-roles.sh` runs these statements after the grants on `recipe`:

```sql
REVOKE CONNECT, TEMPORARY ON DATABASE postgres FROM PUBLIC;
REVOKE CONNECT, TEMPORARY ON DATABASE template1 FROM PUBLIC;
```

The SQL block of `data-model.md` section 5.1 holds both statements. The superuser keeps its access. `pg_isready` and the replication connections of `walarchive` need no `CONNECT` privilege on these databases.

## Consequences

### Positive

- A normal session of `recipe_app`, `recipe_backup`, or `recipe_replicator` reaches only the database `recipe`, or no database for `recipe_replicator`.

### Negative

- An operator tool that uses a login role on `postgres` needs the superuser.

### Risks and mitigations

- Risk: a later service needs a session on `postgres`. Mitigation: that service gets an explicit grant in a new ADR.

## Related

- `data-model.md` section 5.1
- `reliability.md` sections 7.1 and 7.7
- ADR-0020 (deployment, roles, and secrets), ADR-0033 (globals dump of the backup)
