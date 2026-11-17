# Design

## Context

See proposal.md, section "Why". ADR-0176 records the decision, the considered options, and the items that it does not change. ADR-0175 (h) to (k) give the current rules.

The current text on `main` (commit f48daef):

- `docs/ops/runbook.md` "Major upgrade" step 1 (line 1924): "Run `docker compose stop app`. The next dump then holds every change."
- The same section, line 1939: "If a step before step 8 fails, change into the directory of `<old tag>`. Then run `docker compose start app`, so that `app` serves again on the old volume."
- "Recover from the WAL archive" step 4 (line 978 to line 982): "Stop `walarchive`. The `.partial` file then stays stable" and the bash block `docker compose stop walarchive`.
- The same section, step 7 (line 1009): "To end the attempt, delete the new volume. Start `walarchive`, if you stopped it in step 4."
- The same section, step 8, item 5 (line 1022 and line 1023): "When the command prints a path, the file exists. Stop. This volume is a drill volume." and "Also stop when the field `drill` of step 7 is `true`."
- "Rollback" step 1 (line 1596 to line 1628). The first bash block holds `docker compose stop backup` and command 1. The second one holds `docker compose rm -s -f app`.
- "After a host reboot" (line 468 to line 503). Line 477 to line 492 hold the window rules of ADR-0175 (h).
- `reliability.md` §7.7 line 856 ("stops `walarchive`") and §7.10 step 6, line 985 ("Stop `backup` before command 1").
- `deploy/ci/pitr-recovery.sh` line 484 (`compose stop walarchive`) in the function `crash`. `deploy/ci/upgrade.sh` line 651 (`compose stop backup`) and line 663 (`compose up -d --no-deps --wait --wait-timeout 180 backup`) in the function `check_rollback_runbook`.

`compose.yaml` line 127 (`app`), line 268 (`backup`), and line 303 (`walarchive`) set `restart: always`.

No CI script runs the major upgrade. `deploy/ci/upgrade.sh` runs the upgrade of `reliability.md` §7.10 and the check of the rollback set. `deploy/ci/migration_freeze.py` checks only the migration list of a major release.

## Goals / Non-Goals

**Goals:**

- The runbook, `reliability.md`, the spec, and the CI scripts give one command for each delete: `docker compose rm -s -f app`, `docker compose rm -s -f walarchive`, and, with option A of D5, `docker compose rm -s -f backup`.
- Each later start of a deleted service names `docker compose up -d --no-deps`.
- A test of `deploy/ci/` fails when a CI script and the runbook give different commands. A test also fails when a section starts a deleted service with `start` or `restart`.
- "After a host reboot" names the window of the major upgrade and the window of the rollback.

**Non-Goals:**

- The hardening of the `deploy/ci` tests (the review findings D2, D3, and T-01 to T-05 of 2026-11-13). A separate change does it.
- A CI job for the major upgrade.
- Step 2 of "A gap in the WAL stream" (ADR-0176 (k)).
- A change of `compose.yaml`, of an image, or of a pgtools script.

## Decisions

### D1. Commands

| Procedure | Step | Old command | New command |
|---|---|---|---|
| Major upgrade | 1 | `docker compose stop app` | `docker compose rm -s -f app` (ADR-0176 (a)) |
| Major upgrade | failed step before step 8 | `docker compose start app` | `docker compose up -d --no-deps app` (ADR-0176 (b)) |
| Recovery | 4 | `docker compose stop walarchive` | `docker compose rm -s -f walarchive` (ADR-0176 (d)) |
| Recovery | 7, end of the attempt | "Start `walarchive`" | `docker compose up -d --no-deps walarchive` in the directory `deploy/` of `<tag>` (ADR-0176 (f)) |
| Rollback | 1 | `docker compose stop backup` | `docker compose rm -s -f backup` (ADR-0176 (i)) |

The option `-s` stops a running container first. The option `-f` skips the question. Two commands then find a service without a container:

- Step 8 of the major upgrade runs `docker compose rm -s -f app backup` without a container of `app`. ADR-0175 (g) reports only `stop` and `rm -f`. A local probe on Compose 5.7.1 ran `rm -s -f` for such a service. It printed `No stopped containers` and exited with 0. No CI job runs this case, and `ci.yml` pins no Compose version. The result on the runner is not proven.
- Sub-step 1 of the switch in step 8 of the recovery runs `stop` and `rm -f` without a container of `walarchive`. ADR-0175 (g) gives the result of Compose 5.7.1. The CI job `pitr-recovery` proves this case on the runner.

### D2. Integration map

