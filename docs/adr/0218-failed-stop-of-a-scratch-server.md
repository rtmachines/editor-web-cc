# ADR-0218: Failed stop of a scratch server

Status: Accepted. Amends ADR-0067 (g) (the trap of `restore-test`) and ADR-0068 (a new cleanup item of `pitr-check`)
Date: 2026-11-22
Deciders: project owner

## Context

ADR-0067 (g) and `reliability.md` §7.7 say: "A `trap` deletes the scratch cluster on each exit of the script." ADR-0068 states no cleanup rule for `pitr-check`. Neither text says what happens when the scratch server does not stop.

The scripts before this change handle a failed stop in three ways:

- The exit trap of `restore-test` keeps the directory of the current run after a failed stop. It writes an `ERROR` line and sets the exit code 1 (`deploy/pgtools/bin/restore-test`, lines 127 to 136).
- `delete_stale_runs` of `restore-test` ignores the status of the stop (line 186). Then it deletes the stale directory under a live postmaster (line 188).
- The exit trap of `pitr-check` ignores the status of the stop (`deploy/pgtools/bin/pitr-check`, lines 91 and 92). Then it deletes the directory, and the exit code stays 0.

A failed stop can follow a timeout of `scratch.stop_timeout` (60 s, `reliability.md` §7.11, ADR-0077 (g)). In `pitr-check`, the line `pitr check passed` (line 191) comes before the trap. So a run can report success while its scratch postmaster still lives.

A live scratch postmaster keeps memory inside `container.backup_memory` (512 MiB). `restore-test` runs every week in production (ADR-0067 (a)). `pitr-check` runs only with `PITR_CI=1`, so its half of the fault stays in CI.

## Decision drivers

- A weekly test proves that a backup set restores (R31, ADR-0020). A run that reports success must leave no live scratch server.
- A fault must stay visible in the event log and in the result (R35, ADR-0151).
- A background job must not leave a process that holds memory until the next restart of the container (R30).
- Every number comes from `reliability.md` §7.11.

## Considered options

### Option A: Delete the directory only after a successful stop

- Pros: the rule already exists on the current-run path of `restore-test`. A failed stop gives a failed run, and the operator sees it.
- Cons: a kept directory uses space on its disk until the operator acts. Each later run of `restore-test` fails while the stale server lives.

### Option B: Keep the scripts as they are

- Pros: no change of the scripts or the tests.
- Cons: a run can report success while its scratch postmaster lives. The postmaster then runs on a deleted data directory.

### Option C: Send `SIGKILL` to the postmaster after a failed stop, then delete the directory

- Pros: the run can delete the directory in more cases.
- Cons: after a restart of the container, the PID of a stale `postmaster.pid` can belong to another process. A process that waits for disk I/O may not end on `SIGKILL` either. The script needs more code to prove that the process ended.

## Decision

The decision selects Option A.

- (a) The exit trap of `restore-test` and the exit trap of `pitr-check` delete the scratch directory only after a successful stop of the scratch server.
- (b) A failed stop keeps the directory, writes one `ERROR` line, and ends the script with a code other than 0. This is the form of the current-run path of `restore-test`.
- (c) `delete_stale_runs` of `restore-test` uses the same rule. A failed stop of a stale scratch server ends the run before `initdb`. The result file then holds `failed`, because the script arms the result before `delete_stale_runs`.
- (d) The conditions for a stop attempt do not change. `delete_stale_runs` stops a stale server only when `is_scratch_postmaster` agrees. Without such a server, it deletes the stale directory as before.
- (e) The stop keeps the mode `immediate` and the timeout `scratch.stop_timeout`.
- (f) `pitr-check` writes `pitr check passed` before its trap runs. A failed stop after that line still writes one `ERROR` line and ends the script with a code other than 0.
- (g) `deploy/pgtools/tests/lib/wal-stubs.sh` gets a switch for a failed stop, like `STUB_PGCTL_STOP_FAIL` of `restore-stubs.sh`.
- (h) The test of `pitr-check` asserts one cleanup `ERROR` line after the line `pitr check passed`, and an exit code other than 0. The test of `restore-test` also asserts the result `failed`.
- (i) Each suite calls its cases by name. So each new case gets a call in the entry point of its suite. `restore-test-test` calls the case of `restore-test`, and `walarchive-test` calls the case of `pitr-check`.
- (j) `reliability.md` §7.7 replaces the `trap` sentence with the rule of items (a) and (b).
- (k) `docs/ops/runbook.md` gets one step for a scratch server that does not stop. The operator stops the server by hand, then deletes the directory, and then runs the job again.

## Consequences

### Positive

- No run of `restore-test` or `pitr-check` reports success while its scratch server lives.
- The scripts never delete the data directory of a live scratch postmaster.

### Negative

- A kept directory uses space on the backup disk or on the WAL archive disk until the operator acts.
- Each later run of `restore-test` fails while the stale scratch server lives.

### Risks and mitigations

- Risk: the operator does not see a failed stop. Mitigation: the run writes an `ERROR` line, the result file of `restore-test` holds `failed`, and the alerts of ADR-0151 report the result.
- Risk: a later change of a trap drops the status of the stop again. Mitigation: the stub cases of items (g) to (i) fail on such a change.

## Related

- ADR-0020 (recovery point and recovery time), ADR-0067 (restore test, item (g)), ADR-0068 (WAL archive and `pitr-check`)
- ADR-0077 (g) (`scratch.stop_timeout`), ADR-0151 (result files and alerts of the weekly backup checks)
- `reliability.md` §7.7, §7.11
- `operations` "Failed stop of a scratch server"
- `overview.md` §7: R30, R31, R35
