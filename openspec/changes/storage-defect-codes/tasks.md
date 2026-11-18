# Tasks

Each code task writes its failing test first and runs it red before it changes code. The crate list in parentheses names every crate that the task changes. One agent runs one task.

- "Rewrite" means: change the assertion of an existing test to the variant `DraftError::Stored`, its `key()`, and its inner code. Use the helper `assert_stored` of task 1.1. Run the test red, then change the code.
- A test of a request part or of the approval check asserts that the error is not `Stored`, and it asserts the code.
- A "guard" is a test that passes before and after the change. It is not a "Test first" step.
- A test that seeds a defect type stores its rows on the machine and the platform of the draft. `PlatformTypes::collect` drops the rows of another machine (`plant/type_tree.rs` lines 43 to 50).
- Such a test first asserts the resolution result of the seeded type, for example `TypeCycle`, `TypeTooLarge`, or the reason `RowShape`. Then it runs the command or the query.
- A task that changes a function also updates the `# Errors` section of its doc comment.
- Each task ends with a workspace that builds and passes `cargo test --workspace`. A task that breaks an existing test names it and updates it.

## 1. The carrier of a storage defect

- [ ] 1.1 (recipe-domain, recipe-app) Add `StoredDefect` next to `StructureFault` in `recipe/read_check/mod.rs` and the variant `DraftError::Stored(Box<StoredDefect>)` with its `code()` arm (design D1). Add `StorageError::broken_stored_part(defect: StoredDefect)`, which reads the key from `defect.key()`, and the `Stored` arm of `From<DraftError> for AppError` (`error/app.rs` lines 214 to 222). Change the comment of that impl to cite ADR-0195. Add the helper `assert_stored(&error, key, code)` to the test support of `recipe-domain`.
  - Test first in `recipe-domain` `recipe/error/tests.rs`: `code()` of `Stored` returns the inner code, and `key()` returns the key or `None`.
  - Test first in `recipe-app` `error/app/tests.rs`: `Stored` with a key gives `E-DB-008`, and the source chain names the key and the inner code. `Stored` without a key gives `E-DB-008`, and the chain names the inner code.
  - `DraftError` has no `#[non_exhaustive]` (`recipe/error.rs` lines 608 to 619). Add the `Stored` arm to every exhaustive match. In `draft/add/content.rs` lines 36 to 37 and 160 to 161, the arm returns `None`.
  - The test matches are in `draft/s7/tests.rs`, `draft/s7/layout_tests.rs`, `draft/value/tests.rs`, `draft/definition/tests.rs`, `rebind/apply/tests.rs`, `rebind/limit/tests.rs`, and `rebind/value/tests.rs`.

## 2. The draft sites

- [ ] 2.1 (recipe-domain, recipe-app) `Draft::set_value` wraps the error of the stored limits (`draft/value.rs` line 33) as `Stored`.
  - Rewrite `a_stored_limit_outside_the_type_range_gives_e_rcp_006` in `draft/value/tests.rs`. `a_value_above_the_stored_limit_gives_e_rcp_004` asserts not `Stored`.
  - Test first in `recipe-app` `recipe/set_value` tests: a seeded `DATE` limit `max` `72900` on `S7_1500` gives `E-DB-008`, and the memory store holds no write.
- [ ] 2.2 (recipe-domain, recipe-app) `Draft::replace_limits` wraps the error of the stored value (`draft/value.rs` line 63) as `Stored`.
  - Rewrite `a_stored_value_of_the_wrong_form_gives_e_rcp_005_for_limits` in `draft/value/tests.rs`. A new limit outside the type range gives `E-RCP-006` and not `Stored`.
  - Test first in `recipe-app` `recipe/replace_limits` tests: a seeded `DATE` value `72900` on `S7_1500` gives `E-DB-008`, and the memory store holds no write.
