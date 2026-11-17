# Tasks

Each task is for one agent. Codex `gpt-6.1-sol` implements each task in `deploy/ci/`. Haiku does each `[mechanical]` task.

Each test task first writes a failing test and runs it (RED). Then it writes the change that makes the test pass (GREEN). Run each Python suite from `deploy/ci`, for example `cd deploy/ci && python3 -m unittest test_docker_calls test_upgrade_runbook`.

The RED test of each task is a regression case. It gives the guard the input of a finding and expects a hit or an `AssertionError`. It fails on the current guard.

Each GREEN check also runs the mutants that the task names, in a scratch copy of the worktree. Each named mutant is a row of design.md "Evidence". Never change the worktree for a mutant. The mutant now makes at least one test fail.

Task 5.1 is exempt from the failing test. It changes documentation only. `ste-lint.py` checks it.

Two more exceptions apply to the failing test:

- Task 1.1 adds regression cases that pass now: the `label=a#b` case and the five cases that the task marks. They keep the current hits.
- Task 6.1 is the gate of the change. It runs checks and adds no test.

The helper file is `deploy/ci/docker_calls.py` (design.md "Decisions of the owner", Q1). Its unit tests are in `deploy/ci/test_docker_calls.py`.
design.md D3 says which helper goes into `docker_calls.py` and which helper stays in `test_upgrade_runbook.py`.

Each task that adds or changes a helper of `docker_calls.py` first writes a failing helper test in `test_docker_calls.py`. Then it writes its failing suite test.

The four suites are `test_docker_calls`, `test_upgrade_runbook`, `test_restore_switch`, and `test_pitr_recovery`.

`deploy/ci/check.sh` runs each test file with `python3 -I`, so a test loads `docker_calls.py` with `importlib.util.spec_from_file_location`.

Three rules of `crates/recipe-server/tests/deploy_values.rs` apply to each line of `deploy/ci/`, test files too:

- The word `volume` is never directly followed by the word `rm` or `prune`. Split such a test text into two literals, as `test_upgrade_runbook.py` line 409 does.
- A line with the word `down` holds no later `--volumes` and no later short option with the letter `v`.
- No line holds `|| exit 1` or `*) exit 1`. A stub case exits with `exit "${STUB_<NAME>_STATUS:-0}"`.

The line numbers refer to `main` 2324454.

## Run order

1. Tasks 1.1, 1.2, 1.3, 1.4, and 1.5 edit `deploy/ci/test_upgrade_runbook.py` and run in this order.
2. Tasks 1.1, 1.2, and 1.5 also edit `docker_calls.py` and `test_docker_calls.py`. Task 1.1 creates the two files.
3. Task 2.1 runs after task 1.5. It uses the parser of task 1.2. It also edits `docker_calls.py` and `test_docker_calls.py`.
4. Tasks 2.1, 2.2, and 2.3 edit `deploy/ci/test_restore_switch.py` and run in this order. Task 2.3 also edits `docker_calls.py` and `test_docker_calls.py`.
5. Tasks 1.1 to 2.3 run one at a time. So the GREEN and mutant checks of these tasks run the four suites.
6. Each RED command of tasks 1.1 to 2.3 also runs `test_docker_calls`.
7. Tasks 3.1, 4.1, and 4.2 are the parallel group. Each of them starts after task 2.3.
8. Task 4.2 runs after task 3.1, because both edit `test_pitr_recovery.py`. Task 4.1 can run in parallel with tasks 3.1 and 4.2.
9. A task of the parallel group edits only its own suite. It does not edit `docker_calls.py`. Its RED, GREEN, and mutant checks run only its suite.
10. After the parallel group, the orchestrator runs `python3 -m unittest test_docker_calls test_upgrade_runbook test_restore_switch test_pitr_recovery` once. The four suites pass.
11. Task 5.1 can run in parallel with groups 1 to 4. Its file is not in those groups.
12. Task 6.1 runs last. It is the gate of the change.

