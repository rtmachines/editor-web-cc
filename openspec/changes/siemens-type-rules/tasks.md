# Tasks

Each task is for one agent. Sonnet implements each code task. Haiku does each `[mechanical]` task. A Sonnet agent runs the `ste-lint.py` step of each doc task.

The ADRs 0177 to 0189 have the status "Proposed". The owner chose the options on 2026-11-17. The owner did not yet accept the text of the ADRs and of this file. Task 1.0 sets the status "Accepted". The changes of `docs/adr/` and `docs/architecture/` are part of this change, so no other task edits them.

Each code task first writes a failing test and runs the RED command. Then it writes the change and runs the GREEN command. A test name starts with the ADR, for example `adr_0177_`, so the filter of each command finds only the tests of the task. Each code task ends with `cargo test --workspace --all-features --locked`. A test of another crate that pins an earlier rule changes in the same task.

Some kinds of task have no RED step. Each such task names its kind:

- A pin task (1.1, 1.2, 1.3) pins behavior that the code has now. Its test passes with no code change. A failure is a defect: the agent stops and reports it.
- An integration task (2.2, 3.7, 5.2, 9.2) tests the use cases of a rule that an earlier task added. Its tests pass after that task.
- The fixture task 9.1 writes a full dump of a database with the stored rows of the matrix of `design.md`. It uses the code before the rules.
- Task 1.0 changes docs only. Task 1.4 changes two comments only. `cargo doc` checks task 1.4.

Five tests fail at the start of this change, because `reliability.md` §1.2 already holds the row of `E-RCP-058`:

- `registry_matches_reliability` of `recipe-domain`. `DOC_ROW_COUNT` is 37, and the document has 38 rows.
- `error_registry_every_pair_of_row_and_variant_has_a_sample` of `recipe-server`. The row of `E-RCP-058` has no sample.
- `recipe_answer_the_recipe_codes_of_the_registry_are_the_recipe_keys_of_both_bundles` in `crates/recipe-web/src/error/recipe_answer_tests.rs`. No bundle has the key `error-E-RCP-058`.
- `recipe_answer_each_english_message_of_design_d13_equals_its_registry_row` in the same file. The `en` bundle has no message of `E-RCP-058`.
- `recipe_answer_each_code_of_design_d13_has_a_polish_message` in the same file. The `pl` bundle has no message of `E-RCP-058`.

Task 3.1 makes all five tests pass. It also maps the new variant in every crate, so every crate compiles after it. Task 3.1 runs before every other code task, so no other task starts with a failing test.

## Run order

The orchestrator starts a wave after the previous wave merges. Each chain of a wave runs in its own git worktree (`isolation: worktree`), from the same commit. The tasks of one chain run in series in that worktree. Two chains of one wave edit no common file and use no common test target. The orchestrator merges the chains in the listed order. After each merge, it runs `cargo test --workspace --all-features --locked`. A merge conflict stops the run, and the orchestrator reports it.

0. Task 1.0 runs when the owner accepts ADR-0177 to ADR-0189 with this file. No code task starts before it.
1. Wave 1 is task 3.1 alone. It adds the variant of `E-RCP-058` to `recipe-domain`, `recipe-web`, and `recipe-server` in one task, because `recipe-web` and the registry of `recipe-server` match on every variant.
2. Wave 2 has eight chains:
   - Chain A: 1.3, then 2.1. Both edit `crates/recipe-domain/src/plant/type_expr/tests.rs`.
   - Chain B: 3.2, 3.3, 3.4. They edit `draft/s7.rs`, `draft/mod.rs`, `rebind/apply.rs`, and `read_check/mod.rs` of `recipe-domain`, and their tests.
   - Chain C: 4.1, then 5.1. Both edit `crates/recipe-web/i18n/en.ftl` and `crates/recipe-web/i18n/pl.ftl`. Task 4.1 adds the variant `TypeLimit::S7Nesting`, and task 5.1 adds a variant of `Expected`. Each task maps its variant in `recipe-web` in the same task, because `limit_word` and `expected_key` match on every variant.
   - Chain D: 6.1. It adds `plant/siemens_names.rs` and one line of `plant/mod.rs`. Task 4.1 does not edit `plant/mod.rs`.
   - Chain E: 1.1, 1.2, 1.4. They edit three other files of `recipe-domain`, and they share the test target `recipe-domain --lib`.
   - Chain F: 3.6. It edits `pages/approval/blockers.rs` and `pages/parameters/reply.rs` of `recipe-web` and their tests. Chain C edits other files of `recipe-web`.
   - Chain G: 7.1. It edits `recipe-export` only.
   - Chain H: 9.1. Its worktree starts from the commit after wave 1, so it sees no rule of this change.
