# Tasks

Each task is for one agent. Codex `gpt-6.1-sol` implements each task in `deploy/ci/`. Haiku does each `[mechanical]` task. Opus writes the content of each other doc task. A Sonnet agent runs the `ste-lint.py` step of each doc task and fixes only the form.

Each test task first writes a failing test and runs it (RED). Then it writes the change that makes the test pass (GREEN). Run each Python suite from `deploy/ci`, for example `cd deploy/ci && python3 -m unittest test_restore_switch`.

`deploy/ci/check.sh` runs each test file with `python3 -I`, so a test loads a helper of another test file with `importlib.util.spec_from_file_location` (design.md D3).

The letters in parentheses after ADR-0175 name its items.

## Run order

1. Task 1.1 runs when the owner accepts ADR-0175 with this file. No other task starts before it.
2. Task 2.1 runs before task 3.1, because both edit `deploy/ci/test_restore_switch.py`.
3. Task 2.2 runs before task 3.2, because both edit `deploy/ci/test_pitr_recovery.py`.
4. Tasks 2.1 and 2.2 can run in parallel. Tasks 3.1, 3.2, and 3.3 edit `docs/ops/runbook.md` and run in their order.
5. Tasks 4.1 and 4.3 can run in parallel with groups 2 and 3. Their files are not in those groups.
6. Task 4.2 runs after task 3.3. Its check `bash deploy/ci/check.sh` runs each `deploy/ci/test_*.py`, and groups 2 and 3 leave RED tests there between their steps.
7. Task 5.1 runs last. It is the gate of the change.

## 1. Acceptance

- [x] 1.1 (docs/adr) [mechanical] When the owner accepts ADR-0175, make these edits:
  - Set the status of ADR-0175 to "Accepted".
  - In `docs/adr/README.md`, change "Proposed" to "Accepted" in the index row of ADR-0175. Delete " (proposed)" from the entry "Also 0175" of the decision map.
  - Delete the mark " (proposed)" from the note "Amended by ADR-0175" of ADR-0103 and ADR-0106.
  - Check: `grep -rn '0175 (proposed)' docs/adr` prints nothing, and `ste-lint.py` on the four files reports 0 hard violations.

## 2. CI scripts

- [x] 2.1 (deploy/ci) First change the tests of the step `restore` in `deploy/ci/test_restore_switch.py`:
  - `test_restore_follows_the_order_of_adr_0103_b` (line 276) expects `compose rm -s -f app backup`.
  - `test_restore_runs_the_steps_in_order` (line 618) expects `compose --project-name recipe-ci-1-1 .* rm -s -f app backup`.
  - A new test: the function `restore` holds no `stop app backup`.
  - Add the stub case `*" compose "*" rm -s -f app backup"*) exit "${STUB_REMOVE_APP_STATUS:-0}" ;;` to the docker stub, before the case `*" compose "*" stop "*)` (line 453).
  - A new fault test in `RestoreBehaviorTests` sets `STUB_REMOVE_APP_STATUS` to 1. The run exits with 1, stderr holds `CI: restore: cannot delete app and backup.`, and the log holds no `volume create`. The model is `test_pitr_recovery.py` line 1130.
  - Run `python3 -m unittest test_restore_switch` and see the new and changed tests fail.
  - Then change line 146 and line 147 of `deploy/ci/restore-switch.sh` (ADR-0175 (d)). The log line is `restore: delete app and backup`. The fault line is `restore: cannot delete app and backup.`
  - Check: `python3 -m unittest test_restore_switch` passes.
- [x] 2.2 (deploy/ci) First change the tests of the step `crash` in `deploy/ci/test_pitr_recovery.py`:
  - The order tests at line 1036, line 1071, and line 1079 expect `rm -s -f app backup`.
  - The stub case at line 2268 matches `*" rm -s -f app backup")` and reads `STUB_REMOVE_APP_STATUS`.
  - `test_crash_fails_when_the_services_do_not_stop` (line 1130) sets `STUB_REMOVE_APP_STATUS` and expects `CI: crash: cannot delete app and backup.`
  - The two precondition tests at line 1123 and line 1128 change the marker `"stop "` to `"rm -s -f app backup"`.
  - Run `python3 -m unittest test_pitr_recovery`. The order tests, the stub case, and the fault test fail (RED).
  - The two precondition tests are a guard. They pass before and after the script change.
  - Then change line 476 and line 477 of `deploy/ci/pitr-recovery.sh` (ADR-0175 (c)). The log line ends with `Delete app and backup`. The fault line is `crash: cannot delete app and backup.`
  - Check: `python3 -m unittest test_pitr_recovery` passes.

## 3. Runbook

