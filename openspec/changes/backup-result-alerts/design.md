# Design

## Context

See `proposal.md`, section "Why". ADR-0151 holds the decisions. ADR-0060, ADR-0066, ADR-0067, ADR-0068, ADR-0073, ADR-0075, ADR-0083, ADR-0093, ADR-0103, and ADR-0133 hold the context. This design links to them and does not repeat them.

The owner accepted ADR-0151 on 2026-11-08 with option K1 of item (k). The 8-day limits and the 256-byte cap are rows of `reliability.md` §7.11, not keys of `serve`.

The state of the code on the branch `feat/backup-result-alerts` at `0519b9e`. The author read each place in the source:

| Topic | State | Place |
|---|---|---|
| Condition list | `AlertCondition` has ten variants. `ALL` follows the table of `reliability.md` §3.7 | `crates/recipe-app/src/ports/webhook.rs` lines 13 to 70 |
| Dispatch of the job | `run_once` takes `database_now` from the snapshot, then tests the conditions one after the other. `test` sends `LowDisk` and `BackupAge` to their own reads | `crates/recipe-app/src/jobs/alerts.rs` lines 303 to 386 |
| Model of a file condition | `backup_age`: a fault gives a `WARN` line and `true`. `None` uses `in_grace` | `alerts.rs` lines 437 to 450 |
| Settings | `AlertSettings::default()` equals §7.11. `serve/plan.rs` uses the default. No key of `serve` sets it | `alerts.rs` lines 62 to 111, `crates/recipe-server/src/serve/plan.rs` line 178 |
| Ports of the job | `AlertPorts` holds four ports | `alerts.rs` lines 113 to 128 |
| Memory adapter | `MemoryBackupDirectory` with `CallFaults` | `crates/recipe-app/src/memory/backups.rs` |
| Backup scan | `spawn_blocking(...).await` without a limit | `crates/recipe-server/src/backups.rs` line 58 |
| Disk limit | `Call`, `FinishOnDrop`, the type `Stalled`, and `tokio::time::timeout` in `free_space`. `disk/tests.rs` imports `Call`, `Reader`, and `SystemDiskSpace` from `super` | `crates/recipe-server/src/disk.rs` lines 21 to 159 |
| Wiring | The trait `Host` with `backups(&Path)` and its two implementations | `crates/recipe-server/src/serve/host.rs` lines 174 and 240, `serve/process.rs` line 128, `serve/test_support/stub.rs` line 727 |
| Units of §7.11 | `kind_of` gives `Count` to a key that no list names. `Count` accepts no unit word. `EXPLAINED` lists the rows with one word after the number | `crates/recipe-server/tests/deploy_values/units.rs` lines 35 to 100, 155, 186 |
| Time parsing | `recipe-server` uses `time` without the features `parsing` and `macros` | `crates/recipe-server/Cargo.toml` line 37 |
| Exit trap of `restore-test` | `cleanup` computes the final status and exits with it. `TERM`, `INT`, and `HUP` exit with 143, 130, and 129. The lock is `flock -n 9` | `deploy/pgtools/bin/restore-test` lines 105 to 132, 261 to 267 |
| Exit trap of `basebackup-once` | `cleanup` deletes the pgpass file and the temp base backup. Only an `EXIT` trap exists. The lock is `/wal/.lock` | `deploy/pgtools/bin/basebackup-once` lines 80 to 101, 207 to 212 |
| Library directories | `restore-test` sources from `RESTORE_GLOBALS_DIR`. `basebackup-once` sources from `WALARCHIVE_LIB_DIR`. Both default to `/etc/pgtools` | `restore-test` lines 37, 69, `basebackup-once` lines 24, 41, 237 |
| Image inputs | The Dockerfile copies each file of `lib/` by name. The allow list admits each file by its path. `check_library_copies` fails on a missing entry | `deploy/pgtools/Dockerfile` line 25, `deploy/pgtools/.dockerignore`, `crates/recipe-server/tests/deploy_values/files.rs` line 772 |
| Marker check of `walarchive-entry` | The script stops without `/wal/.walarchive-marker` | `deploy/pgtools/bin/walarchive-entry` line 135 |
| Script tests | `restore-test-test` with `tests/lib/restore-run-cases.sh`. `walarchive-test` with `tests/lib/wal-base-cases.sh` and `wal-entry-cases.sh` | `deploy/pgtools/tests/` |
| Listings of the script tests | `wal_listing` lists every top file of `/wal`. `set_listing` lists every file of `/backups` except `.scratch`. Failed and happy cases compare a full listing | `tests/lib/wal-helpers.sh` line 87, `restore-helpers.sh` line 105, `wal-base-cases.sh` lines 32, 219, `restore-helpers.sh` line 117, `restore-run-cases.sh` line 23 |
| CI modes | `prepare.sh` creates `RECIPE_WALARCHIVE_DIR` with `0700`. `modes.sh` expects `700`. `test_restore_switch.py` asserts the `prepare.sh` line | `deploy/ci/prepare.sh` line 21, `deploy/ci/modes.sh` line 8, `deploy/ci/test_restore_switch.py` line 1485 |
| Move of a WAL line | The runbook and `switch-steps.sh` move the segments, the `.partial` file, the history files, and `.system-identifier` | `reliability.md` line 753, `docs/ops/runbook.md` line 765, `deploy/ci/switch-steps.sh` lines 418 to 427 |
| Users and modes | `app` runs as uid 65532. The scripts run as uid 999 with `umask 077`. `/wal` has the mode `0700` | `deploy/Dockerfile` line 62, `deploy/pgtools/Dockerfile` line 32, `docs/ops/runbook.md` lines 167, 168 |

