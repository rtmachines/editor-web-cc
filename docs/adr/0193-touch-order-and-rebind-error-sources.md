# ADR-0193: Touch order of five draft rows, error sources of the rebind row

Status: Accepted. Amends `data-model.md` §6 and `domain-model.md` §3 and §5.2
Date: 2026-11-18
Deciders: project owner

## Context

A review found two gaps in the operation rows of `data-model.md` §6. The review lists them as findings CR-07 and CR-08. A check of the code for CR-08 found one more code and one more source of a code in the row `RebindTypeRevision`. Two reviews of the first draft of this ADR found more gaps in the same rows.

### Position of the touch

ADR-0191 (h) added a sentence to the rule "Every draft command then runs ..." of §6. The sentence puts the touch and `INSERT draft_editor` after the domain check. It also puts them before the first write of a `parameter`, `parameter_limit`, or `parameter_translation` row. Five rows do not agree with this rule:

- The lock column of `SetParameterValue` puts the touch right after the `row_version` compare.
- The rows `ReplaceLimits`, `UpsertTranslation`, `RemoveTranslation`, and `RemoveParameter` copy that lock column with "same".
- The statement columns of the five rows start with `INSERT draft_editor`. The domain check is not in the rows.

The code runs the same steps in all five commands:

1. `OpenDraft::open` takes the recipe lock and the version lock and compares `row_version` (`draft_session.rs` lines 195 to 213).
2. The domain method of the command checks the request.
3. `OpenDraft::touch` runs the touch and `INSERT draft_editor` (`draft_session.rs` line 234, `crates/recipe-db/src/transaction/draft_rows.rs` lines 123 to 141).
4. The adapter writes the `parameter`, `parameter_limit`, or `parameter_translation` rows.

The lines of steps 1 to 4 in `crates/recipe-app/src/recipe/` are:

| Command | Step 1 | Step 2 | Step 3 | Step 4 |
|---|---|---|---|---|
| `SetParameterValue` | `set_value.rs` 77 | 79 | 81 | 84 to 87 |
| `ReplaceLimits` | `replace_limits.rs` 77 | 79 to 81 | 83 | 86 to 89 |
| `UpsertTranslation` | `translation.rs` 93 | 96 to 98 | 100 | 110 to 113 |
| `RemoveTranslation` | `translation.rs` 170 | 172 | 173 | 175 to 179 |
| `RemoveParameter` | `remove_parameter.rs` 71 | 73 | 74 | 76 to 80 |

`OpenDraft::touch` is the only caller of `touch_draft` outside the tests. `OpenDraft::open` writes nothing. The touch updates a row that the command already holds `FOR UPDATE`. The touch and `INSERT draft_editor` take no lock that conflicts with the lock order of §6.

ADR-0191 (f) and (g) moved the touch in the rows `AddParameter`, `UpdateParameterDefinition`, and `RebindTypeRevision`. ADR-0191 (d) added two codes to the error column of the rows `SetParameterValue` and `ReplaceLimits`. ADR-0191 did not move the touch in the five rows of this ADR.

### Domain checks of the three rows without a type

The domain method of `UpsertTranslation` checks only that the parameter exists (`crates/recipe-domain/src/recipe/draft/translation.rs` line 21). The domain method of `RemoveTranslation` also checks the translation (line 39). It gives `TranslationNotFound` (`E-RCP-050`) at line 40. The domain method of `RemoveParameter` checks only the parameter (line 60).

The error cells of `RemoveTranslation` and `RemoveParameter` say "0 rows → `TranslationNotFound`" and "0 rows → `ParameterNotFound`". The domain check gives these two codes before the write. The rule of §6 after "Every draft command then runs ..." already names `ParameterNotFound` (`E-RCP-051`) and `TranslationNotFound` (`E-RCP-050`). A delete that changes 0 rows gives another code:

- `delete_translation` and `delete` return `missing_row` (`crates/recipe-db/src/transaction/draft_writes.rs` lines 309 and 326).
- `missing_row` builds `RowError::Missing` (`crates/recipe-db/src/error.rs` lines 122 and 123). `RowError` converts to `StorageError::Other`.
- `reliability.md` §1.2 gives `StorageError::Other` the code `E-DB-008` with HTTP 500.

### Statement order of the row `RebindTypeRevision`

The statement column of the row `RebindTypeRevision` puts `EnsureDerivedType` before the mapping of the values. Its lock column takes the class-2 lock "when a new array type is needed". The code runs these steps (`rebind.rs` is `crates/recipe-app/src/recipe/rebind.rs`):

