# Design

## Context

The archived change `harden-restore-guard-tests` built the guards of the restore window and of the recovery window of `app` and `backup` (ADR-0175 (h)).
Its design names the windows of ADR-0176 (g) and (h) as a later separate change (owner decision Q2). This change adds them.

- `deploy/ci/docker_calls.py` holds the shared parser: `shell_commands`, `docker_call`, `forbidden_compose_call` (the rule D7 of the archived change), and `forbidden_window_calls`.
- `deploy/ci/test_upgrade_runbook.py` holds the runbook helpers. `runbook_deleted_service_starts(section, services, subcommands)` returns each refused command.
- `RunbookMajorUpgradeDeleteTests` and `FailedRollbackRunbookTests` call that helper with `start` and `restart` only.
- `RunbookRecoveryDeleteTests` (`test_pitr_recovery.py` line 2039) guards the window of `app` and `backup`. It does not guard `walarchive` after step 4.
- The pass-path tests of `pitr-recovery.sh` guard `app` and `backup` in the records of `crash`, `recover`, `switch`, and `wal-line`.

The line numbers in this design refer to `main` 39f1e15.

### Evidence on `main` 39f1e15

All runs used a `git archive` copy of the worktree in the scratch directory. No run changed the worktree.
The baseline run of `python3 -m unittest test_docker_calls test_upgrade_runbook test_runbook_text test_restore_switch test_pitr_recovery` printed `Ran 598 tests in 45.423s` and `OK`.
Each mutant changed one file of a fresh copy and ran the same command. The table "Mutant edits" below gives the exact edit of each mutant.
The column "Guard class" names the test class that must kill the mutant after the change.

| Finding | Name | Guard class | Result on `main` |
|---|---|---|---|
| (g) | `g_major_step3_app` | `test_reboot_windows.RunbookMajorUpgradeWindowTests` | `== g_major_step3_app: Ran 598 tests in 31.237s \| OK` |
| (g) | `g_major_step12_up_all` | `test_reboot_windows.RunbookMajorUpgradeWindowTests` | `== g_major_step12_up_all: Ran 598 tests in 31.313s \| OK` |
| (g) | `g_rollback_step2_backup` | `test_reboot_windows.RunbookRollbackWindowTests` | `== g_rollback_step2_backup: Ran 598 tests in 31.307s \| OK` |
| (g) | `g_rollback_step7_app` | `test_reboot_windows.RunbookRollbackWindowTests` | `== g_rollback_step7_app: Ran 598 tests in 31.341s \| OK` |
| (g) | `g_ci_rollback_backup` | `test_reboot_windows.RollbackCheckScriptTests` | `== g_ci_rollback_backup: Ran 598 tests in 30.307s \| OK` |
| (g) | `g_ci_rollback_continuation` | `test_reboot_windows.RollbackCheckScriptTests` | `== g_ci_rollback_continuation: Ran 598 tests in 21.984s \| OK` |
| (h) | `h_recovery_step5_walarchive` | `test_reboot_windows.RunbookRecoveryWalarchiveWindowTests` | `== h_recovery_step5_walarchive: Ran 598 tests in 31.254s \| OK` |
| (h) | `h_recovery_step9_walarchive` | `test_reboot_windows.RunbookRecoveryWalarchiveWindowTests` | `== h_recovery_step9_walarchive: Ran 598 tests in 30.321s \| OK` |
| (h) | `h_restore_step3_walarchive` | `test_reboot_windows.RunbookRecoveryWalarchiveWindowTests` | `== h_restore_step3_walarchive: Ran 598 tests in 22.211s \| OK` |
| (h) | `h_restore_step5_walarchive` | `test_reboot_windows.RunbookRecoveryWalarchiveWindowTests` | `== h_restore_step5_walarchive: Ran 598 tests in 22.043s \| OK` |
| (h) | `h_restore_step6_walarchive` | `test_reboot_windows.RunbookRecoveryWalarchiveWindowTests` | `== h_restore_step6_walarchive: Ran 598 tests in 22.100s \| OK` |
| (h) | `h_restore_step7_prefix_walarchive` | `test_reboot_windows.RunbookRecoveryWalarchiveWindowTests` | `== h_restore_step7_prefix_walarchive: Ran 598 tests in 22.028s \| OK` |
| (h) | `h_crash_walarchive` | `test_pitr_recovery.StepTests` | `== h_crash_walarchive: Ran 598 tests in 30.292s \| OK` |
| (h) | `h_recover_walarchive` | `test_pitr_recovery.StepTests` | `== h_recover_walarchive: Ran 598 tests in 30.397s \| OK` |
| (h) | `h_wal_line_prefix` | `test_pitr_recovery.StepTests` | `Ran 598 tests in 26.621s \| FAILED (failures=42)` |
| L-1 | `l1_stop_file` | `test_upgrade_runbook.RunbookMajorUpgradeDeleteTests` | `== l1_stop_file: Ran 598 tests in 28.199s \| OK` |
| L-1 | `l1_stop_timeout` | `test_upgrade_runbook.RunbookMajorUpgradeDeleteTests` | `== l1_stop_timeout: Ran 598 tests in 27.964s \| OK` |
| F5 | `f5_cluster_ci` | `test_restore_switch.RestoreBehaviorTests` | `== f5_cluster_ci: Ran 598 tests in 28.055s \| OK` |
| F5 | `f5_cluster_runbook` | `test_restore_switch.RunbookRestoreDeleteTests` | `== f5_cluster_runbook: Ran 598 tests in 35.697s \| OK` |
| F6 | `f6_tab_marker` | `test_pitr_recovery.RunbookRecoveryDeleteTests` | `== f6_tab_marker: Ran 598 tests in 27.923s \| OK` |

