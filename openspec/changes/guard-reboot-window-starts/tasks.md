# Tasks

Each task is for one agent. Codex `gpt-6.1-sol` implements each task in `deploy/ci/`. Haiku does each `[mechanical]` task.

Each test task first writes a failing test and runs it (RED). Then it writes the change that makes the test pass (GREEN).
Run each Python suite from `deploy/ci`, for example `cd deploy/ci && python3 -m unittest test_docker_calls`.
The six suites are `test_docker_calls`, `test_upgrade_runbook`, `test_runbook_text`, `test_reboot_windows`, `test_restore_switch`, and `test_pitr_recovery`.
`SUITES` in a command stands for the suites that exist at that time. `test_reboot_windows` exists from task 3.1.

Each GREEN check also runs the mutants that the task names, in a scratch copy of the worktree. Each named mutant is a row of design.md "Evidence".
Build the copy as in "Probe copies" below. Then apply the edit of the row in the table "Mutant edits". Never change the worktree for a mutant.
Then run `python3 -m unittest SUITES` in `deploy/ci` of the copy. The mutant must be killed by the rule of a kill in design.md "Evidence".
A failed structural test alone is no kill. The failed test must belong to the guard class of the row, and its output must hold the inserted command.

These tasks are exempt from the failing test:

- Tasks 2.3, 4.1, and 5.2 rename or split code. The existing tests are their check, and they must pass unchanged.
- Tasks 4.2 and 5.1 harden existing mutant tests (F3). Their RED run uses the probe copies `f3_structure_only` and `f3_runbook_structure_only` of "Probe copies".
  In these copies, a guard only raises `AssertionError("structure only")`. The current mutant tests pass there. After the task, they fail there.
- Tasks 6.1, 6.2, and 6.5 change documentation only. `ste-lint.py` checks them.
- Task 7.1 is the gate of the change. It runs checks and adds no test.

Three regression cases pass now. They keep a current result:

- `up -dt10 --no-deps` of task 1.2
- `up -dt 10 --no-deps proxy` of task 1.2. On `main`, `forbidden_compose_call("up", ["-dt", "10", "--no-deps", "proxy"])` returns `False`.
- the dependency test of `restore-volume` of task 3.2

The owner chose option A of Q1, Q2, and Q3 (design.md "Decisions of the owner (2026-11-17)").
Tasks 3.2 and 3.5 follow option A of Q1. Task 6.3 follows option A of Q2. Task 6.4 follows option A of Q3.

Three rules of `crates/recipe-server/tests/deploy_values.rs` apply to each line of `deploy/ci/`, test files too:

- The word `volume` is never directly followed by the word `rm` or `prune`. The word `system` is never directly followed by the word `prune` (line 246).
  Split such a test text into two literals, as `test_upgrade_runbook.py` line 195 does.
- A line with the word `down` holds no later `--volumes` and no later short option with the letter `v`.
- No line holds `|| exit 1` or `*) exit 1`.

The line numbers refer to `main` 39f1e15.
`check.sh` runs each test file with `python3 -I`, so a test loads a sibling file with `importlib.util.spec_from_file_location`.

## Probe copies

Build the two copies from the root of the worktree. Each copy holds the current files, also the files that are not committed.
The variable `copy_f3` holds the copy `f3_structure_only`. The variable `copy_f3r` holds the copy `f3_runbook_structure_only`:

```bash
copy_f3=$(mktemp -d)
copy_f3r=$(mktemp -d)
for copy in "$copy_f3" "$copy_f3r"; do
  git ls-files -z --cached --others --exclude-standard | tar --null -T - -cf - | tar -x -C "$copy"
done
```

The copy `f3_structure_only` replaces the two record guards. Run this after the commands above:

```bash
cat >> "$copy_f3/deploy/ci/docker_calls.py" <<'EOF'


def forbidden_window_calls(*args, **kwargs):
    raise AssertionError("structure only")


def forbidden_check_window_calls(*args, **kwargs):
    raise AssertionError("structure only")
EOF
```

The copy `f3_runbook_structure_only` replaces the runbook guard and the command guard of the step `restore`. Run this after the commands above:

