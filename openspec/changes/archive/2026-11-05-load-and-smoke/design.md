# Design: load-and-smoke

## Context

See `proposal.md` for the two findings. The normative sources are these:

- `reliability.md` §6, rows "Load", "Stack", "End to end", and "Soak".
- `reliability.md` §7.11 for every number.
- `api.md` §6 (hashed body), §7.3 (pre-session token), and §7.4 (login protection).
- ADR-0048, ADR-0064 (n) to (p), ADR-0070 (m) and (w) to (y), and ADR-0094.

ADR-0128 (status Accepted) records the decisions of this change. This file names the items of ADR-0128 and does not repeat them.

Facts from the repository that shape the design:

- `deploy/ci/memory.sh` reads `memory.peak`, `memory.stat`, and `memory.events` of a container cgroup from the host. Its functions write `unavailable` for a value that they cannot read.
- `deploy/ci/http_probe.py` logs in through Caddy at `https://localhost:8443` and changes the password of `ci-admin`. After `recovery.sh`, the state file `new_password` holds the password of `ci-admin`.
- `deploy/ci/http_probe.py` keeps only the status and the error code in a failure text (`HttpCheckError`).
- `deploy/ci/login.sh` copies the root certificate of Caddy to `$CI_STATE/root.crt`.
- `deploy/ci/check.sh` runs five named Python tests of `deploy/ci/` and the structure checks of `ci.yml`. Only jobs outside pull requests run it.
- `deploy/Dockerfile` copies the binary from the fixed path of the target `x86_64-unknown-linux-musl`.
- `e2e/playwright.config.ts` always starts two dev servers and requires `E2E_PASSWORD`. It reads tests from `e2e/tests/` only. `e2e/tests/smoke.spec.ts` already exists.
- The golden file `crates/recipe-export/tests/golden/recipe_R-100_v3.json` holds a `content_sha256` that `golden.rs` checks against the serializer.
- `oha -Z` picks a random body line for each request, with replacement. `--output-format json` gives the slowest time, the p99 latency, the status counts, and the error list.
- The pre-session token is a signed nonce with no server store. One token of `GET /login` is valid for every body line (`api.md` §7.3).
- The Argon2 permit comes before T1 of `Login` (`api.md` §7.4). The per-IP budget of 100 failures blocks the runner address for 15 minutes after the flood.
- The log line of `E-WEB-009` has the field `stage`, for example "password hash permit" of `TimeoutStage::Argon2Permit` (ADR-0129, task 5.2).

## Goals / Non-Goals

**Goals:**

- Each bound of the rows "Load" and "End to end" gets a check that fails a CI job.
- Each new check gets a unit test that fails before the code exists.
- Each pull request runs the unit tests of the CI scripts.
- The values of ADR-0064 (n) appear in the job summary of each run on `main`.

**Non-Goals:**

- No change of production code except the log line of `E-WEB-009` (ADR-0129, task 5.2). No change of `deploy/Dockerfile` or of `deploy/compose*.yaml`.
- No new Cargo dependency and no new npm package.
- No soak (decision Q-B (3) of the fix plan).
- No change of a row of `reliability.md` §6 or of a value of §7.11.

## Decisions

### D1. A separate job `load` with two legs

ADR-0128 (a) to (d), (i) to (k). The job reuses `prepare.sh`, `start.sh`, `login.sh`, and `http_probe.py password` of the job `compose`. A leg ends with `diagnostics.sh` on failure and `cleanup.sh` always.

The job runs on the hosted runner `ubuntu-24.04`. The first step checks 4 vCPU, 8 GiB of memory, and cgroup v2. A red `load` on `main` blocks the next release, because a pull request does not start `load`.

Alternative: steps in the job `compose`. That job then exceeds 45 minutes, and the flood blocks every later login (ADR-0128, option B).

### D2. Seed through HTTP

ADR-0128 (l) to (o). Three stdlib Python modules share one client:

- `deploy/ci/load_client.py` holds the cookie jar, the pre-session token, the CSRF token of each form, and `row_version` from `HX-Trigger`. It also creates the second user and replaces its one-time password.
- `deploy/ci/load_seed.py` defines the UDT and sends 4 000 `AddParameter` requests with 8 limit rows each. It also compares the three counts of the draft with the plan.
- `deploy/ci/load_timing.py` measures the three requests of D3.

Each script reads each password from a file of the state directory and never from its arguments. A failure text never holds a response body.

The seed shape fills the three size budgets with the same request count. `CodesysIec` needs no S7 address, so the 65 536-byte block end of the S7 platforms does not apply.

Alternative: a direct SQL seed or a subcommand. ADR-0128, option F, gives the reasons against both.

### D3. Timings

ADR-0128 (p) to (r). The bound is the 10 s of the row "Load". A measurement uses a new connection and a monotonic clock, from before the connect to the last byte. The client follows no redirect and sends the headers of the browser.

The caches are warm after the seed. The design therefore does not call a measurement cold.

### D4. Flood and its checks

ADR-0128 (s) to (x). The flood size is 200 requests at a concurrency of 200, from the row "Load". The bound is 5 s. The semaphore values are `argon2.permits` and `argon2.acquire_timeout` of §7.11. The per-IP budget is `login.ip_budget`.