Mutant edits. The columns "Anchor" and "Text" hold Python string literals. In the table, `\|` stands for `|`.
The anchor occurs once in the file. "After" writes the text directly after the anchor. "Before" writes it directly before the anchor. "Replace" writes the text in place of the anchor.

| Name | File | Anchor | Edit | Text |
|---|---|---|---|---|
| `g_major_step3_app` | `docs/ops/runbook.md` | ``"3. Run `docker compose exec backup restore-test`. Continue only after it passes."`` | After | ``" Then run `docker compose up -d --no-deps app`."`` |
| `g_major_step12_up_all` | `docs/ops/runbook.md` | `"Then run its sub-steps 4 to 7."` | Before | ``"Run `docker compose up -d`. "`` |
| `g_rollback_step2_backup` | `docs/ops/runbook.md` | ``"2. Set `RECIPE_VERSION` in `.env` to the previous tag. Do this only after command 1 of step 1 passes."`` | After | ``" Then run `docker compose up -d --no-deps backup`."`` |
| `g_rollback_step7_app` | `docs/ops/runbook.md` | ``"7. Truncate `session` and `login_attempt`, and re-apply the user changes."`` | After | ``" Run `docker compose up -d --no-deps app`."`` |
| `g_ci_rollback_backup` | `deploy/ci/upgrade.sh` | `'  compose rm -s -f backup \|\| fail "cannot delete backup before the runbook check"'` | After | `"\n  compose up -d --no-deps backup \|\| true"` |
| `g_ci_rollback_continuation` | `deploy/ci/upgrade.sh` | `'  compose rm -s -f backup \|\| fail "cannot delete backup before the runbook check"'` | After | `"\n  compose \\\n    up -d --no-deps backup \|\| true"` |
| `h_recovery_step5_walarchive` | `docs/ops/runbook.md` | `'5. Create the new volume. Run step 3 of "Restore into production". The command prints the name of the volume.'` | After | ``" Run `docker compose up -d --no-deps walarchive`."`` |
| `h_recovery_step9_walarchive` | `docs/ops/runbook.md` | `'9. Set the role passwords. Run step 6 of "Restore into production". The cluster already holds'` | Replace | ``'9. Set the role passwords. Run `docker compose up -d --no-deps walarchive`. Run step 6 of "Restore into production". The cluster already holds'`` |
| `h_restore_step3_walarchive` | `docs/ops/runbook.md` | `"3. Create the new volume."` | After | ``" Run `docker compose up -d --no-deps walarchive`."`` |
| `h_restore_step5_walarchive` | `docs/ops/runbook.md` | `"5. Run the switch (ADR-0103 (f))."` | After | ``" Run `docker compose up -d --no-deps walarchive`."`` |
| `h_restore_step6_walarchive` | `docs/ops/runbook.md` | `"6. Write the start time into the change log ([rule](#change-log-of-the-site))."` | After | ``" Run `docker compose up -d --no-deps walarchive`."`` |
| `h_restore_step7_prefix_walarchive` | `docs/ops/runbook.md` | ``"7. Start the new WAL line before `app` (ADR-0103 (r), (s))."`` | After | ``" Run `docker compose up -d --no-deps walarchive`."`` |
| `h_crash_walarchive` | `deploy/ci/pitr-recovery.sh` | `'  compose rm -s -f walarchive \|\| fail "crash: cannot delete walarchive."'` | After | `"\n  compose up -d --no-deps walarchive \|\| true"` |
| `h_recover_walarchive` | `deploy/ci/pitr-recovery.sh` | `'  log "recover: check the completed line"'` | Before | `"  compose up -d --no-deps walarchive \|\| true\n"` |
| `h_wal_line_prefix` | `deploy/ci/switch-steps.sh` | `'  log "$WAL_LABEL: step 2, create the target directory"'` | Before | `"  compose up -d --no-deps walarchive \|\| true\n"` |
| `l1_stop_file` | `docs/ops/runbook.md` | ``"4. Check that the line `restore test passed` names the `_now` set of step 2."`` | After | ``" Run `docker compose -f compose.yaml stop app`."`` |
| `l1_stop_timeout` | `docs/ops/runbook.md` | ``"4. Check that the line `restore test passed` names the `_now` set of step 2."`` | After | ``" Run `docker compose stop -t 30 app`."`` |
| `f5_cluster_ci` | `deploy/ci/restore-switch.sh` | `'  compose rm -s -f app backup \|\| fail "restore: cannot delete app and backup."'` | After | `"\n  compose up -dt 10 --no-deps \|\| true"` |
| `f5_cluster_runbook` | `docs/ops/runbook.md` | `"3. Create the new volume."` | After | ``" Run `docker compose up -dt 10 --no-deps`."`` |
| `f6_tab_marker` | `docs/ops/runbook.md` | `"\n5. Create the new volume. Run step 3"` | Replace | ``"\n-\tType a lone ` here.\n-\tRun `docker compose start app`.\n\n5. Create the new volume. Run step 3"`` |

