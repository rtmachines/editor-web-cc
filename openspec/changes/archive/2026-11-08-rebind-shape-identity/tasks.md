# Tasks

Marks after the scope:

- `[sonnet]`: a Sonnet subagent with the effort `high` does the task.
- `[mechanical]`: a Haiku subagent does the task.

Each task is one agent. Each task of a crate starts with a failing test and shows the red step before the change.

A test that fails before the change of task 1.1 shows that failure as its red step. A pinning test passes before its task, so its red step uses one mutation of `design.md` D6:

1. Copy `crates/recipe-domain/src/recipe/rebind/level.rs` to the scratch directory.
2. Apply the mutation and run the test. The test fails on the mutation.
3. Restore the file and run `cmp <file> <copy>`. `cmp` gives no output.

The copy holds the state before the mutation of the task, so `cmp` also passes after an earlier task changed `level.rs`.

The report of each task gives the test name, the mutation or the failure before the change, and the failure line. It also states the result of `cmp` after each restore.

The tree passes `cargo test --workspace --all-features --locked` before task 1.1 and after each task. Each task runs the full tests of its crate. The orchestrator runs the workspace command after each wave.

The ADR, `docs/adr/README.md`, and the edits of `domain-model.md` and `data-model.md` are part of this change already. No task writes them.

Serial groups. The tasks of one serial group run one after the other in their order.

| Serial group | Tasks | Shared files |
|---|---|---|
| S1 | 1.1, 1.2, 2.1, 3.1 | `crates/recipe-domain/src/recipe/rebind/level.rs` and the `target/` directory of one tree |

Task 1.1 changes `level.rs`, and task 1.2 adds its `mod tests;` line. The red step of each task mutates and restores `level.rs`. Thus no two tasks of S1 run at the same time, and no task uses a worktree.

Task 4.1 runs after the last task of S1.

## 1. Domain rule

- [x] 1.1 (recipe-domain) [sonnet] Compare the type shape in `kept_members` (ADR-0146 (a) to (f), `design.md` D1, D2, D4, D6, D7).
  - Test file: the new module `crates/recipe-domain/src/recipe/rebind/kept_member_tests.rs`, declared in `rebind/mod.rs` next to `earlier_tests`. Build the seed string rows in this file (D7).
  - Test: on `CodesysIec`, the parameter `LABEL_1` of type `ST_Text@1` has the member `label` of type `STRING(80)`, the value `"ABC"`, and the limit `max_length` 40 on `.label`. `ST_Text@2` has `label` of the seed type `STRING`.
  - Test: assert that `plan_rebind` without new member values succeeds. Assert that the plan keeps the value of `LABEL_1` and gives limit rows with the same content.
  - Test: assert the same for `STRING` to `STRING(80)` on `CodesysIec`, for `STRING(254)` to `STRING` on `S7_1500`, and for `WSTRING(80)` to `WSTRING` on `TwinCat3`.
  - Test: assert the same for the member `lines` of type `ARRAY[1..2] OF STRING(80)` to `ARRAY[1..2] OF STRING` on `CodesysIec`, with the value `["A", "B"]`.
  - Test: for `STRING(80)` to `STRING(81)` on `CodesysIec`, assert the list `E-RCP-028` for `.label`, then `E-RCP-029` for `.label`.
  - Test: for the member `zone` of type `ST_Zone@2` to `ST_Zone@3` with equal members, assert one `E-RCP-028` for each leaf of `.zone`.
  - Test: assert that `rebind_new_leaves` of the first case lists no leaf, and that it lists `.label` for the case `STRING(81)`.
  - Change: add `same_type_shape` to `level.rs` and call it in `kept_members` at line 52 (D1). Change the two doc comments of D4.
  - Red steps: the cases of one shape fail before the change with `E-RCP-028` and `E-RCP-029`. The case `STRING(81)` uses M2. The case `ST_Zone@3` uses M5.
  - Check: `cargo test -p recipe-domain`.
