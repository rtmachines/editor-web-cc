# Tasks

Marks after the scope:

- `[sonnet]`: a Sonnet subagent with the effort `high` does the task.
- `[mechanical]`: a Haiku subagent does the task.
- `[owner]`: the owner does the task. No agent does it.

Each task is one agent. Each task of a crate starts with a failing test and shows the red step before the fix.

The red step of a pinning test uses one mutation of `design.md` D11:

1. Copy the file to the scratch directory.
2. Apply the mutation and run the test. The test fails on the mutation.
3. Revert the mutation and run `cmp <file> <copy>`. `cmp` gives no output.

A test that fails only on a wrong expected value shows no red step. The report of each task gives the test name, the mutation, and the failure line.

The tree passes `cargo test --workspace --all-features --locked` before task 1.1 and after each task (`design.md` D12). Each task runs the full tests of its crate. The orchestrator runs the workspace command after each wave.

The document tests of D10 are these three tests of `cargo test -p recipe-db --test storage`:

- `migration_0001_equals_the_sql_blocks_of_sections_3_0_to_3_17`
- `every_name_of_section_3_exists_in_the_catalog`
- `every_named_constraint_and_unique_index_of_section_3_is_covered_or_exempt`

Serial groups. The tasks of one serial group run one after the other in their order. Two tasks of two serial groups can run in parallel after their dependencies end.

| Serial group | Tasks | Shared files |
|---|---|---|
| S1 | 1.1, 1.2, 1.3, 1.4 | `migrations/0011_plc_type_rules.sql`, `docs/architecture/data-model.md`, `crates/recipe-db/tests/storage/support/document.rs`, `tests/storage/migration_0011.rs`, `tests/storage/constraints/type_rules.rs`, `deploy/ci/migration_rewrite.py` |
| S2 | 2.1, 2.2, 2.3 | `crates/recipe-domain/src/plant/type_name.rs`, `plc_type.rs`, `error.rs`, `type_expr.rs` |
| S3 | 2.2, 3.2 | `crates/recipe-web/i18n/en.ftl`, `crates/recipe-web/i18n/pl.ftl` |
| S4 | 3.3, 3.4, 4.1 | `crates/recipe-app/src/plant/define_udt.rs`, `define_udt/name_tests.rs`, `crates/recipe-app/src/memory/types/mod.rs` |
| S5 | 3.1, 4.2 | `crates/recipe-db/src/transaction/type_rows.rs`, `crates/recipe-db/tests/storage/contract/types/mod.rs` |
| S6 | 5.1, 5.2 | `deploy/db/check-identifier-rules.sql`, `docs/ops/runbook.md` |

Dependencies outside the serial groups:

- Group 3 needs task 2.3. Task 3.1 also needs task 1.1.
- Task 5.1 needs tasks 1.1, 1.4, and 2.1. Task 1.1 and task 5.1 both add a line to `crates/recipe-db/tests/storage/main.rs`.
- Task 5.3 needs tasks 5.1 and 5.2. Task 6.1 needs every other task except 5.3.

## 1. Schema

- [x] 1.1 (recipe-db) [sonnet] Add migration `0011` with the new `dims_valid` (ADR-0145, D6 step 5).
  - Test file: the new module `tests/storage/constraints/type_rules.rs` with an empty `COVERED` list. `constraints/mod.rs` adds the list to both `concat` lists.
  - Test: assert that `dims_valid('[0:0]={5}','{0}')`, `dims_valid('[0:0]={0}','{65536}')`, and `dims_valid('{1}','[0:0]={8}')` give `false`. Assert that `dims_valid('{1}','{8}')` gives `true`.
  - Test: assert that an `INSERT` of an array row of `INT` with the bounds `'[0:0]={5}'` and `'{0}'` fails with `23514` on `plc_type_array_shape`.
  - Test file: the new module `tests/storage/migration_0011.rs`, declared in `tests/storage/main.rs`. Its test compares the function text of `0011` with `data-model.md` §3.3.
  - Change: add `migrations/0011_plc_type_rules.sql` with the `CREATE OR REPLACE FUNCTION dims_valid` of §3.3. Add the two lines of D10, row 1.1, to `dims_valid` in §3.3.
  - Change: in `support/document.rs`, rename `Adr0109Piece` to `ShippedPiece` and `ADR_0109_PIECES` to `PIECES_AFTER_0001`. Add the piece of `dims_valid`.
  - Change: update the `dims_valid` line of `deploy/pgtools/restore-code.expected`. Set `EXPECTED_MIGRATIONS` in `crates/recipe-server/tests/migrate.rs` to 11 and its doc comment to `0001` to `0011`.
  - Red steps: M1a and M1b.
  - Check: `cargo test -p recipe-db --test storage`, which holds the document tests and `restore_code`. Run `cargo test -p recipe-server --test migrate` and `python -I deploy/ci/migration_rewrite.py migrations`.