- [x] 3.1 (docs/ops) First add one helper to `deploy/ci/test_upgrade_runbook.py`. It returns the inline code spans of a text:
  - The helper reads one line at a time and skips each line of a fenced block. A span is the text between two single backticks on one line.
  - Add three unit tests of the helper next to `RunbookHeadingSectionTests` (line 72). One test gives two spans on one line. One test gives a span inside a fenced block, which the helper skips. One test gives a line without a backtick.
  - Then add the class `RunbookRestoreDeleteTests` to `deploy/ci/test_restore_switch.py`:
  - Load `runbook_heading_section`, `runbook_bash_blocks`, `runbook_step`, and the new helper from `test_upgrade_runbook.py` with `importlib.util.spec_from_file_location`.
  - The bash block of step 2 of "Restore into production" is the one line `docker compose rm -s -f app backup`.
  - That line without the prefix `docker ` occurs in the function `restore` of `deploy/ci/restore-switch.sh`.
  - `runbook_step("Restore into production", "4. Fill the new volume.")` holds the exact span `docker compose up -d --no-deps app` twice. It holds the exact span `docker compose up -d --no-deps backup` once. The span with `--force-recreate` does not count.
  - The step holds no text `as in "Stop, start, and restart"`.
  - No inline code span and no bash line of the section holds `docker compose start app` or `docker compose start backup`.
  - Run `python3 -m unittest test_restore_switch` and see the step 2 test and the step 4 test fail.
  - Then change step 2 of "Restore into production" in `docs/ops/runbook.md` (ADR-0175 (a)). The step says "Delete the containers of `app` and `backup`". It keeps its two reasons and adds the reason of the delete in one sentence.
  - Before the bash block of step 2, add one optional sentence with the inline command `docker compose logs --no-color app backup > <file>`. The sentence says that `rm` deletes the logs of the two containers (design.md D4).
  - In step 4, drop each "as in "Stop, start, and restart"" for `app` and `backup` (ADR-0175 (e)).
    - The plain rollback path names `docker compose up -d --no-deps app`. It keeps its one command `docker compose up -d --no-deps --force-recreate backup`, so `backup` starts once.
    - The path "In every other case" names `docker compose up -d --no-deps app`, then `docker compose up -d --no-deps backup`.
  - Check two mutations once in the working tree. Undo each edit after its run:
    1. Change one command of step 4 to `docker compose start app`. The section test fails.
    2. Revert only the line "In every other case" of step 4 to its old text. The step 4 test fails.
  - Check: `python3 -m unittest test_restore_switch test_upgrade_runbook` passes, and `ste-lint.py docs/ops/runbook.md` reports 0 hard violations.
- [x] 3.2 (docs/ops) First add the class `RunbookRecoveryDeleteTests` to `deploy/ci/test_pitr_recovery.py`. Load the helpers of task 3.1 from `test_upgrade_runbook.py` in the same way:
  - The bash block of step 1 of "Recover from the WAL archive" is the one line `docker compose rm -s -f app backup`.
  - That line without the prefix `docker ` occurs in `shell_function("crash")` (line 58), not only in the file.
  - No inline code span and no bash line of the section holds `docker compose start app` or `docker compose start backup`.
  - Run `python3 -m unittest test_pitr_recovery` and see the step 1 test fail.
  - Then change step 1 of "Recover from the WAL archive" in `docs/ops/runbook.md` (ADR-0175 (b)). The step says "Delete the containers of `app` and `backup`".
  - Keep the reason "After this point, `app` writes no audit entry". Add only the reason of the delete of ADR-0175. Add the optional log copy of task 3.1.
  - Check the mutation once: add the inline span `docker compose start backup` to step 7 in the working tree. The section test fails. Undo the edit.
  - Check: `python3 -m unittest test_pitr_recovery` passes, and `ste-lint.py docs/ops/runbook.md` reports 0 hard violations.