3. Wave 3 has three chains:
   - Chain I: 2.2, 3.5, 3.7, 4.2, 4.3, 6.2, 6.3. They share the test target `recipe-app --lib`, and several of them edit the same use cases.
   - Chain J: 5.2. It edits test files of `recipe-domain` only.
   - Chain K: 2.3, 3.8, 4.4, 5.3, 6.4. They share the test target `recipe-db --test storage`. Tasks 2.3, 3.8, and 4.4 also edit `deploy/db/check-s7-layout-rules.sql`.
4. Wave 4 has two chains. Chain L is task 9.2. Chain M is task 8.1, which builds no crate.
5. Wave 5 is task 10.1, the gate of the change.

## 1. Acceptance, pin tests, and comments (R02, R03, R05, ADR-0183)

- [x] 1.0 (docs/adr) [mechanical] When the owner accepts ADR-0177 to ADR-0189 with this file, make these edits:
  - Set the status of ADR-0177 to ADR-0189 to "Accepted". Keep the text "Amends ..." after the status.
  - In `docs/adr/README.md`, change "Proposed" to "Accepted" in the index rows of ADR-0177 to ADR-0189. Delete "proposed: " from the rows of the decision map.
  - Delete the mark " (proposed)" after ADR-0177 to ADR-0189 in each note "Amended by" of the older ADRs and in their index rows. The older ADRs are ADR-0017, ADR-0114, ADR-0115, ADR-0116, ADR-0119, ADR-0120, ADR-0137, ADR-0143, ADR-0147, ADR-0148, and ADR-0166.
  - Check: `grep -rnE 'ADR-01(7[7-9]|8[0-9]) \(proposed\)|proposed: 01(7[7-9]|8[0-9])' docs/adr` prints nothing. `ste-lint.py` on the changed files reports 0 hard violations.
- [ ] 1.1 (recipe-domain) Pin task for review item R02. In `crates/recipe-domain/src/plant/iec_literal/range/tests.rs`, extend `ldt_range_per_platform`: `I63_MAX + 1` on `S7_1500` gives `IecLiteralReason::OutOfRange` under `RuleSet::Current` and `RuleSet::Earlier`. Add the test `adr_0139_the_ldt_literal_after_the_last_s7_1500_count_is_out_of_range` for `LDT#2262-04-11-23:47:16.854775808` on `S7_1500`. Command: `cargo test -p recipe-domain --lib -- ldt_range_per_platform adr_0139_the_ldt_literal` (expect pass).
- [ ] 1.2 (recipe-domain) Pin task for review item R03. In `crates/recipe-domain/src/recipe/value/texts_tests.rs`, add `adr_0183_based_and_typed_literals_fail`. It sends `16#FF`, `2#1111_1111`, `8#377`, `BYTE#16#FF`, and `1_000` to `BYTE`, `WORD`, `DWORD`, and `LWORD` leaves and expects `Expected::Integer`. It sends `REAL#1.0` to a `REAL` leaf and expects `Expected::Decimal`. The doc comment names ADR-0183. Command: `cargo test -p recipe-domain --lib adr_0183_based_and_typed_literals_fail` (expect pass).
- [ ] 1.3 (recipe-domain) Pin task for ADR-0183 (a). In `crates/recipe-domain/src/plant/type_expr/tests.rs`, add `adr_0183_string_with_square_brackets_fails`: `STRING[10]` and `WSTRING[10]` give `E-PLT-013`, and `STRING(10)` parses. Command: `cargo test -p recipe-domain --lib adr_0183_string_with_square_brackets_fails` (expect pass).
- [ ] 1.4 (recipe-domain) [mechanical] Review item R05, comments only. In `crates/recipe-domain/src/plant/string_length.rs`, line 13 and line 16 give the unit of a `WSTRING` length as UTF-16 code units and cite ADR-0137 (i). Check: `RUSTDOCFLAGS="-D warnings" cargo doc -p recipe-domain --no-deps` passes, and `grep -n 'two bytes per character' crates/recipe-domain/src/plant/string_length.rs` prints nothing.

## 2. Array of an array on S7-300 (ADR-0177)

