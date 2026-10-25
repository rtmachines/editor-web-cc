# ADR-0073: Read faults of the jobs and the audit chain alert

Status: Accepted. Amended by ADR-0075 (field `volume` of the `low_disk` line). Amended by ADR-0086 (an empty chain table keeps the anchor `A`)
Date: 2026-10-23
Deciders: project owner

## Context

Step P4.1 builds the jobs `disk_check` and `alerts`. The documents do not decide five points that these jobs need. No two documents disagree.

1. The condition "Audit chain check failed" of `reliability.md` section 3.7 has no window. A row of kind `audit_chain_broken` stays in `security_event` until the purge, so "any row" gives an alert that never ends.
2. `DiskFault` and `BackupFault` of the ports `DiskSpace` and `BackupDirectory` have no `code()`. Section 1.1 names only `ValueFaults` (ADR-0025) and `WebhookFault` (ADR-0060 (r)) as reasons.
3. Section 3.7 and ADR-0060 (f) say that an unreadable backup directory is an error, not "no set". They name no condition for it, and they do not say whether the start grace of ADR-0060 (y) applies.
4. Section 7.8 gives no action when `statvfs` fails on a system that has it (`DiskFault::Unreadable`), for example after a lost mount of `pgdata`. Section 7.8 covers only `DiskFault::Unsupported`.
5. `data-model.md` section 3.17 gives no text form of the three state keys. The jobs write them through `MaintenanceStore`, and `recipe-db` reads them through `OperationsReads`. Both sides must use one form.

Two more facts apply:

- The job `alerts` keeps values between its runs: the counter totals of its last run and the times of recent panics. Section 8 and ADR-0060 (aa) do not list this state.
- The message of `E-WEB-006` in section 1.2 says "because disk space is low". After the decision on gap 4, this sentence can be false.

## Decision drivers

- The project gives data safety priority over performance (R31).
- No command writes before the mode is set (A24, ADR-0061).
- No new error code. A job logs a handled fault once at `WARN` and continues (R35, section 1.1).
- An alert stays active while the fault exists, and it ends without a manual step (R37).
- Each code keeps one stable meaning. The message can change (R36, section 1.1).
- `recipe-app` gets no new crate (ADR-0029, ADR-0060 (dd)).

## Considered options for gap 4

### Option A: Read-only mode on

- Pros: no command writes while the free space of `pgdata` is unknown. A lost mount or a full disk gives the same safe state.
- Cons: a fault of the mount, not of the disk, blocks every command until a measurement succeeds.

### Option B: Keep the mode, write one `WARN` line, and raise an alert

- Pros: the users can write while the disk has space.
- Cons: the application can fill the disk unseen. PostgreSQL then stops at 0 % free, which is the failure that the hard threshold prevents.

The user chose **Option A** on 2026-10-23.

## Decision

- (a) The condition "Audit chain check failed" is active while the newest row of kind `audit_chain_broken` is younger than `audit.max_check_age`.
  - The job `alerts` compares the `occurred_at` of that row with the database time of its snapshot (ADR-0060 (e)).
  - Each check starts at the first row of the chain and stops at the first break (`data-model.md` section 4.9). A row of this kind has `count = 1` and no aggregation (`data-model.md` section 3.8).
  - A chain that stays broken therefore gives a new row at each nightly run, and the alert stays active.
  - The margin of `audit.max_check_age` above the nightly interval covers a slow check (section 7.9). A check that does not run raises the condition of `audit.last_check_at`.
- (b) `DiskFault` and `BackupFault` are reasons of the same kind as `WebhookFault`. They own no error code and have no `code()` (ADR-0025).
  - A job that gets one writes one `WARN` line with the `Display` text of the reason and continues. It does not return `Err` for it (section 1.1, ADR-0060 (p), (q)).
  - The text of each reason names no path.
- (c) An unreadable backup directory (`BackupFault`) makes the condition "Latest dump older than `backup.max_age_alert`" active.
  - The start grace of ADR-0060 (y) does not apply to it. The grace covers a missing dump on a new installation, not a read fault.
  - The job `alerts` writes one `WARN` line of the `BackupFault` in each run that gets it, and the `WARN` line of the condition.
- (d) A `DiskFault::Unreadable` on `pgdata` in the job `disk_check` turns the read-only mode on.
  - The rule also holds for the first run in step 6 of the start (ADR-0061). No command therefore writes before the mode is set.
  - The fault is not a failure of step 6 in the sense of ADR-0061 (b). The start continues in read-only mode.
  - The job writes one `WARN` line of the fault at most every `disk.warn_log_interval`.
  - The job needs new run state for this: the time of its last `WARN` line of a read fault. It is one more atomic in the job, next to the time of the last low-disk `WARN` line.
  - A separate time keeps the first line of a read fault visible after a low-disk line, because the two lines report different facts.
  - The application leaves read-only mode only after a measurement that succeeds and gives free space above `disk.resume_percent` (section 7.8).
  - A fault on `backups` or `walarchive` changes no mode. Item (f) covers it.
