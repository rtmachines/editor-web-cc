# Design

## Context

Three suites in `deploy/ci/` guard the restore paths of ADR-0103 (b), ADR-0106 (i), ADR-0175, and ADR-0176:

- `test_upgrade_runbook.py` holds the runbook helpers. `runbook_deleted_service_starts(section, services=("app", "backup"))` reads inline code spans and fenced lines. It reports `docker compose start` or `restart` of a deleted service.
- `test_restore_switch.py` runs `restore-switch.sh` with Docker stubs. `RestoreBehaviorTests.assert_no_forbidden_docker_commands` reads the stub log of the step `restore`.
- `test_pitr_recovery.py` runs `pitr-recovery.sh` with Docker stubs.
- `RunbookRestoreDeleteTests` and `RunbookRecoveryDeleteTests` load `test_upgrade_runbook.py` with `importlib`. `check.sh` runs each test file with `python3 -I`, so a plain `import` of a sibling file fails.

Each Docker stub writes `echo "$*" >>"$STUB_LOG"`. Many tests match this text log with regular expressions.

### Evidence on `main` 2324454

All runs used a copy of the worktree under the scratch directory. The baseline run printed `Ran 532 tests in 20.527s` and `OK`.
Each mutant changed one file of the copy and ran `python3 -m unittest test_upgrade_runbook test_restore_switch test_pitr_recovery`.
The Opus plan review ran the mutants `up_all`, `up_proxy`, `rm_db`, and `stop_db`. The first update of this plan ran the rows from `t02_check_switch` to `t03_lone_backtick`.
The second update ran the last four rows in a fresh `git archive` copy. Its baseline run printed `Ran 532 tests in 20.501s` and `OK`.