```bash
cat >> "$copy_f3r/deploy/ci/test_upgrade_runbook.py" <<'EOF'


def runbook_deleted_service_starts(*args, **kwargs):
    raise AssertionError("structure only")
EOF
cat >> "$copy_f3r/deploy/ci/test_restore_switch.py" <<'EOF'


def structure_only(self, records):
    raise AssertionError("structure only")


RestoreBehaviorTests.assert_no_forbidden_docker_commands = structure_only
EOF
```

When Python loads the module, a later definition of the same name replaces the first one. So the tests of the copy call the replacement.
A run of the seven RED selectors of tasks 4.2 and 5.1 on `main` printed `OK` in both copies (design.md "Evidence", F3).

## Run order

1. Group 1 edits `docker_calls.py` and `test_docker_calls.py`. Tasks 1.1, 1.2, and 1.3 run in this order.
2. Group 2 edits `test_upgrade_runbook.py` and `test_runbook_text.py`. Tasks 2.1, 2.2, and 2.3 run in this order, after task 1.3. Task 2.3 also edits `test_docker_calls.py`.
3. Group 3 creates and edits `test_reboot_windows.py`. Tasks 3.1 to 3.5 run in this order, after task 2.3.
4. Group 4 edits `test_pitr_recovery.py`. Tasks 4.1, 4.2, and 4.3 run in this order, after task 1.3.
5. Group 5 edits `test_restore_switch.py`. Tasks 5.1 and 5.2 run in this order, after task 1.3.
6. Groups 4 and 5 can start after task 1.3. Groups 3, 4, and 5 can then run in parallel. Their tasks do not edit `docker_calls.py`.
7. Tasks 6.2 and 6.5 can run at any time. Tasks 6.1, 6.3, and 6.4 each edit `docs/ops/runbook.md`.
   They run in the order 6.1, 6.3, 6.4, one after the other. They never run in parallel.
   Task 6.3 starts after task 3.5, because it edits `test_reboot_windows.py`. Task 6.4 edits `test_upgrade_runbook.py`, so it also runs after task 2.3.
8. After groups 3 to 6, the orchestrator runs `python3 -m unittest SUITES` once. All suites pass.
9. Task 7.1 runs last.

## 1. Shared parser

- [ ] 1.1 (deploy/ci) Replace the bare `assert` statements of `docker_calls.py` (F4, design.md D7):
  - First change the structural tests of `WindowCallsTests` (`test_docker_calls.py` line 237) to `assertRaisesRegex` with each message.
  - These are `test_window_rejects_an_empty_run_but_allows_an_empty_slice` (line 259) and `test_check_requires_the_truncate_and_both_exact_end_records` (line 297).
  - Add the test `test_docker_calls_has_no_bare_assert` to `WindowCallsTests`. It parses `docker_calls.py` with `ast` and expects no `ast.Assert` node.
  - Run `python3 -O -m unittest test_docker_calls` and see the two tests and the `ast` test fail. The probe printed `FAILED (failures=2, errors=4)` before these edits.
  - Then replace the five `assert` statements (lines 73, 87, 92, 95, and 99) with `if not ...: raise AssertionError(message)`. Keep each message.
  - Check: `python3 -O -m unittest test_docker_calls` passes. `python3 -m unittest SUITES` passes.
- [ ] 1.2 (deploy/ci) Read short option clusters (F5, design.md D8):
  - First add cases to `ForbiddenComposeCallTests` (`test_docker_calls.py` line 152):
    - `up -dt 10 --no-deps` gives a hit. `up -dt 10 --no-deps proxy` gives no hit. The second case passes now (header).
    - `up -dt10 --no-deps` gives a hit. This case passes now (header).
    - `run --rm --no-deps -Tw proxy` gives a hit, because `proxy` is the value of `-w` in `run`.
  - Add a case to `DockerCallTests` (line 95): the arguments `-Dl debug compose start app` give `(True, "start", ["app"])`.
  - Add a case to `RunbookDeletedServiceSelectionTests` (`test_runbook_text.py` line 138): the span `docker compose up -dt 10 --no-deps` gives one hit with the window subcommands.
  - Run `python3 -m unittest test_docker_calls test_runbook_text` and see the new cases fail.
  - Then add one helper to `docker_calls.py` that gives the count of words of one option. `forbidden_compose_call` and `_skip_global_options` call it.
  - Check: `python3 -m unittest SUITES` passes. The mutants `f5_cluster_ci` and `f5_cluster_runbook` are each killed (header).
