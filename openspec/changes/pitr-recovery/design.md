# Design

## Context

See `proposal.md`, section "Why". ADR-0106 holds every decision of this change. `reliability.md` 7.7, 7.8, and 7.11 already hold the normative text. This design gives the build plan only.

Existing parts that the change reuses:

- `deploy/pgtools/bin/pitr-check` already recovers a scratch cluster from the newest base backup and `/wal` (ADR-0077). It holds the restore command, the start wait, the audit chain SQL, and the marker check.
- `deploy/pgtools/bin/restore-volume` already fills a new volume. It holds the empty-target gate, the uid gate, the shell baselines of ADR-0105 (q), the checks of ADR-0103 (m), and the clean stop.
- `deploy/pgtools/bin/basebackup-once` writes each base backup into a temp directory, runs `pg_verifybackup`, and publishes it with one rename.
- `deploy/pgtools/lib/wal-identity.sh` holds the guard of ADR-0103 (s). `walarchive-entry` and `basebackup-once` source it.
- `deploy/ci/restore-switch.sh` holds the switch, the role passwords, and the new WAL line of the CI job `restore-switch`.
- `crates/recipe-server/tests/deploy_values/` compares `compose.yaml` and the script constants with `reliability.md` 7.11.

## Goals / Non-Goals

**Goals:**

- One production tool `pitr-volume` that shares its recovery steps with `pitr-check`.
- A base backup selection that never replays a fault inside the run of the selected base backup.
- One switch procedure for `restore-volume` and `pitr-volume`, with the changes of ADR-0106 (j).
- A CI job that proves the recovery point, the target time, the refusals, the guard of ADR-0106 (m), and the recovery time.

**Non-Goals:**

- No change of production Rust code, of a migration, or of the role grants.
- No new error code. The pgtools scripts write `ERROR` lines with fixed messages (`reliability.md` 7.7, script rules).
- No change of the gap procedure of ADR-0099, except that the history files stay in `/wal`.

## Decisions

### D1. Shared recovery library

A new file `deploy/pgtools/lib/recovery-steps.sh` holds the steps that `pitr-check` and `pitr-volume` share (ADR-0106 (h)):

- the `restore_command` of ADR-0068 (l) for a given WAL directory
- the check of a copy: `pg_verifybackup --quiet --ignore=backup_end_time`, then the delete of `backup_end_time` from the copy
- the start of the server with its settings and the connection wait of `pitr_check.ready_poll`
- the log tail of `pitr_check.log_tail` after a failed start
- the audit chain SQL and the marker check of ADR-0077 (e)

The file only defines names and functions, as `wal-identity.sh` does. `pitr-check` changes in two points. It passes its settings with `pg_ctl -o` and no longer appends them to `postgresql.auto.conf`. It also uses the check of a copy above.

Its scratch copy is deleted after each run, so its behavior stays the same. The suites `pitr-start-test` and `wal-pitr-cases.sh` prove it.

Alternative: copy the steps into `pitr-volume`. Rejected, because two copies of the chain SQL can drift.

### D2. End time of a base backup

`basebackup-once` gets one step between `pg_verifybackup` and the first `sync` (ADR-0106 (c)):

- It reads `date -u +%s` and adds 1. It writes that time in the grammar of ADR-0106 (a) into `$tmp_dir/backup_end_time`, with one newline.
- The rename then publishes the base backup and the file together. A killed run leaves only a `.tmp-base_*` directory.
- The retention deletes the file with its base backup, because the file lies inside the directory.

Alternative: a file next to the base backup directory. Rejected, because the retention and the readers of ADR-0077 (d) then need a second grammar.

### D3. Script `pitr-volume`

`deploy/pgtools/bin/pitr-volume` follows ADR-0106 (f) step by step. Implementation points:

