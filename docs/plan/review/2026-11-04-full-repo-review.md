# Full repository review, 2026-11-04

- Scope: HEAD `3885a26` on `main`. The review covers `crates/`, `migrations/`, `deploy/`, `e2e/`, `tools/`, `.github/workflows/`, `openspec/`, `docs/architecture/`, `docs/adr/`, `docs/ops/`, `docs/user/`, `README.md`, `CONTRIBUTING.md`, and `CHANGELOG.md`.
- Method: baseline gates, a `code-review-graph` map, eight read-only reviewers (D1 to D8), and a Codex second review of D1 and D3.
- Each CRITICAL or HIGH finding got an adversarial check. One MEDIUM candidate (D7-1) also got one.
- This file is not normative. It records findings only. The review changed no code, doc, spec, or config.

Dimensions:

| Id | Dimension | Reviewer |
|---|---|---|
| D1 | Architecture and domain | Opus |
| D2 | Rust correctness, errors, panic policy, async | Opus |
| D3 | Security | Opus |
| D4 | PostgreSQL, migrations, locks, grants | Opus |
| D5 | Web UI, HTMX, i18n, accessibility | Opus |
| D6 | Tests and coverage | Opus |
| D7 | Deploy, CI, operations | Opus |
| D8 | Docs, specs, traceability | Sonnet |
| X1, X3 | Second review of D1 and D3 | Codex (gpt-6-astra, effort high) |

## 1. Gate results