- [ ] 2.1 (recipe-domain) In `crates/recipe-domain/src/plant/type_expr/tests.rs`, add `adr_0177_array_of_an_array_fails_on_s7_300`. `ARRAY[1..2] OF ARRAY[1..3] OF BYTE` on `S7_300` gives `E-PLT-013` with `NotOnPlatform`, and `ARRAY[1..2,1..3] OF BYTE` passes. RED: `cargo test -p recipe-domain --lib adr_0177`. Then `TypeExpr::require_no_nested_array` refuses the type on the three S7 platforms. Change each test that expects `S7_300` to accept the type, also in other crates. The golden layout rows A09 and A10 stay (ADR-0177 (f)). GREEN: `cargo test -p recipe-domain --lib type_expr` and `cargo test -p recipe-domain --lib s7_layout`.
- [ ] 2.2 (recipe-app) Integration task for task 2.1. Add the use case tests `adr_0177_*` of the requirement "No array of an array on S7-300" in `specs/plant-model/spec.md`. They cover these cases: `AddParameter` gives `E-PLT-013`, a `DefineUdt` member gives `E-PLT-006`, a stored parameter stays approvable, and `CodesysIec` accepts the type. Change the doc comment of `ensure_members` in `crates/recipe-app/src/plant/define_udt.rs` to name the three S7 platforms and ADR-0177. Command: `cargo test -p recipe-app --lib adr_0177`.
- [ ] 2.3 (recipe-db) In `crates/recipe-db/tests/storage/s7_layout_rules.rs`, seed an array row of an array on `S7_300` and a parameter with it. Expect both in the part `nested_array`. RED: `cargo test -p recipe-db --test storage s7_layout_rules`. Then extend the part `nested_array` of `deploy/db/check-s7-layout-rules.sql` to `S7_300`, and update its header comment (ADR-0177 (g)). GREEN: the same command.

## 3. Standard-access rule and E-RCP-058 (ADR-0178, ADR-0187)

- [x] 3.1 (recipe-domain, recipe-app, recipe-web, recipe-server) The variant of `E-RCP-058` in every crate (ADR-0178 (b), ADR-0187). This task makes the five tests of the start of this file pass.
  - RED, domain: in `crates/recipe-domain/src/recipe/draft/s7/tests.rs`, next to the `RCP_057` assertion, add `adr_0178_error_code`. `ErrorCode::RCP_058` is `E-RCP-058`, and `RecipeError::S7TypeNotAddressable` returns it for each reason.
  - RED, registry: in `crates/recipe-domain/src/recipe/error/registry_tests.rs`, set `DOC_ROW_COUNT` to 38. Add a sample of the variant to `parameter_samples`.
  - RED, web: in `crates/recipe-web/src/error/recipe_tests.rs`, add `adr_0178_message_*` tests. They check the message of each reason in `en` and `pl`, and the status 422.
  - RED command: `cargo test -p recipe-domain --lib -- adr_0178_error_code registry_matches_reliability`. It fails.
  - Domain change: add the row `RCP_058` to `crates/recipe-domain/src/error_codes.rs`. Add the variant `S7TypeNotAddressable { tag, address, reason }` to `crates/recipe-domain/src/recipe/error.rs`. `reason` is the closed enum `S7AccessReason { ArrayBoundOutsideInt, BoolArrayWithThreeDimensions }`. The message follows the row of `reliability.md` §1.2.
  - Web change: map the variant in `crates/recipe-web/src/error/recipe.rs` and `crates/recipe-web/src/error/status.rs`. Add `error-E-RCP-058` and one key for each reason to `crates/recipe-web/i18n/en.ftl` and `crates/recipe-web/i18n/pl.ftl`. The `en` text of `error-E-RCP-058` equals the row of `reliability.md` §1.2 in the form of `fluent_text` of `recipe_answer_tests.rs`: `Parameter { $tag } at { $address } has a type that a data block with standard access cannot hold: { $reason }. Change the type.`
  - Server change: add the name of the variant to the `match` of `crates/recipe-server/tests/error_registry/recipe.rs`. Add one sample of the variant there.
  - GREEN: `cargo test -p recipe-domain --lib -- adr_0178_error_code registry_matches_reliability`, `cargo test -p recipe-app --test error_types`, `cargo test -p recipe-web --lib`, and `cargo test -p recipe-server --test error_registry`. The output of `cargo test -p recipe-web --lib` shows the three `recipe_answer_*` tests of the start of this file as `ok`.
