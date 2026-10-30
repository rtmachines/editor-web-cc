# Tasks

Each task is for one agent. The mark after the scope names the model and the effort: `[sonnet high]` (Sonnet, effort high) or `[mechanical] [haiku medium]` (Haiku, effort medium). No task runs on Codex (`design.md` D10).

Each task of groups 1 to 9 first writes a failing test (RED) and runs it. Then it writes the change that makes the test pass (GREEN). The RED of a doc task is a check that finds the old text. Group 10 runs the final checks only.

A pgtools suite runs inside the `recipe-pgtools` image, as `deploy/ci/pgtools.sh` does. Build the image first with `docker compose -f deploy/compose.yaml -f deploy/compose.build.yaml build backup`. The pgtools suites and the CI jobs need Docker. A run on the workstation needs the approval of the user, else it runs in CI.

## Run order

1. Groups 1, 4, 5, and 6 can start in parallel. Inside a group, the tasks run in their order.
2. Group 2 starts after task 1.3. Group 3 starts after group 2. Tasks 2.x and 3.x all edit `bin/pitr-volume`, so they run one after the other.
3. Task 4.2 starts after task 3.4, because it reads the constants of `bin/pitr-volume`.
4. Tasks 5.1 to 5.3 edit `lib/wal-identity.sh` or `bin/basebackup-once`, so they run one after the other.
5. Group 7 starts after groups 3, 4, 5, and 6.
6. The tasks that edit `docs/ops/runbook.md` or `CHANGELOG.md` run one after the other. Group 8 starts after group 7.
7. At most two agents that run Docker or cargo run at the same time.
8. The code review of the workflow follow-up runs after group 8. Group 10 starts after the fixes of that review. Task 10.7 is the gate of the change.

## 1. Shared recovery library

- [ ] 1.1 (deploy/pgtools) [sonnet high] RED: add a case to `tests/lib/wal-pitr-cases.sh`. The stub of `pg_ctl` records its `-o` argument and copies `postgresql.auto.conf` at its stop call. The copy holds no line of `restore_command`, `hot_standby`, or `port`. The `-o` argument holds the settings. See it fail. GREEN: create `lib/recovery-steps.sh` with the start functions of `design.md` D1. `bin/pitr-check` sources it and passes its settings with `pg_ctl -o` (ADR-0106 (h)). Check: `pitr-start-test` and `walarchive-test` pass.
- [ ] 1.2 (deploy/pgtools) [sonnet high] RED: add cases to `tests/lib/wal-pitr-cases.sh`. A base backup with the file `backup_end_time` passes the check of its copy, and the copy then holds no such file. A base backup with another file outside the manifest fails the check. See them fail. GREEN: the check of a copy of `design.md` D1 in `lib/recovery-steps.sh`, used by `bin/pitr-check` (ADR-0106 (c), (h)). Check: `pitr-start-test` and `walarchive-test` pass.
- [ ] 1.3 (deploy/pgtools, recipe-server) [sonnet high] RED: add a test to `tests/deploy_values/files.rs`. The `Dockerfile` of `recipe-pgtools` copies `lib/recovery-steps.sh` to the place of the other library files. See it fail if the copy is missing. GREEN: change `deploy/pgtools/Dockerfile` only when it copies single files of `lib/`. Check: `cargo test -p recipe-server --test deploy_values` passes.

## 2. Script pitr-volume: gates and selection

- [ ] 2.1 (deploy/pgtools) [sonnet high] RED: create the suite `tests/pitr-volume-test` with `tests/lib/pitr-volume-stubs.sh` and `tests/lib/pitr-volume-gate-cases.sh`. Each case below writes one `ERROR` line before any stub of a PostgreSQL tool runs:
  - a target that is not empty, a bad argument, and `--drill` without `--target-time`
  - a target time without `Z`, and the date `2026-02-30`
  - a value `PITR_EXPECTED_LSN` outside its grammar
  - a uid other than 999, and a name other than `postgres`
  - no marker file, a held lock, and a missing or malformed `.system-identifier`

  See them fail. GREEN: create `bin/pitr-volume` with steps 1 to 3 of ADR-0106 (f). Check: `pitr-volume-test` passes.
