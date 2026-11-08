# Design

## Context

See `proposal.md` "Why" for the finding D1-1.

Current state of the code:

- `stage_definition` in `crates/recipe-app/src/recipe/update_definition.rs:158` locks the unit, calls `resolve_type`, and only then compares `resolved.type_id` with the stored type (lines 164 to 171).
- `resolve_type` in `recipe/definition_steps.rs:65` takes the class-2 keys, reads the type rows, and calls `DerivedTypes::ensure_expr`. That call inserts each missing `STRING(n)` row and each missing array row.
- `AddParameter` runs `require_current_type` before `Begin` and `require_no_nested_array` after `OpenDraft::open` (`add_parameter.rs:109`, `:116`). Both run before the unit lock and before `resolve_type`.
- `OpenDraft::open` reads every type row of the machine already (`draft_session.rs:159`, `resolve_trees`). It builds `PlatformTypes` and keeps only the `TypeTrees` of the parameters.
- `type_text` (`plant/queries/type_text.rs:16`) writes the type expression of a row. The tests in `type_text_tests.rs` show that the text parses and resolves to the same row (`a_written_text_parses_and_resolves_to_the_same_row_without_a_new_row`).

Normative sources: `reliability.md` §7.4, ADR-0143 (i2), ADR-0144, ADR-0148 (c), `data-model.md` §3.3, §4.6, §5.2, and the rows `AddParameter` and `UpdateParameterDefinition` of §6.

## Goals / Non-Goals

**Goals:**

- The command finds out whether the type changes without an insert and without a class-2 lock.
- The answer is the same as the answer of the comparison of `type_id` for every request.
- A type that `require_current_type` or `require_no_nested_array` refuses takes no class-2 lock and writes no `plc_type` row.

**Non-Goals:**

- No change of the lock order or of the statements of a successful request.
- No change of a refusal inside `resolve_type`. For example, `E-PLT-013` with the reason `UnknownStruct` and `E-PLT-018` still come after the class-2 locks. They can also come after an insert.
- No change of `TypeExpr`, `RuleSet`, or `recipe-web` (D1-2, see D4).

## Decisions

### D1. The command compares the canonical text of the type

The command computes one boolean before the unit lock:

```text
type changes  <=>  type_text(types, stored) != Some(input.type_expr.to_string())
```

`types` is the `PlatformTypes` of `OpenDraft`. `stored` is the `plc_type` of the parameter in the locked draft.
A new free function `expr_names_row(types, id, expr) -> bool` in `plant/queries/type_text.rs` holds the comparison. It returns `true` when the text of the row `id` equals the text of `expr`.

When `type_text` returns `None`, the function returns `false`. The command then treats the type as changed and runs both checks.

Alternatives:

- A lookup twin of `ensure_expr` that finds a row and never inserts. It is exact by construction. But it copies the resolution logic of `ensure_derived.rs`, and two paths must then stay in step.
- The `TypeTree` of the stored type in `OpenDraft::trees`. A leaf of the tree does not tell a global `STRING` row from a `STRING(n)` row of the machine. On `S7_1500`, `STRING` and `STRING(254)` are two rows with one tree shape. The comparison would then give a wrong answer.

### D2. The comparison equals the comparison of `type_id`

Notation: `m` is the machine and `p` is its platform. `R` is the set of rows that `OpenDraft::open` reads. `s` is the stored type of the parameter. `e` is `input.type_expr`.

The claim: `resolve_type(e).type_id == s` exactly when `type_text(R, s) == Some(e.to_string())`.

Four facts support the claim:

1. F1, immutable rows. Trigger 4.6 `plc_type_immutable` and `data-model.md` §5.2 forbid each `UPDATE` and `DELETE` of `plc_type`. Thus the row `s` and its element rows stay as `R` shows them. A concurrent command can add a row, but a new row is never `s`.
2. F2, one row for one identity. The unique indexes of `data-model.md` §3.3 give one row for each struct name and revision, each global primitive, each `STRING(n)` of a machine, and each array shape. Migration 0004 seeds exactly one global `STRING` row for each platform. It seeds exactly one global `WSTRING` row for each platform except `S7_300`, which has none. No index enforces this rule, because one global row lists several platforms. The seed test of task 1.5 pins it. The test `default_length_matches_the_seed_rows_for_all_ten_combinations` ties their length to `StringLength::default_length`.
3. F3, resolution by structure. `ensure_expr` maps each part of `e` to one row, as the table below shows. `ensure_row` reads the existing rows before it inserts a row.
4. F4, text by structure. `type_text` writes the same piece for each row that `TypeExpr`'s `Display` writes for the matching part. `TypeExpr::from_stored` with `RuleSet::Earlier` reads that text back to the same parts.

| Part of `e` | Row that `resolve_type` finds | Text of the part | Text of the row |
|---|---|---|---|
| Primitive `P` | the global row of `P` on `p` | `P` | `P` |
| `STRING` or `WSTRING` without a length | the global seed row of `p` | `STRING` | `STRING`, the global row has no length in its text |
| `STRING(n)` or `WSTRING(n)` | the row of `m` with the length `n` | `STRING(n)` | `STRING(n)`, a row of `m` keeps its length |
| `Name@r` | the struct row of `m` with the exact name and the revision `r` | `Name@r` | `Name@r` |
| One prefix `ARRAY[d] OF` | the array row of `m` with the element and the bounds `d` | `ARRAY[d] OF ` | `ARRAY[d] OF ` |