- [ ] 3.2 (recipe-domain) In `crates/recipe-domain/src/recipe/draft/s7/tests.rs`, add `adr_0178_*` and `adr_0187_*` tests of `check_definition` and of `check_s7_layout_for_approval`. Cases: the bounds `32767` and `-32768` pass, and `32768` and `-32769` fail. A bound in a struct member and in an array element fails. A parameter without an address passes. A `BOOL` array with three dimensions fails, also as a struct member, and one with two dimensions passes. RED: `cargo test -p recipe-domain --lib -- adr_0178 adr_0187`. Then add `check_standard_access_rule` to `crates/recipe-domain/src/recipe/draft/s7.rs`. Call it after `check_end_rule` in `check_definition` and in `check_layout` with `AddressRules::Run`. Export it from `crates/recipe-domain/src/recipe/draft/mod.rs` for tasks 3.3 and 3.4. GREEN: `cargo test -p recipe-domain --lib draft`.
- [ ] 3.3 (recipe-domain) In `crates/recipe-domain/src/recipe/rebind/apply/tests.rs`, add `adr_0178_rebind_to_a_struct_with_a_wide_bound_fails`. A new revision with a member `ARRAY[40000..40001] OF BYTE` gives `E-RCP-058` for a parameter with an address. RED: `cargo test -p recipe-domain --lib adr_0178_rebind`. Then call `check_standard_access_rule` after `check_end_rule` in `crates/recipe-domain/src/recipe/rebind/apply.rs`. GREEN: `cargo test -p recipe-domain --lib rebind`.
- [ ] 3.4 (recipe-domain) In `crates/recipe-domain/src/recipe/read_check/tests.rs`, add `adr_0178_*` and `adr_0187_*` tests. An `Approved` version has a parameter with an address and a stored bound `32770`, or a stored `BOOL` array with three dimensions. Each case gives a rule fault, so `loadable` is false. RED: `cargo test -p recipe-domain --lib -- read_check::tests::adr_0178 read_check::tests::adr_0187`. Then call the rule in `address_faults` of `crates/recipe-domain/src/recipe/read_check/mod.rs`. GREEN: `cargo test -p recipe-domain --lib read_check`.
- [ ] 3.5 (recipe-app) In `crates/recipe-app/src/recipe/queries/approval_preview_parity_tests.rs`, add `adr_0178_the_blocker_keys_hold_the_key_of_an_e_rcp_058_fault`. Use the parameter `T1` of type `ARRAY[32760..32770] OF BYTE` at `DB1.DBB0` on `S7_300`. The preview and `ApproveVersion` give `ErrorCode::RCP_058`, and `blocker_keys` holds the key of `T1`. RED: `cargo test -p recipe-app --lib adr_0178_the_blocker_keys`. Then add `RecipeError::S7TypeNotAddressable { tag, .. }` to the arm of one tag in `crates/recipe-app/src/recipe/queries/approval_preview.rs` (ADR-0178 (k), ADR-0049 (c)). GREEN: `cargo test -p recipe-app --lib approval_preview`.
- [x] 3.6 (recipe-web) Add `adr_0178_blocker_line_of_e_rcp_058_links_its_parameter` to `crates/recipe-web/src/pages/approval/blockers_tests.rs`. Add `adr_0178_reply_e_rcp_058_stands_under_the_input_s7_address` to `crates/recipe-web/src/pages/parameters/reply/tests.rs`. RED: `cargo test -p recipe-web --lib -- adr_0178_blocker adr_0178_reply`. Then add the variant to `tag_of` in `crates/recipe-web/src/pages/approval/blockers.rs`. Add it to the arm `Some("s7_address")` of `field_of` in `crates/recipe-web/src/pages/parameters/reply.rs` (ADR-0178 (k)). GREEN: `cargo test -p recipe-web --lib`.
- [ ] 3.7 (recipe-app) Integration task for tasks 3.2 and 3.3. Add the use case tests `adr_0178_*` and `adr_0187_*` of two requirements in `specs/recipe-management/spec.md`. The requirements are "Array bounds of a parameter with an S7 address" and "BOOL array with three dimensions and an S7 address". Cover `AddParameter`, `UpdateParameterDefinition`, `RebindTypeRevision`, `ApproveVersion`, a stored parameter of another tag, and `CreateVersion`. Command: `cargo test -p recipe-app --lib -- adr_0178 adr_0187`.
- [ ] 3.8 (recipe-db) In `crates/recipe-db/tests/storage/s7_layout_rules.rs`, seed a parameter with an address and a bound `32770`, and one with a `BOOL` array with three dimensions. Expect them in the new parts `array_bound` and `bool_array_3d`. Seed the same types without an address as non-hits. RED: `cargo test -p recipe-db --test storage s7_layout_rules`. Then add both parts to `deploy/db/check-s7-layout-rules.sql` and its header comment. GREEN: the same command.

## 4. Nesting limit on S7 (ADR-0179)

