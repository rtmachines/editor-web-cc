# Tasks

Marks after the scope:

- `[codex]`: Codex with the model `gpt-6.1-sol` does the task. It writes shell scripts, Docker files, CI files, and their tests.
- `[sonnet]`: a Sonnet subagent with the effort `high` does the task. It writes Rust code and tests, or it runs the STE checks.
- `[mechanical]`: a Haiku subagent does the task. It writes a simple doc edit.
- `[opus]`: an Opus subagent checks the result. It changes no file.

Each task is one agent run. Each task of a crate or a script starts with a failing test and shows the red step before the fix. The report of each task gives the test name, the red step, and the failure line.

The owner accepted ADR-0151 on 2026-11-08 with option K1 of item (k).

## Checks and waves

Rules of the checks:

- A Rust task checks only its own crates with `-p <crate>`: `cargo test -p <crate> --all-features --locked`, `cargo clippy -p <crate> --all-targets --all-features --locked -- -D warnings`, and `cargo fmt --all --check`.
- A red task ends with a test that fails to build or fails. Its check names the failing tests. It runs no clippy.
- Two tasks run in parallel only when neither task changes a non-test file of a crate that the other task builds.
- The orchestrator runs `cargo test --workspace --all-features --locked`, the workspace clippy, and fmt after waves 1, 2, and 7. These waves leave no red Rust test.
- A Haiku task runs no `ste-lint.py`. Task 7.10 runs the STE checks of every changed doc.

| Task | Agent | Crate or path | Wave | Needs |
|---|---|---|---|---|
| 1.1 | `[sonnet]` | `docs/architecture/reliability.md` §7.11, `recipe-server` (`tests/deploy_values/units.rs`) | 1 | none |
| 2.1 | `[codex]` | `deploy/pgtools/tests` (`restore-test`) | 1 | none |
| 2.4 | `[codex]` | `deploy/ci` (mode of `/wal`) | 1 | none |
| 2.2 | `[codex]` | `deploy/pgtools/tests` (`basebackup-once`, `walarchive-entry`) | 2 | 2.1 |
| 2.5 | `[codex]` | `deploy/ci` (switch of the WAL line) | 2 | 2.4 |
| 3.1 | `[sonnet]` | `recipe-app` (port, memory adapter) | 2 | 1.1 |
| 2.3 | `[codex]` | `deploy/pgtools` (scripts, library, image) | 3 | 2.2 |
| 4.1 | `[sonnet]` | `recipe-server` (red test of the scan limit) | 3 | 1.1 |
| 5.1 | `[sonnet]` | `recipe-app` (red tests of the job) | 3 | 3.1 |
| 4.2 | `[sonnet]` | `recipe-server` (`limited_read.rs`) | 4 | 4.1 |
| 4.3 | `[sonnet]` | `recipe-server` (`check_results.rs`, `Host`) | 5 | 3.1, 4.2 |
| 5.2 | `[sonnet]` | `recipe-server` (red test of the served stack) | 6 | 4.3, 5.1 |
| 5.3 | `[sonnet]` | `recipe-app`, `recipe-server` (job logic, wiring) | 7 | 5.1, 5.2 |
| 6.1 | `[opus]` | `crates/recipe-server` (check, no change) | 8 | 5.3 |
| 7.1 | `[mechanical]` | `docs/architecture/reliability.md` §3.7 | 8 | 2.3, 5.3 |
| 7.4 | `[mechanical]` | `docs/ops/runbook.md` | 8 | 2.3, 2.5, 5.3 |
| 7.5 | `[mechanical]` | `docs/adr/` (Status lines, index) | 8 | 5.3 |
| 7.6 | `[mechanical]` | `CHANGELOG.md` | 8 | 5.3 |
| 7.8 | `[mechanical]` | `docs/architecture/domain-model.md` §5 | 8 | 5.3 |
| 7.9 | `[mechanical]` | `docs/architecture/overview.md` §5 | 8 | 5.3 |
| 7.2 | `[mechanical]` | `docs/architecture/reliability.md` §7.7, §8 | 9 | 7.1 |
| 7.3 | `[mechanical]` | `docs/architecture/reliability.md` §7.10 | 10 | 7.2 |
| 7.7 | `[mechanical]` | `docs/architecture/reliability.md` §1.1 | 11 | 7.3 |
| 7.10 | `[sonnet]` | every doc of tasks 1.1 and 7.1 to 7.9 (STE checks) | 12 | 6.1, 7.1 to 7.9 |
| 8.1 | `[sonnet]` | `openspec` (gate) | 13 | every other task |

