# Proposal: load-and-smoke

## Why

The review of 2026-11-04 found two test levels of `reliability.md` §6 without a test (D6-2, D6-4).

- The row "Load" has no `oha` script and no CI job. A slow `ApproveVersion` or a login flood without a bound passes CI.
- The row "End to end" has no smoke suite on the Compose stack. A fault in Caddy, in the grants, or in the migrated schema can break the export flow unseen.
- ADR-0064 (n) asks for an `oha` flood on the musl image and on a `cc-debian13` build. No result exists.
- ADR-0064 (o) applies the p99 bound of the soak also to the flood. The flood queues on the semaphore by design, so every flood fails that bound.

The change satisfies R04, R20, R27, R30, and R31. It uses A26 (size budgets) and A34 (separation of duties).

## What Changes

- A new CI job `load` runs on a push to `main` and on `workflow_dispatch`, like the job `compose` (ADR-0094). A red `load` on `main` blocks the next release.
- The job checks the runner first: 4 vCPU, 8 GiB of memory, and cgroup v2.
- The job seeds one draft through HTTP with 4 000 parameters, 256 000 leaves, and 32 000 limit rows. It checks these counts before it measures `ApproveVersion`, `CreateVersion`, and the version page.
- The job runs a login flood of 200 concurrent `POST /login` with random logins on a new container `app`.
  - It checks exactly 200 answers, the bound of 5 s, and the `503` of the Argon2 semaphore.
  - It also checks that each `503` has a log line with the cause "password hash permit".
- The job reads the memory of `app` twice. An OOM kill, a restart, a peak at 512 MiB, or a missing value fails it.
- The job records the values of ADR-0064 (n) for the musl image and for a `cc-debian13` build. Both legs must build and start. Only the musl leg gates the bounds.
- ADR-0128 amends ADR-0064 (o). For the flood, only the memory bounds of the container limit apply. The p99 bound applies to the soak only.
- A Playwright setup project `smoke-login` types each password with the trace off. The project `smoke-compose` runs the flow of the row "End to end" against Caddy in the job `compose`.
- One script `deploy/ci/smoke.sh` holds the passwords in the environment of its child processes only. A scan of the traces and the report blocks the upload when it finds a password.
- A new pull-request job `ci-scripts` runs the unit tests and the structure checks of `deploy/ci/`.
- ADR-0128 records the jobs, their triggers, and their runner. The rows of `reliability.md` §6 stay unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: seven ADDED requirements.
  - The load test, the cause of each `503` of the flood, and the memory of `app` under load.
  - The tests of the CI scripts on a pull request, and the request times at the size budget.
  - The End-to-end smoke and the passwords of the smoke suite.

## Out of scope

- The Soak level (D6-3). It stays a known gap by decision Q-B (3) of the fix plan.
- A change of a bound or a setting of `reliability.md` §7.11.
- A change of the shipped image or of `deploy/Dockerfile`.
- A switch to the fallback image of ADR-0064 (o). A switch needs its own ADR (ADR-0064 (p)).
- A new subcommand of `recipe-server`.
- A pull-request trigger for the job `load`.
- The branch protection of `main`. The owner adds `ci-scripts` to it after the merge.

## Impact

- New scripts in `deploy/ci/`, each with a unit test. They cover the client, the seed, the timings, the flood, the memory check, the smoke step, and the artifact scan.
- `deploy/ci/check.sh` runs each `deploy/ci/test_*.py` by glob and gets three structure checks.
- A CI-only Dockerfile for the `cc-debian13` build.
- `.github/workflows/ci.yml`: the new jobs `load` and `ci-scripts`, and a new step in the job `compose`.
- `e2e/`: a new config, the projects `smoke-login` and `smoke-compose`, and the RFC 8785 function with its unit test.
- ADR-0128, the "Amended by" note of ADR-0064, the index row, and the decision map in `docs/adr/README.md`.
- One line in `CHANGELOG.md`. `CONTRIBUTING.md` names the two new jobs and the release rule.