## Goals / Non-Goals

**Goals:**

- Each design point of `docs/plan/2026-11-08-fix-plan.md` section 4.1 has one decision below.
- The new reader and the backup scan share the time limit code of the disk adapter.
- The new conditions reuse `Moment`, `in_grace`, and the fault rule of `backup_age`.

**Non-Goals:**

- A change of the webhook adapter, the body, or the client.
- A change of `backup-health` or `walarchive-health`. A failed check does not make a container unhealthy.
- A change of `restore-volume`, `pitr-check`, or `pitr-volume`. They write no result file.
- A change of the comparison time of `backup_age` and of the other conditions.

## Decisions

### D1: Place of each result file (plan point 1)

`restore-test` writes `/backups/restore-test.result`. `basebackup-once` writes `/wal/base-backup.result` (ADR-0151 (a)). Each script writes only to the volume that it already mounts read-write.

The plan expects that `app` can read both files through its read-only mounts. That holds for `/backups` only. `/wal` has the mode `0700` (ADR-0066 (m)), so uid 65532 cannot open a file in it. The owner chose option K1 of ADR-0151 (k): the top directory of `walarchive` gets the mode `0711`.

### D2: Names outside the grammars (plan point 2)

The names `restore-test.result` and `base-backup.result` match neither `^recipe_[0-9]{8}_[0-9]{4}(_premigrate|_now)?\.dump\.sha256$` nor `^base_[0-9]{8}_[0-9]{6}$`. They are no WAL segment names. The retention, `backup-health`, `restore-test`, `FileBackupDirectory`, and `pg_archivecleanup` therefore ignore them (ADR-0151 (d)).

The temp name is `.tmp-<final name>`, as for a set (`reliability.md` §7.7). A test of task 2.1 runs the retention with both names present and checks that both stay.

### D3: Write through a temp name and a rename (plan point 3)

One shared shell function writes the file: `write_check_result <directory> <name> <result>`. It lives in the new file `deploy/pgtools/lib/check-result.sh`.

1. It writes the JSON line to `<directory>/.tmp-<name>`.
2. It sets the mode `0644` on the temp file.
3. It runs `mv -T -f` to the final name. The rename is atomic in one directory, so a reader sees the old file or the new file.

`restore-test` sources the file from `${RESTORE_GLOBALS_DIR:-/etc/pgtools}`. `basebackup-once` sources it from `$LIB_DIR`, which is `${WALARCHIVE_LIB_DIR:-/etc/pgtools}`.

The image needs the file at `/etc/pgtools/check-result.sh`. The `COPY` line of `lib/` in `deploy/pgtools/Dockerfile` and the allow list `deploy/pgtools/.dockerignore` each get the path `lib/check-result.sh`. The test `check_library_copies` of `deploy_values` fails without both entries.

