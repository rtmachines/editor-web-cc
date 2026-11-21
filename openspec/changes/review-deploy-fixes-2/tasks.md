# Tasks

Each task is one Codex run. Read `proposal.md`, `design.md`, and the files of the task first. Write the red test first, see it fail, and then apply the fix. Do not edit files outside the paths of the task.

A task that changes `.github/workflows/ci.yml` or a file `deploy/db/check-*.sql` reads the rest of that file first (fix plan §1, "Coverage gaps"). It reports each fault that it finds there and does not fix it.

## 1. V-02 proof and runbook text

- [x] 1.1 (deploy/ci) Codex applies design D1 with `EARLY_END_REFUSAL_REQUIRED=0` and an empty `EARLY_END_ERROR`. Files: `deploy/ci/pitr-recovery.sh`, `deploy/ci/test_pitr_recovery.py`, `.github/workflows/ci.yml`. In `ci.yml`, change only the name and the comment of the step `refusals`, as D1 says. Read the rest of `ci.yml` first. — red test: add seven tests to `test_pitr_recovery.py`.
  - `test_markers_record_the_record_before_marker_2` asserts the state file `early-end.state`. Its fixture names the segment `000000010000000000000006`.
  - `test_refusals_run_the_early_end_run_on_a_damaged_copy` asserts the order of the new commands and the inverted byte at L plus 20. It also asserts no write to the real `/wal` and no `PITR_EXPECTED_LSN`.
  - `test_refusals_invert_the_first_byte_when_the_header_crosses_a_page` uses an offset of 8176 in a page of 8192 bytes. It asserts that the inverted byte is at L.
  - `test_refusals_log_each_proof_value` asserts one line `refusals: early end: <name>=<value>` for each name of D1 step 7.
  - `test_refusals_accept_a_refusal_of_the_early_end_run` asserts class R.
  - `test_refusals_accept_a_completion_at_marker_1_only_when_not_required` asserts class S with the values `0` and `1`.
  - `test_refusals_fail_on_an_inconclusive_early_end_run` asserts class I.
  - Update `test_refusals_pass_path_runs_the_steps_in_order` and `test_markers_pass_path_runs_the_steps_in_order` for the new commands. Before the fix the step has no early-end run, so the new tests fail. — Check: `bash deploy/ci/check.sh`.
- [ ] 1.2 (openspec/changes/review-deploy-fixes-2/design.md) After task 1.1, run the proof. This task has four actors.
  1. The orchestrator asks the owner for the commit and the push of the branch. The owner gives each commit and each push.
  2. The orchestrator runs `gh workflow run ci.yml --ref fix/review-deploy-fixes-2` and then `gh run view <id> --log`.
  3. A Haiku agent fills "Proof result" of `design.md` from the lines `refusals: early end:` of that log and from the run data.
  4. A class I result goes to an Opus diagnosis and then back to task 1.1.
  - red test: none, because the task records a run. — Check: the job `pitr-recovery` of the dispatch passes, and each row of "Proof result" holds a value or `none`.
- [ ] 1.3 (deploy/ci) After task 1.2, set the final values of D1 from "Proof result" of `design.md`. Files: `deploy/ci/pitr-recovery.sh`, `deploy/ci/test_pitr_recovery.py`. For class R, set `EARLY_END_REFUSAL_REQUIRED=1` and write the recorded message into `EARLY_END_ERROR`. For class S, keep `0` and write the comment `# V-02: change B sets 1 with its fix of ADR-0106 (g) step 2.` — red test: add `test_early_end_requirement_matches_the_proof_result`. It holds the class and the message of "Proof result" as literals. It does not read `design.md`, because the archive moves that file. For class R, it asserts the value `1` and the message. For class S, it asserts the value `0` and the comment. Before the change, the test fails for both classes. — Check: `bash deploy/ci/check.sh`.
- [ ] 1.4 (docs/ops/runbook.md) After task 1.2, apply design D2 on line 1040. Use the text of the class in "Proof result" of `design.md`. Change no other line. No test pins the old text: `rg "checks the end of the archive" deploy crates tools` finds no match. — red test: none, because the file is a document. — Check: `python -I tools/ste-lint/ste-lint.py docs/ops/runbook.md` reports 0 hard violations, and `bash deploy/ci/check.sh` passes. `deploy/ci/test_reboot_windows.py` lines 193 to 232 parse step 6 of that runbook section.

## 2. CHANGELOG entry

- [ ] 2.1 (CHANGELOG.md) [mechanical] After tasks 1.1 to 1.4, add the entries of this change under `[Unreleased]`, as `reliability.md` §8 requires. `### Added` names the early-end run of the CI step `refusals`. `### Changed` names the text of runbook line 1040. Change A `review-code-fixes-2` also adds entries. The change that merges second rebases `CHANGELOG.md` and keeps both sets of entries. — red test: none, because the file is a document. — Check: the ste-lint command of task 3.2 reports 0 hard violations.

## 3. Integration checks

- [ ] 3.1 (.) Run `bash -n deploy/ci/pitr-recovery.sh` and `bash deploy/ci/check.sh`. Then the orchestrator asks the owner for the commit and the push, and dispatches `ci.yml` on the branch as in task 1.2. Check: both commands pass, and the jobs `ci-scripts` and `pitr-recovery` pass.
- [ ] 3.2 (.) Run `openspec validate review-deploy-fixes-2 --strict`, `openspec validate --specs --strict`, and `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md openspec/changes/review-deploy-fixes-2/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'`. Check: both validations pass, and ste-lint reports 0 hard violations.

## Order and shared files

- Tasks 1.1 and 1.3 both edit `deploy/ci/pitr-recovery.sh` and `deploy/ci/test_pitr_recovery.py`. Task 1.3 runs after task 1.2.
- Task 1.2 writes the section "Proof result" of `design.md`. Tasks 1.3 and 1.4 read it and run after task 1.2.
- Tasks 1.3 and 1.4 edit no common file. They can run in parallel.
- Task 2.1 runs after group 1. Group 3 runs last.
- Only task 1.1 changes `.github/workflows/ci.yml`. It reads the rest of that file first. No task changes a file `deploy/db/check-*.sql`.

## Workflow follow-up

- The orchestrator reads "Proof result" of `design.md` before the design of change B (fix plan §4 step 4).
- Change B amends ADR-0106 (p) with the fourth run, and fixes V-06 and V-07 ("Notes for change B" of `design.md`).
- Change B decides "Open points for the owner" item 1 and, for class S, the fix of V-02.