`deploy/ci/load_flood.py` owns every decision of the flood, so a unit test covers each one. It has these commands:

- `runner` checks the vCPU count, the memory, and cgroup v2 of the runner.
- `preflight` checks the CPU limit and `memory.max` of `app`, and the count of failed logins.
- `body` writes the body file of 2 000 lines.
- `check` reads the `oha` output, the status counts, and the count of the log lines.

The hard gate on the `503` stays. The new container `app` and an empty failure count make the semaphore path the expected path, and the `429` count is reported.

`deploy/ci/load.sh` only runs the steps. It downloads `oha`, recreates `app`, runs `oha`, reads the log window and the cgroup, and passes the values to the Python scripts.

### D5. Memory check

ADR-0128 (y) to (aa). `deploy/ci/load_memory.py` owns the memory decisions. The limit is `container.app_memory` of §7.11.

The job reads the cgroup twice: after the timings, and after the flood on the new container. A value `unavailable` fails the job. The job records `anon` next to `soak.memory_ceiling` of §7.11, but only the conditions of ADR-0128 (z) gate the job.

### D6. The amendment of ADR-0064 (o)

ADR-0128 (ab) and (ac). For the flood, the fallback rule of ADR-0064 (o) applies only the memory bounds of the container limit. The p99 bound of §6 applies to the soak only. Each leg writes the memory condition of (o) into the job summary.

### D7. The `cc-debian13` leg

ADR-0128 (f) to (i). `deploy/ci/Dockerfile.cc-debian13` copies `deploy/Dockerfile` with a fixed list of substitutions. The `COPY` line takes the binary from the path of `x86_64-unknown-linux-gnu`.

A check in `check.sh` compares the two files after the substitutions. A test fixture with a changed `RUN` line proves that the check fails. The leg uses its own build cache scope, and the leg `musl` writes no build cache.

Both legs must build, start, and write their values. Only the bound checks of the leg `cc-debian13` have `continue-on-error: true`.

### D8. The smoke suite

ADR-0128 (ad) to (ai). The suite reuses no fixture of `e2e/tests/support/seed.ts`, because that seed targets the dev server. Each run adds a unique suffix to the user login, the machine code, and the reference number.

The setup project `smoke-login` types each password with the trace off. The project `smoke-compose` uses the saved sessions and keeps a trace on failure. The flow file is `e2e/compose/end-to-end.spec.ts`, so it does not share a name with `e2e/tests/smoke.spec.ts`.

The RFC 8785 function and the builder of the hashed body live in `e2e/compose/support/jcs.ts`. The builder sorts each array in the order of `api.md` §6.3. Its unit test lives in `e2e/tests/`, so the regression suite runs it on each pull request.

The export uses the export link of the version page, so `Sec-Fetch-Site` is `same-origin` and the guard of ADR-0055 passes.

### D9. The smoke step in the job `compose`

ADR-0128 (aj) to (al). One step runs `deploy/ci/smoke.sh` after `pgtools.sh` (D11 step 8) and before `stop.sh` (D11 step 9). The step has `timeout-minutes: 10`. The job keeps `timeout-minutes: 45`.

The script holds both passwords only in the environment of its child processes. After the suite, `deploy/ci/secret_scan.py` reads every file of the traces and the HTML report. The script moves the files into the upload directory only after a clean scan.

### D10. The job `ci-scripts`

ADR-0128 (e). `check.sh` runs each `deploy/ci/test_*.py` by glob, so a new test needs no edit of `check.sh`. The pull-request job `ci-scripts` runs `check.sh` without Docker. The owner adds the job to the branch protection of `main` after the merge.

### Answers of the owner

The owner answered the nine open questions of the plan on 2026-11-05. Each answer is now a decision:

| Question | Decision |
|---|---|
| How the job measures the bounds of 5 s and 10 s | D3, D4 |
| What "under its memory limit" means | D5 |
| The p99 bound of ADR-0064 (o) and the flood | D6 |
| The shape of a version at the size budget | D2 |
| The runner | D1 |
| The `cc-debian13` leg | D7 |
| The place of the smoke suite | D9 |
| The one-time password of the second user | D8 |
| The install of `oha` | D4 |

## Risks / Trade-offs

- [The job `compose` exceeds 45 minutes] → A dispatch run without a cache hit records the job time before the owner approves the merge. A step timeout does not give a time reserve. A new ADR then moves the suite to its own job.
- [A hosted runner is slow, and a time near 10 s fails at random] → The job summary records each time. CI keeps 0 retries, so a flaky bound is visible.
- [The flood gives no `503`] → `app` has `container.app_cpus` of 1, and 200 requests compete for 4 permits. The preflight checks remove the `429` path as a cause. The unit test of the checker covers the failure text.
- [The seed time grows faster than the request count] → The seed writes its progress after each 500 requests. Its step has its own timeout. The first dispatch run records the seed time.
- [A password reaches an artifact] → The setup project has no trace, and the scan blocks the upload.

## Migration Plan

No migration. The change adds CI jobs and test files only. A rollback removes the jobs `load` and `ci-scripts` and the smoke step from `ci.yml`.

## Order of the work

`tasks.md` runs the tasks in serial waves. Tasks of one wave touch different files. Two tasks that edit `deploy/ci/check.sh` or `.github/workflows/ci.yml` never run in one wave.