- [ ] 2.2 (deploy/pgtools) [sonnet high] RED: add `tests/lib/pitr-volume-select-cases.sh` with these cases:
  - Without a target, the newest base backup of the grammar wins.
  - With a target, the newest base backup whose `backup_end_time` lies at or before the target wins. A target between the start and the end time of the newest base backup selects the older one.
  - A base backup without `backup_end_time` counts with its `START TIME` in `UTC`. A zone other than `UTC` and a malformed `backup_end_time` never win.
  - A link, a file, a `.tmp-base_*` directory, and a base backup without both lines never win.
  - No match gives one `ERROR` line and copies nothing.
  - Free space that falls below `disk.warn_percent` after the copy gives one `ERROR` line and copies nothing.

  See them fail. GREEN: step 4 of ADR-0106 (f) and the selection of `design.md` D3. Check: `pitr-volume-test` passes.
- [ ] 2.3 (deploy/pgtools) [sonnet high] RED: add `tests/lib/pitr-volume-copy-cases.sh`. A failed check of the copy and a copy of another system identifier each give one `ERROR` line and start no server. A passed copy holds no `backup_end_time`. It keeps the SHA-256 values of the four configuration files in shell variables, and no baseline file exists under `/run/pgpass`. See them fail. GREEN: steps 5 to 7 of ADR-0106 (f) with the check of a copy of `lib/recovery-steps.sh`. Check: `pitr-volume-test` passes.
- [ ] 2.4 (deploy/ci) [mechanical] [haiku medium] RED: `deploy/ci/pgtools.sh` does not run `pitr-volume-test`. GREEN: add `pitr-volume-test` to the list of suites. Check: `bash deploy/ci/check.sh` passes.

## 3. Script pitr-volume: recovery and checks

- [ ] 3.1 (deploy/pgtools) [sonnet high] RED: add `tests/lib/pitr-volume-start-cases.sh`. The stub of `pg_ctl` records its `-o` argument. That argument holds these settings:
  - the `restore_command` of ADR-0068 (l) for `/wal`
  - `hot_standby=off`, `archive_mode=off`, and `listen_addresses=''`
  - the target settings of ADR-0106 (a), only with `--target-time`

  No configuration file of the target changes. A failed start prints `pitr_check.log_tail` lines and one `ERROR` line. See them fail. GREEN: step 8 of ADR-0106 (f) with `lib/recovery-steps.sh`, `pitr_volume.start_timeout` 1800 s, and `pitr_check.ready_poll`. Check: `pitr-volume-test` passes.
- [ ] 3.2 (deploy/pgtools) [sonnet high] RED: add `tests/lib/pitr-volume-check-cases.sh` for checks 1 and 2 of ADR-0106 (g). Each case below gives its `ERROR` line:
  - a cluster still in recovery
  - a timeline that is not above the timeline of the base backup
  - a history file whose switch LSN lies before the newest segment of `/wal`
  - a switch LSN before `PITR_EXPECTED_LSN`

  A `.partial` file counts as its segment. With a target time, check 2 does not run. See them fail. GREEN: checks 1 and 2 as `design.md` D3 says. Check: `pitr-volume-test` passes.
- [ ] 3.3 (deploy/pgtools) [sonnet high] RED: add cases to `tests/lib/pitr-volume-check-cases.sh` for checks 3 to 6 of ADR-0106 (g). Each case below gives its `ERROR` line:
  - a break of the audit chain
  - a missing marker entry, and a bad `PITR_MARKER_SEQ`
  - a changed `postgresql.auto.conf`, and a remaining `recovery.signal`
  - a difference to `restore-roles.expected`

  See them fail. GREEN: checks 3 to 6 with `lib/recovery-steps.sh` and `lib/restore-steps.sh`. Check: `pitr-volume-test` and `restore-volume-test` pass.
- [ ] 3.4 (deploy/pgtools) [sonnet high] RED: add `tests/lib/pitr-volume-stop-cases.sh` with these cases:
  - A passed run stops the server with `--mode=fast` and `pitr_volume.stop_timeout` 600 s, requires `shut down`, and runs `sync`.
  - Its last line `pitr volume completed` holds the values of ADR-0106 (d) and the field `drill`.
  - With `--drill`, the target holds the empty file `pitr-drill`. Without it, no such file exists.
  - A value with a quote or a control character is cleaned. A failed stop gives one `ERROR` line.

  See them fail. GREEN: steps 10 to 12 of ADR-0106 (f). Check: `pitr-volume-test` passes.

## 4. Compose and the values of the deploy files