The file holds no value of §7.11, so `LIBRARY_SCRIPTS` of `deploy_values/inputs.rs` does not change.

The rename replaces the old result. ADR-0074 (b) forbids a replace only for the files of a set (ADR-0151 (g)). A failed write logs one `ERROR` line and changes the exit code to 1. The old result then stays, and the age limit of D6 covers it.

### D4: Content of the file (plan point 4)

One line: `{"result":"passed","finished_at":"2026-11-08T05:41:07Z"}`. The script takes the time from `date -u +%Y-%m-%dT%H:%M:%SZ` at the write. The file holds no set name, no path, and no secret (ADR-0151 (b), (b1), (c)).

The reader in `recipe-server` accepts exactly these two keys (`serde(deny_unknown_fields)`). It accepts at most `check_result.max_bytes` (256) bytes.

The reader parses `finished_at` with one fixed format description: `[year]-[month]-[day]T[hour]:[minute]:[second]Z`. The `T` and the `Z` are literals. It does not use the RFC 3339 parser, because that parser also accepts an offset, a lowercase `t` or `z`, and a fraction.

The `time` dependency of `recipe-server` gets the features `parsing` and `macros` for this format. Any other content is the fault `Malformed`.

The reader uses `symlink_metadata` first. A link or a directory with the name is `Malformed`, like a link for a manifest (`backups.rs` line 93). A missing file is `Ok(None)`. Every other `io::Error` is `Unreadable`.

### D5: Two new conditions after `backup_age` (plan point 5)

`AlertCondition` gets `RestoreTest` (name `restore_test`) and `BaseBackup` (name `base_backup`). `ALL` becomes `[Self; 12]`, with the two variants directly after `BackupAge` (ADR-0151 (h)). The §3.7 table gets the two rows in the same place.

The new port of `recipe-app`, in `crates/recipe-app/src/ports/check_results.rs`:

- `ScheduledCheck` with `RestoreTest` and `BaseBackup`.
- `CheckOutcome` with `Passed` and `Failed`.
- `CheckResult { outcome: CheckOutcome, finished_at: Timestamp }`.
- `CheckResultFault` with `Unreadable { check: ScheduledCheck, source: io::Error }` and `Malformed { check: ScheduledCheck }`. It is a reason type without a code (ADR-0151 (j3)). Its `Display` text names the check and no path.
- `trait CheckResults: Send + Sync` with `async fn newest(&self, check: ScheduledCheck) -> Result<Option<CheckResult>, CheckResultFault>`.

`AlertPorts` gets the field `checks: Arc<dyn CheckResults>`. The memory adapter `MemoryCheckResults` follows `MemoryBackupDirectory`. It also takes a planned delay, so a test can move the paused clock during a read.

The job gets one method `check_result(check, limit, snapshot, moment, started) -> bool`. `test` calls it for both new variants. It applies ADR-0151 (i), (j1), (j2), and (j2a). D11 defines the comparison moment `at`:

| Read | Active |
|---|---|
| `Err(fault)` | Yes. One `WARN` line of the fault, also in the grace |
| `Ok(Some(result))` with `Failed` | Yes |
| `Ok(Some(result))` with `Passed` | When `at.older_than(finished_at, limit)`. An untrusted time counts as older |
| `Ok(None)` | When `!at.in_grace(first_setting_at, limit)` |

### D6: Limits in §7.11 (plan point 6)

The new rows are `restore_test.max_age_alert` 8 days, `basebackup.max_age_alert` 8 days, and `check_result.max_bytes` 256 bytes (ADR-0151 (c), (j)). The checks run each Sunday, so 8 days leave one day for a long run.

`AlertSettings` gets the fields `restore_test_max_age_alert` and `basebackup_max_age_alert`. They are no keys of `serve`, like `backup_max_age_alert`.

The deploy test reads every row of §7.11. `kind_of` gives `Count` to an unknown key, and `Count` rejects the word `days`. Task 1.1 therefore changes `deploy_values/units.rs` together with the rows:

- `DURATION_KEYS` gets `restore_test.max_age_alert` and `basebackup.max_age_alert`.
- `EXPLAINED` gets `("check_result.max_bytes", "bytes")`.

