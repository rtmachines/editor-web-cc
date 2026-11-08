# ADR-0133: Time limit of the disk measurement

Status: Accepted. Amends ADR-0073 (d)
Date: 2026-11-05
Deciders: project owner

## Context

The job `disk_check` measures the free space of `pgdata` with `statvfs`. The job `alerts` measures `pgdata`, `backups`, and `walarchive` in the same way. Both jobs share one disk adapter, so they can measure `pgdata` at the same time. The adapter of `recipe-server` runs the call in `spawn_blocking` and waits without a limit (`reliability.md` §2.4, §7.8).

A stalled mount, for example a lost network file system, can hold `statvfs` for a long time. The review of 2026-11-04 found the effect (finding D2-2).

- At start, the first run of `disk_check` waits before the listeners open.
- During operation, each later run waits. The read-only mode does not turn on while the disk fills.

ADR-0073 (d) turns the read-only mode on after a `DiskFault::Unreadable` on `pgdata`.

## Decision drivers

- Every numeric value comes from `reliability.md` §7.11.
- A volume that does not answer is not a safe volume for writes.
- A stalled call must not start a new blocked thread at each run.

## Considered options

### Option A: A new setting for the limit

- Pros: an operator can tune the limit.
- Cons: a new key, a new row of §7.11, and new validation.

### Option B: The limit is `health.disk_check_interval`

- Pros: no new setting. A measurement that does not end before the next run is not useful.
- Cons: the limit of 30 s is long for a local disk.

## Decision

The project uses **Option B**.

- (a) The disk adapter waits at most `health.disk_check_interval` for one measurement.
- (b) A measurement that reaches the limit gives `DiskFault::Unreadable`. On `pgdata`, ADR-0073 (d) then turns the read-only mode on.
- (c) A call that reaches the limit marks its path as stalled. While the stalled call still runs, a new measurement of that path gives `DiskFault::Unreadable` at once. It starts no second blocking call.
- (d) Calls that have not reached the limit run side by side. Two jobs that measure the same path at the same time both get a reading.
- (e) The blocking call clears the mark of its path when it ends. A call that ends at the limit leaves no mark. The next measurement after that runs a new call.
- (f) The set of stalled paths is a `parking_lot::Mutex` in the adapter. It is a new place of interior mutability in `reliability.md` §8.
- (g) This ADR amends ADR-0073 (d).

## Consequences

### Positive

- A stalled mount turns the read-only mode on within one interval.
- A stalled mount holds at most one blocking thread per path.

### Negative

- A slow but healthy disk that needs more than 30 s turns the read-only mode on. Only a measurement that succeeds ends the mode (ADR-0073 (d)).

### Risks and mitigations

- Risk: the first run at start waits up to 30 s. Mitigation: the start order of ADR-0061 already allows this wait.

## Related

- ADR-0073 (d), (e), ADR-0061, ADR-0089 (b)
- `reliability.md` §2.4, §3.7, §7.8, §8, §7.11 row `health.disk_check_interval`
- Change `low-round-c`, capability `operations`
- `overview.md` §7: R31. §9: A24
