# Tasks

Marks after the scope:

- `[codex]`: Codex with the model `gpt-6.1-sol` does the task. It writes CI files, shell, the Python CI scripts, and their tests.
- `[sonnet]`: a Sonnet subagent with the effort `high` does the task. It writes Rust and TypeScript tests, or it runs the STE checks.
- `[mechanical]`: a Haiku subagent does the task. It writes a simple doc edit.
- `[orchestrator]`: the orchestrator does the task with the consent of the owner. It pushes the branch or starts a CI run.

Each task is one agent run. Each task starts with a failing test or a check that fails today. The report of each task gives the red step, the failure line, and the green step.

The owner decided Q1 and Q3 of `design.md` "Open Questions" on 2026-11-09. Tasks 2.1 and 2.2 use the decision of Q1 in design D3. Q2 stays open until the owner approves this file. Task 2.5 then sets ADR-0153 to "Accepted".

## Checks and waves

Rules of the checks:

- A Rust task checks only its own crate: `cargo test -p recipe-web --all-features --locked`, `cargo clippy -p recipe-web --all-targets --all-features --locked -- -D warnings`, and `cargo fmt --all --check`.
- A red task ends with a test that fails. Its check names the failing test. It runs no clippy.
- Two tasks run in parallel only when they change no common file.
- The orchestrator runs `cargo test --workspace --all-features --locked`, the workspace clippy, and fmt after waves 3 and 4.
- A Haiku task runs no `ste-lint.py`. Task 5.2 runs the STE checks of every changed doc.

| Task | Agent | File or crate | Wave | Needs |
|---|---|---|---|---|
| 1.1 | `[codex]` | `deploy/ci/check.sh` | 1 | none |
| 1.4 | `[mechanical]` | `CONTRIBUTING.md` | 1 | none |
| 2.1 | `[codex]` | `deploy/ci/test_migration_rewrite_forms.py` | 1 | none |
| 2.5 | `[mechanical]` | `docs/adr/0153-*.md`, `docs/adr/README.md` | 1 | Q2 |
| 3.1 | `[sonnet]` | `recipe-web` (`src/pages/xss_coverage_tests.rs`, `src/pages.rs`) | 1 | none |
| 4.1 | `[sonnet]` | `e2e/tests/export-content.spec.ts` | 1 | none |
| 1.2 | `[codex]` | `.github/workflows/ci.yml` | 2 | 1.1 |
| 2.2 | `[codex]` | `deploy/ci/test_migration_rewrite_forms.py` | 2 | 2.1 |
| 3.2 | `[sonnet]` | `recipe-web` (`src/pages/xss_tests.rs`, `src/pages/xss_coverage_tests.rs`) | 2 | 3.1 |
| 4.2 | `[sonnet]` | `e2e/compose/support/export-content.ts` | 2 | 4.1 |
| 1.3 | `[codex]` | `crates/recipe-server/src/config/serve.rs` (check only, no change) | 3 | 1.2 |
| 2.3 | `[codex]` | `deploy/ci/migration_rewrite.py` | 3 | 2.2 |
| 3.3 | `[sonnet]` | `recipe-web` (`src/pages/xss_tests.rs`, `src/pages/xss_coverage_tests.rs`) | 3 | 3.2 |
| 4.3 | `[sonnet]` | `e2e/compose/end-to-end.spec.ts` | 3 | 4.2 |
| 2.4 | `[codex]` | `migrations/` (check only, no change) | 4 | 2.3 |
| 5.1 | `[mechanical]` | `CHANGELOG.md` | 4 | 1.2, 2.3, 3.3, 4.3 |
| 4.4 | `[orchestrator]` | `.github/workflows/ci.yml` (job `compose`, dispatch only) | 5 | 1.2, 4.3 |
| 5.2 | `[sonnet]` | changed docs | 5 | 1.4, 2.5, 5.1 |
| 5.3 | `[sonnet]` | workspace | 6 | every other task |

