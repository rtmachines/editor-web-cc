# Tasks

Each task is for one agent. Codex `gpt-6.1-sol` implements each task in `deploy/ci/`. Haiku does each `[mechanical]` task. Opus writes the content of each runbook task. A Sonnet agent runs the `ste-lint.py` step of each doc task and changes only the form.

Each test task first writes a failing test and runs it (RED). Then it writes the change that makes the test pass (GREEN). Run each Python suite from `deploy/ci`, for example `cd deploy/ci && python3 -m unittest test_pitr_recovery`.

Tasks 1.1, 5.1, 5.2, and 5.3 are exempt from the failing test. They change documentation or a comment only. `ste-lint.py`, `bash deploy/ci/check.sh`, and the grep checks of task 6.1 check them.

`deploy/ci/check.sh` runs each test file with `python3 -I`, so a test loads a helper of another test file with `importlib.util.spec_from_file_location` (design.md D4).

Two rules of `crates/recipe-server/tests/deploy_values.rs` apply to each line of `deploy/ci/`, test files too:

- The test `deploy_values_ci_volume_deletions_stay_in_throwaway_scripts` refuses a line where the word `volume` is directly followed by the word `rm`. Split such a test text into two literals, as `test_upgrade_runbook.py` line 384 does.
- `deploy_values` refuses `|| exit 1` and `*) exit 1`. A stub case exits with `exit "${STUB_<NAME>_STATUS:-0}"`.

The letters in parentheses after ADR-0176 name its items. The tasks marked "(option A)" follow option A of design.md D5. The owner chose option A on 2026-11-16.

## Run order

1. Task 1.1 runs when the owner accepts ADR-0176. No other task starts before it.
2. Tasks 2.1 and 3.1 can run in parallel. Their files differ.
3. Task 2.1 runs before tasks 3.2, 4.1, 4.2, 4.3, and 4.4, because they use its helpers.
4. Task 3.1 runs before task 4.2, because both edit `deploy/ci/test_pitr_recovery.py`.
5. Tasks 2.1, 3.2, 4.1, 4.3, and 4.4 edit `deploy/ci/test_upgrade_runbook.py` and run in this order.
6. Tasks 4.1, 4.2, 4.3, and 4.4 edit `docs/ops/runbook.md` and run in their order. Task 4.4 names the windows of tasks 4.1 to 4.3.
7. Tasks 5.1 and 5.3 can run in parallel with groups 2 to 4. Their files are not in those groups.
8. Task 5.2 runs after task 4.4. Its check `bash deploy/ci/check.sh` runs each `deploy/ci/test_*.py`, and groups 2 to 4 leave RED tests there between their steps.
9. Task 6.1 runs last. It is the gate of the change.

## 1. Acceptance

- [x] 1.1 (docs/adr) [mechanical] (exempt: documentation only, see the header) When the owner accepts ADR-0176, make these edits:
  - Set the status of ADR-0176 to "Accepted".
  - Set the status of ADR-0175 to "Accepted. Amended by ADR-0176 (items (h), (i), (j), (k): delete `app` in the major upgrade, `walarchive` in the recovery, and `backup` in the rollback)".
  - Add "Amended by ADR-0176 (items (i) step 4 and (p) step 5: delete the container of `walarchive`)" to the Status line of ADR-0106.
  - (option A) Add "Amended by ADR-0176 (item (aj): delete the container of `backup` before command 1)" to the Status line of ADR-0174.
  - In `docs/adr/README.md`, add the index row of ADR-0176 after the row of ADR-0175. Add the same "Amended by" notes to the rows of ADR-0175, ADR-0106, and, with option A, ADR-0174.
  - The row of ADR-0106 (line 118) has no note of ADR-0175 now. Add both notes in this order: the note of ADR-0175 from the Status line of ADR-0106, then the note of ADR-0176.
  - Compare the status column of each changed row with the Status line of its ADR. Each "Amended by" note of the Status line occurs in the row.
  - In the decision map row "Deployment, migrations, backup, panic policy, health", add "Also 0176 (...)" after the entry "Also 0175".
  - Check: `grep -n '0176' docs/adr/README.md` prints the index row and the map entry. `ste-lint.py` on each changed file reports 0 hard violations.

## 2. Shared helpers