- [ ] 2.3 (recipe-domain, recipe-app) `set_value` and `replace_limits` wrap a failed resolution of the stored type as `Stored` (design D2, ADR-0195 (w)). That is the `PlantError` of `resolved_tree`, matched by variant and reason: `TypeCycle`, `TypeTooLarge`, or `DerivedTypeInvalid` with the reason `RowShape`.
  - Test first in `draft/value/tests.rs`: one table test for each method with hand-built `TypeTrees`. A `TypeCycle` result, a `TypeTooLarge` result, and an array-only cycle (`RowShape`) give `Stored`.
  - In the same table, two cases give `E-PLT-013` with the reason `NotOnPlatform` and not `Stored`. They are an asked root without a row and a struct with a member that the platform does not hold. `a_type_that_the_trees_do_not_offer_gives_e_plt_013_for_a_value` stays green.
  - Move the cycle seed of `plant/queries/types_tests.rs` lines 384 to 390 into a shared fixture `seed_struct_cycle(fixture, machine, platform)` in `recipe-app` `memory/fixtures/`. The rows there belong to `M12` through `owner()` (line 48), so that test passes `owner()`.
  - Test first in `recipe-app` `recipe/set_value` tests: seed the cycle on `M1` and the platform of the draft of `scenarios::zone_draft` (`memory/fixtures/scenarios.rs` line 106). First assert that the root resolves to `TypeCycle`.
  - In the same tests: a parameter of that cycle gives `E-DB-008` and no write. A parameter of a seeded array-only cycle on `M1` gives `E-DB-008` through `ReplaceLimits`. Its root first resolves to the reason `RowShape`.
- [ ] 2.4 (recipe-domain, recipe-app, recipe-web) `check_layout` with `AddressRules::Skip` wraps the errors of `check_platform_rule`, `check_tree_rules`, and `occupy` as `Stored` (`draft/s7.rs` lines 122 to 150).
  - Test first in `draft/s7/tests.rs`: `set_value` on `FEED` with a stored `DINT` parameter `SPEED` at `DB1.DBW0` on `S7_300` gives `Stored` with the key of `SPEED`. `check_s7_layout_for_approval` on the same draft gives `E-RCP-054` and not `Stored`.
  - In the same file: `replace_limits` on a parameter with its own stored misaligned address gives `Stored`. `add_parameter` with a request address of the wrong width gives `E-RCP-054` and not `Stored`.
  - Guard in `draft/approval/tests.rs`: `check_for_approval` on a draft with a stored `INT` value `40000` gives `DraftError::Recipe` with `RecipeError::ValidationFailed` (`E-RCP-020`) and an item `E-RCP-005`. It is not `Stored` (ADR-0120 (f)).
  - Test first in `recipe-app`: `AddParameter`, `UpdateParameterDefinition`, and `RebindTypeRevision` on a draft with a seeded `DINT` parameter at `DB1.DBW0` on `S7_300` give `E-DB-008` and commit nothing. The command does not change that parameter.
  - Test first in `recipe-web`, in the HTTP tests of `pages/parameters`: `POST .../{key}/value` for `FEED` on the same draft gives `500`. The answer shows `E-DB-008` and the `request_id`. The test captures one `ERROR` line with `error_code` `E-DB-008` and the key of `SPEED` in `chain`.
  - This task breaks the tests of `draft/s7/tests.rs` that run `check_every` (`draft/support.rs` lines 65 to 68) and expect a rule code. Move each to `check_s7_layout_for_approval`, or rewrite it with `assert_stored`.
  - The broken tests in `draft/s7/tests.rs`: `an_unresolved_parameter_with_an_address_on_twincat3_gives_e_rcp_053`, `an_unresolved_parameter_without_an_address_on_s7_300_gives_e_rcp_003`, `a_struct_at_an_odd_offset_gives_e_rcp_023`, `bool_at_a_byte_address_gives_e_rcp_054_with_width_x`, `codesys_with_an_address_gives_e_rcp_053`.
  - Also: `int_at_a_double_word_address_gives_e_rcp_054_with_width_w`, `int_at_an_odd_offset_gives_e_rcp_023`, `s7_300_without_an_address_gives_e_rcp_003`, `the_first_error_is_the_one_of_the_first_tag`, `twincat3_with_an_address_gives_e_rcp_053`.
  - Check the users of `check_every` in `draft/s7/layout_tests.rs`, `draft/s7/prop.rs`, and `draft/approval/tests.rs` the same way. The overlap tests (`E-RCP-022`) stay green.