The tasks 7.1, 7.2, 7.3, and 7.7 change one file, so they run one after the other.

## 1. Settings first

- [x] 1.1 (docs/architecture, recipe-server) [sonnet] Add the three rows of §7.11 and their units in the deploy test (ADR-0151 (c), (j), design D6).
  - Red step: add the rows first. Then `cargo test -p recipe-server --test deploy_values --all-features --locked` fails on the units of the new rows. Paste the failure lines.
  - Row: `restore_test.max_age_alert` with the value 8 days and the sections 3.7 and 7.7, after the row `restore_test.time`.
  - Row: `basebackup.max_age_alert` with the value 8 days and the sections 3.7 and 7.7, after the row `basebackup.time` / `basebackup.retention`.
  - Row: `check_result.max_bytes` with the value `256 bytes` and the sections 3.7 and 7.7.
  - Each new cell cites ADR-0151. No key of `serve` gets a value.
  - Change in `crates/recipe-server/tests/deploy_values/units.rs`: `DURATION_KEYS` gets both new duration keys. `EXPLAINED` gets `("check_result.max_bytes", "bytes")`.
  - Check: the deploy test passes. The checks of `-p recipe-server` pass. `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/architecture/reliability.md'` reports 0 hard violations.

## 2. Scripts, image, and CI

- [x] 2.1 (deploy/pgtools/tests) [codex] Failing test: `restore-test` writes its result file (ADR-0151 (a) to (g), design D2 to D4, D8, D13).
  - Test file: a new case file `deploy/pgtools/tests/lib/restore-result-cases.sh`, sourced and run by `restore-test-test`.
  - Case: a passed run leaves `/backups/restore-test.result` with mode `0644` and one line that matches `^\{"result":"passed","finished_at":"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z"\}$`.
  - Case: a run that fails in the count step leaves the result `failed`. A run that a `TERM` signal ends leaves the result `failed`.
  - Case: an old file with `failed` becomes `passed` after a passed run. No name `.tmp-restore-test.result` stays.
  - Case: a run without `.backup-marker` and a run that finds the lock held write no file and keep an old file unchanged.
  - Case: a failed write of the file (a stub of `mv` that fails) gives one `ERROR` line and the exit code 1.
  - Case: a run in the image with `RESTORE_GLOBALS_DIR` unset sources `/etc/pgtools/check-result.sh` and writes the result.
  - Case in `tests/lib/delete-cases.sh` of `backup-test`: the retention of `backup-once` keeps `restore-test.result` and `.tmp-restore-test.result`.
  - Change of the helper `set_listing` in `tests/lib/restore-helpers.sh` (line 105): exclude only the exact path `./restore-test.result`.
  - `restore_failed_checks` (line 117) and the happy case of `restore-run-cases.sh` (line 23) also check the result file on its own.
  - Check: `restore-test-test` and `backup-test` in the image `recipe-pgtools` fail on the new cases only. Paste the failure lines.
- [x] 2.2 (deploy/pgtools/tests) [codex] Failing test: `basebackup-once` writes its result file. `walarchive-entry` sets the mode of `/wal` (ADR-0151 (a) to (g), (f1), (k), design D8, D13).
  - Test file: `deploy/pgtools/tests/lib/wal-base-cases.sh`.
  - Case: a passed run leaves `/wal/base-backup.result` with mode `0644` and the result `passed`.
  - Case: a run with a failed `pg_verifybackup` leaves the result `failed`. The case keeps the check of `case_wal_base_verify_fails` that no base backup and no segment goes.
  - Case: a run with a failed `pg_basebackup` and a run with a failed `pg_archivecleanup` leave the result `failed`.
  - Case: a run that a `TERM` signal ends during `pg_basebackup` leaves the result `failed` and the exit code 143. The temp base backup goes.
  - Case: a run without `.walarchive-marker` and a run that finds `/wal/.lock` held write no file.
  - Case: the stub of `pg_archivecleanup` gets no argument that names `base-backup.result`.
  - Case: a run in the image with `WALARCHIVE_LIB_DIR` unset sources `/etc/pgtools/check-result.sh` and writes the result.
  - Change of the helper `wal_listing` in `tests/lib/wal-helpers.sh` (line 87): exclude only the exact path `./base-backup.result`.
  - `base_expect_unchanged` (line 32) and `case_wal_base_verify_fails` (line 219) also check the result file on its own.
  - Test file: `deploy/pgtools/tests/lib/wal-entry-cases.sh`. Case: after the marker check, `walarchive-entry` gives `/wal` the mode `0711`. Without the marker, the mode stays.
  - Check: `walarchive-test` in the image `recipe-pgtools` fails on the new cases only. Paste the failure lines.
