# Design

## Context

See `proposal.md`, section Why. ADR-0195 gives the sites, the evidence, the options, and the decision. `reliability.md` §1.1 gives the carriers and the log rule. `data-model.md` §6 and `domain-model.md` §5 give the shared rule of a storage defect.

## Goals / Non-Goals

**Goals:**

- One mechanism from the domain to `E-DB-008` for every site of ADR-0195 items (f) to (n).
- The request part keeps its code at every site.
- A user with `ParameterEdit` can replace a type that does not resolve in the browser (ADR-0195 (q), (ab), (ad), (ae)).
- A failing test first for each site.

**Non-Goals:**

- The sites of the section "Gaps outside this ADR" of ADR-0195.
- A change of the read checks of ADR-0118. The reads keep `check_stored_parameter`.

## Decisions

### D1: The carrier `DraftError::Stored`

ADR-0195 items (a) to (c). `StoredDefect` lives next to `StructureFault` in `recipe_domain::recipe::read_check`. It is a classification, not an error type with a row in `reliability.md` §1.2. Its constructors take the key of the parameter when the site has one.

`From<DraftError> for AppError` maps `Stored` in one arm. A new constructor `StorageError::broken_stored_part(defect: StoredDefect)` builds the cause. It reads the key from `defect.key()`, so the key has one source. `broken_stored_parameter` keeps its arguments `ParameterKey` and `StructureFault` (`crates/recipe-app/src/error/storage.rs` lines 145 to 150). The cause names the key when one exists, and the code of the broken rule. Option B and option C of ADR-0195 are the alternatives.

### D2: Classify at the call site, not in the rule function

The rule functions stay shared with the approval check and the reads. The caller wraps the error of a stored part:

- `set_value` wraps the error of `stored_limits_under` (`draft/value.rs` line 33).
- `replace_limits` wraps the error of `stored_value_under` (`draft/value.rs` line 63).
- `set_value` and `replace_limits` wrap a failed resolution of the stored type, as ADR-0195 (w) defines it. That is the `PlantError` of `resolved_tree` (`draft/add/step_definition.rs` lines 93 to 104), matched by variant and reason. The wrapped errors are `PlantError::TypeCycle`, `PlantError::TypeTooLarge`, and `PlantError::DerivedTypeInvalid` with the reason `RowShape`.
- Every other resolution error stays unwrapped. That is `E-PLT-013` with the reason `NotOnPlatform`, for a root or a member that the platform does not hold (ADR-0195 (x)).
- `check_layout` wraps the errors of `check_platform_rule`, `check_tree_rules`, and `occupy` only when `rules` is `AddressRules::Skip` (`draft/s7.rs` lines 122 to 150). `find_overlap` stays unwrapped, so an overlap keeps `E-RCP-022` (ADR-0149 (a), (h)).
- The rebind wraps the stored errors of `map_value`, `map_limits`, and the I7 check (D3). It wraps the platform, width, and alignment errors of `check_definition` at `rebind/apply.rs` line 212.
- `Draft::load` returns `Stored` for a duplicate. It keeps `RecipeError::VersionNotEditable` as `DraftError::Recipe`.

An array-only cycle gives `E-PLT-013` with the reason `RowShape` (`plant/type_tree.rs` lines 473 to 485). The doc comment of `RowShape` says that only a corrupt stored row or a bug gives it (`plant/error.rs` lines 57 to 61). So the reason tells a storage defect apart from a type that the platform does not hold, without a new variant. `DefineUdt` keeps its codes. The queries keep their codes, except `GetParameterForm` (D7) and `GetParameter` and `ListParameters` (D8).

A stored row that fails `PlcType::from_stored` never reaches the resolution. The adapter of `recipe-db` maps it to `corrupt` (`crates/recipe-db/src/transaction/type_rows.rs` lines 137 to 139 and 202). `type_rows` then fails for the whole machine. So every read and command that reads the type rows of that machine gives `500 E-DB-008`. `GetMachine` reads no type rows (`crates/recipe-app/src/plant/queries/machines.rs` lines 222 to 236). ADR-0195 (ab), (ad), and (ae) do not cover such a row.

### D3: A defect stops the rebind plan