- [ ] 1.3 (deploy/ci) Add `record_window` to `docker_calls.py` (design.md D5):
  - First add tests to `WindowCallsTests` of `test_docker_calls.py`:
    - With a start record, the slice starts after it. With an end record, the slice ends before it.
    - A start record or an end record that occurs zero times or twice raises `AssertionError` with a message that names the record.
    - The function compares the subcommand and the operands that `docker_call` returns, so the options of Compose do not matter.
  - Run `python3 -m unittest test_docker_calls` and see the tests fail.
  - Then add the function. It raises `AssertionError` explicitly, as task 1.1 does.
  - Check: `python3 -O -m unittest test_docker_calls` and `python3 -m unittest SUITES` pass.

## 2. Runbook helpers

- [ ] 2.1 (deploy/ci) Accept a tab after a list marker (F6, design.md D9):
  - First add cases to `test_list_items_end_code_spans` (`test_runbook_text.py` line 189).
  - The two lines ``-<TAB>Type a lone ` here.`` and ``-<TAB>Run `docker compose start app`.`` give the span `docker compose start app`.
  - The same holds for the marker `1.` with a tab.
  - Run `python3 -m unittest test_runbook_text` and see the cases fail.
  - Then change the pattern of `runbook_inline_code_spans` (`test_upgrade_runbook.py` line 113) from `(?: |$)` to `(?:[ \t]|$)`.
  - Check: `python3 -m unittest SUITES` passes, and a test kills the mutant `f6_tab_marker` (header).
- [ ] 2.2 (deploy/ci) Parse the stop guard of the major upgrade (L-1, design.md D11):
  - First add a fixture test to `RunbookMajorUpgradeDeleteTests` (`test_upgrade_runbook.py` line 378). It patches `runbook_heading_section`.
  - Each of these sections makes `test_section_has_no_stop_app_command` raise `AssertionError` with the command in the message:
    - `` Run `docker compose -f compose.yaml stop app`. `` and `` Run `docker compose stop -t 30 app`. ``
    - `` Run `docker compose stop proxy app`. `` and a Bash fence with the line `docker compose  stop app`
  - The section `` Run `docker compose stop db`. `` passes.
  - Run `python3 -m unittest test_upgrade_runbook` and see the fixture test fail.
  - Then change `test_section_has_no_stop_app_command` (line 417). It calls `runbook_deleted_service_starts` with `services=("app",)` and `subcommands=("stop",)`.
  - Check: `python3 -m unittest SUITES` passes. The mutants `l1_stop_file` and `l1_stop_timeout` are each killed (header).
- [ ] 2.3 (deploy/ci) [mechanical] (exempt: rename, see the header) Rename two names (L-6, F7, design.md D10, D12):
  - Rename `test_restore_and_recovery_window_has_conditional_wait_and_skip` (`test_upgrade_runbook.py` line 240) to `test_reboot_windows_have_conditional_wait_and_skip`.
  - Rename the module-level name `spec` to `DOCKER_CALLS_SPEC` in `test_upgrade_runbook.py` (line 14) and in `test_docker_calls.py` (line 7).
  - Check: `rg -n '^spec\b|test_restore_and_recovery_window_has_conditional_wait_and_skip' deploy/ci` prints nothing. `python3 -m unittest SUITES` passes.

## 3. Runbook windows of ADR-0176

- [ ] 3.1 (deploy/ci) Create `deploy/ci/test_reboot_windows.py` with the window of the major upgrade (design.md D2, D3, D13):
  1. The file loads `test_upgrade_runbook.py` and `docker_calls.py` with `importlib`. It ends with `unittest.main()`.
  2. Add a class `RunbookMajorUpgradeWindowTests`. Its method `test_major_upgrade_window_keeps_app_without_a_container` first only reads the section.
  3. Add a mutant test. It applies the edit of the rows `g_major_step3_app` and `g_major_step12_up_all` of the table "Mutant edits" (design.md).
  4. It patches `runbook_heading_section` and expects `assertRaisesRegex` with the escaped inserted command, for example `docker compose up -d --no-deps app`.
  5. Add a test that inserts a second line `3. Run the check.` before step 4. It expects the message `duplicate major upgrade step 3`.
  6. Run `python3 -m unittest test_reboot_windows` and see the two tests fail.
  7. Then write the window method with the rules of design.md D2 and D3. The range ends at the reference of step 12.
  8. The allowed span of the paragraph "If a step before step 8 fails" occurs once. The text before it and after it gives no hit.
  9. Steps 1 to 7 use `services=("app",)`. Steps 8 to the reference use `services=("app", "backup")`.
  10. Check: `python3 -m unittest test_reboot_windows test_upgrade_runbook` passes. The two mutants of item 3 are each killed (header).
- [ ] 3.2 (deploy/ci) Add the window of the rollback to `test_reboot_windows.py` (design.md D2, D3, D4, Q1 option A):
  1. Add a class `RunbookRollbackWindowTests`. Its method `test_rollback_window_keeps_app_and_backup_without_a_container` first only reads the section.
  2. Add a mutant test for the rows `g_rollback_step2_backup` and `g_rollback_step7_app`, as task 3.1 does.
  3. Add a test that reads the block of `restore-volume` in `deploy/compose.yaml` (design.md D4).
     It asserts that the block has no key `depends_on`, `links`, or `volumes_from`, and no `network_mode` value that starts with `service:`.
  4. Run `python3 -m unittest test_reboot_windows` and see the mutant test fail.
  5. Then write the window method. The range starts at `1. Delete the container of `backup`.` and ends before the Bash block of step 8.
  6. That block has exactly the lines `docker compose up -d --no-deps app` and `docker compose up -d --no-deps proxy backup`, in this order.
  7. Step 1 gives exactly the hit list `[ROLLBACK_CHECK]`. Steps 2 to 8 give no hit. Each step number from 1 to 8 occurs once in the range.
  8. Check: `python3 -m unittest test_reboot_windows` passes. The two mutants of item 2 are each killed (header).
- [ ] 3.3 (deploy/ci) Add the window of `walarchive` in the recovery to `test_reboot_windows.py` (design.md D2, D3 items 8 to 10):
  1. Add a class `RunbookRecoveryWalarchiveWindowTests`. Its method `test_recovery_window_keeps_walarchive_without_a_container` first only reads the two sections.
  2. Add a mutant test for the rows `h_recovery_step5_walarchive` and `h_recovery_step9_walarchive`. It expects `assertRaisesRegex` as task 3.1 does.
     It patches `runbook_heading_section` with the function of item 3 for the title "Recover from the WAL archive" (design.md D3 item 10).
  3. Add a second mutant test for the rows `h_restore_step3_walarchive`, `h_restore_step5_walarchive`, `h_restore_step6_walarchive`, and `h_restore_step7_prefix_walarchive`.
     It patches `runbook_heading_section` with a function for the title "Restore into production".
     The function keeps the original `runbook_heading_section` before the patch. It returns the mutant for the mutated title and the real text for each other title.
  4. Run `python3 -m unittest test_reboot_windows` and see the two mutant tests fail.
  5. Then write the window method. The recovery range holds steps 4 to 9 and the text of step 10 before its reference.
  6. The reference `Run step 7 of "Restore into production".` occurs once in step 10.
  7. The span `docker compose up -d --no-deps walarchive` occurs once in recovery step 7. The text before it and after it gives no hit.
  8. The method also reads restore steps 3, 5, and 6 whole. It reads restore step 7 up to the opening backtick of the span of sub-step 7.7.
     Each of these restore steps occurs once. The span `docker compose up -d --no-deps walarchive` occurs once in sub-step 7.7.
  9. The guard uses `services=("walarchive",)`. Each step number from 4 to 10 occurs once in the recovery range.
  10. Check: `python3 -m unittest test_reboot_windows` passes. The six mutants of items 2 and 3 are each killed (header).
- [ ] 3.4 (deploy/ci) Add the stop guards of the rollback and the recovery to `test_reboot_windows.py` (L-1, design.md D11):
  - First add fixture tests. `` Run `docker compose stop -t 30 backup`. `` fails the rollback guard, and `` Run `docker compose -f compose.yaml stop walarchive`. `` fails the recovery guard.
  - Run `python3 -m unittest test_reboot_windows` and see the fixture tests fail. The guard methods do not exist yet.
  - Then add the two guard methods. The rollback uses `services=("app", "backup")`. The recovery uses `services=("walarchive",)`. Both use `subcommands=("stop",)`.
  - Check: `python3 -m unittest test_reboot_windows` passes.
- [ ] 3.5 (deploy/ci) Add the text guard of `check_rollback_runbook` to `test_reboot_windows.py` (design.md D6, Q1 option A):
  - Add a class `RollbackCheckScriptTests`. Its method `test_check_rollback_runbook_keeps_backup_without_a_container` holds the guard.
  - First add a mutant test. It applies the edit of the row `g_ci_rollback_backup` to a copy of the function text (`upgrade.sh` line 651).
  - The test expects `assertRaisesRegex` with `up -d --no-deps backup`.
  - Add a regression test with the edit of the row `g_ci_rollback_continuation`. The text `compose \` and `up -d --no-deps backup` are on two lines.
    It also expects `assertRaisesRegex` with `up -d --no-deps backup`.
  - Run `python3 -m unittest test_reboot_windows` and see the two tests fail.
  - Then add the guard. The delete line (line 651) and the start line (line 663) each occur once in the function.
  - The guard joins each line that ends with a backslash with the next line (design.md D6 item 3).
  - It reads each joined line between the delete line and the start line with `shell_commands`. A first word `compose` starts a Compose call. A token `docker` starts a call of `docker`.
  - The hit list is exactly the command of line 659.
  - Check: `python3 -m unittest test_reboot_windows test_upgrade_runbook` passes. The mutants `g_ci_rollback_backup` and `g_ci_rollback_continuation` are each killed (header).

## 4. Records of `pitr-recovery`

- [ ] 4.1 (deploy/ci) (exempt: refactor, see the header) Move the records of `step_result` into its dict (F7, design.md D10):
  - `step_result` (`test_pitr_recovery.py` line 78) puts the records under the key `argv`. It sets no attribute on the process object.
  - Each caller reads `files["argv"]`. The callers are at lines 510, 512, 1092, 1241, 1362, 1558, and 1704.
  - Check: `rg -n 'done\.docker_calls' deploy/ci/test_pitr_recovery.py` prints nothing. `python3 -m unittest test_pitr_recovery` passes.
- [ ] 4.2 (deploy/ci) (exempt: hardening, see the header) Assert the exact hits of the mutant tests (F3, design.md D7):
  - `assert_window_mutant_rejected` (line 494) gets a new parameter `expected`, the expected call, for example `(True, "up", ["-d", "--no-deps", "app", "backup"])`.
  - It derives the log line from `expected`: a space, the subcommand and the operands joined by spaces, and a line break. This replaces the fixed text of line 509.
  - It compares the hits, read with `docker_call`, with `[expected]`. No text of `app` or `backup` stays fixed in the method.
  - The callers of lines 514 and 520 pass `(True, "up", ["-d", "--no-deps", "app", "backup"])`. Task 4.3 passes the call of `walarchive`.
  - `test_recovery_window_guard_rejects_runbook_mutants` (line 2146) uses `assertRaisesRegex` with the escaped inserted command.
  - Before the edit, build both probe copies (header). Run the RED commands below. Each prints `OK` now.
  - After the edit, build both probe copies again and run the same commands. Each must now print `FAILED`:

  ```bash
  cd "$copy_f3/deploy/ci"  # copy f3_structure_only
  python3 -m unittest test_pitr_recovery.StepTests.test_window_guard_rejects_crash_mutant test_pitr_recovery.StepTests.test_window_guard_rejects_check_prefix_mutant
  cd "$copy_f3r/deploy/ci"  # copy f3_runbook_structure_only
  python3 -m unittest test_pitr_recovery.RunbookRecoveryDeleteTests.test_recovery_window_guard_rejects_runbook_mutants
  ```

  - Check: `python3 -m unittest test_pitr_recovery` passes in the worktree.
- [ ] 4.3 (deploy/ci) Guard the records of the `walarchive` window (design.md D5):
  - First add a mutant test to `StepTests` (line 470). It applies the edit of the row `h_crash_walarchive` (anchor of `crash()` line 484).
  - The test uses the method of task 4.2 with `expected=(True, "up", ["-d", "--no-deps", "walarchive"])`.
  - It passes the guard `lambda records: self.walarchive_window_hits("crash", records)`. The method of task 4.2 calls `guard(records)` with one argument.
  - Its guard is a new method `walarchive_window_hits(step, records)` of `StepTests`.
  - Add one assertion `self.assertEqual(self.walarchive_window_hits(step, files["argv"]), [])` to each pass-path test:
    - `test_crash_pass_path_runs_the_steps_in_order` (line 1089) and `test_recover_pass_path_runs_the_steps_in_order` (line 1238)
    - `test_switch_pass_path_runs_the_steps_in_order` (line 1359) and `test_wal_line_pass_path_runs_the_steps_in_order` (line 1555)
  - Run `python3 -m unittest test_pitr_recovery` and see the new tests fail. The method does not exist yet.
  - Then add the method. It calls `forbidden_window_calls` with `services=("walarchive",)` and a slice of `record_window` (task 1.3):
    - `crash`: the slice after `rm -s -f walarchive`
    - `recover` and `switch`: the whole run
    - `wal-line`: the slice before `up -d --no-deps walarchive`
  - Do not edit `docker_calls.py`.
  - Check: `python3 -m unittest test_pitr_recovery` passes. The mutants `h_crash_walarchive` and `h_recover_walarchive` are each killed (header).
  - Check: a test kills the mutant `h_wal_line_prefix` (header). On `main`, only tests that pin the order kill it (design.md "Evidence").
  - Check: the message of `record_window` holds `up -d --no-deps walarchive` for that mutant. The file grows by at most 25 lines (design.md D13).

## 5. Tests of `restore-switch`

- [ ] 5.1 (deploy/ci) (exempt: hardening, see the header) Assert the exact hits of the mutant tests (F3, design.md D7):
  - `test_window_guard_rejects_restore_mutants` (`test_restore_switch.py` line 739) compares the hits, read with `docker_call`, with the one inserted call.
  - `test_window_guard_rejects_a_start_before_truncate` (line 1963) does the same with `forbidden_check_window_calls`.
  - `test_command_guard_rejects_restore_mutants` (line 713) uses `assertRaisesRegex` with the subcommand and the first operand of the inserted call.
  - `test_restore_window_guard_rejects_runbook_mutants` (line 3082) uses `assertRaisesRegex` with the escaped inserted command.
  - Before the edit, build both probe copies (header). Run the RED commands below. Each prints `OK` now.
  - After the edit, build both probe copies again and run the same commands. Each must now print `FAILED`:

  ```bash
  cd "$copy_f3/deploy/ci"  # copy f3_structure_only
  python3 -m unittest test_restore_switch.RestoreBehaviorTests.test_window_guard_rejects_restore_mutants test_restore_switch.CheckBehaviorTests.test_window_guard_rejects_a_start_before_truncate
  cd "$copy_f3r/deploy/ci"  # copy f3_runbook_structure_only
  python3 -m unittest test_restore_switch.RestoreBehaviorTests.test_command_guard_rejects_restore_mutants test_restore_switch.RunbookRestoreDeleteTests.test_restore_window_guard_rejects_runbook_mutants
  ```

  - Check: `python3 -m unittest test_restore_switch` passes in the worktree.
- [ ] 5.2 (deploy/ci) (exempt: refactor, see the header) Split `run_subcommand` (F7, design.md D10):
  - `run_subcommand` (line 559) keeps its signature and its three return values.
  - Free functions take the state and tree files, the stubs, the environment, and the collection of the files.
  - Check: an `ast` count gives at most 50 lines for `run_subcommand` and for each new function. `python3 -m unittest test_restore_switch` passes.

## 6. Documents

- [ ] 6.1 (docs/ops) [mechanical] (exempt: documentation only) Use one name for a restart of Docker (L-5, design.md D12):
  - Order: task 6.1 runs first of the three runbook tasks 6.1, 6.3, and 6.4 (Run order item 7).
  - In `docs/ops/runbook.md` lines 1638 and 1640, replace "a restart of the container engine" with "a restart of Docker".
  - Check: `rg -c 'container engine' docs/ops/runbook.md` prints nothing. The `ste-lint.py` command of `CLAUDE.md` reports 0 hard violations.
- [ ] 6.2 (docs/adr) [mechanical] (exempt: documentation only) Name the amended items of ADR-0176 (L-4, design.md D12):
  - In `docs/adr/README.md` line 188, the status cell becomes "Accepted. Amends ADR-0175 (h), (i), (j), (k), ADR-0106 (i) step 4 and (p) step 5, and ADR-0174 (aj)".
  - In the decision map (line 205), the text of 0176 names the same items.
  - Check: the `ste-lint.py` command of `CLAUDE.md` reports 0 hard violations.
- [ ] 6.3 (deploy/ci) Keep one delete of the new volume in the recovery (L-2, Q2 option A):
  - Order: task 6.3 starts after tasks 6.1 and 3.5 are complete. Task 6.4 waits for it.
  - First add a test to `test_reboot_windows.py`. Recovery step 6 holds no `docker volume` delete. Step 7 holds `To end the attempt, delete the new volume.`
  - Split the text of the delete into two literals (header, rule of `deploy_values`).
  - Run `python3 -m unittest test_reboot_windows` and see the test fail.
  - Then change `docs/ops/runbook.md` line 1008 to "After a failed run, end the attempt as in step 7."
  - Check: `python3 -m unittest test_reboot_windows test_pitr_recovery` passes. The `ste-lint.py` command reports 0 hard violations.
- [ ] 6.4 (deploy/ci) Name the major upgrade steps and the rollback steps in "After a host reboot" (L-3, Q3 option A):
  - Order: task 6.4 starts after task 6.3 is complete. It is the last of the three runbook tasks.
  - First add the two sentences of Q3 option A to `RunbookHostRebootTests.required_sentences` (`test_upgrade_runbook.py` line 173).
  - Run `python3 -m unittest test_upgrade_runbook` and see `test_reboot_windows_have_conditional_wait_and_skip` fail.
  - Then add the two sentences after the two cases of the section (`docs/ops/runbook.md` line 497).
  - Check: `python3 -m unittest test_upgrade_runbook` passes. The paragraph and list limits of the class pass. The `ste-lint.py` command reports 0 hard violations.
- [ ] 6.5 (CHANGELOG.md) [mechanical] (exempt: documentation only) Add one line under `### Changed` of `[Unreleased]`:
  - The line says that the guard tests of `deploy/ci/` keep `app`, `backup`, and `walarchive` without a container in the windows of ADR-0176.
  - Check: `python -I tools/ste-lint/ste-lint.py CHANGELOG.md` reports 0 hard violations.

