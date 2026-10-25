# ADR-0088: Run interface of the jobs and the cut count of the jobs

Status: Accepted. Amended by ADR-0089 (b) (items (a) and (d): the first run of `disk_check` inside the span `job`). Amended by ADR-0089 (c) (item (e): the stop line only for a cut)
Date: 2026-10-24
Deciders: project owner

## Context

ADR-0060 gives the supervised jobs and their supervisor. Task 7.5 writes the supervisor and the schedules. The review of that code found these gaps:

1. `reliability.md` section 3.2 gives the span `job` the field `name`. The JSON formatter of `tracing-subscriber` writes its own key `name` for the name of each span. A span field `name` therefore collides with that key.
2. No span encloses a run of a job. The `WARN` lines of a job and the line of the panic hook get no field `job`. The fields `iteration` and `outcome` of section 3.2 never occur.
3. A run does not know which trigger started it. `Schedule::next_delay` returns only the wait. The catch-up run of `audit_chain_check` must skip after a newer completed check (ADR-0060 (w)). The nightly run must never skip.
4. `cleanup` has two cadences: `session.cleanup_interval` and `login.attempt_cleanup_interval`. Two loops of one job can run at the same time. ADR-0073 (g) assumes that a job has one iteration at a time.
5. ADR-0060 (g) gives `run_once(&self)` without an argument. ADR-0060 (s) gives each run its own `request_id`. A job that makes a second id gives rows and log lines that do not agree.
6. ADR-0087 (d) gives the `WARN` line of the stop two numbers: the cuts of the two listeners. ADR-0060 (j) counts the cut job iterations. No document says where that count goes.

## Decision drivers

- Each error and each run is visible in the log with the name of its job (R35).
- No two iterations of one job run at the same time (ADR-0060 (w), ADR-0073 (g)).
- The rows of a run hold the `request_id` of that run, and no job makes a second id (ADR-0060 (s)).
- No new error code, no new number in section 7.11, and no new crate in `recipe-app`.

## Considered options for the field `job`

### Option A: A span `job` that the supervisor opens around each run

- Pros: every event inside the run gets the field. This includes the line of the panic hook, which the job cannot write itself.
- Cons: the span field cannot have the name `name` of section 3.2.

### Option B: Each job writes the field `job` itself

- Pros: no span.
- Cons: each `WARN` line repeats the field. The line of the panic hook gets no field `job`.

The user chose **Option A** on 2026-10-24.

## Considered options for the trigger of a run

### Option A: The schedule returns the fired trigger, and the run gets a run kind

- Pros: the job knows the reason of its run. The nightly run and the catch-up keep one loop.
- Cons: the interface of every job gets one more argument.

### Option B: The job finds the trigger from the time of day

- Pros: no new argument.
- Cons: a late nightly run and a catch-up near 03:00 UTC look the same. The job then guesses.

The user chose **Option A** on 2026-10-24.

## Considered options for the two cadences of `cleanup`

### Option A: One loop with the shorter interval, and due times in the job

- Pros: one iteration of the job at a time. The job keeps its due times like the time of its last security event purge.
- Cons: a longer cadence runs at the first loop run after its interval. Its precision is one loop interval.

### Option B: Two loops

- Pros: each cadence keeps its exact interval.
- Cons: two iterations of one job can run at the same time, against ADR-0073 (g).

The user chose **Option A** on 2026-10-24.

## Considered options for the cut count of the jobs

### Option A: A third number in the `WARN` line of the stop

- Pros: one line holds every cut of the stop.
- Cons: the line of ADR-0087 (d) changes.

### Option B: A separate `WARN` line of the supervisor

- Pros: the line of the listeners stays.
- Cons: two lines describe one stop. The supervisor writes a log line that its caller owns (design D4).

The user chose **Option A** on 2026-10-24.

## Decision

- (a) The supervisor opens the span `job` around each run. The span has the fields `job`, `iteration`, and `outcome`.
  - `job` holds the name of the job, for example `cleanup`.
  - The field has the name `job` and not `name`, because the JSON formatter writes its own key `name` for the span name.
  - `iteration` holds the number of the run of this job since the start of the process. The first run has the number 1. A restart after a panic gets the next number.
  - `outcome` holds `ok`, the code of the `Err` of the run, or `panic`. The supervisor writes the value after the join of the run.
  - The supervisor attaches the span to the future of the run task. Every event of the run gets the field `job` from the span. This includes the `WARN` lines of the job and the line of the panic hook.
  - The supervisor writes its own `ERROR` line of an `Err` or of a panic inside the same span. That line does not repeat the field `job`.
  - The rule of section 3.2 stays: a line of a job has no `request_id`.
