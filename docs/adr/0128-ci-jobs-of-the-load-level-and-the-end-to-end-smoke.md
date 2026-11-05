# ADR-0128: CI jobs of the Load level and the End-to-end smoke

Status: Accepted. Amended by ADR-0129 (item (w) counts the field `stage` of the log line)
Date: 2026-11-05
Deciders: project owner

## Context

`reliability.md` §6 names the test levels "Load" and "End to end". The review of 2026-11-04 found no test for either level (D6-2, D6-4).

- The row "Load" asks for an `oha` login flood of 200 concurrent `POST /login` with random logins. It also asks for the bound of 10 s on three requests at the size budget.
- The row "End to end" asks for a Playwright smoke suite on the Compose stack (ADR-0048 (c)).
- ADR-0064 (n) asks for one `oha` login flood on the musl image and on a `cc-debian13` build. It records the logins per second, the p99 latency, and `memory.peak`.
- Six ADRs name the load test as a risk mitigation, for example ADR-0039 and ADR-0122.

These facts shape the decision:

- The job `compose` runs on `ubuntu-24.04` on `workflow_dispatch` and on a push to `main` (ADR-0094). It has `timeout-minutes: 45`.
- A hosted runner of a public repository has 4 vCPU and 16 GB of memory. ADR-0070 (m) asks for 4 vCPU and 8 GiB.
- `deploy/ci/memory.sh` already reads `memory.peak` and `oom_kill` of a container from the cgroup v2 files of the host. Its functions write `unavailable` for a value that they cannot read.
- `deploy/ci/http_probe.py` already logs in through Caddy with the pre-session token. That token is a signed nonce with no server store (`api.md` §7.3).
- The `Login` use case takes an Argon2 permit before T1. Thus a throttled login also waits for a permit (`api.md` §7.4).
- After 100 failed logins from one `client_ip` in 15 minutes, each login from that address gets `429 E-IDN-005`.
- The flood queues on the semaphore by design, so its p99 latency is near `argon2.acquire_timeout`. ADR-0064 (o) also applies the p99 bound of the soak to the flood.
- The step `stop.sh` of the job `compose` stops `app`. A browser test must run before that step.
- `e2e/playwright.config.ts` always starts two dev servers and seeds them. It cannot run against the Compose stack.
- `CreateUser` shows a one-time password on the page. The new user must change it at the first login.
- A Playwright trace stores the value of each `fill()` and the body of each request. GitHub masks values in the log, but not in an artifact.

## Decision drivers

- Each bound of the two rows gets a check that fails the job (R30, R31).
- The shipped image and `deploy/Dockerfile` stay unchanged.
- No password is in the repository or in a CI artifact (ADR-0048 (e)).
- A pull request does not wait for a long stack job (ADR-0094).
- The numbers of the checks equal `reliability.md` §7.11.

## Considered options

### Place of the Load level

#### Option A: A new job `load` with its own stack

- Pros: the flood and the seed do not change the state of the stack checks. The per-IP budget of the flood blocks no later step.
- Cons: the job builds the images and starts a stack once more.

#### Option B: More steps in the job `compose`

- Pros: one stack and one image build.
- Cons: the job reaches its limit of 45 minutes. After the flood, every login from the runner gets `429` for 15 minutes.

### Place of the End-to-end smoke

#### Option C: More steps in the job `compose`

- Pros: the stack exists and passed its checks. The fix plan names this place.
- Cons: Node and Chromium add time to a job of 45 minutes.

#### Option D: A new job with its own stack

- Pros: the job `compose` keeps its run time.
- Cons: a third image build and a third stack start.

### Seed of the version at the size budget

#### Option E: A stdlib Python script through the HTTP routes

- Pros: the seed uses the real path of each command. It needs no new subcommand and no new dependency.
- Cons: the seed sends several thousand requests.

#### Option F: A direct SQL seed or a new subcommand of `recipe-server`

- Pros: the seed is fast.
- Cons: SQL skips the use cases and the audit chain. A subcommand adds production code for a test.

### The flood and the fallback rule of ADR-0064 (o)

#### Option G: Amend ADR-0064 (o) now

- Pros: no two normative documents disagree. The job can report the condition of (o) for each leg.
- Cons: ADR-0064 gets an "Amended by" note.

#### Option H: Record the values, and let a later ADR decide

- Pros: ADR-0064 stays unchanged.
- Cons: read literally, every flood fails (o), because its p99 latency is near 2 s. The conflict stays open.

### Password steps of the smoke suite

#### Option I: A setup project without a trace, and a scan of the artifacts

