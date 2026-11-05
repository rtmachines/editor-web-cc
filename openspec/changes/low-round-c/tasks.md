# Tasks

Each task is for one agent. The mark after the scope names the agent: `[sonnet]` (effort high), `[hard]` (Sonnet, effort high, a difficult Rust task), or `[mechanical]` (Haiku). `design.md` D7 gives the run order. A `[mechanical]` agent runs no `ste-lint.py`. Task 7.1 runs the STE checks.

A pinning test shows its red step on a temporary mutation of the code under test, a plausible bug. The agent sees the test fail, reverts the mutation, and reports both runs. A wrong expected value in the test is not a red step.

## 1. Decision records and normative documents

- [x] 1.1 (docs/adr) [mechanical] Set the Status of ADR-0130 to ADR-0133 to "Accepted". Add the four rows to the index of `docs/adr/README.md` and to the decision map. Add an "Amended by" note and the matching index Status to ADR-0035 (j) (by ADR-0130), ADR-0041 (g) (by ADR-0132), and ADR-0073 (d) (by ADR-0133). ADR-0131 amends no ADR. Check: `grep -c "^Status: Accepted" docs/adr/013[0-3]-*.md` gives 1 for each file, and `git diff --stat -- docs/adr/0035-*.md docs/adr/0041-*.md docs/adr/0073-*.md docs/adr/README.md` names these four files.
- [x] 1.2 (docs/architecture) [mechanical] Apply the rows of `design.md` D6 for `domain-model.md` §5, `data-model.md` §6 (rows `UnlockLogin` and `ChangeOwnPassword`), and `api.md` §7.3. Check: `grep -n "ADR-0130" docs/architecture/domain-model.md docs/architecture/data-model.md` finds a line in each file. `grep -n "ADR-0132" docs/architecture/data-model.md` and `grep -n "ADR-0131" docs/architecture/api.md` each find one line. `grep -n "the use case returns the storage error" docs/architecture/data-model.md` finds no line.
- [x] 1.3 (docs/architecture) [sonnet] Apply the rows of `design.md` D6 for `reliability.md` §1.1, §2.4, §3.7, §7.8, and §8. For the three code shapes of D2-3, find the ADR that added each with `git log -S` and cite it. Check: `grep -n "MissingBasicCredentials" docs/architecture/reliability.md` finds a line in §1.1, and `grep -n "ADR-0133" docs/architecture/reliability.md` finds the lines of §2.4, §3.7, §7.8, and §8.
- [x] 1.4 (docs/architecture) [mechanical] Apply the row of `design.md` D6 for `design-system.md` §1. Name sections 10.12, 10.13, and 10.14 and the multi-line field of section 10.5 (ADR-0112). Check: `grep -n "10.12" docs/architecture/design-system.md` finds the line of §1.
- [x] 1.5 (project rules) [mechanical] Apply the rows of `design.md` D6 for `CLAUDE.md`, `.claude/rules/rust.md`, and `.claude/rules/web.md`. Add one sentence to each rule. The sentence cites the ADR. Change no other line. Check: `grep -n "ADR-0130" CLAUDE.md .claude/rules/rust.md` finds a line in each file, and `grep -n "ADR-0132" .claude/rules/web.md` finds one line.

## 2. recipe-app

- [ ] 2.1 (recipe-app) [sonnet] D1-2. Write a table test that pins the requirement, the name, and the command flag of every `Operation`. It passes. Change the requirement of one operation in the code, see the test fail, and revert. Then apply `design.md` D5 item 1. Check: `cargo test -p recipe-app operation` passes, no hand-written function in `auth/operation/` has more than 50 lines, and `cargo doc -p recipe-app --no-deps` shows the doc comment of each variant.
- [x] 2.2 (recipe-app) [sonnet] D1-3. Before the refactor, disable the check of the last Administrator in `identity/admin_change.rs`, see the tests of `SetUserRole` and `DeactivateUser` fail, and revert. Then apply `design.md` D5 item 2. Check: `cargo test -p recipe-app identity` passes, and `run_admin_change` has fewer than 50 lines.
- [ ] 2.3 (recipe-app) [hard] D1-4. Write the test of `design.md` D1 item 3 with a security sink that fails after the commit. Write a second test with a unit of work whose commit fails. In that test, the result is `Err` and the sink gets no call. The attempt rows stay. Run them and see the first test fail on the `Err` result. Then apply `design.md` D1 item 1 and rewrite the doc comment of `execute`. Check: `cargo test -p recipe-app unlock` passes, and `cargo clippy -p recipe-app --all-targets -- -D warnings` passes.

## 3. recipe-db