### D7: No new error code (plan point 7)

No variant gets a code. `CheckResultFault` is a reason type, like `BackupFault` and `DiskFault` (ADR-0025, ADR-0073 (b)). The supervisor logs no error, because the job handles the fault and continues. `reliability.md` §1.2 therefore gets no row. The task list keeps no §1.2 task.

The rules of `reliability.md` §1.1 name `DiskFault` and `BackupFault` as reasons. Task 7.7 adds `CheckResultFault` of the port `CheckResults` to that sentence.

### D8: Exit code and lock decide the result

Each script sets the flag `result_armed=1` directly after its `flock` passes. `cleanup` calls `write_check_result` last, only with the flag set. The final status 0 gives `passed`, every other status gives `failed` (ADR-0151 (e), (f)).

A run that the marker check or the lock check stops writes nothing. The run that holds the lock writes the result. A `SIGKILL` writes nothing, and the age limit covers it.

`basebackup-once` gets the three traps of `restore-test`: `trap 'exit 143' TERM`, `trap 'exit 130' INT`, and `trap 'exit 129' HUP` (ADR-0151 (f1)). Without them, a signal can reach the `EXIT` trap with the status 0, and the run publishes `passed`.

`walarchive-entry` runs `chmod 0711 -- "$WAL_DIR"` after its marker check (ADR-0151 (k1)). The CI preparation creates the directory with `0711`, and the CI mode check expects `711`.

### D9: The first run after the upgrade starts no clock (plan risk)

The open risk of the plan: an existing installation has no result file until the first Sunday run. ADR-0151 (l) to (n) decide that the first run does not start a clock.

Reasons:

1. A clock from the upgrade needs a stored time of the upgrade. That is a new key of `app_setting` and a new write path (ADR-0062).
2. Such a clock hides a failed or missing check for up to 8 days after each upgrade. Data safety comes first.
3. Step 5 of `reliability.md` §7.10 already runs `restore-test` with the new image. One more command, `docker compose exec walarchive basebackup-once`, writes the second file.
4. The start grace of ADR-0060 (x) already covers a new installation. Its length is the limit of each condition, so the first Sunday falls inside it.

The cost: in the first upgrade with ADR-0151, both alerts can be active from step 4 until step 5 ends. The runbook and the release notes name them.

### D10: One free function for the time limit (plan section 4.2, task 3)

The new module `crates/recipe-server/src/limited_read.rs` holds the code that `disk.rs` holds today:

- the type `StalledPaths` (today `Stalled`), `Call`, and `FinishOnDrop`,
- the enum `LimitFault` with `Stalled`, `TimedOut`, and `Join(tokio::task::JoinError)`,
- the free function

```rust
pub(crate) async fn read_with_limit<T, F>(
    limit: Duration,
    stalled: &StalledPaths,
    path: PathBuf,
    read: F,
) -> Result<T, LimitFault>
where
    T: Send + 'static,
    F: FnOnce(&Path) -> T + Send + 'static;
```

The function first checks the mark of `path`. Then it runs `read` in `spawn_blocking` under `tokio::time::timeout(limit, ..)`. It keeps the rules of ADR-0133 (c) to (e) without a change.

`disk/tests.rs` does not change. `disk.rs` therefore keeps this surface for its test module:

- `use crate::limited_read::Call`, so `super::Call` still resolves,
- `Call::new(PathBuf, StalledPaths)`,
- the type `Reader` and `SystemDiskSpace::with_reader`,
- `SystemDiskSpace::is_stalled(&Path)` and `SystemDiskSpace::stalled_handle() -> StalledPaths`.

Three callers use it:

| Adapter | Key of the mark | Mapping of `LimitFault` |
|---|---|---|
| `SystemDiskSpace::free_space` | the mount path | `DiskFault::Unreadable` with the kind `TimedOut` and the texts of `disk.rs` today |
| `FileBackupDirectory::newest_complete_set` | the directory | `BackupFault::Unreadable` with the kind `TimedOut` |
| `FileCheckResults::newest` | the path of the result file | `CheckResultFault::Unreadable` with the kind `TimedOut` |