## 1. Secure refusal on each pull request (D6-1, design D5)

- [x] 1.1 (deploy/ci) [codex] Add the structure check `test_secure_refusal_step` to `deploy/ci/check.sh`.
  - The check fails unless exactly one step of `ci.yml` runs `bash deploy/ci/rust-tests.sh secure`.
  - That step is in the job `test`. The job `test` has no `if:` and no `needs:`. The step has no `if:` and no `continue-on-error`.
  - Follow the pattern of `test_ci_scripts_job` in the same file.
  - Red: `bash deploy/ci/check.sh` fails today with the message of the new check, because only `compose` runs the step.
- [x] 1.2 (.github/workflows) [codex] Add a step to the job `test` after "Run tests". Name it "Check the production refusal of insecure cookies".
  - The step runs `bash deploy/ci/rust-tests.sh secure`.
  - Remove the three steps of the job `compose`. They install the toolchain, restore the cache `compose-config`, and run the secure step.
  - Check: `bash deploy/ci/check.sh` passes. `bash deploy/ci/rust-tests.sh secure` passes and prints `1 passed`.
  - Measure the time of `bash deploy/ci/rust-tests.sh secure` after a warm `cargo test --workspace --all-features --locked`. Record the time in the report.
  - Result 2026-11-09: the local run gives no build time. The first CI run of the job `test` on this branch gives it. CI run 42605757381 gives 61 s for the secure step and 450 s for the job `test`.
- [x] 1.3 (crates/recipe-server) [codex] Show that only the new step finds a production build with the dev flag. Change no committed file.
  - In the work tree, set `const DEV_BUILD: bool = true;` in `crates/recipe-server/src/config/serve.rs` line 52.
  - Check: `cargo test --workspace --all-features --locked` passes with the change.
  - Red: `bash deploy/ci/rust-tests.sh secure` exits with a code other than 0.
  - Undo the change. Check: `git diff --exit-code crates/recipe-server` exits 0, and the step passes again.
- [x] 1.4 (CONTRIBUTING.md) [mechanical] Add `bash deploy/ci/rust-tests.sh secure` to the block of "Local checks" after the `cargo test` line.
  - Add one sentence after the block. The command tests the refusal of `session.secure=false` in a build without the feature `dev` (ADR-0062 (b)).
  - Red: `grep -c 'rust-tests.sh secure' CONTRIBUTING.md` prints 0 today. Check: it prints 1.

## 2. Rewrite form `add column of a constrained domain` (D4-1, ADR-0153, design D3)

- [x] 2.1 (deploy/ci) [codex] Add red cases of ADR-0153 (a) to `test_migration_rewrite_forms.py`. Each case expects one line `file:line: add column of a constrained domain`.
  - The probe of the review: `CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0)` and `ALTER TABLE recipe ADD COLUMN weight pos_int` in one new file.
  - Variants: with a `DEFAULT 1`, with an array suffix, without the word `COLUMN`, and with `IF NOT EXISTS`.
  - Variants: a schema-qualified type, a quoted type name, and a domain inside a function body and inside an `EXECUTE` string.
  - A volatile column default on a constrained domain gives the two forms `volatile default` and `add column of a constrained domain`.
  - Body variant: one function body creates `pos_int` with a `CHECK` and then adds `weight pos_int` to `recipe`. Expect one line of the form.
  - In the same file, a later top-level `ALTER TABLE recipe ADD COLUMN w2 pos_int` gives no line. The body does not change the file state (ADR-0153 (f)).
  - Red: `python3 -I -B deploy/ci/test_migration_rewrite_forms.py` fails on each new case.