- [x] 2.1 (deploy/ci) First add unit tests to `deploy/ci/test_upgrade_runbook.py` for two helper changes:
  - `runbook_deleted_service_starts(section, services=("app", "backup"))` (line 123). A test gives `docker compose start walarchive` with `services=("walarchive",)` and expects one hit. The same text without the parameter gives no hit. The existing tests of `RunbookDeletedServiceStartsTests` (line 163) stay unchanged.
  - `runbook_step(title, start, level=3)` (line 523). A test reads a step of a heading of level 2 with a patched `runbook_heading_section`. A test without `level` still reads level 3.
  - Run `python3 -m unittest test_upgrade_runbook` and see the new tests fail.
  - A negative test gives `docker compose start proxy` with `services=("app", "backup", "walarchive")` and expects no hit.
  - Then add the two parameters. The helper already has a local list `services` (line 152 and line 156). Rename that list to `named`. Line 157 becomes `set(services).intersection(named)`.
  - Check: `python3 -m unittest test_upgrade_runbook test_restore_switch` passes. Task 3.1 can leave RED tests in `test_pitr_recovery`, so this check does not run that file.

## 3. CI scripts

- [x] 3.1 (deploy/ci) First change the tests of the step `crash` in `deploy/ci/test_pitr_recovery.py` (ADR-0176 (e)):
  - `test_crash_pass_path_runs_the_steps_in_order` (line 1046) expects `rm -s -f walarchive$`.
  - `test_crash_stops_the_writers_before_it_reads_the_position` (line 1074) expects `rm -s -f walarchive` after `kill -s SIGKILL db`.
  - Line 1139 and line 1157 use `rm -s -f walarchive`.
  - The stub case at line 2311 becomes `*" rm -s -f walarchive") exit "${STUB_REMOVE_WALARCHIVE_STATUS:-0}" ;;`.
  - Rename `test_crash_fails_when_walarchive_does_not_stop` (line 1149) to `test_crash_fails_when_walarchive_cannot_be_deleted`. It sets `STUB_REMOVE_WALARCHIVE_STATUS` to 1 and expects `CI: crash: cannot delete walarchive.`
  - A new test: `shell_function("crash")` holds no `stop walarchive`.
  - Run `python3 -m unittest test_pitr_recovery` and see the changed and new tests fail.
  - Then change line 484 of `deploy/ci/pitr-recovery.sh` to `compose rm -s -f walarchive || fail "crash: cannot delete walarchive."`. Change the comment of line 467 if it names the stop.
  - Check: `python3 -m unittest test_pitr_recovery` passes.
- [x] 3.2 (deploy/ci) (option A) First add a test to `deploy/ci/test_upgrade_runbook.py` (ADR-0176 (i)):
  - `shell_function("check_rollback_runbook")` holds the line `compose rm -s -f backup || fail "cannot delete backup before the runbook check"`.
  - The function holds no `compose stop backup`.
  - The function still holds `compose up -d --no-deps --wait --wait-timeout 180 backup` after the check.
  - Run `python3 -m unittest test_upgrade_runbook` and see the test fail.
  - Then change line 651 of `deploy/ci/upgrade.sh`. Change the comment of line 650 to "Delete backup to release the restore lock."
  - Check: `python3 -m unittest test_upgrade_runbook` passes, and `bash -n deploy/ci/upgrade.sh` exits with 0.

## 4. Runbook

- [x] 4.1 (docs/ops) First add the class `RunbookMajorUpgradeDeleteTests` to `deploy/ci/test_upgrade_runbook.py` (ADR-0176 (a), (b), (j)):
  - `runbook_step("Major upgrade", "1. Delete the container of `app`.", level=2)` has one bash block. The block is the one line `docker compose rm -s -f app`.
  - The same step holds the exact inline span `docker compose logs --no-color app > <file>`.
  - `runbook_heading_section("Major upgrade", 2)` holds the exact span `docker compose up -d --no-deps app` in the text "If a step before step 8 fails".
  - `runbook_deleted_service_starts` finds no start of `app` or `backup` in the section.
  - The section holds no span `docker compose stop app`.
  - Run `python3 -m unittest test_upgrade_runbook` and see the new tests fail.
  - Then change step 1 and the text "If a step before step 8 fails" of "Major upgrade" in `docs/ops/runbook.md` (design.md D2). Step 1 keeps the reason "The next dump then holds every change". Add the reason of the delete in one sentence and the optional log copy.
  - Check the mutation once: change the span of the failed path back to `docker compose start app` in the working tree. The section test fails. Undo the edit.
  - Check: `python3 -m unittest test_upgrade_runbook` passes, and `ste-lint.py docs/ops/runbook.md` reports 0 hard violations.
