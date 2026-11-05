# Tasks: load-and-smoke

Each task names its agent. One task goes to one agent. Each task starts with a failing test where a test is possible. The decisions D1 to D10 are in `design.md`. ADR-0128 holds the decision items.

Executors: Codex (gpt-6-astra) writes shell scripts, Compose files, CI workflows, and Python CI scripts. Sonnet writes TypeScript code and the Playwright tests. Haiku does simple doc edits. This change holds no Rust code.

The owner answers are in `design.md` (section "Answers of the owner") and in ADR-0128, status Accepted.

## Waves

The waves run in this order. Tasks of one wave touch different files and can run in parallel. Two tasks that edit `deploy/ci/check.sh` or `.github/workflows/ci.yml` never share a wave.

1. Wave 1: 1.1, 2.1, 3.1, 3.2, 6.1, 6.2, 6.5
2. Wave 2: 2.2, 2.3, 4.1, 6.3, 6.6
3. Wave 3: 3.3, 6.4
4. Wave 4: 5.1
5. Wave 5: 6.7
6. Wave 6: 7.1, 7.2
7. Wave 7: 8.1

## 1. CI scripts on pull requests

- [x] 1.1 (deploy/ci, .github/workflows) Add a temporary `deploy/ci/test_zz_fail.py` that exits 1, and run `bash deploy/ci/check.sh`. It passes, because `check.sh` names its tests. Replace the five named test lines with a loop over `deploy/ci/test_*.py`, then remove the temporary file. Add the structure check `test_ci_scripts_job` to `check.sh`. Then add the job `ci-scripts` to `ci.yml` to ADR-0128 (e): trigger `pull_request`, `ubuntu-24.04`, a job timeout, and `bash deploy/ci/check.sh` without Docker. Check: `check.sh` fails with the temporary file and passes without it, also with `test_migration_rewrite.py`. The job passes on the pull request. Agent: Codex.

## 2. Seed and timings

- [x] 2.1 (deploy/ci) Write `deploy/ci/test_load_client.py` first, with `unittest` and a fake connection in the pattern of `test_http_probe.py`. Cover the login with the pre-session token, the CSRF token of a form, and `row_version` from `HX-Trigger`. Cover `CreateUser`, the parse of the one-time password from a fake page, and its replacement. Cover a failed answer whose body holds a password-like string: the failure text holds the status and the code, not that string. Cover a password from a state file and never from `sys.argv`. Then write `deploy/ci/load_client.py` to D2 and ADR-0128 (l). Reuse `HttpCheckError`. Check: the test fails before `load_client.py` exists and passes after it. Agent: Codex.
- [x] 2.2 (deploy/ci) Write `deploy/ci/test_load_seed.py` first. Cover the `DefineUdt` body of the UDT with 8 members `ARRAY[1..8] OF INT` on `CodesysIec`. Cover one `AddParameter` body with the value and the 8 limit rows `.m1[*]` to `.m8[*]`. Cover the progress line after each 500 requests. Cover the count check: 4 000, 256 000, and 32 000 pass, and 31 999 limit rows fail with the name of the count. Then write `deploy/ci/load_seed.py` to D2 and ADR-0128 (m) to (o) on the client of 2.1. Check: the test fails before `load_seed.py` exists and passes after it. Agent: Codex.
- [x] 2.3 (deploy/ci) Write `deploy/ci/test_load_timing.py` first. Cover a time of 9.9 s (passes) and of 10 s (fails with the name of the request). Cover an answer that is not successful (fails with the status). Cover a redirect answer that the client does not follow. Cover `HX-Request: true` on the two form posts. Cover a monotonic clock from before the connect to the last byte. Cover the summary line of the seed time and the three times. Then write `deploy/ci/load_timing.py` to D3 and ADR-0128 (p) to (r). Check: the test fails before `load_timing.py` exists and passes after it. Agent: Codex.

## 3. Login flood and memory check