- [x] 2.3 (deploy/pgtools) [codex] Write the result files and the signal traps. Set the mode of `/wal` (ADR-0151 (a) to (k1), design D3, D8).
  - Change: the new file `deploy/pgtools/lib/check-result.sh` with the function `write_check_result <directory> <name> <result>` of design D3. It uses `chmod 0644` and `mv -T -f`.
  - Change: `restore-test` sources it from `${RESTORE_GLOBALS_DIR:-/etc/pgtools}`. `basebackup-once` sources it from `$LIB_DIR` (`WALARCHIVE_LIB_DIR`).
  - Change: both scripts set `result_armed=1` after `flock` and call `write_check_result` last in `cleanup`.
  - Change: `basebackup-once` gets `trap 'exit 143' TERM`, `trap 'exit 130' INT`, and `trap 'exit 129' HUP` after `trap cleanup EXIT`.
  - Change: `walarchive-entry` runs `chmod 0711 -- "$WAL_DIR"` after the marker check.
  - Change: `deploy/pgtools/Dockerfile` (line 25) copies `lib/check-result.sh` to `/etc/pgtools/`. `deploy/pgtools/.dockerignore` admits `!lib/check-result.sh`.
  - Change: the header comments of the three scripts name the result file and ADR-0151.
  - Check: `restore-test-test`, `backup-test`, and `walarchive-test` pass in the image. `shellcheck` reports nothing on the changed files.
  - Check: `cargo test -p recipe-server --test deploy_values --all-features --locked` passes.
- [x] 2.4 (deploy/ci) [codex] Failing test and fix: the CI preparation gives `/wal` the mode `0711` (ADR-0151 (k1), design D8).
  - Red step: `deploy/ci/test_restore_switch.py` (line 1485) asserts `sudo install -d -o 999 -g 999 -m 0711 "$RECIPE_WALARCHIVE_DIR"`. The test fails.
  - Change: `deploy/ci/prepare.sh` (line 21) uses `-m 0711`. `deploy/ci/modes.sh` (line 8) expects `711`, and its message names `711`.
  - Check: `python -I -m unittest` on `deploy/ci/test_restore_switch.py` passes. `shellcheck` reports nothing on the changed files.
- [x] 2.5 (deploy/ci) [codex] Failing test and fix: the step `wal-line` moves `/wal/base-backup.result` (ADR-0151 (q), design D12).
  - Red step: a case of `deploy/ci/test_restore_switch.py` expects the result file in the moved paths and in the target directory. The test fails.
  - Change: `deploy/ci/switch-steps.sh` (lines 418 to 427) adds `$RECIPE_WALARCHIVE_DIR/base-backup.result` to the moved paths when the file exists.
  - Check: `python -I -m unittest` on `deploy/ci/test_restore_switch.py` passes. `shellcheck` reports nothing on the changed files.

## 3. Port of the results

- [x] 3.1 (recipe-app) [sonnet] Failing test and fix: the port `CheckResults` and the memory adapter `MemoryCheckResults` (ADR-0151 (j3), (r), design D5).
  - Test file: the new `crates/recipe-app/src/memory/check_results/tests.rs`.
  - Test: a new adapter gives `Ok(None)` for each `ScheduledCheck`. A set result comes back for its check only.
  - Test: an unreadable check gives `CheckResultFault::Unreadable`. A planned fault fails one call. A clone shares the values.
  - Test: a planned delay makes `newest` return after that delay on the paused clock of `tokio`.
  - Test file: `crates/recipe-app/src/ports/object_tests.rs`. Test: the `Display` text of each `CheckResultFault` names no path.
  - Change: the new `crates/recipe-app/src/ports/check_results.rs`, exported in `ports/mod.rs`. The new `crates/recipe-app/src/memory/check_results.rs`, built like `memory/backups.rs`.
  - Check: the checks of `-p recipe-app` pass.