- Arguments: none, `--target-time <time>`, and `--drill`. `--drill` without `--target-time` fails. The grammar check runs first. Then `date -u -d` must give back the same text, so a date such as `2026-02-30` fails.
- Selection with a target: the script reads `backup_end_time` of each base backup of the grammar. The file must match the grammar of ADR-0106 (a). A base backup without the file uses the line `START TIME: <YYYY-MM-DD HH:MM:SS> UTC` of `backup_label`.
- A base backup with a malformed file, a zone other than `UTC`, or neither line never wins. No match stops the run.
- Free space: `df --output=avail,size -B1` of the target gives the free bytes and the disk size. The free bytes minus two sizes must stay at or above `disk.warn_percent` of the disk size. The two sizes are the size of the base backup and the size of the segment files at the top of `/wal`.
- Copy: `cp -a` of the contents of the base backup into the target, then `chmod 0700` of the target. Then the check of a copy of D1 runs.
- Settings: `pg_ctl -o` passes the settings of ADR-0106 (f) step 8. The private socket lies in a `mktemp -d` directory under `/run/pgpass`.
- The server reads the `pg_hba.conf` of the copy. Its line `local all postgres peer` admits the script, because the script runs as uid 999 with the name `postgres`.
- End check of ADR-0106 (g) step 2: the script reads the last line of `pg_wal/<new timeline>.history` in the target. That line gives the switch LSN.
- `pg_walfile_name()` of the switch LSN gives the segment. The 16 hex characters after the timeline must be at or above the newest segment of `/wal` on the base timeline. A `.partial` file counts as its segment.
- With `PITR_EXPECTED_LSN`, the switch LSN must also be at or above that value. `pg_lsn` in SQL compares the two values. A value outside the grammar `^[0-9A-F]{1,8}/[0-9A-F]{1,8}$` stops the run before the copy.
- Roles: the checks of ADR-0103 (m) reuse the functions of `restore-volume` through `lib/restore-steps.sh`. `COMPOSE_SUBNET` fills the placeholder.
- Output: the values of ADR-0106 (d) come from one `psql` query each. Text values pass the function `clean_json_text` of `pitr-volume` before they enter the JSON line. It deletes each unsafe character, so a value of only unsafe characters becomes empty and stops the run. The line also holds `"drill":true` or `"drill":false`.
- Drill: with `--drill`, the script writes the empty file `pitr-drill` at the top of the target after the clean stop and before `sync`.
- Constants: `pitr_volume.start_timeout`, `pitr_volume.stop_timeout`, and `disk.warn_percent` are named constants with a comment that names their row of 7.11. The test `deploy_values` compares them with the row.

### D4. Service in Compose

`compose.yaml` gets the service `pitr-volume` next to `restore-volume`, with the same keys except the mounts. The mounts are `walarchive:/wal:ro` and the tmpfs `/run/pgpass`. The entrypoint is the script, so the target time is its argument.

The tests `compose.rs` and `compose_mutations.rs` of `deploy_values` add the service to the lists of `restore-volume`.

### D5. Timeline check of the guard

`wal_identity_guard` of `wal-identity.sh` also keeps field 2 of the answer of `IDENTIFY_SYSTEM`, the timeline. After the identifier check, it reads the first 8 hex characters of each segment name at the top of `/wal`. It compares them with that timeline as an 8-digit upper-case hex number.

A difference stops the script with the `ERROR` line of ADR-0106 (m). A `.partial` file of another timeline also counts, because the grammar of `has_segment` matches `.partial`. A history file does not count, because it is not a segment.

The suite `walarchive-test` gets the cases in `wal-entry-cases.sh` and `wal-base-cases.sh`.

### D6. Move of the history files

The runbook step 4 of "Restore into production" step 7 gets a second expression: `-regextype posix-extended -regex '.*/[0-9A-F]{8}\.history'`. It matches the grammar `^[0-9A-F]{8}\.history$` of ADR-0106 (k).

The section "A gap in the WAL stream" keeps its patterns. `deploy/ci/restore-switch.sh` uses the same expression, and `test_restore_switch.py` pins it. The value `WAL_IDENTITY_STEP` of `upgrade.sh` does not change, because it only reads segments.

### D7. Shared switch steps in CI

`deploy/ci/restore-switch.sh` holds the switch, the role passwords, and the new WAL line. A new file `deploy/ci/switch-steps.sh` takes these functions, and both job scripts source it. The move happens in two tasks: first the passwords and the WAL line, then the switch.

The switch function gets one parameter for the source line: `restore volume completed` or `pitr volume completed`. With `pitr volume completed`, it changes three points (ADR-0106 (j)):

- It skips the state check of the old volume and writes `unreadable` when `pg_controldata` fails.
- It compares the identifier and the timeline of the new volume with the line.
- It refuses a volume with the file `pitr-drill` and a line with `"drill":true`.

The change log file of the CI run also gets the timeline (ADR-0106 (l)). The switch back compares the timeline only when the entry holds one.

Alternative: copy the functions into the new script. Rejected, because the switch is the most safety-critical part, and two copies can drift.

### D8. CI job `pitr-recovery`

`deploy/ci/pitr-recovery.sh` has one subcommand per step of the job, as `restore-switch.sh` has. The subcommands are `prepare`, `markers`, `drill`, `refusals`, `crash`, `recover`, `switch`, `wal-line`, `check`, `refuse`, and `cleanup`.

