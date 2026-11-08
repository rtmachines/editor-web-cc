# Tasks

Marks after the scope:

- `[sonnet]`: a Sonnet subagent with the effort `high` does the task.
- `[mechanical]`: a Haiku subagent does the task.
- `[owner]`: the owner does the task. No agent does it.

Each task is one agent. Each task of a crate starts with a failing test and shows the red step before the fix.

A pinning test passes before the change. Its red step uses one mutation:

1. Copy the file to the scratch directory.
2. Apply the mutation and run the test. The test fails on the mutation.
3. Revert the mutation and run `cmp <file> <copy>`. `cmp` gives no output.

The report of each task gives the test name, the red step, and the failure line.

The tree passes `cargo test --workspace --all-features --locked` before task 1.1 and after each task. Each task runs the full tests of its crate. The orchestrator runs the workspace command after each wave.

Serial groups. The tasks of one serial group run one after the other in their order. Two tasks of two serial groups can run in parallel after their dependencies end.

| Serial group | Tasks | Shared files |
|---|---|---|
| S1 | 1.1, 1.2, 1.3, 1.4, 1.5 | `crates/recipe-domain/src/plant/s7_layout.rs`, `s7_layout/reference.rs`, `s7_layout/prop.rs`, `s7_layout/tests.rs`, `crates/recipe-domain/src/recipe/draft/s7.rs`, `draft/s7/layout_tests.rs`, `draft/replace.rs`, `crates/recipe-domain/src/export/rule_check.rs` |
| S2 | 3.1, 3.2, 3.3, 3.4, 4.2 | The test fixtures of `crates/recipe-app`, for example a machine on `S7_1500` in the memory adapter |
| S3 | 5.1, 5.2 | `deploy/db/check-s7-layout-rules.sql`, `docs/ops/runbook.md` |
| S4 | 4.1, 4.3 | The test fixtures of `crates/recipe-web` |

The agents share one working tree. The order of the waves therefore is:

1. Group S1 runs first. Task 2.1 runs after task 1.5. No other task runs during this wave, because each task of the wave changes `recipe-domain`.
2. Groups S2, S3, and S4 start after task 2.1. They can run in parallel.
3. Task 5.3 needs tasks 5.1 and 5.2. Task 6.1 needs every other task except 5.3.

## 1. S7 layout

- [x] 1.1 (recipe-domain) [sonnet] Start a `SINT` or `USINT` struct member on an even byte (ADR-0147 (a) to (c), design D1).
  - Test file: `crates/recipe-domain/src/plant/s7_layout/tests.rs`.
  - Test: the struct `BYTE`, `CHAR`, `SINT`, `USINT` has the size 6. `s7_members` gives the offsets 0.0, 1.0, 2.0, and 4.0.
  - Test: the struct `BOOL`, `SINT` has the size 4 with `.b` at 2.0. The struct `SINT`, `BYTE` has the size 2 with `.b` at 1.0.
  - Test: `ARRAY[1..3] OF SINT` keeps the size 4. A leaf of `SINT` keeps the start class `Byte`.
  - Test file: the new module `crates/recipe-domain/src/recipe/draft/s7/layout_tests.rs`, declared in `draft/s7.rs`. `draft/s7/tests.rs` has 686 lines, so it gets no new test.
  - Test: the scenario "SINT and USINT members start on an even byte" of the delta spec `recipe-management`.
  - Test: the scenario "Parameter after a struct with SINT and USINT members" of the same delta spec.
  - Test: a `SINT` parameter at `DB1.DBB1` keeps its place (ADR-0147 (c)).
  - Change: add the free function of D1 to `s7_layout.rs`. Call it in `Frame::finish` for a struct and in `members_of_root`.
  - Change: in `reference.rs`, set row C04 to the size 6 and `.d` to 4.0. In `prop.rs`, give `first_fit` and `reference_width` the rule for P06 and P07 as members.
  - Check: `cargo test -p recipe-domain`.
- [x] 1.2 (recipe-domain) [sonnet] Start each line of a `BOOL` array at the next byte (ADR-0147 (d) to (g), design D2).
  - Test file: `crates/recipe-domain/src/plant/s7_layout/tests.rs`.
  - Test: `ARRAY[1..3,1..3] OF BOOL` has the size 4. `ARRAY[1..2,1..9] OF BOOL` has the size 4. `ARRAY[1..3,1..3,1..3] OF BOOL` has the size 10.
  - Test: `ARRAY[1..17] OF BOOL` keeps the size 4. `ARRAY[0..1,1..2] OF INT` keeps 8. `ARRAY[1..2,1..3] OF BYTE` has 6.
  - Test file: `crates/recipe-domain/src/recipe/draft/s7/layout_tests.rs`.
  - Test: the scenario "Lines of a BOOL array with two dimensions" of the delta spec `recipe-management`.
  - Test: the scenario "Parameter after a BOOL array with two dimensions" of the same delta spec.
  - Change: `array_layout` takes the `ArrayDimensions` and applies the steps of D2 with `checked_*` and `try_from`.
  - Change: in `reference.rs`, set row A02 to the size 4 and row A03 to the size 10.
  - Check: `cargo test -p recipe-domain`.
