# ADR-0008: Optimistic concurrency with a row version on the draft

Status: Accepted. Amended by ADR-0015 (locking model, idempotent approve)
Date: 2026-10-12
Deciders: project owner

## Context

Two process engineers can open the same `Draft` version. The draft does not say what happens when both save. "Last write wins" silently loses the work of one of them. That conflicts with "Give data safety and reliability priority over performance."

## Decision drivers

- No silent data loss (R31).
- No locks that stay after a browser crash, a logout, or a network loss in 24/7 operation (R30).
- The mechanism must work with HTML forms and HTMX (ADR-0001).
- Simple to reason about and to test.

## Considered options

### Option A: Optimistic lock with a `row_version` column

- Pros: no lock state, a conflict is detected at save time, the user sees both values and decides, one column and one `WHERE` clause.
- Cons: the second user must redo their change after a conflict.

### Option B: Pessimistic check-out by a user with a timeout

- Pros: the second user is warned before editing.
- Cons: a stale lock blocks work until the timeout, an Administrator needs a "break lock" action, the timeout is a guess.

### Option C: Last write wins

- Pros: nothing to implement.
- Cons: silent data loss. Rejected by R31.

## Decision

We use Option A.

- `recipe_version.row_version` is a `BIGINT` that starts at 1. Every successful write to the version or to one of its parameters increments it in the same transaction.
- Every edit form and every HTMX fragment that mutates the draft carries the current `row_version` in a hidden field.
- The use case locks the version row `FOR UPDATE` and compares `row_version` with the expected value. A mismatch returns `Err(ConcurrentEdit)` (I12). Then the touch statement updates the row, and a trigger increments `row_version` (amended by ADR-0015).
- `recipe-web` maps `ConcurrentEdit` to HTTP `409 Conflict` and renders the fragment `conflict.html`. It shows the current server `row_version`, the login of the last editor, the time of the last change, and a "Reload version" button. The unsaved input stays in the form until the reload (`api.md` section 4).
- The parameter rows do not have their own version column. The version row is the lock unit. This keeps I1 to I4 in one place and avoids partial conflicts inside one version.
- Approval also checks `row_version`. An approve request with a stale version is rejected.

## Consequences

### Positive

- No lock table, no lock cleanup, no "who holds the lock" UI.
- A conflict is visible and the user keeps control.
- Idempotent retries: a repeated request with the same `row_version` after a success returns `409`, not a double write. A repeated approve returns `AlreadyApproved` instead (ADR-0015).

### Negative

- Two engineers who edit different parameters of the same draft still conflict. The lock unit is the version, not the parameter.
- The UI must handle `409` on every mutating action.

### Risks and mitigations

- Risk: frequent conflicts on large drafts. Mitigation: measure. If needed, a later ADR can move the lock unit to the parameter row. The hidden field pattern stays the same.
- Risk: a developer forgets the `row_version` compare. Mitigation: the repository method signature takes `expected_row_version`. Integration test for `ConcurrentEdit`.

## Related

- ADR-0001 (hidden field in forms)
- ADR-0009 (only drafts are mutable)
- `domain-model.md` invariant I12
- `data-model.md` (`recipe_version.row_version`), `api.md` (409 response)
