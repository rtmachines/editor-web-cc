# ADR-0089: Errors of the jobs, the first run of `disk_check`, and the stop line

Status: Accepted. Amended by ADR-0090 (answers of two open items: a `NULL` `sent_lsn` and `CloseOutcome::TimedOut`)
Date: 2026-10-24
Deciders: project owner

## Context

ADR-0088 gives the run interface of the jobs and the span `job`. The review of the supervisor found these gaps:

1. `AppError` has no variant that holds an `AuditError`. ADR-0088 (d) gives every run the return type `Result<(), AppError>`. The job `audit_chain_check` therefore cannot return `E-AUD-002`.
   - `data-model.md` section 4.9 and design D6 say that the job returns `AuditError::ChainBroken`, and that the supervisor logs it.
2. ADR-0088 (d) runs the first run of `disk_check` in step 6 of the start outside the supervisor. ADR-0088 (a) lets only the supervisor open the span `job`.
   - The `WARN` lines of that run therefore get no field `job`. This includes the one `Unsupported` line of ADR-0060 (n) and the read fault line of ADR-0073 (d).
   - The field `iteration` does not count that run, although it counts the runs since the start of the process.
3. ADR-0058 (o) and ADR-0088 (e) write the `WARN` line of the cut counts at every stop. A stop without a cut then writes a `WARN` line with three zeros. Section 3.3 gives `WARN` to a degraded state only.
4. No document says what the job `alerts` does when `OperationsReads::snapshot` fails.

## Decision drivers

- Each error and each run is visible in the log with the name of its job (R35).
- Each error gets one log line (`reliability.md` section 1.1).
- A `WARN` line reports a degraded state (`reliability.md` section 3.3).
- No new error code, no new number in section 7.11, no new counter, and no new alert condition.

## Considered options for the chain break

### Option A: A new variant of `AppError` that carries the `AuditError`

- Pros: the job returns `E-AUD-002`. The supervisor logs it once, as every other error of a job.
- Cons: `recipe-web` must map one more variant, although no request returns it.

### Option B: The job logs the break itself and returns `Ok`

- Pros: `AppError` stays.
- Cons: the job logs an error of level `ERROR` itself, against ADR-0060 (q). The span field `outcome` shows `ok` for a break.

The user chose **Option A** on 2026-10-24.

## Considered options for the first run of `disk_check`

### Option A: A method of the supervisor that runs the first run in the span `job`

- Pros: only the supervisor opens the span. The numbers of `iteration` stay continuous.
- Cons: the supervisor exists from step 6 of the start, before the loops start in step 9.

### Option B: The caller of step 6 opens a span `job` itself

- Pros: the supervisor stays.
- Cons: two places open the span. The loop does not know the first run, so it starts again at the number 1.

The user chose **Option A** on 2026-10-24.

## Considered options for the stop line

### Option A: The line only when a count is above 0

- Pros: a clean stop writes no `WARN` line.
- Cons: a reader cannot see from a missing line that the stop finished. The `INFO` line of the shutdown covers this (section 3.3).

### Option B: The line at every stop

- Pros: each stop writes the same lines.
- Cons: each clean stop writes a `WARN` line for a state that is not degraded.

The user chose **Option A** on 2026-10-24.

## Considered options for a failed snapshot of `alerts`

### Option A: The run returns the error, and the supervisor writes a `WARN` line

- Pros: the job evaluates no condition from a part of the snapshot. A database outage gives one `WARN` line per run, not one `ERROR` line per minute.
- Cons: the run raises no alert and sends no webhook call while the read fails.

### Option B: The run evaluates the conditions that need no snapshot

- Pros: the conditions `panics`, `not_ready`, `low_disk`, `backup_age`, and `export_hash_mismatch` stay active during a database outage.
- Cons: the run evaluates a part of the conditions. The job needs a second path for each condition.

The user chose **Option A** on 2026-10-24.

## Decision

- (a) `AppError` gets the variant `Audit(#[from] AuditError)`. It follows the variants `Plant`, `Recipe`, `Export`, `Identity`, and `Storage`.
  - The variant has `#[error(transparent)]`. `Display` gives the message of `E-AUD-002` in section 1.2.
  - `AppError::code()` returns the code of the carried error. For `AuditError::ChainBroken { chain_seq }` it is `E-AUD-002`.
  - The variant carries only `chain_seq`, so it never carries a secret (section 1.1).
  - After the security event and the state keys of `data-model.md` section 4.9, the job returns `AppError::Audit(AuditError::ChainBroken { chain_seq })`.
  - The supervisor writes the one `ERROR` line of the run inside the span `job`, as for every other `Err` (ADR-0088 (a)). The field `error_code` and the span field `outcome` hold `E-AUD-002`.
  - `JobRun::run` and `run_once` keep the return type `Result<(), AppError>` (ADR-0088 (d)).
  - No request returns the variant. The `WebError` mapping gives it the status of its row in section 1.2, which is `500`.