Proof in two directions:

- From the id to the text. Assume that `resolve_type` returns `s`. Then each part of `e` matches one row of the chain of `s` (F3). By the table, `type_text` writes the text of each part in the same order (F4). Thus the two texts are equal.
- From the text to the id. Assume that the two texts are equal. `Display` loses no part, so `e` has the parts of the chain of `s` (F4). Each row of that chain exists (F1) and is the only row of its identity (F2). `ensure_row` finds it before an insert (F3). Thus `resolve_type` returns `s`.
- `type_text` returns `None` only when `s` is not on `p`, when a row of its chain is missing, or when the rows form a cycle. `resolve_type` returns only rows on `p` with a full chain, so it cannot return `s`. Both answers are then "the type changes".

The comparison differs from the comparison of `type_id` in no case today. Two future changes would break it:

- A second global `STRING` or `WSTRING` row on one platform with a different length. `type_text` writes both rows as `STRING`. F2 excludes it today, and `EnsureDerivedType` inserts only rows of a machine.
- A struct lookup without case. The requirement "Stored struct from before the identifier rules" of `plant-model` requires the exact stored spelling.

Task 1.1 adds a unit test that checks the claim over the expression table of `type_text_tests.rs`. A change that breaks F3 or F4 then fails that test. A migration that breaks F2 fails the seed test of task 1.5.

### D3. `OpenDraft` keeps its `PlatformTypes`, and the checks run before the unit lock

`resolve_trees` returns the `PlatformTypes` that it builds, and `OpenDraft` keeps them in a new field `types`. No statement is new. The rows are valid for the stored type by F1, although the command reads them before the class-2 lock.

Alternative: a second `type_rows` read in `stage_definition`. It adds one statement of all type rows of the machine for no gain.

The new order of `stage_definition`:

1. `parameter_id` and the stored type from the locked draft.
2. When the type changes (D1): `require_current_type` and `require_no_nested_array` with the platform of the draft.
3. `lock_live_unit`.
4. `resolve_type`, then `set_definition` and `definition_diff` as today.

This order equals the order of `AddParameter`. Step 2 needs no I/O, so the command waits for no I/O between the draft lock and the unit lock (`reliability.md` §7.4).

A request with a refused changed type and a bad unit now gets `E-PLT-013` before `E-PLT-020` or `E-PLT-009`. `AddParameter` gives the same order. No spec or document states this order.

A changed type that the two checks refuse now gets `E-PLT-013` before `E-PLT-018`. An example is a nested array or an array of a reserved struct name that is also too large. Before this change, `resolve_type` gave `E-PLT-018` first. A reference to a missing struct revision now gets the reason `ReservedName` instead of `UnknownStruct`. The code stays `E-PLT-013`.

The checks stay inside the transaction, as the check of `AddParameter` after `OpenDraft::open` does. The comparison needs the locked draft. Thus the checks cannot run before `Begin`. The owner fixed this order on 2026-11-08, and `reliability.md` §7.4 does not change.

### D4. D1-2 does not join this change

Plan section 6.2 lets D1-2 join only when the type-level fix is small. It is not small:

- Direction A, a `TypeExpr` that keeps its `RuleSet`. It changes `recipe-domain` (`type_expr.rs` and its derived `PartialEq`). It changes `recipe-app` (`current_type.rs`, `resolve_type`, and the three calls at `define_udt.rs:101`, `add_parameter.rs:109`, `update_definition.rs:167`). It also touches `recipe-web`: four calls of `TypeExpr::from_stored` (`definition.rs:141`, `fields.rs:407`, `prepare.rs:83`, `parameter_table.rs:183`) and the `TypeExpr` compares in `pages/parameters/reply.rs`.
- Direction B, a command input that only `RuleSet::Current` builds. It cannot serve `UpdateParameterDefinition`, because a kept type can be `Int@1` (ADR-0143 (i2)). `recipe-web` cannot know whether the type changes. Thus B closes D1-2 only for `AddParameter` and `DefineUdt`.

Both directions touch three crates and at least eight files. That size is the reason. Thus the change writes no ADR, and D1-2 stays open.

Plan section 2 rule 2 says that WP1, WP2, and WP3 touch disjoint crates. That premise is wrong, because WP1 also changes `recipe-app`. Thus this design does not use the rule as a reason.

A later D1-2 change takes the next free ADR number. The plan reserves ADR-0153 for WP4.

## Risks / Trade-offs

- [The rows of `OpenDraft` come before the class-2 lock] → F1 makes them valid for the stored type. A row that a concurrent command adds can only answer "the type changes", and `resolve_type` then reads the rows again under the lock.
- [A later change of `type_text` or of `Display` breaks D2] → The equivalence test of task 1.1 compares both answers over every expression of the table.
- [A later migration adds a second global `STRING` or `WSTRING` row to one platform] → The seed test of task 1.5 fails.
- [The error order changes for a request with two faults, also for `E-PLT-013` before `E-PLT-018`] → The order equals `AddParameter`. Task 1.2 runs the full tests of `recipe-app` and stops on a test that asserts the old order.
- [`OpenDraft` holds more memory] → The struct holds rows that `open` reads already. The extra memory is the `PlatformTypes` of one machine.

## Migration Plan

No migration and no data change. A rollback is a revert of the commit.
