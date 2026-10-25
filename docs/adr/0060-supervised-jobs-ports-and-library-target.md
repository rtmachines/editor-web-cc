# ADR-0060: Supervised jobs, their ports, the catch-up, and the library target

Status: Accepted. Amended by ADR-0073 (read faults of the jobs and the audit chain alert). Amended by ADR-0088 (the method `run_once(request_id, kind)`, the one loop of `cleanup` and its due times). Amended by ADR-0089 (d) (item (q): the `Err` of a failed snapshot of `alerts` at `WARN`)
Date: 2026-10-23
Deciders: project owner

## Context

`reliability.md` section 2.4 lists six supervised jobs: `cleanup`, `disk_check`, `audit_chain_check`, `heartbeat`, `clock_check`, and `alerts`. `domain-model.md` section 5 routes each write through a port of `recipe-app`. Step P4.1 builds the jobs, and the plan found these gaps:

- No document says which crate holds the loop of a job and its schedule. No document names the crate of the adapters of the disk, the backup directory, and the webhook.
- `recipe-app` has a closed list of dependencies (ADR-0029, ADR-0035 (g)). It has `tokio` with `rt`, `time`, and `sync`, but not `tokio-util`.
- R28 puts the permission check first in every use case. `api.md` section 3.2 names only `EnsureDerivedType` as an internal operation without a permission.
- `WebhookFault` is a new error type without a code. Section 1.1 says that every error type has `code()` and that a job logs its errors at its loop boundary.
- The row `audit_chain_broken` of `security_event` needs a `request_id` (`data-model.md` section 3.8). Section 3.2 says that a line of a job has no `request_id`.
- After a restart at the time of the nightly check, the next check runs one day later. The alert of `audit.max_check_age` can then fire without a fault.
- On a new installation, no dump and no completed check exist. The alerts of `backup.max_age_alert` and `audit.max_check_age` would fire on the first day.
- `recipe-server` is a binary crate. Code that only `serve` calls gives `dead_code` warnings until the wiring, and an integration test cannot import an item of a binary.
- Section 2.4 names `login.attempt_cleanup_interval` for the attempt purge. The plan ran the purge on the session interval.

## Decision drivers

- Every job runs without a stop of the service (R30, A41).
- Every write goes through a port, and every port has an in-memory adapter for the tests (`reliability.md` section 6).
- The dependency rule of `overview.md` section 5 stays, and `recipe-app` gets no new crate.
- No new error code.
- No false alert on a new installation or after a restart (R37).

## Considered options

### Option A: Ports and one iteration in `recipe-app`, the loop in `recipe-server`, and a library target

- Pros: a unit test calls one iteration with in-memory adapters. `recipe-app` keeps its closed list. The library target lets the tests import the modules of `recipe-server`.
- Cons: `recipe-server` gets more code and more direct dependencies.

### Option B: The supervisor in `recipe-app` with `tokio::sync::watch`

- Pros: the whole job lives in one crate.
- Cons: more code for the same cancel signal. The schedule and the panic restart move into a library without I/O wiring.

### Option C: The jobs in `recipe-server` with SQL in the job code

- Pros: fewer ports.
- Cons: it bypasses `MaintenanceStore` and the commit paths of `domain-model.md` section 5. `recipe-server` gets `sqlx`, against `deny.toml`.

### Option D: As Option A, but a binary crate with `#[expect(dead_code)]` on each new module

- Pros: no library target.
- Cons: each attribute needs a later change. The test of the panic hook cannot import the hook.

## Decision

The project uses **Option A**.

Ports and adapters:

- (a) `recipe-app` gets the ports `MaintenanceStore`, `AuditChainReads`, `OperationsReads`, `ReadinessProbe`, `DiskSpace`, `BackupDirectory`, `AlertWebhook`, and `ReadOnlySwitch`. `SettingRepository` joins `Transaction`.
- (b) `recipe-db` implements `MaintenanceStore`, `AuditChainReads`, `OperationsReads`, `ReadinessProbe` on the health pool, and `SettingRepository`.
- (c) `recipe-server` implements `DiskSpace` with `rustix::fs::statvfs` on Unix. On another system the adapter returns `Unsupported`. `recipe-server` also implements `BackupDirectory` and `AlertWebhook`.
- (d) `recipe-web::ReadOnlyMode` implements `ReadOnlySwitch`. Each port has an in-memory adapter.
- (e) `OperationsReads::snapshot` reads every value of one run of `alerts` in one `READ ONLY` transaction. It returns a stored state key as a typed time that `recipe-db` parses, so `recipe-app` keeps its closed list.
- (f) `BackupDirectory::newest_complete_set` returns a `Result`. An unreadable directory gives `Err`, not "no set".

Jobs and supervisor:

- (g) Each job is one file in `recipe-app` with `run_once(&self) -> Result<(), AppError>`. It has no loop.
- (h) The supervisor in `recipe-server` runs one `tokio::spawn` per iteration. It stops on a `CancellationToken` of `tokio-util`, and each job gets a child token.
- (i) After a panic, the supervisor starts the job again after `job.restart_delay`. It starts the next run one interval after the end of the last run, so a late run gives no burst.
- (j) The handle of the supervisor has `drain(deadline) -> DrainOutcome` for the stop of ADR-0058 (m). After the deadline, `drain` aborts the open job iterations with no wait. The outcome counts the cut iterations (ADR-0087 (d)).
- (k) `recipe-server` also holds the schedules, the watchdog thread, the panic hook, and the wiring.

