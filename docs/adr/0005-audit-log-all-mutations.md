# ADR-0005: Append-only audit log for every mutating operation

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

The draft says: "Record all changes." It also says: "Handle and record errors and unexpected behavior." The draft does not say which entities are in scope and does not say what a record contains.

The recipes control an industrial process. An investigation after an incident must answer: who changed what, when, and from what value.

## Decision drivers

- Completeness: a missing record makes the whole log untrustworthy.
- Atomicity: a change without its record, or a record without its change, must be impossible (R29).
- Simplicity: the team must be able to read the log with SQL.
- Tamper resistance: the application must not be able to edit or delete a record.

## Considered options

### Option A: Append-only audit table written by every command in the same transaction

- Pros: complete, atomic with the change, readable with SQL, simple to implement once in the `UnitOfWork`.
- Cons: the table grows forever, `before`/`after` JSON duplicates data.

### Option B: Audit only recipes and versions

- Pros: less data.
- Cons: a change of a user role, a machine platform, or an S7 address has no record. The log cannot answer every incident question.

### Option C: Event sourcing

- Pros: the event stream is the single source of truth, any past state can be rebuilt.
- Cons: a large change of the whole architecture, projections and rebuilds to maintain, new skills, and no requirement that needs it.

## Decision

We use Option A.

- The table is `audit_log`. The record fields are in `domain-model.md` section 2.4.
- Every command in `recipe-app` records at least one `AuditEntry`. The `UnitOfWork::commit` method takes the entries as an argument. A command without an entry does not compile (I13).
- The entry and the change are in one transaction. A rollback removes both.
- `before` and `after` hold the JSON form of the changed entity. For a parameter edit, they hold the parameter row. For an approval, they hold the status fields of both versions.
- The database role of the application has `INSERT` and `SELECT` on `audit_log`. It does not have `UPDATE` or `DELETE`.
- Login, failed login, logout, and export are recorded as well (Assumption A8).
- Queries do not write audit records, except export.
- The audit log is not an event store. The entity tables stay the source of truth.

## Consequences

### Positive

- One rule covers every operation. Reviews check one thing: does the command pass an entry to `commit`.
- The log survives application bugs, because the application cannot delete it.
- An Administrator can show the history of a recipe, a parameter key, or a user with one query.

### Negative

- Storage grows with every change. At the expected scale (Assumption A19) this is a few gigabytes per decade.
- `before`/`after` JSON must be versioned with the entity shape. A schema migration must not break old records. Old records are read as opaque JSON.

### Risks and mitigations

- Risk: a developer adds a command that writes without an audit entry. Mitigation: the `commit` signature requires entries. An integration test asserts one entry per command.
- Risk: the audit table slows writes. Mitigation: one index on `(entity_type, entity_id, occurred_at)` and one on `(occurred_at)`. No other indexes.

## Related

- ADR-0009 (snapshots make `before`/`after` small per version)
- ADR-0011 (soft delete keeps the entity that the log refers to)
- `domain-model.md` section 2.4, invariant I13
- `data-model.md` (`audit_log`, grants)