- (e) `DiskFault::Unsupported` keeps its behavior: one `WARN` line per process and no read-only mode.
  - Reason: only a development system on Windows has no `statvfs`. The image of `app` runs on Linux, where the adapter always uses `statvfs` (ADR-0060 (c), ADR-0064).
  - For `Unsupported`, the condition "Free disk below `disk.warn_percent`" stays inactive, and the job `alerts` writes no line for it.
- (f) A `DiskFault::Unreadable` on one of the three read-only mounts makes the condition "Free disk below `disk.warn_percent`" active for that volume.
  - The job `alerts` writes one `WARN` line of the fault in each run that gets it, and the `WARN` line of the condition.
- (g) The job `alerts` keeps run state between runs:
  - the totals of the two counters of `AlertCounters` (ADR-0060 (z)) at its last run, as two `AtomicU64`,
  - the times of the newest panics, in `alerts.panic_count` + 1 slots of `AtomicI64` and one `AtomicUsize` for the next slot.
  - A run that finds new panics writes the time of the run into one slot for each new panic. With more new panics than slots, it writes each slot once.
  - The panic condition is active when every slot holds a time and the oldest time is inside `alerts.panic_window`. The precision is one `alerts.interval`.
  - The `E-EXP-002` condition is active when the export total is above the total of the last run.
  - The form is atomics, for these reasons. A `parking_lot::Mutex` needs `parking_lot` outside the feature `memory`, against the closed list of ADR-0029.
  - Only the job writes the state, and the supervisor never runs two iterations of one job at once (ADR-0060 (h)). `Ordering::Relaxed` is therefore enough.
  - This item amends ADR-0060 (aa). Section 8 lists the state.
- (h) The code `E-WEB-006` stays. No new code. The message of its row in section 1.2 changes, because the mode now also follows a read fault:
  - The new English text is "The system is in read-only mode because the free disk space is low or unknown. Contact the administrator."
  - The new Polish text is "System działa w trybie tylko do odczytu, bo brakuje miejsca na dysku lub nie można go zmierzyć. Skontaktuj się z administratorem."
  - The bundle key `readonly-banner` of ADR-0071 (d) changes in the same way. This item amends ADR-0071 (d):

| Key | en | pl |
|---|---|---|
| `readonly-banner` | Read-only mode: the free disk space is low or unknown. Changes are blocked. You can view, export, log in, and log out. | Tryb tylko do odczytu: brakuje miejsca na dysku lub nie można go zmierzyć. Zmiany są zablokowane. Możesz przeglądać, eksportować, logować się i wylogować. |

- (i) The three state keys of `app_setting` have one text form each:
  - `audit.last_check_seq` holds the decimal text of the `chain_seq`, with no sign and no leading zero. An empty chain gives `0`.
  - `audit.last_check_at` and `heartbeat.last_at` hold an instant in RFC 3339 form in UTC. The text has the suffix `Z` and exactly six fraction digits, for example `2026-10-23T02:00:00.000000Z`.
  - Six digits equal the precision of `TIMESTAMPTZ`.
  - A reader that finds another form returns a `StorageError`, not "no value". A bad form is a storage fault, not a missing key.

## Consequences

### Positive

- The audit chain alert stays active while the chain stays broken, and it ends 26 h after the last check that found the break.
- A lost mount of `pgdata` stops every write, as a full disk does. PostgreSQL never fills the disk unseen.
- An unreadable backup directory raises an alert also on a new installation.
- No new error code and no new crate.
- The writer and the reader of the state keys share one text form.

### Negative

- A fault of the mount alone blocks every command until a measurement succeeds.
- The panic window has the precision of one `alerts.interval`.
- The message of `E-WEB-006` and the banner name two causes, so the user does not see which one applies.

### Risks and mitigations

- Risk: a short fault of `statvfs` turns the mode on for a short time. Mitigation: the next run after `health.disk_check_interval` turns it off when the free space is above `disk.resume_percent`.
- Risk: the job `alerts` panics between two writes of its run state. Mitigation: the job writes the slots before the totals. The next run then counts the same panics again, which can give an extra alert but never hides one.
- Risk: a reader treats a bad state value as a missing key and starts a needless catch-up. Mitigation: the storage tests of `recipe-db` cover a bad form of each state key.

## Related

- Amends ADR-0060 (aa) (run state of the jobs) and ADR-0071 (d) (text of the banner).
- Amends ADR-0020 (read-only mode on a failed measurement of the free space) and ADR-0061 (b) (a failed measurement in step 6 of the start).
- ADR-0025 (reasons), ADR-0029 (dependencies of `recipe-app`), ADR-0063 (`WebhookFault`)
- `docs/architecture/reliability.md` sections 1.1, 1.2, 2.4, 3.7, 7.8, and 8
- `docs/architecture/data-model.md` sections 3.8, 3.17, and 4.9
- Baseline requirements: operations "Read-only mode start" and "Alert conditions". The delta changes "Alerts on a new installation".
- `overview.md` section 7: R31, R35, R36, R37. Section 9: A24