- [ ] 4.1 (deploy, recipe-server) [sonnet high] RED: add `pitr-volume` to the lists of `restore-volume` in `tests/deploy_values/compose.rs` and `compose_mutations.rs`. The service has these keys:
  - the profile `restore`, `network_mode: none`, and no secret
  - the mounts `walarchive:/wal:ro` and the tmpfs `/run/pgpass`, and `COMPOSE_SUBNET`
  - 512 MiB without swap, and 0.5 CPUs

  See them fail. GREEN: add the service of `design.md` D4 to `deploy/compose.yaml`. Check: `cargo test -p recipe-server --test deploy_values` passes. `docker compose -f deploy/compose.yaml config` with the CI values passes.
- [ ] 4.2 (recipe-server) [sonnet high] RED: add rows to `tests/deploy_values/file_mutations.rs`. These constants of `bin/pitr-volume` equal their rows of `reliability.md` 7.11:
  - `pitr_volume.start_timeout` and `pitr_volume.stop_timeout`
  - `pitr_check.ready_poll`, `pitr_check.log_tail`, and `disk.warn_percent`

  A mutation of each value fails the test. See them fail. GREEN: name the constants in `bin/pitr-volume` with a comment that names the row. Check: `cargo test -p recipe-server --test deploy_values` passes.

## 5. WAL archive scripts: guard and end time

- [ ] 5.1 (deploy/pgtools) [sonnet high] RED: add cases to `tests/lib/wal-entry-cases.sh`. The stub of `IDENTIFY_SYSTEM` answers timeline 2.
  - A segment or a `.partial` file of timeline 1 next to a matching `.system-identifier` gives the `ERROR` line `segments of another timeline are in the WAL archive`. The script writes no file.
  - A history file of timeline 1 passes. Segments of timeline 2 pass.

  See them fail. GREEN: `wal_identity_guard` of `lib/wal-identity.sh` as `design.md` D5 says (ADR-0106 (m)). Check: `walarchive-test` passes.
- [ ] 5.2 (deploy/pgtools) [sonnet high] RED: add the cases of task 5.1 for `basebackup-once` to `tests/lib/wal-base-cases.sh`. A refused run writes no base backup and deletes nothing. See them fail if the guard does not run there. GREEN: change `bin/basebackup-once` only if it does not call the guard before its first write. Check: `walarchive-test` passes.
- [ ] 5.3 (deploy/pgtools) [sonnet high] RED: add cases to `tests/lib/wal-base-cases.sh`. A passed run publishes a base backup with `backup_end_time`. The file holds one time of the grammar of ADR-0106 (a), at or after the time of the stub of `pg_verifybackup`. A failed `pg_verifybackup` writes no such file. See them fail. GREEN: the step of `design.md` D2 in `bin/basebackup-once` (ADR-0106 (c)). Check: `walarchive-test` passes.

## 6. Shared switch steps in CI

- [ ] 6.1 (deploy/ci) [sonnet high] RED: add a test to `deploy/ci/test_restore_switch.py`. `restore-switch.sh` sources `deploy/ci/switch-steps.sh`, and the functions of the role passwords and of the WAL line come from that file. See it fail. GREEN: move these two functions into `switch-steps.sh` without a change of behavior (`design.md` D7). Check: `python3 -I -B deploy/ci/test_restore_switch.py` passes, and `bash deploy/ci/check.sh` passes.
- [ ] 6.2 (deploy/ci) [sonnet high] RED: add a test to `test_restore_switch.py`. The switch function also comes from `switch-steps.sh`. See it fail. GREEN: move the switch function into `switch-steps.sh` without a change of behavior. Check: `test_restore_switch.py` passes, and `bash deploy/ci/check.sh` passes.
- [ ] 6.3 (deploy/ci) [sonnet high] RED: add cases to `test_restore_switch.py` with these rules:
  - The move of the WAL line also takes a file `00000002.history` and no other new name.
  - The change log file of the run holds the timeline of the old volume.
  - The switch back compares the identifier and the timeline. An entry without a timeline compares the identifier only.

  See them fail. GREEN: change `switch-steps.sh` (ADR-0106 (k), (l)). Check: `test_restore_switch.py` passes.
