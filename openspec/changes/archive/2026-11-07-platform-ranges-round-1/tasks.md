# Tasks

Each task is for one agent. The mark after the scope names the agent: `[sonnet]` (Sonnet, effort high), `[mechanical]` (Haiku), or `[owner]` (the project owner, no agent). The propose step already wrote ADR-0139 to ADR-0142 and the document edits.

The propose step changed the row `E-RCP-057` of `reliability.md` §1.2. Thus two tests fail now:

- `registry_matches_reliability` of `recipe-domain` fails with "E-RCP-057 has an unfilled field".
- `recipe_answer_each_english_message_of_design_d13_equals_its_registry_row` of `recipe-web` fails, because `crates/recipe-web/src/error/recipe_answer_tests.rs` line 107 compares `en.ftl` with the row.

Task 1.1 runs first and makes the workspace green again. After task 1.1, `cargo test --workspace` passes after each task. The orchestrator commits the documents of the propose step together with task 1.1. No push holds these documents without task 1.1.

Order and serial groups:

- No two agent tasks run at the same time. Each agent runs `cargo test --workspace` after its task, and a red step of a parallel agent breaks that run.
- The order is 1.1, 2.1, 2.2, 5.1, 3.1 to 3.5, 4.1 to 4.4, and 6.1.
- Tasks 3.1 to 3.5 and 4.1 to 4.4 mutate `range.rs` and `s7.rs`. Tasks 3.4 and 4.2 both change `read_check/tests.rs` and `export/rule_check/tests.rs`. Tasks 3.5 and 4.3 both change the new file of `recipe-app`.
- Task 2.1 changes no production code, but its red steps break the query test. Tasks 2.2 and 5.1 both change `docs/ops/runbook.md`.
- Task 2.3 is for the owner. It starts after task 2.2 and can run next to the agent tasks. The merge waits for task 2.3.

Each pinning test shows its red step on a mutation of `design.md` D7. Before the mutation, the agent copies the file to the scratch directory. The agent runs the new test on the mutation and sees it fail. Then it reverts the mutation and runs `cmp <file> <copy>`. The report shows both test runs and the empty output of `cmp`. A wrong expected value in the test is not a red step.

The review record `docs/plan/review/2026-11-06-plc-type-review.md` gives the findings #8, #13, #18 to #21. Each new test name starts with the prefix of the other tests of its file. A test file that would pass 800 lines gets a sibling test file. After each task, the agent runs the three commands of `openspec/config.yaml` section `apply`.

## 1. Message of E-RCP-057 (#8)

- [x] 1.1 (recipe-domain, recipe-web) [sonnet] #8, ADR-0140 (f), `design.md` D3. The two failing tests of the preamble are the red step. First, change these tests:
  - `crates/recipe-domain/src/recipe/error/registry_tests.rs`: add `max: 16_000` to the constructor at line 359 and `("max", "16000")` to its sample list at line 363.
  - `crates/recipe-domain/src/recipe/draft/s7/tests.rs` lines 466 to 471 and 506 to 511: assert the new text "uses a data block above 16000. The controller of the machine has no such data block."
  - `crates/recipe-web/src/error/recipe_tests.rs` line 367: add `max: 16_000`. Assert the new `en` and `pl` texts with 16000, and add one case with 59999.
  - `crates/recipe-web/src/pages/approval/blockers_tests.rs` line 190 and `crates/recipe-web/src/pages/parameters/reply/tests.rs` line 41: add `max: 16_000` to the constructors.

  See the tests fail. Then add `max: u16` to `S7DataBlockOutOfRange` in `crates/recipe-domain/src/recipe/error.rs` line 238. Change its message to the row of `reliability.md` §1.2. Change its doc comment at line 233, which names only the S7-300 controller. In `crates/recipe-domain/src/recipe/draft/s7.rs` line 163, fill `max` with 16 000.

  Give the variant its own arm in `crates/recipe-web/src/error/recipe.rs` line 200, with `max` as a string argument. Change line 479 of `crates/recipe-web/i18n/en.ftl` and `pl.ftl` to the texts of D3. Check: `cargo test -p recipe-domain` and `cargo test -p recipe-web` pass.

## 2. Query of the stored data

