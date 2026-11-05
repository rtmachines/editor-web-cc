# Tasks: test-gates-round-1

Each task names its agent. One task goes to one agent. The decisions D1 to D8 are in `design.md`.

## 1. ADR and documents

- [x] 1.1 (docs/adr) Write ADR-0127 with the decisions D1, D2, D5, D6, and D7 of `design.md`. Add its row to the index of `docs/adr/README.md`. Add the topic "Test gates" to the decision map. Check: `ste-lint.py` reports 0 hard violations on the ADR and the README. Agent: Opus writes, Sonnet runs the STE check.
- [x] 1.2 (docs/architecture) [mechanical] Change the coverage line and the row "Integration, storage" of `reliability.md` §6. Add the `PUBLIC` rows to `data-model.md` §5.3. Add the script name, the `DO` rule, and the list of exceptions to `data-model.md` §8, as ADR-0127 states. Check: `ste-lint.py` on both files. Agent: Haiku.
- [x] 1.3 (CONTRIBUTING.md) [mechanical] Change the coverage section. The gate counts production code. It fails on an inline test module and on a production file with a test file name. It also fails on a crate that is missing from the report. Check: `ste-lint.py` on `CONTRIBUTING.md`. Agent: Haiku.

## 2. Coverage gate

- [x] 2.1 (tools) Write `tools/test_coverage_gate.py` with `unittest` and temporary workspaces. First write regression tests of the current exits. Cover a crate below 80 % (exit 1), a bad report (exit 2), and a crate without a production file ("no code"). Then cover each rule of D1 and D2. Cover the attribute forms `pub(crate)`, doc comments, stacked attributes, and `any`, `all`, `not`. Cover `lib.rs` and `mod.rs`, a nested test directory, and `#[path]` (exit 2). Cover an inline module in a production file (exit 1, file and line) and in a test file (passes). Cover a production file `foo_tests.rs`, `foo-tests.rs`, `tests.rs`, and a file in a `tests/` directory under `src/` (exit 1 each). Cover a crate that is missing from the report (exit 1). Also cover the option `--source-only`, which runs the source scan without a report. Check: the regression tests pass on the current gate, and the new tests fail. Agent: Codex.
- [x] 2.2 (tools) Change `tools/coverage-gate.py` to D1 and D2. Add the option `--source-only`, and update the docstring. Check: `python3 -I -B tools/test_coverage_gate.py` passes. `python3 -I tools/coverage-gate.py --source-only` names the 64 inline modules and no other file. Agent: Codex.
- [x] 2.3 (.github/workflows) Change the install step of the job `coverage` to exactly `cargo-llvm-cov@0.6.24`. Add the step `python3 -I -B tools/test_coverage_gate.py` before the measurement (D7). Check: the workflow file parses as YAML, the pin has three version parts, and the step runs before `cargo llvm-cov`. Agent: Codex.

## 3. Inline test modules to files

Each task moves the modules as D3 states. A failing test exists before each task: `coverage-gate.py --source-only` names the modules of the task. Check for each task: the gate names no module of the task, and `cargo test -p <crate> --all-features --locked` reports the same number of passed tests as before.

- [x] 3.1 (recipe-domain) [mechanical] Move the 18 inline modules of the files directly in `src/plant/`. Agent: Haiku.
- [x] 3.2 (recipe-domain) [mechanical] Move the 7 inline modules of `src/plant/iec_literal/` and `src/plant/type_tree/`, and the 10 inline modules of `src/recipe/`. Agent: Haiku.
- [x] 3.3 (recipe-domain) [mechanical] Move the 15 inline modules of `src/audit/`, `src/identity/`, `src/export/`, `error_codes.rs`, `text.rs`, and `timestamp.rs`. Agent: Haiku.
- [x] 3.4 (recipe-app) [mechanical] Move the 4 inline modules. Agent: Haiku.
- [x] 3.5 (recipe-db) [mechanical] Move the 7 inline modules, `probe_text_tests` included. Agent: Haiku.
- [x] 3.6 (recipe-server) [mechanical] Move the inline module of `pg_env.rs`. Agent: Haiku.
- [x] 3.7 (recipe-web) [mechanical] Move the inline modules of `form_text.rs` and `pages/request.rs`. Check also: `--source-only` passes for the whole workspace. Agent: Haiku.

## 4. Route matrix headers

