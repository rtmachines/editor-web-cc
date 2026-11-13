# ADR-0161: Bound of the stalled calls of a path

Status: Accepted. Amends ADR-0133 (the consequence of one blocking thread per path)
Date: 2026-11-13
Deciders: project owner

## Context

ADR-0133 says that a stalled mount holds at most one blocking thread per path. The statement is too strong. The set of stalled paths belongs to one adapter. Two adapters that read the same path keep two sets. Two jobs can also use one adapter. ADR-0133 (d) lets their calls run side by side until they reach the limit.

ADR-0151 (o) and (p) extend the time limit to the scan of the backup directory and to the reads of the result files. Each of those readers has its own set of stalled paths. A path can therefore hold more than one blocked thread.

The count is small and finite. The text of ADR-0133 does not state it.

## Decision drivers

- A claim about blocked threads must be true.
- A new job or a new adapter must show its effect on the count.
- No new sync point for a case that a small bound already limits.

## Considered options

### Option A: State the true bound

- Pros: no new sync point and no new production path in normal operation. The bound follows from the job structure. A test holds each premise.
- Cons: the bound depends on the structure of the jobs. A new pair of a job and an adapter raises it.

### Option B: Share one read of a path between jobs

- Pros: one stalled call per path.
- Cons: new code and a new sync point between jobs, for a case that the bound already limits.

## Decision

The project uses **Option A**.

- (a) A path holds at most one stalled call for each pair of a job and an adapter that reads the path.
- (b) The bound holds for three reasons. Each job waits for each of its reads, so a read that reaches the limit marks its path. One run of a job reads one path at most once at a time through one adapter. No two runs of one job overlap. Each adapter also keeps one set of stalled paths for the life of the process. `serve/jobs.rs` builds each adapter once. A second call of `Host::backups` or `Host::check_results` makes a new adapter and therefore a new pair.
- (c) The totals per volume are in this table.

| Volume | Stalled calls | Pairs |
|---|---|---|
| `pgdata` | 2 | `disk_check` and `alerts` through `SystemDiskSpace` |
| `backups` directory | 2 | `alerts` through `SystemDiskSpace` and through `FileBackupDirectory` |
| `backups/restore-test.result` | 1 | `alerts` through `FileCheckResults` |
| `walarchive` | 1 | `alerts` through `SystemDiskSpace` |
| `walarchive/base-backup.result` | 1 | `alerts` through `FileCheckResults` |

- (d) All paths together hold at most 7 blocked threads. This number is far below the blocking pool of Tokio.
- (e) A new pair raises the bound by one. A new pair is a new job that reads a path, or a new adapter that reads a path. The author of such a change updates the table of item (c).
- (f) The test in `crates/recipe-app/src/jobs/alerts/tests/disks.rs` holds the second premise. A disk double counts the calls in flight on each path. The test asserts a peak of 1 on each path during one run of `alerts`. A change that reads one path twice at the same time in one run fails this test.
- (g) The supervisor tests hold the third premise through `overlaps()` in `supervisor/support.rs`.
- (h) This ADR amends the consequence "at most one blocking thread per path" of ADR-0133. The items (a) to (e) of ADR-0133 stay in force.
- (i) `mark_if_running` counts the marks of a path with `checked_add`. On `None` it gives the fault `LimitFault::MarkOverflow`. The count then stays, and the call keeps no mark. `finish` runs in `Drop` and cannot return an error. It uses `checked_sub`. On `None` it removes the entry, like a count of zero. A marked call always has its mark in the count, so this `None` cannot occur. This is the one exception to the rule "Return `Err` on `None`" of `reliability.md` §2.2. Item (d) also makes an overflow of the count impossible in practice.

## Consequences

### Positive

- The text states the real bound of blocked threads.
- A change that adds a pair sees the effect of the change.
- No new sync point exists. No new production path runs in normal operation.

### Negative

- A reader must update the table of item (c) when a job or an adapter reads a new path.

### Risks and mitigations

- Risk: a later change reads one path twice at the same time in one run. Mitigation: the test of item (f) fails.
- Risk: a later change lets two runs of one job overlap. Mitigation: the supervisor tests of item (g) fail.

## Related

- ADR-0133 (c) to (e), ADR-0151 (o), (p)
- `reliability.md` §7.8, §8
- Change `full-repo-review-fixes`, capability `operations`
- `overview.md` §7: R30, R31. §9: A24
