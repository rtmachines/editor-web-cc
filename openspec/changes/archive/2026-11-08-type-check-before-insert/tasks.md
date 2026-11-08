# Tasks

Marks after the scope:

- `[sonnet]`: a Sonnet subagent with the effort `high` does the task.
- `[mechanical]`: a Haiku subagent does the task.

Each task is one agent run. Each task of a crate starts with a failing test and shows the red step before the change.

A pinning test passes before its task. Its red step uses one mutation:

1. Copy the file to the scratch directory.
2. Apply the mutation and run the test. The test fails on the mutation.
3. Revert the mutation and run `cmp <file> <copy>`. `cmp` gives no output.

The report of each task gives the test names, the red step, and the failure line.

The tree passes `cargo test --workspace --all-features --locked` before task 1.1 and after each task. Each task runs the full tests of its crate. The orchestrator runs the workspace command, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, and `cargo fmt --all --check` after each wave.

Serial groups. The tasks of one serial group run one after the other in their order.

| Serial group | Tasks | Shared files |
|---|---|---|
| S1 | 1.1, 1.2, 1.4, 1.3 | `crates/recipe-app/src/plant/queries/type_text.rs`, `crates/recipe-app/src/recipe/draft_session.rs`, `crates/recipe-app/src/recipe/update_definition.rs` and its test modules, `crates/recipe-db/tests/storage/race/` |
| S2 | 1.5 | `crates/recipe-db/tests/storage/seed.rs` |
| S3 | 2.1, 2.2 | `CHANGELOG.md` |

Task 1.3 is in S1, because its red step changes `update_definition.rs`. Task 1.4 also changes that file.

Order of the waves:

1. Tasks 1.1 and 1.2 run first, one after the other.
2. Tasks 1.4, 1.5, and 2.1 start after task 1.2. They can run in parallel.
3. Task 1.3 starts after task 1.4. Task 2.2 starts after task 2.1.

## 1. Type check before the first insert (D1-1)

- [x] 1.1 (recipe-app) [sonnet] Add `expr_names_row` and prove that it equals the comparison of `type_id` (design D1, D2).
  - Test file: `crates/recipe-app/src/plant/queries/type_text_tests.rs`.
  - Test: for each text of `EXPRESSIONS` and for each row of the fixture, `expr_names_row(types, id, expr)` is `true` exactly when `DerivedTypes::ensure_expr` resolves `expr` to `id`.
  - Test: on the platform of the fixture, `STRING` names the global seed row and not the row `STRING(80)` of the machine. `STRING(80)` names only the row of the machine.
  - Test: the stored struct `Int@1`, read with `RuleSet::Earlier`, names its row. `int@1` names no row.
  - Test: `expr_names_row` gives `false` for an identifier that `types` does not hold.
  - Red step: the tests fail to compile before the function exists. Show the compiler line.
  - Change: add the free function `expr_names_row(types: &PlatformTypes, id: PlcTypeId, expr: &TypeExpr) -> bool` to `type_text.rs`. It compares `type_text(types, id)` with `expr.to_string()`.
  - Change: add no re-export. The tests import the function through `super::type_text`, as they import `type_text`.
  - Change: put `#[cfg_attr(not(test), expect(dead_code, reason = "task 1.2 adds the caller"))]` on the function. Without a caller outside the tests, `dead_code` fails clippy with `-D warnings`.
  - Change: the doc comment names the problem that the function solves and cites design D2 of this change.
  - Check: `cargo test -p recipe-app type_text`, then `cargo test -p recipe-app`, then `cargo clippy -p recipe-app --all-targets --all-features --locked -- -D warnings`.
- [x] 1.2 (recipe-app) [sonnet] Check a changed type before the unit lock and `resolve_type` (design D3, plan 6.1 tasks 1, 3).
  - Test file: `crates/recipe-app/src/recipe/update_definition/nested_array_tests.rs`.
  - Test: on an `S7_1500` draft, an update of `SPEED` to `ARRAY[1..2] OF ARRAY[1..3] OF INT` gives `E-PLT-013` with the reason `NotOnPlatform`. The journal holds no line of `DERIVED_TYPE_LOCK_TABLE` and no `unit` lock. The type rows of the store do not change.
  - Test file: `crates/recipe-app/src/recipe/update_definition/tests.rs`.
  - Test: on `twincat_draft` with `seed_zone_on_int_struct`, an update of `SPEED` to `ARRAY[1..2] OF Int@1` (read with `RuleSet::Earlier`) gives `E-PLT-013` with the reason `ReservedName`. The journal holds no line of `DERIVED_TYPE_LOCK_TABLE` and no `unit` lock. The draft does not change.
  - Red step: run both tests on the current code. Each test fails on the line of `DERIVED_TYPE_LOCK_TABLE` with the key `ARRAY`.
  - Change: `resolve_trees` in `draft_session.rs` returns its `PlatformTypes`. `OpenDraft` keeps them in a new field `types` with a doc comment.
  - Change: export `expr_names_row` through `plant/queries/mod.rs` and `plant/mod.rs` with `pub(crate) use`, as `type_text`.
  - Change: remove the `expect(dead_code)` attribute of task 1.1. With the caller, the attribute fails clippy through `unfulfilled_lint_expectations`.
  - Change: `stage_definition` in `update_definition.rs` follows the order of design D3. It calls `expr_names_row` with the stored type of the parameter. It runs `require_current_type` and `require_no_nested_array` when the function gives `false`. Remove the check after `resolve_type`.
  - Change: the doc comment of `UpdateParameterDefinition` says that the command checks a changed type before the unit lock.
  - Check: `cargo test -p recipe-app update_definition`, then `cargo test -p recipe-app`, then `cargo clippy -p recipe-app --all-targets --all-features --locked -- -D warnings`.
  - Stop: when a test asserts `E-PLT-020`, `E-PLT-009`, or `E-PLT-018` before `E-PLT-013`, stop and report it. Do not change that test.