- [x] 4.2 (docs/ops) First add six tests to `RunbookRecoveryDeleteTests` (line 1973) of `deploy/ci/test_pitr_recovery.py` (ADR-0176 (d), (f), (j)):
  - The bash block of step 4 of "Recover from the WAL archive" is the one line `docker compose rm -s -f walarchive`.
  - That line without the prefix `docker ` occurs in `shell_function("crash")`.
  - Step 4 holds the exact inline span `docker compose logs --no-color walarchive > <file>`.
  - `runbook_step("Recover from the WAL archive", "7. Compare")` holds the exact span `docker compose up -d --no-deps walarchive`. It holds no text "Start `walarchive`, if you stopped it in step 4".
  - A tag order test covers a failed attempt with a `<recovery tag>` that differs from `<tag>`. It reads step 7 without backticks. These spans occur in this order:
    1. "change into the directory deploy/ of <tag>"
    2. "Set RECIPE_VERSION in .env to <tag>"
    3. "docker compose up -d --no-deps walarchive"
  - `runbook_step("Recover from the WAL archive", "8. Run the switch")` holds the span "This volume is a drill volume. Do not switch. End the attempt as in step 7." It holds no text "Stop. This volume is a drill volume."
  - Rename `test_section_has_no_start_app_or_start_backup_command` (line 1992) to `test_section_has_no_start_of_a_deleted_service`. It passes `("app", "backup", "walarchive")` to the helper.
  - Its message (line 1995) becomes "recovery section must use up -d --no-deps for app, backup, and walarchive after the delete".
  - The guard tests call the renamed test at line 2005 and line 2010. Change both calls to the new name.
  - Add a mutation case `docker compose start walarchive` to the guard test of line 1997.
  - Run `python3 -m unittest test_pitr_recovery` and see the step 4 tests, the step 7 test, the tag order test, and the step 8 test fail.
  - Then change step 4, step 7, and item 5 of step 8 of "Recover from the WAL archive" in `docs/ops/runbook.md` (design.md D2). Step 4 keeps the reason "The `.partial` file then stays stable". Step 7 uses the order of design.md D2.
  - Check the mutation once: add the span `docker compose start walarchive` to step 7 in the working tree. The section test fails. Undo the edit.
  - Check the mutation of the tag order test once: move the create of `walarchive` before the change of the directory in the working tree. The tag order test fails. Undo the edit.
  - Check: `python3 -m unittest test_pitr_recovery` passes, and `ste-lint.py docs/ops/runbook.md` reports 0 hard violations.
- [x] 4.3 (docs/ops) (option A) First change the rollback tests of `deploy/ci/test_upgrade_runbook.py` (ADR-0176 (i), (j)):
  - `BACKUP_STOP` (line 512) becomes `BACKUP_REMOVE = "docker compose rm -s -f backup"`. Lines 560, 589, and 605 use it.
  - `rollback_stop` (line 556) and line 594 use the start "1. Delete the container of `backup`."
  - A new constant `BACKUP_LOG = "docker compose logs --no-color backup > <file>"` joins the allowed set of `test_rollback_stop_raw_commands` (line 589).
  - A new test: the step holds the block `[BACKUP_LOG]` once, and that block comes before the block `[BACKUP_REMOVE, ROLLBACK_CHECK]`.
  - A new test: the block of command 1 starts with `BACKUP_REMOVE`. That line without the prefix `docker ` occurs in `shell_function("check_rollback_runbook")`.
  - A new test: `runbook_deleted_service_starts` finds no start of `app` or `backup` in `runbook_heading_section("Rollback", 3)`. This test is a guard. It passes before and after the runbook change.
  - Run `python3 -m unittest test_upgrade_runbook` and see the changed tests fail.
  - Then change step 1 of the restore of the `_premigrate` set in "Rollback" (line 1596) in `docs/ops/runbook.md` (design.md D2). Keep the order: `backup` before command 1, `app` after a pass. Keep the reason of the lock of `restore-volume`.
  - Put the log copy in its own bash block before the block of command 1. Do not use the word "Docker" in the prose of step 1. Write "the container engine", as line 1621 does.
  - Check the mutation once: put `docker compose stop backup` back as the first line of the block of command 1 in the working tree. `test_rollback_stop_raw_commands` (line 587) fails. Undo the edit.
  - Check: `python3 -m unittest test_upgrade_runbook` passes, and `ste-lint.py docs/ops/runbook.md` reports 0 hard violations.