1. `OpenDraft::open` with `MachineLock::Shared` (`rebind.rs` line 132).
2. `Draft::plan_rebind` checks the target and maps the value and the limits of every moved parameter (`rebind.rs` line 152).
3. The S7 nesting check of the type of each moved parameter (`rebind.rs` lines 155 to 157).
4. The class-2 lock with the key `ARRAY` (`rebind.rs` lines 158 to 160). `rebind_locks` gives this key for every plan with an array shape (`crates/recipe-app/src/plant/derived_lock.rs` lines 44 to 49). Every moved array parameter gives a shape.
5. `EnsureDerivedType` for each array shape (`rebind.rs` line 88).
6. `Draft::apply_rebind` checks the definition of every moved parameter and the changed draft (`rebind.rs` lines 90 to 92).
7. The touch and `INSERT draft_editor` (`rebind.rs` line 93), then the writes.

### Error sources of the row `RebindTypeRevision`

The last sentence of the error column of the row `RebindTypeRevision` names `TypeTooLarge` (`E-PLT-018`) from `EnsureDerivedType` for a new array type. The same sentence then lists `E-RCP-055`, `E-RCP-056`, and `E-RCP-057`. A reader can think that `EnsureDerivedType` gives all four codes. The code gives these sources:

- `E-PLT-018` from the plan: `plan_rebind` calls `TypeTree::rebind` (`crates/recipe-domain/src/recipe/rebind/target.rs` line 76). It gives `PlantError::TypeTooLarge` above `MAX_LEAVES` or `MAX_DEPTH` (`crates/recipe-domain/src/plant/type_tree/rebind.rs` lines 33, 38, and 41). The test `a_target_tree_above_the_leaf_limit_gives_plt_018_and_changes_nothing` (`rebind/tests.rs` line 185) covers this path.
- `E-PLT-018` from the S7 nesting check: `rebind.rs` lines 155 to 157 call `TypeTree::check_s7_nesting` for the type of each moved parameter. It gives `PlantError::TypeTooLarge` with `TypeLimit::S7Nesting` (`crates/recipe-domain/src/plant/type_tree.rs` lines 218 to 224). The test `adr_0179_rebind_an_array_of_ten_levels_on_s7_gives_plt_018_and_changes_nothing` (`rebind/tests.rs` line 356) covers this path. ADR-0179 (f) gives the rule.
- `E-PLT-018` from `EnsureDerivedType`: `ensure_new_types` (`crates/recipe-app/src/recipe/rebind/new_types.rs` line 38) calls `DerivedTypes::ensure_shape` (`crates/recipe-app/src/plant/ensure_derived.rs` line 116).
- `E-RCP-055` from the plan: `map_value` (`crates/recipe-domain/src/recipe/rebind/value.rs` line 38) runs inside `plan_rebind`. A new leaf with too many significant digits gives `RecipeError::PrecisionExceeded` through `leaf_error` (`value.rs` line 158).
- `E-RCP-056`, `E-RCP-057`, and `E-RCP-058` from the domain check of the definitions: `Draft::apply_rebind` calls `check_definition` for each moved parameter (`crates/recipe-domain/src/recipe/rebind/apply.rs` line 212). `check_definition` runs the data block rule, the end rule, and the standard access rule (`crates/recipe-domain/src/recipe/draft/s7.rs` lines 166, 168, and 169).
- `E-RCP-058` in detail: `check_standard_access_rule` gives `RecipeError::S7TypeNotAddressable` (`s7.rs` line 242). The tests at lines 88 and 108 of `rebind/adr_0178_tests.rs` cover this path.

The row names no `E-RCP-058`, no plan source, and no S7 nesting check. The rebind row of `domain-model.md` §5.2 also names no plan source of `E-PLT-018`. The list of `RebindNeedsValue` and `RebindDropsLimit` has one entry per missing leaf and per dropped limit (`crates/recipe-domain/src/recipe/rebind/map.rs` lines 64 and 65). The row says "one list entry per failing parameter".

### Invariant I12

The mechanism of I12 in `domain-model.md` §3 says that the use case compares `row_version` and then runs the touch statement. It does not name the domain check between the two steps.

### Gaps outside this ADR

These earlier gaps stay out of the scope of this ADR. The fix plan records them:

- The rebind codes that only stored data that breaks a rule can give, for example `E-RCP-004` and `E-RCP-018` of a kept limit.
- The stored-data codes of the rows `SetParameterValue` and `ReplaceLimits`.
- The shared codes of `api.md` §5 in the rows of §6, and `E-WEB-011` in the row `EnsureDerivedType`.
- The SQL text of the rows: the predicates by `parameter_key` and the bulk insert of the limit rows.
- `InvalidLangCode` (`E-RCP-027`) in the error cell of `RemoveTranslation`.

## Decision drivers

- The normative text states the behavior of the shipped code when the code meets the requirements (R34).
- Each command lists its complete error set (R35).
- The error column names the source of each code when the row names a source.
- One rule of §6 and the rows of §6 give the same statement order.

## Considered options

### Option A: Change the text to the code

- Pros: no code change. The rows then state the statement order of the code.
- Cons: none.

### Option B: Change the code to the text