- [x] 2.2 (deploy/ci) [codex] Add the cases of ADR-0153 (b) to (d) and (i) to `test_migration_rewrite_forms.py`.
  - Fail cases: a domain with `NOT NULL`, a domain over a constrained domain, and a constrained domain of an earlier file.
  - Fail cases: `ALTER DOMAIN ... ADD CHECK ... NOT VALID` and `SET NOT NULL` in an earlier file, and a rename or `SET SCHEMA` in an earlier file.
  - Fail cases: `ALTER TYPE ... RENAME TO` and `ALTER TYPE ... SET SCHEMA` on a constrained domain in an earlier file. The new name gives the form.
  - Fail case: `DROP CONSTRAINT` on a constrained domain keeps the form (ADR-0153 (c)).
  - Fail cases of `volatile default`: a column without a `DEFAULT` of the type `row_key AS UUID DEFAULT gen_random_uuid()`.
  - Fail cases of `volatile default`: a domain over `row_key` without its own `DEFAULT`, and `ALTER DOMAIN ... SET DEFAULT gen_random_uuid()` in an earlier file.
  - Pass cases: a domain without a constraint, and a domain with only `NULL`, a `DEFAULT` that is not volatile, or `COLLATE`.
  - Pass case: a column of the type `row_key` with its own `DEFAULT` that is not volatile.
  - Pass case: a domain over `row_key` with such a `DEFAULT`.
  - Pass cases: a `DEFAULT` with `IS NOT NULL`, and a column on a table of the same file.
  - Pass case: a domain after `DROP DOMAIN` and a new `CREATE DOMAIN` without a constraint.
  - Red: the test file fails on each fail case of an earlier file.
- [x] 2.3 (deploy/ci) [codex] Add the form to `deploy/ci/migration_rewrite.py` as design D3 says.
  - Add the sets `constrained` and `volatile_domains` to `check_files`. Pass them to `check_statement`, `other_forms`, `alter_table`, `forbidden_forms`, and `body_forms`.
  - In `body_forms`, copy the two sets once for each body. Pass the copies through each statement and each `EXECUTE` string of that body.
  - Handle `ALTER TYPE ... RENAME TO` and `SET SCHEMA` for the two sets.
  - Add the form name and the domain default rule to the docstring list.
  - Check: `python3 -I -B deploy/ci/test_migration_rewrite.py` and `python3 -I -B deploy/ci/test_migration_rewrite_forms.py` pass.
- [x] 2.4 (migrations) [codex] Check the 11 shipped migrations with the new script.
  - Red: a temporary copy of `migrations/` with the probe of task 2.1 as `0012_probe.sql` gives exit code 1. The output names the form. Remove the copy.
  - Check: `python3 -I -B deploy/ci/migration_rewrite.py migrations` exits 0 and prints no line. `bash deploy/ci/check.sh` passes.
  - Probe: run on the image `postgres:16.15-bookworm` of `deploy/compose.yaml`. Create a table `recipe` with 1000 rows.
  - Probe: for each case, record `pg_class.relfilenode` of `recipe`.
  - Probe: read it before and after `ALTER TABLE recipe ADD COLUMN ...`.
  - Probe cases: a domain with `CHECK`, a domain with `NOT NULL`, a domain with a volatile `DEFAULT`.
  - Probe cases: a domain with a stable `DEFAULT`, and an array of a constrained domain.
  - Expected by ADR-0153: the first three cases change `relfilenode`. The stable default and the array case keep it.
  - Record each result in the task notes. A case that differs from ADR-0153 stops this task and goes to the orchestrator.
  - Result 2026-11-09, red: the copy with `0012_probe.sql` gives exit code 1 and the line `0012_probe.sql:2: add column of a constrained domain`.
  - Result 2026-11-09, check: `migration_rewrite.py migrations` exits 0 and prints no line. `bash deploy/ci/check.sh` exits 0.
  - Result 2026-11-09, probe on `postgres:16.15-bookworm@sha256:d81f7b51697014367236d07ec58b54153fc28f828b72bf247fe5c7487520bcb0` (PostgreSQL 16.15), `relfilenode` before and after:
  - `CHECK`: 16399 to 16404, changed. `NOT NULL` with `DEFAULT 1`: 16408 to 16414, changed. Volatile `DEFAULT gen_random_uuid()`: 16418 to 16423, changed.
  - Stable `DEFAULT now()`: 16427 to 16427, kept. Array `pos_int[]`: 16432 to 16432, kept. Each case equals ADR-0153.
