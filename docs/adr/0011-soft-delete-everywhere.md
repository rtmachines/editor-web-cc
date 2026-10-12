# ADR-0011: Soft delete for recipes, versions, machines, and users

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

The draft says an Administrator or Process Engineer can "delete recipes". It also says "Record all changes" and that the application is critical to industrial work. A physical delete destroys the entity that the audit log refers to and makes an export from the past unreproducible.

## Decision drivers

- The history must stay complete (R20, ADR-0005).
- A deleted recipe must not block a new recipe with the same reference number forever, or the plant loses its numbering scheme.
- An `Approved` version is in use on a machine. Deleting it must not be possible by accident.
- The application must not be able to destroy data, even with a bug (R31).

## Considered options

### Option A: Soft delete everywhere

- Pros: history stays complete, undelete is possible, the application role needs no `DELETE` grant.
- Cons: every list query must filter on `deleted_at IS NULL`. Unique constraints must handle deleted rows.

### Option B: Hard delete only when no `Approved` or `Retired` version exists

- Pros: no filter on queries for clean recipes.
- Cons: audit rows point to rows that no longer exist. Two code paths for delete. A bug in the "no approved version" check destroys data.

## Decision

We use Option A.

- `recipe.deleted_at`, `recipe_version.deleted_at` (discard of a `Draft`), `machine.archived_at`, and `app_user.is_active = false` are the soft-delete markers.
- `DeleteRecipe` returns `Err(RecipeHasApprovedVersion)` when an `Approved` version exists (Assumption A13, I14). The Administrator must first retire the version with `RetireVersion`.
- A `ProcessEngineer` can delete a recipe only when it has no `Approved` and no `Retired` version. An `Administrator` can delete any recipe without an `Approved` version (ADR-0013).
- Only a `Draft` can be discarded. `Approved` and `Retired` versions are never deleted.
- The application database role has no `DELETE` grant on `recipe`, `recipe_version`, `parameter`, `parameter_limit`, `parameter_translation`, `machine`, `unit`, `plc_type`, `plc_type_member`, `app_user`, or `audit_log`. `DELETE` is granted only on `session`.
- `parameter` rows of a discarded draft stay in place. The draft is hidden, not emptied.
- Unique constraints on `recipe.reference_number` and `machine.code` are partial: `WHERE deleted_at IS NULL` and `WHERE archived_at IS NULL`. A deleted reference number can be reused. The old recipe keeps its number in history.
- Every read model query filters soft-deleted rows by default. An Administrator view shows them with a "deleted" mark (POLA, `overview.md` section 8).
- An `Administrator` can restore a deleted recipe. The restore fails when the reference number is in use again.

## Consequences

### Positive

- No operation in the application can destroy a row.
- Every audit entry points to an existing row.
- Restore is a one-column update.

### Negative

- Every repository query carries a soft-delete filter. A missed filter shows a deleted row. An integration test per list query covers this.
- Storage is never reclaimed. At the expected scale this is acceptable (Assumption A19).

### Risks and mitigations

- Risk: a user sees a deleted recipe in a dropdown. Mitigation: the repository has one `active()` scope used by all list queries. Reviews check for raw queries.
- Risk: a plant needs data removal for a legal reason. Mitigation: a manual procedure with the migration role. It is recorded outside the application.

## Related

- ADR-0005 (audit entries refer to rows)
- ADR-0013 (who can delete)
- `domain-model.md` invariant I14, section 4 (`Discarded`)
- `data-model.md` (partial unique indexes, grants)
