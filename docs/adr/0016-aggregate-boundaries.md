# ADR-0016: Aggregate boundaries and command split

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

`domain-model.md` defined one aggregate `Recipe` that holds every version with every snapshot parameter. The architecture review (F3) showed that a parameter save would load `versions × parameters × leaves` rows. A whole-aggregate save would hit the immutability trigger on approved rows. The approval rule already lived in SQL as well.

The same review (F2) found that `UpsertParameter` replaced all limits and translations on every call, while each form sent only its part. A value save deleted the limits.

The domain method `upsert_parameter` took a `&TypeCatalog`, which is an I/O port in `recipe-app` (traceability B66). `UpsertParameter` also created `plc_type` rows, which crossed the context boundary (architecture F5).

## Decision drivers

- A use case must load only what it changes.
- The approval rule must exist in one place (R18, R27).
- `recipe-domain` has no I/O and no dependency on `recipe-app` (overview section 5).
- Each form changes only its own rows (R20, R29).

## Considered options

### Option A: One `Recipe` aggregate with lazy loading

- Pros: one model.
- Cons: lazy loading in a domain object needs a port inside the domain, which breaks the dependency rule.

### Option B: Small `Recipe` root, `Draft` as its own aggregate, read models for closed versions

- Pros: each command loads one version, closed versions are never loaded for writing, the trigger never sees a rewrite of approved rows.
- Cons: two aggregates share the version state machine, so the transition rule needs one shared function.

### Option C: Keep the big aggregate and write SQL per command

- Pros: no domain refactor.
- Cons: the rules drift between Rust and SQL. This is what the review observed.

## Decision

The project uses **Option B**.

Aggregates:

- `Recipe` (root): `id`, `machine`, `reference_number`, `name`, `deleted_at`, `versions: Vec<VersionHeader { id, number, status, row_version }>`. It owns `approve()`, `retire()`, `delete()`, `restore()` and `create_version()`.
- `Draft`: one `RecipeVersion` with `status = Draft` and its `Vec<Parameter>`. It owns every parameter command.
- `ApprovedVersion`, `RetiredVersion`: read models with `row_version`, so `RetireVersion` keeps its optimistic check.

Domain inputs: every domain method takes resolved values only: `&TypeTree`, `&PlatformTypes`, `&LiveUnits`, `&ApprovalPolicy`, `&DraftEditors`. The use case loads them through ports before it calls the domain.

Commands (one use case, one transaction each):

| Context | Commands |
|---|---|
| Plant Model | `CreateMachine`, `RenameMachine`, `ArchiveMachine`, `AddUnit`, `RenameUnit`, `ArchiveUnit`, `DefineUdt` |
| Recipe Management | `CreateRecipe`, `RenameRecipe`, `DeleteRecipe`, `RestoreRecipe`, `CreateVersion`, `DiscardDraft`, `ApproveVersion`, `RetireVersion`, `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, `UpsertTranslation`, `RemoveTranslation`, `RemoveParameter`, `RebindTypeRevision` |
| Identity and Access | `Login`, `Logout`, `CreateUser`, `SetUserRole`, `DeactivateUser`, `ActivateUser`, `ResetPassword`, `ChangeOwnPassword`, `UnlockLogin` |
| Export | `ExportVersion` |

`EnsureDerivedType` is an internal operation of the Plant Model, not a use case. It has no permission row. `AddParameter`, `UpdateParameterDefinition`, `DefineUdt`, and `RebindTypeRevision` call it through a port inside their own transaction. Recipe Management never writes `plc_type` itself (ADR-0017).

Queries are use cases too. Each query checks its permission first (ADR-0013).

Every draft command inserts the user into `draft_editor(recipe_version_id, user_id, first_edit_at)`. ADR-0022 uses this table.

## Consequences

### Positive

- A parameter save loads one draft, not the history.
- A value save cannot delete a limit. Each command touches one row family.
- The approval rule is `Recipe::approve()` in Rust. SQL only enforces it as defence in depth (ADR-0015).

### Negative

- Two aggregates for one table. The persistence adapter maps both.
- The command list grew from 24 to 33 command use cases plus one internal operation. Most of the new ones are small.

### Risks and mitigations

- Risk: an implementer loads the whole history for the version page. Mitigation: the version page is a read model query with pagination (architecture F7).
- Risk: the `VersionHeader` list goes stale inside a transaction. Mitigation: ADR-0015 locks the `recipe` row first, so the list is current for the transaction.

## Related

- ADR-0009 (snapshot) stays. ADR-0015 (locking), ADR-0017 (derived types), ADR-0022 (draft_editor).
- `domain-model.md` sections 2.2, 5 and 6.
- `api.md` section 2.
- Review findings: architecture F2, F3, F5, F14. Traceability B53, B54, B66.