- [x] 3.3 (docs/ops) First add a test to `deploy/ci/test_upgrade_runbook.py`. It reads `runbook_heading_section("After a host reboot", 3)`:
  - The section links `[Restore into production](#restore-into-production)` and `[Recover from the WAL archive](#recover-from-the-wal-archive)`.
  - The test reads the section text without backticks and joins each run of white space into one space. That text holds these sentences:
    1. `From step 2 of [Restore into production](#restore-into-production) until the procedure starts app and backup again, the two services can have no container.`
    2. `The same holds from step 1 of [Recover from the WAL archive](#recover-from-the-wal-archive).`
    3. `In that window, proxy cannot become healthy, because its healthcheck calls http://app:9000/healthz.`
    4. `Skip step 4 in that window.`
    5. `Step 4 runs docker compose exec app.`
    6. `In that window, step 1 waits only for db and walarchive, and only for a service that docker compose ps -a lists.`
    7. `The switch deletes the containers of db and walarchive, and later steps create them again.`
    8. `Until step 8 of the recovery deletes the container of db, skip this wait when db did not run at step 1 of the recovery.`
    9. `Such a db cannot become healthy, and the healthcheck of walarchive needs a running db.`
    10. `After step 1 of this section, continue the procedure at the step where it stopped.`
    11. `Three cases need an action before you continue:`
    12. `If the reboot came between step 4 and step 8 of the recovery, walarchive runs again.`
    13. `Run step 4 of the recovery again.`
    14. `If the reboot stopped step 4 of the restore or step 6 of the recovery, the new volume is not complete.`
    15. `Delete it with docker volume rm <new volume>.`
    16. `Then start again at step 3 of the restore or at step 5 of the recovery.`
    17. `If the reboot stopped step 7 of the restore or step 10 of the recovery, the shell lost the variable target.`
    18. `Set target to the directory that sub-step 1 made.`
    19. `If sub-step 1 made no directory, run sub-step 1 again.`
  - That text also holds the phrase `until the procedure starts app and backup again`.
  - Sentence 6 comes before sentence 10, and sentence 10 comes before sentence 13. The wait of step 1 comes before the new run of step 4 of the recovery.
  - That text does not hold the two sentences of the old rule. One sentence let step 1 skip each wait when `db` did not run at step 1 of the recovery. The other one said that `db` and `walarchive` never become `healthy`. The test file holds their exact text.
  - That text also does not hold the old bound of the skip. That sentence ended the skip at the start of step 8 of the recovery, before the drill check. The test file holds its exact text.
  - That text does not hold the old order either. One old sentence ran step 4 of the recovery again before the wait. The other one continued the procedure without a reference to step 1. The test file holds their exact text.
  - Run `python3 -m unittest test_upgrade_runbook` and see the test fail.
  - Then put three paragraphs and one list into the section "After a host reboot" of `docs/ops/runbook.md` (ADR-0175 (h)). They replace the paragraph that starts "From step 2 of". The model is the paragraph of the failed rollback restore at line 474:
    - The first paragraph writes both section names as the links `[Restore into production](#restore-into-production)` and `[Recover from the WAL archive](#recover-from-the-wal-archive)`. The test requires these links.
    - The first paragraph holds sentences 1 to 5 of the test. Write no line number of `compose.yaml`.
    - The second paragraph holds sentences 6 to 9 of the test.
    - The third paragraph holds sentences 10 and 11 of the test. A bulleted list of three cases follows it.
    - The first case holds sentences 12 and 13. Step 4 of the recovery only stops `walarchive`, and a reboot starts it again (ADR-0175 (h), (k)).
    - The second case holds sentences 14 to 16. The fill refuses a volume that is not empty (ADR-0175 (h)).
    - The third case holds sentences 17 to 19. A reboot inside step 7 of the restore or step 10 of the recovery loses the shell variable `target` (ADR-0175 (h)).
  - Check: `python3 -m unittest test_upgrade_runbook` passes, and `ste-lint.py docs/ops/runbook.md` reports 0 hard violations.

## 4. Other documents

- [x] 4.1 (docs/architecture) [mechanical] In `docs/architecture/reliability.md` §7.7, change "stops `app` and `backup`" to "deletes the containers of `app` and `backup`" at line 795 and line 855. Add "ADR-0175" to the citation of each bullet. Check: `grep -n 'stops `app` and `backup`' docs/architecture/reliability.md` prints nothing, and `ste-lint.py` on the file reports 0 hard violations.
- [x] 4.2 (.github) [mechanical] In `.github/workflows/ci.yml` line 598, change "stop app and backup" to "delete app and backup". Check: `bash deploy/ci/check.sh` passes.
- [x] 4.3 (CHANGELOG.md) [mechanical] Add one line under `### Fixed` of `[Unreleased]`. The line says that a restore into production and a recovery from the WAL archive delete the containers of `app` and `backup`. It cites ADR-0175. Check: `ste-lint.py CHANGELOG.md` reports 0 hard violations.

## 5. Gate

- [ ] 5.1 (deploy/ci) Run the local checks and the CI dispatch:
  1. `bash deploy/ci/check.sh` passes. It runs each `deploy/ci/test_*.py`.
  2. `openspec validate restore-delete-app-backup --strict` reports valid.
  3. The `ste-lint.py` command of `CLAUDE.md` reports 0 hard violations. Run it with `globstar` on `openspec/changes/restore-delete-app-backup/**/*.md` too.
  4. `grep -rn 'stop app backup' docs/architecture docs/ops docs/adr deploy .github openspec/specs` prints only the ADR lines that quote the old command.
  5. On the order of the owner, push the branch with `git push -u origin fix/restore-delete-app-backup`. The dispatch runs the commit of the remote branch.
  6. Run `gh workflow run CI --ref fix/restore-delete-app-backup`. The jobs `restore-switch` and `pitr-recovery` do not run on a pull request (ADR-0094).
  7. Both jobs pass in that run. Record the URL of the run in the pull request.

## Workflow follow-up

- After the gate, run `code-review`, fix each CRITICAL, HIGH, and MEDIUM finding, and run `openspec-archive-change`.