| Finding | Name | Mutant | Result |
|---|---|---|---|
| RDAB-D2 | `d2_semicolon` | Recovery step 7 gets `` `docker compose start backup;` `` | `== d2_semicolon: Ran 532 tests in 25.258s \| OK` |
| T-01 | `t01_amp` | Recovery step 7 gets `` `docker compose start app&&echo` `` | `== t01_amp: Ran 532 tests in 25.153s \| OK` |
| T-01 | `t01_context` | Recovery step 7 gets `` `docker --context x compose start app` `` | `== t01_context: Ran 532 tests in 25.200s \| OK` |
| T-01 | `t01_subst` | Recovery step 7 gets `` `$(docker compose start app)` `` | `== t01_subst: Ran 532 tests in 25.250s \| OK` |
| T-01 | `t01_fence_split` | Recovery step 7 gets a `text` fence with `docker compose \` and `start app` | `== t01_fence_split: Ran 532 tests in 25.244s \| OK` |
| RDAB-D3 | `d3_quoted` | `restore()` gets `compose -f "/tmp/start in checkout/compose.yaml" start app \|\| true` | `== d3_quoted: Ran 532 tests in 27.221s \| OK` |
| T-02 | `t02_restore` | `restore()` gets `compose up -d --no-deps app backup` after the delete | `== t02_restore: Ran 532 tests in 27.236s \| OK` |
| T-02 | `t02_crash` | `crash()` gets `compose up -d --no-deps app backup` after the delete | `== t02_crash: Ran 532 tests in 27.243s \| OK` |
| T-02 | `t02_runbook_step3` | Restore step 3 gets `` `docker compose up -d --no-deps app backup` `` | `== t02_runbook_step3: Ran 532 tests in 27.157s \| OK` |
| T-02 | `up_all` | `restore()` gets `compose up -d` after the delete | `== up_all: Ran 532 tests in 20.874s OK` |
| T-02 | `up_proxy` | `restore()` gets `compose up -d proxy` after the delete | `== up_proxy: Ran 532 tests in 22.737s OK` |
| T-05 | `t05_docker_start` | `restore()` gets `docker start recipe-ci-1-1-app-1 \|\| true` | `== t05_docker_start: Ran 532 tests in 27.234s \| OK` |
| T-05 | `t05_container_start` | `restore()` gets `docker container start recipe-ci-1-1-app-1 \|\| true` | `== t05_container_start: Ran 532 tests in 27.285s \| OK` |
| T-05 | `t05_rm_db` | `restore()` gets `docker rm -f recipe-ci-1-1-db-1 \|\| true` | `== t05_rm_db: Ran 532 tests in 27.241s \| OK` |
| T-05 | `rm_db` | `restore()` gets `compose rm -s -f db` after the delete | `== rm_db: Ran 532 tests in 20.590s OK` |
| T-05 | `stop_db` | `restore()` gets `compose stop db` after the delete | `== stop_db: Ran 532 tests in 20.462s OK` |
| T-02 | `t02_check_switch` | `check()` of `restore-switch.sh` gets `compose up -d --no-deps app backup \|\| true` before the TRUNCATE | `== restore-switch.sh check prefix: Ran 532 tests in 21.057s \| OK` |
| T-02 | `t02_check_pitr` | `check()` of `pitr-recovery.sh` gets the same line before the TRUNCATE | `== pitr-recovery.sh check prefix: Ran 532 tests in 21.505s \| OK` |
| T-02 | `t02_runbook_step9_2` | Restore sub-step 9.2 gets `` Then run `docker compose up -d --no-deps app`. `` | `== t02_runbook_step9_2: Ran 532 tests in 20.922s \| OK` |
| T-02 | `t02_runbook_recovery_step5` | Recovery step 5 gets `` Run `docker compose up -d --no-deps app`. `` | `== t02_runbook_recovery_step5: Ran 532 tests in 20.926s \| OK` |
| T-03 | `t03_lone_backtick` | Restore step 3 gets the two list items of the T-03 probe below | `== t03_lone_backtick: Ran 532 tests in 22.440s \| OK` |
| T-02 | `t02_runbook_step9_4_prefix` | Restore sub-step 9.4 gets `` Run `docker compose up -d --no-deps backup`. `` before the start of `app` | `== t02_runbook_step9_4_prefix: \| Ran 532 tests in 20.499s \| OK` |
| T-02 | `t02_runbook_step9_4_backup` | Restore sub-step 9.4 gets `` Then run `docker compose up -d --no-deps backup`. `` after the start of `app` | `== t02_runbook_step9_4_backup: \| Ran 532 tests in 20.492s \| OK` |
| T-02 | `t02_runbook_recovery_step11_2` | Recovery sub-step 11.2 gets `` Run `docker compose up -d --no-deps app`. `` before its reference to restore step 9 | `== t02_runbook_recovery_step11_2: \| Ran 532 tests in 20.499s \| OK` |
| N1 | `n1_rollback_placeholder` | The fence of Rollback step 1 gets `RECIPE_VERSION=<previous tag> docker compose start app` | `== n1_rollback_placeholder: FAIL: test_rollback_has_no_start_of_a_deleted_service (test_upgrade_runbook.FailedRollbackRunbookTests.test_rollback_has_no_start_of_a_deleted_service) \| Ran 532 tests in 20.381s \| FAILED (failures=1)` |

The row `n1_rollback_placeholder` already fails on `main`. It is a regression check. After the change, it must still fail at least one test.

Two direct probes add detail:

- RDAB-D3: the stub log held `... -f /tmp/start in checkout/compose.yaml start app`. The guard printed `assert_no_forbidden_docker_commands: PASSED`.
- T-03: the text `` - Type a lone ` here.`` and `` - Run `docker compose start app`.`` on two lines gave `spans: ['here. - Run']` and `guard: []`.

## Goals / Non-Goals

**Goals:**

- Each mutant of the table above makes at least one test fail.
- Each new test passes on the current scripts and on the current runbook.
- One parser reads a call of `docker` for all guards.

**Non-Goals:**

- T-04. The change `restore-reboot-hazards` added the two sentences to `RunbookHostRebootTests.required_sentences`. It deleted the rerun of recovery step 4 from the runbook, and the test lists that text as obsolete. Mutants of the copy fail:
  - `== t04_drop_such_db: FAIL: test_restore_and_recovery_window_has_conditional_wait_and_skip (...) (sentence='Such a db cannot become healthy, and the healthcheck of walarchive needs a running db.') | Ran 532 tests in 23.171s | FAILED (failures=1)`
  - `== t04_drop_switch_deletes: FAIL: test_restore_and_recovery_window_has_conditional_wait_and_skip (...) (sentence='The switch deletes the containers of db and walarchive, and later steps create them again.') | Ran 532 tests in 23.155s | FAILED (failures=1)`
  - `== t04_order_swap: FAIL: test_recovery_restart_instruction_follows_the_wait (...) | Ran 532 tests in 20.636s | FAILED (failures=1)`