- [x] 2.1 (recipe-db) [sonnet] ADR-0139 risks, ADR-0140 risks, `design.md` D4. The task writes the SQL file `deploy/db/check-platform-ranges.sql` and its test. Write the test `crates/recipe-db/tests/storage/platform_ranges.rs` first, and add `mod platform_ranges;` to `main.rs`. The test reads the SQL file as `restore_code.rs` reads its query: a `const` path with `concat!(env!("CARGO_MANIFEST_DIR"), ...)` (lines 18 to 21) and `std::fs::read_to_string` (lines 46 to 50). It seeds these rows on the harness database, each parameter with its own tag:
  - value hits: an `LDT` value `9223372036854775808` on `TwinCat3` in a draft, and an `LDATE` value `9223372800000000000` (2262-04-12) on `CodesysIec` in an approved version
  - array hit: element 2 of an `ARRAY[1..2] OF LDT` with `10413792000000000000` in a retired version
  - nested struct hit: an `LDT` member of a struct inside a struct in a deleted draft
  - mixed value hit: a struct with a `STRING`, a `BOOL`, and an `LDT` member, with the `LDT` value `10413792000000000000`, in a recipe with `deleted_at`
  - soft-deleted hit: an `LDT` value `10413792000000000000` on a machine with `archived_at`
  - limit hits: an `LDT` limit `max` `10413792000000000000`, and an `LDT` limit `min` `9223372036854775808` with no `max`
  - address hits: an `INT` at `DB60000.DBW0` on `S7_1500`, and an `INT` at `DB65535.DBW0` on `S7_1200`
  - non-hits: an `LDT` value `9223372036854775807`, an `LDATE` value `9223286400000000000`, the mixed struct with the `LDT` value `9223372036854775807`, and an `LDT` limit `max` `9223372036854775807`
  - other non-hits: an `INT` at `DB59999.DBW0` on `S7_1200`, an `INT` at `DB60000.DBW0` on `S7_300`, and a `ULINT` value `18446744073709551615` with no `LDATE` or `LDT` in its type tree

  An error of the query fails the test, as `ON_ERROR_STOP=1` stops `psql`. It asserts the exact set of hit rows by machine code, reference number, version number, status, tag, and part, and the number of rows. It also asserts that the query writes nothing, and that it runs as `recipe_app`.

  Red step: a file that holds only a query with no rows. See the test fail. Then write the query of D4. A second red step: change `59999` to `59998` in the query, see the test fail, revert, and run `cmp`. Check: `cargo test -p recipe-db --test storage platform_ranges` passes with Docker.
- [x] 2.2 (docs/ops) [mechanical] ADR-0139 risks, ADR-0140 risks. Add the section "Check the stored data against the platform ranges" to `docs/ops/runbook.md`, before "## Upgrade and rollback", and add it to the Contents list. The section is a numbered procedure:
  - It gives the command of `design.md` D4, which runs in the directory `deploy/`.
  - It says that the owner reads each output row and signs it off or fixes it.
  - The fix of a draft row is `SetParameterValue`, `ReplaceLimits`, or `UpdateParameterDefinition`. The fix of an approved row is a new version.
  - It writes the result into the change log of the site (`#change-log-of-the-site`).

  Also add one sub-step to step 2 of the subsection "Upgrade", after the three parts. The sub-step runs the new section from the new directory `deploy/`, after the segment check and before step 3. The upgrade continues only when the result has 0 rows or the owner signed off each row. Do not change the numbers of the steps, because other steps cite them. Check: `python -I tools/ste-lint/ste-lint.py docs/ops/runbook.md` reports 0 hard violations.
- [ ] 2.3 (deploy) [owner] The pull request body holds the required box `- [ ] Platform range check: 0 rows, or the owner signed off each row`, a hard gate of the merge. Run the query of task 2.1 on the production database with the command of task 2.2. Record the number of rows and the identity of each row in the pull request: machine code, reference number, version number, status, tag, and part. The stored text stays in the change log of the site and does not go into the pull request. Check the box only when the result has 0 rows or the owner signed off each row. Check: the box is checked, and the pull request holds the number of rows and the identities.

## 3. Range of LDATE and LDT (#20)

- [x] 3.1 (recipe-domain) [sonnet] #20, ADR-0139 (a), (b), (e), `design.md` D1. In `crates/recipe-domain/src/plant/iec_literal/range/tests.rs`, first change `ldate_range_per_platform` (line 244) and `ldt_range_per_platform` (line 306) to the end `9_223_372_036_854_775_807` for `Current` and `18_446_744_073_709_551_615` for `Earlier`. Rename the test at line 188 as in D1 and add `LDATE` and `LDT` on `CodesysIec` and `TwinCat3`. Add literal tests on both platforms:
  - `LDT#2262-04-11-23:47:16.854775807` and `LD#2262-04-11` pass `IecLiteral::parse`, and their counts pass `IecLiteral::from_count` in both sets
  - `LDT#2262-04-11-23:47:16.854775808` and `LD#2262-04-12` give `OutOfRange` from `IecLiteral::parse`, which checks with `Current` (`mod.rs` line 120)
  - `IecLiteral::from_count` with `RuleSet::Earlier` passes for the counts `9223372036854775808` (`LDT`) and `9223372800000000000` (`LDATE`), and fails with `RuleSet::Current`

  See the tests fail. Then change `range.rs` lines 30 and 40 and add the arms of D1. Change the doc comment of `range_under` in `crates/recipe-domain/src/plant/iec_literal/mod.rs` line 64, which says that only `DATE` differs. Red step for the `Earlier` pins: M3. See them fail, revert, and run `cmp`. Check: `cargo test -p recipe-domain range` passes.