- [x] 4.1 (recipe-web) Write failing unit tests of a new `assert_common_headers(reply, path, what)` in the tests of `pages/test_helpers.rs`. A reply without `X-Request-Id`, with a version 4 id, or without one header of `api.md` §9 panics. A reply to a page path with `Cache-Control: public, max-age=...` panics. A complete reply to a page path and a reply to a `/static/` path with the static value pass. Then add the table `SECURITY_HEADERS` and the function (D4). `health/tests.rs` uses the table and drops its own list. Check: `cargo test -p recipe-web --all-features --locked`. Agent: Sonnet.
- [x] 4.2 (recipe-web) Call `assert_common_headers` in `assert_want` before the `match`. In `routes/matrix/tests.rs`, the factory `reply()` adds every header of the table and a version 7 `X-Request-Id`. Each `#[should_panic]` test gets an `expected` text that only its own assertion prints. Add two negative tests: a reply without one header and a reply with a version 4 id. Check: every cell of the route matrix and every test of `matrix/tests.rs` pass in `cargo test -p recipe-web --all-features --locked`. A cell that fails shows a defect. Stop and report it, and a new task fixes it. Agent: Sonnet.

## 5. Grant test and PUBLIC

- [x] 5.1 (recipe-db) Add `grant_difference` to `grants/checks.rs` and a failing test. As `recipe_migrator`, the test gives `PUBLIC` in a test database five rights, one after the other. The first right is `EXECUTE` on a new function without a `REVOKE`. The other four are `SELECT` on a table, `UPDATE` on a column, `USAGE` on a sequence, and `USAGE` on the schema `public`. For each right, `grant_difference` must name exactly that `PUBLIC:` line as granted but not in the file. The test also asserts that no `PUBLIC` line names `replication_state(...)`. Check: the test fails with the current queries. Agent: Sonnet.
- [x] 5.2 (recipe-db) Add the four `PUBLIC` queries of D5 to `actual_lines`. Add the current `PUBLIC:` lines to `grants.expected`. Check: the test of 5.1 and every grant test pass in `cargo test -p recipe-db --all-features --locked`. Agent: Sonnet.

## 6. Migration rewrite check

- [x] 6.1 (deploy/ci) Write `deploy/ci/test_migration_rewrite.py` with `unittest`. Add the fixtures of this list. Check: `python3 -I -B deploy/ci/test_migration_rewrite.py` fails. Agent: Codex.
  - One fixture for each form of D6, also a `DO` block with direct DDL and a `DO` block with `EXECUTE` (fail).
  - A table that an unconditional `CREATE TABLE` in the same file creates (passes). A table from an earlier file after `CREATE TABLE IF NOT EXISTS` (fails).
  - `now()` and literal defaults (pass). A `nextval()` default (fails).
  - `SET NOT NULL` after a `CHECK` in an earlier file that ran `VALIDATE CONSTRAINT` (passes), and after a `NOT VALID` check without validation (fails).
  - `SET NOT NULL` after a valid check that a later file drops (fails). `SET NOT NULL, DROP CONSTRAINT` in one statement (fails). A valid check after `RENAME CONSTRAINT` (passes). A check on another column or another table (fails).
  - `public.recipe`, `ALTER TABLE ONLY`, `ALTER TABLE IF EXISTS`, and `SET DATA TYPE` (fail). An `ALTER TABLE` with several actions.
  - A form inside a `$$` or `$fn$` function body (fails) and inside a comment (passes). An unterminated quote (exit 2).
  - An entry in `EXEMPT` that matches (passes) and an entry that matches no form (fails as a dead entry).
  - The shipped files `0001` to `0010` (pass).
- [x] 6.2 (deploy/ci) Write `deploy/ci/migration_rewrite.py` to D6. Check: the tests of 6.1 pass, and `python3 -I -B deploy/ci/migration_rewrite.py migrations` exits 0. Agent: Codex.
- [x] 6.3 (.github/workflows) Add the job `migration-rewrite` of D7 with the two steps. Check: the workflow file parses as YAML, the job has no `if`, and the job `migration-freeze` stays unchanged. Agent: Codex.

## 7. Measurement and verification

- [x] 7.1 (workspace) Run `cargo llvm-cov --workspace --all-features --locked --json --summary-only --output-path target/llvm-cov-summary.json` and then `python3 -I tools/coverage-gate.py target/llvm-cov-summary.json`. Record the percent of each crate in the PR body. A crate below 80 % stops the apply for a decision of the owner. Agent: Sonnet.
- [x] 7.2 (workspace) Run `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked`, `cargo fmt --all --check`, `openspec validate test-gates-round-1 --strict`, and the `ste-lint.py` command of `CLAUDE.md`. Check: each command passes. Agent: orchestrator.
- [x] 7.3 (GitHub settings) After the merge, prepare the `gh api` call that protects `main` (D7). It requires each job of `ci.yml` and `playwright.yml` that runs on a pull request. Show the exact list to the owner. Run the call only after the owner approves. Check: `gh api repos/{owner}/{repo}/branches/main/protection` returns the list. Agent: Codex prepares, the orchestrator runs.

## Workflow follow-up

- `code-review` (Opus) and a Codex second review (gpt-6-astra, effort high). Fix every CRITICAL and HIGH finding.
- `superpowers:verification-before-completion` with the clippy, test, and fmt output.
- `openspec-archive-change` after `cargo test` and `openspec validate` pass. Task 7.3 follows the merge.
