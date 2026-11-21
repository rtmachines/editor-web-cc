# Design

## Context

See `proposal.md` - Why. The rules come from these sources. ADR-0106 (g) step 2 gives the end check of `pitr-volume`. ADR-0106 (p) step 4 gives its CI refusals. ADR-0094 runs the stack jobs only on `main` and on a dispatch.

Facts at `20d21fb`:

- `pitr-volume` lines 552 and 553: without `PITR_EXPECTED_LSN`, check 2 compares only the segment of the switch LSN with the newest segment of `/wal`.
- The pgtools suites in `deploy/pgtools/tests/` use stubs of `pg_ctl`, `psql`, and the other PostgreSQL tools. The header of `walarchive-test` excludes the real recovery. So no pgtools suite can show what PostgreSQL does with a damaged record.
- `deploy/ci/pitr-recovery.sh` runs a real stack and real WAL replay in the CI job `pitr-recovery`. Its step `refusals` already runs `pitr-volume` on a copy of `/wal` without one segment (ADR-0106 (p) step 4).
- `deploy/ci/test_pitr_recovery.py` tests `pitr-recovery.sh` with a docker stub. `deploy/ci/check.sh` runs it in the job `ci-scripts` on each pull request.
- The job `pitr-recovery` does not run on a pull request. The step `refusals` of run 44908754315 on `main` (`20d21fb`) took 7 s of its limit of 10 minutes.

## Goals / Non-Goals

**Goals:**

- The V-02 proof gives one recorded result before the design of change B.
- The CI step `refusals` keeps the damage setup after the proof, so change B gets a red test.
- No new error code, no new key of `reliability.md` §7.11, no new ADR, and no change of `docs/architecture/`.

**Non-Goals:**

- A fix of V-02. Change B decides it after the proof.
- V-06 and V-07. They move to change B ("Notes for change B").
- A change of `pitr-volume` or of the pgtools suites.
- A change of `.github/workflows/ci.yml` other than the name and the comment of the step `refusals` (D1).

## Decisions

### D1. V-02 proof in the CI step `refusals`

Decision of the owner (2026-11-21): the proof runs in `deploy/ci/pitr-recovery.sh`, step `refusals`, as its fourth run, with the constant `EARLY_END_REFUSAL_REQUIRED`. The fix plan named `wal-pitr-cases.sh`, but that file uses stubs.

Damage point:

- The step `markers` reads `pg_current_wal_insert_lsn()` after `commit_marker 1` and before `commit_marker 2`. Call this position L.
- Assumption: L is the start of the next WAL record, also when another backend writes WAL. A class I result shows that the assumption is wrong.
- The same step reads `pg_walfile_name_offset(L)`. It reads `wal_segment_size` and `wal_block_size` in bytes from the column `setting` of `pg_settings`.
- It writes L, the segment name, the byte offset, and the two sizes to `$CI_STATE/early-end.state`.
- After `wait_marker_flush`, the step checks that the segment of L is the segment of the flushed position. Else it fails with `markers: the record before marker 2 is not in the newest segment.`

Run in `refusals`, after the gap run and its `delete_wal_copy`:

1. `copy_wal_archive` makes a new copy of `/wal`.
2. The step checks that `<segment>.partial` exists in the copy and that `<segment>` does not exist. This file is the newest segment of the base timeline.
3. The step inverts one byte of the record at L in `<segment>.partial`. The byte is the first byte of `xl_crc`, at L plus 20.
   - If L modulo `wal_block_size` is above `wal_block_size` minus 24, the header crosses a page. The step then inverts the byte at L.
   - Assumption: the record header has 24 bytes, and `xl_crc` starts at byte 20. A class I result shows a wrong assumption.
4. The step reads the byte again with `sudo od` and checks that it changed. It writes with `sudo dd ... conv=notrunc` and never touches the real `/wal`.
5. On a new scratch volume, the step runs `pitr-volume` with the copy as `/wal:ro`. It sets no `PITR_EXPECTED_LSN` and no `PITR_MARKER_*`. This is the run without `db` of runbook line 1040.
6. The step deletes the scratch volume and the copy, as the gap run does.
7. The step writes one log line for each value of "Proof result" that the run knows. Each line has the form `refusals: early end: <name>=<value>`.
   - The names are `lsn`, `segment`, `offset`, `wal_segment_size`, `wal_block_size`, `byte_offset`, `byte_old`, `byte_new`, `marker1_seq`, `marker2_seq`, `exit_code`, `error_line`, `last_line`, and `class`.
   - A value that does not apply is `none`. The job uploads no artifact, so these lines are the only record of the run.