- [ ] 4.1 (recipe-domain, recipe-web) The S7 count and its limit kind (ADR-0179 (a) to (c), (l)).
  - RED, domain: in `crates/recipe-domain/src/plant/type_tree/limits.rs`, add `adr_0179_*` tests. `MAX_S7_NESTING` equals 8 (`reliability.md` §7.11). The count follows ADR-0179 (b) and (c). A struct counts 1, and an array of a struct counts 3. An array of a primitive or a string counts 0.
  - More domain cases: a type with 8 levels passes on `S7_1500`. A type with 9 levels gives `E-PLT-018` with `TypeLimit::S7Nesting` on the three S7 platforms. `TwinCat3` accepts the type with 9 levels.
  - RED, limit text: in `crates/recipe-domain/src/plant/error/tests.rs`, `TypeLimit::S7Nesting.to_string()` is "8 levels of the S7 count". `TypeLimit::Depth.to_string()` stays "32 levels".
  - RED, web: in `crates/recipe-web/src/error/plant_args_tests.rs`, add `adr_0179_*` tests. The message of `E-PLT-018` with `TypeLimit::S7Nesting` names 8 in `en` and in `pl`. The message with `TypeLimit::Depth` still names 32.
  - RED commands: `cargo test -p recipe-domain --lib adr_0179` and `cargo test -p recipe-web --lib adr_0179`.
  - Domain change: add `MAX_S7_NESTING` to `crates/recipe-domain/src/plant/limits.rs`. Add the variant `TypeLimit::S7Nesting` and its `Display` to `crates/recipe-domain/src/plant/error.rs`. Add the count to the metrics of `type_tree.rs`, and one public method of `TypeTree` that takes the platform. The task does not edit `plant/mod.rs`.
  - Web change: map the variant in `limit_word` of `crates/recipe-web/src/error/plant.rs` to the key `type-limit-s7-nesting`. Add the key to `crates/recipe-web/i18n/en.ftl` and `crates/recipe-web/i18n/pl.ftl`.
  - GREEN: `cargo test -p recipe-domain --lib type_tree`, `cargo test -p recipe-domain --lib plant::error`, and `cargo test -p recipe-web --lib`.
- [ ] 4.2 (recipe-app) Add the use case tests `adr_0179_*` of the requirement "Nesting limit of a type on S7" in `specs/plant-model/spec.md`. Task 4.3 covers the scenario of the rebind. Cover `DefineUdt`, `AddParameter`, a changed type of `UpdateParameterDefinition`, and `TwinCat3`. Cover a struct `ST_Top@1` with nine levels on `S7_1500`, stored with `RuleSet::Earlier`, with these results:
  - `AddParameter` with the type `ST_Top@1` gives `E-PLT-018` with `TypeLimit::S7Nesting`.
  - A stored parameter `TOP` of type `ST_Top@1` does not stop `SetParameterValue` on another parameter of the draft.
  - `ApproveVersion` approves that draft (ADR-0179 (i)).
  - `ExportApprovedVersion` of that version gives `loadable` true.
  - RED: `cargo test -p recipe-app --lib adr_0179`. Then call the check of task 4.1 in `crates/recipe-app/src/plant/define_udt.rs`, `crates/recipe-app/src/recipe/add_parameter.rs`, and `crates/recipe-app/src/recipe/update_definition.rs`, next to `require_no_nested_array`. GREEN: `cargo test -p recipe-app --lib`.
- [ ] 4.3 (recipe-app) The rebind case of ADR-0179 (f). In `crates/recipe-app/src/recipe/rebind/tests.rs`, add `adr_0179_rebind_*` tests with these cases:
  - On `S7_1500`, `ST_Mid@1` has six levels, and `ST_Mid@2` has eight levels in the S7 count.
  - A parameter `BUF` of type `ARRAY[1..2] OF ST_Mid@1` moves to `ARRAY[1..2] OF ST_Mid@2`, with ten levels. The command gives `E-PLT-018` with that type text and `TypeLimit::S7Nesting`. The draft does not change.
  - In another draft, a parameter of type `ST_Mid@1` moves to `ST_Mid@2`, with eight levels. The command passes.
  - On `TwinCat3`, the move of `BUF` passes.
  - A parameter of another struct, stored over the limit, keeps its type, because the command does not move it.
  - RED: `cargo test -p recipe-app --lib adr_0179_rebind`. Then, in `crates/recipe-app/src/recipe/rebind.rs`, run the check of task 4.1 on the resulting type tree of each moved parameter before `apply_rebind`. Name `E-PLT-018` for the S7 count in the doc comment of `execute`. GREEN: `cargo test -p recipe-app --lib rebind`.
- [ ] 4.4 (recipe-db) In `crates/recipe-db/tests/storage/s7_layout_rules.rs`, seed a struct row and a parameter type with 9 levels on `S7_1500`. Seed a type with 8 levels as a non-hit. Expect the part `nesting`. RED: `cargo test -p recipe-db --test storage s7_layout_rules`. Then add the part `nesting` with the count of ADR-0179 (b) to `deploy/db/check-s7-layout-rules.sql`. GREEN: the same command.

## 5. WCHAR and WSTRING characters on S7 (ADR-0180)