The rule of a kill:

1. A test of the guard class of the row fails.
2. For a runbook mutant, the failure output holds the inserted command, for example `docker compose up -d --no-deps app`.
3. For a script mutant, the failure output holds the subcommand and each later word of the call, for example `up`, `-d`, `--no-deps`, and `walarchive`.
4. A failure of a structural test alone is no kill. A structural test checks an anchor, a step number, or a pinned text.

On `main`, the row `h_wal_line_prefix` fails 42 tests, but no guard of a deleted service fails. A rerun listed the failed classes.
They are `SwitchBackBehaviorTests` (19 tests), `WalLineBehaviorTests` (21 tests), `WalLineTextTests` (1 test), and `StepTests` (1 test, `test_wal_line_stops_when_the_new_line_file_is_wrong`).
These tests pin the order and the result of the records of `start_wal_line`. After the change, the guard of D5 must kill this row too.

Seven direct probes add detail:

- Windows: `probe_windows.py` ran the current guard with `up`, `create`, `run`, `start`, and `restart` on each runbook window of D2. The results:
  - Major upgrade, lines 1943 to 1986: `['docker compose up -d --no-deps app']`. This hit is the allowed start of a failed step (line 1966).
  - Rollback, lines 1607 to 1665: `['env -u COMPOSE_FILE RECIPE_VERSION=<new tag> docker compose -f compose.yaml run --rm restore-volume --check-only <stamp>']`. This hit is command 1 (line 1622). See Q1.
  - Recovery, lines 983 to 1038, with the service `walarchive`: `['docker compose up -d --no-deps walarchive']`. This hit is the allowed start of a failed attempt (line 1020).
  - The step numbers inside each window are unique: `['1', ..., '12']`, `['1', ..., '8']`, and `['4', ..., '10']`. The whole sections "Major upgrade" and "Rollback" repeat the numbers 1 to 3.
  - The `stop` guard of L-1 gave `[]` on the sections "Major upgrade", "Rollback", and "Recover from the WAL archive".
- Records: `probe_pitr_records.py` printed the Compose records of the pass paths. `crash` has `[10] compose rm -s -f walarchive`. `wal-line` has `[1] compose up -d --no-deps walarchive`.
  - `recover` and `switch` hold no refused record for `walarchive`. The switch holds `[0] compose stop proxy app backup walarchive` and `[5] compose rm -f proxy app backup walarchive db`.
- F4: `python3 -O -m unittest test_docker_calls` printed `Ran 35 tests in 0.007s` and `FAILED (failures=2, errors=4)`. Without `-O`, the suite printed `OK`.
- F3: the copy `f3_structure_only` replaced `forbidden_window_calls` and `forbidden_check_window_calls` with functions that only raise `AssertionError("structure only")`.
  - Four mutant tests and `CheckBehaviorTests` ran. The run printed `Ran 26 tests in 4.141s` and `FAILED (failures=1)`.
  - Only `test_check_runs_every_step_in_order` failed. The four mutant tests passed with a guard that reads no record.
  - The second copy `f3_runbook_structure_only` replaced `runbook_deleted_service_starts` and `RestoreBehaviorTests.assert_no_forbidden_docker_commands` in the same way.
  - tasks.md "Probe copies" gives the commands that build the two copies.
  - The seven mutant tests of tasks 4.2 and 5.1 ran in each copy. `f3_structure_only` printed `Ran 7 tests in 1.042s` and `OK`. `f3_runbook_structure_only` printed `Ran 7 tests in 1.024s` and `OK`.
  - So the tests of lines 713, 2146, and 3082 also pass with a guard that reads nothing. Only the second copy can show their RED run.