- [x] 1.3 (recipe-domain) [sonnet] Add the mark `Source::Documented` and remove A09 and A10 (ADR-0147 (h), (i), ADR-0148 (j), design D3).
  - Test file: `crates/recipe-domain/src/plant/s7_layout/reference.rs`.
  - Test: replace `reference_table_marks_every_composite_row_unverified` with a test that asserts the mark of each composite row by its id.
  - Test: the 21 rows of D3 have `Documented` with the rule ids of the research record. C01, C04, C08, and A11 end with "medium confidence".
  - Test: A03 has `Unverified("needs a TIA measurement")`. No composite row has `Unverified("Q10")`. No row has the id A09 or A10.
  - Test file: `crates/recipe-domain/src/plant/s7_layout/tests.rs`.
  - Test: `ARRAY[1..2] OF ARRAY[1..3] OF BYTE` on `S7_300` has the size 8. `ARRAY[1..3] OF ARRAY[1..3] OF BOOL` on `S7_300` has the size 6. A comment names `domain-model.md` §2.1 and ADR-0148 (i).
  - Red step of the two golden cases: they pin the current sizes. Apply one mutation to the stride of the `Even` branch of `array_layout`. Both cases fail.
  - Change: add the variant `Documented(&'static str)` with a doc comment and its `describe` arm. Set the marks of D3.
  - Change: remove A09, A10, and the catalog rows 212, 213, 409, and 410 from `reference.rs`. Update the doc comments of `composite` and of `S7Layout::of`.
  - Check: `cargo test -p recipe-domain` and `cargo doc --no-deps -p recipe-domain` with `RUSTDOCFLAGS="-D warnings"`.
- [x] 1.4 (recipe-domain) [sonnet] Report only an overlap that holds a written parameter (ADR-0149 (a) to (d), design D9).
  - Test file: the new module `crates/recipe-domain/src/recipe/draft/s7/written_overlap_tests.rs`, declared in `draft/s7.rs`.
  - Seed a draft on `S7_1500` with two stored overlap pairs under the layout of task 1.1 and task 1.2. Pair 1 is `BYTES` of type `ST_Bytes@1` at `DB1.DBB0` and `MODE` of type `BYTE` at `DB1.DBB4`.
  - Pair 2 is `GRID` of type `ARRAY[1..3,1..3] OF BOOL` at `DB2.DBB0` and `X` of type `BYTE` at `DB2.DBB2`. Add `SPEED` of type `INT` at `DB1.DBW8`.
  - Test: the scenario "Edit of a parameter outside a stored overlap" of the delta spec `recipe-management`. `SetParameterValue` and `ReplaceLimits` on `SPEED` pass.
  - Test: the scenario "Edit of a parameter of a stored overlap". `SetParameterValue` and `ReplaceLimits` on `MODE` give `E-RCP-022`.
  - Test: the scenario "Move of one parameter of a stored overlap". `UpdateParameterDefinition` on `MODE` with `DB1.DBB6` passes.
  - Test: `AddParameter` of `TEMP` of type `INT` at `DB1.DBW10` passes. A rebind of a parameter outside both pairs passes.
  - Test: the scenario "Approval of a draft with a stored overlap". `check_s7_layout_for_approval` gives `E-RCP-022`.
  - Change: `replace_parameters` passes the keys of its replacements to `check_s7_layout`. `find_overlap` skips a pair that holds no written key.
  - Change: update the doc comments of `replace_parameters`, `check_s7_layout`, and `check_s7_layout_for_approval`. Name ADR-0149.
  - Check: `cargo test -p recipe-domain`.