## 1. Runbook guard

- [x] 1.1 (deploy/ci) First create `deploy/ci/test_docker_calls.py` with tests of the lexer function of design.md D2:
  - The file loads `deploy/ci/docker_calls.py` with `importlib` and ends with `unittest.main()` (design.md D3).
  - `$(docker compose start app)` gives exactly the simple commands `$` and `docker compose start app`.
  - `docker compose start app&&echo` gives exactly the simple commands `docker compose start app` and `echo`.
  - `true&&(docker compose start app);` gives the two simple commands `true` and `docker compose start app` (N3).
  - `docker compose start 2>/dev/null >>log app` gives the simple command `docker compose start app`.
  - `-f 'a;b'` keeps the word `a;b`. An unclosed quote gives the split at white space.
  - Then add regression cases to `RunbookDeletedServiceStartsTests` (line 163) of `deploy/ci/test_upgrade_runbook.py` (RDAB-D2, T-01):
  - Each of these spans gives one hit: `docker compose start backup;`, `docker compose start app;`, `docker compose start app&&echo`, and `$(docker compose start app)`.
  - The span `true&&(docker compose start app);` gives one hit. The current guard gives no hit (N3).
  - The span `docker compose start >/dev/null app` gives one hit. Bash passes `app` after the redirection to Compose.
  - Move the four redirection cases of `test_shell_operators_end_the_service_list` (line 207) into a new test. Each case gives one hit.
  - The span `docker compose ps --filter label=a#b && docker compose start app` gives one hit. It passes now and fails with `commenters = "#"`.
  - The spans `RECIPE_VERSION=<previous tag> docker compose start app` and `docker compose --env-file <file> start app` each give one hit.
  - The span `docker compose start 2>/dev/null app` gives one hit.
  - The spans `docker compose exec -T db psql <dump.sql && docker compose start app >/dev/null` and `cat <in.txt | docker compose start app >/dev/null` each give one hit.
  - These five cases pass now. They keep the current hits after the change (design.md D2).
  - Run `python3 -m unittest test_docker_calls test_upgrade_runbook` and see the new tests fail.
  - The helper tests fail, because `docker_calls.py` does not exist yet.
  - Then create `docker_calls.py` with the lexer function of design.md D2.
  - The function reads one command with `shlex.shlex(command, posix=True, punctuation_chars=True)`.
  - Set `whitespace_split = True` and `commenters = ""` on the lexer.
  - A control token consists only of `;`, `&`, `|`, `(`, and `)`. Split the tokens into simple commands at each control token.
  - A redirection token consists only of `<`, `>`, and `&`. Skip it and its target word, then read the next operands.
  - Also drop a word of digits directly before a redirection token.
  - Keep the whitespace fallback for `ValueError`.
  - Then change `runbook_deleted_service_starts` (line 123). First replace each placeholder of design.md D2 with one word.
  - Then read each command with the lexer function. The placeholder step stays in `test_upgrade_runbook.py` (design.md D3).
  - Check: the four suites pass. The control-operator cases of `test_shell_operators_end_the_service_list` stay unchanged. The cases of `test_allowed_commands` stay unchanged.
  - Check: the mutants `d2_semicolon`, `t01_amp`, `t01_subst`, and `n1_rollback_placeholder` each fail at least one test.
- [x] 1.2 (deploy/ci) First add tests of a parser of a `docker` call to `deploy/ci/test_docker_calls.py` (design.md D3):
  - The cases are a compose call, a call without `compose`, the form `--option=value`, and a missing subcommand.
  - Then add regression cases to `RunbookDeletedServiceStartsTests` of `deploy/ci/test_upgrade_runbook.py` (T-01):
  - Each of these spans gives one hit: `docker --context x compose start app`, `docker -c x compose start app`, `docker --host=unix:///x compose restart backup`, and `docker -H x compose start app`.
  - `docker --context x compose logs app` gives no hit.
  - Run `python3 -m unittest test_docker_calls test_upgrade_runbook` and see the new tests fail.
  - Then add the parser to `docker_calls.py`. It reads the arguments after the program `docker`.
  - The function skips the global options of `docker` and of Compose. It returns the subcommand and its operands.
  - The function holds the one table of value options for each program (design.md D3). `runbook_deleted_service_starts` uses it. The token `docker-compose` starts at the options of Compose.
  - Check: the four suites pass, and the mutant `t01_context` fails at least one test.
