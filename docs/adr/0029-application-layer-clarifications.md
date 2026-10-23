# ADR-0029: Approval token ownership, audit row images, and application clarifications

Status: Accepted. Amended by ADR-0035 (dependencies of recipe-app, row image of LoginUnlocked), ADR-0039 (transport of `new_member_values`), and ADR-0056 (scope of `timeout.export`)
Date: 2026-10-13
Deciders: project owner

## Context

The design of the application layer in `recipe-app` showed gaps between the documents and code that keeps every rule:

- `ApprovalReady` borrows the `Draft` (ADR-0028). `spawn_blocking` needs a closure with the lifetime `'static`, so a borrowed token cannot leave it. `reliability.md` section 4.4 puts the snapshot check of `ApproveVersion` in `spawn_blocking` before the transaction.
- The state rule of ADR-0005 stores "the full row of the changed entity". The documents do not say where the row comes from. PostgreSQL 16 (A15) has no `RETURNING OLD`, so one write statement cannot give the row before the change.
- `ParameterAdded` writes limit rows, and `ParameterRemoved` deletes limit rows and translation rows. `UdtDefined` writes member rows. A row of the parent table alone does not record these child rows.
- `UpdateParameterDefinition` without a type change carries neither a value nor limits (`api.md` section 2.6). The documents give no result for a request that carries both without a type change. `Draft::set_definition` accepts both.
- `Machine::archive_unit` changes nothing when the unit is already archived. The commit needs one audit entry, and the row `ArchiveUnit` of `domain-model.md` section 5.1 has no code for this case.
- A `Forbidden` result writes one `security_event` row (ADR-0013, ADR-0022). The documents give no result when that write fails.
- `overview.md` section 5 names `recipe-domain`, `async-trait`, and `tracing` as examples of the dependencies of `recipe-app`. `thiserror` (R33) and the `spawn_blocking` of `reliability.md` section 2.2 need more crates.
- `data-model.md` section 6 checks "the limit count" of `ApproveVersion` again under the locks. The documents do not say what the second check compares.
- `ApproveVersion` retires the approved version with one `UPDATE`. No lock statement of the row `ApproveVersion` in `data-model.md` section 6 gives the row image of that version before the change.
- `api.md` section 2.5 calls a number in `new_member_values` "a decimal text". `reliability.md` section 4.4 keeps a request number as a JSON number.
- The rows `RebindTypeRevision` list `E-PLT-013` for a new array type. The array shape comes from the dimensions of a valid tree and from the checked target struct, so no request can cause `E-PLT-013`.
- `timeout.export` covers the whole use case, also the commit. A timeout during the commit can hide a committed `VersionExported` entry from the client.
- The rebuild of the hashed body runs on an async worker. A Tokio timeout stops a task only at an await point, so `timeout.export` cannot stop the rebuild.

## Decision drivers

- The snapshot check runs before the transaction, and no use case waits for I/O inside a transaction (`reliability.md` sections 4.4 and 7.4).
- A command without an audit entry does not compile (I13).
- An audit entry records every row that the command changes (R20).
- No error disappears without a log line (`reliability.md` section 1.1).
- The code follows the documents (CLAUDE.md, source of truth).

## Considered options

### Option A: One ADR with the clarifications of the application layer

- Pros: the documents and the code agree. One record holds the decisions of one design step, as in ADR-0028.
- Cons: one ADR covers several related points.

### Option B: Record the decisions only in the design of the change

- Pros: no change of the documents.
- Cons: `domain-model.md`, `data-model.md`, and `api.md` then contradict the code.

## Decision

The project uses **Option A**.

Approval token:

- `Draft::check_for_approval(self, &LiveUnits, &TypeTrees, allow_outdated)` consumes the `Draft`. The token `ApprovalReady` owns it, and `ApprovalReady::draft` gives read access. While the token lives, no command can change the draft. This amends ADR-0028.
- The token can therefore leave `spawn_blocking`. The check runs once, before the transaction.
- `Recipe::approve` compares the row version of the token with the locked header. A match proves that the locked draft equals the checked draft. The second `check_limits` under the locks is therefore the "limit count" check of `data-model.md` section 6. The second `check_outdated` reads the type rows of the transaction and finds a new revision.

Audit row images:

- The lock statement of a command gives the row image before the change. The write statement gives the row image after the change. The storage adapter builds each image with `to_jsonb`, without `password_hash`.
- The full row of a parameter holds its limit rows and its translation rows. The full row of a struct holds its member rows. A command writes no separate audit entry for these child rows. This amends ADR-0005.
- `ApproveVersion` locks the approved version `FOR UPDATE` after the draft, and this lock gives its row image before the retirement. The recipe lock comes first, so this second version lock cannot cause a deadlock.

Commands:

- `UpdateParameterDefinition` without a type change can carry a new value and new limits. Then they replace the old value and the old limits, with the same checks as after a type change. Without them, the value and the limits stay.
- `ArchiveUnit` on a unit that is already archived returns `E-PLT-009 UnitArchived`. It changes no unit and writes no audit entry.
- When the `security_event` row of a `Forbidden` result cannot be written, the use case returns the storage error instead of `E-WEB-002`. The use case changes no data in both cases. This amends ADR-0013 and ADR-0022.
- A number leaf in `new_member_values` is a JSON number, as in the field `value` (`reliability.md` section 4.4). A duration or instant leaf is an IEC literal in a JSON string.
- The error set of `RebindTypeRevision` has no `E-PLT-013`. A new array type of a rebind can fail only with `E-PLT-018`.

Export:

- The rebuild of the hashed body and its SHA-256 run in `spawn_blocking`. A failed task gives `E-WEB-011` with the stage `export`.
- `timeout.export` covers the read, the locks, the rebuild, and the hash compare. The commit of the audit entry runs after that limit, and `db.statement_timeout` limits each of its statements.

Dependencies of `recipe-app`:

- `recipe-domain`, `thiserror`, `async-trait`, and `tracing`.
- `tokio` with the features `rt` and `time`, for `spawn_blocking` and the limits of `reliability.md` section 7.4.
- `serde_json` for the audit states, and `uuid` with the feature `v4` for the identifiers of new rows.
- `parking_lot` and `time`, only with the feature `memory` of the in-memory adapters. The adapters lock their state with `parking_lot` (`reliability.md` section 2.2), and the in-memory clock uses `time`.
- The forbidden list of `overview.md` section 5 stays: `sqlx`, `axum`, `recipe-export`, and `recipe-db`.

## Consequences

### Positive

- The approval check runs in `spawn_blocking` once, and only a checked draft can become `Approved`.
- An audit entry records the child rows of a parameter and of a struct.
- Every request has one defined result, also an update of a value without a type change and an archive of an archived unit.
- A client that gets `E-WEB-009` from an export finds no `VersionExported` entry for that request.

### Negative

- `Draft::check_for_approval` has a different signature than ADR-0028 gives.
- Every write method of a storage port returns a row image, so the adapters of P2.4 have more work.

### Risks and mitigations

- Risk: the row images of the in-memory adapter and of the storage adapter differ. Mitigation: the image keys are the column names of `data-model.md` section 3, and the storage tests of P2.4 compare them.

## Related

- `overview.md` section 5
- `domain-model.md` sections 2.4, 5.1, 5.2, 6
- `data-model.md` sections 6, 10
- `api.md` sections 2.2, 2.5, 2.6, 3.2
- `reliability.md` sections 1.1, 2.2, 2.4, 4.4, 7.4
- ADR-0005 (audit log), ADR-0013 (permissions), ADR-0015 (locking), ADR-0022 (security events), ADR-0028 (approval token)
