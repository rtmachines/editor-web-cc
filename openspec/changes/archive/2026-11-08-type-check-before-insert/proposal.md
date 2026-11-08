# Proposal

## Why

The full repo review of 2026-11-08 found D1-1 (MEDIUM) in `UpdateParameterDefinition` (`docs/plan/review/2026-11-08-full-repo-review.md`).
The command finds out whether the type changes only after `resolve_type`.
Thus it takes the class-2 locks and inserts derived type rows before the two checks of a changed type refuse the text.
The rollback leaves no data, but the command holds its locks and the key `ARRAY` during useless inserts.

`reliability.md` §7.4 requires validation first. `AddParameter` runs the same checks before `resolve_type`.
The change keeps the critical operation short (R29, R30, R31) and keeps the type rules of the S7 platforms (R12, R13, A38).
It applies ADR-0143 (i2), ADR-0144, and ADR-0148 (c). The scope is WP3 of `docs/plan/2026-11-08-fix-plan.md` section 6.

## What Changes

- `recipe-app`: `UpdateParameterDefinition` finds out whether the type changes from the type rows that it read with the draft. It does not insert a row for this.
- `recipe-app`: when the type changes, the command runs `require_current_type` and `require_no_nested_array` before the unit lock and before `resolve_type`. This is the order of `AddParameter`.
- `recipe-app`: `OpenDraft` keeps the type rows of the platform that it reads already. No statement is new.
- `recipe-app`: a unit test shows that the new comparison gives the same answer as the comparison of `type_id`.
- `recipe-db`: a PostgreSQL test shows that a type that `require_no_nested_array` refuses takes no class-2 lock and writes no `DerivedTypeCreated` entry.
- `recipe-db`: a seed test shows that each platform has exactly one global `STRING` row and at most one global `WSTRING` row (design D2, F2).
- `CHANGELOG.md`: one entry in `[Unreleased]`.

The error codes, the audit entries, and the written rows of a successful request do not change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. Two requirements of `plant-model` already state the behavior: "No array of an array on S7-1200 and S7-1500" and "Stored struct from before the identifier rules". Thus `.openspec.yaml` sets `skip_specs: true`.

## Out of scope

- D1-2 (LOW): a `TypeExpr` that keeps its `RuleSet`. `design.md` D4 gives the reason. D1-2 stays open for a later change with its own ADR.
- That later change takes the next free ADR number. The plan reserves ADR-0153 for WP4.
- A move of the two checks before `Begin`. The check of a changed type needs the locked draft.
- A change of `reliability.md` §7.4. The checks run inside the transaction before the unit lock, as in `AddParameter`.
- The drift of the touch order in `data-model.md` §6.
- A change of a refusal inside `resolve_type`. For example, `E-PLT-013` with the reason `UnknownStruct` and `E-PLT-018` still come after the class-2 locks.
- A change of `AddParameter`, `DefineUdt`, or `RebindTypeRevision`.
- A change of `docs/architecture/` or of an ADR.

## Impact

- Code: `crates/recipe-app/src/recipe/update_definition.rs`, `crates/recipe-app/src/recipe/draft_session.rs`, `crates/recipe-app/src/plant/queries/type_text.rs`.
- Tests: `crates/recipe-app` (memory adapter) and `crates/recipe-db/tests/storage` (PostgreSQL, modules `race` and `seed`).
- Error order: a request with a refused changed type and a bad unit now gets `E-PLT-013` before `E-PLT-020` or `E-PLT-009`, as `AddParameter` does.
- Error order: a changed type that the two checks refuse now gets `E-PLT-013` before `E-PLT-018`. An example is a nested array or an array of a reserved struct name that is also too large.