- `map_value` and `map_limits` return the defect apart from the entries of the request.
- `map_limits` first checks every stored limit row against the old tree with `stored_limits_under(..., old_tree, RuleSet::Earlier)`. A row on a path that the old tree does not have is a defect. A valid row of a removed member stays the entry `E-RCP-029`.
- `map_parameters` finds the I7 defect between a kept value leaf and a kept limit, also when the same parameter has request entries. It checks only the kept paths, as ADR-0195 (l) says. So `plan_rebind` finds the I7 defect that `rebound_parameter` finds today (`rebind/apply.rs` lines 203 to 209).
- Today `map_value` returns `Err(entries)` when any entry exists (`rebind/value.rs` lines 55 to 58). `map_parameters` keeps the mapped value only when both mappings succeed (`rebind/map.rs` lines 58 to 66). So the I7 check cannot use the mapped value. One way is a check of the stored leaves at the kept paths against the kept limit rows.
- The author checks that a kept leaf keeps its stored value. `kept_members` selects the kept members (`rebind/level.rs` line 41, called at `rebind/value.rs` line 45). `map_element` copies the value of a kept member (`rebind/value.rs` lines 101 and 113 to 114).
- `map_parameters` returns the first defect in tag order and drops the entries (ADR-0195 item (n)).
- The address faults of ADR-0195 (i) and (m) come after a plan without entries. They need the real tree of the new type. That tree exists only after `ensure_new_types` (`crates/recipe-app/src/recipe/rebind.rs` lines 86 to 92).
- `rebound_parameter` keeps its I7 check and wraps its fault as `Stored` too.

### D4: The arms of the application

`OpenDraft::open` and `lock_live_unit` use `StorageError::invariant` (ADR-0195 items (o) and (p)). `OpenDraft::open` and `RebindTypeRevision` turn the error of `PlatformTypes::collect` into `E-DB-008` through a `StoredDefect` without a key (item (k)).

### D5: Tests that build a stored defect

- `recipe-domain`: a hand-built `StoredDraft` and hand-built `TypeTrees` hold the defect. `Draft::load` and the draft methods run on them. The helper `assert_stored(&error, key, code)` checks the variant `Stored`, its `key()`, and its inner code.
- A test of a request part or of the approval check asserts that the error is not `Stored`, and asserts its code.
- `recipe-app`: the memory store of `recipe-app/src/memory/` seeds a parameter row without a trigger, for example with `seed_parameter` (`memory/recipe/seed.rs` line 86).
- `seed_struct_chain` and `seed_oversized_struct` give a type above a size limit (`memory/fixtures/deep.rs` lines 35 and 111).
- A shared fixture `seed_struct_cycle(fixture, machine, platform)` in `memory/fixtures/` stores two structs that contain each other. It moves from the test at `plant/queries/types_tests.rs` lines 384 to 390. Each test seeds a defect type on the machine and the platform of its draft.
- The memory store has only the hooks `on_read_draft` and `on_lock_version_shared` (`memory/core/store.rs` lines 326 and 346). Tasks 4.1 and 4.3 add the two hooks of this change.
- `seed_type` replaces a row with the same identifier (`memory/types/seed.rs` lines 143 to 151). So two kept type rows with one identifier need the hook of task 4.3.
- `recipe-web`: route tests with the memory store check the `500` answer, the one `ERROR` line, and the repair through the page.
- `recipe-db` needs no test. The adapter reads the rows and does not classify them.

### D6: The audit diff of a type that does not resolve

ADR-0195 item (q). The fallback applies only to the old side of `definition_diff_of`, when the type changes and `tree_of` fails for the old type. The old side then holds the whole stored value under the key `""`. The diff always writes `before_state.value` and `after_state.value` in that case, also when the two sides are equal.

The new side, `value_diff_of`, and `rebind_diff_of` keep the error of a type without a tree (`audit/diff.rs` lines 74 to 118). The test `a_type_without_a_tree_gives_a_plant_error` stays green (`audit/diff/tests.rs` lines 345 to 353). `LeafPath::whole()` is `pub(crate)` in `recipe-domain` (`plant/leaf_path.rs` lines 87 to 90). So the diff uses the literal key `""` or a public text constant.

### D7: The parameter form query