- [x] 1.5 (recipe-domain) [sonnet] Add the overlap check to the export check (ADR-0149 (e) to (j), design D10).
  - Test file: `crates/recipe-domain/src/export/rule_check/tests.rs`.
  - Test: a version with pair 1 of task 1.4 gives a rule fault `E-RCP-022` with `RuleField::Address` for `BYTES` and for `MODE`.
  - Test: a version with pair 1 and pair 2 gives the fault for all four parameters, in parameter order.
  - Test: a parameter with a fault of the end rule and an overlap keeps both faults. A version without an overlap gives no new fault.
  - Test: an overlap gives no `BrokenParameter`.
  - Change: add the step of D10 to `check_version_parameters`. Reuse `occupy` of `draft/s7.rs`, and find every overlapping pair.
  - Change: update the doc comment of `RuleFault` with the code `E-RCP-022`. Update the doc comment of `check_version_parameters`.
  - Check: `cargo test -p recipe-domain` and `cargo doc --no-deps -p recipe-domain` with `RUSTDOCFLAGS="-D warnings"`.

## 2. Rule of the array of an array

- [x] 2.1 (recipe-domain) [sonnet] Add the method of D4 to `TypeExpr` (ADR-0148 (a), (b), (d), (i)).
  - Test file: `crates/recipe-domain/src/plant/type_expr/tests.rs`.
  - Test: `ARRAY[1..2] OF ARRAY[1..3] OF BYTE` on `S7_1200` and on `S7_1500` gives `E-PLT-013` with the reason `NotOnPlatform` and the text of the expression.
  - Test: the same text on `S7_300`, `CodesysIec`, and `TwinCat3` passes. `ARRAY[1..2,1..3] OF BYTE`, `ARRAY[1..2] OF ST_Zone@1`, and `INT` pass on `S7_1500`.
  - Change: add the method with a doc comment that names the problem that it solves and ADR-0148.
  - Check: `cargo test -p recipe-domain` and `cargo doc --no-deps -p recipe-domain` with `RUSTDOCFLAGS="-D warnings"`.

## 3. Commands

- [x] 3.1 (recipe-app) [sonnet] Call the rule in `AddParameter` (design D5).
  - Test file: `crates/recipe-app/src/recipe/add_parameter/tests.rs`.
  - Test: the scenario "Parameter of an array of an array" of the delta spec `plant-model`. Assert that the memory adapter holds no new type row and no new parameter.
  - Test: the scenarios "Array with two dimensions", "Array of a struct with an array member", and "Array of an array on TwinCAT 3".
  - Test: the scenario "Stored array type in a new parameter". Seed the array rows through the memory adapter. Assert `E-PLT-013` and no new parameter.
  - Change: call the method of task 2.1 after `OpenDraft::open` and before `resolve_type`.
  - Check: `cargo test -p recipe-app`.
- [x] 3.2 (recipe-app) [sonnet] Call the rule in `UpdateParameterDefinition` (design D5).
  - Test file: `crates/recipe-app/src/recipe/update_definition/tests.rs`.
  - Test: the scenario "Changed type to an array of an array". Assert that `SPEED` keeps `INT` and that no type row is new.
  - Test: the scenario "Parameter keeps a stored array of an array". Seed the stored type through the memory adapter and not through a command.
  - Change: call the method of task 2.1 in the branch that calls `require_current_type`.
  - Red step of the keep test: call the method before the branch. The test fails with `E-PLT-013`.
  - Check: `cargo test -p recipe-app`.
- [x] 3.3 (recipe-app) [sonnet] Call the rule in `DefineUdt` (design D5).
  - Test file: `crates/recipe-app/src/plant/define_udt/tests.rs`.
  - Test: the scenario "Member of an array of an array". Assert `E-PLT-006` with the member `cells` and that no type row is new.
  - Test: a member `ARRAY[1..3,1..3] OF BOOL` on `S7_1200` passes.
  - Test: the scenario "New revision of a stored struct with an array of an array". Seed `ST_Grid@1` through the memory adapter. Assert `E-PLT-006` for `cells` and no revision `ST_Grid@2`.
  - Change: call the method of task 2.1 in `ensure_members` before `ensure_expr`, so `member_error` maps the error.
  - Check: `cargo test -p recipe-app`.
- [x] 3.4 (recipe-app) [sonnet] Pin the rebind of a stored array of an array (ADR-0148 (f), design D5).
  - Test file: `crates/recipe-app/src/recipe/rebind/kept_member_tests.rs`.
  - Test: the scenario "Rebind of a stored array of an array". Seed `ZONES` through the memory adapter.
  - Red step: in `ensure_shape` of `crates/recipe-app/src/plant/ensure_derived.rs`, return `E-PLT-013` with the reason `NotOnPlatform` when the platform is `S7_1200` or `S7_1500` and `shape.dimensions().len() > 1`. The test fails with `E-PLT-013`.
  - Check: `cargo test -p recipe-app`.

## 4. Type page, export, and version page