- References: `probe_references.py` ran the current guard with the five window subcommands on each text that a window runs by reference:
  - Restore steps 3, 5, and 6, and restore step 7 before the span of sub-step 7.7, with `walarchive`: `[]` each.
  - Restore step 9 before sub-step 4, with `app` and `backup`: `[]`.
  - "Upgrade" step 2 with `app`, and "Load the images" and "Leave the file compose.previous.yaml" with the three services: `[]` each.
  - "Probe of the previous image (ADR-0174 (aa))" with `app` and `backup`: `[]`.
  - "Switch back" with the three services: `['docker compose up -d --no-deps walarchive', 'docker compose up -d --no-deps app', 'docker compose up -d --no-deps proxy backup']`.
- Restore window guard: a copy got ``" Then run `docker compose up -d --no-deps app`."`` after `'   2. If the command prints no row, record "missing".'`.
  - `test_restore_switch.RunbookRestoreDeleteTests.test_restore_window_keeps_app_and_backup_without_a_container` printed `AssertionError: Lists differ: ['docker compose up -d --no-deps app'] != []` and `FAILED (failures=1)`.
  - With `backup` in place of `app`, it printed `AssertionError: Lists differ: ['docker compose up -d --no-deps backup'] != []`.
- D6: `probe_d6.py` applied the edit of `g_ci_rollback_continuation` to `check_rollback_runbook` and read the lines between the delete and the start of `backup`.
  - Each line alone gave only command 1: `['env -u COMPOSE_FILE RECIPE_VERSION=ci-upgrade docker compose -f compose.yaml run --rm restore-volume --check-only $stamp']`.
  - With joined continuation lines, the hits started with `'compose up -d --no-deps backup'`, followed by the same command 1.

## Goals / Non-Goals

**Goals:**

- A test kills each mutant of the table above, by the rule of a kill in "Evidence".
- Each new test passes on the current scripts and on the current runbook.
- Each LOW finding F3 to F7 and L-1, L-4, L-5 (not applied, owner decision 2026-11-17), and L-6 gets its fix. L-2 and L-3 get the fix that the owner chose (Q2 and Q3).

**Non-Goals:**

- The window of "A gap in the WAL stream". ADR-0176 (k) keeps its `stop` of `walarchive`.
- A guard against a bare `docker compose stop` (D11).
- A change of a script of `deploy/ci/`, or of a command of a procedure without an owner decision.

## Decisions

### D1. No ADR and no spec delta

The change adds and changes tests in `deploy/ci/`. It also fixes words in `docs/ops/runbook.md` and `docs/adr/README.md`.
The test helpers are not design decisions of `docs/architecture/`, so no ADR records them.
The specs state behavior. The three requirements of `operations` about the containers of ADR-0176 stay true.
They are "Container of app in a major upgrade", "Container of walarchive in a recovery", and "Check of the rollback set with the new image".
So `.openspec.yaml` sets `skip_specs: true`, as the archived change `harden-restore-guard-tests` did.
The owner chose Q1 option A. Command 1 stays as ADR-0174 (aj) and `reliability.md` 7.10 pin it, so this change needs no ADR.

### D2. The windows and their end events

Each window is a range of the runbook. A start of a deleted service inside the range gives a hit, except an allowed end start.

| Window | Start | End of the main path | Allowed end start of a failed path | Services |
|---|---|---|---|---|
| Major upgrade (g) | Step 1, line 1943 | Step 12 reference `Then run its sub-steps 4 to 7.`, line 1986 | Span `docker compose up -d --no-deps app`, line 1966 | `app` in steps 1 to 7. `app` and `backup` from step 8 |
| Rollback (g) | Step 1 of the second list, line 1607 | Step 8 Bash block, lines 1666 and 1667 | None in the range | `app` and `backup` |
| Recovery (h) | Step 4, line 983 | Span `docker compose up -d --no-deps walarchive` of restore sub-step 7.7, line 839. Step 10 reaches it by the reference `Run step 7 of "Restore into production".`, line 1038 | Span `docker compose up -d --no-deps walarchive` in step 7, line 1020 | `walarchive` |

- Major upgrade: step 8 runs step 2 of "Restore into production", which also deletes `backup` (ADR-0176 (c)). So from step 8, the window also refuses `backup`.
- Major upgrade: the paragraph "If a step before step 8 fails" holds the allowed start. It lies between step 4 and step 5.
- Rollback: step 1 deletes `backup` before command 1 and `app` after it. The window refuses both services from step 1, as the restore window does.
- Rollback: the first block line `docker compose up -d --no-deps app` ends the window of `app`. The second line `docker compose up -d --no-deps proxy backup` ends the window of `backup`.
- Rollback: the failed paths of step 1 and step 3 start `backup` by a reference to "Failed rollback restore after a failed upgrade". That section lies after the range. `FailedRollbackRunbookTests` pins its blocks.
- Recovery: the window of `app` and `backup` stays as the archived change built it. This change adds the window of `walarchive`.

Text that a window runs by reference:

A step of a window can run a step of another section by a reference. That text then runs inside the window. The real start can lie in the other section.
Plan review R1 found this gap in the recovery window of `walarchive`. The check of each window gave these results (probes of "Evidence"):

