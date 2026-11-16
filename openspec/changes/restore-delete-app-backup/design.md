# Design

## Context

See proposal.md, section "Why". ADR-0175 records the decision, the considered options, and the items that it does not change.

The current text of the two steps:

- `docs/ops/runbook.md` "Restore into production" step 2 (line 667 to line 671). ADR-0103 (b) step 2. `reliability.md` §7.7 (line 795).
- `docs/ops/runbook.md` "Recover from the WAL archive" step 1 (line 932 to line 936). ADR-0106 (i) step 1. `reliability.md` §7.7 (line 855).
- `deploy/ci/restore-switch.sh` line 146 and line 147, function `restore`. `deploy/ci/pitr-recovery.sh` line 476 and line 477, function of the step `crash`.

`compose.yaml` line 127 (`app`) and line 268 (`backup`) set `restart: always`.

## Goals / Non-Goals

**Goals:**

- The runbook, `reliability.md`, the spec, and the two CI scripts give one command: `docker compose rm -s -f app backup`.
- A test of `deploy/ci/` fails when a CI script and the runbook give different commands.
- Each start of `app` or `backup` after the delete names `docker compose up -d --no-deps`.

**Non-Goals:**

- The items (i), (j), and (k) of ADR-0175. These are the rollback, step 1 of the major upgrade, and the stop of `walarchive`.
- A change of `compose.yaml`, of an image, or of a pgtools script.

## Decisions

### D1. One command in both procedures

Both procedures use `docker compose rm -s -f app backup` (ADR-0175 (a), (b)). The option `-s` stops a running container first. The option `-f` skips the question. The switch of ADR-0103 (f) and the rollback of ADR-0174 (aj) already use `rm`.

The switch step 1 then finds no container of `app` and `backup`. On the host of this design, Docker Compose 5.7.1 ended `stop` and `rm -f` for such a service with the exit code 0. A scratch project gave that result.

Nobody checked the lowest supported version, Compose 2.24 (runbook section "Check the host"). The CI dispatch of the gate task proves the result only for the Compose version of the runner (ADR-0175 (g)).

### D2. Integration map

| Place | Change |
|---|---|
| Runbook "Restore into production" step 2 | Text "Delete the containers of `app` and `backup`" and the bash block `docker compose rm -s -f app backup`. Keep both reasons of the step. Add the reason of the delete (ADR-0175). Add the optional log copy of D4 before the bash block |
| Runbook "Restore into production" step 4, the paths that end the attempt | Drop each "as in "Stop, start, and restart"" for `app` and `backup`. The plain rollback path names `docker compose up -d --no-deps app` and keeps its one command `docker compose up -d --no-deps --force-recreate backup`. The path "In every other case" names `docker compose up -d --no-deps app`, then `docker compose up -d --no-deps backup` |
| Runbook "Recover from the WAL archive" step 1 | The same bash block as step 2 of the restore. Keep the reason "After this point, `app` writes no audit entry". Add only the reason of the delete (ADR-0175) and the optional log copy of D4 |
| Runbook "Recover from the WAL archive" steps 6 and 7, the paths that end the attempt | No own change. They end the attempt as step 4 of the restore does |
| Runbook "Major upgrade" step 8 | No change. It runs steps 2 to 5 of the restore, so it gets the delete (ADR-0175 (f)) |
| Runbook "Rollback" step 1 | No change. Its sentence "Run no step 2 of that list" stays true |
| Runbook "After a host reboot" | Three paragraphs and one list. The first one names the window. The window starts at step 2 of the restore and at step 1 of the recovery. It ends when the procedure starts `app` and `backup` again. In the window, the two services can have no container. `proxy` cannot become `healthy` without `app`. The operator skips step 4, because it runs `docker compose exec app`. The second one: in the window, step 1 waits only for `db` and `walarchive`, and only for a service that `docker compose ps -a` lists. The switch deletes the containers of `db` and `walarchive`, and later steps create them again. Step 8 of the recovery deletes the container of `db` in sub-step 1 of the switch. The drill check of step 8 runs before that sub-step. Until that delete, the operator skips the wait when `db` did not run at step 1 of the recovery. The third one: after step 1 of the section, the operator continues the procedure at the step where it stopped. The list names three cases that need an action before that. A reboot between step 4 and step 8 of the recovery starts `walarchive` again (`restart: always`). The operator then runs step 4 of the recovery again, after the wait of step 1. A reboot during a fill (step 4 of the restore, step 6 of the recovery) leaves a volume that is not complete. The fill refuses a volume that is not empty. The operator deletes the new volume and starts again at step 3 of the restore or step 5 of the recovery. A reboot inside step 7 of the restore or step 10 of the recovery loses the shell variable `target`. The operator sets `target` to the directory of sub-step 1, or runs sub-step 1 again. The text names the healthcheck target `http://app:9000/healthz` and no line number of `compose.yaml`. The precedent is the paragraph of the failed rollback restore at line 474 (ADR-0175 (h)) |
| `reliability.md` §7.7, restore bullet (line 795) | "deletes the containers of `app` and `backup`" (ADR-0175 (a)) |
| `reliability.md` §7.7, recovery bullet (line 855) | The same words (ADR-0175 (b)) |
| `deploy/ci/restore-switch.sh` `restore` | Log line and command. The fault line is `restore: cannot delete app and backup.` |
| `deploy/ci/pitr-recovery.sh` `crash` | Log line and command. The fault line is `crash: cannot delete app and backup.` |
| `.github/workflows/ci.yml` line 598 | The comment says "delete app and backup" |
| `CHANGELOG.md` | One line under `### Fixed` of `[Unreleased]` |