- [ ] 3.1 (recipe-db) [sonnet] D4-3. Write a unit test that asserts `ORDER BY id FOR UPDATE` in the four statements of `design.md` D4. Run it and see it fail. Then apply D4 items 1 to 4. Run the behavior tests that `design.md` names under "What already exists" by name before and after the change. Check: `cargo test -p recipe-db` passes in full, the five behavior tests pass, and `grants.expected` does not change. The test needs Docker.
- [x] 3.2 (recipe-db) [sonnet] D6-8. In `tests/storage/reset_password.rs`, assert the code `E-IDN-003` of the failed Basic check of the old password. For the red step, break the SQL of the Basic check so it gives a storage error. See the new assertion fail where `is_err()` passed, and revert. Check: `cargo test -p recipe-db reset_password` passes. The test needs Docker.

## 4. recipe-server

- [ ] 4.1 (recipe-server) [hard] D2-2. Write the unit tests of `design.md` D2 item 6:
  1. A reader that blocks gives `Unreadable` after the limit.
  2. A second call while the first call blocks past the limit gives `Unreadable` at once and starts no second reader.
  3. A call after the end of the first reader runs a new reader.
  4. Two concurrent fast calls of one path both give `Ok` and run two readers.
  5. A reader that ends at the limit leaves no mark, so the next call runs a new reader.

  Run them and see tests 1 and 2 fail. Then apply `design.md` D2 items 1 to 5 and add `parking_lot = { workspace = true }` to `crates/recipe-server/Cargo.toml`. Check: `cargo test -p recipe-server disk` passes, `cargo clippy -p recipe-server --all-targets -- -D warnings` passes, and `cargo deny check` passes.

## 5. recipe-web

- [x] 5.1 (recipe-web) [sonnet] D5-3. Write an HTTP test: `POST /logout` with a JSON body and the header token gives `422 E-WEB-008`, and the session stays open. Write a second test: the form of the logout button still ends the session. Run them and see the first test fail. Then apply `design.md` D5 item 4. Check: `cargo test -p recipe-web logout` passes.
- [x] 5.2 (recipe-web) [sonnet] D5-4. Change the test at `src/error/tests.rs` line 603 to assert the Polish sentence of `E-AUD-003`. Run it and see it fail. Then add `error-E-AUD-003` to both bundles. Check: `cargo test -p recipe-web error_mapping` passes.
- [ ] 5.3 (recipe-web, recipe-app) [hard] D5-5. After task 5.1. Write the HTTP tests of the three scenarios of the requirement "Field answer of the own password form". Add a test: a wrong current password and a new password of the breach list give one line `E-IDN-006` under `password` and no login attempt row. Use distinct marker passwords in each test. Assert that no answer body holds a marker and that each password input is empty, for the field answer and for `E-IDN-008`. Run them and see the field tests fail. Then apply `design.md` D3, also the input type in `identity/change_own_password.rs`. Check: `cargo test -p recipe-web password` passes, `cargo test -p recipe-web login` passes, and `cargo test -p recipe-app password` passes.
- [ ] 5.4 (recipe-web) [sonnet] D1-4. After task 2.3. Write the HTTP test of the scenario "Security event table rejects the unlock event". Run it and see it fail. Then apply `design.md` D1 item 2. Check: `cargo test -p recipe-web unlock` passes.
- [ ] 5.5 (recipe-web) [sonnet] D3-1. Write the HTTP tests of the two scenarios of the requirement "Token of the language switch with a session". They pass, because the code does not change. Change `Access::Lang => TokenKind::Either` to `TokenKind::Session` in `csrf.rs`, see the test with `csrf_pre` fail, and revert. Check: `cargo test -p recipe-web lang` passes.

## 6. Release notes

- [ ] 6.1 (CHANGELOG.md) [mechanical] Add lines to the section "Unreleased" for ADR-0130, ADR-0132, and ADR-0133. Add lines for the ordered deletes, the logout form, and the text of `E-AUD-003`. Do not change the sections of earlier releases. Check: `git diff -U0 -- CHANGELOG.md | grep -c "^-[^-]"` gives 0.

## 7. Checks

- [ ] 7.1 (docs) [sonnet] Run `ste-lint.py` on the scope of ADR-0023 and on the files of this change, also the four new ADRs. Fix each hard violation without a change of meaning. Check: `ste-lint.py` reports 0 hard violations, and `openspec validate low-round-c --strict` passes.
- [ ] 7.2 (workspace) [sonnet] Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked`, and `cargo doc --no-deps` with `-D warnings`. Check: the four commands pass, and the output is in the report.

## Workflow follow-up

- Code review with the skill `code-review` (Opus) and a Codex second review. Fix every CRITICAL and HIGH finding.
- `superpowers:verification-before-completion`, then `openspec-archive-change` after `cargo test` and `openspec validate` pass.