- [x] 3.2 (recipe-domain) [sonnet] #20, ADR-0139 (d), (f). Write the sibling test file `crates/recipe-domain/src/recipe/draft/earlier_ldt_tests.rs`, declared next to `earlier_tests.rs`. Model it on `earlier_tests.rs`. It pins the scenarios of the requirement "LDATE and LDT range on CodesysIec and TwinCat3":
  - `SetParameterValue` stores `9223372036854775807` and `9223286400000000000`
  - `SetParameterValue` gives `E-RCP-005` for `LDT#2262-04-11-23:47:16.854775808` and `LD#2262-04-12`, and the draft does not change
  - `ReplaceLimits` gives `E-RCP-006` for the `max` `LDT#2262-04-11-23:47:16.854775808`
  - `SetParameterValue` with `LDT#2026-10-12-12:30:00` stores `1791808200000000000` next to the stored limit `max` `10413792000000000000`

  Red step: M1 for the refusals and M3 for the stored limit. See each test fail, revert, and run `cmp`. Check: `cargo test -p recipe-domain earlier_ldt` passes.
- [x] 3.3 (recipe-domain) [sonnet] #20, ADR-0139 (f). In `crates/recipe-domain/src/recipe/snapshot/tests.rs`, model two tests on `adr_0120_the_approval_gives_e_rcp_005_for_the_date_value_72700` (line 412). On `TwinCat3`, the approval check of a stored `LDT` value `10413792000000000000` gives `E-RCP-020` with the item `E-RCP-005`. The approval check of an `LDT` limit `max` with that count gives the item `E-RCP-006`. The snapshot check with `RuleSet::Earlier` passes for both parts.

  Red step: M1 for the approval and M3 for the earlier check. See each test fail, revert, and run `cmp`. Check: `cargo test -p recipe-domain snapshot` passes.
- [x] 3.4 (recipe-domain) [sonnet] #20, ADR-0139 (g), scenario "Approved version with an LDT after the current range on TwinCat3". In `crates/recipe-domain/src/recipe/read_check/tests.rs`, pin that a stored `LDT` value `10413792000000000000` on `TwinCat3` gives a rule fault and no structure fault. Pin that `18446744073709551616` gives a structure fault. In `crates/recipe-domain/src/export/rule_check/tests.rs`, pin that the first value gives `loadable` `false`.

  Red step: M1 and M3 for the rule fault, M1 for `loadable`, and M4 for the structure fault. If the count fails before the range check, the report names that check and its mutation. See each test fail, revert, and run `cmp`. Check: `cargo test -p recipe-domain read_check` and `cargo test -p recipe-domain rule_check` pass.
- [x] 3.5 (recipe-app) [sonnet] #20, ADR-0139 (f), scenario "New draft from a base with an LDT after the current range". `create_version/tests.rs` has 773 lines. Thus write the sibling file `crates/recipe-app/src/recipe/create_version/earlier_range_tests.rs`, modeled on `an_approved_base_with_the_earlier_rule_set_parts_is_copied_unchanged` (line 699). `CreateVersion` copies a `TwinCat3` base with the stored `LDT` value `10413792000000000000` unchanged.

  Red step: M3. See the test fail, revert, and run `cmp`. Check: `cargo test -p recipe-app create_version` passes.

## 4. Data block number on S7_1200 and S7_1500 (#8)