ADR-0195 items (z) to (ac). `GetParameterForm` maps three faults to `E-DB-008`:

- The error of `PlatformTypes::collect` (`queries/parameter_form.rs` lines 274 to 275).
- A failed resolution of the stored type in `rule_faults_of` (`parameter_form.rs` line 169), as D2 defines it.
- A missing type text. `text_of` already gives `E-DB-008` (`queries/parameters.rs` lines 154 to 157). `type_text` gives no text for a missing row on the array chain of the root (`plant/queries/type_text.rs` lines 15 to 24). That row is the root row or an element row.

A member type that the platform does not hold keeps `422 E-PLT-013`. The preview of a type of the request keeps its codes (`parameter_form.rs` lines 283 to 286). The error of `PlatformTypes::collect` covers two kept type rows with one identifier (`plant/type_tree.rs` lines 52 to 53).

For the purpose `UpdateParameterDefinition`, a failed resolution, a type that the platform does not hold, and a missing type text open the form.

The repair needs two posts. The edit route shows the form again and writes nothing when the sent type differs from `value_type` (`pages/parameters/prepare.rs` lines 165 to 167). A page without a grid has an empty `value_type` (`pages/parameters/form_view.rs` lines 333 to 334). So the first post runs the form query with the purpose `UpdateParameterDefinition` and a preview of the sent type. The form then has no rule faults and, for a missing text, no type text. The form query writes `type_text` as an `Option` already (`parameter_form.rs` lines 277 to 281).

### D8: The parameter pages

ADR-0195 items (ad) and (ae). `ParameterView` gets an optional tree and an optional type text. For a row whose type does not resolve, `ListParameters` skips the tree, the value text, and the rule faults. The code is in `queries/parameters.rs` lines 210 to 218 and 266 to 290. `texts_of` at line 372 skips a type without a text, so its map holds no entry for that type. `GetParameter` does the same (`queries/parameters.rs` lines 529 to 535).

The read check of a row needs its tree (`queries/parameters.rs` lines 274 to 276). So a row without a tree skips the read check that ADR-0121 (c) requires for every row. ADR-0195 (ad) records this exception, and task 6.1 adds the note to ADR-0121.

Today the table cell of a row without a value text shows the word of `parameters-value-composite` (`pages/versions/parameter_table.rs` lines 100 to 110 and 298 to 320). A row with a type text keeps that word. A row without a type text gets an empty value cell, because its kind is unknown. A missing root row, a missing element row, and a root array-only cycle have no type text. A missing type text shows an empty type text (`unwrap_or_default`).

`recipe-web` uses the tree of the view in these places:

- `GridTexts::stored` (`pages/parameters/form_view.rs` lines 114 to 122).
- `DefinitionTexts::stored` reads `view.type_text` (`pages/parameters/form_view.rs` line 69). The type field of the edit form is empty for a view without a type text.
- `stored_grid` reads the tree through `GridTexts::stored` (`pages/parameters/page.rs` lines 79 to 86).
- `values_answer` and the value route (`pages/parameters/value.rs` lines 93 to 95 and 162 to 168).
- `stored_shown` and `form_or_grid` (`pages/parameters/page.rs` lines 96 to 134).

`Shown.grid` is an `Option` already (`pages/parameters/form_view.rs` lines 328 to 337). The page leaves it empty for a view without a tree. A call site that needs the tree answers `500` for such a view. The value route never reaches that case, because the form query answers first (D7).

## Risks / Trade-offs

- [The rebind mapping gets a second result path] → D3 keeps the entries of the request in one list and the defect apart. A test covers a draft with both.
- [Existing tests assert the old codes] → each task names the tests that it changes. Each task ends with a workspace that builds and passes `cargo test`.
- [The parameter page gets a second render path] → route tests cover the editor and a reader without `ParameterEdit`. `is_editable` decides between the two paths (`pages/parameters/page.rs` lines 67 to 71). It also checks the mode, so read-only mode gives the read-only definition (`pages/request.rs` lines 117 to 120).
- [The log chain holds a stored value] → `reliability.md` §1.1 forbids only secrets in an error. A value is not a secret.

## Migration Plan

No migration. The change ships with the next release. A rollback is the previous image.