- [x] 1.2 (recipe-db) [sonnet] Add the check of the stored rows to migration `0011` (D6 steps 1 to 4).
  - Test file: `tests/storage/migration_0011.rs`.
  - Test: reset `dims_valid` to the text of `0001` as the migrator. Drop `plc_type_array_shape` and use `DROP INDEX IF EXISTS` on both indexes of ADR-0143 (j).
  - Test: insert an array row with `dim_lower` at the subscript 0 and an array row with only `dim_upper` at the subscript 0. Insert the structs `st_zone@1` and `ST_Zone@1` and a struct with the members `speed` and `Speed`.
  - Test: run the text of `0011` as the migrator. Assert the error text of D6 with the counts 2, 1, and 1. Assert that `pg_get_functiondef` of `dims_valid` still gives the text of `0001`. Assert that neither index exists.
  - Test: run the text of `0011` on rows that follow the rules. Assert the new `dims_valid` and that `plc_type_rule_check` does not exist after the run.
  - Change: add the steps 1 to 4 of D6 before the `CREATE OR REPLACE FUNCTION` of `0011`.
  - Red steps: M10a and M10b.
  - Check: `cargo test -p recipe-db --test storage` and `python -I deploy/ci/migration_rewrite.py migrations`.
- [x] 1.3 (recipe-db) [sonnet] Add the index of the struct names to migration `0011` (ADR-0143 (j), (k), D7).
  - Test file: `tests/storage/constraints/type_rules.rs`. Add `plc_type_struct_name_ci_unique` to its `COVERED` list.
  - Test: insert the struct `ST_Zone@1` and then `st_zone@1` on one machine as `recipe_app`. Assert `23505` on `plc_type_struct_name_ci_unique`. Assert that `st_zone@1` on another machine and `st_zone@2` next to `ST_Zone@1` succeed.
  - Test: in `tests/storage/migration_0011.rs`, compare the statement of `0011` with §3.3. The test of the rows that follow the rules also asserts that the index exists.
  - Change: add step 6 of D6 to `0011` and the lines of D10, row 1.3, to §3.3. Add the piece to `PIECES_AFTER_0001`.
  - Change: add the entry `("0011_plc_type_rules.sql", "index without concurrently"): "ADR-0143"` to `EXEMPT` of `deploy/ci/migration_rewrite.py`.
  - Red steps: M2 and M17.
  - Check: `cargo test -p recipe-db --test storage` and `python -I deploy/ci/migration_rewrite.py migrations`. Run `python -I -B deploy/ci/test_migration_rewrite.py` and `python -I -B deploy/ci/test_migration_rewrite_forms.py`.
- [x] 1.4 (recipe-db) [sonnet] Add the index of the member names to migration `0011` (ADR-0143 (j), (k), D7).
  - Test file: `tests/storage/constraints/type_rules.rs`. Add `plc_type_member_name_ci_unique` to its `COVERED` list.
  - Test: insert a struct with the members `speed` and `Speed` as `recipe_app` in one transaction. Assert `23505` on `plc_type_member_name_ci_unique`.
  - Test: in `tests/storage/migration_0011.rs`, compare the statement of `0011` with §3.4. The test of the rows that follow the rules also asserts that the index exists.
  - Change: add step 7 of D6 to `0011` and the lines of D10, row 1.4, to §3.4. Add the piece to `PIECES_AFTER_0001`.
  - Red step: M3.
  - Check: `cargo test --workspace --all-features --locked` and the three `python` commands of task 1.3.

## 2. Domain rules

- [x] 2.1 (recipe-domain) [sonnet] Add the reserved words and `TypeName::from_stored` (ADR-0143 (c) to (e), D1).
  - Test file: `crates/recipe-domain/src/plant/type_name/tests.rs`.
  - Test: assert that `TypeName::parse` refuses `Int`, `real`, `ARRAY`, `OF`, `STRUCT`, `END_STRUCT`, and `end_method` with `TypeNameReason::Reserved`. Assert that it accepts `ST_Zone`.
  - Test: assert that `TypeName::from_stored("Int", RuleSet::Earlier)` gives `Int` and that `RuleSet::Current` refuses it. Assert that `RESERVED_KEYWORDS` has 182 words in upper case, without a duplicate, and without a primitive name.
  - Change: replace the test `reserved_check_is_case_sensitive` and the doc comment of lines 13 to 14 of `type_name.rs`. Add `RESERVED_KEYWORDS`, `is_reserved_name`, and `from_stored`. The author found no other fixture with a reserved struct name in the workspace.
  - Red steps: M4, M5, and M7 on `TypeName::from_stored`.
  - Check: `cargo test -p recipe-domain`.