- Pros: the steps that type a password leave no trace. The scan proves that no artifact holds a password.
- Cons: the config has two projects and a scan script.

#### Option J: No trace for the whole suite

- Pros: no artifact holds a request body.
- Cons: a failed test leaves no trace. That conflicts with ADR-0048 (e).

## Decision

The project uses **Option A**, **Option C**, **Option E**, **Option G**, and **Option I**. This ADR amends ADR-0064 (o). It extends ADR-0048 (a) with `e2e/compose/` and `e2e/playwright.compose.config.ts`. The rows of `reliability.md` §6 stay unchanged. The job measures through Caddy, so its environment includes the environment of the row "Load".

### Jobs and runner

- (a) `ci.yml` gets a job `load` with `if: github.event_name != 'pull_request'`. It runs on `workflow_dispatch` and on a push to `main`, like the job `compose`.
- (b) The job runs on the hosted runner `ubuntu-24.04` with `timeout-minutes: 60`. The job has a matrix `image: [musl, cc-debian13]` with `fail-fast: false`.
- (c) The first step of each leg checks the runner. It requires at least 4 vCPU, 8 GiB of memory, and cgroup v2, as ADR-0070 (m) asks for the host.
- (d) A pull request does not start `load`, so `load` finds a slow command only after the merge. A red `load` on `main` blocks the next release.
- (e) A new job `ci-scripts` runs on each pull request without Docker. It runs `deploy/ci/check.sh`. That script runs each unit test `deploy/ci/test_*.py` by glob and the structure checks of `deploy/ci/`.
  - The owner adds `ci-scripts` to the branch protection of `main` after the merge of this change.

### Images

- (f) The leg `musl` builds `deploy/Dockerfile`, the shipped image. It reads the build cache scope `recipe-server` and writes no build cache.
- (g) The leg `cc-debian13` builds `deploy/ci/Dockerfile.cc-debian13`. No release uses that file. The leg uses its own build cache scope `recipe-server-cc-debian13`.
  - The file builds the target `x86_64-unknown-linux-gnu`. Its `COPY` line takes the binary from the path of that target.
  - Its runtime image is `gcr.io/distroless/cc-debian13:nonroot`, pinned by digest.
- (h) A check in `deploy/ci/check.sh` compares that file with `deploy/Dockerfile` after a fixed list of substitutions. Each other difference fails the check. The list has these lines:
  - the value of `RECIPE_TARGET`, the `CC_` line, and the install of `musl-tools`
  - the runtime `FROM` line with its digest, and the source path of the `COPY` line
- (i) Each leg must build its image, start the stack, and write its values. Only the leg `musl` gates the bounds.
  - In the leg `cc-debian13`, each step of a bound check has `continue-on-error: true`, because no release ships that image.

### Order of a leg

- (j) Each leg starts its own stack from zero with the scripts of the job `compose`. It then runs these steps in this order:
  1. The runner check of item (c).
  2. The first administrator and its password change, as in the job `compose`.
  3. A second user with the role `ProcessEngineer`.
  4. The seed, the count check, and the three timings.
  5. The first memory reading.
  6. A new container `app`, the checks before the flood, and the flood.
  7. The second memory reading and the memory check.
- (k) The flood runs last. After the flood, the per-IP budget blocks each login from the runner for 15 minutes.

### Seed and timings

- (l) `deploy/ci/load_seed.py` seeds one recipe through the HTTP routes of Caddy. It uses only the standard library of Python.
  - A failure text holds the status and the error code of the answer. It never holds a response body, because the answer of `CreateUser` holds the one-time password.
- (m) The seed uses the platform `CodesysIec`, which has no S7 address. One `DefineUdt` defines a UDT with 8 members. Each member is an `ARRAY[1..8] OF INT`.
  - Each of the `budget.parameters_per_version` parameters has this type, 64 leaves, and 8 limit rows `.m1[*]` to `.m8[*]`.
  - One `AddParameter` request carries the value and the 8 limit rows. The seed sends no separate limit request.
  - The draft then has 4 000 parameters, 256 000 leaves, and 32 000 limit rows. These counts are near the three size budgets of §7.11.
- (n) Before the first timing, the job reads the three counts of the draft from the database. A count other than a count of item (m) fails the leg.
- (o) The seed writes its request count and its elapsed time after each 500 requests. The seed step has `timeout-minutes: 30`.
  - The first dispatch run records the seed time in the pull request.
  - When the seed takes more than half of the job timeout, this change raises the job timeout before the merge.