- [x] 2.5 (docs/adr) [mechanical] After the owner accepts ADR-0153 (Q2), set its Status line to "Accepted. Amends ADR-0136 (a) (the table of forms) and (b) (the exemptions), and ADR-0127 (p) (the form volatile default)".
  - Copy the same text into the row of ADR-0153 in `docs/adr/README.md`.
  - Red: `grep -c '^Status: Accepted' docs/adr/0153-add-column-of-a-constrained-domain.md` prints 0 today. Check: it prints 1, and the row equals the Status line.

## 3. XSS corpus through the password fields (D6-3, design D4)

- [x] 3.1 (recipe-web) [sonnet] Add the guard test of design D4 in `src/pages/xss_coverage_tests.rs`. Declare the module in `src/pages.rs` under `#[cfg(test)]`.
  - Use `test_support::source_walk::files_below` and `is_test_path`. Read the wire name from `rename`.
  - Scan each item with a `Deserialize` derive. Require `deny_unknown_fields`, also in a `#[serde(...)]` attribute with more items.
  - Fail on `rename_all`, `flatten`, and `alias`.
  - Fail on an item that is not a struct with named fields, and on a count mismatch.
  - Fill the table with each field that a corpus test covers today. Give each field without free text a reason entry.
  - Add a unit test of the parser for `rename`, doc comments, attributes with more items, and each refused key.
  - Red: the guard fails and names `LoginFields.password`, `PasswordFields.current_password`, and `PasswordFields.password`.
  - The report lists every other field without an entry. The orchestrator adds one task for each such text field.
- [x] 3.2 (recipe-web) [sonnet] Add `xss_corpus_password_field_of_post_login_is_escaped` to `src/pages/xss_tests.rs`. Add its entry to the guard table.
  - Send each payload of the corpus as `password` to `POST /login` with a fixed login. Use a new `Fixture` for each payload.
  - Assert that no answer holds the payload without escape. Use `assert_escaped` of the same file.
  - Red: before the entry, the guard of task 3.1 fails on `LoginFields.password`. Check: the guard names only the two fields of `PasswordFields`.
- [x] 3.3 (recipe-web) [sonnet] Add the corpus tests of `current_password` and `password` of `POST /me/password` to `src/pages/xss_tests.rs`. Add their entries to the guard table.
  - Send each payload in one field, with a valid value in the other field. Use a new `Fixture` with an open session for each payload.
  - A payload in `password` can be a valid new password. Then accept the successful change, and still check its answer.
  - Assert that no answer holds the payload without escape.
  - Red: the guard fails on the two fields of `PasswordFields`. Check: the guard passes, and `cargo test -p recipe-web --all-features --locked` passes.

## 4. Smoke check of the exported content (D6-2, design D6)

- [x] 4.1 (e2e) [sonnet] Add the pure test `e2e/tests/export-content.spec.ts`. Follow the pattern of `e2e/tests/jcs.spec.ts`.
  - The golden file `recipe_R-100_v3.json` gives no fault for its own values.
  - The values are `M12`, `R-100`, `3`, `U1`, `SPEED`, `INT`, `120`, `10`, and `250`.
  - A copy with one changed field gives a fault that names that field. Do this for each compared field.
  - A copy without the parameter `SPEED` gives a fault that names `SPEED`.
  - Red: `cd e2e && npx playwright test tests/export-content.spec.ts` fails, because `compose/support/export-content.ts` does not exist.
- [x] 4.2 (e2e) [sonnet] Add `exportContentFaults(document, expected)` in `e2e/compose/support/export-content.ts`, as design D6 says.
  - Check: `npx playwright test tests/export-content.spec.ts` passes. `E2E_PASSWORD=<12 to 128 characters> npm test` passes.