Outcomes. The step sorts the run into exactly one class:

| Class | Observation | Meaning |
|---|---|---|
| R (refusal) | Exit code other than 0. The log holds the configuration-hash line of `pitr-volume`, so `pitr-volume` passed its preconditions. The log holds exactly one `ERROR` line. That line is the last line and comes from `pitr-volume`. The step records the message | The run refuses the damaged record. The message can differ from the gap line, because check 2 passes for an end inside the newest segment |
| S (completion) | Exit code 0. The log holds no `ERROR` line. The last line is the `INFO` line `pitr volume completed` of `pitr-volume`, with the `chain_seq` of marker 1 and the action `OwnPasswordChanged` | V-02 is proven. The replay ended before marker 2, and marker 2 is lost |
| I (inconclusive) | Every other result is class I. A refusal before the configuration-hash line is one example. Other examples: no `ERROR` line or more than one, a completed line with marker 2, a failed precondition, or a timeout | The proof did not work |

A timeout of the step kills it, so the log holds no line `class`. This result also counts as class I.

CI on both outcomes. `EARLY_END_REFUSAL_REQUIRED` selects the accepted classes:

- `0`: the step accepts R and S. It writes one log line with the class and fails on I.
- `1`: the step accepts only R with the message in `EARLY_END_ERROR`. It fails on S, on I, and on another message.

Task 1.1 sets `0` and an empty `EARLY_END_ERROR`. Task 1.2 runs the proof. Task 1.3 sets the final values from the proof result:

- Result R: task 1.3 sets `1` and writes the recorded message into `EARLY_END_ERROR`. The case then asserts the refusal in each run of the job.
- Result S: task 1.3 keeps `0`. A comment names V-02 and change B. Change B sets `1` as its red test, in the same commit as its fix.

Reason for this option: CI stays green, and no run asserts that the completion is right. The damage setup runs in each job, so change B gets a ready red test. A class I always fails, so a broken setup never counts as a proof.

Rejected options:

- A skipped case. `pitr-recovery.sh` has no skip, and a skip under `CI=true` is a failure in the pgtools suites.
- A case that asserts the class S. It locks in the defect.
- A proof outside the repository. Change B then has no red test, and the setup can drift.

Step limit and step name in `.github/workflows/ci.yml`:

- One more recovery fits in the 10 minutes of `refusals`, so the limit stays. Task 1.1 changes the limit only if the dispatch of task 1.2 shows that the run needs more time.
- Task 1.1 may change the name and the comment of the step `refusals`, so that they name the fourth run. It reads the rest of `ci.yml` first.
- The tests of `test_pitr_recovery.py` that read the step names and the budget comment must still pass.

Decision of the owner (2026-11-21): the ADR of change B amends ADR-0106 (p) to name the fourth run. This holds for both results of the proof. This change edits no ADR.

### D2. Runbook line 1040

Task 1.4 replaces the second sentence of line 1040. The first sentence stays: "If `db` does not run, omit the three options `-e`."

- Result R: "Without them, the script checks only that the recovery ends in the newest segment of the archive. A damaged record in that segment can stop the run. In CI, <check> then wrote the line `<message>`." The text uses the message and the check of "Proof result".
- Result S: "Without them and without a target time, the end check accepts an end in the newest archive segment or later. A damaged record in that segment can end the replay early. The recovered volume then lacks the commits after that record. The run still completes. Compare the newest audit entry of the last line with your expectation in step 7."

## Risks / Trade-offs

- [The proof needs a dispatch of `ci.yml`, because `pitr-recovery` does not run on a pull request] → Task 1.2 names the actors and the commands.
- [Another backend writes WAL after the read of L] → L is still the start of a record. Marker 2 still follows it.
- [The damaged byte changes `xl_tot_len` when the header crosses a page] → PostgreSQL then reads a bad length or a bad checksum. Both are invalid records. A class I result shows a damage that does not work.
- [The fourth run is not in ADR-0106 (p) until change B merges] → The owner accepted this order (D1).