- [x] 1.2 (recipe-domain) [sonnet] Pin the boundaries of `same_type_shape` (ADR-0146 (c) to (f), `design.md` D1, D6, D7).
  - Test file: the new module `crates/recipe-domain/src/recipe/rebind/level/tests.rs`, declared in `level.rs`. The tests call `struct_level` and `kept_members` on two struct trees.
  - Test: assert that `kept_members` keeps these members: one row of `INT` in both revisions, and one struct row `ST_Zone@2` in both revisions.
  - Test: on `CodesysIec`, assert that `kept_members` also keeps a member with two rows of one shape. The source has the machine row `STRING(80)`, and the target has the seed row `STRING` (D7).
  - Test: assert the same for `ARRAY[1..2] OF STRING(80)` to `ARRAY[1..2] OF STRING` on `CodesysIec`.
  - Test: assert that `kept_members` drops these members: `INT` to `DINT`, `INT` to `UINT`, `STRING(80)` to `WSTRING(80)`, and `ARRAY[1..2] OF STRING` to `ARRAY[1..3] OF STRING`.
  - Test: assert that `kept_members` also drops these members: `INT` to `ARRAY[1..1] OF INT`, `ST_Zone@2` to `ST_Zone@3` with equal members, and a member that only the source has.
  - Change: none outside the test module and its declaration.
  - Red steps: M1 (the cases with two rows), M3 (`INT` to `UINT`), M4, and M5.
  - Check: `cargo test -p recipe-domain`.

## 2. Use case

- [x] 2.1 (recipe-app) [sonnet] Pin the writes and the audit entry of a rebind that keeps every member (ADR-0146 (h), (i), `design.md` D5).
  - Test file: the new module `crates/recipe-app/src/recipe/rebind/kept_member_tests.rs`, declared in `rebind.rs` next to `write_tests`. Add the seed row of `STRING` on `CodesysIec` in this file if the memory fixture lacks it.
  - Test: on a `CodesysIec` machine, `ST_Text@1` has `label` of type `STRING(80)` and `ST_Text@2` has `label` of the seed type `STRING`. `LABEL_1` holds `"ABC"` and the limit `max_length` 40 on `.label`.
  - Test: run `RebindTypeRevision` without new member values. Assert success. Assert that `LABEL_1` has the type `ST_Text@2`, keeps its value, and has limit rows with the same content.
  - Test: assert one `TypeRevisionRebound` entry. Its diff holds `plc_type_id` and no key `value` or `limits`. Assert no `DerivedTypeCreated` entry.
  - Test: with `label` of type `STRING(81)` in `ST_Text@2`, assert the lines `E-RCP-028` and `E-RCP-029` for `.label`. Assert that the command writes nothing.
  - Change: none outside the test module and its declaration.
  - Red step: M1.
  - Check: `cargo test -p recipe-app`.

## 3. Storage

- [x] 3.1 (recipe-db) [sonnet] Pin the rebind across the seed row of `STRING` on PostgreSQL (ADR-0146 (c), (g), (h), `design.md` D5).
  - Test file: the new module `crates/recipe-db/tests/storage/rebind_shape.rs`, declared in `tests/storage/main.rs`.
  - Test: on a `CodesysIec` machine, run `DefineUdt` for `ST_Text` with `label` of type `STRING(80)`. Then run it for revision 2 with `label` of type `STRING`.
  - Test: assert that the member of revision 2 references the seed row `5eed0000-0000-4000-8000-00000000001e`. Assert that the member of revision 1 references a row of the machine with `string_length = 80`.
  - Test: run `AddParameter` for `LABEL_1` of type `ST_Text@1` with `"ABC"` and the limit `max_length` 40 on `.label`. Run `RebindTypeRevision` without new member values.
  - Test: assert that the row of `LABEL_1` references revision 2 and keeps its `value`. Assert one audit row `TypeRevisionRebound` more than before.
  - Test: assert that `LABEL_1` has one `parameter_limit` row. Assert that `leaf_path`, `min_value`, `max_value`, and `max_length` equal the values before the rebind. The rebind writes the row again, so the test does not compare `created_at` (D5).
  - Change: none outside the test module and its declaration.
  - Red step: M1.
  - Check: `cargo test -p recipe-db --test storage`. The test needs Docker.

## 4. Gate

- [x] 4.1 (workspace) [sonnet] Run each command of the gate and put the output into the report.
  - The gate runs `cargo fmt --check` and the clippy command of `CLAUDE.md`. It runs the full test command of `CLAUDE.md` with Docker. It runs `cargo doc --no-deps --workspace` with `RUSTDOCFLAGS="-D warnings"`.
  - The gate also runs `openspec validate --all --strict` and the `ste-lint.py` command of `CLAUDE.md`.
  - Check: each command exits with 0, and `ste-lint.py` reports 0 hard violations.

## Workflow follow-up

- Run `code-review` on the diff of the change and fix each CRITICAL and HIGH finding.
- Run `superpowers:verification-before-completion`.
- Archive with `openspec-archive-change` after `cargo test` and `openspec validate` pass.
