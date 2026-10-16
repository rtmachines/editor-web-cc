# ADR-0027: Error lists and the error of a draft command

Status: Accepted. Amended by ADR-0041 (the draft commands of a value and of limits return `RecipeErrors`)
Date: 2026-10-13
Deciders: project owner

## Context

Three rules return more than one error of one code:

- ADR-0022 gives `E-RCP-016 LimitMissing` "with the list of leaves".
- ADR-0018 and `domain-model.md` section 5.2 give one `E-RCP-028` or `E-RCP-029` entry for each failing parameter and leaf in one list.
- `api.md` section 2.5 shows one line for each failing parameter of a rebind.

Each of these variants in `reliability.md` section 1.2 holds one tag and one leaf. `reliability.md` section 1.1 gives `AppError` a `#[from]` for each error enum of `recipe-domain`, but for no list.

A draft command can also fail with a code of the Plant Model. `domain-model.md` section 5.2 lists `E-PLT-009`, `E-PLT-013`, `E-PLT-018`, and `E-PLT-020` for `AddParameter`, `UpdateParameterDefinition`, and `RebindTypeRevision`. The `Draft` checks that a unit and a type tree belong to its machine (ADR-0028). A failure of that check is a `PlantError`, not a `RecipeError`.

## Decision drivers

- Every code keeps one owner variant (R36).
- A user sees every failing leaf in one response, not one leaf for each attempt (R39).
- The registry of `reliability.md` section 1.2 stays unchanged.

## Considered options

### Option A: A list type of the domain

- Pros: the variants and their message templates stay as they are. A list of one entry has the same form as a single error.
- Cons: `AppError` gets one more `#[from]`.

### Option B: List fields in the variants

- Pros: no new type.
- Cons: three rows of section 1.2 and their messages change. A message for a list of leaves is long and hard to read.

### Option C: Only the first error

- Pros: no change.
- Cons: it contradicts ADR-0018, ADR-0022, and `api.md` section 2.5.

## Decision

The project uses **Option A**.

- `recipe-domain` has the type `RecipeErrors`. It holds one or more `RecipeError` values in a fixed order. It cannot be empty. `code()` returns the code of the first entry.
- `Draft::check_for_approval` returns `DraftError`. Its list holds one `E-RCP-016` entry for each leaf without a limit. A `PlantError` of the S7 layout uses the variant `Plant`.
- The rebind plan returns `DraftError`. Its list holds every `E-RCP-028`, `E-RCP-029`, and `E-RCP-005` entry of every affected parameter. A target tree above a size limit uses the variant `Plant` (`E-PLT-018`).
- `recipe-domain` has the type `DraftError`. It is a sum of `RecipeError`, `PlantError`, and `RecipeErrors`. It owns no code. `code()` returns the code of its inner error.
- A draft command that can fail with a code of the Plant Model returns `DraftError`. The other draft commands return `RecipeError`.
- `AppError` gets `#[from] RecipeErrors`. A `From<DraftError>` for `AppError` takes the inner error. It adds no variant.
- `recipe-web` shows one line for each entry, with its code, its message, and the `request_id`.

## Consequences

### Positive

- The registry test of `reliability.md` section 1.2 needs no change.
- The approve page and the rebind response list every failing leaf at once.

### Negative

- `AppError` and the mapping of `recipe-web` handle one more type.

### Risks and mitigations

- Risk: a list grows to thousands of entries for a large draft. Mitigation: the budgets of `reliability.md` section 7.11 limit the leaves. The approve page shows the count and the first entries.

## Related

- `reliability.md` sections 1.1, 1.2
- `domain-model.md` section 5.2
- `api.md` sections 2.5, 5
- ADR-0018 (rebind), ADR-0022 (limits at approval), ADR-0028 (draft binding)