- [ ] 6.4 (deploy/ci) [sonnet high] RED: add cases to `test_restore_switch.py` for the parameter of the source line.
  - With `pitr volume completed`, the switch compares the identifier and the timeline and skips the state check of the old volume. It writes `unreadable` when `pg_controldata` fails.
  - With `pitr volume completed`, the switch refuses a volume with the file `pitr-drill` and a line with `"drill":true`.
  - With `restore volume completed`, the behavior stays the same.

  See them fail. GREEN: the parameter of `design.md` D7 (ADR-0106 (j)). Check: `test_restore_switch.py` passes.

## 7. CI job pitr-recovery

- [ ] 7.1 (deploy/ci, .github/workflows) [sonnet high] RED: create `deploy/ci/test_pitr_recovery.py` with these rules:
  - `ci.yml` has the job `pitr-recovery` with the trigger of the job `compose`.
  - The job has the steps of `design.md` D8 in their order, a step of failure diagnostics, and a `cleanup` step with `if: always()`.
  - `bash -n` passes on the script.

  See it fail. GREEN: add the job and `deploy/ci/pitr-recovery.sh` with `prepare` and `cleanup`. Add the test to `deploy/ci/check.sh`. Check: `python3 -I -B deploy/ci/test_pitr_recovery.py` passes.
- [ ] 7.2 (deploy/ci) [sonnet high] RED: add stubbed cases for `markers` and `drill`.
  - `markers` commits two `ChangeOwnPassword` commands and records a target time between them. It waits for the flush of the marker 2 without `pg_switch_wal()`.
  - `drill` runs `pitr-volume --target-time --drill` with the marker 1 and checks the newest audit entry.
  - `drill` expects a refusal of the switch check on the drill volume. Then it deletes the drill volume.

  See them fail. GREEN: the two subcommands. Check: `test_pitr_recovery.py` passes.
- [ ] 7.3 (deploy/ci) [sonnet high] RED: add stubbed cases for `refusals`. A target time after the newest commit gives the `ERROR` line of ADR-0106 (a). A copy of `/wal` without one segment after the base backup gives the `ERROR` line of the gap. No production volume changes. See them fail. GREEN: the subcommand. Check: `test_pitr_recovery.py` passes.
- [ ] 7.4 (deploy/ci) [sonnet high] RED: add stubbed cases for `crash` and `recover`.
  - `crash` stops `app` and `backup` and reads `pg_current_wal_lsn()`. It fails when the segment of that position exists in `/wal` as a complete file.
  - `crash` then kills `db` with `SIGKILL`, stops `walarchive`, deletes `global/pg_control` of the old volume, and records the start time.
  - `recover` creates a new volume and runs `pitr-volume` with the marker 2 and `PITR_EXPECTED_LSN`.

  See them fail. GREEN: the two subcommands. Check: `test_pitr_recovery.py` passes.
- [ ] 7.5 (deploy/ci) [sonnet high] RED: add stubbed cases for `switch` and `wal-line`. They call `switch-steps.sh` with `pitr volume completed`. The change log file holds `unreadable` for the old volume. The move takes the history files. See them fail. GREEN: the two subcommands. Check: `test_pitr_recovery.py` passes.
- [ ] 7.6 (deploy/ci) [sonnet high] RED: add stubbed cases for `check`.
  - A login with the password of the marker 2 passes, and the password of the marker 1 fails.
  - `backup-now` and `restore-test` pass. `basebackup-once`, a new marker commit, and `pitr-check` pass on the new WAL line.
  - The time of each step is in the log. A time above 3600 s from `crash` to the start of `app` fails the step.

  See them fail. GREEN: the subcommand. Check: `test_pitr_recovery.py` passes.
- [ ] 7.7 (deploy/ci) [sonnet high] RED: add stubbed cases for `refuse`. A copy of `/wal` with one segment of the old timeline gives the `ERROR` line of ADR-0106 (m), and no production volume changes. See them fail. GREEN: the subcommand. Check: `test_pitr_recovery.py` passes, and `bash deploy/ci/check.sh` passes.

## 8. Runbook and change log