- [ ] 5.1 (recipe-domain, recipe-web) The value rule and its expected form (ADR-0180 (a) to (d)).
  - RED, domain: in `crates/recipe-domain/src/recipe/value/leaf/tests.rs`, add `adr_0180_*` tests. On `S7_1500` and `S7_1200` with `RuleSet::Current`, these results apply:
    - `WCHAR` `U+D7FF` passes, and `U+E000` gives `E-RCP-005` with `Expected::Char { max: 0xD7FF }`.
    - `WSTRING` `U+CFFF` passes. `U+D000` and `U+1F600` give `E-RCP-005` with the new `WSTRING` form.
  - More domain cases: under `RuleSet::Earlier`, all these values pass. On `CodesysIec` and `TwinCat3`, the `WSTRING` values pass. `WCHAR` has a seed row only on `S7_1200` and `S7_1500` (`migrations/0004_seed_plc_types.sql`), so no test sends a `WCHAR` to another platform.
  - RED, text: add a test of the `Display` text of the new form to `expected_tests.rs`.
  - RED, web: in `crates/recipe-web/src/error/recipe_tests.rs`, add `adr_0180_*` tests of the requirement "Expected form of a wide text value on S7" in `specs/web-ui/spec.md`, in `en` and `pl`.
  - RED commands: `cargo test -p recipe-domain --lib adr_0180` and `cargo test -p recipe-web --lib adr_0180`.
  - Domain change: change `crates/recipe-domain/src/recipe/value/leaf.rs`, and add the variant to `expected.rs`.
  - Web change: map the variant in `expected_key` of `crates/recipe-web/src/error/recipe.rs`. Add its key to `crates/recipe-web/i18n/en.ftl` and `crates/recipe-web/i18n/pl.ftl`.
  - GREEN: `cargo test -p recipe-domain --lib value` and `cargo test -p recipe-web --lib`.
- [ ] 5.2 (recipe-domain) Integration task for task 5.1. Add `adr_0180_*` tests for a stored `WCHAR` `U+E000` on `S7_1500` and a stored `WSTRING(10)` value `"A\u{D000}"` on `S7_1200`. For each value, these results apply:
  - A draft load passes.
  - `ApproveVersion` gives an item of `E-RCP-020`.
  - `CreateVersion` copies the value.
  - The export check makes `loadable` false.
  - Put them in `crates/recipe-domain/src/recipe/draft/earlier_tests.rs` and `crates/recipe-domain/src/recipe/read_check/tests.rs`.
  - In `crates/recipe-domain/src/recipe/rebind/apply/tests.rs`, add a rebind case (ADR-0180 (e)). A new member `mark` of type `WCHAR` with the text `"\u{E000}"` gives `E-RCP-005` on `S7_1500`.
  - Command: `cargo test -p recipe-domain --lib adr_0180`.
- [ ] 5.3 (recipe-db) In `crates/recipe-db/tests/storage/platform_ranges.rs`, seed a `WCHAR` value `U+E000` and a `WSTRING` value with `U+D000` on `S7_1500`. Seed a `WSTRING` value with `U+D000` on `CodesysIec` as a non-hit. `WCHAR` has no seed row on `CodesysIec`. Expect the part `wide_text`. RED: `cargo test -p recipe-db --test storage platform_ranges`. Then add the part to `deploy/db/check-platform-ranges.sql` and its header comment. GREEN: the same command.

## 6. Siemens reserved struct names on S7 (ADR-0186)

- [ ] 6.1 (recipe-domain) Add the module `crates/recipe-domain/src/plant/siemens_names.rs` with tests `adr_0186_*`. These cases apply:
  - The list has 269 names: the 253 names of ADR-0186 (d1) and the 16 words of (d2). No name occurs twice in any case.
  - No name is in `RESERVED_KEYWORDS` or is a primitive name. No name of the list matches the family of (d3).
  - The family `PD_TEL<n>` of (d3) is one pattern: `PD_TEL` and one or more ASCII digits, in any case. A leading zero is a digit of the pattern (ADR-0186 (d3)).
  - `is_siemens_reserved_name` compares without case. The real telegram name `PD_TEL3` and `pd_tel3` are reserved. `PD_TELx`, `PD_TEL`, and `PD_TEL3_IN` are not reserved.
  - Test `adr_0186_the_telegram_family_holds_leading_zeros`: `PD_TEL0`, `PD_TEL03`, and `PD_TEL003` are reserved, as the superset of (d3) says.
  - A check that takes the platform refuses a struct name on the three S7 platforms with `E-PLT-013` and `ReservedName`. It accepts the name on `CodesysIec` and `TwinCat3`.
  - RED: `cargo test -p recipe-domain --lib adr_0186`. Then write the module and export it from `plant/mod.rs`. GREEN: the same command.