- (p) The administrator edits the draft. The second user approves it. The job measures three requests once each:
  - `ApproveVersion` of the seeded draft by the second user.
  - `CreateVersion` from the `Approved` version.
  - `GET` of the version page of the `Approved` version.
- (q) A measurement uses a new connection and a monotonic clock. It starts before the connect and stops at the last byte of the answer.
  - The client follows no redirect. It sends the headers of the browser, also `HX-Request: true` for a form with `hx-post`.
  - The seed fills the caches, so a measurement is not a cold start.
- (r) Each time must be less than 10 s. A request that is not successful fails the job. The job writes the seed time and the three times into the job summary.

### Login flood

- (s) The job runs a pinned release of `oha`. One place in `deploy/ci/load.sh` holds its version and its SHA-256. The job checks the SHA-256 before it runs the binary.
- (t) Before the flood, the job replaces the container `app` with a new container and waits for `/readyz`. Then it checks these conditions:
  - The container `app` has `container.app_cpus` of 1 and the `memory.max` of `container.app_memory`.
  - The table `login_attempt` holds no failed login. Thus the `429` path of `login.ip_budget` cannot replace the path of the semaphore.
- (u) The flood is one call of `oha` with 200 requests and a concurrency of 200. It sends `POST /login` to `https://localhost:8443` through Caddy.
  - `oha` trusts the root certificate of `login.sh` with `--cacert`. It does not use `--insecure`.
  - The bodies come from a file of 2 000 form lines with random logins (`oha -Z`). `-Z` picks lines with replacement, so the file has 10 lines for each request.
  - Each line holds the `csrf_pre` token of one `GET /login`. The pre-session cookie and a matching `Origin` header go into fixed headers.
  - `oha` runs without a request timeout. The step timeout limits the call.
- (v) `deploy/ci/load_flood.py` reads the JSON output of `oha`. It refuses an output that does not match the schema of the pinned version. Unit fixtures pin the unit of each time field.
- (w) The flood fails the job in each of these cases:
  - The result does not hold exactly 200 answers, or a time is not finite.
  - The slowest request takes more than 5 s.
  - The error list of `oha` is not empty.
  - No answer has the status `503`.
  - An answer has a status other than `401`, `429`, or `503`.
  - The count of `E-WEB-009` lines with the cause "password hash permit" differs from the count of `503` answers.
- (x) The job reads only the log lines of `app` in the time window of the flood. It reports the count of each status, also the count of `429`.
  - The logins per second are the count of `401` and `429` answers divided by the total time of the flood.

### Memory check

- (y) The job reads the cgroup of `app` twice with the functions of `deploy/ci/memory.sh`. The first reading follows the timings. The second reading follows the flood on the new container.
- (z) A reading fails the job in each of these cases:
  - A value is `unavailable` or is not a number.
  - The count `oom`, `oom_kill`, or `max` of `memory.events` is above 0.
  - `memory.peak` reaches `container.app_memory`.
  - Docker reports `OOMKilled` or a restart count above 0.
- (aa) The job reports the peak of each reading. The larger peak is the peak of the whole run. The job records `anon` for information only.

### Values of ADR-0064 (n) and the amendment of ADR-0064 (o)

- (ab) Each leg writes the logins per second, the p99 latency, and the flood peak to the job summary and a JSON artifact.
- (ac) This item amends ADR-0064 (o). For the flood, a failure under (o) is an OOM kill, a restart, or a peak at `container.app_memory`.
  - The soak ceiling and the p99 bound of §6 apply to the soak only.
  - Each leg writes the line "ADR-0064 (o) memory condition" with the value "met" or "not met" into the job summary.
  - A switch to the fallback image stays a new ADR (ADR-0064 (p)).

### End-to-end smoke

- (ad) A new file `e2e/playwright.compose.config.ts` holds the projects `smoke-login` and `smoke-compose`. Their tests are in `e2e/compose/`.
  - The config starts no web server and runs no global setup.
  - Its base URL is the published port of Caddy. It uses 1 worker and 0 retries.
  - Only this config ignores the TLS errors of the certificate of `tls internal`.
  - The config stops before its first request when a password variable is missing or has fewer than 12 characters.
- (ae) The setup project `smoke-login` runs each step that types a password. It sets `trace`, `screenshot`, and `video` to `off`.
  - The administrator logs in with `E2E_ADMIN_PASSWORD` and creates a user with the role `ProcessEngineer`.
  - The project reads the one-time password from the page. That user logs in and sets `E2E_APPROVER_PASSWORD` at once.
  - The project saves the two sessions as `storageState` files in a directory outside the upload path.