- [ ] 8.1 (docs/ops/runbook.md) [sonnet high] RED: `grep` finds `This page gives no steps for that recovery` in the section "Restore into production". GREEN: add the section "Recover from the WAL archive" after "Restore into production" with the order of ADR-0106 (i) and the switch of (j). Link it from the warning. Check: `ste-lint.py` reports 0 hard violations on `docs/ops/runbook.md`.
- [ ] 8.2 (docs/ops/runbook.md) [sonnet high] RED: `grep` finds no section "Recovery drill". GREEN: add the section of ADR-0106 (o) after "Recover from the WAL archive". It records the time of the fill and of the last base backup and compares the sum with 1 hour. The last step of "Upgrade" links the drill. Check: `ste-lint.py` reports 0 hard violations on `docs/ops/runbook.md`.
- [ ] 8.3 (docs/ops/runbook.md, deploy/ci) [sonnet high] RED: add a test to `test_restore_switch.py`. The `find` command of step 7 sub-step 4 of "Restore into production" equals the expression of `switch-steps.sh`, with the history files. The section "Switch back" names the timeline and the rule for an entry without a timeline. See it fail. GREEN: change these two places of the runbook (ADR-0106 (k), (l)). Check: `test_restore_switch.py` passes, and `ste-lint.py` reports 0 hard violations.
- [ ] 8.4 (docs/ops/runbook.md) [sonnet high] RED: `grep` finds no `TimeLineID` in three places. The first place is the change log line of step 5 sub-step 4 of "Restore into production". The second place is the file `old-system-identifier` of step 7 sub-step 3. The third place is the change log entry of "Upgrade". GREEN: add the timeline to these three places (ADR-0106 (k), (l), `design.md` D9). Check: `ste-lint.py` reports 0 hard violations.
- [ ] 8.5 (docs/ops/runbook.md) [sonnet high] RED: `grep` finds no rule for the start time of a manual statement as a superuser. GREEN: state the rule once in the section on the change log, and link it from each step that runs such a statement (ADR-0106 (b)). Check: `ste-lint.py` reports 0 hard violations.
- [ ] 8.6 (docs/ops/runbook.md) [mechanical] [haiku medium] RED: `grep` finds two bullets of "Known gaps". They are about the recovery from the WAL archive and the recovery time objective. GREEN: delete the two bullets. Add ADR-0106 to the list "Related". Check: `ste-lint.py` reports 0 hard violations.
- [ ] 8.7 (CHANGELOG.md) [mechanical] [haiku medium] RED: `grep` finds no `pitr-volume` in the section `[Unreleased]`. GREEN: add the entries of `proposal.md`, section "What Changes", with the **BREAKING** guard of ADR-0106 (m). Add the step of the Migration Plan: run `basebackup-once` once after the release. Check: `ste-lint.py` reports 0 hard violations on `CHANGELOG.md`.

## 9. Spec of the main capability

- [ ] 9.1 (openspec) [mechanical] [haiku medium] RED: a `grep -rF` loop over each quoted `ERROR` line of `specs/operations/spec.md` and the names `backup_end_time` and `pitr-drill` searches `deploy/`. The loop fails for each text that the code does not hold. GREEN: change a delta only when the code of groups 1 to 8 changed a name or a message of the spec. Check: the loop passes, and `openspec validate pitr-recovery --strict` passes.

## 10. Verification

- [ ] 10.1 (workspace) [sonnet high] Run `cargo fmt --all --check`. Check: exit code 0.
- [ ] 10.2 (workspace) [sonnet high] Run `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`. Check: exit code 0 and no warning.
- [ ] 10.3 (workspace) [sonnet high] Run `cargo test --workspace --all-features --locked` with Docker. Check: exit code 0 and 0 failed tests.
- [ ] 10.4 (deploy/pgtools) [sonnet high] Build the image and run every suite of `deploy/ci/pgtools.sh` with `CI=true`. Check: every suite passes with 0 skipped cases.
- [ ] 10.5 (openspec) [sonnet high] Run `openspec validate --all --strict`. Check: every item is valid.
- [ ] 10.6 (docs) [sonnet high] Run `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'`. Check: 0 hard violations.
- [ ] 10.7 (.github/workflows) [sonnet high] Start `ci.yml` with `workflow_dispatch` on the branch. This run is the gate of the change, because the job `compose` and its pgtools suites do not run on a pull request (ADR-0094). Check: the jobs `compose`, `upgrade`, `restore-switch`, `pitr-recovery`, and `migration-freeze` pass. The pull request description links the run.

## Workflow follow-up

- Review the plan with `plan-eng-review` (Opus) before the approval of `tasks.md`.
- Review the code with `code-review` (Opus, effort high) after group 8. Fix every CRITICAL and HIGH finding, with one Sonnet agent per finding.
- Archive the change with `openspec-archive-change` after task 10.7.