- [ ] 2.5 (recipe-domain, recipe-app) `Draft::load` returns `DraftError` and gives `Stored` for two parameters with one tag, key, or identifier (`draft/load.rs` lines 27 to 39).
  - Rewrite in `draft/load/tests.rs`: `a_duplicate_tag_gives_e_rcp_014_with_the_tag_of_the_second_parameter`, `a_duplicate_key_gives_e_rcp_014_with_the_tag_of_the_second_parameter`, and `two_parameters_with_one_identifier_give_e_rcp_014_with_the_second_tag`. Each asserts `Stored` with the key of the second row.
  - Change the three state tests `a_discarded_draft_gives_e_rcp_002_with_status_draft`, `an_approved_version_gives_e_rcp_002_with_status_approved`, and `a_retired_version_gives_e_rcp_002_with_status_retired` to match `DraftError::Recipe`.
  - The callers in `recipe-app` convert through `From<DraftError> for AppError` (`recipe/draft_session.rs` line 149, `recipe/approve_version/prepare.rs` lines 181 to 189, `recipe/queries/approval_preview.rs` line 120).
  - Test first in `recipe-app`: `UpsertTranslation` with two seeded parameters with one tag gives `E-DB-008`. `ApproveVersion` with two seeded parameters with one tag gives `E-DB-008` and still reads the root again after the failed load. `GetApprovalPreview` with two seeded parameters with one tag gives `E-DB-008`. The memory store keys the parameters by identifier (`memory/core/state.rs` line 52), so it cannot hold two rows with one identifier. The rewritten domain test `two_parameters_with_one_identifier_give_e_rcp_014_with_the_second_tag` covers that case.

## 3. The rebind sites

- [ ] 3.1 (recipe-domain, recipe-app) The rebind plan stops at a stored defect (design D3, ADR-0195 (l), (n)). `map_value` and `map_limits` return the defect apart from the entries. `map_limits` first checks the stored rows against the old tree. `map_parameters` runs the I7 check of a kept value against its kept limits, and returns the first defect in tag order as `Stored` with the key.
  - Rewrite `stored_value_of_the_wrong_shape_gives_e_rcp_005` in `rebind/value/tests.rs`. Rewrite `rebind_refuses_a_date_above_the_earlier_end` in `rebind/earlier_tests.rs` (lines 119 to 127): the stored `DATE` count `72900` gives `Stored` with the inner code `E-RCP-005`.
  - Rewrite in `rebind/limit/tests.rs`: `kept_limit_that_the_planned_tree_rejects_gives_e_rcp_018` gives `Stored` with the key and no list. `dropped_limit_comes_before_the_error_of_a_rejected_kept_limit` gives `Stored` with the key and no list, in place of `[E-RCP-029, E-RCP-018]`.
  - Test first in `rebind/limit/tests.rs`: a stored limit on a path that the old struct does not have gives `Stored`. A valid limit of a removed member still gives the entry `E-RCP-029`.
  - Test first in `rebind/target/tests.rs`: parameter `A` lacks a sent value (`E-RCP-028`), and parameter `B` has a stored value of the wrong shape. `plan_rebind` gives `Stored` with the key of `B`. A second case gives `B` a stored value above its stored limit, with the same result. A list with only request entries stays a list.
  - Test first in `rebind/target/tests.rs`: one parameter lacks a sent value for a new member (`E-RCP-028`). Its stored value is above its kept stored limit. `plan_rebind` gives `Stored` with its key and no list. The I7 check uses the stored leaves at the kept paths (design D3).
  - Rewrite `stored_value_above_its_stored_limit_gives_e_rcp_004_and_no_change` in `rebind/apply/tests.rs`: `plan_rebind` now gives `Stored`, and the draft does not change.
  - Test first in `recipe-app` `recipe/rebind` tests: `RebindTypeRevision` with a seeded struct value that is no JSON object gives `E-DB-008` and commits nothing.