- [x] 4.1 (recipe-web) [sonnet] Pin the type page of a stored array of an array on `S7_1500` (ADR-0148 (e)).
  - Test file: `crates/recipe-web/src/pages/types/tree_tests.rs`.
  - Test: the scenario "Type page of a stored array of an array". Seed the rows directly on a machine with `S7_1500`. Assert `200`, the member `cells`, and the table S7 layout.
  - Red step: call the method of task 2.1 on the type text of each member in `crates/recipe-web/src/pages/types/tree.rs`. The test fails.
  - Check: `cargo test -p recipe-web`.
- [x] 4.2 (recipe-app) [sonnet] Pin the export check of an approved version with an overlap (ADR-0149 (e) to (j), design D10).
  - Test file: `crates/recipe-app/src/export/rule_check_tests.rs`.
  - Test: the scenario "Approved version with an overlap after a change of the layout rules" of the delta spec `export`. Assert status 200, `loadable` `false`, and the stored `content_sha256`.
  - Test: a unit document whose scope holds only `BYTES` gives `loadable` `false` (ADR-0149 (i)).
  - Test file: `crates/recipe-app/src/recipe/queries/version_tests.rs`.
  - Test: the version page query of the same version lists `E-RCP-022` once.
  - Red step: the tests pin task 1.5. Remove the step of D10 from `check_version_parameters` and run the tests. They fail.
  - Check: `cargo test -p recipe-app`.
- [x] 4.3 (recipe-web) [sonnet] Pin the notice of an approved version with an overlap (ADR-0149 (g), design D10).
  - Test file: `crates/recipe-web/src/pages/versions/export_notice_tests.rs`.
  - Test: the scenario "Approved version with an overlap after a change of the layout rules" of the delta spec `web-ui`. Assert the notice with `E-RCP-022` and the `aria-describedby` of the export link.
  - Red step: the test pins task 1.5. Remove the step of D10 from `check_version_parameters` and run the test. It fails.
  - Check: `cargo test -p recipe-web`.

## 5. Check of the stored data

- [x] 5.1 (recipe-db) [sonnet] Add the ops query `deploy/db/check-s7-layout-rules.sql` (design D8).
  - Test file: the new module `crates/recipe-db/tests/storage/s7_layout_rules.rs`, declared in `tests/storage/main.rs`. Copy the form of `tests/storage/identifier_rules.rs`.
  - Test: seed an array of an array on `S7_1500` and on `TwinCat3`, and a parameter of each type. Assert rows of `nested_array` only for `S7_1500`.
  - Test: seed a parameter with an address of a struct with a `SINT` member and a parameter of `ARRAY[1..3,1..3] OF BOOL`. Assert one row of `layout_changed` for each.
  - Test: rows that the change does not affect give no row. The query writes nothing and runs as `recipe_app`.
  - Change: add the file with the header of `check-identifier-rules.sql`.
  - Check: `cargo test -p recipe-db --test storage`.
- [x] 5.2 (docs/ops) [mechanical] Add the section of the runbook (design D8).
  - Add the section "Check the stored data against the S7 layout rules" to `docs/ops/runbook.md` and its line in the contents. Copy the form of the section "Check the stored type rows against the identifier rules".
  - Add the run of `check-s7-layout-rules.sql` as a sub-step of step 2 of "Upgrade".
  - The Haiku agent does only the edit. It does not run the STE check.
  - Check: the orchestrator runs `ste-lint.py` on `docs/ops/runbook.md` in a Sonnet agent. The run gives 0 hard violations.
- [ ] 5.3 (deploy) [owner] Run `deploy/db/check-s7-layout-rules.sql` on production with the command of task 5.2.
  - Put the result into the pull request. Sign off each row of `nested_array` and `layout_changed`.
  - On 2026-11-08 no production exists. The task stays open until a production exists.
  - Check: the pull request holds the result and the sign-off.

## 6. Gate

- [x] 6.1 (workspace) [sonnet] Run each command of the gate and put the output into the report.
  - The gate runs `cargo fmt --check` and the clippy command of `CLAUDE.md`. It runs the full test command of `CLAUDE.md` with Docker. It runs `cargo doc --no-deps --workspace` with `RUSTDOCFLAGS="-D warnings"`.
  - The gate also runs `openspec validate --all --strict` and the `ste-lint.py` command of `CLAUDE.md`.
  - Check: each command exits with 0, and `ste-lint.py` reports 0 hard violations.

## Workflow follow-up

- Run `code-review` on the diff of the change and fix each CRITICAL and HIGH finding.
- Run `superpowers:verification-before-completion`.
- Archive with `openspec-archive-change` after task 5.3 and after `cargo test` and `openspec validate` pass.