- New forms of `start` and `restart` in the windows of ADR-0176 (g) and (h). The windows are the major upgrade, the rollback, and `walarchive` in the recovery. The existing tests already read them:
  - `RunbookMajorUpgradeDeleteTests` calls `runbook_deleted_service_starts` on the section "Major upgrade" (`test_upgrade_runbook.py` line 626).
  - `FailedRollbackRunbookTests` calls it on the section "Rollback" (`test_upgrade_runbook.py` line 716).
  - `RunbookRecoveryDeleteTests` calls it on the recovery section with `app`, `backup`, and `walarchive` (`test_pitr_recovery.py` line 2043).
  - Tasks 1.1 to 1.4 change that function. So these tests then also refuse the new forms of `start` and `restart` of D2, D3, D4, and D8.
  - These tests do not refuse `up`, `create`, or `run` inside these windows. A later separate change adds these window guards (owner decision Q2).
- A change of a production script or of the runbook text.
- A complete shell parser. The guards read the forms of the findings: operators, redirections, command substitution, global options, line continuation, and quoted values.

## Decisions

### D1. No ADR and no spec delta

The change adds and changes tests in `deploy/ci/` only. No behavior of the product or of a procedure changes.
The decisions below choose the structure of test helpers. They are not design decisions of `docs/architecture/`, so no ADR records them.
The specs state behavior. No requirement changes, so `.openspec.yaml` sets `skip_specs: true`, as the archived change `plc-type-test-coverage` did.

### D2. Shell operators are separate tokens

Before the lexer runs, the runbook guard replaces each runbook placeholder with one word, for example `PLACEHOLDER`.
A placeholder is `<`, a letter, any characters other than `<`, `>`, a backtick, `|`, `;`, `&`, `(`, `)`, `$`, and quotes, then `>`. Examples are `<previous tag>`, `<file>`, and `<new volume>`.
So a placeholder cannot join an input redirection, a `docker` call, and a later output redirection into one word.
Without this step, the lexer reads `<previous tag>` as two redirections. The skip below then drops the word `docker` of `RECIPE_VERSION=<previous tag> docker compose start app`.
The current guard reports that command, so the change keeps that hit.

A lexer function of `docker_calls.py` reads each command with `shlex.shlex(text, posix=True, punctuation_chars=True)`, `whitespace_split = True`, and `commenters = ""`.
The empty `commenters` keeps a `#` inside its word, as `shlex.split` does. With the default `#`, the lexer drops the rest of the line.
The lexer then gives each run of `;`, `&`, `|`, `(`, `)`, `<`, and `>` as a separate token. A probe gave `['$', '(', 'docker', 'compose', 'start', 'app', ')']` for `$(docker compose start app)`.
A quoted operator inside a longer word stays in that word. For example, `-f 'a;b'` gives the word `a;b`.
A word that holds only a quoted operator gives the same token as the operator. For example, `-f ';'` gives the token `;`.
A control token consists only of the characters `;`, `&`, `|`, `(`, and `)`. The guard splits the tokens into simple commands at each control token.
The lexer gives a run of these characters as one token. So a merged run such as `);` or `&&(` is a control token, as `;`, `&&`, `||`, and `|&` are.
For example, `true&&(docker compose start app);` gives the two simple commands `true` and `docker compose start app`. The rule of D7 then never counts `);` as a service.
A redirection token consists only of the characters `<`, `>`, and `&`, and holds `<` or `>`. Examples are `>`, `>>`, `<`, `>&`, and `&>`.
So a quoted word such as `"<new volume>:/var/lib/postgresql/data"` is not a redirection token.
The guard skips a redirection token and its target word, then reads the next operands.
The lexer gives `2>` as the two tokens `2` and `>`. The guard also drops a word of digits directly before a redirection token.
Bash passes each word after a redirection to the command. So `docker compose start >/dev/null app` starts `app`, and the guard reports it.
An unclosed quote raises `ValueError`. The guard then splits at white space, as it does now.