- [ ] 3.2 (recipe-domain) `rebound_parameter` wraps its I7 fault and the platform, width, and alignment errors of `check_definition` as `Stored` (`rebind/apply.rs` lines 203 to 212).
  - Test first in `rebind/apply/tests.rs`: `apply_rebind` with a moved parameter at a stored odd byte gives `Stored` with its key.
  - Test first in the same file: `apply_rebind` with a plan built by `RebindPlan::new` (`rebind/mod.rs` line 116) and a planned value above a kept limit gives `Stored`.
  - Rewrite `adr_0178_d_rebind_with_a_wrong_width_and_a_wide_bound_gives_e_rcp_054_first` in `rebind/apply/tests.rs` (lines 641 to 647): the stored `DB1.DBW0` gives `Stored` with the inner code `E-RCP-054`.
  - The tests of `E-RCP-056`, `E-RCP-057`, and `E-RCP-058` assert not `Stored` and stay green.
  - Guard in `rebind/target/tests.rs`: `plan_rebind` with a missing sent value and a moved parameter at a stored odd byte gives the list with `E-RCP-028`. It is not `Stored`, because the address check runs after a plan without entries (ADR-0195 (n)).

## 4. The sites of the application

- [ ] 4.1 (recipe-app) Add a fault hook to the memory store that makes the next `lock_machine_shared`, `lock_recipe`, `lock_draft`, or `live_units` return `Ok(None)`. Follow the existing hooks (`memory/core/store.rs` lines 320 to 350). Test first in the tests of `memory/core`. With the hook on `lock_recipe`, the next lock returns `None`, and the lock after it returns the row.
- [ ] 4.2 (recipe-app) The three `NotFound` arms of `OpenDraft::open` and the `NotFound` arm of `lock_live_unit` give `StorageError::invariant`. The arms are in `recipe/draft_session.rs` lines 105 to 111, 120 to 126, and 143 to 148, and `recipe/definition_steps.rs` lines 46 to 53.
  - Test first: one table test with the hook of task 4.1. `AddParameter` gives `E-DB-008` for the machine lock and for `live_units`. `SetParameterValue` gives `E-DB-008` for the recipe lock and the version lock. Each case commits nothing.
  - `SetParameterValue` opens with `MachineLock::None` and takes no machine lock (`recipe/set_value.rs` line 77, `recipe/draft_session.rs` lines 102 to 104).
- [ ] 4.3 (recipe-app) Add a fault hook to the memory store for the n-th next `type_rows` call. The hook adds one more row with a given identifier to its result. Test first in the tests of `memory/core`. With the hook on the first call, that call returns two rows with one identifier, and the next call does not.
- [ ] 4.4 (recipe-app) `OpenDraft::open` and `RebindTypeRevision` turn the error of `PlatformTypes::collect` into `E-DB-008` through a `StoredDefect` without a key (`recipe/draft_session.rs` line 169, `recipe/rebind.rs` line 135). Test first: with the hook of task 4.3 on the first call, `SetParameterValue` gives `E-DB-008`. With the hook on the second call, `RebindTypeRevision` gives `E-DB-008` and commits nothing.

## 5. The repair of a type that does not resolve

- [ ] 5.1 (recipe-app) The type changes, and the old type has no tree. Then `definition_diff_of` writes the whole stored value of the old side under the key `""` (design D6, ADR-0195 (q)).
  - Test first in `audit/diff/tests.rs`: an old type with a `TypeCycle` result gives `{"": <stored value>}` in `before_state.value` and the leaves of the new type in `after_state.value`. A stored `0` replaced by `INT` `0` still writes both sections.
  - In the same file: a new side without a tree, `value_diff_of`, and `rebind_diff_of` keep the error. `a_type_without_a_tree_gives_a_plant_error` stays green.
  - Test first in the `recipe/update_definition` tests: use the draft of `scenarios::zone_draft` on `TwinCat3`, so no parameter has an S7 address (`memory/fixtures/parameter.rs` lines 31 to 32). Seed `seed_struct_cycle` on `M1`.
  - A parameter of that cycle has the value `0` and no limits. `UpdateParameterDefinition` with the type `INT` and the value `0` commits. The audit entry holds the old value under the key `""` of `before_state.value`.