- [x] 2.2 (recipe-domain) [sonnet] Add the current and the earlier rules of a struct row to `PlcType` (ADR-0143 (b), (i), D2).
  - Test file: `crates/recipe-domain/src/plant/plc_type/tests.rs`.
  - Test: assert that `PlcType::try_from` and `PlcType::from_stored(_, RuleSet::Earlier)` refuse a struct with the members `speed` and `Speed` with `E-PLT-021` for `Speed`.
  - Test: assert that `PlcType::try_from` refuses the struct `Int@1` with `E-PLT-013` and `DerivedTypeReason::ReservedName`. Assert that `PlcType::from_stored(_, RuleSet::Earlier)` accepts it. Assert that a primitive row `INT` still passes.
  - Test: in `crates/recipe-web/src/error/plant.rs`, assert the key `type-reason-reserved-name`. Assert that the `Display` text of `ReservedName` equals its `en` text.
  - Change: add `from_stored` and `ReservedName`. Add the key to `error/plant.rs` and the texts of D2 to `en.ftl` and `pl.ftl`.
  - Red steps: M6 and M7 on `PlcType::from_stored`.
  - Check: `cargo test -p recipe-domain` and `cargo test -p recipe-web`.
- [x] 2.3 (recipe-domain) [sonnet] Give `parse_udt` a rule set (ADR-0143 (i1), (i2), D3a).
  - Test file: `crates/recipe-domain/src/plant/type_expr/tests.rs`.
  - Test: assert that `TypeExpr::parse("Int@1")` and `TypeExpr::parse("ARRAY@1")` give `E-PLT-013` with `ReservedName`. Assert that `TypeExpr::parse("INT@1")` still gives `UnknownStruct`.
  - Test: assert that `TypeExpr::from_stored("Int@1", RuleSet::Earlier)` resolves to the stored struct `Int@1` of the type tree. Assert that `INT@1` still gives `UnknownStruct`.
  - Change: add the `RuleSet` argument to `parse_udt` and add `TypeExpr::from_stored`.
  - Red steps: M7 on `TypeExpr::from_stored` and M14.
  - Check: `cargo test -p recipe-domain`.

## 3. Stored data and mapping

- [x] 3.1 (recipe-db) [sonnet] Read the stored type rows with the earlier rules (D3).
  - Test file: the new module `tests/storage/contract/types/stored.rs`, declared in `contract/types/mod.rs`.
  - Test: insert the struct `Int@1` as `recipe_app`. Assert that the read of the type rows of the machine gives `Int@1` and its members. Assert that the audit label of `Int@1` reads.
  - Change: `shape_of`, `build_type`, and line 174 of `audit_reads.rs` use `RuleSet::Earlier`. `ensure_derived` keeps `PlcType::try_from`.
  - Red step: M7 on `build_type`, which then passes `RuleSet::Current`.
  - Check: `cargo test -p recipe-db --test storage`.
- [x] 3.2 (recipe-web) [sonnet] Serve a stored struct with a reserved name and refuse a new one (D3).
  - Test file: the new module `crates/recipe-web/src/pages/types/reserved_name_tests.rs`, declared in `pages/types.rs` next to `define_error_tests`.
  - Test: with a stored struct `Int@1` from `PlcType::from_stored`, assert that `GET` of the type page of `Int` gives `200` and shows the members.
  - Test: assert that the form of a new struct with the name `Int`, `real`, `ARRAY`, `OF`, `STRUCT`, or `end_struct` gives `E-WEB-008` for `type_name`. Assert that the form creates no type.
  - Test: assert that the form of a new revision of `Int` gives `E-PLT-013` with the text of `type-reason-reserved-name`.
  - Change: `path_values::type_name` uses `RuleSet::Earlier`. The texts of `reason-type-name` become the texts of D3.
  - Red step: M7 on `path_values::type_name`, which then passes `RuleSet::Current`.
  - Check: `cargo test -p recipe-web`.
