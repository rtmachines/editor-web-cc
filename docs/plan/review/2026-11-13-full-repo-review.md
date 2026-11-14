# Full repository review: 2026-11-13

This page is a review record. It is not normative. The review changed no code, doc, spec, or test.

## 1. Scope and HEAD

- Repository: `editor-web-cc`, branch `main`.
- HEAD: `448ddd01afdae38e461e557233762781c6b3f929` (merge of PR #61). `git status --porcelain` was empty before and after the review.
- Scope: the whole tree at HEAD, in 12 review units ordered by risk:
  1. `recipe-web`
  2. `recipe-server`
  3. `recipe-db`
  4. `recipe-app`
  5. `recipe-export`
  6. `recipe-domain`
  7. `migrations/`
  8. `deploy/`
  9. Web assets (templates, `static/`, i18n bundles)
  10. `.github/`
  11. `tools/`
  12. `e2e/`
- The code-review-graph was rebuilt first: 1942 files, 26344 nodes, 275449 edges. The agents used it for the module map, hubs, callers, and tests.
- Severity scale: `~/.claude/rules/common/code-review.md`. CRITICAL is a security vulnerability or a data loss risk. HIGH is a bug or a significant quality issue. MEDIUM is a maintainability concern. LOW is style or a minor issue.
- Verdicts: CONFIRMED means that the verifier saw the defect in the code, or the conflict in both documents. PLAUSIBLE means that the code matches the claim, but the failure needs a condition that the verifier did not confirm.

### 1.1 Agents, models, and effort

The orchestrator used 10 agents of its own. Every Claude agent ran at the default effort. Codex ran at the effort `high`.

| # | Agent | Aspect | Model | Effort | Output |
|---|---|---|---|---|---|
| 1 | Gate runner | Preflight gates | Haiku | default | `gates/` |
| 2 | Security reviewer | a. Security | Opus | default | SEC |
| 3 | rust-pr-review A (`recipe-export`, `recipe-app`, `recipe-domain`) | b. Correctness | Opus | default | RPA |
| 4 | rust-pr-review B (`recipe-server`, `recipe-db`, `recipe-web`) | b. Correctness | Opus | default | RPB |
| 5 | Architecture reviewer | c. Architecture | Opus | default | ARC |
| 6 | SQL reviewer | d. SQL | Opus | default | SQL |
| 7 | Deploy and CI reviewer | e. Deploy and CI | Opus | default | DEP |
| 8 | Test-quality reviewer | f. Tests | Sonnet | default | TST |
| 9 | open-code-review `delegate-review` | OCR | Opus | default | OCR |
| 10 | Merge and adversarial verifier | Merge and confirm | Opus | default | F-01 to F-33 |
| – | Codex CLI | g. Second review | gpt-6-astra | high | CDX |
| – | rust-review (Trail of Bits) skill workers 1 to 17, dedup judge, fp judge | a. Security pass over `crates/` | Opus | default | CANCELSAFETY-001 |

The rust-review workers and judges are skill-internal workers. They do not count against the budget of 10 agents. The OCR agent started 8 Opus sub-reviewers of its own. Section 6 records this as a deviation.

## 2. Gate results

The Haiku gate runner ran every gate at HEAD. All seven gates exited with 0.

| Gate | Command | Exit | Result |
|---|---|---|---|
| clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | 0 `warning:` or `error:` lines |
| test | `cargo test --workspace --all-features --locked` | 0 | 9093 passed, 0 failed, 0 ignored |
| fmt | `cargo fmt --check` | 0 | No output |
| deny | `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| llvm-cov | `cargo llvm-cov --workspace` | 0 | TOTAL lines 98.06 %, regions 96.44 %, functions 95.43 % |
| openspec | `openspec validate --all` | 0 | `Totals: 7 passed, 0 failed (7 items)` |
| ste-lint | `tools/ste-lint/ste-lint.py` on the scope of ADR-0023 and ADR-0135 | 0 | `337 violations (0 hard, baseline 0)` |

Line coverage per crate, from the per-file rows of llvm-cov:

| Crate | Lines |
|---|---|
| `recipe-app` | 97.75 % |
| `recipe-db` | 95.30 % |
| `recipe-domain` | 98.95 % |
| `recipe-export` | 99.30 % |
| `recipe-server` | 96.66 % |
| `recipe-web` | 98.46 % |

Every crate is above the gate of 80 % lines of `CONTRIBUTING.md`. Other tool runs:

- The DEP agent ran `bash -n` on every shell file in `deploy/` and `tools/`, with 0 failures.
- `py_compile` on copies of every `.py` file gave 1 `SyntaxWarning` (F-28).
- Codex ran `coverage-gate.py --source-only`, which exited with 0.
- Codex ran reviewdog on the STE output. It exited with 1 for advisory warnings only. The hard STE gate did not fail.

## 3. Coverage matrix

Status values:

- **Full**: the agent read every production file of the unit for its aspect.
- **Partial**: the agent read a part of the unit. The note names the part.
- **Pattern**: a tool-driven pass of bug classes or a grep over the whole unit, without a full read.
- **Skipped**: the unit is in the scope of the aspect, but the agent did not review it.
- **N/A**: the unit is outside the aspect.

| Unit | a. Security (SEC, Opus) | a. rust-review (17 Opus workers) | b. Correctness (RPA/RPB, Opus) | c. Architecture (ARC, Opus) | d. SQL (SQL, Opus) | e. Deploy and CI (DEP, Opus) | f. Tests (TST, Sonnet) | g. Codex (gpt-6-astra, high) | OCR (Opus + 8 Opus) |
|---|---|---|---|---|---|---|---|---|---|
| 1. `recipe-web` | Partial: routes, CSRF, session, login, export guard, headers. `path_values.rs` skimmed | Pattern | Partial: error mapping, pipeline, timeouts, gates, selected pages | Partial: gates, routes, error, logging sites | N/A | N/A | Partial: CSRF table, XSS registry, test grep | Partial: middleware, CSRF, sessions, export, selected forms | Full: production `src/` |
| 2. `recipe-server` | Partial: listener, panic hook, config secrets | Pattern | Partial: serve, listener, watchdog, supervisor, config, create-admin. `migrate` and `healthcheck` thin | Partial: watchdog, supervisor, listener, main | N/A | N/A | Partial: test grep, healthcheck tests | Partial: config, startup, listener, probes, commands | Full: production `src/` |
| 3. `recipe-db` | Partial: session store, login throttle | Pattern | Partial: 19 files. `machines.rs`, `plc_connection.rs`, read adapters skipped | Partial: SQLSTATE map, every lock statement, every `commit()` | Partial: trigger code map, `grants.expected` | N/A | Partial: `tests/storage/{atomic,race,triggers}` | Partial: transactions, audit, sessions, locks | Full: production `src/` |
| 4. `recipe-app` | Partial: permission first in all 73 use cases, identity commands | Pattern | Partial: about 15 % read, scripted confirmations over all `src` | Partial: first statement of every `pub async fn`, transaction openings | N/A | N/A | Partial: permission matrix tests | Partial: permissions, identity, approval, export | Partial: 28 of 235 production files |
| 5. `recipe-export` | Skipped (rust-review covered it) | Pattern | Full: `src/`. v2 schema text skipped | Partial: dependency edges | N/A | N/A | Pattern: test grep | Partial: writers, schema v1, golden tests | Full: `src/` |
| 6. `recipe-domain` | Skipped (rust-review covered it) | Pattern | Partial: about 15 % read (`export/*`, selected parsers) | Partial: `error_codes.rs` against §1.2, value-object constructors | N/A | N/A | Pattern: test grep, proptest | Partial: hash, canonical form, invariants, parsers | Skipped: 177 production files |
| 7. `migrations/` | N/A | N/A | N/A | Partial: docs comparison | Full: 15 files | Partial: migration CI jobs | Partial: trigger tests in `recipe-db` | Full: 15 files | Full |
| 8. `deploy/` | Partial: Compose exposure, Caddy | N/A | N/A | Partial: numeric settings in Compose and Caddy | Partial: `01-roles.sh`, grants | Partial: most of `pitr-volume`, `pitr-check`, CI harness scripts skipped | Skipped: `deploy/pgtools/tests`, `deploy/ci/test_*.py` | Partial: Compose, Caddy, roles, backup, restore guards | Partial: `deploy/ci`, `deploy/pgtools/tests` skipped |
| 9. Web assets | Partial: templates and static JS (filter `safe`, `hx-*`, inline code) | N/A | N/A | N/A | N/A | N/A | N/A | Partial: base and selected templates, scripts | Full: templates, static JS, i18n |
| 10. `.github/` | N/A | N/A | N/A | N/A | N/A | Partial: main CI jobs read, load, upgrade, and PITR jobs by grep | N/A | Partial: triggers, permissions, gates | Full: workflows |
| 11. `tools/` | N/A | N/A | N/A | N/A | N/A | Partial: coverage gate, `ste-lint.py` | N/A | Partial: coverage gate, STE, dependency tests | Partial: 5 files skipped |
| 12. `e2e/` | N/A | N/A | N/A | N/A | N/A | Partial: `playwright.yml` | Partial: all specs by grep, 2 specs read in full | Partial: configs, setup, Compose scenario | Partial: 10 files skipped |

Limits of the coverage:

- No unit had a full read for every aspect. The review is risk-based. It is not exhaustive.
- `recipe-domain` and most of `recipe-app` got only pattern passes and a sample read. Their test coverage is high (section 2), but a semantic read of each command is open (section 7).
- The test-quality agent (TST) finished before the llvm-cov log existed. It did not use the coverage numbers of section 2.
- No agent ran Docker, a backup, a restore, or a browser. The SQL and concurrency claims are static.

## 4. Findings

Totals after the merge and the verification:

| Severity | Total | CONFIRMED | PLAUSIBLE |
|---|---|---|---|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 0 | 0 | 0 |
| MEDIUM | 5 | 5 | 0 |
| LOW | 28 | 20 | 8 |

- The input files had 46 findings and 3 unfiled notes (N-1 to N-3). The verifier merged them into 33 findings and rejected 10 input IDs.
- The verifier re-rated each finding with the scale of section 1. Two re-ratings differ from the source:
  - CDX-01 went from HIGH to LOW (F-08).
  - CDX-02 went from MEDIUM to LOW (F-33).
- The `sources` line of each finding names the original IDs, the tool, and the model.

### 4.1 MEDIUM

#### F-01: A read after a commit can fail, so the user sees an error for a stored change
- severity: MEDIUM
- verdict: CONFIRMED
- locations: `crates/recipe-web/src/pages/parameters/remove.rs:100-118` (`version_content`), crates/recipe-web/src/pages/approval/approve.rs:196-204, crates/recipe-web/src/pages/versions/actions.rs:330-338 (`retire`), crates/recipe-web/src/pages/units/add.rs:226-255 (`tree_answer`)
- rule: api.md §4 ("A success sets `HX-Trigger: {"rowVersion": <new>}`"), ADR-0041 (h) (a success answer shows the saved state), CLAUDE.md purpose (data safety first). The crate states the rule itself at pages/users.rs:233-237 ("After a commit ... a second try of it is harmless").
- failure scenario: the command commits. Then `version(...)`, `version_answer(...)`, or `GetUnitTree` / `GetUnitForm` runs in a new transaction and fails (pool timeout `E-DB-004`, statement timeout, a concurrent `ArchiveMachine` that gives 404). The handler returns `WebError`. The client gets an error fragment and no `HX-Trigger` with the new `row_version`. After a parameter deletion the next save gets `409 E-WEB-001`. After `AddUnit` the form keeps the segment, and a second submit gets a duplicate unit path. After `ApproveVersion` or `RetireVersion` the user sees a failure for a version that changed state.
- sources: RPB-02 (rust-pr-review, Opus), OCR-03 (OCR delegate, Opus), N-3 (RPB agent note)
- verification note: `remove.rs` runs `version(State(state), values, query, request).await?` before `with_trigger`. approve.rs:200 and actions.rs:335 run `version_answer(...).await?` after the command, and `with_row_version` comes after it. add.rs:226-245 runs two queries after `AddUnit` commits. `discard` (actions.rs:354-362) answers `303` with no read, so it is not affected. No ADR accepts an error answer after a commit. No data is lost, so the rating is MEDIUM.

#### F-02: `/approved` export answers `404 E-EXP-001` while an Approved version exists, after a concurrent approval
- severity: MEDIUM
- verdict: CONFIRMED
- locations: crates/recipe-app/src/export/flow.rs:46-73 (`pick`), crates/recipe-app/src/export/flow.rs:79-95 (`check_exportable`), crates/recipe-app/src/export/flow.rs:207-220 (`lock_and_check`)
- rule: ADR-0019 and data-model.md §6 row `ExportVersion` ("No approved version → `NoApprovedVersion` (404)"), api.md §2.7 polling rule. ADR-0109 (ah) has the sentence "An approval can occur between the list and the export"
- failure scenario: v3 is Approved and v4 is a draft. Step 2 of `GET /export/v1/recipes/R-100/approved` picks v3. `ApproveVersion` of v4 commits and retires v3. Step 3 locks v3 `FOR SHARE`, reads `Retired`, and `check_exportable(Retired, Target::Approved)` gives `NoApprovedVersion`. The transfer client gets 404 although v4 is Approved. The Basic route and the v2 unit route share the flow.
- sources: RPA-01 (rust-pr-review, Opus)
- verification note: flow.rs:88 maps every non-Approved state on `Target::Approved` to `NoApprovedVersion`. The comment at flow.rs:77-78 names the race but does not re-pick the new approved version. api.md §2.7 expects approvals during polling, so a false "no approved version" contradicts the documented 404 meaning. The answer is transient, so MEDIUM.

#### F-03: The registry test of reliability.md §1.2 ("a test in `recipe-server` collects the `code()` of every variant") does not exist
- severity: MEDIUM
- verdict: CONFIRMED
- locations: docs/architecture/reliability.md:50, crates/recipe-server (no use of `ErrorCode`), crates/recipe-domain/src/error_codes/tests.rs:56
- rule: reliability.md §1.2 registry guard, CLAUDE.md hard rule "Never reuse a code"
- failure scenario: a new variant maps to an existing constant, for example a second `=> ErrorCode::PLT_020`. No test collects the codes of all variants, so a reused code passes CI. The `ErrorCode` newtype stops only an unregistered code.
- sources: ARC-01 (architecture, Opus)
- verification note: `rg ErrorCode crates/recipe-server` prints nothing. The only uniqueness test is `registry_is_sorted_and_unique` (error_codes/tests.rs:56), which tests the table and not the variants. The document states a guard that the code does not have.

#### F-04: `Uuid::new_v4` is a panic path in 11 production call sites of recipe-app, although the crate has `random_uuid`
- severity: MEDIUM
- verdict: CONFIRMED
- locations: crates/recipe-app/src/recipe/add_parameter.rs:121-122, crates/recipe-app/src/recipe/create_version.rs:150, :371, crates/recipe-app/src/recipe/create_recipe.rs:130-131, crates/recipe-app/src/plant/define_udt.rs:175, crates/recipe-app/src/plant/create_machine.rs:79-80, crates/recipe-app/src/plant/add_unit.rs:96, crates/recipe-app/src/plant/ensure_derived.rs:234. Compare crates/recipe-app/src/identity/issue.rs:62-66
- rule: CLAUDE.md hard rule "No panic paths" (reliability.md §2)
- failure scenario: the system gives no random bytes (seccomp profile, entropy fault). `Uuid::new_v4()` panics inside the use case, in four sites inside an open transaction. The inner `CatchPanicLayer` answers `E-WEB-010` and not the typed `E-WEB-011 TaskFailed { stage: Token }` of the identity commands. The transaction rolls back, so no data is lost.
- sources: OCR-07 (OCR delegate, Opus)
- verification note: `rg 'Uuid::new_v4\(\)'` outside tests and memory lists exactly these 11 sites. issue.rs:64-65 documents "`Uuid::new_v4` panics when the operating system gives no random bytes" and wraps it for identity only. The trigger is improbable, but the hard rule and the crate's own pattern disagree with these sites.

#### F-05: The pager computation exists in four copies, and the users copy writes `q` into the URL without encoding
- severity: MEDIUM
- verdict: CONFIRMED
- locations: crates/recipe-web/src/pages/machines/list.rs:154-172, crates/recipe-web/src/pages/recipes.rs:103-120, crates/recipe-web/src/pages/types/usage.rs:94-112, crates/recipe-web/src/pages/users.rs:144-151, :171-189
- rule: coding-style.md DRY. pages/render.rs:100-106 requires `page_url` for list links.
- failure scenario: a later change widens the alphabet of the users search (for example to the `TypedLogin` grammar). `/users?q={q}&page=` then breaks for `&`, `#`, `+`, or `%`, while the other lists stay valid. A correction of the "after the end" rule must be made four times.
- sources: OCR-04 (OCR delegate, Opus)
- verification note: a diff of machines/list.rs:154-172 and recipes.rs:103-120 shows only a `path` parameter difference. recipes.rs already has a `pub(crate)` form. users.rs:145-150 formats `q` raw. Today users/query.rs:28-31 limits `q` to `[a-z0-9._-]`, so the defect is latent.

### 4.2 LOW

#### F-06: The one-time password of `CreateUser` or `ResetPassword` can be lost after the commit
- severity: LOW
- verdict: PLAUSIBLE
- locations: crates/recipe-web/src/timeout.rs:44-47, crates/recipe-app/src/identity/reset_password.rs:105, crates/recipe-web/src/pages/users.rs:401-416, crates/recipe-web/templates/user-new.html:6
- rule: ADR-0014 (the one-time password appears once), comment users.rs:401 ("an error after the commit must never hide the one-time password"), web.md `hx-sync="closest form:drop"` (ADR-0049) by analogy
- failure scenario: (a) `request_timeout` expires while `uow.commit(...).await` waits for the COMMIT reply, for example under a WAL flush stall. The server commits, the future drops, and the client gets `503 E-WEB-009` without the password. (b) The create-user form is a plain `POST /users` with no `hx-boost` and no `hx-sync`. A double submit creates the user in the first request and shows the duplicate-login refusal of the second. In both cases the administrator must run `ResetPassword` again.
- sources: RPB-03 (rust-pr-review, Opus), OCR-08 (OCR delegate, Opus)
- verification note: the last await of `ResetPassword` after the write is the commit at reset_password.rs:105. The rest of the handler is synchronous, so window (a) is one COMMIT round trip. user-new.html:6 has no HTMX attribute and base.html:21 and :40 have no boosted ancestor. api.md:278 defines a full `200` page for `POST /users`, so the plain form is a designed path. No data is lost and the recovery is one reset, so LOW.

#### F-07: A dropped request future loses a security event after a committed step
- severity: LOW
- verdict: PLAUSIBLE
- locations: crates/recipe-app/src/identity/unlock_login.rs:79-90, crates/recipe-web/src/pages/users.rs:444-449, crates/recipe-app/src/identity/login.rs:130-141 and :192-193 (`refuse`), crates/recipe-app/src/identity/change_own_password.rs:85-103
- rule: ADR-0130 (a lost `unlock` event leaves one `WARN` line), ADR-0015 and ADR-0022 (security events in their own transaction)
- failure scenario: `UnlockLogin` commits `LoginUnlocked`, then awaits `security.record(unlock)`. A client disconnect or `request_timeout` drops the future: no `unlock` row and no `WARN` line. The same gap exists on the refusal path of `Login` and `ChangeOwnPassword`: T1 commits the `login_attempt` row, and the `login_failed` event is written later in its own transaction.
- sources: CANCELSAFETY-001 (rust-review, Opus workers, fp-judge LIKELY_TP LOW), N-2 (rust-review fp-judge note)
- verification note: unlock_login.rs:79 commits through `delete_attempts`, and :87 awaits `record` with no detached task. For N-2: `Login` T2 success ends with the commit (login.rs:256) and writes no event after it, so the note as stated does not hold. The analogous gap is T1 (committed attempt) followed by `refuse` → `security.record`. The audit log and the attempt rows stay complete, and the window is one short transaction.

#### F-08: A half-empty UDT member row hides the error of its other field
- severity: LOW
- verdict: CONFIRMED
- locations: crates/recipe-web/src/pages/types/define.rs:91-95
- rule: ADR-0041 (a), ADR-0138 (f) stage 1 ("One answer lists every bad field of this stage. Examples are ... the member rows of the UDT form"), web.md "A form answer lists every bad field at once"
- failure scenario: `members[0].name=9bad&members[0].type=`. The `(false, true)` arm returns only `MemberRows` under the type and never runs `MemberName::parse`. The user fixes the type and gets the name error in a second answer. The `(true, false)` arm skips the type test in the same way.
- sources: CDX-01 (Codex CLI, gpt-6-astra, effort high)
- verification note: define.rs:93-94 `return Err(bad(...))` before line 97. The doc comment of `member_input` says `"The row checks both inputs, so it can give two lines (ADR-0041 (a))"`. The name and the type are separate fields with separate lines (`RowField::Name`, `RowField::Type`), so the rule applies to two fields of one row. Impact is one extra round trip and no wrong data, so LOW and not HIGH.

#### F-09: `session.secure=false` is accepted and recorded, but no code applies it
- severity: LOW
- verdict: CONFIRMED
- locations: crates/recipe-server/src/config/serve.rs:335, :369, crates/recipe-app/src/operations/compare_settings.rs:142, crates/recipe-web/src/cookies.rs:48, :59, :76, crates/recipe-web/src/csrf/pre_session.rs:72
- rule: ADR-0062 (b) and reliability.md §7.12 (store the effective value of every key), A34
- failure scenario: a `dev` build starts with `RECIPE_SESSION__SECURE=false`. `CompareSettings` records `session.secure = false` and a `SettingChanged` entry, but every cookie still has `Secure`. A release build can only hold `true`.
- sources: OCR-06 (OCR delegate, Opus)
- verification note: `rg session_secure` outside tests finds only the config, the field, and the compare pair. Every `.secure(` call in recipe-web passes the literal `true`. api.md:713 and ADR-0062 (b) do not say what `false` changes in a `dev` build.

#### F-10: Dead code stays behind `expect(dead_code)` with stale "wave" reasons
- severity: LOW
- verdict: CONFIRMED
- locations: crates/recipe-web/src/error/lists.rs:283-319, crates/recipe-web/src/error.rs:35-42, crates/recipe-web/src/pages/parameters/grid.rs:8-11, crates/recipe-web/src/pages/versions/parameter_table.rs:358-362
- rule: coding-style.md YAGNI. A lint suppression outside tests needs a true reason (spirit of rust.md "A review rejects every other `#[allow]`").
- failure scenario: `DraftFault` and `draft_lines` have no non-test caller and drift from the real `DraftError` mapping while their test keeps passing. The inner attribute of grid.rs covers four files, so a new dead item there gives no warning.
- sources: OCR-01 (OCR delegate, Opus), OCR-02 (OCR delegate, Opus)
- verification note: `rg 'draft_lines|DraftFault'` outside tests lists only the definitions and the re-export at error.rs:42 (CONFIRMED). For OCR-02 the handlers use `ParameterRowFragment` (new.rs:114) and the grid. Without a build I cannot name the dead item that fulfils the expectation, so that part is PLAUSIBLE. The reasons "wave 4" and "wave 5" are stale in both places.

#### F-11: Shared form helpers live in the `recipes` and `users` feature modules
- severity: LOW
- verdict: CONFIRMED
- locations: crates/recipe-web/src/pages/recipes/fields.rs:13 (`FieldLines`), crates/recipe-web/src/pages/recipes.rs:82 (`Choice`), crates/recipe-web/src/pages/users.rs:61 (`TokenFields`)
- rule: coding-style.md (high cohesion, low coupling), module comment pages/machines.rs:4-5 (each area owns its files)
- failure scenario: a change to the recipe form helpers also changes the audit filter, the own-password form, and the PLC connection form.
- sources: OCR-05 (OCR delegate, Opus)
- verification note: 17 non-test files import these three items from `recipes` or `users`. This is a structure issue with no runtime effect.

#### F-12: `PlantErrors` has no `code()`, and CLAUDE.md and reliability.md §1.1 disagree on the exceptions
- severity: LOW
- verdict: CONFIRMED
- locations: crates/recipe-domain/src/plant/errors.rs:14, :94, crates/recipe-domain/src/recipe/error.rs:530-534, crates/recipe-app/src/ports/check_results.rs:79, CLAUDE.md:66, docs/architecture/reliability.md:17, :28
- rule: CLAUDE.md "Every error type has `code()`, except the marker `AcceptLoopEnded`", reliability.md §1.1
- failure scenario: a reviewer that applies CLAUDE.md rejects `PlantErrors` and `CheckResultFault`. A reviewer that applies reliability.md:17 accepts them. reliability.md:17 also says `PlantErrors` owns no code "like `RecipeErrors`", but `RecipeErrors` has `pub fn code()`.
- sources: ARC-02 (architecture, Opus)
- verification note: errors.rs:94 is `impl std::error::Error for PlantErrors {}` with no `code`. recipe/error.rs:532 gives `RecipeErrors::code`. reliability.md:28 says "Every error type has a `code(&self) -> ErrorCode` method", which contradicts line 17 in the same section.

#### F-13: Three value objects have a second public constructor `from_stored`
- severity: LOW
- verdict: CONFIRMED
- locations: crates/recipe-domain/src/plant/type_expr.rs:67, crates/recipe-domain/src/plant/type_name.rs:304, crates/recipe-domain/src/recipe/limit/parse.rs:85, docs/architecture/reliability.md:455, CLAUDE.md:69-70
- rule: reliability.md §4.3 ("Each value object has one constructor `parse`"), CLAUDE.md hard rule (names `TypeExpr`)
- failure scenario: the stored parsers take a rule set (ADR-0120 (i), ADR-0143). No document names `from_stored`, so a reviewer flags valid code, or a new value object copies the second constructor without review.
- sources: ARC-03 (architecture, Opus)
- verification note: the three signatures exist as quoted. `rg from_stored docs` finds nothing. ADR-0120 (i) adds a rule-set parameter but does not amend §4.3.

#### F-14: ADR-0013 has no back reference to ADR-0046, which refines it
- severity: LOW
- verdict: CONFIRMED
- locations: docs/adr/0013-rbac-fixed-permissions.md:3, `docs/adr/0046-permission-check-of-a-form-route.md:51`
- rule: CLAUDE.md "An accepted ADR changes only by its Status line and an 'Amended by' note", docs.md (ADRs)
- failure scenario: a reader of ADR-0013 does not learn that a form route runs its form query first (ADR-0046 (a)). Only the decision map links them.
- sources: ARC-04 (architecture, Opus)
- verification note: ADR-0046:51 "amends ADR-0041 (a) and refines ADR-0013". The Status line of ADR-0013 lists ADR-0022, ADR-0029, and ADR-0109 only. ADR-0041 does list ADR-0046.

#### F-15: `CreateUser`, `ResetPassword`, and `CreateRecipe` open an undocumented read-only transaction before the command transaction
- severity: LOW
- verdict: CONFIRMED
- locations: crates/recipe-app/src/identity/create_user.rs:84, :98, crates/recipe-app/src/identity/reset_password.rs:65, :82, crates/recipe-app/src/recipe/create_recipe.rs:57, :96, docs/architecture/data-model.md:2214, :2230, :2234
- rule: CLAUDE.md "A command runs in one transaction", domain-model.md §5, data-model.md §6 rows
- failure scenario: no data fault (read-only, rechecked by constraints). data-model.md §6 documents the pattern for `SetMachineDataBlock` ("Before the transaction: read ...") but not for these three rows.
- sources: ARC-05 (architecture, Opus)
- verification note: each file opens `Isolation::ReadOnly` and later `Isolation::ReadCommitted`. Rows 2214, 2230, 2234 have no "Before the transaction" read.

#### F-16: The audit guard function uses `search_path = pg_catalog, pg_temp`, but the general rule has no exception
- severity: LOW
- verdict: CONFIRMED
- locations: docs/architecture/data-model.md:22, docs/architecture/data-model.md:1797-1798, .claude/rules/sql.md:14, deploy/db/initdb/01-roles.sh:105-106
- rule: data-model.md §1.1 and sql.md ("Every function sets `search_path = pg_catalog, public, pg_temp`") against data-model.md §5.1
- failure scenario: no runtime defect. A maintainer or a lint that applies §1.1 adds `public` to the event-trigger function and so contradicts §5.1.
- sources: SQL-04 (SQL, Opus), CDX-03 (Codex CLI, gpt-6-astra, effort high)
- verification note: 01-roles.sh:106 and data-model.md:1798 both have `SET search_path = pg_catalog, pg_temp`. data-model.md:22 states the general rule with no exception. The narrower path is the safer one.

#### F-17: The exception table of data-model.md §1.1 omits `plc_type_member` and `draft_editor`
- severity: LOW
- verdict: CONFIRMED
- locations: docs/architecture/data-model.md:13, :19, :29-39, migrations/0001_schema.sql:176-188, :378-383
- rule: data-model.md §1.1 (entity tables have `id UUID` and `created_at`)
- failure scenario: `plc_type_member` has the key `(struct_type_id, ordinal)` and no `id`. `draft_editor` has `(recipe_version_id, user_id)` and `first_edit_at` instead of `created_at`. The table lists `parameter_limit` and `parameter_translation` for the same pattern only.
- sources: SQL-01 (SQL, Opus)
- verification note: both DDL blocks match the quote. The exception table has no row for either table.

#### F-18: data-model.md §1.3 says `login_attempt.login` has no control character, but the DDL of §3.7 tests only the length
- severity: LOW
- verdict: CONFIRMED
- locations: docs/architecture/data-model.md:79, docs/architecture/data-model.md:634, migrations/0001_schema.sql:247
- rule: data-model.md §1.3 against §3.7
- failure scenario: a value such as `a\u0007b` passes the database test. Only the value object refuses it. §1.3 reads as a database limit.
- sources: SQL-02 (SQL, Opus)
- verification note: data-model.md:79 "accepts 1 to 64 with no control character (ADR-0035)". :634 and 0001:247 hold `CHECK (length(login) BETWEEN 1 AND 64)` only.

#### F-19: The table-level INSERT grant on `recipe_version` lets `recipe_app` insert a row as `approved` or `retired`
- severity: LOW
- verdict: PLAUSIBLE
- locations: migrations/0003_grants.sql:34-37, migrations/0002_triggers.sql:219-221, migrations/0001_schema.sql:328, :355-362
- rule: defense in depth for I4 (domain-model.md §3, data-model.md §4.7)
- failure scenario: an application defect or a compromised `recipe_app` session inserts a version with `status = 'approved'`, a forged `content_sha256`, and any `row_version`. No trigger fires on INSERT. The version stays empty, because trigger 4.1 blocks parameter writes.
- sources: SQL-03 (SQL, Opus)
- verification note: 0003:34-37 grants table-level INSERT on `recipe_version`. The transition trigger is `BEFORE UPDATE` only. I4 in domain-model.md:585 covers changes, not inserts, so no normative rule is broken. The current adapter never names `status` on insert, so the path needs a defect.

#### F-20: A panic of a connection task gives two ERROR lines
- severity: LOW
- verdict: CONFIRMED
- locations: crates/recipe-server/src/listener.rs:254-260
- rule: reliability.md §1.1 "Log each error once", ADR-0058 (f) ("the panic hook logs and counts it")
- failure scenario: a panic outside the inner `CatchPanicLayer` gives one hook line and a second line from `Connections::collect`. An alert that counts ERROR lines counts the panic twice.
- sources: RPB-01 (rust-pr-review, Opus)
- verification note: `collect` writes `tracing::error!` on `is_panic()`. ADR-0058:56 gives the logging to the panic hook. The doc comment says "Log a panic once".

#### F-21: `create-admin` reports "cannot read" for a long non-ASCII input that cuts a character at byte 1025
- severity: LOW
- verdict: CONFIRMED
- locations: crates/recipe-server/src/create_admin.rs:62-75
- rule: doc comment of `read_bounded` ("tell exactly the limit from too long")
- failure scenario: `take(1025).read_to_string` fails with invalid UTF-8 before the length test, so the operator gets the wrong message. Nothing unsafe happens.
- sources: RPB-04 (rust-pr-review, Opus)
- verification note: the read error returns through `with_context` before `if length > STDIN_LIMIT_BYTES`.

#### F-22: A cancelled `read_with_limit` leaves a running blocking read without a mark
- severity: LOW
- verdict: PLAUSIBLE
- locations: crates/recipe-server/src/limited_read.rs:151-177
- rule: ADR-0133 (c) (a stalled path gets a mark, so no second blocking read starts)
- failure scenario: the caller future drops before `limit`. No branch calls `mark_if_running`, so the next call starts one more blocking thread on a stalled mount.
- sources: RPB-05 (rust-pr-review, Opus)
- verification note: the mark is set only in the `Err(_elapsed)` arm. The callers are the disk, backup, and result-test adapters (disk.rs:77, backups.rs:90, check_results.rs:137). A drop before the limit needs an abort or an outer timeout.

#### F-23: `is_site_path` accepts a tab, which browsers strip, so `/\t/evil.com` becomes `//evil.com`
- severity: LOW
- verdict: PLAUSIBLE
- locations: crates/recipe-web/src/pages/render.rs:133-139, :64-75. Compare crates/recipe-web/src/lang.rs:40-49
- rule: comment render.rs:58-59 (the guard "stops a later caller that passes text of a user from sending the browser to another site")
- failure scenario: a future caller passes user text to `see_other_path`. A tab passes `is_ascii` and `HeaderValue::from_str`. The browser drops it and follows a scheme-relative URL.
- sources: N-1 (rust-review worker-11 note)
- verification note: every current caller formats value objects (for example compare.rs:52, rebind.rs:329), so no path is reachable today. `same_origin_path` of `POST /lang` already refuses control and white-space characters, so the guard is weaker than its sibling.

#### F-24: `01-roles.sh` does not refuse a NUL byte in a secret file
- severity: LOW
- verdict: CONFIRMED
- locations: deploy/db/initdb/01-roles.sh:39-49. Compare deploy/pgtools/bin/backup-once:140-143
- rule: ADR-0031 (one line per secret file), consistency with the pgtools readers
- failure scenario: Bash drops the NUL in `content=$(cat ...)`, the test passes, and psql sets a password that differs from the bytes that recipe-server reads. The first start fails at the `recipe_app` login, far from the cause.
- sources: DEP-04 (deploy-ci, Opus)
- verification note: `check_secret_file` tests empty, CR, and multi-line only. backup-once counts `\000` and fails.

#### F-25: `db` keeps the passwords of all login roles mounted after the first init
- severity: LOW
- verdict: CONFIRMED
- locations: deploy/compose.yaml:237-242
- rule: least privilege of secrets (ADR-0062)
- failure scenario: a compromise of the `db` process also gives the clear text of `app_password`, `migrator_password`, `backup_password`, and `replicator_password` for reuse outside `db`.
- sources: DEP-05 (deploy-ci, Opus)
- verification note: the five secrets are mounted. I found no document that gives a reason. Only `01-roles.sh` reads them on an empty volume.

#### F-26: The Rust caches of PR runs are saved in the clippy, test, doc, and coverage jobs
- severity: LOW
- verdict: CONFIRMED
- locations: .github/workflows/ci.yml:702, :718, :738, :784. Compare ci.yml:648-651
- rule: consistency with the `save-if` main-only pattern of ci.yml and playwright.yml
- failure scenario: PR saves fill the 10 GB repository cache and evict the caches of `main`.
- sources: DEP-06 (deploy-ci, Opus)
- verification note: the four `rust-cache` steps have no `with:` block. The rust-cache default saves on every event.

#### F-27: No PR job builds with the pinned release toolchain 1.99.1
- severity: LOW
- verdict: PLAUSIBLE
- locations: rust-toolchain.toml:3, deploy/Dockerfile:17, Cargo.toml:13 (`rust-version = "1.99"`)
- rule: CLAUDE.md "Commands" gates, reliability.md §8
- failure scenario: a PR uses syntax that is newer than 1.99. Every PR gate passes on current stable. The image build of the compose job fails after the merge.
- sources: DEP-07 (deploy-ci, Opus)
- verification note: `channel = "stable"` and `ENV RUSTUP_TOOLCHAIN=1.99.1` as quoted. `clippy::incompatible_msrv` covers std APIs, not syntax. The defect needs a PR with such syntax.

#### F-28: An invalid escape `\$` in a CI test string depends on deprecated Python behavior
- severity: LOW
- verdict: CONFIRMED
- locations: deploy/ci/test_pitr_recovery.py:2106 (inside the non-raw string `STEP_DOCKER_STUB` that starts at :2007), `deploy/ci/check.sh:11-13`
- rule: `check.sh` runs every `deploy/ci/test_*.py`
- failure scenario: a Python release that turns the `SyntaxWarning` into an error stops `check.sh` and every CI job that runs it.
- sources: DEP-08 (deploy-ci, Opus)
- verification note: line 2106 holds `"^volume rm $name\$"` inside `"""` without an `r` prefix.

#### F-29: The header of `Caddyfile` names two differences from `Caddyfile.dev`, but there are three
- severity: LOW
- verdict: CONFIRMED
- locations: deploy/Caddyfile:2, deploy/Caddyfile.dev:2
- rule: ADR-0065 (o)
- failure scenario: a maintainer syncs the files by the comment and drops or adds `skip_install_trust`.
- sources: DEP-09 (deploy-ci, Opus)
- verification note: a diff without comments shows the tls line, the `auto_https` line, and `skip_install_trust`. Caddyfile:2 names two.

#### F-30: Healthcheck tests assert wall-clock bounds of a child process
- severity: LOW
- verdict: PLAUSIBLE
- locations: crates/recipe-server/tests/healthcheck.rs:40-43, :206-217, :235-239
- rule: reliability.md §6 (deterministic tests)
- failure scenario: a loaded CI runner delays the child, and the span leaves 2.5 s to 4 s, so the test fails.
- sources: TST-01 (tests, Sonnet)
- verification note: `SPAN_FLOOR = 2500 ms` and `SPAN_CEILING = 4 s` around a 3 s budget. The file comment admits that a loaded host can break the floor. A budget test needs real time, so the risk is flakiness only. Re-rated from MEDIUM to LOW.

#### F-31: A "work did not start" test proves absence with a 50 ms sleep
- severity: LOW
- verdict: PLAUSIBLE
- locations: crates/recipe-app/src/blocking/tests.rs:448-464
- rule: reliability.md §6 (a test fails when the behavior breaks)
- failure scenario: a regression that returns `E-WEB-009` and still starts the closure later than 50 ms passes the test.
- sources: TST-02 (tests, Sonnet)
- verification note: the test asserts the code `WEB_009` and the permit deterministically. Only `!ran` depends on timing. Re-rated from MEDIUM to LOW.

#### F-32: Positive limit-parse tests assert `is_ok()` without the parsed value
- severity: LOW
- verdict: CONFIRMED
- locations: crates/recipe-domain/src/recipe/limit/parse/tests.rs:452-456, :479-483
- rule: reliability.md §4.3 (the parse result is the contract)
- failure scenario: `parse` returns `Ok` with a rounded bound for a 7-digit REAL or a 15-digit LREAL, and the test still passes.
- sources: TST-04 (tests, Sonnet)
- verification note: both tests end in `assert!(limit.is_ok())`.

#### F-33: The limit-change hash test also changes the parameter set and an address
- severity: LOW
- verdict: CONFIRMED
- locations: crates/recipe-domain/src/export/hash/tests.rs:153-161, :25-43
- rule: AGENTS.md:51 (a test of a required state must fail on a plausible bug), api.md §6.1 (`limits[]` is in the hash)
- failure scenario: limits drop out of the hashed body. This test still sees different hashes, because `after` has one parameter and no `s7_address`.
- sources: CDX-02 (Codex CLI, gpt-6-astra, effort high)
- verification note: `before` uses both example parameters, and `after` uses only a new `speed` without `s7_address`. The golden text test of the same file (`expected_canonical_text`, :51-72) pins `"limits":[...]` in the canonical body, so the plausible bug is caught elsewhere. LOW and not MEDIUM.

## 5. Rejected inputs and doc conflicts

### 5.1 Rejected inputs

- SEC-01: the pre-session token with no age follows api.md §7.3 (api.md:743 "no `Max-Age`"), a normative accepted design with no conflict. Hardening only.
- SEC-02: reliability.md §2.4 rule 2 (reliability.md:264) requires the hook to log the panic message. The reviewer found no reachable panic.
- RPB-06: ADR-0069 (d) bounds only the connect. A peer that stops to answer stalls `migrator.run` before `close`, so the close adds no new unbounded wait. The cited rule is not broken.
- DEP-01: ADR-0084 (b) accepts code of the dump that runs as the scratch superuser, and ADR-0095 (h) records the path as a known limit. No code differs from the ADRs. The open follow-up ADR is a governance item for the owner.
- DEP-02: docs/ops/runbook.md:651-654 stops `backup` before `restore-volume` ("no retention deletes the set"). restore-test reads the newest set, which the retention keeps.
- DEP-03: the refusal is fail-closed by design, and docs/ops/runbook.md:2502-2510 already names the cause `another backup run holds the lock` and the action.
- TST-03: `read_webhook_file` returns `anyhow::Result` (config/webhook.rs:128), so no `code()` exists to assert. The test tests its stated purpose: Set fails, Default gives `None`.
- TST-05: no normative document sets a case count. The comment at value/prop.rs:17 records the choice of 64 for large trees.
- TST-06: smoke.spec.ts also asserts `form[action="/login"]` and the h1, so an error page fails the test.
- TST-07: a "no request" proof needs a time window. The wait is the negative test, not a synchronization.
- N-2 as worded (an event after the T2 commit): `Login` T2 writes no event after its commit (login.rs:256). The real variant is merged into F-07.

### 5.2 Doc conflicts

- F-12: CLAUDE.md:66 and reliability.md:28 ("every error type has `code()`") against reliability.md:17 (`PlantErrors` owns no code "like `RecipeErrors`", although `RecipeErrors` has `code()`).
- F-16: data-model.md §1.1 (:22) and .claude/rules/sql.md:14 against data-model.md §5.1 (:1798), the `search_path` of the audit guard.
- F-18: data-model.md §1.3 (:79) against data-model.md §3.7 (:634), the control-character rule of `login_attempt.login`.
- F-13 (partial): reliability.md §4.3 and CLAUDE.md ("one constructor `parse`") against the stored parsers of ADR-0120 (i) and ADR-0143, which the code implements as `from_stored`. The ADRs do not name a second constructor, so this is mainly a code-to-doc gap.
- F-14 (index): ADR-0046:51 refines ADR-0013, but the Status line of ADR-0013 has no note.

## 6. Tools that were missing or failed

The review continued with the other tools after each failure. No tool was installed.

| Tool or step | What happened | Effect |
|---|---|---|
| `ocr scan` (open-code-review) | `ocr llm test` printed "resolve LLM endpoint: no valid LLM endpoint configured". | The user changed the step to `open-code-review:delegate-review`. OCR selected the files and gave the rules. An Opus agent did the review. |
| `ocr delegate preview --from <empty tree>` | It failed with "expected commit type, but the object dereferences to tree type". | The file list of `ocr scan --preview` (1927 files, 525 excluded) replaced it. |
| `ocr` flag `--background-file` | The flag does not exist. Only `-b` / `--background` exists. | The agent used `-b`. |
| OCR agent budget | The OCR agent started 8 Opus sub-reviewers of its own. | This exceeds the budget of 10 own agents. The OCR coverage of 425 of 1927 files (22.1 %) comes from these sub-reviewers. |
| `rust-pr-review` | The skill file `SKILL.md` was not on disk under `~/.claude`. | The agents invoked the skill through the Skill tool. RPA and RPB read only a part of each crate (section 3). |
| rust-review ledger | The Task ledger tools of the skill (`TaskCreate`, `TaskList`) were not available. | The orchestrator tracked the 17 workers in the state file of the scratchpad. |
| rust-review spawn mode | The harness runs every agent asynchronously. The rule "foreground only" of the skill could not be applied. | The sharing of the primer cache between the workers is not certain. |
| rust-review concurrency | The harness printed "Concurrent subagent limit reached. You can run 20 subagents at once." | The orchestrator started workers 7 to 17 in batches when slots became free. All 17 workers completed and passed `validate_artifacts`. |
| rust-review `has_ffi` probe | The probe matched only test sources (`source_tests.rs`). | The orchestrator set `has_ffi=false`. The plan went from 19 to 17 workers. |
| Codex CLI, first run | It stopped at "Reading additional input from stdin..." with 0 % CPU for about 11 minutes. | The orchestrator stopped it and ran it again with stdin closed. The second run exited with 0. |
| rust-review worker 5 | It left an `rg` process that waited on stdin. | The orchestrator stopped the process. The worker output was complete. |
| `shellcheck` | It is not installed on this host. | No agent ran a shell lint. The DEP agent ran `bash -n` only. |

## 7. Open questions

1. **DEP-01 follow-up ADR.** ADR-0084 (b) and ADR-0095 (h) accept that `restore-test` runs dump SQL as the scratch superuser. ADR-0095 (h) asks the owner for a later ADR, and that ADR does not exist. Does the owner want to write it now?
2. **Doc conflicts F-12, F-16, and F-18.** Each one needs a decision about which document wins. A change of `docs/architecture/` needs a new ADR in the same change.
3. **F-02 (`/approved` export race).** Must the flow pick the approved version again after a concurrent approval, or must a doc accept a transient 404? Both answers need an ADR.
4. **F-08 severity.** Codex rated the half-empty UDT member row HIGH. The verifier rated it LOW, because the user loses no data. The owner decides the priority.
5. **Rejected SEC-01.** The pre-session CSRF token has no age, as api.md §7.3 states. Does the owner want a hardening ADR for an age limit?
6. **Coverage gaps.** `recipe-domain` and most of `recipe-app` got only pattern passes and samples (section 3). Does the owner want one more semantic pass over the command use cases?