- Pros: the rows stay.
- Cons: the touch then writes before the domain method of every failed command. The rollback undoes that write, so the change adds work and no safety. ADR-0191 rejected the same option for the same reason. The codes come from the domain rules, so the code cannot drop them.

## Decision

The project uses **Option A**. No code, no migration, and no spec change.

Row `SetParameterValue` of `data-model.md` §6:

- (a) The lock column drops the touch. The new text is: "`recipe FOR UPDATE`, `recipe_version FOR UPDATE` with the `row_version` compare. No machine lock and no unit lock".
- (b) The statement column starts with the domain check. The touch and `INSERT draft_editor` follow it, before `UPDATE parameter`.
- (c) The last sentence of the statement column becomes "The command writes no limit row and no translation row".

Row `ReplaceLimits`:

- (d) The statement column starts with the domain check. The touch and `INSERT draft_editor` follow it, before `DELETE parameter_limit`.

Rows `UpsertTranslation`, `RemoveTranslation`, and `RemoveParameter`:

- (e) Each statement column starts with the domain check. The touch and `INSERT draft_editor` follow it, before the write of the row. The lock columns keep "same", so after item (a) they name no touch.
- (f) In `UpsertTranslation` and `RemoveParameter`, the first sentence is "The domain checks the parameter". In `RemoveTranslation`, the first sentence is "The domain checks the parameter and the translation".
- (g) The error cell of `RemoveTranslation` becomes "`TranslationNotFound` from the domain check. 0 rows → `E-DB-008`". The error cell of `RemoveParameter` becomes "0 rows → `E-DB-008`". The rule of §6 after "Every draft command then runs ..." gives `ParameterNotFound`.

Row `RebindTypeRevision`:

- (h) The lock column takes the class-2 lock with the key `ARRAY` when a moved parameter is an array. The lock follows the plan and the S7 nesting check.
- (i) The statement column follows the order of the code. The plan checks the target and maps the values and the limits. The S7 nesting check, `EnsureDerivedType`, and the domain check of the definitions follow. Then the touch, `INSERT draft_editor`, and the writes follow.
- (j) The error column gives three sentences for `TypeTooLarge` (`E-PLT-018`), one for each source: the plan, the S7 nesting check (ADR-0179 (f)), and `EnsureDerivedType`.
- (k) New sentences of the error column give the codes of the plan and of the domain check: "`PrecisionExceeded` (`E-RCP-055`) from the plan. Domain check of the definitions: `S7AddressPastBlockEnd` (`E-RCP-056`), `S7DataBlockOutOfRange` (`E-RCP-057`), `S7TypeNotAddressable` (`E-RCP-058`)". Like the other cells of §6, the cell has no final period.
- (l) The list of `RebindNeedsValue` and `RebindDropsLimit` has "one list entry per missing leaf and per dropped limit".

`domain-model.md`:

- (m) The row `RebindTypeRevision` of §5.2 adds the plan source of `E-PLT-018`. The new sentence is "It also covers the type of a moved parameter with more leaves or levels than the size limits of section 2.1".
- (n) The mechanism of I12 in §3 runs the touch statement after the domain check. The new sentence is "Then, after the domain check, it runs the touch statement (`data-model.md` section 6)".

## Consequences

### Positive

- The rows `SetParameterValue`, `ReplaceLimits`, `UpsertTranslation`, `RemoveTranslation`, and `RemoveParameter` state the statement order of the code. They agree with the rule of ADR-0191 (h).
- The row `RebindTypeRevision` states the statement order of the code.
- The error cell of `RebindTypeRevision` gives the complete error set of a request on valid stored data. It names the source of each `E-PLT-018` path and of each code of the plan and of the domain check of the definitions. The statement column gives the step of every other code.

### Negative

- The Negative consequence of ADR-0191 names three rows with another touch position. After this ADR, all eight draft rows have that position. ADR-0191 is Accepted and stays unchanged, so its bullet no longer holds.
- The gaps of the section "Gaps outside this ADR" stay open until the fix plan closes them.

### Risks and mitigations

- Risk: a later change adds a write before the domain method of one of the five commands. Then its row no longer states the code.
- Mitigation: the doc comment of `OpenDraft::touch` states the position of the touch (`draft_session.rs` line 228). No test checks the position.
- The atomic tests of `crates/recipe-db/tests/storage/atomic/draft.rs` prove that a failed command leaves no partial write. Examples are `set_value_fails_on_the_parameter_update_and_keeps_the_value_and_the_touch_state` and `replace_limits_fails_on_the_limit_insert_and_keeps_the_old_limit`.

## Related

- ADR-0015, ADR-0116, ADR-0117, ADR-0144, ADR-0146, ADR-0149, ADR-0178, ADR-0179 (f), ADR-0187, ADR-0191 (f), (g), and (h)
- `data-model.md` §6, `domain-model.md` §3 and §5.2, `reliability.md` §1.2
- Review findings CR-07 and CR-08, and the two reviews of the first draft of this ADR
- `overview.md` §7: R27, R29, R34, R35
