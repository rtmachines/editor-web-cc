# Full repository review: 2026-11-21

This page is a review record. It is not normative. The review changed no code, doc, spec, or test.

## 1. Scope and HEAD

- Repository: `editor-web-cc`, branch `main`.
- HEAD: `f6cbac905b32243c61098925bf3c954528d54576` (merge of PR #91). The review ran in a separate detached worktree. `git status --porcelain` was empty before the review. After the review it showed only this record.
- Scope: the whole tree at HEAD, in 13 review units:
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
  13. `docs/` and `openspec/`
- The orchestrator rebuilt the code-review-graph in the worktree first: 2114 files, 28859 nodes, 289057 edges. The agents used it and `rg` for callers, importers, and tests.
- Severity scale: `~/.claude/rules/common/code-review.md`. CRITICAL is a security vulnerability or a data loss risk. HIGH is a bug or a significant quality issue. MEDIUM is a maintainability concern. LOW is style or a minor issue.
- Verdicts: `CONFIRMED` means that the verifier saw the defect in the code, or the conflict in both documents. PLAUSIBLE means that the code matches the claim, but the failure needs a condition that the verifier did not check.
- Status: the verifier compared each finding with six earlier records. They are the 2026-11-13 full review (F-01 to F-33), its fix plan and closure record, and the 2026-11-17 semantic review. They also include the 2026-11-19 type coverage review (R01 to R11) and the 2026-11-20 code review. The values are `new`, `known`, `owner-decided`, and `regression`.
- Threat model of the security passes: remote and local unprivileged (BOTH). Severity filter: all.

### 1.1 Agents, models, and effort

The orchestrator used 15 agents of its own. Section 6 records the difference from the budget of 11. "default" means that the call set no effort.

| # | Agent | Aspect and scope | Model | Effort | Output |
|---|---|---|---|---|---|
| 1 | GAT | Gates (section 2). Also the STE form pass of this record | Sonnet | default | `gates.md` |
| 2 | SEC | Security: auth, sessions, CSRF, CSP, secrets, user input, `deploy/` | Opus | default | SEC |
| 3 | RPA1 | Correctness: `recipe-app` | Opus | default | RPA1 |
| 4 | RPA2 | Correctness: `recipe-export`, `recipe-domain` (export module, part of the rest) | Opus | default | RPA2 |
| 5 | RPA3 | Correctness: `recipe-domain` plant module | Opus | default | RPA3 |
| 6 | RPA4 | Correctness: `recipe-domain` recipe module | Opus | default | RPA4 |
| 7 | RPA5 | Correctness: `recipe-domain` identity, audit, and other modules | Opus | default | RPA5 |
| 8 | RPB | Correctness: `recipe-db`, `recipe-web`, `recipe-server` | Opus | default | RPB |
| 9 | ARC | Architecture: dependency rule, code against ADRs and `docs/architecture/`, panic policy, names | Opus | default | ARC |
| 10 | SQL | `migrations/`, grants, triggers, lock order, transactions | Opus | default | SQL |
| 11 | WEB | Templates, `static/`, HTMX, CSP, i18n, error contract of `api.md` §1 to §9 | Opus | default | WEB |
| 12 | DEP | `deploy/`, `.github/`, `tools/`, shell scripts | Opus | default | DEP |
| 13 | TST | Test quality, tests without assertions, gaps against `reliability.md` §6, `e2e/` | Sonnet | default | TST |
| 14 | OCR | open-code-review in `delegate` mode | Opus | default | OCR |
| 15 | VER | Merge and adversarial verification | Opus | high | V-01 to V-36 |
| – | Codex CLI | Independent second review of the whole repository | gpt-6.2-astra | high | CDX |
| – | rust-review (Trail of Bits) cache primer, workers 1 to 17, dedup judge, fp judge | Bug-class pass over `crates/` | Opus | default | TOCTOU-001 |

The rust-review agents and Codex are skill-internal or external workers. They do not count against the budget. No agent started subagents of its own.

## 2. Gate results

The Sonnet agent GAT ran every gate at HEAD. All seven gates exited with 0.

| Gate | Command | Exit | Result |
|---|---|---|---|
| clippy | `cargo clippy --all-targets --all-features -- -D warnings` | 0 | No warning |
| test | `cargo test --workspace --all-features --locked` | 0 | 9911 passed, 0 failed, 2 ignored. Docker was reachable |
| fmt | `cargo fmt --check` | 0 | No output |
| deny | `cargo deny check` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| llvm-cov | `cargo llvm-cov --workspace` | 0 | TOTAL lines 98.09 %, regions 96.54 %, functions 95.52 % |
| openspec | `openspec validate --all` | 0 | 7 passed, 0 failed |
| ste-lint | `tools/ste-lint/ste-lint.py` on the scope of ADR-0023 and ADR-0135 | 0 | 0 hard violations (baseline 0), 366 advisory |

Line coverage per crate, from the per-file rows of llvm-cov:

| Crate | Lines |
|---|---|
| `recipe-app` | 97.78 % |
| `recipe-db` | 95.38 % |
| `recipe-domain` | 98.87 % |
| `recipe-export` | 99.33 % |
| `recipe-server` | 96.44 % |
| `recipe-web` | 98.57 % |

Every crate is above the gate of 80 % lines of `CONTRIBUTING.md`. The DEP agent also ran `bash -n` on 53 shell files, with 0 syntax errors.

## 3. Coverage matrix

Status values:

- **Full**: the agent read every production file of the unit for its aspect.
- **Partial**: the agent read a part of the unit. The note names the part.
- **Pattern**: a tool-driven pass of bug classes or a scripted search over the whole unit, without a full read.
- **Skipped**: the unit is in the scope of the aspect, but the agent did not review it. The note gives the reason.
- **N/A**: the unit is outside the aspect.

Column keys: Sec = SEC (Opus). RR = rust-review (17 Opus workers). Corr = RPA1 to RPA5 and RPB (Opus). Arc = ARC (Opus). SQL = SQL (Opus). Web = WEB (Opus). Dep = DEP (Opus). Tst = TST (Sonnet). Cdx = Codex (gpt-6.2-astra, high). Ocr = OCR (Opus).

| Unit | Sec | RR | Corr | Arc | SQL | Web | Dep | Tst | Cdx | Ocr |
|---|---|---|---|---|---|---|---|---|---|---|
| 1. `recipe-web` | Partial: security modules in full, templates by pattern, feature pages not read | Pattern | Partial (RPB): all 37 `POST` handlers, error mapping, CSRF, limits, timeouts, gates, session | Pattern | N/A | Partial: render, header, and status code | N/A | Pattern: XSS coverage guard, cookie tests | Partial: parameter forms, auth, CSRF, error mapping | Partial: trust-boundary modules of 320 selected files |
| 2. `recipe-server` | Partial: `listener`, most of `serve`, and `backups` not read | Pattern | Partial (RPB): `serve`, listener, supervisor, watchdog, panic hook in full. `probe.rs` and `create_admin.rs` in part | Pattern | N/A | N/A | N/A | Pattern: registry, watchdog, and signal tests | Partial: config, startup, probes | Partial: listener, webhook, backups |
| 3. `recipe-db` | Skipped (outside the brief: SQL and OCR read the session and identity adapters) | Pattern | Partial (RPB): error, transaction, unit of work, audit, sessions, migrator, settings in full | Pattern | Full for SQL text (191 statements). Partial for the Rust call order | N/A | N/A | Pattern: trigger codes and 159 constraints mapped to tests | Partial: transactions, audit, locks | Partial: 10 of 53 files |
| 4. `recipe-app` | Partial: identity commands | Pattern | Full (RPA1): commands, queries, ports. `memory/` is Pattern, because it is a test double | Pattern | N/A | N/A | N/A | Pattern: permission matrix, real-clock search | Partial: approval, export, identity | Partial: 2 files |
| 5. `recipe-export` | Skipped (no trust boundary of its own. RPA2 and OCR read it in full) | Pattern | Full (RPA2) | Pattern | N/A | N/A | N/A | Partial: hash, writer order, and consumer parts of the golden tests | Partial: writers, schema, golden tests | Full: production `src/` |
| 6. `recipe-domain` | Skipped (no I/O. RPA2 to RPA5 read it in full) | Pattern | Full (RPA2, RPA3, RPA4, RPA5) | Pattern: error codes against §1.2, 52 `parse` types, constants against §7.11 | N/A | N/A | N/A | Pattern: assertion scan of 9290 test functions, 3 proptest files | Partial: hash, canonical form, invariants, parsers | Partial: 1 file |
| 7. `migrations/` | N/A | N/A | N/A | N/A | Full: 18 files, grants, `grants.expected` | N/A | Partial: migration CI jobs | Pattern: trigger tests in `recipe-db` | Partial: grants, triggers | Partial: 6 of 19 files |
| 8. `deploy/` | Partial: Compose, Caddy, roles. Restore and PITR scripts by pattern | N/A | N/A | N/A | Full: `01-roles.sh`, `globals-grammar.sh` | N/A | Full: Compose. Partial: `pgtools`, `deploy/db` (three `check-*.sql` not read) | Skipped (the brief scope is Rust tests and `e2e/`) | Partial: backup, restore, PITR, shell tests | Partial: 3 files. Compose, Caddy, `deploy/db` skipped for budget |
| 9. Web assets | Full: the three own scripts. Templates by pattern | N/A | N/A | N/A | N/A | Full: 81 templates, `static/`, 4 i18n bundles | N/A | N/A | Partial: parameter grid templates | Partial: the three own scripts |
| 10. `.github/` | N/A | N/A | N/A | N/A | N/A | N/A | Partial: `ci.yml` in part, every pin, permission, and trigger by search | N/A | Partial: sample of CI gates | Pattern: pins, permissions, triggers |
| 11. `tools/` | N/A | N/A | N/A | N/A | N/A | N/A | Pattern: coverage gate, `ste-lint.py` exit rule | N/A | Partial: sample | Skipped (budget) |
| 12. `e2e/` | N/A | N/A | N/A | N/A | N/A | N/A | N/A | Partial: 8 specs read, search over all 18 specs | Partial: sample of browser assertions | Skipped (budget) |
| 13. `docs/`, `openspec/` | N/A | N/A | N/A | Partial: scripted tests of §1.2, §2.1, §7.11, all 209 ADR Status lines against the index | N/A | N/A | Partial: `docs/ops/runbook.md` drift | N/A | Partial: rules, decision map, runbook | Skipped (budget) |

Limits of the coverage:

- `recipe-domain` and `recipe-app` have a full correctness read in this review. The 2026-11-13 review gave them only pattern passes and samples.
- RPA2 stopped at its context budget. RPA3, RPA4, and RPA5 read the files that RPA2 did not read, so the units are complete.
- OCR reviewed 58 of the 1087 files that it selected (5.3 %). The other aspects cover the units that OCR skipped.
- No agent ran Docker, a backup, a restore, or a browser. The SQL, shell, and concurrency claims are static. The gate run of section 2 used Docker for the `recipe-db` tests only.

## 4. Findings

Totals after the merge and the verification:

| Severity | Total | `CONFIRMED` | PLAUSIBLE |
|---|---|---|---|
| CRITICAL | 0 | 0 | 0 |
| HIGH | 2 | 2 | 0 |
| MEDIUM | 2 | 1 | 1 |
| LOW | 32 | 30 | 2 |

- The input files had 42 finding IDs. By prefix: CDX 7, SEC 1, RPA1 3, RPA2 3, RPA3 4, RPA4 3, RPA5 2, RPB 1. Others: ARC 5, SQL 1, WEB 2, DEP 1, TST 7, OCR 1, rust-review 1.
- The verifier merged RPA1-01 and RPA1-02 into V-08. It rejected 5 input IDs (section 5.1). Each input ID occurs once.
- Every finding has the status `new`. No finding is `known`, `owner-decided`, or a `regression`. Several findings are new instances of a class that an earlier record named. Their status line names that class.
- The verifier changed the severity of five Codex findings and kept one:
  - CDX-01 went from CRITICAL to HIGH (V-01).
  - CDX-02 went from CRITICAL to MEDIUM (V-02).
  - CDX-03 went from HIGH to LOW (V-03).
  - CDX-05 went from HIGH to LOW (V-05).
  - CDX-07 went from MEDIUM to LOW (V-07).
  - CDX-04 stayed HIGH (V-04).
- The `sources` line of each finding names the original IDs, the tool, and the model.

### 4.1 HIGH

#### V-01: A save of a parameter form silently drops each line feed of a stored `STRING` or `WSTRING` value
- severity: HIGH (was: CRITICAL). An LF reaches a stored value only through a hand-built request. The browser input cannot hold one, and no production path builds `ValueInput::Json`. The diff of ADR-0050 (c) compares the value per leaf, so the change is visible before approval. But `separation_of_duties` is a setting, so a second approver is not guaranteed.
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-web/templates/components/grid.html:9`, `crates/recipe-domain/src/recipe/value/texts.rs:315`, `crates/recipe-domain/src/recipe/value/leaf.rs:219`, `crates/recipe-web/src/pages/parameters/edit.rs:53`
- rule: ADR-0137 (b), (f). Only U+0000 is refused, so LF is a valid character. ADR-0039 (e): string text is kept as sent. ADR-0039 (i): one save sends the value with the definition. ADR-0112 context line 12: an `input type="text"` strips every LF and CR.
- failure scenario: A client posts `value=A%0AB` for a `STRING(20)` leaf. The domain accepts it. A user later opens the edit form and changes only the description. The browser value of the `<input type="text">` is `AB`, the save sends `AB`, and the draft stores a changed PLC value.
- evidence: `texts.rs:315` `(LeafClass::String | LeafClass::Char, Value::String(text)) => Some(text.clone()),` · `grid.html:9` `<input id="{{ cell.id }}" name="{{ cell.name }}" type="text" ... value="{{ cell.text }}"` · `leaf.rs:219` `let has_nul = text.contains('\0');` (the only character check)
- refutation tried: `leaf_text` writes the raw text with no escape. No value path in `crates/recipe-web/src/pages/parameters/` or `form_text.rs` refuses a control character in a value leaf. `ValueInput::Json` has no production producer: outside tests only the match arm in `draft/add/step_content.rs:73` reads it. `e2e/tests/multi-line.spec.ts` covers the description `textarea` only. No test renders a string leaf with LF and saves it. ADR-0112 records the browser rule for descriptions but no ADR covers it for value leaves.
- sources: CDX-01 (Codex gpt-6.2-astra)

#### V-04: The edit form of a parameter near the leaf budget can exceed the fixed 4 MiB body limit. A valid parameter then cannot be saved from the UI
- severity: HIGH (kept). The parameter is valid under every domain budget. The limit is not an operator setting. The UI has no other way to save it.
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-web/src/pages/parameters/grid/walk.rs:374`, `crates/recipe-web/src/pages/parameters/grid/walk.rs:434`, `crates/recipe-web/src/settings.rs:21`, `crates/recipe-web/src/limits.rs:86`, `crates/recipe-server/src/serve/plan.rs:183`, `docs/adr/0039-parameter-value-form.md:70`
- rule: ADR-0039 (g) ("A value inside `budget.value_bytes` then fits in the fields of its leaves"), `reliability.md` §4.4 and §7.11 (16 384 leaves, `budget.value_bytes` 1 MiB, `http.body_limit.value` 4 MiB)
- failure scenario: A struct has four nested members of 64 characters (the `MAX_NAME_LENGTH` of `type_name.rs:10`) that lead to `ARRAY[0..16383] OF INT`. The JSON value is below 40 KiB. Each value field name repeats the 260-character prefix, so the value fields alone exceed 4 194 304 bytes. The save receives `413 E-WEB-003` before validation. With one limit per element, each leaf also sends `limits[i].min` and `limits[i].max`. Each leaf also sends a hidden `limits[i].leaf_path` with the full path again. So shorter names reach the limit too.
- evidence: `name: format!("value{path}"),` · `name: format!("limits[{index}].leaf_path"), value: spot.limit.to_owned(),` · `const DEFAULT_BODY_LIMIT_VALUE: usize = 4 * 1024 * 1024;` · `web: WebSettings::default(),`
- refutation tried: `http.body_limit.value` is not read from the configuration. `recipe-server` builds `WebSettings::default()` (`plan.rs:183`), so an operator cannot raise it. No test posts a form of a parameter with 16 384 leaves (`rg 16_384` over the web tests finds only the rebind bound and message tests). No cap refuses the long path. `LeafPath` has no length limit. `parameter_limit_path_format` (`data-model.md:897`) permits 64-character segments without a total length. `json.max_depth` is 32. The S7 nesting limit does not apply to TwinCAT.
- sources: CDX-04 (Codex gpt-6.2-astra)

### 4.2 MEDIUM

#### V-02: Without a running `db`, `pitr-volume` checks the recovery end only to the segment, so a replay that stops early inside the newest segment reports `pitr volume completed`
- severity: MEDIUM (was: CRITICAL). The script implements ADR-0106 (g) step 2 exactly. The lost transactions come from WAL damage and not from the script. The gap is limited to the tail of the newest segment when the source cluster is gone. The defect is a recovery end that the run calls checked but cannot check.
- verdict: PLAUSIBLE
- status: new
- locations: `deploy/pgtools/bin/pitr-volume:552`, `deploy/pgtools/bin/pitr-volume:553`, `deploy/pgtools/bin/pitr-volume:710`, `docs/ops/runbook.md:1040`, `docs/adr/0106-recovery-of-production-from-the-wal-archive.md:115`
- rule: ADR-0106 (a) (default target is the end of the archive), ADR-0106 (g) step 2 and (i) step 3 (`PITR_EXPECTED_LSN` exists only when `db` runs)
- failure scenario: `db` is lost. A damaged record in the newest archived segment ends replay, because PostgreSQL treats an invalid record as the end of WAL. The end still lies in the newest segment, so check 2 passes. The audit chain prefix passes check 3. The run prints `pitr volume completed`, and the runbook line 1040 says that the script "checks the end of the archive".
- evidence: `[[ ${segment:8:16} > $newest || ${segment:8:16} == "$newest" ]] || fail "$RECOVERY_END_ERROR"` · `[ -n "$PITR_EXPECTED_LSN" ] || return 0`
- refutation tried: The archive has no per-segment hash or manifest (`rg sha256` over `deploy/pgtools/bin/*wal*` and `recovery-steps.sh` finds none). ADR-0106 "Negative" and "Risks" name a gap between segments but not an early end inside the newest segment. `deploy/pgtools/tests/lib/wal-pitr-cases.sh` tests the gap case only. The code follows the ADR, so the defect is in the design and in the runbook claim. The verdict is PLAUSIBLE. The early end depends on the PostgreSQL rule that an invalid record ends replay. No test of this repository shows that rule.
- sources: CDX-02 (Codex gpt-6.2-astra)

#### V-06: `restore-test` (stale runs) and `pitr-check` delete a scratch data directory after a failed stop. A live scratch postmaster can then stay behind a run that reports success
- severity: MEDIUM (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `deploy/pgtools/bin/restore-test:186`, `deploy/pgtools/bin/restore-test:188`, `deploy/pgtools/bin/pitr-check:91`, `deploy/pgtools/bin/pitr-check:92`, `deploy/pgtools/bin/pitr-check:191`
- rule: `reliability.md` §7.7 (a `trap` deletes the scratch cluster on each exit, the cluster uses `scratch.*` of §7.11), ADR-0077 (stop timeout of the scratch server)
- failure scenario: `pg_ctl stop` of a stale scratch run times out. `delete_stale_runs` ignores the status and deletes the directory under the live postmaster. In `pitr-check`, `pitr check passed` is already written, the cleanup ignores the stop status, `rm -r` succeeds, and the exit status stays 0. The leaked postmaster keeps the memory of `container.backup_memory`.
- evidence: `restore-test:186` `stop_server "$dir/data" || true` · `restore-test:188` `remove_dir "$dir" "$BACKUP_DIR/$SCRATCH_NAME" || fail "cannot delete a stale scratch directory"` · `pitr-check:91-92` `pg_ctl --pgdata="$data_dir" --mode=immediate --wait --timeout="$STOP_TIMEOUT_SECONDS" stop >/dev/null 2>&1 || true`
- refutation tried: The current-run path of `restore-test` (lines 127 to 132) sets `stopped=0` and `status=1` on a failed stop, so the script already knows the safe form. `delete_stale_runs` and `pitr-check` do not use it. `pitr-check` runs only with `PITR_CI=1` (line 155), which limits that half to CI. `restore-test` runs weekly in production.
- sources: CDX-06 (Codex gpt-6.2-astra)

### 4.3 LOW

#### V-03: The struct-member trigger trusts the transaction start time as a transaction identity. So a second transaction with the same `now()` could add a member to a committed struct
- severity: LOW (was: HIGH). The only production insert of members runs in the same function right after the struct insert. So the failure needs a writer outside the application plus two transactions that start in the same microsecond. The trigger is a backstop. data-model.md §4.6 states a claim that is not exact.
- verdict: `CONFIRMED`
- status: new
- locations: `migrations/0002_triggers.sql:155`, `migrations/0002_triggers.sql:160`, `migrations/0003_grants.sql:38`, `docs/architecture/data-model.md:1316`, `crates/recipe-db/src/transaction/type_rows.rs:69`
- rule: ADR-0006, `data-model.md` §4.6 ("members can be inserted only in the transaction that created the struct row")
- failure scenario: Transactions A and B start in the same microsecond. A inserts a struct and commits. B, as `recipe_app` under `READ COMMITTED`, inserts a `plc_type_member` row for that struct. `v_created = now()` is true in B, so the trigger accepts the row and the committed revision changes.
- evidence: `SELECT kind, created_at INTO v_kind, v_created FROM plc_type WHERE id = NEW.struct_type_id;` · `IF v_created <> now() THEN` · data-model.md §4.6: "`created_at = now()` is true only inside that transaction."
- refutation tried: No later migration replaces `plc_type_immutable` (`rg` over `migrations/`). The grant excludes `created_at`, so the application cannot forge it. `INSERT_MEMBERS` is the only member insert. Its one caller `insert_struct` (`type_rows.rs:433`) runs `INSERT_STRUCT` and then `INSERT_MEMBERS` on the same connection, so the application cannot trigger the case. `crates/recipe-db/tests/storage/triggers/immutable_types.rs:141` tests a late insert with a different start time only. A check of the row `xmin` against `pg_current_xact_id()` would make the claim exact.
- sources: CDX-03 (Codex gpt-6.2-astra)

#### V-05: `POST .../definition` without value and limit fields for an unchanged type gives `422 E-RCP-005`, although `api.md` makes both optional
- severity: LOW (was: HIGH). The edit form always sends every leaf and the limits (ADR-0039 (i)). So only a hand-built request meets the case. It receives a `422` and stores nothing.
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-web/src/pages/parameters/edit.rs:53`, `crates/recipe-web/src/pages/parameters/prepare.rs:125`, `crates/recipe-web/src/pages/parameters/fields.rs:535`, `crates/recipe-domain/src/recipe/draft/definition.rs:76`, `crates/recipe-domain/src/recipe/value/texts.rs:285`, `docs/architecture/api.md:215`
- rule: `api.md` §2.6 row `POST .../parameters/{key}/definition` ("Otherwise both are optional (ADR-0029)")
- failure scenario: A client sends a new `name_en` with the current `row_version` and the same `type`, and no `value` or `limits[i]` field. `leaf_texts` gives an empty `LeafTexts`, the handler passes `retype: Some(..)`, and `set_definition` parses the content. The first leaf has no text, so the answer is `422 E-RCP-005` and the name stays unchanged.
- evidence: `edit.rs:53` `retype: Some(RetypeInput {` · `definition.rs:76` `let content = match retype { Some(retype) => { ... parse_content(&update.tag, &retype.value, &retype.limits, tree)?;` · `texts.rs:285` `let Some((position, text)) = texts.text_at(path) else { return Err(Expected::Leaf.into()); };`
- refutation tried: `recipe-app` supports `retype: None` (`update_definition/requests.rs:28`), but no `recipe-web` path builds it (`rg retype crates/recipe-web/src` finds only tests of the diff and approval pages). The domain does not skip the parse when the type is unchanged. No web test posts the definition route without value fields.
- sources: CDX-05 (Codex gpt-6.2-astra)

#### V-07: `backup-health` accepts a manifest with a modification time in the future, so the `backup` container can stay healthy after the dumps stop
- severity: LOW (was: MEDIUM). The `backup_age` alert of the job `alerts` treats a future dump time as untrusted and stays active (ADR-0093 (a), `reliability.md` §3.7). So the operator still receives an alert. Only the container health status is wrong.
- verdict: `CONFIRMED`
- status: new
- locations: `deploy/pgtools/bin/backup-health:64`, `deploy/pgtools/bin/backup-health:65`
- rule: `reliability.md` §7.1 and §7.7 (a complete set younger than the maximum age). ADR-0093 (a): a stored time ahead by more than the tolerance is untrusted.
- failure scenario: The host clock is one month ahead when a dump completes. The clock is set back to the true time and the dumps stop. `age` is negative, `-lt "$MAX_AGE_SECONDS"` is true, and the healthcheck prints `backup is healthy` for about one month.
- evidence: `age=$((now - $(stat -c %Y -- "$manifest")))` · `if [ "$age" -lt "$MAX_AGE_SECONDS" ]; then newest_set=$base`
- refutation tried: The script has no lower bound for `age`. ADR-0093 (a) gives the rule for the job `alerts` only, and that job makes `backup_age` active for an untrusted time (ADR-0093 line 66). That reduces the impact but does not refute the wrong health status.
- sources: CDX-07 (Codex gpt-6.2-astra)

#### V-08: Six plant commands and the recipe and draft commands open a `READ ONLY` transaction before the command transaction. But `data-model.md` §6 says that every command runs in exactly one transaction
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new (class of F-15: the 2026-11-13 record and ADR-0168 (a) named only `CreateUser`, `ResetPassword`, and `CreateRecipe`)
- locations: `crates/recipe-app/src/plant/resolve.rs:43`, `crates/recipe-app/src/plant/add_unit.rs:79`, `crates/recipe-app/src/plant/archive_machine.rs:78`, `crates/recipe-app/src/plant/archive_unit.rs:144`, `crates/recipe-app/src/plant/define_udt.rs:102`, `crates/recipe-app/src/plant/rename_machine.rs:66`, `crates/recipe-app/src/plant/rename_unit.rs:68`, `crates/recipe-app/src/recipe/resolve.rs:12`, `crates/recipe-app/src/recipe/resolve.rs:28`, `docs/architecture/data-model.md:2227`
- rule: CLAUDE.md hard rule "A command runs in one transaction", `data-model.md` §6 intro (line 2227) and rows, ADR-0168 (a), (e)
- failure scenario: `AddUnit` calls `live_machine_id`, which opens `Isolation::ReadOnly`, reads `machine_by_code`, and ends. Then the command opens its `READ COMMITTED` transaction. `RenameRecipe`, `DeleteRecipe`, `RestoreRecipe`, `DiscardDraft`, `RetireVersion`, and every draft command do the same through `read_recipe` or `resolve_version`. No data fault follows, because the command transaction locks and checks again, but the §6 rows state one transaction.
- evidence: `resolve.rs:43` `let mut tx = ctx.uow.begin(Isolation::ReadOnly).await?;` · `let machine = tx.plant().machine_by_code(code).await?;` · `data-model.md:2227` "Every command runs in exactly one transaction, except `Login` and `ChangeOwnPassword`"
- refutation tried: `domain-model.md:682` permits a read of `recipe.machine_id` without a lock, but not a second transaction, and not the code-to-identifier read. The §6 rows "Before the transaction" exist only for `SetMachineDataBlock`, `SetMachineStringEncoding`, `SetPlcConnection`, `CreateRecipe`, `CreateUser`, and `ResetPassword` (lines 2256 to 2285). The doc comment of `live_machine_id` cites a design D10, which is not normative.
- sources: RPA1-01 (Opus), RPA1-02 (Opus)

#### V-09: `ListParameters` resolves the type trees of a page while its `REPEATABLE READ` transaction is still open, against ADR-0042 (f)
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-app/src/recipe/queries/parameters.rs:369`, `crates/recipe-app/src/recipe/queries/parameters.rs:375`, `crates/recipe-app/src/recipe/queries/parameters.rs:376`, `crates/recipe-app/src/recipe/queries/parameters.rs:377`
- rule: ADR-0042 (f) ("A query ends the transaction before it resolves a type tree or builds a view"), `reliability.md` §7.4
- failure scenario: A page holds parameters with large struct or array types. `PlatformTypes::collect` and `TypeTree::resolve` run for every used type between two statements of the open transaction. The connection stays idle in the transaction during the CPU work, which counts against `db.idle_in_transaction_timeout` (15 s, §7.11).
- evidence: `let trees = row_trees(&types, &rows);` · `let values = read_row_values(tx.as_mut(), scope.detail.id, &rows).await?;` · `drop(tx);`
- refutation tried: `read_row_values` (line 298) needs only the row keys and not the trees. So the code can read the values first and drop the transaction before the resolve. The sibling queries named by the source drop the transaction first. No test pins the order.
- sources: RPA1-03 (Opus)

#### V-10: The doc comment of `TryFrom<StoredMachine>` lists the unit checks before the data block and encoding rules, but the code runs those two rules first
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-domain/src/plant/machine.rs:439`, `crates/recipe-domain/src/plant/machine.rs:444`
- rule: `.claude/rules/rust.md` (every public item has a doc comment), `reliability.md` §1.1
- failure scenario: A stored live S7 machine has no data block number and two unit rows with one path. The comment gives `E-PLT-004` first. The code returns `E-PLT-050`.
- evidence: comment `"The checks run in this order: duplicate path, missing parent, archived parent of a live unit, no live main unit, and the data block rule"`. Code `check_stored_data_block(&stored)?; check_string_encoding(..)?; let units = index_units(stored.units)?;`
- refutation tried: No ADR fixes the order of these checks, so the comment is the only statement. It is wrong against the code.
- sources: RPA3-01 (Opus)

#### V-11: `check_s7_nesting_of_expr` uses `saturating_add` for a limit check, which the arithmetic rule permits only for a UI counter
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-domain/src/plant/type_tree.rs:269`
- rule: `.claude/rules/rust.md` "Use `saturating_*` only for a counter that the UI shows", `reliability.md` §2
- failure scenario: No wrong result today, because a saturated `u32::MAX` is still above `MAX_S7_NESTING`. The call is the only `saturating_*` in `plant/` production code, against the rule.
- evidence: `let levels = self.s7_nesting.saturating_add(extra);`
- refutation tried: `rg saturating_ crates/recipe-domain/src/plant` without tests gives only this line. The value is not a UI counter.
- sources: RPA3-02 (Opus)

#### V-12: `PlcConnection::for_platform` lists a mode-dependent missing login (`E-PLT-023`) after `E-PLT-025` and `E-PLT-026`, but `domain-model.md` §2.1 puts every `E-PLT-023` at step 2
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-domain/src/plant/connection/shape.rs:262`, `crates/recipe-domain/src/plant/connection/shape.rs:268`, `docs/architecture/domain-model.md:356`
- rule: `domain-model.md` §2.1 numbered check order, ADR-0041
- failure scenario: A `TwinCat3` request with mode `CodesysV3`, no login, and no password gives `[E-PLT-025, E-PLT-023 login, E-PLT-023 password]`, so `PlantErrors::code()` is `E-PLT-025`. The document order gives `E-PLT-023` first.
- evidence: `faults.check_mode(mode);` · `faults.check_ams_net_id(address);` · `faults.check_credentials(mode, fields.login.is_some(), fields.password.is_some(), ..)` (after the mode checks)
- refutation tried: Step 2 of the document says "A required field without a value". The login is required only by the mode, so a reader can place it in step 2 or step 4. The test in `shape/tests.rs:781` pins the code order, so the document does not state where the credential check runs.
- sources: RPA3-03 (Opus)

#### V-13: `recipe-db` and the memory adapter build `PlcConnection` variants directly, but `domain-model.md` §2.1 names `for_platform` as the one constructor
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new (class of F-13: ADR-0166 names `from_stored` on other types and no stored constructor for `PlcConnection`)
- locations: `crates/recipe-domain/src/plant/connection/shape.rs:11`, `crates/recipe-db/src/transaction/plc_connection.rs:216`, `crates/recipe-db/src/transaction/plc_connection.rs:228`, `crates/recipe-app/src/memory/plc_connection.rs:95`, `docs/architecture/domain-model.md:351`
- rule: `domain-model.md` §2.1 ("`PlcConnection::for_platform(machine, platform, fields, stored)` is the one constructor"), `reliability.md` §4.3, ADR-0166 (a)
- failure scenario: The stored read parses each field and builds `PlcConnection::Iec` without the platform rules of `for_platform`. The document says that no such path exists.
- evidence: `Ok(PlcConnection::Iec { host, port: parse_number("port", record.port, TcpPort::parse)?, address: ..., mode: parse_text("comm_mode", ..)?,`
- refutation tried: The source example of a corrupt row is refuted. `migrations/0012_plc_connection.sql:54` (`plc_connection_mode_value`) refuses `Ads` on `CodesysIec`. Line 85 (`plc_connection_ams_net_id`) refuses an `Ads` address that is not an AMS Net ID. The doc conflict stays, because `PlcConnection` is a public enum with public fields and the document names one constructor.
- sources: RPA3-04 (Opus)

#### V-14: A trusted-proxy entry in IPv4-mapped IPv6 form never matches the canonical peer address. So every client behind that proxy receives the proxy address as `client_ip`
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new (not the SEC-01 of the 2026-11-13 record, which was the pre-session token age)
- locations: `crates/recipe-web/src/settings/cidr.rs:65`, `crates/recipe-web/src/settings/cidr.rs:86`, `crates/recipe-web/src/client.rs:70`, `crates/recipe-server/src/config/serve.rs:39`
- rule: `api.md` §7.1 ("The server maps every address to its canonical form first, so `::ffff:a.b.c.d` and `a.b.c.d` are one address")
- failure scenario: The operator sets `http.trusted_proxies = ["::ffff:172.28.0.2/128"]`. `Cidr::parse` keeps an IPv6 network. The peer is canonicalized to IPv4 `172.28.0.2`, and `contains` returns false because the widths differ. `X-Forwarded-For` is ignored, all users share the per-address login budget, and events record the proxy address. The failure is closed for spoofing.
- evidence: `let network: IpAddr = address.parse().map_err(|_bad| CidrReason::BadAddress)?;` · `if width != given_width { return false; }`
- refutation tried: `http.trusted_proxies` is an operator setting (`config/serve.rs:39`, `plan.rs:257`), so the input is reachable. `parse` neither canonicalizes nor refuses a mapped network. The doc comment of `contains` says the caller maps the address, but the network side has no such step.
- sources: SEC-01 (Opus)

#### V-15: A `dev` build with `session.secure=false` served on plain HTTP to a non-loopback host may refuse every mutating request, because the origin check requires `https://`
- severity: LOW (kept)
- verdict: PLAUSIBLE
- status: new (follow-up of F-09, which ADR-0171 closed for the cookies only)
- locations: `crates/recipe-web/src/csrf.rs:46`, `crates/recipe-web/src/csrf.rs:95`, `crates/recipe-web/src/test_support.rs:469`, `docs/adr/0171-cookies-without-secure-in-a-dev-build.md:66`
- rule: ADR-0171 Consequences ("A `dev` build with `false` gets working cookies on plain HTTP"), ADR-0035 (e), `api.md` §7.3
- failure scenario: A `dev` build serves `http://192.0.2.5:8080`. A browser that sends no `Sec-Fetch-Site` header to a URL that is not potentially trustworthy sends `Origin: http://192.0.2.5:8080`. `origin_matches` compares it with `https://` and the host and fails, so `POST /login` receives `403 E-WEB-007`.
- evidence: `const ORIGIN_PREFIX: &str = "https://";` · `!host.is_empty() && origin.strip_prefix(ORIGIN_PREFIX) == Some(host)` · `test_support.rs:469` `.header("sec-fetch-site", "same-origin");`
- refutation tried: The code path is visible. The browser behavior (`Fetch Metadata` only to potentially trustworthy URLs) is not proven by a browser run in this review, so the verdict stays PLAUSIBLE. The dev example binds `127.0.0.1`, which is loopback, so the default dev setup is not affected. ADR-0171 does not amend ADR-0035 (e) or `api.md` §7.3.
- sources: OCR-01 (open-code-review delegate mode, Opus)

#### V-16: `SessionToken` and `CsrfToken` have a second public constructor `from_bytes` that no document permits
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new (class of F-13, other types)
- locations: `crates/recipe-domain/src/identity/secret.rs:219`, `crates/recipe-domain/src/identity/secret.rs:293`, `docs/adr/0166-constructor-for-stored-data.md:77`
- rule: CLAUDE.md hard rule (one `parse` constructor), `reliability.md` §4.3, ADR-0166 (u) (names `OneTimePassword::from_bytes` only)
- failure scenario: Any crate can build a token from a constant array and skip the random source. A review cannot reject a new unlisted constructor, because two already exist.
- evidence: `pub const fn from_bytes(bytes: [u8; SESSION_TOKEN_BYTES]) -> Self { Self(bytes) }` · `pub const fn from_bytes(bytes: [u8; CSRF_TOKEN_BYTES]) -> Self { Self(bytes) }`
- refutation tried: `rg from_bytes docs/ CLAUDE.md` finds only ADR-0166 (u) for `OneTimePassword`. No other document lists the token constructors.
- sources: RPA2-01 (Opus)

#### V-17: ADR-0166 (r) cites `leaf.rs` line 209 for the significant-digit check of item (o), but the check is at line 322
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `docs/adr/0166-constructor-for-stored-data.md:71`, `crates/recipe-domain/src/recipe/value/leaf.rs:322`
- rule: ADR-0166 (r), `.claude/rules/docs.md` (cite with a document and a section)
- failure scenario: A reader who follows (r) lands in the doc comment of `fn string` (line 205 to 210). The reader does not land in the `RuleSet::Current` branch of the numeric check.
- evidence: ADR-0166 (r) "Item (o) lives in `crates/recipe-domain/src/recipe/value/leaf.rs` line 209" · `leaf.rs:322` `if rules == RuleSet::Current && let Some(max) = max_significant_digits(primitive)`
- refutation tried: The line drifted after the ADR. An accepted ADR changes only by its Status line and an "Amended by" note (CLAUDE.md). So the fix is an amendment note or a citation by function name in a later ADR.
- sources: RPA2-02 (Opus)

#### V-18: `LoginThrottle`, which `domain-model.md` §2.3 calls a value object, has a public `new` and no `parse`, and its policy is not checked
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-domain/src/identity/throttle.rs:175`, `crates/recipe-domain/src/identity/throttle.rs:90`, `crates/recipe-app/src/identity/throttle.rs:61`, `docs/architecture/domain-model.md:523`
- rule: CLAUDE.md hard rule ("A value object has one `parse` constructor and no public `new`"), `reliability.md` §4.3
- failure scenario: The document and the code disagree. `LoginThrottle::new(policy)` takes a `ThrottlePolicy` with public, unchecked fields, so a value outside `reliability.md` §7.11 reaches the decision without a check.
- evidence: `domain-model.md:523` "**LoginThrottle** (value object, `reliability.md` 7.11, Assumption A22)" · `pub const fn new(policy: ThrottlePolicy) -> Self { Self { policy } }`
- refutation tried: The only production caller passes the policy from the settings, which come from §7.11 defaults, so no wrong value reaches it today. The rule conflict stays.
- sources: RPA2-03 (Opus)

#### V-19: The `# Errors` docs of `Draft::add_parameter`, `check_for_approval`, and `check_definition` omit `E-RCP-056`, `E-RCP-057`, or `E-RCP-058` that the code returns
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-domain/src/recipe/draft/add.rs:56`, `crates/recipe-domain/src/recipe/draft/approval.rs:155`, `crates/recipe-domain/src/recipe/draft/s7.rs:190`, `crates/recipe-domain/src/recipe/draft/add/step_definition.rs:34`
- rule: `domain-model.md` §5.2 rows `ApproveVersion` and `UpdateParameterDefinition`, ADR-0178 (d), ADR-0187, `.claude/rules/rust.md` (doc comments of public items)
- failure scenario: `AddParameter` with a data block above the platform maximum returns `E-RCP-057` from the first group, and an S7 array bound outside `-32768..=32767` returns `E-RCP-058`. The doc of the first group does not list them. `check_for_approval` can return `E-RCP-058` from `check_standard_access_rule`, and its doc does not list it.
- evidence: `add.rs:56` "The first group gives one error: `E-RCP-014`, `E-PLT-020`, `E-PLT-013`, `E-RCP-003`, `E-RCP-053`, `E-RCP-054`, or `E-RCP-023`." · `approval.rs:155` "`E-RCP-023`, `E-RCP-022`, `E-RCP-057`, or `E-RCP-056`." · `s7.rs:194` `check_standard_access_rule(tag, address, tree)?;`
- refutation tried: The S7 rules run in the first group (`s7.rs:190` to `194`), so the codes reach the caller. No other doc comment lists them.
- sources: RPA4-01 (Opus)

#### V-20: ADR-0028 says that `check_for_approval` and `VersionSnapshot::check` reject `LiveUnits` of another machine, but the code compares only the machine of the trees
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-domain/src/recipe/snapshot/check.rs:37`, `crates/recipe-domain/src/recipe/snapshot/check.rs:113`, `crates/recipe-domain/src/recipe/draft/approval.rs:155`, `docs/adr/0028-version-header-and-approval-token.md:63`
- rule: ADR-0028 line 63 ("reject `TypeTrees` and `LiveUnits` of another machine or platform with `E-PLT-013` or `E-PLT-020`")
- failure scenario: A caller passes the `LiveUnits` of machine B for a draft of machine A. Each parameter gives an `UnknownUnit` item inside `E-RCP-020`, not a rejection with `E-PLT-020`. A draft without parameters accepts the foreign units. The app loads the units by `draft.machine()`, so no route reaches the case today.
- evidence: `check.rs:37` `&& trees.machine() != recipe.machine()` (trees only) · no comparison of `units` with the recipe machine in `check.rs` or `approval.rs`
- refutation tried: `rg "units.machine|machine()"` over both files finds only the tree comparison. The source names tests that pin the current behavior, so the ADR or the code must change.
- sources: RPA4-03 (Opus)

#### V-21: `SecurityEvent::forbidden` accepts a missing client address that the table refuses, so such an event would turn `E-WEB-002` into a storage error
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-domain/src/audit/security.rs:142`, `crates/recipe-domain/src/audit/security.rs:146`, `crates/recipe-app/src/auth/authorize.rs:48`, `migrations/0001_schema.sql:280`
- rule: `domain-model.md` §2.4 (`client_ip` is absent only for `AuditChainBroken`), `data-model.md` §3.8 (`security_event_client_ip_present`), ADR-0029
- failure scenario: A `RequestMeta` with `client_ip = None` meets a denied permission. `refuse` builds `SecurityEvent::forbidden(.., None, ..)`, the insert breaks the `CHECK` with SQLSTATE 23514, and the use case returns a storage error instead of `403 E-WEB-002`.
- evidence: `pub fn forbidden(user: UserId, operation: &str, request_id: RequestId, client_ip: Option<IpAddr>, ...` · `CONSTRAINT security_event_client_ip_present CHECK (client_ip IS NOT NULL OR kind = 'audit_chain_broken')`
- refutation tried: The production web paths always pass `Some`, because the layer `client_address` answers `E-WEB-010` without a client. The defect is latent: the type permits a state that the domain rule and the table refuse. The other request constructors take a plain `IpAddr`.
- sources: RPA5-01 (Opus)

#### V-22: `domain-model.md` §2.4 says that three audit actions store a state object, but `data-model.md` §10 and the code have four
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `docs/architecture/domain-model.md:564`, `docs/architecture/data-model.md:2494`, `crates/recipe-domain/src/audit/entry/kind.rs:61`
- rule: `domain-model.md` §2.4 (claims the same rule as ADR-0005 and `data-model.md` §10), `.claude/rules/docs.md` (each rule lives in one document), ADR-0154 (j)
- failure scenario: A reader of §2.4 treats `PlcConnectionExported` as a full-row action. The code puts its state object in `after`, as §10 says.
- evidence: `domain-model.md:564` `"Three actions store a specific state object instead of a row."` · `data-model.md:2494` `"Four actions store a specific state object instead of a row."` · `kind.rs:61` `pub const ALL: [Self; 4] = [...]`
- refutation tried: Both lines read as quoted. The code follows §10, so §2.4 is the stale side.
- sources: RPA5-02 (Opus)

#### V-23: A render or header fault after the commit of `ChangeOwnPassword` drops the new session cookie. So the user sees `500 E-WEB-010` and is logged out for a stored password change
- severity: LOW (kept)
- verdict: PLAUSIBLE
- status: new (class of F-01 and F-06 of the 2026-11-13 record, which do not name this handler)
- locations: `crates/recipe-web/src/pages/own.rs:268`, `crates/recipe-web/src/pages/own.rs:282`, `crates/recipe-web/src/pages/own.rs:287`, `crates/recipe-web/src/pages/own.rs:290`, `crates/recipe-web/src/pages/own.rs:143`
- rule: `api.md` §2 row `POST /me/password` (a new session replaces the current one). ADR-0163: a committed command does not answer with an error that hides the saved change.
- failure scenario: The command commits, deletes every session of the user, and returns the new token. `render(..)?` fails for an HTMX request before line 290 appends the cookie. The answer is `500 E-WEB-010` without `Set-Cookie`, and the next request receives `E-WEB-004`.
- evidence: `let cookie = session_header(&request, issued.token(), state.settings().secure)?;` · `let mut response = render(&PasswordChanged::new(texts), request.request_id())?.into_response();` · `headers.append(axum::http::header::SET_COOKIE, cookie);`
- refutation tried: The order is visible in the code. Both triggers (an askama render fault and a header build fault) are unlikely, and no test forces them, so the verdict stays PLAUSIBLE. `Login` at line 143 has the same order.
- sources: RPB-01 (Opus)

#### V-24: `overview.md` §5 says that `recipe-app` uses `parking_lot` only with the feature `memory`, but the production build always links it since ADR-0172 (y)
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `docs/architecture/overview.md:133`, `crates/recipe-app/Cargo.toml:21`, `crates/recipe-app/Cargo.toml:22`, `docs/adr/0029-application-layer-clarifications.md:79`, `docs/adr/0172-dropped-future-after-a-committed-step.md:100`
- rule: CLAUDE.md "Source of truth and change rule" (change `docs/architecture/` in the same change as the ADR), `overview.md` §5, ADR-0172 (y)
- failure scenario: A reviewer checks the dependency rule against §5 and reads `parking_lot` as a test-only edge of `recipe-app`.
- evidence: `overview.md:133` "`parking_lot` (feature `memory` only) (ADR-0029, ADR-0035)" · `Cargo.toml:21-22` "# The detached tasks hold their `JoinSet` behind a `parking_lot::Mutex` (ADR-0172 (y))." / `parking_lot = { workspace = true }` (not optional)
- refutation tried: The dependency is in `[dependencies]` without `optional = true`. ADR-0029 has no "Amended by ADR-0172" note for this item.
- sources: ARC-01 (Opus)

#### V-25: Five rows of the ADR index omit "Amended by ADR-0174" that the Status line of their ADR file holds
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `docs/adr/README.md:78`, `docs/adr/README.md:79`, `docs/adr/README.md:86`, `docs/adr/README.md:107`, `docs/adr/README.md:115`
- rule: `docs/adr/README.md` "How to add an ADR" step 4 and "Status values". CLAUDE.md: an accepted ADR changes by its Status line and an "Amended by" note.
- failure scenario: A reader of the index takes ADR-0066, ADR-0067, ADR-0074, ADR-0095, and ADR-0103 as not amended by the backup-manifest change of ADR-0174.
- evidence: `README.md:86` `| [0074](0074-names-on-the-backup-disks.md) | Names of the files on the backup disks | Accepted | 2026-10-23 |` · the Status line of each of the five files names ADR-0174. None of the five index rows does (checked with `rg` per number)
- refutation tried: A loop over the five numbers compared the `Status:` line of each file with its index row. Each file has `0174`, and each row has none.
- sources: ARC-02 (Opus)

#### V-26: `recipe-web` copies the distinct-tag count of the `E-RCP-020` message, so the page number can drift from the domain `Display` text
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-web/src/error/recipe.rs:231`, `crates/recipe-domain/src/recipe/error.rs:451`
- rule: `~/.claude/rules/common/coding-style.md` DRY (no project document holds a DRY rule), `reliability.md` §1.2 row `E-RCP-020`
- failure scenario: A later change of the count rule in `recipe-domain` changes the log text of `E-RCP-020`. The Fluent argument of the page keeps the old count.
- evidence: `recipe.rs:228` "/// Count the distinct tags of the items of `E-RCP-020`, as the `Display` of the error does." · both bodies `.filter_map(|item| match item { SnapshotItem::Parameter { tag, .. } | SnapshotItem::UnknownUnit { tag, .. } => Some(tag), SnapshotItem::Version { .. } => None })`
- refutation tried: The domain function is private, so the web layer cannot call it. No test compares the two counts for one item list.
- sources: ARC-03 (Opus)

#### V-27: The closed list of interior mutability in `reliability.md` §8 omits five production places
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `docs/architecture/reliability.md:1248`, `crates/recipe-app/src/identity/detached.rs:77`, `crates/recipe-app/src/identity/detached.rs:79`, `crates/recipe-app/src/blocking.rs:269`, `crates/recipe-app/src/identity/basic_permit.rs:18`, `crates/recipe-web/src/pipeline.rs:70`
- rule: `reliability.md` §8 ("Interior mutability exists only in these places"), `.claude/rules/rust.md` ("No global mutable state")
- failure scenario: A review against the list cannot tell a permitted place from a new one. The detached-task slots and `JoinSet` mutex (ADR-0172 (x), (y)), the permits of `blocking.rs`, the Basic permits, and the request-id counter are not in the list.
- evidence: `reliability.md:1248-1249` "Interior mutability exists only in these places:" / "- the connection pool and the Argon2 semaphore," · `detached.rs:77,79` `slots: Arc<Semaphore>,` / `tasks: Mutex<TaskSet>,` · `pipeline.rs:70` `pub(crate) fn request_id_state() -> Arc<AtomicU64>`
- refutation tried: The list (lines 1248 to 1260) has twelve items, and none names these five places. `recipe-server/src/listener.rs:84-90` treats the list as closed.
- sources: ARC-04 (Opus)

#### V-28: `reliability.md` §8 and `CONTRIBUTING.md` disagree on what `cargo deny check` fails on
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `docs/architecture/reliability.md:1244`, `CONTRIBUTING.md:77`
- rule: `reliability.md` §8 (CONTRIBUTING.md repeats the rules of §2 and §8), CLAUDE.md "When two documents disagree"
- failure scenario: A reader of the normative §8 does not learn that the CI gate also enforces the banned dependency edges of `deny.toml` `[bans]`.
- evidence: `reliability.md:1244` `"It fails on a denied license, a yanked crate, or a security advisory."` · `CONTRIBUTING.md:77` `"It fails on a denied license, a yanked crate, a security advisory, or a banned dependency edge."`
- refutation tried: Both lines read as quoted. CLAUDE.md "Commands" also names "banned dependency edges", so the normative text is the side that is short.
- sources: ARC-05 (Opus)

#### V-29: The answer of a failed `POST /login` shows the language switch. `design-system.md` §10.3 and ADR-0036 (j) forbid it on every page that answers a non-GET method. But the `web-ui` spec requires it
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-web/src/layout.rs:219`, `crates/recipe-web/src/pages/own/templates.rs:46`, `openspec/specs/web-ui/spec.md:578`, `docs/architecture/design-system.md:394`, `docs/adr/0036-design-system-and-ui-foundation.md:100`
- rule: `design-system.md` §10.3 and ADR-0036 (j) against `openspec/specs/web-ui/spec.md:578`
- failure scenario: A wrong password gives the login page through `Layout::for_login` with `next: Some("/login")`, so the full page of a POST answer holds the `POST /lang` form. The code follows the spec, and the normative documents state the rule without a login exception.
- evidence: `design-system.md:394` `"A full page that answers another method has no switch."` · `spec.md:578` "The login page SHALL always hold the switch with `next` set to `/login`, also as the answer of a failed `POST /login`"
- refutation tried: `design-system.md:359` names the switch on `GET /login` only. No ADR that mentions `POST /login` states an exception (`rg` over `docs/adr`). The reason of the rule (a one-time password page) does not apply to the login page. So the fix is a doc exception with an ADR.
- sources: WEB-01 (Opus)

#### V-30: `api.md` §4 step 1 says that every form on the version page holds `row_version`, but the `GET` compare form and the header forms have none
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `docs/architecture/api.md:410`, `crates/recipe-web/templates/version.html:36`, `crates/recipe-web/templates/base.html:28`, `crates/recipe-web/templates/base.html:35`
- rule: `api.md` §4 step 1 and `.claude/rules/web.md` against ADR-0108 (d) and `api.md` §8
- failure scenario: A test that applies step 1 literally to the version page of a Draft finds three forms without `row_version`. They are the compare form, `POST /lang`, and `POST /logout`. None of them mutates the version. So the code is right and the rule text is too wide.
- evidence: `api.md:410` "renders the current `row_version` of the version into a hidden field `row_version` of every form on the page." · `version.html:36` `<form method="get" action="{{ form.action }}" class="actions" hx-boost="true" ...>` without the field
- refutation tried: Step 2 ("Every mutating request on that version sends `row_version`") is the real rule, and the code follows it. Step 1 has no exception for a `GET` form or a header form.
- sources: WEB-02 (Opus)

#### V-31: `reliability.md` §7.7 names the drill command `pitr-volume --drill`, but ADR-0106 and the script require `--target-time` with `--drill`
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new (not the DEP-01 of the 2026-11-13 fix plan, which is the follow-up ADR of ADR-0095 (h))
- locations: `docs/architecture/reliability.md:871`, `docs/adr/0106-recovery-of-production-from-the-wal-archive.md:97`, `deploy/pgtools/bin/pitr-volume:233`
- rule: `reliability.md` §7.7 against ADR-0106 (arguments of `pitr-volume`)
- failure scenario: An operator runs `pitr-volume --drill` as §7.7 names it. The script writes `--drill needs --target-time` and exits 1.
- evidence: `reliability.md:871` "The recovery drill runs `pitr-volume --drill` on a drill volume" · ADR-0106:97 "`--drill` requires `--target-time`." · `pitr-volume:233-234` `if [ "$drill" -eq 1 ] && [ "$have_target_time" -eq 0 ]; then fail "--drill needs --target-time"`
- refutation tried: The runbook gives the full form with `--target-time`, so an operator who follows the runbook is not affected. The normative §7.7 sentence is the short side.
- sources: DEP-01 (Opus)

#### V-32: Watchdog tests prove "no probe after disarm or drop" and "never probes" with fixed real sleeps. So a stalled scheduler can fail a watchdog without a defect
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new (class of F-31, which the 2026-11-13 fix closed for `blocking/tests.rs` only)
- locations: `crates/recipe-server/src/watchdog/tests.rs:278`, `crates/recipe-server/src/watchdog/tests.rs:317`, `crates/recipe-server/src/watchdog/tests.rs:456`
- rule: `reliability.md` §6 (deterministic tests)
- failure scenario: A loaded CI host stalls the probe thread for more than 40 ms after `disarm()`. One more 5 ms probe runs between the two reads, and `settled != later` fails. `a_watchdog_that_is_not_armed_never_probes` proves absence with an 80 ms sleep.
- evidence: `watchdog.disarm(); std::thread::sleep(Duration::from_millis(40)); let settled = calls.load(Ordering::SeqCst); std::thread::sleep(Duration::from_millis(60));`
- refutation tried: The closure record row F-31 names `crates/recipe-app/src/blocking/tests.rs:454` to `:468` only. These watchdog tests still use real sleeps, as read at the cited lines.
- sources: TST-01 (Sonnet)

#### V-33: The signal-wait test has no failure mode other than a hang, and it drops the stop cause without an assertion
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-server/src/serve/tests/signal.rs:99`, `crates/recipe-server/src/serve/tests/signal.rs:104`
- rule: `reliability.md` §6
- failure scenario: If `wait_for_signal` ignores the fired signal, the test blocks until the CI job timeout and gives no assertion message. It also does not assert that the cause is the signal.
- evidence: `host.fire.now(); server.wait_for_signal(); let _report = server.close();`
- refutation tried: The test body at lines 99 to 104 has no deadline and no assertion on `_report`.
- sources: TST-02 (Sonnet)

#### V-34: The XSS coverage guard accepts a corpus test by the text `fn name(` and trusts every `NoText` reason without a check
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-web/src/pages/xss_coverage_tests/compare.rs:89`, `crates/recipe-web/src/pages/xss_coverage_tests/registry.rs:15`
- rule: `reliability.md` §6 (HTTP row: XSS payload corpus through every text field), `.claude/rules/web.md` last bullet
- failure scenario: A corpus test that loses `#[test]` or sits in a disabled `#[cfg(..)]` block still satisfies its entry. A field marked `Cover::NoText` receives no corpus test, and a later handler that echoes it fails no test.
- evidence: `fn has_test_fn(sources: &[Source], name: &str) -> bool { let needle = format!("fn {name}(");` · `Cover::NoText("A number or a flag. A bad text gives a form error without the sent text.")`
- refutation tried: The guard reads source text only. No part of the guard checks the attribute or the reason.
- sources: TST-03 (Sonnet)

#### V-35: An e2e test proves "the link sends no request" with a fixed `waitForTimeout`
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new (class of F-31, in the browser suite)
- locations: `e2e/tests/leaf-grid.spec.ts:156`
- rule: ADR-0048, `reliability.md` §6
- failure scenario: A request that starts after `NO_REQUEST_WINDOW_MS` on a slow runner is missed, so a regression passes. The wait also adds the window to every run.
- evidence: `await page.waitForTimeout(NO_REQUEST_WINDOW_MS); expect(requests, 'the link sends no request').toEqual([]);`
- refutation tried: The test has no other signal of "no request", as read at lines 151 to 157.
- sources: TST-05 (Sonnet)

#### V-36: `read_result_file` checks the result file with `symlink_metadata` and then opens it again by path. So a rename race can swap in a link, a FIFO, or a device
- severity: LOW (kept)
- verdict: `CONFIRMED`
- status: new
- locations: `crates/recipe-server/src/check_results.rs:165`, `crates/recipe-server/src/check_results.rs:170`, `crates/recipe-server/src/check_results.rs:173`
- rule: the doc comment of `read_result_file` ("a link never gives the content of its target"), ADR-0151 (result files of the weekly checks). No architecture document states a link rule for these files.
- failure scenario: A writer of the top directory of `backups` or `walarchive` (root or uid 999) renames a link or a FIFO over the checked file. The rename happens between the two calls. `File::open` follows it. A FIFO pins one blocking thread, and later runs report `Unreadable`. A link target is parsed but not leaked, because `Malformed` carries no bytes.
- evidence: `let metadata = match fs::symlink_metadata(path) {` · `if !metadata.is_file() || metadata.len() > MAX_RESULT_BYTES {` · `let file = match File::open(path) {`
- refutation tried: No `O_NOFOLLOW`, `custom_flags`, or descriptor `fstat` exists in `crates/` (`rg`). The precondition (a privileged writer that can already forge the result line) keeps the severity at LOW.
- sources: TOCTOU-001 (rust-review pipeline, Opus)

### 4.4 Checks of earlier findings and notes

**F-03.** Stale, the entry is fixed. `crates/recipe-server/tests/error_registry.rs` exists. Its `SCOPE` (lines 61 to 108) names nine types: `AuditError`, `StorageError`, `ExportError`, `IdentityError`, `PlantError`, `RecipeError`, `AppError`, `CheckResultFault`, and `WebError`. `SnapshotItem` is the documented exclusion (line 54). The tests at lines 137, 149, and 164 compare the samples with the rows of `reliability.md` §1.2. The 2026-11-13 closure record marks F-03 CLOSED by PR #63. The notes of ARC (item 2) and RPA2 (item 5) find 132 codes, 132 rows, and 132 `REGISTRY` entries with no duplicate, which agrees.

**F-12.** Stale, the entry is fixed. `PlantErrors::code` exists at `crates/recipe-domain/src/plant/errors.rs:34` and returns the code of the first entry. `CheckResultFault::code` exists at `crates/recipe-app/src/ports/check_results.rs:71`. `reliability.md:22` names `AcceptLoopEnded` as the one exception, which equals CLAUDE.md. The closure record marks F-12 CLOSED by PR #66 (ADR-0165). The ARC note (item 2) and the RPA2 note (item 5) also find `code()` on every error type.

**w13.** Not a defect under the current ADRs. The note names `crates/recipe-web/src/authorize.rs:55`, but the file is `crates/recipe-app/src/auth/authorize.rs:55` (`refuse`). `refuse` and `csrf::reject` (`crates/recipe-web/src/csrf.rs:198`) await `record` with no committed step before it, so a dropped future loses no committed effect and no delivered refusal. ADR-0172 (g) limits the detached-task rule to `Login`, `ChangeOwnPassword`, `UnlockLogin`, and `AuthenticateBasic`, which commit a step first. ADR-0190 (c) keeps the inline write of `csrf_rejected` on purpose. "D18" is not in `reliability.md`: it is design D18 of the archived change `2026-10-13-app-use-cases-and-ports` (a failed write gives the storage error), and ADR-0029 records it. Residual risk: a client that closes the connection before the INSERT commits can suppress one `forbidden` or `csrf_rejected` event, which the `security_events` alert of `reliability.md` §3.7 reads. No document names this limit. That is a candidate for a LOW doc note, not a finding of this review.

**RPA1-01/02 vs F-15.** Same class, other commands, so V-08 is `new` and not a regression. F-15 of the 2026-11-13 record named only `CreateUser`, `ResetPassword`, and `CreateRecipe` (record line 258). The fix (ADR-0168 (a) to (e), closure row F-15 CLOSED by #66) added "Before the transaction" to those three rows of `data-model.md` §6 only. The code of `plant/resolve.rs` and `recipe/resolve.rs` was not changed by that fix, and the record did not list it, so the earlier review missed these paths. A fix needs one general sentence in the §6 intro or one row note per command, plus an ADR, because §6 is normative.

**RPA3-04 vs F-13.** Same class, other type, so V-13 is `new`. F-13 covered `from_stored` on value objects, and ADR-0166 (a) to (g) closed it by naming `from_stored` as a permitted second constructor for the listed holders. `PlcConnection` has no `from_stored`. The stored read of `recipe-db` builds the enum variants directly. The database `CHECK` constraints of migration 0012 make the corrupt-row scenario of RPA3-04 impossible, so only the doc conflict remains.

## 5. Rejected inputs and doc conflicts

### 5.1 Rejected inputs

| source id | reason (one sentence, with file:line) |
|---|---|
| TST-06 | An independent consumer test exists: `crates/recipe-export/tests/golden_types.rs:579` rebuilds `content_sha256` of each golden file with `serde_json_canonicalizer` and `sha2` and not the domain hash, and `crates/recipe-domain/src/export/hash/tests.rs:90` hashes a hand-written canonical text with `sha2`. |
| RPA4-02 | The bug needs a state that cannot arise. `ArchiveMachine` refuses a machine with a live draft in code (`crates/recipe-app/src/plant/archive_machine.rs:89`, `E-PLT-011 MachineHasDraft`, `docs/architecture/data-model.md:2258`). It also locks every recipe of the machine `FOR UPDATE`, which serializes it with `ApproveVersion`. So approval never sees a valid draft on an archived machine. |
| SQL-01 | No rule is broken. `.claude/rules/sql.md` limits column-level UPDATE grants. Every entity table in `migrations/0003_grants.sql:34` (`machine`, `unit`, `recipe`, `app_user`, and others) has the same table-level INSERT grant. Only `recipe_version` has an insert guard (`migrations/0016_recipe_version_insert.sql:7`, ADR-0169) for its state rule. The only INSERT of `plc_connection` (`crates/recipe-db/src/transaction/plc_connection.rs:55`) names neither column. So the finding is a hardening idea without a concrete failure. The F-19 class does not carry over, because F-19 and ADR-0169 protect the documented state rule of a new `recipe_version` row. No document gives a database rule for the first `row_version` of `plc_connection`. |
| TST-04 | The state cannot exist on PostgreSQL. The version is locked `FOR UPDATE`, so the recorded `row_version` cannot differ from the row (`crates/recipe-db/tests/storage/contract/recipe_write/stale_approve.rs:3`). The same holds for `stale_record.rs:3`. A PostgreSQL test cannot reach the case, so the memory-only test is the only possible test. |
| TST-07 | `docs/architecture/api.md` §6.2 states `"The hash value in the example is a placeholder."`, so the sample is not a test vector, and the tests mask or replace it on purpose. |

### 5.2 Doc conflicts

Each row is a finding of section 4 with a conflict between two documents, or between a document and the code.

| V-nn | document A § | document B § |
|---|---|---|
| V-05 | `api.md` §2.6 row `POST .../definition` (value and limits optional) | `crates/recipe-web/src/pages/parameters/edit.rs:53` (always `retype: Some`) |
| V-08 | `data-model.md` §6 intro (one transaction per command) | `crates/recipe-app/src/plant/resolve.rs:43`, `crates/recipe-app/src/recipe/resolve.rs:12` (a READ ONLY pre-read) |
| V-09 | ADR-0042 (f), `reliability.md` §7.4 | `crates/recipe-app/src/recipe/queries/parameters.rs:375` (resolve inside the transaction) |
| V-10 | doc comment of `TryFrom<StoredMachine>` | `crates/recipe-domain/src/plant/machine.rs:444` (check order) |
| V-12 | `domain-model.md` §2.1 check order (step 2) | `crates/recipe-domain/src/plant/connection/shape.rs:268` (credential check after the mode) |
| V-13 | `domain-model.md` §2.1 (one constructor `for_platform`) | `crates/recipe-db/src/transaction/plc_connection.rs:216` (direct variants) |
| V-15 | ADR-0171 Consequences (plain HTTP works) | ADR-0035 (e), `api.md` §7.3, `crates/recipe-web/src/csrf.rs:46` (`https://` only) |
| V-16 | ADR-0166 (u), `reliability.md` §4.3 | `crates/recipe-domain/src/identity/secret.rs:219`, `:293` (`from_bytes`) |
| V-17 | ADR-0166 (r) (line 209) | `crates/recipe-domain/src/recipe/value/leaf.rs:322` |
| V-18 | `domain-model.md` §2.3 (`LoginThrottle` is a value object) | `crates/recipe-domain/src/identity/throttle.rs:175` (public `new`) |
| V-19 | `domain-model.md` §5.2 rows `ApproveVersion`, `UpdateParameterDefinition` | `crates/recipe-domain/src/recipe/draft/add.rs:56`, `approval.rs:155` (doc lists) |
| V-20 | ADR-0028 line 63 (foreign `LiveUnits` rejected) | `crates/recipe-domain/src/recipe/snapshot/check.rs:37` (trees only) |
| V-21 | `domain-model.md` §2.4, `data-model.md` §3.8 | `crates/recipe-domain/src/audit/security.rs:146` (`Option<IpAddr>`) |
| V-22 | `domain-model.md` §2.4 (three actions) | `data-model.md` §10 (four actions) |
| V-24 | `overview.md` §5 (feature `memory` only) | ADR-0172 (y), `crates/recipe-app/Cargo.toml:22` |
| V-25 | `docs/adr/README.md` index rows 0066, 0067, 0074, 0095, 0103 | Status lines of those ADR files (Amended by ADR-0174) |
| V-27 | `reliability.md` §8 (closed list of interior mutability) | ADR-0172 (x), (y), `crates/recipe-app/src/blocking.rs:269`, `crates/recipe-web/src/pipeline.rs:70` |
| V-28 | `reliability.md` §8 (`cargo deny` fail cases) | `CONTRIBUTING.md:77` |
| V-29 | `design-system.md` §10.3, ADR-0036 (j) | `openspec/specs/web-ui/spec.md:578` |
| V-30 | `api.md` §4 step 1 | ADR-0108 (d), `api.md` §8 |
| V-31 | `reliability.md` §7.7 (`pitr-volume --drill`) | ADR-0106 arguments (`--drill` requires `--target-time`) |

## 6. Tools that were missing or failed

The review continued with the other tools after each failure. No tool was installed.

| Tool or step | What happened | Effect |
|---|---|---|
| `ocr scan` (open-code-review) | `ocr llm test` found no LLM endpoint. | The user chose the `delegate` mode. OCR selected the files and gave the rules. An Opus agent did the review. |
| `ocr delegate preview --from <empty tree>` | It failed with "expected commit type". | The file list of `ocr scan --preview` replaced it. |
| OCR coverage | The agent reviewed 58 of 1087 selected files (5.3 %). | The other aspects cover the units that OCR skipped (section 3). |
| Agent budget | The prompt set 11 own agents. The run used 15. | The user approved the OCR agent and the three extra agents RPA3 to RPA5. RPA was split into RPA1 and RPA2. GAT also did the STE pass, so no 16th agent was necessary. |
| RPA2 context budget | RPA2 stopped at about 570k tokens before it read all files of `recipe-domain`. | RPA3, RPA4, and RPA5 read the remaining files. `recipe-domain` is Full. |
| review-council agent types | `architecture-reviewer` and `interface-reviewer` have only Read, Glob, and Grep. | ARC and WEB ran as general-purpose Opus agents. |
| Skill `htmx:htmx` | The skill was not in the skill list of the session. | WEB used `api.md` §9 and `.claude/rules/web.md` as the HTMX rules. |
| Skill name `rust-pr-review:rust-pr-review` | The name is not valid. | RPA5 loaded `rust-pr-review`. The gate script and the LSP steps of the skill did not run, because the briefs forbid cargo. |
| rust-review plan | The plan used relative paths, and the agents start in the main checkout. | The orchestrator built the plan again with absolute worktree paths. |
| rust-review `has_ffi` probe | The probe matched only test sources. | The orchestrator set `has_ffi=false`. |
| rust-review ledger | `TaskCreate` and `TaskList` were not available. | The orchestrator tracked the 17 workers in its state file. |
| rust-review spawn mode | The harness runs every agent asynchronously. | The sharing of the primer cache between the workers is not certain. |
| rust-review concurrency | The harness limits the number of parallel subagents. | The orchestrator started the workers in two waves. All 17 workers completed and passed the artifact check. |
| `shellcheck` | It is not installed on this host. | DEP ran `bash -n` on 53 shell files. No agent ran a shell lint. |
| `serena`, `pdf-viewer` | The MCP servers failed to connect. | No agent needed them. |
| `advisor` call of RPB | The call was rate-limited. | RPB continued without it. |
| context-mode hook | The hook cuts each tool result above about 4 KB. | The agents read files in chunks. |
| Checkpoint hook | A hook asked for a checkpoint file under `target/p33-agents/` in the worktree. | The agents did not write it, because the worktree was read-only. |
| Browser | No agent ran a browser. | V-15 stays PLAUSIBLE. |

## 7. Open questions

1. **V-01 (LF in a string leaf).** Must the domain refuse LF and CR in a value leaf? Or must the form keep the text, for example with a `textarea`? Both answers need an ADR, because ADR-0137 permits every character except U+0000.
2. **V-04 (body limit).** Must `http.body_limit.value` become an operator setting, follow the leaf budget, or must a rule limit the total length of a leaf path? The answer changes `reliability.md` §7.11 and needs an ADR.
3. **V-02 (PITR end check).** Does the owner accept the gap of ADR-0106 (g) and amend the runbook text? Or does the archive need a manifest or a hash per segment?
4. **Doc conflicts of section 5.2.** Each row needs a decision about which document wins. A change of `docs/architecture/` needs a new ADR in the same change.
5. **Lost security events (w13 note).** A client that closes the connection before the INSERT commits can suppress one `forbidden` or `csrf_rejected` event. No document names this limit. Does the owner want a doc note?
6. **V-15 (plain HTTP in `dev`).** A browser run must show if the CSRF origin check refuses every mutating request on plain HTTP.
7. **Coverage gaps.** OCR covered 5.3 % of its files. `deploy/db/check-*.sql`, parts of `ci.yml`, and most of `e2e/` got no full read. Does the owner want one more pass over these units?