## 4. Time limit and result reader

- [x] 4.1 (recipe-server) [sonnet] Failing test: a blocked scan of the backup directory gives `BackupFault::Unreadable` after the limit (D2-1, ADR-0151 (o), design D10).
  - Test file: `crates/recipe-server/src/backups/tests.rs`. The task changes no other file.
  - Test: a reader that blocks on a barrier gives `Unreadable` with the kind `TimedOut` after a limit of 50 ms. Use `SystemDiskSpace::with_reader` in `disk.rs` as the model of the hook.
  - Test: while that blocked call still runs, a second call gives `Unreadable` at once and starts no second reader call.
  - Test: after the blocked call ends, the next call runs the reader again.
  - Check: `cargo test -p recipe-server --all-features --locked backups` fails to build or fails on the new tests only.
- [x] 4.2 (recipe-server) [sonnet] Move the limit code into `read_with_limit` and use it in both adapters (ADR-0133, ADR-0151 (o), (p), design D10).
  - Change: the new module `crates/recipe-server/src/limited_read.rs` with `StalledPaths`, `Call`, `FinishOnDrop`, `LimitFault`, and `read_with_limit`.
  - Change: `SystemDiskSpace::free_space` and `FileBackupDirectory::newest_complete_set` call `read_with_limit`. `FileBackupDirectory::new` takes the limit.
  - Change: `disk.rs` keeps the surface of design D10 for `disk/tests.rs`: `use crate::limited_read::Call`, `Call::new(PathBuf, StalledPaths)`, `Reader`, `with_reader`, `is_stalled`, and `stalled_handle`.
  - Change: the implementation of `Host::backups` in `serve/process.rs` passes `health.disk_check_interval`. `serve/test_support/stub.rs` keeps its memory adapter.
  - Test file: the new `crates/recipe-server/src/limited_read/tests.rs` with the mark tests of ADR-0133 (c) to (e) on the free function.
  - Check: every test of `crates/recipe-server/src/disk/tests.rs` passes without a change. The tests of task 4.1 pass. The checks of `-p recipe-server` pass.
- [x] 4.3 (recipe-server) [sonnet] Failing test and fix: `FileCheckResults` reads a result file under the time limit. See ADR-0151 (b), (b1), (c), (o), design D4, D10.
  - Test file: the new `crates/recipe-server/src/check_results/tests.rs`.
  - Test: a file with `passed` and a valid time gives `Ok(Some(..))`. A missing file gives `Ok(None)`.
  - Test: each of these gives `Malformed`, not a passed result:
    - an unknown key, a missing key, or the result `ok`,
    - a time without `Z`, a time with `+00:00`, a lowercase `t` or `z`, or a fraction of a second,
    - a second line, or a file above `check_result.max_bytes`,
    - a link, or a directory.
  - Test: a file without a read right gives `Unreadable`. A reader that blocks gives `Unreadable` with the kind `TimedOut`.
  - Change: the new module `crates/recipe-server/src/check_results.rs`. It calls `read_with_limit` with the path of the file as the key. It parses the time with the fixed format of design D4.
  - Change: `crates/recipe-server/Cargo.toml` gives `time` the features `parsing` and `macros`. `Cargo.lock` does not change.
  - Change: the trait `Host` (`serve/host.rs` line 174) gets `check_results(&self, backups: &Path, walarchive: &Path) -> Arc<dyn CheckResults>`.
  - `serve/process.rs` and `serve/test_support/stub.rs` implement it. The stub gives a hook that sets a result for each check. Task 5.3 calls the method.
  - Check: the checks of `-p recipe-server` pass.

## 5. Alert conditions