Alternative: the package `bashlex`. The suites use the standard library only, and this change adds no package.

### D3. One parser of a `docker` call

A new function in `deploy/ci/docker_calls.py` reads the arguments after the program `docker`.
It skips the global options of `docker`, then the word `compose`, then the global options of Compose. It returns the subcommand and its operands.
The value options of `docker` are `--context`, `-c`, `--host`, `-H`, `--config`, `--log-level`, `-l`, `--tlscacert`, `--tlscert`, and `--tlskey`. The form `--option=value` takes no second token.
The runbook guard calls it after the token `docker`. The token `docker-compose` starts at the global options of Compose.
The stub-log guards call it with each record of D5. The two copies of the Compose value options in the two files become one table.
The string cases of the stub-log guard start with `compose`, `docker compose`, or `docker`. Their adapter drops a leading `docker` token, as the current guard does in `test_restore_switch.py` line 624.

The owner chose the location of the shared helpers (owner decision Q1). These rules place each helper:

- `deploy/ci/docker_calls.py` holds the shared parsing:
  - the lexer of D2 and the parser of D3
  - the record parser of D5
  - the table of value options, the rule of refused calls, and the window function of D7
- `deploy/ci/test_docker_calls.py` holds the unit tests of these helpers. It ends with `unittest.main()`, because `check.sh` runs each test file as a script.
- `test_upgrade_runbook.py` keeps the helpers that read runbook text: sections, code spans (D8), fenced lines (D4), and placeholders (D2).
- So `runbook_inline_code_spans` and `runbook_deleted_service_starts` stay in `test_upgrade_runbook.py`. They call the functions of `docker_calls.py`.
- Each suite loads `docker_calls.py` with `importlib.util.spec_from_file_location`, because `check.sh` runs each test file with `python3 -I`.
- `check.sh` compiles each `deploy/ci/*.py` and runs each `deploy/ci/test_*.py` (line 21). So it needs no change.

### D4. Fenced lines join at a final backslash