Schedules:

- (l) `cleanup` deletes the expired sessions every `session.cleanup_interval` and the old attempts every `login.attempt_cleanup_interval`. It calls the security event purge when its last purge is older than `security_event.purge_interval`.
- (m) A failed batch of `cleanup` keeps the batches before it, and the next run continues.
- (n) `disk_check` runs every `health.disk_check_interval` and once at the start (ADR-0061). `Unsupported` gives one `WARN` line per process.
- (o) `clock_check` runs once at the start and then every `clock.check_interval`.

Errors, logs, and the `request_id`:

- (p) A job writes its `WARN` lines itself. It logs a handled fault once at `WARN`, for example a `WebhookFault` (ADR-0063).
- (q) A job returns `Err` only for an error of level `ERROR` of section 3.3. The supervisor logs that `Err` once with the field `job`.
- (r) `WebhookFault` is a reason type without `code()`, as ADR-0025 permits. Section 1.1 lists it like `ValueFaults`.
- (s) Each run of a job gets its own `request_id`. Only the rows of the run hold it, for example the row `audit_chain_broken`. The log lines of a job hold the field `job` and no `request_id` (section 3.2).

Internal operations:

- (t) The six jobs and the startup settings comparison `CompareSettings` are internal operations. They have no permission and no row of the matrix.
- (u) `api.md` section 3.2, `reliability.md` section 6 (row "Permission matrix"), and `overview.md` section 5 list them next to `EnsureDerivedType`.

Catch-up and start grace:

- (v) At the start, `audit.last_check_at` can be missing or older than `audit.catchup_age` (24 h). Then the job `audit_chain_check` runs one more check after `audit.catchup_delay` (10 min).
- (w) The nightly run and the catch-up share one loop and never overlap. The catch-up reads `audit.last_check_at` again and skips after a newer completed check.
- (x) The first start of the installation is the oldest `created_at` in `app_setting`.
- (y) During `backup.max_age_alert` (26 h) after the first start, a missing dump raises no alert. During `audit.max_check_age` (26 h) after the first start, a missing `audit.last_check_at` raises no alert.

Counters and state:

- (z) `AlertCounters` holds two `Arc<AtomicU64>`: the panics and the `E-EXP-002` results. `ExportVersion` increments the second counter. `AppContext::with_counters` takes them, and `AppContext::new` stays.
- (aa) The list of interior mutability in `reliability.md` section 8 gets three items. Two of them are the cancel tokens of the supervisor and the flags of the watchdog handle (`arm`, `disarm`, `stop`). A third item is the run state of a job between runs, as atomics in the job. It holds the start of the last security event purge of `cleanup`, the time of the last low-disk `WARN` line of `disk_check`, and its `Unsupported` flag.

Crate shape and dependencies:

- (bb) `recipe-server` gets `src/lib.rs` with every module and a thin `src/main.rs`. `Cargo.toml` sets `publish = false`. `overview.md` section 5 and the workspace table of CLAUDE.md say "bin + lib". The library target holds the modules of the binary and uses `anyhow` like `main`. It is not a library crate of A37, because it is not published and only the binary and the tests of the crate use it.
- (cc) `recipe-server` gets the direct dependencies `axum`, `hyper`, `hyper-util`, `tokio-util`, `tower`, and `time`. It also gets `rustix` on Unix and the feature `env-filter` of `tracing-subscriber`. ADR-0063 adds the crates of the webhook.
- (dd) `recipe-web` gets the features `catch-panic` and `timeout` of `tower-http`. `recipe-app` gets no new crate. Only `matchers` 0.2.0 is new in `Cargo.lock`.

Settings:

- (ee) Section 7.11 gets the row `audit.catchup_delay` / `audit.catchup_age` with the values 10 min / 24 h.

## Consequences

### Positive

- A unit test runs one iteration of each job with in-memory adapters.
- A restart or a new installation gives no false alert.
- `recipe-app` keeps its closed list, and `sqlx` stays in `recipe-db`.

### Negative

- Eight new ports and their in-memory adapters.
- `recipe-server` is a library and a binary. Its public items need doc comments.
- The catch-up can run a full chain check 10 minutes after a start. The check reads the whole chain.

### Risks and mitigations

- Risk: a job holds a connection for a long time when `db` is down. Mitigation: `alerts` reads in one transaction per run, and the next run starts one interval after the end of the last run.

## Related

- ADR-0013, ADR-0015, ADR-0022, ADR-0025, ADR-0029, ADR-0035 (g) and (h), ADR-0058, ADR-0061, ADR-0063
- `docs/architecture/domain-model.md` section 5 (commit paths and ports)
- `docs/architecture/overview.md` section 5 (dependencies, "bin + lib", the internal operations) and CLAUDE.md (the workspace table)
- `docs/architecture/api.md` section 3.2
- `docs/architecture/reliability.md` section 1.1 (`WebhookFault`, the `WARN` line of a handled fault) and section 2.4 (the interval of the attempt purge)
- `docs/architecture/reliability.md` sections 3.2, 3.3, 3.7, 6 (row "Permission matrix"), 7.9, 7.11, and 8
- `docs/architecture/data-model.md` section 4.9 (the catch-up)
- Baseline requirements: operations "Supervised jobs", "Cleanup job", and "Alert conditions", audit "Nightly audit chain check"
- `overview.md` section 7: R28, R30, R37. Section 9: A41