- (af) The project `smoke-compose` depends on `smoke-login`. It keeps a trace on failure (ADR-0048 (e)). A session cookie in a trace ends with the stack of the job.
- (ag) Each run adds a unique suffix to the user login, the machine code, and the reference number. A second run on one stack then finds no duplicate. The flow is in `e2e/compose/end-to-end.spec.ts`.
- (ah) The administrator creates the machine, the recipe, and the parameter, and sets the limits. The second user approves the draft.
  - The administrator then exports the version with the export link of the version page and checks the hash.
  - At the end, the administrator checks the audit page.
  - The suite also checks that the editor of the draft gets `E-RCP-015` for an approval. The row "End to end" does not name it. It proves the stored default of `approval.separation_of_duties`.
- (ai) The hash check builds the hashed body of `api.md` §6 from an allow list of the fields of §6.1. It sorts each array in the order of §6.3.
  - It compares the SHA-256 of the RFC 8785 form with `content_sha256`. The suite has its own RFC 8785 function and adds no npm package.
  - Unit tests check the golden file of `crates/recipe-export/tests/golden/`, a copy with shuffled arrays, non-ASCII text, and one changed value.

### Smoke step in the job `compose`

- (aj) The job `compose` runs `deploy/ci/smoke.sh` after the step of the pgtools tests and before `stop.sh`. The step has `timeout-minutes: 10`.
  - The script reads `E2E_ADMIN_PASSWORD` from the state file `new_password`. It creates `E2E_APPROVER_PASSWORD` with `openssl rand -hex 16`.
  - It masks both values with `::add-mask::` first. It gives them only to the environment of its child process.
  - It writes no value to `$GITHUB_ENV` or to a command line.
- (ak) After the suite, `deploy/ci/secret_scan.py` unpacks each trace. It reads each file of the traces and of the HTML report.
  - A file that holds a password value fails the job. The script then moves no file into the upload directory.
  - After a clean scan, the job uploads the traces and the report with a short retention.
- (al) The job reuses the npm cache of `setup-node` and caches the browser of Playwright.
  - Before the owner approves the merge, a dispatch run without a cache hit records the time of the job `compose`.
  - A step timeout does not give a time reserve. When the job exceeds 45 minutes, a new ADR moves the suite to its own job (Option D).

## Consequences

### Positive

- Each bound of the rows "Load" and "End to end" has a check that fails CI.
- The repository holds the values of ADR-0064 (n) for both images after each run on `main`.
- ADR-0064 (o) gives a clear result for the flood on each image.
- A fault in Caddy, in the grants of `recipe_app`, or in the migrated schema fails the smoke suite.
- A pull request runs the unit tests of the CI scripts in about one minute.

### Negative

- Each run on `main` builds the server image three times: once in `compose` and once in each leg of `load`.
- The job `compose` takes more time for Node, Chromium, and the suite.
- The seed sends several thousand requests and adds minutes to each leg.
- A hosted runner has no fixed CPU speed. A time near 10 s can fail at random.
- A slow command reaches `main` before `load` finds it. The release waits for a green `load`.

### Risks and mitigations

- Risk: the job `compose` exceeds 45 minutes. Mitigation: item (al) records the time without a cache. A new ADR then moves the suite to its own job.
- Risk: a hosted runner is slower than the plant host. Mitigation: the job summary records each time, so a trend is visible before a bound fails.
- Risk: the flood gives no `503` on a fast runner. Mitigation: item (t) checks 1 CPU and a fresh `login_attempt` table. The flood has 200 requests for 4 permits.
- Risk: the seed time grows faster than the request count. Mitigation: item (o) records the seed time before the merge.
- Risk: a password reaches a CI artifact. Mitigation: items (ae) and (ak).
- Risk: the leg `cc-debian13` drifts from the shipped build. Mitigation: the check of item (h).

## Related

- ADR-0048 (a), (c), (e) (files of `e2e/`, smoke suite on the Compose stack, passwords from the environment)
- ADR-0064 (n) to (p) (flood on two images, fallback rule). This ADR amends (o)
- ADR-0070 (m), (w) to (y), ADR-0094 (host size, job `compose` and its trigger)
- ADR-0035 (pre-session token, per-IP budget), ADR-0055 (guard of `/export/`), ADR-0127 (test gates)
- `reliability.md` §6 rows "Load", "Stack", "End to end", and "Soak", and §7.11
- `api.md` §6 (hashed body), §7.3 (pre-session token), and §7.4 (login protection)
- Change `load-and-smoke`, capability `operations`
- `overview.md` §7: R04, R20, R27, R30, R31. Section 9: A26, A34