- [ ] 5.2 (recipe-app) `GetParameterForm` maps three faults to `E-DB-008` (design D7, ADR-0195 (z) to (ab)). They are the error of `PlatformTypes::collect`, a failed resolution of the stored type, and a missing type text. For the purpose `UpdateParameterDefinition`, a stored type that does not resolve returns the form without rule faults.
  - Test first in `recipe/queries/parameter_form_tests.rs`: one table test with the purpose `SetParameterValue`. A struct cycle, an oversized struct, and a struct whose member enters an array-only cycle give `E-DB-008`.
  - Today these three rows give `E-PLT-005`, `E-PLT-018`, and `422 E-PLT-013` with the reason `RowShape` (`plant/type_tree.rs` lines 473 to 485). The struct has a type text (`plant/queries/type_text.rs` lines 33 to 34).
  - In the same table, three guard rows give `E-DB-008` today through `text_of`. They are a root array-only cycle, a missing root row, and an array whose element row is missing. `type_text` gives no text for them (`type_text.rs` lines 21 to 24).
  - `seed_type` stores a row without a check (`memory/types/seed.rs` lines 143 to 151). So the test can seed an array row without its element row.
  - In the same table: two kept type rows with one identifier (the hook of task 4.3) give `E-DB-008`. A struct with a member type that the platform does not hold gives `422 E-PLT-013`.
  - Second table with the purpose `UpdateParameterDefinition`: each stored case except the duplicate rows returns the form. The root array-only cycle, the missing root row, and the array whose element row is missing give a form without a type text. The duplicate rows give `E-DB-008`.
  - The tables seed every type on `M1`, the machine of the drafts of `world` and `s7_world` (lines 33 and 40).
  - In the second table, use the stored struct cycle and the key of `ZONE`. The type `INT` of the request gives the form with the preview of `INT`. The type `ARRAY[1..16385] OF INT` gives `E-PLT-018`. This covers the first post of the repair (design D7).
  - `a_type_above_the_leaf_limit_gives_plt_018` (the preview of a type of the request) stays green.
- [ ] 5.3 (recipe-app, recipe-web) `GetParameter` and `ListParameters` return a view without a tree for a parameter whose type does not resolve (design D8, ADR-0195 (ad)). `ParameterView` gets an optional tree and an optional type text.
  - Test first in `recipe/queries/parameter_reads_tests.rs`: `GetParameter` on a parameter of `seed_struct_cycle` gives a view without a tree and without rule faults. A missing root row gives a view without a type text.
  - Test first in the same file: `ListParameters` on a draft with a parameter with a missing root row next to a healthy parameter. Assert both rows apart. The affected row has no type text and no value text. The healthy row does not change. Today `row_trees` gives `E-PLT-013` for it (`queries/parameters.rs` lines 210 to 218 and 369). After that change, `texts_of` still gives `E-DB-008` (lines 154 to 168 and 372).
  - Test first in `recipe-web` `pages/parameters/route_tests.rs`: `GET .../parameters` of a draft with a parameter of a stored struct cycle gives `200`. Its row shows the word of `parameters-value-composite` and no value text (design D8).
  - Test first in the same file: `ZONE` has a type id without a stored row. `GET .../parameters` gives `200`. The row of `ZONE` has an empty type text and an empty value cell, without the word of `parameters-value-composite`. Today `row_trees` makes the route answer `E-PLT-013`.
  - Change `ValueView::new` or `row_view` so that a row without a type text gets an empty value cell (`pages/versions/parameter_table.rs` lines 105 to 107 and 301 to 320).
  - Guard: `an_approved_version_with_an_unresolvable_type_gives_db_008` (`queries/version_tests.rs` lines 261 to 277) stays green. The table route runs `GetVersion` first (`pages/parameters/table.rs` lines 68 and 99).
  - The test of `recipe-web` needs a struct cycle. `seed_cycle` of `pages/types/test_kit.rs` (line 245) is `pub(super)`, and `pages/types.rs` line 126 keeps `test_kit` private. Move `seed_cycle` with the helpers that it calls to `test_support/plant/`. `pages/types/test_kit.rs` then uses the moved helper. The cycle belongs to `M7`.
  - The test also needs a parameter of that type on a version of `R-100` of `M7`. Add a helper next to `seed_int_named_parameter` (`test_support/recipes/earlier_name_world.rs` lines 59 to 90).
  - The helper takes a version id, a type id, and a stored value. It seeds `ZONE` at `DB6.DBB0` through `fixture.store.seed_parameter`. `ZONE` gets one translation in `pl`. `recipe_world` returns the same `World` (`pages/recipes/test_world.rs` line 32).
  - Change the comment of `check_rule_codes` to cite ADR-0119 (l) and ADR-0118 (d) in place of ADR-0119 (k) (`queries/versions.rs` lines 316 to 317).
  - Update the tests that read `view.tree` and `view.type_text`: `parameter_reads_tests.rs` lines 97, 98, 112, and 194. Line 195 stays.
  - Update the `recipe-web` call sites so the workspace builds. They are `form_view.rs` line 69 and lines 114 to 122, `value.rs` lines 93 to 95 and 162 to 168, and `summary.rs` line 63. They are also `page.rs` lines 79 to 86 and 96 to 134. The type field of the edit form is empty for a view without a type text. A call site that needs the tree answers `500` for a view without a tree.