| Window | Text that runs before the real start | Guard of that text on `main` | Result |
|---|---|---|---|
| Major upgrade, `app` and `backup` | Steps 8 to 11 run restore steps 2 to 8. Step 12 runs restore sub-steps 9.1 to 9.3 before its reference. The real starts are restore sub-step 9.4 for `app` (line 858) and 9.5 for `backup` (line 859) | `test_restore_window_keeps_app_and_backup_without_a_container` (`test_restore_switch.py` line 3008) reads restore steps 2 to 8 and step 9 up to these two spans | No defect. A probe start in restore sub-step 9.2 fails that test with the inserted command |
| Rollback, `app` and `backup` | Steps 1 and 3 to 7 run restore steps 3 to 8. Step 8 runs restore sub-steps 9.1 to 9.3. The real starts are the Bash block of step 8 | The same restore window test | No defect |
| Recovery, `app` and `backup` (archived change) | Steps 5 and 8 to 11 run restore steps 3, 5, 6, 7, sub-step 8.1, and step 9 | The same restore window test | No defect |
| Recovery, `walarchive` | Steps 5, 8, and 9 run restore steps 3, 5, and 6. Step 10 runs restore sub-steps 7.1 to 7.6 before the real start in sub-step 7.7 | None. The restore window test reads `app` and `backup` only | Defect. The four mutants `h_restore_*` pass on `main` |

- Recovery: the window of `walarchive` also reads restore steps 3, 5, and 6. It reads restore step 7 up to the span of sub-step 7.7 (D3 item 8).
- "Switch back" starts `walarchive`, `app`, `proxy`, and `backup`. A window runs the whole section only after a failed sub-step 7 of the switch. So it is the end of a failed path.
- Recovery sub-step 8.1 (line 1023) also uses the rule of step 2 of "Switch back" on the main path, inside the window of `walarchive`.
  On `main`, that step (lines 1827 to 1835) holds one `docker run` of `pg_controldata` and no start. No window test reads it. "Risks / Trade-offs" records this limit.
- Major upgrade steps 6 and 7 run "Upgrade" step 2, "Load the images", and "Leave the file compose.previous.yaml". Rollback step 2 can run "Probe of the previous image (ADR-0174 (aa))".
  No window test reads these texts. The probe gave `[]` for each. "Risks / Trade-offs" records this limit.

### D3. Real ranges and unique step numbers

Each window test builds its range from the real text, as the restore and recovery windows of the archived change do:

1. The test finds the start text once in the section. The range then ends at the end event of D2.
2. The test splits the range at each line that starts with a number, a period, and a space. Each step number must occur once in the range.
3. Each step number of the window must exist, for example steps 1 to 12 of the major upgrade.
4. The test finds each end span with `runbook_code_span_offsets`. The span must occur once in its step or paragraph.
5. The test cuts the range directly before the opening backtick of an end span. So a cut never splits a code span.
6. For an allowed start of a failed path, the test reads the text before and after the span. Both parts give no hit.
7. For the end block of the rollback, the test finds the fenced Bash block of step 8. Its stripped lines must equal the two start lines, in this order.
8. The recovery window of `walarchive` also reads text of "Restore into production". It reads restore steps 3, 5, and 6 whole.
   It reads restore step 7 up to the opening backtick of the span `docker compose up -d --no-deps walarchive` of sub-step 7.7.
9. Each of the restore steps 3, 5, 6, and 7 must occur once in that section. The span must occur once in sub-step 7.7.
10. The window reads two sections. So a mutant test patches `runbook_heading_section` with a function that changes only the text of the mutated section.

The sections "Major upgrade" and "Rollback" repeat the numbers 1 to 3 outside the range. So the check of unique numbers reads the range only.
The guard uses `runbook_deleted_service_starts` with `subcommands=("up", "create", "run", "start", "restart")`.

### D4. Command 1 of the rollback

Command 1 runs `docker compose -f compose.yaml run --rm restore-volume --check-only <stamp>` without `--no-deps`. The rule D7 of the archived change refuses it.
`restore-volume` has no `depends_on`, `links`, or `volumes_from` in `deploy/compose.yaml` (lines 360 to 388). Its `network_mode` is `none`. So this `run` starts no other service.
With Q1 option A, the rollback window test expects exactly the hit list `[ROLLBACK_CHECK]` in step 1. Steps 2 to 8 give no hit.
Compose adds a dependency by `depends_on`, `links`, `volumes_from`, and a `network_mode` value that starts with `service:`.
A second test reads the block of `restore-volume` in `compose.yaml`. It asserts that the block has none of these keys and values.
So a later dependency of `restore-volume` fails a test, and the exception then needs a new decision.

### D5. The record window of `walarchive`

`pitr-recovery.sh` deletes `walarchive` in `crash()` (line 484). `start_wal_line` of `switch-steps.sh` creates it again in the step `wal-line` (line 544).
A new function `record_window(records, *, start=None, end=None)` of `docker_calls.py` returns a `slice`.