- [x] 4.3 (e2e) [sonnet] Call `exportContentFaults` in the export test of `e2e/compose/end-to-end.spec.ts` after the hash check.
  - Pass `run.machineCode`, `run.reference`, the version number `1`, the unit `U1`, and the values of `PARAMETER`. Expect an empty list.
  - Red: `grep -c exportContentFaults e2e/compose/end-to-end.spec.ts` prints 0 today.
  - Check: `npx playwright test --config playwright.compose.config.ts --list` lists the export test without an error.
  - The config stops without `E2E_ADMIN_PASSWORD` and `E2E_APPROVER_PASSWORD`. Set both to dummy values of 12 or more characters for the check.
- [x] 4.4 (.github/workflows) [orchestrator] Dispatch the job `compose` on the branch `fix/test-gates-round-2` (ADR-0094 (d)).
  - The owner permits the push of the branch first.
  - Run `gh workflow run ci.yml --ref fix/test-gates-round-2`. Then read the run with `gh run watch`.
  - Check: the job `compose` passes, and the step "Run the end-to-end smoke suite" passes. The report gives the run URL.
  - A red run goes to an Opus agent for the diagnosis and the fix.
  - Result 2026-11-09: CI run https://github.com/rtmachines/editor-web-cc/actions/runs/42605757381 is green in all 18 jobs.

## 5. Changelog and gates

- [x] 5.1 (CHANGELOG.md) [mechanical] Add the `[Unreleased]` entry of this change.
  - It names the secure step in the job `test` and the form `add column of a constrained domain` (ADR-0153).
  - It also names the corpus tests of the password fields, the guard, and the content check of the smoke suite.
  - Red: `grep -c 'ADR-0153' CHANGELOG.md` prints 0 today. Check: it prints 1 or more.
- [x] 5.2 (docs) [sonnet] Run the STE checks of every changed doc. Fix only the form.
  - Files: ADR-0153, ADR-0136, `docs/adr/README.md`, `data-model.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, and the artifacts of this change.
  - Run `python -I tools/ste-lint/ste-lint.py` on each file. Check the sentence length, the active voice, no semicolons, and one word for one meaning.
  - Check: `ste-lint.py` reports 0 hard violations on each file. The report pastes the last line of each run.
- [x] 5.3 (openspec) [sonnet] Run the gates of the change.
  - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked`, and `cargo fmt --all --check`.
  - `openspec validate test-gates-round-2 --strict` and `openspec validate --all --strict`.
  - `cargo deny --locked check`.
  - `cargo llvm-cov --workspace --all-features --locked --json --summary-only --output-path target/llvm-cov-summary.json`, then `python3 -I -B tools/coverage-gate.py target/llvm-cov-summary.json`.
  - `bash deploy/ci/check.sh` and `python3 -I -B deploy/ci/migration_rewrite.py migrations`.
  - `python3 -I -B deploy/ci/test_migration_rewrite.py` and `python3 -I -B deploy/ci/test_migration_rewrite_forms.py`.
  - `cd e2e && npx playwright test tests/export-content.spec.ts`.
  - `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'`.
  - Check: every command passes. The report pastes the last lines of each output.
  - Result 2026-11-09: every gate passes. Tests give 8290 passed and 0 failed. Each crate has at least 95 % line coverage. ste-lint gives 0 hard violations. The local e2e run with a random `E2E_PASSWORD` gives 14 passed. The checks `rust-tests.sh secure`, `cargo doc`, and `test_coverage_gate.py` of `CONTRIBUTING.md` also pass.

## Workflow follow-up

- Review the plan with `plan-eng-review` (Opus) and apply the findings with `openspec-update-change`.
- After the code tasks, run `code-review` (Opus) and the Codex second review (gpt-6-astra, effort high).
  - Result 2026-11-09: three review rounds of Opus and Codex found 3 HIGH, 7 MEDIUM, and 9 LOW. Each finding has a fix and a test.
- Archive the change with `openspec-archive-change` after `cargo test` and `openspec validate` pass.