- [ ] 5.4 (recipe-web) The parameter page shows a parameter whose type does not resolve without the value grid (design D8, ADR-0195 (ae)).
  - Test first in `pages/parameters/edit_tests.rs`: `ZONE` on the draft has a type of `seed_cycle`. It uses the two helpers of task 5.3. `GET .../{key}` for a ProcessEngineer gives `200` with the definition form and no value grid.
  - The same test sends the rendered edit form with the type `INT` and the S7 address `DB6.DBW0`. An `INT` leaf needs a word address (`recipe-domain` `plant/s7_width.rs` line 29). The answer shows the form with the grid of `INT` and writes nothing.
  - The test then sends that form with its `row_version` and the value `0`. The answer saves the parameter. A new `GET .../{key}` shows the value grid of `INT` and the old key. It also shows the translation in `pl` with its language and its text.
  - Test first: `GET .../{key}` for an Operator gives `200` with the read-only definition and no value grid.
  - Test first: `GET .../{key}` for a ProcessEngineer in read-only mode gives `200` with the read-only definition and no value grid. The page reuses `is_editable` (`pages/parameters/page.rs` lines 67 to 71). The test sets the mode with `fixture.state.read_only().set(true)`, as `csrf/tests.rs` line 604 does.
  - Test first: `ZONE` on the approved version 2 (`APPROVED_ID`, `test_support/plant.rs` line 61) has a type of `seed_cycle`. `GET .../versions/2/parameters/{key}` for a ProcessEngineer gives `200` with the read-only definition and no value grid.

## 6. Text and checks

- [ ] 6.1 (docs/adr) [mechanical] After the approval of `tasks.md`, set the Status of ADR-0195 to Accepted, and its row in `docs/adr/README.md`. Add the note "Amended by ADR-0195" with the amended items to the Status lines of ten ADRs. They are ADR-0005, ADR-0018, ADR-0027, ADR-0028, ADR-0039, ADR-0041, ADR-0043, ADR-0118, ADR-0120, and ADR-0121. Add the same note to the index rows of these ten ADRs in `docs/adr/README.md`. Remove ", proposed" from the six map rows of 0195. Check with `ste-lint.py` on the scope of ADR-0023 and ADR-0135.
- [ ] 6.2 (openspec) [mechanical] Run `openspec validate storage-defect-codes --strict --no-interactive` and `openspec validate --all --strict --no-interactive`. Both pass.
- [ ] 6.3 (crates) [mechanical] Run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features --locked`, `cargo fmt --check`, and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace`. All pass.

## Workflow follow-up

- Code review with `code-review`, then `superpowers:verification-before-completion`.
- Archive with `openspec-archive-change` after `cargo test` and `openspec validate` pass.