- `start` and `end` are a Compose subcommand and its operands, for example `("rm", ["-s", "-f", "walarchive"])`. The function compares them with the result of `docker_call`.
- Each given record must occur exactly once. Else the function raises `AssertionError`.
  The message holds the subcommand and each operand of the record, joined by spaces, and the count. So it satisfies the rule of a kill.
- The slice starts after the start record and ends before the end record.

The pass-path tests then call `forbidden_window_calls` with `services=("walarchive",)`:

- `crash`: the slice after `compose rm -s -f walarchive`.
- `recover` and `switch`: the whole run.
- `wal-line`: the slice before `compose up -d --no-deps walarchive`.

The steps `refuse` and `check` run after `wal-line`, so they are outside the window.

### D6. CI scripts in the windows

The result of the check of each CI script:

- `pitr-recovery.sh`: yes. The `walarchive` window runs from line 484 through `recover` and `switch` to line 544 of `switch-steps.sh`. D5 guards the records.
- `upgrade.sh`: yes, for the rollback. `check_rollback_runbook` deletes `backup` at line 651 and creates it again at line 663. Line 659 runs command 1.
- `upgrade.sh`: no window of the major upgrade. The script deletes no container of `app`. `restore_previous` (lines 670 to 699) deletes no container.
- `restore-switch.sh`: no window of ADR-0176. Its windows are the windows of ADR-0175, which the archived change guards.

`upgrade.sh` has no Docker stub harness, and line 659 runs inside `sudo bash -s` with a here-document.
A record guard needs a new harness for this one line. So a text guard reads the function instead:

1. The test reads `check_rollback_runbook` with `shell_function`.
2. It finds the delete line and the start line of `backup`. Each must occur once.
3. It joins each line that ends with a backslash with the next line. `runbook_deleted_service_starts` does the same for a fenced line (`test_upgrade_runbook.py` line 148).
4. It reads each joined line between the delete line and the start line with `shell_commands`.
5. A simple command that starts with the shell function `compose` is a Compose call. A token `docker` starts a call of `docker`.
6. The rule `forbidden_compose_call` reads each call with `services=("backup",)`.
7. With Q1 option A, the hit list is exactly the command of line 659.