In a fence, a line that ends with `\` joins the next line before the guard reads it. The runbook already splits a long command this way, for example in line 1503.

### D5. Lossless argument records of the stubs

Each Docker stub that a guard reads runs this bash line: `printf '%s\0' "$#" "$@" >>"$STUB_LOG.argv"`.
A record is the count of arguments and each argument, each one followed by a NUL byte. A parser in `docker_calls.py` reads the records.
The record line is the first command of each stub, before each `case` and each `exit`. So a branch that exits early still writes its record.
The text log `$STUB_LOG` stays unchanged, so the existing regular expressions stay valid.

The Python text of the line depends on the kind of string literal:

- The stubs of `test_restore_switch.py` are raw strings (`r"""`). Their Python text holds the line as it is.
- `STEP_DOCKER_STUB` of `test_pitr_recovery.py` is a plain string (`"""`). It already uses `\\n` escapes, so it cannot become a raw string. Its Python text holds `printf '%s\\0'`.
- With `\0` in a plain string, Python writes a NUL byte into the stub. The Opus plan review then measured `plain rc 126 stderr ./stub_plain: ./stub_plain: cannot execute binary file`.
- A test of each stub kind runs the stub with an argument with a space, an empty argument, and an argument with a newline. It compares the bytes of `$STUB_LOG.argv` with the expected bytes.

A guard that reads records raises `AssertionError` when the record list of the whole run is empty. A missing record line or a run without a call of `docker` then fails the test.
A window slice of the run can be empty. For example, `check()` calls no `docker` before the TRUNCATE. An empty slice does not fail the test.

Alternatives:

- `printf '%q '` in one line per call. Bash quotes a control character as `$'...'`, and `shlex` cannot read that form.
- A change of the text log. Many tests match it, so each of them would change.

### D6. Forbidden calls in the step `restore`

Outside Compose, the guard of the step `restore` also refuses `start`, `restart`, `kill`, and `rm`, with and without the word `container`. It keeps the refusal of `volume rm`.
In Compose, it also refuses `stop`. A Compose `rm` must name at least one service, and each service must be `app` or `backup`.
ADR-0103 (b) keeps `db` running on the old volume until the switch. `compose rm -s -f db` and `compose stop db` stop it as `docker rm -f` does.
The records of the restore pass path hold only `compose rm -s -f app backup`. The Opus plan review printed `rec_restore.argv records 8 window [] d6 []`.
The guard does not apply to the step `switch`, because `stop_services` of `switch-steps.sh` runs `docker kill`.

### D7. The windows without a container of `app` and `backup`

ADR-0175 (h) ends each window when the procedure starts `app` and `backup` again. ADR-0103 (b) step 2 gives the reason for the delete.
A window guard reads the calls inside a window. It refuses `up`, `create`, `run`, `start`, and `restart` of Compose when the operands name `app` or `backup`.
It also refuses `up`, `create`, and `run` without `--no-deps` or without a service. `compose up -d` starts each service, and `proxy` depends on `app` (`compose.yaml` line 32).
So `up -d --no-deps proxy` passes, but `up -d` and `up -d proxy` give a hit.

The window guard separates the values of options from the services. It drops each option of the subcommand and the value of each value option. The other words are the operands.
The value options come from the help text of Compose 5.7.1 (`docker compose up --help`, `create --help`, and `run --help`):

- `up`: `--attach`, `--exit-code-from`, `--no-attach`, `--pull`, `--scale`, `-t`, `--timeout`, and `--wait-timeout`.
- `create`: `--pull` and `--scale`.
- `run`: `--cap-add`, `--cap-drop`, `--entrypoint`, `-e`, `--env`, `--env-from-file`, `-l`, `--label`, `--name`, `-p`, `--publish`, `--pull`, `-u`, `--user`, `-v`, `--volume`, `-w`, and `--workdir`.

The form `--option=value` takes no second token. `-w` is a value option of `run` only. In `up`, `-w` is the flag `--watch`.
So `up -d --no-deps --wait-timeout 180` has no operand and gives a hit. `up -d --no-deps --wait --wait-timeout 180 proxy` gives no hit.
The table lives in `docker_calls.py`. The runbook guard and the window guard of the records use the same table.

`start` and `restart` without a service give no hit. Restore step 5 names `docker compose start` in the sentence that forbids it.
The guard of D2 gets a parameter `subcommands` for this rule.

Each window ends at a concrete record or at a concrete text of the runbook:

- CI `restore-switch`: the records of the steps `restore`, `switch`, `access`, and `wal-line` are inside the window.
- CI `pitr-recovery`: the records of the steps `crash`, `recover`, `switch`, and `wal-line` are inside the window.
- CI step `check` of both scripts: the records before the TRUNCATE record are inside the window. The TRUNCATE is in `restore-switch.sh` line 303 and in `pitr-recovery.sh` line 624.
  - The record directly after the TRUNCATE must be `up -d --no-deps --wait --wait-timeout 180 app`. It ends the window of `app`.
  - The window of `backup` goes on until the record `up -d --no-deps --wait --wait-timeout 180 proxy backup`. That record ends it.
- Runbook "Restore into production": the window of `app` and `backup` holds steps 2 to 8 and sub-steps 9.1 to 9.3.
  - It also holds the text of sub-step 9.4 before the allowed start of `app`.
  - The allowed start of `app` is the code span `docker compose up -d --no-deps app` of sub-step 9.4. It must occur once in sub-step 9.4.
  - The window of `backup` goes on after that span. It holds the rest of sub-step 9.4 and the text of sub-step 9.5 before the allowed start of `backup`.
  - The allowed start of `backup` is the code span `docker compose up -d --no-deps proxy backup`. It must occur once in sub-step 9.5.
  - In the part after the start of `app`, the guard reads with the service `backup` only.
  - Each part ends directly before the opening backtick of its end span. So a cut never splits a code span.
  - Steps 2, 3, and 5 to 8 and the text before each end span have no such command.
  - Step 4 has these four commands of a failed attempt (ADR-0175 (e)), in this order:
    1. `docker compose up -d --no-deps app`
    2. `docker compose up -d --no-deps --force-recreate backup`
    3. `docker compose up -d --no-deps app`
    4. `docker compose up -d --no-deps backup`
- Runbook "Recover from the WAL archive": the window holds steps 1 to 10, the first paragraph of step 11, and sub-step 11.1.
  - It also holds the text of sub-step 11.2 before its reference to restore step 9.
  - The reference is the sentence `Run step 9 of "Restore into production".` It must occur once in sub-step 11.2. It ends the window of `app` and `backup`.
  - The window has no such command.

A probe on `main` ran the rules of D2, D3, D7, and D8 (`proto3.py` of the second plan update). The restore windows gave exactly the four commands of step 4, in order.
The quoted word `"<new volume>:/var/lib/postgresql/data"` of step 4 gave no fifth hit. The recovery window printed `== recovery window []`.
The mutants `t02_runbook_step9_4_prefix`, `t02_runbook_step9_4_backup`, and `t02_runbook_recovery_step11_2` each gave one more hit.
The section guards of "Major upgrade", "Rollback", "Restore into production", "Recover from the WAL archive", and "After a host reboot" each printed `[]`.
The rule of `--no-deps` and a service gives no false hit on the current files:

- The Opus plan review ran the rule on the records of the eight pass paths of the window steps. It reported no hit, for example `rec_access ... strict: []`.
- In restore steps 2, 3, and 5 to 8, each `up` and `run` has `--no-deps` and a service. The same review found this in recovery steps 1 to 11.
- Sub-steps 9.1 to 9.3 of the restore hold no code span with a command. Before the TRUNCATE, `check()` of both scripts calls no `docker`.
- The table of value options drops only option values. In the scripts, each `up` call and each `run` call with literal arguments names a service after its options.

### D8. Code spans end at a list item

CommonMark reads a list item as a container block, so a code span cannot cross two items.
`runbook_inline_code_spans` starts a new block at each line that starts a list item, an ATX heading, or a table row.

- A list item starts with `-`, `*`, `+`, or a number with `.` or `)`, after optional spaces.
- A space or the end of the line follows the list marker. So a wrapped line such as `--no-deps app` does not start a list item.
- An ATX heading is a block of one line. So the line after a heading also starts a new block.
- A table row is a line that starts with `|`, after optional spaces. Each row starts a new block, so a backtick cannot pair across two rows.

## Risks / Trade-offs

- A word that holds only a quoted operator, for example `';'`, gives an operator token in D2. The guard then splits the call of `docker` at that word. It can miss a later subcommand. No runbook command has such a word.
- The window guard counts an option outside the table of D7 as a flag. A new value option of Compose needs a new row in that table. Until then, its value can hide a missing service.
- The placeholder rule of D2 also joins a redirection pair such as `<in >out` into one word when no operator lies between them. No runbook command has such a pair.
- The guards read more forms, so a future runbook text can fail a test with a false hit. The test message names the command, so the writer can change the text or the test.
- D7 pins the four commands of restore step 4, the end records of `check`, and the three end texts of the runbook windows. A change of these lines must change the test too.
- Each stubbed run writes one more file. The run time of the suites changes little.

## Decisions of the owner (2026-11-17)

- Q1, the location of the shared helpers: option B. The new file `deploy/ci/docker_calls.py` holds them. The new file `deploy/ci/test_docker_calls.py` tests them (D3).
- Q2, the window guards for `up`, `create`, and `run` in the windows of ADR-0176 (g) and (h): option A. A later separate change adds them (Non-Goals).
- Q3, the LOW findings of the plan reviews: this plan fixes each one. The list gives the place of each fix:
  - L1: D8 and task 1.4 (list marker, ATX heading, table row)
  - L3: the first update of the plan (no task sentence has more than 20 words)
  - L4: task 3.1 (a text anchor with a count check)
  - N3: D2 and tasks 1.1 and 1.5 (a merged operator run ends a simple command)
  - N4: D5 and task 2.3 (the record list of the whole run)
  - N5: D5 and tasks 2.1, 2.3, and 3.1 (the record line is the first command of a stub)
  - N6: the header and the commands of tasks 1.1 to 2.3 (each command also runs `test_docker_calls`)
  - N7: the header of tasks.md (the exceptions to the failing test)