- [x] 3.3 (recipe-app) [sonnet] Map the two new indexes and compare the struct names without case in the memory adapter (D4).
  - Test file: the new module `crates/recipe-app/src/plant/define_udt/name_tests.rs`, declared next to `define_udt/tests.rs`.
  - Test: inject `PLC_TYPE_STRUCT_NAME_CI_UNIQUE` on the struct insert. Assert that `DefineUdt` for `st_zone` gives `E-PLT-015` and writes nothing. This test covers the spec scenario "Two names in two cases at one time".
  - Test: inject `PLC_TYPE_MEMBER_NAME_CI_UNIQUE`. Assert `E-PLT-021` for the first member.
  - Test: assert that the memory adapter refuses `st_zone@1` next to `ST_Zone@1` with `PLC_TYPE_STRUCT_NAME_CI_UNIQUE`.
  - Change: add the two constants to `error/mapping.rs` and the mapping to `define_udt.rs`. Add the compare without case to `memory/types/mod.rs` and the two names to `memory/fault.rs`.
  - Red step: M12.
  - Check: `cargo test -p recipe-app`.
- [x] 3.4 (recipe-app) [sonnet] Compare the name of a new struct without case in `next_revision` (D4).
  - Test file: `crates/recipe-app/src/plant/define_udt/name_tests.rs`.
  - Test: with `ST_Zone@1` stored on `M12`, assert that `DefineUdt` in the mode `New` with the name `st_zone` gives `E-PLT-015` for `ST_Zone` and writes nothing. Assert that the mode `Revision` with `st_zone` gives `NotFound`.
  - Test: assert that `DefineUdt` in the mode `New` with the name `st_zone` on `M7` creates `st_zone@1`.
  - Test: assert that the members `speed` and `Speed` give `E-PLT-021` for `Speed` and write nothing. Assert that the member `REAL` of type `INT` creates `ST_Text@1`.
  - Test: with the stored struct `Int@1` from `PlcType::from_stored`, assert that the mode `Revision` gives `E-PLT-013` with `ReservedName` and writes nothing.
  - Change: `next_revision` of `define_udt.rs`.
  - Red step: M11.
  - Check: `cargo test -p recipe-app`.
- [x] 3.5 (recipe-app) [sonnet] Check a type text that a command writes with the current rule (ADR-0143 (i2), D3a).
  - Test file: `crates/recipe-app/src/recipe/add_parameter/tests.rs` and `crates/recipe-app/src/recipe/update_definition/tests.rs`.
  - Test: with the stored struct `Int@1`, assert that `AddParameter` with the type `Int@1` gives `E-PLT-013` with `ReservedName`. Assert that it adds no parameter and takes no lock.
  - Test: assert that `AddParameter` with the type `st_zone@1` next to `ST_Zone@1` gives `E-PLT-013` with `UnknownStruct`.
  - Test: assert that `UpdateParameterDefinition` on `SPEED` of type `INT` with the type `Int@1` gives `E-PLT-013` with `ReservedName`. `SPEED` keeps `INT`.
  - Test: assert that `UpdateParameterDefinition` on `ZONE` of type `Int@1` with the type `Int@1` and a new name stores the name. `ZONE` keeps `Int@1`.
  - Change: add `require_current_type` to `definition_steps.rs`. Call it in `add_parameter.rs` and `update_definition.rs` as D3a says.
  - Red step: M15.
  - Check: `cargo test -p recipe-app`.
- [x] 3.6 (recipe-web) [sonnet] Parse a stored parameter type text with the earlier rule (D3).
  - Test file: `crates/recipe-web/src/pages/parameters/edit_tests.rs` and `crates/recipe-web/src/pages/versions/parameter_table_tests.rs`.
  - Test: with the parameter `ZONE` of type `Int@1`, assert that the edit form shows the grid of `Int@1`. Assert that a save with a new name gives `200` and keeps the type.
  - Test: assert that the version table shows `ZONE` of type `Int@1` without an error.
  - Change: the four parse sites of D3 use `TypeExpr::from_stored(_, RuleSet::Earlier)`.
  - Red step: M7 on `definition::check`, which then calls `TypeExpr::parse`.
  - Check: `cargo test -p recipe-web`.

## 4. Lock order

- [x] 4.1 (recipe-app) [sonnet] Collect the key texts in one order in `recipe-app` (ADR-0144 (d), D5).
  - Test file: `crates/recipe-app/src/plant/derived_lock/tests.rs` and `crates/recipe-app/src/memory/types/tests.rs`.
  - Test: assert that the keys of `DefineUdt` for a revision of the stored struct `ARRAY` with a member of type `STRING(5)` equal the keys of `AddParameter` for `ARRAY[1..2] OF STRING(5)`. Both give the texts `ARRAY` and `STRING` once each, in the order of the texts.
  - Test: assert that the keys of `DefineUdt` for `ST_Zone` with the member `temps` of type `ARRAY[1..2] OF REAL` are `ARRAY` and `ST_Zone`.
  - Change: replace the tests at lines 45 and 59 of `derived_lock/tests.rs` and the test at line 386 of `memory/types/tests.rs`. Change `derived_type_locks`, `rebind_locks`, and the doc comment of `DerivedTypeLock`.
  - Red step: M8.
  - Check: `cargo test -p recipe-app`.
