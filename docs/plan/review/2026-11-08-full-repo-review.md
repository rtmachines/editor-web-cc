# Full repository review, 2026-11-08

- Scope: HEAD `3258863` on `main`, 2229 tracked files. The review covers `crates/`, `migrations/`, `deploy/`, `e2e/tests/`, `tools/`, `.github/workflows/`, `openspec/`, `docs/architecture/`, `docs/adr/`, `docs/ops/`, `docs/user/`, `README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, `Cargo.toml`, and `deny.toml`.
- Extra depth: the 120 commits of `3885a26..HEAD`. OCR delegate mode gave 426 reviewable files of 594 changed files in that range.
- Method: baseline gates, a `code-review-graph` map, eight read-only reviewers (D1 to D8), and a Codex second review of D1 and D3.
- No reviewer reported a CRITICAL or HIGH finding. So no adversarial check ran. The orchestrator checked D4-1 on a live PostgreSQL server.
- This file is not normative. It records findings only. The review changed no code, doc, spec, migration, or config.

Dimensions:

| Id | Dimension | Reviewer |
|---|---|---|
| D1 | Architecture, domain, crate dependency rule | Opus, effort default |
| D2 | Rust correctness, errors, panic policy, async | Opus, effort default |
| D3 | Security | Opus, effort default |
| D4 | PostgreSQL, migrations, triggers, locks, grants | Opus, effort default |
| D5 | Web UI, HTMX, i18n, accessibility | Opus, effort default |
| D6 | Tests and coverage | Opus, effort default |
| D7 | Deploy, Compose, CI, operations | Opus, effort default |
| D8 | Docs, specs, traceability, STE | Sonnet, effort default |
| X1, X3 | Second review of D1 and D3 | Codex (gpt-6-astra, effort high) |

## 1. Gate results

| Gate | Command | Result | Evidence |
|---|---|---|---|
| Lint | `cargo clippy --all-targets --all-features -- -D warnings` | pass | Exit code 0 |
| Tests | `cargo test --workspace --all-features --locked` | pass | 34 suites, 8150 passed, 0 failed, 0 ignored, 289 s. The `recipe-db` suites ran on testcontainers. |
| Format | `cargo fmt --check` | pass | Exit code 0 |
| Dependencies | `cargo deny check` | pass | advisories, bans, licenses, and sources ok. Warnings for duplicate crate versions only. |
| Docs | `cargo doc --no-deps` with `-D warnings` | pass on retry | Run 1: rustdoc crashed with `STATUS_ACCESS_VIOLATION` (0xc0000005) on `recipe-server`. Run 2: exit code 0, no warnings. |
| Specs | `openspec validate --all` | pass | 7 passed, 0 failed |
| STE lint | `ste-lint.py` on the scope of ADR-0023 and ADR-0135 | pass | 0 hard violations. 329 advisory items. 356546 words. |
| Coverage | `cargo llvm-cov --workspace` plus `tools/coverage-gate.py` | pass | Lines: `recipe-app` 97.24 %, `recipe-db` 95.19 %, `recipe-domain` 96.18 %, `recipe-export` 99.30 %, `recipe-server` 95.66 %, `recipe-web` 97.98 %. Total 97.61 %. Gate: 80 % lines per crate. |
| Gate tests | `tools/test_coverage_gate.py` | pass | 59 tests OK |
| ShellCheck | `shellcheck` on `deploy/` | not run | The tool is not installed on the review host. |
| Browser suite | `e2e/` Playwright | not run | It needs a password secret and a server. CI job `playwright.yml` owns it. |
| Compose stack | `docker compose ... up` | not run | The review started no dev stack. |

## 2. Counts

Counts are after the merge. The merge found no duplicate across dimensions. Section 5 names the findings of an earlier defect class.

| Dimension | CRITICAL | HIGH | MEDIUM | LOW | Total |
|---|---|---|---|---|---|
| D1 Architecture | 0 | 0 | 1 | 1 | 2 |
| D2 Rust | 0 | 0 | 1 | 1 | 2 |
| D3 Security | 0 | 0 | 1 | 1 | 2 |
| D4 PostgreSQL | 0 | 0 | 1 | 2 | 3 |
| D5 Web UI | 0 | 0 | 3 | 2 | 5 |
| D6 Tests | 0 | 0 | 4 | 3 | 7 |
| D7 Deploy and CI | 0 | 0 | 1 | 3 | 4 |
| D8 Docs and specs | 0 | 0 | 2 | 2 | 4 |
| X1, X3 Codex | 0 | 0 | 0 | 0 | 0 |
| Total | 0 | 0 | 14 | 15 | 29 |

Status: 26 NEW, 3 KNOWN (D2-2, D6-6, D7-3), 0 REGRESSED.

Verdicts:

| Id | Reviewer severity | Check | Verdict | Final severity |
|---|---|---|---|---|
| none | CRITICAL or HIGH | Adversarial Opus check | No candidate | Not applicable |
| D4-1 | MEDIUM | Orchestrator probe on PostgreSQL | `CONFIRMED` | MEDIUM |

The D4-1 probe used `postgres:16.15-bookworm`, the digest of `deploy/compose.yaml:186`, with a table of 1000 rows.
`pg_relation_filenode` stayed 16384 after `ADD COLUMN a plain_int`.
It changed to 16392 after `ADD COLUMN b pos_int`, a domain with a `CHECK` and no `DEFAULT`. So PostgreSQL rewrote the table.

X1 and X3 reported no finding. Their output holds no notes per area, and each run had 3 failed reads. Thus their result is thin evidence of a clean state.

## 3. Findings

The findings are in order of severity. Inside one severity, the order is by risk.

### MEDIUM

#### D2-1 The backup directory scan of the job `alerts` has no time limit

- severity: MEDIUM
- location: `crates/recipe-server/src/backups.rs:58`
- rule: ADR-0133 (time limit on a blocking file system probe), `reliability.md` §3.7 row `backup_age`, §2.4
- evidence:
  ```
  let scan = tokio::task::spawn_blocking(move || newest_manifest_time(&directory)).await;
  ```
  The disk adapter has the limit: `match tokio::time::timeout(self.limit, task).await {` (`crates/recipe-server/src/disk.rs:148`).
- failure scenario:
  1. The backups mount stalls, for example after the loss of an NFS server.
  2. `read_dir` blocks, and `backup_age` never returns.
  3. `AlertCondition::ALL` checks `BackupAge` before `LowDisk` and before the webhook send. So the run never ends.
  4. The supervisor starts no next run. No alert reaches the webhook until a restart. Each stalled run holds one blocking thread.
- fix direction: wrap the join in `tokio::time::timeout` with the disk limit. Map an elapsed wait to `BackupFault::Unreadable`.
- status: NEW. Old D2-2 had the same defect class in the disk adapter. ADR-0133 closed it there.

#### D7-1 A failed weekly restore test or base backup raises no alert

- severity: MEDIUM
- location: `deploy/pgtools/crontab.backup:6`, `deploy/pgtools/crontab.walarchive:4`, alert table of `reliability.md` §3.7
- rule: `reliability.md` §7.7 (the restore test and the base backup find a bad set), ADR-0020 (RTO 1 h), `reliability.md` §7.11 `pitr_volume.start_timeout`, CLAUDE.md (data safety first)
- evidence: `0 5 * * 0 restore-test`. The §3.7 conditions are `panics`, `not_ready`, `backup_age`, `replication`, `low_disk`, `failed_logins`, `security_events`, `audit_chain_broken`, `audit_check_age`, and `export_hash_mismatch`.
- failure scenario:
  1. `restore-test` fails each Sunday, or `basebackup-once` fails on `pg_verifybackup`.
  2. Each run writes one `ERROR` line to the container log only.
  3. No §3.7 condition reads either result. `backup_age` checks only that a complete set exists. `replication` checks only the stream.
  4. At an incident, the newest set may not restore. Or the newest base backup may be older than one week, so the replay can exceed 1800 s and the RTO of 1 h.
- The "Known gaps" list of the runbook does not name this gap.
- fix direction: record each scheduled result where `app` can read it, and add an alert condition through a new ADR. At a minimum, list the gap in "Known gaps" with a weekly manual log check.
- status: NEW

#### D3-1 The per-login throttle limit that ADR-0035 deferred to step P4.2 has no decision

- severity: MEDIUM
- location: `docs/adr/0035-identity-sessions-clarifications.md:71`
- rule: ADR-0035 ("Step P4.2 decides on a per-login limit in a new ADR"), `.claude/rules/docs.md` (a new decision is a new ADR). Step P4.2 is done (`hardening-round-1`, merge `732c25f`). ADR-0096 closed item M7. No ADR closes item M9.
- evidence: `crates/recipe-db/src/login_attempts.rs:30-38`:
  ```
  count(*) FILTER (WHERE login = $1) AS pair_failures, ... count(*) AS address_failures
  FROM login_attempt WHERE client_ip = $2::inet AND NOT succeeded
  ```
  No count covers one login over all addresses.
- failure scenario:
  1. An attacker on the plant network controls several client addresses, for example several hosts or many IPv6 addresses.
  2. The attacker targets one known login, for example an Administrator from `approved_by`.
  3. Each address gets 3 free attempts, then the delay `2^(n-3)`, and its own budget of 100.
  4. The total rate against the account grows with the number of addresses. Only the alert `failed_logins` reacts, and it does not stop the attempts.
- fix direction: write the ADR that P4.2 owed. Add a per-login failure count over all addresses in T1 under the class-1 lock. Or accept the limit with the alert `failed_logins` as the control. In both cases, amend the Status line of ADR-0035.
- status: NEW

#### D6-1 The only test of the production refusal of `session.secure=false` never runs on a pull request

- severity: MEDIUM
- location: `crates/recipe-server/src/config/serve/tests.rs:346`
- rule: ADR-0062 (b), `reliability.md` §7.11 `session.secure`, `CONTRIBUTING.md` (CI runs the same checks on each pull request)
- evidence: `#[cfg(not(feature = "dev"))] #[test] fn serve_secure_false_is_refused_in_a_build_without_the_dev_feature()`. The jobs `test` and `coverage` run `cargo test --workspace --all-features` (`.github/workflows/ci.yml:729`). The step `rust-tests.sh secure` is in the job `compose` only (`ci.yml:146-147`).
- failure scenario:
  1. A pull request inverts the `dev` check of `check_secure`. A release build then accepts `session.secure=false`, and the cookie has no `Secure` flag.
  2. `--all-features` turns on `dev`, so the build of the jobs `test` and `coverage` leaves out this test.
  3. The job `compose` has `if: github.event_name != 'pull_request'` (`ci.yml:42`).
  4. The pull request is green, and the defect lands on `main`.
- fix direction: run `rust-tests.sh secure` in a job that runs on every pull request. It needs no Docker. List it in `CONTRIBUTING.md`.
- status: NEW

#### D4-1 The rewrite check passes `ADD COLUMN` of a domain type with a constraint

- severity: MEDIUM
- location: `deploy/ci/migration_rewrite.py:271`
- rule: `data-model.md` §8 (CI rejects a migration that rewrites a table), `reliability.md` §5, `.claude/rules/sql.md`. The Risks lists of ADR-0127 and ADR-0136 do not name this case.
- evidence: `volatile_default` flags only `serial*`, identity columns, and calls outside the allowlist in a `DEFAULT`. `added_forms` (line 352) reads only inline constraint forms. The check never compares the column type with the tracked set `domains`.
- failure scenario:
  1. A migration holds `CREATE DOMAIN pos_int AS INTEGER CHECK (VALUE > 0);`.
  2. It then holds `ALTER TABLE parameter ADD COLUMN weight pos_int;`.
  3. PostgreSQL rewrites the table under `ACCESS EXCLUSIVE`, because the type is a domain with a constraint. This occurs without a `DEFAULT`.
  4. The checker prints no violation, and the job `migration-rewrite` passes.
- fix direction: reject `ADD COLUMN` with a type that is a domain with a constraint. Or record the case as a known passing form in a new ADR.
- status: NEW. The orchestrator probe of section 2 shows the rewrite.

#### D1-1 `UpdateParameterDefinition` checks the type text after it takes the class-2 locks and inserts the derived type rows

- severity: MEDIUM
- location: `crates/recipe-app/src/recipe/update_definition.rs:165`
- rule: `reliability.md` §7.4 (validation runs before the transaction), CLAUDE.md hard rule, ADR-0143 (i2), ADR-0148 (c)
- evidence:
  ```
  let resolved = resolve_type(open, &input.type_expr, audit_context(user, meta)).await?;
  if resolved.type_id != open.draft.parameter(input.key)?.plc_type() {
      require_current_type(&input.type_expr)?;
      input.type_expr.require_no_nested_array(open.draft.platform())?;
  }
  ```
- failure scenario:
  1. On an `S7_1500` draft, a user changes a parameter type to `ARRAY[1..2] OF ARRAY[1..3] OF INT` or `ARRAY[1..2] OF Int@1`.
  2. `resolve_type` takes the class-2 key `ARRAY`, reads all type rows, and inserts new array rows with `DerivedTypeCreated` entries.
  3. Only then do lines 167 and 170 run the two checks, and the command fails with `E-PLT-013`.
  4. The rollback leaves no data. But the command holds the locks of machine, recipe, version, unit, and the key `ARRAY` during useless inserts.
- `AddParameter` runs the same checks before `resolve_type` (`add_parameter.rs:109`, `:116`).
- fix direction: find out whether the type changes without an insert. Then run both checks before the first `plc_type` insert, as `AddParameter` does.
- status: NEW

#### D8-2 `CHANGELOG.md` `[Unreleased]` is empty, but ADR-0137 to ADR-0149 changed behavior after 0.7.1

- severity: MEDIUM
- location: `CHANGELOG.md:5`. The stale known gap is at `CHANGELOG.md:23`, `:45`, `:83`, `README.md:9`, and `README.md:13`.
- rule: `CONTRIBUTING.md:146` (a pull request needs an entry in `CHANGELOG.md` when behavior changes)
- evidence: the line `## [Unreleased]` comes directly before `## [0.7.1] - 2026-11-06`.
- failure scenario:
  1. The last commit of the changelog is `7f102d1` (0.7.1).
  2. Pull requests #34 to #43 change behavior. Migration `0011_plc_type_rules.sql` can stop on stored rows. The export flag `loadable` can become `false`.
  3. An operator who upgrades from 0.7.1 finds none of this in the changelog.
  4. The known gap "S7 composite rows unverified" is stale. `s7_layout/reference.rs` has 21 marks `Documented(...)`, and only A03 stays `Unverified`.
- fix direction: write the `[Unreleased]` entries for ADR-0137 to ADR-0149, and fix the S7 known gap.
- status: NEW

#### D5-2 After an HTMX approval or a parameter removal, the language switch goes back to the old URL

- severity: MEDIUM
- location: `crates/recipe-web/templates/base.html:5`
- rule: web-ui spec "Language switch" (`next` holds the path of the shown request, R10, A17), ADR-0049 (e)
- evidence: `next: request.path_and_query().map(str::to_owned),` and `{% if let Some(next) = layout.next %}<input type="hidden" id="lang-next" ... hx-swap-oob="true">`
- failure scenario:
  1. A boosted `GET` opens the approve page. `#lang-next` holds `/recipes/R-100/versions/3/approve`.
  2. The approve `POST` answers with the version content and `HX-Push-Url` (`src/pages/approval/approve.rs:200`).
  3. That layout comes from a `POST`, so `next` is `None` (`src/layout.rs:210`, `request.rs:168`). No `#lang-next` goes out of band.
  4. The user clicks "English" and gets `409 E-RCP-007`.
- The parameter removal (`src/pages/parameters/remove.rs:113`) has the same gap and gives `404 E-RCP-051`.
- fix direction: when a `POST` answer sets `HX-Push-Url`, also send `#lang-next` out of band with the pushed path.
- status: NEW

#### D5-1 The submit button of the own password form has no fixed `id`, so a refused answer loses the focus

- severity: MEDIUM
- location: `crates/recipe-web/templates/password.html:12`
- rule: ADR-0041 (e) (each submit button has a fixed `id` so that HTMX keeps the focus), ADR-0132 (a), WCAG 2.4.3
- evidence: `<p class="actions"><button type="submit" class="btn btn-primary">{{ t.get("password-submit") }}</button></p>`
- failure scenario:
  1. A keyboard user activates "Change password" with a bad new password.
  2. The `422` answer swaps `#password-form` out of band (`refuse_password`, `src/pages/own.rs:212`).
  3. HTMX 2 puts the focus back only on an element with an `id`. The focus goes to `<body>`.
- Every other out-of-band refused form has a fixed button `id`.
- fix direction: add `id="password-submit"` and a template test.
- status: NEW

#### D5-3 The added and removed diff sections show `name_en` without the translation and without the marker `(en)`

- severity: MEDIUM
- location: `crates/recipe-web/src/pages/diff_view.rs:631`
- rule: web-ui spec "Translation fallback", ADR-0126 (b), (c), (f), `api.md` §8. ADR-0050 (d) names `name_en` for these sections. The two documents disagree.
- evidence: `DiffPart::Parameter { name_en, .. } => vec![ in_language(Lang::En.as_str(), text_cell("", name_en)),`
- failure scenario:
  1. A Polish Approver opens the approve preview or the diff page.
  2. A new parameter with the translation `Prędkość linii` shows as `Line speed` without a marker.
  3. The parameter table of the same version shows `Prędkość linii`.
  4. A name without a translation shows without `(en)`.
- fix direction: the owner decides which document applies. Then the code uses `shown_name`, or an amendment of ADR-0126 (f) exempts the diff sections.
- status: NEW. Section 4 records the open question.

#### D6-2 The end-to-end smoke test checks the export hash but not the exported parameter or its limits

- severity: MEDIUM
- location: `e2e/compose/end-to-end.spec.ts:206`
- rule: `reliability.md` §6 row "End to end", ADR-0048 (c)
- evidence: lines 203 to 206 assert only `loadable`, the hash format, and the rebuilt hash.
- failure scenario: an export defect drops the parameter `SPEED` or its limits 0 to 100. The server hashes the shorter body. The rebuilt hash matches, and `loadable` is `true`. The test passes.
- fix direction: assert the machine code, the reference, and the version. Also assert the tag `SPEED` with `INT` and 50, and the limits 0 and 100.
- status: NEW

#### D6-3 The XSS corpus never goes through the password fields of `POST /login` and `POST /me/password`

- severity: MEDIUM
- location: `crates/recipe-web/src/pages/xss_tests.rs:211`
- rule: `.claude/rules/web.md` (the HTTP test suite sends an XSS payload corpus through every text field), `reliability.md` §6 row "HTTP"
- evidence: `format!("login={}&password=wrong", ...)` and `PasswordFields { current_password: String, password: String }` (`pages/own.rs:63-65`)
- failure scenario: a change of a template or a field error shows a typed `password` or `current_password` without escape. No test finds it. The login test sends the fixed `password=wrong`. `/me/password` occurs only in the query test of `next`.
- The tests cover the other 35 form structs with `deny_unknown_fields`.
- fix direction: add corpus tests for `password` on `/login`, and for `current_password` and `password` on `/me/password`.
- status: NEW

#### D6-6 The Soak test level has no test and no nightly job

- severity: MEDIUM
- location: `docs/architecture/reliability.md:513`
- rule: `reliability.md` §6 row "Soak"
- evidence: the row text. `README.md:15` says that no soak script and no nightly job exist.
- failure scenario: a slow leak above 384 MiB or a p99 drift above 500 ms over 24 h shows only in production. `audit.yml` is the only scheduled workflow.
- fix direction: keep it as a known gap until the owner schedules the job.
- status: KNOWN. D6-3 of 2026-11-04, owner decision Q-B (3).

#### D8-1 ADR index rows differ from the Status line of their file

- severity: MEDIUM
- location: `docs/adr/README.md:135`, `:136`, `:137`, `:138`, `:139`, `:142`, `:144`, `:145`, `:147` (rows 0123, 0124, 0125, 0126, 0127, 0130, 0132, 0133, 0135). Also `:55` (0043), `:72` (0060), `:110` (0098), `:116` (0104).
- rule: the index convention of `docs/adr/README.md` (the row status equals the Status line of the file and its "Amended by" notes)
- evidence: the row `| [0123](...) | ... | Accepted. Amends ADR-0031 | 2026-11-04 |` against the file line `Status: Accepted` (`docs/adr/0123-no-slow-statement-line-in-the-init-script.md:3`).
- failure scenario:
  1. Row 0123 says "Accepted. Amends ADR-0031", but the file says only `Status: Accepted`.
  2. The rows 0124, 0125, 0126, 0130, 0132, 0133, and 0135 have the same defect.
  3. Row 0127 has a sentence that the Status line of the file does not have.
  4. The rows 0043, 0060, 0098, and 0104 drop a note that the file keeps.
- fix direction: add the note "Amends ADR-X" to the Status line of the nine amending ADR files. Copy the four dropped notes into the rows.
- status: NEW. Old D8-1 and defect 1 of the closure had the same defect class in other rows.

### LOW

#### D1-2 A `TypeExpr` does not record its rule set, so a command parses its text a second time

- severity: LOW
- location: `crates/recipe-app/src/plant/current_type.rs:17`
- rule: `reliability.md` §4.3 (valid by construction), CLAUDE.md hard rule (no second check after construction)
- evidence:
  ```
  pub(crate) fn require_current_type(expr: &TypeExpr) -> Result<(), PlantError> {
      TypeExpr::parse(&expr.to_string()).map(|_| ())
  }
  ```
- failure scenario: `recipe-web` builds every request `TypeExpr` with `RuleSet::Earlier` (`pages/parameters/definition.rs:141`, `pages/parameters/fields.rs:407`). Each write path must call `require_current_type` by hand (`define_udt.rs:101`, `add_parameter.rs:109`, `update_definition.rs:167`). A new write command without the call accepts a reserved struct name such as `Int@1` (ADR-0143 (i2)).
- fix direction: a command input type that only `RuleSet::Current` can build, or a `TypeExpr` that keeps its `RuleSet`.
- status: NEW. Related to D1-1.

#### D2-2 `crates/recipe-db/build.rs` has no `#![forbid(unsafe_code)]`

- severity: LOW
- location: `crates/recipe-db/build.rs:1`
- rule: CLAUDE.md, `.claude/rules/rust.md` (every crate has `#![forbid(unsafe_code)]`)
- evidence: the file has no `#![forbid(unsafe_code)]` attribute.
- failure scenario: an `unsafe` block in the build script passes every gate, because `lib.rs` does not cover the target `build.rs`.
- fix direction: add the attribute after the module doc comment.
- status: KNOWN. D2-4 of 2026-11-04. The fix plan did not cover it.

#### D3-2 `api.md` §7.1 says that the server reads `X-Forwarded-Proto`, but no code reads it

- severity: LOW
- location: `docs/architecture/api.md:653`
- rule: `api.md` §7.1 ("The server reads `X-Forwarded-For` and `X-Forwarded-Proto` only from the trusted-proxy CIDR list")
- evidence: `crates/recipe-web/src/client.rs:16-17` declares only `const X_FORWARDED_FOR: &str = "x-forwarded-for";`. `X-Forwarded-Proto` occurs only at `api.md:653`.
- failure scenario: the code ignores the header from every peer. The CSRF layer compares `Origin` with a fixed `https://` prefix (`crates/recipe-web/src/csrf.rs:46`, `:108`). The doc names a trust input that does not exist. The security effect today is nil, because the code is stricter than the doc.
- fix direction: remove `X-Forwarded-Proto` from `api.md` §7.1, or write an ADR and code that reads it.
- status: NEW

#### D4-2 The security event sink opens its transaction without an isolation level

- severity: LOW
- location: `crates/recipe-db/src/security_events.rs:64`
- rule: `.claude/rules/sql.md` (isolation is `READ COMMITTED`), `data-model.md` §1.1, §6. Every other adapter names the level (`unit_of_work.rs:19`, `maintenance.rs:27`, `login_attempts.rs:130`, `audit_chain.rs:33`, `operations.rs:33`).
- evidence: `let mut tx = self.pool.begin().await.map_err(map_error)?;`
- failure scenario: the database has `default_transaction_isolation = 'repeatable read'` or `'serializable'`. Two concurrent upserts of `login_failed` on one address and minute get `40001`. The code maps it to `E-DB-001`, and the event is lost.
- fix direction: `begin_with("BEGIN ISOLATION LEVEL READ COMMITTED")`.
- status: NEW

#### D4-3 The span reason of an array dimension names 65536 elements, but 65536 elements are valid

- severity: LOW
- location: `crates/recipe-domain/src/plant/array_dimension.rs:35` (doc comment at line 25)
- rule: `domain-model.md` §2.1 (`upper - lower < 65536`). The SQL check `dims_valid` agrees with Rust. Only the text is wrong.
- evidence: `Self::Span => write!(f, "a dimension has {MAX_SPAN} or more elements")` with `if span >= MAX_SPAN` and `span = upper - lower`.
- failure scenario: `ARRAY[0..65535] OF BYTE` has 65536 elements and is valid. `ARRAY[0..65536]` fails with "a dimension has 65536 or more elements". A user can then make a valid array smaller than necessary.
- fix direction: say "more than 65536 elements".
- status: NEW

#### D5-4 The English base text inputs of the parameter form have no `lang="en"`

- severity: LOW
- location: `crates/recipe-web/templates/parameter-form.html:13` (also `:14`)
- rule: ADR-0126 (f), WCAG 3.1.2. `parameter-summary.html:12` and `translations.html:5` already set `lang="en"`.
- evidence: `{{ fields::named(t, "name_en", ..., form.shown.texts.name_en, form.errors.lines("name_en")) }}` without a language argument.
- failure scenario: on a Polish page, a screen reader reads `name_en` and `description_en` with Polish pronunciation.
- fix direction: pass `"en"` as the language argument of both macros.
- status: NEW. Old D5-2 had the same defect class in another template.

#### D5-5 Stale `expect(dead_code)` reasons keep dead production code and hide new dead items

- severity: LOW
- location: `crates/recipe-web/src/error/lists.rs:290` (also `:305`, `src/pages/parameters/grid.rs:8-11`, `src/pages/versions/parameter_table.rs:358`)
- rule: `.claude/rules/rust.md` (review of lint attributes), `reliability.md` §2, YAGNI
- evidence: `#![cfg_attr(not(test), expect(dead_code, reason = "the parameter handlers of wave 5 call it"))]`
- failure scenario: `draft_lines` and `DraftFault` (`lists.rs:288-306`) have no caller outside `error/draft_lines_tests.rs`. `grid.rs` has a module-wide expectation "wave 5 call it", but five modules use it. Clippy passes, so each expectation is fulfilled. Thus some production code is dead, and new dead items there stay silent.
- fix direction: remove the expectations. Then remove each item that the compiler reports as dead, or move it to the tests.
- status: NEW

#### D6-4 A browser regression test uses a fixed sleep

- severity: LOW
- location: `e2e/tests/leaf-grid.spec.ts:156`
- rule: ADR-0048 Risks ("no fixed sleep")
- evidence: `await page.waitForTimeout(NO_REQUEST_WINDOW_MS); expect(requests, ...).toEqual([]);`
- failure scenario: on a slow runner, a request after `NO_REQUEST_WINDOW_MS` stays unseen. So the check "the link sends no request" passes falsely.
- fix direction: assert that the link has no `hx-*` attribute and no boosted ancestor. Or record the deviation in ADR-0048.
- status: NEW

#### D6-5 `constraint_names_match_the_documents` compares each constant with a copy of itself

- severity: LOW
- location: `crates/recipe-app/src/error/mapping/tests.rs:125`
- rule: the test name contradicts its body, `reliability.md` §6 row "Integration, storage"
- evidence: `for (constant, text) in names { assert_eq!(constant, text); }`
- failure scenario: a later migration renames `plc_type_member_name_unique` or `plc_type_member_name_ci_unique`. The test still passes, because it reads no document and no schema. These two constants have no contract test in `recipe-db`. The other nine have one. A real `23505` then maps to `E-DB-006` instead of `E-PLT-021`.
- fix direction: in `recipe-db`, compare each exported constant with `pg_constraint` and `pg_indexes`. Or rename the test.
- status: NEW

#### D6-7 The `SET ROLE` refusal test accepts any error

- severity: LOW
- location: `crates/recipe-db/tests/storage/grants/checks.rs:268`
- rule: `data-model.md` §5.2. The test must assert the code when the code is the point.
- evidence: `assert!(result.is_err(), "recipe_app switched to {role}");`
- failure scenario: a renamed role or a typo gives `42704`. The test still passes, so it no longer shows the missing membership (`42501`).
- fix direction: assert SQLSTATE `42501`.
- status: NEW. Old D6-8 had the same defect class in the reset test.

#### D7-2 The CI concurrency comment promises a complete result for every push to `main`, but GitHub cancels pending runs

- severity: LOW
- location: `.github/workflows/ci.yml:19`, `.github/workflows/playwright.yml:11`
- rule: GitHub concurrency semantics (a new run replaces a pending run of the same group, whatever the value of `cancel-in-progress`). ADR-0128 (d) (a red `load` on `main` blocks the next release).
- evidence: `group: ci-${{ github.ref }}`, `# A push to main always gets a complete CI result.`, `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`
- failure scenario: three pushes reach `main` while a stack run goes on. The third push cancels the pending run of the second. The second commit never gets a result of `compose`, `load`, `upgrade`, `restore-switch`, or `pitr-recovery`. A `workflow_dispatch` on `main` shares the group.
- fix direction: add `github.sha` to the group for events other than pull requests. Or fix the comment.
- status: NEW

#### D7-3 Lost line continuations remain in three shell files

- severity: LOW
- location: `deploy/pgtools/lib/restore-steps.sh:337`, `deploy/pgtools/lib/wal-identity.sh:64`, `deploy/ci/switch-steps.sh:240`. A literal newline in a `printf` format is at `restore-steps.sh:356` and `pitr-volume:422`.
- rule: skill `bash-scripting`, old finding D7-7
- evidence: `... >"$want"     || fail "cannot read restore-roles.expected"`
- failure scenario: the commands work today. A later edit wraps one of these lines without `\`. Then `|| fail` separates from its command. A failed `sed` of `restore-roles.expected` or a failed timeline read no longer stops the restore or the WAL guard.
- fix direction: put back the `\` continuations and the `\n` escapes.
- status: KNOWN. D7-7 of 2026-11-04, partly fixed.

#### D7-4 The nightly npm audit does not read the lock file of the OpenSpec CLI

- severity: LOW
- location: `.github/workflows/audit.yml:45`
- rule: `reliability.md` §8 (nightly dependency audit), ADR-0048 (b). The job `openspec` (`ci.yml:679`) installs `deploy/ci/openspec/package-lock.json` on every pull request.
- evidence: `working-directory: e2e` and `run: npm audit`
- failure scenario: a registry publishes an advisory for a package in that lock file. The job `npm-audit` reads only `e2e/package-lock.json`, so no job reports it.
- fix direction: add a second `npm audit` step with `working-directory: deploy/ci/openspec`.
- status: NEW

#### D8-3 The Purpose of the export spec names "the two export routes"

- severity: LOW
- location: `openspec/specs/export/spec.md:4`
- rule: `api.md` §2.7 (five export routes, ADR-0109)
- evidence: `It covers the two export routes, the content hash, and the audit entry of each export.`
- failure scenario: the Purpose covers two routes, but the spec has requirements for the three transfer routes too (`:426`, `:511`, `:558`, `:687`, `:705`). A reader of the Purpose misses three routes.
- fix direction: name the five export routes in the Purpose.
- status: NEW

#### D8-4 "reject" and "refuse" rotate for one meaning in the same spec file

- severity: LOW
- location: `openspec/specs/identity-access/spec.md:194`, `:715`, `:733`. Also `openspec/specs/export/spec.md:328`, `:446`, `:558`.
- rule: `.claude/rules/docs.md` (one word for one meaning in one file)
- evidence: `:194 The system SHALL reject SetUserRole ...` against `:733 SetUserRole SHALL refuse a change ...`
- failure scenario: the spec "rejects" `SetUserRole` at `:194` and "refuses" it at `:733` with the same meaning. A reader can think that "refuse" has a different meaning.
- fix direction: use "reject" for an error answer. Keep "refuse" for a start or a database guard.
- status: NEW

## 4. Open questions

Each question names a disagreement that the review did not decide.

- Q1 (D5-3): which name do the added and removed diff sections show?
  - ADR-0050 (d) names `name_en`.
  - ADR-0126 (b), (c), (f) and the web-ui spec "Translation fallback" require the translation and the marker `(en)`.
  - The code follows ADR-0050 (d) (`diff_view.rs:631`).
- Q2 (D3-1): does the throttle need a per-login limit over all addresses? ADR-0035 deferred the decision to step P4.2, and no ADR records it.
- Q3 (D3-2): does the server read `X-Forwarded-Proto`? `api.md` §7.1 says yes. The code reads only `X-Forwarded-For`.
- Q4 (D1-1): `reliability.md` §7.4 requires validation before the transaction. `UpdateParameterDefinition` finds out whether the type changed only after `resolve_type`. The owner can decide whether the check moves or an ADR records the order.

## 5. Delta against 2026-11-04

Items of the 2026-11-04 review and of the closure of 2026-11-05 that the reviewers checked again at HEAD `3258863`:

| Old id | Verdict at HEAD | Evidence |
|---|---|---|
| D1-1 | still closed | `deploy/ci/dependency_rule.py`, `ci.yml:767` |
| D1-2 | still closed | one macro table for the operation catalog |
| D1-4 | closed | ADR-0130. The unlock handler logs a lost event `unlock` once at `WARN`. |
| D2-1 | still closed | `race_stop`, exit code 70 |
| D2-2 | closed | `disk.rs:148` wraps the `statvfs` join in `tokio::time::timeout` (ADR-0133). New D2-1 is the same class in `backups.rs`. |
| D2-4 | open | new D2-2 (KNOWN) |
| D3-1 | closed | ADR-0131 |
| D4-1 to D4-5 | still closed | D4 checked each in the source. |
| D5-1, D5-2 | still closed | D5 checked both. New D5-4 is the class of D5-2 in another template. |
| D5-3 to D5-6 | closed | D5 checked each in the source. |
| D6-3 | open | new D6-6 (KNOWN, owner decision Q-B (3)) |
| D6-8 | not checked again | New D6-7 is the same class in another test. |
| D7-2 to D7-6 | closed | D7 checked each in the source. |
| D7-7 | partly closed | new D7-3 (KNOWN) |
| D7-1 follow-up | closed | `CHANGELOG.md:66` |
| D8-1 | closed in its rows | New D8-1 is the same class in 13 other rows. |
| D8-3 | closed | ADR-0135 adds `openspec/specs/` to the STE scope. |
| D8-5 | closed | `CLAUDE.md` says "starting at A1". D8 found A1 to A46 without a gap. |
| Closure defect 2 | closed | `migration_rewrite.py:460` |

The reviewers did not check again D1-3, D2-3, D6-8, D8-2, D8-4, D8-6, D8-7, and D8-8.

New since 2026-11-04:

- 26 NEW findings. 11 of them are MEDIUM.
- 18 findings have a location in a file that changed in `3885a26..HEAD`.
- Four NEW findings repeat a closed defect class in another place: D2-1, D5-4, D6-7, and D8-1.
- No finding is REGRESSED. No closed item failed again at its old place.
- The gates improved: the coverage gate ran and passed. The test count went from 7783 to 8150.

## 6. Not covered

- Runtime: the review started no Compose stack and sent no request to a running server.
- Browser: no axe, keyboard, or screen reader run. The `e2e/` suite did not run.
- Shell: ShellCheck is not installed. D7 read the scripts by hand.
- Codex: X1 and X3 left no notes per area. Their result of 0 findings is thin evidence.
- Docs: D8 did not check the lost hedges in depth.
- Live database: the orchestrator ran only the D4-1 probe. D4-2 is a reading of the code.
- Specs against code: the reviewers checked routes, error codes, permission names, and requirement cites by script. They read a sample of the requirements by hand.
- Use cases: a script checked the first statement of all 85 use cases. The reviewers read the lock order of a sample only.