## Migration Plan

No migration. The change touches one CI script, its test, and one runbook line.

## Notes for change B

Decision of the owner (2026-11-21): V-06 and V-07 move to change B. Each one changes or adds a rule of a normative document. These findings at `20d21fb` stay for change B.

V-06, failed stop of a scratch server:

- `restore-test` line 186: `delete_stale_runs` calls `stop_server "$dir/data" || true` and then deletes the directory.
- Line 288 sets `result_armed=1` before the call on line 293. So a `fail` in `delete_stale_runs` writes the result `failed`.
- The current-run path of `cleanup` (lines 127 to 136) keeps the directory and sets `status=1` on a failed stop. This is the safe form.
- `pitr-check` lines 90 to 93: `cleanup` runs `pg_ctl ... stop ... || true` and then `rm -r`. The exit status stays 0 after `pitr check passed`.
- Rule to change: ADR-0067 (g) and `reliability.md` §7.7 say that a trap deletes the scratch cluster on each exit.
- Test hooks: `STUB_PGCTL_STOP_FAIL=1` of `restore-stubs.sh` also fails the stop of the current run. So the red assertion is a kept stale directory and no `initdb` event, not the exit code. `wal-stubs.sh` has no stop-fail switch yet.

V-07, manifest time ahead of the clock:

- `backup-health` lines 64 and 65: `age=$((now - mtime))` has no lower bound. A negative age counts as young.
- Tolerance source: `alerts.interval` (60 s, `reliability.md` §7.11). The last bullet of ADR-0093 (a) names it as the largest skew that the job `alerts` trusts. That rule compares with `database_now`.
- `reliability.md` §7.1 and §7.7 name no clock rule for `backup-health`. The §7.11 row of `alerts.interval` names the sections 2.4 and 3.7 only.
- A new constant needs a row in `SCRIPT_ASSIGNMENTS` (length 24) of `deploy_values/files/scripts.rs`. It also needs a row in `SCRIPT_CONSTANTS` (length 15) of `deploy_values/file_mutations/scripts.rs`.
- The date stub of `tests/lib/stubs.sh` passes `date +%s` to the real clock. A border case of 60 s and 61 s needs a fixed epoch.

## Open points for the owner

1. V-02 and the spec. The scenario "Recovery after the loss of the pgdata disk" of the `operations` spec has the GIVEN "an intact WAL archive disk". Its THEN says that `db` runs with every commit that the archive received.
   - Question: does an archive with a damaged record count as "an intact WAL archive disk"?
   - If it counts, result S makes the THEN wrong, and change B changes the scenario. This change adds no V-02 delta.

## Proof result

A Haiku agent fills this section from the log of the dispatch (task 1.2). Write each value as the log shows it. Do not round or explain. Write `none` for a row that does not apply.

- Commit of the run: 2c264e5 (branch `fix/review-deploy-fixes-2`, run 44921173499, 2026-11-21)
- Run URL of the job `pitr-recovery`: https://github.com/rtmachines/editor-web-cc/actions/runs/44921173499
- Class (R, S, or I): S. V-02 is proven. The replay ended before marker 2, and `pitr-volume` reported completion.
- L, segment name, and byte offset from the lines `lsn`, `segment`, and `offset`: `lsn` 0/50D1BE0, `segment` 000000010000000000000005, `offset` 859104.
- Values of `wal_segment_size` and `wal_block_size`: 16777216 and 8192.
- Offset and old and new value of the inverted byte: offset 859124, old value 164, new value 91.
- `chain_seq` of marker 1 and of marker 2: 23 and 24.
- Exit code of `pitr-volume`: 0.
- `ERROR` line of `pitr-volume`, for class R: none.
- Check of `pitr-volume` that writes that line, for class R: none.
- Last line of `pitr-volume`: `{"timestamp":"2026-11-21T15:56:16Z","level":"INFO","job":"pitr-volume","message":"pitr volume completed","base_backup":"base_20261121_155109","system_identifier":"7710652830984839193","timeline":2,"chain_seq":23,"occurred_at":"2026-11-21T15:56:05.932648Z","action":"OwnPasswordChanged","heartbeat_last_at":"2026-11-21T15:56:04.911859Z","drill":false}`