- [x] 4.1 (recipe-domain) [sonnet] #8, ADR-0140 (a), (b), `design.md` D2. In `crates/recipe-domain/src/recipe/draft/s7/tests.rs`, next to `the_data_block_16001_passes_on_s7_1200_and_s7_1500` (line 475), first write these tests of `check_definition`:
  - `DB59999.DBW0` passes on `S7_1200` and `S7_1500`
  - `DB60000.DBW0` and `DB65535.DBW0` give `E-RCP-057` with `max` 59 999 on both platforms
  - `DB16001.DBW0` still passes on both platforms

  See the tests fail. Then change `check_data_block_rule` in `s7.rs` line 154 as in D2, with a named `u16` constant for 59 999. Change its doc comment, which still says "The other platforms keep `1..=65535`". The `DB59999.DBW0` pin passes before the change, so its red step is M5. See it fail, revert, and run `cmp`. Check: `cargo test -p recipe-domain s7` passes.
- [x] 4.2 (recipe-domain) [sonnet] #8, ADR-0140 (c), (e). Pin the paths of the rule on `S7_1500`. Each pin except the rebind uses an `INT` `SPEED` at `DB60000.DBW0`:
  - approval: `E-RCP-057` in `crates/recipe-domain/src/recipe/draft/approval/tests.rs`
  - fix path: in `approval/tests.rs`, `Draft::set_definition` (`draft/definition.rs` line 61, the step of `UpdateParameterDefinition`) moves `SPEED` to `DB59999.DBW0`, and then the approval check gives a token
  - rebind: in `crates/recipe-domain/src/recipe/rebind/apply/tests.rs`, next to the test at line 428, `rebind_s7_zone(PlcPlatform::S7_1500, "DB60000.DBB0")` gives `E-RCP-057` and keeps `ZONE_2`, and `rebind_s7_zone(PlcPlatform::S7_1500, "DB59999.DBB0")` applies
  - Skip path: `AddParameter` of `FEED` at `DB2.DBW0` passes next to the stored `SPEED`, in `s7/tests.rs`
  - read: a rule fault and no structure fault, in `read_check/tests.rs`
  - export: `loadable` `false`, in `export/rule_check/tests.rs`

  Red step: M2 for the approval, rebind refusal, read, and export pins. The fix path and the applied rebind use M5. For the Skip path, the red step runs `AddressRules::Run` in `check_s7_layout` (`s7.rs` line 56). See each test fail, revert, and run `cmp`. Check: `cargo test -p recipe-domain` passes.
- [x] 4.3 (recipe-app) [sonnet] #8, ADR-0140 (e), scenario "New draft from a base with a data block above 59999". In `crates/recipe-app/src/recipe/create_version/earlier_range_tests.rs` of task 3.5, pin that `CreateVersion` copies an `S7_1500` base with `SPEED` at `DB60000.DBW0` unchanged. Red step: a mutation that runs `check_data_block_rule` on each parameter of the base. The report names the line. See the test fail, revert, and run `cmp`. Check: `cargo test -p recipe-app create_version` passes.
- [x] 4.4 (recipe-web) [sonnet] #8, ADR-0140 (f). Pin the web answer on `S7_1500`. `DB60000.DBW0` in the add form gives `422 E-RCP-057` with the `en` text "above 59999" under the field `s7_address`. `DB59999.DBW0` gives no line.

  Use `crates/recipe-web/src/pages/parameters/add_error_tests.rs`. Its address cases use the `S7_300` of `M7` (line 97), so add an `S7_1500` world or use the world of `parameter_table_address_tests.rs`. The report names the choice.

  Red step: M2. See the test fail, revert, and run `cmp`. Check: `cargo test -p recipe-web parameters` passes.

## 5. TwinCAT version of the long date types (#21)

- [x] 5.1 (docs/ops) [mechanical] #21, ADR-0142 (b), A46. Add the section "Check the TwinCAT version before you add a TwinCat3 machine" to `docs/ops/runbook.md`, and add it to the Contents list. The section is a numbered procedure. It tells the operator to read the TwinCAT version of the controller. It names TwinCAT 3.1.4026.0 as the lowest version for `LDATE`, `LTOD`, and `LDT`, and it cites A46 and ADR-0142.

  Run this task after task 2.2. Check: `python -I tools/ste-lint/ste-lint.py docs/ops/runbook.md` reports 0 hard violations.

## 6. Verification

- [x] 6.1 (recipe-domain) [sonnet] Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked` with Docker, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked`. Run `openspec validate --all --strict`. Run `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'` and the same script on `openspec/changes/platform-ranges-round-1/**/*.md`. Check: each command passes, ste-lint reports 0 hard violations, and the report shows the output of each command.

## Workflow follow-up

- Review the code with `code-review` (Opus) and the Codex second review. Fix every CRITICAL and HIGH finding.
- Run `superpowers:verification-before-completion`.
- Merge only when the box of task 2.3 is checked.
- Archive with `openspec-archive-change` after `cargo test` and `openspec validate` pass and after task 2.3.