| Place | Change |
|---|---|
| Runbook "Major upgrade" step 1 | Text "Delete the container of `app`. The next dump then holds every change." Add the reason of ADR-0176 in one sentence. Add the optional log copy `docker compose logs --no-color app > <file>`. Then one bash block with the one line `docker compose rm -s -f app` |
| Runbook "Major upgrade", line 1939 | "Then run `docker compose up -d --no-deps app`, so that `app` serves again on the old volume." |
| Runbook "Recover from the WAL archive" step 4 | Text "Delete the container of `walarchive`. The `.partial` file then stays stable." Add the reason and the optional log copy `docker compose logs --no-color walarchive > <file>`. Then one bash block with the one line `docker compose rm -s -f walarchive` |
| Runbook "Recover from the WAL archive" step 7 | The end of the attempt gets the order of the list after this table. The old text started `walarchive` before the reset of the tag. With `up`, that order gives `walarchive` the image of `<recovery tag>`, against ADR-0083 (a) |
| Runbook "Recover from the WAL archive" step 6 | No own change. A failed run ends the attempt as in step 7 |
| Runbook "Recover from the WAL archive" step 8, item 5 (line 1022 and line 1023) | The old "Stop" left `walarchive` without a container in the directory `deploy/` of `<recovery tag>`. Line 1022 becomes "When the command prints a path, the file exists. This volume is a drill volume. Do not switch. End the attempt as in step 7." Line 1023 becomes "Do the same when the field `drill` of step 7 is `true`." |
| Runbook "Rollback" step 1 of the restore of the `_premigrate` set (line 1596) | "1. Delete the container of `backup`." The block of command 1 holds `docker compose rm -s -f backup`, then command 1. The reason text of line 1618 names the restart after a reboot. Line 1598 says that the commands below delete `backup` and `app`. The optional log copy is its own bash block before the block of command 1. The block is the one line `docker compose logs --no-color backup > <file>`, because `test_rollback_stop_raw_commands` allows only listed raw lines with the word `docker`. The prose of step 1 does not use the word "Docker". Line 1621 says "the container engine" |
| Runbook "After a host reboot" | D3 |
| `reliability.md` §7.7, line 856 | "deletes the container of `walarchive`" (ADR-0176 (d)) |
| `reliability.md` §7.10 step 6, line 985 | "Delete the container of `backup` with `docker compose rm -s -f backup` before command 1" (ADR-0176 (i)) |
| `deploy/ci/pitr-recovery.sh` `crash`, line 484 | `compose rm -s -f walarchive \|\| fail "crash: cannot delete walarchive."` |
| `deploy/ci/upgrade.sh` `check_rollback_runbook`, line 651 | `compose rm -s -f backup \|\| fail "cannot delete backup before the runbook check"`. Line 663 stays, because `up` creates the container |
| `.github/workflows/ci.yml` line 598 | The comment says "delete walarchive" |
| `CHANGELOG.md` | One line under `### Fixed` of `[Unreleased]` |
| ADR-0174, ADR-0175, ADR-0106, `docs/adr/README.md` | Status lines, index row, and decision map in task 1.1 |

The new text of step 7 ends the attempt in this order:

1. "To end the attempt, delete the new volume."
2. "If `<recovery tag>` differs from `<tag>`, change into the directory `deploy/` of `<tag>`."
3. "Set `RECIPE_VERSION` in `.env` to `<tag>`."
4. "If step 4 ran, create `walarchive` again with `docker compose up -d --no-deps walarchive`."
5. "Then end the attempt as in step 4 of "Restore into production". Do not switch."

### D3. Section "After a host reboot"

The text replaces line 477 to line 492. It keeps four paragraphs and one list. The test of task 4.4 reads the text without backticks and with single spaces.

1. The first paragraph names the windows. It holds these sentences:
   - "From step 2 of [Restore into production](#restore-into-production) until the procedure starts app and backup again, the two services can have no container." (unchanged)
   - "The same holds from step 1 of [Recover from the WAL archive](#recover-from-the-wal-archive)." (unchanged)
   - "From step 1 of [Major upgrade](#major-upgrade) until the procedure starts app again, app can have no container."
   - "In [Rollback](#rollback), step 1 of the restore of the _premigrate set deletes backup."
   - "From that step until the procedure starts app and backup again, backup can have no container."
   - "From the pass of command 1 in that step, app can have no container too."
2. The second paragraph holds the rules for each window:
   - "While app has no container, proxy can stay unhealthy, because its healthcheck calls http://app:9000/healthz."
   - "Skip step 4 in each window."
   - "Step 4 runs docker compose exec app." (unchanged)