- [ ] 6.2 (recipe-app) Add the use case tests `adr_0186_*` of the requirement "Siemens reserved struct names on S7" in `specs/plant-model/spec.md`. Cover these cases:
  - A new struct `PID_Compact` and a new struct `PD_TEL3`, a new revision, a member name, `AddParameter` with a stored struct, and `CodesysIec`.
  - `RebindTypeRevision` from a stored `PID_Compact@1` to a stored `PID_Compact@2` passes. Test a parameter of type `PID_Compact@1` and one of type `ARRAY[1..2] OF PID_Compact@1` (ADR-0186 (i)).
  - RED: `cargo test -p recipe-app --lib adr_0186`. Then call the check in `crates/recipe-app/src/plant/define_udt.rs` for the struct name. Give `require_current_type` in `crates/recipe-app/src/plant/current_type.rs` the platform, so it also refuses a struct reference with a Siemens name on S7. The rebind keeps a stored name.
  - The call of `require_current_type` moves to the first place where the use case knows the platform. The check is pure and waits for no I/O (`reliability.md` §7.4).
  - `DefineUdt` (`define_udt.rs`): the loop over the members moves from before `live_machine_id` to after `require_live(locked, input.code)`. It uses `machine.platform()` and runs before `lock_derived_types`.
  - `AddParameter` (`crates/recipe-app/src/recipe/add_parameter.rs`): the call moves from before `OpenDraft::open` to after it. It uses `open.draft.platform()` and runs before `require_no_nested_array`.
  - `UpdateParameterDefinition` (`crates/recipe-app/src/recipe/update_definition.rs`): the call stays in `stage_definition` and uses `open.draft.platform()`.
  - The move changes the order of two faults in one request. A missing machine (`E-WEB-005`) or a stale `row_version` (`E-WEB-001`) now comes before the type fault. A test that pins the earlier order changes in this task.
  - GREEN: `cargo test -p recipe-app --lib`.
- [ ] 6.3 (recipe-app) Add `adr_0186_*` tests of the type proposals of the struct form and of the parameter form. On an S7 machine, the proposals leave out a stored struct with a Siemens name. On `CodesysIec`, they keep it. RED: `cargo test -p recipe-app --lib adr_0186_proposals`. Then filter in `crates/recipe-app/src/plant/queries/types.rs` and `crates/recipe-app/src/recipe/queries/parameter_form.rs`. GREEN: `cargo test -p recipe-app --lib queries`.
- [ ] 6.4 (recipe-db) In `crates/recipe-db/tests/storage/identifier_rules.rs`, seed a struct `PID_Compact`, a struct `PD_TEL3`, and a struct `pd_tel03` on `S7_1500` and on `CodesysIec`. Expect only the S7 rows in the part `s7_reserved_struct_name`. Seed `PD_TEL3_IN` on `S7_1500` as a non-hit. Add a test that compares the list and the pattern of the SQL file with those of `recipe-domain`. RED: `cargo test -p recipe-db --test storage identifier_rules`. Then add the part to `deploy/db/check-identifier-rules.sql`. GREEN: the same command.

## 7. Pattern of the S7 address in the export schema v2 (ADR-0189)

- [ ] 7.1 (recipe-export) Add the module `addresses` to `crates/recipe-export/tests/schema.rs` with `adr_0189_*` tests. The forms `DB1.DBX0.7`, `DB2.DBB10`, `DB65535.DBW65534`, `DB12.DBD4`, and null pass the v2 file. The seven forms of ADR-0189 (d) fail with the keyword `pattern`. Each form gives the same answer from `S7Address::parse`. `DB1.DBX0.8` still passes the v1 file. RED: `cargo test -p recipe-export --test schema adr_0189`. Then add the pattern of `design.md`, section Decisions, to `crates/recipe-export/schema/export-v2.schema.json`. GREEN: `cargo test -p recipe-export`.

## 8. Runbook

- [ ] 8.1 (docs/ops) In `docs/ops/runbook.md`, near the sections of `check-identifier-rules.sql` and `check-s7-layout-rules.sql`, name the new parts `nested_array` on `S7_300`, `array_bound`, `bool_array_3d`, `nesting`, `wide_text`, and `s7_reserved_struct_name`. Say which ADR each part follows and what a listed row means for an upgrade. Use the two kinds of step 2 of the section Migration Plan in `design.md`. Check: `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/ops/**/*.md'` reports 0 hard violations.

## 9. Stored data matrix (design.md, section Stored data matrix)