- `markers` uses the HTTP calls and the flush wait of the job `compose` (ADR-0077 (a) to (c)). It waits until the next full second after the marker 1 and records that second as the target time. It waits one more second before the marker 2.
- `drill` runs `pitr-volume --target-time --drill` into a drill volume while the stack runs. It requires the marker 1 as `PITR_MARKER_SEQ` and `PITR_MARKER_HASH`.
- `drill` checks that the newest audit entry in the last line is the marker 1. It runs the switch check on the drill volume and expects a refusal. Then it deletes the drill volume.
- `refusals` runs `pitr-volume` with a target time after the newest commit and expects the `ERROR` line of ADR-0106 (a).
- `refusals` also copies `/wal` into a scratch volume and deletes one segment after the base backup. A run with that copy as `/wal` expects the `ERROR` line of the gap.
- `crash` runs `docker compose stop app backup` and reads `pg_current_wal_lsn()`. The segment of that position must exist in `/wal` only as a `.partial` file.
- Then `crash` runs `docker compose kill -s SIGKILL db` and `docker compose stop walarchive`. It deletes `global/pg_control` of the old volume in a container. It records the start time of the recovery time.
- `recover` runs `pitr-volume` without a target, with the marker 2 and `PITR_EXPECTED_LSN`.
- `check` logs in with the password of the marker 2 and checks that the password of the marker 1 fails. It runs `backup-now` and `restore-test`.
- `check` also runs `basebackup-once`, a new marker commit, and `pitr-check` on the new WAL line. It writes the time of each step and fails above 3600 s, the recovery time objective of `reliability.md` 7.7.
- `refuse` copies a segment of the old timeline from the target directory of the move into a copy of `/wal` in a scratch volume. It runs `walarchive-entry` on the copy and expects the `ERROR` line of ADR-0106 (m).
- The 60 s flush wait is a bound of the CI job, as in ADR-0077 (c). The 3600 s bound is the recovery time objective. Section 7.11 gets no row for them.

`deploy/ci/test_pitr_recovery.py` pins the order of the steps in `ci.yml` and in the script, the text of the commands, and the subcommands with a stubbed `docker`, as `test_restore_switch.py` does.

### D9. Runbook

`docs/ops/runbook.md` gets two sections after "Restore into production": "Recover from the WAL archive" and "Recovery drill". They follow ADR-0106 (i) and (o). Each step that starts `db` or `app` names the tag of the volume (ADR-0105 (a)).

Other places of the runbook change too:

- The change log line of step 5 sub-step 4 of "Restore into production" also holds `Latest checkpoint's TimeLineID`.
- The file `old-system-identifier` of step 7 sub-step 3 also holds the timeline.
- The change log entry of "Upgrade" holds the timeline, and its last step links the drill.
- The section "Switch back" names the timeline and the rule for an entry without a timeline.
- Each step that runs a statement as a superuser first writes its start time in UTC into the change log (ADR-0106 (b)). The section on the change log states this rule once, and the steps link it.
- "Known gaps" loses the bullets of the recovery from the WAL archive and of the recovery time. The warning of "Restore into production" links the new section.

### D10. Agents

Each code or test task runs on Sonnet with effort high. A `[mechanical]` task runs on Haiku. Opus reviews the code after group 8 (`code-review`). STE checks of docs run on Sonnet. No task runs on Codex.

## Risks / Trade-offs

- [The end check reads the history file of the new timeline] → PostgreSQL 16 writes it at the promotion. A missing file stops the run with an `ERROR` line.
- [`pitr-check` changes its way of passing the settings and its check of a copy] → its suites and the job `compose` prove the same result. The change lands with D1 in one push.
- [A base backup from before this change has no `backup_end_time`] → its `START TIME` counts, and a target inside its run stops with `FATAL`. The Migration Plan runs `basebackup-once` once after the release.
- [The drill holds `/wal/.lock` while the weekly base backup wants it] → the runbook runs the drill outside `basebackup.time`. `basebackup-once` refuses without a delete, and the operator runs it again at once.
- [The timeline check refuses an archive that a restore left without the move] → that state is wrong anyway. The `ERROR` line names the action.
- [CI kills `db` on a runner disk] → the job uses its own Compose project and volumes, as `restore-switch` does, and `cleanup` deletes them.
- [One week of WAL may not replay in 1800 s on a slow host] → the drill measures the time. A longer time needs a new ADR with a new value.
- [A manual fault without a record of its start time] → the operator estimates the time. The values of ADR-0106 (d) and the unchanged old volume limit the damage.

## Migration Plan

1. The release with this change installs the new guard. An existing archive holds segments of one timeline, so the guard passes.
2. After the release, the operator runs `basebackup-once` once. Then a base backup with `backup_end_time` exists.
3. The release notes name the new service, the drill, the end time file, and the timeline in the change log.
4. A rollback to the previous release deletes `pitr-volume` and the timeline check. The archive stays valid for the old guard.
5. After a rollback, the old `pitr-check` fails on a base backup with `backup_end_time`, because its manifest does not list the file. `pitr-check` runs only in CI (ADR-0077), so production does not change.