- [x] 1.4 (recipe-app) [sonnet] Pin that an update which keeps a stored type passes (plan 6.1 task 4, ADR-0143 (i2), ADR-0148 (c)).
  - Test file: `crates/recipe-app/src/recipe/update_definition/tests.rs`. Use `nested_array_tests.rs` for the `S7_1500` case.
  - Test: a parameter of type `ARRAY[1..2] OF Int@1` keeps that type with a new name. The command writes no `DerivedTypeCreated` entry.
  - Test: a parameter of the machine type `STRING(20)` keeps that type with a new name.
  - Test: on `S7_1500`, a parameter of the stored type `ARRAY[1..2] OF ARRAY[1..2] OF INT` keeps that type with a new name and a new value.
  - Red step (pinning test): mutation in `update_definition.rs` that runs both checks for each request. The tests of `Int@1` and of the array of an array fail with `E-PLT-013`.
  - Check: `cargo test -p recipe-app update_definition`, then `cargo test -p recipe-app`.
- [x] 1.3 (recipe-db) [sonnet] Pin on PostgreSQL that a refused type takes no class-2 lock (plan 6.1 task 2).
  - Test file: the new module `crates/recipe-db/tests/storage/race/refused_type.rs`, declared in `race/mod.rs`. Copy the lock pattern of `race/derived_locks.rs` and the request of `atomic/draft.rs`.
  - Test: `App::create()` with the default pool. A machine on `S7_1500` has a draft that holds `SPEED` of type `INT`. A third connection holds the class-2 lock with the key `ARRAY` of that machine.
  - Test: `UpdateParameterDefinition` of `SPEED` to `ARRAY[1..2] OF ARRAY[1..3] OF INT` gives `E-PLT-013`. The test sets no wall-clock bound.
  - Test: after the command, the audit log holds no new `DerivedTypeCreated` entry, `plc_type` holds no new row, and `SPEED` keeps `INT`.
  - Red step (pinning test): mutation in `update_definition.rs` that moves the two checks after `resolve_type`. The command then waits for the held lock until `db.lock_timeout` and gets `E-DB-001`. The test fails on the code.
  - Check: `cargo test -p recipe-db --test storage refused_type`, then `cargo test -p recipe-db`. The tests need Docker.
- [x] 1.5 (recipe-db) [sonnet] Pin the global `STRING` and `WSTRING` seed rows of each platform (design D2, F2).
  - Test file: `crates/recipe-db/tests/storage/seed.rs`.
  - Test: each of the five platforms has exactly one global `STRING` row. The query counts the rows with `machine_id IS NULL`, the name, and the platform in `platforms`.
  - Test: `CodesysIec`, `TwinCat3`, `S7_1200`, and `S7_1500` each have exactly one global `WSTRING` row. `S7_300` has no global `WSTRING` row.
  - Red step (pinning test): mutation in `migrations/0004_seed_plc_types.sql` that removes `'S7_300'` from the platforms of the row `5eed0000-0000-4000-8000-000000000010`. The `STRING` test fails for `S7_300`.
  - Check: `cargo test -p recipe-db --test storage seed`, then `cargo test -p recipe-db`. The tests need Docker.

## 2. Docs

- [x] 2.1 (CHANGELOG.md) [mechanical] Add one line under `[Unreleased]` `### Fixed`. The line says: `UpdateParameterDefinition` checks a changed type before the unit lock and the first `plc_type` insert (D1-1).
- [x] 2.2 (CHANGELOG.md) [sonnet] Check the line of task 2.1 against the ASD-STE100 rules of `.claude/rules/docs.md`.
  - Check: `python -I tools/ste-lint/ste-lint.py CHANGELOG.md` from Bash reports 0 hard violations.
  - Check: the line has at most 25 words, the active voice, and no semicolon.
  - Change: when a check fails, correct the line and run both checks again.

## Workflow follow-up

- `plan-eng-review` (Opus), then `openspec-update-change`.
- Approval of `tasks.md` by the owner before task 1.1.
- `code-review` (Opus) and a Codex second review (gpt-6-astra, effort high).
- `superpowers:verification-before-completion` with the clippy, test, and fmt output.
- `openspec-archive-change` after `cargo test` and `openspec validate type-check-before-insert --strict` pass.
