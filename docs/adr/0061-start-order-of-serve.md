# ADR-0061: Start order of serve

Status: Accepted. Amended by ADR-0073 (a failed measurement of the free space in step 6 of the start). Amended by ADR-0080 (a `RUST_LOG` without a directive, the stderr text after the `ERROR` line). Amended by ADR-0089 (b) (step 6 makes the supervisor and runs the first run of `disk_check` through it). Amended by a note (item (j)).
Date: 2026-10-23
Deciders: project owner

## Context

`reliability.md` section 7.12 compares the stored settings at the start, before the listeners open. Section 2.4 rule 5 starts the watchdog thread before the Tokio runtime, and the watchdog probes the internal listener. Step P4.1 wires the start of `serve`, and the plan found these gaps:

- A probe before the bind of the internal listener fails. Three failed probes in `watchdog.window` stop the process with code 70 during a slow start.
- The read-only flag is off at the start. Without a disk check before the public listener opens, commands can write for up to `health.disk_check_interval` on a full disk.
- A24 permits three audit actions in read-only mode. The startup comparison writes `SettingChanged`, which A24 does not name.
- Section 3.3 sets the level through `RUST_LOG`. No document says what a bad directive does.
- In a container the process is PID 1. Without a handler, a `SIGTERM` during the start has no effect.
- `container.app_cpus` is 1. Tokio then starts one worker. A future that holds the worker for more than `watchdog.probe_timeout` fails a probe of the watchdog.

## Decision drivers

- No command writes before the disk check sets the mode (A24).
- Every setting change has an audit entry (A34, ADR-0022).
- A failed start writes one `ERROR` line and exits with code 1 (section 2.3).
- The watchdog counts only the probes of a running server (A41).
- Every number comes from section 7.11.

## Considered options

### Option A: The comparison before the listeners, and the watchdog armed after the internal listener opens

- Pros: section 7.12 keeps its order. The watchdog needs no new number.
- Cons: the watchdog has two states, armed and not armed.

### Option B: The internal listener first with `not ready`, and the comparison after it

- Pros: the watchdog finds the listener at once.
- Cons: section 7.12 and the baseline requirement "Failed startup settings comparison" change.

### Option C: A grace time of the watchdog as a new key of section 7.11

- Pros: no state in the watchdog.
- Cons: a new number, and a slow start can still pass the grace time.

## Decision

The project uses **Option A**. `serve` runs these steps in this order:

1. It installs a fallback logger, then the logger of `RUST_LOG`. It installs the panic hook with a clone of the panic counter (ADR-0060 (z)).
2. It refuses a set `PG*` variable (ADR-0069). It reads the configuration (ADR-0062), both key files, and the webhook file (ADR-0063).
3. It starts the watchdog thread, not armed. Then it builds the runtime with `runtime.worker_threads` workers.
4. It registers the signal stream: `SIGTERM` on Unix, and Ctrl-C on every system.
5. It opens the main pool (`db.pool_size`) and the health pool (`db.health_pool_size`). It runs the role check of `reliability.md` section 5.
6. It builds the web state with its read-only flag. It runs the first iteration of `disk_check`, which sets the flag.
7. It runs `CompareSettings` in one transaction (section 7.12).
8. It opens the internal listener on `0.0.0.0` and `http.internal_port`. Then it arms the watchdog.
9. It starts the supervised jobs (ADR-0060). `clock_check` runs once at once.
10. It opens the public listener on `0.0.0.0` and `http.public_port`. Readiness turns on after the first successful check (ADR-0059).

The letters give the rules of the steps:

- (a) The key file check of step 2 uses `KeyRing::parse` and the carriage return rule of ADR-0031 and ADR-0035 (d).
- (b) A failure in steps 1 to 10 writes one `ERROR` line and returns exit code 1.
- (c) `RUST_LOG` goes through `EnvFilter` with field values as literal text. A missing variable gives the default `info,sqlx=warn,hyper=warn` of section 3.3.
- (d) A bad directive of `RUST_LOG` stops the start. The fallback logger of step 1 writes its one `ERROR` line.
- (e) `CompareSettings` writes `SettingChanged` also in read-only mode. A24 and section 7.8 add the startup comparison to the audit actions of read-only mode.
- (f) A signal before step 10 ends the start. The server closes what it opened and exits with code 0.
- (g) The watchdog has the handle methods `arm`, `disarm`, and `stop`. It counts no failed probe before step 8.
- (h) The runtime has two workers, so a long render does not stop `/healthz` on one CPU. Section 7.11 gets the row `runtime.worker_threads` with the value 2.
- (i) ADR-0058 gives the stop after the signal.
- (j) Note on (b): The `ERROR` line of (b) is the line that `main` writes for the failed start. A panic during the start also writes its own `ERROR` line through the panic hook (ADR-0089 (b), `reliability.md` section 2.4 rule 2). The hook line is not counted. A failed start with a panic therefore writes two `ERROR` lines: the hook line and the line of `main`.

## Consequences

### Positive

- No command writes before the first disk check.
- A start on a full disk still records each setting change.
- A bad configuration, a bad key file, or a bad `RUST_LOG` stops the start with one clear line.

### Negative

- The audit log can get `SettingChanged` entries in read-only mode. They use some of the space margin of `disk.readonly_percent`.
- The watchdog has a second state.

### Risks and mitigations

- Risk: a later change moves a step and breaks the order. Mitigation: a unit test runs the start with stubs of the ports. It covers a low disk at the start, a failed comparison, and a signal during the start.

## Related

- ADR-0020 (read-only mode). This ADR amends it.
- ADR-0022, ADR-0031, ADR-0035 (d), ADR-0058, ADR-0059, ADR-0060, ADR-0062, ADR-0063, ADR-0069
- `docs/architecture/reliability.md` section 3.1 (`RUST_LOG`), sections 7.8, 7.11, and 7.12
- `docs/architecture/overview.md` section 9: A24, and the copy of A24 in `domain-model.md` section 8
- Baseline requirements: operations "Startup settings comparison" and "Failed startup settings comparison"
- `overview.md` section 7: R30. Section 9: A24, A34, A41