- [x] 1.3 (deploy/ci) First add a regression case to `RunbookDeletedServiceStartsTests` (T-01, design.md D4):
  - A `text` fence and a `bash` fence each hold the two lines `docker compose \` and `  start app`. Each gives one hit.
  - Run `python3 -m unittest test_docker_calls test_upgrade_runbook` and see the case fail.
  - Then join a fenced line that ends with `\` and the next fenced line before the guard reads them.
  - The join stays in `test_upgrade_runbook.py`, because it reads runbook text (design.md D3).
  - Check: the four suites pass, and the mutant `t01_fence_split` fails at least one test.
- [x] 1.4 (deploy/ci) First add regression cases to `RunbookInlineCodeSpansTests` (line 242) (T-03, design.md D8):
  - The two lines ``- Type a lone ` here.`` and ``- Run `docker compose start app`.`` give the span `docker compose start app`, and the guard gives one hit.
  - The same holds for the markers `*`, `+`, `1.`, `1)`, and for a nested item `   2.`.
  - An ATX heading line between two backticks ends the first block.
  - An ATX heading is a block of one line. A backtick in a heading does not pair with a backtick of the next line (L1).
  - So the lines ``Text ` here``, ``## Heading ` x``, and ``Run `docker compose start app`.`` give exactly the span list `['docker compose start app']`.
  - The two lines ``## Heading ` x`` and ``Run `docker compose start app`.`` give one hit.
  - The table rows ``| a ` b |`` and ``| Run `docker compose start app` |`` give one hit (L1).
  - A list marker needs a space or the end of the line after it (L1).
  - So a span that wraps to a line that starts with `--no-deps app` stays one span.
  - Run `python3 -m unittest test_docker_calls test_upgrade_runbook` and see the new cases fail.
  - Then change `runbook_inline_code_spans` (line 97) with the rules of design.md D8.
  - A line that starts a list item, an ATX heading, or a table row starts a new block. A heading also ends its block.
  - Check: the four suites pass. The existing case `test_span_can_cross_a_line_break` stays unchanged.
  - Check: the mutant `t03_lone_backtick` fails at least one test.
- [x] 1.5 (deploy/ci) First add tests of the rule of refused calls of design.md D7 to `deploy/ci/test_docker_calls.py`:
  - The arguments after `compose` of `up -d --no-deps --wait-timeout 180` give a hit. With `proxy` after `180`, they give no hit.
  - `up -d` gives a hit. `run --rm --no-deps -w proxy` gives a hit, because `proxy` is the value of `-w` in `run`.
  - `up -d --no-deps -w proxy` gives no hit, because `-w` is a flag in `up`.
  - `create --no-deps --pull=never proxy` gives no hit, because the form `--pull=never` takes no second token.
  - Then add tests to `RunbookDeletedServiceSelectionTests` (line 220) of `deploy/ci/test_upgrade_runbook.py` for a new parameter `subcommands` (T-02):
  - With `subcommands=("up", "create", "run", "start", "restart")`, each of these spans gives one hit:
    - `docker compose up -d --no-deps app backup` and `docker compose run --rm backup sh`
    - `docker compose up -d` and `docker compose up -d proxy`, because they lack `--no-deps`
    - `docker compose up -d --no-deps --wait-timeout 180` and `docker compose create --no-deps --pull never`, because they name no service
    - `docker compose run --rm --no-deps -v x:/data`, because it names no service
  - With the same value, `docker compose up -d --no-deps proxy` and `docker compose start` give no hit.
  - With the same value, `docker compose up -d --no-deps --wait --wait-timeout 180 proxy` gives no hit.
  - With the same value, restore step 4 of design.md D7 has a `run` with the quoted word `"<new volume>:/var/lib/postgresql/data"`.
  - That `run` gives no hit. The quoted word is not a redirection (design.md D2).
  - With the same value, `(docker compose up -d);` gives one hit. The control token `);` is not a service (N3, design.md D2).
  - Without the parameter, `docker compose up -d --no-deps app` and `docker compose up -d` give no hit.
  - Run `python3 -m unittest test_docker_calls test_upgrade_runbook` and see the new tests fail.
  - Then add `subcommands=("start", "restart")` to `runbook_deleted_service_starts`.
  - When the value holds `up`, `create`, or `run`, these subcommands also need `--no-deps` and a service (design.md D7).
  - Add the table of value options of `up`, `create`, and `run` (design.md D7) to `docker_calls.py`.
  - Add the rule of refused calls to `docker_calls.py` too. `runbook_deleted_service_starts` calls it.
  - The operands are the words that remain after the options and the values of the value options.
  - Check: the four suites pass. No runbook test uses the parameter yet, so task 4.1 runs the mutant `t02_runbook_step3` against it.

## 2. Stub-log guards of `restore-switch`

- [x] 2.1 (deploy/ci) First add tests for the argument records of design.md D5 (RDAB-D3):
  - Start with unit tests of a record parser in `deploy/ci/test_docker_calls.py`. The records hold an argument with spaces, an empty argument, and an argument with a newline.
  - A truncated record raises `ValueError`.
  - In `test_restore_switch.py`, add a test that runs `DOCKER_SWITCH_STUB` with the arguments `a b`, an empty argument, and `x` newline `y`.
  - That test compares the bytes of `$STUB_LOG.argv` with `b"3\x00a b\x00\x00x\ny\x00"`.
  - In `RestoreBehaviorTests` (line 618), add a test like `test_command_guard_rejects_restore_mutants` (line 663).
  - The mutant line is `compose -f "/tmp/start in checkout/compose.yaml" start app || true`. The guard reads the records of the run and raises `AssertionError`.
  - Add a test that gives the guard the empty record list of a whole run. The guard raises `AssertionError` (design.md D5).
  - Run `python3 -m unittest test_docker_calls test_upgrade_runbook test_restore_switch` and see the new tests fail.
  - Then add the record parser to `docker_calls.py`. Add `printf '%s\0' "$#" "$@" >>"$STUB_LOG.argv"` to `DOCKER_SWITCH_STUB` (line 383).
  - Put the record line as the first command of the stub, before each `case` and each `exit` (N5).
  - `DOCKER_SWITCH_STUB` is a raw string, so the line goes in as it is.
  - `run_subcommand` (line 534) puts the parsed records into its dict `files` under the key `argv`. It puts `env.log` there in the same way now (line 600).
  - The three return values and the text log stay unchanged, because many callers unpack them.
  - `assert_no_forbidden_docker_commands` (line 619) takes a list of argument lists. It raises `AssertionError` on an empty record list of the whole run.
  - The guard uses the function of task 1.2 and drops its own table of value options.
  - The string cases of lines 639 to 661 pass `shlex.split` of each line. The adapter drops a leading `docker` token, as line 624 does now.
  - Lines 646 and 661 need that drop, because their strings start with `docker`.
  - `test_restore_runs_the_steps_in_order` (line 690) and line 683 pass the records.
  - Check: the four suites pass, and the mutant `d3_quoted` fails at least one test.
- [x] 2.2 (deploy/ci) First extend two tests of `RestoreBehaviorTests` (T-05, design.md D6):
  - `test_command_guard_rejects_forbidden_docker_commands` (line 643) adds `start x`, `restart x`, `rm -f x`, and `container` with `start`, `restart`, `kill`, and `rm -f`.
  - The same test adds `compose stop db`, `compose rm -s -f db`, `compose rm -s -f app db`, and `compose rm -s -f`.
  - `test_command_guard_rejects_restore_mutants` (line 663) adds `docker start recipe-ci-1-1-app-1`, `docker container start recipe-ci-1-1-app-1`, and `docker rm -f recipe-ci-1-1-db-1`.
  - The same test adds `compose rm -s -f db` and `compose stop db`.
  - A new case: `run --rm --network none x`, `volume create x`, `inspect x`, and `compose rm -s -f app backup` pass the guard.
  - Run `python3 -m unittest test_docker_calls test_restore_switch` and see the extended tests fail.
  - Then extend the branch of the guard for calls outside Compose. The guard of the step `restore` stays in `test_restore_switch.py`.
  - In the branch for Compose, refuse `stop`. Let `rm` name only `app` and `backup`, and at least one of them.
  - Check: the four suites pass. The mutants `t05_docker_start`, `t05_container_start`, `t05_rm_db`, `rm_db`, and `stop_db` each fail at least one test.
- [x] 2.3 (deploy/ci) First add a unit test of a window function to `deploy/ci/test_docker_calls.py` (T-02, design.md D7):
  - The test gives the record `compose up -d --no-deps --wait-timeout 180`. The function returns that record.
  - With the service `proxy` after `180`, the function returns no record.
  - The function raises `AssertionError` when the record list of the whole run is empty (N4).
  - An empty window slice of a run with records gives no `AssertionError`.
  - Then add a test like `test_command_guard_rejects_restore_mutants` to `RestoreBehaviorTests` of `deploy/ci/test_restore_switch.py`:
  - The mutant lines are `compose up -d --no-deps app backup`, `compose up -d`, and `compose up -d proxy`. Each goes after the anchor of line 664.
  - A fourth mutant line is `compose up -d --no-deps --wait-timeout 180`. It names no service.
  - A window guard reads the records of each run and raises `AssertionError`.
  - Add a mutant test to `CheckBehaviorTests` (line 1875). It puts `compose up -d --no-deps app backup || true` before the TRUNCATE of `check()` (line 303).
  - The window guard reads the records of the step `check` on that copy and raises `AssertionError`.
  - Add the window guard to the pass-path tests of the steps in the window:
    - `test_restore_runs_the_steps_in_order` (line 690)
    - `test_switch_runs_the_steps_of_adr_0103_f_in_order` (line 863)
    - `test_access_checks_before_the_passwords_and_logs_in_after` (line 1218)
    - `test_wal_line_runs_the_steps_in_order_and_deletes_nothing` (line 1769)
    - `test_check_runs_every_step_in_order` (line 1895), with the window ends of `check` in design.md D7
  - Run `python3 -m unittest test_docker_calls test_upgrade_runbook test_restore_switch` and see the new tests fail.
  - Then add the window function to `docker_calls.py`. It returns each record that design.md D7 refuses.
  - The function uses the table of value options and the rule of refused calls of task 1.5.
  - The function raises `AssertionError` when the record list of the whole run is empty. An empty window slice is valid.
  - For the step `check`, it reads the records up to the end records of design.md D7. It asserts that these end records exist.
  - Add the record line of task 2.1 to each Docker stub that these five tests use. Examples are `DOCKER_ACCESS_STUB` (line 1179) and `DOCKER_WAL_STUB` (line 1338).
  - Put the record line as the first command of each stub, before each `case` and each `exit` (N5).
  - Check: the four suites pass. The mutants `t02_restore`, `up_all`, `up_proxy`, and `t02_check_switch` each fail at least one test.

## 3. Stub-log guard of `pitr-recovery`

- [x] 3.1 (deploy/ci) First add mutant tests to `deploy/ci/test_pitr_recovery.py` (T-02, design.md D5, D7):
  - Copy `pitr-recovery.sh` and its sourced files to a temporary directory.
  - The anchor is the line `compose rm -s -f app backup || fail "crash: cannot delete app and backup."` of `crash()`, with its two leading spaces (L4).
  - Assert that the anchor occurs once in the script. Then add `compose up -d --no-deps app backup` after the anchor of the copy.
  - Run the step `crash` on the copy. The window guard of task 2.3 reads the records of the run and raises `AssertionError`.
  - A second copy gets `compose up -d --no-deps app backup || true` before the TRUNCATE of `check()` (line 624).
  - Run the step `check` on the second copy. The window guard raises `AssertionError`.
  - Add a test that runs `STEP_DOCKER_STUB` with the arguments `a b`, an empty argument, and `x` newline `y`.
  - That test compares the bytes of `$STUB_LOG.argv` with `b"3\x00a b\x00\x00x\ny\x00"`.
  - Add the window guard to these pass-path tests:
    - `test_crash_pass_path_runs_the_steps_in_order` (line 1031)
    - `test_recover_pass_path_runs_the_steps_in_order` (line 1179)
    - `test_switch_pass_path_runs_the_steps_in_order` (line 1299)
    - `test_wal_line_pass_path_runs_the_steps_in_order` (line 1494)
    - `test_check_pass_path_runs_the_steps_in_order` (line 1639), with the window ends of `check` in design.md D7
  - Run `python3 -m unittest test_pitr_recovery` and see the new tests fail.
  - Then add the record line to `STEP_DOCKER_STUB` (line 2273). It is a plain string with `\\n` escapes.
  - So its Python text is `printf '%s\\0' "$#" "$@" >>"$STUB_LOG.argv"` (design.md D5). Never write `\0` there.
  - Put the record line as the first command of the stub, before each `case` and each `exit` (N5).
  - `run_step` (line 468) gives the parsed records to the test. Keep its three return values, because many callers unpack them.
  - Load `docker_calls.py` with `importlib`. Do not edit `docker_calls.py` or `test_docker_calls.py`.
  - Check: `python3 -m unittest test_pitr_recovery` passes (Run order, step 9). The mutants `t02_crash` and `t02_check_pitr` each fail at least one test.

## 4. Runbook windows

- [x] 4.1 (deploy/ci) Add a window test to `RunbookRestoreDeleteTests` in `deploy/ci/test_restore_switch.py` (T-02, design.md D7). Edit only this file:
  1. Keep `test_section_has_no_start_app_or_start_backup_command` (line 2902) and its fixture tests unchanged.
  2. The fixture `test_section_guard_allows_bare_start_and_other_services` (line 2917) patches a section of two sentences. It stays valid.
  3. First add a new method `test_restore_window_keeps_app_and_backup_without_a_container`. Its first body only calls the existing section test.
  4. Add a mutant test. It reads the real section, then inserts one text after an anchor that occurs once.
  5. The mutant test patches `runbook_heading_section` with that full section and expects `AssertionError` from the new method.
  6. The mutant cases are the inserts of the rows of design.md "Evidence":
     - `t02_runbook_step3` and `t02_runbook_step9_2`
     - `t02_runbook_step9_4_prefix` and `t02_runbook_step9_4_backup`
  7. Each mutant keeps the structure of the section, so each end text of design.md D7 stays.
  8. Run `python3 -m unittest test_restore_switch` and see the mutant test fail. The existing assertion reads only `start` and `restart`.
  9. Then replace the body of the new method with the windows of design.md D7 and the `subcommands` of task 1.5.
  10. The method asserts that each end span occurs once in its sub-step.
  11. In the windows, the text outside step 4 gives no hit. Step 4 gives the four commands of design.md D7, in that order.
  12. Run `python3 -m unittest test_restore_switch` and see the mutant test pass.
  13. Check: the four mutants of item 6 each fail at least one test of `test_restore_switch`.
- [x] 4.2 (deploy/ci) Add a window test to `RunbookRecoveryDeleteTests` in `deploy/ci/test_pitr_recovery.py` (T-02, design.md D7). Edit only this file:
  1. Keep `test_section_has_no_start_of_a_deleted_service` (line 2043) and its fixture tests unchanged.
  2. The fixture `test_section_guard_allows_bare_start_and_other_services` (line 2059) patches a section of two sentences. It stays valid.
  3. First add a new method `test_recovery_window_keeps_app_and_backup_without_a_container`. Its first body only calls the existing section test.
  4. Add a mutant test. It reads the real section, then inserts one text after an anchor that occurs once.
  5. The mutant test patches `runbook_heading_section` with that full section and expects `AssertionError` from the new method.
  6. The mutant cases are the inserts of the rows `t02_runbook_recovery_step5` and `t02_runbook_recovery_step11_2` of design.md "Evidence".
  7. Each mutant keeps the structure of the section, so the end text of design.md D7 stays.
  8. Run `python3 -m unittest test_pitr_recovery` and see the mutant test fail. The existing assertion reads only `start` and `restart`.
  9. Then replace the body of the new method with the window of design.md D7 and the `subcommands` of task 1.5.
  10. The method asserts that the reference to restore step 9 occurs once in sub-step 11.2.
  11. The window holds the text of sub-step 11.2 before that reference. The window gives no hit.
  12. Run `python3 -m unittest test_pitr_recovery` and see the mutant test pass.
  13. Check: the two mutants of item 6 each fail at least one test of `test_pitr_recovery`.

## 5. Other documents

- [x] 5.1 (CHANGELOG.md) [mechanical] (exempt: documentation only, see the header) Add one line under `### Changed` of `[Unreleased]`:
  - The line says that the guard tests of `deploy/ci/` read more shell forms and lossless stub records.
  - It also says that they keep `app` and `backup` without a container in the restore windows.
  - The line cites ADR-0175.
  - Check: `python -I tools/ste-lint/ste-lint.py CHANGELOG.md` reports 0 hard violations.