- [x] 3.1 (deploy/ci) Write `deploy/ci/test_load_flood.py` first, with `unittest` and JSON fixtures of the pinned `oha` version. Cover each failure case of ADR-0128 (w) with its own fixture and its failure text. Cover an output that does not match the schema of the pinned version. Pin the unit of each time field with a fixture. Cover the 429 count and the logins per second of ADR-0128 (x). Cover the commands `runner` and `preflight` of D4 with a pass case and a fail case for each condition. Cover the body file: 2 000 lines, each login in the grammar of `TypedLogin`, the one `csrf_pre` value on each line, and form encoding. Then write `deploy/ci/load_flood.py` to D4 and ADR-0128 (c), (t) to (x). Check: the test fails before `load_flood.py` exists and passes after it. Agent: Codex.
- [x] 3.2 (deploy/ci) Write `deploy/ci/test_load_memory.py` first. Cover each failure case of ADR-0128 (z) with its own fixture. The cases are `unavailable`, `oom`, `oom_kill`, `max`, a peak at 512 MiB, `OOMKilled`, and a restart. Cover a valid pair of readings that passes and reports both peaks and the peak of the whole run. Cover the line "ADR-0064 (o) memory condition" with "met" and "not met". Then write `deploy/ci/load_memory.py` to D5, D6, and ADR-0128 (y) to (ac). Check: the test fails before `load_memory.py` exists and passes after it. Agent: Codex.
- [ ] 3.3 (deploy/ci) Write `deploy/ci/test_load_sh.py` first. It runs `load.sh` with stub `oha`, `docker`, and `curl` programs on `PATH`. Cover a bad SHA-256 of the download, a non-zero exit of `oha`, and the arguments of `oha`. The arguments include `https://localhost:8443/login`, a matching `Origin`, `--cacert "$CI_STATE/root.crt"`, `-n 200 -c 200`, `-Z`, and no `-t` and no `--insecure`. Cover the log window of the flood and the recreate of `app` before the flood. Then write `deploy/ci/load.sh` to D4 and ADR-0128 (j), (s) to (u), and (y). One place holds the version and the SHA-256 of `oha`. Check: the test fails before `load.sh` exists and passes after it, and `bash deploy/ci/check.sh` passes. Agent: Codex.

## 4. The `cc-debian13` build

- [x] 4.1 (deploy/ci) Add a structure check to `deploy/ci/check.sh` to ADR-0128 (h). It compares `deploy/ci/Dockerfile.cc-debian13` with `deploy/Dockerfile` after the fixed list of substitutions. Add a fixture with one changed `RUN` line that the check must refuse. The check fails, because the file does not exist. Then write `deploy/ci/Dockerfile.cc-debian13` to D7 and ADR-0128 (g). Its `COPY` line takes the binary from `x86_64-unknown-linux-gnu`. Check: the check passes, and `docker build -f deploy/ci/Dockerfile.cc-debian13 .` builds an image. The leg `cc-debian13` of task 5.1 starts the stack with it. Agent: Codex.

## 5. CI job `load`

- [ ] 5.1 (deploy/ci, .github/workflows) Add a structure check of the job `load` to `deploy/ci/check.sh`, in the pattern of `test_migration_freeze_job`. It requires the `if:`, the runner, the job timeout, the matrix with `fail-fast: false`, and the steps of ADR-0128 (j). It also requires the cache scopes of ADR-0128 (f) and (g), the seed step timeout, and `continue-on-error` only on the bound steps of the leg `cc-debian13`. The check fails, because the job does not exist. Then add the job `load` to `ci.yml` to D1 and ADR-0128 (a) to (d), (f), (g), (i), (j), and (ab). Each leg uploads its JSON values as an artifact. Check: the check passes, and the workflow file parses as YAML. A `workflow_dispatch` run on the branch passes the leg `musl`, and the leg `cc-debian13` builds, starts, and writes its values. Agent: Codex.

## 6. End-to-end smoke