3. The third paragraph holds the wait rule:
   - "In each window, step 1 waits only for db and walarchive, and only for a service that docker compose ps -a lists."
   - "Step 4 of the recovery deletes the container of walarchive."
   - "The switch deletes the containers of db and walarchive, and later steps create them again." (unchanged)
   - The two sentences of the skip rule for `db` stay unchanged.
4. The fourth paragraph holds two sentences:
   - "After step 1 of this section, continue the procedure at the step where it stopped."
   - "Two cases need an action before you continue:"
5. The list keeps the case of the fill and the case of the variable `target`. The case of `walarchive` drops (ADR-0176 (h)).

The wait rule stays correct without a new word. From step 4 of the recovery until step 10, or the end of a failed attempt, creates it again, `docker compose ps -a` lists no `walarchive`. Step 1 therefore does not wait for it. In the window of the major upgrade before step 8, `db`, `walarchive`, and `backup` run on the old volume. Step 1 waits for `db` and `walarchive`.

### D4. Tests

`deploy/ci/check.sh` runs each test file with `python3 -I`. A test that uses a helper of another test file loads that file with `importlib.util.spec_from_file_location`, as `test_pitr_recovery.py` line 1977 does.

- `deploy/ci/test_upgrade_runbook.py`, shared helpers:
  - `runbook_deleted_service_starts(section)` (line 123) checks the services `app` and `backup` only (line 157). It gets a parameter `services` with the default `("app", "backup")`.
  - The helper already has a local list `services` (line 152 and line 156). That list gets the new name `named`. Line 157 becomes `set(services).intersection(named)`.
  - `runbook_step(title, start)` (line 523) reads only a heading of level 3. "Major upgrade" has level 2. The helper gets a parameter `level` with the default 3.
  - Each helper change has its own unit tests.
- `deploy/ci/test_pitr_recovery.py`:
  - The order test (line 1046), the order test of the writers (line 1074), and the tests at line 1139 and line 1157 expect `rm -s -f walarchive`.
  - The stub case at line 2311 matches `*" rm -s -f walarchive")` and reads `STUB_REMOVE_WALARCHIVE_STATUS`.
  - `test_crash_fails_when_walarchive_does_not_stop` (line 1149) gets the name `test_crash_fails_when_walarchive_cannot_be_deleted`. It sets that variable and expects `CI: crash: cannot delete walarchive.`
  - `RunbookRecoveryDeleteTests` (line 1973) gets six tests. The bash block of step 4 is the one line `docker compose rm -s -f walarchive`. That line without `docker ` occurs in `shell_function("crash")`. Step 7 holds the exact span `docker compose up -d --no-deps walarchive`.
  - Step 4 holds the exact span `docker compose logs --no-color walarchive > <file>` (ADR-0176 (j)).
  - A tag order test covers a failed attempt with a `<recovery tag>` that differs from `<tag>`. It reads step 7 without backticks.
  - In that text, these spans occur in this order:
    1. "change into the directory deploy/ of <tag>"
    2. "Set RECIPE_VERSION in .env to <tag>"
    3. "docker compose up -d --no-deps walarchive"
  - Step 8 holds the span "This volume is a drill volume. Do not switch. End the attempt as in step 7." It holds no text "Stop. This volume is a drill volume." (ADR-0176 (f)).
  - The section test of line 1992 gets the name `test_section_has_no_start_of_a_deleted_service`. It passes `("app", "backup", "walarchive")` to the helper. Its message (line 1995) names `walarchive` too. The two calls in the guard tests (line 2005 and line 2010) use the new name.
- `deploy/ci/test_upgrade_runbook.py`, sections:
  - A new class for "Major upgrade". Step 1 has one bash block with the one line `docker compose rm -s -f app`. Step 1 holds the exact span `docker compose logs --no-color app > <file>`. The section holds the span `docker compose up -d --no-deps app`. The helper finds no start of `app` or `backup` in the section.
  - Option A: `BACKUP_STOP` (line 512) becomes `BACKUP_REMOVE = "docker compose rm -s -f backup"`. `rollback_stop` (line 556) and line 594 use the start "1. Delete the container of `backup`."
  - Option A: a new constant `BACKUP_LOG = "docker compose logs --no-color backup > <file>"` joins the allowed set of `test_rollback_stop_raw_commands` (line 589). A new test finds the block `[BACKUP_LOG]` once, before the block of command 1.
  - Option A: a new test reads `shell_function("check_rollback_runbook")`. It holds `compose rm -s -f backup` and no `compose stop backup`.
  - `RunbookHostRebootTests` (line 364) gets the sentences of D3. The old sentences of the case of `walarchive` and "Three cases need an action before you continue:" move to `old_texts`. The three old texts with "In that window" or "in that window" move there too.
  - `test_recovery_restart_instruction_follows_the_wait` (line 455) keeps the order of its markers. Marker 1 (line 458) becomes "In each window, step 1 waits only for". Marker 3 becomes the first sentence of the case of the fill. The message of line 467 becomes "After a host reboot: handle the cases after the wait".