- (b) `Schedule::next_delay` returns the fired trigger together with the wait. The supervisor gives the run a `RunKind`.
  - `RunKind::CatchUp` is the kind of a run that a one-shot trigger fires. `RunKind::Scheduled` is the kind of a run that an interval or a time of day fires.
  - A restart after a panic keeps the kind of the run that panicked.
  - The catch-up run of `audit_chain_check` reads `audit.last_check_at` again. It skips when the value is newer than the value that the start read for ADR-0060 (v).
  - The nightly run of `audit_chain_check` never skips.
  - A job that has one trigger ignores the kind.
- (c) `cleanup` has one loop with the shorter of `session.cleanup_interval` and `login.attempt_cleanup_interval`.
  - The job keeps a due time for each of the two cadences, in the same way as for `security_event.purge_interval`.
  - A cadence runs when no earlier run of it exists in the process, or when its interval passed since its last run.
  - The due times belong to the run state of the job, as atomics in the job (`reliability.md` section 8). This item amends ADR-0060 (l) and (aa).
  - The job therefore has one iteration at a time (ADR-0073 (g)).
- (d) `RunKind` lives in the module `jobs` of `recipe-app`. The jobs and the supervisor share it, and `recipe-server` depends on `recipe-app`.
  - The method of every job is `run_once(&self, request_id: RequestId, kind: RunKind) -> Result<(), AppError>`. This item amends ADR-0060 (g).
  - The supervisor passes the `RequestId` that it makes for the run with `new_request_id`. The rows of the run therefore hold the id that the supervisor made.
  - The first `disk_check` run in step 6 of the start runs outside the supervisor. Its caller makes a `RequestId` and passes `RunKind::Scheduled`.
- (e) The `WARN` line of the stop gives three numbers. This item amends ADR-0087 (d).
  - `cut_public` holds the cut connections of the public listener.
  - `cut_internal` holds the cut connections of the internal listener.
  - `cut_jobs` holds the cut job iterations of `DrainOutcome` of the supervisor (ADR-0060 (j)).
  - The caller of the stop writes the line (design D4).
- (f) A back-off for a daily job that panics in each run is not decided here. The restart after `job.restart_delay` of ADR-0060 (i) stays.

## Consequences

### Positive

- Every line of a run names its job, also the line of a panic.
- The catch-up skips after a newer check, and the nightly run never skips. No run guesses its trigger.
- `cleanup` never runs two iterations at the same time.
- One line of the stop holds every cut.

### Negative

- The interface of every job has two arguments, and a job with one trigger ignores one of them.
- The longer cadence of `cleanup` runs up to one loop interval late.
- The field `name` of the span `job` in section 3.2 changes to `job`.

### Risks and mitigations

- Risk: the wall clock steps back, and a due time of `cleanup` moves later by that step. Mitigation: the security event purge has the same behavior, and `clock_check` writes a `WARN` line for a skew between `app` and `db`.
- Risk: a daily job that panics in each run starts again every `job.restart_delay`. Mitigation: the alert of `alerts.panic_count` in `alerts.panic_window` fires (section 3.7).

## Related

- Amends ADR-0060 (g) (the method of a job), (l) (one loop of `cleanup`), and (aa) (the due times of `cleanup`).
- Amends ADR-0087 (d) (the `WARN` line of the stop gives three numbers). ADR-0087 (d) amends ADR-0058 (o).
- ADR-0060 (i), (j), (s), (v), (w). ADR-0073 (g).
- `docs/architecture/reliability.md` sections 1.1, 2.4, 3.2, 7.3, and 8.
- `docs/architecture/data-model.md` section 4.9 (the catch-up).
- `crates/recipe-server/src/supervisor.rs` (`JobRun`, `DrainOutcome`) and `crates/recipe-server/src/schedule.rs` (`Schedule::next_delay`).
- `tracing-subscriber` 0.3.23, the JSON formatter.
- Baseline requirements: operations "Supervised jobs", "Cleanup job", and "Graceful shutdown". Audit: "Nightly audit chain check". The audit delta: "Catch-up audit chain check".
- `overview.md` section 7: R20, R30, R35. Section 9: A41.