- [x] 6.1 (e2e) Write the failing test `e2e/tests/jcs.spec.ts`. It covers four cases of ADR-0128 (ai). The first case is the golden file `crates/recipe-export/tests/golden/recipe_R-100_v3.json`. The second case is a copy with shuffled arrays and the same hash. The third case is non-ASCII text. The text case uses Polish letters and one character outside the BMP. The fourth case changes one value and expects a different hash. Then write `e2e/compose/support/jcs.ts` with no new npm package. It builds the hashed body from an allow list of the fields of `api.md` §6.1 and sorts each array by §6.3. Check: the test fails before the function exists and passes after it in `npx playwright test tests/jcs.spec.ts`. Agent: Sonnet.
- [x] 6.2 (e2e) Write `e2e/playwright.compose.config.ts` to ADR-0128 (ad) and (ae) with the projects `smoke-login` and `smoke-compose` and `testDir` `./compose`. `smoke-login` sets `trace`, `screenshot`, and `video` to `off`. `smoke-compose` depends on it and keeps `retain-on-failure`. The config stops with the name of the variable when `E2E_ADMIN_PASSWORD` or `E2E_APPROVER_PASSWORD` is missing or has fewer than 12 characters. It reads the base URL from `E2E_BASE_URL` and the session directory from `E2E_STATE_DIR`. Check: `npx playwright test -c playwright.compose.config.ts --list` without the variables exits non-zero and names the variable. `npx playwright test --list` of the regression config lists no file of `compose/`. Agent: Sonnet.
- [x] 6.3 (e2e) Write `e2e/compose/login.setup.ts` for the project `smoke-login` to ADR-0128 (ae). The administrator logs in and creates a user with a unique suffix and the role `ProcessEngineer`. The script reads the one-time password from the page, and the new user replaces it with `E2E_APPROVER_PASSWORD` at once. The script saves both sessions in `E2E_STATE_DIR`. Check: on the local dev stack of `CLAUDE.md`, the project passes twice in a row, and `test-results/` holds no trace of it. Agent: Sonnet.
- [ ] 6.4 (e2e) Write `e2e/compose/end-to-end.spec.ts` with `playwright-cli` to D8 and ADR-0128 (af) to (ai). The flow follows the scenario "Flow of the smoke suite" of the spec delta. It also covers the scenario "Approval by the editor of the draft". Each run uses a unique suffix for the machine code and the reference number. Check: the suite passes twice in a row against the local dev stack with both password variables set. Agent: Sonnet.
- [x] 6.5 (deploy/ci) Write `deploy/ci/test_secret_scan.py` first. Cover a trace zip with a password value in a request body, and a report file with that value. Cover a clean set of files. Cover a failure text that does not print the value. Then write `deploy/ci/secret_scan.py` to ADR-0128 (ak). It reads the values from its environment, unpacks each zip, and reads each file. Check: the test fails before `secret_scan.py` exists and passes after it. Agent: Codex.
- [x] 6.6 (deploy/ci) Write `deploy/ci/test_smoke_sh.py` first. It runs `smoke.sh` with a stub `npx` on `PATH`. Cover a stub that writes a trace with a password: the script fails, and the upload directory stays empty. Cover a clean run: the files reach the upload directory. Cover the environment of the child: both variables are set, no command line holds a value, and nothing writes to `$GITHUB_ENV`. Then write `deploy/ci/smoke.sh` to D9 and ADR-0128 (aj) and (ak). Check: the test fails before `smoke.sh` exists and passes after it. Agent: Codex.
- [ ] 6.7 (deploy/ci, .github/workflows) Add a structure check of the smoke step to `deploy/ci/check.sh`. It requires that the step runs `deploy/ci/smoke.sh` after `pgtools.sh` and before `stop.sh`, with `timeout-minutes: 10`. It requires no write to `$GITHUB_ENV` in the step, and an upload path of the upload directory of `smoke.sh` only. The check fails, because the step does not exist. Then add the steps to the job `compose` to D9 and ADR-0128 (aj) to (al). They are Node from `e2e/.nvmrc` with the npm cache, `npm ci`, the cached Chromium, `smoke.sh`, and the upload with a short retention. Check: the check passes, and a `workflow_dispatch` run on the branch passes the job `compose` within 45 minutes. Agent: Codex.

## 7. Docs

- [ ] 7.1 (CHANGELOG.md) [mechanical] Add one line under "Unreleased", "Added": the CI jobs `load` and `ci-scripts` and the smoke suite on the Compose stack (ADR-0128). Change no line of "Known gaps", because the soak gap stays. Check: `ste-lint.py` reports 0 hard violations on `CHANGELOG.md`. Agent: Haiku.
- [ ] 7.2 (CONTRIBUTING.md) [mechanical] In the section "Jobs outside pull requests", name the job `load`. Add the rule that a red `load` on `main` blocks the next release (ADR-0128 (d)). Name the pull-request job `ci-scripts` (ADR-0128 (e)). Check: `ste-lint.py` reports 0 hard violations on `CONTRIBUTING.md`. Agent: Haiku.

## 8. Verification

- [ ] 8.1 (workspace) Run `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked`, `cargo fmt --all --check`, `openspec validate load-and-smoke --strict`, and the `ste-lint.py` command of `CLAUDE.md`. Record these values of the dispatch run in the pull request body:
  - the seed time, the three times, the flood values, and both peaks of each leg
  - the line "ADR-0064 (o) memory condition" of each leg
  - the time of the job `compose` in a run without a cache hit

  Check: each command passes. When the seed takes more than half of the job timeout, ADR-0128 (o) applies before the merge. Agent: orchestrator.

## After the merge

- The owner approves the addition of the job `ci-scripts` to the branch protection of `main`. No task of this change makes that edit.

## Workflow follow-up

- `code-review` (Opus) and a Codex second review (gpt-6-astra, effort high). Fix every CRITICAL and HIGH finding.
- `superpowers:verification-before-completion` with the clippy, test, and fmt output.
- `openspec-archive-change` after `cargo test` and `openspec validate` pass.