- [x] 4.2 (recipe-db) [sonnet] Take the class-2 locks in the order of their key values (ADR-0144 (a) to (c), D5).
  - Test file: `tests/storage/contract/types/advisory.rs`.
  - Test: replace `postgres_only_the_keys_are_taken_in_the_given_order`. Assert that `pg_locks` holds the class-2 locks of the keys in ascending order.
  - Test: find two texts with one `hashtext` value for the test machine with `generate_series`. Run `recipe_db::statements::DERIVED_LOCK_KEYS` on the texts and two other texts. Assert each key value once and in ascending order. Assert one lock in `pg_locks` for the two texts.
  - Test file: the new module `tests/storage/race/derived_locks.rs`, declared in `race/mod.rs`.
  - Test: a third connection holds the lock `ARRAY`. Transaction B takes `[ARRAY, STRING]`. Poll `pg_locks` until B waits, as `advisory.rs` lines 129 to 185 do. Then transaction A takes `[STRING, ARRAY]`. Poll until A waits. Then the third connection commits.
  - Test: assert that A and B commit within `db.lock_timeout` and that no statement gives `40P01`.
  - Change: `lock_derived` of `type_rows.rs` runs the statement of D5. Add `DERIVED_LOCK_KEYS` to `recipe_db::statements`.
  - Red steps: M9 and M13.
  - Check: `cargo test -p recipe-db --test storage`.

## 5. Check of the stored data

- [x] 5.1 (recipe-db) [sonnet] Add the ops query `deploy/db/check-identifier-rules.sql` (D9).
  - Test file: the new module `tests/storage/identifier_rules.rs`, declared in `tests/storage/main.rs`. Copy the form of `tests/storage/platform_ranges.rs`.
  - Test: assert that the reserved list between the marker lines of the file equals `RESERVED_KEYWORDS` and the names of `Primitive::ALL`.
  - Test: drop the two indexes of ADR-0143 (j) and `plc_type_array_shape`. Seed one case pair of struct names, one case pair of member names, one array row at the subscript 0, and one struct `Int@1`.
  - Test: assert two rows for each case pair, one row for the array row, and one row for `Int@1`. Assert that rows that follow the rules give no row. Assert that the query writes nothing and runs as `recipe_app`.
  - Change: add the file.
  - Red step: M16.
  - Check: `cargo test -p recipe-db --test storage`.
- [x] 5.2 (docs/ops) [mechanical] Add the section of the runbook (D9).
  - Add the section "Check the stored type rows against the identifier rules" to `docs/ops/runbook.md` and its line in the contents. Copy the form of the section "Check the stored data against the platform ranges".
  - Add the run of `check-identifier-rules.sql` as a sub-step of step 2 of "Upgrade".
  - Check: `ste-lint.py` on `docs/ops/runbook.md` gives 0 hard violations.
- [ ] 5.3 (deploy) [owner] Run `deploy/db/check-identifier-rules.sql` on production with the command of task 5.2.
  - Put the result into the pull request. The parts `struct_name_case`, `member_name_case`, and `array_subscript` must have 0 rows. Sign off each row of `reserved_struct_name`.
  - Check: the pull request holds the result and the sign-off.

## 6. Gate

- [x] 6.1 (workspace) [sonnet] Run each command of the gate and put the output into the report.
  - The gate runs `cargo fmt --check` and the clippy command of `CLAUDE.md`. It runs the full test command of `CLAUDE.md` with Docker. It runs `cargo doc --no-deps --workspace` with `RUSTDOCFLAGS="-D warnings"`.
  - The gate also runs `python -I -B deploy/ci/migration_rewrite.py migrations`, `openspec validate --all --strict`, and the `ste-lint.py` command of `CLAUDE.md`.
  - Check: each command exits with 0, and `ste-lint.py` reports 0 hard violations.

## Workflow follow-up

- Run `code-review` on the diff of the change and fix each CRITICAL and HIGH finding.
- Run `superpowers:verification-before-completion`.
- Archive with `openspec-archive-change` after task 5.3 and after `cargo test` and `openspec validate` pass.