- [ ] 9.1 (recipe-db) Fixture task. It runs in chain H of wave 2, on the code before the rules of this change.
  - Test support comes first. Task 9.1 and task 9.2 use it.
    - In `crates/recipe-db/tests/storage/support/database.rs`, add `TestDatabase::create_empty`. It creates an empty database from `template0`, owned by `recipe_migrator`.
    - `TestDatabase::create_empty` also runs the three `REVOKE` and `GRANT` statements of `TestDatabase::create` for the new database.
    - In `crates/recipe-db/tests/storage/support/app.rs`, add `App::database_name`. It returns the name of the private field `_database`. Task 9.1 gives this name to `pg_dump`.
    - In the same file, add `App::over(database: TestDatabase)`. It builds the pool as `recipe_app` and the context over an existing database, as `create_with` does.
    - `App::over` reads the `id` of the administrator of the fixture from `app_user`. It inserts no user.
    - In `support/database.rs`, add `DumpDirectory`. It is a new directory for one test under `std::env::temp_dir()`.
    - Clean-up: `App::over` keeps its `TestDatabase`, so the `Drop` of `TestDatabase` deletes the database. The `Drop` of `DumpDirectory` deletes the directory and its dump files.
  - Add `crates/recipe-db/tests/storage/stored_rules.rs` and its line `mod stored_rules;` in `main.rs`. Add the ignored test `stored_rules_write_the_fixture`.
  - The test builds the seven cases of the matrix through the use cases on PostgreSQL (`crate::support::app::App`). Each case is one recipe with one version that `ApproveVersion` approves.
  - Then the test writes a full dump of that database: the schema, the data of every table, and `_sqlx_migrations`. It runs the command as `restore_code.rs` does: `docker exec --user postgres <container_id> pg_dump --format=plain --dbname=<database>`. The command has no `--data-only` and no `--exclude-table-data`.
  - The test asserts the exit status 0. It writes the standard output to `crates/recipe-db/tests/storage/fixtures/stored_rules.sql`.
  - A plain dump creates each table with its `CHECK` and `NOT NULL` constraints before it loads the data. It creates the foreign keys and the triggers after the data. Thus no trigger runs on a row of the dump.
  - It writes `crates/recipe-db/tests/storage/fixtures/stored_rules.hashes` with one line `<reference> <content_sha256>` for each case. It reads the hash from `recipe_version`.
  - Command: `cargo test -p recipe-db --test storage stored_rules_write_the_fixture -- --ignored`. Run it once, because a second run gives other UUIDs. The two files go into the commit of the task.
- [ ] 9.2 (recipe-db) Integration task for groups 2 to 6. In `crates/recipe-db/tests/storage/stored_rules.rs`, add one test for each row of the matrix, for example `adr_0180_stored_wstring_exports_with_its_hash`. Each test does these steps:
  1. Restore. `TestDatabase::create_empty` creates the empty database `<name>` from `template0`.
  2. Load. Run `docker exec -i --user postgres <container_id> psql --set ON_ERROR_STOP=1 --single-transaction --quiet --dbname=<name>` with `fixtures/stored_rules.sql` on the standard input. Assert the exit status 0.
  3. Upgrade. Call `recipe_db::migrate(&server().target(Role::Migrator, <name>), &MigrateSettings::default())` and assert `Ok`. The migrator of this release runs on the data of the earlier release, so this step also tests the upgrade path. Then `App::over` builds the `App` over `<name>`.
  4. Read. `GetVersion` reads the version through the adapter. For a type row, `GetTypeTree` also reads the type. The stored type text and the stored value do not change.
  5. Export. `ExportApprovedVersion` gives the `content_sha256` of `fixtures/stored_rules.hashes` and the `loadable` value of the matrix.
  6. Copy. `CreateVersion` from the approved version creates a draft with the same rows.
  7. Backup. Run `docker exec --user postgres <container_id> pg_dump --format=custom --dbname=<name>`. Write the standard output to `<name>.dump` in the `DumpDirectory` of the test. Make a second empty database `<copy>` as in step 1.
  8. Restore the backup. Run `docker exec -i --user postgres <container_id> pg_restore --exit-on-error --single-transaction --dbname=<copy>` with `<name>.dump` on the standard input. Assert the exit status 0. Step 5 with `App::over` on `<copy>` gives the same document.
  - Clean-up: the `Drop` of each `TestDatabase` deletes `<name>` and `<copy>`. The `Drop` of `DumpDirectory` deletes `<name>.dump`. No file stays at a fixed path in `/tmp`.
  - Command: `cargo test -p recipe-db --test storage stored_rules`.
  - Evidence: the task report shows the line of this command that starts with `test result: ok. 7 passed; 0 failed; 1 ignored;`. The seven tests are the seven rows of the matrix. The ignored test is `stored_rules_write_the_fixture`.

## 10. Gate

- [ ] 10.1 (workspace) Run each command and show its output:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-features --locked`
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
  - `openspec validate siemens-type-rules --strict`
  - `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md openspec/changes/siemens-type-rules/**/*.md'`

## Workflow follow-up

- Run `code-review` after group 10. Fix each CRITICAL, HIGH, and MEDIUM finding.
- Commit the review report and `docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md` in the PR of this change.
- Archive the change with `openspec-archive-change` after `cargo test` and `openspec validate` pass.