`docs/adr/README.md`, ADR-0103, and ADR-0106 got their edits with this proposal.

### D3. Tests

`deploy/ci/check.sh` runs each test file with `python3 -I`, so the directory `deploy/ci` is not on `sys.path`. A test that uses a helper of another test file loads that file with `importlib.util.spec_from_file_location`, as `test_restore_switch.py` line 2856 does.

The runbook tests use the helpers `runbook_heading_section`, `runbook_bash_blocks`, and `runbook_step` of `test_upgrade_runbook.py`. One more helper there returns the inline code spans of a text. It reads one line at a time and skips the lines of fenced blocks. A run of backticks opens a span, and the next run of the same length closes it, as in CommonMark. One shared helper finds the fenced blocks for each parser of the file. The helpers have their own unit tests.

- `deploy/ci/test_restore_switch.py`:
  - The order test (line 276) and the log test (line 618) expect `rm -s -f app backup`.
  - A new stub case `*" compose "*" rm -s -f app backup"*)` of the docker stub exits with `STUB_REMOVE_APP_STATUS`. It comes before the case `*" compose "*" stop "*)` (line 453).
  - A new fault test sets `STUB_REMOVE_APP_STATUS` to 1. It expects `CI: restore: cannot delete app and backup.` and no `volume create` in the log.
  - A new runbook test: the bash block of step 2 of "Restore into production" is the one line `docker compose rm -s -f app backup`. That line without the prefix `docker ` occurs in the function `restore`.
  - A new runbook test of step 4 (`runbook_step("Restore into production", "4. Fill the new volume.")`). The step holds the exact span `docker compose up -d --no-deps app` twice and the exact span `docker compose up -d --no-deps backup` once. The span with `--force-recreate` does not count. The step holds no text `as in "Stop, start, and restart"`.
  - A new runbook test of the section: no inline code span and no bash line holds `docker compose start app` or `docker compose start backup`.
- `deploy/ci/test_pitr_recovery.py`:
  - The order tests (lines 1036, 1071, 1079) expect `rm -s -f app backup`. The stub case (line 2268) matches `" rm -s -f app backup"`.
  - The stub variable `STUB_STOP_APP_STATUS` becomes `STUB_REMOVE_APP_STATUS`. The fault test (line 1131, line 1132) expects the new fault line.
  - The two precondition tests at line 1123 and line 1128 use the marker `"rm -s -f app backup"` in place of `"stop "`. Else a delete before the volume check or the label check passes both tests. These two tests are a guard. They pass before and after the script change.
  - A new runbook test: the bash block of step 1 of "Recover from the WAL archive" is the one line `docker compose rm -s -f app backup`. That line without the prefix `docker ` occurs in `shell_function("crash")` (line 58).
  - A new runbook test of the section: no inline code span and no bash line holds `docker compose start app` or `docker compose start backup`.
- `deploy/ci/test_upgrade_runbook.py`: a new test of the section "After a host reboot" (D2). It checks the required sentences in the text of the whole section, not in one paragraph. These include the start of each window, the end of the skip at step 8 of the recovery, and the repeat of step 4. It also checks that the sentences of the old rules are absent.
- A mutation of a start command of the two sections back to `docker compose start app` or `docker compose start backup` makes a test fail. A revert of only the line "In every other case" of step 4 to its old text also makes a test fail. Each runbook task shows its mutations once in the working tree and then undoes the edits.

### D4. Optional log copy

`rm` deletes the container logs of `app` and `backup` of the driver `json-file` (ADR-0175, Negative). Before the delete, the step gives one optional command: `docker compose logs --no-color app backup > <file>`. The operator chooses `<file>`. The command is an inline code span in the text of the step, so the step keeps one bash block.

### D5. Implementers

Codex `gpt-6.1-sol` implements the tasks in `deploy/ci/`. Haiku does each `[mechanical]` task. Opus writes the content of the runbook task. A Sonnet agent runs `ste-lint.py` on each doc task and fixes only the form.

## Risks / Trade-offs

- [Risk] A later Docker Compose release ends `stop` or `rm -f` with a code other than 0 for a service without a container. → The switch step 1 then fails. The CI jobs `restore-switch` and `pitr-recovery` run that step after the delete. The gate task dispatches both jobs on the branch.
- [Risk] An operator starts a deleted container with `docker compose start`. → The command fails with a message and starts nothing. Each start in both procedures names `up -d --no-deps`.
- [Trade-off] After a reboot in the window, `docker compose ps -a` can list no `app` and no `backup`. After sub-step 1 of the switch, it can also list no `db` and no `walarchive`. → The runbook section "After a host reboot" names that state. Step 1 waits only for `db` and `walarchive`, and only when `docker compose ps -a` lists the service. The operator skips step 4.
- [Trade-off] Until step 8 of the recovery deletes the container of `db`, a `db` that did not run at step 1 stays listed. → The operator skips the wait for such a `db`.
- [Trade-off] Step 4 of the recovery only stops `walarchive`. A reboot between step 4 and step 8 of the recovery starts it again. → The operator runs step 1 of "After a host reboot" and then step 4 of the recovery again. The wait comes first, because `docker compose ps -a` lists the stopped `walarchive`. ADR-0175 (k) keeps the full fix out of scope.

## Migration Plan

The change edits documents and CI scripts only. No image, no `compose.yaml` value, and no step of an upgrade changes. An installation applies it with the next release of the runbook.

## Open Questions

None for this change. The report to the owner lists the related items (i), (j), and (k) of ADR-0175.
