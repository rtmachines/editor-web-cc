# ADR-0021: Soft delete, unique keys and delete rules

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

ADR-0011 chose soft delete for every entity. The design review found that it contradicted the schema and the other documents (traceability B18 to B20 and B31, architecture F12, security L3):

- ADR-0011 said that unique keys were partial, so a deleted reference number could be reused. The DDL, R16 and I11 used a plain `UNIQUE`.
- ADR-0011 said that `DELETE` was granted only on `session`. The DDL granted it on parameter rows, and `RemoveParameter` needs it.
- The recipe delete rule had a different condition in five documents.
- ADR-0011 mentioned an Administrator restore, but no command, endpoint or permission existed.
- A deleted recipe kept its live draft open, and another user could still approve it.

The architecture review (F11) added that a controller replacement needs a new machine, but the old machine code and reference numbers stay reserved forever. The owner decided that controller replacement is out of scope, and that machine codes are unique among live machines only.

## Decision drivers

- No loss of history (R20, I14).
- One delete rule that the permission-matrix test can generate (R23, R28).
- A deleted recipe must not change state through an open draft (R29).
- A replaced machine can keep its code (A32).

## Considered options

### Option A: Partial unique keys everywhere (deleted rows do not block reuse)

- Pros: any code or reference can return.
- Cons: `{ref}` URLs become ambiguous, the export document for `(ref, n)` is no longer reproducible, history pages must disambiguate.

### Option B: Plain unique keys for recipes, partial for machine codes only

- Pros: reference numbers stay unique forever, which the export format needs. A machine can be replaced under the same code.
- Cons: a deleted recipe reserves its reference number. `RestoreRecipe` returns it to use.

### Option C: Hard delete when nothing is approved

- Pros: no reserved numbers.
- Cons: loses the audit chain target and contradicts R20.

## Decision

The project uses **Option B**.

Unique keys:

- `recipe.reference_number`: plain `UNIQUE`. A deleted recipe keeps its number.
- `machine.code`: `UNIQUE WHERE archived_at IS NULL`. An archived machine frees its code. `/machines/{code}` resolves to the live machine. `/machines?show_archived=1` lists the archived machines, and `/machines/by-id/{uuid}` shows one of them (`api.md` 2.2).
- `unit.path`: `UNIQUE (machine_id, path)`. An archived unit keeps its path. There is no `UnarchiveUnit` in this release.

Delete rules (one rule, used in every document):

| Command | Role | Condition | Error |
|---|---|---|---|
| `DeleteRecipe` | ProcessEngineer | No Approved and no Retired version | `E-RCP-026 RecipeHasRetiredVersion`, 409 |
| `DeleteRecipe` | Administrator | No Approved version | `E-RCP-010 RecipeHasApprovedVersion`, 409 |
| `RestoreRecipe` | Administrator | Recipe is deleted and the machine is live | `E-RCP-025 RecipeNotDeleted`, `E-PLT-010 MachineArchived`, 409 |
| `DiscardDraft` | ProcessEngineer, Administrator | Version is a live Draft | `E-RCP-007`, 409 |
| `ArchiveMachine` | ProcessEngineer, Administrator | No live Draft on any recipe of the machine | `E-PLT-011 MachineHasDraft`, 409 |
| `ArchiveUnit` | ProcessEngineer, Administrator | No Approved version and no live Draft references the unit or a descendant. The unit is not the last live main unit | `E-PLT-012 UnitReferencedByDraft`, `E-PLT-014 UnitReferencedByApproved`, `E-PLT-008 LastMainUnit`, 409 |

- `DeleteRecipe` discards the live draft of the recipe in the same transaction. The discard writes its own audit entry. `ArchiveMachine` does not discard drafts. It fails while a draft exists.
- `ArchiveUnit` cascades `archived_at` to every descendant.
- `ArchiveMachine` leaves the recipes readable and exportable. `RestoreRecipe` on a recipe of an archived machine fails with `E-PLT-010 MachineArchived`.
- The recipe list hides deleted recipes. `show_deleted=1` is available to Administrator only.

Grants: `recipe_app` has `DELETE` on `session`, `login_attempt`, `draft_editor`, `parameter`, `parameter_limit` and `parameter_translation`. The immutability trigger limits these deletes to live drafts. `recipe_app` can update only the column `succeeded` of `login_attempt`, for `Login` T2 (`data-model.md` 5.2). There is no `DELETE` on `machine`, `unit`, `recipe`, `recipe_version`, `app_user`, `plc_type`, `plc_type_member`, `audit_log` and `security_event`. `security_event` rows older than 400 days are deleted by `security_event_purge()`, a `SECURITY DEFINER` function owned by `audit_owner` (ADR-0022).

## Consequences

### Positive

- The permission test generates the delete rows from one table.
- A deleted recipe cannot gain an Approved version through a stale draft.
- Machine M12 can be rebuilt with a new controller under the code M12 (A32). The recipes are created again by hand in this release.

### Negative

- A reference number of a deleted recipe is reserved until an Administrator restores the recipe.
- A controller replacement is manual. ADR-0018-style bulk commands can come later.

### Risks and mitigations

- Risk: two live machines with one code during a swap. Mitigation: `ArchiveMachine` of the old one is a precondition, and the partial unique index enforces it.

## Related

- Supersedes ADR-0011.
- ADR-0013 (roles), ADR-0015 (lock on the recipe row makes the draft discard atomic), ADR-0016 (commands).
- `api.md` section 3.
- `data-model.md` sections 3.1, 3.9, 5.
- `domain-model.md` section 5.
- Review findings: architecture F6, F11, F12. Security L3. Traceability B18, B19, B20, B31, B57, B58, B80.