## 7. Gate

- [ ] 7.1 (deploy/ci) Run the local checks:
  1. `bash deploy/ci/check.sh` passes. It runs each `deploy/ci/test_*.py`, also `test_reboot_windows.py`.
  2. `python3 -O -m unittest test_docker_calls` passes in `deploy/ci`.
  3. `cargo test -p recipe-server --test deploy_values --locked` passes.
  4. `openspec validate guard-reboot-window-starts --strict` reports valid.
  5. The `ste-lint.py` command of `CLAUDE.md` reports 0 hard violations. Run it on `openspec/changes/guard-reboot-window-starts/*.md` too.
  6. Run each mutant of design.md "Evidence" again in a scratch copy, as the header says. A test kills each of the 20 mutants:
     - A test of the guard class of its row fails.
     - The failure output of that test holds the inserted command. For a script mutant, it holds the subcommand and each later word of the inserted call.
     - A failure of a structural test alone does not count. Record the name of the failed guard test for each mutant.
  7. `wc -l deploy/ci/*.py` shows no file of groups 1 to 3 over 800 lines. The new file `test_reboot_windows.py` is one of these files.
  8. No script of `deploy/ci/` changes, so the CI job `ci-scripts` of the pull request runs the tests. The change needs no dispatch of `upgrade`, `restore-switch`, or `pitr-recovery`.

## Workflow follow-up

- After the gate, run `code-review`, fix each CRITICAL, HIGH, and MEDIUM finding, and run `openspec-archive-change`.