| Gate | Command | Result | Evidence |
|---|---|---|---|
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` | pass | Exit code 0 |
| Tests | `cargo test --workspace --all-features --locked` | pass | 33 suites, 7783 passed, 0 failed, 0 ignored. The `recipe-db` storage suite ran 1179 tests on testcontainers (Docker 29.8.2). |
| Format | `cargo fmt --check` | pass | Exit code 0 |
| Dependencies | `cargo deny check` | pass | advisories, bans, licenses, and sources ok. Warnings for duplicate crate versions only, for example `winnow`. |
| Specs | `openspec validate --all` | pass | 7 passed, 0 failed |
| STE lint | `ste-lint.py` on the scope of ADR-0023 | pass | 0 hard violations (baseline 0). 306 advisory items (passive voice, present perfect). |
| Coverage | `cargo llvm-cov --workspace` | not run | The review did not measure coverage. D6-1 is about the gate itself. |
| Browser suite | `e2e/` Playwright | not run | It needs a password secret and a server. CI job `playwright.yml` owns it. |
| Compose stack | `docker compose ... up` | not run | The review started no dev stack. |
| ShellCheck | `shellcheck` on `deploy/` | not run | The tool is not installed on the review host. |

## 2. Counts

Counts are after the adversarial checks and after the merge of duplicates. D8-4 is merged into D6-3.

| Dimension | CRITICAL | HIGH | MEDIUM | LOW | Total |
|---|---|---|---|---|---|
| D1 Architecture | 0 | 0 | 0 | 4 | 4 |
| D2 Rust | 0 | 0 | 1 | 3 | 4 |
| D3 Security | 0 | 0 | 0 | 1 | 1 |
| D4 PostgreSQL | 0 | 0 | 2 | 3 | 5 |
| D5 Web UI | 0 | 1 | 0 | 5 | 6 |
| D6 Tests | 0 | 1 | 5 | 2 | 8 |
| D7 Deploy and CI | 0 | 1 | 0 | 6 | 7 |
| D8 Docs and specs | 0 | 0 | 1 | 6 | 7 |
| X1, X3 Codex | 0 | 0 | 0 | 0 | 0 |
| Total | 0 | 3 | 9 | 30 | 42 |

Adversarial verdicts:

| Id | Reviewer severity | Verdict | Final severity |
|---|---|---|---|
| D7-1 | MEDIUM | CONFIRMED | HIGH |
| D5-1 | HIGH | CONFIRMED | HIGH |
| D6-2 | HIGH | CONFIRMED | HIGH |
| D4-1 | HIGH | CONFIRMED, narrower effect | MEDIUM |
| D6-1 | HIGH | DISPROVED as stated. A residual stays. | MEDIUM |
| D6-3 | HIGH | CONFIRMED, KNOWN | MEDIUM |

## 3. Findings

The findings are in order of severity. Inside one severity, the order is by risk.

### HIGH

#### D7-1 The init script lets a slow `CREATE ROLE` write a plain password to the log

- severity: HIGH (the reviewer gave MEDIUM)
- location: `deploy/db/initdb/01-roles.sh:72`
- rule: `reliability.md` §3.4 (the log never holds a password). `docs/ops/runbook.md:722` names the same risk for `ALTER ROLE ... PASSWORD`.
- evidence:
  ```
  SET log_statement = 'none';
  SET log_min_error_statement = 'panic';
  CREATE ROLE recipe_migrator   LOGIN PASSWORD :'migrator_password';     -- owner of the schema
  ```
- failure scenario:
  1. `deploy/compose.yaml:218` gives `db` the option `log_min_duration_statement=1000`.
  2. The entrypoint of the postgres image passes the `command:` options to its init server. So the option is live while `01-roles.sh` runs.
  3. On the first start, a WAL flush stall makes one `CREATE ROLE ... PASSWORD` run for 1 s or more.
  4. The server writes `duration: ... statement: CREATE ROLE ... PASSWORD '<plain>'` to stderr.
  5. The json-file log of `db` keeps the line for up to 20 files of 50 MB (`compose.yaml:18-22`). A forwarded log collector copies it off the host.
- The four passwords of `recipe_migrator`, `recipe_app`, the backup role, and the replication role are at risk.
- `docs/ops/runbook.md:711`, `:722`, `:1639` and `deploy/ci/switch-steps.sh:52` already add `SET log_min_duration_statement = -1;` for the same statement shape.
- `data-model.md` §5.1 (lines 1561 and 1562) and ADR-0031 line 49 name only the two settings. The gap is in the normative text too.
- fix direction: add `SET log_min_duration_statement = -1;` after line 71 and to the block of `data-model.md` §5.1, through a new ADR.
- status: CONFIRMED. The mechanism is traced end to end. The leak needs a statement of 1 s or more. Not KNOWN.

#### D5-1 The parameter table ignores the UI language and the `(en)` marker

- severity: HIGH
- location: `crates/recipe-web/src/pages/versions/parameter_table.rs:282`
- rule: `api.md` §8 (line 734), `openspec/specs/web-ui/spec.md:284` Requirement "Translation fallback" with its scenario at line 290 (R10, A17), ADR-0012 line 43
- evidence:
  ```rust
  ParameterRowView {
      href: format!("{base}/parameters/{}", row.key),
      tag: row.tag.to_string(),
      name: row.name_en.to_string(),
  ```
- failure scenario: the parameter `SPEED` has the English name `Line speed` and a stored `pl` translation. A user with `recipe_lang=pl` opens a version page. The table shows `Line speed`. Without the `pl` translation, the spec expects `Line speed (en)`, and the page shows `Line speed`.
- A second site has the same defect: `crates/recipe-web/src/pages/parameters/new.rs:101` builds the row fragment after a parameter is created.
- Root cause: the read-port row `ParameterRow` (`crates/recipe-app/src/ports/recipe_reads.rs:118`) has no translation field. The fix touches `recipe-app`, `recipe-db`, and `recipe-web`.
- Translations exist through the whole stack: the `parameter_translation` table, `Parameter::translations()`, the use cases, and the route `.../translations/{lang}`. No test covers the spec scenario.
- fix direction: give the rows the translation of the UI language. Render it when it exists, else `name_en` with the `(en)` marker. Add the spec scenario as a test.
- status: CONFIRMED. Not KNOWN.

#### D6-2 The Load test level of `reliability.md` §6 has no test

- severity: HIGH
- location: `docs/architecture/reliability.md:501`
- rule: `reliability.md` §6 row "Load". It asks for an `oha` flood of 200 concurrent `POST /login`, the memory limit, the 5 s bound, and the `503` of the Argon2 semaphore. It also asks for the 10 s budget of `CreateVersion`, `ApproveVersion`, and the version page at the size budget.
- evidence:
  ```
  crates/recipe-web/src/pages/versions/large_tests.rs:16
  const BYTE_BOUND: usize = 40_960;
  ```
  The repo holds no `oha` or `k6` script. No workflow has a flood step. The large-page test checks bytes, not time.
- failure scenario: a change makes `ApproveVersion` on a draft with 4 000 parameters take 30 s. Or a login flood queues on the Argon2 semaphore without bound. No CI job fails.
- Six accepted ADRs name "the load test of step P4.2" as a risk mitigation: ADR-0039:116, ADR-0045:72, ADR-0050:117, ADR-0051:141, ADR-0056:69, ADR-0122:75.
- ADR-0064 (n) asks for one `oha` flood on the musl and `cc-debian13` builds. The repo holds no result. ADR-0064 (o) uses that result as the trigger of the musl fallback.
- P4.2 and P4.3 ran. `implementation-prompts.md:428-449` (P4.2) has no separate Load item, so the level fell through.
- fix direction: add the `oha` load script and a CI job on the Compose stack that asserts the four bounds of the row.
- status: CONFIRMED. Not KNOWN. Unlike the Soak gap, no "Known gaps" list names it. `2026-10-12-architecture.md:43` only asked for the test.

### MEDIUM

#### D4-1 A cancelled `BEGIN` can return an open transaction to the pool (sqlx 0.8.6)

- severity: MEDIUM (the reviewer gave HIGH)
- location: `crates/recipe-db/src/unit_of_work.rs:42`
- rule: `reliability.md` §7.4 (bounded transactions), the isolation rules of `data-model.md` §6, ADR-0042, ADR-0050. ADR-0058 (m) says that the open transaction of an aborted connection rolls back. That is false inside this window.
- evidence:
  ```
  unit_of_work.rs:42                         .begin_with(begin_statement(isolation))
  sqlx-postgres-0.8.6 src/transaction.rs:34  rollback.conn.wait_until_ready().await?;
  sqlx-postgres-0.8.6 src/transaction.rs:38  rollback.conn.inner.transaction_depth += 1;
  ```
- failure scenario:
  1. The request timeout (`recipe-web/src/timeout.rs:45`), the export timeout (`recipe-app/src/blocking.rs:156`), or a client disconnect drops the future after `BEGIN` is queued and before the depth increments.
  2. The drop guard sends a `ROLLBACK` only when `transaction_depth > 0`, so it sends nothing.
  3. The `ping` of `return_to_pool` drains the reply of `BEGIN` and returns the connection to the pool with an open server transaction. `database.rs:187-208` sets no `after_release` or `before_acquire` hook.
  4. The next `BEGIN` on that connection gets only the warning `25001`. A leaked `READ ONLY` transaction makes the next write fail with `25006`. A leaked `READ COMMITTED` transaction takes away the single snapshot of the draft, preview, and diff queries. `now()` is stale.
  5. The autocommit session touch (`session_store.rs:93`) can be lost, and its row lock stays until the next `COMMIT` or the 15 s `idle_in_transaction` kill. The next `COMMIT` commits command and audit writes.
- The same pattern is at `login_attempts.rs:130`, `:169`, and `security_events.rs:64`. The window is one round trip of `BEGIN`.
- fix direction: add an `after_release` hook that asks the server for an open transaction and closes such a connection. Or move to sqlx 0.9 through a new ADR that amends A15.
- Note: `Connection::is_in_transaction()` reads `transaction_depth`, which is 0 here, so a hook cannot use it. A server query such as `SELECT transaction_timestamp() <> statement_timestamp()` works.
- status: CONFIRMED in the sqlx 0.8.6 source, not reproduced against a live database. Not KNOWN.

#### D6-1 The coverage gate still counts test helper files and inline test modules

- severity: MEDIUM (the reviewer gave HIGH)
- location: `tools/coverage-gate.py:56`
- rule: `reliability.md` §6 (line 508): "Coverage target: 80 % line coverage per crate, measured with `cargo llvm-cov`."
- evidence:
  ```
  coverage-gate.py:11-12  The lines of an inline `#[cfg(test)]` module count as lines of the crate.
  coverage-gate.py:56     crate = crate_of_file(entry["filename"], crates)
  ci.yml:511              cargo llvm-cov --workspace ... --summary-only
  ```
- The original claim is disproved. CI installs `cargo-llvm-cov` 0.6.24. Since 0.6.22 its default regex excludes `tests.rs`, `*_tests.rs`, and every `tests/` directory from the report.
- failure scenario: the report still counts `test_world.rs`, `test_kit.rs`, `test_helpers.rs`, `test_support*`, the files in `components_tests/` and `style_tests/`, and inline `#[cfg(test)]` modules.
- These lines are about 29 % of the counted `recipe-web` lines and about 23 % of `recipe-domain`. Production code then passes the gate at about 72 % to 74 %, not 80 %. The figures are estimates from raw lines.
- fix direction: add an `--ignore-filename-regex` for the helper paths to CI and `CONTRIBUTING.md`, and state in `reliability.md` §6 whether inline test modules count.
- status: CONFIRMED for the residual. Not KNOWN.

#### D6-6 The grant test does not see grants to `PUBLIC`

- severity: MEDIUM
- location: `crates/recipe-db/tests/storage/grants/checks.rs:47`
- rule: ADR-0069 (q), `.claude/rules/sql.md` "Tests" (the grants of `recipe_app` equal `grants.expected`)
- evidence:
  ```
  WHERE g.grantee = 'recipe_app' AND g.specific_schema = 'public'";
  ```
  The table query (line 34) and the column query (line 40) filter on the grantee in the same way.
- failure scenario: a new migration adds a `SECURITY DEFINER` function and forgets `REVOKE EXECUTE ... FROM PUBLIC`. `recipe_app` can then run it as the owner. The grant test passes, because the grant goes to `PUBLIC`.
- fix direction: compare the effective rights of `recipe_app` with `has_function_privilege` and `has_table_privilege`, or add the `PUBLIC` rows to the compared set.
- status: CONFIRMED for the mechanism. No current function has this fault.

#### D2-1 A panic of an accept loop closes the public port without a stop

- severity: MEDIUM
- location: `crates/recipe-server/src/serve/opened.rs:183`
- rule: `reliability.md` §2.4 rule 6 (six places catch a panic), §2.4 rule 5 (watchdog), R30
- evidence:
  ```
  match handle.into_task().await {
      Ok(outcome) => outcome.cut(),
      Err(error) => {
          // The panic hook has written the `ERROR` line of a panic. The join adds one `WARN` line.
          tracing::warn!(
  ```
- failure scenario: `start.rs:386` spawns each accept loop. Only `join_listener` reads its handle, after the stop signal. A panic ends the public loop, and Caddy gets "connection refused". The watchdog probes only the internal listener, so `/healthz` stays `ok`. The process serves nothing until a person restarts it.
- This join is a seventh place that catches a panic. The join of the watchdog thread (`watchdog.rs:192`) is an eighth. §2.4 rule 6 names neither.
- fix direction: race the signal against the end of each accept loop, and stop with an error when a loop ends. Add both joins to §2.4 rule 6 through an ADR.
- status: PLAUSIBLE. The loop body has no known panic path.

#### D4-2 No CI step rejects a table-rewriting migration

- severity: MEDIUM
- location: `.github/workflows/ci.yml:547`
- rule: `data-model.md` §8 (line 2063) and `reliability.md` §5 (line 483): "CI rejects a migration that rewrites a table or scans it under an exclusive lock".
- evidence:
  ```
  run: python3 -I -B deploy/ci/migration_freeze.py
  ```
  That script checks only a change of the major version. No other rewrite check exists in `crates/`, `tools/`, `deploy/`, or `.github/`.
- failure scenario: a new migration with `ALTER TABLE ... TYPE` passes CI. On a large `audit_log` table it holds an `ACCESS EXCLUSIVE` lock during the upgrade, and the `lock_timeout` retries of the migrator fail. Migrations 0001 to 0010 comply today.
- fix direction: add a CI step that scans new files in `migrations/` for the forbidden forms of §8.
- status: CONFIRMED

#### D6-4 The end-to-end smoke suite on the Compose stack does not exist

- severity: MEDIUM
- location: `e2e/tests/smoke.spec.ts:3`
- rule: `reliability.md` §6 row "End to end" (line 504), ADR-0048 (c)
- evidence:
  ```
  test('the login page opens and shows its heading', async ({ page }) => {
  ```
  This is the only smoke test. `e2e/playwright.config.ts:56` starts `cargo run -p recipe-web --example dev_server`, not the Compose stack.
- failure scenario: a fault in Caddy, in the `recipe_app` grants, or in the migrated schema breaks the export flow. The Rust tests use in-memory ports or a bare database, and Playwright uses the dev server. No test sees the fault.
- fix direction: add a Playwright project that runs the flow of the row against the Compose stack of the `compose` job.
- status: CONFIRMED

#### D6-5 The route matrix does not assert `X-Request-Id` or the security headers

- severity: MEDIUM
- location: `crates/recipe-web/src/pages/test_helpers.rs:73`
- rule: `reliability.md` §6 row "HTTP": "Every route: status code, error code in the body, `X-Request-Id` header, security headers, no secret in the body".
- evidence:
  ```
  assert_eq!(reply.status, status, "{what}: {}", reply.body);
  assert!(reply.body.contains(code), "{what}: {}", reply.body);
  ```
  `assert_want` (`routes/matrix.rs:211`) checks only the status for `Want::Page`. The full `SECURITY_HEADERS` list is checked only on the health and panic routes.
- failure scenario: a new router is mounted outside the layer that sets `X-Request-Id` and `X-Content-Type-Options`. Its routes answer without these headers, and the matrix still passes.
- fix direction: make `assert_want` assert a version 7 `X-Request-Id` and every header of `SECURITY_HEADERS` on each cell.
- status: CONFIRMED

#### D6-3 The Soak test level has no test and no nightly job

- severity: MEDIUM (the reviewer gave HIGH)
- location: `docs/architecture/reliability.md:505`
- rule: `reliability.md` §6 row "Soak" (24 h nightly, memory of `app` under 384 MiB, no `ERROR` line, no restart, p99 under 500 ms), `soak.memory_ceiling` in §7.11, R30
- evidence:
  ```
  | Soak | Compose stack, nightly CI | The same request mix for 24 h at a constant rate, with a login flood every hour | ...
  ```
  `.github/workflows/` has no soak job. `git ls-files | grep -i soak` is empty.
- failure scenario: a slow leak grows `app` past its 512 MiB limit after some days. The OOM kill restarts the 24/7 service. No test shows the leak before a release.
- D8-4 is merged here. The "2 h versus 24 h" question of D8-4 is not a conflict. P4.2 asks for a 24 h nightly job and a separate 2 h local run. The gap text names both as missing.
- fix direction: add the nightly soak workflow with the four checks of the row.
- status: KNOWN (`README.md:13`, `CHANGELOG.md:63`, `docs/ops/runbook.md:2322`, "Known gaps of 0.6.0"). The gap is accepted, so the severity is MEDIUM.

#### D8-1 ADR index rows omit amendments that the ADR files record

- severity: MEDIUM
- location: `docs/adr/README.md:27`
- rule: `.claude/rules/docs.md` ("An accepted ADR changes only by its Status line and an 'Amended by' note"). The index is the entry point to the ADRs.
- evidence: index row 0015 reads `| Accepted |`. The Status line of `docs/adr/0015-concurrency-locking-model.md` reads "Accepted. Amended by ADR-0096 ...".
- The same gap is at `:34` (0022, ADR-0096), `:46` (0034, ADR-0097), `:47` (0035, ADR-0096), and `:90` (0078, ADR-0095).
- failure scenario: a reader sees 0015 or 0035 as plain "Accepted" in the index. The reader then applies the old isolation and retry text that ADR-0096 changes.
- fix direction: copy the "Amended by" clauses of the five Status lines into their index rows.
- status: CONFIRMED

### LOW

#### D1-1 Part of the forbidden dependency edges has no automatic gate

- severity: LOW
- location: `deny.toml:5`
- rule: `overview.md` §5, column "Forbidden"
- evidence: `# Review checks the other forbidden dependencies, for example `axum` and `tokio`.`
- failure scenario: a change adds `axum` to `recipe-app` or `tokio` to `recipe-domain`. `cargo deny`, clippy, and the tests pass.
- fix direction: add `[bans] deny` entries with `wrappers`, or a test that compares `cargo metadata` with the table of `overview.md` §5.
- status: CONFIRMED. At HEAD every edge is clean.

#### D1-2 The operation catalog is three parallel exhaustive matches of 71 to 81 lines

- severity: LOW
- location: `crates/recipe-app/src/auth/operation/requirement.rs:41`, `crates/recipe-app/src/auth/operation/mod.rs:241`, `crates/recipe-app/src/auth/operation/mod.rs:315`
- rule: `~/.claude/rules/common/code-review.md` ("Functions are focused (<50 lines)")
- evidence: `requirement` has 81 lines, `name` has 71, and `is_command` has 73.
- failure scenario: a new operation needs one arm in each match. An arm in the wrong group of `is_command` lets a new command run in read-only mode (A24). The tests pin the count 34, so they catch a wrong count only.
- fix direction: move the three facts into one `const` table row per variant.
- status: CONFIRMED

#### D1-3 `run_admin_change` holds five steps in 65 lines

- severity: LOW
- location: `crates/recipe-app/src/identity/admin_change.rs:40`
- rule: `~/.claude/rules/common/code-review.md` ("Functions are focused (<50 lines)")
- evidence: `tx.users().lock_administrator_guard().await?;` starts a body that also holds the second permission check, the target lock, three domain checks, the writes, and the audit entry.
- failure scenario: a change of one step needs a read of all five. The code follows the lock order of ADR-0096 (e) today.
- fix direction: extract the target checks (lines 67 to 87) into one function.
- status: CONFIRMED

#### D1-4 `UnlockLogin` returns an error after a successful commit when the `unlock` event fails

- severity: LOW
- location: `crates/recipe-app/src/identity/unlock_login.rs:65`
- rule: `domain-model.md` §5 commit paths, `api.md` §5 (an error says that the command did not complete)
- evidence:
  ```
  let target = self.delete_attempts(auth, meta, login).await?;
  self.ctx.app.security.record(event).await?;
  ```
- failure scenario: `delete_attempts` commits `LoginUnlocked`. The sink write then fails. The Administrator gets an error page, but the login is unlocked. A retry writes a second `LoginUnlocked` entry.
- fix direction: decide in `domain-model.md` §5 whether a failed event after the commit is an error of the command.
- status: PLAUSIBLE. The documents do not state the expected answer.

#### D2-2 `statvfs` in `spawn_blocking` has no time limit

- severity: LOW
- location: `crates/recipe-server/src/disk.rs:24`
- rule: `reliability.md` §2.4 jobs table (`disk_check`), §7.8, ADR-0073 (d)
- evidence: `match tokio::task::spawn_blocking(move || read_free_space(&path)).await {`
- failure scenario: a stalled mount holds `statvfs`. At start, `run_at_start` (`start.rs:284`) waits before the listeners open. During operation, every later run of `disk_check` waits, and the read-only mode does not turn on while the disk fills.
- fix direction: wrap the call in `tokio::time::timeout` and map the elapsed wait to `DiskFault::Unreadable`.
- status: PLAUSIBLE. `pgdata` is normally a local volume.

#### D2-3 Error type shapes in code differ from `reliability.md` §1.1 and §1.2

- severity: LOW
- location: `crates/recipe-app/src/error/app.rs:140`, `crates/recipe-web/src/error.rs:182`, `crates/recipe-web/src/error.rs:212`
- rule: `reliability.md` §1.1 (variant lists), §1.2 (owner column)
- evidence: `AuditWriteFailed { cause: StorageError }`, `failure: Option<Box<AppError>>`, `MissingBasicCredentials`
- failure scenario: §1.1 lists `AuditWriteFailed` and `WebError::Fields` without these fields and does not list `MissingBasicCredentials`. §1.2 has its row `E-WEB-012`. A reader who uses §1.1 as the catalog misses one variant and two fields.
- fix direction: add the fields and the variant to §1.1 and §1.2 through the ADRs that added them.
- status: CONFIRMED

#### D2-4 The build script of `recipe-db` has no `#![forbid(unsafe_code)]`

- severity: LOW
- location: `crates/recipe-db/build.rs:1`
- rule: `reliability.md` §2.1 ("Every crate has `#![forbid(unsafe_code)]`")
- evidence: `//! Build script of `recipe-db`.` with no `forbid` attribute
- failure scenario: a build script compiles as its own crate. A later edit can add an `unsafe` block, and no lint rejects it.
- fix direction: add `unsafe_code = "forbid"` to `[workspace.lints.rust]`, or add the attribute to `build.rs`.
- status: CONFIRMED

#### D3-1 `POST /lang` accepts the pre-session token from a request with a valid session

- severity: LOW
- location: `crates/recipe-web/src/csrf.rs:83`
- rule: `api.md` §7.3 (lines 653 and 654)
- evidence:
  ```
  83:        Access::Lang => TokenKind::Either,
  238:        TokenKind::Either => by_session | by_cookie,
  ```
- failure scenario: a request with a valid session sends `POST /lang` with only `csrf_pre`. The layer accepts it. `api.md` §7.3 asks for the session token. The security effect is nil, because the route sets only the language cookie.
- fix direction: choose the token kind from the session, or record the "either" rule in `api.md` §7.3 through an ADR.
- status: CONFIRMED. Only the archived design `2026-10-14-identity-and-sessions/design.md:240` states the rule.

#### D4-3 Multi-row `DELETE` statements do not lock rows in `id` order

- severity: LOW
- location: `crates/recipe-db/src/maintenance.rs:34`
- rule: `data-model.md` §6 intro ("A statement that locks several rows of one table orders them by `id`")
- evidence: `DELETE FROM session WHERE expires_at < now() OR ...`. The same shape is at `maintenance.rs:40`, `transaction/identity/attempts.rs:28`, and `transaction/identity/sessions.rs:23`.
- failure scenario: the cleanup job deletes expired sessions while `DeactivateUser` deletes all sessions of a user. The two statements lock rows in different orders, and one gets `40P01`. The command fails without a retry.
- fix direction: lock the rows first with `SELECT id ... ORDER BY id FOR UPDATE` in a CTE, then delete by `id`.
- status: PLAUSIBLE. The window is narrow and not reproduced.

#### D4-4 The migration lists of `data-model.md` §8 and `reliability.md` §5 omit 0009 and 0010

- severity: LOW
- location: `docs/architecture/data-model.md:2051`
- rule: `data-model.md` §8 lists the shipped migration files.
- evidence: `- Files of the first release and of ADR-0098:` lists `0001` to `0008` only. `reliability.md:473` also ends at `0008_audit_log_insert_columns.sql`.
- failure scenario: an author does not find `0009` and `0010` and picks a wrong number or misses the two-step validation pattern.
- fix direction: add the rows for `0009` and `0010` with their ADR to both documents.
- status: CONFIRMED

#### D4-5 The Rust-to-SQL role mapping lacks `export_client`

- severity: LOW
- location: `docs/architecture/data-model.md:62`
- rule: `data-model.md` §1.2 maps every Rust name to its SQL name. §3.5 and migration 0009 add `export_client`.
- evidence: `| User.role (Administrator, ProcessEngineer, Operator) | app_user.role (administrator, process_engineer, operator) |`
- failure scenario: an author uses §1.2 as the authority and omits `export_client` from a new `CHECK` or query.
- fix direction: add `ExportClient` and `export_client` to the row.
- status: CONFIRMED

#### D5-2 English base text in a Polish page has no `lang="en"`

- severity: LOW
- location: `crates/recipe-web/templates/components/parameter.html:4`
- rule: WCAG 2.2 SC 3.1.2 (AA, `design-system.md` §1, A44). `templates/parameter-summary.html:12` already marks the same text.
- evidence: `<td>{{ row.name }}</td>`
- failure scenario: with `<html lang="pl">`, a screen reader reads `Line speed` with Polish pronunciation.
- fix direction: add `lang="en"` to the cell when it shows the English base text, as part of the fix of D5-1.
- status: CONFIRMED

#### D5-3 `POST /logout` reads no form

- severity: LOW
- location: `crates/recipe-web/src/pages/own.rs:161`
- rule: `.claude/rules/web.md` (every mutating route accepts only `application/x-www-form-urlencoded` and uses `deny_unknown_fields`), `api.md` §1
- evidence:
  ```rust
  pub(crate) async fn logout(
      State(state): State<WebState>,
      request: PageRequest,
  ) -> Result<Response, WebError> {
  ```
- failure scenario: a client sends `POST /logout` with a JSON body and the header token. The handler ends the session. Every other `POST` handler answers `422 E-WEB-008`. The CSRF check still runs, so the security effect is nil.
- fix direction: take `Result<Form<TokenFields>, FormRejection>` like the archive handlers in `pages/machines/commands.rs:182`.
- status: CONFIRMED

#### D5-4 `E-AUD-003` has no bundle text

- severity: LOW
- location: `crates/recipe-web/src/error/answer.rs:213`
- rule: `api.md` §5.2 (one sentence in the UI language), `api.md` §8, ADR-0012, `reliability.md` §1.2 row `E-AUD-003`
- evidence:
  ```rust
  .message(ctx.lang, &format!("error-{shown}"), Some(args))
  .unwrap_or_else(|| error.to_string())
  ```
  Neither the `pl` nor the `en` bundle has the key `error-E-AUD-003`.
- failure scenario: the trigger raises `E-AUD-003`. A user with `recipe_lang=pl` gets the English sentence "The rule of the database stopped the change.". That sentence is not the registry text and does not tell the user to contact the administrator.
- fix direction: add `error-E-AUD-003` to both bundles and change the test at `src/error/tests.rs:603`.
- status: CONFIRMED

#### D5-5 The deferral of ADR-0041 (g) outlived step P4.2

- severity: LOW
- location: `crates/recipe-web/src/pages/own.rs:117`
- rule: ADR-0041 (g): "The forms of the login, the own password, and the users keep one line per answer until step P4.2."
- evidence:
  ```rust
  let login = TypedLogin::parse(&form.login).map_err(|reason| WebError::BadRequest {
      field: "login",
  ```
- failure scenario: a user sends an empty login and an empty password. The answer lists only `login`. A wrong current password (`E-IDN-008`) marks no input with `aria-invalid`.
- fix direction: bring the two forms under ADR-0041, or amend ADR-0041 (g) to drop "until step P4.2".
- status: PLAUSIBLE. The intent after P4.2 is not written down. P4.2 ran as `2026-10-26-hardening-round-1`.

#### D5-6 `design-system.md` §1 does not assign every component of `components.css`

- severity: LOW
- location: `crates/recipe-web/static/components.css:258`
- rule: `design-system.md` §1 (which rule goes into which file)
- evidence: `/* The multi-line text field and the text that it shows (design-system.md 10.5, ADR-0112) */`. `components.css` also holds `.pager` (10.12), `.diff-*` (10.13), and `pre.json` (10.14), which §1 assigns to no file.
- failure scenario: a contributor follows §1 and puts the next pager rule into `app.css`. Two files then split one component.
- fix direction: extend §1 to 10.12 to 10.14 and the multi-line field.
- status: CONFIRMED

#### D6-7 The coverage gate passes a crate that is missing from the report

- severity: LOW
- location: `tools/coverage-gate.py:93`
- rule: `reliability.md` §6 ("The CI job fails below the target")
- evidence: `if count == 0:` prints "no code" and continues.
- failure scenario: after a filter change, the report holds no file of `recipe-export`. The gate prints "no code" and exits 0.
- fix direction: fail when a crate with `.rs` files under `src/` has no line in the report.
- status: CONFIRMED

#### D6-8 The old-password check of the reset test accepts any error

- severity: LOW
- location: `crates/recipe-db/tests/storage/reset_password.rs:58`
- rule: skill `rust-testing` ("assert the error variant, not only the failure")
- evidence: `assert!(old.is_err(), "the old password must fail the Basic check");`
- failure scenario: the second Basic check fails for another reason, for example a storage error. The test passes and hides a bug that keeps the old hash valid.
- fix direction: match the code of a failed Basic check.
- status: CONFIRMED

#### D7-2 `compose.build.yaml` has no build section for `pitr-volume`

- severity: LOW
- location: `deploy/compose.build.yaml:21`
- rule: line 3 of the same file ("Every service of an image of the project gets the same build section"), ADR-0064 (i)
- evidence:
  ```
  pitr-check:
    build: *pgtools-build
  restore-volume:
    build: *pgtools-build
  ```
- failure scenario: `docker compose -f compose.yaml -f compose.build.yaml build pitr-volume` finds no build section and tries a pull, which fails. The test `deploy_values/overrides.rs:29-32` checks only the allowed keys.
- fix direction: add `pitr-volume: build: *pgtools-build` and make the test require a build section for each project image.
- status: CONFIRMED

#### D7-3 The `pitr-volume` comment gives a positional target time that the script refuses

- severity: LOW
- location: `deploy/compose.yaml:379`
- rule: `deploy/pgtools/bin/pitr-volume:69` and `docs/ops/runbook.md:927-929` (only `--target-time <time>`)
- evidence: `# "docker compose run --rm -v <new volume>:/var/lib/postgresql/data pitr-volume <target time>".`
- failure scenario: an operator copies the command during a recovery. The script stops with its usage line. This costs time inside the recovery time objective of 1 h.
- fix direction: change the comment to the usage line of the script.
- status: CONFIRMED

#### D7-4 `reliability.md` §7.1 counts two one-shot services, but `compose.yaml` has three

- severity: LOW
- location: `docs/architecture/reliability.md:526`
- rule: `reliability.md` §7.7 line 754, ADR-0106 (e)
- evidence: `Two one-shot services are not part of the seven. `pitr-check` has the profile `pitr`, and `restore-volume` has the profile `restore` ...`
- failure scenario: a reader takes the list as complete and misses `pitr-volume` in a review of the Compose layout.
- fix direction: name three one-shot services, with `pitr-volume` (ADR-0106 (e)).
- status: CONFIRMED

#### D7-5 No CI job runs `openspec validate --all`

- severity: LOW
- location: `.github/workflows/ci.yml:401`
- rule: `CLAUDE.md` "Workflow" step 6 and "Commands"
- evidence: `grep -n openspec .github/workflows/*.yml` finds 0 lines.
- failure scenario: a pull request breaks a requirement heading in `openspec/specs/`. CI stays green. The fault shows only at the next local archive.
- fix direction: add a job that installs the pinned `openspec` CLI and runs `openspec validate --all`.
- status: CONFIRMED

#### D7-6 `CONTRIBUTING.md` names two jobs outside pull requests, but four exist

- severity: LOW
- location: `CONTRIBUTING.md:122`
- rule: `.github/workflows/ci.yml:188` and `:283` (`restore-switch` and `pitr-recovery` skip pull requests), ADR-0094 (d)
- evidence: `The jobs `compose` and `upgrade` of `.github/workflows/ci.yml` run only on a push to `main` and on a manual dispatch (ADR-0094).`
- failure scenario: a contributor changes `pitr-volume`, dispatches only `compose` and `upgrade`, and merges without a run of the two recovery jobs.
- fix direction: name all four jobs.
- status: CONFIRMED

#### D7-7 Lost line continuations leave long runs of spaces inside commands

- severity: LOW
- location: `deploy/pgtools/bin/restore-volume:321`
- rule: skill `bash-scripting` (readable structure). The same pattern is at `restore-volume:329`, `:332` and `pitr-volume:187`, `:424`, `:465`.
- evidence: `pg_ctl ... stop >/dev/null     2>"$work_dir/pg_ctl.err"     || fail "the temporary server did not stop" ...`
- failure scenario: the commands run today. A later edit that adds a line break without `\` splits `|| fail` from its command, and the step no longer stops on an error.
- fix direction: restore the `\` line continuations.
- status: CONFIRMED

#### D8-2 Paragraphs over six sentences in normative docs

- severity: LOW
- location: `docs/architecture/data-model.md:1738`
- rule: `.claude/rules/docs.md` ("A paragraph has one topic and at most six sentences")
- evidence: about 45 paragraphs in 8 files. Examples: `data-model.md:1738` (about 18 sentences), `:580` (13), `domain-model.md:598` (13), `overview.md:55` (10), `reliability.md:559` (10), `api.md:758` (10).
- failure scenario: `ste-lint.py` does not count sentences per paragraph, so the gate passes. A reader gets blocks with several topics.
- fix direction: split the listed paragraphs by topic.
- status: CONFIRMED. A sentence splitter gave the counts, so they are approximate.

#### D8-3 `openspec/specs/` is outside the STE gate, and one line has two long sentences

- severity: LOW
- location: `openspec/specs/operations/spec.md:191`
- rule: `.claude/rules/openspec.md` ("Specs follow the ASD-STE100 structural rules"), R40, ADR-0023
- evidence: `The service `backup` SHALL run the restore test every Sunday at 05:00 UTC, on demand before each release, and on demand in steps 5 and 6 of the upgrade procedure.` (29 words, and the next sentence has 28)
- failure scenario: spec text drifts from the STE rules without a gate. It did so once already.
- fix direction: split the two sentences. See open question Q3 for the scope.
- status: CONFIRMED

#### D8-5 `CLAUDE.md` says A1 to A44, but the table has A45

- severity: LOW
- location: `CLAUDE.md:30`
- rule: `.claude/rules/docs.md` (Assumptions: "Use the next free number"), `overview.md` §9
- evidence: `§9 lists the Assumptions A1 to A44.` and `overview.md:284` `| A45 | A technical account with the role ExportClient ...`
- failure scenario: an agent trusts the range, takes A45 as free, and assigns a duplicate number.
- fix direction: change the range, or drop the fixed upper bound.
- status: CONFIRMED

#### D8-6 No document names R32, R33, R41 to R45, and A37 as outside the specs

- severity: LOW
- location: `.claude/rules/openspec.md:7`
- rule: `.claude/rules/openspec.md` (every requirement cites an R-number or an A-number), traceability of `overview.md` §7
- evidence: over 519 requirements, R32, R33, R41, R42, R43, R44, R45, and A37 occur 0 times.
- failure scenario: a reader of the specs cannot tell whether R32 (no panic) is left out on purpose.
- fix direction: name these numbers in one sentence as "enforced by lints and process, not by OpenSpec".
- status: PLAUSIBLE. The gap looks intended, but no record states it.

#### D8-7 Diátaxis types mix in how-to pages and the runbook

- severity: LOW
- location: `docs/user/how-to-export-a-version.md:52`
- rule: `.claude/rules/docs.md` ("Each page is one Diátaxis type")
- evidence: `## The content hash ... The hash does not cover `loadable`, `version.status`, ...` The same mix is in `how-to-compare-two-versions.md:68-103`, `how-to-approve-a-version.md:96-126`, and `docs/ops/runbook.md:2313-2348` (2349 lines).
- failure scenario: the hash field list lives in the how-to and in `export-format.md`. A change to the hash fields needs two edits.
- fix direction: move the reference text to `export-format.md` and link it.
- status: CONFIRMED. The runbook part is a judgment call.

#### D8-8 Two `docs/user/` file names have no type prefix

- severity: LOW
- location: `docs/user/export-format.md:1`
- rule: the naming of `docs/user/README.md` and the other 8 pages (`how-to-*`, `reference-*`, `explanation-*`, `tutorial-*`)
- evidence: `export-format.md` is a reference and `check-export-document.md` is a how-to. Neither has the prefix.
- failure scenario: a reader or a tool that sorts by prefix puts both pages in the wrong group.
- fix direction: rename both files with the prefix and fix the links.
- status: CONFIRMED

## 4. Open questions

Each question names two normative texts that disagree. The review chose no side.

- Q1 (X1, Codex): which statement comes first in the draft commands?
  - `domain-model.md:507` (I19) calls the selection of the recipe the "first statement".
  - `data-model.md:1943` and ADR-0015:47 require a lock of `machine` before it for some commands.
  - The code locks `machine`, then `recipe`, then `recipe_version` (`crates/recipe-app/src/recipe/draft_session.rs:193`).
- Q2 (X3, Codex): does the `must_change_password` gate apply to `POST /login`?
  - `api.md:680` sends a session with `must_change_password` to `/me/password` on every route outside an allowlist, and `/login` is not on it.
  - ADR-0035:78 permits a login with the pre-session token while a session exists.
  - The code ignores the session cookie on the login route (`requirement.rs:115`, `session.rs:35`), so `gates.rs:59` cannot see the flag there.
- Q3 (D8): does the STE gate cover `openspec/specs/`?
  - `overview.md` R40 and `.claude/rules/openspec.md` say that the specs follow the STE structural rules.
  - ADR-0023 defines the gate scope without `openspec/specs/`.
- Q4 (D2-1): `reliability.md` §2.4 rule 6 names six places that catch a panic. The code has two more joins that catch one (`serve/opened.rs:183`, `watchdog.rs:192`). This is a gap between code and one document, not between two documents. The owner can decide whether an ADR adds them.
- Q5 (D1-4, D5-5): two findings depend on intent that no document states. These are the answer of `UnlockLogin` after a failed event, and the end of the deferral of ADR-0041 (g).
- Q6 (D5, note): `routes.rs:304` answers an unknown path with a plain `404` without a code and a `request_id`. ADR-0053 (g) keeps this by decision and says that a later step can change it. The owner can decide whether that step is due.

The earlier question of D8 about the soak figure (2 h versus 24 h) is closed. See D6-3.

## 5. Not covered

- Runtime: the review started no Compose stack and sent no request to a running server. Caddy, header defaults, and the `Host` value were read from `deploy/Caddyfile` only.
- Browser: no axe, keyboard, or screen reader run. The `e2e/` suite did not run.
- Coverage: `cargo llvm-cov` did not run. The percentages of D6-1 are estimates from raw line counts.
- Shell: ShellCheck and shfmt are not installed. D7 read the scripts by hand. `pitr-volume`, `pitr-check`, `restore-test`, the CI scripts of `deploy/ci/`, and `deploy/pgtools/tests/` got a partial read only.
- Runbook: `docs/ops/runbook.md` (2349 lines) got a check of the commands, the secret setup, and the restore and recovery steps only.
- Triggers: D1 checked that each mechanism of I1 to I23 exists. D4 and D6 checked the trigger codes and their tests. Nobody read every trigger body.
- Use cases: the first statement of all 69 use cases was checked. The full body and the lock order were checked for a sample only.
- `data-model.md` §6: the lock sequences were checked for a sample of commands, not for every row.
- Value objects: the top-level parsers were compared with the grammars. The full grammar of `IecLiteral`, `ArrayDimension`, `StringLength`, and the `S7Address` layout rules was not compared. Property tests cover them.
- `recipe-db` live behavior: D4-1 and D4-3 were not reproduced against a live database. The check of D4-1 traced the sqlx 0.8.6 source only.
- Specs against code: 12 of 519 requirements were sampled by hand. Routes, error codes, status codes, and permission names were checked by script.
- Query plans and index use of the read queries were not checked.
- The breached-password list and the Argon2 parameters inside the image were not checked.