- [x] 5.1 (recipe-app) [sonnet] Failing test: the job `alerts` tests `restore_test` and `base_backup` in the order of `AlertCondition::ALL` (ADR-0151 (h) to (j2a), design D5, D11).
  - Test file: the new module `crates/recipe-app/src/jobs/alerts/tests/checks.rs`, declared in the test module of `alerts.rs`.
  - Test: each scenario of three delta requirements, for both checks. The requirements are "Alerts of the restore test and the base backup", "Read fault of a result file", and "Comparison time of a result".
  - Test: a passed result 1 second inside the limit is inactive. A passed result 1 second over the limit is active.
  - Test: a read fault during the start grace is active and writes one `WARN` line of the fault.
  - Test: one run with both conditions active sends both names to the webhook port in the order of `ALL`.
  - Test of design D11 on the paused clock: a read of 50 s and a result 100 s after `database_now` are inactive.
  - Test of design D11: a result later than the comparison moment by `alerts.interval` plus 1 s is active (ADR-0093 (a)).
  - Changed test: `crates/recipe-app/src/ports/webhook_tests.rs` lines 17 and 18 expect 12 entries. `RestoreTest` and `BaseBackup` follow `BackupAge`, with the names `restore_test` and `base_backup`.
  - Changed test: `jobs/alerts/tests/conditions.rs` lines 203 to 225 list twelve names. `make_every_condition_active` (line 241) sets `failed` for both new checks.
  - Changed test: `jobs/alerts/tests/settings.rs` expects both new defaults of 8 days.
  - Changed test support: `jobs/alerts/tests/support.rs` and `tests/faults.rs` build `AlertPorts` with `MemoryCheckResults`.
  - Check: `cargo test -p recipe-app --all-features --locked` fails to build, or fails only on the new and the changed tests.
- [x] 5.2 (recipe-server) [sonnet] Failing test: the served stack sends `restore_test` and `base_backup` through the webhook (delta spec, scenario "Failed result").
  - Test file: a new sibling module of `crates/recipe-server/src/serve/tests/counters.rs`.
  - Test: a stub host with a `failed` result for each check sends the bodies `{"alert":"restore_test"}` and `{"alert":"base_backup"}` once in one run.
  - The test names the conditions only as text, so it builds before task 5.3.
  - Change: only the test support, if the test needs a hook.
  - Red step: the test builds and fails, because the job does not read the results yet. Paste the failure line.
  - Check: `cargo test -p recipe-server --all-features --locked` fails on the new test only.
- [x] 5.3 (recipe-app, recipe-server) [sonnet] Add the two variants, the two limits, the comparison moment, and the job logic. Wire the port (design D5, D6, D11).
  - Change: `crates/recipe-app/src/ports/webhook.rs` (two variants, `ALL` of 12). `AlertSettings` gets the two limits. `AlertPorts` gets `checks`.
  - Change: `Moment::after(Duration)`. `run_once` notes `tokio::time::Instant::now()` after the snapshot. `Alerts::test` calls the new method `check_result`.
  - Change: the module doc of `alerts.rs` and the doc of `run_once` say twelve conditions.
  - Change in `recipe-server`: `serve/jobs.rs` (line 189) sets `checks` from `Host::check_results`.
  - Change in `recipe-server`: `webhook/tests.rs` lines 252, 258, and 389 follow `AlertCondition::ALL.len()`. Lines 409 and 425 do not use `ALL` and stay.
  - Check: the tests of tasks 5.1 and 5.2 pass. The checks of `-p recipe-app` and `-p recipe-server` pass.

## 6. Check of the shared function

- [x] 6.1 (crates/recipe-server) [opus] Check that each blocking read of a backup volume uses `read_with_limit` (plan section 4.2, task 3).
  - Read `disk.rs`, `backups.rs`, `check_results.rs`, and `limited_read.rs`.
  - Search with `rg -n 'spawn_blocking' crates/recipe-server/src`. Each hit in an adapter of a backup volume or of the disk lies inside `read_with_limit`.
  - Check: the report lists each hit and its place. A finding goes back to the task that owns the file.

## 7. Documents

- [x] 7.1 (docs/architecture) [mechanical] Add the two rows of `reliability.md` §3.7 after `backup_age`, and the grace sentence (ADR-0151 (h) to (j2a), (l)).
  - The row `restore_test` names the source `/backups/restore-test.result`. The row `base_backup` names `/wal/base-backup.result`.
  - Each new row says that a read fault makes the condition active, also during the start grace.
  - The row `backup_age` gets the sentence: "A scan that waits longer than `health.disk_check_interval` is also `BackupFault::Unreadable` (ADR-0151 (o))."
  - The paragraph "Start grace" names the two new conditions and their limits.
  - The paragraph "Clock trust" names the comparison moment of the two result reads (ADR-0151 (j2)).
