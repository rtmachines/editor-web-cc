# ADR-0030: Guard of a version status write

Status: Accepted
Date: 2026-10-13
Deciders: project owner

## Context

The storage adapter of `recipe-db` implements the ports of `recipe-app` (P2.4). Three rows of `data-model.md` section 6 do not agree with these ports:

- `DiscardDraft` and `RetireVersion` write `... AND row_version = $expected` and map "0 rows" to `VersionNotDraft`, `VersionNotApproved`, or `ConcurrentEdit`.
- `ApproveVersion` writes `... WHERE id = $v AND row_version = $expected` and retires the approved version with `WHERE recipe_id = $r AND status = 'approved'`.
- The ports `discard_version`, `retire_version`, and `approve_version` of `recipe-app` take the version identifier only. They do not take the expected row version.
- ADR-0008 and ADR-0028 already give the compare to the use case. The use case locks the version row `FOR UPDATE`, and the domain compares the expected row version with the locked row.
- ADR-0029 locks the approved version before its retirement, so the retirement can name that row by its identifier.

Under the lock, no other transaction can change the row. A status write therefore never finds a different row version, unless a lock path has a defect.

## Decision drivers

- The ports of P2.3 stay as they are (`app-use-cases-and-ports`, decision D5).
- A defect of a lock path must stop the command and must not write a row silently (R29).
- The domain gives the user error of a stale row version, before any write (ADR-0028).
- A trigger code of `reliability.md` section 1.2 stays the error of an illegal transition.

## Considered options

### Option A: The guard uses the row version that the lock read

- Pros: the ports do not change. A write that does not follow a lock in the same transaction fails. The domain keeps its compare.
- Cons: the adapter keeps the row version of each locked version for the life of the transaction.

### Option B: The ports take the expected row version

- Pros: the statement text of `data-model.md` section 6 stays.
- Cons: the ports, the use cases, and the in-memory adapter of P2.3 change. The compare then runs twice, in the domain and in the statement.

### Option C: No guard

- Pros: the simplest statement.
- Cons: a defect of a lock path writes the status without a check.

## Decision

The project uses **Option A**.

- `lock_version`, `lock_versions`, and `lock_draft` keep the `row_version` that they read in the transaction.
- `discard_version`, `retire_version`, and `approve_version` run `UPDATE recipe_version ... WHERE id = $v AND row_version = $locked`. `$locked` is the kept value of that row.
- A status write without a kept value, or a statement that changes 0 rows, gives `E-DB-008` (`StorageError::Other`) and writes nothing.
- The status condition is not part of the `WHERE`. Trigger 4.7 rejects an illegal transition with `E-RCP-033`, as the port contracts of `recipe-app` say.
- `touch_draft` and each status write also keep the new `row_version` of their row. Their `UPDATE` holds the row lock until the end of the transaction, so a later status write of the same transaction finds the current value.
- `ApproveVersion` retires the approved version by its identifier and the row version of its second lock.
- The rows `DiscardDraft`, `RetireVersion`, and `ApproveVersion` of `data-model.md` section 6 hold these statements.

## Consequences

### Positive

- The ports and the use cases of P2.3 do not change.
- A status write that does not follow its lock fails and writes nothing.
- The user error of a stale row version keeps one source: the domain compare of ADR-0028.

### Negative

- `E-DB-008` replaces the result "0 rows → `ConcurrentEdit`" of the old text. Under the lock the old result can never occur.
- The transaction of the adapter holds a small map of row versions.

### Risks and mitigations

- Risk: a new command writes a status without a lock. Mitigation: the write gives `E-DB-008`, and a contract test of `recipe-db` covers this case.

## Related

- `data-model.md` section 6
- `reliability.md` section 1.2 (`E-DB-008`, `E-RCP-033`)
- ADR-0008 (row version), ADR-0015 (locking), ADR-0028 (row version check in the domain), ADR-0029 (lock of the approved version)