Without step 3, the mutant `g_ci_rollback_continuation` puts `compose \` and `up -d --no-deps backup` on two lines. The probe of "Evidence" found no hit then.
A regression test of task 3.5 uses this mutant.

Alternative: a Docker stub harness for `upgrade.sh`. It costs more than the risk of one function with one command.

### D7. Explicit errors and exact hits (F3, F4)

`forbidden_window_calls` and `forbidden_check_window_calls` use five bare `assert` statements (lines 73, 87, 92, 95, and 99). Under `python3 -O` they do not run.
Each one becomes `if not ...: raise AssertionError(message)`. The messages stay unchanged. The new `record_window` uses the same form.
The structural tests of `test_docker_calls.py` use `assertRaisesRegex` with these messages. `python3 -O -m unittest test_docker_calls` then passes.
A new test of `test_docker_calls.py` parses `docker_calls.py` with `ast` and finds no `ast.Assert` node. So a later bare `assert` fails a test.

A mutant test asserts the exact hits and not only an `AssertionError`:

- A record mutant test compares the hits, read with `docker_call`, with the one inserted call.
- `assert_window_mutant_rejected` (`test_pitr_recovery.py` line 494) gets the expected call as a parameter. Line 509 now holds the fixed text `" up -d --no-deps app backup\n"`.
  The method derives the log line and the expected hits from the parameter. So task 4.3 can use it for `walarchive`.
- A runbook mutant test uses `assertRaisesRegex` with the escaped inserted command.
- The command guard of the step `restore` gives the record as the message of its assertion. Its mutant test uses `assertRaisesRegex` with the subcommand and the first operand of the inserted call.

The places are `test_pitr_recovery.py` lines 494 and 2146, and `test_restore_switch.py` lines 713, 739, 1963, and 3082.

### D8. Short option clusters (F5)

One helper of `docker_calls.py` gives the count of words that an option takes. Both `forbidden_compose_call` and `_skip_global_options` use it:

- `--name=value` takes one word. `--name` takes two words when it is a value option.
- A word with one dash and more than one letter is a cluster, for example `-dt`.
- The helper reads the letters from left to right. The first letter `x` with a value option `-x` ends the cluster.
- When other letters follow `x`, they are the value, for example `-t10`. The cluster then takes one word.
- When `x` is the last letter, the next word is the value. The cluster then takes two words.
- A cluster without a value letter takes one word.

So `up -dt 10 --no-deps` has no operand and gives a hit. `up -dt 10 --no-deps proxy` gives no hit.
The help of `docker compose up` (Compose 5.7.1) lists `-d, --detach` and `-t, --timeout int`.
A local probe ran `docker compose -f /dev/null up <cluster> --no-deps` with three clusters:

- `-dt 10` and `-dt10` each printed `empty compose file`. So Compose accepted the cluster and its value.
- `-dx` printed `unknown shorthand flag: 'x' in -x`.

### D9. A tab after a list marker (F6)

CommonMark accepts a space or a tab after a list marker. `runbook_inline_code_spans` (line 113) accepts only a space.
The pattern `(?: |$)` becomes `(?:[ \t]|$)`. So a tab after `-`, `*`, `+`, or a number also starts a new block.

### D10. Names and sizes (F7)

- The module-level name `spec` in `test_upgrade_runbook.py` (line 14) and in `test_docker_calls.py` (line 7) becomes `DOCKER_CALLS_SPEC`. `test_pitr_recovery.py` (line 32) uses that name now.
- `run_subcommand` of `test_restore_switch.py` (lines 559 to 635) has 77 lines. It keeps its signature and its three return values.
  Free functions take its parts: the state and tree files, the stubs, the environment, and the collection of the files. Each function has at most 50 lines.
- `step_result` of `test_pitr_recovery.py` (line 78) puts the records on the process object as `done.docker_calls`.
  It puts them into its dict under the key `argv` instead, as `run_subcommand` does (line 628). Each caller reads `files["argv"]`.

### D11. The stop guard (L-1)

`test_section_has_no_stop_app_command` (`test_upgrade_runbook.py` line 417) looks for the text `docker compose stop app` only.
It now calls `runbook_deleted_service_starts(section, services=("app",), subcommands=("stop",))` and expects `[]`.
So an option before or after `stop`, more spaces, and a list of services give a hit.
The new file also guards "Rollback" for `app` and `backup`, and "Recover from the WAL archive" for `walarchive`. ADR-0176 (d) and (i) replaced these stops.
A bare `docker compose stop` gives no hit. `forbidden_compose_call` refuses only a named service for subcommands other than `up`, `create`, and `run`.

### D12. Words (L-4, L-5, L-6)

- L-4: the index row of ADR-0176 in `docs/adr/README.md` (line 188) gets the amended items.
  The text is "Amends ADR-0175 (h), (i), (j), (k), ADR-0106 (i) step 4 and (p) step 5, and ADR-0174 (aj)".
  The decision map (line 205) names the same items. The text comes from "Related" of ADR-0176.
- L-5: the runbook says "a restart of Docker" in lines 690, 958, 984, and 1944. Lines 1638 and 1640 say "a restart of the container engine". These two places get "a restart of Docker". The owner decided on 2026-11-17 that L-5 stays unfixed, because `test_rollback_stop_raw_commands` refuses these words in step 1 of the rollback.
- L-6: `test_restore_and_recovery_window_has_conditional_wait_and_skip` (`test_upgrade_runbook.py` line 240) reads four windows. Its new name is `test_reboot_windows_have_conditional_wait_and_skip`.

### D13. Files and sizes

- `test_restore_switch.py` (3245 lines) and `test_pitr_recovery.py` (2536 lines) get no new test class.
- `test_pitr_recovery.py` gets one new mutant method and one more assertion in four pass-path tests. The growth is at most 25 lines.
- The new file `deploy/ci/test_reboot_windows.py` holds the new classes: the three runbook windows, the two new stop guards, and the guard of `check_rollback_runbook`.
  It loads `test_upgrade_runbook.py` and `docker_calls.py` with `importlib.util.spec_from_file_location`, because `check.sh` runs each test file with `python3 -I`.
  It ends with `unittest.main()`. `check.sh` line 21 runs each `deploy/ci/test_*.py`, so `check.sh` needs no change.
- `test_upgrade_runbook.py` (563 lines), `test_runbook_text.py` (359 lines), and `test_docker_calls.py` (314 lines) stay below 800 lines.

## Risks / Trade-offs

- D2 pins the end texts and the allowed starts of three windows. A change of these runbook lines must change the tests too. The test message names the text.
- The major upgrade window refuses `backup` from step 8. A future text that starts `backup` before step 12 then fails a test.
- The exception of D4 holds only while `restore-volume` has no dependency. The test of D4 makes this condition visible.
- The text guard of D6 reads the shell text, not the run. A command that the function builds at run time escapes it. The function holds no such command now.
- The rule of D8 counts an unknown letter of a cluster as a flag. A new short value option of Compose needs a new row in the table.
- The stop guard of D11 does not refuse a bare `docker compose stop`. No runbook section of these windows holds one.
- The windows guard the text of "Restore into production" that they run. They do not guard other sections that they run by reference (D2).
  These are "Upgrade" step 2, "Load the images", "Leave the file compose.previous.yaml", and the probe of ADR-0174 (aa). These texts refer to more sections in turn.
  The probe gave no hit in them on `main`. A later start of `app` or `backup` in one of them escapes the window tests.
- The recovery window of `walarchive` does not read step 2 of "Switch back", which recovery sub-step 8.1 uses (D2). A later start of `walarchive` there escapes the window tests.
- The recovery window of `walarchive` reads restore steps 3, 5, 6, and 7. A new reference of the recovery to another restore step needs a change of that list.

## Decisions of the owner (2026-11-17)

The owner chose option A for Q1, Q2, and Q3. The tasks follow these decisions.

### Q1. Command 1 of the rollback runs without `--no-deps`

Command 1 (runbook line 1622, `upgrade.sh` line 659) is the only `run` of a window without `--no-deps`. Restore step 4 (line 707) and `restore_previous` (`upgrade.sh` line 681) use `run --rm --no-deps`.

The owner chose option A. The window tests allow the exact command 1 as a pinned exception (D4), and no procedure changes.

- Reason: the command starts no other service now, and the test of D4 fails when that changes.
- The test of D4 refuses each of the four ways of Compose to add a dependency in the block of `restore-volume`.
- Rejected: option B (`--no-deps` in command 1 with a new ADR) and option C (a general rule for each service without a dependency).

### Q2. Recovery step 6 deletes the new volume twice (L-2)

Recovery step 6 (line 1008) says "After a failed run, delete the new volume". Then it says "end the attempt as in step 7". Step 7 (line 1017) deletes the new volume again.
A second `docker volume rm` of a deleted volume can print an error and confuse the operator.

The owner chose option A. Step 6 drops its delete. Its last item becomes "After a failed run, end the attempt as in step 7." Step 7 keeps the delete.

- Reason: one step then holds the delete, and the text of step 7 already gives the order of the end of an attempt.
- Rejected: option B (a conditional delete in step 7) and option C (no change).

### Q3. The two cases of "After a host reboot" name only the restore and the recovery (L-3)

The first case names step 4 of the restore and step 6 of the recovery. Its restart point is step 3 of the restore or step 5 of the recovery.
The second case names step 7 of the restore and step 10 of the recovery. So each case uses three restore steps: 3, 4, and 7.

| Restore step | Recovery | Major upgrade | Rollback |
|---|---|---|---|
| 3 (restart point of the first case) | Step 5 | Step 8 | Step 1 |
| 4 (first case) | No step. Step 6 fills the volume with `pitr-volume` | Step 8 | Step 3 |
| 7 (second case) | Step 10 | Step 10 | Step 6 |

Rollback step 1 also holds command 1 and the two deletes. Its restart point is the sentence "Then create the new volume. Run step 3 of "Restore into production"."

The owner chose option A. Two sentences follow the cases:

- "In the major upgrade, step 8 runs restore steps 3 and 4, and step 10 runs restore step 7."
- "In the rollback, step 1 runs restore step 3, step 3 runs restore step 4, and step 6 runs restore step 7."

Reason: option A keeps each case short, and `RunbookHostRebootTests.required_sentences` can pin the new sentences.
Rejected: option B (sentences for each procedure in each case) and option C (no change).

## LOW findings of the plan reviews

The plan reviews R1 and R2 (Opus) gave these LOW findings. The owner chose to fix each one. Each item names the place of its fix.

- L1 (R1): the check of task 2.3 searches the full old name. Fix: `tasks.md:156`.
- L2 (R1): the size check of task 7.1 names the files of groups 1 to 3 and the new file. Fix: `tasks.md:317`.
- L4 (R1): the header names the third regression case `up -dt 10 --no-deps proxy`. Fix: `tasks.md:23` to `tasks.md:28` and `tasks.md:118`.
- L5 (R1): a permanent `ast` test refuses a bare `assert` in `docker_calls.py`. Fix: `design.md:251` and `tasks.md:112`.
- L6 (R1): the test of D4 refuses the four keys of a dependency. Fix: `design.md:197` to `design.md:201`, `design.md:344`, and `tasks.md:174` to `tasks.md:175`.
- L7 (R1): the list of `deploy_values` rules names the pair `system prune`. Fix: `tasks.md:34`.
- N1 (R2): the probe copies use the variables `copy_f3` and `copy_f3r`. Fix: `tasks.md:44` to `tasks.md:52`, `tasks.md:58`, `tasks.md:73`, `tasks.md:79`, `tasks.md:230` to `tasks.md:232`, and `tasks.md:266` to `tasks.md:268`.
- N2 (R2): both recovery mutant tests of task 3.3 patch by title and keep the original function. Fix: `tasks.md:183` to `tasks.md:187`.
- N3 (R2): the mutant test of task 4.3 passes a guard with one argument. Fix: `tasks.md:240`.
- N4 (R2): D2 and the risks name step 2 of "Switch back" that recovery sub-step 8.1 uses. Fix: `design.md:170` to `design.md:171` and `design.md:330`.
- N5 (R2): the owner rejected option B of Q1. The line of the rejected options holds no cost list, so this finding needs no text. Place: `design.md:345`.