- [x] 4.4 (docs/ops) First change `RunbookHostRebootTests` (line 364) of `deploy/ci/test_upgrade_runbook.py` (ADR-0176 (g), (h)):
  - `required_sentences` holds the sentences of design.md D3. Write "Delete it with docker volume" and " rm <new volume>." as two literals, as line 384 does now.
  - The test requires the links `[Major upgrade](#major-upgrade)` and `[Rollback](#rollback)` in addition to the two links of line 432.
  - `old_texts` gets these six texts:
    1. "If the reboot came between step 4 and step 8 of the recovery, walarchive runs again."
    2. "Run step 4 of the recovery again."
    3. "Three cases need an action before you continue:"
    4. "In that window, proxy cannot become healthy"
    5. "Skip step 4 in that window."
    6. "In that window, step 1 waits only for"
  - `test_recovery_restart_instruction_follows_the_wait` (line 455): marker 1 (line 458) becomes "In each window, step 1 waits only for". Marker 3 becomes the first sentence of the case of the fill. The message of line 467 becomes "After a host reboot: handle the cases after the wait".
  - Run `python3 -m unittest test_upgrade_runbook` and see the test fail.
  - Then change line 477 to line 492 of "After a host reboot" in `docs/ops/runbook.md` (design.md D3). Each paragraph holds at most six sentences, and each list item at most three.
  - Check the mutation once: put the sentence "Run step 4 of the recovery again." back into the list of cases in the working tree. `test_restore_and_recovery_window_has_conditional_wait_and_skip` (line 429) fails. Undo the edit.
  - Check: `python3 -m unittest test_upgrade_runbook` passes, and `ste-lint.py docs/ops/runbook.md` reports 0 hard violations.

## 5. Other documents

- [x] 5.1 (docs/architecture) [mechanical] (exempt: documentation only, see the header) In `docs/architecture/reliability.md`:
  - §7.7 line 856: change "and stops `walarchive`." to "and deletes the container of `walarchive` (ADR-0176)." The line has no citation now.
  - (option A) §7.10 step 6, line 985: change "Stop `backup` before command 1." to the new text below. The citation "(7.7)" at the end of the third sentence stays.
    - "Delete the container of `backup` with `docker compose rm -s -f backup` before command 1 (ADR-0176)."
  - Check: `grep -n 'stops `walarchive`' docs/architecture/reliability.md` prints nothing, and `ste-lint.py` on the file reports 0 hard violations.
- [x] 5.2 (.github) [mechanical] (exempt: comment only, see the header) In `.github/workflows/ci.yml` line 598, change "stop walarchive" to "delete walarchive". Check: `bash deploy/ci/check.sh` passes.
- [x] 5.3 (CHANGELOG.md) [mechanical] (exempt: documentation only, see the header) Add one line under `### Fixed` of `[Unreleased]`. The line says that the major upgrade deletes the container of `app`, and the recovery from the WAL archive deletes the container of `walarchive`. With option A, it also names `backup` in the rollback. It cites ADR-0176. Check: `ste-lint.py CHANGELOG.md` reports 0 hard violations.

## 6. Gate

- [x] 6.1 (deploy/ci) Run the local checks and the CI dispatch:
  1. `bash deploy/ci/check.sh` passes. It runs each `deploy/ci/test_*.py`.
  2. `cargo test -p recipe-server --test deploy_values` passes.
  3. `openspec validate restore-reboot-hazards --strict` reports valid.
  4. Check the synced main spec. Copy `openspec/` to a scratch directory. In the copy, apply the delta to `openspec/specs/operations/spec.md`: append each ADDED requirement and replace each MODIFIED requirement whole. Then `openspec validate --specs --strict` in the copy reports valid. CI runs `openspec validate --all --strict` after the archive.
  5. The `ste-lint.py` command of `CLAUDE.md` reports 0 hard violations. Run it with `globstar` on `openspec/changes/restore-reboot-hazards/**/*.md` too.
  6. `grep -n -E 'docker compose (stop|start) (app|walarchive|backup)' docs/ops/runbook.md` prints two lines only: step 4 of the subsection "Stop the whole stack" and step 2 of "A gap in the WAL stream".
  7. On the order of the owner, push the branch with `git push -u origin fix/restore-reboot-hazards`. The dispatch runs the commit of the remote branch.
  8. Run `gh workflow run CI --ref fix/restore-reboot-hazards`. The jobs `upgrade`, `restore-switch`, and `pitr-recovery` do not run on a pull request (ADR-0094).
  9. The three jobs pass in that run. Record the URL of the run in the pull request.

## Workflow follow-up

- After the gate, run `code-review`, fix each CRITICAL, HIGH, and MEDIUM finding, and run `openspec-archive-change`.