- Each runbook task shows a mutation once in the working tree and then undoes the edit.

Two rules of `crates/recipe-server/tests/deploy_values.rs` apply to each line of `deploy/ci/`, test files too. The test `deploy_values_ci_volume_deletions_stay_in_throwaway_scripts` refuses a line where the word `volume` is directly followed by the word `rm`. `deploy_values` also refuses `|| exit 1` and `*) exit 1`. A test text that holds such a command splits it into two literals, as line 384 of `test_upgrade_runbook.py` does.

### D5. Hazard (i): the rollback

Step 1 of "Rollback" stops `backup` before command 1 and deletes `app` after a pass. `backup` keeps a stopped container until sub-step 1 of the switch in step 4 of the rollback. A reboot in that window has these effects:

- Docker starts the old container of `backup` with its image and its configuration. In the plain rollback, that is the new image with the key. After a failed upgrade, it is the previous image of the fault list (`reliability.md` §7.10 step 3).
- `backup` mounts the old volume and dumps the old cluster through `db`. `app` writes nothing after the pass of command 1, so a new daily set holds the state of the old cluster.
- The retention of `backup-once` keeps the newest daily set and the newest `_premigrate` set (`reliability.md` §7.7). The rollback set therefore stays when it is the newest `_premigrate` set.
- `restore-test` on Sunday at 05:00 UTC takes the lock `/backups/.scratch/.lock`. Command 1 or the fill of step 3 then stops with "another restore test or restore volume holds the lock". The data stays safe, but the rollback stops for a cause that the set does not cause, and the downtime grows.
- Before the pass of command 1, `app` still runs. A reboot then starts `app` again, as without the rollback. A repeat of step 1 stops `backup` again.
- "After a host reboot" names no rollback window now. After the pass, `app` has no container, so step 1 of that section waits without an end.

The options:

- Option A (recommended): delete `backup` with `docker compose rm -s -f backup` before command 1. Each later start of `backup` already uses `up -d --no-deps`. The cost is one test constant, one CI line, and the log copy. It amends ADR-0174 (aj).
- Option B: keep `docker compose stop backup`. "After a host reboot" names the rollback window and adds a case: run `docker compose stop backup` again before you continue. A cron run can start before the operator acts.
- Option C: keep step 1. "After a host reboot" names the rollback window for `app` only. A lock fault stays possible on a Sunday.

The owner chose option A on 2026-11-16. These parts belong to option A:

- ADR-0176 (i).
- The two MODIFIED requirements of the rollback in the delta spec.
- Tasks 3.2 and 4.3, and part of tasks 1.1, 4.4, 5.1, and 5.3.

### D6. Implementers

Codex `gpt-6.1-sol` implements the tasks in `deploy/ci/`. Haiku does each `[mechanical]` task. Opus writes the content of each runbook task. A Sonnet agent runs `ste-lint.py` on each doc task and changes only the form.

## Risks / Trade-offs

- [Risk] A later Docker Compose release ends `stop`, `rm -f`, or `rm -s -f` with a code other than 0 for a service without a container. → The CI job `pitr-recovery` runs the `stop` and `rm -f` of the switch after the delete of `walarchive`. No CI job runs `rm -s -f` for such a service. The major upgrade has no CI proof.
- [Risk] `deploy/ci/upgrade.sh` line 663 creates `backup` again after the delete (option A). The new container must get the CI overlay and its disabled schedule. → The function `compose` of the CI passes the same files to `up`. The CI job `upgrade` of the gate task shows the result.
- [Trade-off] `rm` deletes the container logs of `app`, `walarchive`, and `backup`. → Each step gives an optional log copy (ADR-0176 (j)).
- [Trade-off] Step 2 of "A gap in the WAL stream" keeps `docker compose stop walarchive`. → ADR-0176 (k) records it. A later change can apply the same rule.

## Migration Plan

The change edits documents and CI scripts only. No image and no `compose.yaml` value changes. An installation applies it with the next release of the runbook.

## Open Questions

- None. The owner decided on 2026-11-16 that the major upgrade gets runbook guard tests only. A CI job for it is a separate change.