## 6. Gate

- [ ] 6.1 (deploy/ci) Run the local checks and the CI dispatch:
  1. `bash deploy/ci/check.sh` passes. It runs each `deploy/ci/test_*.py`.
  2. `cargo test -p recipe-server --test deploy_values --locked` passes.
  3. `openspec validate harden-restore-guard-tests --strict` reports valid.
  4. The `ste-lint.py` command of `CLAUDE.md` reports 0 hard violations. Run it with `globstar` on `openspec/changes/harden-restore-guard-tests/**/*.md` too.
  5. Run each mutant of design.md "Evidence" again in a scratch copy. Each mutant fails at least one test.
  6. On the order of the owner, push the branch with `git push -u origin test/harden-restore-guard-tests`. The dispatch runs the commit of the remote branch.
  7. Run `gh workflow run CI --ref test/harden-restore-guard-tests`. The jobs `upgrade`, `restore-switch`, and `pitr-recovery` do not run on a pull request (ADR-0094).
  8. The three jobs pass in that run. Record the URL of the run in the pull request.

## Workflow follow-up

- After the gate, run `code-review`, fix each CRITICAL, HIGH, and MEDIUM finding, and run `openspec-archive-change`.

## Code review fixes (2026-11-17)

- The review found no CRITICAL or HIGH finding. Codex `gpt-6.1-sol` fixed each MEDIUM finding with a failing test first.
- `deploy/ci/test_upgrade_runbook.py` had more than 800 lines. The test classes of the runbook text helpers moved to the new file `deploy/ci/test_runbook_text.py`. The helpers stay in `test_upgrade_runbook.py` (design.md D3).
- `shell_commands` also reads a command substitution inside quotes and a backtick substitution.
- The rule of design.md D7 counts only a real `--no-deps` flag, not the value of a value option.
- The runbook window tests refuse a repeated step number, and they cut a window at the real range of its end span.