- [x] 7.2 (docs/architecture) [mechanical] Add the result files to `reliability.md` §7.7 and §8 (ADR-0151 (a) to (g), (k), (k1), (o) to (q2)).
  - §7.7 "Rules of every script": the result file is the one file with the mode `0644`. The top directory of `walarchive` has `0711`.
  - §7.7 "WAL archive" and "Copies and restore": one bullet each on the result file of `basebackup-once` and of `restore-test`.
  - §7.7 "WAL line after a switch" (line 753): the move also takes `/wal/base-backup.result`. In a gap, the file stays.
  - §8: the map of stalled paths moves to `limited_read.rs`. Each of the three adapters owns one map.
- [x] 7.3 (docs/architecture) [mechanical] Add command 4 to step 5 of `reliability.md` §7.10 (ADR-0151 (m), (n)).
  - Step 5 (line 883): "these three commands" becomes "these four commands".
  - Command 4: `docker compose exec walarchive basebackup-once`. Check that the run writes `passed`.
  - One bullet: in the first upgrade with ADR-0151, `restore_test` and `base_backup` can be active until step 5 ends.
- [x] 7.4 (docs/ops) [mechanical] Update the runbook (ADR-0151, design D9, D12).
  - Add the rows `restore_test` and `base_backup` to the table of the section "Alarms", after `backup_age`. Each row names the log command, the first action, and the manual run.
  - The `install -d` line of `RECIPE_WALARCHIVE_DIR` (line 168) uses the mode `0711`. The sentence at line 522 names `0711`.
  - The section "Upgrade" names command 4 of step 5 and the two expected alerts.
  - Step 4 of the switch (line 765): the `find` command also moves `base-backup.result`. Step 6 also checks that no `base-backup.result` remains.
  - Step 7 of the gap procedure (near line 1997): the list of the files that stay names `base-backup.result`.
- [x] 7.5 (docs/adr) [mechanical] Amend the Status lines with "Amended by ADR-0151 (...)". Copy each note into its row of `docs/adr/README.md`.
  - ADR-0060: items (a) (the port `CheckResults`) and (y) (the start grace of the two new conditions).
  - ADR-0066: item (m) (the mode `0711` of `/wal`).
  - ADR-0067: step 9 (the result file of `restore-test`).
  - ADR-0068: item (g) (the result file of `basebackup-once`).
  - ADR-0083: item (b) (step 5 ends with four commands). The note says that "the three commands of (b)" in items (b) to (d) are now four.
  - ADR-0103: item (s) step 4 (the move of `base-backup.result`).
  - Check: the numbers in each Status line equal the numbers in its index row.
- [x] 7.6 (CHANGELOG.md) [mechanical] Add the `[Unreleased]` entry. It names the two alerts, the result files, the mode of `/wal`, command 4 of step 5, and the scan limit.
- [x] 7.7 (docs/architecture) [mechanical] Add the reason type to `reliability.md` §1.1 (ADR-0151 (j3), design D7).
  - The rule sentence that names `DiskFault` and `BackupFault` (line 29) also names `CheckResultFault` of the port `CheckResults`.
- [x] 7.8 (docs/architecture) [mechanical] Add the port to `domain-model.md` §5 (ADR-0151 (r)).
  - The table of the ports of the jobs (line 595) gets the row `CheckResults` after `BackupDirectory`. The adapter is `recipe-server`. The operation: the job `alerts` reads the newest result of each weekly check.
- [x] 7.9 (docs/architecture) [mechanical] Add the port to `overview.md` §5 (ADR-0151 (r)).
  - The bullet of `recipe-server` (line 125) names `CheckResults` with `DiskSpace`, `BackupDirectory`, and `AlertWebhook`.
- [x] 7.10 (docs) [sonnet] Run the STE checks of every doc that tasks 1.1 and 7.1 to 7.9 changed. Fix only the form.
  - Files: `reliability.md`, `domain-model.md`, `overview.md`, `docs/ops/runbook.md`, the changed ADRs, `docs/adr/README.md`, and `CHANGELOG.md`.
  - Run `ste-lint.py` on each file. Check the sentence length, the active voice, no semicolons, and one word for one meaning.
  - Check: `ste-lint.py` reports 0 hard violations on each file. The report pastes the last line of each run.

## 8. Gate

- [x] 8.1 (openspec) [sonnet] Run the checks of the change.
  - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked`, `cargo fmt --all --check`.
  - `openspec validate backup-result-alerts --strict`.
  - `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'`.
  - Check: every command passes. The report pastes the last lines of each output.