Each adapter owns its own `StalledPaths`. The limit is `health.disk_check_interval` (ADR-0151 (o), (p)). `FileBackupDirectory::new` and `FileCheckResults::new` take the limit, and the implementation of `Host` passes it. A reader hook in each adapter lets a unit test give a reader that blocks, as `SystemDiskSpace::with_reader` does.

Alternative: a generic adapter type that wraps each port. Rejected, because the three ports have different methods and faults. One free function covers the shared part.

### D11: Comparison moment of a result read

`run_once` takes `database_now` from the snapshot before all reads. The backup scan waits up to `health.disk_check_interval` (30 s). The read of `restore-test.result` waits up to 30 s more.

A run of a script can write a result during these reads. Its `finished_at` is then later than `database_now` by up to 60 s plus the time of the snapshot. With the skew of `alerts.interval` (60 s), such a result can count as untrusted.

The job therefore notes `started = tokio::time::Instant::now()` directly after the snapshot. After each result read, it builds the comparison moment `at = moment.after(started.elapsed())`. `Moment::after` adds the elapsed time to `now` and keeps the skew (ADR-0151 (j2), (j2a)).

The rule keeps the clock check. A result later than `at` by more than `alerts.interval` stays untrusted. The other conditions keep the moment of the snapshot.

A test of task 5.1 uses the paused clock of `tokio` and the planned delay of `MemoryCheckResults`:

- A read that takes 50 s and a result 100 s after `database_now` give an inactive condition.
- A result later than `at` by `alerts.interval` plus 1 s gives an active condition.

### D12: Result file at a switch of the WAL line

Step 4 of ADR-0103 (s) moves the files of the old WAL line into the target directory. It also moves `/wal/base-backup.result` (ADR-0151 (q)).

Move, not delete, for three reasons:

1. The result describes the base backups of the old cluster. Those base backups move into the same target directory.
2. ADR-0103 (t) lets no step delete the files of the old line before its three conditions hold.
3. A killed first base backup of the new line then leaves no file. The condition `base_backup` becomes active after the grace, and no old `passed` result hides the fault.

ADR-0103 (r) needs a base backup of the new line before the first write of `app`. That run writes a new result, so a switch that follows ADR-0103 (r) raises no alert.

The change touches three places:

- the bullet of `reliability.md` §7.7 "WAL line after a switch",
- step 4 of the runbook section of the switch,
- the step `wal-line` of `deploy/ci/switch-steps.sh`.

In a gap of the WAL stream, the file stays (ADR-0151 (q2)).

### D13: Listings of the existing script tests

The failed and happy cases of the script tests compare a full listing of `/wal` or `/backups` before and after a run. A run now writes a result file, so each listing changes.

The helpers `wal_listing` and `set_listing` exclude only the exact path of the result file: `./base-backup.result` or `./restore-test.result`. They keep the temp name, so a test still finds a temp file that stays.

Each case that compares a listing also checks the result file on its own: its content, its mode, or its absence.

## Risks / Trade-offs

- [A stalled backup mount holds one blocking thread per path] → ADR-0133 (c) allows one thread per path. The backup scan and two result files add at most three threads.
- [Two writers of one result file] → The lock serializes the runs. `restore-test` uses the lock of its scratch directory. `basebackup-once` uses `/wal/.lock`.
- [A clock skew of the backup containers] → All containers use the host clock (`reliability.md` §7.9). ADR-0093 (a) and D11 handle a time ahead of the comparison moment.
- [The moved code of `disk.rs` changes behavior] → Task 4.2 keeps every test of `crates/recipe-server/src/disk/tests.rs` without a change. D10 names the surface that `disk.rs` keeps.
- [A red test of one task breaks the build of a parallel task] → Each task checks its own crate with `-p`. The orchestrator runs the workspace commands only after a wave without a red Rust test (`tasks.md`).

## Migration Plan

1. The release adds no migration and no key of `serve`.
2. After the upgrade, step 5 of §7.10 writes both result files.
3. `walarchive-entry` sets the mode `0711` of `/wal` at its next start in step 5, command 1.
4. Rollback: the previous `app` does not read the files. The files stay and do no harm. The previous `walarchive-entry` does not change the mode back.

## Open Questions

None. The owner decided item (k) of ADR-0151 on 2026-11-08 (option K1).
