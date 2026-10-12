# ADR-0007: Process limits per scalar leaf of a parameter

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

The draft says: "Each parameter must have ... process limits. A process engineer defines the process limits. These limits describe the process. They are not only the limits of the data type."

A parameter can be a scalar, an array, or a UDT with nested members (ADR-0006). A single `min`/`max` pair cannot describe the limits of a struct with a temperature and a pressure.

## Decision drivers

- A limit must be possible for every scalar inside an array or a struct.
- A limit must be narrower than or equal to the data type range.
- A `BOOL` has no meaningful limit. A `STRING` has a length limit, not a range.
- The model must stay simple for the common case: one scalar parameter with one `min` and one `max`.

## Considered options

### Option A: One limit per scalar leaf, addressed by a leaf path

- Pros: works for every type shape, the common scalar case is one row with an empty path, limits for a UDT member can differ per element.
- Cons: a UDT with many members needs many rows, the leaf path must be validated against the type tree.

### Option B: One limit per parameter

- Pros: one row, simple UI.
- Cons: cannot express limits for a struct or a multidimensional array. Violates the draft for UDT parameters.

## Decision

We use Option A.

- A `ProcessLimit` has a `leaf_path`, optional `min` and `max` for numeric leaves, and optional `max_length` for string leaves. The grammar of `leaf_path` is in `domain-model.md` section 2.1.
- A `BOOL` leaf cannot have a limit. The domain returns `Err(LimitNotApplicable)`.
- A limit is optional. A leaf without a limit accepts the full data type range.
- When both `min` and `max` are set, `min <= max`. Both must be inside the data type range.
- The rows are in `parameter_limit`, unique on `(parameter_id, leaf_path)`.
- The domain validates every value leaf first against the type range, then against the limit for that leaf (I7).
- The UI shows the limits inline in the type tree editor (ADR-0001), one row per leaf.
- A limit change is an edit of the `Draft`. It therefore creates a new version when it concerns an approved recipe (ADR-0009).

## Consequences

### Positive

- Every scalar in every type shape can have a process limit.
- The scalar case stays one row.
- Limits are queryable: "all parameters with max above X" is SQL.

### Negative

- A large UDT parameter has many limit rows. The editor must make the table readable.
- The `leaf_path` must be re-validated when a parameter changes its type. The domain removes limits whose path no longer resolves and reports that to the user.

### Risks and mitigations

- Risk: a process engineer forgets a limit on a critical leaf. Mitigation: the approval screen lists leaves without a limit. Approval does not block on it (the draft does not require a limit on every leaf).
- Risk: array limits are repeated per element. Mitigation: a future "apply to all elements" action in the UI. It expands to one row per element. No change of the model.

## Related

- ADR-0006 (type tree, leaf path)
- ADR-0009 (limit change is a version change)
- `domain-model.md` section 2.2 (`ProcessLimit`), invariant I7
- `data-model.md` (`parameter_limit`)