- (b) The supervisor gets the method `run_at_start`. It runs the first run of `disk_check` inside the span `job` with `iteration` 1. This item amends ADR-0088 (a) and (d).
  - Input: the `JobName` and the job. The later `spawn` of that job gets the same job instance. The run state of the job then continues in the loop, for example the `Unsupported` flag of ADR-0060 (n).
  - The method makes the `RequestId` of the run with `new_request_id` and passes `RunKind::Scheduled`.
  - The method runs the run in its own task and waits for the join. The join catches a panic (`reliability.md` section 2.4 rule 6). The method records the span field `outcome` after the join.
  - Output: the end of the run. The end is `Ok`, the `Err` of the run, or a panic.
  - Error rule: the method writes no line of its own for an `Err` or a panic. The panic hook writes its line inside the span.
  - Step 6 of the start (ADR-0061, design D4) makes the supervisor and calls the method. An `Err` or a panic is a failure of step 6. The start then writes one `ERROR` line and exits with code 1 (ADR-0061 (b)).
  - A `DiskFault` is no `Err` (ADR-0073 (b)). `Unreadable` on `pgdata` turns the read-only mode on, and the start continues (ADR-0073 (d)). `Unsupported` writes its one `WARN` line per process (ADR-0060 (n)).
  - Step 9 starts the loops on the same supervisor. The loop of `disk_check` continues the numbers with 2.
  - The first run of that loop comes one `health.disk_check_interval` after its start. The loop therefore does not repeat the first run.
  - The method serves only before the `spawn` of the same job. Only `disk_check` uses it.
  - A signal during step 6 keeps the rule of ADR-0061 (f).
- (c) The caller of the stop writes the `WARN` line with `cut_public`, `cut_internal`, and `cut_jobs` only when one of the numbers is above 0. This item amends ADR-0058 (o) and ADR-0088 (e).
  - When all three numbers are 0, the stop writes no such line.
  - The exit code stays 0 in both cases.
- (d) When `OperationsReads::snapshot` fails, the run of `alerts` returns the error. This item amends ADR-0060 (q).
  - The run reads the snapshot before it evaluates a condition (ADR-0060 (e)). A failed read therefore ends the run with no alert and with no webhook call.
  - The job does not evaluate a condition from a part of the snapshot.
  - The job `alerts` returns `Err` only for this case. The supervisor therefore writes the one line of every `Err` of `alerts` at `WARN`, as a handled fault.
  - The line is inside the span `job`. The field `error_code` and the span field `outcome` hold the code of the error, for example `E-DB-005`.
  - Every other `Err` of a job keeps its `ERROR` line (ADR-0060 (q)).
  - The next run follows one `alerts.interval` after the end of the failed run (ADR-0060 (i)).
  - No new counter and no new alert condition exist for this case.

## Open items

These points are not decided here:

- What a `NULL` `sent_lsn` in `replication_state()` means for `connected` and `lag_bytes`. The doc of `ReplicationState::lag_bytes` says `None` only when the receiver is not connected. `data-model.md` section 5.5 gives `NULL` only when no row exists. A row can exist before `sent_lsn` has a value.
- How `serve` reacts to `CloseOutcome::TimedOut` of `Database::close` (task 7.9).
- A back-off for a daily job that panics in each run. ADR-0088 (f) stays.

## Consequences

### Positive

- A chain break reaches the log with its code, the field `job`, and the span field `outcome`.
- Every line of the first run of `disk_check` names its job and has `iteration` 1.
- A clean stop writes no `WARN` line.
- A database outage gives one `WARN` line of `alerts` per `alerts.interval`.

### Negative

- `recipe-web` maps a variant that no request returns.
- The supervisor exists from step 6 of the start.
- While the snapshot read fails, the job raises no alert and sends no webhook call. This also holds for `panics` and `not_ready`.
- The level of the line of an `Err` depends on the job.

### Risks and mitigations

- Risk: a long database outage stops every alert of `app`. Mitigation: `/readyz` fails, and Docker marks the container unhealthy after `health.check_failures` failures (`reliability.md` section 7.2). The other jobs write their `ERROR` lines.
- Risk: a later change gives `alerts` another `Err`, which then gets `WARN`. Mitigation: a unit test of `alerts` covers each path that returns `Err`.

## Related

- Amends ADR-0088 (a) (only the supervisor opens the span, also for the first run) and (d) (the first run of `disk_check` through the supervisor).
- Amends ADR-0088 (e) (the stop line only for a cut).
- Amends ADR-0058 (o) (the stop line only for a cut).
- Amends ADR-0060 (q) (the `Err` of a failed snapshot of `alerts` at `WARN`).
- Amends ADR-0061 (step 6 makes the supervisor and runs the first run of `disk_check` through it).
- ADR-0060 (e), (i), (n). ADR-0073 (b), (d). ADR-0087 (d).
- `docs/architecture/reliability.md` sections 1.1, 3.2, 3.7, and 7.3.
- `docs/architecture/data-model.md` section 4.9. `docs/architecture/overview.md` section 5.
- `crates/recipe-app/src/error/app.rs` (`AppError`), `crates/recipe-app/src/error/storage.rs` (`AuditError`), `crates/recipe-server/src/supervisor.rs` (`Supervisor`, `JobRun`).
- Baseline requirements: operations "Supervised jobs", "Graceful shutdown", and "Alert conditions". Audit: "Nightly audit chain check".
- `overview.md` section 7: R30, R35, R37. Section 9: A41.
